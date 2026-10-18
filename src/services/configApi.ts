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

export function setUploadsEnabled(enabled: boolean): Promise<AppConfig> {
  return invoke<AppConfig>("set_uploads_enabled", { enabled });
}

export function setUploadDir(template: string | null): Promise<AppConfig> {
  return invoke<AppConfig>("set_upload_dir", { template });
}
