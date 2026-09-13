/**
 * Thin client-side mirror of the Rust kebab-name rule (`domain/name.rs`).
 * The backend re-validates on generate AND accept — this only keeps the
 * form honest while typing (D8).
 */
export const MAX_NAME_LENGTH = 64;

const KEBAB_RE = /^[a-z0-9]+(-[a-z0-9]+)*$/;

export function isKebabName(value: string): boolean {
  const trimmed = value.trim();
  return (
    trimmed.length > 0 &&
    trimmed.length <= MAX_NAME_LENGTH &&
    KEBAB_RE.test(trimmed)
  );
}
