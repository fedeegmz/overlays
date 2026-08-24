import { invoke } from "@tauri-apps/api/core";

import type { Manifest } from "../types";

export function listTemplates(): Promise<Manifest> {
  return invoke<Manifest>("list_templates");
}
