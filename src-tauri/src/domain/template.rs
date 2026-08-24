use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub fields: Vec<OverlayField>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Manifest {
    pub templates: Vec<TemplateInfo>,
}
