<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { LOCALES } from "../i18n";
import { getAppVersion } from "../services/appApi";
import { copyText } from "../services/clipboardApi";
import { useConfigStore } from "../stores/config";
import BooleanField from "./BooleanField.vue";

const configStore = useConfigStore();
const { appConfig, configError, lanUrls } = storeToRefs(configStore);
const {
  pickOverlaysDir,
  setLanguage,
  setUploadsEnabled,
  setUploadDir,
  pickUploadDir,
  refreshLanUrls,
} = configStore;

const uploadDirDraft = ref(appConfig.value.upload_dir ?? "");
watch(
  () => appConfig.value.upload_dir,
  (value) => {
    uploadDirDraft.value = value ?? "";
  },
);

async function commitUploadDir() {
  if (uploadDirDraft.value.trim() === (appConfig.value.upload_dir ?? "")) {
    return;
  }
  await setUploadDir(uploadDirDraft.value);
  // Restore the stored value when the backend rejected the input.
  uploadDirDraft.value = appConfig.value.upload_dir ?? "";
}

const lanUrl = computed(() => lanUrls.value[0] ?? null);
const lanUrlCopied = ref(false);
let copyTimer: ReturnType<typeof setTimeout> | null = null;

async function copyLanUrl() {
  if (!lanUrl.value) return;
  try {
    await copyText(lanUrl.value);
    lanUrlCopied.value = true;
  } catch {
    lanUrlCopied.value = false;
  }
  if (copyTimer) clearTimeout(copyTimer);
  copyTimer = setTimeout(() => {
    lanUrlCopied.value = false;
  }, 1500);
}

const appVersion = ref("");
onMounted(async () => {
  void refreshLanUrls();
  try {
    appVersion.value = await getAppVersion();
  } catch {
    appVersion.value = "";
  }
});

const { t, locale } = useI18n();
</script>

<template>
  <div class="settings-page">
    <div class="page-header">
      <div>
        <div class="page-title">{{ t("settings.title") }}</div>
        <div class="page-subtitle">{{ t("settings.subtitle") }}</div>
      </div>
    </div>

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <div class="settings-row-label">
            {{ t("settings.language.label") }}
          </div>
          <div class="settings-row-desc">
            {{ t("settings.language.description") }}
          </div>
        </div>
        <div class="theme-toggle">
          <button
            v-for="option in LOCALES"
            :key="option.code"
            type="button"
            :class="{ active: locale === option.code }"
            @click="setLanguage(option.code)"
          >
            {{ option.name }}
          </button>
        </div>
      </div>

      <div class="settings-row">
        <div>
          <div class="settings-row-label">
            {{ t("settings.overlaysDir.label") }}
          </div>
          <div class="settings-row-desc">
            {{ t("settings.overlaysDir.description") }}
          </div>
        </div>
        <div class="settings-row-right">
          <span v-if="appConfig.overlays_dir" class="settings-row-value">
            {{ appConfig.overlays_dir }}
          </span>
          <span v-else class="settings-row-value settings-row-empty">
            {{ t("settings.overlaysDir.notConfigured") }}
          </span>
          <button type="button" class="btn" @click="pickOverlaysDir">
            {{ appConfig.overlays_dir ? t('settings.overlaysDir.change') : t('settings.overlaysDir.choose') }}
          </button>
        </div>
      </div>

      <div class="settings-row">
        <div>
          <div class="settings-row-label">
            {{ t("settings.uploads.enabled.label") }}
          </div>
          <div class="settings-row-desc">
            {{ t("settings.uploads.enabled.description") }}
          </div>
        </div>
        <BooleanField
          :model-value="appConfig.uploads_enabled ? 'true' : 'false'"
          @update:model-value="setUploadsEnabled($event === 'true')"
        />
      </div>

      <div class="settings-row">
        <div>
          <div class="settings-row-label">
            {{ t("settings.uploads.dir.label") }}
          </div>
          <div class="settings-row-desc">
            {{ t("settings.uploads.dir.description") }}
          </div>
          <div class="settings-row-desc">
            {{ t("settings.uploads.dir.variablesHint") }}
          </div>
        </div>
        <div class="settings-row-right">
          <input
            v-model="uploadDirDraft"
            type="text"
            class="settings-input"
            :placeholder="t('settings.uploads.dir.placeholder')"
            spellcheck="false"
            @change="commitUploadDir"
          >
          <button type="button" class="btn" @click="pickUploadDir">
            {{ t("settings.uploads.dir.choose") }}
          </button>
        </div>
      </div>

      <div class="settings-row">
        <div>
          <div class="settings-row-label">
            {{ t("settings.uploads.lanUrl.label") }}
          </div>
          <div class="settings-row-desc">
            {{ t("settings.uploads.lanUrl.description") }}
          </div>
        </div>
        <div class="settings-row-right">
          <span v-if="lanUrl" class="settings-row-value">{{ lanUrl }}</span>
          <span v-else class="settings-row-value settings-row-empty">
            {{ t("settings.uploads.lanUrl.unavailable") }}
          </span>
          <button
            type="button"
            class="btn"
            :disabled="!lanUrl"
            @click="copyLanUrl"
          >
            {{ lanUrlCopied ? t("overlayDetail.copied") : t("settings.uploads.lanUrl.copy") }}
          </button>
        </div>
      </div>
      <p v-if="configError" class="config-error">{{ configError }}</p>
    </div>

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <div class="settings-row-label">
            {{ t("settings.about.label") }}
          </div>
          <div class="settings-row-desc">
            {{ t("settings.about.description") }}
          </div>
        </div>
        <div class="settings-row-right">
          <span v-if="appVersion" class="settings-row-value">
            v{{ appVersion }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
  max-width: 980px;
}

.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 28px;
}

.page-title {
  font-size: 22px;
  font-weight: 650;
  letter-spacing: -0.015em;
}

.page-subtitle {
  font-size: 13.5px;
  color: var(--text-secondary);
  margin-top: 4px;
}

.settings-section {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-panel);
  margin-bottom: 18px;
}

.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px;
  gap: 20px;
}

.settings-row-label {
  font-size: 13.5px;
  font-weight: 600;
}

.settings-row-desc {
  font-size: 12.5px;
  color: var(--text-secondary);
  margin-top: 3px;
}

.settings-row-value {
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 12px;
  color: var(--text-secondary);
  background: var(--surface-sunken);
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  max-width: 340px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.settings-row-empty {
  color: var(--text-faint);
  font-style: italic;
  font-family: inherit;
}

.settings-row-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.theme-toggle {
  display: flex;
  gap: 4px;
  background: var(--surface-sunken);
  padding: 3px;
  border-radius: 8px;
}

.theme-toggle button {
  border: none;
  background: none;
  font-size: 12px;
  font-weight: 600;
  padding: 6px 12px;
  border-radius: 6px;
  cursor: pointer;
  color: var(--text-secondary);
  font-family: inherit;
  transition:
    background 0.12s ease,
    color 0.12s ease;
}

.theme-toggle button.active {
  background: var(--surface);
  color: var(--text);
  box-shadow: 0 1px 2px rgba(24, 24, 27, 0.08);
}

.theme-toggle button:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.settings-input {
  width: 260px;
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-sunken);
  color: var(--text);
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 12px;
}

.config-error {
  padding: 0 18px 14px;
  color: var(--danger);
  font-size: 12.5px;
}
</style>
