use serde::Serialize;
use std::collections::HashMap;

use super::template::{Manifest, TemplateInfo};

/// A live overlay instance owned by the registry.
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub id: String,
    pub template_id: String,
    pub number: u32,
    pub custom_name: Option<String>,
    pub fields: HashMap<String, String>,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Desktop,
    Remote,
}

/// Who caused a registry change; lets a client recognize its own echo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventSource {
    pub kind: SourceKind,
    pub client_id: String,
}

impl EventSource {
    pub fn desktop(client_id: impl Into<String>) -> Self {
        Self {
            kind: SourceKind::Desktop,
            client_id: client_id.into(),
        }
    }

    pub fn remote(client_id: impl Into<String>) -> Self {
        Self {
            kind: SourceKind::Remote,
            client_id: client_id.into(),
        }
    }
}

/// Wire form of an `Instance`, with the derived display name.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceDto {
    pub id: String,
    pub template_id: String,
    pub number: u32,
    pub custom_name: Option<String>,
    pub display_name: String,
    pub fields: HashMap<String, String>,
    pub visible: bool,
}

/// Full registry state published on every mutation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancesSnapshot {
    pub revision: u64,
    pub source: EventSource,
    pub instances: Vec<InstanceDto>,
}

/// Initial field values of a template: progress falls back to its `min`
/// (or 0), booleans to `"false"`, other fields only when a default exists.
pub fn defaults_for(template: &TemplateInfo) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    for field in &template.fields {
        match field.field_type.as_str() {
            "progress" => {
                let value = field
                    .default
                    .clone()
                    .unwrap_or_else(|| field.min.unwrap_or(0.0).to_string());
                fields.insert(field.key.clone(), value);
            }
            "boolean" => {
                let value = field.default.clone().unwrap_or_else(|| "false".into());
                fields.insert(field.key.clone(), value);
            }
            _ => {
                if let Some(default) = &field.default {
                    fields.insert(field.key.clone(), default.clone());
                }
            }
        }
    }
    fields
}

/// The instance fields keyed by every field the template declares, with
/// `""` for the ones never set. This is what overlays receive.
pub fn fields_for(instance: &Instance, template: &TemplateInfo) -> HashMap<String, String> {
    template
        .fields
        .iter()
        .map(|f| {
            (
                f.key.clone(),
                instance.fields.get(&f.key).cloned().unwrap_or_default(),
            )
        })
        .collect()
}

/// Ordered in-memory set of live instances.
#[derive(Debug, Default)]
pub struct Registry {
    instances: Vec<Instance>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    pub fn get(&self, id: &str) -> Option<&Instance> {
        self.instances.iter().find(|i| i.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Instance> {
        self.instances.iter_mut().find(|i| i.id == id)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    pub fn push(&mut self, instance: Instance) {
        self.instances.push(instance);
    }

    pub fn remove(&mut self, id: &str) -> Option<Instance> {
        let index = self.instances.iter().position(|i| i.id == id)?;
        Some(self.instances.remove(index))
    }

    /// Smallest positive number not used by another instance of the template.
    pub fn next_number(&self, template_id: &str) -> u32 {
        let mut candidate = 1;
        while self
            .instances
            .iter()
            .any(|i| i.template_id == template_id && i.number == candidate)
        {
            candidate += 1;
        }
        candidate
    }

    /// Moves `from` to `to` (clamped to the last index). Returns whether the
    /// order changed; an out-of-range `from` or an identical target is a no-op.
    pub fn move_to(&mut self, from: usize, to: usize) -> bool {
        if from >= self.instances.len() {
            return false;
        }
        let to = to.min(self.instances.len() - 1);
        if from == to {
            return false;
        }
        let moved = self.instances.remove(from);
        self.instances.insert(to, moved);
        true
    }

    /// The custom name, or the template name (id when the template is not in
    /// the catalog), suffixed with `#number` when several instances share it.
    pub fn display_name(&self, instance: &Instance, manifest: &Manifest) -> String {
        if let Some(name) = &instance.custom_name {
            return name.clone();
        }
        let base = manifest
            .templates
            .iter()
            .find(|t| t.id == instance.template_id)
            .map(|t| t.name.as_str())
            .unwrap_or(&instance.template_id);
        let same_template = self
            .instances
            .iter()
            .filter(|i| i.template_id == instance.template_id)
            .count();
        if same_template <= 1 {
            base.to_string()
        } else {
            format!("{base} #{}", instance.number)
        }
    }

    pub fn dtos(&self, manifest: &Manifest) -> Vec<InstanceDto> {
        self.instances
            .iter()
            .map(|i| InstanceDto {
                id: i.id.clone(),
                template_id: i.template_id.clone(),
                number: i.number,
                custom_name: i.custom_name.clone(),
                display_name: self.display_name(i, manifest),
                fields: i.fields.clone(),
                visible: i.visible,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::template::OverlayField;

    fn field(key: &str, field_type: &str, default: Option<&str>, min: Option<f64>) -> OverlayField {
        OverlayField {
            key: key.into(),
            label: key.into(),
            field_type: field_type.into(),
            default: default.map(str::to_string),
            min,
            max: None,
            step: None,
            accept: None,
        }
    }

    fn template(fields: Vec<OverlayField>) -> TemplateInfo {
        TemplateInfo {
            id: "t".into(),
            name: "Title".into(),
            path: "t/index.html".into(),
            folder: String::new(),
            fields,
        }
    }

    fn instance(id: &str, template_id: &str, number: u32) -> Instance {
        Instance {
            id: id.into(),
            template_id: template_id.into(),
            number,
            custom_name: None,
            fields: HashMap::new(),
            visible: false,
        }
    }

    #[test]
    fn defaults_progress_uses_default_then_min_then_zero() {
        let t = template(vec![
            field("a", "progress", Some("40"), Some(10.0)),
            field("b", "progress", None, Some(5.0)),
            field("c", "progress", None, None),
            field("d", "progress", None, Some(1.5)),
        ]);
        let d = defaults_for(&t);
        assert_eq!(d["a"], "40");
        assert_eq!(d["b"], "5");
        assert_eq!(d["c"], "0");
        assert_eq!(d["d"], "1.5");
    }

    #[test]
    fn defaults_boolean_falls_back_to_false_and_other_types_need_a_default() {
        let t = template(vec![
            field("on", "boolean", None, None),
            field("keep", "boolean", Some("true"), None),
            field("text", "text", Some("hi"), None),
            field("empty", "text", None, None),
        ]);
        let d = defaults_for(&t);
        assert_eq!(d["on"], "false");
        assert_eq!(d["keep"], "true");
        assert_eq!(d["text"], "hi");
        assert!(!d.contains_key("empty"));
    }

    #[test]
    fn fields_for_covers_every_declared_key() {
        let t = template(vec![
            field("a", "text", None, None),
            field("b", "text", None, None),
        ]);
        let mut i = instance("x", "t", 1);
        i.fields.insert("a".into(), "1".into());
        i.fields.insert("stale".into(), "z".into());
        let f = fields_for(&i, &t);
        assert_eq!(f.len(), 2);
        assert_eq!(f["a"], "1");
        assert_eq!(f["b"], "");
    }

    #[test]
    fn next_number_fills_gaps_per_template() {
        let mut r = Registry::new();
        r.push(instance("a", "t", 1));
        r.push(instance("b", "t", 3));
        r.push(instance("c", "other", 1));
        assert_eq!(r.next_number("t"), 2);
        assert_eq!(r.next_number("other"), 2);
        assert_eq!(r.next_number("fresh"), 1);
    }

    #[test]
    fn move_to_clamps_target_and_ignores_invalid_source() {
        let mut r = Registry::new();
        for id in ["a", "b", "c"] {
            r.push(instance(id, "t", 1));
        }
        let ids = |r: &Registry| {
            r.instances()
                .iter()
                .map(|i| i.id.clone())
                .collect::<Vec<_>>()
        };

        assert!(r.move_to(2, 0));
        assert_eq!(ids(&r), ["c", "a", "b"]);
        assert!(r.move_to(0, 99));
        assert_eq!(ids(&r), ["a", "b", "c"]);
        assert!(!r.move_to(5, 0));
        assert!(!r.move_to(1, 1));
        assert_eq!(ids(&r), ["a", "b", "c"]);
    }

    #[test]
    fn display_name_numbers_only_when_template_is_shared() {
        let manifest = Manifest {
            templates: vec![template(vec![])],
            folders: vec![],
        };
        let mut r = Registry::new();
        r.push(instance("a", "t", 1));
        r.push(instance("z", "gone", 1));
        assert_eq!(r.display_name(r.get("a").unwrap(), &manifest), "Title");
        assert_eq!(r.display_name(r.get("z").unwrap(), &manifest), "gone");

        r.push(instance("b", "t", 2));
        assert_eq!(r.display_name(r.get("b").unwrap(), &manifest), "Title #2");

        r.get_mut("b").unwrap().custom_name = Some("Mine".into());
        assert_eq!(r.display_name(r.get("b").unwrap(), &manifest), "Mine");
    }

    #[test]
    fn snapshot_serializes_camel_case() {
        let snap = InstancesSnapshot {
            revision: 3,
            source: EventSource::remote("c1"),
            instances: vec![],
        };
        assert_eq!(
            serde_json::to_value(snap).unwrap(),
            serde_json::json!({
                "revision": 3,
                "source": { "kind": "remote", "clientId": "c1" },
                "instances": []
            })
        );
    }
}
