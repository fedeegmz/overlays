use crate::application::ports::KeyStore;
use crate::domain::error::{DomainError, DomainResult};

/// Keyring service name — isolates app entries from other software.
pub const KEYRING_SERVICE: &str = "com.fedeegmz.overlays";

/// Which store operation failed — `NoEntry` means different things per op:
/// a missing entry on `get` is a plain "not configured" signal, while a
/// missing entry on `delete` is a genuine delete failure.
#[derive(Debug, Clone, Copy)]
enum KeyringOp {
    Get,
    Set,
    Delete,
}

/// Map a keyring crate error to the domain contract. Secrets never appear in
/// `detail` (K4) — only provider + variant-level info.
fn map_keyring_error(provider: &str, e: keyring::Error, op: KeyringOp) -> DomainError {
    match (op, e) {
        (_, keyring::Error::NoStorageAccess(_)) => DomainError::KeyringUnavailable,
        (KeyringOp::Get, keyring::Error::NoEntry) => DomainError::KeyringEntryMissing {
            provider: provider.into(),
        },
        (KeyringOp::Delete, keyring::Error::NoEntry) => DomainError::KeyringDeleteFailed,
        (_, other) => DomainError::KeyringFailed {
            detail: format!("{provider}: {other}"),
        },
    }
}

/// OS keyring-backed `KeyStore`. One entry per provider:
/// service = `com.fedeegmz.overlays`, user = provider id.
pub struct KeyringStore;

impl KeyringStore {
    fn entry(provider: &str) -> DomainResult<keyring::Entry> {
        keyring::Entry::new(KEYRING_SERVICE, provider)
            .map_err(|e| map_keyring_error(provider, e, KeyringOp::Get))
    }
}

impl KeyStore for KeyringStore {
    fn set(&self, provider: &str, secret: &str) -> DomainResult<()> {
        Self::entry(provider)?
            .set_password(secret)
            .map_err(|e| map_keyring_error(provider, e, KeyringOp::Set))
    }

    fn get(&self, provider: &str) -> DomainResult<String> {
        Self::entry(provider)?
            .get_password()
            .map_err(|e| map_keyring_error(provider, e, KeyringOp::Get))
    }

    fn delete(&self, provider: &str) -> DomainResult<()> {
        Self::entry(provider)?
            .delete_credential()
            .map_err(|e| map_keyring_error(provider, e, KeyringOp::Delete))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_no_storage_access_to_unavailable() {
        let platform = Box::new(std::io::Error::other("locked"));
        let err = map_keyring_error(
            "anthropic",
            keyring::Error::NoStorageAccess(platform),
            KeyringOp::Get,
        );
        assert!(matches!(err, DomainError::KeyringUnavailable));
    }

    #[test]
    fn maps_no_entry_on_get_to_entry_missing() {
        let err = map_keyring_error("anthropic", keyring::Error::NoEntry, KeyringOp::Get);
        assert!(matches!(
            err,
            DomainError::KeyringEntryMissing { provider } if provider == "anthropic"
        ));
    }

    #[test]
    fn maps_no_entry_on_delete_to_delete_failed() {
        let err = map_keyring_error("anthropic", keyring::Error::NoEntry, KeyringOp::Delete);
        assert!(matches!(err, DomainError::KeyringDeleteFailed));
    }

    #[test]
    fn maps_no_entry_on_set_to_keyring_failed() {
        // NoEntry while setting is unexpected — surface as a generic failure.
        let err = map_keyring_error("anthropic", keyring::Error::NoEntry, KeyringOp::Set);
        assert!(matches!(err, DomainError::KeyringFailed { .. }));
    }

    #[test]
    fn maps_other_errors_to_keyring_failed_without_secret() {
        let err = map_keyring_error(
            "anthropic",
            keyring::Error::TooLong("secret".into(), 64),
            KeyringOp::Get,
        );
        match err {
            DomainError::KeyringFailed { detail } => {
                assert!(!detail.contains("sk-"));
                assert!(detail.contains("anthropic"));
            }
            other => panic!("expected KeyringFailed, got {other:?}"),
        }
    }

    /// End-to-end probe against the real OS keyring. `cargo test -- --ignored`
    /// runs it — CI keeps the step wired so the gate exists even when there is
    /// no Secret Service (the probe then exits early).
    ///
    /// Uses a THROWAWAY provider id (`probe-<pid>`), never "anthropic": a
    /// failure here must never clobber a real configured key.
    #[test]
    #[ignore = "touches the real OS keyring; run via cargo test -- --ignored (CI)"]
    fn round_trip_against_secret_service() {
        let store = KeyringStore;
        let probe_secret = format!("probe-{}", std::process::id());
        let probe_provider = crate::domain::ai::keyring_probe_provider_id();
        match store.set(&probe_provider, &probe_secret) {
            Err(DomainError::KeyringUnavailable) => {
                return; // no Secret Service available — nothing to assert
            }
            Err(e) => panic!("unexpected keyring error: {e}"),
            Ok(()) => {}
        }

        assert_eq!(store.get(&probe_provider).unwrap(), probe_secret);
        store.delete(&probe_provider).unwrap();
        assert!(matches!(
            store.get(&probe_provider),
            Err(DomainError::KeyringEntryMissing { provider })
                if provider == probe_provider
        ));
    }
}
