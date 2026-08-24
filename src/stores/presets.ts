import { defineStore } from "pinia";
import { ref } from "vue";

import { commandErrorMessage } from "../lib/errors";
import {
  deletePreset as deletePresetCommand,
  listPresets,
  savePreset as savePresetCommand,
} from "../services/presetsApi";
import { sendOverlayUpdate } from "../services/overlayApi";
import { translate } from "../i18n";
import type { Preset } from "../types";
import { useInstanceStore } from "./instances";
import { useTemplateStore } from "./templates";

export const usePresetStore = defineStore("presets", () => {
  const instanceStore = useInstanceStore();
  const templateStore = useTemplateStore();

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

  async function applyPreset(preset: Preset): Promise<void> {
    const instance = instanceStore.activeInstance;
    if (!instance) return;
    if (!templateStore.getById(preset.template)) {
      presetsError.value = translate("errors.templateMissing", {
        template: preset.template,
      });
      return;
    }
    instance.fields = { ...preset.fields };
    await sendOverlayUpdate({
      instanceId: instance.id,
      template: instance.templateId,
      action: "show",
      fields: { ...preset.fields },
    });
    instance.visible = true;
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
