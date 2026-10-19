<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onMounted, ref } from "vue";
import { useChecklistStore } from "../stores/checklists";
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

// Create and edit arrive with the editor (WU6); until then they are no-ops.
function noop() {}
</script>

<template>
  <div class="checklists-page">
    <p v-if="checklistsError" class="error">{{ checklistsError }}</p>
    <ChecklistRunView
      v-if="selected"
      :checklist="selected"
      :checked-ids="selectedChecked"
      @toggle="toggleItem"
      @uncheck-all="uncheckAll"
      @back="selectedId = null"
      @edit="noop"
    />
    <ChecklistListView
      v-else
      :rows="rows"
      @open="(id) => (selectedId = id)"
      @create="noop"
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
