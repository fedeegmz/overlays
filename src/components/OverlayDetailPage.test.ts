// @vitest-environment jsdom
import { mount } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { reactive, ref } from "vue";
import { createAppI18n } from "../i18n";
import en from "../i18n/locales/en.json";
import es from "../i18n/locales/es.json";

const URL_VALUE = "http://localhost:4848/overlay/demo?instance=abc123";

const mocks = vi.hoisted(() => ({
  copyText: vi.fn<(text: string) => Promise<void>>(),
}));

vi.mock("./PreviewPanel.vue", () => ({ default: { template: "<div />" } }));
vi.mock("./ContentPanel.vue", () => ({ default: { template: "<div />" } }));
vi.mock("../services/clipboardApi", () => ({ copyText: mocks.copyText }));
vi.mock("../stores/instances", () => ({
  useInstanceStore: () =>
    reactive({
      activeInstance: ref({ id: "abc123456789" }),
      activeTemplate: ref({ name: "Lower third" }),
      overlayUrl: ref(URL_VALUE),
    }),
}));

import OverlayDetailPage from "./OverlayDetailPage.vue";

function mountPage() {
  return mount(OverlayDetailPage, {
    global: { plugins: [createAppI18n("en")] },
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  mocks.copyText.mockReset();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("OverlayDetailPage copy-only URL", () => {
  it("does not render the URL as text and exposes a copy button", () => {
    const wrapper = mountPage();

    expect(wrapper.html()).not.toContain(URL_VALUE);
    const button = wrapper.find("button.copy-btn");
    expect(button.exists()).toBe(true);
    expect(button.attributes("aria-label")).toBe(en.overlayDetail.copyUrl);
    expect(button.attributes("title")).toBe(en.overlayDetail.copyUrl);
  });

  it("copies the full URL, shows the copied state and resets after 2s", async () => {
    mocks.copyText.mockResolvedValue(undefined);
    const wrapper = mountPage();
    const button = wrapper.find("button.copy-btn");

    await button.trigger("click");
    await vi.advanceTimersByTimeAsync(0);

    expect(mocks.copyText).toHaveBeenCalledWith(URL_VALUE);
    expect(button.attributes("aria-label")).toBe(en.overlayDetail.copied);
    expect(button.classes()).toContain("copy-success");

    await vi.advanceTimersByTimeAsync(2000);

    expect(button.attributes("aria-label")).toBe(en.overlayDetail.copyUrl);
    expect(button.classes()).not.toContain("copy-success");
  });

  it("shows the failure message and no copied state when copying rejects", async () => {
    mocks.copyText.mockRejectedValue(new Error("denied"));
    const wrapper = mountPage();
    const button = wrapper.find("button.copy-btn");

    await button.trigger("click");
    await vi.advanceTimersByTimeAsync(0);

    expect(wrapper.text()).toContain(en.overlayDetail.copyFailed);
    expect(button.attributes("aria-label")).toBe(en.overlayDetail.copyUrl);
    expect(button.classes()).not.toContain("copy-success");
  });
});

describe("overlayDetail locale keys", () => {
  it("keeps the same keys in en and es as before the change", () => {
    const expected = ["copied", "copyFailed", "copyUrl"];

    expect(Object.keys(en.overlayDetail).sort()).toEqual(expected);
    expect(Object.keys(es.overlayDetail).sort()).toEqual(expected);
  });
});
