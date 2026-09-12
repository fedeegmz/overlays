//! Shared test doubles for application-level tests (single source of truth).
//!
//! These used to be duplicated in `key_service.rs` and `generation_service.rs`
//! with subtly divergent semantics — missing-entry `get` returned different
//! errors in each copy. The canonical behavior lives here:
//!
//! - `get` on a missing entry → `DomainError::KeyringEntryMissing { provider }`
//! - `delete` on a missing entry → `DomainError::KeyringDeleteFailed`
//! - `set` always overwrites and succeeds
//!
//! `unique_temp_dir` also prevents parallel-test collisions: every call
//! returns a different directory, so no two tests can ever fight over a
//! shared temp path.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};

use crate::application::ports::{ConfigRepository, KeyStore};
use crate::domain::config::AppConfig;
use crate::domain::error::{DomainError, DomainResult};

pub struct MemoryKeyStore {
    entries: Mutex<HashMap<String, String>>,
}

impl Default for MemoryKeyStore {
    fn default() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl MemoryKeyStore {
    pub fn new() -> Self {
        Self::default()
    }
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
        self.entries.lock().unwrap().get(provider).cloned().ok_or(
            DomainError::KeyringEntryMissing {
                provider: provider.to_string(),
            },
        )
    }

    fn delete(&self, provider: &str) -> DomainResult<()> {
        match self.entries.lock().unwrap().remove(provider) {
            Some(_) => Ok(()),
            None => Err(DomainError::KeyringDeleteFailed),
        }
    }
}

pub struct MemoryConfigRepo {
    config: RwLock<AppConfig>,
    fail_on_save: AtomicBool,
}

impl Default for MemoryConfigRepo {
    fn default() -> Self {
        Self {
            config: RwLock::new(AppConfig::default()),
            fail_on_save: AtomicBool::new(false),
        }
    }
}

impl MemoryConfigRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_fail_on_save(&self, fail: bool) {
        self.fail_on_save.store(fail, Ordering::SeqCst);
    }
}

impl ConfigRepository for MemoryConfigRepo {
    fn load(&self) -> AppConfig {
        self.config.read().unwrap().clone()
    }

    fn save(&self, config: &AppConfig) -> DomainResult<()> {
        if self.fail_on_save.load(Ordering::SeqCst) {
            return Err(DomainError::ConfigSaveFailed {
                detail: "injected failure".into(),
            });
        }
        *self.config.write().unwrap() = config.clone();
        Ok(())
    }
}

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Unique temp directory per call — never shared between tests, so two tests
/// using the same tag can run in parallel without racing (AtomicU64 pattern,
/// same as overlay_writer.rs tests).
pub fn unique_temp_dir(tag: &str) -> PathBuf {
    let n = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("overlays-{tag}-{}-{n}", std::process::id()))
}
