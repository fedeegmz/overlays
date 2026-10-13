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
   * Creates a new instance per entry and shows it. A failing entry is
   * reported without aborting the rest. The previously active instance is
   * restored afterwards.
   */
  async function startGroup(group: GroupPreset): Promise<GroupStartResult[]> {
    const previousActiveId = instanceStore.activeInstanceId;
    const results: GroupStartResult[] = [];
    for (const entry of group.entries) {
      if (!templateStore.getById(entry.template)) {
        results.push({
          template: entry.template,
          ok: false,
          reason: "templateMissing",
        });
        continue;
      }
      const id = instanceStore.createInstance(entry.template);
      const instance = instanceStore.instances.find((i) => i.id === id);
      if (!instance) continue;
      instance.fields = {
        ...templateStore.getDefaults(entry.template),
        ...entry.fields,
      };
      try {
        await instanceStore.showInstance(id, instanceStore.fieldsFor(instance));
        results.push({ template: entry.template, ok: true, instanceId: id });
      } catch {
        instanceStore.removeInstance(id);
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
