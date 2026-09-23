import { invoke } from "@tauri-apps/api/core";

export function resolveOverlayAssetPath(
  templateId: string,
  absolutePath: string,
): Promise<string> {
  return invoke<string>("resolve_overlay_asset_path", {
    templateId,
    absolutePath,
  });
}
