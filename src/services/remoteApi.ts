import { invoke } from "@tauri-apps/api/core";

import type { PairingInfo, RemoteStatus } from "../types";

export function getRemoteStatus(): Promise<RemoteStatus> {
  return invoke<RemoteStatus>("get_remote_status");
}

export function setRemoteEnabled(enabled: boolean): Promise<RemoteStatus> {
  return invoke<RemoteStatus>("set_remote_enabled", { enabled });
}

export function startPairing(): Promise<PairingInfo> {
  return invoke<PairingInfo>("start_pairing");
}

export function cancelPairing(): Promise<void> {
  return invoke<void>("cancel_pairing");
}

export function revokeRemoteAccess(): Promise<RemoteStatus> {
  return invoke<RemoteStatus>("revoke_remote_access");
}
