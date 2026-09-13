import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../services/configApi", () => ({
  getConfig: vi.fn(),
  setLanguage: vi.fn(),
  setOverlaysDir: vi.fn(),
}));

import { createAppI18n } from "../i18n";
import { useConfigStore } from "../stores/config";
import SideNav from "./SideNav.vue";

function mountNav(enabled: boolean): ReturnType<typeof mount> {
  const pinia = createPinia();
  setActivePinia(pinia);
  const config = useConfigStore();
  config.appConfig.ai_generator_enabled = enabled;
  const i18n = createAppI18n("en");
  return mount(SideNav, {
    props: { currentPage: "overlays" },
    global: { plugins: [pinia, i18n] },
  });
}

describe("SideNav generator entry", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows the generate entry when the feature flag is on", () => {
    const wrapper = mountNav(true);
    const entries = wrapper
      .findAll("button")
      .filter((b) => b.text() === "Generate");
    expect(entries.length).toBe(1);
  });

  it("hides the generate entry when the feature flag is off", () => {
    const wrapper = mountNav(false);
    expect(wrapper.text()).not.toContain("Generate");
  });
});
