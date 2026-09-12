use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::template::OverlayField;

/// AI provider identity (only Anthropic ships in this change; the port is
/// ready for more adapters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Anthropic,
}

impl ProviderKind {
    pub const ANTHROPIC: &'static str = "anthropic";
    pub const ALL: &'static [&'static str] = &[Self::ANTHROPIC];
}

/// A provider id guaranteed NOT to collide with a real configured provider:
/// used by keyring probes (availability checks, integration tests) so a probe
/// can never read or clobber a genuine key entry.
pub fn keyring_probe_provider_id() -> String {
    format!("probe-{}", std::process::id())
}

/// Presence-only key metadata. Carries NO secret — only this crosses IPC (K1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiKeyPresence {
    pub provider: String,
    pub configured: bool,
    pub last4: Option<String>,
}

/// The 4-file overlay contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFiles {
    pub overlay_json: String,
    pub index_html: String,
    pub style_css: String,
    pub script_js: String,
}

/// Normalized provider response (G5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedOverlay {
    pub directory: String,
    pub name: String,
    pub files: GeneratedFiles,
}

/// A single validation failure with a machine-readable `code` (i18n key:
/// `errors.generation.issue.<code>`) and interpolation `params`. Backend
/// layers never emit user-facing text — the UI renders the message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub params: HashMap<String, String>,
}

impl ValidationIssue {
    pub fn new(code: &str) -> Self {
        Self {
            code: code.to_string(),
            params: HashMap::new(),
        }
    }

    pub fn param(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.insert(key.to_string(), value.into());
        self
    }
}

/// Deterministic validation outcome.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn valid() -> Self {
        Self {
            valid: true,
            issues: Vec::new(),
        }
    }

    pub fn invalid(issues: Vec<ValidationIssue>) -> Self {
        Self {
            valid: false,
            issues,
        }
    }
}

/// Raw provider output before normalization (D3).
#[derive(Debug, Clone)]
pub struct AiText {
    pub text: String,
    pub truncated: bool,
}

/// Transport-level provider errors (mapped to `DomainError` by the service).
#[derive(Debug, Clone)]
pub enum AiError {
    Unauthorized,
    RateLimited,
    Timeout,
    Network {
        detail: String,
    },
    /// 5xx — the provider is reachable but unhealthy; distinct from a malformed
    /// 2xx payload (`InvalidResponse`).
    ServerError {
        detail: String,
    },
    InvalidResponse {
        detail: String,
    },
}

/// What crosses IPC after a generation: the staging id (so the UI can
/// accept/discard), the template identity, and the editable fields for the
/// preview panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedOverlaySummary {
    pub staging_id: String,
    pub directory: String,
    pub name: String,
    pub fields: Vec<OverlayField>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kind_serializes_lowercase() {
        assert_eq!(ProviderKind::ANTHROPIC, "anthropic");
        assert_eq!(
            serde_json::to_value(ProviderKind::Anthropic).unwrap(),
            serde_json::json!("anthropic")
        );
    }

    #[test]
    fn key_presence_serializes_only_metadata() {
        let presence = ApiKeyPresence {
            provider: "anthropic".into(),
            configured: true,
            last4: Some("1234".into()),
        };
        let json = serde_json::to_value(&presence).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "provider": "anthropic", "configured": true, "last4": "1234" })
        );
    }

    #[test]
    fn validation_report_constructors() {
        assert!(ValidationReport::valid().valid);
        assert!(ValidationReport::valid().issues.is_empty());
        let report = ValidationReport::invalid(vec![ValidationIssue::new("missing_hide")]);
        assert!(!report.valid);
        assert_eq!(report.issues[0].code, "missing_hide");
    }

    #[test]
    fn generated_overlay_round_trips() {
        let files = GeneratedFiles {
            overlay_json: r#"{"name":"Mi Overlay","fields":[]}"#.into(),
            index_html: "<html></html>".into(),
            style_css: "body { background: transparent; }".into(),
            script_js: "const TEMPLATE_ID = \"mi-overlay\";".into(),
        };
        let overlay = GeneratedOverlay {
            directory: "mi-overlay".into(),
            name: "Mi Overlay".into(),
            files: files.clone(),
        };
        let value = serde_json::to_value(&overlay).unwrap();
        assert_eq!(value["directory"], "mi-overlay");
        let back: GeneratedOverlay = serde_json::from_value(value).unwrap();
        assert_eq!(back.files.script_js, files.script_js);
        assert_eq!(back.name, "Mi Overlay");
    }
}
