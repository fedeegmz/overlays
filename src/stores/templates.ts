import { defineStore } from "pinia";
import { ref } from "vue";

import { commandErrorMessage } from "../lib/errors";
import { listTemplates } from "../services/templatesApi";
import type { TemplateInfo } from "../types";

export const useTemplateStore = defineStore("templates", () => {
  const templates = ref<TemplateInfo[] | null>(null);
  const folders = ref<string[]>([]);
  const templatesError = ref<string | null>(null);

  async function refreshTemplates(): Promise<void> {
    templatesError.value = null;
    try {
      const manifest = await listTemplates();
      templates.value = manifest.templates;
      folders.value = manifest.folders ?? [];
    } catch (err) {
      templatesError.value = commandErrorMessage(err);
    }
  }

  function getById(templateId: string): TemplateInfo | undefined {
    return templates.value?.find((t) => t.id === templateId);
  }

  return {
    templates,
    folders,
    templatesError,
    refreshTemplates,
    getById,
  };
});
