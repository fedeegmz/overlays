use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::error::{DomainError, DomainResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupPresetEntry {
    pub template: String,
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupPreset {
    pub name: String,
    pub entries: Vec<GroupPresetEntry>,
}

impl GroupPreset {
    pub fn new(name: impl Into<String>, entries: Vec<GroupPresetEntry>) -> DomainResult<Self> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(DomainError::GroupPresetEmptyName);
        }
        if entries.is_empty() {
            return Err(DomainError::GroupPresetNoEntries);
        }
        if let Some(index) = entries.iter().position(|e| e.template.trim().is_empty()) {
            return Err(DomainError::GroupPresetInvalidEntry { index });
        }
        Ok(Self { name, entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(template: &str) -> GroupPresetEntry {
        GroupPresetEntry {
            template: template.to_string(),
            fields: HashMap::new(),
        }
    }

    #[test]
    fn trims_name_on_creation() {
        let group = GroupPreset::new("  Show  ", vec![entry("a")]).unwrap();
        assert_eq!(group.name, "Show");
    }

    #[test]
    fn rejects_empty_name() {
        assert!(matches!(
            GroupPreset::new("   ", vec![entry("a")]),
            Err(DomainError::GroupPresetEmptyName)
        ));
    }

    #[test]
    fn rejects_zero_entries() {
        assert!(matches!(
            GroupPreset::new("Show", vec![]),
            Err(DomainError::GroupPresetNoEntries)
        ));
    }

    #[test]
    fn rejects_entry_with_empty_template() {
        assert!(matches!(
            GroupPreset::new("Show", vec![entry("a"), entry("  ")]),
            Err(DomainError::GroupPresetInvalidEntry { index: 1 })
        ));
    }

    #[test]
    fn keeps_entry_order_and_duplicates() {
        let group = GroupPreset::new("Show", vec![entry("a"), entry("a"), entry("b")]).unwrap();
        let templates: Vec<_> = group.entries.iter().map(|e| e.template.as_str()).collect();
        assert_eq!(templates, ["a", "a", "b"]);
    }
}
