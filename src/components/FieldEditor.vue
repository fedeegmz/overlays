<script setup lang="ts">
import type { TemplateField } from "../types";
import BooleanField from "./BooleanField.vue";
import ColorField from "./ColorField.vue";
import ProgressField from "./ProgressField.vue";

defineProps<{
  fields: TemplateField[];
  values: Record<string, string>;
  disabled?: boolean;
}>();

const emit = defineEmits<(e: "update", key: string, value: string) => void>();

defineSlots<{
  /** File fields: the desktop supplies a picker, the remote shows the value read-only. */
  file?(props: {
    field: TemplateField;
    value: string;
    inputId: string;
    disabled: boolean;
    update: (value: string) => void;
  }): unknown;
}>();
</script>

<template>
  <div v-for="field in fields" :key="field.key" class="field">
    <label class="field-label" :for="`field-${field.key}`"
      >{{ field.label }}</label
    >
    <ColorField
      v-if="field.type === 'color'"
      :model-value="values[field.key] ?? ''"
      :input-id="`field-${field.key}`"
      :disabled="disabled"
      @update:model-value="emit('update', field.key, $event)"
    />
    <ProgressField
      v-else-if="field.type === 'progress'"
      :model-value="values[field.key] ?? ''"
      :input-id="`field-${field.key}`"
      :disabled="disabled"
      :min="field.min"
      :max="field.max"
      :step="field.step"
      @update:model-value="emit('update', field.key, $event)"
    />
    <BooleanField
      v-else-if="field.type === 'boolean'"
      :model-value="values[field.key] ?? 'false'"
      :input-id="`field-${field.key}`"
      :disabled="disabled"
      @update:model-value="emit('update', field.key, $event)"
    />
    <slot
      v-else-if="field.type === 'file'"
      name="file"
      :field="field"
      :value="values[field.key] ?? ''"
      :input-id="`field-${field.key}`"
      :disabled="disabled ?? false"
      :update="(value: string) => emit('update', field.key, value)"
    >
      <span class="field-readonly">{{ values[field.key] ?? "" }}</span>
    </slot>
    <input
      v-else
      :id="`field-${field.key}`"
      :value="values[field.key] ?? ''"
      class="field-input"
      type="text"
      :placeholder="field.label"
      :disabled="disabled"
      @input="emit('update', field.key, ($event.target as HTMLInputElement).value)"
    >
  </div>
</template>

<style scoped>
.field-readonly {
  display: block;
  font-size: 13px;
  color: var(--text-secondary);
  overflow-wrap: anywhere;
}
</style>
