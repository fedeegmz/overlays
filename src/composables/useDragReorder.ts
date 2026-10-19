import { ref } from "vue";

export interface DropTarget {
  index: number;
  after: boolean;
}

/**
 * Native HTML5 drag-and-drop reorder for a list of rows. The caller binds the
 * returned handlers to each draggable row and receives `onMove(from, to)` with
 * the final index of the moved row once it is dropped.
 */
export function useDragReorder(onMove: (from: number, to: number) => void) {
  const dragFromIndex = ref<number | null>(null);
  const dropTarget = ref<DropTarget | null>(null);

  function handleDragStart(event: DragEvent, index: number) {
    dragFromIndex.value = index;
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      // Firefox/WebKit only start a drag when some data is set.
      event.dataTransfer.setData("text/plain", String(index));
    }
  }

  function handleDragOver(event: DragEvent, index: number) {
    if (dragFromIndex.value === null) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
    const row = event.currentTarget as HTMLElement;
    const rect = row.getBoundingClientRect();
    const after = event.clientY > rect.top + rect.height / 2;
    dropTarget.value = { index, after };
  }

  function handleDragLeave(event: DragEvent, index: number) {
    const row = event.currentTarget as HTMLElement;
    const next = event.relatedTarget as Node | null;
    if (next && row.contains(next)) return;
    if (dropTarget.value?.index === index) dropTarget.value = null;
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    const from = dragFromIndex.value;
    const target = dropTarget.value;
    resetDrag();
    if (from === null || !target) return;
    // Insertion point in the original list, then shift left if the source
    // sits before it (it is removed before being re-inserted).
    const insertAt = target.after ? target.index + 1 : target.index;
    const to = insertAt > from ? insertAt - 1 : insertAt;
    onMove(from, to);
  }

  function resetDrag() {
    dragFromIndex.value = null;
    dropTarget.value = null;
  }

  return {
    dragFromIndex,
    dropTarget,
    handleDragStart,
    handleDragOver,
    handleDragLeave,
    handleDrop,
    resetDrag,
  };
}
