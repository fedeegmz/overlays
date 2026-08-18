use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub nombre: String,
    pub template: String,
    pub fields: HashMap<String, String>,
}

pub fn load_presets(path: &Path) -> Vec<Preset> {
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn save_presets(path: &Path, presets: &[Preset]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("no se pudo crear {dir:?}: {e}"))?;
    }
    let json = serde_json::to_string_pretty(presets).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| format!("no se pudo escribir {path:?}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> std::path::PathBuf {
        let dir = std::env::temp_dir();
        dir.join(format!("overlays-test-{}.json", std::process::id()))
    }

    #[test]
    fn missing_file_loads_empty() {
        let path = std::env::temp_dir().join("overlays-test-does-not-exist.json");
        assert!(load_presets(&path).is_empty());
    }

    #[test]
    fn round_trip_preserves_data() {
        let path = temp_path();
        let fields = HashMap::from([("titulo".into(), "Fede".into())]);
        let presets = vec![Preset {
            nombre: "Fede - Dev".into(),
            template: "lower-third-basico".into(),
            fields,
        }];

        save_presets(&path, &presets).unwrap();
        let loaded = load_presets(&path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].nombre, "Fede - Dev");
        assert_eq!(loaded[0].template, "lower-third-basico");
        assert_eq!(loaded[0].fields["titulo"], "Fede");

        let _ = std::fs::remove_file(&path);
    }
}
