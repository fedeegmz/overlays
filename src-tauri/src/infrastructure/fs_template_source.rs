use std::path::Path;

use crate::application::ports::TemplateSource;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::template::{OverlayField, TemplateInfo};

#[derive(Debug, Clone, serde::Deserialize)]
struct OverlayMeta {
    id: Option<String>,
    name: String,
    fields: Vec<OverlayField>,
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
                .filter(|field| {
                    if field.field_type != "progress" {
                        return true;
                    }
                    match (field.min, field.max) {
                        (Some(min), Some(max)) if min < max => true,
                        _ => {
                            eprintln!(
                                "[overlays] skipping field {:?} in {:?}: progress fields require min and max with min < max",
                                field.key, meta_path
                            );
                            false
                        }
                    }
                })
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
}
