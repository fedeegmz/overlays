use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::error::{DomainError, DomainResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupPresetEntry {
    pub template: String,
    pub fields: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
}

fn normalize(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupPreset {
    pub name: String,
    pub entries: Vec<GroupPresetEntry>,
}

impl GroupPreset {
    pub fn new(name: impl Into<String>, mut entries: Vec<GroupPresetEntry>) -> DomainResult<Self> {
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
        let mut seen = HashSet::new();
        for (index, entry) in entries.iter_mut().enumerate() {
            entry.instance_id = normalize(entry.instance_id.take());
            entry.custom_name = normalize(entry.custom_name.take());
            if let Some(id) = &entry.instance_id {
                if !seen.insert(id.clone()) {
                    return Err(DomainError::GroupPresetDuplicateInstanceId { index });
                }
            }
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
            instance_id: None,
            custom_name: None,
        }
    }

    fn identified(template: &str, id: Option<&str>, name: Option<&str>) -> GroupPresetEntry {
        GroupPresetEntry {
            instance_id: id.map(str::to_string),
            custom_name: name.map(str::to_string),
            ..entry(template)
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

    #[test]
    fn rejects_duplicate_instance_ids_with_index_of_second() {
        let entries = vec![
            identified("a", Some("x"), None),
            identified("b", Some("y"), None),
            identified("a", Some("x"), None),
        ];
        assert!(matches!(
            GroupPreset::new("Show", entries),
            Err(DomainError::GroupPresetDuplicateInstanceId { index: 2 })
        ));
    }

    #[test]
    fn allows_multiple_absent_instance_ids() {
        let entries = vec![entry("a"), entry("a"), identified("b", Some("  "), None)];
        assert!(GroupPreset::new("Show", entries).is_ok());
    }

    #[test]
    fn normalizes_blank_identity_and_trims_name() {
        let entries = vec![
            identified("a", Some("  "), Some("   ")),
            identified("b", Some(" id1 "), Some("  Main  ")),
        ];
        let group = GroupPreset::new("Show", entries).unwrap();
        assert_eq!(group.entries[0].instance_id, None);
        assert_eq!(group.entries[0].custom_name, None);
        assert_eq!(group.entries[1].instance_id.as_deref(), Some("id1"));
        assert_eq!(group.entries[1].custom_name.as_deref(), Some("Main"));
    }
}
