use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayPayload {
    pub instance_id: String,
    pub template: String,
    pub action: String,
    pub fields: HashMap<String, String>,
}

pub struct AppState {
    pub tx: broadcast::Sender<String>,
    pub current: Arc<Mutex<HashMap<String, OverlayPayload>>>,
    pub connected: Arc<AtomicUsize>,
    pub port: Arc<Mutex<Option<u16>>>,
    pub overlay_dir: Arc<Mutex<PathBuf>>,
    pub presets_path: PathBuf,
    pub config_path: PathBuf,
}
