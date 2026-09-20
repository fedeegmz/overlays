<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  modelValue: string;
  inputId?: string;
  disabled?: boolean;
}>();

const emit = defineEmits<(e: "update:modelValue", value: string) => void>();

const checked = computed(() => props.modelValue === "true");

function handleChange(event: Event) {
  const isChecked = (event.target as HTMLInputElement).checked;
  emit("update:modelValue", isChecked ? "true" : "false");
}
</script>

<template>
  <div class="switch">
    <input
      :id="inputId"
      class="switch-input"
      type="checkbox"
      :checked="checked"
      :disabled="disabled"
      @change="handleChange"
    >
    <span class="switch-track" aria-hidden="true"></span>
  </div>
</template>

<style scoped>
.switch {
  position: relative;
  display: inline-block;
  width: 40px;
  height: 22px;
  flex: none;
  cursor: pointer;
}

.switch-input {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  margin: 0;
  opacity: 0;
  cursor: pointer;
  z-index: 1;
}

.switch-track {
  position: absolute;
  inset: 0;
  border-radius: 999px;
  background: var(--surface-sunken);
  border: 1px solid var(--border-strong);
  transition:
    background 0.15s ease,
    border-color 0.15s ease;
}

.switch-track::after {
  content: "";
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--text-secondary);
  transition:
    transform 0.15s ease,
    background 0.15s ease;
}

.switch-input:checked + .switch-track {
  background: var(--accent, #38bdf8);
  border-color: var(--accent, #38bdf8);
}

.switch-input:checked + .switch-track::after {
  transform: translateX(18px);
  background: #fff;
}

.switch-input:focus-visible + .switch-track {
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.switch-input:disabled {
  cursor: not-allowed;
}

.switch-input:disabled + .switch-track {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
