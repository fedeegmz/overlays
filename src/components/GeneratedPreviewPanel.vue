<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { isKebabName } from "../lib/names";
import { useGenerateStore } from "../stores/generate";

const { t } = useI18n();

const generateStore = useGenerateStore();
const { pending, saving, error } = storeToRefs(generateStore);
const { accept, discard } = generateStore;

/**
 * Escaped code-only preview (D14) — never a live render: CSP is null and
 * the files are untrusted LLM output, so the contents go through Vue's
 * default escaping into a `<pre>`.
 */
const FILE_TABS = [
  "overlay_json",
  "index_html",
  "style_css",
  "script_js",
] as const;
type FileKey = (typeof FILE_TABS)[number];

const FILE_LABELS: Record<FileKey, string> = {
  overlay_json: "overlay.json",
  index_html: "index.html",
  style_css: "style.css",
  script_js: "script.js",
};

const activeFile = ref<FileKey>("overlay_json");
const editedName = ref("");

watch(pending, (p) => {
  editedName.value = p?.name ?? "";
  activeFile.value = "overlay_json";
});

/** Client-side kebab mirror (D8) — the backend re-validates on accept. */
const nameValid = computed(() => isKebabName(editedName.value));

function fileContent(key: FileKey): string {
  const files = pending.value?.files;
  if (!files) return "";
  return files[key];
}
</script>

<template>
  <section v-if="pending" class="panel" data-testid="generated-preview">
    <div class="panel-header">{{ t("generatePage.staged.title") }}</div>
    <div class="panel-body">
      <dl class="staged-meta">
        <div class="meta-row">
          <dt>{{ t("generatePage.staged.directory") }}</dt>
          <dd class="mono">{{ pending.directory }}</dd>
        </div>
      </dl>

      <div class="staged-name">
        <label class="fields-title" for="edited-name">
          {{ t("generatePage.staged.nameEdit") }}
        </label>
        <input
          id="edited-name"
          v-model="editedName"
          class="form-input"
          maxlength="64"
          :disabled="saving"
          :aria-invalid="!nameValid"
          data-testid="generate-name-input"
        >
        <p
          class="form-hint"
          :class="{ 'hint-error': !nameValid }"
          data-testid="name-hint"
        >
          {{ nameValid
              ? t("generatePage.staged.nameEditHint")
              : t("generatePage.staged.nameEditInvalid") }}
        </p>
      </div>

      <div
        class="file-tabs"
        role="tablist"
        :aria-label="t('generatePage.staged.files')"
      >
        <button
          v-for="key in FILE_TABS"
          :key="key"
          type="button"
          role="tab"
          :aria-selected="activeFile === key"
          :class="{ active: activeFile === key }"
          :data-testid="`file-tab-${FILE_LABELS[key]}`"
          @click="activeFile = key"
        >
          {{ FILE_LABELS[key] }}
        </button>
      </div>
      <pre class="file-view" data-testid="file-view"><code>{{
        fileContent(activeFile)
      }}</code></pre>

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
          :disabled="saving || !nameValid"
          data-testid="accept-button"
          @click="accept(editedName)"
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

.staged-name {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-input {
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 12.5px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-sunken);
  color: var(--text);
  outline: none;
}

.form-input:focus {
  border-color: var(--accent);
}

.form-input[aria-invalid="true"] {
  border-color: var(--danger);
}

.form-hint {
  margin: 0;
  font-size: 11.5px;
  color: var(--text-faint);
}

.hint-error {
  color: var(--danger);
}

.file-tabs {
  display: flex;
  gap: 4px;
  border-bottom: 1px solid var(--border);
}

.file-tabs button {
  border: none;
  background: none;
  font-family: var(--font-mono);
  font-size: 11.5px;
  padding: 7px 10px;
  border-radius: 6px 6px 0 0;
  color: var(--text-secondary);
  cursor: pointer;
  border-bottom: 2px solid transparent;
}

.file-tabs button.active {
  color: var(--text);
  border-bottom-color: var(--accent);
}

.file-view {
  margin: 0;
  padding: 12px 14px;
  background: var(--surface-sunken);
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  overflow: auto;
  max-height: 320px;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.5;
  white-space: pre;
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

.gen-error {
  padding: 10px 14px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  background: var(--danger-soft);
  color: var(--danger);
  white-space: pre-line;
}

.staged-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
