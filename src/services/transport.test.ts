import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { InstancesSnapshot } from "../types";
import {
  createTransport,
  REVOKED_CLOSE_CODE,
  type SocketLike,
  TOKEN_STORAGE_KEY,
} from "./transport";

const BASE = "http://192.168.1.20:4848/remote/api";

class FakeSocket implements SocketLike {
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onclose: ((event: { code: number }) => void) | null = null;
  closed = false;
  constructor(readonly url: string) {}
  close(): void {
    this.closed = true;
  }
  open(): void {
    this.onopen?.();
  }
  push(snapshot: Partial<InstancesSnapshot>): void {
    this.onmessage?.({ data: JSON.stringify({ type: "state", ...snapshot }) });
  }
  drop(code = 1006): void {
    this.onclose?.({ code });
  }
}

function memoryStorage(initial?: string) {
  const map = new Map<string, string>();
  if (initial) map.set(TOKEN_STORAGE_KEY, initial);
  return {
    getItem: (k: string) => map.get(k) ?? null,
    setItem: (k: string, v: string) => void map.set(k, v),
    removeItem: (k: string) => void map.delete(k),
  };
}

function reply(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status });
}

function setup(opts: { token?: string; fetchImpl?: typeof fetch } = {}) {
  const sockets: FakeSocket[] = [];
  const fetchFn = vi.fn(
    opts.fetchImpl ?? (async () => reply(200, { revision: 1 })),
  ) as unknown as typeof fetch & ReturnType<typeof vi.fn>;
  const onUnauthorized = vi.fn();
  const storage = memoryStorage(opts.token);
  const transport = createTransport({
    baseUrl: BASE,
    storage,
    fetchFn,
    createSocket: (url) => {
      const socket = new FakeSocket(url);
      sockets.push(socket);
      return socket;
    },
    clientId: "client-1",
    onUnauthorized,
  });
  return { transport, fetchFn, sockets, onUnauthorized, storage };
}

function lastCall(fetchFn: ReturnType<typeof vi.fn>) {
  const calls = fetchFn.mock.calls;
  const [url, init] = calls[calls.length - 1] as [string, RequestInit];
  return { url, init };
}

describe("transport requests", () => {
  it("group_name_with_space_and_slash_is_encoded_in_the_path", async () => {
    const { transport, fetchFn } = setup({ token: "t" });
    await transport.startGroup("Intro / Day 1");
    expect(lastCall(fetchFn).url).toBe(
      `${BASE}/groups/Intro%20%2F%20Day%201/start`,
    );
  });

  it("instance_id_is_encoded_in_the_path", async () => {
    const { transport, fetchFn } = setup({ token: "t" });
    await transport.hideInstance("a b/c");
    expect(lastCall(fetchFn).url).toBe(`${BASE}/instances/a%20b%2Fc/hide`);
  });

  it("sends_bearer_token_and_client_id", async () => {
    const { transport, fetchFn } = setup({ token: "secret" });
    await transport.getState();
    const { init } = lastCall(fetchFn);
    const headers = init.headers as Record<string, string>;
    expect(headers.Authorization).toBe("Bearer secret");
    expect(headers["X-Client-Id"]).toBe("client-1");
  });

  it("update_posts_only_the_given_fields", async () => {
    const { transport, fetchFn } = setup({ token: "t" });
    await transport.updateInstance("i1", { title: "Hi" });
    const { url, init } = lastCall(fetchFn);
    expect(url).toBe(`${BASE}/instances/i1/update`);
    expect(JSON.parse(init.body as string)).toEqual({
      fields: { title: "Hi" },
    });
  });

  it("pair_stores_the_token_without_sending_one", async () => {
    const { transport, fetchFn, storage } = setup({
      fetchImpl: async () => reply(200, { token: "issued" }),
    });
    await transport.pair("123456");
    expect(storage.getItem(TOKEN_STORAGE_KEY)).toBe("issued");
    const headers = lastCall(fetchFn).init.headers as Record<string, string>;
    expect(headers.Authorization).toBeUndefined();
    expect(transport.hasToken()).toBe(true);
  });

  it("wrong_pin_401_is_an_error_not_an_unauthorized_signal", async () => {
    const { transport, onUnauthorized } = setup({
      fetchImpl: async () => reply(401, { code: "remote.unauthorized" }),
    });
    await expect(transport.pair("000000")).rejects.toEqual({
      code: "remote.unauthorized",
    });
    expect(onUnauthorized).not.toHaveBeenCalled();
  });

  it("rejected_token_401_clears_it_and_signals_repair", async () => {
    const { transport, onUnauthorized, storage } = setup({
      token: "old",
      fetchImpl: async () => reply(401, { code: "remote.unauthorized" }),
    });
    await expect(transport.getState()).rejects.toEqual({
      code: "remote.unauthorized",
    });
    expect(storage.getItem(TOKEN_STORAGE_KEY)).toBeNull();
    expect(onUnauthorized).toHaveBeenCalledOnce();
  });

  it("network_failure_maps_to_unreachable", async () => {
    const { transport } = setup({
      token: "t",
      fetchImpl: async () => {
        throw new TypeError("offline");
      },
    });
    await expect(transport.getState()).rejects.toEqual({
      code: "remote.unreachable",
    });
  });
});

describe("transport websocket", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  function connect(t: ReturnType<typeof setup>) {
    const onState = vi.fn();
    const onStatus = vi.fn();
    const connection = t.transport.connect({ onState, onStatus });
    return { connection, onState, onStatus };
  }

  it("ws_url_uses_ws_scheme_and_the_encoded_token", () => {
    const t = setup({ token: "a+b/c" });
    connect(t);
    expect(t.sockets[0].url).toBe(
      "ws://192.168.1.20:4848/remote/api/ws?token=a%2Bb%2Fc",
    );
  });

  it("first_state_after_each_connect_is_fresh", async () => {
    const t = setup({ token: "t" });
    const { onState } = connect(t);
    t.sockets[0].open();
    t.sockets[0].push({ revision: 5 });
    t.sockets[0].push({ revision: 6 });
    expect(onState.mock.calls.map((c) => c[1])).toEqual([true, false]);

    t.sockets[0].drop();
    await vi.advanceTimersByTimeAsync(1000);
    t.sockets[1].open();
    t.sockets[1].push({ revision: 1 });
    expect(onState.mock.calls[onState.mock.calls.length - 1][1]).toBe(true);
  });

  it("reconnects_with_growing_backoff_and_resets_after_open", async () => {
    const t = setup({ token: "t" });
    connect(t);
    t.sockets[0].open();
    t.sockets[0].drop();
    await vi.advanceTimersByTimeAsync(999);
    expect(t.sockets).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(t.sockets).toHaveLength(2);

    // never opened: the next wait doubles
    t.sockets[1].drop();
    await vi.advanceTimersByTimeAsync(1999);
    expect(t.sockets).toHaveLength(2);
    await vi.advanceTimersByTimeAsync(1);
    expect(t.sockets).toHaveLength(3);
  });

  it("revoked_close_4001_clears_token_and_does_not_reconnect", async () => {
    const t = setup({ token: "t" });
    connect(t);
    t.sockets[0].open();
    t.sockets[0].drop(REVOKED_CLOSE_CODE);
    await vi.advanceTimersByTimeAsync(60_000);
    expect(t.sockets).toHaveLength(1);
    expect(t.onUnauthorized).toHaveBeenCalledOnce();
    expect(t.transport.hasToken()).toBe(false);
  });

  it("refused_handshake_probes_over_rest_and_stops_on_401", async () => {
    const t = setup({
      token: "stale",
      fetchImpl: async () => reply(401, { code: "remote.unauthorized" }),
    });
    connect(t);
    t.sockets[0].drop(); // never opened
    await vi.advanceTimersByTimeAsync(60_000);
    expect(t.onUnauthorized).toHaveBeenCalledOnce();
    expect(t.sockets).toHaveLength(1);
  });

  it("close_stops_reconnecting", async () => {
    const t = setup({ token: "t" });
    const { connection } = connect(t);
    t.sockets[0].open();
    connection.close();
    expect(t.sockets[0].closed).toBe(true);
    await vi.advanceTimersByTimeAsync(60_000);
    expect(t.sockets).toHaveLength(1);
  });
});
