use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use crate::application::group_preset_service::GroupPresetService;
use crate::application::instance_service::InstanceService;
use crate::application::preset_service::PresetService;
use crate::application::remote_access_service::RemoteAccessService;
use crate::application::template_catalog::{OverlaysDirHandle, TemplateCatalog};
use crate::application::upload_service::UploadService;
use crate::infrastructure::instance_events::BroadcastInstanceEvents;
use crate::infrastructure::overlay_bus::BroadcastOverlayBus;
use crate::infrastructure::session_revoker::WatchSessionRevoker;

/// Per-file upload limit: 2 GiB.
pub const MAX_UPLOAD_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Everything the `/remote` surface needs beyond the shared services.
pub struct RemoteDeps {
    pub access: Arc<RemoteAccessService>,
    /// Revocation epoch the remote WebSocket sessions listen to.
    pub sessions: Arc<WatchSessionRevoker>,
    pub presets: Arc<PresetService>,
    pub group_presets: Arc<GroupPresetService>,
    /// Registry change feed for the remote WebSocket.
    pub events: Arc<BroadcastInstanceEvents>,
}

pub struct HttpState {
    pub bus: Arc<BroadcastOverlayBus>,
    pub catalog: Arc<TemplateCatalog>,
    pub overlays_dir: OverlaysDirHandle,
    pub upload: Arc<UploadService>,
    pub instances: Arc<InstanceService>,
    pub remote: Arc<RemoteAccessService>,
    pub sessions: Arc<WatchSessionRevoker>,
    pub presets: Arc<PresetService>,
    pub group_presets: Arc<GroupPresetService>,
    pub events: Arc<BroadcastInstanceEvents>,
    /// Per-file upload limit in bytes; a field so tests can use a tiny value.
    pub max_upload_bytes: u64,
    pub connected: AtomicUsize,
    pub port: Mutex<Option<u16>>,
}

impl HttpState {
    pub fn new(
        bus: Arc<BroadcastOverlayBus>,
        catalog: Arc<TemplateCatalog>,
        overlays_dir: OverlaysDirHandle,
        upload: Arc<UploadService>,
        instances: Arc<InstanceService>,
        remote: RemoteDeps,
    ) -> Self {
        Self {
            bus,
            catalog,
            overlays_dir,
            upload,
            instances,
            remote: remote.access,
            sessions: remote.sessions,
            presets: remote.presets,
            group_presets: remote.group_presets,
            events: remote.events,
            max_upload_bytes: MAX_UPLOAD_FILE_BYTES,
            connected: AtomicUsize::new(0),
            port: Mutex::new(None),
        }
    }
}

#[cfg(test)]
impl HttpState {
    /// State for HTTP tests: example templates, in-memory stores, fixed clock
    /// (2026-03-07 09:05:01), no Downloads dir, remote access off.
    pub fn for_test(
        upload_settings: crate::application::upload_service::UploadSettingsHandle,
    ) -> Self {
        use crate::application::upload_service::Clock;
        use crate::domain::path_template::DateParts;
        use crate::infrastructure::clock::InstantClock;
        use crate::infrastructure::fs_template_source::FsTemplateSource;
        use crate::infrastructure::test_util::{MemConfigRepo, MemGroupPresetRepo, MemPresetRepo};

        struct FixedClock;
        impl Clock for FixedClock {
            fn now(&self) -> DateParts {
                DateParts {
                    year: 2026,
                    month: 3,
                    day: 7,
                    hour: 9,
                    minute: 5,
                    second: 1,
                }
            }
        }

        let examples = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("examples");
        let bus = Arc::new(BroadcastOverlayBus::new());
        let catalog = Arc::new(TemplateCatalog::new(
            Arc::new(FsTemplateSource),
            OverlaysDirHandle::new(examples.clone()),
        ));
        let presets = Arc::new(PresetService::new(Arc::new(MemPresetRepo::default())));
        let group_presets = Arc::new(GroupPresetService::new(Arc::new(
            MemGroupPresetRepo::default(),
        )));
        let events = Arc::new(BroadcastInstanceEvents::new());
        let instances = Arc::new(InstanceService::new(
            catalog.clone(),
            bus.clone(),
            events.clone(),
            presets.clone(),
            group_presets.clone(),
        ));
        let sessions = Arc::new(WatchSessionRevoker::new());
        let access = Arc::new(RemoteAccessService::new(
            Arc::new(MemConfigRepo::default()),
            Arc::new(InstantClock::new()),
            sessions.clone(),
        ));
        Self::new(
            bus,
            catalog,
            OverlaysDirHandle::new(examples),
            Arc::new(UploadService::new(
                Arc::new(FixedClock),
                upload_settings,
                None,
            )),
            instances,
            RemoteDeps {
                access,
                sessions,
                presets,
                group_presets,
                events,
            },
        )
    }
}
