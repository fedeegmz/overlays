import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  GroupPreset,
  GroupStartResult,
  InstancesSnapshot,
} from "../types";

/** Random per window, so a client can recognize the echo of its own change. */
export const clientId: string = crypto.randomUUID();

export interface CreateInstanceResult {
  id: string;
  snapshot: InstancesSnapshot;
}

export interface StartGroupResult {
  results: GroupStartResult[];
  snapshot: InstancesSnapshot;
}

export function listInstances(): Promise<InstancesSnapshot> {
  return invoke<InstancesSnapshot>("list_instances", { clientId });
}

export function createInstance(
  templateId: string,
  opts?: { id?: string; customName?: string },
): Promise<CreateInstanceResult> {
  return invoke<CreateInstanceResult>("create_instance", {
    clientId,
    templateId,
    id: opts?.id,
    customName: opts?.customName,
  });
}

export function removeInstance(id: string): Promise<InstancesSnapshot> {
  return invoke<InstancesSnapshot>("remove_instance", { clientId, id });
}

export function renameInstance(
  id: string,
  name: string,
): Promise<InstancesSnapshot> {
  return invoke<InstancesSnapshot>("rename_instance", { clientId, id, name });
}

export function moveInstance(
  from: number,
  to: number,
): Promise<InstancesSnapshot> {
  return invoke<InstancesSnapshot>("move_instance", { clientId, from, to });
}

export function setInstanceFields(
  id: string,
  fields: Record<string, string>,
): Promise<InstancesSnapshot> {
  return invoke<InstancesSnapshot>("set_instance_fields", {
    clientId,
    id,
    fields,
  });
}

export function showInstance(id: string): Promise<InstancesSnapshot> {
  return invoke<InstancesSnapshot>("show_instance", { clientId, id });
}

export function updateInstance(id: string): Promise<InstancesSnapshot> {
  return invoke<InstancesSnapshot>("update_instance", { clientId, id });
}

export function hideInstance(id: string): Promise<InstancesSnapshot> {
  return invoke<InstancesSnapshot>("hide_instance", { clientId, id });
}

export function applyPreset(
  id: string,
  name: string,
): Promise<InstancesSnapshot> {
  return invoke<InstancesSnapshot>("apply_preset", { clientId, id, name });
}

export function startGroupPreset(name: string): Promise<StartGroupResult> {
  return invoke<StartGroupResult>("start_group_preset", { clientId, name });
}

export function saveGroupFromInstances(name: string): Promise<GroupPreset[]> {
  return invoke<GroupPreset[]>("save_group_from_instances", { name });
}

export function onInstancesChanged(
  handler: (snapshot: InstancesSnapshot) => void,
): Promise<UnlistenFn> {
  return listen<InstancesSnapshot>("instances-changed", (event) =>
    handler(event.payload),
  );
}
