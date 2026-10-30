import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { ConnectHandlers, Transport } from "../services/transport";
import type { InstancesSnapshot, OverlayInstance } from "../types";
import { createRemoteSession, pinFromHash } from "./session";

function instance(
  id: string,
  fields: Record<string, string> = {},
): OverlayInstance {
  return {
    id,
    templateId: "lower",
    number: 1,
    displayName: id,
    fields,
    visible: false,
  };
}

function snap(
  revision: number,
  instances: OverlayInstance[],
): InstancesSnapshot {
  return {
    revision,
    source: { kind: "desktop", clientId: "d" },
    instances,
  };
}

const manifest = {
  templates: [
    {
      id: "lower",
      name: "Lower",
      path: "lower",
      folder: "",
      fields: [
        { key: "title", label: "Title", type: "text" },
        { key: "logo", label: "Logo", type: "file" },
      ],
    },
  ],
  folders: [],
};

function setup(opts: { token?: boolean } = {}) {
  let handlers!: ConnectHandlers;
  const transport = {
    hasToken: vi.fn(() => opts.token ?? true),
    pair: vi.fn(async () => undefined),
    getTemplates: vi.fn(async () => manifest),
    getPresets: vi.fn(async () => []),
    getGroups: vi.fn(async () => []),
    showInstance: vi.fn(async (_id: string, _f?: Record<string, string>) =>
      snap(50, []),
    ),
    updateInstance: vi.fn(async (_id: string, _f: Record<string, string>) =>
      snap(50, []),
    ),
    hideInstance: vi.fn(async () => snap(50, [])),
    applyPreset: vi.fn(async () => snap(50, [])),
    startGroup: vi.fn(async () => ({ results: [], snapshot: snap(50, []) })),
    connect: vi.fn((h: ConnectHandlers) => {
      handlers = h;
      return { close: vi.fn() };
    }),
  };
  const session = createRemoteSession(transport as unknown as Transport, 250);
  return {
    session,
    transport,
    push: (s: InstancesSnapshot, fresh = false) => handlers.onState(s, fresh),
  };
}

describe("pinFromHash", () => {
  it("reads_a_six_digit_pin_from_the_fragment", () => {
    expect(pinFromHash("#pin=042137")).toBe("042137");
    expect(pinFromHash("#pin=12345")).toBeNull();
    expect(pinFromHash("#pin=abcdef")).toBeNull();
    expect(pinFromHash("")).toBeNull();
  });
});

describe("remote session", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("drops_snapshot_with_revision_le_last", async () => {
    const t = setup();
    await t.session.start("");
    t.push(snap(5, [instance("a", { title: "new" })]), true);
    t.push(snap(5, [instance("a", { title: "dup" })]));
    t.push(snap(4, [instance("a", { title: "old" })]));
    expect(t.session.instances.value[0].fields.title).toBe("new");
    t.push(snap(6, [instance("a", { title: "newer" })]));
    expect(t.session.instances.value[0].fields.title).toBe("newer");
  });

  it("fresh_snapshot_replaces_state_even_with_a_lower_revision", async () => {
    const t = setup();
    await t.session.start("");
    t.push(snap(90, [instance("a")]), true);
    t.push(snap(2, [instance("b")]), true); // desktop restarted
    expect(t.session.instances.value.map((i) => i.id)).toEqual(["b"]);
  });

  it("empty_registry_shows_empty_state_and_clears_when_an_instance_appears", async () => {
    const t = setup();
    await t.session.start("");
    expect(t.session.isEmpty.value).toBe(false); // nothing received yet
    t.push(snap(1, []), true);
    expect(t.session.isEmpty.value).toBe(true);
    t.push(snap(2, [instance("a")]));
    expect(t.session.isEmpty.value).toBe(false);
  });

  it("echo_of_own_write_does_not_overwrite_text_being_typed", async () => {
    const t = setup();
    await t.session.start("");
    t.push(snap(1, [instance("a", { title: "" })]), true);
    await vi.advanceTimersByTimeAsync(0);

    t.session.setField("a", "title", "Hel");
    t.session.setField("a", "title", "Hello");
    // an older echo ("Hel") arrives while the user already typed "Hello"
    t.push(snap(2, [instance("a", { title: "Hel" })]));
    expect(t.session.instances.value[0].fields.title).toBe("Hello");
  });

  it("untouched_fields_follow_the_server_while_another_is_typed", async () => {
    const t = setup();
    await t.session.start("");
    t.push(snap(1, [instance("a", { title: "", sub: "" })]), true);
    t.session.setField("a", "title", "Hi");
    t.push(snap(2, [instance("a", { title: "x", sub: "from desktop" })]));
    expect(t.session.instances.value[0].fields).toEqual({
      title: "Hi",
      sub: "from desktop",
    });
  });

  it("edits_are_sent_debounced_as_one_update_with_only_edited_keys", async () => {
    const t = setup();
    await t.session.start("");
    t.push(snap(1, [instance("a", { title: "", sub: "keep" })]), true);
    await vi.advanceTimersByTimeAsync(0);
    t.session.setField("a", "title", "H");
    await vi.advanceTimersByTimeAsync(200);
    t.session.setField("a", "title", "Hi");
    await vi.advanceTimersByTimeAsync(249);
    expect(t.transport.updateInstance).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(t.transport.updateInstance).toHaveBeenCalledOnce();
    expect(t.transport.updateInstance).toHaveBeenCalledWith("a", {
      title: "Hi",
    });
  });

  it("file_fields_are_read_only", async () => {
    const t = setup();
    await t.session.start("");
    t.push(snap(1, [instance("a", { logo: "x.png" })]), true);
    await vi.advanceTimersByTimeAsync(0); // templates loaded
    t.session.setField("a", "logo", "evil.png");
    await vi.advanceTimersByTimeAsync(500);
    expect(t.transport.updateInstance).not.toHaveBeenCalled();
    expect(t.session.instances.value[0].fields.logo).toBe("x.png");
  });

  it("show_carries_the_pending_edit", async () => {
    const t = setup();
    await t.session.start("");
    t.push(snap(1, [instance("a", { title: "" })]), true);
    await vi.advanceTimersByTimeAsync(0);
    t.session.setField("a", "title", "Hi");
    await t.session.show("a");
    expect(t.transport.showInstance).toHaveBeenCalledWith("a", { title: "Hi" });
    await vi.advanceTimersByTimeAsync(500);
    expect(t.transport.updateInstance).not.toHaveBeenCalled();
  });

  it("group_start_reports_conflicts_and_missing_templates", async () => {
    const t = setup();
    await t.session.start("");
    t.transport.startGroup.mockResolvedValueOnce({
      results: [
        { template: "lower", ok: true, instanceId: "a", idConflict: true },
        { template: "gone", ok: false, reason: "templateMissing" },
        { template: "lower", ok: true, instanceId: "b", idConflict: false },
      ],
      snapshot: snap(9, [instance("a"), instance("b")]),
    } as never);
    await t.session.startGroup("Intro / Day 1");
    expect(t.transport.startGroup).toHaveBeenCalledWith("Intro / Day 1");
    expect(t.session.notice.value).toEqual({
      name: "Intro / Day 1",
      conflicts: 1,
      missing: 1,
    });
    expect(t.session.instances.value).toHaveLength(2);
  });

  it("unauthorized_returns_to_pairing_with_a_repair_flag", async () => {
    const t = setup();
    await t.session.start("");
    t.session.handleUnauthorized();
    expect(t.session.phase.value).toBe("pair");
    expect(t.session.needsRepair.value).toBe(true);
  });

  it("pin_in_the_fragment_pairs_automatically", async () => {
    const t = setup({ token: false });
    expect(t.session.phase.value).toBe("pair");
    await t.session.start("#pin=042137");
    expect(t.transport.pair).toHaveBeenCalledWith("042137");
    expect(t.session.phase.value).toBe("control");
    expect(t.transport.connect).toHaveBeenCalled();
  });

  it("bad_pin_stays_on_pairing_with_the_server_error", async () => {
    const t = setup({ token: false });
    t.transport.pair.mockRejectedValueOnce({ code: "remote.unauthorized" });
    await t.session.start("");
    await t.session.pairWithPin("000000");
    expect(t.session.phase.value).toBe("pair");
    expect(t.session.pairError.value).toEqual({ code: "remote.unauthorized" });
  });

  it("malformed_pin_is_rejected_locally", async () => {
    const t = setup({ token: false });
    await t.session.pairWithPin("12");
    expect(t.transport.pair).not.toHaveBeenCalled();
    expect(t.session.pairError.value).toEqual({ code: "remote.invalid_pin" });
  });
});
