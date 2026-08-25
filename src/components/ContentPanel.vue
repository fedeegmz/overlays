<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { commandErrorMessage } from "../lib/errors";
import { useInstanceStore } from "../stores/instances";
import { usePresetStore } from "../stores/presets";
import ColorField from "./ColorField.vue";

const { t } = useI18n();
const instanceStore = useInstanceStore();
const presetStore = usePresetStore();
const { activeInstance, activeTemplate } = storeToRefs(instanceStore);
const { presets } = storeToRefs(presetStore);
const { savePreset, applyPreset, deletePreset } = presetStore;

const presetName = ref("");
const sending = ref(false);
const sendError = ref<string | null>(null);
const confirmingDelete = ref<string | null>(null);
let confirmTimer: ReturnType<typeof setTimeout> | null = null;

const filteredPresets = computed(() =>
  presets.value.filter((p) => p.template === activeInstance.value?.templateId),
);

const saveDisabled = computed(
  () =>
    !activeInstance.value ||
    !activeTemplate.value ||
    presetName.value.trim().length === 0 ||
    sending.value,
);

function updateField(key: string, value: string) {
  if (!activeInstance.value) return;
  activeInstance.value.fields[key] = value;
}

async function handleSavePreset() {
  if (saveDisabled.value) return;
  sendError.value = null;
  sending.value = true;
  try {
    await savePreset(presetName.value);
    presetName.value = "";
  } catch (err) {
    sendError.value = commandErrorMessage(err);
  } finally {
    sending.value = false;
  }
}

async function handleApply(preset: { name: string }) {
  const found = presets.value.find((p) => p.name === preset.name);
  if (!found) return;
  sendError.value = null;
  sending.value = true;
  try {
    await applyPreset(found);
  } catch (err) {
    sendError.value = commandErrorMessage(err);
  } finally {
    sending.value = false;
  }
}

function handleDelete(name: string) {
  if (confirmingDelete.value !== name) {
    confirmingDelete.value = name;
    if (confirmTimer) clearTimeout(confirmTimer);
    confirmTimer = setTimeout(() => {
      confirmingDelete.value = null;
    }, 2000);
    return;
  }
  if (confirmTimer) clearTimeout(confirmTimer);
  confirmingDelete.value = null;
  void deletePreset(name);
}
</script>

<template>
  <div class="panel">
    <div class="panel-header">{{ t("content.title") }}</div>
    <div class="panel-body">
      <template v-if="activeInstance && activeTemplate">
        <div
          v-for="field in activeTemplate.fields"
          :key="field.key"
          class="field"
        >
          <label class="field-label" :for="`field-${field.key}`"
            >{{ field.label }}</label
          >
          <ColorField
            v-if="field.type === 'color'"
            :model-value="activeInstance.fields[field.key] ?? ''"
            :input-id="`field-${field.key}`"
            :disabled="sending"
            @update:model-value="updateField(field.key, $event)"
          />
          <input
            v-else
            :id="`field-${field.key}`"
            :value="activeInstance.fields[field.key] ?? ''"
            class="field-input"
            type="text"
            :placeholder="field.label"
            :disabled="sending"
            @input="updateField(field.key, ($event.target as HTMLInputElement).value)"
          >
        </div>

        <p v-if="sendError" class="send-error">{{ sendError }}</p>

        <div class="divider"></div>

        <div class="field-label">{{ t("content.presets") }}</div>

        <div v-if="filteredPresets.length > 0" class="preset-list">
          <div
            v-for="preset in filteredPresets"
            :key="preset.name"
            class="preset-row"
          >
            <span class="preset-row-name">{{ preset.name }}</span>
            <div class="preset-row-actions">
              <button
                type="button"
                class="icon-btn"
                :title="t('content.applyPreset')"
                :disabled="sending"
                @click="handleApply(preset)"
              >
                <svg
                  aria-hidden="true"
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <polygon points="5 3 19 12 5 21 5 3" />
                </svg>
              </button>
              <button
                type="button"
                class="icon-btn"
                :class="{ 'confirm-delete': confirmingDelete === preset.name }"
                :title="t('content.deletePreset')"
                @click="handleDelete(preset.name)"
              >
                <svg
                  aria-hidden="true"
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path
                    d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m3 0-1 14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2L4 6"
                  />
                </svg>
              </button>
            </div>
          </div>
        </div>
        <p v-else class="preset-empty">{{ t("content.emptyPresets") }}</p>

        <div class="save-preset-row">
          <input
            v-model="presetName"
            class="field-input"
            type="text"
            :placeholder="t('content.presetNamePlaceholder')"
            :disabled="sending"
          >
          <button
            type="button"
            class="btn"
            :disabled="saveDisabled"
            @click="handleSavePreset"
          >
            {{ t("content.saveCurrent") }}
          </button>
        </div>
      </template>

      <p v-else class="no-template">{{ t("content.selectTemplate") }}</p>
    </div>
  </div>
</template>

<style scoped>
.panel {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-panel);
}

.panel-header {
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
  font-size: 12.5px;
  font-weight: 650;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.panel-body {
  padding: 18px;
}

.send-error {
  color: var(--danger);
  font-size: 12.5px;
  margin-bottom: 10px;
}

.divider {
  height: 1px;
  background: var(--border);
  margin: 18px 0;
}

.preset-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.preset-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 9px 11px;
  border-radius: var(--radius-sm);
  background: var(--surface-sunken);
  font-size: 13px;
}

.preset-row-name {
  font-weight: 550;
}

.preset-row-actions {
  display: flex;
  gap: 4px;
}

.icon-btn.confirm-delete,
.icon-btn.confirm-delete:hover {
  color: var(--danger);
  background: var(--danger-soft);
}

.preset-empty {
  font-size: 12.5px;
  color: var(--text-faint);
  text-align: center;
  padding: 18px 0;
}

.save-preset-row {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.save-preset-row .field-input {
  flex: 1;
}

.no-template {
  color: var(--text-faint);
  font-size: 13.5px;
}
</style>
