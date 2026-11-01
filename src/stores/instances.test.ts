import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { InstancesSnapshot, OverlayInstance } from "../types";

vi.mock("../services/instancesApi", () => ({
  clientId: "me",
  listInstances: vi.fn(),
  createInstance: vi.fn(),
  removeInstance: vi.fn(),
  renameInstance: vi.fn(),
  moveInstance: vi.fn(),
  setInstanceFields: vi.fn(),
  showInstance: vi.fn(),
  updateInstance: vi.fn(),
  hideInstance: vi.fn(),
  applyPreset: vi.fn(),
  startGroupPreset: vi.fn(),
  saveGroupFromInstances: vi.fn(),
  onInstancesChanged: vi.fn(),
}));
vi.mock("../services/overlayApi", () => ({
  getServerStatus: vi.fn(),
  sendOverlayUpdate: vi.fn(),
}));
vi.mock("../services/templatesApi", () => ({ listTemplates: vi.fn() }));
vi.mock("@tauri-apps/api/webviewWindow", () => ({ WebviewWindow: {} }));

import * as api from "../services/instancesApi";
import { useInstanceStore } from "./instances";

function instance(id: string, fields: Record<string, string> = {}) {
  return {
    id,
    templateId: "lower",
    number: 1,
    customName: null,
    displayName: "Lower",
    fields,
    visible: false,
  } satisfies OverlayInstance;
}

function snap(
  revision: number,
  instances: OverlayInstance[],
  clientId = "other",
): InstancesSnapshot {
  return { revision, source: { kind: "desktop", clientId }, instances };
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.useFakeTimers();
  vi.clearAllMocks();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("revision guard", () => {
  it("drops_snapshot_with_revision_le_last", () => {
    const store = useInstanceStore();
    store.applySnapshot(snap(5, [instance("a")]));
    store.applySnapshot(snap(5, [instance("b")]));
    store.applySnapshot(snap(4, [instance("c")]));
    expect(store.instances.map((i) => i.id)).toEqual(["a"]);
    expect(store.revision).toBe(5);
  });

  it("applies_newer_snapshot", () => {
    const store = useInstanceStore();
    store.applySnapshot(snap(1, [instance("a")]));
    store.applySnapshot(snap(2, [instance("a"), instance("b")]));
    expect(store.instances.map((i) => i.id)).toEqual(["a", "b"]);
  });

  it("invoke_result_snapshot_then_duplicate_event_dropped", async () => {
    const store = useInstanceStore();
    store.applySnapshot(snap(1, [instance("a")]));
    vi.mocked(api.renameInstance).mockResolvedValue(
      snap(2, [{ ...instance("a"), displayName: "Renamed" }], "me"),
    );
    await store.renameInstance("a", "Renamed");
    // The same revision arrives later as an event with stale content.
    store.applySnapshot(snap(2, [instance("a")], "me"));
    expect(store.instances[0].displayName).toBe("Renamed");
  });

  it("own_client_id_echo_recognized", () => {
    const store = useInstanceStore();
    store.applySnapshot(snap(1, [], "me"));
    expect(store.lastChangeWasOwn).toBe(true);
    store.applySnapshot(snap(2, [], "phone"));
    expect(store.lastChangeWasOwn).toBe(false);
  });
});

describe("local drafts", () => {
  it("dirty_draft_not_overwritten_by_snapshot", () => {
    const store = useInstanceStore();
    store.applySnapshot(snap(1, [instance("a", { title: "old" })]));
    store.setField("a", "title", "typing");
    store.applySnapshot(snap(2, [instance("a", { title: "remote" })], "phone"));
    expect(store.instances[0].fields.title).toBe("typing");
  });

  it("set_fields_debounced_250ms", async () => {
    const store = useInstanceStore();
    store.applySnapshot(snap(1, [instance("a", { title: "" })]));
    vi.mocked(api.setInstanceFields).mockResolvedValue(
      snap(2, [instance("a", { title: "abc" })], "me"),
    );
    store.setField("a", "title", "a");
    store.setField("a", "title", "ab");
    vi.advanceTimersByTime(200);
    store.setField("a", "title", "abc");
    vi.advanceTimersByTime(249);
    expect(api.setInstanceFields).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(api.setInstanceFields).toHaveBeenCalledTimes(1);
    expect(api.setInstanceFields).toHaveBeenCalledWith("a", { title: "abc" });
  });

  it("draft_cleared_after_commit_so_later_snapshots_apply", async () => {
    const store = useInstanceStore();
    store.applySnapshot(snap(1, [instance("a", { title: "" })]));
    vi.mocked(api.setInstanceFields).mockResolvedValue(
      snap(2, [instance("a", { title: "x" })], "me"),
    );
    store.setField("a", "title", "x");
    await vi.advanceTimersByTimeAsync(250);
    store.applySnapshot(snap(3, [instance("a", { title: "remote" })], "phone"));
    expect(store.instances[0].fields.title).toBe("remote");
  });
});

describe("local selection", () => {
  it("activeInstanceId_and_previews_stay_local", () => {
    const store = useInstanceStore();
    store.applySnapshot(snap(1, [instance("a"), instance("b")]));
    store.selectInstance("b");
    store.applySnapshot(snap(2, [instance("a"), instance("b")], "phone"));
    expect(store.activeInstanceId).toBe("b");
    expect(store.previewInstanceId).toBe("preview-b");
    expect(api.setInstanceFields).not.toHaveBeenCalled();
  });

  it("selection_falls_back_when_active_instance_disappears", () => {
    const store = useInstanceStore();
    store.applySnapshot(snap(1, [instance("a"), instance("b")]));
    store.selectInstance("b");
    store.applySnapshot(snap(2, [instance("a")], "phone"));
    expect(store.activeInstanceId).toBe("a");
    store.applySnapshot(snap(3, [], "phone"));
    expect(store.activeInstanceId).toBeNull();
  });
});
