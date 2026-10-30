//! Token and PIN primitives for remote access. Pure functions, no I/O.

// The PIN constants are read by the desktop pairing commands (S2c).
#![allow(dead_code)]

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Pairing PIN length in digits.
pub const PIN_DIGITS: usize = 6;

/// How long a PIN stays valid.
pub const PIN_TTL_MS: u64 = 120_000;

/// Wrong guesses after which the active PIN is burned.
pub const PIN_MAX_FAILURES: u32 = 5;

/// Pairing attempts allowed per IP in one `RATE_WINDOW_MS`.
pub const RATE_MAX_ATTEMPTS: usize = 10;
pub const RATE_WINDOW_MS: u64 = 60_000;

/// Lowercase hex SHA-256 of a token; the only form that is ever stored.
pub fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// True when `token` hashes to any stored hash. Every stored hash is compared
/// in constant time and no comparison short-circuits the others, so timing
/// reveals neither which hash matched nor how close a guess was.
pub fn matches_any(hashes: &[String], token: &str) -> bool {
    let candidate = hash_token(token);
    let mut found = subtle::Choice::from(0);
    for stored in hashes {
        found |= stored.as_bytes().ct_eq(candidate.as_bytes());
    }
    found.into()
}

/// Constant-time equality for PINs.
pub fn pin_matches(expected: &str, given: &str) -> bool {
    expected.as_bytes().ct_eq(given.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_sha256_hex() {
        assert_eq!(
            hash_token("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn matches_any_checks_every_hash() {
        let hashes = vec![hash_token("one"), hash_token("two")];
        assert!(matches_any(&hashes, "one"));
        assert!(matches_any(&hashes, "two"));
        assert!(!matches_any(&hashes, "three"));
        assert!(!matches_any(&[], "one"));
        assert!(!matches_any(&hashes, ""));
    }

    #[test]
    fn pin_matches_is_exact() {
        assert!(pin_matches("012345", "012345"));
        assert!(!pin_matches("012345", "012346"));
        assert!(!pin_matches("012345", "12345"));
    }
}
