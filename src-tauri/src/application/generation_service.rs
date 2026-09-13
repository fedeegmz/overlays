//! Generation orchestration: prompt → AI → normalize → validate → stage.
//!
//! Fail-closed pipeline (D3/D8): a truncated response, an unparseable one,
//! or one that violates the overlay contract is NEVER staged — each surfaces
//! as `GenerationInvalidOutput` with the collected issues. A content-quality
//! failure triggers exactly ONE retry whose second prompt embeds the issues;
//! transport errors are never retried.
//!
//! `accept`/`discard` operate on the staging id only (uuid4, sweep-guarded —
//! D4) and derive the template id from the STAGED manifest, never from a
//! caller-supplied path segment. The writer is rebuilt from the live
//! overlays dir on every operation, so config changes are always honored.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use uuid::{Uuid, Version};

use crate::application::key_service::KeyService;
use crate::application::ports::AiProvider;
use crate::application::template_catalog::OverlaysDirHandle;
use crate::domain::ai::{
    AiError, AiText, GeneratedFiles, GeneratedOverlay, GeneratedOverlaySummary, ValidationIssue,
};
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::name::{normalize_name, validate_name};
use crate::domain::template::OverlayField;
use crate::infrastructure::overlay_writer::OverlayWriter;
use crate::infrastructure::validator::validate_generated_overlay;

pub struct GenerationService {
    keys: Arc<KeyService>,
    provider: Arc<dyn AiProvider>,
    /// Full system prompt (composed in the infrastructure layer and injected
    /// — the application layer must not import prompt constants).
    system: String,
    overlays_dir: OverlaysDirHandle,
}

impl GenerationService {
    pub fn new(
        keys: Arc<KeyService>,
        provider: Arc<dyn AiProvider>,
        system: String,
        overlays_dir: OverlaysDirHandle,
    ) -> Self {
        Self {
            keys,
            provider,
            system,
            overlays_dir,
        }
    }

    /// Generate, normalize, validate and stage an overlay. The provider must
    /// be a supported adapter (`ProviderKind::ALL`), the model one of that
    /// adapter's `models()`, and the requested name MUST normalize to a valid
    /// kebab-case id — the backend never trusts the UI. Returns the staging
    /// summary (with the four file contents) for the preview panel.
    pub fn generate(
        &self,
        provider: &str,
        model: &str,
        name: &str,
        prompt: &str,
    ) -> DomainResult<GeneratedOverlaySummary> {
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return Err(DomainError::GenerationEmptyPrompt);
        }
        // Param gates run BEFORE any keyring access: unknown provider/model
        // or an unrevalidatable name fail fast without touching the store.
        if !crate::domain::ai::ProviderKind::ALL.contains(&provider) {
            return Err(DomainError::UnknownProvider);
        }
        if !self.provider.models().iter().any(|m| m == model) {
            return Err(DomainError::UnknownModel);
        }
        let name = name.trim();
        if name.is_empty() {
            return Err(DomainError::InvalidName {
                reason: "name must not be empty".into(),
            });
        }
        let requested_directory = normalize_name(name);
        validate_name(&requested_directory)?;

        let root = self.overlays_dir.get();
        if root.as_os_str().is_empty() {
            return Err(DomainError::OverlaysDirMissing);
        }
        // The writer must track config changes, so rebuild it per call.
        let writer = OverlayWriter::new(root);

        let secret = self.keys.secret(provider)?;
        let full_prompt = format!(
            "The requested overlay name is \"{name}\". Use it as the visible name.\n\n{prompt}"
        );

        // Exactly one retry on content-quality failures (truncated/invalid
        // output), embedding the collected issues into the second prompt.
        // Transport errors are returned immediately and never retried.
        let first = self
            .provider
            .generate(model, &full_prompt, &self.system, &secret)
            .map_err(map_ai_error)?;
        let overlay = match self.to_overlay(&first) {
            Ok(overlay) => overlay,
            Err(DomainError::GenerationInvalidOutput { issues }) => {
                let feedback = feedback_block(&issues);
                let second = self
                    .provider
                    .generate(
                        model,
                        &format!("{full_prompt}\n\n{feedback}"),
                        &self.system,
                        &secret,
                    )
                    .map_err(map_ai_error)?;
                self.to_overlay(&second)?
            }
            Err(e) => return Err(e),
        };

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
            files: overlay.files,
        })
    }

    /// Models available for a provider (front of the UI model picker). The
    /// provider is validated the same way `generate` validates it.
    pub fn models(&self, provider: &str) -> DomainResult<Vec<String>> {
        if !crate::domain::ai::ProviderKind::ALL.contains(&provider) {
            return Err(DomainError::UnknownProvider);
        }
        Ok(self.provider.models())
    }

    /// Fail-closed conversion (D3/D8): truncation or contract violations
    /// surface as `GenerationInvalidOutput` — nothing is staged.
    fn to_overlay(&self, response: &AiText) -> DomainResult<GeneratedOverlay> {
        if response.truncated {
            return Err(DomainError::GenerationInvalidOutput {
                issues: vec![ValidationIssue::new("truncated")],
            });
        }
        let overlay = normalize_generated(&response.text)?;
        let report = validate_generated_overlay(&overlay.files, &overlay.directory);
        if !report.valid {
            return Err(DomainError::GenerationInvalidOutput {
                issues: report.issues,
            });
        }
        Ok(overlay)
    }

    /// Promote a staged overlay into the template tree. The template id comes
    /// from STAGED CONTENT only — the script's pinned `TEMPLATE_ID` (the
    /// overlay contract's authority) — never from a caller-supplied path
    /// segment. An optional `edited_name` renames the overlay first via a
    /// targeted, fail-closed rewrite (D11/D13): the script must pin EXACTLY
    /// one template id, and the manifest's `name` must normalize to it —
    /// otherwise the accept fails with zero writes.
    pub fn accept(&self, staging_id: &str, edited_name: &str) -> DomainResult<()> {
        let root = self.overlays_dir.get();
        if root.as_os_str().is_empty() {
            return Err(DomainError::StagedOverlayMissing);
        }
        // Rebuilt per call from the LIVE overlays dir (the constructor must
        // not freeze a writer): a config change since generation is honored.
        let writer = OverlayWriter::new(root.clone());
        let staged = staging_path(&root, staging_id)?;

        let manifest = fs::read_to_string(staged.join("overlay.json"))
            .map_err(|_| DomainError::StagedOverlayMissing)?;
        let value: Value =
            serde_json::from_str(&manifest).map_err(|_| DomainError::StagedOverlayMissing)?;
        let manifest_name = value["name"]
            .as_str()
            .filter(|n| !n.trim().is_empty())
            .ok_or(DomainError::StagedOverlayMissing)?;
        let index_html = fs::read_to_string(staged.join("index.html"))
            .map_err(|_| DomainError::StagedOverlayMissing)?;
        let style_css = fs::read_to_string(staged.join("style.css"))
            .map_err(|_| DomainError::StagedOverlayMissing)?;
        let script_js = fs::read_to_string(staged.join("script.js"))
            .map_err(|_| DomainError::StagedOverlayMissing)?;

        // (a) The script must pin EXACTLY ONE template id. Zero ids or
        // several mean the bundle is not trustworthy — fail closed.
        let pinned = pinned_template_ids(&script_js);
        if pinned.len() != 1 {
            return Err(DomainError::GenerationInvalidOutput {
                issues: vec![ValidationIssue::new("rewrite_template_id_absent")
                    .param("occurrences", pinned.len().to_string())],
            });
        }
        let old_directory = pinned[0].clone();
        validate_name(&old_directory)?;

        // (b) The manifest's name must normalize to the pinned id — accept
        // never guesses when the staged bundle disagrees with itself.
        if normalize_name(manifest_name) != old_directory {
            return Err(DomainError::GenerationInvalidOutput {
                issues: vec![
                    ValidationIssue::new("rewrite_name_mismatch").param("expected", old_directory)
                ],
            });
        }

        // Target identity: the user edit wins when present (revalidated
        // backend-side), otherwise the pinned id is kept.
        let edited_name = edited_name.trim();
        let (target_directory, target_manifest_name) = if edited_name.is_empty() {
            (old_directory.clone(), manifest_name.to_string())
        } else {
            let target_directory = normalize_name(edited_name);
            validate_name(&target_directory)?;
            (target_directory, edited_name.to_string())
        };

        if target_directory == old_directory {
            // No rename: the bundle is coherent, so promote as-is.
            return writer.accept(&staged, &old_directory);
        }

        // Rename: rewrite IN MEMORY FIRST — every check must pass before a
        // single byte is written, then the final bundle is re-validated
        // against the TARGET id (D3/D8/D11).
        let old_anchor = format!("TEMPLATE_ID = \"{old_directory}\"");
        let new_script = script_js.replace(
            &old_anchor,
            &format!("TEMPLATE_ID = \"{target_directory}\""),
        );
        let mut new_inner = value;
        new_inner["name"] = Value::String(target_manifest_name);
        let new_manifest = serde_json::to_string_pretty(&new_inner).map_err(|_| {
            DomainError::GenerationInvalidOutput {
                issues: vec![ValidationIssue::new("manifest_invalid_json")],
            }
        })?;
        let files = GeneratedFiles {
            overlay_json: new_manifest,
            index_html,
            style_css,
            script_js: new_script,
        };
        let report = validate_generated_overlay(&files, &target_directory);
        if !report.valid {
            return Err(DomainError::GenerationInvalidOutput {
                issues: report.issues,
            });
        }

        // All checks passed — persist the rewrite (still inside .staging),
        // then promote no-clobber.
        fs::write(staged.join("overlay.json"), &files.overlay_json).map_err(|e| {
            DomainError::TemplateWriteFailed {
                detail: format!("rewrite overlay.json: {e}"),
            }
        })?;
        fs::write(staged.join("script.js"), &files.script_js).map_err(|e| {
            DomainError::TemplateWriteFailed {
                detail: format!("rewrite script.js: {e}"),
            }
        })?;
        writer.accept(&staged, &target_directory)
    }

    /// Remove a staged overlay (user rejected it or it expired).
    pub fn discard(&self, staging_id: &str) -> DomainResult<()> {
        let root = self.overlays_dir.get();
        if root.as_os_str().is_empty() {
            return Err(DomainError::StagedOverlayMissing);
        }
        let writer = OverlayWriter::new(root.clone());
        let staged = staging_path(&root, staging_id)?;
        writer.discard(&staged)
    }
}

/// Sweep-guarded staging path (D4): only a uuid4-shaped id maps to a path
/// (anything else is a missing staged overlay, full stop).
fn staging_path(root: &Path, staging_id: &str) -> DomainResult<PathBuf> {
    let id = Uuid::parse_str(staging_id).map_err(|_| DomainError::StagedOverlayMissing)?;
    if id.get_version() != Some(Version::Random) {
        return Err(DomainError::StagedOverlayMissing);
    }
    Ok(OverlayWriter::new(root.to_path_buf())
        .staging_root()
        .join(id.to_string()))
}

/// Extract every template id pinned by `TEMPLATE_ID = "<id>"` assignments
/// (the trailing `;` is optional — the validator only requires containment).
/// The ACCEPT rewrite requires EXACTLY ONE pinned id.
fn pinned_template_ids(script: &str) -> Vec<String> {
    let marker = "TEMPLATE_ID = \"";
    let mut ids = Vec::new();
    let mut rest = script;
    while let Some(start) = rest.find(marker) {
        let after = &rest[start + marker.len()..];
        match after.find('"') {
            Some(end) => {
                ids.push(after[..end].to_string());
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    ids
}

/// Model-facing feedback block appended to the retry prompt. Issue codes are
/// stable identifiers; params are sorted so the text is deterministic.
fn feedback_block(issues: &[ValidationIssue]) -> String {
    if issues.is_empty() {
        return "The previous response was truncated. Respond again with the complete JSON only — no placeholder text.".into();
    }
    let mut out = String::from(
        "The previous response was rejected. Fix every point below and respond again with a single complete JSON object, no markdown fences:\n",
    );
    for (i, issue) in issues.iter().enumerate() {
        out.push_str(&format!("{}. {}\n", i + 1, issue.code));
        let mut params: Vec<(&str, &str)> = issue
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        params.sort_unstable();
        for (k, v) in params {
            out.push_str(&format!("   - {k}: {v}\n"));
        }
    }
    out
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
        issues: vec![ValidationIssue::new("no_json_object")],
    })?;
    let value: Value =
        serde_json::from_str(&json).map_err(|e| DomainError::GenerationInvalidOutput {
            issues: vec![ValidationIssue::new("invalid_json").param("error", e.to_string())],
        })?;
    let name = value["name"]
        .as_str()
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| DomainError::GenerationInvalidOutput {
            issues: vec![ValidationIssue::new("missing_name")],
        })?
        .to_string();
    let directory = normalize_name(&name);
    validate_name(&directory)?;

    let files = value["files"]
        .as_object()
        .ok_or_else(|| DomainError::GenerationInvalidOutput {
            issues: vec![ValidationIssue::new("missing_files")],
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
                issues: vec![ValidationIssue::new("missing_file").param("file", "overlay_json")],
            }
        })?,
        index_html: read_str("index_html").ok_or_else(|| DomainError::GenerationInvalidOutput {
            issues: vec![ValidationIssue::new("missing_file").param("file", "index_html")],
        })?,
        style_css: read_str("style_css").ok_or_else(|| DomainError::GenerationInvalidOutput {
            issues: vec![ValidationIssue::new("missing_file").param("file", "style_css")],
        })?,
        script_js: read_str("script_js").ok_or_else(|| DomainError::GenerationInvalidOutput {
            issues: vec![ValidationIssue::new("missing_file").param("file", "script_js")],
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
        replies: Vec<Result<AiText, AiError>>,
        next: std::sync::atomic::AtomicUsize,
        prompts: std::sync::Mutex<Vec<String>>,
        model: String,
    }

    impl StubProvider {
        fn new(replies: Vec<Result<AiText, AiError>>, model: &str) -> Self {
            assert!(!replies.is_empty(), "stub needs at least one reply");
            Self {
                replies,
                next: std::sync::atomic::AtomicUsize::new(0),
                prompts: std::sync::Mutex::new(Vec::new()),
                model: model.into(),
            }
        }

        fn call_count(&self) -> usize {
            self.prompts.lock().unwrap().len()
        }

        fn prompts(&self) -> Vec<String> {
            self.prompts.lock().unwrap().clone()
        }
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
            prompt: &str,
            _system: &str,
            _key: &str,
        ) -> Result<AiText, AiError> {
            self.prompts.lock().unwrap().push(prompt.to_string());
            let i = self.next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let idx = i.min(self.replies.len() - 1);
            self.replies[idx].clone()
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
    // shared counter in test_utils makes every tag unique too. The service
    // is given a literal system prompt — the application layer must not
    // depend on infrastructure prompt constants (layering, item 22).
    fn service_with(
        tag: &str,
        replies: Vec<Result<AiText, AiError>>,
    ) -> (GenerationService, OverlaysDirHandle, Arc<StubProvider>) {
        let keystore = Arc::new(MemoryKeyStore::new());
        keystore.set("anthropic", "sk-test").unwrap();
        let keys = Arc::new(KeyService::new(Arc::new(MemoryConfigRepo::new()), keystore));
        let stub = Arc::new(StubProvider::new(replies, "claude-test"));
        let provider: Arc<dyn AiProvider> = stub.clone();
        let dir = unique_temp_dir(&format!("gen-{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let handle = OverlaysDirHandle::new(dir.clone());
        (
            GenerationService::new(
                keys,
                provider,
                "You are a helpful overlay generator.".into(),
                handle.clone(),
            ),
            handle,
            stub,
        )
    }

    fn cleanup(dir: &OverlaysDirHandle) {
        let _ = fs::remove_dir_all(dir.get());
    }

    #[test]
    fn generates_and_stages_a_valid_overlay() {
        let (service, dir, _stub) = service_with(
            "valid",
            vec![Ok(AiText {
                text: valid_ai_json("Mi Overlay"),
                truncated: false,
            })],
        );

        let summary = service
            .generate(
                "anthropic",
                "claude-test",
                "Mi Overlay",
                "zócalo inferior minimalista",
            )
            .unwrap();
        assert_eq!(summary.directory, "mi-overlay");
        assert_eq!(summary.name, "Mi Overlay");
        assert_eq!(summary.fields.len(), 1);
        assert_eq!(summary.fields[0].key, "titulo");
        // The four file contents cross IPC for the preview panel.
        assert!(summary
            .files
            .script_js
            .contains("const TEMPLATE_ID = \"mi-overlay\";"));
        assert!(summary.files.overlay_json.contains("\"titulo\""));
        assert!(dir
            .get()
            .join(".staging")
            .join(&summary.staging_id)
            .is_dir());
        cleanup(&dir);
    }

    #[test]
    fn rejects_empty_prompt_and_never_calls_the_provider() {
        let (service, dir, stub) = service_with(
            "empty",
            vec![Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            })],
        );
        assert!(matches!(
            service.generate("anthropic", "claude-test", "X", "   "),
            Err(DomainError::GenerationEmptyPrompt)
        ));
        assert_eq!(stub.call_count(), 0, "provider must not be called");
        cleanup(&dir);
    }

    #[test]
    fn rejects_missing_overlays_dir() {
        let (service, dir, _stub) = service_with(
            "nodir",
            vec![Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            })],
        );
        dir.set(PathBuf::new());
        assert!(matches!(
            service.generate("anthropic", "claude-test", "X", "algo"),
            Err(DomainError::OverlaysDirMissing)
        ));
        cleanup(&dir);
    }

    #[test]
    fn surfaces_keyring_failure_when_no_key() {
        let keystore = Arc::new(MemoryKeyStore::new());
        let keys = Arc::new(KeyService::new(Arc::new(MemoryConfigRepo::new()), keystore));
        let provider: Arc<dyn AiProvider> = Arc::new(StubProvider::new(
            vec![Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            })],
            "m",
        ));
        let dir = unique_temp_dir("gen-nokey");
        fs::create_dir_all(&dir).unwrap();
        let service = GenerationService::new(
            keys,
            provider,
            "You are a helpful overlay generator.".into(),
            OverlaysDirHandle::new(dir.clone()),
        );

        assert!(matches!(
            service.generate("anthropic", "m", "X", "algo"),
            Err(DomainError::KeyringEntryMissing { .. })
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn truncated_response_never_stages() {
        let (service, dir, _stub) = service_with(
            "trunc",
            vec![Ok(AiText {
                text: valid_ai_json("Mi Overlay"),
                truncated: true,
            })],
        );
        assert!(matches!(
            service.generate("anthropic", "claude-test", "X", "algo"),
            Err(DomainError::GenerationInvalidOutput { ref issues })
                if issues.iter().any(|i| i.code == "truncated")
        ));
        assert!(!dir.get().join(".staging").exists());
        cleanup(&dir);
    }

    #[test]
    fn invalid_json_is_rejected() {
        let (service, dir, _stub) = service_with(
            "badjson",
            vec![Ok(AiText {
                text: "esto no es json".into(),
                truncated: false,
            })],
        );
        assert!(matches!(
            service.generate("anthropic", "claude-test", "X", "algo"),
            Err(DomainError::GenerationInvalidOutput { .. })
        ));
        cleanup(&dir);
    }

    #[test]
    fn contract_violations_are_collected() {
        // style.css is opaque → fails validation.
        let (service, dir, _stub) = service_with("contract", vec![Ok(AiText {
            text: r#"{"name":"X","files":{"overlay_json":"{\"name\":\"X\",\"fields\":[]}","index_html":"<b></b>","style_css":"body{background:#000}","script_js":"const TEMPLATE_ID=\"x\";function show(f){}function update(f){}function hide(){}"}}"#.into(),
            truncated: false,
        })]);
        assert!(matches!(
            service.generate("anthropic", "claude-test", "X", "algo"),
            Err(DomainError::GenerationInvalidOutput { ref issues })
                if issues.iter().any(|i| i.code == "style_not_transparent")
        ));
        cleanup(&dir);
    }

    #[test]
    fn provider_errors_map_to_domain_errors_without_retry() {
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
            let (service, dir, stub) = service_with("perr", vec![Err(ai)]);
            assert!(
                matches!(service.generate("anthropic", "claude-test", "X", "algo"), Err(e) if std::mem::discriminant(&e) == std::mem::discriminant(&domain))
            );
            assert_eq!(
                stub.call_count(),
                1,
                "transport errors are surfaced, never retried"
            );
            cleanup(&dir);
        }
    }

    #[test]
    fn retries_exactly_once_with_feedback_then_succeeds() {
        let (service, dir, stub) = service_with(
            "retry-ok",
            vec![
                // First attempt: unparseable → no_json_object issue.
                Ok(AiText {
                    text: "not json".into(),
                    truncated: false,
                }),
                // Second attempt (retry): valid → staged.
                Ok(AiText {
                    text: valid_ai_json("Mi Overlay"),
                    truncated: false,
                }),
            ],
        );

        let summary = service
            .generate("anthropic", "claude-test", "Mi Overlay", "un contador")
            .unwrap();
        assert_eq!(summary.directory, "mi-overlay");
        assert_eq!(stub.call_count(), 2, "exactly one retry");
        let prompts = stub.prompts();
        assert!(
            prompts[1].contains("no_json_object"),
            "second prompt must embed the collected issue codes, got: {}",
            prompts[1]
        );
        assert!(prompts[1].contains("The previous response was rejected"));
        assert!(
            prompts[0].ends_with("un contador"),
            "user prompt must be preserved in the first call"
        );
        cleanup(&dir);
    }

    #[test]
    fn retries_once_when_response_is_truncated() {
        let (service, dir, stub) = service_with(
            "retry-trunc",
            vec![
                Ok(AiText {
                    text: valid_ai_json("X"),
                    truncated: true,
                }),
                Ok(AiText {
                    text: valid_ai_json("X"),
                    truncated: false,
                }),
            ],
        );

        service
            .generate("anthropic", "claude-test", "X", "algo")
            .unwrap();
        assert_eq!(stub.call_count(), 2);
        assert!(stub.prompts()[1].contains("truncated"));
        cleanup(&dir);
    }

    #[test]
    fn no_retry_when_first_attempt_is_valid() {
        let (service, dir, stub) = service_with(
            "retry-none",
            vec![Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            })],
        );

        service
            .generate("anthropic", "claude-test", "X", "algo")
            .unwrap();
        assert_eq!(stub.call_count(), 1, "valid output must not be retried");
        cleanup(&dir);
    }

    #[test]
    fn retry_does_not_loop_on_repeated_failure() {
        let (service, dir, stub) = service_with(
            "retry-loop",
            vec![
                // First attempt: unparseable.
                Ok(AiText {
                    text: "not json".into(),
                    truncated: false,
                }),
                // Second attempt still violates the contract.
                Ok(AiText {
                    text: r#"{"name":"X","files":{"overlay_json":"{\"name\":\"X\",\"fields\":[]}","index_html":"<b></b>","style_css":"body{background:#000}","script_js":"const TEMPLATE_ID=\"x\";function show(f){}function update(f){}function hide(){}"}}"#.into(),
                    truncated: false,
                }),
            ],
        );

        let err = service
            .generate("anthropic", "claude-test", "X", "algo")
            .unwrap_err();
        assert!(
            matches!(&err, DomainError::GenerationInvalidOutput { ref issues }
                if issues.iter().any(|i| i.code == "style_not_transparent")),
            "surfaced issues come from the second (final) attempt"
        );
        assert_eq!(
            stub.call_count(),
            2,
            "exactly one retry even when it also fails"
        );
        cleanup(&dir);
    }

    #[test]
    fn accept_derives_template_id_from_staged_manifest() {
        let (service, dir, _stub) = service_with(
            "accept",
            vec![Ok(AiText {
                text: valid_ai_json("Zócalo Deportes"),
                truncated: false,
            })],
        );
        let summary = service
            .generate(
                "anthropic",
                "claude-test",
                "Zócalo Deportes",
                "zócalo de deportes",
            )
            .unwrap();

        service.accept(&summary.staging_id, "").unwrap();
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
        let (service, dir, _stub) = service_with(
            "accept",
            vec![Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            })],
        );
        assert!(matches!(
            service.accept("../../etc/passwd", ""),
            Err(DomainError::StagedOverlayMissing)
        ));
        // A valid-format but v1 uuid (not random/v4) is also refused.
        assert!(matches!(
            service.accept("550e8400-e29b-11d4-a716-446655440000", ""),
            Err(DomainError::StagedOverlayMissing)
        ));
        cleanup(&dir);
    }

    #[test]
    fn discard_removes_staging_and_keeps_templates() {
        let (service, dir, _stub) = service_with(
            "acc-ref",
            vec![Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            })],
        );
        let summary = service
            .generate("anthropic", "claude-test", "X", "algo")
            .unwrap();

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

    #[test]
    fn accept_uses_live_overlays_dir_after_config_change() {
        // The writer must NOT be frozen at construction: a config change
        // between generate and accept has to be honored (item 9).
        let (service, dir, _stub) = service_with(
            "acc-live",
            vec![Ok(AiText {
                text: valid_ai_json("Mi Overlay"),
                truncated: false,
            })],
        );
        let summary = service
            .generate("anthropic", "claude-test", "X", "algo")
            .unwrap();

        let other = unique_temp_dir("gen-other");
        fs::create_dir_all(&other).unwrap();
        dir.set(other.clone());

        assert!(
            matches!(
                service.accept(&summary.staging_id, ""),
                Err(DomainError::StagedOverlayMissing)
            ),
            "staging from the old dir must not resolve under the new root"
        );
        cleanup(&dir);
        let _ = fs::remove_dir_all(&other);
    }

    #[test]
    fn accept_rewrites_script_and_manifest_when_name_is_edited() {
        let (service, dir, _stub) = service_with(
            "acc-rename",
            vec![Ok(AiText {
                text: valid_ai_json("Zócalo Deportes"),
                truncated: false,
            })],
        );
        let summary = service
            .generate("anthropic", "claude-test", "Zócalo Deportes", "algo")
            .unwrap();
        assert_eq!(summary.directory, "zocalo-deportes");

        service.accept(&summary.staging_id, "Zócalo Final").unwrap();

        let promoted = dir.get().join("zocalo-final");
        assert!(promoted.join("overlay.json").is_file(), "renamed dir");
        assert!(!dir.get().join("zocalo-deportes").exists());
        let script = fs::read_to_string(promoted.join("script.js")).unwrap();
        assert!(script.contains("TEMPLATE_ID = \"zocalo-final\";"));
        assert!(!script.contains("zocalo-deportes"));
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(promoted.join("overlay.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["name"], "Zócalo Final");
        cleanup(&dir);
    }

    #[test]
    fn accept_fails_closed_when_template_id_is_absent() {
        // Tamper the staged script so the id appears ZERO times — the rename
        // must fail with zero writes: no template dir, staging unchanged.
        let (service, dir, _stub) = service_with(
            "acc-no-id",
            vec![Ok(AiText {
                text: valid_ai_json("Mi Overlay"),
                truncated: false,
            })],
        );
        let summary = service
            .generate("anthropic", "claude-test", "Mi Overlay", "algo")
            .unwrap();
        let staged_dir = dir.get().join(".staging").join(&summary.staging_id);
        let script = fs::read_to_string(staged_dir.join("script.js")).unwrap();
        fs::write(
            staged_dir.join("script.js"),
            script.replace("TEMPLATE_ID = \"mi-overlay\";", "const X = 1;"),
        )
        .unwrap();

        let err = service
            .accept(&summary.staging_id, "Otro Nombre")
            .unwrap_err();
        assert!(
            matches!(&err, DomainError::GenerationInvalidOutput { ref issues }
                if issues.iter().any(|i| i.code == "rewrite_template_id_absent")),
            "got {err:?}"
        );
        assert!(!dir.get().join("otro-nombre").exists(), "zero writes");
        assert!(!dir.get().join("mi-overlay").exists(), "zero writes");
        // Staging still holds the ORIGINAL script (nothing rewritten).
        let staged_script = fs::read_to_string(staged_dir.join("script.js")).unwrap();
        assert!(staged_script.contains("const X = 1;"));
        cleanup(&dir);
    }

    #[test]
    fn accept_fails_closed_on_inner_manifest_name_mismatch() {
        // The inner overlay.json "name" disagrees with the staged name — a
        // rename must not guess; it fails closed with zero writes.
        let (service, dir, _stub) = service_with(
            "acc-mismatch",
            vec![Ok(AiText {
                text: valid_ai_json("Mi Overlay"),
                truncated: false,
            })],
        );
        let summary = service
            .generate("anthropic", "claude-test", "Mi Overlay", "algo")
            .unwrap();
        let staged_dir = dir.get().join(".staging").join(&summary.staging_id);
        let mut inner: Value =
            serde_json::from_str(&fs::read_to_string(staged_dir.join("overlay.json")).unwrap())
                .unwrap();
        inner["name"] = Value::String("Otro Inner".into());
        fs::write(
            staged_dir.join("overlay.json"),
            serde_json::to_string(&inner).unwrap(),
        )
        .unwrap();

        let err = service
            .accept(&summary.staging_id, "Renombrado")
            .unwrap_err();
        assert!(
            matches!(&err, DomainError::GenerationInvalidOutput { ref issues }
                if issues.iter().any(|i| i.code == "rewrite_name_mismatch"))
        );
        assert!(!dir.get().join("renombrado").exists(), "zero writes");
        assert!(!dir.get().join("mi-overlay").exists(), "zero writes");
        cleanup(&dir);
    }

    #[test]
    fn accept_edit_must_still_validate_end_to_end() {
        // Renaming to an id that breaks the script contract is caught by the
        // post-rewrite re-validation with zero writes.
        let (service, dir, _stub) = service_with(
            "acc-reval",
            vec![Ok(AiText {
                text: valid_ai_json("Mi Overlay"),
                truncated: false,
            })],
        );
        // Generated script's TEMPLATE_ID is "mi-overlay" — the rewrite keeps
        // it consistent, so this asserts the happy path stays valid under
        // the FULL validator after a name edit (script + manifest).
        let summary = service
            .generate("anthropic", "claude-test", "Mi Overlay", "algo")
            .unwrap();
        service.accept(&summary.staging_id, "Zócalo Final").unwrap();
        let promoted = dir.get().join("zocalo-final");
        assert!(promoted.join("script.js").is_file());
        // Only a placeholder id was used in the stubbed payload: verify the
        // promoted bundle passes the same validator used at generate time.
        let files = GeneratedFiles {
            overlay_json: fs::read_to_string(promoted.join("overlay.json")).unwrap(),
            index_html: fs::read_to_string(promoted.join("index.html")).unwrap(),
            style_css: fs::read_to_string(promoted.join("style.css")).unwrap(),
            script_js: fs::read_to_string(promoted.join("script.js")).unwrap(),
        };
        let report = validate_generated_overlay(&files, "zocalo-final");
        assert!(report.valid, "promoted bundle must pass validation");
        cleanup(&dir);
    }

    #[test]
    fn generate_rejects_unknown_provider_without_calling_it() {
        let (service, dir, stub) = service_with(
            "gen-prov",
            vec![Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            })],
        );
        assert!(matches!(
            service.generate("openai", "claude-test", "X", "algo"),
            Err(DomainError::UnknownProvider)
        ));
        assert_eq!(
            stub.call_count(),
            0,
            "unknown provider fails before the AI call"
        );
        cleanup(&dir);
    }

    #[test]
    fn generate_rejects_unknown_model_without_calling_it() {
        let (service, dir, stub) = service_with(
            "gen-model",
            vec![Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            })],
        );
        assert!(matches!(
            service.generate("anthropic", "gpt-4", "X", "algo"),
            Err(DomainError::UnknownModel)
        ));
        assert_eq!(
            stub.call_count(),
            0,
            "unknown model fails before the AI call"
        );
        cleanup(&dir);
    }

    #[test]
    fn generate_revalidates_the_requested_name() {
        let (service, dir, stub) = service_with(
            "gen-name",
            vec![Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            })],
        );
        // Empty name → invalid name (mandatory param, backend-revalidated).
        assert!(matches!(
            service.generate("anthropic", "claude-test", "   ", "algo"),
            Err(DomainError::InvalidName { .. })
        ));
        // A name that normalizes to nothing is not a valid id either.
        assert!(matches!(
            service.generate("anthropic", "claude-test", "!!!", "algo"),
            Err(DomainError::InvalidName { .. })
        ));
        assert_eq!(stub.call_count(), 0, "bad names never reach the AI call");
        cleanup(&dir);
    }

    #[test]
    fn generate_embeds_the_requested_name_in_the_prompt() {
        let (service, dir, stub) = service_with(
            "gen-nameprompt",
            vec![Ok(AiText {
                text: valid_ai_json("Mi Contador"),
                truncated: false,
            })],
        );
        service
            .generate("anthropic", "claude-test", "Mi Contador", "algo")
            .unwrap();
        assert!(
            stub.prompts()[0].contains("Mi Contador"),
            "requested name must reach the model, got: {}",
            stub.prompts()[0]
        );
        cleanup(&dir);
    }

    #[test]
    fn models_are_listed_only_for_supported_providers() {
        let (service, dir, _stub) = service_with(
            "gen-models",
            vec![Ok(AiText {
                text: valid_ai_json("X"),
                truncated: false,
            })],
        );
        assert_eq!(service.models("anthropic").unwrap(), vec!["claude-test"]);
        assert!(matches!(
            service.models("mistral"),
            Err(DomainError::UnknownProvider)
        ));
        cleanup(&dir);
    }
}
