use crate::application::ports::KeyStore;
use crate::domain::error::{DomainError, DomainResult};

/// Keyring service name — isolates app entries from other software.
pub const KEYRING_SERVICE: &str = "com.fedeegmz.overlays";

/// Map a keyring crate error to the domain contract. Secrets never appear in
/// `detail` (K4) — only provider + variant-level info.
fn map_keyring_error(provider: &str, e: keyring::Error) -> DomainError {
    match e {
        keyring::Error::NoStorageAccess(_) => DomainError::KeyringUnavailable,
        keyring::Error::NoEntry => DomainError::KeyringDeleteFailed,
        other => DomainError::KeyringFailed {
            detail: format!("{provider}: {other}"),
        },
    }
}

/// OS keyring-backed `KeyStore`. One entry per provider:
/// service = `com.fedeegmz.overlays`, user = provider id.
pub struct KeyringStore;

impl KeyringStore {
    fn entry(provider: &str) -> DomainResult<keyring::Entry> {
        keyring::Entry::new(KEYRING_SERVICE, provider).map_err(|e| map_keyring_error(provider, e))
    }
}

impl KeyStore for KeyringStore {
    fn set(&self, provider: &str, secret: &str) -> DomainResult<()> {
        Self::entry(provider)?
            .set_password(secret)
            .map_err(|e| map_keyring_error(provider, e))
    }

    fn get(&self, provider: &str) -> DomainResult<String> {
        Self::entry(provider)?
            .get_password()
            .map_err(|e| map_keyring_error(provider, e))
    }

    fn delete(&self, provider: &str) -> DomainResult<()> {
        Self::entry(provider)?
            .delete_credential()
            .map_err(|e| map_keyring_error(provider, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_no_storage_access_to_unavailable() {
        let platform = Box::new(std::io::Error::new(std::io::ErrorKind::Other, "locked"));
        let err = map_keyring_error("anthropic", keyring::Error::NoStorageAccess(platform));
        assert!(matches!(err, DomainError::KeyringUnavailable));
    }

    #[test]
    fn maps_no_entry_to_delete_failed() {
        let err = map_keyring_error("anthropic", keyring::Error::NoEntry);
        assert!(matches!(err, DomainError::KeyringDeleteFailed));
    }

    #[test]
    fn maps_other_errors_to_keyring_failed_without_secret() {
        let err = map_keyring_error("anthropic", keyring::Error::TooLong("secret".into(), 64));
        match err {
            DomainError::KeyringFailed { detail } => {
                assert!(!detail.contains("sk-"));
                assert!(detail.contains("anthropic"));
            }
            other => panic!("expected KeyringFailed, got {other:?}"),
        }
    }

    /// End-to-end probe against the real OS keyring. Skips (passes) when no
    /// Secret Service is reachable — on CI runners there is none; on a
    /// developer machine with gnome-keyring/kwallet this exercises the real
    /// store. Always cleans up its own entry.
    #[test]
    fn round_trip_against_secret_service() {
        let store = KeyringStore;
        let probe_secret = format!("probe-{}", std::process::id());
        match store.set("anthropic", &probe_secret) {
            Err(DomainError::KeyringUnavailable) => {
                return; // no Secret Service available — nothing to assert
            }
            Err(e) => panic!("unexpected keyring error: {e}"),
            Ok(()) => {}
        }

        assert_eq!(store.get("anthropic").unwrap(), probe_secret);
        store.delete("anthropic").unwrap();
        assert!(matches!(
            store.get("anthropic"),
            Err(DomainError::KeyringDeleteFailed)
        ));
    }
}
