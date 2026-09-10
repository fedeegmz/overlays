use std::fmt;

#[derive(Debug, Clone)]
pub enum DomainError {
    PresetEmptyName,
    PresetSaveFailed { detail: String },
    TemplateDiscoveryFailed { detail: String },
    LanguageUnsupported { lang: String },
    OverlaysDirInvalid { path: String },
    ConfigSaveFailed { detail: String },
    // Constructed by keyring/generation infra; wired in PR2/PR3.
    KeyringUnavailable,
    KeyringFailed { detail: String },
    KeyringDeleteFailed,
    ProviderUnauthorized,
    ProviderRateLimited,
    ProviderTimeout,
    ProviderNetwork { detail: String },
    ProviderInvalidResponse { detail: String },
    GenerationInvalidOutput { issues: Vec<String> },
    GenerationEmptyPrompt,
    OverlaysDirMissing,
    TemplateExists { name: String },
    TemplateWriteFailed { detail: String },
    StagedOverlayMissing,
    UnknownProvider,
    UnknownModel,
    InvalidName { reason: String },
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
            Self::ProviderNetwork { detail } => write!(f, "provider network error: {detail}"),
            Self::ProviderInvalidResponse { detail } => {
                write!(f, "provider invalid response: {detail}")
            }
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
