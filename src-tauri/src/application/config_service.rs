use std::path::PathBuf;
use std::sync::Arc;

use super::ports::ConfigRepository;
use super::template_catalog::OverlaysDirHandle;
use crate::domain::config::AppConfig;
use crate::domain::error::{DomainError, DomainResult};

pub struct ConfigService {
    repo: Arc<dyn ConfigRepository>,
    overlays_dir: OverlaysDirHandle,
}

impl ConfigService {
    pub fn new(repo: Arc<dyn ConfigRepository>, overlays_dir: OverlaysDirHandle) -> Self {
        Self { repo, overlays_dir }
    }

    pub fn get(&self) -> AppConfig {
        self.repo.load()
    }

    pub fn set_language(&self, lang: String) -> DomainResult<AppConfig> {
        let mut config = self.repo.load();
        config.set_language(&lang)?;
        self.repo.save(&config)?;
        Ok(config)
    }

    pub fn set_overlays_dir(&self, path: String) -> DomainResult<AppConfig> {
        let dir = PathBuf::from(&path);
        if !dir.is_dir() {
            return Err(DomainError::OverlaysDirInvalid { path });
        }

        let mut config = self.repo.load();
        config.overlays_dir = Some(dir.clone());
        self.repo.save(&config)?;
        self.overlays_dir.set(dir);
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct InMemoryRepo(Mutex<AppConfig>);

    impl ConfigRepository for InMemoryRepo {
        fn load(&self) -> AppConfig {
            self.0.lock().unwrap().clone()
        }

        fn save(&self, config: &AppConfig) -> DomainResult<()> {
            *self.0.lock().unwrap() = config.clone();
            Ok(())
        }
    }

    fn service() -> (ConfigService, OverlaysDirHandle) {
        let dir = OverlaysDirHandle::default();
        let svc = ConfigService::new(Arc::new(InMemoryRepo(Mutex::new(AppConfig::default()))), dir.clone());
        (svc, dir)
    }

    #[test]
    fn set_language_persists() {
        let (svc, _) = service();
        let config = svc.set_language("en".into()).unwrap();
        assert_eq!(config.language.as_deref(), Some("en"));
        assert_eq!(svc.get().language.as_deref(), Some("en"));
    }

    #[test]
    fn set_language_rejects_unsupported() {
        let (svc, _) = service();
        assert!(svc.set_language("fr".into()).is_err());
    }

    #[test]
    fn set_overlays_dir_rejects_missing_path() {
        let (svc, _) = service();
        assert!(matches!(
            svc.set_overlays_dir("/definitely/not/a/dir".into()),
            Err(DomainError::OverlaysDirInvalid { .. })
        ));
    }

    #[test]
    fn set_overlays_dir_updates_runtime_handle_and_config() {
        let tmp = std::env::temp_dir();
        let (svc, handle) = service();

        svc.set_overlays_dir(tmp.to_string_lossy().into_owned())
            .unwrap();

        assert_eq!(handle.get(), tmp);
        assert_eq!(svc.get().overlays_dir, Some(tmp));
    }
}
