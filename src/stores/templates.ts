import { defineStore } from "pinia";
import { ref } from "vue";

import { commandErrorMessage } from "../lib/errors";
import { listTemplates } from "../services/templatesApi";
import type { TemplateInfo } from "../types";

export const useTemplateStore = defineStore("templates", () => {
  const templates = ref<TemplateInfo[] | null>(null);
  const templatesError = ref<string | null>(null);

  async function refreshTemplates(): Promise<void> {
    templatesError.value = null;
    try {
      const manifest = await listTemplates();
      templates.value = manifest.templates;
    } catch (err) {
      templatesError.value = commandErrorMessage(err);
    }
  }

  function getById(templateId: string): TemplateInfo | undefined {
    return templates.value?.find((t) => t.id === templateId);
  }

  function getDefaults(templateId: string): Record<string, string> {
    const template = getById(templateId);
    const fields: Record<string, string> = {};
    if (!template) return fields;
    for (const f of template.fields) {
      if (f.default !== undefined) fields[f.key] = f.default;
    }
    return fields;
  }

  return {
    templates,
    templatesError,
    refreshTemplates,
    getById,
    getDefaults,
  };
});
