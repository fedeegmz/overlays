import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../services/instancesApi", () => ({
  clientId: "me",
  listInstances: vi.fn(),
  setInstanceFields: vi.fn(),
  saveGroupFromInstances: vi.fn(),
  startGroupPreset: vi.fn(),
  onInstancesChanged: vi.fn(),
}));
vi.mock("../services/groupPresetsApi", () => ({
  listGroupPresets: vi.fn(),
  deleteGroupPreset: vi.fn(),
}));
vi.mock("../services/overlayApi", () => ({
  getServerStatus: vi.fn(),
  sendOverlayUpdate: vi.fn(),
}));
vi.mock("../services/templatesApi", () => ({ listTemplates: vi.fn() }));
vi.mock("@tauri-apps/api/webviewWindow", () => ({ WebviewWindow: {} }));

import * as api from "../services/instancesApi";
import { useGroupPresetStore } from "./groupPresets";

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
});

describe("saveFromInstances", () => {
  it("save_calls_backend_with_name_only", async () => {
    const saved = [{ name: "Intro", entries: [] }];
    vi.mocked(api.saveGroupFromInstances).mockResolvedValue(saved);
    const store = useGroupPresetStore();

    await store.saveFromInstances("Intro");

    expect(api.saveGroupFromInstances).toHaveBeenCalledTimes(1);
    expect(api.saveGroupFromInstances).toHaveBeenCalledWith("Intro");
    expect(store.groups).toEqual(saved);
  });
});
