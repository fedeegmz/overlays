import { defineStore } from "pinia";
import { ref } from "vue";
import { setActiveLocale, toLocaleCode, translate } from "../i18n";
import { commandErrorMessage } from "../lib/errors";
import {
  getConfig,
  setLanguage as setLanguageCommand,
  setOverlaysDir as setOverlaysDirCommand,
} from "../services/configApi";
import { pickOverlaysDir as pickOverlaysDirDialog } from "../services/dialogApi";
import type { AppConfig } from "../types";
import { useTemplateStore } from "./templates";

export const useConfigStore = defineStore("config", () => {
  const templateStore = useTemplateStore();

  const appConfig = ref<AppConfig>({ overlays_dir: null, language: null });
  const configError = ref<string | null>(null);

  async function refreshConfig(): Promise<void> {
    configError.value = null;
    try {
      appConfig.value = await getConfig();
    } catch (err) {
      configError.value = commandErrorMessage(err);
    }
  }

  async function setOverlaysDir(path: string): Promise<void> {
    configError.value = null;
    try {
      appConfig.value = await setOverlaysDirCommand(path);
      await templateStore.refreshTemplates();
    } catch (err) {
      configError.value = commandErrorMessage(err);
      throw err;
    }
  }

  async function setLanguage(lang: string): Promise<void> {
    configError.value = null;
    try {
      appConfig.value = await setLanguageCommand(lang);
      const locale = toLocaleCode(appConfig.value.language);
      if (locale) setActiveLocale(locale);
    } catch (err) {
      configError.value = commandErrorMessage(err);
      throw err;
    }
  }

  async function pickOverlaysDir(): Promise<void> {
    configError.value = null;
    try {
      const selected = await pickOverlaysDirDialog(
        translate("dialog.selectOverlaysDir"),
      );
      if (selected) {
        await setOverlaysDir(selected);
      }
    } catch (err) {
      configError.value = commandErrorMessage(err);
    }
  }

  return {
    appConfig,
    configError,
    refreshConfig,
    setOverlaysDir,
    setLanguage,
    pickOverlaysDir,
  };
});
