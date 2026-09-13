<script setup lang="ts">
import { computed, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    inputId?: string;
    disabled?: boolean;
    min?: number;
    max?: number;
  }>(),
  {
    min: 0,
    max: 100,
  },
);

const emit = defineEmits<(e: "update:modelValue", value: string) => void>();

const text = ref(props.modelValue);
const inputEl = ref<HTMLInputElement | null>(null);

function clamp(value: number): number {
  return Math.min(props.max, Math.max(props.min, value));
}

function toDisplay(value: string): string {
  const n = Number(value);
  return Number.isFinite(n) ? String(clamp(n)) : String(props.min);
}

watch(
  () => props.modelValue,
  (value) => {
    if (document.activeElement === inputEl.value) return;
    const next = toDisplay(value);
    if (next !== text.value) text.value = next;
  },
);

const numeric = computed(() => clamp(Number(toDisplay(text.value))));

function handleRange(event: Event) {
  const value = (event.target as HTMLInputElement).value;
  text.value = value;
  emit("update:modelValue", value);
}

function handleNumber(event: Event) {
  const raw = (event.target as HTMLInputElement).value;
  const n = Number(raw);
  if (!Number.isFinite(n)) return;
  const clamped = clamp(n);
  text.value = String(clamped);
  emit("update:modelValue", String(clamped));
}
</script>

<template>
  <div class="progress-field">
    <input
      :id="inputId"
      class="progress-range"
      type="range"
      :min="props.min"
      :max="props.max"
      :step="1"
      :value="numeric"
      :disabled="disabled"
      @input="handleRange"
    >
    <input
      ref="inputEl"
      class="field-input progress-number"
      type="number"
      :min="props.min"
      :max="props.max"
      :value="numeric"
      :disabled="disabled"
      @input="handleNumber"
    >
  </div>
</template>

<style scoped>
.progress-field {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.progress-range {
  flex: 1;
  min-width: 0;
  accent-color: var(--accent, #38bdf8);
}

.progress-number {
  width: 72px;
  flex: none;
  text-align: right;
}
</style>
