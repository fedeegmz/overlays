import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../services/providerApi", () => ({
  listConfiguredProviders: vi.fn(),
  addProviderKey: vi.fn(),
  deleteProviderKey: vi.fn(),
}));

vi.mock("../services/dialogApi", () => ({
  pickOverlaysDir: vi.fn(),
}));

vi.mock("../services/templatesApi", () => ({
  listTemplates: vi.fn(),
}));

import { createAppI18n } from "../i18n";
import {
  deleteProviderKey,
  listConfiguredProviders,
} from "../services/providerApi";
import SettingsPage from "./SettingsPage.vue";

const mockedList = vi.mocked(listConfiguredProviders);
const mockedDelete = vi.mocked(deleteProviderKey);

const PROVIDER = [{ provider: "anthropic", configured: true, last4: "abcd" }];

function mountPage(): ReturnType<typeof mount> {
  const pinia = createPinia();
  const i18n = createAppI18n("en");
  return mount(SettingsPage, {
    global: { plugins: [pinia, i18n] },
  });
}

/** Modals teleport to <body> — the content nodes land there, not in the wrapper. */
function bodyContains(text: string): boolean {
  return (document.body.textContent ?? "").includes(text);
}

function getFromBody(testId: string): HTMLElement {
  const el = document.body.querySelector<HTMLElement>(
    `[data-testid="${testId}"]`,
  );
  if (!el) throw new Error(`Missing [data-testid="${testId}"] in <body>`);
  return el;
}

describe("SettingsPage API keys", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

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
      providers: PROVIDER,
      keyring_available: true,
    });

    const wrapper = mountPage();
    await flushPromises();

    expect(wrapper.text()).toContain("sk-…abcd");
    expect(wrapper.text()).not.toContain("sk-ant-1234abcd");
  });

  it("deletes the provider key only after the user confirms", async () => {
    mockedList.mockResolvedValue({
      providers: PROVIDER,
      keyring_available: true,
    });
    mockedDelete.mockResolvedValue([]);

    const wrapper = mountPage();
    await flushPromises();

    await wrapper.get('[data-testid="delete-key-button"]').trigger("click");
    await flushPromises();

    expect(mockedDelete).not.toHaveBeenCalled();
    expect(bodyContains("Delete API key?")).toBe(true);

    getFromBody("confirm-delete").click();
    await flushPromises();

    expect(mockedDelete).toHaveBeenCalledWith("anthropic");
    expect(wrapper.text()).toContain("No keys configured");
  });

  it("keeps the key when the delete confirmation is cancelled", async () => {
    mockedList.mockResolvedValue({
      providers: PROVIDER,
      keyring_available: true,
    });

    const wrapper = mountPage();
    await flushPromises();

    await wrapper.get('[data-testid="delete-key-button"]').trigger("click");
    await flushPromises();

    const cancelButton = Array.from(
      document.body.querySelectorAll("button"),
    ).find((b) => b.textContent === "Cancel");
    expect(cancelButton).toBeDefined();
    cancelButton?.click();
    await flushPromises();

    expect(mockedDelete).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("sk-…abcd");
  });

  it("surfaces a distinct error when the keyring is unavailable", async () => {
    mockedList.mockResolvedValue({
      providers: PROVIDER,
      keyring_available: false,
    });

    const wrapper = mountPage();
    await flushPromises();

    expect(wrapper.text()).toContain("system keyring is not available");
  });
});
