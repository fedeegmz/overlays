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
            return Err(DomainError::KeyringFailed {
                detail: "secret is empty".into(),
            });
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

    /// Gate probe: is the keyring entry for this provider actually readable?
    pub fn probe(&self, provider: &str) -> bool {
        self.keystore.get(provider).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::AppConfig;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryKeyStore {
        entries: Mutex<HashMap<String, String>>,
    }

    impl KeyStore for MemoryKeyStore {
        fn set(&self, provider: &str, secret: &str) -> DomainResult<()> {
            self.entries
                .lock()
                .unwrap()
                .insert(provider.to_string(), secret.to_string());
            Ok(())
        }

        fn get(&self, provider: &str) -> DomainResult<String> {
            self.entries
                .lock()
                .unwrap()
                .get(provider)
                .cloned()
                .ok_or(DomainError::KeyringFailed {
                    detail: "no entry".into(),
                })
        }

        fn delete(&self, provider: &str) -> DomainResult<()> {
            match self.entries.lock().unwrap().remove(provider) {
                Some(_) => Ok(()),
                None => Err(DomainError::KeyringDeleteFailed),
            }
        }
    }

    struct MemoryConfigRepo {
        config: Mutex<AppConfig>,
        fail_on_save: AtomicBool,
    }

    impl Default for MemoryConfigRepo {
        fn default() -> Self {
            Self {
                config: Mutex::new(AppConfig::default()),
                fail_on_save: AtomicBool::new(false),
            }
        }
    }

    impl ConfigRepository for MemoryConfigRepo {
        fn load(&self) -> AppConfig {
            self.config.lock().unwrap().clone()
        }

        fn save(&self, config: &AppConfig) -> DomainResult<()> {
            if self.fail_on_save.load(Ordering::SeqCst) {
                return Err(DomainError::ConfigSaveFailed {
                    detail: "injected failure".into(),
                });
            }
            *self.config.lock().unwrap() = config.clone();
            Ok(())
        }
    }

    fn service() -> (KeyService, Arc<MemoryKeyStore>, Arc<MemoryConfigRepo>) {
        let keystore = Arc::new(MemoryKeyStore::default());
        let config_repo = Arc::new(MemoryConfigRepo::default());
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
            Err(DomainError::KeyringFailed { .. })
        ));
    }

    #[test]
    fn add_compensates_keyring_when_metadata_save_fails() {
        let (service, keystore, config_repo) = service();
        config_repo.fail_on_save.store(true, Ordering::SeqCst);

        let result = service.add("anthropic", SECRET);
        assert!(matches!(result, Err(DomainError::ConfigSaveFailed { .. })));
        // D13: the secret must be rolled back from the keyring.
        assert!(matches!(
            keystore.get("anthropic"),
            Err(DomainError::KeyringFailed { .. })
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
            Err(DomainError::KeyringFailed { .. })
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
        let (service, keystore, config_repo) = service();
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
        let _ = keystore;
    }

    #[test]
    fn probe_reflects_entry_readability() {
        let (service, keystore, _) = service();
        assert!(!service.probe("anthropic"));
        keystore.set("anthropic", SECRET).unwrap();
        assert!(service.probe("anthropic"));
    }

    #[test]
    fn last4_extracts_alphanumeric_tail() {
        assert_eq!(last4("sk-ant-1234abcd"), "abcd");
        assert_eq!(last4("sk-123456"), "3456");
        assert_eq!(last4("abc"), "abc");
        assert_eq!(last4(""), "");
    }
}
