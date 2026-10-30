<script setup lang="ts">
import type { FolderEntry } from "../lib/folderTree";
import type { TemplateInfo } from "../types";
import FolderCard from "./FolderCard.vue";
import OverlayCard from "./OverlayCard.vue";

defineProps<{
  folders: FolderEntry[];
  templates: TemplateInfo[];
}>();

const emit = defineEmits<{
  select: [id: string];
  openFolder: [path: string];
}>();
</script>

<template>
  <div class="overlay-grid">
    <FolderCard
      v-for="folder in folders"
      :key="`folder:${folder.path}`"
      :folder="folder"
      @open="emit('openFolder', $event)"
    />
    <OverlayCard
      v-for="template in templates"
      :key="template.id"
      :template="template"
      @click="emit('select', template.id)"
    />
  </div>
</template>

<style scoped>
.overlay-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
  gap: 16px;
}
</style>
