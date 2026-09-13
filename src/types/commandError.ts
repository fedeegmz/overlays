export interface CommandError {
  code: string;
  params?: Record<string, string>;
}

/** Mirror of the Rust `CommandError` code constants (src-tauri/src/infrastructure/error.rs). */
export const COMMAND_ERROR_CODES = {
  PRESET_EMPTY_NAME: "preset.empty_name",
  PRESET_SAVE_FAILED: "preset.save_failed",
  TEMPLATE_DISCOVERY_FAILED: "template.discovery_failed",
  LANGUAGE_UNSUPPORTED: "language.unsupported",
  OVERLAYS_DIR_INVALID: "overlays_dir.invalid",
  CONFIG_SAVE_FAILED: "config.save_failed",
  KEYRING_UNAVAILABLE: "keyring.unavailable",
  KEYRING_FAILED: "keyring.failed",
  KEYRING_DELETE_FAILED: "keyring.delete_failed",
  KEYRING_ENTRY_MISSING: "keyring.entry_missing",
  KEYRING_EMPTY_SECRET: "keyring.empty_secret",
  PROVIDER_UNAUTHORIZED: "provider.unauthorized",
  PROVIDER_RATE_LIMITED: "provider.rate_limited",
  PROVIDER_TIMEOUT: "provider.timeout",
  PROVIDER_NETWORK: "provider.network",
  PROVIDER_UNAVAILABLE: "provider.unavailable",
  PROVIDER_INVALID_RESPONSE: "provider.invalid_response",
  PROVIDER_UNKNOWN: "provider.unknown",
  MODEL_UNKNOWN: "model.unknown",
  GENERATION_INVALID_OUTPUT: "generation.invalid_output",
  GENERATION_INVALID_NAME: "generation.invalid_name",
  GENERATION_EMPTY_PROMPT: "generation.empty_prompt",
  OVERLAYS_DIR_MISSING: "overlays_dir.missing",
  TEMPLATE_EXISTS: "template.exists",
  TEMPLATE_WRITE_FAILED: "template.write_failed",
  STAGED_MISSING: "staged.missing",
  COMMON_INTERNAL: "common.internal",
} as const;

export function isCommandError(value: unknown): value is CommandError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof (value as { code: unknown }).code === "string"
  );
}
