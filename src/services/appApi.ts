import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function getAppVersion(): Promise<string> {
  return getVersion();
}

export function onCloseRequested(
  shouldClose: () => Promise<boolean>,
): Promise<() => void> {
  return getCurrentWindow().onCloseRequested(async (event) => {
    if (!(await shouldClose())) event.preventDefault();
  });
}
