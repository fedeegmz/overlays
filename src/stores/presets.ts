import { defineStore } from "pinia";
import { ref } from "vue";
import { commandErrorMessage } from "../lib/errors";
import {
  deletePreset as deletePresetCommand,
  listPresets,
  savePreset as savePresetCommand,
} from "../services/presetsApi";
import type { Preset } from "../types";
import { useInstanceStore } from "./instances";

export const usePresetStore = defineStore("presets", () => {
  const instanceStore = useInstanceStore();

  const presets = ref<Preset[]>([]);
  const presetsError = ref<string | null>(null);

  async function refreshPresets(): Promise<void> {
    presetsError.value = null;
    try {
      presets.value = await listPresets();
    } catch (err) {
      presetsError.value = commandErrorMessage(err);
    }
  }

  async function savePreset(name: string): Promise<void> {
    const trimmed = name.trim();
    if (!trimmed) return;
    const instance = instanceStore.activeInstance;
    const template = instanceStore.activeTemplate;
    if (!instance || !template) return;
    presetsError.value = null;
    try {
      presets.value = await savePresetCommand({
        name: trimmed,
        template: instance.templateId,
        fields: instanceStore.buildFields(),
      });
    } catch (err) {
      presetsError.value = commandErrorMessage(err);
      throw err;
    }
  }

  /** Applies the preset to the active instance and shows it (backend-side). */
  async function applyPreset(preset: Preset): Promise<void> {
    const instance = instanceStore.activeInstance;
    if (!instance) return;
    await instanceStore.applyPreset(instance.id, preset.name);
  }

  async function deletePreset(name: string): Promise<void> {
    presetsError.value = null;
    try {
      presets.value = await deletePresetCommand(name);
    } catch (err) {
      presetsError.value = commandErrorMessage(err);
    }
  }

  return {
    presets,
    presetsError,
    refreshPresets,
    savePreset,
    applyPreset,
    deletePreset,
  };
});
