import { invoke } from "@tauri-apps/api/core";

export type OverlayAction = "show" | "update" | "hide";

export interface ServerStatus {
  running: boolean;
  port: number;
}

export interface SendOverlayUpdateArgs {
  instanceId: string;
  template: string;
  action: OverlayAction;
  fields: Record<string, string>;
}

export function sendOverlayUpdate(
  args: SendOverlayUpdateArgs,
): Promise<void> {
  return invoke("send_overlay_update", {
    instanceId: args.instanceId,
    template: args.template,
    action: args.action,
    fields: args.fields,
  });
}

export async function getServerStatus(): Promise<ServerStatus | null> {
  try {
    return await invoke<ServerStatus>("get_server_status");
  } catch {
    // server may not respond yet in dev until Rust starts
    return null;
  }
}
