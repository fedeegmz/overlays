import { describe, expect, it } from "vitest";
import type { TemplateInfo } from "../types";
import { breadcrumbSegments, folderContents } from "./folderTree";

function tpl(id: string, folder: string, name = id): TemplateInfo {
  return { id, name, path: `${folder}/${id}/index.html`, folder, fields: [] };
}

const templates = [
  tpl("zeta", ""),
  tpl("alpha", ""),
  tpl("a1", "sports"),
  tpl("a2", "sports"),
  tpl("deep", "sports/football"),
  tpl("n1", "news"),
];
const folders = ["sports", "sports/football", "news", "empty", "sports/empty"];

describe("folderContents", () => {
  it("returns root folders and templates sorted by name", () => {
    const c = folderContents(templates, folders, "");
    expect(c.folders.map((f) => f.path)).toEqual(["empty", "news", "sports"]);
    expect(c.templates.map((t) => t.id)).toEqual(["alpha", "zeta"]);
  });

  it("returns only direct children of a nested folder", () => {
    const c = folderContents(templates, folders, "sports");
    expect(c.folders.map((f) => f.path)).toEqual([
      "sports/empty",
      "sports/football",
    ]);
    expect(c.folders.map((f) => f.name)).toEqual(["empty", "football"]);
    expect(c.templates.map((t) => t.id)).toEqual(["a1", "a2"]);
  });

  it("counts only direct overlays", () => {
    const c = folderContents(templates, folders, "");
    const byPath = Object.fromEntries(
      c.folders.map((f) => [f.path, f.directCount]),
    );
    expect(byPath).toEqual({ empty: 0, news: 1, sports: 2 });
  });

  it("lists empty folders", () => {
    const c = folderContents(templates, folders, "empty");
    expect(c.folders).toEqual([]);
    expect(c.templates).toEqual([]);
  });

  it("derives intermediate folders missing from the list", () => {
    const c = folderContents([tpl("x", "a/b")], [], "");
    expect(c.folders.map((f) => f.path)).toEqual(["a"]);
  });
});

describe("breadcrumbSegments", () => {
  it("is empty at the root", () => {
    expect(breadcrumbSegments("")).toEqual([]);
  });

  it("builds cumulative paths", () => {
    expect(breadcrumbSegments("a/b/c")).toEqual([
      { name: "a", path: "a" },
      { name: "b", path: "a/b" },
      { name: "c", path: "a/b/c" },
    ]);
  });
});
