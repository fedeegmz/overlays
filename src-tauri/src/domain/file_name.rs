const MAX_NAME_BYTES: usize = 255;

const RESERVED_STEMS: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Reduces `raw` to a single safe path component. Returns `None` when nothing usable remains.
///
/// Separators and characters invalid on common filesystems become `_`, control characters are
/// removed, dot runs collapse, edge dots and spaces are trimmed, reserved Windows stems get a
/// `_` prefix and the result is truncated to 255 bytes on a UTF-8 boundary keeping the extension.
pub fn sanitize_file_name(raw: &str) -> Option<String> {
    let mut mapped = String::with_capacity(raw.len());
    for c in raw.chars() {
        if c.is_control() {
            continue;
        }
        if matches!(c, '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*') {
            mapped.push('_');
        } else if c == '.' && mapped.ends_with('.') {
            continue;
        } else {
            mapped.push(c);
        }
    }

    let trimmed = mapped
        .trim_start_matches(|c: char| c == '.' || c.is_whitespace())
        .trim_end_matches(|c: char| c == '.' || c.is_whitespace());
    if trimmed.is_empty() {
        return None;
    }

    let stem_end = trimmed.find('.').unwrap_or(trimmed.len());
    let mut name = if RESERVED_STEMS
        .iter()
        .any(|r| r.eq_ignore_ascii_case(&trimmed[..stem_end]))
    {
        format!("_{trimmed}")
    } else {
        trimmed.to_string()
    };

    if name.len() > MAX_NAME_BYTES {
        name = truncate_keeping_extension(&name);
    }
    Some(name)
}

fn truncate_keeping_extension(name: &str) -> String {
    let ext_start = match name.rfind('.') {
        Some(i) if i > 0 && name.len() - i < MAX_NAME_BYTES => i,
        _ => name.len(),
    };
    let (stem, ext) = name.split_at(ext_start);
    let mut cut = MAX_NAME_BYTES - ext.len();
    while !stem.is_char_boundary(cut) {
        cut -= 1;
    }
    let stem = stem[..cut].trim_end_matches(|c: char| c == '.' || c.is_whitespace());
    format!("{stem}{ext}")
}

/// Name to try for the `n`-th collision: `n == 0` is `base`, otherwise `stem (n).ext`.
pub fn candidate_name(base: &str, n: u32) -> String {
    if n == 0 {
        return base.to_string();
    }
    match base.rfind('.') {
        Some(i) if i > 0 => format!("{} ({n}){}", &base[..i], &base[i..]),
        _ => format!("{base} ({n})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traversal_stays_a_single_component() {
        let name = sanitize_file_name("../../evil.sh").unwrap();
        assert!(!name.contains('/') && !name.contains('\\'));
        assert!(!name.contains(".."));
        assert!(name.ends_with("evil.sh"));
    }

    #[test]
    fn backslashes_are_replaced() {
        assert_eq!(sanitize_file_name("a\\b\\c.txt").unwrap(), "a_b_c.txt");
    }

    #[test]
    fn windows_path_has_no_separators() {
        let name = sanitize_file_name("C:\\Users\\x\\photo.jpg").unwrap();
        assert!(!name.contains('\\') && !name.contains(':'));
        assert!(name.ends_with("photo.jpg"));
    }

    #[test]
    fn dots_only_yield_none() {
        assert_eq!(sanitize_file_name(".."), None);
        assert_eq!(sanitize_file_name("."), None);
        assert_eq!(sanitize_file_name("..."), None);
    }

    #[test]
    fn empty_and_blank_yield_none() {
        assert_eq!(sanitize_file_name(""), None);
        assert_eq!(sanitize_file_name("   "), None);
    }

    #[test]
    fn control_characters_are_removed() {
        assert_eq!(sanitize_file_name("a\0b\nc.txt").unwrap(), "abc.txt");
    }

    #[test]
    fn invalid_filesystem_characters_are_replaced() {
        assert_eq!(
            sanitize_file_name("a<b>c:d\"e|f?g*h").unwrap(),
            "a_b_c_d_e_f_g_h"
        );
    }

    #[test]
    fn leading_dots_and_trailing_dots_spaces_are_trimmed() {
        assert_eq!(sanitize_file_name("..hidden.txt").unwrap(), "hidden.txt");
        assert_eq!(sanitize_file_name("name. . ").unwrap(), "name");
        assert_eq!(sanitize_file_name("  spaced.txt  ").unwrap(), "spaced.txt");
    }

    #[test]
    fn dot_runs_collapse() {
        assert_eq!(sanitize_file_name("a...b.txt").unwrap(), "a.b.txt");
    }

    #[test]
    fn reserved_stems_are_prefixed() {
        assert_eq!(sanitize_file_name("CON").unwrap(), "_CON");
        assert_eq!(sanitize_file_name("nul.txt").unwrap(), "_nul.txt");
        assert_eq!(sanitize_file_name("Com1.log").unwrap(), "_Com1.log");
        assert_eq!(sanitize_file_name("LPT9").unwrap(), "_LPT9");
    }

    #[test]
    fn non_reserved_lookalikes_are_untouched() {
        assert_eq!(sanitize_file_name("console.txt").unwrap(), "console.txt");
        assert_eq!(sanitize_file_name("COM10").unwrap(), "COM10");
    }

    #[test]
    fn long_names_are_truncated_keeping_extension() {
        let raw = format!("{}.mp4", "a".repeat(400));
        let name = sanitize_file_name(&raw).unwrap();
        assert_eq!(name.len(), 255);
        assert!(name.ends_with(".mp4"));
    }

    #[test]
    fn truncation_respects_utf8_boundaries() {
        let raw = format!("{}.txt", "ñ".repeat(200));
        let name = sanitize_file_name(&raw).unwrap();
        assert!(name.len() <= 255);
        assert!(name.ends_with(".txt"));
        assert!(name.starts_with('ñ'));
    }

    #[test]
    fn sanitizing_is_idempotent() {
        let once = sanitize_file_name("../CON:x.txt").unwrap();
        assert_eq!(sanitize_file_name(&once).unwrap(), once);
    }

    #[test]
    fn candidate_zero_is_base() {
        assert_eq!(candidate_name("photo.jpg", 0), "photo.jpg");
    }

    #[test]
    fn candidate_inserts_counter_before_extension() {
        assert_eq!(candidate_name("photo.jpg", 1), "photo (1).jpg");
        assert_eq!(candidate_name("photo.jpg", 12), "photo (12).jpg");
    }

    #[test]
    fn candidate_without_extension_appends_counter() {
        assert_eq!(candidate_name("notes", 1), "notes (1)");
    }

    #[test]
    fn candidate_with_multiple_dots_splits_at_last_dot() {
        assert_eq!(candidate_name("archive.tar.gz", 1), "archive.tar (1).gz");
    }
}
