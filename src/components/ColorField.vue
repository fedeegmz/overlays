<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const props = defineProps<{
  modelValue: string;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

const PRESETS = [
  "#ffffff",
  "#0f172a",
  "#ef4444",
  "#f97316",
  "#eab308",
  "#22c55e",
  "#38bdf8",
  "#818cf8",
  "#ec4899",
];

const HEX_RE = /^#(?:[0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/;

const text = ref(props.modelValue);
const touched = ref(false);
const inputEl = ref<HTMLInputElement | null>(null);

function normalize(input: string): string | null {
  let v = input.trim();
  if (v.length === 0) return null;
  if (!v.startsWith("#")) v = `#${v}`;
  v = v.toLowerCase();
  if (!HEX_RE.test(v)) return null;
  const d = v.slice(1);
  if (d.length <= 4) {
    return `#${[...d].map((c) => c + c).join("")}`;
  }
  return v;
}

watch(
  () => props.modelValue,
  (value) => {
    if (document.activeElement === inputEl.value) return;
    const n = normalize(value);
    if (n !== null && n !== text.value) {
      text.value = n;
    }
  },
);

const expanded = computed(() => normalize(text.value));
const isValid = computed(() => expanded.value !== null);
const showInvalid = computed(() => touched.value && !isValid.value);

const rgbHex = computed(() =>
  isValid.value ? expanded.value!.slice(1, 7) : "000000",
);

const alphaByte = computed(() =>
  isValid.value && expanded.value!.length === 9
    ? expanded.value!.slice(7, 9)
    : "ff",
);

const opacityPct = computed(() =>
  Math.round((parseInt(alphaByte.value, 16) / 255) * 100),
);

const swatchBackground = computed(() =>
  isValid.value ? expanded.value! : "transparent",
);

function handleInput() {
  touched.value = true;
  const n = normalize(text.value);
  if (n !== null) {
    emit("update:modelValue", n);
  }
}

function handleBlur() {
  touched.value = true;
  const n = normalize(text.value);
  if (n !== null && n !== text.value) {
    text.value = n;
  }
}

function handlePicker(event: Event) {
  const rgb = (event.target as HTMLInputElement).value.slice(1);
  emit("update:modelValue", `#${rgb}${alphaByte.value}`);
}

function handleOpacity(event: Event) {
  const pct = Number((event.target as HTMLInputElement).value);
  const byte = Math.round((pct / 100) * 255)
    .toString(16)
    .padStart(2, "0");
  emit("update:modelValue", `#${rgbHex.value}${byte}`);
}

function applyPreset(hex: string) {
  emit("update:modelValue", `${hex}${alphaByte.value}`);
}
</script>

<template>
  <div class="color-field">
    <div class="color-row">
      <label class="swatch" :title="t('colorField.openPicker')">
        <span class="swatch-fill" :style="{ background: swatchBackground }"></span>
        <input
          class="native-picker"
          type="color"
          :value="`#${rgbHex}`"
          :disabled="disabled"
          tabindex="-1"
          @input="handlePicker"
        />
      </label>
      <input
        ref="inputEl"
        :value="text"
        class="field-input hex-input"
        :class="{ 'hex-invalid': showInvalid }"
        type="text"
        spellcheck="false"
        autocomplete="off"
        placeholder="#RRGGBBAA"
        :disabled="disabled"
        @input="handleInput"
        @blur="handleBlur"
      />
    </div>

    <div class="alpha-row">
      <input
        class="alpha-slider"
        type="range"
        min="0"
        max="100"
        step="1"
        :value="opacityPct"
        :disabled="disabled"
        @input="handleOpacity"
      />
      <span class="alpha-pct">{{ opacityPct }}%</span>
    </div>

    <div class="preset-row">
      <button
        v-for="preset in PRESETS"
        :key="preset"
        type="button"
        class="preset-dot"
        :style="{ background: preset }"
        :title="preset"
        :disabled="disabled"
        @click="applyPreset(preset)"
      ></button>
    </div>
  </div>
</template>

<style scoped>
.color-row {
  display: flex;
  gap: 8px;
  align-items: stretch;
}

.swatch {
  position: relative;
  width: 38px;
  flex-shrink: 0;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-strong);
  overflow: hidden;
  cursor: pointer;
  background-image: conic-gradient(
    var(--surface-sunken) 0 25%,
    var(--surface) 0 50%,
    var(--surface-sunken) 0 75%,
    var(--surface) 0
  );
  background-size: 12px 12px;
}

.swatch-fill {
  position: absolute;
  inset: 0;
}

.native-picker {
  position: absolute;
  inset: 0;
  opacity: 0;
  width: 100%;
  height: 100%;
  border: none;
  padding: 0;
  cursor: pointer;
}

.hex-input {
  flex: 1;
  font-family: ui-monospace, monospace;
}

.hex-invalid {
  border-color: var(--danger);
}

.hex-invalid:focus {
  box-shadow: 0 0 0 3px var(--danger-soft);
}

.alpha-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 8px;
}

.alpha-slider {
  flex: 1;
  accent-color: var(--accent);
}

.alpha-pct {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--text-secondary);
  width: 36px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.preset-row {
  display: flex;
  gap: 5px;
  margin-top: 8px;
}

.preset-dot {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: 1px solid var(--border-strong);
  cursor: pointer;
  padding: 0;
  transition: transform 0.1s ease;
}

.preset-dot:hover {
  transform: scale(1.15);
}

.preset-dot:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}
</style>
