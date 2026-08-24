use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverlayAction {
    Show,
    Update,
    Hide,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayPayload {
    pub instance_id: String,
    pub template: String,
    pub action: OverlayAction,
    pub fields: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_serializes_to_lowercase() {
        let json = serde_json::to_value(OverlayAction::Show).unwrap();
        assert_eq!(json, serde_json::json!("show"));
    }

    #[test]
    fn action_deserializes_from_lowercase() {
        let action: OverlayAction = serde_json::from_value(serde_json::json!("hide")).unwrap();
        assert_eq!(action, OverlayAction::Hide);
    }

    #[test]
    fn action_rejects_unknown_value() {
        assert!(serde_json::from_value::<OverlayAction>(serde_json::json!("flash")).is_err());
    }
}
