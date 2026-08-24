import { createApp } from "vue";
import { createPinia } from "pinia";
import "./styles/variables.css";
import "./styles/components.css";
import App from "./App.vue";
import {
  createAppI18n,
  detectSystemLocale,
  setActiveLocale,
  toLocaleCode,
} from "./i18n";
import { getConfig } from "./services/configApi";

async function resolveInitialLocale() {
  const language = await getConfig()
    .then((config) => config.language)
    .catch(() => null);
  const locale = toLocaleCode(language) ?? detectSystemLocale();
  setActiveLocale(locale);
  return locale;
}

async function bootstrap(): Promise<void> {
  const locale = await resolveInitialLocale();
  const i18n = createAppI18n(locale);
  createApp(App).use(i18n).use(createPinia()).mount("#app");
}

void bootstrap();
