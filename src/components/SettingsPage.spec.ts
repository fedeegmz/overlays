import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../services/providerApi", () => ({
  listConfiguredProviders: vi.fn(),
  addProviderKey: vi.fn(),
  deleteProviderKey: vi.fn(),
}));

vi.mock("../services/dialogApi", () => ({
  pickOverlaysDir: vi.fn(),
}));

import { createAppI18n } from "../i18n";
import {
  deleteProviderKey,
  listConfiguredProviders,
} from "../services/providerApi";
import SettingsPage from "./SettingsPage.vue";

const mockedList = vi.mocked(listConfiguredProviders);
const mockedDelete = vi.mocked(deleteProviderKey);

function mountPage(): ReturnType<typeof mount> {
  const pinia = createPinia();
  const i18n = createAppI18n("en");
  return mount(SettingsPage, {
    global: { plugins: [pinia, i18n] },
  });
}

describe("SettingsPage API keys", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    mockedList.mockResolvedValue({
      providers: [],
      keyring_available: true,
    });
  });

  it("shows the masked key and never the full secret", async () => {
    mockedList.mockResolvedValue({
      providers: [{ provider: "anthropic", configured: true, last4: "abcd" }],
      keyring_available: true,
    });

    const wrapper = mountPage();
    await flushPromises();

    expect(wrapper.text()).toContain("sk-…abcd");
    expect(wrapper.text()).not.toContain("sk-ant-1234abcd");
  });

  it("deletes the provider key when the user confirms", async () => {
    mockedList.mockResolvedValue({
      providers: [{ provider: "anthropic", configured: true, last4: "abcd" }],
      keyring_available: true,
    });
    mockedDelete.mockResolvedValue([]);

    const wrapper = mountPage();
    await flushPromises();

    const deleteButton = wrapper
      .findAll("button")
      .find((b) => b.text() === "Delete");
    expect(deleteButton).toBeDefined();
    await deleteButton?.trigger("click");
    await flushPromises();

    expect(mockedDelete).toHaveBeenCalledWith("anthropic");
    expect(wrapper.text()).toContain("No keys configured");
  });

  it("surfaces a distinct error when the keyring is unavailable", async () => {
    mockedList.mockResolvedValue({
      providers: [{ provider: "anthropic", configured: true, last4: "abcd" }],
      keyring_available: false,
    });

    const wrapper = mountPage();
    await flushPromises();

    expect(wrapper.text()).toContain("system keyring is not available");
  });
});
