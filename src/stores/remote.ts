import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { commandErrorMessage } from "../lib/errors";
import {
  cancelPairing as cancelPairingCommand,
  getRemoteStatus,
  revokeRemoteAccess,
  setRemoteEnabled as setRemoteEnabledCommand,
  startPairing as startPairingCommand,
} from "../services/remoteApi";
import type { PairingInfo, RemoteStatus } from "../types";

export const useRemoteStore = defineStore("remote", () => {
  const enabled = ref(false);
  const pairedDevices = ref(0);
  const lanUrls = ref<string[]>([]);
  const pairing = ref<PairingInfo | null>(null);
  const selectedUrl = ref<string | null>(null);
  const error = ref<string | null>(null);

  /** URL the QR points at: the chosen LAN URL with the PIN in the fragment. */
  const qrPayload = computed(() => {
    const url = selectedUrl.value ?? lanUrls.value[0];
    if (!url || !pairing.value) return null;
    return `${url}#pin=${pairing.value.pin}`;
  });

  function applyStatus(status: RemoteStatus): void {
    // A single-use PIN is burned by a successful pairing or by turning remote off.
    if (
      pairing.value &&
      (!status.enabled || status.pairedDevices > pairedDevices.value)
    ) {
      pairing.value = null;
    }
    enabled.value = status.enabled;
    pairedDevices.value = status.pairedDevices;
    lanUrls.value = status.lanUrls;
    if (selectedUrl.value && !status.lanUrls.includes(selectedUrl.value)) {
      selectedUrl.value = null;
    }
  }

  async function run(action: () => Promise<void>): Promise<void> {
    error.value = null;
    try {
      await action();
    } catch (err) {
      error.value = commandErrorMessage(err);
    }
  }

  async function refresh(): Promise<void> {
    await run(async () => {
      applyStatus(await getRemoteStatus());
    });
  }

  async function startPairing(): Promise<void> {
    await run(async () => {
      pairing.value = await startPairingCommand();
    });
  }

  async function cancelPairing(): Promise<void> {
    await run(async () => {
      await cancelPairingCommand();
      pairing.value = null;
    });
  }

  async function setEnabled(value: boolean): Promise<void> {
    await run(async () => {
      applyStatus(await setRemoteEnabledCommand(value));
      if (value) pairing.value = await startPairingCommand();
    });
  }

  async function revoke(): Promise<void> {
    await run(async () => {
      pairing.value = null;
      applyStatus(await revokeRemoteAccess());
    });
  }

  function selectUrl(url: string): void {
    selectedUrl.value = url;
  }

  return {
    enabled,
    pairedDevices,
    lanUrls,
    pairing,
    selectedUrl,
    error,
    qrPayload,
    refresh,
    startPairing,
    cancelPairing,
    setEnabled,
    revoke,
    selectUrl,
  };
});
