import { defineStore } from "pinia";
import { ref } from "vue";
import { commandErrorMessage } from "../lib/errors";
import {
  deleteChecklist as deleteChecklistApi,
  listChecklists,
  saveChecklist as saveChecklistApi,
} from "../services/checklistsApi";
import type { Checklist, ChecklistProgress } from "../types";

export const useChecklistStore = defineStore("checklists", () => {
  const checklists = ref<Checklist[]>([]);
  const checklistsError = ref<string | null>(null);
  /**
   * Checked item ids per list id. Memory only on purpose: checks are never
   * persisted, so every app start begins unchecked.
   */
  const checked = ref<Map<string, Set<string>>>(new Map());

  /** Drops unknown lists and item ids that no longer exist. */
  function pruneChecked(): void {
    const next = new Map<string, Set<string>>();
    for (const list of checklists.value) {
      const set = checked.value.get(list.id);
      if (!set) continue;
      const ids = new Set(list.items.map((i) => i.id));
      const kept = new Set([...set].filter((id) => ids.has(id)));
      if (kept.size > 0) next.set(list.id, kept);
    }
    checked.value = next;
  }

  function applyLists(lists: Checklist[]): void {
    checklists.value = lists;
    pruneChecked();
  }

  async function refreshChecklists(): Promise<void> {
    checklistsError.value = null;
    try {
      applyLists(await listChecklists());
    } catch (err) {
      checklistsError.value = commandErrorMessage(err);
    }
  }

  /** Re-throws so the editor can show the error next to the form. */
  async function saveChecklist(draft: Checklist): Promise<void> {
    checklistsError.value = null;
    try {
      applyLists(await saveChecklistApi(draft.id, draft.name, draft.items));
    } catch (err) {
      checklistsError.value = commandErrorMessage(err);
      throw err;
    }
  }

  async function deleteChecklist(id: string): Promise<void> {
    checklistsError.value = null;
    try {
      applyLists(await deleteChecklistApi(id));
    } catch (err) {
      checklistsError.value = commandErrorMessage(err);
    }
  }

  function isChecked(listId: string, itemId: string): boolean {
    return checked.value.get(listId)?.has(itemId) ?? false;
  }

  function toggleItem(listId: string, itemId: string): void {
    const next = new Map(checked.value);
    const set = new Set(next.get(listId));
    if (set.has(itemId)) set.delete(itemId);
    else set.add(itemId);
    if (set.size > 0) next.set(listId, set);
    else next.delete(listId);
    checked.value = next;
  }

  function uncheckAll(listId: string): void {
    if (!checked.value.has(listId)) return;
    const next = new Map(checked.value);
    next.delete(listId);
    checked.value = next;
  }

  function progress(listId: string): ChecklistProgress {
    const list = checklists.value.find((l) => l.id === listId);
    if (!list) return { done: 0, total: 0 };
    const done = list.items.filter((i) => isChecked(listId, i.id)).length;
    return { done, total: list.items.length };
  }

  function isComplete(listId: string): boolean {
    const { done, total } = progress(listId);
    return total > 0 && done === total;
  }

  return {
    checklists,
    checklistsError,
    checked,
    refreshChecklists,
    saveChecklist,
    deleteChecklist,
    isChecked,
    toggleItem,
    uncheckAll,
    progress,
    isComplete,
    pruneChecked,
  };
});
