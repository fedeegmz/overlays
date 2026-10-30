export interface RemoteStatus {
  enabled: boolean;
  pairedDevices: number;
  /** Remote page URLs reachable from the LAN (`http://<ip>:<port>/remote/`). */
  lanUrls: string[];
}

export interface PairingInfo {
  pin: string;
  /** Unix epoch milliseconds at which the PIN stops being valid. */
  expiresAtMs: number;
  urls: string[];
}
