import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../services/remoteApi", () => ({
  getRemoteStatus: vi.fn(),
  setRemoteEnabled: vi.fn(),
  startPairing: vi.fn(),
  cancelPairing: vi.fn(),
  revokeRemoteAccess: vi.fn(),
}));

import * as api from "../services/remoteApi";
import { useRemoteStore } from "./remote";

const URL_A = "http://192.168.1.20:4848/remote/";
const URL_B = "http://10.0.0.7:4848/remote/";

function status(
  over: Partial<{ enabled: boolean; pairedDevices: number }> = {},
) {
  return {
    enabled: true,
    pairedDevices: 0,
    lanUrls: [URL_A, URL_B],
    ...over,
  };
}

const pairing = { pin: "042137", expiresAtMs: 5_000, urls: [URL_A] };

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
});

describe("remote store", () => {
  it("enabling_turns_on_then_starts_pairing", async () => {
    vi.mocked(api.setRemoteEnabled).mockResolvedValue(status());
    vi.mocked(api.startPairing).mockResolvedValue(pairing);
    const store = useRemoteStore();

    await store.setEnabled(true);

    expect(api.setRemoteEnabled).toHaveBeenCalledWith(true);
    expect(store.enabled).toBe(true);
    expect(store.pairing).toEqual(pairing);
  });

  it("disabling_clears_pairing_and_does_not_start_one", async () => {
    vi.mocked(api.setRemoteEnabled).mockResolvedValueOnce(status());
    vi.mocked(api.startPairing).mockResolvedValue(pairing);
    const store = useRemoteStore();
    await store.setEnabled(true);

    vi.mocked(api.setRemoteEnabled).mockResolvedValueOnce(
      status({ enabled: false }),
    );
    await store.setEnabled(false);

    expect(store.enabled).toBe(false);
    expect(store.pairing).toBeNull();
    expect(api.startPairing).toHaveBeenCalledTimes(1);
  });

  it("qr_payload_uses_selected_url_with_pin_fragment", async () => {
    vi.mocked(api.setRemoteEnabled).mockResolvedValue(status());
    vi.mocked(api.startPairing).mockResolvedValue(pairing);
    const store = useRemoteStore();
    await store.setEnabled(true);

    expect(store.qrPayload).toBe(`${URL_A}#pin=042137`);
    store.selectUrl(URL_B);
    expect(store.qrPayload).toBe(`${URL_B}#pin=042137`);
  });

  it("qr_payload_is_null_without_pairing", () => {
    expect(useRemoteStore().qrPayload).toBeNull();
  });

  it("refresh_drops_pin_when_a_device_paired", async () => {
    vi.mocked(api.setRemoteEnabled).mockResolvedValue(status());
    vi.mocked(api.startPairing).mockResolvedValue(pairing);
    const store = useRemoteStore();
    await store.setEnabled(true);

    vi.mocked(api.getRemoteStatus).mockResolvedValue(
      status({ pairedDevices: 1 }),
    );
    await store.refresh();

    expect(store.pairedDevices).toBe(1);
    expect(store.pairing).toBeNull();
  });

  it("revoke_clears_devices_and_pairing", async () => {
    vi.mocked(api.revokeRemoteAccess).mockResolvedValue(
      status({ pairedDevices: 0 }),
    );
    const store = useRemoteStore();
    store.pairedDevices = 2;

    await store.revoke();

    expect(api.revokeRemoteAccess).toHaveBeenCalledTimes(1);
    expect(store.pairedDevices).toBe(0);
    expect(store.pairing).toBeNull();
  });

  it("cancel_pairing_calls_backend_and_clears_pin", async () => {
    vi.mocked(api.setRemoteEnabled).mockResolvedValue(status());
    vi.mocked(api.startPairing).mockResolvedValue(pairing);
    vi.mocked(api.cancelPairing).mockResolvedValue();
    const store = useRemoteStore();
    await store.setEnabled(true);

    await store.cancelPairing();

    expect(api.cancelPairing).toHaveBeenCalledTimes(1);
    expect(store.pairing).toBeNull();
  });

  it("command_error_is_localized_into_error", async () => {
    vi.mocked(api.startPairing).mockRejectedValue({ code: "remote.disabled" });
    const store = useRemoteStore();

    await store.startPairing();

    expect(store.error).toBeTruthy();
    expect(store.pairing).toBeNull();
  });
});
