use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::ports::ConfigRepository;
use super::template_catalog::OverlaysDirHandle;
use super::upload_service::UploadSettingsHandle;
use crate::domain::config::AppConfig;
use crate::domain::error::{DomainError, DomainResult};

pub struct ConfigService {
    repo: Arc<dyn ConfigRepository>,
    overlays_dir: OverlaysDirHandle,
    upload: UploadSettingsHandle,
}

impl ConfigService {
    pub fn new(
        repo: Arc<dyn ConfigRepository>,
        overlays_dir: OverlaysDirHandle,
        upload: UploadSettingsHandle,
    ) -> Self {
        Self {
            repo,
            overlays_dir,
            upload,
        }
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

    pub fn set_uploads_enabled(&self, enabled: bool) -> DomainResult<AppConfig> {
        let mut config = self.repo.load();
        config.uploads_enabled = enabled;
        self.repo.save(&config)?;
        self.upload.set_enabled(enabled);
        Ok(config)
    }

    /// Stores the destination template verbatim (variables are expanded per upload).
    /// Blank input clears it, so the default Downloads folder applies.
    pub fn set_upload_dir(&self, template: Option<String>) -> DomainResult<AppConfig> {
        let template = template
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        if let Some(t) = &template {
            validate_upload_dir(t)?;
        }

        let mut config = self.repo.load();
        config.upload_dir = template.clone();
        self.repo.save(&config)?;
        self.upload.set_dir_template(template);
        Ok(config)
    }
}

fn validate_upload_dir(template: &str) -> DomainResult<()> {
    let has_traversal = template
        .split(['/', '\\'])
        .any(|component| component == "..");
    if template.contains('\0') || has_traversal || !Path::new(template).is_absolute() {
        return Err(DomainError::UploadDirInvalid {
            path: template.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::upload_service::UploadSettings;
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

    fn service_with_upload() -> (ConfigService, OverlaysDirHandle, UploadSettingsHandle) {
        let dir = OverlaysDirHandle::default();
        let upload = UploadSettingsHandle::new(UploadSettings {
            enabled: true,
            dir_template: None,
        });
        let svc = ConfigService::new(
            Arc::new(InMemoryRepo(Mutex::new(AppConfig::default()))),
            dir.clone(),
            upload.clone(),
        );
        (svc, dir, upload)
    }

    fn service() -> (ConfigService, OverlaysDirHandle) {
        let (svc, dir, _) = service_with_upload();
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

    #[test]
    fn set_uploads_enabled_persists_and_updates_handle() {
        let (svc, _, upload) = service_with_upload();

        let config = svc.set_uploads_enabled(false).unwrap();

        assert!(!config.uploads_enabled);
        assert!(!svc.get().uploads_enabled);
        assert!(!upload.is_enabled());

        svc.set_uploads_enabled(true).unwrap();
        assert!(upload.is_enabled());
    }

    #[test]
    fn set_upload_dir_stores_template_verbatim() {
        let (svc, _, upload) = service_with_upload();

        let config = svc.set_upload_dir(Some("/data/%CCYY/%MM".into())).unwrap();

        assert_eq!(config.upload_dir.as_deref(), Some("/data/%CCYY/%MM"));
        assert_eq!(svc.get().upload_dir.as_deref(), Some("/data/%CCYY/%MM"));
        assert_eq!(
            upload.get().dir_template.as_deref(),
            Some("/data/%CCYY/%MM")
        );
    }

    #[test]
    fn set_upload_dir_accepts_unknown_sequences() {
        let (svc, _, _) = service_with_upload();
        let config = svc.set_upload_dir(Some("/mnt/%Q".into())).unwrap();
        assert_eq!(config.upload_dir.as_deref(), Some("/mnt/%Q"));
    }

    #[test]
    fn set_upload_dir_trims_surrounding_whitespace() {
        let (svc, _, _) = service_with_upload();
        let config = svc.set_upload_dir(Some("  /mnt/share  ".into())).unwrap();
        assert_eq!(config.upload_dir.as_deref(), Some("/mnt/share"));
    }

    #[test]
    fn set_upload_dir_clears_on_blank_or_none() {
        let (svc, _, upload) = service_with_upload();
        svc.set_upload_dir(Some("/mnt/share".into())).unwrap();

        let config = svc.set_upload_dir(Some("   ".into())).unwrap();
        assert!(config.upload_dir.is_none());
        assert!(upload.get().dir_template.is_none());

        svc.set_upload_dir(Some("/mnt/share".into())).unwrap();
        let config = svc.set_upload_dir(None).unwrap();
        assert!(config.upload_dir.is_none());
    }

    #[test]
    fn set_upload_dir_rejects_invalid_and_keeps_previous_value() {
        let (svc, _, upload) = service_with_upload();
        svc.set_upload_dir(Some("/mnt/share".into())).unwrap();

        for bad in ["uploads/%DD", "/data/../etc", "/data/a\0b"] {
            let err = svc.set_upload_dir(Some(bad.into())).unwrap_err();
            assert!(matches!(err, DomainError::UploadDirInvalid { .. }));
        }

        assert_eq!(svc.get().upload_dir.as_deref(), Some("/mnt/share"));
        assert_eq!(upload.get().dir_template.as_deref(), Some("/mnt/share"));
    }

    #[test]
    fn upload_setters_preserve_other_settings() {
        let (svc, _, _) = service_with_upload();
        svc.set_language("en".into()).unwrap();
        let tmp = std::env::temp_dir();
        svc.set_overlays_dir(tmp.to_string_lossy().into_owned())
            .unwrap();

        svc.set_uploads_enabled(false).unwrap();
        let config = svc.set_upload_dir(Some("/mnt/share".into())).unwrap();

        assert_eq!(config.language.as_deref(), Some("en"));
        assert_eq!(config.overlays_dir, Some(tmp));

        let config = svc.set_language("es".into()).unwrap();
        assert!(!config.uploads_enabled);
        assert_eq!(config.upload_dir.as_deref(), Some("/mnt/share"));
    }
}
