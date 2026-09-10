<script setup lang="ts">
import { storeToRefs } from "pinia";
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { LOCALES } from "../i18n";
import { useConfigStore } from "../stores/config";
import GenericModal from "./GenericModal.vue";

const configStore = useConfigStore();
const { appConfig, configError, providerKeys, keyringAvailable, keysError } =
  storeToRefs(configStore);
const {
  pickOverlaysDir,
  setLanguage,
  refreshProviderKeys,
  addProviderKey,
  deleteProviderKey,
} = configStore;

const { t, locale } = useI18n();

const addKeyModalOpen = ref(false);
const newKey = ref("");

function openAddKeyModal(): void {
  newKey.value = "";
  addKeyModalOpen.value = true;
}

async function submitNewKey(): Promise<void> {
  const key = newKey.value.trim();
  if (!key) return;
  try {
    await addProviderKey("anthropic", key);
    addKeyModalOpen.value = false;
  } catch {
    // error surfaced via keysError
  }
}

onMounted(() => {
  refreshProviderKeys();
});
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
      <p v-if="configError" class="config-error">{{ configError }}</p>
    </div>

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <div class="settings-row-label">
            {{ t("settings.apiKeys.label") }}
          </div>
          <div class="settings-row-desc">
            {{ t("settings.apiKeys.description") }}
          </div>
        </div>
        <div class="settings-row-right">
          <template v-if="providerKeys.length > 0">
            <span
              v-for="key in providerKeys"
              :key="key.provider"
              class="settings-row-value"
              data-testid="provider-key-mask"
            >
              {{ key.last4 ? `sk-…${key.last4}` : "" }}
            </span>
            <button
              type="button"
              class="btn"
              @click="deleteProviderKey('anthropic')"
            >
              {{ t("settings.apiKeys.delete") }}
            </button>
          </template>
          <span v-else class="settings-row-value settings-row-empty">
            {{ t("settings.apiKeys.notConfigured") }}
          </span>
          <button type="button" class="btn" @click="openAddKeyModal">
            {{ t("settings.apiKeys.add") }}
          </button>
        </div>
      </div>
      <p v-if="keyringAvailable === false" class="config-error">
        {{ t("settings.apiKeys.keyringUnavailable") }}
      </p>
      <p v-if="keysError" class="config-error">{{ keysError }}</p>
    </div>

    <GenericModal
      :title="t('settings.apiKeys.addTitle')"
      :open="addKeyModalOpen"
      @close="addKeyModalOpen = false"
    >
      <label class="key-form-label" for="new-api-key">
        {{ t("settings.apiKeys.provider") }}: Anthropic
      </label>
      <input
        id="new-api-key"
        v-model="newKey"
        type="password"
        class="key-form-input"
        :placeholder="t('settings.apiKeys.keyPlaceholder')"
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        data-testid="new-api-key-input"
        @keyup.enter="submitNewKey"
      >
      <template #footer>
        <button type="button" @click="addKeyModalOpen = false">
          {{ t("settings.apiKeys.cancel") }}
        </button>
        <button
          type="button"
          class="primary"
          :disabled="!newKey.trim()"
          @click="submitNewKey"
        >
          {{ t("settings.apiKeys.save") }}
        </button>
      </template>
    </GenericModal>

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <div class="settings-row-label">
            {{ t("settings.appearance.label") }}
          </div>
          <div class="settings-row-desc">
            {{ t("settings.appearance.description") }}
          </div>
        </div>
        <div class="theme-toggle">
          <button type="button" class="active">
            {{ t("settings.appearance.light") }}
          </button>
          <button type="button" disabled>
            {{ t("settings.appearance.dark") }}
          </button>
          <button type="button" disabled>
            {{ t("settings.appearance.system") }}
          </button>
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

.config-error {
  padding: 0 18px 14px;
  color: var(--danger);
  font-size: 12.5px;
}

.key-form-label {
  display: block;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.key-form-input {
  width: 100%;
  box-sizing: border-box;
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 12.5px;
  padding: 9px 11px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-sunken);
  color: var(--text);
  outline: none;
}

.key-form-input:focus {
  border-color: var(--accent);
}
</style>
