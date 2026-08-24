import { hasTranslation, translate } from "../i18n";
import { isCommandError } from "../types";

type TranslateKey = Parameters<typeof translate>[0];

export function commandErrorMessage(err: unknown): string {
  if (!isCommandError(err)) return translate("errors.fallback");
  const key = `errors.${err.code}`;
  if (hasTranslation(key)) {
    return translate(
      key as TranslateKey,
      err.params as Record<string, unknown> | undefined,
    );
  }
  return translate("errors.fallback");
}
