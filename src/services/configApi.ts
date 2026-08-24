import { invoke } from "@tauri-apps/api/core";

import type { AppConfig } from "../types";

export function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export function setOverlaysDir(path: string): Promise<AppConfig> {
  return invoke<AppConfig>("set_overlays_dir", { path });
}

export function setLanguage(lang: string): Promise<AppConfig> {
  return invoke<AppConfig>("set_language", { lang });
}
