import { defineStore, storeToRefs } from "pinia";
import { computed, ref, watch } from "vue";

import { folderContents, folderExists } from "../lib/folderTree";
import { useTemplateStore } from "./templates";

function parentOf(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? "" : path.slice(0, i);
}

export const useOverlayExplorerStore = defineStore("overlayExplorer", () => {
  const { templates, folders } = storeToRefs(useTemplateStore());
  const currentFolder = ref("");

  const contents = computed(() =>
    folderContents(templates.value ?? [], folders.value, currentFolder.value),
  );

  function open(path: string): void {
    currentFolder.value = path;
  }

  function up(): void {
    currentFolder.value = parentOf(currentFolder.value);
  }

  // After a reload the folder may be gone: fall back to the nearest ancestor.
  watch([templates, folders], () => {
    if (templates.value === null) return;
    let path = currentFolder.value;
    while (!folderExists(templates.value, folders.value, path)) {
      path = parentOf(path);
    }
    currentFolder.value = path;
  });

  return { currentFolder, contents, open, up };
});
