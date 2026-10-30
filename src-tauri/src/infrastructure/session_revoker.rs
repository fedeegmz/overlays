use tokio::sync::watch;

use crate::application::ports::SessionRevoker;

/// Revocation epoch shared with every remote WebSocket task. Bumping it wakes
/// each subscriber, which then closes its socket (code 4001).
pub struct WatchSessionRevoker {
    tx: watch::Sender<u64>,
}

impl WatchSessionRevoker {
    pub fn new() -> Self {
        let (tx, _rx) = watch::channel(0);
        Self { tx }
    }

    /// A receiver that has already seen the current epoch, so only revocations
    /// that happen after subscribing wake it.
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.tx.subscribe()
    }
}

impl Default for WatchSessionRevoker {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRevoker for WatchSessionRevoker {
    fn revoke_all(&self) {
        // `send_modify` bumps the epoch even when nobody is subscribed.
        self.tx.send_modify(|epoch| *epoch += 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn revoke_wakes_existing_subscribers_only() {
        let revoker = WatchSessionRevoker::new();
        let mut early = revoker.subscribe();
        revoker.revoke_all();
        early.changed().await.unwrap();
        assert_eq!(*early.borrow(), 1);

        let mut late = revoker.subscribe();
        assert!(!late.has_changed().unwrap());
        revoker.revoke_all();
        late.changed().await.unwrap();
        assert_eq!(*late.borrow(), 2);
    }

    #[test]
    fn revoke_without_subscribers_does_not_panic() {
        WatchSessionRevoker::new().revoke_all();
    }
}
