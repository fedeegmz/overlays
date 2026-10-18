use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use crate::application::template_catalog::{OverlaysDirHandle, TemplateCatalog};
use crate::application::upload_service::UploadService;
use crate::infrastructure::overlay_bus::BroadcastOverlayBus;

/// Per-file upload limit: 2 GiB.
pub const MAX_UPLOAD_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

pub struct HttpState {
    pub bus: Arc<BroadcastOverlayBus>,
    pub catalog: Arc<TemplateCatalog>,
    pub overlays_dir: OverlaysDirHandle,
    pub upload: Arc<UploadService>,
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
    ) -> Self {
        Self {
            bus,
            catalog,
            overlays_dir,
            upload,
            max_upload_bytes: MAX_UPLOAD_FILE_BYTES,
            connected: AtomicUsize::new(0),
            port: Mutex::new(None),
        }
    }
}

#[cfg(test)]
impl HttpState {
    /// State for HTTP tests: example templates, fixed clock (2026-03-07 09:05:01), no Downloads dir.
    pub fn for_test(
        upload_settings: crate::application::upload_service::UploadSettingsHandle,
    ) -> Self {
        use crate::application::template_catalog::OverlaysDirHandle;
        use crate::application::upload_service::Clock;
        use crate::domain::path_template::DateParts;
        use crate::infrastructure::fs_template_source::FsTemplateSource;

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
        Self::new(
            Arc::new(BroadcastOverlayBus::new()),
            Arc::new(TemplateCatalog::new(
                Arc::new(FsTemplateSource),
                OverlaysDirHandle::new(examples.clone()),
            )),
            OverlaysDirHandle::new(examples),
            Arc::new(UploadService::new(
                Arc::new(FixedClock),
                upload_settings,
                None,
            )),
        )
    }
}
