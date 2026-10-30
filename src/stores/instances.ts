import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { commandErrorMessage } from "../lib/errors";
import {
  applyPreset as applyPresetCommand,
  clientId,
  createInstance as createInstanceCommand,
  hideInstance as hideInstanceCommand,
  listInstances,
  moveInstance as moveInstanceCommand,
  onInstancesChanged,
  removeInstance as removeInstanceCommand,
  renameInstance as renameInstanceCommand,
  setInstanceFields,
  showInstance as showInstanceCommand,
  updateInstance as updateInstanceCommand,
} from "../services/instancesApi";
import { getServerStatus, sendOverlayUpdate } from "../services/overlayApi";
import type { InstancesSnapshot, OverlayInstance } from "../types";
import { useTemplateStore } from "./templates";

/** Idle time after the last keystroke before fields are committed. */
const DRAFT_DEBOUNCE_MS = 250;

function sameFields(
  a: Record<string, string>,
  b: Record<string, string>,
): boolean {
  const keys = Object.keys(a);
  return (
    keys.length === Object.keys(b).length && keys.every((k) => a[k] === b[k])
  );
}

export const useInstanceStore = defineStore("instances", () => {
  const templateStore = useTemplateStore();

  const instances = ref<OverlayInstance[]>([]);
  const activeInstanceId = ref<string | null>(null);
  const serverPort = ref(0);
  const revision = ref(-1);
  const lastChangeWasOwn = ref(false);
  const instancesError = ref<string | null>(null);

  // Uncommitted local field edits per instance id, and their commit timers.
  const drafts: Record<string, Record<string, string>> = {};
  const draftTimers = new Map<string, ReturnType<typeof setTimeout>>();
  let unlisten: (() => void) | undefined;

  const activeInstance = computed<OverlayInstance | null>(() => {
    if (!activeInstanceId.value) return null;
    return instances.value.find((i) => i.id === activeInstanceId.value) ?? null;
  });

  const activeTemplate = computed(() => {
    if (!activeInstance.value) return null;
    return templateStore.getById(activeInstance.value.templateId) ?? null;
  });

  const overlayUrl = computed<string | null>(() => {
    if (!serverPort.value || !activeInstance.value || !activeTemplate.value)
      return null;
    return `http://localhost:${serverPort.value}/overlay/${activeTemplate.value.path}?instance=${activeInstance.value.id}`;
  });

  const previewInstanceId = computed<string | null>(() => {
    if (!activeInstance.value) return null;
    return `preview-${activeInstance.value.id}`;
  });

  const previewUrl = computed<string | null>(() => {
    if (
      !serverPort.value ||
      !activeInstance.value ||
      !activeTemplate.value ||
      !previewInstanceId.value
    )
      return null;
    const template = activeTemplate.value;
    return `http://localhost:${serverPort.value}/overlay/${template.path}?instance=${previewInstanceId.value}`;
  });

  async function sendPreviewShow(): Promise<void> {
    const instanceId = previewInstanceId.value;
    const templateId = activeInstance.value?.templateId;
    if (!instanceId || !templateId) return;
    await sendOverlayUpdate({
      instanceId,
      template: templateId,
      action: "show",
      fields: buildFields(),
    });
  }

  function sendPreviewHide(): void {
    const instance = activeInstance.value;
    const instanceId = previewInstanceId.value;
    if (!instance || !instanceId) return;
    void sendOverlayUpdate({
      instanceId,
      template: instance.templateId,
      action: "hide",
      fields: {},
    });
  }

  async function openPreviewWindow(): Promise<void> {
    const template = activeTemplate.value;
    const instance = activeInstance.value;
    if (!previewUrl.value || !template || !instance) return;
    const label = `preview-${instance.id}`;
    const existing = await WebviewWindow.getByLabel(label);
    if (existing) {
      await existing.setFocus();
      return;
    }
    new WebviewWindow(label, {
      url: previewUrl.value,
      title: template.name,
      width: 800,
      height: 450,
    });
  }

  function instanceDefaultName(instance: OverlayInstance): string {
    const baseName =
      templateStore.getById(instance.templateId)?.name ?? instance.templateId;
    const sameTemplate = instances.value.filter(
      (i) => i.templateId === instance.templateId,
    );
    if (sameTemplate.length <= 1) return baseName;
    return `${baseName} #${instance.number}`;
  }

  function instanceDisplayName(instance: OverlayInstance): string {
    return instance.displayName;
  }

  // --- Snapshot handling -------------------------------------------------

  function clearDraft(id: string): void {
    delete drafts[id];
    const timer = draftTimers.get(id);
    if (timer) clearTimeout(timer);
    draftTimers.delete(id);
  }

  /**
   * Applies a registry snapshot unless it is not newer than the last one
   * (an invoke result and its event both arrive; the second is dropped).
   * Fields with an uncommitted local draft are kept over the snapshot's.
   */
  function applySnapshot(snapshot: InstancesSnapshot): void {
    if (snapshot.revision <= revision.value) return;
    revision.value = snapshot.revision;
    lastChangeWasOwn.value = snapshot.source.clientId === clientId;

    const live = new Set(snapshot.instances.map((i) => i.id));
    for (const id of Object.keys(drafts)) {
      if (!live.has(id)) clearDraft(id);
    }
    instances.value = snapshot.instances.map((i) =>
      drafts[i.id] ? { ...i, fields: { ...drafts[i.id] } } : i,
    );

    if (activeInstanceId.value && !live.has(activeInstanceId.value)) {
      activeInstanceId.value =
        instances.value.length > 0 ? instances.value[0].id : null;
    }
  }

  async function init(): Promise<void> {
    unlisten?.();
    unlisten = await onInstancesChanged(applySnapshot);
    applySnapshot(await listInstances());
  }

  // --- Drafts ------------------------------------------------------------

  function setField(id: string, key: string, value: string): void {
    const instance = instances.value.find((i) => i.id === id);
    if (!instance) return;
    const next = { ...(drafts[id] ?? instance.fields), [key]: value };
    drafts[id] = next;
    instance.fields = { ...next };
    const pending = draftTimers.get(id);
    if (pending) clearTimeout(pending);
    draftTimers.set(
      id,
      setTimeout(() => {
        void flushDraft(id).catch((err) => {
          instancesError.value = commandErrorMessage(err);
        });
      }, DRAFT_DEBOUNCE_MS),
    );
  }

  /** Commits a pending draft now (before show/update, or on save). */
  async function flushDraft(id: string): Promise<void> {
    const draft = drafts[id];
    if (!draft) return;
    const pending = draftTimers.get(id);
    if (pending) clearTimeout(pending);
    draftTimers.delete(id);
    const sent = { ...draft };
    const snapshot = await setInstanceFields(id, sent);
    // Still the same text: the server now holds it, so stop shielding it.
    const unchanged = drafts[id] !== undefined && sameFields(drafts[id], sent);
    if (unchanged) delete drafts[id];
    applySnapshot(snapshot);
  }

  async function flushAllDrafts(): Promise<void> {
    for (const id of Object.keys(drafts)) await flushDraft(id);
  }

  // --- Commands ----------------------------------------------------------

  async function createInstance(
    templateId: string,
    opts?: { id?: string; customName?: string },
  ): Promise<string> {
    sendPreviewHide();
    const result = await createInstanceCommand(templateId, opts);
    applySnapshot(result.snapshot);
    activeInstanceId.value = result.id;
    return result.id;
  }

  async function renameInstance(id: string, name: string): Promise<void> {
    applySnapshot(await renameInstanceCommand(id, name));
  }

  async function removeInstance(id: string): Promise<void> {
    clearDraft(id);
    applySnapshot(await removeInstanceCommand(id));
  }

  async function moveInstance(from: number, to: number): Promise<void> {
    applySnapshot(await moveInstanceCommand(from, to));
  }

  async function applyPreset(id: string, name: string): Promise<void> {
    clearDraft(id);
    applySnapshot(await applyPresetCommand(id, name));
  }

  function selectInstance(id: string): void {
    sendPreviewHide();
    activeInstanceId.value = id;
  }

  function fieldsFor(instance: OverlayInstance): Record<string, string> {
    const template = templateStore.getById(instance.templateId);
    if (!template) return {};
    const result: Record<string, string> = {};
    for (const field of template.fields) {
      result[field.key] = instance.fields[field.key] ?? "";
    }
    return result;
  }

  function buildFields(): Record<string, string> {
    if (!activeInstance.value) return {};
    return fieldsFor(activeInstance.value);
  }

  async function show(): Promise<void> {
    const id = activeInstanceId.value;
    if (!id) return;
    await flushDraft(id);
    applySnapshot(await showInstanceCommand(id));
  }

  async function update(): Promise<void> {
    const id = activeInstanceId.value;
    if (!id) return;
    await flushDraft(id);
    applySnapshot(await updateInstanceCommand(id));
  }

  async function hide(): Promise<void> {
    const id = activeInstanceId.value;
    if (!id) return;
    applySnapshot(await hideInstanceCommand(id));
  }

  async function toggleVisibility(): Promise<void> {
    if (!activeInstance.value) return;
    if (activeInstance.value.visible) {
      await hide();
    } else {
      await show();
    }
  }

  async function fetchServerPort(): Promise<void> {
    const status = await getServerStatus();
    if (status) serverPort.value = status.port;
  }

  return {
    instances,
    activeInstanceId,
    serverPort,
    revision,
    lastChangeWasOwn,
    instancesError,
    activeInstance,
    activeTemplate,
    overlayUrl,
    previewInstanceId,
    previewUrl,
    instanceDefaultName,
    instanceDisplayName,
    renameInstance,
    applySnapshot,
    init,
    setField,
    flushDraft,
    flushAllDrafts,
    applyPreset,
    createInstance,
    removeInstance,
    moveInstance,
    selectInstance,
    fieldsFor,
    buildFields,
    sendPreviewShow,
    openPreviewWindow,
    show,
    update,
    hide,
    toggleVisibility,
    fetchServerPort,
  };
});
