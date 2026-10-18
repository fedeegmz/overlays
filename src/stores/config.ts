import { defineStore } from "pinia";
import { ref } from "vue";
import { setActiveLocale, toLocaleCode, translate } from "../i18n";
import { commandErrorMessage } from "../lib/errors";
import {
  getConfig,
  setLanguage as setLanguageCommand,
  setOverlaysDir as setOverlaysDirCommand,
  setUploadDir as setUploadDirCommand,
  setUploadsEnabled as setUploadsEnabledCommand,
} from "../services/configApi";
import { pickOverlaysDir as pickOverlaysDirDialog } from "../services/dialogApi";
import { getServerStatus } from "../services/overlayApi";
import type { AppConfig } from "../types";
import { useTemplateStore } from "./templates";

export const useConfigStore = defineStore("config", () => {
  const templateStore = useTemplateStore();

  const appConfig = ref<AppConfig>({
    overlays_dir: null,
    language: null,
    uploads_enabled: true,
    upload_dir: null,
  });
  const configError = ref<string | null>(null);
  const lanUrls = ref<string[]>([]);

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

  async function refreshLanUrls(): Promise<void> {
    const status = await getServerStatus();
    lanUrls.value = status?.lan_urls ?? [];
  }

  async function setUploadsEnabled(enabled: boolean): Promise<void> {
    configError.value = null;
    try {
      appConfig.value = await setUploadsEnabledCommand(enabled);
    } catch (err) {
      configError.value = commandErrorMessage(err);
    }
  }

  /** Blank input clears the template; the backend validates and keeps the previous value on error. */
  async function setUploadDir(template: string): Promise<void> {
    configError.value = null;
    const trimmed = template.trim();
    try {
      appConfig.value = await setUploadDirCommand(
        trimmed === "" ? null : trimmed,
      );
    } catch (err) {
      configError.value = commandErrorMessage(err);
    }
  }

  async function pickUploadDir(): Promise<void> {
    configError.value = null;
    try {
      const selected = await pickOverlaysDirDialog(
        translate("settings.uploads.dir.choose"),
      );
      if (selected) {
        await setUploadDir(selected);
      }
    } catch (err) {
      configError.value = commandErrorMessage(err);
    }
  }

  return {
    appConfig,
    configError,
    lanUrls,
    refreshConfig,
    refreshLanUrls,
    setUploadsEnabled,
    setUploadDir,
    pickUploadDir,
    setOverlaysDir,
    setLanguage,
    pickOverlaysDir,
  };
});
