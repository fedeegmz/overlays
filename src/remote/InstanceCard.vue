<script setup lang="ts">
import { useI18n } from "vue-i18n";
import FieldEditor from "../components/FieldEditor.vue";
import type { OverlayInstance, Preset, TemplateField } from "../types";

defineProps<{
  instance: OverlayInstance;
  fields: TemplateField[];
  presets: Preset[];
}>();

const emit = defineEmits<{
  show: [];
  hide: [];
  edit: [key: string, value: string];
  preset: [name: string];
}>();

const { t } = useI18n();

function pickPreset(event: Event): void {
  const select = event.target as HTMLSelectElement;
  if (select.value) emit("preset", select.value);
  select.value = "";
}
</script>

<template>
  <section class="card">
    <header class="card-head">
      <h2 class="card-name">{{ instance.displayName }}</h2>
      <span class="state" :class="{ live: instance.visible }">
        {{ instance.visible ? t("remote.instance.live") : t("remote.instance.hidden") }}
      </span>
    </header>

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
      <button class="btn btn-primary" type="button" @click="emit('show')">
        {{ t("remote.instance.show") }}
      </button>
      <button class="btn" type="button" @click="emit('hide')">
        {{ t("remote.instance.hide") }}
      </button>
    </div>
  </section>
</template>

<style scoped>
.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 14px;
  margin-bottom: 12px;
  box-shadow: var(--shadow-panel);
}
.card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 12px;
}
.card-name {
  font-size: 15px;
  overflow-wrap: anywhere;
}
.state {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
}
.state.live {
  color: var(--success);
}
.preset {
  margin-bottom: 12px;
}
.actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.actions .btn {
  justify-content: center;
  padding: 11px;
}
</style>
