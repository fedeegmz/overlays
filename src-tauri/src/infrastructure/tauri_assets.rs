//! `StaticAssets` backed by the files Tauri embeds from `frontendDist`.

use tauri::{AssetResolver, Runtime};

use crate::application::ports::StaticAssets;

pub struct TauriStaticAssets<R: Runtime> {
    resolver: AssetResolver<R>,
}

impl<R: Runtime> TauriStaticAssets<R> {
    pub fn new(resolver: AssetResolver<R>) -> Self {
        Self { resolver }
    }
}

/// Tauri answers an unknown path with `index.html` (single-page fallback).
/// That is wrong for a file request, so a page that was not asked for is a miss.
fn is_page_fallback(path: &str, mime: &str) -> bool {
    mime.starts_with("text/html") && !path.ends_with(".html")
}

impl<R: Runtime> StaticAssets for TauriStaticAssets<R> {
    fn get(&self, path: &str) -> Option<(Vec<u8>, String)> {
        let asset = self.resolver.get(path.to_string())?;
        if is_page_fallback(path, &asset.mime_type) {
            return None;
        }
        Some((asset.bytes, asset.mime_type))
    }
}

#[cfg(test)]
mod tests {
    use super::is_page_fallback;

    #[test]
    fn html_answer_for_a_non_page_path_is_a_miss() {
        assert!(is_page_fallback("assets/missing.js", "text/html"));
        assert!(!is_page_fallback("remote.html", "text/html"));
        assert!(!is_page_fallback("assets/app.js", "text/javascript"));
    }
}
