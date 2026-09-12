//! Generation orchestration: prompt → AI → normalize → validate → stage.
//!
//! Fail-closed pipeline (D3/D8): a truncated response, an unparseable one,
//! or one that violates the overlay contract is NEVER staged — each surfaces
//! as `GenerationInvalidOutput` with the collected issues.
//!
//! `accept`/`discard` operate on the staging id only (uuid4, sweep-guarded —
//! D4) and derive the template id from the STAGED manifest, never from a
//! caller-supplied path segment.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use uuid::{Uuid, Version};

use crate::application::key_service::KeyService;
use crate::application::ports::AiProvider;
use crate::application::template_catalog::OverlaysDirHandle;
use crate::domain::ai::{AiError, GeneratedFiles, GeneratedOverlay, GeneratedOverlaySummary};
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::name::{normalize_name, validate_name};
use crate::domain::template::OverlayField;
use crate::infrastructure::overlay_writer::OverlayWriter;
use crate::infrastructure::validator::validate_generated_overlay;

/// Fixed suffix appended to the base prompt: the EXACT JSON shape the model
/// must answer with (nothing else, no fences, no prose).
const OUTPUT_SHAPE: &str = r#"
RESPOND ONLY with a single valid JSON object, no markdown fences, no text before or after, using this exact shape:
{
  "name": "Nombre visible del overlay",
  "files": {
    "overlay_json": "contenido completo de overlay.json (sin campo id top-level)",
    "index_html": "contenido completo de index.html",
    "style_css": "contenido completo de style.css",
    "script_js": "contenido completo de script.js"
  }
}
The folder name will be derived from "name" (kebab-case) — you do not provide the id.
"#;

/// The real system prompt: base contract + output shape instruction.
fn system_prompt() -> String {
    format!(
        "{}\n\n{}",
        crate::infrastructure::ai::BASE_PROMPT,
        OUTPUT_SHAPE
    )
}

pub struct GenerationService {
    keys: Arc<KeyService>,
    provider: Arc<dyn AiProvider>,
    writer: OverlayWriter,
    overlays_dir: OverlaysDirHandle,
}

impl GenerationService {
    pub fn new(
        keys: Arc<KeyService>,
        provider: Arc<dyn AiProvider>,
        overlays_dir: OverlaysDirHandle,
    ) -> Self {
        let writer = OverlayWriter::new(overlays_dir.get());
        Self {
            keys,
            provider,
            writer,
            overlays_dir,
        }
    }

    /// Generate, normalize, validate and stage an overlay from a free-text
    /// description. Returns the staging summary for the UI.
    pub fn generate(&self, prompt: &str) -> DomainResult<GeneratedOverlaySummary> {
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return Err(DomainError::GenerationEmptyPrompt);
        }
        let root = self.overlays_dir.get();
        if root.as_os_str().is_empty() {
            return Err(DomainError::OverlaysDirMissing);
        }
        // The writer must track config changes, so rebuild it per call.
        let writer = OverlayWriter::new(root);

        let secret = self
            .keys
            .secret(crate::domain::ai::ProviderKind::ANTHROPIC)?;
        let model = self
            .provider
            .models()
            .first()
            .cloned()
            .ok_or(DomainError::UnknownModel)?;

        let response = self
            .provider
            .generate(&model, prompt, &system_prompt(), &secret)
            .map_err(map_ai_error)?;

        if response.truncated {
            return Err(DomainError::GenerationInvalidOutput {
                issues: vec![
                    "la respuesta de la IA quedó truncada — probá con una descripción más corta"
                        .into(),
                ],
            });
        }

        let overlay = normalize_generated(&response.text)?;
        let report = validate_generated_overlay(&overlay.files, &overlay.directory);
        if !report.valid {
            return Err(DomainError::GenerationInvalidOutput {
                issues: report.errors,
            });
        }

        let staged = writer.stage(&overlay)?;
        let staging_id = staged
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let fields = extract_fields(&overlay.files.overlay_json);

        Ok(GeneratedOverlaySummary {
            staging_id,
            directory: overlay.directory,
            name: overlay.name,
            fields,
        })
    }

    /// Promote a staged overlay into the template tree. The template id is
    /// derived from the staged manifest's `name` (normalized), never from a
    /// caller-supplied directory.
    pub fn accept(&self, staging_id: &str) -> DomainResult<()> {
        let staged = self.staging_path(staging_id)?;
        let manifest = fs::read_to_string(staged.join("overlay.json"))
            .map_err(|_| DomainError::StagedOverlayMissing)?;
        let value: Value =
            serde_json::from_str(&manifest).map_err(|_| DomainError::StagedOverlayMissing)?;
        let name = value["name"]
            .as_str()
            .ok_or(DomainError::StagedOverlayMissing)?;
        let directory = normalize_name(name);
        validate_name(&directory)?;
        self.writer.accept(&staged, &directory)
    }

    /// Remove a staged overlay (user rejected it or it expired).
    pub fn discard(&self, staging_id: &str) -> DomainResult<()> {
        let staged = self.staging_path(staging_id)?;
        self.writer.discard(&staged)
    }

    /// Sweep-guarded staging path: only a uuid4-shaped id maps to a path
    /// (D4) — anything else is a missing staged overlay, full stop.
    fn staging_path(&self, staging_id: &str) -> DomainResult<PathBuf> {
        let id = Uuid::parse_str(staging_id).map_err(|_| DomainError::StagedOverlayMissing)?;
        if id.get_version() != Some(Version::Random) {
            return Err(DomainError::StagedOverlayMissing);
        }
        Ok(self.writer.staging_root().join(id.to_string()))
    }
}

fn map_ai_error(e: AiError) -> DomainError {
    match e {
        AiError::Unauthorized => DomainError::ProviderUnauthorized,
        AiError::RateLimited => DomainError::ProviderRateLimited,
        AiError::Timeout => DomainError::ProviderTimeout,
        AiError::Network { detail } => DomainError::ProviderNetwork { detail },
        AiError::ServerError { detail } => DomainError::ProviderUnavailable { detail },
        AiError::InvalidResponse { detail } => DomainError::ProviderInvalidResponse { detail },
    }
}

/// Parse raw provider text into a `GeneratedOverlay`, deriving the template
/// id from the visible `name` (normalized kebab-case) — the AI-provided id,
/// if any, is ignored (D3).
fn normalize_generated(text: &str) -> DomainResult<GeneratedOverlay> {
    let json = extract_json(text).ok_or_else(|| DomainError::GenerationInvalidOutput {
        issues: vec!["la respuesta de la IA no contiene un objeto JSON válido".into()],
    })?;
    let value: Value =
        serde_json::from_str(&json).map_err(|e| DomainError::GenerationInvalidOutput {
            issues: vec![format!("JSON inválido: {e}")],
        })?;
    let name = value["name"]
        .as_str()
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| DomainError::GenerationInvalidOutput {
            issues: vec!["falta el campo \"name\" en la respuesta".into()],
        })?
        .to_string();
    let directory = normalize_name(&name);
    validate_name(&directory)?;

    let files = value["files"]
        .as_object()
        .ok_or_else(|| DomainError::GenerationInvalidOutput {
            issues: vec!["falta el objeto \"files\" en la respuesta".into()],
        })?;
    let read_str = |key: &str| {
        files
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    let generated_files = GeneratedFiles {
        overlay_json: read_str("overlay_json").ok_or_else(|| {
            DomainError::GenerationInvalidOutput {
                issues: vec!["falta files.overlay_json".into()],
            }
        })?,
        index_html: read_str("index_html").ok_or_else(|| DomainError::GenerationInvalidOutput {
            issues: vec!["falta files.index_html".into()],
        })?,
        style_css: read_str("style_css").ok_or_else(|| DomainError::GenerationInvalidOutput {
            issues: vec!["falta files.style_css".into()],
        })?,
        script_js: read_str("script_js").ok_or_else(|| DomainError::GenerationInvalidOutput {
            issues: vec!["falta files.script_js".into()],
        })?,
    };

    Ok(GeneratedOverlay {
        directory,
        name,
        files: generated_files,
    })
}

/// Locate the first balanced JSON object in a text (strips markdown fences
/// and any prose the model might have added).
fn extract_json(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in text[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..start + i + c.len_utf8()].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_fields(overlay_json: &str) -> Vec<OverlayField> {
    let Ok(value) = serde_json::from_str::<Value>(overlay_json) else {
        return Vec::new();
    };
    let Some(fields) = value["fields"].as_array() else {
        return Vec::new();
    };
    fields
        .iter()
        .filter_map(|f| serde_json::from_value::<OverlayField>(f.clone()).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::KeyStore;
    use crate::application::test_utils::{unique_temp_dir, MemoryConfigRepo, MemoryKeyStore};
    use crate::domain::ai::{AiText, ProviderKind};

    struct StubProvider {
        reply: Result<AiText, AiError>,
        model: String,
    }

    impl AiProvider for StubProvider {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Anthropic
        }

        fn models(&self) -> Vec<String> {
            vec![self.model.clone()]
        }

        fn generate(
            &self,
            _model: &str,
            _prompt: &str,
            _system: &str,
            _key: &str,
        ) -> Result<AiText, AiError> {
            self.reply.clone()
        }
    }

    fn valid_ai_json(name: &str) -> String {
        format!(
            r#"{{
  "name": "{name}",
  "files": {{
    "overlay_json": "{{\"name\":\"{name}\",\"fields\":[{{\"key\":\"titulo\",\"label\":\"Título\",\"type\":\"text\",\"default\":\"Hola\"}}]}}",
    "index_html": "<!doctype html><link rel=\"stylesheet\" href=\"style.css\"><script src=\"script.js\"></script>",
    "style_css": "body {{ background: transparent; }}",
    "script_js": "const TEMPLATE_ID = \"{slug}\";const INSTANCE_ID = new URLSearchParams(window.location.search).get(\"instance\");let ws;function connect(){{ws = new WebSocket(`ws://${{location.host}}/ws`);ws.onmessage = (e) => {{const msg = JSON.parse(e.data);if (msg.template !== TEMPLATE_ID || (INSTANCE_ID && msg.instance_id !== INSTANCE_ID)) return;if (msg.action === \"show\") show(msg.fields);if (msg.action === \"update\") update(msg.fields);if (msg.action === \"hide\") hide();}};ws.onclose = () => setTimeout(connect, 2000);}}connect();function show(f){{}}function update(f){{}}function hide(){{}}"
  }}
}}"#,
            name = name,
            slug = normalize_name(name)
        )
    }

    // Unique dir per test: parallel tests share the pid (flaky-race class
    // already fixed in fs_template_source/http/overlay_writer tests); the
    // shared counter in test_utils makes every tag unique too.
    fn service_with(
        tag: &str,
        reply: Result<AiText, AiError>,
    ) -> (GenerationService, OverlaysDirHandle) {
        let keystore = Arc::new(MemoryKeyStore::new());
        keystore.set("anthropic", "sk-test").unwrap();
        let keys = Arc::new(KeyService::new(Arc::new(MemoryConfigRepo::new()), keystore));
        let provider: Arc<dyn AiProvider> = Arc::new(StubProvider {
            reply,
            model: "claude-test".into(),
        });
        let dir = unique_temp_dir(&format!("gen-{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let handle = OverlaysDirHandle::new(dir.clone());
        (
            GenerationService::new(keys, provider, handle.clone()),
            handle,
        )
    }

    fn cleanup(dir: &OverlaysDirHandle) {
        let _ = fs::remove_dir_all(dir.get());
    }

    #[test]
    fn generates_and_stages_a_valid_overlay() {
        let (service, dir) = service_with(
            "valid",
            Ok(AiText {
                text: valid_ai_json("Mi Overlay"),
                truncated: false,
            }),
        );

        let summary = service.generate("zócalo inferior minimalista").unwrap();
        assert_eq!(summary.directory, "mi-overlay");
        assert_eq!(summary.name, "Mi Overlay");
        assert_eq!(summary.fields.len(), 1);
        assert_eq!(summary.fields[0].key, "titulo");
        assert!(dir
            .get()
            .join(".staging")
            .join(&summary.staging_id)
            .is_dir());
        cleanup(&dir);
    }

    #[test]
    fn rejects_empty_prompt() {
        let (service, dir) = service_with(
            "empty",
            Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            }),
        );
        assert!(matches!(
            service.generate("   "),
            Err(DomainError::GenerationEmptyPrompt)
        ));
        cleanup(&dir);
    }

    #[test]
    fn rejects_missing_overlays_dir() {
        let (service, dir) = service_with(
            "nodir",
            Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            }),
        );
        dir.set(PathBuf::new());
        assert!(matches!(
            service.generate("algo"),
            Err(DomainError::OverlaysDirMissing)
        ));
        cleanup(&dir);
    }

    #[test]
    fn surfaces_keyring_failure_when_no_key() {
        let keystore = Arc::new(MemoryKeyStore::new());
        let keys = Arc::new(KeyService::new(Arc::new(MemoryConfigRepo::new()), keystore));
        let provider: Arc<dyn AiProvider> = Arc::new(StubProvider {
            reply: Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            }),
            model: "m".into(),
        });
        let dir = unique_temp_dir("gen-nokey");
        fs::create_dir_all(&dir).unwrap();
        let service = GenerationService::new(keys, provider, OverlaysDirHandle::new(dir.clone()));

        assert!(matches!(
            service.generate("algo"),
            Err(DomainError::KeyringEntryMissing { .. })
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn truncated_response_never_stages() {
        let (service, dir) = service_with(
            "trunc",
            Ok(AiText {
                text: valid_ai_json("Mi Overlay"),
                truncated: true,
            }),
        );
        assert!(matches!(
            service.generate("algo"),
            Err(DomainError::GenerationInvalidOutput { ref issues })
                if issues.iter().any(|i| i.contains("truncada"))
        ));
        assert!(!dir.get().join(".staging").exists());
        cleanup(&dir);
    }

    #[test]
    fn invalid_json_is_rejected() {
        let (service, dir) = service_with(
            "badjson",
            Ok(AiText {
                text: "esto no es json".into(),
                truncated: false,
            }),
        );
        assert!(matches!(
            service.generate("algo"),
            Err(DomainError::GenerationInvalidOutput { .. })
        ));
        cleanup(&dir);
    }

    #[test]
    fn contract_violations_are_collected() {
        // style.css is opaque → fails validation.
        let (service, dir) = service_with("contract", Ok(AiText {
            text: r#"{"name":"X","files":{"overlay_json":"{\"name\":\"X\",\"fields\":[]}","index_html":"<b></b>","style_css":"body{background:#000}","script_js":"const TEMPLATE_ID=\"x\";function show(f){}function update(f){}function hide(){}"}}"#.into(),
            truncated: false,
        }));
        assert!(matches!(
            service.generate("algo"),
            Err(DomainError::GenerationInvalidOutput { ref issues })
                if issues.iter().any(|i| i.contains("transparent"))
        ));
        cleanup(&dir);
    }

    #[test]
    fn provider_errors_map_to_domain_errors() {
        let cases = [
            (AiError::Unauthorized, DomainError::ProviderUnauthorized),
            (AiError::RateLimited, DomainError::ProviderRateLimited),
            (AiError::Timeout, DomainError::ProviderTimeout),
            (
                AiError::Network { detail: "x".into() },
                DomainError::ProviderNetwork { detail: "x".into() },
            ),
            (
                AiError::ServerError { detail: "x".into() },
                DomainError::ProviderUnavailable { detail: "x".into() },
            ),
            (
                AiError::InvalidResponse { detail: "x".into() },
                DomainError::ProviderInvalidResponse { detail: "x".into() },
            ),
        ];
        for (ai, domain) in cases {
            let (service, dir) = service_with("perr", Err(ai));
            assert!(
                matches!(service.generate("algo"), Err(e) if std::mem::discriminant(&e) == std::mem::discriminant(&domain))
            );
            cleanup(&dir);
        }
    }

    #[test]
    fn accept_derives_template_id_from_staged_manifest() {
        let (service, dir) = service_with(
            "accept",
            Ok(AiText {
                text: valid_ai_json("Zócalo Deportes"),
                truncated: false,
            }),
        );
        let summary = service.generate("zócalo de deportes").unwrap();

        service.accept(&summary.staging_id).unwrap();
        assert!(dir
            .get()
            .join("zocalo-deportes")
            .join("overlay.json")
            .is_file());
        assert!(!dir
            .get()
            .join(".staging")
            .join(&summary.staging_id)
            .exists());
        cleanup(&dir);
    }

    #[test]
    fn accept_refuses_non_uuid_and_non_v4_ids() {
        let (service, dir) = service_with(
            "accept",
            Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            }),
        );
        assert!(matches!(
            service.accept("../../etc/passwd"),
            Err(DomainError::StagedOverlayMissing)
        ));
        // A valid-format but v1 uuid (not random/v4) is also refused.
        assert!(matches!(
            service.accept("550e8400-e29b-11d4-a716-446655440000"),
            Err(DomainError::StagedOverlayMissing)
        ));
        cleanup(&dir);
    }

    #[test]
    fn discard_removes_staging_and_keeps_templates() {
        let (service, dir) = service_with(
            "acc-ref",
            Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            }),
        );
        let summary = service.generate("algo").unwrap();

        service.discard(&summary.staging_id).unwrap();
        assert!(!dir
            .get()
            .join(".staging")
            .join(&summary.staging_id)
            .exists());
        cleanup(&dir);
    }

    #[test]
    fn extract_json_handles_fences_and_prose() {
        assert_eq!(
            extract_json("```json\n{\"a\":1}\n```").unwrap(),
            "{\"a\":1}"
        );
        assert_eq!(
            extract_json("aquí va: {\"a\":{\"b\":2}} y nada más").unwrap(),
            "{\"a\":{\"b\":2}}"
        );
        assert_eq!(extract_json("sin llaves"), None);
        // Braces inside strings are not counted.
        assert_eq!(extract_json(r#"{"s":"}"}"#).unwrap(), r#"{"s":"}"}"#);
    }
}
