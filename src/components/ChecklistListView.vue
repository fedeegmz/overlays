<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useConfirmClick } from "../composables/useConfirmClick";
import type { Checklist, ChecklistProgress } from "../types";

export interface ChecklistRow {
  checklist: Checklist;
  progress: ChecklistProgress;
  complete: boolean;
}

defineProps<{
  rows: ChecklistRow[];
}>();

const emit = defineEmits<{
  open: [id: string];
  create: [];
  delete: [id: string];
}>();

const { t } = useI18n();

const { pending: confirmingDelete, confirmClick: handleDelete } =
  useConfirmClick<string>((id) => emit("delete", id));
</script>

<template>
  <div class="checklists-list">
    <div class="page-header">
      <div>
        <div class="page-title">{{ t("checklists.title") }}</div>
        <div class="page-subtitle">{{ t("checklists.subtitle") }}</div>
      </div>
      <button type="button" class="btn btn-primary" @click="emit('create')">
        {{ t("checklists.create") }}
      </button>
    </div>

    <div v-if="rows.length > 0" class="cards">
      <div
        v-for="row in rows"
        :key="row.checklist.id"
        class="card"
        :class="{ completed: row.complete }"
      >
        <button
          type="button"
          class="card-main"
          @click="emit('open', row.checklist.id)"
        >
          <span class="card-name">{{ row.checklist.name }}</span>
          <span class="card-progress">
            {{ t("checklists.progress", { done: row.progress.done, total: row.progress.total }) }}
            <span v-if="row.complete" class="card-done">
              · {{ t("checklists.completed") }}
            </span>
          </span>
        </button>
        <button
          type="button"
          class="btn btn-ghost"
          :class="{ 'confirm-delete': confirmingDelete === row.checklist.id }"
          @click="handleDelete(row.checklist.id)"
        >
          {{ confirmingDelete === row.checklist.id
              ? t("checklists.confirmDelete")
              : t("checklists.delete") }}
        </button>
      </div>
    </div>
    <div v-else class="empty">
      <p class="muted">{{ t("checklists.empty") }}</p>
      <button type="button" class="btn" @click="emit('create')">
        {{ t("checklists.create") }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.checklists-list {
  max-width: 980px;
}

.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
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

.cards {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.card {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px 6px 6px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-card);
}

.card.completed {
  background: var(--success-soft);
  border-color: var(--success);
}

.card-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  padding: 8px 10px;
  border: none;
  background: none;
  color: inherit;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
}

.card-name {
  font-size: 14.5px;
  font-weight: 600;
}

.card-progress {
  font-size: 12.5px;
  color: var(--text-secondary);
}

.card-done {
  color: var(--success);
  font-weight: 600;
}

.confirm-delete,
.confirm-delete:hover {
  color: var(--danger);
  background: var(--danger-soft);
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 12px;
}

.muted {
  color: var(--text-faint);
  font-size: 13px;
}
</style>
