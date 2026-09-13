//! Deterministic validation of AI-generated overlay output against the
//! template contract in `prompt-base-overlays.md`. Every check is a plain
//! string/scan rule — no heuristics, no AI. Failing checks are collected
//! into the returned `ValidationReport` (structured issue codes + params) so
//! the UI can show all problems at once, localized.
//!
//! Fail-closed stance (D8): anything ambiguous is an error, never a silent
//! pass. In particular the `TEMPLATE_ID` in `script.js` MUST match the
//! template directory name exactly, `overlay.json` MUST NOT carry a
//! top-level `id` field (the directory name is the identifier), and every
//! asset reference must be a RELATIVE local file — remote refs (scheme://,
//! //, absolute) are rejected (D15).

use std::collections::HashSet;

use serde_json::Value;

use crate::domain::ai::{GeneratedFiles, ValidationIssue, ValidationReport};

/// Known field types accepted in `overlay.json` `fields[]`.
const FIELD_TYPES: &[&str] = &["text", "number", "color", "image", "select", "boolean"];

/// Validate the four generated files against the overlay contract.
///
/// `template_id` is the KEBAB-CASE directory name the overlay will live in —
/// it is matched against `TEMPLATE_ID` in `script.js` exactly (D8).
pub fn validate_generated_overlay(files: &GeneratedFiles, template_id: &str) -> ValidationReport {
    let mut issues: Vec<ValidationIssue> = Vec::new();

    for (path, content) in [
        ("overlay.json", files.overlay_json.as_str()),
        ("index.html", files.index_html.as_str()),
        ("style.css", files.style_css.as_str()),
        ("script.js", files.script_js.as_str()),
    ] {
        if content.trim().is_empty() {
            issues.push(ValidationIssue::new("empty_file").param("file", path));
        }
    }

    validate_manifest(&files.overlay_json, &mut issues);
    validate_script(&files.script_js, template_id, &mut issues);
    validate_style(&files.style_css, &mut issues);
    validate_markup(&files.index_html, &mut issues);

    if issues.is_empty() {
        ValidationReport::valid()
    } else {
        ValidationReport::invalid(issues)
    }
}

fn validate_manifest(overlay_json: &str, issues: &mut Vec<ValidationIssue>) {
    let parsed: Value = match serde_json::from_str(overlay_json) {
        Ok(v) => v,
        Err(e) => {
            issues
                .push(ValidationIssue::new("manifest_invalid_json").param("error", e.to_string()));
            return;
        }
    };
    let obj = match parsed.as_object() {
        Some(obj) => obj,
        None => {
            issues.push(ValidationIssue::new("manifest_not_object"));
            return;
        }
    };

    if obj.contains_key("id") {
        issues.push(ValidationIssue::new("manifest_top_level_id"));
    }

    match obj.get("name") {
        Some(Value::String(name)) if !name.trim().is_empty() => {}
        _ => issues.push(ValidationIssue::new("manifest_missing_name")),
    }

    match obj.get("fields") {
        Some(Value::Array(fields)) => {
            let mut seen: HashSet<&str> = HashSet::new();
            for (i, field) in fields.iter().enumerate() {
                let Some(fobj) = field.as_object() else {
                    issues.push(
                        ValidationIssue::new("manifest_field_not_object")
                            .param("index", i.to_string()),
                    );
                    continue;
                };
                for required in ["key", "label", "type"] {
                    if !fobj.contains_key(required) {
                        issues.push(
                            ValidationIssue::new("manifest_field_missing")
                                .param("index", i.to_string())
                                .param("key", required),
                        );
                    }
                }
                if let Some(k) = fobj.get("key").and_then(Value::as_str) {
                    if k.trim().is_empty() {
                        issues.push(
                            ValidationIssue::new("manifest_field_empty_key")
                                .param("index", i.to_string()),
                        );
                    } else if !seen.insert(k) {
                        issues.push(
                            ValidationIssue::new("manifest_duplicate_key")
                                .param("index", i.to_string())
                                .param("key", k),
                        );
                    }
                }
                if let Some(t) = fobj.get("type").and_then(Value::as_str) {
                    if !FIELD_TYPES.contains(&t) {
                        issues.push(
                            ValidationIssue::new("manifest_unknown_type")
                                .param("index", i.to_string())
                                .param("type", t),
                        );
                    }
                }
            }
        }
        Some(_) => issues.push(ValidationIssue::new("manifest_fields_not_array")),
        None => issues.push(ValidationIssue::new("manifest_fields_missing")),
    }
}

fn validate_script(script_js: &str, template_id: &str, issues: &mut Vec<ValidationIssue>) {
    if !script_js.contains(&format!("TEMPLATE_ID = \"{template_id}\"")) {
        issues.push(ValidationIssue::new("script_template_id").param("template_id", template_id));
    }
    for (name, signature) in [
        ("show", "function show("),
        ("update", "function update("),
        ("hide", "function hide("),
    ] {
        if !script_js.contains(signature) {
            issues.push(ValidationIssue::new("script_missing_handler").param("handler", name));
        }
    }
    if !script_js.contains("instance_id") {
        issues.push(ValidationIssue::new("script_no_instance_filter"));
    }
    for needle in ["WebSocket", "onclose", "setTimeout"] {
        if !script_js.contains(needle) {
            issues.push(ValidationIssue::new("script_no_reconnect").param("needle", needle));
        }
    }

    scan_js_assets(script_js, issues);
}

fn validate_style(style_css: &str, issues: &mut Vec<ValidationIssue>) {
    if !style_css.contains("transparent") {
        issues.push(ValidationIssue::new("style_not_transparent"));
    }
    scan_css_assets(style_css, issues);
}

fn validate_markup(index_html: &str, issues: &mut Vec<ValidationIssue>) {
    for reference in ["style.css", "script.js"] {
        if !index_html.contains(reference) {
            issues.push(ValidationIssue::new("html_missing_ref").param("ref", reference));
        }
    }
    scan_html_assets(index_html, issues);
}

/// Is this URL token relative, i.e. a safe local file reference?
/// Rejects `scheme://`, `//host`, absolute paths (`/x`), empty tokens and
/// `..` traversal (fail-closed, D8/D15).
fn is_relative_ref(token: &str) -> bool {
    let token = token.trim();
    if token.is_empty() {
        return false;
    }
    if token.starts_with("//") || token.starts_with('/') {
        return false;
    }
    if token.contains("..") {
        return false;
    }
    // A scheme is anything like `https:` before the first `/`.
    if let Some(colon) = token.find(':') {
        if token[..colon].chars().all(|c| c.is_ascii_alphanumeric()) {
            return false;
        }
    }
    true
}

/// D15: every `src`/`href`/`srcset` reference in the markup must resolve to
/// a local file. Remote refs (scheme://, //, absolute) are rejected.
fn scan_html_assets(html: &str, issues: &mut Vec<ValidationIssue>) {
    for attr in ["src", "href", "srcset"] {
        for (token_start, token) in attr_tokens(html, attr) {
            if !is_relative_ref(token) {
                issues.push(
                    ValidationIssue::new("remote_asset")
                        .param("file", "index.html")
                        .param("ref", token)
                        .param("offset", token_start.to_string()),
                );
            }
        }
    }
}

/// D15: every `url(...)` / `@import` token in the stylesheet must reference a
/// local file — covers `image-set(url(...))` and `cursor: url(...)` too, since
/// both flow through a `url(` token.
fn scan_css_assets(css: &str, issues: &mut Vec<ValidationIssue>) {
    let css_bytes = css.as_bytes();
    for (i, b) in css_bytes.iter().enumerate() {
        if *b != b'u' || !css[i..].starts_with("url") {
            continue;
        }
        let after = &css[i + 3..];
        let open = after.find('(');
        let Some(open) = open else { continue };
        let after_open = &after[open + 1..];
        let close = after_open.find(')');
        let Some(close) = close else { continue };
        let token = after_open[..close].trim().trim_matches(['"', '\'']);
        if !is_relative_ref(token) {
            issues.push(
                ValidationIssue::new("remote_asset")
                    .param("file", "style.css")
                    .param("ref", token),
            );
        }
    }
    // `@import "..."` / `@import '...'` (url-less form).
    let mut search_from = 0;
    while let Some(rel) = css[search_from..].find("@import") {
        let start = search_from + rel;
        let rest = &css[start + "@import".len()..];
        let trimmed = rest.trim_start();
        if let Some(quote) = trimmed.chars().next() {
            if quote == '"' || quote == '\'' {
                if let Some(end) = trimmed[1..].find(quote) {
                    let token = &trimmed[1..1 + end];
                    if !is_relative_ref(token) {
                        issues.push(
                            ValidationIssue::new("remote_asset")
                                .param("file", "style.css")
                                .param("ref", token),
                        );
                    }
                    search_from = start + "@import".len() + 1 + end;
                    continue;
                }
            }
        }
        search_from = start + "@import".len() + 1;
    }
}

/// D15 + contract: `fetch()` must not target remote/absolute addresses, and
/// `WebSocket(...)` URLs must be built from `location.host` (loopback server).
fn scan_js_assets(js: &str, issues: &mut Vec<ValidationIssue>) {
    // WebSocket: the URL must involve location.host.
    let mut from = 0;
    while let Some(rel) = js[from..].find("WebSocket(") {
        let start = from + rel;
        let rest = &js[start + "WebSocket(".len()..];
        // Capture until the first ')' — URLs never contain parens here.
        let arg = match rest.find(')') {
            Some(end) => &rest[..end],
            None => rest,
        };
        if !arg.contains("location.host") {
            issues.push(ValidationIssue::new("js_ws_not_loopback").param("ref", arg.trim()));
        }
        from = start + "WebSocket(".len() + arg.len() + 1;
    }

    // fetch: only relative local addresses allowed.
    from = 0;
    while let Some(rel) = js[from..].find("fetch(") {
        let start = from + rel;
        let rest = &js[start + "fetch(".len()..];
        let arg = match rest.find(')') {
            Some(end) => &rest[..end],
            None => rest,
        };
        let token = arg.trim().trim_matches(['"', '\'', '`']);
        if !is_relative_ref(token) {
            issues.push(ValidationIssue::new("js_remote_fetch").param("ref", token));
        }
        from = start + "fetch(".len() + arg.len() + 1;
    }
}

/// Find the quoted values of an HTML attribute (`src=".."`, `src='..'`,
/// `src = ".."`). Naive by design: comments or text that look like the
/// attribute are still checked — validation is fail-closed, so an overlarge
/// match only ever produces a rejection, never a silent pass.
fn attr_tokens<'a>(html: &'a str, attr: &str) -> Vec<(usize, &'a str)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = html[from..].find(attr) {
        let start = from + rel;
        let after = &html[start + attr.len()..];
        let after = after.trim_start();
        if !after.starts_with('=') {
            from = start + attr.len();
            continue;
        }
        let after_eq = after[1..].trim_start();
        let Some(quote) = after_eq.chars().next() else {
            break;
        };
        if quote != '"' && quote != '\'' {
            from = start + attr.len();
            continue;
        }
        let Some(end) = after_eq[1..].find(quote) else {
            break;
        };
        // For `srcset`, the value is a comma/space list — take the first token.
        let value = &after_eq[1..1 + end];
        let first = value.split([',', ' ']).next().unwrap_or(value);
        out.push((start, first));
        from = start + attr.len() + 1 + end;
    }
    out
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

    fn codes(report: &ValidationReport) -> Vec<&str> {
        report.issues.iter().map(|i| i.code.as_str()).collect()
    }

    #[test]
    fn accepts_a_fully_conforming_overlay() {
        let files = valid_files("mi-overlay");
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(report.valid, "expected valid: {:?}", report.issues);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn rejects_manifest_with_top_level_id() {
        let mut files = valid_files("mi-overlay");
        files.overlay_json = files
            .overlay_json
            .replace(r#"{"name""#, r#"{"id":"mi-overlay","name""#);
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(codes(&report).contains(&"manifest_top_level_id"));
    }

    #[test]
    fn rejects_missing_fields_array() {
        let mut files = valid_files("mi-overlay");
        files.overlay_json = r#"{"name":"Solo nombre"}"#.into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(codes(&report).contains(&"manifest_fields_missing"));
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
        assert!(codes(&report).contains(&"manifest_duplicate_key"));
        assert!(codes(&report).contains(&"manifest_unknown_type"));
    }

    #[test]
    fn rejects_wrong_template_id_exactly() {
        let files = valid_files("mi-other-overlay");
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(codes(&report).contains(&"script_template_id"));
    }

    #[test]
    fn rejects_missing_handlers_and_reconnect() {
        let mut files = valid_files("mi-overlay");
        files.script_js = files.script_js.replace("function hide() {}", "");
        files.script_js = files.script_js.replace("setTimeout", "setInterval");
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(codes(&report).contains(&"script_missing_handler"));
        assert!(codes(&report).contains(&"script_no_reconnect"));
    }

    #[test]
    fn rejects_opaque_body() {
        let mut files = valid_files("mi-overlay");
        files.style_css = "body { background: #000000; }".into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(codes(&report).contains(&"style_not_transparent"));
    }

    #[test]
    fn rejects_missing_local_references_and_empty_files() {
        let mut files = valid_files("mi-overlay");
        files.index_html = "<!doctype html><div>nope</div>".into();
        files.style_css = "".into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert!(codes(&report).contains(&"html_missing_ref"));
        assert!(codes(&report).contains(&"empty_file"));
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
        assert!(report.issues.len() >= 6);
    }

    // --- D15 asset scans: remote/absolute refs are rejected, local pass ---

    #[test]
    fn rejects_remote_asset_in_html_img_src() {
        let mut files = valid_files("mi-overlay");
        files.index_html =
            r#"<img src="https://cdn.example.com/logo.png"><script src="script.js"></script>"#
                .into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(codes(&report).contains(&"remote_asset"));
    }

    #[test]
    fn rejects_protocol_relative_and_absolute_srcset() {
        let mut files = valid_files("mi-overlay");
        files.index_html =
            r#"<img srcset="//cdn.example.com/a.png 2x"><script src="script.js"></script>"#.into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(codes(&report).contains(&"remote_asset"));
    }

    #[test]
    fn rejects_absolute_href() {
        let mut files = valid_files("mi-overlay");
        files.index_html =
            r#"<link rel="stylesheet" href="/assets/style.css"><script src="script.js"></script>"#
                .into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(codes(&report).contains(&"remote_asset"));
    }

    #[test]
    fn rejects_remote_url_in_css() {
        let mut files = valid_files("mi-overlay");
        files.style_css = "body { background: transparent; }\n#bg { background-image: url(\"https://x.example/bg.png\"); }".into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(codes(&report).contains(&"remote_asset"));
    }

    #[test]
    fn rejects_remote_import_in_css() {
        let mut files = valid_files("mi-overlay");
        files.style_css =
            "@import url(https://fonts.googleapis.com/css2);\nbody { background: transparent; }"
                .into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(codes(&report).contains(&"remote_asset"));
    }

    #[test]
    fn rejects_remote_in_image_set_and_cursor() {
        let mut files = valid_files("mi-overlay");
        files.style_css = "body { background: transparent; }\n#a { background-image: image-set(url(\"https://x/a.png\") 1x); }\n#b { cursor: url(https://x/cur.cur), auto; }".into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(!report.valid);
        assert_eq!(
            report
                .issues
                .iter()
                .filter(|i| i.code == "remote_asset")
                .count(),
            2,
            "both url() forms must be caught: {:?}",
            report.issues
        );
    }

    #[test]
    fn rejects_absolute_fetch_in_js() {
        let mut files = valid_files("mi-overlay");
        files.script_js = format!(
            "{}\nfetch(\"https://evil.example/data\")",
            valid_script("mi-overlay")
        );
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(codes(&report).contains(&"js_remote_fetch"));
    }

    #[test]
    fn rejects_non_loopback_websocket_in_js() {
        let mut files = valid_files("mi-overlay");
        files.script_js = format!(
            "{}\nfunction alt() {{ const ws = new WebSocket(\"wss://evil.example/ws\"); }}",
            valid_script("mi-overlay")
        );
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(codes(&report).contains(&"js_ws_not_loopback"));
    }

    #[test]
    fn allows_relative_assets_everywhere() {
        let mut files = valid_files("mi-overlay");
        files.index_html = r#"<!doctype html><img src="assets/logo.png"><img srcset="./tiles/x.png 1x, ./tiles/y.png 2x"><link rel="stylesheet" href="style.css"><script src="script.js"></script>"#.into();
        files.style_css = "body { background: transparent; }\n#bg { background-image: url(\"tiles/bg.png\"); cursor: url(cursors/cur.cur), auto; }".into();
        let report = validate_generated_overlay(&files, "mi-overlay");
        assert!(
            report.valid,
            "relative assets must pass: {:?}",
            report.issues
        );
    }
}
