use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub overlays_dir: Option<PathBuf>,
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

pub fn save_config(path: &Path, config: &AppConfig) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("could not create {dir:?}: {e}"))?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| format!("could not write {path:?}: {e}"))
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
    }

    #[test]
    fn round_trip_preserves_data() {
        let path = temp_path();
        let config = AppConfig {
            overlays_dir: Some(PathBuf::from("/home/user/overlays")),
        };

        save_config(&path, &config).unwrap();
        let loaded = load_config(&path);
        assert_eq!(
            loaded.overlays_dir,
            Some(PathBuf::from("/home/user/overlays"))
        );

        let _ = std::fs::remove_file(&path);
    }
}
