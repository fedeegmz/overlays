import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../services/generatorApi", () => ({
  generateOverlay: vi.fn(),
  acceptOverlay: vi.fn(),
  discardOverlay: vi.fn(),
  listProviderModels: vi.fn(),
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

vi.mock("../services/templatesApi", () => ({
  listTemplates: vi.fn(),
}));

vi.mock("../services/dialogApi", () => ({
  pickOverlaysDir: vi.fn(),
}));

import { createAppI18n } from "../i18n";
import { getConfig } from "../services/configApi";
import {
  acceptOverlay,
  discardOverlay,
  generateOverlay,
  listProviderModels,
} from "../services/generatorApi";
import { listConfiguredProviders } from "../services/providerApi";
import { listTemplates } from "../services/templatesApi";
import type { AppConfig, GeneratedOverlaySummary } from "../types";
import GeneratePage from "./GeneratePage.vue";

const mockedGenerate = vi.mocked(generateOverlay);
const mockedAccept = vi.mocked(acceptOverlay);
const mockedDiscard = vi.mocked(discardOverlay);
const mockedModels = vi.mocked(listProviderModels);
const mockedList = vi.mocked(listConfiguredProviders);
const mockedGetConfig = vi.mocked(getConfig);
const mockedListTemplates = vi.mocked(listTemplates);

const config: AppConfig = {
  overlays_dir: "/home/user/overlays",
  language: null,
  provider_keys: [],
  ai_generator_enabled: true,
};

const summary: GeneratedOverlaySummary = {
  staging_id: "stg-1234",
  directory: "zocalo-final",
  name: "Zócalo final",
  fields: [
    { key: "titulo", label: "Título", type: "text" },
    { key: "subtitulo", label: "Subtítulo", type: "text" },
  ],
  files: {
    overlay_json: '{"name": "zocalo-final"}',
    index_html: "<html><body></body></html>",
    style_css: "body { background: transparent; }",
    script_js: 'TEMPLATE_ID = "zocalo-final";',
  },
};

const PROVIDER = [{ provider: "anthropic", configured: true, last4: "abcd" }];
const MODELS = ["claude-sonnet-4-5"];

function mountPage(): ReturnType<typeof mount> {
  const pinia = createPinia();
  const i18n = createAppI18n("en");
  return mount(GeneratePage, {
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

describe("GeneratePage", () => {
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
    mockedGetConfig.mockResolvedValue(config);
    mockedModels.mockResolvedValue(MODELS);
    mockedListTemplates.mockResolvedValue({ templates: [] });
  });

  it("shows the no-key gate modal and Open Settings navigates there", async () => {
    const wrapper = mountPage();
    await flushPromises();

    expect(bodyContains("No API key configured")).toBe(true);
    getFromBody("gate-open-settings").click();
    await flushPromises();
    expect(wrapper.emitted("navigate")).toEqual([["settings"]]);
  });

  it("shows a distinct gate modal when the keyring is unavailable", async () => {
    mockedList.mockResolvedValue({
      providers: PROVIDER,
      keyring_available: false,
    });

    const wrapper = mountPage();
    await flushPromises();

    expect(bodyContains("System keyring unavailable")).toBe(true);
    void wrapper;
  });

  it("shows a distinct gate modal when the overlays folder is not configured", async () => {
    mockedGetConfig.mockResolvedValue({ ...config, overlays_dir: null });

    const wrapper = mountPage();
    await flushPromises();

    expect(bodyContains("Overlays folder not configured")).toBe(true);
    void wrapper;
  });

  it("generates with provider, model and requested name, then previews", async () => {
    mockedList.mockResolvedValue({
      providers: PROVIDER,
      keyring_available: true,
    });
    mockedGenerate.mockResolvedValue(summary);

    const wrapper = mountPage();
    await flushPromises();

    await wrapper
      .get('[data-testid="generate-prompt"]')
      .setValue("  zócalo inferior con acento  ");
    await wrapper.get('[data-testid="generate-name"]').setValue("zocalo-final");
    await wrapper.get('[data-testid="generate-button"]').trigger("click");
    await flushPromises();

    expect(mockedGenerate).toHaveBeenCalledWith(
      "anthropic",
      "claude-sonnet-4-5",
      "zocalo-final",
      "zócalo inferior con acento",
    );
    expect(wrapper.get('[data-testid="generated-preview"]').text()).toContain(
      "zocalo-final",
    );
    expect(wrapper.get('[data-testid="file-view"]').text()).toContain(
      '{"name": "zocalo-final"}',
    );
  });

  it("shows the script file tab when selected", async () => {
    mockedList.mockResolvedValue({
      providers: PROVIDER,
      keyring_available: true,
    });
    mockedGenerate.mockResolvedValue(summary);

    const wrapper = mountPage();
    await flushPromises();

    await wrapper.get('[data-testid="generate-name"]').setValue("zocalo-final");
    await wrapper.get('[data-testid="generate-prompt"]').setValue("mi overlay");
    await wrapper.get('[data-testid="generate-button"]').trigger("click");
    await flushPromises();

    await wrapper.get('[data-testid="file-tab-script.js"]').trigger("click");
    expect(wrapper.get('[data-testid="file-view"]').text()).toContain(
      'TEMPLATE_ID = "zocalo-final"',
    );
  });

  it("accepts the staged overlay with the edited name", async () => {
    mockedList.mockResolvedValue({
      providers: PROVIDER,
      keyring_available: true,
    });
    mockedGenerate.mockResolvedValue(summary);
    mockedAccept.mockResolvedValue(undefined);

    const wrapper = mountPage();
    await flushPromises();

    await wrapper.get('[data-testid="generate-name"]').setValue("zocalo-final");
    await wrapper.get('[data-testid="generate-prompt"]').setValue("mi overlay");
    await wrapper.get('[data-testid="generate-button"]').trigger("click");
    await flushPromises();

    await wrapper
      .get('[data-testid="generate-name-input"]')
      .setValue("zocalo-mejorado");
    await wrapper.get('[data-testid="accept-button"]').trigger("click");
    await flushPromises();

    expect(mockedAccept).toHaveBeenCalledWith("stg-1234", "zocalo-mejorado");
    expect(wrapper.find('[data-testid="generated-preview"]').exists()).toBe(
      false,
    );
    expect(wrapper.get('[data-testid="generate-accepted"]').text()).toContain(
      "Template saved",
    );
  });

  it("blocks accept while the edited name breaks the kebab rule", async () => {
    mockedList.mockResolvedValue({
      providers: PROVIDER,
      keyring_available: true,
    });
    mockedGenerate.mockResolvedValue(summary);

    const wrapper = mountPage();
    await flushPromises();

    await wrapper.get('[data-testid="generate-name"]').setValue("zocalo-final");
    await wrapper.get('[data-testid="generate-prompt"]').setValue("mi overlay");
    await wrapper.get('[data-testid="generate-button"]').trigger("click");
    await flushPromises();

    await wrapper
      .get('[data-testid="generate-name-input"]')
      .setValue("Zócalo Final");
    const acceptButton = wrapper.get('[data-testid="accept-button"]');
    expect((acceptButton.element as HTMLButtonElement).disabled).toBe(true);

    await wrapper
      .get('[data-testid="generate-name-input"]')
      .setValue("zocalo-final-v2");
    expect((acceptButton.element as HTMLButtonElement).disabled).toBe(false);
  });

  it("discards the staged overlay", async () => {
    mockedList.mockResolvedValue({
      providers: PROVIDER,
      keyring_available: true,
    });
    mockedGenerate.mockResolvedValue(summary);
    mockedDiscard.mockResolvedValue(undefined);

    const wrapper = mountPage();
    await flushPromises();

    await wrapper.get('[data-testid="generate-name"]').setValue("zocalo-final");
    await wrapper.get('[data-testid="generate-prompt"]').setValue("mi overlay");
    await wrapper.get('[data-testid="generate-button"]').trigger("click");
    await flushPromises();

    const discardButton = wrapper
      .findAll("button")
      .find((b) => b.text() === "Discard");
    expect(discardButton).toBeDefined();
    await discardButton?.trigger("click");
    await flushPromises();

    expect(mockedDiscard).toHaveBeenCalledWith("stg-1234");
    expect(wrapper.find('[data-testid="generated-preview"]').exists()).toBe(
      false,
    );
  });

  it("renders each validation issue for an invalid output", async () => {
    mockedList.mockResolvedValue({
      providers: PROVIDER,
      keyring_available: true,
    });
    mockedGenerate.mockRejectedValue({
      code: "generation.invalid_output",
      params: {
        issues: JSON.stringify([
          { code: "manifest_missing_name", params: {} },
          { code: "script_no_reconnect", params: { needle: "setInterval" } },
        ]),
      },
    });

    const wrapper = mountPage();
    await flushPromises();

    await wrapper.get('[data-testid="generate-name"]').setValue("zocalo-final");
    await wrapper.get('[data-testid="generate-prompt"]').setValue("mi overlay");
    await wrapper.get('[data-testid="generate-button"]').trigger("click");
    await flushPromises();

    const error = wrapper.get('[data-testid="generate-error"]').text();
    expect(error).toContain("must have a non-empty name");
    expect(error).toContain("reconnection logic");
  });

  it("surfaces a mapped provider error", async () => {
    mockedList.mockResolvedValue({
      providers: PROVIDER,
      keyring_available: true,
    });
    mockedGenerate.mockRejectedValue({
      code: "provider.unauthorized",
      params: {},
    });

    const wrapper = mountPage();
    await flushPromises();

    await wrapper.get('[data-testid="generate-name"]').setValue("zocalo-final");
    await wrapper.get('[data-testid="generate-prompt"]').setValue("mi overlay");
    await wrapper.get('[data-testid="generate-button"]').trigger("click");
    await flushPromises();

    expect(wrapper.get('[data-testid="generate-error"]').text()).toContain(
      "The AI provider rejected the API key.",
    );
    expect(wrapper.find('[data-testid="generated-preview"]').exists()).toBe(
      false,
    );
  });
});
