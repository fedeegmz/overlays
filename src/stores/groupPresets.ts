import { defineStore } from "pinia";
import { ref } from "vue";
import { commandErrorMessage } from "../lib/errors";
import {
  deleteGroupPreset,
  listGroupPresets,
} from "../services/groupPresetsApi";
import {
  saveGroupFromInstances,
  startGroupPreset,
} from "../services/instancesApi";
import type { GroupPreset, GroupStartResult } from "../types";
import { useInstanceStore } from "./instances";

export const useGroupPresetStore = defineStore("groupPresets", () => {
  const instanceStore = useInstanceStore();

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

  /** Saves every open instance (hidden included); the backend builds the entries. */
  async function saveFromInstances(name: string): Promise<void> {
    groupsError.value = null;
    try {
      await instanceStore.flushAllDrafts();
      groups.value = await saveGroupFromInstances(name);
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
   * Starts a saved group in the backend, which restores each entry's saved
   * instance identity in one step. Selection stays where it was (local state).
   */
  async function startGroup(group: GroupPreset): Promise<GroupStartResult[]> {
    await instanceStore.flushAllDrafts();
    const { results, snapshot } = await startGroupPreset(group.name);
    instanceStore.applySnapshot(snapshot);
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
