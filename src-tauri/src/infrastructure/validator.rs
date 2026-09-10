//! Deterministic validation of AI-generated overlay output against the
//! template contract in `prompt-base-overlays.md`. Every check is a plain
//! string/parse rule — no heuristics, no AI. Failing checks are collected
//! into the returned `ValidationReport` so the UI can show all issues at once.
//!
//! Fail-closed stance (D8): anything ambiguous is an error, never a silent
//! pass. In particular the `TEMPLATE_ID` in `script.js` MUST match the
//! template directory name exactly, and `overlay.json` MUST NOT carry a
//! top-level `id` field (the directory name is the identifier).

//!
use std::collections::HashSet;

use serde_json::Value;

use crate::domain::ai::{GeneratedFiles, ValidationReport};

/// Known field types accepted in `overlay.json` `fields[]`.
const FIELD_TYPES: &[&str] = &["text", "number", "color", "image", "select", "boolean"];

/// Validate the four generated files against the overlay contract.
///
/// `template_id` is the KEBAB-CASE directory name the overlay will live in —
/// it is matched against `TEMPLATE_ID` in `script.js` exactly (D8).
pub fn validate_generated_overlay(files: &GeneratedFiles, template_id: &str) -> ValidationReport {
    let mut errors: Vec<String> = Vec::new();

    for (path, content) in [
        ("overlay.json", files.overlay_json.as_str()),
        ("index.html", files.index_html.as_str()),
        ("style.css", files.style_css.as_str()),
        ("script.js", files.script_js.as_str()),
    ] {
        if content.trim().is_empty() {
            errors.push(format!("{path} is empty"));
        }
    }

    validate_manifest(&files.overlay_json, &mut errors);
    validate_script(&files.script_js, template_id, &mut errors);
    validate_style(&files.style_css, &mut errors);
    validate_markup(&files.index_html, &mut errors);

    if errors.is_empty() {
        ValidationReport::valid()
    } else {
        ValidationReport::invalid(errors)
    }
}

fn validate_manifest(overlay_json: &str, errors: &mut Vec<String>) {
    let parsed: Value = match serde_json::from_str(overlay_json) {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("overlay.json is not valid JSON: {e}"));
            return;
        }
    };
    let obj = match parsed.as_object() {
        Some(obj) => obj,
        None => {
            errors.push("overlay.json must be a JSON object".into());
            return;
        }
    };

    if obj.contains_key("id") {
        errors.push("overlay.json must not include a top-level 'id' field (the directory name is the identifier)".into());
    }

    match obj.get("name") {
        Some(Value::String(name)) if !name.trim().is_empty() => {}
        _ => errors.push("overlay.json must have a non-empty 'name' string".into()),
    }

    match obj.get("fields") {
        Some(Value::Array(fields)) => {
            let mut seen: HashSet<&str> = HashSet::new();
            for (i, field) in fields.iter().enumerate() {
                let Some(fobj) = field.as_object() else {
                    errors.push(format!("overlay.json fields[{i}] is not an object"));
                    continue;
                };
                for required in ["key", "label", "type"] {
                    if !fobj.contains_key(required) {
                        errors.push(format!("overlay.json fields[{i}] is missing '{required}'"));
                    }
                }
                if let Some(k) = fobj.get("key").and_then(Value::as_str) {
                    if k.trim().is_empty() {
                        errors.push(format!("overlay.json fields[{i}] has an empty 'key'"));
                    } else if !seen.insert(k) {
                        errors.push(format!("overlay.json fields[{i}] duplicates key '{k}'"));
                    }
                }
                if let Some(t) = fobj.get("type").and_then(Value::as_str) {
                    if !FIELD_TYPES.contains(&t) {
                        errors.push(format!("overlay.json fields[{i}] has unknown type '{t}'"));
                    }
                }
            }
        }
        Some(_) => errors.push("overlay.json 'fields' must be an array".into()),
        None => errors.push("overlay.json must have a 'fields' array".into()),
    }
}

fn validate_script(script_js: &str, template_id: &str, errors: &mut Vec<String>) {
    if !script_js.contains(&format!("TEMPLATE_ID = \"{template_id}\"")) {
        errors.push(format!(
            "script.js must declare `const TEMPLATE_ID = \"{template_id}\"` exactly"
        ));
    }
    for (name, signature) in [
        ("show", "function show("),
        ("update", "function update("),
        ("hide", "function hide("),
    ] {
        if !script_js.contains(signature) {
            errors.push(format!("script.js must define the '{name}(...)' handler"));
        }
    }
    if !script_js.contains("instance_id") {
        errors.push(
            "script.js must filter WebSocket messages by instance (compare msg.instance_id)".into(),
        );
    }
    for needle in ["WebSocket", "onclose", "setTimeout"] {
        if !script_js.contains(needle) {
            errors.push(format!(
                "script.js must handle reconnection (missing '{needle}')"
            ));
        }
    }
}

fn validate_style(style_css: &str, errors: &mut Vec<String>) {
    if !style_css.contains("transparent") {
        errors.push(
            "style.css must keep the body background transparent (OBS Browser Source)".into(),
        );
    }
}

fn validate_markup(index_html: &str, errors: &mut Vec<String>) {
    for reference in ["style.css", "script.js"] {
        if !index_html.contains(reference) {
            errors.push(format!("index.html must reference local '{reference}'"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_files(template_id: &str) -> GeneratedFiles {
        GeneratedFiles {
            overlay_json: r#"{"name":"Mi Overlay","fields":[{"key":"titulo","label":"Titulo","type":"text","default":"Hola"}]}"#.into(),
            index_html: r#"<!doctype html><link rel="stylesheet" href="style.css"><div id="root"></div><script src="script.js"></script>"#.into(),
            style_css: "body { background: transparent; font-family: sans-serif; }".into(),
            script_js: valid_script(template_id),
        }
    }

    fn valid_script(template_id: &str) -> String {
        format!(
            r#"const TEMPLATE_ID = "{template_id}";
const INSTANCE_ID = new URLSearchParams(window.location.search).get("instance");
let ws;
function connect() {{
  ws = new WebSocket(`ws://${{location.host}}/ws`);
  ws.onmessage = (event) => {{
    const msg = JSON.parse(event.data);
    if (msg.template !== TEMPLATE_ID || (INSTANCE_ID && msg.instance_id !== INSTANCE_ID)) return;
    if (msg.action === "show") show(msg.fields);
    if (msg.action === "update") update(msg.fields);
    if (msg.action === "hide") hide();
  }};
  ws.onclose = () => setTimeout(connect, 2000);
}}
connect();
function show(fields) {{}}
function update(fields) {{}}
function hide() {{}}
"#
        )
    }

    #[test]
    fn accepts_a_fully_conforming_overlay() {
        let files = valid_files("mi-overlay");
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(report.valid, "expected valid: {:?}", report.errors);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn rejects_manifest_with_top_level_id() {
        let mut files = valid_files("mi-overlay");
        files.overlay_json = files
            .overlay_json
            .replace(r#"{"name""#, r#"{"id":"mi-overlay","name""#);
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(report.errors.iter().any(|e| e.contains("'id'")));
    }

    #[test]
    fn rejects_missing_fields_array() {
        let mut files = valid_files("mi-overlay");
        files.overlay_json = r#"{"name":"Solo nombre"}"#.into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(report.errors.iter().any(|e| e.contains("'fields'")));
    }

    #[test]
    fn rejects_duplicate_field_keys_and_unknown_types() {
        let mut files = valid_files("mi-overlay");
        files.overlay_json = r#"{"name":"X","fields":[
            {"key":"a","label":"A","type":"text"},
            {"key":"a","label":"B","type":"text"},
            {"key":"b","label":"B","type":"hologram"}
        ]}"#
        .into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(report.errors.iter().any(|e| e.contains("duplicates key")));
        assert!(report.errors.iter().any(|e| e.contains("unknown type")));
    }

    #[test]
    fn rejects_wrong_template_id_exactly() {
        let files = valid_files("mi-other-overlay");
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(report.errors.iter().any(|e| e.contains("TEMPLATE_ID")));
    }

    #[test]
    fn rejects_missing_handlers_and_reconnect() {
        let mut files = valid_files("mi-overlay");
        files.script_js = files.script_js.replace("function hide() {}", "");
        files.script_js = files.script_js.replace("setTimeout", "setInterval");
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(report.errors.iter().any(|e| e.contains("'hide(")));
        assert!(report.errors.iter().any(|e| e.contains("reconnection")));
    }

    #[test]
    fn rejects_opaque_body() {
        let mut files = valid_files("mi-overlay");
        files.style_css = "body { background: #000000; }".into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(report.errors.iter().any(|e| e.contains("transparent")));
    }

    #[test]
    fn rejects_missing_local_references_and_empty_files() {
        let mut files = valid_files("mi-overlay");
        files.index_html = "<!doctype html><div>nope</div>".into();
        files.style_css = "".into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(report.errors.iter().any(|e| e.contains("style.css")));
        assert!(report
            .errors
            .iter()
            .any(|e| e.contains("style.css is empty")));
    }

    #[test]
    fn reports_all_problems_at_once() {
        let files = GeneratedFiles {
            overlay_json: "{not json".into(),
            index_html: "".into(),
            style_css: "body{}".into(),
            script_js: "".into(),
        };
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(report.errors.len() >= 6);
    }
}
