use std::fmt;

#[derive(Debug, Clone)]
pub enum DomainError {
    PresetEmptyName,
    PresetSaveFailed { detail: String },
    GroupPresetEmptyName,
    GroupPresetNoEntries,
    GroupPresetInvalidEntry { index: usize },
    GroupPresetNotFound { name: String },
    GroupPresetLoadFailed { detail: String },
    GroupPresetSaveFailed { detail: String },
    TemplateDiscoveryFailed { detail: String },
    LanguageUnsupported { lang: String },
    OverlaysDirInvalid { path: String },
    ConfigSaveFailed { detail: String },
    AssetOutsideOverlaysDir { path: String },
    AssetResolveFailed { path: String },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PresetEmptyName => write!(f, "preset name is empty"),
            Self::PresetSaveFailed { detail } => write!(f, "preset save failed: {detail}"),
            Self::GroupPresetEmptyName => write!(f, "group preset name is empty"),
            Self::GroupPresetNoEntries => write!(f, "group preset has no entries"),
            Self::GroupPresetInvalidEntry { index } => {
                write!(f, "group preset entry {index} has an empty template")
            }
            Self::GroupPresetNotFound { name } => write!(f, "group preset not found: {name}"),
            Self::GroupPresetLoadFailed { detail } => {
                write!(f, "group preset load failed: {detail}")
            }
            Self::GroupPresetSaveFailed { detail } => {
                write!(f, "group preset save failed: {detail}")
            }
            Self::TemplateDiscoveryFailed { detail } => {
                write!(f, "template discovery failed: {detail}")
            }
            Self::LanguageUnsupported { lang } => write!(f, "unsupported language: {lang}"),
            Self::OverlaysDirInvalid { path } => write!(f, "invalid overlays dir: {path}"),
            Self::ConfigSaveFailed { detail } => write!(f, "config save failed: {detail}"),
            Self::AssetOutsideOverlaysDir { path } => {
                write!(f, "asset outside overlays dir: {path}")
            }
            Self::AssetResolveFailed { path } => write!(f, "could not resolve asset: {path}"),
        }
    }
}

impl std::error::Error for DomainError {}

pub type DomainResult<T> = Result<T, DomainError>;
