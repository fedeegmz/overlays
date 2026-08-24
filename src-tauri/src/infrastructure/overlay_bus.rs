use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::broadcast;

use crate::application::ports::OverlayBus;
use crate::domain::overlay::OverlayPayload;

pub struct BroadcastOverlayBus {
    tx: broadcast::Sender<String>,
    current: Mutex<HashMap<String, OverlayPayload>>,
}

impl BroadcastOverlayBus {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(128);
        Self {
            tx,
            current: Mutex::new(HashMap::new()),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }
}

impl Default for BroadcastOverlayBus {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayBus for BroadcastOverlayBus {
    fn publish(&self, payload: &OverlayPayload) {
        self.current
            .lock()
            .unwrap()
            .insert(payload.instance_id.clone(), payload.clone());
        if let Ok(json) = serde_json::to_string(payload) {
            let _ = self.tx.send(json);
        }
    }

    fn snapshot(&self) -> Vec<OverlayPayload> {
        self.current.lock().unwrap().values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::overlay::OverlayAction;

    fn payload(instance_id: &str) -> OverlayPayload {
        OverlayPayload {
            instance_id: instance_id.to_string(),
            template: "lower-third-basico".into(),
            action: OverlayAction::Show,
            fields: HashMap::from([("titulo".to_string(), "Fede".to_string())]),
        }
    }

    #[tokio::test]
    async fn publish_broadcasts_and_stores_snapshot() {
        let bus = BroadcastOverlayBus::new();
        let mut rx = bus.subscribe();

        bus.publish(&payload("i1"));

        let received = rx.recv().await.unwrap();
        let value: serde_json::Value = serde_json::from_str(&received).unwrap();
        assert_eq!(value["instance_id"], "i1");
        assert_eq!(value["action"], "show");

        assert_eq!(bus.snapshot().len(), 1);
    }

    #[test]
    fn snapshot_reflects_latest_payload_per_instance() {
        let bus = BroadcastOverlayBus::new();

        bus.publish(&payload("i1"));
        let mut hidden = payload("i1");
        hidden.action = OverlayAction::Hide;
        bus.publish(&hidden);
        bus.publish(&payload("i2"));

        let snapshot = bus.snapshot();
        assert_eq!(snapshot.len(), 2);
        let i1 = snapshot.iter().find(|p| p.instance_id == "i1").unwrap();
        assert_eq!(i1.action, OverlayAction::Hide);
    }
}
