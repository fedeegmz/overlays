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
    pub const TEMPLATE_DISCOVERY_FAILED: &str = "template.discovery_failed";
    pub const LANGUAGE_UNSUPPORTED: &str = "language.unsupported";
    pub const OVERLAYS_DIR_INVALID: &str = "overlays_dir.invalid";
    pub const CONFIG_SAVE_FAILED: &str = "config.save_failed";
    pub const KEYRING_UNAVAILABLE: &str = "keyring.unavailable";
    pub const KEYRING_FAILED: &str = "keyring.failed";
    pub const KEYRING_DELETE_FAILED: &str = "keyring.delete_failed";
    pub const PROVIDER_UNAUTHORIZED: &str = "provider.unauthorized";
    pub const PROVIDER_RATE_LIMITED: &str = "provider.rate_limited";
    pub const PROVIDER_TIMEOUT: &str = "provider.timeout";
    pub const PROVIDER_NETWORK: &str = "provider.network";
    pub const PROVIDER_INVALID_RESPONSE: &str = "provider.invalid_response";
    pub const PROVIDER_UNKNOWN: &str = "provider.unknown";
    pub const MODEL_UNKNOWN: &str = "model.unknown";
    pub const GENERATION_INVALID_OUTPUT: &str = "generation.invalid_output";
    pub const GENERATION_EMPTY_PROMPT: &str = "generation.empty_prompt";
    pub const OVERLAYS_DIR_MISSING: &str = "overlays_dir.missing";
    pub const TEMPLATE_EXISTS: &str = "template.exists";
    pub const TEMPLATE_WRITE_FAILED: &str = "template.write_failed";
    pub const STAGED_MISSING: &str = "staged.missing";
    pub const COMMON_INTERNAL: &str = "common.internal";

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
            DomainError::KeyringUnavailable => Self::new(Self::KEYRING_UNAVAILABLE),
            DomainError::KeyringFailed { detail } => {
                Self::new(Self::KEYRING_FAILED).param("detail", detail)
            }
            DomainError::KeyringDeleteFailed => Self::new(Self::KEYRING_DELETE_FAILED),
            DomainError::ProviderUnauthorized => Self::new(Self::PROVIDER_UNAUTHORIZED),
            DomainError::ProviderRateLimited => Self::new(Self::PROVIDER_RATE_LIMITED),
            DomainError::ProviderTimeout => Self::new(Self::PROVIDER_TIMEOUT),
            DomainError::ProviderNetwork => Self::new(Self::PROVIDER_NETWORK),
            DomainError::ProviderInvalidResponse => Self::new(Self::PROVIDER_INVALID_RESPONSE),
            DomainError::GenerationInvalidOutput { issues } => {
                Self::new(Self::GENERATION_INVALID_OUTPUT)
                    .param("issues", serde_json::to_string(&issues).unwrap_or_default())
            }
            DomainError::GenerationEmptyPrompt => Self::new(Self::GENERATION_EMPTY_PROMPT),
            DomainError::OverlaysDirMissing => Self::new(Self::OVERLAYS_DIR_MISSING),
            DomainError::TemplateExists { name } => {
                Self::new(Self::TEMPLATE_EXISTS).param("name", name)
            }
            DomainError::TemplateWriteFailed { detail } => {
                Self::new(Self::TEMPLATE_WRITE_FAILED).param("detail", detail)
            }
            DomainError::StagedOverlayMissing => Self::new(Self::STAGED_MISSING),
            DomainError::UnknownProvider => Self::new(Self::PROVIDER_UNKNOWN),
            DomainError::UnknownModel => Self::new(Self::MODEL_UNKNOWN),
            DomainError::InvalidName { reason } => {
                Self::new(Self::COMMON_INTERNAL).param("reason", reason)
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
}
