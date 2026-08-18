<script setup lang="ts">
import { computed } from "vue";
import { useOverlayControl } from "../composables/useOverlayControl";
import PreviewPanel from "./PreviewPanel.vue";
import ContentPanel from "./ContentPanel.vue";

const { activeInstance, activeTemplate, overlayUrl } = useOverlayControl();

const emit = defineEmits<{
  back: [];
}>();

const instanceLabel = computed(() => {
  if (!activeInstance.value) return "";
  return activeInstance.value.id.slice(0, 6);
});
</script>

<template>
  <div class="detail-page" v-if="activeInstance && activeTemplate">
    <div class="breadcrumb" @click="emit('back')">
      <svg
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
        <div v-if="overlayUrl" class="page-subtitle">{{ overlayUrl }}</div>
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
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 4px;
}

.detail-layout {
  display: grid;
  grid-template-columns: 1.1fr 0.9fr;
  gap: 24px;
  align-items: start;
}
</style>
