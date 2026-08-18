<script setup lang="ts">
import { useOverlayControl } from "../composables/useOverlayControl";
import OverlayGrid from "./OverlayGrid.vue";

const { templates, templatesError, createInstance } = useOverlayControl();

const emit = defineEmits<{
  openDetail: [];
}>();

function handleSelect(id: string) {
  createInstance(id);
  emit("openDetail");
}
</script>

<template>
  <div class="overlays-page">
    <div class="page-header">
      <div>
        <div class="page-title">Overlays</div>
        <div class="page-subtitle">Elegí una plantilla para crear una instancia y mostrarla en OBS</div>
      </div>
    </div>

    <p v-if="templatesError" class="error">{{ templatesError }}</p>
    <p v-else-if="templates === null" class="muted">Cargando plantillas...</p>
    <p v-else-if="templates.length === 0" class="muted">
      No hay plantillas disponibles.
    </p>
    <OverlayGrid
      v-else
      :templates="templates"
      @select="handleSelect"
    />
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

.error {
  color: var(--danger);
  font-size: 13.5px;
}

.muted {
  color: var(--text-faint);
  font-size: 13.5px;
}
</style>
