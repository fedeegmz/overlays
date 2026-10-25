export interface GroupPresetEntry {
  template: string;
  fields: Record<string, string>;
  instance_id?: string;
  custom_name?: string;
}

export interface GroupPreset {
  name: string;
  entries: GroupPresetEntry[];
}

export type GroupStartResult =
  | { template: string; ok: true; instanceId: string; idConflict: boolean }
  | { template: string; ok: false; reason: "templateMissing" | "sendFailed" };
