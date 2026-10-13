use std::sync::Arc;

use super::ports::GroupPresetRepository;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::group_preset::{GroupPreset, GroupPresetEntry};

pub struct GroupPresetService {
    repo: Arc<dyn GroupPresetRepository>,
}

impl GroupPresetService {
    pub fn new(repo: Arc<dyn GroupPresetRepository>) -> Self {
        Self { repo }
    }

    pub fn list(&self) -> DomainResult<Vec<GroupPreset>> {
        self.repo.load()
    }

    pub fn save(
        &self,
        name: String,
        entries: Vec<GroupPresetEntry>,
    ) -> DomainResult<Vec<GroupPreset>> {
        let group = GroupPreset::new(name, entries)?;
        // Load first: a corrupt store aborts here, before anything is written.
        let mut groups = self.repo.load()?;
        match groups.iter_mut().find(|g| g.name == group.name) {
            Some(existing) => *existing = group,
            None => groups.push(group),
        }
        self.repo.save(&groups)?;
        Ok(groups)
    }

    pub fn delete(&self, name: &str) -> DomainResult<Vec<GroupPreset>> {
        let mut groups = self.repo.load()?;
        let before = groups.len();
        groups.retain(|g| g.name != name);
        if groups.len() == before {
            return Err(DomainError::GroupPresetNotFound {
                name: name.to_string(),
            });
        }
        self.repo.save(&groups)?;
        Ok(groups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct InMemoryRepo {
        groups: Mutex<Vec<GroupPreset>>,
        fail_load: bool,
        saves: Mutex<usize>,
    }

    impl InMemoryRepo {
        fn new(fail_load: bool) -> Arc<Self> {
            Arc::new(Self {
                groups: Mutex::new(Vec::new()),
                fail_load,
                saves: Mutex::new(0),
            })
        }
    }

    impl GroupPresetRepository for InMemoryRepo {
        fn load(&self) -> DomainResult<Vec<GroupPreset>> {
            if self.fail_load {
                return Err(DomainError::GroupPresetLoadFailed {
                    detail: "corrupt".into(),
                });
            }
            Ok(self.groups.lock().unwrap().clone())
        }

        fn save(&self, groups: &[GroupPreset]) -> DomainResult<()> {
            *self.saves.lock().unwrap() += 1;
            *self.groups.lock().unwrap() = groups.to_vec();
            Ok(())
        }
    }

    fn entry(template: &str) -> GroupPresetEntry {
        GroupPresetEntry {
            template: template.to_string(),
            fields: HashMap::new(),
        }
    }

    fn service(repo: &Arc<InMemoryRepo>) -> GroupPresetService {
        GroupPresetService::new(repo.clone())
    }

    #[test]
    fn save_upserts_by_trimmed_name_replacing_entries() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        svc.save("Show".into(), vec![entry("a"), entry("b")])
            .unwrap();
        svc.save("  Show ".into(), vec![entry("c")]).unwrap();

        let groups = svc.list().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].entries.len(), 1);
        assert_eq!(groups[0].entries[0].template, "c");
    }

    #[test]
    fn save_keeps_duplicate_templates_in_order() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        let groups = svc
            .save("Show".into(), vec![entry("a"), entry("a"), entry("b")])
            .unwrap();

        let templates: Vec<_> = groups[0]
            .entries
            .iter()
            .map(|e| e.template.as_str())
            .collect();
        assert_eq!(templates, ["a", "a", "b"]);
    }

    #[test]
    fn save_rejects_invalid_group_without_writing() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        assert!(matches!(
            svc.save("Show".into(), vec![]),
            Err(DomainError::GroupPresetNoEntries)
        ));
        assert_eq!(*repo.saves.lock().unwrap(), 0);
    }

    #[test]
    fn delete_removes_existing_group() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        svc.save("A".into(), vec![entry("t")]).unwrap();
        svc.save("B".into(), vec![entry("t")]).unwrap();

        let remaining = svc.delete("A").unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "B");
    }

    #[test]
    fn delete_unknown_group_is_not_found_without_writing() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        svc.save("A".into(), vec![entry("t")]).unwrap();
        let before = *repo.saves.lock().unwrap();

        assert!(matches!(
            svc.delete("Nope"),
            Err(DomainError::GroupPresetNotFound { name }) if name == "Nope"
        ));
        assert_eq!(*repo.saves.lock().unwrap(), before);
    }

    #[test]
    fn list_save_and_delete_fail_without_writing_when_load_fails() {
        let repo = InMemoryRepo::new(true);
        let svc = service(&repo);

        assert!(matches!(
            svc.list(),
            Err(DomainError::GroupPresetLoadFailed { .. })
        ));
        assert!(matches!(
            svc.save("Show".into(), vec![entry("a")]),
            Err(DomainError::GroupPresetLoadFailed { .. })
        ));
        assert!(matches!(
            svc.delete("Show"),
            Err(DomainError::GroupPresetLoadFailed { .. })
        ));
        assert_eq!(*repo.saves.lock().unwrap(), 0);
    }
}
