import { invoke } from "@tauri-apps/api/core";

import type { Checklist, ChecklistItem } from "../types";

export function listChecklists(): Promise<Checklist[]> {
  return invoke<Checklist[]>("list_checklists");
}

export function saveChecklist(
  id: string,
  name: string,
  items: ChecklistItem[],
): Promise<Checklist[]> {
  return invoke<Checklist[]>("save_checklist", { id, name, items });
}

export function deleteChecklist(id: string): Promise<Checklist[]> {
  return invoke<Checklist[]>("delete_checklist", { id });
}
