import { hasTranslation, translate } from "../i18n";
import type { ValidationIssue } from "../types";
import { isCommandError } from "../types";

type TranslateKey = Parameters<typeof translate>[0];

function isIssue(value: unknown): value is ValidationIssue {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof (value as { code: unknown }).code === "string"
  );
}

/**
 * `generation.invalid_output` carries the deterministic issue list as a
 * JSON string in `params.issues` — render each issue through its own i18n
 * key (`errors.generation.issue.<code>`) with its own params.
 */
function parseIssues(
  params: Record<string, string> | undefined,
): ValidationIssue[] {
  const raw = params?.issues;
  if (typeof raw !== "string") return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter(isIssue) : [];
  } catch {
    return [];
  }
}

export function commandErrorMessage(err: unknown): string {
  if (!isCommandError(err)) return translate("errors.fallback");
  if (err.code === "generation.invalid_output") {
    const issues = parseIssues(err.params);
    if (issues.length > 0) {
      const lines = issues.map((issue) => {
        const key = `errors.generation.issue.${issue.code}`;
        return hasTranslation(key)
          ? translate(key as TranslateKey, issue.params)
          : translate("errors.fallback");
      });
      return [translate("errors.generation.invalid_output"), ...lines].join(
        "\n",
      );
    }
  }
  const key = `errors.${err.code}`;
  if (hasTranslation(key)) {
    return translate(
      key as TranslateKey,
      err.params as Record<string, unknown> | undefined,
    );
  }
  return translate("errors.fallback");
}
