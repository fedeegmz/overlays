use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

use crate::domain::error::DomainError;

/// Structured error returned by every Tauri command.
///
/// The frontend receives this object as the promise rejection value and
/// translates `code` into a localized message via vue-i18n (`errors.<code>`).
#[derive(Debug, Clone, Serialize)]
pub struct CommandError {
    pub code: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, String>,
}

impl CommandError {
    pub const PRESET_EMPTY_NAME: &str = "preset.empty_name";
    pub const PRESET_SAVE_FAILED: &str = "preset.save_failed";
    pub const GROUP_PRESET_EMPTY_NAME: &str = "group_preset.empty_name";
    pub const GROUP_PRESET_NO_ENTRIES: &str = "group_preset.no_entries";
    pub const GROUP_PRESET_INVALID_ENTRY: &str = "group_preset.invalid_entry";
    pub const GROUP_PRESET_NOT_FOUND: &str = "group_preset.not_found";
    pub const GROUP_PRESET_LOAD_FAILED: &str = "group_preset.load_failed";
    pub const GROUP_PRESET_SAVE_FAILED: &str = "group_preset.save_failed";
    pub const TEMPLATE_DISCOVERY_FAILED: &str = "template.discovery_failed";
    pub const LANGUAGE_UNSUPPORTED: &str = "language.unsupported";
    pub const OVERLAYS_DIR_INVALID: &str = "overlays_dir.invalid";
    pub const CONFIG_SAVE_FAILED: &str = "config.save_failed";
    pub const FILE_ASSET_OUTSIDE_OVERLAYS_DIR: &str = "file.asset_outside_overlays_dir";
    pub const FILE_RESOLVE_FAILED: &str = "file.resolve_failed";

    pub fn new(code: &str) -> Self {
        Self {
            code: code.to_string(),
            params: HashMap::new(),
        }
    }

    pub fn param(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.insert(key.to_string(), value.into());
        self
    }
}

impl From<DomainError> for CommandError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::PresetEmptyName => Self::new(Self::PRESET_EMPTY_NAME),
            DomainError::PresetSaveFailed { detail } => {
                Self::new(Self::PRESET_SAVE_FAILED).param("detail", detail)
            }
            DomainError::GroupPresetEmptyName => Self::new(Self::GROUP_PRESET_EMPTY_NAME),
            DomainError::GroupPresetNoEntries => Self::new(Self::GROUP_PRESET_NO_ENTRIES),
            DomainError::GroupPresetInvalidEntry { index } => {
                Self::new(Self::GROUP_PRESET_INVALID_ENTRY).param("index", index.to_string())
            }
            DomainError::GroupPresetNotFound { name } => {
                Self::new(Self::GROUP_PRESET_NOT_FOUND).param("name", name)
            }
            DomainError::GroupPresetLoadFailed { detail } => {
                Self::new(Self::GROUP_PRESET_LOAD_FAILED).param("detail", detail)
            }
            DomainError::GroupPresetSaveFailed { detail } => {
                Self::new(Self::GROUP_PRESET_SAVE_FAILED).param("detail", detail)
            }
            DomainError::TemplateDiscoveryFailed { detail } => {
                Self::new(Self::TEMPLATE_DISCOVERY_FAILED).param("detail", detail)
            }
            DomainError::LanguageUnsupported { lang } => {
                Self::new(Self::LANGUAGE_UNSUPPORTED).param("lang", lang)
            }
            DomainError::OverlaysDirInvalid { path } => {
                Self::new(Self::OVERLAYS_DIR_INVALID).param("path", path)
            }
            DomainError::ConfigSaveFailed { detail } => {
                Self::new(Self::CONFIG_SAVE_FAILED).param("detail", detail)
            }
            DomainError::AssetOutsideOverlaysDir { path } => {
                Self::new(Self::FILE_ASSET_OUTSIDE_OVERLAYS_DIR).param("path", path)
            }
            DomainError::AssetResolveFailed { path } => {
                Self::new(Self::FILE_RESOLVE_FAILED).param("path", path)
            }
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code)?;
        if !self.params.is_empty() {
            write!(f, " {:?}", self.params)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_code_without_params() {
        let json =
            serde_json::to_value(CommandError::new(CommandError::PRESET_EMPTY_NAME)).unwrap();
        assert_eq!(json, serde_json::json!({ "code": "preset.empty_name" }));
    }

    #[test]
    fn serializes_params_when_present() {
        let err = CommandError::new(CommandError::LANGUAGE_UNSUPPORTED).param("lang", "fr");
        let json = serde_json::to_value(err).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "language.unsupported", "params": { "lang": "fr" } })
        );
    }

    #[test]
    fn display_shows_code_and_params() {
        let err = CommandError::new(CommandError::OVERLAYS_DIR_INVALID).param("path", "/tmp/x");
        assert!(err.to_string().starts_with("overlays_dir.invalid"));
    }

    #[test]
    fn maps_domain_error_to_i18n_code_with_params() {
        let json = serde_json::to_value(CommandError::from(DomainError::OverlaysDirInvalid {
            path: "/tmp/x".into(),
        }))
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "overlays_dir.invalid", "params": { "path": "/tmp/x" } })
        );
    }

    #[test]
    fn maps_group_preset_errors_to_codes() {
        let code = |e: DomainError| serde_json::to_value(CommandError::from(e)).unwrap();
        assert_eq!(
            code(DomainError::GroupPresetEmptyName),
            serde_json::json!({ "code": "group_preset.empty_name" })
        );
        assert_eq!(
            code(DomainError::GroupPresetNoEntries),
            serde_json::json!({ "code": "group_preset.no_entries" })
        );
        assert_eq!(
            code(DomainError::GroupPresetInvalidEntry { index: 2 }),
            serde_json::json!({ "code": "group_preset.invalid_entry", "params": { "index": "2" } })
        );
        assert_eq!(
            code(DomainError::GroupPresetNotFound {
                name: "Show".into()
            }),
            serde_json::json!({ "code": "group_preset.not_found", "params": { "name": "Show" } })
        );
        assert_eq!(
            code(DomainError::GroupPresetLoadFailed {
                detail: "bad".into()
            }),
            serde_json::json!({ "code": "group_preset.load_failed", "params": { "detail": "bad" } })
        );
        assert_eq!(
            code(DomainError::GroupPresetSaveFailed {
                detail: "io".into()
            }),
            serde_json::json!({ "code": "group_preset.save_failed", "params": { "detail": "io" } })
        );
    }
}
