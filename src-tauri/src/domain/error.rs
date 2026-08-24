use std::fmt;

#[derive(Debug, Clone)]
pub enum DomainError {
    PresetEmptyName,
    PresetSaveFailed { detail: String },
    TemplateDiscoveryFailed { detail: String },
    LanguageUnsupported { lang: String },
    OverlaysDirInvalid { path: String },
    ConfigSaveFailed { detail: String },
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
        }
    }
}

impl std::error::Error for DomainError {}

pub type DomainResult<T> = Result<T, DomainError>;
