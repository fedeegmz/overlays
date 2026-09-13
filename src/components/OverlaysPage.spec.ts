import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../services/templatesApi", () => ({
  listTemplates: vi.fn(),
}));

vi.mock("../services/providerApi", () => ({
  listConfiguredProviders: vi.fn(),
  addProviderKey: vi.fn(),
  deleteProviderKey: vi.fn(),
}));

vi.mock("../services/configApi", () => ({
  getConfig: vi.fn(),
  setLanguage: vi.fn(),
  setOverlaysDir: vi.fn(),
}));

vi.mock("../services/dialogApi", () => ({
  pickOverlaysDir: vi.fn(),
}));

import { createAppI18n } from "../i18n";
import { getConfig } from "../services/configApi";
import { listConfiguredProviders } from "../services/providerApi";
import { listTemplates } from "../services/templatesApi";
import type { AppConfig } from "../types";
import OverlaysPage from "./OverlaysPage.vue";

const mockedList = vi.mocked(listConfiguredProviders);
const mockedGetConfig = vi.mocked(getConfig);
const mockedListTemplates = vi.mocked(listTemplates);

const config: AppConfig = {
  overlays_dir: "/home/user/overlays",
  language: null,
  provider_keys: [],
  ai_generator_enabled: true,
};

function mountPage(): ReturnType<typeof mount> {
  const pinia = createPinia();
  const i18n = createAppI18n("en");
  return mount(OverlaysPage, {
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

describe("OverlaysPage generator entry (G1)", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    mockedGetConfig.mockResolvedValue(config);
    mockedList.mockResolvedValue({
      providers: [],
      keyring_available: true,
    });
    mockedListTemplates.mockResolvedValue({ templates: [] });
  });

  it("navigates to the generate page when not gated", async () => {
    mockedList.mockResolvedValue({
      providers: [{ provider: "anthropic", configured: true, last4: "abcd" }],
      keyring_available: true,
    });

    const wrapper = mountPage();
    await flushPromises();

    await wrapper.get('[data-testid="generate-entry"]').trigger("click");
    expect(wrapper.emitted("navigate")).toEqual([["generate"]]);
  });

  it("opens the no-key gate modal instead of navigating when gated", async () => {
    const wrapper = mountPage();
    await flushPromises();

    expect(bodyContains("No API key configured")).toBe(true);
    expect(wrapper.emitted("navigate")).toBeUndefined();
  });

  it("Open Settings in the gate navigates to settings", async () => {
    const wrapper = mountPage();
    await flushPromises();

    getFromBody("gate-open-settings").click();
    await flushPromises();
    expect(wrapper.emitted("navigate")).toEqual([["settings"]]);
  });

  it("hides the generate entry when the feature flag is off", async () => {
    mockedGetConfig.mockResolvedValue({
      ...config,
      ai_generator_enabled: false,
    });

    const wrapper = mountPage();
    await flushPromises();

    expect(wrapper.find('[data-testid="generate-entry"]').exists()).toBe(false);
  });
});
