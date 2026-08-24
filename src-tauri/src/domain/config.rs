use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::error::{DomainError, DomainResult};

pub const SUPPORTED_LANGUAGES: [&str; 2] = ["es", "en"];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub overlays_dir: Option<PathBuf>,
    #[serde(default)]
    pub language: Option<String>,
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
}
