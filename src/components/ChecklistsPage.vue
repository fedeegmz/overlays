<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onMounted, ref } from "vue";
import { commandErrorMessage } from "../lib/errors";
import { useChecklistStore } from "../stores/checklists";
import type { Checklist } from "../types";
import ChecklistEditor from "./ChecklistEditor.vue";
import ChecklistListView, { type ChecklistRow } from "./ChecklistListView.vue";
import ChecklistRunView from "./ChecklistRunView.vue";

const store = useChecklistStore();
const { checklists, checklistsError, checked } = storeToRefs(store);

const selectedId = ref<string | null>(null);

// A list deleted or removed elsewhere falls back to the list view.
const selected = computed(
  () => checklists.value.find((l) => l.id === selectedId.value) ?? null,
);

const rows = computed<ChecklistRow[]>(() =>
  checklists.value.map((checklist) => ({
    checklist,
    progress: store.progress(checklist.id),
    complete: store.isComplete(checklist.id),
  })),
);

const selectedChecked = computed<ReadonlySet<string>>(
  () => checked.value.get(selectedId.value ?? "") ?? new Set<string>(),
);

onMounted(() => {
  void store.refreshChecklists();
});

function toggleItem(itemId: string) {
  if (selectedId.value) store.toggleItem(selectedId.value, itemId);
}

function uncheckAll() {
  if (selectedId.value) store.uncheckAll(selectedId.value);
}

const editing = ref<Checklist | null>(null);
const editingIsNew = ref(false);
const editorError = ref<string | null>(null);
const saving = ref(false);

function startCreate() {
  editing.value = {
    id: crypto.randomUUID(),
    name: "",
    items: [{ id: crypto.randomUUID(), text: "" }],
  };
  editingIsNew.value = true;
  editorError.value = null;
}

function startEdit() {
  if (!selected.value) return;
  editing.value = selected.value;
  editingIsNew.value = false;
  editorError.value = null;
}

// Cancelling a new list returns to the list view; an existing one returns to
// its run view (selectedId is untouched).
function closeEditor() {
  editing.value = null;
}

async function saveEditor(draft: Checklist) {
  saving.value = true;
  editorError.value = null;
  try {
    await store.saveChecklist(draft);
    editing.value = null;
  } catch (err) {
    editorError.value = commandErrorMessage(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="checklists-page">
    <p v-if="checklistsError && !editing" class="error">
      {{ checklistsError }}
    </p>
    <ChecklistEditor
      v-if="editing"
      :key="editing.id"
      :checklist="editing"
      :is-new="editingIsNew"
      :error="editorError"
      :saving="saving"
      @save="saveEditor"
      @cancel="closeEditor"
    />
    <ChecklistRunView
      v-else-if="selected"
      :checklist="selected"
      :checked-ids="selectedChecked"
      @toggle="toggleItem"
      @uncheck-all="uncheckAll"
      @back="selectedId = null"
      @edit="startEdit"
    />
    <ChecklistListView
      v-else
      :rows="rows"
      @open="(id) => (selectedId = id)"
      @create="startCreate"
      @delete="(id) => void store.deleteChecklist(id)"
    />
  </div>
</template>

<style scoped>
.error {
  color: var(--danger);
  font-size: 13px;
  margin-bottom: 14px;
}
</style>
