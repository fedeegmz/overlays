use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::CommandError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub overlays_dir: Option<PathBuf>,
    #[serde(default)]
    pub language: Option<String>,
}

impl AppConfig {
    pub fn set_language(&mut self, lang: &str) {
        self.language = Some(lang.to_string());
    }
}

pub fn load_config(path: &Path) -> AppConfig {
    if !path.exists() {
        return AppConfig::default();
    }
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(path: &Path, config: &AppConfig) -> Result<(), CommandError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| {
            CommandError::new(CommandError::CONFIG_SAVE_FAILED)
                .param("detail", format!("could not create {dir:?}: {e}"))
        })?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| {
        CommandError::new(CommandError::CONFIG_SAVE_FAILED).param("detail", e.to_string())
    })?;
    std::fs::write(path, json).map_err(|e| {
        CommandError::new(CommandError::CONFIG_SAVE_FAILED)
            .param("detail", format!("could not write {path:?}: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> PathBuf {
        std::env::temp_dir().join(format!("overlays-test-config-{}.json", std::process::id()))
    }

    #[test]
    fn missing_file_loads_default() {
        let path = std::env::temp_dir().join("overlays-test-config-does-not-exist.json");
        let config = load_config(&path);
        assert!(config.overlays_dir.is_none());
        assert!(config.language.is_none());
    }

    #[test]
    fn round_trip_preserves_data() {
        let path = temp_path();
        let config = AppConfig {
            overlays_dir: Some(PathBuf::from("/home/user/overlays")),
            language: Some("es".to_string()),
        };

        save_config(&path, &config).unwrap();
        let loaded = load_config(&path);
        assert_eq!(
            loaded.overlays_dir,
            Some(PathBuf::from("/home/user/overlays"))
        );
        assert_eq!(loaded.language, Some("es".to_string()));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn legacy_config_without_language_loads_default() {
        let path = temp_path();
        std::fs::write(&path, r#"{ "overlays_dir": "/home/user/overlays" }"#).unwrap();
        let loaded = load_config(&path);
        assert!(loaded.language.is_none());
        assert_eq!(
            loaded.overlays_dir,
            Some(PathBuf::from("/home/user/overlays"))
        );

        let _ = std::fs::remove_file(&path);
    }
}
