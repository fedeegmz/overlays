import { invoke } from "@tauri-apps/api/core";

import type { ApiKeyPresence, ConfiguredProviders } from "../types";

export function listConfiguredProviders(): Promise<ConfiguredProviders> {
  return invoke<ConfiguredProviders>("list_configured_providers");
}

export function addProviderKey(
  provider: string,
  key: string,
): Promise<ApiKeyPresence[]> {
  return invoke<ApiKeyPresence[]>("add_provider_key", { provider, key });
}

export function deleteProviderKey(provider: string): Promise<ApiKeyPresence[]> {
  return invoke<ApiKeyPresence[]>("delete_provider_key", { provider });
}
