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
        Ok(Manifest {
            templates: self.source.discover(&self.overlays_dir.get())?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::template::{OverlayField, TemplateInfo};
    use std::path::Path;

    struct FakeSource;

    impl TemplateSource for FakeSource {
        fn discover(&self, overlay_dir: &Path) -> DomainResult<Vec<TemplateInfo>> {
            if overlay_dir.as_os_str().is_empty() {
                return Ok(Vec::new());
            }
            Ok(vec![TemplateInfo {
                id: "fake".into(),
                name: "Fake".into(),
                path: "fake/index.html".into(),
                fields: vec![OverlayField {
                    key: "titulo".into(),
                    label: "Título".into(),
                    field_type: "text".into(),
                    default: None,
                }],
            }])
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
}
