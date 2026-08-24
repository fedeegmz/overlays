use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::error::{DomainError, DomainResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub template: String,
    pub fields: HashMap<String, String>,
}

impl Preset {
    pub fn new(
        name: impl Into<String>,
        template: impl Into<String>,
        fields: HashMap<String, String>,
    ) -> DomainResult<Self> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(DomainError::PresetEmptyName);
        }
        Ok(Self {
            name,
            template: template.into(),
            fields,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_name_on_creation() {
        let preset = Preset::new("  Fede  ", "t", HashMap::new()).unwrap();
        assert_eq!(preset.name, "Fede");
    }

    #[test]
    fn rejects_empty_name() {
        assert!(matches!(
            Preset::new("   ", "t", HashMap::new()),
            Err(DomainError::PresetEmptyName)
        ));
    }
}
