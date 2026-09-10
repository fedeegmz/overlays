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
            // Skip hidden dirs (".staging", ".hidden", ...): the manifest must
            // never expose staged or internal directories (D12).
            let is_hidden = path
                .file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(false);
            if is_hidden {
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

            templates.push(TemplateInfo {
                id,
                name: meta.name,
                path: format!("{folder_name}/index.html"),
                fields: meta.fields,
            });
        }

        templates.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(templates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("overlays-fs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_template(root: &Path, dir: &str, name: &str) {
        let template_dir = root.join(dir);
        std::fs::create_dir_all(&template_dir).unwrap();
        std::fs::write(
            template_dir.join("overlay.json"),
            format!(r#"{{ "name": "{name}", "fields": [] }}"#),
        )
        .unwrap();
        std::fs::write(template_dir.join("index.html"), "<html></html>").unwrap();
    }

    #[test]
    fn discovers_non_hidden_template_dirs_only() {
        let root = temp_root("discover");
        write_template(&root, "lower-third", "Lower Third");
        write_template(&root, ".staging", "Staged");
        write_template(&root, ".hidden", "Hidden");

        let templates = FsTemplateSource.discover(&root).unwrap();

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].id, "lower-third");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_dir_yields_empty_manifest() {
        let root = temp_root("empty");
        assert!(FsTemplateSource.discover(&root).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
