import { describe, expect, it } from "vitest";
import en from "./locales/en.json";
import es from "./locales/es.json";

type Json = Record<string, unknown>;

function flattenKeys(obj: Json, prefix = ""): string[] {
  const keys: string[] = [];
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === "object" && !Array.isArray(value)) {
      keys.push(...flattenKeys(value as Json, path));
    } else {
      keys.push(path);
    }
  }
  return keys.sort();
}

describe("i18n locales", () => {
  it("es and en expose the same key structure", () => {
    expect(flattenKeys(es)).toEqual(flattenKeys(en));
  });

  it("expose an errors section with fallback", () => {
    expect(es).toHaveProperty("errors.fallback");
    expect(en).toHaveProperty("errors.fallback");
  });
});
