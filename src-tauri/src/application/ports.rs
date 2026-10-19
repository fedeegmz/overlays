use std::path::Path;

use crate::domain::checklist::Checklist;
use crate::domain::config::AppConfig;
use crate::domain::error::DomainResult;
use crate::domain::group_preset::GroupPreset;
use crate::domain::overlay::OverlayPayload;
use crate::domain::preset::Preset;
use crate::domain::template::TemplateInfo;

pub trait PresetRepository: Send + Sync {
    fn load(&self) -> Vec<Preset>;
    fn save(&self, presets: &[Preset]) -> DomainResult<()>;
}

/// Unlike `PresetRepository`, loading is strict: a corrupt store is an error,
/// never an empty list, so a later save cannot wipe the user's data.
pub trait GroupPresetRepository: Send + Sync {
    fn load(&self) -> DomainResult<Vec<GroupPreset>>;
    fn save(&self, groups: &[GroupPreset]) -> DomainResult<()>;
}

/// Strict like `GroupPresetRepository`: a corrupt store is an error, never an
/// empty list, so a later save cannot wipe the user's data.
pub trait ChecklistRepository: Send + Sync {
    fn load(&self) -> DomainResult<Vec<Checklist>>;
    fn save(&self, checklists: &[Checklist]) -> DomainResult<()>;
}

pub trait ConfigRepository: Send + Sync {
    fn load(&self) -> AppConfig;
    fn save(&self, config: &AppConfig) -> DomainResult<()>;
}

pub trait TemplateSource: Send + Sync {
    fn discover(&self, overlay_dir: &Path) -> DomainResult<Vec<TemplateInfo>>;
}

pub trait OverlayBus: Send + Sync {
    fn publish(&self, payload: &OverlayPayload);
    fn snapshot(&self) -> Vec<OverlayPayload>;
}
