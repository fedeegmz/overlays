import { createI18n } from "vue-i18n";
import en from "./locales/en.json";
import es from "./locales/es.json";

export type LocaleCode = "es" | "en";
export type MessageSchema = typeof es;

export interface LocaleOption {
  code: LocaleCode;
  name: string;
}

export const LOCALES: readonly LocaleOption[] = [
  { code: "es", name: "Español" },
  { code: "en", name: "English" },
] as const;

export const FALLBACK_LOCALE: LocaleCode = "en";

const messages = { es, en };

declare module "vue-i18n" {
  export interface DefineLocaleMessage extends MessageSchema {}
}

type AppI18n = ReturnType<typeof createI18nInstance>;

let instance: AppI18n | null = null;

function isLocaleCode(value: string | null | undefined): value is LocaleCode {
  return value === "es" || value === "en";
}

export function toLocaleCode(
  value: string | null | undefined,
): LocaleCode | null {
  return isLocaleCode(value) ? value : null;
}

export function detectSystemLocale(): LocaleCode {
  const language = navigator.language?.toLowerCase() ?? "";
  if (language === "es" || language.startsWith("es-")) return "es";
  return FALLBACK_LOCALE;
}

function createI18nInstance(locale: LocaleCode) {
  return createI18n<[MessageSchema], LocaleCode, false>({
    legacy: false,
    locale,
    fallbackLocale: FALLBACK_LOCALE,
    messages,
  });
}

export function createAppI18n(locale: LocaleCode = FALLBACK_LOCALE): AppI18n {
  instance = createI18nInstance(locale);
  return instance;
}

export function getActiveLocale(): LocaleCode {
  if (!instance) return FALLBACK_LOCALE;
  return (
    toLocaleCode(instance.global.locale.value as string) ?? FALLBACK_LOCALE
  );
}

export function setActiveLocale(locale: LocaleCode): void {
  if (!instance) return;
  instance.global.locale.value = locale;
  document.documentElement.lang = locale;
}

export function translate(
  key: Parameters<NonNullable<typeof instance>["global"]["t"]>[0],
  params?: Record<string, unknown>,
): string {
  if (!instance) return String(key);
  return instance.global.t(key, params ?? {});
}

export function hasTranslation(key: string): boolean {
  if (!instance) return false;
  return instance.global.te(key);
}
