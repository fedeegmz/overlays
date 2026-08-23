import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

import type {
  AppConfig,
  Manifest,
  OverlayInstance,
  Preset,
  TemplateInfo,
} from "../types";
import { setActiveLocale, toLocaleCode, translate } from "../i18n";

const templates = ref<TemplateInfo[] | null>(null);
const templatesError = ref<string | null>(null);
const instances = ref<OverlayInstance[]>([]);
const activeInstanceId = ref<string | null>(null);
const serverPort = ref(0);
const presets = ref<Preset[]>([]);
const presetsError = ref<string | null>(null);
const appConfig = ref<AppConfig>({ overlays_dir: null, language: null });
const configError = ref<string | null>(null);

const activeInstance = computed<OverlayInstance | null>(() => {
  if (!activeInstanceId.value) return null;
  return instances.value.find((i) => i.id === activeInstanceId.value) ?? null;
});

const activeTemplate = computed<TemplateInfo | null>(() => {
  if (!activeInstance.value || !templates.value) return null;
  return (
    templates.value.find((t) => t.id === activeInstance.value!.templateId) ??
    null
  );
});

const overlayUrl = computed<string | null>(() => {
  if (!serverPort.value || !activeInstance.value || !activeTemplate.value)
    return null;
  return `http://localhost:${serverPort.value}/overlay/${activeTemplate.value.path}?instance=${activeInstance.value.id}`;
});

function instanceDisplayName(instance: OverlayInstance): string {
  const template = templates.value?.find((t) => t.id === instance.templateId);
  const baseName = template?.name ?? instance.templateId;
  const sameTemplate = instances.value.filter(
    (i) => i.templateId === instance.templateId,
  );
  if (sameTemplate.length <= 1) return baseName;
  const index = sameTemplate.indexOf(instance) + 1;
  return `${baseName} #${index}`;
}

function generateId(): string {
  return crypto.randomUUID();
}

async function refreshConfig(): Promise<void> {
  configError.value = null;
  try {
    appConfig.value = await invoke<AppConfig>("get_config");
  } catch (err) {
    configError.value = String(err);
  }
}

async function setOverlaysDir(path: string): Promise<void> {
  configError.value = null;
  try {
    appConfig.value = await invoke<AppConfig>("set_overlays_dir", { path });
    await refreshTemplates();
  } catch (err) {
    configError.value = String(err);
    throw err;
  }
}

export async function loadStoredLanguage(): Promise<string | null> {
  try {
    appConfig.value = await invoke<AppConfig>("get_config");
    return appConfig.value.language;
  } catch {
    return null;
  }
}

async function setLanguage(lang: string): Promise<void> {
  configError.value = null;
  try {
    appConfig.value = await invoke<AppConfig>("set_language", { lang });
    const locale = toLocaleCode(appConfig.value.language);
    if (locale) setActiveLocale(locale);
  } catch (err) {
    configError.value = String(err);
    throw err;
  }
}

async function pickOverlaysDir(): Promise<void> {
  configError.value = null;
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      directory: true,
      multiple: false,
      title: translate("dialog.selectOverlaysDir"),
    });
    if (selected) {
      await setOverlaysDir(selected as string);
    }
  } catch (err) {
    configError.value = String(err);
  }
}

async function refreshTemplates(): Promise<void> {
  templatesError.value = null;
  try {
    const manifest = await invoke<Manifest>("list_templates");
    templates.value = manifest.templates;
  } catch (err) {
    templatesError.value = String(err);
  }
}

async function fetchServerPort(): Promise<void> {
  try {
    const status = await invoke<{ running: boolean; port: number }>(
      "get_server_status",
    );
    serverPort.value = status.port;
  } catch {
    // server may not respond yet in dev until Rust starts
  }
}

async function refreshPresets(): Promise<void> {
  presetsError.value = null;
  try {
    presets.value = await invoke<Preset[]>("list_presets");
  } catch (err) {
    presetsError.value = String(err);
  }
}

async function savePreset(nombre: string): Promise<void> {
  const trimmed = nombre.trim();
  if (!trimmed || !activeInstance.value || !activeTemplate.value) return;
  presetsError.value = null;
  try {
    presets.value = await invoke<Preset[]>("save_preset", {
      nombre: trimmed,
      template: activeInstance.value.templateId,
      fields: buildFields(),
    });
  } catch (err) {
    presetsError.value = String(err);
    throw err;
  }
}

async function applyPreset(preset: Preset): Promise<void> {
  if (!activeInstance.value) return;
  if (!templates.value?.some((t) => t.id === preset.template)) {
    presetsError.value = translate("errors.templateMissing", {
      template: preset.template,
    });
    return;
  }
  activeInstance.value.fields = { ...preset.fields };
  await send("show", { ...preset.fields });
  activeInstance.value.visible = true;
}

async function deletePreset(nombre: string): Promise<void> {
  presetsError.value = null;
  try {
    presets.value = await invoke<Preset[]>("delete_preset", { nombre });
  } catch (err) {
    presetsError.value = String(err);
  }
}

function createInstance(templateId: string): string {
  const id = generateId();
  const template = templates.value?.find((t) => t.id === templateId);
  const fields: Record<string, string> = {};
  if (template) {
    for (const f of template.fields) {
      if (f.default !== undefined) fields[f.key] = f.default;
    }
  }
  const instance: OverlayInstance = {
    id,
    templateId,
    fields,
    visible: false,
  };
  instances.value.push(instance);
  activeInstanceId.value = id;
  return id;
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

function selectInstance(id: string): void {
  activeInstanceId.value = id;
}

function buildFields(): Record<string, string> {
  if (!activeInstance.value || !activeTemplate.value) return {};
  const result: Record<string, string> = {};
  for (const campo of activeTemplate.value.fields) {
    result[campo.key] = activeInstance.value.fields[campo.key] ?? "";
  }
  return result;
}

async function sendToInstance(
  instanceId: string,
  action: string,
  payloadFields: Record<string, string>,
) {
  const instance = instances.value.find((i) => i.id === instanceId);
  if (!instance) return;
  await invoke("send_overlay_update", {
    instanceId,
    template: instance.templateId,
    action,
    fields: payloadFields,
  });
}

async function send(action: string, payloadFields: Record<string, string>) {
  if (!activeInstance.value) return;
  await sendToInstance(activeInstance.value.id, action, payloadFields);
}

async function show() {
  await send("show", buildFields());
  if (activeInstance.value) activeInstance.value.visible = true;
}

async function update() {
  await send("update", buildFields());
}

async function hide() {
  await send("hide", {});
  if (activeInstance.value) activeInstance.value.visible = false;
}

async function toggleVisibility() {
  if (!activeInstance.value) return;
  if (activeInstance.value.visible) {
    await hide();
  } else {
    await show();
  }
}

export function useOverlayControl() {
  return {
    templates,
    templatesError,
    instances,
    activeInstanceId,
    activeInstance,
    activeTemplate,
    overlayUrl,
    presets,
    presetsError,
    appConfig,
    configError,
    instanceDisplayName,
    refreshConfig,
    refreshTemplates,
    refreshPresets,
    createInstance,
    removeInstance,
    selectInstance,
    show,
    update,
    hide,
    toggleVisibility,
    savePreset,
    applyPreset,
    deletePreset,
    pickOverlaysDir,
    setOverlaysDir,
    setLanguage,
    loadStoredLanguage,
  };
}

export function initOverlayControl(): void {
  void refreshConfig();
  void refreshTemplates();
  void refreshPresets();
  void fetchServerPort();
}
