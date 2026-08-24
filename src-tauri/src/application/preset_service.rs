use std::collections::HashMap;
use std::sync::Arc;

use super::ports::PresetRepository;
use crate::domain::error::DomainResult;
use crate::domain::preset::Preset;

pub struct PresetService {
    repo: Arc<dyn PresetRepository>,
}

impl PresetService {
    pub fn new(repo: Arc<dyn PresetRepository>) -> Self {
        Self { repo }
    }

    pub fn list(&self) -> Vec<Preset> {
        self.repo.load()
    }

    pub fn save(
        &self,
        name: String,
        template: String,
        fields: HashMap<String, String>,
    ) -> DomainResult<Vec<Preset>> {
        let preset = Preset::new(name, template, fields)?;
        let mut presets = self.repo.load();
        match presets.iter_mut().find(|p| p.name == preset.name) {
            Some(existing) => *existing = preset,
            None => presets.push(preset),
        }
        self.repo.save(&presets)?;
        Ok(presets)
    }

    pub fn delete(&self, name: &str) -> DomainResult<Vec<Preset>> {
        let mut presets = self.repo.load();
        presets.retain(|p| p.name != name);
        self.repo.save(&presets)?;
        Ok(presets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct InMemoryRepo(Mutex<Vec<Preset>>);

    impl InMemoryRepo {
        fn new() -> Self {
            Self(Mutex::new(Vec::new()))
        }
    }

    impl PresetRepository for InMemoryRepo {
        fn load(&self) -> Vec<Preset> {
            self.0.lock().unwrap().clone()
        }

        fn save(&self, presets: &[Preset]) -> DomainResult<()> {
            *self.0.lock().unwrap() = presets.to_vec();
            Ok(())
        }
    }

    fn service() -> PresetService {
        PresetService::new(Arc::new(InMemoryRepo::new()))
    }

    #[test]
    fn save_rejects_empty_name() {
        let svc = service();
        assert!(svc.save("   ".into(), "t".into(), HashMap::new()).is_err());
    }

    #[test]
    fn save_upserts_by_name() {
        let svc = service();
        svc.save("Fede".into(), "t1".into(), HashMap::new()).unwrap();
        svc.save("Fede".into(), "t2".into(), HashMap::new()).unwrap();

        let presets = svc.list();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].template, "t2");
    }

    #[test]
    fn delete_removes_matching_name() {
        let svc = service();
        svc.save("A".into(), "t".into(), HashMap::new()).unwrap();
        svc.save("B".into(), "t".into(), HashMap::new()).unwrap();

        let remaining = svc.delete("A").unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "B");
    }
}
