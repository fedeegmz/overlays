//! Test-only helpers shared by infrastructure tests.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A unique temporary directory removed on drop.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "overlays-test-{}-{}-{n}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Sorted names of the entries directly inside `dir` (empty when it does not exist).
    pub fn list(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map(|rd| {
                rd.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// In-memory `ConfigRepository` for tests that must not touch the disk.
#[derive(Default)]
pub struct MemConfigRepo(std::sync::Mutex<crate::domain::config::AppConfig>);

impl crate::application::ports::ConfigRepository for MemConfigRepo {
    fn load(&self) -> crate::domain::config::AppConfig {
        self.0.lock().unwrap().clone()
    }

    fn save(
        &self,
        config: &crate::domain::config::AppConfig,
    ) -> crate::domain::error::DomainResult<()> {
        *self.0.lock().unwrap() = config.clone();
        Ok(())
    }
}

/// In-memory `PresetRepository` for tests.
#[derive(Default)]
pub struct MemPresetRepo(std::sync::Mutex<Vec<crate::domain::preset::Preset>>);

impl crate::application::ports::PresetRepository for MemPresetRepo {
    fn load(&self) -> Vec<crate::domain::preset::Preset> {
        self.0.lock().unwrap().clone()
    }

    fn save(
        &self,
        presets: &[crate::domain::preset::Preset],
    ) -> crate::domain::error::DomainResult<()> {
        *self.0.lock().unwrap() = presets.to_vec();
        Ok(())
    }
}

/// In-memory `GroupPresetRepository` for tests.
#[derive(Default)]
pub struct MemGroupPresetRepo(std::sync::Mutex<Vec<crate::domain::group_preset::GroupPreset>>);

impl crate::application::ports::GroupPresetRepository for MemGroupPresetRepo {
    fn load(
        &self,
    ) -> crate::domain::error::DomainResult<Vec<crate::domain::group_preset::GroupPreset>> {
        Ok(self.0.lock().unwrap().clone())
    }

    fn save(
        &self,
        groups: &[crate::domain::group_preset::GroupPreset],
    ) -> crate::domain::error::DomainResult<()> {
        *self.0.lock().unwrap() = groups.to_vec();
        Ok(())
    }
}
