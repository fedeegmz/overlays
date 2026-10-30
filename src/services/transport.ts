// HTTP/WebSocket client for the phone page. It runs in a plain browser, so it
// must not import anything from Tauri; every browser capability it needs is
// injected, which also keeps it testable under node.

import type {
  CommandError,
  GroupPreset,
  GroupStartResponse,
  InstancesSnapshot,
  Manifest,
  Preset,
} from "../types";
import { isCommandError } from "../types";

export const TOKEN_STORAGE_KEY = "overlays.remote.token";
export const REVOKED_CLOSE_CODE = 4001;

const RECONNECT_BASE_MS = 1000;
const RECONNECT_MAX_MS = 15000;

/** Local failure codes; they map to `errors.remote.*` strings. */
export const UNREACHABLE: CommandError = { code: "remote.unreachable" };

export interface TokenStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

/** The part of `WebSocket` the transport relies on. */
export interface SocketLike {
  onopen: (() => void) | null;
  onmessage: ((event: { data: unknown }) => void) | null;
  onclose: ((event: { code: number }) => void) | null;
  close(): void;
}

export interface TransportDeps {
  /** Absolute `http(s)://host:port/remote/api` (no trailing slash). */
  baseUrl: string;
  storage: TokenStorage;
  fetchFn: typeof fetch;
  createSocket: (url: string) => SocketLike;
  /** Random per page load; the backend echoes it as the change source. */
  clientId: string;
  /** The token was rejected or revoked: the page must pair again. */
  onUnauthorized: () => void;
}

export interface ConnectHandlers {
  /** `fresh` is true for the first state after each (re)connect. */
  onState: (snapshot: InstancesSnapshot, fresh: boolean) => void;
  onStatus: (online: boolean) => void;
}

export interface Connection {
  close: () => void;
}

/** One URL path segment; ids and group names may hold spaces or `/`. */
export function segment(value: string): string {
  return encodeURIComponent(value);
}

export function createTransport(deps: TransportDeps) {
  const { baseUrl, storage, fetchFn, createSocket, clientId } = deps;

  const token = (): string | null => storage.getItem(TOKEN_STORAGE_KEY);

  function forgetToken(): void {
    storage.removeItem(TOKEN_STORAGE_KEY);
    deps.onUnauthorized();
  }

  async function readError(res: Response): Promise<CommandError> {
    const body: unknown = await res.json().catch(() => null);
    return isCommandError(body) ? body : { code: `http.${res.status}` };
  }

  async function request<T>(
    method: "GET" | "POST",
    path: string,
    body?: unknown,
    authenticated = true,
  ): Promise<T> {
    const headers: Record<string, string> = { "X-Client-Id": clientId };
    const current = token();
    if (authenticated && current) headers.Authorization = `Bearer ${current}`;
    if (body !== undefined) headers["Content-Type"] = "application/json";

    let res: Response;
    try {
      res = await fetchFn(`${baseUrl}${path}`, {
        method,
        headers,
        body: body === undefined ? undefined : JSON.stringify(body),
      });
    } catch {
      throw UNREACHABLE;
    }
    if (!res.ok) {
      const error = await readError(res);
      if (res.status === 401 && authenticated) forgetToken();
      throw error;
    }
    return (await res.json()) as T;
  }

  async function pair(pin: string): Promise<void> {
    const { token: issued } = await request<{ token: string }>(
      "POST",
      "/pair",
      { pin },
      false,
    );
    storage.setItem(TOKEN_STORAGE_KEY, issued);
  }

  function socketUrl(): string {
    const wsBase = baseUrl.replace(/^http/, "ws");
    return `${wsBase}/ws?token=${encodeURIComponent(token() ?? "")}`;
  }

  /**
   * Keeps one state socket alive: reconnects with capped exponential backoff
   * and stops for good once the token is revoked (close code 4001) or a
   * probe request is rejected with 401.
   */
  function connect(handlers: ConnectHandlers): Connection {
    let stopped = false;
    let attempt = 0;
    let socket: SocketLike | null = null;
    let timer: ReturnType<typeof setTimeout> | undefined;

    function scheduleReconnect(): void {
      const delay = Math.min(
        RECONNECT_BASE_MS * 2 ** attempt,
        RECONNECT_MAX_MS,
      );
      attempt += 1;
      timer = setTimeout(open, delay);
    }

    async function afterClose(opened: boolean, code: number): Promise<void> {
      handlers.onStatus(false);
      if (stopped) return;
      if (code === REVOKED_CLOSE_CODE) {
        forgetToken();
        return;
      }
      // A refused handshake gives no close code; ask over REST why.
      if (!opened) await getState().catch(() => undefined);
      if (stopped || token() === null) return;
      scheduleReconnect();
    }

    function open(): void {
      if (stopped || token() === null) return;
      const current = createSocket(socketUrl());
      socket = current;
      let opened = false;
      let first = true;
      current.onopen = () => {
        opened = true;
        attempt = 0;
        handlers.onStatus(true);
      };
      current.onmessage = (event) => {
        if (typeof event.data !== "string") return;
        let message: { type?: string } & Partial<InstancesSnapshot>;
        try {
          message = JSON.parse(event.data);
        } catch {
          return;
        }
        if (message.type !== "state") return;
        handlers.onState(message as InstancesSnapshot, first);
        first = false;
      };
      current.onclose = (event) => {
        void afterClose(opened, event.code);
      };
    }

    open();
    return {
      close() {
        stopped = true;
        if (timer) clearTimeout(timer);
        const current = socket;
        socket = null;
        if (current) {
          current.onclose = null;
          current.close();
        }
      },
    };
  }

  function getState(): Promise<InstancesSnapshot> {
    return request("GET", "/state");
  }

  return {
    hasToken: () => token() !== null,
    pair,
    getState,
    getTemplates: () => request<Manifest>("GET", "/templates"),
    getPresets: () => request<Preset[]>("GET", "/presets"),
    getGroups: () => request<GroupPreset[]>("GET", "/groups"),
    showInstance: (id: string, fields?: Record<string, string>) =>
      request<InstancesSnapshot>("POST", `/instances/${segment(id)}/show`, {
        fields: fields ?? {},
      }),
    updateInstance: (id: string, fields: Record<string, string>) =>
      request<InstancesSnapshot>("POST", `/instances/${segment(id)}/update`, {
        fields,
      }),
    hideInstance: (id: string) =>
      request<InstancesSnapshot>("POST", `/instances/${segment(id)}/hide`),
    applyPreset: (id: string, name: string) =>
      request<InstancesSnapshot>("POST", `/instances/${segment(id)}/preset`, {
        name,
      }),
    startGroup: (name: string) =>
      request<GroupStartResponse>("POST", `/groups/${segment(name)}/start`),
    connect,
  };
}

export type Transport = ReturnType<typeof createTransport>;

/** `crypto.randomUUID` needs a secure context; a phone on plain http has none. */
function randomClientId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(12));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

/**
 * Transport bound to this page. In production the page and the API share an
 * origin. Under `vite dev` the page comes from :1420 while the backend listens
 * on 4848, so the base is absolute; the backend answers that origin's CORS.
 */
export function createBrowserTransport(onUnauthorized: () => void): Transport {
  const apiPort = import.meta.env.VITE_REMOTE_API_PORT ?? "4848";
  const baseUrl = import.meta.env.DEV
    ? `${location.protocol}//${location.hostname}:${apiPort}/remote/api`
    : `${location.origin}/remote/api`;
  return createTransport({
    baseUrl,
    storage: localStorage,
    fetchFn: (input, init) => fetch(input, init),
    createSocket: (url) => new WebSocket(url) as unknown as SocketLike,
    clientId: randomClientId(),
    onUnauthorized,
  });
}
