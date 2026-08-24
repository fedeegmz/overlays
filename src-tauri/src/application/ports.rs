use std::path::Path;

use crate::domain::config::AppConfig;
use crate::domain::error::DomainResult;
use crate::domain::overlay::OverlayPayload;
use crate::domain::preset::Preset;
use crate::domain::template::TemplateInfo;

pub trait PresetRepository: Send + Sync {
    fn load(&self) -> Vec<Preset>;
    fn save(&self, presets: &[Preset]) -> DomainResult<()>;
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
