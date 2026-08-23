import { createApp } from "vue";
import "./styles/variables.css";
import "./styles/components.css";
import App from "./App.vue";
import {
  createAppI18n,
  detectSystemLocale,
  setActiveLocale,
  toLocaleCode,
} from "./i18n";
import { loadStoredLanguage } from "./composables/useOverlayControl";

async function resolveInitialLocale() {
  const stored = toLocaleCode(await loadStoredLanguage());
  const locale = stored ?? detectSystemLocale();
  setActiveLocale(locale);
  return locale;
}

async function bootstrap(): Promise<void> {
  const locale = await resolveInitialLocale();
  const i18n = createAppI18n(locale);
  createApp(App).use(i18n).mount("#app");
}

void bootstrap();
