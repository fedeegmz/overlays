export interface AppConfig {
  overlays_dir: string | null;
  language: string | null;
  /** Key presence metadata ONLY — secrets never reach the frontend (K1). */
  provider_keys: ApiKeyPresence[];
  ai_generator_enabled: boolean;
}

export interface ApiKeyPresence {
  provider: string;
  configured: boolean;
  last4: string | null;
}

export interface ConfiguredProviders {
  providers: ApiKeyPresence[];
  keyring_available: boolean;
}

export function isConfiguredProviders(
  value: unknown,
): value is ConfiguredProviders {
  return (
    typeof value === "object" &&
    value !== null &&
    "providers" in value &&
    "keyring_available" in value
  );
}
