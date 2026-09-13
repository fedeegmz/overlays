<script setup lang="ts">
import { useI18n } from "vue-i18n";

defineProps<{ title: string; open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const { t } = useI18n();
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="modal-layer">
      <button
        type="button"
        class="modal-backdrop"
        :aria-label="t('common.close')"
        @click="emit('close')"
      ></button>
      <div
        class="modal"
        role="dialog"
        aria-modal="true"
        @keydown.esc="emit('close')"
      >
        <div class="modal-header">
          <div class="modal-title">{{ title }}</div>
          <button
            type="button"
            class="modal-close"
            :aria-label="t('common.close')"
            @click="emit('close')"
          >
            ✕
          </button>
        </div>
        <div class="modal-body">
          <slot />
        </div>
        <div v-if="$slots.footer" class="modal-footer">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-layer {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal-backdrop {
  position: absolute;
  inset: 0;
  border: none;
  padding: 0;
  background: rgba(16, 16, 20, 0.45);
  backdrop-filter: blur(2px);
  cursor: default;
}

.modal {
  position: relative;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: 0 18px 48px rgba(0, 0, 0, 0.28);
  width: min(440px, calc(100vw - 48px));
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 18px 12px;
}

.modal-title {
  font-size: 15px;
  font-weight: 650;
}

.modal-close {
  border: none;
  background: none;
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
  padding: 4px 6px;
  border-radius: 6px;
}

.modal-close:hover {
  background: var(--surface-sunken);
  color: var(--text);
}

.modal-body {
  padding: 0 18px 16px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 12px 18px 16px;
  border-top: 1px solid var(--border);
}

.modal-footer :deep(button) {
  font-size: 12.5px;
  padding: 7px 14px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  cursor: pointer;
  font-family: inherit;
  font-weight: 600;
}

.modal-footer :deep(button.primary) {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
</style>
