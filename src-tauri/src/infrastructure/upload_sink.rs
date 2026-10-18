use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::domain::error::{DomainError, DomainResult};
use crate::domain::file_name::candidate_name;

/// How many `name (N)` candidates are tried before giving up.
const MAX_ATTEMPTS: u32 = 9999;

/// An upload being written to disk.
///
/// The final name is reserved up front with an empty placeholder (`create_new`, race-safe), data
/// streams into a hidden temp file in the same directory and `commit` renames it over the
/// placeholder. Dropping without `commit` removes both files, so aborted uploads leave nothing.
pub struct PendingUpload {
    final_path: PathBuf,
    temp_path: PathBuf,
    file: Option<File>,
    committed: bool,
}

impl PendingUpload {
    pub async fn create(dir: &Path, base: &str) -> DomainResult<Self> {
        Self::create_with_limit(dir, base, MAX_ATTEMPTS).await
    }

    pub(crate) async fn create_with_limit(dir: &Path, base: &str, max: u32) -> DomainResult<Self> {
        tokio::fs::create_dir_all(dir).await.map_err(write_failed)?;

        let mut reserved = None;
        for n in 0..=max {
            let name = candidate_name(base, n);
            let path = dir.join(&name);
            match tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .await
            {
                Ok(_) => {
                    reserved = Some((name, path));
                    break;
                }
                Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(write_failed(e)),
            }
        }
        let Some((name, final_path)) = reserved else {
            return Err(DomainError::UploadNameExhausted);
        };

        // From here on the placeholder is owned by `pending`, so any early return cleans it up.
        let mut pending = Self {
            final_path,
            temp_path: PathBuf::new(),
            file: None,
            committed: false,
        };
        let temp_path = dir.join(temp_name(&name));
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .await
            .map_err(write_failed)?;
        pending.temp_path = temp_path;
        pending.file = Some(file);
        Ok(pending)
    }

    pub fn file_name(&self) -> String {
        self.final_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub async fn write(&mut self, chunk: &[u8]) -> DomainResult<()> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| DomainError::UploadWriteFailed {
                detail: "upload already closed".into(),
            })?;
        file.write_all(chunk).await.map_err(write_failed)
    }

    /// Moves the finished temp file to its reserved final name.
    pub async fn commit(mut self) -> DomainResult<String> {
        if let Some(mut file) = self.file.take() {
            file.flush().await.map_err(write_failed)?;
        }
        tokio::fs::rename(&self.temp_path, &self.final_path)
            .await
            .map_err(write_failed)?;
        self.committed = true;
        Ok(self.file_name())
    }
}

impl Drop for PendingUpload {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        // Close the handle first (required on Windows), then remove the temp file and placeholder.
        self.file = None;
        if !self.temp_path.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.temp_path);
        }
        let _ = std::fs::remove_file(&self.final_path);
    }
}

fn write_failed(e: std::io::Error) -> DomainError {
    DomainError::UploadWriteFailed {
        detail: e.to_string(),
    }
}

/// Hidden, unique temp name in the destination directory. The final name is truncated so the
/// result stays below filesystem name limits.
fn temp_name(final_name: &str) -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let mut prefix = String::new();
    for c in final_name.chars() {
        if prefix.len() + c.len_utf8() > 100 {
            break;
        }
        prefix.push(c);
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    format!(".{prefix}.part-{}-{nanos}-{seq}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::test_util::TempDir;

    async fn upload(dir: &Path, base: &str, data: &[u8]) -> String {
        let mut p = PendingUpload::create(dir, base).await.unwrap();
        p.write(data).await.unwrap();
        p.commit().await.unwrap()
    }

    #[tokio::test]
    async fn stores_file_under_base_name() {
        let tmp = TempDir::new();
        let name = upload(tmp.path(), "photo.jpg", b"hello").await;
        assert_eq!(name, "photo.jpg");
        assert_eq!(
            std::fs::read(tmp.path().join("photo.jpg")).unwrap(),
            b"hello"
        );
        assert_eq!(TempDir::list(tmp.path()), vec!["photo.jpg"]);
    }

    #[tokio::test]
    async fn creates_missing_directory() {
        let tmp = TempDir::new();
        let dir = tmp.path().join("a/b");
        upload(&dir, "x.txt", b"1").await;
        assert!(dir.join("x.txt").is_file());
    }

    #[tokio::test]
    async fn collisions_get_numbered_suffixes_without_touching_existing() {
        let tmp = TempDir::new();
        upload(tmp.path(), "photo.jpg", b"one").await;
        assert_eq!(
            upload(tmp.path(), "photo.jpg", b"two").await,
            "photo (1).jpg"
        );
        assert_eq!(
            upload(tmp.path(), "photo.jpg", b"three").await,
            "photo (2).jpg"
        );
        assert_eq!(std::fs::read(tmp.path().join("photo.jpg")).unwrap(), b"one");
        assert_eq!(
            std::fs::read(tmp.path().join("photo (1).jpg")).unwrap(),
            b"two"
        );
    }

    #[tokio::test]
    async fn collision_without_extension_and_multi_dot() {
        let tmp = TempDir::new();
        upload(tmp.path(), "notes", b"a").await;
        assert_eq!(upload(tmp.path(), "notes", b"b").await, "notes (1)");
        upload(tmp.path(), "archive.tar.gz", b"a").await;
        assert_eq!(
            upload(tmp.path(), "archive.tar.gz", b"b").await,
            "archive.tar (1).gz"
        );
    }

    #[tokio::test]
    async fn concurrent_same_name_uploads_get_distinct_files() {
        let tmp = TempDir::new();
        let dir = tmp.path().to_path_buf();
        let mut handles = Vec::new();
        for i in 0..8u8 {
            let dir = dir.clone();
            handles.push(tokio::spawn(async move {
                let mut p = PendingUpload::create(&dir, "a.txt").await.unwrap();
                tokio::task::yield_now().await;
                p.write(&[i; 64]).await.unwrap();
                (i, p.commit().await.unwrap())
            }));
        }
        let mut names = Vec::new();
        for h in handles {
            let (i, name) = h.await.unwrap();
            assert_eq!(std::fs::read(dir.join(&name)).unwrap(), vec![i; 64]);
            names.push(name);
        }
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 8);
        assert_eq!(TempDir::list(&dir).len(), 8);
    }

    #[tokio::test]
    async fn drop_without_commit_leaves_nothing() {
        let tmp = TempDir::new();
        {
            let mut p = PendingUpload::create(tmp.path(), "a.txt").await.unwrap();
            p.write(b"partial").await.unwrap();
            // while pending, the final name is not mistaken for a completed upload
            assert!(!TempDir::list(tmp.path()).is_empty());
        }
        assert!(TempDir::list(tmp.path()).is_empty());
    }

    #[tokio::test]
    async fn pending_temp_is_hidden_and_does_not_block_other_names() {
        let tmp = TempDir::new();
        let _p = PendingUpload::create(tmp.path(), "a.txt").await.unwrap();
        let other = upload(tmp.path(), "b.txt", b"x").await;
        assert_eq!(other, "b.txt");
        let hidden: Vec<_> = TempDir::list(tmp.path())
            .into_iter()
            .filter(|n| n.starts_with('.'))
            .collect();
        assert_eq!(hidden.len(), 1);
    }

    #[tokio::test]
    async fn exhausting_candidates_fails_and_cleans_up() {
        let tmp = TempDir::new();
        std::fs::write(tmp.path().join("a.txt"), b"0").unwrap();
        std::fs::write(tmp.path().join("a (1).txt"), b"1").unwrap();
        let err = PendingUpload::create_with_limit(tmp.path(), "a.txt", 1)
            .await
            .err()
            .unwrap();
        assert!(matches!(err, DomainError::UploadNameExhausted));
        assert_eq!(TempDir::list(tmp.path()).len(), 2);
    }

    #[tokio::test]
    async fn long_names_still_get_a_valid_temp_file() {
        let tmp = TempDir::new();
        let base = format!("{}.txt", "a".repeat(250));
        let name = upload(tmp.path(), &base, b"x").await;
        assert_eq!(name, base);
    }

    #[tokio::test]
    async fn unwritable_directory_fails_with_write_error() {
        let tmp = TempDir::new();
        let blocker = tmp.path().join("file");
        std::fs::write(&blocker, b"x").unwrap();
        let err = PendingUpload::create(&blocker.join("sub"), "a.txt")
            .await
            .err()
            .unwrap();
        assert!(matches!(err, DomainError::UploadWriteFailed { .. }));
    }
}
