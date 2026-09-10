use std::fmt;

#[derive(Debug, Clone)]
pub enum DomainError {
    PresetEmptyName,
    PresetSaveFailed {
        detail: String,
    },
    TemplateDiscoveryFailed {
        detail: String,
    },
    LanguageUnsupported {
        lang: String,
    },
    OverlaysDirInvalid {
        path: String,
    },
    ConfigSaveFailed {
        detail: String,
    },
    // Constructed by keyring/generation infra; wired in PR2/PR3.
    #[allow(dead_code)]
    KeyringUnavailable,
    #[allow(dead_code)]
    KeyringFailed {
        detail: String,
    },
    #[allow(dead_code)]
    KeyringDeleteFailed,
    #[allow(dead_code)]
    ProviderUnauthorized,
    #[allow(dead_code)]
    ProviderRateLimited,
    #[allow(dead_code)]
    ProviderTimeout,
    #[allow(dead_code)]
    ProviderNetwork,
    #[allow(dead_code)]
    ProviderInvalidResponse,
    #[allow(dead_code)]
    GenerationInvalidOutput {
        issues: Vec<String>,
    },
    #[allow(dead_code)]
    GenerationEmptyPrompt,
    #[allow(dead_code)]
    OverlaysDirMissing,
    #[allow(dead_code)]
    TemplateExists {
        name: String,
    },
    #[allow(dead_code)]
    TemplateWriteFailed {
        detail: String,
    },
    #[allow(dead_code)]
    StagedOverlayMissing,
    #[allow(dead_code)]
    UnknownProvider,
    #[allow(dead_code)]
    UnknownModel,
    #[allow(dead_code)]
    InvalidName {
        reason: String,
    },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PresetEmptyName => write!(f, "preset name is empty"),
            Self::PresetSaveFailed { detail } => write!(f, "preset save failed: {detail}"),
            Self::TemplateDiscoveryFailed { detail } => {
                write!(f, "template discovery failed: {detail}")
            }
            Self::LanguageUnsupported { lang } => write!(f, "unsupported language: {lang}"),
            Self::OverlaysDirInvalid { path } => write!(f, "invalid overlays dir: {path}"),
            Self::ConfigSaveFailed { detail } => write!(f, "config save failed: {detail}"),
            Self::KeyringUnavailable => write!(f, "keyring is not available"),
            Self::KeyringFailed { detail } => write!(f, "keyring operation failed: {detail}"),
            Self::KeyringDeleteFailed => write!(f, "keyring delete failed"),
            Self::ProviderUnauthorized => write!(f, "provider rejected the API key"),
            Self::ProviderRateLimited => write!(f, "provider rate limit exceeded"),
            Self::ProviderTimeout => write!(f, "provider request timed out"),
            Self::ProviderNetwork => write!(f, "provider network error"),
            Self::ProviderInvalidResponse => write!(f, "provider returned an invalid response"),
            Self::GenerationInvalidOutput { issues } => {
                write!(f, "generated output is invalid: {:?}", issues)
            }
            Self::GenerationEmptyPrompt => write!(f, "generation prompt is empty"),
            Self::OverlaysDirMissing => write!(f, "overlays directory is not configured"),
            Self::TemplateExists { name } => write!(f, "template already exists: {name}"),
            Self::TemplateWriteFailed { detail } => {
                write!(f, "template write failed: {detail}")
            }
            Self::StagedOverlayMissing => write!(f, "staged overlay is missing"),
            Self::UnknownProvider => write!(f, "unknown provider"),
            Self::UnknownModel => write!(f, "unknown model"),
            Self::InvalidName { reason } => write!(f, "invalid name: {reason}"),
        }
    }
}

impl std::error::Error for DomainError {}

pub type DomainResult<T> = Result<T, DomainError>;
