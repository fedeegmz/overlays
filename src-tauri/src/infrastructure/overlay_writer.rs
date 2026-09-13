//! Staging + promotion of generated overlays inside the overlays directory.
//!
//! Flow: `stage` writes the generated 4-file bundle into
//! `<root>/.staging/<uuid4>/`; `accept` promotes it atomically (no-clobber)
//! to `<root>/<template-id>/`; `discard` removes the staging dir.
//! `sweep_orphans` removes leftovers from a crashed run at startup (D4).
//!
//! Safety rules:
//! - `accept` NEVER overwrites an existing template (D9) — on Linux the
//!   rename is no-clobber at the syscall level (RENAME_NOREPLACE); other
//!   platforms fall back to a checked `rename`.
//! - `discard` refuses to delete anything that is not a single uuid4-shaped
//!   direct child of `.staging/` (`..`, nested or absolute paths are refused).
//! - Template ids are validated with `validate_name` before any path is
//!   built (defense in depth; the generation service already normalizes).

use std::fs;
use std::path::{Path, PathBuf};

use uuid::{Uuid, Version};

use crate::domain::ai::GeneratedOverlay;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::name::validate_name;

const STAGING_DIR: &str = ".staging";

pub struct OverlayWriter {
    root: PathBuf,
}

impl OverlayWriter {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Directory that holds staged (not yet accepted) overlays.
    pub fn staging_root(&self) -> PathBuf {
        self.root.join(STAGING_DIR)
    }

    /// Write the generated files into a fresh `<root>/.staging/<uuid4>/`
    /// directory and return its path. The overlay's `directory` field is
    /// validated as a template name before staging starts.
    pub fn stage(&self, overlay: &GeneratedOverlay) -> DomainResult<PathBuf> {
        validate_name(&overlay.directory)?;
        let dir = self.staging_root().join(Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).map_err(|e| DomainError::TemplateWriteFailed {
            detail: format!("create staging dir: {e}"),
        })?;
        let files = [
            ("overlay.json", overlay.files.overlay_json.as_str()),
            ("index.html", overlay.files.index_html.as_str()),
            ("style.css", overlay.files.style_css.as_str()),
            ("script.js", overlay.files.script_js.as_str()),
        ];
        for (name, content) in files {
            let target = dir.join(name);
            if let Err(e) = fs::write(&target, content) {
                let _ = fs::remove_dir_all(&dir);
                return Err(DomainError::TemplateWriteFailed {
                    detail: format!("write {name}: {e}"),
                });
            }
        }
        Ok(dir)
    }

    /// Promote a staging dir to `<root>/<template_id>/` without overwriting
    /// an existing template. `staged` MUST be inside `.staging/`.
    pub fn accept(&self, staged: &Path, template_id: &str) -> DomainResult<()> {
        validate_name(template_id)?;
        self.ensure_staged(staged)?;
        let target = self.root.join(template_id);
        if target.exists() {
            return Err(DomainError::TemplateExists {
                name: template_id.to_string(),
            });
        }

        if cfg!(target_os = "linux") {
            // Full paths on both sides (renameat_with resolves dirfd-relative;
            // CWD is only the base — root differs in tests and in production).
            let from = rustix::ffi::CString::new(staged.as_os_str().to_str().ok_or_else(|| {
                DomainError::TemplateWriteFailed {
                    detail: "staged path is not UTF-8".into(),
                }
            })?)
            .map_err(|e| DomainError::TemplateWriteFailed {
                detail: format!("invalid staged path: {e}"),
            })?;
            let to = rustix::ffi::CString::new(target.as_os_str().to_str().ok_or_else(|| {
                DomainError::TemplateWriteFailed {
                    detail: "target path is not UTF-8".into(),
                }
            })?)
            .map_err(|e| DomainError::TemplateWriteFailed {
                detail: format!("invalid target name: {e}"),
            })?;
            rustix::fs::renameat_with(
                rustix::fs::CWD,
                &from,
                rustix::fs::CWD,
                &to,
                rustix::fs::RenameFlags::NOREPLACE,
            )
            .map_err(|e| match e {
                rustix::io::Errno::EXIST => DomainError::TemplateExists {
                    name: template_id.to_string(),
                },
                other => DomainError::TemplateWriteFailed {
                    detail: format!("rename failed: {other}"),
                },
            })?;
        } else {
            fs::rename(staged, &target).map_err(|e| DomainError::TemplateWriteFailed {
                detail: format!("rename failed: {e}"),
            })?;
        }
        Ok(())
    }

    /// Remove a staging dir. Refuses paths outside `.staging/`.
    pub fn discard(&self, staged: &Path) -> DomainResult<()> {
        self.ensure_staged(staged)?;
        fs::remove_dir_all(staged).map_err(|e| DomainError::TemplateWriteFailed {
            detail: format!("remove staging dir: {e}"),
        })
    }

    /// Sweep leftover staging dirs from a crashed run (D4). Called at
    /// startup, ONLY when the overlays dir is configured. Removes every
    /// direct child of `.staging/` that is a real directory, NOT a symlink,
    /// whose name is a uuid4 — anything else is skipped (and logged), never
    /// recursed into. Returns the number of removed dirs.
    pub fn sweep_orphans(&self) -> usize {
        let Ok(entries) = fs::read_dir(self.staging_root()) else {
            return 0; // no .staging yet (or unset root) — nothing to sweep
        };
        let mut removed = 0;
        for entry in entries.flatten() {
            let Ok(meta) = entry.file_type() else {
                continue;
            };
            if !meta.is_dir() {
                // A plain file inside .staging is not ours — leave it.
                continue;
            }
            if meta.is_symlink() {
                eprintln!(
                    "[overlays] sweep: skipping symlink in {STAGING_DIR}: {:?}",
                    entry.path()
                );
                continue;
            }
            let Ok(name) = entry.file_name().into_string() else {
                eprintln!(
                    "[overlays] sweep: skipping non-UTF-8 name in {STAGING_DIR}: {:?}",
                    entry.path()
                );
                continue;
            };
            let Ok(id) = Uuid::parse_str(&name) else {
                eprintln!("[overlays] sweep: skipping non-uuid4 name in {STAGING_DIR}: {name:?}");
                continue;
            };
            if id.get_version() != Some(Version::Random) {
                eprintln!("[overlays] sweep: skipping non-random uuid in {STAGING_DIR}: {name:?}");
                continue;
            }
            if fs::remove_dir_all(entry.path()).is_ok() {
                removed += 1;
            }
        }
        removed
    }

    /// Guard: `staged` must exist and be a SINGLE uuid4-shaped direct child
    /// of `.staging/` — dot segments (`..`, `.`), non-uuid names (defense in
    /// depth against path traversal into the real template tree) and nested
    /// paths are refused. The generation service performs the same uuid
    /// check on the staging id before this runs.
    fn ensure_staged(&self, staged: &Path) -> DomainResult<()> {
        let staging = self.staging_root();
        let Ok(rel) = staged.strip_prefix(&staging) else {
            return Err(DomainError::TemplateWriteFailed {
                detail: format!("refusing path outside {STAGING_DIR}: {}", staged.display()),
            });
        };
        let mut components = rel.components();
        let (Some(first), None) = (components.next(), components.next()) else {
            return Err(DomainError::TemplateWriteFailed {
                detail: format!(
                    "refusing nested path inside {STAGING_DIR}: {}",
                    staged.display()
                ),
            });
        };
        let name = first.as_os_str().to_str().unwrap_or_default();
        if name == "." || name == ".." {
            return Err(DomainError::TemplateWriteFailed {
                detail: format!(
                    "refusing dot segment in {STAGING_DIR}: {}",
                    staged.display()
                ),
            });
        }
        let id = Uuid::parse_str(name).map_err(|_| DomainError::StagedOverlayMissing)?;
        if id.get_version() != Some(Version::Random) {
            return Err(DomainError::StagedOverlayMissing);
        }
        if !staged.is_dir() {
            return Err(DomainError::StagedOverlayMissing);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ai::GeneratedFiles;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TAG: AtomicU64 = AtomicU64::new(0);

    /// Unique temp root per test (parallel tests share a pid).
    fn temp_root(tag: &str) -> PathBuf {
        let id = TAG.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("overlays-writer-{tag}-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn overlay(directory: &str) -> GeneratedOverlay {
        GeneratedOverlay {
            directory: directory.into(),
            name: directory.to_string(),
            files: GeneratedFiles {
                overlay_json: r#"{"name":"X","fields":[]}"#.into(),
                index_html: "<div></div>".into(),
                style_css: "body { background: transparent; }".into(),
                script_js: format!("const TEMPLATE_ID = \"{directory}\";"),
            },
        }
    }

    #[test]
    fn stage_writes_four_files_under_staging() {
        let root = temp_root("stage");
        let writer = OverlayWriter::new(root.clone());

        let staged = writer.stage(&overlay("mi-overlay")).unwrap();

        assert!(staged.starts_with(root.join(STAGING_DIR)));
        for name in ["overlay.json", "index.html", "style.css", "script.js"] {
            assert!(staged.join(name).is_file(), "{name} missing");
        }
        assert!(staged.join("overlay.json").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stage_rejects_invalid_directory_names() {
        let root = temp_root("stage-invalid");
        let writer = OverlayWriter::new(root.clone());

        let result = writer.stage(&overlay("Mi Overlay"));
        assert!(matches!(result, Err(DomainError::InvalidName { .. })));
        // Nothing was staged.
        assert!(!root.join(STAGING_DIR).exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn accept_promotes_staging_into_template_id() {
        let root = temp_root("accept");
        let writer = OverlayWriter::new(root.clone());

        let staged = writer.stage(&overlay("mi-overlay")).unwrap();
        writer.accept(&staged, "mi-overlay").unwrap();

        assert!(root.join("mi-overlay").join("overlay.json").is_file());
        assert!(!staged.exists(), "staging dir should be gone");

        // Discarding an already-promoted dir must fail (it no longer exists).
        assert!(matches!(
            writer.discard(&staged),
            Err(DomainError::StagedOverlayMissing)
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn accept_never_overwrites_an_existing_template() {
        let root = temp_root("accept-clobber");
        fs::create_dir_all(root.join("mi-overlay")).unwrap();
        fs::write(root.join("mi-overlay").join("overlay.json"), "existing").unwrap();
        let writer = OverlayWriter::new(root.clone());

        let staged = writer.stage(&overlay("mi-overlay")).unwrap();
        let result = writer.accept(&staged, "mi-overlay");

        assert!(matches!(result, Err(DomainError::TemplateExists { .. })));
        // Existing template is untouched and staging is still there.
        assert_eq!(
            fs::read_to_string(root.join("mi-overlay").join("overlay.json")).unwrap(),
            "existing"
        );
        assert!(staged.join("overlay.json").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn accept_rejects_invalid_template_ids() {
        let root = temp_root("accept-invalid");
        let writer = OverlayWriter::new(root.clone());
        let staged = writer.stage(&overlay("mi-overlay")).unwrap();

        assert!(matches!(
            writer.accept(&staged, "../escape"),
            Err(DomainError::InvalidName { .. })
        ));
        // Staging untouched.
        assert!(staged.join("overlay.json").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn discard_removes_only_staging_and_refuses_escape_attempts() {
        let root = temp_root("discard");
        let writer = OverlayWriter::new(root.clone());
        fs::create_dir_all(root.join("precious")).unwrap();
        fs::write(root.join("precious").join("overlay.json"), "keep me").unwrap();

        // Outside staging → refused.
        assert!(matches!(
            writer.discard(&root.join("precious")),
            Err(DomainError::TemplateWriteFailed { .. })
        ));
        assert!(root.join("precious").join("overlay.json").is_file());

        // Nested staging paths (traversal) → refused.
        let staged = writer.stage(&overlay("mi-overlay")).unwrap();
        let nested = staged.join("..");
        assert!(matches!(
            writer.discard(&nested),
            Err(DomainError::TemplateWriteFailed { .. })
        ));

        // Legit discard works.
        writer.discard(&staged).unwrap();
        assert!(!staged.exists());
        assert!(root.join(STAGING_DIR).exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn discard_single_dot_component_is_refused_and_root_is_safe() {
        let root = temp_root("discard-dotdot");
        let writer = OverlayWriter::new(root.clone());
        let precious = root.join("precious");
        fs::create_dir_all(&precious).unwrap();
        fs::write(precious.join("overlay.json"), "keep me").unwrap();

        // `.staging/..` resolves to the ROOT — without the dot-segment guard
        // this would remove_dir_all the whole overlays directory.
        let dotdot = writer.staging_root().join("..");
        let result = writer.discard(&dotdot);
        assert!(
            matches!(result, Err(DomainError::TemplateWriteFailed { .. })),
            "single '..' component must be refused, got {result:?}"
        );
        assert!(root.is_dir(), "root must survive a '..' discard attempt");
        assert!(precious.join("overlay.json").is_file());

        // A single '.' component is equally refused.
        assert!(matches!(
            writer.discard(&writer.staging_root().join(".")),
            Err(DomainError::TemplateWriteFailed { .. })
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn discard_refuses_non_uuid_single_component() {
        let root = temp_root("discard-nonuuid");
        let writer = OverlayWriter::new(root.clone());
        // A real directory with a non-uuid name inside .staging.
        fs::create_dir_all(root.join(STAGING_DIR).join("not-a-uuid")).unwrap();

        assert!(matches!(
            writer.discard(&root.join(STAGING_DIR).join("not-a-uuid")),
            Err(DomainError::StagedOverlayMissing)
        ));
        // Nothing was deleted.
        assert!(root.join(STAGING_DIR).join("not-a-uuid").is_dir());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sweep_unset_root_is_a_noop() {
        let writer = OverlayWriter::new(PathBuf::new());
        assert_eq!(writer.sweep_orphans(), 0);
    }

    #[test]
    fn sweep_removes_only_uuid4_shaped_orphans() {
        let root = temp_root("sweep");
        let writer = OverlayWriter::new(root.clone());
        fs::create_dir_all(root.join(STAGING_DIR)).unwrap();

        // Real orphans: two uuid4 dirs.
        let orphan_a = writer.staging_root().join(Uuid::new_v4().to_string());
        let orphan_b = writer.staging_root().join(Uuid::new_v4().to_string());
        fs::create_dir_all(&orphan_a).unwrap();
        fs::create_dir_all(&orphan_b).unwrap();
        fs::write(orphan_a.join("overlay.json"), "leftover").unwrap();

        // Must be skipped: a file, a non-uuid dir, a uuid1 dir, a symlink.
        fs::write(writer.staging_root().join("notes.txt"), "note").unwrap();
        fs::create_dir_all(writer.staging_root().join("not-a-uuid")).unwrap();
        let uuid1 = Uuid::parse_str("00000000-0000-1000-8000-000000000000").unwrap();
        fs::create_dir_all(writer.staging_root().join(uuid1.to_string())).unwrap();
        let symlink_target = root.join("symlink-target");
        fs::create_dir_all(&symlink_target).unwrap();
        fs::write(symlink_target.join("keep.txt"), "keep").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            &symlink_target,
            writer.staging_root().join(Uuid::new_v4().to_string()),
        )
        .unwrap();

        let removed = writer.sweep_orphans();

        assert_eq!(removed, 2, "only the two uuid4 dirs are orphans");
        assert!(!orphan_a.exists() && !orphan_b.exists(), "orphans removed");
        assert!(writer.staging_root().join("notes.txt").is_file());
        assert!(writer.staging_root().join("not-a-uuid").is_dir());
        assert!(writer.staging_root().join(uuid1.to_string()).is_dir());
        assert!(
            symlink_target.join("keep.txt").is_file(),
            "symlink target untouched"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
