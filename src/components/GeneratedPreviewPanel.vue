<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import { useGenerateStore } from "../stores/generate";

const { t } = useI18n();

const generateStore = useGenerateStore();
const { pending, saving, error } = storeToRefs(generateStore);
const { accept, discard } = generateStore;

const FILES = ["overlay.json", "index.html", "style.css", "script.js"] as const;
</script>

<template>
  <section v-if="pending" class="panel" data-testid="generated-preview">
    <div class="panel-header">{{ t("generatePage.staged.title") }}</div>
    <div class="panel-body">
      <dl class="staged-meta">
        <div class="meta-row">
          <dt>{{ t("generatePage.staged.name") }}</dt>
          <dd>{{ pending.name }}</dd>
        </div>
        <div class="meta-row">
          <dt>{{ t("generatePage.staged.directory") }}</dt>
          <dd class="mono">{{ pending.directory }}</dd>
        </div>
      </dl>

      <div class="staged-files">
        <span v-for="file in FILES" :key="file" class="file-chip"
          >{{ file }}</span
        >
      </div>

      <div class="staged-fields">
        <div class="fields-title">{{ t("generatePage.staged.fields") }}</div>
        <div v-for="field in pending.fields" :key="field.key" class="field-row">
          <span class="mono">{{ field.key }}</span>
          <span class="field-type">{{ field.type }}</span>
        </div>
        <p v-if="pending.fields.length === 0" class="fields-empty">—</p>
      </div>

      <p v-if="error" class="gen-error">{{ error }}</p>

      <div class="staged-actions">
        <button type="button" class="btn" :disabled="saving" @click="discard()">
          {{ t("generatePage.discard") }}
        </button>
        <button
          type="button"
          class="btn primary"
          :disabled="saving"
          data-testid="accept-button"
          @click="accept()"
        >
          {{ saving ? t("generatePage.saving") : t("generatePage.accept") }}
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.panel {
  margin-top: 20px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  overflow: hidden;
}

.panel-header {
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
  font-weight: 650;
  font-size: 14px;
}

.panel-body {
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.staged-meta {
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: 13px;
}

.meta-row {
  display: flex;
  gap: 12px;
}

.meta-row dt {
  width: 88px;
  flex-shrink: 0;
  color: var(--text-faint);
}

.meta-row dd {
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mono {
  font-family: var(--font-mono);
  font-size: 12.5px;
}

.staged-files {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.file-chip {
  padding: 3px 9px;
  border-radius: 999px;
  background: var(--surface-sunken);
  border: 1px solid var(--border);
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--text-secondary);
}

.staged-fields {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.fields-title {
  font-size: 12px;
  font-weight: 650;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-faint);
}

.field-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  background: var(--surface-sunken);
  font-size: 13px;
}

.field-type {
  color: var(--text-faint);
  font-size: 12px;
}

.fields-empty {
  margin: 0;
  color: var(--text-faint);
}

.staged-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
