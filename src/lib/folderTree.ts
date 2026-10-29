import type { TemplateInfo } from "../types";

export interface FolderEntry {
  name: string;
  path: string;
  directCount: number;
}

export interface FolderContents {
  folders: FolderEntry[];
  templates: TemplateInfo[];
}

function parentOf(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? "" : path.slice(0, i);
}

function baseName(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? path : path.slice(i + 1);
}

/** Direct children (folders and overlays) of `current`, each sorted by name. */
export function folderContents(
  templates: TemplateInfo[],
  folders: string[],
  current: string,
): FolderContents {
  const allFolders = new Set<string>();
  const addWithAncestors = (path: string) => {
    let p = path;
    while (p !== "" && !allFolders.has(p)) {
      allFolders.add(p);
      p = parentOf(p);
    }
  };
  for (const f of folders) addWithAncestors(f);
  for (const t of templates) addWithAncestors(t.folder);

  const counts = new Map<string, number>();
  for (const t of templates)
    counts.set(t.folder, (counts.get(t.folder) ?? 0) + 1);

  const entries: FolderEntry[] = [...allFolders]
    .filter((p) => parentOf(p) === current)
    .map((path) => ({
      name: baseName(path),
      path,
      directCount: counts.get(path) ?? 0,
    }))
    .sort((a, b) => a.name.localeCompare(b.name));

  const direct = templates
    .filter((t) => t.folder === current)
    .sort((a, b) => a.name.localeCompare(b.name));

  return { folders: entries, templates: direct };
}

/** Whether `path` is the root or an existing folder. */
export function folderExists(
  templates: TemplateInfo[],
  folders: string[],
  path: string,
): boolean {
  if (path === "") return true;
  const prefix = `${path}/`;
  return (
    folders.some((f) => f === path || f.startsWith(prefix)) ||
    templates.some((t) => t.folder === path || t.folder.startsWith(prefix))
  );
}

export function breadcrumbSegments(
  current: string,
): { name: string; path: string }[] {
  if (current === "") return [];
  const parts = current.split("/");
  return parts.map((name, i) => ({
    name,
    path: parts.slice(0, i + 1).join("/"),
  }));
}
