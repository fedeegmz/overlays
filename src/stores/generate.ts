import { defineStore } from "pinia";
import { ref } from "vue";
import { commandErrorMessage } from "../lib/errors";
import {
  acceptOverlay as acceptOverlayCommand,
  discardOverlay as discardOverlayCommand,
  generateOverlay as generateOverlayCommand,
} from "../services/generatorApi";
import type { GeneratedOverlaySummary } from "../types";
import { useTemplateStore } from "./templates";

export const useGenerateStore = defineStore("generate", () => {
  const templateStore = useTemplateStore();

  const prompt = ref("");
  const generating = ref(false);
  const saving = ref(false);
  const pending = ref<GeneratedOverlaySummary | null>(null);
  const error = ref<string | null>(null);
  const accepted = ref(false);

  async function generate(): Promise<void> {
    const trimmed = prompt.value.trim();
    if (!trimmed || generating.value) return;
    error.value = null;
    accepted.value = false;
    generating.value = true;
    try {
      pending.value = await generateOverlayCommand(trimmed);
    } catch (err) {
      error.value = commandErrorMessage(err);
    } finally {
      generating.value = false;
    }
  }

  async function accept(): Promise<void> {
    if (!pending.value || saving.value) return;
    error.value = null;
    saving.value = true;
    try {
      await acceptOverlayCommand(pending.value.staging_id);
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
