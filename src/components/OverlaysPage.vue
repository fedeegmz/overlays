<script setup lang="ts">
import { storeToRefs } from "pinia";
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useInstanceStore } from "../stores/instances";
import { useTemplateStore } from "../stores/templates";
import OverlayGrid from "./OverlayGrid.vue";

const { t } = useI18n();
const templateStore = useTemplateStore();
const instanceStore = useInstanceStore();
const { templates, templatesError } = storeToRefs(templateStore);
const { createInstance } = instanceStore;

const reloading = ref(false);

const emit = defineEmits<{
  openDetail: [];
}>();

function handleSelect(id: string) {
  createInstance(id);
  emit("openDetail");
}

async function handleReload() {
  reloading.value = true;
  try {
    await templateStore.refreshTemplates();
  } finally {
    reloading.value = false;
  }
}
</script>

<template>
  <div class="overlays-page">
    <div class="page-header">
      <div>
        <div class="page-title">Overlays</div>
        <div class="page-subtitle">{{ t("overlaysPage.subtitle") }}</div>
      </div>
      <button
        type="button"
        class="reload-btn"
        :disabled="reloading || templates === null"
        @click="handleReload"
      >
        <svg
          aria-hidden="true"
          class="reload-icon"
          :class="{ spin: reloading }"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M21 12a9 9 0 1 1-2.64-6.36L21 8" />
          <path d="M21 3v5h-5" />
        </svg>
        {{ t("overlaysPage.reload") }}
      </button>
    </div>

    <p v-if="templatesError" class="error">{{ templatesError }}</p>
    <p v-else-if="templates === null" class="muted">
      {{ t("overlaysPage.loading") }}
    </p>
    <p v-else-if="templates.length === 0" class="muted">
      {{ t("overlaysPage.empty") }}
    </p>
    <OverlayGrid v-else :templates="templates" @select="handleSelect" />
  </div>
</template>

<style scoped>
.overlays-page {
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

.reload-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface);
  color: var(--text-secondary);
  font-size: 12.5px;
  font-weight: 550;
  cursor: pointer;
}

.reload-icon.spin {
  animation: reload-icon-spin 0.8s linear infinite;
}

@keyframes reload-icon-spin {
  to {
    transform: rotate(360deg);
  }
}

.reload-btn:hover:not(:disabled) {
  color: var(--text-primary);
  border-color: var(--text-faint);
}

.reload-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.error {
  color: var(--danger);
  font-size: 13.5px;
}

.muted {
  color: var(--text-faint);
  font-size: 13.5px;
}
</style>
