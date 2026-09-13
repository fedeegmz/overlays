import { invoke } from "@tauri-apps/api/core";

import type { GeneratedOverlaySummary } from "../types";

export function generateOverlay(
  provider: string,
  model: string,
  name: string,
  prompt: string,
): Promise<GeneratedOverlaySummary> {
  return invoke<GeneratedOverlaySummary>("generate_overlay", {
    provider,
    model,
    name,
    prompt,
  });
}

export function acceptOverlay(stagingId: string, name: string): Promise<void> {
  return invoke<void>("accept_overlay", { stagingId, name });
}

export function discardOverlay(stagingId: string): Promise<void> {
  return invoke<void>("discard_overlay", { stagingId });
}

export function listProviderModels(provider: string): Promise<string[]> {
  return invoke<string[]>("list_provider_models", { provider });
}
