import { defineStore } from "pinia";
import { ref } from "vue";
import { commandErrorMessage } from "../lib/errors";
import {
  deleteGroupPreset,
  listGroupPresets,
  saveGroupPreset,
} from "../services/groupPresetsApi";
import type { GroupPreset, GroupStartResult } from "../types";
import { useInstanceStore } from "./instances";
import { useTemplateStore } from "./templates";

export const useGroupPresetStore = defineStore("groupPresets", () => {
  const instanceStore = useInstanceStore();
  const templateStore = useTemplateStore();

  const groups = ref<GroupPreset[]>([]);
  const groupsError = ref<string | null>(null);

  async function refreshGroups(): Promise<void> {
    groupsError.value = null;
    try {
      groups.value = await listGroupPresets();
    } catch (err) {
      groupsError.value = commandErrorMessage(err);
    }
  }

  function hasGroup(name: string): boolean {
    const trimmed = name.trim();
    return groups.value.some((g) => g.name === trimmed);
  }

  /** Snapshots every open instance (hidden included) in sidebar order. */
  async function saveFromInstances(name: string): Promise<void> {
    groupsError.value = null;
    const entries = instanceStore.instances.map((instance) => ({
      template: instance.templateId,
      fields: instanceStore.fieldsFor(instance),
      instance_id: instance.id,
      custom_name: instance.customName,
    }));
    try {
      groups.value = await saveGroupPreset(name, entries);
    } catch (err) {
      groupsError.value = commandErrorMessage(err);
      throw err;
    }
  }

  async function deleteGroup(name: string): Promise<void> {
    groupsError.value = null;
    try {
      groups.value = await deleteGroupPreset(name);
    } catch (err) {
      groupsError.value = commandErrorMessage(err);
    }
  }

  /**
   * Restores each entry's saved instance identity and shows it. A live
   * same-template instance with the saved id is updated in place; an id held
   * by another template (or claimed earlier in this run) gets a fresh id and
   * is flagged as a conflict. A failing entry is reported without aborting the
   * rest. The previously active instance is restored afterwards.
   */
  async function startGroup(group: GroupPreset): Promise<GroupStartResult[]> {
    const previousActiveId = instanceStore.activeInstanceId;
    const results: GroupStartResult[] = [];
    const claimed = new Set<string>();
    for (const entry of group.entries) {
      if (!templateStore.getById(entry.template)) {
        results.push({
          template: entry.template,
          ok: false,
          reason: "templateMissing",
        });
        continue;
      }
      const fields = {
        ...templateStore.getDefaults(entry.template),
        ...entry.fields,
      };
      const savedId = entry.instance_id?.trim() || undefined;
      const savedName = entry.custom_name?.trim() || undefined;
      const live = savedId
        ? instanceStore.instances.find((i) => i.id === savedId)
        : undefined;
      const isClaimed = savedId !== undefined && claimed.has(savedId);

      let id: string;
      let created = false;
      let idConflict = false;
      let previous: { fields: Record<string, string>; customName?: string };
      if (live && !isClaimed && live.templateId === entry.template) {
        id = live.id;
        previous = { fields: live.fields, customName: live.customName };
        live.fields = fields;
        if (savedName) live.customName = savedName;
      } else {
        const free = savedId !== undefined && !live && !isClaimed;
        idConflict = savedId !== undefined && !free;
        id = instanceStore.createInstance(
          entry.template,
          free
            ? { id: savedId, customName: savedName }
            : { customName: savedName },
        );
        created = true;
        previous = { fields: {} };
      }
      claimed.add(id);

      const instance = instanceStore.instances.find((i) => i.id === id);
      if (!instance) continue;
      if (created) instance.fields = fields;
      try {
        await instanceStore.showInstance(id, instanceStore.fieldsFor(instance));
        results.push({
          template: entry.template,
          ok: true,
          instanceId: id,
          idConflict,
        });
      } catch {
        if (created) {
          instanceStore.removeInstance(id);
        } else {
          instance.fields = previous.fields;
          instance.customName = previous.customName;
        }
        results.push({
          template: entry.template,
          ok: false,
          reason: "sendFailed",
        });
      }
    }
    instanceStore.activeInstanceId = previousActiveId;
    return results;
  }

  return {
    groups,
    groupsError,
    refreshGroups,
    hasGroup,
    saveFromInstances,
    deleteGroup,
    startGroup,
  };
});
