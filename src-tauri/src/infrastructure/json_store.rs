use std::path::{Path, PathBuf};

use crate::application::ports::{ConfigRepository, GroupPresetRepository, PresetRepository};
use crate::domain::config::AppConfig;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::group_preset::GroupPreset;
use crate::domain::preset::Preset;

fn write_json(
    path: &Path,
    value: &impl serde::Serialize,
    failed: fn(String) -> DomainError,
) -> DomainResult<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| failed(format!("could not create {dir:?}: {e}")))?;
    }
    let json = serde_json::to_string_pretty(value).map_err(|e| failed(e.to_string()))?;
    std::fs::write(path, json).map_err(|e| failed(format!("could not write {path:?}: {e}")))
}

pub struct JsonPresetRepository {
    path: PathBuf,
}

impl JsonPresetRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl PresetRepository for JsonPresetRepository {
    fn load(&self) -> Vec<Preset> {
        if !self.path.exists() {
            return Vec::new();
        }
        match std::fs::read_to_string(&self.path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    fn save(&self, presets: &[Preset]) -> DomainResult<()> {
        write_json(&self.path, &presets, |detail| {
            DomainError::PresetSaveFailed { detail }
        })
    }
}

pub struct JsonGroupPresetRepository {
    path: PathBuf,
}

impl JsonGroupPresetRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl GroupPresetRepository for JsonGroupPresetRepository {
    fn load(&self) -> DomainResult<Vec<GroupPreset>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&self.path).map_err(|e| {
            DomainError::GroupPresetLoadFailed {
                detail: format!("could not read {:?}: {e}", self.path),
            }
        })?;
        serde_json::from_str(&content).map_err(|e| DomainError::GroupPresetLoadFailed {
            detail: format!("could not parse {:?}: {e}", self.path),
        })
    }

    fn save(&self, groups: &[GroupPreset]) -> DomainResult<()> {
        write_json(&self.path, &groups, |detail| {
            DomainError::GroupPresetSaveFailed { detail }
        })
    }
}

pub struct JsonConfigRepository {
    path: PathBuf,
}

impl JsonConfigRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl ConfigRepository for JsonConfigRepository {
    fn load(&self) -> AppConfig {
        if !self.path.exists() {
            return AppConfig::default();
        }
        match std::fs::read_to_string(&self.path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    }

    fn save(&self, config: &AppConfig) -> DomainResult<()> {
        write_json(&self.path, config, |detail| DomainError::ConfigSaveFailed {
            detail,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn temp_path(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{prefix}-{}.json", std::process::id()))
    }

    #[test]
    fn missing_preset_file_loads_empty() {
        let repo = JsonPresetRepository::new(std::env::temp_dir().join("overlays-does-not-exist"));
        assert!(repo.load().is_empty());
    }

    #[test]
    fn presets_round_trip_preserves_data() {
        let path = temp_path("overlays-test-presets");
        let repo = JsonPresetRepository::new(path.clone());
        let fields = HashMap::from([("titulo".to_string(), "Fede".to_string())]);
        let preset = Preset::new("Fede - Dev", "lower-third-basico", fields).unwrap();

        repo.save(&[preset]).unwrap();
        let loaded = repo.load();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Fede - Dev");
        assert_eq!(loaded[0].template, "lower-third-basico");
        assert_eq!(loaded[0].fields["titulo"], "Fede");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_config_file_loads_default() {
        let repo = JsonConfigRepository::new(std::env::temp_dir().join("overlays-does-not-exist"));
        assert!(repo.load().overlays_dir.is_none());
        assert!(repo.load().language.is_none());
    }

    #[test]
    fn config_round_trip_preserves_data() {
        let path = temp_path("overlays-test-config-rt");
        let repo = JsonConfigRepository::new(path.clone());
        let config = AppConfig {
            overlays_dir: Some(PathBuf::from("/home/user/overlays")),
            language: Some("es".to_string()),
            ..AppConfig::default()
        };

        repo.save(&config).unwrap();
        let loaded = repo.load();
        assert_eq!(
            loaded.overlays_dir,
            Some(PathBuf::from("/home/user/overlays"))
        );
        assert_eq!(loaded.language, Some("es".to_string()));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn legacy_config_without_language_loads_default() {
        let path = temp_path("overlays-test-config-legacy");
        std::fs::write(&path, r#"{ "overlays_dir": "/home/user/overlays" }"#).unwrap();
        let repo = JsonConfigRepository::new(path.clone());
        let loaded = repo.load();
        assert!(loaded.language.is_none());
        assert_eq!(
            loaded.overlays_dir,
            Some(PathBuf::from("/home/user/overlays"))
        );

        let _ = std::fs::remove_file(&path);
    }

    fn group(name: &str, templates: &[&str]) -> GroupPreset {
        let entries = templates
            .iter()
            .map(|t| crate::domain::group_preset::GroupPresetEntry {
                template: t.to_string(),
                fields: HashMap::from([("titulo".to_string(), t.to_string())]),
            })
            .collect();
        GroupPreset::new(name, entries).unwrap()
    }

    #[test]
    fn missing_group_file_loads_empty() {
        let repo =
            JsonGroupPresetRepository::new(std::env::temp_dir().join("overlays-groups-missing"));
        assert!(repo.load().unwrap().is_empty());
    }

    #[test]
    fn groups_round_trip_preserves_order_and_duplicates() {
        let path = temp_path("overlays-test-groups");
        let repo = JsonGroupPresetRepository::new(path.clone());

        repo.save(&[group("Show", &["a", "b", "a"]), group("Other", &["c"])])
            .unwrap();
        let loaded = repo.load().unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "Show");
        let templates: Vec<_> = loaded[0]
            .entries
            .iter()
            .map(|e| e.template.as_str())
            .collect();
        assert_eq!(templates, ["a", "b", "a"]);
        assert_eq!(loaded[0].entries[1].fields["titulo"], "b");
        assert_eq!(loaded[1].name, "Other");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_group_file_fails_to_load() {
        let path = temp_path("overlays-test-groups-corrupt-load");
        std::fs::write(&path, "{ not json").unwrap();
        let repo = JsonGroupPresetRepository::new(path.clone());

        assert!(matches!(
            repo.load(),
            Err(DomainError::GroupPresetLoadFailed { .. })
        ));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_group_file_is_untouched_by_failed_save() {
        use crate::application::group_preset_service::GroupPresetService;

        let path = temp_path("overlays-test-groups-corrupt-save");
        std::fs::write(&path, "{ not json").unwrap();
        let svc = GroupPresetService::new(std::sync::Arc::new(JsonGroupPresetRepository::new(
            path.clone(),
        )));

        let entries = group("Show", &["a"]).entries;
        assert!(svc.save("Show".into(), entries).is_err());
        assert!(svc.delete("Show").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");

        let _ = std::fs::remove_file(&path);
    }
}
