<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { FolderEntry } from "../lib/folderTree";

defineProps<{
  folder: FolderEntry;
}>();

const emit = defineEmits<{
  open: [path: string];
}>();

const { t } = useI18n();
</script>

<template>
  <button class="folder-card" type="button" @click="emit('open', folder.path)">
    <svg
      aria-hidden="true"
      class="folder-icon"
      width="22"
      height="22"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path
        d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"
      />
    </svg>
    <div class="folder-text">
      <div class="folder-name">{{ folder.name }}</div>
      <div class="folder-count">
        {{ t("overlaysPage.folderCount", folder.directCount) }}
      </div>
    </div>
  </button>
</template>

<style scoped>
.folder-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card);
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  color: inherit;
  transition:
    box-shadow 0.15s ease,
    transform 0.15s ease,
    border-color 0.15s ease;
}

.folder-card:hover {
  box-shadow: var(--shadow-card-hover);
  transform: translateY(-2px);
  border-color: var(--border-strong);
}

.folder-icon {
  flex-shrink: 0;
  color: var(--accent);
}

.folder-name {
  font-size: 13.5px;
  font-weight: 600;
}

.folder-count {
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 2px;
}
</style>
