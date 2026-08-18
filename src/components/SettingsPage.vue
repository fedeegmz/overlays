<script setup lang="ts">
import { useOverlayControl } from "../composables/useOverlayControl";
const {
  appConfig,
  configError,
  pickOverlaysDir,
} = useOverlayControl();
</script>

<template>
  <div class="settings-page">
    <div class="page-header">
      <div>
        <div class="page-title">Ajustes</div>
        <div class="page-subtitle">Configuración general de la aplicación</div>
      </div>
    </div>

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <div class="settings-row-label">Carpeta de overlays</div>
          <div class="settings-row-desc">Las plantillas se leen desde esta ubicación</div>
        </div>
        <div class="settings-row-right">
          <span v-if="appConfig.overlays_dir" class="settings-row-value">
            {{ appConfig.overlays_dir }}
          </span>
          <span v-else class="settings-row-value settings-row-empty">
            No configurado
          </span>
          <button class="btn" @click="pickOverlaysDir">
            {{ appConfig.overlays_dir ? 'Cambiar' : 'Elegir directorio' }}
          </button>
        </div>
      </div>
      <p v-if="configError" class="config-error">{{ configError }}</p>
    </div>

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <div class="settings-row-label">Apariencia</div>
          <div class="settings-row-desc">Tema de la interfaz de la aplicación</div>
        </div>
        <div class="theme-toggle">
          <button class="active">Claro</button>
          <button disabled>Oscuro</button>
          <button disabled>Sistema</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
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

.settings-section {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-panel);
  margin-bottom: 18px;
}

.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px;
  gap: 20px;
}

.settings-row-label {
  font-size: 13.5px;
  font-weight: 600;
}

.settings-row-desc {
  font-size: 12.5px;
  color: var(--text-secondary);
  margin-top: 3px;
}

.settings-row-value {
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 12px;
  color: var(--text-secondary);
  background: var(--surface-sunken);
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  max-width: 340px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.settings-row-empty {
  color: var(--text-faint);
  font-style: italic;
  font-family: inherit;
}

.settings-row-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.theme-toggle {
  display: flex;
  gap: 4px;
  background: var(--surface-sunken);
  padding: 3px;
  border-radius: 8px;
}

.theme-toggle button {
  border: none;
  background: none;
  font-size: 12px;
  font-weight: 600;
  padding: 6px 12px;
  border-radius: 6px;
  cursor: pointer;
  color: var(--text-secondary);
  font-family: inherit;
  transition: background 0.12s ease, color 0.12s ease;
}

.theme-toggle button.active {
  background: var(--surface);
  color: var(--text);
  box-shadow: 0 1px 2px rgba(24, 24, 27, 0.08);
}

.theme-toggle button:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.config-error {
  padding: 0 18px 14px;
  color: var(--danger);
  font-size: 12.5px;
}
</style>
