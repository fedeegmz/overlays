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
    ChecklistInvalidId,
    ChecklistEmptyName,
    ChecklistNoItems,
    ChecklistEmptyItem { index: usize },
    ChecklistDuplicateName { name: String },
    ChecklistNotFound { id: String },
    ChecklistLoadFailed { detail: String },
    ChecklistSaveFailed { detail: String },
    TemplateDiscoveryFailed { detail: String },
    LanguageUnsupported { lang: String },
    OverlaysDirInvalid { path: String },
    ConfigSaveFailed { detail: String },
    AssetOutsideOverlaysDir { path: String },
    AssetResolveFailed { path: String },
    UploadDisabled,
    UploadTooLarge,
    UploadMissingFile,
    UploadBadRequest { detail: String },
    UploadNameExhausted,
    UploadDestinationUnavailable,
    UploadWriteFailed { detail: String },
    UploadDirInvalid { path: String },
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
            Self::ChecklistInvalidId => write!(f, "checklist or item id is empty or duplicated"),
            Self::ChecklistEmptyName => write!(f, "checklist name is empty"),
            Self::ChecklistNoItems => write!(f, "checklist has no items"),
            Self::ChecklistEmptyItem { index } => {
                write!(f, "checklist item {index} has empty text")
            }
            Self::ChecklistDuplicateName { name } => {
                write!(f, "checklist name already in use: {name}")
            }
            Self::ChecklistNotFound { id } => write!(f, "checklist not found: {id}"),
            Self::ChecklistLoadFailed { detail } => {
                write!(f, "checklist load failed: {detail}")
            }
            Self::ChecklistSaveFailed { detail } => {
                write!(f, "checklist save failed: {detail}")
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
            Self::UploadDisabled => write!(f, "uploads are disabled"),
            Self::UploadTooLarge => write!(f, "uploaded file is too large"),
            Self::UploadMissingFile => write!(f, "upload has no file"),
            Self::UploadBadRequest { detail } => write!(f, "bad upload request: {detail}"),
            Self::UploadNameExhausted => write!(f, "no free file name available"),
            Self::UploadDestinationUnavailable => write!(f, "upload destination unavailable"),
            Self::UploadWriteFailed { detail } => write!(f, "upload write failed: {detail}"),
            Self::UploadDirInvalid { path } => write!(f, "invalid upload dir: {path}"),
        }
    }
}

impl std::error::Error for DomainError {}

pub type DomainResult<T> = Result<T, DomainError>;
