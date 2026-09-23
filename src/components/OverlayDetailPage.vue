<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { copyText } from "../services/clipboardApi";
import { useInstanceStore } from "../stores/instances";
import ContentPanel from "./ContentPanel.vue";
import PreviewPanel from "./PreviewPanel.vue";

const { t } = useI18n();
const { activeInstance, activeTemplate, overlayUrl } = storeToRefs(
  useInstanceStore(),
);

const emit = defineEmits<{
  back: [];
}>();

const copied = ref(false);
const copyError = ref(false);
let copyTimer: ReturnType<typeof setTimeout> | null = null;

const instanceLabel = computed(() => {
  if (!activeInstance.value) return "";
  return activeInstance.value.id.slice(0, 6);
});

async function handleCopyUrl() {
  if (!overlayUrl.value) return;
  try {
    await copyText(overlayUrl.value);
    copyError.value = false;
    copied.value = true;
  } catch {
    copyError.value = true;
  }
  if (copyTimer) clearTimeout(copyTimer);
  copyTimer = setTimeout(() => {
    copied.value = false;
    copyError.value = false;
  }, 2000);
}
</script>

<template>
  <div class="detail-page" v-if="activeInstance && activeTemplate">
    <!-- biome-ignore lint/a11y/useSemanticElements: kept as div to inherit breadcrumb styling without extra CSS reset -->
    <div
      class="breadcrumb"
      role="button"
      tabindex="0"
      @click="emit('back')"
      @keydown.enter.prevent="emit('back')"
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
        <path d="M19 12H5M12 19l-7-7 7-7" />
      </svg>
      Overlays / {{ activeTemplate.name }}
      <span class="breadcrumb-id">{{ instanceLabel }}</span>
    </div>

    <div class="page-header">
      <div>
        <div class="page-title">
          {{ activeTemplate.name }}
          <span class="page-id">{{ instanceLabel }}</span>
        </div>
        <div v-if="overlayUrl" class="page-subtitle">
          <span class="page-subtitle-url">{{ overlayUrl }}</span>
          <button
            type="button"
            class="icon-btn copy-btn"
            :class="{ 'copy-success': copied }"
            :title="copied ? t('overlayDetail.copied') : t('overlayDetail.copyUrl')"
            :aria-label="copied ? t('overlayDetail.copied') : t('overlayDetail.copyUrl')"
            @click="handleCopyUrl"
          >
            <svg
              v-if="copied"
              aria-hidden="true"
              width="13"
              height="13"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M20 6 9 17l-5-5" />
            </svg>
            <svg
              v-else
              aria-hidden="true"
              width="13"
              height="13"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
              <path
                d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
              />
            </svg>
          </button>
        </div>
        <p v-if="copyError" class="copy-error">
          {{ t("overlayDetail.copyFailed") }}
        </p>
      </div>
    </div>

    <div class="detail-layout">
      <PreviewPanel />
      <ContentPanel />
    </div>
  </div>
</template>

<style scoped>
.detail-page {
  max-width: 980px;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  color: var(--text-faint);
  margin-bottom: 14px;
  cursor: pointer;
  width: fit-content;
}

.breadcrumb:hover {
  color: var(--text-secondary);
}

.breadcrumb-id {
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 11px;
  color: var(--text-faint);
  background: var(--surface-sunken);
  padding: 1px 5px;
  border-radius: 3px;
}

.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 20px;
}

.page-title {
  font-size: 22px;
  font-weight: 650;
  letter-spacing: -0.015em;
  display: flex;
  align-items: center;
  gap: 10px;
}

.page-id {
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-faint);
  background: var(--surface-sunken);
  padding: 2px 7px;
  border-radius: 4px;
}

.page-subtitle {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
  min-width: 0;
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 12px;
  color: var(--text-secondary);
}

.page-subtitle-url {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.copy-btn {
  flex-shrink: 0;
}

.copy-btn.copy-success {
  color: var(--success);
}

.copy-error {
  color: var(--danger);
  font-size: 12.5px;
  margin-top: 6px;
}

.detail-layout {
  display: grid;
  grid-template-columns: 1.1fr 0.9fr;
  gap: 24px;
  align-items: start;
}
</style>
