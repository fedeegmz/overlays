import { defineStore } from "pinia";
import { ref, watch } from "vue";
import { commandErrorMessage } from "../lib/errors";
import {
  acceptOverlay as acceptOverlayCommand,
  discardOverlay as discardOverlayCommand,
  generateOverlay as generateOverlayCommand,
  listProviderModels,
} from "../services/generatorApi";
import type { GeneratedOverlaySummary } from "../types";
import { useTemplateStore } from "./templates";

export const useGenerateStore = defineStore("generate", () => {
  const templateStore = useTemplateStore();

  const prompt = ref("");
  const name = ref("");
  const provider = ref("");
  const model = ref("");
  const models = ref<string[]>([]);
  const generating = ref(false);
  const saving = ref(false);
  const pending = ref<GeneratedOverlaySummary | null>(null);
  const error = ref<string | null>(null);
  const accepted = ref(false);

  /** Provider switch reloads the model picker (the backend re-validates on generate). */
  watch(provider, async (next) => {
    models.value = [];
    model.value = "";
    if (!next) return;
    try {
      models.value = await listProviderModels(next);
      model.value = models.value[0] ?? "";
    } catch (err) {
      error.value = commandErrorMessage(err);
    }
  });

  async function generate(): Promise<void> {
    const trimmed = prompt.value.trim();
    if (!trimmed || generating.value || !provider.value || !model.value) {
      return;
    }
    error.value = null;
    accepted.value = false;
    generating.value = true;
    try {
      pending.value = await generateOverlayCommand(
        provider.value,
        model.value,
        name.value.trim(),
        trimmed,
      );
    } catch (err) {
      error.value = commandErrorMessage(err);
    } finally {
      generating.value = false;
    }
  }

  async function accept(editedName: string): Promise<void> {
    if (!pending.value || saving.value) return;
    error.value = null;
    saving.value = true;
    try {
      await acceptOverlayCommand(pending.value.staging_id, editedName.trim());
      pending.value = null;
      accepted.value = true;
      await templateStore.refreshTemplates();
    } catch (err) {
      error.value = commandErrorMessage(err);
    } finally {
      saving.value = false;
    }
  }

  async function discard(): Promise<void> {
    if (!pending.value || saving.value) return;
    error.value = null;
    saving.value = true;
    try {
      await discardOverlayCommand(pending.value.staging_id);
      pending.value = null;
    } catch (err) {
      error.value = commandErrorMessage(err);
    } finally {
      saving.value = false;
    }
  }

  function reset(): void {
    pending.value = null;
    error.value = null;
    accepted.value = false;
  }

  return {
    prompt,
    name,
    provider,
    model,
    models,
    generating,
    saving,
    pending,
    error,
    accepted,
    generate,
    accept,
    discard,
    reset,
  };
});
