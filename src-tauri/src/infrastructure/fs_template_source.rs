use std::path::{Component, Path, PathBuf};

use crate::application::ports::TemplateSource;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::template::{OverlayField, TemplateInfo};

#[derive(Debug, Clone, serde::Deserialize)]
struct OverlayMeta {
    id: Option<String>,
    name: String,
    fields: Vec<OverlayField>,
}

fn validate_field(field: &OverlayField, meta_path: &Path) -> bool {
    if field.field_type == "progress" {
        if let (Some(min), Some(max)) = (field.min, field.max) {
            if min < max {
                return true;
            }
        }
        eprintln!(
            "[overlays] skipping field {:?} in {:?}: progress fields require min and max with min < max",
            field.key, meta_path
        );
        return false;
    }

    if field.field_type == "boolean" {
        return match field.default.as_deref() {
            Some("true") | Some("false") | None => true,
            Some(other) => {
                eprintln!(
                    "[overlays] skipping field {:?} in {:?}: boolean fields require default \"true\" or \"false\", got {:?}",
                    field.key, meta_path, other
                );
                false
            }
        };
    }

    if field.field_type == "file" {
        if let Some(accept) = &field.accept {
            if accept.is_empty() || !accept.iter().all(|ext| is_valid_extension(ext)) {
                eprintln!(
                    "[overlays] skipping field {:?} in {:?}: file fields require non-empty extension filters without dots, got {:?}",
                    field.key, meta_path, accept
                );
                return false;
            }
        }
    }

    true
}

fn is_valid_extension(ext: &str) -> bool {
    let normalized = normalize_extension(ext);
    !normalized.is_empty() && normalized.chars().all(|c| c.is_ascii_alphanumeric())
}

fn normalize_extension(ext: &str) -> String {
    ext.trim().trim_start_matches('.').to_ascii_lowercase()
}

fn normalize_file_field(mut field: OverlayField) -> OverlayField {
    if field.field_type == "file" {
        if let Some(accept) = field.accept.take() {
            field.accept = Some(accept.iter().map(|e| normalize_extension(e)).collect());
        }
    }
    field
}

/// Resolves an absolute file path into a path relative to the template
/// folder, guaranteed to live inside the overlays directory.
///
/// The returned value is what the overlay receives as a field value: a
/// relative path (e.g. `logo.png` or `../assets/logo.png`) that the local
/// HTTP server can serve under `/overlay/*`.
pub fn resolve_asset_path(
    overlays_dir: &Path,
    template_id: &str,
    absolute_path: &str,
) -> Result<String, DomainError> {
    let canonical_dir =
        dunce::canonicalize(overlays_dir).map_err(|_| DomainError::AssetResolveFailed {
            path: absolute_path.to_string(),
        })?;
    let source = Path::new(absolute_path);
    let canonical_source =
        dunce::canonicalize(source).map_err(|_| DomainError::AssetResolveFailed {
            path: absolute_path.to_string(),
        })?;
    if !canonical_source.starts_with(&canonical_dir) {
        return Err(DomainError::AssetOutsideOverlaysDir {
            path: absolute_path.to_string(),
        });
    }

    let template_dir = dunce::canonicalize(canonical_dir.join(template_id)).map_err(|_| {
        DomainError::AssetResolveFailed {
            path: absolute_path.to_string(),
        }
    })?;

    let relative = relative_between(&template_dir, &canonical_source);
    Ok(path_to_web(relative))
}

fn relative_between(base: &Path, target: &Path) -> PathBuf {
    let base: Vec<PathBuf> = base
        .components()
        .map(|c| PathBuf::from(c.as_os_str()))
        .collect();
    let target: Vec<PathBuf> = target
        .components()
        .map(|c| PathBuf::from(c.as_os_str()))
        .collect();
    let common = base
        .iter()
        .zip(target.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut rel = PathBuf::new();
    for _ in common..base.len() {
        rel.push("..");
    }
    for comp in &target[common..] {
        rel.push(comp);
    }
    rel
}

fn path_to_web(path: PathBuf) -> String {
    path.components()
        .filter_map(|c| {
            if matches!(c, Component::Prefix(_) | Component::RootDir) {
                None
            } else {
                Some(c.as_os_str().to_string_lossy().into_owned())
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub struct FsTemplateSource;

impl TemplateSource for FsTemplateSource {
    fn discover(&self, overlay_dir: &Path) -> DomainResult<Vec<TemplateInfo>> {
        if !overlay_dir.is_dir() {
            return Ok(Vec::new());
        }

        let read_failed = || DomainError::TemplateDiscoveryFailed {
            detail: format!("could not read {overlay_dir:?}"),
        };

        let mut templates = Vec::new();
        let entries = std::fs::read_dir(overlay_dir).map_err(|_| read_failed())?;

        for entry in entries {
            let path = entry.map_err(|_| read_failed())?.path();
            if !path.is_dir() {
                continue;
            }

            let meta_path = path.join("overlay.json");
            if !meta_path.exists() {
                continue;
            }

            let html_path = path.join("index.html");
            if !html_path.exists() {
                eprintln!(
                    "[overlays] skipping {:?}: no index.html found",
                    path.file_name()
                );
                continue;
            }

            let content = match std::fs::read_to_string(&meta_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[overlays] could not read {:?}: {e}", meta_path);
                    continue;
                }
            };

            let meta: OverlayMeta = match serde_json::from_str(&content) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[overlays] could not parse {:?}: {e}", meta_path);
                    continue;
                }
            };

            let folder_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();

            let id = meta.id.unwrap_or_else(|| folder_name.clone());

            let fields = meta
                .fields
                .into_iter()
                .filter(|field| validate_field(field, &meta_path))
                .map(normalize_file_field)
                .collect();

            templates.push(TemplateInfo {
                id,
                name: meta.name,
                path: format!("{folder_name}/index.html"),
                fields,
            });
        }

        templates.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(templates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_template(dir: &Path, id: &str, fields: &str) {
        let tpl = dir.join(id);
        std::fs::create_dir_all(&tpl).unwrap();
        std::fs::write(tpl.join("index.html"), "<html><body></body></html>").unwrap();
        std::fs::write(
            tpl.join("overlay.json"),
            format!(r#"{{ "name": "T {id}", "fields": {fields} }}"#),
        )
        .unwrap();
    }

    #[test]
    fn progress_field_with_valid_min_max_is_kept() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-a",
            r#"[{ "key": "p", "label": "P", "type": "progress", "min": 0, "max": 100, "default": "50" }]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap();
        assert_eq!(templates.len(), 1);
        let field = &templates[0].fields[0];
        assert_eq!(field.field_type, "progress");
        assert_eq!(field.min, Some(0.0));
        assert_eq!(field.max, Some(100.0));
        assert_eq!(field.default.as_deref(), Some("50"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn progress_field_without_min_max_is_dropped() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-b",
            r#"[
                { "key": "bad", "label": "Bad", "type": "progress" },
                { "key": "inverted", "label": "Inv", "type": "progress", "min": 100, "max": 0 },
                { "key": "texto", "label": "T", "type": "text" }
            ]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap();
        assert_eq!(templates.len(), 1);
        let keys: Vec<&str> = templates[0].fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["texto"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn boolean_field_with_valid_default_is_kept() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-bool",
            r#"[{ "key": "b", "label": "B", "type": "boolean", "default": "true" }]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap();
        assert_eq!(templates.len(), 1);
        let field = &templates[0].fields[0];
        assert_eq!(field.field_type, "boolean");
        assert_eq!(field.default.as_deref(), Some("true"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn boolean_field_without_default_is_kept() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-bool",
            r#"[{ "key": "b", "label": "B", "type": "boolean" }]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap();
        assert_eq!(templates.len(), 1);
        let field = &templates[0].fields[0];
        assert_eq!(field.default, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn boolean_field_with_invalid_default_is_dropped() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-bool",
            r#"[
                { "key": "bad", "label": "Bad", "type": "boolean", "default": "yes" },
                { "key": "texto", "label": "T", "type": "text" }
            ]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap();
        assert_eq!(templates.len(), 1);
        let keys: Vec<&str> = templates[0].fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["texto"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_field_with_valid_accept_is_kept_and_normalized() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-file",
            r#"[{ "key": "logo", "label": "Logo", "type": "file", "accept": [".PNG", "jpg", " svg "] }]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap();
        assert_eq!(templates.len(), 1);
        let field = &templates[0].fields[0];
        assert_eq!(field.field_type, "file");
        let expected: Vec<String> = ["png", "jpg", "svg"].map(String::from).to_vec();
        assert_eq!(field.accept.as_deref(), Some(expected.as_slice()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_field_without_accept_is_kept() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-file",
            r#"[{ "key": "logo", "label": "Logo", "type": "file" }]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap();
        assert_eq!(templates.len(), 1);
        let field = &templates[0].fields[0];
        assert_eq!(field.field_type, "file");
        assert_eq!(field.accept, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_field_with_invalid_accept_is_dropped() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-file",
            r#"[
                { "key": "bad", "label": "Bad", "type": "file", "accept": ["exe../"] },
                { "key": "no-filter", "label": "No filter", "type": "file", "accept": [] },
                { "key": "texto", "label": "T", "type": "text" }
            ]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap();
        assert_eq!(templates.len(), 1);
        let keys: Vec<&str> = templates[0].fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["texto"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_asset_path_returns_relative_path_inside_template_folder() {
        let dir = temp_dir("overlays-resolve-test");
        let tpl = dir.join("tpl-a");
        std::fs::create_dir_all(&tpl).unwrap();
        let asset = tpl.join("logo.png");
        std::fs::write(&asset, b"png").unwrap();

        let resolved = resolve_asset_path(&dir, "tpl-a", asset.to_str().unwrap()).unwrap();
        assert_eq!(resolved, "logo.png");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_asset_path_accepts_files_elsewhere_inside_overlays_dir() {
        let dir = temp_dir("overlays-resolve-test");
        let tpl = dir.join("tpl-a");
        std::fs::create_dir_all(&tpl).unwrap();
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        let asset = dir.join("assets").join("logo.png");
        std::fs::write(&asset, b"png").unwrap();

        let resolved = resolve_asset_path(&dir, "tpl-a", asset.to_str().unwrap()).unwrap();
        assert_eq!(resolved, "../assets/logo.png");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_asset_path_rejects_files_outside_overlays_dir() {
        let dir = temp_dir("overlays-resolve-test");
        let tpl = dir.join("tpl-a");
        std::fs::create_dir_all(&tpl).unwrap();
        let outside = std::env::temp_dir().join("overlays-outside-test-file.png");
        std::fs::write(&outside, b"png").unwrap();

        let error = resolve_asset_path(&dir, "tpl-a", outside.to_str().unwrap()).unwrap_err();
        assert!(matches!(error, DomainError::AssetOutsideOverlaysDir { .. }));

        let _ = std::fs::remove_file(&outside);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
