// State and behavior of the phone page, kept apart from the views so it can be
// tested without a DOM. Like everything under src/remote it must not import
// Tauri: it runs in a phone browser.

import { computed, reactive, ref } from "vue";

import type { Connection, Transport } from "../services/transport";
import type {
  CommandError,
  GroupPreset,
  InstancesSnapshot,
  OverlayInstance,
  Preset,
  TemplateField,
  TemplateInfo,
} from "../types";
import { isCommandError } from "../types";

const PIN_PATTERN = /^\d{6}$/;

export interface GroupNotice {
  name: string;
  /** Entries whose saved instance id was taken, so a new id was used. */
  conflicts: number;
  /** Entries skipped because their template no longer exists. */
  missing: number;
}

/** `#pin=NNNNNN` from the pairing QR, or null. */
export function pinFromHash(hash: string): string | null {
  const match = /^#pin=(\d{6})$/.exec(hash);
  return match ? match[1] : null;
}

function asError(err: unknown): CommandError {
  return isCommandError(err) ? err : { code: "remote.unreachable" };
}

export type RemoteSession = ReturnType<typeof createRemoteSession>;

export function createRemoteSession(transport: Transport) {
  const phase = ref<"pair" | "control">("pair");
  const pairing = ref(false);
  const pairError = ref<CommandError | null>(null);
  /** The device was paired but its token stopped working. */
  const needsRepair = ref(false);

  const instances = ref<OverlayInstance[]>([]);
  const templates = ref<TemplateInfo[]>([]);
  const presets = ref<Preset[]>([]);
  const groups = ref<GroupPreset[]>([]);
  const online = ref(false);
  const loaded = ref(false);
  const notice = ref<GroupNotice | null>(null);
  const actionError = ref<CommandError | null>(null);

  const isEmpty = computed(() => loaded.value && instances.value.length === 0);

  let revision = -1;
  let connection: Connection | null = null;
  // Edited keys not yet confirmed by the server, per instance id. They stay
  // local until the user sends them with update or show.
  const drafts = reactive<Record<string, Record<string, string>>>({});

  function templateOf(instance: OverlayInstance): TemplateInfo | undefined {
    return templates.value.find((t) => t.id === instance.templateId);
  }

  function fieldsOf(instance: OverlayInstance): TemplateField[] {
    return templateOf(instance)?.fields ?? [];
  }

  function presetsFor(templateId: string): Preset[] {
    return presets.value.filter((p) => p.template === templateId);
  }

  function clearDraft(id: string): void {
    delete drafts[id];
  }

  function hasPendingEdits(id: string): boolean {
    return drafts[id] !== undefined;
  }

  /**
   * Applies a registry snapshot unless it is not newer than the last one (a
   * request result and its WebSocket echo both arrive). `fresh` snapshots, the
   * first after each (re)connect, always apply: the desktop may have restarted
   * and counted revisions from zero again. Keys with an unsent local edit keep
   * the local text, so an echo never overwrites what is being typed.
   */
  function applySnapshot(snapshot: InstancesSnapshot, fresh = false): void {
    if (!fresh && snapshot.revision <= revision) return;
    revision = snapshot.revision;
    const live = new Set(snapshot.instances.map((i) => i.id));
    for (const id of Object.keys(drafts)) {
      if (!live.has(id)) clearDraft(id);
    }
    instances.value = snapshot.instances.map((i) =>
      drafts[i.id] ? { ...i, fields: { ...i.fields, ...drafts[i.id] } } : i,
    );
    loaded.value = true;
  }

  async function loadReference(): Promise<void> {
    try {
      const [manifest, allPresets, allGroups] = await Promise.all([
        transport.getTemplates(),
        transport.getPresets(),
        transport.getGroups(),
      ]);
      templates.value = manifest.templates;
      presets.value = allPresets;
      groups.value = allGroups;
    } catch (err) {
      reportFailure(err);
    }
  }

  function reportFailure(err: unknown): void {
    // A rejected token already sent the page back to pairing.
    if (phase.value === "control") actionError.value = asError(err);
  }

  function enterControl(): void {
    connection?.close();
    phase.value = "control";
    needsRepair.value = false;
    pairError.value = null;
    connection = transport.connect({
      onState: (snapshot, fresh) => {
        if (fresh) void loadReference();
        applySnapshot(snapshot, fresh);
      },
      onStatus: (isOnline) => {
        online.value = isOnline;
        if (isOnline) actionError.value = null;
      },
    });
  }

  /** `hash` is `location.hash`: a QR scan arrives with `#pin=NNNNNN`. */
  async function start(hash: string): Promise<void> {
    const pin = pinFromHash(hash);
    if (pin) return pairWithPin(pin);
    if (transport.hasToken()) enterControl();
  }

  async function pairWithPin(pin: string): Promise<void> {
    const trimmed = pin.trim();
    if (!PIN_PATTERN.test(trimmed)) {
      pairError.value = { code: "remote.invalid_pin" };
      return;
    }
    pairing.value = true;
    pairError.value = null;
    try {
      await transport.pair(trimmed);
      enterControl();
    } catch (err) {
      pairError.value = asError(err);
    } finally {
      pairing.value = false;
    }
  }

  /** The backend rejected or revoked the token: back to the pairing view. */
  function handleUnauthorized(): void {
    connection?.close();
    connection = null;
    for (const id of Object.keys(drafts)) clearDraft(id);
    online.value = false;
    loaded.value = false;
    instances.value = [];
    revision = -1;
    phase.value = "pair";
    needsRepair.value = true;
    pairError.value = null;
  }

  // --- Editing -------------------------------------------------------------

  /** Drops from the draft every key the server now holds as sent. */
  function settle(id: string, sent: Record<string, string>): void {
    const draft = drafts[id];
    if (!draft) return;
    for (const [key, value] of Object.entries(sent)) {
      if (draft[key] === value) delete draft[key];
    }
    if (Object.keys(draft).length === 0) delete drafts[id];
  }

  async function commit(
    id: string,
    send: (fields: Record<string, string>) => Promise<InstancesSnapshot>,
  ): Promise<void> {
    const sent = { ...(drafts[id] ?? {}) };
    try {
      const snapshot = await send(sent);
      settle(id, sent);
      applySnapshot(snapshot);
    } catch (err) {
      reportFailure(err);
    }
  }

  function setField(id: string, key: string, value: string): void {
    const instance = instances.value.find((i) => i.id === id);
    if (!instance) return;
    // File fields are read-only here; the backend drops them as well.
    if (fieldsOf(instance).some((f) => f.key === key && f.type === "file")) {
      return;
    }
    drafts[id] = { ...(drafts[id] ?? {}), [key]: value };
    instance.fields = { ...instance.fields, [key]: value };
  }

  async function update(id: string): Promise<void> {
    await commit(id, (fields) => transport.updateInstance(id, fields));
  }

  async function show(id: string): Promise<void> {
    await commit(id, (fields) => transport.showInstance(id, fields));
  }

  async function hide(id: string): Promise<void> {
    try {
      applySnapshot(await transport.hideInstance(id));
    } catch (err) {
      reportFailure(err);
    }
  }

  async function applyPreset(id: string, name: string): Promise<void> {
    clearDraft(id);
    try {
      applySnapshot(await transport.applyPreset(id, name));
    } catch (err) {
      reportFailure(err);
    }
  }

  async function startGroup(name: string): Promise<void> {
    notice.value = null;
    try {
      const response = await transport.startGroup(name);
      applySnapshot(response.snapshot);
      notice.value = {
        name,
        conflicts: response.results.filter((r) => r.ok && r.idConflict).length,
        missing: response.results.filter((r) => !r.ok).length,
      };
    } catch (err) {
      reportFailure(err);
    }
  }

  function dispose(): void {
    connection?.close();
    connection = null;
    for (const id of Object.keys(drafts)) clearDraft(id);
  }

  return {
    phase,
    pairing,
    pairError,
    needsRepair,
    instances,
    templates,
    groups,
    online,
    loaded,
    isEmpty,
    notice,
    actionError,
    fieldsOf,
    presetsFor,
    hasPendingEdits,
    start,
    pairWithPin,
    handleUnauthorized,
    setField,
    update,
    show,
    hide,
    applyPreset,
    startGroup,
    dispose,
  };
}
