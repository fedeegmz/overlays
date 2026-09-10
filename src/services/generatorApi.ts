import { invoke } from "@tauri-apps/api/core";

import type { GeneratedOverlaySummary } from "../types";

export function generateOverlay(
  prompt: string,
): Promise<GeneratedOverlaySummary> {
  return invoke<GeneratedOverlaySummary>("generate_overlay", { prompt });
}

export function acceptOverlay(stagingId: string): Promise<void> {
  return invoke<void>("accept_overlay", { stagingId });
}

export function discardOverlay(stagingId: string): Promise<void> {
  return invoke<void>("discard_overlay", { stagingId });
}
