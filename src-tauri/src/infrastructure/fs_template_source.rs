use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use crate::application::ports::TemplateSource;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::template::{Manifest, OverlayField, TemplateInfo};

#[derive(Debug, Clone, serde::Deserialize)]
struct OverlayMeta {
    id: Option<String>,
    name: String,
    fields: Vec<OverlayField>,
}

fn validate_field(field: &OverlayField, meta_path: &Path) -> bool {
    if field.field_type == "progress" {
        let range_ok = matches!((field.min, field.max), (Some(min), Some(max)) if min < max);
        let step_ok = field.step.is_none_or(|step| step > 0.0);
        if range_ok && step_ok {
            return true;
        }
        eprintln!(
            "[overlays] skipping field {:?} in {:?}: progress fields require min and max with min < max, and step > 0 when present",
            field.key, meta_path
        );
        return false;
    }

    if field.field_type == "boolean" {
        return match field.default.as_deref() {
            Some("true") | Some("false") | None => true,
            Some(other) => {
                eprintln!(
                    "[overlays] skipping field {:?} in {:?}: boolean fields require default \"true\" or \"false\", got {:?}",
                    field.key, meta_path, other
                );
                false
            }
        };
    }

    if field.field_type == "file" {
        if let Some(accept) = &field.accept {
            if accept.is_empty() || !accept.iter().all(|ext| is_valid_extension(ext)) {
                eprintln!(
                    "[overlays] skipping field {:?} in {:?}: file fields require non-empty extension filters without dots, got {:?}",
                    field.key, meta_path, accept
                );
                return false;
            }
        }
    }

    true
}

fn is_valid_extension(ext: &str) -> bool {
    let normalized = normalize_extension(ext);
    !normalized.is_empty() && normalized.chars().all(|c| c.is_ascii_alphanumeric())
}

fn normalize_extension(ext: &str) -> String {
    ext.trim().trim_start_matches('.').to_ascii_lowercase()
}

fn normalize_file_field(mut field: OverlayField) -> OverlayField {
    if field.field_type == "file" {
        if let Some(accept) = field.accept.take() {
            field.accept = Some(accept.iter().map(|e| normalize_extension(e)).collect());
        }
    }
    field
}

/// Resolves an absolute file path into a path relative to the template
/// folder (`template_rel_dir`, `/`-separated, relative to the overlays dir), guaranteed to live inside the overlays directory.
///
/// The returned value is what the overlay receives as a field value: a
/// relative path (e.g. `logo.png` or `../assets/logo.png`) that the local
/// HTTP server can serve under `/overlay/*`.
pub fn resolve_asset_path(
    overlays_dir: &Path,
    template_rel_dir: &str,
    absolute_path: &str,
) -> Result<String, DomainError> {
    let canonical_dir =
        dunce::canonicalize(overlays_dir).map_err(|_| DomainError::AssetResolveFailed {
            path: absolute_path.to_string(),
        })?;
    let source = Path::new(absolute_path);
    let canonical_source =
        dunce::canonicalize(source).map_err(|_| DomainError::AssetResolveFailed {
            path: absolute_path.to_string(),
        })?;
    if !canonical_source.starts_with(&canonical_dir) {
        return Err(DomainError::AssetOutsideOverlaysDir {
            path: absolute_path.to_string(),
        });
    }

    let template_dir = dunce::canonicalize(canonical_dir.join(template_rel_dir)).map_err(|_| {
        DomainError::AssetResolveFailed {
            path: absolute_path.to_string(),
        }
    })?;

    let relative = relative_between(&template_dir, &canonical_source);
    Ok(path_to_web(relative))
}

fn relative_between(base: &Path, target: &Path) -> PathBuf {
    let base: Vec<PathBuf> = base
        .components()
        .map(|c| PathBuf::from(c.as_os_str()))
        .collect();
    let target: Vec<PathBuf> = target
        .components()
        .map(|c| PathBuf::from(c.as_os_str()))
        .collect();
    let common = base
        .iter()
        .zip(target.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut rel = PathBuf::new();
    for _ in common..base.len() {
        rel.push("..");
    }
    for comp in &target[common..] {
        rel.push(comp);
    }
    rel
}

fn path_to_web(path: PathBuf) -> String {
    path.components()
        .filter_map(|c| {
            if matches!(c, Component::Prefix(_) | Component::RootDir) {
                None
            } else {
                Some(c.as_os_str().to_string_lossy().into_owned())
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Deepest folder level that gets listed; its children are not walked.
const MAX_DEPTH: usize = 5;

pub struct FsTemplateSource;

struct Walk {
    templates: Vec<TemplateInfo>,
    folders: Vec<String>,
    seen_ids: HashSet<String>,
}

fn join_rel(rel: &str, name: &str) -> String {
    if rel.is_empty() {
        name.to_string()
    } else {
        format!("{rel}/{name}")
    }
}

fn load_template(dir: &Path, rel: &str, child_rel: &str, name: &str) -> Option<TemplateInfo> {
    let meta_path = dir.join("overlay.json");
    if !dir.join("index.html").exists() {
        eprintln!("[overlays] skipping {child_rel:?}: no index.html found");
        return None;
    }

    let content = match std::fs::read_to_string(&meta_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[overlays] could not read {:?}: {e}", meta_path);
            return None;
        }
    };

    let meta: OverlayMeta = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[overlays] could not parse {:?}: {e}", meta_path);
            return None;
        }
    };

    let id = meta.id.unwrap_or_else(|| name.to_string());
    let fields = meta
        .fields
        .into_iter()
        .filter(|field| validate_field(field, &meta_path))
        .map(normalize_file_field)
        .collect();

    Some(TemplateInfo {
        id,
        name: meta.name,
        path: format!("{child_rel}/index.html"),
        folder: rel.to_string(),
        fields,
    })
}

fn walk(abs_dir: &Path, rel: &str, depth: usize, out: &mut Walk) -> DomainResult<()> {
    let read = std::fs::read_dir(abs_dir).and_then(|it| it.collect::<Result<Vec<_>, _>>());
    let mut entries = match read {
        Ok(entries) => entries,
        Err(e) if depth == 0 => {
            return Err(DomainError::TemplateDiscoveryFailed {
                detail: format!("could not read {abs_dir:?}: {e}"),
            });
        }
        Err(e) => {
            eprintln!("[overlays] could not read {abs_dir:?}: {e}");
            return Ok(());
        }
    };
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }

        let path = entry.path();
        let child_rel = join_rel(rel, &name);

        if path.join("overlay.json").exists() {
            let Some(info) = load_template(&path, rel, &child_rel, &name) else {
                continue;
            };
            if !out.seen_ids.insert(info.id.clone()) {
                eprintln!(
                    "[overlays] skipping duplicate id {:?} at {child_rel:?}",
                    info.id
                );
                continue;
            }
            out.templates.push(info);
        } else {
            out.folders.push(child_rel.clone());
            if depth + 1 < MAX_DEPTH {
                walk(&path, &child_rel, depth + 1, out)?;
            }
        }
    }
    Ok(())
}

impl TemplateSource for FsTemplateSource {
    fn discover(&self, overlay_dir: &Path) -> DomainResult<Manifest> {
        let mut out = Walk {
            templates: Vec::new(),
            folders: Vec::new(),
            seen_ids: HashSet::new(),
        };
        if overlay_dir.is_dir() {
            walk(overlay_dir, "", 0, &mut out)?;
        }

        out.templates.sort_by(|a, b| a.name.cmp(&b.name));
        out.folders.sort();
        Ok(Manifest {
            templates: out.templates,
            folders: out.folders,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_template(dir: &Path, id: &str, fields: &str) {
        let tpl = dir.join(id);
        std::fs::create_dir_all(&tpl).unwrap();
        std::fs::write(tpl.join("index.html"), "<html><body></body></html>").unwrap();
        std::fs::write(
            tpl.join("overlay.json"),
            format!(r#"{{ "name": "T {id}", "fields": {fields} }}"#),
        )
        .unwrap();
    }

    fn write_tpl(dir: &Path, rel: &str, id: Option<&str>) {
        let tpl = dir.join(rel);
        std::fs::create_dir_all(&tpl).unwrap();
        std::fs::write(tpl.join("index.html"), "<html></html>").unwrap();
        let id_json = id.map(|i| format!(r#""id": "{i}","#)).unwrap_or_default();
        std::fs::write(
            tpl.join("overlay.json"),
            format!(r#"{{ {id_json} "name": "T {rel}", "fields": [] }}"#),
        )
        .unwrap();
    }

    #[test]
    fn discovers_nested_templates_with_folder_and_path() {
        let dir = temp_dir("overlays-nested-test");
        write_tpl(&dir, "root-tpl", None);
        write_tpl(&dir, "a/b/deep", None);

        let manifest = FsTemplateSource.discover(&dir).unwrap();
        let root = manifest
            .templates
            .iter()
            .find(|t| t.id == "root-tpl")
            .unwrap();
        assert_eq!(root.folder, "");
        assert_eq!(root.path, "root-tpl/index.html");
        let deep = manifest.templates.iter().find(|t| t.id == "deep").unwrap();
        assert_eq!(deep.folder, "a/b");
        assert_eq!(deep.path, "a/b/deep/index.html");
        assert_eq!(manifest.folders, vec!["a", "a/b"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_folders_are_listed() {
        let dir = temp_dir("overlays-nested-test");
        std::fs::create_dir_all(dir.join("empty/inner")).unwrap();

        let manifest = FsTemplateSource.discover(&dir).unwrap();
        assert!(manifest.templates.is_empty());
        assert_eq!(manifest.folders, vec!["empty", "empty/inner"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hidden_directories_are_skipped() {
        let dir = temp_dir("overlays-nested-test");
        write_tpl(&dir, ".hidden/tpl", None);
        write_tpl(&dir, "visible/.git/tpl", None);

        let manifest = FsTemplateSource.discover(&dir).unwrap();
        assert!(manifest.templates.is_empty());
        assert_eq!(manifest.folders, vec!["visible"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn walk_stops_at_depth_five() {
        let dir = temp_dir("overlays-nested-test");
        write_tpl(&dir, "d1/d2/d3/d4/ok", None);
        write_tpl(&dir, "d1/d2/d3/d4/d5/too-deep", None);

        let manifest = FsTemplateSource.discover(&dir).unwrap();
        let ids: Vec<&str> = manifest.templates.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["ok"]);
        assert!(manifest.folders.contains(&"d1/d2/d3/d4".to_string()));
        assert!(manifest.folders.contains(&"d1/d2/d3/d4/d5".to_string()));
        assert_eq!(manifest.folders.len(), 5);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn template_directory_is_a_leaf() {
        let dir = temp_dir("overlays-nested-test");
        write_tpl(&dir, "leaf", None);
        write_tpl(&dir, "leaf/inner", None);

        let manifest = FsTemplateSource.discover(&dir).unwrap();
        let ids: Vec<&str> = manifest.templates.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["leaf"]);
        assert!(manifest.folders.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn meta_id_differs_from_directory_name() {
        let dir = temp_dir("overlays-nested-test");
        write_tpl(&dir, "grp/dirname", Some("custom-id"));

        let manifest = FsTemplateSource.discover(&dir).unwrap();
        assert_eq!(manifest.templates[0].id, "custom-id");
        assert_eq!(manifest.templates[0].path, "grp/dirname/index.html");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_id_first_in_alphabetical_walk_wins() {
        let dir = temp_dir("overlays-nested-test");
        write_tpl(&dir, "x", None);
        write_tpl(&dir, "a/x", None);

        let manifest = FsTemplateSource.discover(&dir).unwrap();
        assert_eq!(manifest.templates.len(), 1);
        assert_eq!(manifest.templates[0].path, "a/x/index.html");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_directories_are_skipped() {
        let dir = temp_dir("overlays-nested-test");
        let target = temp_dir("overlays-symlink-target");
        write_tpl(&target, "linked", None);
        std::os::unix::fs::symlink(target.join("linked"), dir.join("link")).unwrap();
        std::os::unix::fs::symlink(&target, dir.join("linkdir")).unwrap();

        let manifest = FsTemplateSource.discover(&dir).unwrap();
        assert!(manifest.templates.is_empty());
        assert!(manifest.folders.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&target);
    }

    #[test]
    fn progress_field_with_valid_min_max_is_kept() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-a",
            r#"[{ "key": "p", "label": "P", "type": "progress", "min": 0, "max": 100, "default": "50" }]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap().templates;
        assert_eq!(templates.len(), 1);
        let field = &templates[0].fields[0];
        assert_eq!(field.field_type, "progress");
        assert_eq!(field.min, Some(0.0));
        assert_eq!(field.max, Some(100.0));
        assert_eq!(field.default.as_deref(), Some("50"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn progress_field_with_positive_step_is_kept() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-step",
            r#"[{ "key": "p", "label": "P", "type": "progress", "min": 0, "max": 100, "step": 0.1 }]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap().templates;
        assert_eq!(templates[0].fields[0].step, Some(0.1));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn progress_field_with_non_positive_step_is_dropped() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-bad-step",
            r#"[
                { "key": "zero", "label": "Z", "type": "progress", "min": 0, "max": 100, "step": 0 },
                { "key": "neg", "label": "N", "type": "progress", "min": 0, "max": 100, "step": -1 },
                { "key": "texto", "label": "T", "type": "text" }
            ]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap().templates;
        let keys: Vec<&str> = templates[0].fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["texto"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn progress_field_without_min_max_is_dropped() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-b",
            r#"[
                { "key": "bad", "label": "Bad", "type": "progress" },
                { "key": "inverted", "label": "Inv", "type": "progress", "min": 100, "max": 0 },
                { "key": "texto", "label": "T", "type": "text" }
            ]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap().templates;
        assert_eq!(templates.len(), 1);
        let keys: Vec<&str> = templates[0].fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["texto"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn boolean_field_with_valid_default_is_kept() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-bool",
            r#"[{ "key": "b", "label": "B", "type": "boolean", "default": "true" }]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap().templates;
        assert_eq!(templates.len(), 1);
        let field = &templates[0].fields[0];
        assert_eq!(field.field_type, "boolean");
        assert_eq!(field.default.as_deref(), Some("true"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn boolean_field_without_default_is_kept() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-bool",
            r#"[{ "key": "b", "label": "B", "type": "boolean" }]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap().templates;
        assert_eq!(templates.len(), 1);
        let field = &templates[0].fields[0];
        assert_eq!(field.default, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn boolean_field_with_invalid_default_is_dropped() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-bool",
            r#"[
                { "key": "bad", "label": "Bad", "type": "boolean", "default": "yes" },
                { "key": "texto", "label": "T", "type": "text" }
            ]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap().templates;
        assert_eq!(templates.len(), 1);
        let keys: Vec<&str> = templates[0].fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["texto"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_field_with_valid_accept_is_kept_and_normalized() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-file",
            r#"[{ "key": "logo", "label": "Logo", "type": "file", "accept": [".PNG", "jpg", " svg "] }]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap().templates;
        assert_eq!(templates.len(), 1);
        let field = &templates[0].fields[0];
        assert_eq!(field.field_type, "file");
        let expected: Vec<String> = ["png", "jpg", "svg"].map(String::from).to_vec();
        assert_eq!(field.accept.as_deref(), Some(expected.as_slice()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_field_without_accept_is_kept() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-file",
            r#"[{ "key": "logo", "label": "Logo", "type": "file" }]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap().templates;
        assert_eq!(templates.len(), 1);
        let field = &templates[0].fields[0];
        assert_eq!(field.field_type, "file");
        assert_eq!(field.accept, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_field_with_invalid_accept_is_dropped() {
        let dir = temp_dir("overlays-src-test");
        write_template(
            &dir,
            "tpl-file",
            r#"[
                { "key": "bad", "label": "Bad", "type": "file", "accept": ["exe../"] },
                { "key": "no-filter", "label": "No filter", "type": "file", "accept": [] },
                { "key": "texto", "label": "T", "type": "text" }
            ]"#,
        );

        let templates = FsTemplateSource.discover(&dir).unwrap().templates;
        assert_eq!(templates.len(), 1);
        let keys: Vec<&str> = templates[0].fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["texto"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_asset_path_returns_relative_path_inside_template_folder() {
        let dir = temp_dir("overlays-resolve-test");
        let tpl = dir.join("tpl-a");
        std::fs::create_dir_all(&tpl).unwrap();
        let asset = tpl.join("logo.png");
        std::fs::write(&asset, b"png").unwrap();

        let resolved = resolve_asset_path(&dir, "tpl-a", asset.to_str().unwrap()).unwrap();
        assert_eq!(resolved, "logo.png");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_asset_path_accepts_files_elsewhere_inside_overlays_dir() {
        let dir = temp_dir("overlays-resolve-test");
        let tpl = dir.join("tpl-a");
        std::fs::create_dir_all(&tpl).unwrap();
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        let asset = dir.join("assets").join("logo.png");
        std::fs::write(&asset, b"png").unwrap();

        let resolved = resolve_asset_path(&dir, "tpl-a", asset.to_str().unwrap()).unwrap();
        assert_eq!(resolved, "../assets/logo.png");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_asset_path_handles_nested_template_dir() {
        let dir = temp_dir("overlays-resolve-test");
        let tpl = dir.join("grp").join("sub").join("tpl-a");
        std::fs::create_dir_all(&tpl).unwrap();
        let asset = tpl.join("logo.png");
        std::fs::write(&asset, b"png").unwrap();
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        let shared = dir.join("assets").join("bg.png");
        std::fs::write(&shared, b"png").unwrap();

        let resolved = resolve_asset_path(&dir, "grp/sub/tpl-a", asset.to_str().unwrap()).unwrap();
        assert_eq!(resolved, "logo.png");
        let resolved = resolve_asset_path(&dir, "grp/sub/tpl-a", shared.to_str().unwrap()).unwrap();
        assert_eq!(resolved, "../../../assets/bg.png");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_asset_path_uses_directory_not_meta_id() {
        let dir = temp_dir("overlays-resolve-test");
        write_tpl(&dir, "grp/dirname", Some("custom-id"));
        let asset = dir.join("grp/dirname/logo.png");
        std::fs::write(&asset, b"png").unwrap();

        let resolved = resolve_asset_path(&dir, "grp/dirname", asset.to_str().unwrap()).unwrap();
        assert_eq!(resolved, "logo.png");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_asset_path_rejects_files_outside_overlays_dir() {
        let dir = temp_dir("overlays-resolve-test");
        let tpl = dir.join("tpl-a");
        std::fs::create_dir_all(&tpl).unwrap();
        let outside = std::env::temp_dir().join("overlays-outside-test-file.png");
        std::fs::write(&outside, b"png").unwrap();

        let error = resolve_asset_path(&dir, "tpl-a", outside.to_str().unwrap()).unwrap_err();
        assert!(matches!(error, DomainError::AssetOutsideOverlaysDir { .. }));

        let _ = std::fs::remove_file(&outside);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
