<script setup lang="ts">
import { storeToRefs } from "pinia";
import { onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { useConfigStore } from "../stores/config";
import { useGenerateStore } from "../stores/generate";
import GeneratedPreviewPanel from "./GeneratedPreviewPanel.vue";

const { t } = useI18n();

const configStore = useConfigStore();
const generateStore = useGenerateStore();

const { providerKeys } = storeToRefs(configStore);
const { prompt, generating, error, accepted } = storeToRefs(generateStore);
const { generate } = generateStore;

onMounted(() => {
  void configStore.refreshProviderKeys();
});

const canGenerate = () => providerKeys.value.length > 0;
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>{{ t("generatePage.title") }}</h1>
      <p class="page-subtitle">{{ t("generatePage.subtitle") }}</p>
    </header>

    <p v-if="!canGenerate()" class="gen-gate" data-testid="generate-gate">
      {{ t("generatePage.noProviderKey") }}
    </p>
    <p v-if="accepted" class="gen-success" data-testid="generate-accepted">
      {{ t("generatePage.accepted") }}
    </p>
    <p v-if="error" class="gen-error" data-testid="generate-error">
      {{ error }}
    </p>

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
        :disabled="generating || !canGenerate()"
        data-testid="generate-prompt"
      ></textarea>
      <div class="prompt-actions">
        <button
          type="button"
          class="btn primary"
          :disabled="generating || !canGenerate() || prompt.trim() === ''"
          data-testid="generate-button"
          @click="generate"
        >
          {{ generating ? t("generatePage.generating") : t("generatePage.generate") }}
        </button>
      </div>
    </div>

    <GeneratedPreviewPanel />
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

.gen-gate,
.gen-error,
.gen-success {
  padding: 10px 14px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  margin: 0 0 16px;
}

.gen-gate {
  background: var(--accent-soft);
  color: var(--accent-hover);
}

.gen-success {
  background: var(--success-soft);
  color: var(--success);
}

.gen-error {
  background: var(--danger-soft);
  color: var(--danger);
}

.prompt-box {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.prompt-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
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
