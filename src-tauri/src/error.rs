use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

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
    pub const INTERNAL: &str = "common.internal";
    pub const PRESET_EMPTY_NAME: &str = "preset.empty_name";
    pub const PRESET_SAVE_FAILED: &str = "preset.save_failed";
    pub const TEMPLATE_DISCOVERY_FAILED: &str = "template.discovery_failed";
    pub const LANGUAGE_UNSUPPORTED: &str = "language.unsupported";
    pub const OVERLAYS_DIR_INVALID: &str = "overlays_dir.invalid";
    pub const CONFIG_SAVE_FAILED: &str = "config.save_failed";

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
        let json = serde_json::to_value(CommandError::new(CommandError::PRESET_EMPTY_NAME))
            .unwrap();
        assert_eq!(json, serde_json::json!({ "code": "preset.empty_name" }));
    }

    #[test]
    fn serializes_params_when_present() {
        let err = CommandError::new(CommandError::LANGUAGE_UNSUPPORTED)
            .param("lang", "fr");
        let json = serde_json::to_value(err).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "language.unsupported", "params": { "lang": "fr" } })
        );
    }

    #[test]
    fn display_shows_code_and_params() {
        let err =
            CommandError::new(CommandError::OVERLAYS_DIR_INVALID).param("path", "/tmp/x");
        assert!(err.to_string().starts_with("overlays_dir.invalid"));
    }
}
