use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::domain::error::{DomainError, DomainResult};
use crate::domain::file_name::sanitize_file_name;
use crate::domain::path_template::{expand, DateParts};

/// Source of the machine's local date and time, injectable for deterministic tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateParts;
}

/// Runtime upload settings, mirrored from the persisted config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadSettings {
    pub enabled: bool,
    /// Unexpanded destination template; `None` means the default Downloads folder.
    pub dir_template: Option<String>,
}

/// Shared handle so settings changes apply without restarting the server.
#[derive(Clone)]
pub struct UploadSettingsHandle(Arc<RwLock<UploadSettings>>);

impl UploadSettingsHandle {
    pub fn new(settings: UploadSettings) -> Self {
        Self(Arc::new(RwLock::new(settings)))
    }

    pub fn get(&self) -> UploadSettings {
        self.0.read().unwrap().clone()
    }

    pub fn is_enabled(&self) -> bool {
        self.0.read().unwrap().enabled
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.0.write().unwrap().enabled = enabled;
    }

    pub fn set_dir_template(&self, template: Option<String>) {
        self.0.write().unwrap().dir_template = template;
    }
}

/// Where an upload goes: the destination directory and the preferred file name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadTarget {
    pub dir: PathBuf,
    pub base: String,
}

pub struct UploadService {
    clock: Arc<dyn Clock>,
    settings: UploadSettingsHandle,
    downloads_dir: Option<PathBuf>,
}

impl UploadService {
    pub fn new(
        clock: Arc<dyn Clock>,
        settings: UploadSettingsHandle,
        downloads_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            clock,
            settings,
            downloads_dir,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.settings.is_enabled()
    }

    /// Resolves the destination directory and file name for one upload.
    ///
    /// Local time is read once, so the directory and the name expand consistently.
    pub fn plan(&self, original: &str, typed: Option<&str>) -> DomainResult<UploadTarget> {
        let now = self.clock.now();
        let dir = self.resolve_dir(&now)?;

        // The browser-reported name is taken literally: final path component only, no expansion.
        let original_name = sanitize_file_name(final_component(original));

        let typed_name = typed
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(|t| expand(t, &now))
            .and_then(|expanded| sanitize_file_name(final_component(&expanded)))
            .map(|name| with_original_extension(name, original_name.as_deref()));

        let base = typed_name
            .or(original_name)
            .unwrap_or_else(|| FALLBACK_NAME.to_string());
        Ok(UploadTarget { dir, base })
    }

    fn resolve_dir(&self, now: &DateParts) -> DomainResult<PathBuf> {
        let template = self.settings.get().dir_template;
        match template.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            Some(template) => {
                let expanded = expand(template, now);
                let dir = PathBuf::from(&expanded);
                if dir.is_absolute() {
                    Ok(dir)
                } else {
                    Err(DomainError::UploadDirInvalid { path: expanded })
                }
            }
            None => self
                .downloads_dir
                .as_ref()
                .map(|d| d.join(DEFAULT_SUBDIR))
                .ok_or(DomainError::UploadDestinationUnavailable),
        }
    }
}

const FALLBACK_NAME: &str = "upload";
const DEFAULT_SUBDIR: &str = "Overlays";

/// Text after the last `/` or `\`, so Windows and Unix client paths both reduce to a file name.
fn final_component(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

fn extension(name: &str) -> Option<&str> {
    name.rfind('.').filter(|&i| i > 0).map(|i| &name[i..])
}

/// A typed name without extension inherits the original file's extension.
fn with_original_extension(name: String, original: Option<&str>) -> String {
    if extension(&name).is_some() {
        return name;
    }
    match original.and_then(extension) {
        Some(ext) => sanitize_file_name(&format!("{name}{ext}")).unwrap_or(name),
        None => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedClock;
    impl Clock for FixedClock {
        fn now(&self) -> DateParts {
            DateParts {
                year: 2026,
                month: 3,
                day: 7,
                hour: 9,
                minute: 5,
                second: 1,
            }
        }
    }

    fn service(template: Option<&str>, downloads: Option<&str>) -> UploadService {
        UploadService::new(
            Arc::new(FixedClock),
            UploadSettingsHandle::new(UploadSettings {
                enabled: true,
                dir_template: template.map(String::from),
            }),
            downloads.map(PathBuf::from),
        )
    }

    fn base(original: &str, typed: Option<&str>) -> String {
        service(None, Some("/dl"))
            .plan(original, typed)
            .unwrap()
            .base
    }

    #[test]
    fn empty_name_uses_original() {
        assert_eq!(base("report.pdf", Some("")), "report.pdf");
        assert_eq!(base("report.pdf", None), "report.pdf");
    }

    #[test]
    fn whitespace_name_uses_original() {
        assert_eq!(base("report.pdf", Some("   ")), "report.pdf");
    }

    #[test]
    fn typed_name_without_extension_gets_original_extension() {
        assert_eq!(base("video.mp4", Some("clip")), "clip.mp4");
    }

    #[test]
    fn typed_name_with_extension_is_kept() {
        assert_eq!(base("video.mp4", Some("clip.mov")), "clip.mov");
    }

    #[test]
    fn typed_name_without_extension_and_original_without_extension() {
        assert_eq!(base("notes", Some("clip")), "clip");
    }

    #[test]
    fn name_reducing_to_nothing_falls_back_to_original() {
        assert_eq!(base("report.pdf", Some("..")), "report.pdf");
        assert_eq!(base("report.pdf", Some("/")), "report.pdf");
    }

    #[test]
    fn empty_original_and_empty_typed_uses_upload() {
        assert_eq!(base("", None), "upload");
        assert_eq!(base("..", Some("..")), "upload");
    }

    #[test]
    fn typed_name_is_expanded() {
        assert_eq!(base("a.jpg", Some("stage-%hh%mm")), "stage-0905.jpg");
        assert_eq!(base("a.jpg", Some("clip-%hh%mm")), "clip-0905.jpg");
    }

    #[test]
    fn original_name_is_not_expanded() {
        assert_eq!(base("100%%.txt", None), "100%%.txt");
        assert_eq!(base("%CCYY.txt", Some("")), "%CCYY.txt");
    }

    #[test]
    fn original_name_uses_final_path_component() {
        assert_eq!(base("C:\\Users\\x\\photo.jpg", None), "photo.jpg");
        assert_eq!(base("dir/photo.jpg", None), "photo.jpg");
        assert_eq!(base("a/b\\c/photo.jpg", None), "photo.jpg");
    }

    #[test]
    fn traversal_in_typed_name_stays_a_single_component() {
        let b = base("a.txt", Some("../../etc/passwd"));
        assert_eq!(b, "passwd.txt");
        let b = base("a.txt", Some("../../evil.sh"));
        assert_eq!(b, "evil.sh");
        let b = base("a.txt", Some("a\\b\\c.txt"));
        assert_eq!(b, "c.txt");
        assert!(!base("a.txt", Some("sub/dir/file.txt")).contains('/'));
    }

    #[test]
    fn default_dir_is_downloads_overlays() {
        let t = service(None, Some("/home/u/Downloads"))
            .plan("a.txt", None)
            .unwrap();
        assert_eq!(t.dir, PathBuf::from("/home/u/Downloads/Overlays"));
    }

    #[test]
    fn whitespace_template_is_treated_as_unset() {
        let t = service(Some("   "), Some("/dl"))
            .plan("a.txt", None)
            .unwrap();
        assert_eq!(t.dir, PathBuf::from("/dl/Overlays"));
    }

    #[test]
    fn dir_template_is_expanded() {
        let t = service(Some("/srv/up/%CCYY/%MM"), None)
            .plan("a.txt", None)
            .unwrap();
        assert_eq!(t.dir, PathBuf::from("/srv/up/2026/03"));
    }

    #[test]
    fn missing_downloads_dir_without_template_fails() {
        let err = service(None, None).plan("a.txt", None).unwrap_err();
        assert!(matches!(err, DomainError::UploadDestinationUnavailable));
    }

    #[test]
    fn relative_expanded_template_is_rejected() {
        let err = service(Some("rel/%DD"), Some("/dl"))
            .plan("a.txt", None)
            .unwrap_err();
        assert!(matches!(err, DomainError::UploadDirInvalid { .. }));
    }

    #[test]
    fn settings_handle_updates_are_visible() {
        let svc = service(None, Some("/dl"));
        assert!(svc.is_enabled());
        svc.settings.set_enabled(false);
        assert!(!svc.is_enabled());
        svc.settings.set_dir_template(Some("/x".into()));
        assert_eq!(svc.settings.get().dir_template.as_deref(), Some("/x"));
    }
}
