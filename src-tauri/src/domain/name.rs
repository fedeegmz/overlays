use unicode_normalization::UnicodeNormalization;

use super::error::{DomainError, DomainResult};

/// Maximum length (bytes) of a validated overlay name.
pub const MAX_NAME_LEN: usize = 64;

/// Map an accented lowercase char to its ASCII expansion, if any.
fn accent_map(c: char) -> Option<&'static str> {
    Some(match c {
        'á' | 'à' | 'ä' | 'â' | 'ã' | 'å' => "a",
        'é' | 'è' | 'ë' | 'ê' => "e",
        'í' | 'ì' | 'ï' | 'î' => "i",
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' | 'ø' => "o",
        'ú' | 'ù' | 'ü' | 'û' => "u",
        'ñ' => "n",
        'ç' => "c",
        'ý' | 'ÿ' => "y",
        'æ' => "ae",
        'œ' => "oe",
        'ß' => "ss",
        _ => return None,
    })
}

/// Normalize free text to a kebab-case ASCII slug: NFC, accents→ASCII,
/// lowercase; any non-alphanumeric, non-accent char separates words
/// (whitespace, `_`, `-`, `/`, emoji, symbols).
pub fn normalize_name(input: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    for c in input.nfc() {
        let lower: Vec<char> = c.to_lowercase().collect();
        let useful = lower
            .iter()
            .any(|lc| accent_map(*lc).is_some() || lc.is_ascii_alphanumeric());
        if useful {
            for lc in lower {
                if let Some(repl) = accent_map(lc) {
                    current.push_str(repl);
                } else if lc.is_ascii_alphanumeric() {
                    current.push(lc);
                }
            }
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words.join("-")
}

/// Strict kebab-case contract: `^[a-z0-9]+(-[a-z0-9]+)*$`, max `MAX_NAME_LEN`,
/// rejects `/`, `\`, `.` (incl. leading `.` and `..`).
pub fn validate_name(name: &str) -> DomainResult<()> {
    let invalid = |reason: &str| {
        Err(DomainError::InvalidName {
            reason: reason.to_string(),
        })
    };
    if name.is_empty() {
        return invalid("name is empty");
    }
    if name.len() > MAX_NAME_LEN {
        return invalid(&format!("name is longer than {MAX_NAME_LEN} characters"));
    }
    if name.contains(['/', '\\', '.']) {
        return invalid("name cannot contain '/', '\\' or '.'");
    }
    let mut prev_dash = false;
    for (i, &b) in name.as_bytes().iter().enumerate() {
        if b.is_ascii_lowercase() || b.is_ascii_digit() {
            prev_dash = false;
        } else if b == b'-' {
            if i == 0 || prev_dash {
                return invalid("name cannot start or end with '-' or contain '--'");
            }
            prev_dash = true;
        } else {
            return invalid("name must be kebab-case: lowercase letters, digits and single dashes");
        }
    }
    if prev_dash {
        return invalid("name cannot end with '-'");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_spaces_to_kebab() {
        assert_eq!(normalize_name("Mi Overlay"), "mi-overlay");
    }

    #[test]
    fn normalizes_accents_to_ascii() {
        assert_eq!(normalize_name("Árbol De Ñandú"), "arbol-de-nandu");
        assert_eq!(normalize_name("CAFÉ"), "cafe");
        assert_eq!(normalize_name("Über"), "uber");
    }

    #[test]
    fn normalizes_whitespace_underscore_and_symbols() {
        assert_eq!(normalize_name("  Hola   Mundo  "), "hola-mundo");
        assert_eq!(normalize_name("Mi_Overlay/Título"), "mi-overlay-titulo");
        assert_eq!(normalize_name("123 Foo"), "123-foo");
    }

    #[test]
    fn normalizes_emoji_and_foreign_letters_away() {
        assert_eq!(normalize_name("overlay 😀 test"), "overlay-test");
    }

    #[test]
    fn accepts_valid_kebab_names() {
        for name in ["mi-overlay", "a", "ab-cd-ef", "mi-overlay-2026"] {
            validate_name(name).unwrap_or_else(|e| panic!("{name} should be valid: {e}"));
        }
        validate_name(&"a".repeat(MAX_NAME_LEN)).unwrap();
    }

    #[test]
    fn rejects_empty_and_overlong_names() {
        assert!(matches!(
            validate_name(""),
            Err(DomainError::InvalidName { .. })
        ));
        assert!(matches!(
            validate_name(&"a".repeat(MAX_NAME_LEN + 1)),
            Err(DomainError::InvalidName { .. })
        ));
    }

    #[test]
    fn rejects_non_kebab_shapes() {
        for name in [
            "MiOverlay",
            "mi overlay",
            "mi_overlay",
            "-mi-overlay",
            "mi-overlay-",
            "mi--overlay",
        ] {
            assert!(
                matches!(validate_name(name), Err(DomainError::InvalidName { .. })),
                "{name} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_path_and_dot_names() {
        for name in ["mi.overlay", "..", ".hidden", "mi/overlay", "mi\\overlay"] {
            assert!(
                matches!(validate_name(name), Err(DomainError::InvalidName { .. })),
                "{name} should be rejected"
            );
        }
    }

    #[test]
    fn normalized_name_passes_validation() {
        let normalized = normalize_name("Mi Overlay 2026");
        validate_name(&normalized).unwrap();
        assert_eq!(normalized, "mi-overlay-2026");
    }
}
