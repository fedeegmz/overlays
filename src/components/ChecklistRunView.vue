<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { Checklist } from "../types";

const props = defineProps<{
  checklist: Checklist;
  checkedIds: ReadonlySet<string>;
}>();

const emit = defineEmits<{
  toggle: [itemId: string];
  uncheckAll: [];
  back: [];
  edit: [id: string];
}>();

const { t } = useI18n();

const total = computed(() => props.checklist.items.length);
const done = computed(
  () => props.checklist.items.filter((i) => props.checkedIds.has(i.id)).length,
);
const complete = computed(() => total.value > 0 && done.value === total.value);
</script>

<template>
  <div class="checklist-run">
    <div class="page-header">
      <div>
        <div class="page-title">{{ checklist.name }}</div>
        <div class="page-subtitle" :class="{ completed: complete }">
          {{ t("checklists.progress", { done, total }) }}
          <span v-if="complete"> · {{ t("checklists.completed") }}</span>
        </div>
      </div>
      <div class="header-actions">
        <button type="button" class="btn btn-ghost" @click="emit('back')">
          {{ t("checklists.back") }}
        </button>
        <button
          type="button"
          class="btn"
          :disabled="done === 0"
          @click="emit('uncheckAll')"
        >
          {{ t("checklists.uncheckAll") }}
        </button>
        <button type="button" class="btn" @click="emit('edit', checklist.id)">
          {{ t("checklists.edit") }}
        </button>
      </div>
    </div>

    <div class="items">
      <label
        v-for="item in checklist.items"
        :key="item.id"
        class="item"
        :class="{ checked: checkedIds.has(item.id) }"
      >
        <input
          type="checkbox"
          :checked="checkedIds.has(item.id)"
          @change="emit('toggle', item.id)"
        >
        <span class="item-text">{{ item.text }}</span>
      </label>
    </div>
  </div>
</template>

<style scoped>
.checklist-run {
  max-width: 980px;
}

.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
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

.page-subtitle.completed {
  color: var(--success);
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 6px;
}

.items {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  background: var(--surface-sunken);
  font-size: 14px;
  cursor: pointer;
}

.item.checked .item-text {
  color: var(--text-faint);
  text-decoration: line-through;
}

.item input {
  accent-color: var(--accent);
  width: 16px;
  height: 16px;
}
</style>
