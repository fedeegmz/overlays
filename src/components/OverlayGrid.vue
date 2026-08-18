<script setup lang="ts">
import type { TemplateInfo } from "../types";
import OverlayCard from "./OverlayCard.vue";

defineProps<{
  templates: TemplateInfo[];
}>();

const emit = defineEmits<{
  select: [id: string];
}>();
</script>

<template>
  <div class="overlay-grid">
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
