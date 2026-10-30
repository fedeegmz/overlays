import { createApp } from "vue";
import "../styles/variables.css";
import "../styles/components.css";
import "./remote.css";
import { createAppI18n, detectSystemLocale, setActiveLocale } from "../i18n";
import App from "./App.vue";

const locale = detectSystemLocale();
setActiveLocale(locale);
createApp(App).use(createAppI18n(locale)).mount("#app");
