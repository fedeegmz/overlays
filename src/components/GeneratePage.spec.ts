import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../services/generatorApi", () => ({
  generateOverlay: vi.fn(),
  acceptOverlay: vi.fn(),
  discardOverlay: vi.fn(),
}));

vi.mock("../services/providerApi", () => ({
  listConfiguredProviders: vi.fn(),
  addProviderKey: vi.fn(),
  deleteProviderKey: vi.fn(),
}));

vi.mock("../services/templatesApi", () => ({
  listTemplates: vi.fn(),
}));

vi.mock("../services/dialogApi", () => ({
  pickOverlaysDir: vi.fn(),
}));

import { createAppI18n } from "../i18n";
import {
  acceptOverlay,
  discardOverlay,
  generateOverlay,
} from "../services/generatorApi";
import { listConfiguredProviders } from "../services/providerApi";
import { listTemplates } from "../services/templatesApi";
import type { GeneratedOverlaySummary } from "../types";
import GeneratePage from "./GeneratePage.vue";

const mockedGenerate = vi.mocked(generateOverlay);
const mockedAccept = vi.mocked(acceptOverlay);
const mockedDiscard = vi.mocked(discardOverlay);
const mockedList = vi.mocked(listConfiguredProviders);
const mockedListTemplates = vi.mocked(listTemplates);

const summary: GeneratedOverlaySummary = {
  staging_id: "stg-1234",
  directory: "zocalo-final",
  name: "Zócalo final",
  fields: [
    { key: "titulo", label: "Título", type: "text" },
    { key: "subtitulo", label: "Subtítulo", type: "text" },
  ],
};

function mountPage(): ReturnType<typeof mount> {
  const pinia = createPinia();
  const i18n = createAppI18n("en");
  return mount(GeneratePage, {
    global: { plugins: [pinia, i18n] },
  });
}

describe("GeneratePage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    mockedList.mockResolvedValue({
      providers: [],
      keyring_available: true,
    });
    mockedListTemplates.mockResolvedValue({ templates: [] });
  });

  it("shows a gate when no provider key is configured", async () => {
    const wrapper = mountPage();
    await flushPromises();

    expect(wrapper.get('[data-testid="generate-gate"]').text()).toContain(
      "Add an Anthropic API key in Settings",
    );
  });

  it("generates a staged summary and previews it", async () => {
    mockedList.mockResolvedValue({
      providers: [{ provider: "anthropic", configured: true, last4: "abcd" }],
      keyring_available: true,
    });
    mockedGenerate.mockResolvedValue(summary);

    const wrapper = mountPage();
    await flushPromises();

    await wrapper
      .get('[data-testid="generate-prompt"]')
      .setValue("  zócalo inferior con acento  ");
    await wrapper.get('[data-testid="generate-button"]').trigger("click");
    await flushPromises();

    expect(mockedGenerate).toHaveBeenCalledWith("zócalo inferior con acento");
    expect(wrapper.get('[data-testid="generated-preview"]').text()).toContain(
      "zocalo-final",
    );
    expect(wrapper.get('[data-testid="generated-preview"]').text()).toContain(
      "subtitulo",
    );
  });

  it("accepts the staged overlay and confirms the new template", async () => {
    mockedList.mockResolvedValue({
      providers: [{ provider: "anthropic", configured: true, last4: "abcd" }],
      keyring_available: true,
    });
    mockedGenerate.mockResolvedValue(summary);
    mockedAccept.mockResolvedValue(undefined);

    const wrapper = mountPage();
    await flushPromises();

    await wrapper.get('[data-testid="generate-prompt"]').setValue("mi overlay");
    await wrapper.get('[data-testid="generate-button"]').trigger("click");
    await flushPromises();

    await wrapper.get('[data-testid="accept-button"]').trigger("click");
    await flushPromises();

    expect(mockedAccept).toHaveBeenCalledWith("stg-1234");
    expect(wrapper.find('[data-testid="generated-preview"]').exists()).toBe(
      false,
    );
    expect(wrapper.get('[data-testid="generate-accepted"]').text()).toContain(
      "Template saved",
    );
  });

  it("discards the staged overlay", async () => {
    mockedList.mockResolvedValue({
      providers: [{ provider: "anthropic", configured: true, last4: "abcd" }],
      keyring_available: true,
    });
    mockedGenerate.mockResolvedValue(summary);
    mockedDiscard.mockResolvedValue(undefined);

    const wrapper = mountPage();
    await flushPromises();

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

  it("surfaces a mapped provider error", async () => {
    mockedList.mockResolvedValue({
      providers: [{ provider: "anthropic", configured: true, last4: "abcd" }],
      keyring_available: true,
    });
    mockedGenerate.mockRejectedValue({
      code: "provider.unauthorized",
      params: {},
    });

    const wrapper = mountPage();
    await flushPromises();

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
