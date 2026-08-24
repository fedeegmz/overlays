import { defineStore } from "pinia";
import { computed, ref } from "vue";

import {
  getServerStatus,
  sendOverlayUpdate,
  type OverlayAction,
} from "../services/overlayApi";
import type { OverlayInstance } from "../types";
import { useTemplateStore } from "./templates";

function generateId(): string {
  return crypto.randomUUID();
}

export const useInstanceStore = defineStore("instances", () => {
  const templateStore = useTemplateStore();

  const instances = ref<OverlayInstance[]>([]);
  const activeInstanceId = ref<string | null>(null);
  const serverPort = ref(0);

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

  function instanceDisplayName(instance: OverlayInstance): string {
    const baseName =
      templateStore.getById(instance.templateId)?.name ?? instance.templateId;
    const sameTemplate = instances.value.filter(
      (i) => i.templateId === instance.templateId,
    );
    if (sameTemplate.length <= 1) return baseName;
    const index = sameTemplate.indexOf(instance) + 1;
    return `${baseName} #${index}`;
  }

  function createInstance(templateId: string): string {
    const id = generateId();
    const instance: OverlayInstance = {
      id,
      templateId,
      fields: templateStore.getDefaults(templateId),
      visible: false,
    };
    instances.value.push(instance);
    activeInstanceId.value = id;
    return id;
  }

  function selectInstance(id: string): void {
    activeInstanceId.value = id;
  }

  function buildFields(): Record<string, string> {
    if (!activeInstance.value || !activeTemplate.value) return {};
    const result: Record<string, string> = {};
    for (const field of activeTemplate.value.fields) {
      result[field.key] = activeInstance.value.fields[field.key] ?? "";
    }
    return result;
  }

  async function sendToInstance(
    instanceId: string,
    action: OverlayAction,
    fields: Record<string, string>,
  ): Promise<void> {
    const instance = instances.value.find((i) => i.id === instanceId);
    if (!instance) return;
    await sendOverlayUpdate({
      instanceId,
      template: instance.templateId,
      action,
      fields,
    });
  }

  async function show(): Promise<void> {
    if (!activeInstance.value) return;
    await sendToInstance(activeInstance.value.id, "show", buildFields());
    activeInstance.value.visible = true;
  }

  async function update(): Promise<void> {
    if (!activeInstance.value) return;
    await sendToInstance(activeInstance.value.id, "update", buildFields());
  }

  async function hide(): Promise<void> {
    if (!activeInstance.value) return;
    await sendToInstance(activeInstance.value.id, "hide", {});
    activeInstance.value.visible = false;
  }

  async function toggleVisibility(): Promise<void> {
    if (!activeInstance.value) return;
    if (activeInstance.value.visible) {
      await hide();
    } else {
      await show();
    }
  }

  function removeInstance(id: string): void {
    const instance = instances.value.find((i) => i.id === id);
    if (instance && instance.visible) {
      void sendToInstance(id, "hide", {});
      instance.visible = false;
    }
    instances.value = instances.value.filter((i) => i.id !== id);
    if (activeInstanceId.value === id) {
      activeInstanceId.value =
        instances.value.length > 0 ? instances.value[0].id : null;
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
    activeInstance,
    activeTemplate,
    overlayUrl,
    instanceDisplayName,
    createInstance,
    removeInstance,
    selectInstance,
    buildFields,
    show,
    update,
    hide,
    toggleVisibility,
    fetchServerPort,
  };
});
