<script setup lang="ts">
import { storeToRefs } from "pinia";
import { onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { commandErrorMessage } from "../lib/errors";
import { useInstanceStore } from "../stores/instances";

const { t } = useI18n();
const instanceStore = useInstanceStore();
const { activeInstance, previewUrl } = storeToRefs(instanceStore);
const { update, toggleVisibility, sendPreviewShow, openPreviewWindow } =
  instanceStore;

const sending = ref<"toggle" | "update" | null>(null);
const sendError = ref<string | null>(null);
const loading = ref(true);

let previewTimer: ReturnType<typeof setTimeout> | null = null;
let loadTimer: ReturnType<typeof setTimeout> | null = null;

const visible = () => activeInstance.value?.visible ?? false;

const disabled = () => sending.value !== null || !activeInstance.value;

watch(
  () => activeInstance.value?.fields,
  () => {
    if (previewTimer) clearTimeout(previewTimer);
    previewTimer = setTimeout(() => {
      void sendPreviewShow().catch((err) => {
        sendError.value = commandErrorMessage(err);
      });
    }, 200);
  },
  { deep: true },
);

watch(
  previewUrl,
  () => {
    loading.value = true;
    if (loadTimer) clearTimeout(loadTimer);
    loadTimer = setTimeout(() => {
      loading.value = false;
    }, 4000);
  },
  { immediate: true },
);

async function handleFrameLoad() {
  loading.value = false;
  if (loadTimer) clearTimeout(loadTimer);
  try {
    await sendPreviewShow();
  } catch (err) {
    sendError.value = commandErrorMessage(err);
  }
}

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

async function handleOpenWindow() {
  sendError.value = null;
  try {
    await openPreviewWindow();
  } catch (err) {
    sendError.value = commandErrorMessage(err);
  }
}

onBeforeUnmount(() => {
  if (previewTimer) clearTimeout(previewTimer);
  if (loadTimer) clearTimeout(loadTimer);
});
</script>

<template>
  <div class="panel">
    <div class="panel-header">{{ t("preview.title") }}</div>
    <div class="panel-body">
      <div class="stage">
        <iframe
          v-if="previewUrl"
          class="stage-frame"
          :src="previewUrl"
          title="overlay preview"
          @load="handleFrameLoad"
        ></iframe>
        <div class="stage-overlay">
          <span v-if="previewUrl && !loading" class="stage-live"
            >{{ t("preview.live") }}</span
          >
          <span v-if="loading" class="stage-message"
            >{{ t("preview.loading") }}</span
          >
          <span v-else-if="!previewUrl" class="stage-message"
            >{{ t("preview.unavailable") }}</span
          >
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
          <span class="toggle-label"
            >{{ visible() ? t("preview.visibleInObs") : t("preview.hidden") }}</span
          >
        </label>

        <button
          type="button"
          class="btn"
          :disabled="disabled() || !visible()"
          @click="runUpdate"
        >
          {{ t("preview.update") }}
        </button>

        <button
          type="button"
          class="btn btn-ghost"
          :disabled="!previewUrl"
          @click="handleOpenWindow"
        >
          {{ t("preview.openWindow") }}
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
  overflow: hidden;
}

.stage-frame {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  border: 0;
  background: transparent;
  pointer-events: none;
}

.stage-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 10px;
  pointer-events: none;
}

.stage-live {
  background: var(--accent);
  color: #fff;
  font-size: 10.5px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  padding: 3px 7px;
  border-radius: 999px;
}

.stage-message {
  align-self: center;
  color: var(--text-faint);
  font-size: 12.5px;
  background: rgba(19, 19, 23, 0.72);
  padding: 6px 10px;
  border-radius: var(--radius-sm);
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

.btn-ghost {
  margin-left: auto;
}
</style>
