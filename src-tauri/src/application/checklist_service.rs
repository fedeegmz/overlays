use std::sync::Arc;

use super::ports::ChecklistRepository;
use crate::domain::checklist::{Checklist, ChecklistItem};
use crate::domain::error::{DomainError, DomainResult};

pub struct ChecklistService {
    repo: Arc<dyn ChecklistRepository>,
}

impl ChecklistService {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }

    pub fn list(&self) -> DomainResult<Vec<Checklist>> {
        self.repo.load()
    }

    /// Upserts by id: an existing list keeps its position, a new one is appended.
    pub fn save(
        &self,
        id: String,
        name: String,
        items: Vec<ChecklistItem>,
    ) -> DomainResult<Vec<Checklist>> {
        let list = Checklist::new(id, name, items)?;
        // Load first: a corrupt store aborts here, before anything is written.
        let mut lists = self.repo.load()?;
        let wanted = list.name.to_lowercase();
        // The list being saved is excluded so a case-only rename of itself passes.
        if lists
            .iter()
            .any(|l| l.id != list.id && l.name.trim().to_lowercase() == wanted)
        {
            return Err(DomainError::ChecklistDuplicateName { name: list.name });
        }
        match lists.iter_mut().find(|l| l.id == list.id) {
            Some(existing) => *existing = list,
            None => lists.push(list),
        }
        self.repo.save(&lists)?;
        Ok(lists)
    }

    pub fn delete(&self, id: &str) -> DomainResult<Vec<Checklist>> {
        let mut lists = self.repo.load()?;
        let before = lists.len();
        lists.retain(|l| l.id != id);
        if lists.len() == before {
            return Err(DomainError::ChecklistNotFound { id: id.to_string() });
        }
        self.repo.save(&lists)?;
        Ok(lists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct InMemoryRepo {
        lists: Mutex<Vec<Checklist>>,
        fail_load: bool,
        saves: Mutex<usize>,
    }

    impl InMemoryRepo {
        fn new(fail_load: bool) -> Arc<Self> {
            Arc::new(Self {
                lists: Mutex::new(Vec::new()),
                fail_load,
                saves: Mutex::new(0),
            })
        }
    }

    impl ChecklistRepository for InMemoryRepo {
        fn load(&self) -> DomainResult<Vec<Checklist>> {
            if self.fail_load {
                return Err(DomainError::ChecklistLoadFailed {
                    detail: "corrupt".into(),
                });
            }
            Ok(self.lists.lock().unwrap().clone())
        }

        fn save(&self, checklists: &[Checklist]) -> DomainResult<()> {
            *self.saves.lock().unwrap() += 1;
            *self.lists.lock().unwrap() = checklists.to_vec();
            Ok(())
        }
    }

    fn item(id: &str, text: &str) -> ChecklistItem {
        ChecklistItem {
            id: id.to_string(),
            text: text.to_string(),
        }
    }

    fn service(repo: &Arc<InMemoryRepo>) -> ChecklistService {
        ChecklistService::new(repo.clone())
    }

    #[test]
    fn save_appends_new_list() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        svc.save("l1".into(), "A".into(), vec![item("i1", "x")])
            .unwrap();
        let lists = svc
            .save("l2".into(), "B".into(), vec![item("i2", "y")])
            .unwrap();

        let names: Vec<_> = lists.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["A", "B"]);
    }

    #[test]
    fn save_rename_by_id_keeps_position_and_replaces_items() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        svc.save("l1".into(), "A".into(), vec![item("i1", "x")])
            .unwrap();
        svc.save("l2".into(), "B".into(), vec![item("i2", "y")])
            .unwrap();

        let lists = svc
            .save(
                "l1".into(),
                "  Renamed ".into(),
                vec![item("i1", "x2"), item("i3", "z")],
            )
            .unwrap();

        assert_eq!(lists.len(), 2);
        assert_eq!(lists[0].id, "l1");
        assert_eq!(lists[0].name, "Renamed");
        assert_eq!(lists[0].items.len(), 2);
        assert_eq!(lists[0].items[0].text, "x2");
        assert_eq!(lists[1].name, "B");
    }

    #[test]
    fn save_rejects_duplicate_name_case_insensitively_without_writing() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        svc.save("l1".into(), "Pre-stream".into(), vec![item("i1", "x")])
            .unwrap();
        let before = *repo.saves.lock().unwrap();

        assert!(matches!(
            svc.save("l2".into(), " pre-STREAM ".into(), vec![item("i2", "y")]),
            Err(DomainError::ChecklistDuplicateName { name }) if name == "pre-STREAM"
        ));
        assert_eq!(*repo.saves.lock().unwrap(), before);
    }

    #[test]
    fn save_allows_case_only_rename_of_itself() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        svc.save("l1".into(), "Pre-stream".into(), vec![item("i1", "x")])
            .unwrap();

        let lists = svc
            .save("l1".into(), "PRE-STREAM".into(), vec![item("i1", "x")])
            .unwrap();
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].name, "PRE-STREAM");
    }

    #[test]
    fn save_keeps_duplicate_item_texts_in_order() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        let lists = svc
            .save(
                "l1".into(),
                "A".into(),
                vec![item("i1", "a"), item("i2", "a"), item("i3", "b")],
            )
            .unwrap();

        let texts: Vec<_> = lists[0].items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(texts, ["a", "a", "b"]);
    }

    #[test]
    fn save_rejects_invalid_list_without_writing() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        assert!(matches!(
            svc.save("l1".into(), "A".into(), vec![]),
            Err(DomainError::ChecklistNoItems)
        ));
        assert_eq!(*repo.saves.lock().unwrap(), 0);
    }

    #[test]
    fn delete_removes_existing_list() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        svc.save("l1".into(), "A".into(), vec![item("i1", "x")])
            .unwrap();
        svc.save("l2".into(), "B".into(), vec![item("i2", "y")])
            .unwrap();

        let remaining = svc.delete("l1").unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "l2");
    }

    #[test]
    fn delete_unknown_list_is_not_found_without_writing() {
        let repo = InMemoryRepo::new(false);
        let svc = service(&repo);
        svc.save("l1".into(), "A".into(), vec![item("i1", "x")])
            .unwrap();
        let before = *repo.saves.lock().unwrap();

        assert!(matches!(
            svc.delete("nope"),
            Err(DomainError::ChecklistNotFound { id }) if id == "nope"
        ));
        assert_eq!(*repo.saves.lock().unwrap(), before);
    }

    #[test]
    fn list_save_and_delete_fail_without_writing_when_load_fails() {
        let repo = InMemoryRepo::new(true);
        let svc = service(&repo);

        assert!(matches!(
            svc.list(),
            Err(DomainError::ChecklistLoadFailed { .. })
        ));
        assert!(matches!(
            svc.save("l1".into(), "A".into(), vec![item("i1", "x")]),
            Err(DomainError::ChecklistLoadFailed { .. })
        ));
        assert!(matches!(
            svc.delete("l1"),
            Err(DomainError::ChecklistLoadFailed { .. })
        ));
        assert_eq!(*repo.saves.lock().unwrap(), 0);
    }
}
