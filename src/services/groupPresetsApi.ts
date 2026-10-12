import { invoke } from "@tauri-apps/api/core";

import type { GroupPreset, GroupPresetEntry } from "../types";

export function listGroupPresets(): Promise<GroupPreset[]> {
  return invoke<GroupPreset[]>("list_group_presets");
}

export function saveGroupPreset(
  name: string,
  entries: GroupPresetEntry[],
): Promise<GroupPreset[]> {
  return invoke<GroupPreset[]>("save_group_preset", { name, entries });
}

export function deleteGroupPreset(name: string): Promise<GroupPreset[]> {
  return invoke<GroupPreset[]>("delete_group_preset", { name });
}
