use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::ai::ApiKeyPresence;
use super::error::{DomainError, DomainResult};

pub const SUPPORTED_LANGUAGES: [&str; 2] = ["es", "en"];

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub overlays_dir: Option<PathBuf>,
    #[serde(default)]
    pub language: Option<String>,
    // Key presence metadata ONLY — secrets never touch config.json (K1).
    #[serde(default)]
    pub provider_keys: Vec<ApiKeyPresence>,
    // Feature flag: legacy/missing config must keep the generator enabled (D11).
    #[serde(default = "default_true")]
    pub ai_generator_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            overlays_dir: None,
            language: None,
            provider_keys: Vec::new(),
            ai_generator_enabled: true,
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
    fn legacy_config_without_ai_flag_loads_default() {
        let config: AppConfig =
            serde_json::from_str(r#"{ "overlays_dir": "/home/user/overlays", "language": "es" }"#)
                .unwrap();
        assert_eq!(
            config.overlays_dir,
            Some(PathBuf::from("/home/user/overlays"))
        );
        assert_eq!(config.language.as_deref(), Some("es"));
        assert!(config.ai_generator_enabled);
        assert!(config.provider_keys.is_empty());
    }

    #[test]
    fn provider_keys_and_flag_round_trip() {
        let config = AppConfig {
            overlays_dir: None,
            language: None,
            provider_keys: vec![ApiKeyPresence {
                provider: "anthropic".into(),
                configured: true,
                last4: Some("1234".into()),
            }],
            ai_generator_enabled: false,
        };
        let json = serde_json::to_value(&config).unwrap();
        let back: AppConfig = serde_json::from_value(json).unwrap();
        assert_eq!(back.provider_keys.len(), 1);
        assert_eq!(back.provider_keys[0].provider, "anthropic");
        assert!(!back.ai_generator_enabled);
    }

    #[test]
    fn default_config_has_ai_enabled() {
        assert!(AppConfig::default().ai_generator_enabled);
        assert!(AppConfig::default().provider_keys.is_empty());
    }
}
