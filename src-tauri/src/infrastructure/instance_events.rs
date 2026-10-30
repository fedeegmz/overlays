//! Adapters for the `InstanceEvents` port.
//!
//! They run while `InstanceService` holds its registry lock, so they must stay
//! non-blocking and must never call back into the service.

use std::sync::{Arc, Mutex};

use tauri::Emitter;
use tokio::sync::broadcast;

use crate::application::ports::InstanceEvents;
use crate::domain::instance::InstancesSnapshot;

/// Name of the Tauri event carrying an `InstancesSnapshot`.
pub const INSTANCES_CHANGED_EVENT: &str = "instances-changed";

/// Emits every snapshot to the desktop webviews.
pub struct TauriInstanceEvents {
    app: tauri::AppHandle,
}

impl TauriInstanceEvents {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl InstanceEvents for TauriInstanceEvents {
    fn changed(&self, snapshot: &InstancesSnapshot) {
        if let Err(e) = self.app.emit(INSTANCES_CHANGED_EVENT, snapshot) {
            eprintln!("[overlays] could not emit {INSTANCES_CHANGED_EVENT}: {e}");
        }
    }
}

/// Feeds in-process subscribers (the remote WebSocket).
/// Keeps the latest snapshot so a lagged subscriber can resync.
pub struct BroadcastInstanceEvents {
    tx: broadcast::Sender<InstancesSnapshot>,
    latest: Mutex<Option<InstancesSnapshot>>,
}

impl BroadcastInstanceEvents {
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self {
            tx,
            latest: Mutex::new(None),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<InstancesSnapshot> {
        self.tx.subscribe()
    }

    /// The most recent snapshot, for a subscriber that lagged or just joined.
    pub fn latest(&self) -> Option<InstancesSnapshot> {
        self.latest
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl Default for BroadcastInstanceEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl InstanceEvents for BroadcastInstanceEvents {
    fn changed(&self, snapshot: &InstancesSnapshot) {
        *self.latest.lock().unwrap_or_else(|e| e.into_inner()) = Some(snapshot.clone());
        // No subscribers is not an error.
        let _ = self.tx.send(snapshot.clone());
    }
}

/// Delivers each snapshot to every sink, in order.
pub struct FanOut {
    sinks: Vec<Arc<dyn InstanceEvents>>,
}

impl FanOut {
    pub fn new(sinks: Vec<Arc<dyn InstanceEvents>>) -> Self {
        Self { sinks }
    }
}

impl InstanceEvents for FanOut {
    fn changed(&self, snapshot: &InstancesSnapshot) {
        for sink in &self.sinks {
            sink.changed(snapshot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::instance::EventSource;
    use std::sync::Mutex;

    fn snap(revision: u64) -> InstancesSnapshot {
        InstancesSnapshot {
            revision,
            source: EventSource::desktop("w1"),
            instances: Vec::new(),
        }
    }

    struct Recorder(Mutex<Vec<u64>>);

    impl InstanceEvents for Recorder {
        fn changed(&self, snapshot: &InstancesSnapshot) {
            self.0.lock().unwrap().push(snapshot.revision);
        }
    }

    #[tokio::test]
    async fn fanout_calls_tauri_and_broadcast() {
        let tauri_side = Arc::new(Recorder(Mutex::new(Vec::new())));
        let broadcast = Arc::new(BroadcastInstanceEvents::new());
        let mut rx = broadcast.subscribe();
        let fan = FanOut::new(vec![tauri_side.clone(), broadcast.clone()]);

        fan.changed(&snap(3));

        assert_eq!(*tauri_side.0.lock().unwrap(), vec![3]);
        assert_eq!(rx.recv().await.unwrap().revision, 3);
    }

    #[tokio::test]
    async fn broadcast_lagged_subscriber_can_resubscribe_and_get_latest() {
        let broadcast = BroadcastInstanceEvents::with_capacity(2);
        let mut rx = broadcast.subscribe();
        for revision in 1..=5 {
            broadcast.changed(&snap(revision));
        }

        assert!(matches!(
            rx.recv().await,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_))
        ));
        assert_eq!(broadcast.latest().unwrap().revision, 5);
        let mut fresh = broadcast.subscribe();
        broadcast.changed(&snap(6));
        assert_eq!(fresh.recv().await.unwrap().revision, 6);
    }

    #[test]
    fn fanout_without_subscribers_does_not_error() {
        let broadcast = Arc::new(BroadcastInstanceEvents::new());
        let fan = FanOut::new(vec![broadcast.clone()]);
        fan.changed(&snap(1));
        assert_eq!(broadcast.latest().unwrap().revision, 1);
    }
}
