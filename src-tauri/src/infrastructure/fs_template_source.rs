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

        let read_failed = || {
            DomainError::TemplateDiscoveryFailed {
                detail: format!("could not read {overlay_dir:?}"),
            }
        };

        let mut templates = Vec::new();
        let entries =
            std::fs::read_dir(overlay_dir).map_err(|_| read_failed())?;

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
