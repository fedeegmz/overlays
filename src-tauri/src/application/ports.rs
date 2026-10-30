use std::path::Path;

use crate::domain::checklist::Checklist;
use crate::domain::config::AppConfig;
use crate::domain::error::DomainResult;
use crate::domain::group_preset::GroupPreset;
use crate::domain::instance::InstancesSnapshot;
use crate::domain::overlay::OverlayPayload;
use crate::domain::preset::Preset;
use crate::domain::template::Manifest;

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
    fn discover(&self, overlay_dir: &Path) -> DomainResult<Manifest>;
}

pub trait OverlayBus: Send + Sync {
    fn publish(&self, payload: &OverlayPayload);
    fn snapshot(&self) -> Vec<OverlayPayload>;
}

/// Consumer of registry change events (Tauri emitter, remote broadcast).
pub trait InstanceEvents: Send + Sync {
    fn changed(&self, snapshot: &InstancesSnapshot);
}

/// Read access to the template catalog. Implementations may be expensive
/// (the real one walks the overlays tree), so callers read it once per
/// operation.
pub trait TemplateLookup: Send + Sync {
    fn manifest(&self) -> DomainResult<Manifest>;
}

/// Monotonic milliseconds since an arbitrary origin, for TTLs and rate
/// windows. Unlike wall time it never jumps backwards.
pub trait MonotonicClock: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// Closes every open remote session (WebSocket) at once.
pub trait SessionRevoker: Send + Sync {
    fn revoke_all(&self);
}

/// Read access to the bundled frontend files the phone app is served from.
/// `path` is relative to the build output root (`remote.html`, `assets/x.js`);
/// the result is the file bytes and their MIME type.
pub trait StaticAssets: Send + Sync {
    fn get(&self, path: &str) -> Option<(Vec<u8>, String)>;
}
