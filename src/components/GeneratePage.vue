<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { isKebabName } from "../lib/names";
import { useConfigStore } from "../stores/config";
import { useGenerateStore } from "../stores/generate";
import GateModal from "./GateModal.vue";
import GeneratedPreviewPanel from "./GeneratedPreviewPanel.vue";

const emit = defineEmits<{
  navigate: [page: "overlays" | "settings" | "generate"];
}>();

const { t } = useI18n();

const configStore = useConfigStore();
const generateStore = useGenerateStore();

const { providerKeys, keyringAvailable, appConfig } = storeToRefs(configStore);
const { prompt, name, provider, model, models, generating, error, accepted } =
  storeToRefs(generateStore);
const { generate } = generateStore;

const ready = ref(false);
const dismissed = ref(false);

onMounted(async () => {
  await Promise.all([
    configStore.refreshProviderKeys(),
    configStore.refreshConfig(),
  ]);
  ready.value = true;
});

/** G2/G3 gate: noDir before keyring before noKey, only after config loads. */
const gate = computed<"noDir" | "keyring" | "noKey" | null>(() => {
  if (!ready.value) return null;
  if (!appConfig.value.overlays_dir) return "noDir";
  if (keyringAvailable.value === false) return "keyring";
  if (providerKeys.value.length === 0) return "noKey";
  return null;
});

watch(gate, () => {
  dismissed.value = false;
});

const gateOpen = computed(() => gate.value !== null && !dismissed.value);
const enabled = computed(() => gate.value === null);
const nameValid = computed(() => isKebabName(name.value));
const canGenerate = computed(
  () =>
    enabled.value &&
    !generating.value &&
    prompt.value.trim() !== "" &&
    provider.value !== "" &&
    model.value !== "" &&
    nameValid.value,
);

// Default the provider to the first configured key once they load.
watch(providerKeys, (keys) => {
  if (keys.length > 0 && provider.value === "") {
    provider.value = keys[0].provider;
  }
});

function openSettings(): void {
  dismissed.value = true;
  emit("navigate", "settings");
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>{{ t("generatePage.title") }}</h1>
      <p class="page-subtitle">{{ t("generatePage.subtitle") }}</p>
    </header>

    <p v-if="accepted" class="gen-success" data-testid="generate-accepted">
      {{ t("generatePage.accepted") }}
    </p>
    <p v-if="error" class="gen-error" data-testid="generate-error">
      {{ error }}
    </p>

    <div class="gen-form" :class="{ 'is-gated': !enabled }">
      <div class="form-row">
        <div class="form-field">
          <label class="prompt-label" for="generate-provider">
            {{ t("generatePage.provider") }}
          </label>
          <select
            id="generate-provider"
            v-model="provider"
            class="form-select"
            :disabled="!enabled || generating || providerKeys.length === 0"
            data-testid="generate-provider"
          >
            <option
              v-for="key in providerKeys"
              :key="key.provider"
              :value="key.provider"
            >
              {{ key.provider }}
            </option>
          </select>
        </div>
        <div class="form-field">
          <label class="prompt-label" for="generate-model">
            {{ t("generatePage.model") }}
          </label>
          <select
            id="generate-model"
            v-model="model"
            class="form-select"
            :disabled="!enabled || generating || models.length === 0"
            data-testid="generate-model"
          >
            <option v-for="m in models" :key="m" :value="m">
              {{ m }}
            </option>
          </select>
        </div>
      </div>

      <div class="form-field">
        <label class="prompt-label" for="generate-name">
          {{ t("generatePage.nameLabel") }}
        </label>
        <input
          id="generate-name"
          v-model="name"
          class="form-input"
          maxlength="64"
          :disabled="!enabled || generating"
          :placeholder="t('generatePage.namePlaceholder')"
          data-testid="generate-name"
        >
        <p
          class="form-hint"
          :class="{ 'hint-error': name.trim() !== '' && !nameValid }"
        >
          {{ name.trim() === "" || nameValid
              ? t("generatePage.nameHint")
              : t("generatePage.nameInvalid") }}
        </p>
      </div>

      <div class="prompt-box">
        <label class="prompt-label" for="generate-prompt">
          {{ t("generatePage.promptLabel") }}
        </label>
        <textarea
          id="generate-prompt"
          v-model="prompt"
          class="prompt-input"
          rows="4"
          :placeholder="t('generatePage.promptPlaceholder')"
          :disabled="generating || !enabled"
          data-testid="generate-prompt"
        ></textarea>
        <div class="prompt-actions">
          <button
            type="button"
            class="btn primary"
            :disabled="!canGenerate"
            data-testid="generate-button"
            @click="generate"
          >
            {{ generating ? t("generatePage.generating") : t("generatePage.generate") }}
          </button>
        </div>
      </div>
    </div>

    <GeneratedPreviewPanel />

    <GateModal
      :gate="gateOpen ? gate : null"
      @close="dismissed = true"
      @open-settings="openSettings"
    />
  </div>
</template>

<style scoped>
.page-header {
  margin-bottom: 20px;
}

.page-header h1 {
  font-size: 21px;
  font-weight: 700;
  margin: 0 0 4px;
  letter-spacing: -0.01em;
}

.page-subtitle {
  margin: 0;
  color: var(--text-secondary);
  font-size: 13.5px;
}

.gen-error,
.gen-success {
  padding: 10px 14px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  margin: 0 0 16px;
  white-space: pre-line;
}

.gen-success {
  background: var(--success-soft);
  color: var(--success);
}

.gen-error {
  background: var(--danger-soft);
  color: var(--danger);
}

.gen-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.is-gated {
  opacity: 0.6;
}

.form-row {
  display: flex;
  gap: 14px;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: 1;
}

.prompt-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
}

.form-select,
.form-input {
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  font-family: inherit;
  font-size: 13px;
}

.form-input {
  font-family: "SF Mono", "JetBrains Mono", monospace;
}

.form-select:focus,
.form-input:focus {
  outline: none;
  border-color: var(--accent-hover);
}

.form-hint {
  margin: 0;
  font-size: 11.5px;
  color: var(--text-faint);
}

.hint-error {
  color: var(--danger);
}

.prompt-box {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.prompt-input {
  resize: vertical;
  min-height: 96px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  font-family: inherit;
  font-size: 13.5px;
  line-height: 1.55;
}

.prompt-input:focus {
  outline: none;
  border-color: var(--accent-hover);
}

.prompt-actions {
  display: flex;
  justify-content: flex-end;
}
</style>
