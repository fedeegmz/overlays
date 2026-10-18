use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::error::{DomainError, DomainResult};

pub const SUPPORTED_LANGUAGES: [&str; 2] = ["es", "en"];

fn default_uploads_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub overlays_dir: Option<PathBuf>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default = "default_uploads_enabled")]
    pub uploads_enabled: bool,
    /// Destination template for LAN uploads; kept as a string so `%` variables survive.
    #[serde(default)]
    pub upload_dir: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            overlays_dir: None,
            language: None,
            uploads_enabled: default_uploads_enabled(),
            upload_dir: None,
        }
    }
}

impl AppConfig {
    pub fn set_language(&mut self, lang: &str) -> DomainResult<()> {
        if !SUPPORTED_LANGUAGES.contains(&lang) {
            return Err(DomainError::LanguageUnsupported {
                lang: lang.to_string(),
            });
        }
        self.language = Some(lang.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_language() {
        let mut config = AppConfig::default();
        config.set_language("es").unwrap();
        assert_eq!(config.language.as_deref(), Some("es"));
    }

    #[test]
    fn rejects_unsupported_language() {
        let mut config = AppConfig::default();
        assert!(matches!(
            config.set_language("fr"),
            Err(DomainError::LanguageUnsupported { .. })
        ));
    }

    #[test]
    fn legacy_config_without_language_loads_default() {
        let config: AppConfig =
            serde_json::from_str(r#"{ "overlays_dir": "/home/user/overlays" }"#).unwrap();
        assert!(config.language.is_none());
        assert_eq!(
            config.overlays_dir,
            Some(PathBuf::from("/home/user/overlays"))
        );
    }

    #[test]
    fn legacy_config_enables_uploads() {
        let config: AppConfig =
            serde_json::from_str(r#"{ "overlays_dir": "/home/user/overlays", "language": "es" }"#)
                .unwrap();
        assert!(config.uploads_enabled);
        assert!(config.upload_dir.is_none());
    }

    #[test]
    fn empty_config_enables_uploads() {
        let config: AppConfig = serde_json::from_str("{}").unwrap();
        assert!(config.uploads_enabled);
        assert!(AppConfig::default().uploads_enabled);
    }

    #[test]
    fn persisted_false_is_honored() {
        let config: AppConfig = serde_json::from_str(r#"{ "uploads_enabled": false }"#).unwrap();
        assert!(!config.uploads_enabled);
    }

    #[test]
    fn upload_dir_round_trips_verbatim() {
        let config: AppConfig =
            serde_json::from_str(r#"{ "upload_dir": "/data/%CCYY/%MM" }"#).unwrap();
        assert_eq!(config.upload_dir.as_deref(), Some("/data/%CCYY/%MM"));
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["upload_dir"], "/data/%CCYY/%MM");
        assert_eq!(json["uploads_enabled"], true);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let config: AppConfig =
            serde_json::from_str(r#"{ "future_field": 1, "uploads_enabled": true }"#).unwrap();
        assert!(config.uploads_enabled);
    }
}
