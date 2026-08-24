use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use crate::application::template_catalog::{OverlaysDirHandle, TemplateCatalog};
use crate::infrastructure::overlay_bus::BroadcastOverlayBus;

pub struct HttpState {
    pub bus: Arc<BroadcastOverlayBus>,
    pub catalog: Arc<TemplateCatalog>,
    pub overlays_dir: OverlaysDirHandle,
    pub connected: AtomicUsize,
    pub port: Mutex<Option<u16>>,
}

impl HttpState {
    pub fn new(
        bus: Arc<BroadcastOverlayBus>,
        catalog: Arc<TemplateCatalog>,
        overlays_dir: OverlaysDirHandle,
    ) -> Self {
        Self {
            bus,
            catalog,
            overlays_dir,
            connected: AtomicUsize::new(0),
            port: Mutex::new(None),
        }
    }
}
