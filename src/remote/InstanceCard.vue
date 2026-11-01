<script setup lang="ts">
import { ref, useId } from "vue";
import { useI18n } from "vue-i18n";
import FieldEditor from "../components/FieldEditor.vue";
import type { OverlayInstance, Preset, TemplateField } from "../types";

defineProps<{
  instance: OverlayInstance;
  fields: TemplateField[];
  presets: Preset[];
  pending: boolean;
}>();

const emit = defineEmits<{
  show: [];
  update: [];
  hide: [];
  edit: [key: string, value: string];
  preset: [name: string];
}>();

const { t } = useI18n();

const expanded = ref(false);
const bodyId = useId();

function pickPreset(event: Event): void {
  const select = event.target as HTMLSelectElement;
  if (select.value) emit("preset", select.value);
  select.value = "";
}
</script>

<template>
  <section class="card">
    <h2 class="card-heading">
      <!-- The body only exists while expanded, and aria-controls must not point
           at a missing element. -->
      <button
        class="card-head"
        type="button"
        :aria-expanded="expanded"
        :aria-controls="expanded ? bodyId : undefined"
        @click="expanded = !expanded"
      >
        <svg
          class="chevron"
          :class="{ open: expanded }"
          viewBox="0 0 16 16"
          width="16"
          height="16"
          aria-hidden="true"
        >
          <path
            d="M6 3.5 10.5 8 6 12.5"
            fill="none"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <span class="card-name">{{ instance.displayName }}</span>
        <span v-if="pending" class="pending">
          {{ t("remote.instance.pending") }}
        </span>
        <span class="state" :class="{ live: instance.visible }">
          {{ instance.visible ? t("remote.instance.live") : t("remote.instance.hidden") }}
        </span>
      </button>
    </h2>

    <div v-if="expanded" :id="bodyId" class="card-body">
      <FieldEditor
        :fields="fields"
        :values="instance.fields"
        @update="(key, value) => emit('edit', key, value)"
      />

      <select
        v-if="presets.length > 0"
        class="field-input preset"
        :aria-label="t('remote.instance.preset')"
        @change="pickPreset"
      >
        <option value="">{{ t("remote.instance.presetPlaceholder") }}</option>
        <option v-for="p in presets" :key="p.name" :value="p.name">
          {{ p.name }}
        </option>
      </select>

      <div class="actions">
        <button
          class="btn"
          :class="{ 'btn-primary': !instance.visible }"
          type="button"
          @click="emit('show')"
        >
          {{ t("remote.instance.show") }}
        </button>
        <button
          class="btn"
          :class="{ 'btn-primary': instance.visible }"
          type="button"
          :disabled="!instance.visible"
          @click="emit('update')"
        >
          {{ t("remote.instance.update") }}
        </button>
        <button class="btn" type="button" @click="emit('hide')">
          {{ t("remote.instance.hide") }}
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  margin-bottom: 12px;
  box-shadow: var(--shadow-panel);
}
.card-heading {
  margin: 0;
  font-size: 15px;
}
.card-head {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 14px;
  border: 0;
  border-radius: var(--radius-lg);
  background: none;
  color: inherit;
  font: inherit;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
}
.card-head:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.chevron {
  flex-shrink: 0;
  color: var(--text-secondary);
  transition: transform 0.15s ease;
}
.chevron.open {
  transform: rotate(90deg);
}
.card-name {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}
.pending {
  font-size: 11px;
  font-weight: 600;
  color: var(--accent);
  text-align: right;
}
.state {
  flex-shrink: 0;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
}
.state.live {
  color: var(--success);
}
.card-body {
  padding: 0 14px 14px;
}
.preset {
  margin-bottom: 12px;
}
.actions {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}
.actions .btn {
  justify-content: center;
  padding: 11px 6px;
}
</style>
