export interface GroupPresetEntry {
  template: string;
  fields: Record<string, string>;
}

export interface GroupPreset {
  name: string;
  entries: GroupPresetEntry[];
}

export type GroupStartResult =
  | { template: string; ok: true; instanceId: string }
  | { template: string; ok: false; reason: "templateMissing" | "sendFailed" };
