import { describe, expect, it } from "vitest";
import { COMMAND_ERROR_CODES } from "./commandError";

/**
 * Parity guard: the FE error-code map must mirror EXACTLY the Rust
 * `CommandError` constants in src-tauri/src/infrastructure/error.rs.
 * Update both sides together when a new code is added.
 *
 * The reverse direction (every Rust constant exists in both locale files) is
 * enforced by the Rust `include_str!` test in infrastructure/error.rs.
 */
const RUST_COMMAND_ERROR_CODES = [
  "preset.empty_name",
  "preset.save_failed",
  "template.discovery_failed",
  "language.unsupported",
  "overlays_dir.invalid",
  "config.save_failed",
  "keyring.unavailable",
  "keyring.failed",
  "keyring.delete_failed",
  "provider.unauthorized",
  "provider.rate_limited",
  "provider.timeout",
  "provider.network",
  "provider.unavailable",
  "provider.invalid_response",
  "provider.unknown",
  "model.unknown",
  "generation.invalid_output",
  "generation.invalid_name",
  "generation.empty_prompt",
  "overlays_dir.missing",
  "template.exists",
  "template.write_failed",
  "staged.missing",
  "common.internal",
] as const;

describe("COMMAND_ERROR_CODES parity", () => {
  it("mirrors the Rust CommandError constants exactly", () => {
    const fe = Object.values(COMMAND_ERROR_CODES).sort();
    const rust = [...RUST_COMMAND_ERROR_CODES].sort();
    expect(fe).toEqual(rust);
  });
});
