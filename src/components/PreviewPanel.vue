<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useOverlayControl, commandErrorMessage } from "../composables/useOverlayControl";

const { t } = useI18n();
const {
  activeInstance,
  update,
  toggleVisibility,
} = useOverlayControl();

const sending = ref<"toggle" | "update" | null>(null);
const sendError = ref<string | null>(null);

const visible = () => activeInstance.value?.visible ?? false;

const disabled = () => sending.value !== null || !activeInstance.value;

async function handleToggle() {
  if (disabled()) return;
  sendError.value = null;
  sending.value = "toggle";
  try {
    await toggleVisibility();
  } catch (err) {
    sendError.value = commandErrorMessage(err);
  } finally {
    sending.value = null;
  }
}

async function runUpdate() {
  if (disabled()) return;
  sendError.value = null;
  sending.value = "update";
  try {
    await update();
  } catch (err) {
    sendError.value = commandErrorMessage(err);
  } finally {
    sending.value = null;
  }
}
</script>

<template>
  <div class="panel">
    <div class="panel-header">{{ t("preview.title") }}</div>
    <div class="panel-body">
      <div class="stage" :class="{ 'stage-dimmed': !visible() }">
        <div class="stage-lowerthird">
          <div class="lt-title">
            {{ activeInstance?.fields?.titulo || activeInstance?.fields?.texto || '...' }}
          </div>
          <div class="lt-sub">{{ activeInstance?.fields?.subtitulo || '' }}</div>
        </div>
      </div>

      <p v-if="sendError" class="send-error">{{ sendError }}</p>

      <div class="action-row">
        <label class="toggle-row" :class="{ disabled: disabled() }">
          <button
            type="button"
            class="toggle"
            :class="{ active: visible() }"
            role="switch"
            :aria-checked="visible()"
            :disabled="disabled()"
            @click="handleToggle"
          >
            <span class="toggle-thumb"></span>
          </button>
          <span class="toggle-label">{{ visible() ? t("preview.visibleInObs") : t("preview.hidden") }}</span>
        </label>

        <button
          class="btn"
          :disabled="disabled() || !visible()"
          @click="runUpdate"
        >
          {{ t("preview.update") }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.panel {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-panel);
}

.panel-header {
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
  font-size: 12.5px;
  font-weight: 650;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.panel-body {
  padding: 18px;
}

.stage {
  aspect-ratio: 16 / 9;
  background: #131317
    url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='40' height='40'%3E%3Crect width='40' height='40' fill='none'/%3E%3Cpath d='M0 40L40 0' stroke='%231D1D22' stroke-width='1'/%3E%3C/svg%3E");
  border-radius: var(--radius-md);
  position: relative;
  display: flex;
  align-items: flex-end;
  padding: 22px;
  transition: opacity 0.2s ease;
}

.stage-dimmed {
  opacity: 0.45;
}

.stage-lowerthird {
  background: rgba(18, 18, 22, 0.75);
  backdrop-filter: blur(3px);
  border-left: 4px solid var(--accent);
  padding: 10px 16px;
  border-radius: 4px;
  max-width: 70%;
}

.lt-title {
  color: #fff;
  font-size: 17px;
  font-weight: 700;
}

.lt-sub {
  color: #C9C9D1;
  font-size: 12.5px;
  margin-top: 2px;
}

.send-error {
  color: var(--danger);
  font-size: 12.5px;
  margin-top: 10px;
}

.action-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 18px;
}

.toggle-row {
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  user-select: none;
}

.toggle-row.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.toggle {
  position: relative;
  width: 40px;
  height: 22px;
  border-radius: 11px;
  border: none;
  background: var(--border-strong);
  cursor: pointer;
  transition: background 0.2s ease;
  flex-shrink: 0;
  padding: 0;
}

.toggle:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.toggle.active {
  background: var(--accent);
}

.toggle:disabled {
  cursor: not-allowed;
}

.toggle-thumb {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: white;
  transition: transform 0.2s ease;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
}

.toggle.active .toggle-thumb {
  transform: translateX(18px);
}

.toggle-label {
  font-size: 13px;
  font-weight: 550;
  color: var(--text-secondary);
}
</style>
