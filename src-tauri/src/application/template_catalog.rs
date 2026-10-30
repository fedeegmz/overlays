use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use super::ports::TemplateSource;
use crate::domain::error::DomainResult;
use crate::domain::template::Manifest;

#[derive(Clone, Default)]
pub struct OverlaysDirHandle(Arc<RwLock<PathBuf>>);

impl OverlaysDirHandle {
    pub fn new(dir: PathBuf) -> Self {
        Self(Arc::new(RwLock::new(dir)))
    }

    pub fn get(&self) -> PathBuf {
        self.0.read().unwrap().clone()
    }

    pub fn set(&self, dir: PathBuf) {
        *self.0.write().unwrap() = dir;
    }
}

pub struct TemplateCatalog {
    source: Arc<dyn TemplateSource>,
    overlays_dir: OverlaysDirHandle,
}

impl TemplateCatalog {
    pub fn new(source: Arc<dyn TemplateSource>, overlays_dir: OverlaysDirHandle) -> Self {
        Self {
            source,
            overlays_dir,
        }
    }

    pub fn manifest(&self) -> DomainResult<Manifest> {
        self.source.discover(&self.overlays_dir.get())
    }

    /// Directory of the template with this id, relative to the overlays dir.
    pub fn template_dir(&self, id: &str) -> DomainResult<Option<String>> {
        Ok(self
            .manifest()?
            .templates
            .into_iter()
            .find(|t| t.id == id)
            .map(|t| {
                t.path
                    .strip_suffix("/index.html")
                    .unwrap_or(&t.path)
                    .to_string()
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::template::{OverlayField, TemplateInfo};
    use std::path::Path;

    struct FakeSource;

    impl TemplateSource for FakeSource {
        fn discover(&self, overlay_dir: &Path) -> DomainResult<Manifest> {
            if overlay_dir.as_os_str().is_empty() {
                return Ok(Manifest {
                    templates: Vec::new(),
                    folders: Vec::new(),
                });
            }
            Ok(Manifest {
                folders: vec!["grp".into()],
                templates: vec![TemplateInfo {
                    id: "fake".into(),
                    name: "Fake".into(),
                    path: "fake/index.html".into(),
                    folder: String::new(),
                    fields: vec![OverlayField {
                        key: "titulo".into(),
                        label: "Título".into(),
                        field_type: "text".into(),
                        default: None,
                        min: None,
                        max: None,
                        step: None,
                        accept: None,
                    }],
                }],
            })
        }
    }

    #[test]
    fn manifest_reflects_current_dir() {
        let dir = OverlaysDirHandle::default();
        let catalog = TemplateCatalog::new(Arc::new(FakeSource), dir.clone());

        assert!(catalog.manifest().unwrap().templates.is_empty());

        dir.set(PathBuf::from("/some/overlays"));
        assert_eq!(catalog.manifest().unwrap().templates.len(), 1);
    }

    #[test]
    fn template_dir_strips_index_html() {
        let dir = OverlaysDirHandle::new(PathBuf::from("/some/overlays"));
        let catalog = TemplateCatalog::new(Arc::new(FakeSource), dir);

        assert_eq!(
            catalog.template_dir("fake").unwrap().as_deref(),
            Some("fake")
        );
    }

    #[test]
    fn template_dir_of_unknown_id_is_none() {
        let dir = OverlaysDirHandle::new(PathBuf::from("/some/overlays"));
        let catalog = TemplateCatalog::new(Arc::new(FakeSource), dir);

        assert_eq!(catalog.template_dir("nope").unwrap(), None);
    }
}
