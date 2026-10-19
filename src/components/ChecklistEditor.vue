<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useDragReorder } from "../composables/useDragReorder";
import type { Checklist } from "../types";

const props = defineProps<{
  checklist: Checklist;
  isNew: boolean;
  /** Localized backend error from the last save attempt, if any. */
  error: string | null;
  saving: boolean;
}>();

const emit = defineEmits<{
  save: [draft: Checklist];
  cancel: [];
}>();

const { t } = useI18n();

// Deep copy that keeps the existing ids, so checks survive the edit.
const draft = ref<Checklist>({
  id: props.checklist.id,
  name: props.checklist.name,
  items: props.checklist.items.map((i) => ({ id: i.id, text: i.text })),
});

const submitted = ref(false);
const itemsList = ref<HTMLElement | null>(null);

const nameInvalid = computed(
  () => submitted.value && draft.value.name.trim() === "",
);
const noItems = computed(
  () => submitted.value && draft.value.items.length === 0,
);
const blankItemIds = computed(
  () =>
    new Set(
      submitted.value
        ? draft.value.items.filter((i) => i.text.trim() === "").map((i) => i.id)
        : [],
    ),
);

const validationMessage = computed<string | null>(() => {
  if (nameInvalid.value) return t("errors.checklist.empty_name");
  if (noItems.value) return t("errors.checklist.no_items");
  if (blankItemIds.value.size > 0) return t("errors.checklist.empty_item");
  return null;
});

function moveItem(from: number, to: number) {
  if (from === to) return;
  const items = [...draft.value.items];
  const [moved] = items.splice(from, 1);
  if (!moved) return;
  items.splice(to, 0, moved);
  draft.value.items = items;
}

const {
  dragFromIndex,
  dropTarget,
  handleDragStart,
  handleDragOver,
  handleDragLeave,
  handleDrop,
  resetDrag,
} = useDragReorder(moveItem);

async function focusItem(id: string) {
  await nextTick();
  itemsList.value
    ?.querySelector<HTMLInputElement>(`[data-item-id="${id}"] input`)
    ?.focus();
}

function addItem() {
  const id = crypto.randomUUID();
  draft.value.items.push({ id, text: "" });
  void focusItem(id);
}

function removeItem(index: number) {
  draft.value.items.splice(index, 1);
}

async function moveByKeyboard(index: number, delta: number) {
  const to = index + delta;
  if (to < 0 || to >= draft.value.items.length) return;
  const id = draft.value.items[index]?.id;
  moveItem(index, to);
  if (id) await focusItem(id);
}

function save() {
  submitted.value = true;
  if (
    draft.value.name.trim() === "" ||
    draft.value.items.length === 0 ||
    draft.value.items.some((i) => i.text.trim() === "")
  ) {
    return;
  }
  emit("save", {
    id: draft.value.id,
    name: draft.value.name,
    items: draft.value.items.map((i) => ({ id: i.id, text: i.text })),
  });
}
</script>

<template>
  <form class="checklist-editor" @submit.prevent="save">
    <div class="page-header">
      <div class="page-title">
        {{ isNew ? t("checklists.editor.createTitle") : t("checklists.editor.editTitle") }}
      </div>
      <div class="header-actions">
        <button type="button" class="btn btn-ghost" @click="emit('cancel')">
          {{ t("checklists.editor.cancel") }}
        </button>
        <button type="submit" class="btn btn-primary" :disabled="saving">
          {{ t("checklists.editor.save") }}
        </button>
      </div>
    </div>

    <label class="field-label" for="checklist-name">
      {{ t("checklists.editor.nameLabel") }}
    </label>
    <input
      id="checklist-name"
      v-model="draft.name"
      class="field-input"
      :class="{ invalid: nameInvalid }"
      type="text"
      :placeholder="t('checklists.editor.namePlaceholder')"
    >

    <div class="field-label items-label">
      {{ t("checklists.editor.itemsLabel") }}
    </div>
    <div ref="itemsList" class="items">
      <!-- biome-ignore lint/a11y/noStaticElementInteractions: drag-and-drop row; keyboard reordering lives on the inner input (Alt+Up/Down) -->
      <div
        v-for="(item, index) in draft.items"
        :key="item.id"
        class="item-row"
        :class="{
          invalid: blankItemIds.has(item.id),
          dragging: dragFromIndex === index,
          'drop-before':
            dropTarget?.index === index &&
            !dropTarget.after &&
            dragFromIndex !== index,
          'drop-after':
            dropTarget?.index === index &&
            dropTarget.after &&
            dragFromIndex !== index,
        }"
        :data-item-id="item.id"
        draggable="true"
        @dragstart="handleDragStart($event, index)"
        @dragover="handleDragOver($event, index)"
        @dragleave="handleDragLeave($event, index)"
        @drop="handleDrop"
        @dragend="resetDrag"
      >
        <span class="drag-handle" aria-hidden="true">⠿</span>
        <input
          v-model="item.text"
          class="field-input"
          type="text"
          :placeholder="t('checklists.editor.itemPlaceholder')"
          :aria-label="t('checklists.editor.itemLabel', { n: index + 1 })"
          @keydown.alt.up.prevent="moveByKeyboard(index, -1)"
          @keydown.alt.down.prevent="moveByKeyboard(index, 1)"
        >
        <button
          type="button"
          class="btn btn-danger-ghost"
          :title="t('checklists.editor.removeItem')"
          :aria-label="t('checklists.editor.removeItem')"
          @click="removeItem(index)"
        >
          ×
        </button>
      </div>
    </div>

    <button type="button" class="btn add-item" @click="addItem">
      {{ t("checklists.editor.addItem") }}
    </button>
    <p class="hint">{{ t("checklists.editor.reorderHint") }}</p>

    <p v-if="validationMessage" class="error" role="alert">
      {{ validationMessage }}
    </p>
    <p v-else-if="error" class="error" role="alert">{{ error }}</p>
  </form>
</template>

<style scoped>
.checklist-editor {
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

.header-actions {
  display: flex;
  gap: 6px;
}

.field-label {
  display: block;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.items-label {
  margin-top: 20px;
}

.items {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.item-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 2px;
  border-radius: var(--radius-sm);
}

.item-row.dragging {
  opacity: 0.45;
}

.item-row.drop-before {
  box-shadow: inset 0 2px 0 var(--accent);
}

.item-row.drop-after {
  box-shadow: inset 0 -2px 0 var(--accent);
}

.drag-handle {
  color: var(--text-faint);
  cursor: grab;
  user-select: none;
}

.field-input.invalid,
.item-row.invalid .field-input {
  border-color: var(--danger);
}

.add-item {
  margin-top: 10px;
}

.hint {
  font-size: 12px;
  color: var(--text-faint);
  margin-top: 8px;
}

.error {
  color: var(--danger);
  font-size: 13px;
  margin-top: 14px;
}
</style>
