import { invoke } from "@tauri-apps/api/core";

import type { Preset } from "../types";

export interface SavePresetArgs {
  name: string;
  template: string;
  fields: Record<string, string>;
}

export function listPresets(): Promise<Preset[]> {
  return invoke<Preset[]>("list_presets");
}

export function savePreset(args: SavePresetArgs): Promise<Preset[]> {
  return invoke<Preset[]>("save_preset", {
    name: args.name,
    template: args.template,
    fields: args.fields,
  });
}

export function deletePreset(name: string): Promise<Preset[]> {
  return invoke<Preset[]>("delete_preset", { name });
}
