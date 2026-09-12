use std::sync::Arc;

use crate::application::ports::{ConfigRepository, KeyStore};
use crate::domain::ai::{ApiKeyPresence, ProviderKind};
use crate::domain::error::{DomainError, DomainResult};

/// Last 4 ASCII alphanumeric chars of a secret, for display only.
fn last4(secret: &str) -> String {
    let alnum: Vec<char> = secret
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let len = alnum.len();
    alnum[len.saturating_sub(4)..].iter().collect()
}

pub struct KeyService {
    config: Arc<dyn ConfigRepository>,
    keystore: Arc<dyn KeyStore>,
}

impl KeyService {
    pub fn new(config: Arc<dyn ConfigRepository>, keystore: Arc<dyn KeyStore>) -> Self {
        Self { config, keystore }
    }

    /// Presence metadata only — secrets never leave the keyring (K1).
    pub fn list(&self) -> Vec<ApiKeyPresence> {
        self.config.load().provider_keys
    }

    /// Store the secret in the keyring, then persist presence metadata.
    /// If the metadata save fails, the keyring entry is rolled back (D13).
    pub fn add(&self, provider: &str, secret: &str) -> DomainResult<Vec<ApiKeyPresence>> {
        if !ProviderKind::ALL.contains(&provider) {
            return Err(DomainError::UnknownProvider);
        }
        if secret.trim().is_empty() {
            return Err(DomainError::KeyringEmptySecret);
        }

        // 1. Secret goes to the keyring first (D13 order: set → upsert → save).
        self.keystore.set(provider, secret)?;

        // 2. Presence metadata goes to config.
        let mut config = self.config.load();
        let presence = ApiKeyPresence {
            provider: provider.to_string(),
            configured: true,
            last4: Some(last4(secret)),
        };
        match config
            .provider_keys
            .iter_mut()
            .find(|p| p.provider == provider)
        {
            Some(existing) => *existing = presence,
            None => config.provider_keys.push(presence),
        }

        // 3. Persist; on failure compensate by deleting the keyring entry.
        if let Err(e) = self.config.save(&config) {
            let _ = self.keystore.delete(provider);
            return Err(e);
        }
        Ok(config.provider_keys)
    }

    /// Remove presence metadata first, then delete the keyring entry (K2).
    /// A failed keyring delete leaves the metadata removed and surfaces the
    /// error — the orphaned secret stays unreachable by the app (K1).
    pub fn delete(&self, provider: &str) -> DomainResult<Vec<ApiKeyPresence>> {
        if !ProviderKind::ALL.contains(&provider) {
            return Err(DomainError::UnknownProvider);
        }
        let mut config = self.config.load();
        config.provider_keys.retain(|p| p.provider != provider);
        self.config.save(&config)?;
        self.keystore.delete(provider)?;
        Ok(config.provider_keys)
    }

    /// Independent keyring availability probe: distinct from the provider
    /// list, so "no keys configured" and "keyring unavailable" are separate
    /// UI states (G2). Reads a throwaway probe id — never a real provider.
    pub fn keyring_available(&self) -> bool {
        match self
            .keystore
            .get(&crate::domain::ai::keyring_probe_provider_id())
        {
            Ok(_) => true,
            Err(DomainError::KeyringUnavailable) => false,
            // A missing probe entry, or any other store error, means the
            // keyring itself responded — it is reachable.
            Err(_) => true,
        }
    }

    /// Read the raw secret for a provider (used only by the generation
    /// service; the secret goes straight from the keyring to the AI call).
    pub fn secret(&self, provider: &str) -> DomainResult<String> {
        if !ProviderKind::ALL.contains(&provider) {
            return Err(DomainError::UnknownProvider);
        }
        self.keystore.get(provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::test_utils::{MemoryConfigRepo, MemoryKeyStore};
    use crate::domain::ai::ApiKeyPresence;

    fn service() -> (KeyService, Arc<MemoryKeyStore>, Arc<MemoryConfigRepo>) {
        let keystore = Arc::new(MemoryKeyStore::new());
        let config_repo = Arc::new(MemoryConfigRepo::new());
        let service = KeyService::new(config_repo.clone(), keystore.clone());
        (service, keystore, config_repo)
    }

    const SECRET: &str = "sk-ant-1234abcd";

    #[test]
    fn add_stores_secret_in_keyring_and_metadata_only_in_config() {
        let (service, keystore, _) = service();
        let list = service.add("anthropic", SECRET).unwrap();

        assert_eq!(keystore.get("anthropic").unwrap(), SECRET);
        assert_eq!(list.len(), 1);
        let presence = &list[0];
        assert!(presence.configured);
        assert_eq!(presence.last4.as_deref(), Some("abcd"));
        // The full secret must never be serializable from presence metadata.
        let serialized = serde_json::to_string(&presence).unwrap();
        assert!(!serialized.contains(SECRET));
        assert!(!serialized.contains("1234abcd"));
    }

    #[test]
    fn list_returns_empty_when_nothing_configured() {
        let (service, _, _) = service();
        assert!(service.list().is_empty());
    }

    #[test]
    fn add_rejects_unknown_provider() {
        let (service, _, _) = service();
        assert!(matches!(
            service.add("openai", SECRET),
            Err(DomainError::UnknownProvider)
        ));
    }

    #[test]
    fn add_rejects_empty_secret() {
        let (service, _, _) = service();
        assert!(matches!(
            service.add("anthropic", "   "),
            Err(DomainError::KeyringEmptySecret)
        ));
    }

    #[test]
    fn add_compensates_keyring_when_metadata_save_fails() {
        let (service, keystore, config_repo) = service();
        config_repo.set_fail_on_save(true);

        let result = service.add("anthropic", SECRET);
        assert!(matches!(result, Err(DomainError::ConfigSaveFailed { .. })));
        // D13: the secret must be rolled back from the keyring.
        assert!(matches!(
            keystore.get("anthropic"),
            Err(DomainError::KeyringEntryMissing { .. })
        ));
    }

    #[test]
    fn delete_removes_metadata_and_secret() {
        let (service, keystore, _) = service();
        service.add("anthropic", SECRET).unwrap();

        let list = service.delete("anthropic").unwrap();
        assert!(list.is_empty());
        assert!(matches!(
            keystore.get("anthropic"),
            Err(DomainError::KeyringEntryMissing { .. })
        ));
    }

    #[test]
    fn delete_rejects_unknown_provider() {
        let (service, _, _) = service();
        assert!(matches!(
            service.delete("openai"),
            Err(DomainError::UnknownProvider)
        ));
    }

    #[test]
    fn delete_with_orphaned_metadata_surfaces_delete_failed() {
        let (service, _, config_repo) = service();
        // Presence metadata exists, but the keyring entry is gone (orphaned).
        let mut config = config_repo.load();
        config.provider_keys.push(ApiKeyPresence {
            provider: "anthropic".into(),
            configured: true,
            last4: Some("abcd".into()),
        });
        config_repo.save(&config).unwrap();

        let result = service.delete("anthropic");
        assert!(matches!(result, Err(DomainError::KeyringDeleteFailed)));
        // Metadata was already removed (K2 order: remove+save first).
        assert!(service.list().is_empty());
    }

    #[test]
    fn keyring_available_is_independent_of_provider_list() {
        let (service, _, _) = service();
        // Empty provider list, but the keyring store itself answers — the
        // keyring is available and the gate must not conflate the two.
        assert!(service.keyring_available());
        assert!(service.list().is_empty());
    }

    #[test]
    fn secret_surfaces_missing_entry_clearly() {
        let (service, _, _) = service();
        assert!(matches!(
            service.secret("anthropic"),
            Err(DomainError::KeyringEntryMissing { provider })
                if provider == "anthropic"
        ));
    }

    #[test]
    fn last4_extracts_alphanumeric_tail() {
        assert_eq!(last4("sk-ant-1234abcd"), "abcd");
        assert_eq!(last4("sk-123456"), "3456");
        assert_eq!(last4("abc"), "abc");
        assert_eq!(last4(""), "");
    }
}
