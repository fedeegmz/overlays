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
    pub const KEYRING_ENTRY_MISSING: &str = "keyring.entry_missing";
    pub const KEYRING_EMPTY_SECRET: &str = "keyring.empty_secret";
    pub const PROVIDER_UNAUTHORIZED: &str = "provider.unauthorized";
    pub const PROVIDER_RATE_LIMITED: &str = "provider.rate_limited";
    pub const PROVIDER_TIMEOUT: &str = "provider.timeout";
    pub const PROVIDER_NETWORK: &str = "provider.network";
    pub const PROVIDER_UNAVAILABLE: &str = "provider.unavailable";
    pub const PROVIDER_INVALID_RESPONSE: &str = "provider.invalid_response";
    pub const PROVIDER_UNKNOWN: &str = "provider.unknown";
    pub const MODEL_UNKNOWN: &str = "model.unknown";
    pub const GENERATION_INVALID_OUTPUT: &str = "generation.invalid_output";
    pub const GENERATION_EMPTY_PROMPT: &str = "generation.empty_prompt";
    pub const GENERATION_INVALID_NAME: &str = "generation.invalid_name";
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
            DomainError::KeyringEntryMissing { provider } => {
                Self::new(Self::KEYRING_ENTRY_MISSING).param("provider", provider)
            }
            DomainError::KeyringEmptySecret => Self::new(Self::KEYRING_EMPTY_SECRET),
            DomainError::ProviderUnauthorized => Self::new(Self::PROVIDER_UNAUTHORIZED),
            DomainError::ProviderRateLimited => Self::new(Self::PROVIDER_RATE_LIMITED),
            DomainError::ProviderTimeout => Self::new(Self::PROVIDER_TIMEOUT),
            DomainError::ProviderNetwork { detail } => {
                Self::new(Self::PROVIDER_NETWORK).param("detail", detail)
            }
            DomainError::ProviderUnavailable { detail } => {
                Self::new(Self::PROVIDER_UNAVAILABLE).param("detail", detail)
            }
            DomainError::ProviderInvalidResponse { detail } => {
                Self::new(Self::PROVIDER_INVALID_RESPONSE).param("detail", detail)
            }
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
                Self::new(Self::GENERATION_INVALID_NAME).param("reason", reason)
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

    /// Every code `CommandError` can emit. Kept in one place so the locale
    /// parity test below always sees the full surface.
    fn all_codes() -> Vec<&'static str> {
        vec![
            CommandError::PRESET_EMPTY_NAME,
            CommandError::PRESET_SAVE_FAILED,
            CommandError::TEMPLATE_DISCOVERY_FAILED,
            CommandError::LANGUAGE_UNSUPPORTED,
            CommandError::OVERLAYS_DIR_INVALID,
            CommandError::CONFIG_SAVE_FAILED,
            CommandError::KEYRING_UNAVAILABLE,
            CommandError::KEYRING_FAILED,
            CommandError::KEYRING_DELETE_FAILED,
            CommandError::KEYRING_ENTRY_MISSING,
            CommandError::KEYRING_EMPTY_SECRET,
            CommandError::PROVIDER_UNAUTHORIZED,
            CommandError::PROVIDER_RATE_LIMITED,
            CommandError::PROVIDER_TIMEOUT,
            CommandError::PROVIDER_NETWORK,
            CommandError::PROVIDER_UNAVAILABLE,
            CommandError::PROVIDER_INVALID_RESPONSE,
            CommandError::PROVIDER_UNKNOWN,
            CommandError::MODEL_UNKNOWN,
            CommandError::GENERATION_INVALID_OUTPUT,
            CommandError::GENERATION_INVALID_NAME,
            CommandError::GENERATION_EMPTY_PROMPT,
            CommandError::OVERLAYS_DIR_MISSING,
            CommandError::TEMPLATE_EXISTS,
            CommandError::TEMPLATE_WRITE_FAILED,
            CommandError::STAGED_MISSING,
            CommandError::COMMON_INTERNAL,
        ]
    }

    fn locale_codes(locale: &str) -> serde_json::Map<String, serde_json::Value> {
        fn walk(
            obj: &serde_json::Map<String, serde_json::Value>,
            prefix: &str,
            out: &mut serde_json::Map<String, serde_json::Value>,
        ) {
            for (k, v) in obj {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                if let Some(nested) = v.as_object() {
                    walk(nested, &key, out);
                } else {
                    out.insert(key, v.clone());
                }
            }
        }
        let value: serde_json::Value = serde_json::from_str(locale).expect("locale must parse");
        let errors = value["errors"]
            .as_object()
            .expect("locale must have an errors object");
        let mut out = serde_json::Map::new();
        walk(errors, "", &mut out);
        out
    }

    #[test]
    fn every_emittable_code_exists_in_both_locales() {
        // The English and Spanish UI locales must know about every code the
        // backend can emit, so a new code can never surface as a
        // missing-translation key.
        let en = locale_codes(include_str!("../../../src/i18n/locales/en.json"));
        let es = locale_codes(include_str!("../../../src/i18n/locales/es.json"));
        for code in all_codes() {
            let key = format!("errors.{code}");
            assert!(en.contains_key(code), "missing {key} in en.json");
            assert!(es.contains_key(code), "missing {key} in es.json");
        }
    }

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
