# Design: Overlay Folder Explorer

## Technical Approach

The backend walks the overlays dir recursively and still returns a flat `Manifest`. It adds two things: a relative `folder` on each `TemplateInfo`, and a top-level `folders` list so empty folders show. The frontend turns these into the contents of the current folder using a pure function. A new explorer store holds the current folder, and the page renders a breadcrumb, folder cards and overlay cards. Template ids, the WS payload and `/overlay/{*path}` serving stay the same.

## Architecture Decisions

| Decision | Options | Tradeoff | Choice |
|---|---|---|---|
| Wire shape | flat + `folder` + `folders[]` / nested tree | a tree breaks `Manifest` consumers (`/api/templates`, the planned `/remote/templates`) | Flat, additive fields only |
| `folder` format | `String` (`""` = root, `/`-separated) / `Option<String>` | the non-optional form keeps the TS type simple | `String`, always serialized |
| Port return | `discover -> Manifest` / new `Discovery` struct / a second port method | a second method walks twice; a new struct duplicates `Manifest` | `TemplateSource::discover` returns `Manifest` |
| Walk order | `read_dir` order / sorted per level, depth-first pre-order | `read_dir` order is nondeterministic across filesystems | Sort entries by file name (byte order) and recurse in order. The first id found wins |
| Symlinks | follow / skip | following risks loops and escaping the dir | Skip, using `DirEntry::file_type()` (which does not follow links) |
| Asset base dir | id-joined path (current) / IPC sends the path / backend looks up the id | id != dir name and nested dirs break the first option; the second trusts the UI | Look up the id in the backend (`TemplateCatalog::template_dir`) |
| Unknown id on resolve | new error code / reuse `AssetResolveFailed` | a new code needs new i18n strings | Reuse `AssetResolveFailed` |
| Current-folder state | local `ref` in `OverlaysPage` / `stores/templates.ts` / new store | a local ref resets when the user returns from the detail page; `templates.ts` conflicts with `remote-overlay-control` | New `stores/overlayExplorer.ts`, kept in memory only |
| Frontend tests | add vitest / no runner | `package.json` has no test runner today | Keep the logic in a pure function. Adding vitest is an open question |

## Discovery Algorithm (`fs_template_source.rs`)

```
walk(abs_dir, rel, depth):
  entries = read_dir(abs_dir) sorted by file_name   // root error -> TemplateDiscoveryFailed; subdir error -> eprintln + return
  for e in entries:
    ft = e.file_type(); skip if !ft.is_dir() || ft.is_symlink() || name starts with '.'
    child_rel = join(rel, name)                      // '/'-separated
    if child/overlay.json exists:                    // leaf: never recurse
      parse as today (index.html, meta, fields); id = meta.id || name
      if seen_ids contains id: eprintln duplicate warning, skip
      push TemplateInfo{ id, name, path: "{child_rel}/index.html", folder: rel, fields }
    else:
      folders.push(child_rel)                        // listed even when empty
      if depth + 1 < MAX_DEPTH (5): walk(child, child_rel, depth + 1)
```

`MAX_DEPTH = 5` is the deepest folder level that gets listed. Its children are not walked. Templates stay sorted by `name` (current behavior), and `folders` is sorted.

## Data Flow

```
FS ─→ FsTemplateSource::discover ─→ Manifest{templates[folder], folders}
        ├─→ list_templates (IPC) ─→ templatesApi ─→ templates store
        │                                         └─→ overlayExplorer store (currentFolder)
        │                                               └─→ folderContents() ─→ OverlaysPage
        └─→ /api/templates (HTTP, unchanged route)
resolve_overlay_asset_path(id, abs) ─→ catalog.template_dir(id) ─→ resolve_asset_path(dir, rel_dir, abs)
```

## File Changes

| File | Action | Description |
|---|---|---|
| `src-tauri/src/domain/template.rs` | Modify | Add `TemplateInfo.folder: String` and `Manifest.folders: Vec<String>` |
| `src-tauri/src/application/ports.rs` | Modify | `discover` returns `DomainResult<Manifest>` |
| `src-tauri/src/application/template_catalog.rs` | Modify | Pass the manifest through; add `template_dir(id) -> DomainResult<Option<String>>`, which strips `/index.html` from `path`; update `FakeSource` |
| `src-tauri/src/infrastructure/fs_template_source.rs` | Modify | Recursive walk; `resolve_asset_path(overlays_dir, template_rel_dir, abs)` |
| `src-tauri/src/infrastructure/tauri/commands.rs` | Modify | Look up the id through `http.catalog`; an unknown id returns `AssetResolveFailed` |
| `src/types/template.ts` | Modify | Add `folder: string` and `folders: string[]` |
| `src/lib/folderTree.ts` | Create | Pure functions `folderContents()` and `breadcrumbSegments()` |
| `src/stores/overlayExplorer.ts` | Create | `currentFolder`, `contents`, `open(path)`, `up()`, and a fallback to the nearest existing ancestor after a reload |
| `src/stores/templates.ts` | Modify | Store `folders` next to `templates` (minimal edit) |
| `src/components/OverlaysPage.vue` | Modify | Container: wires the breadcrumb, the grid and the empty-folder message |
| `src/components/OverlayGrid.vue` | Modify | Presentational: renders folder cards first, then overlay cards |
| `src/components/FolderCard.vue` | Create | Presentational: name, direct count, emits `open` |
| `src/components/FolderBreadcrumb.vue` | Create | Presentational: segments, emits `navigate(path)` |
| `src/i18n/locales/{es,en}.json` | Modify | Root label, overlay count (plural), empty-folder message |

## Interfaces / Contracts

```ts
export interface FolderEntry { name: string; path: string; directCount: number }
export function folderContents(templates: TemplateInfo[], folders: string[], current: string):
  { folders: FolderEntry[]; templates: TemplateInfo[] } // direct children only; each list sorted by name (localeCompare)
export function breadcrumbSegments(current: string): { name: string; path: string }[]
```

## Testing Strategy

| Layer | What | Approach |
|---|---|---|
| Rust unit (TDD) | nested discovery, `folder`/`path` values, empty folders listed, hidden dirs skipped, depth cutoff, a leaf with a nested `overlay.json` is not walked, the deterministic duplicate winner, symlink skipped (`#[cfg(unix)]`) | temp dirs, following the existing `temp_dir` helper |
| Rust unit | asset resolution for flat, nested and `meta.id != dir` templates; outside-dir rejection; unknown id | temp dirs; `TemplateCatalog::template_dir` with `FakeSource` |
| Frontend | `folderContents` and `breadcrumbSegments` | No runner exists today, so this relies on `vue-tsc` and manual checks unless vitest is approved |
| Manual | `pnpm tauri dev`: navigating nested folders, OBS URL for a nested overlay, file field in a nested overlay | checklist |

## Migration / Rollout

No data migration is needed. The fields are additive. The merge with `remote-overlay-control` can go in either order, and whichever merges second rebases `OverlaysPage.vue` and `ports.rs`. Explorer state lives in a new store to stay out of `templates.ts`, which that change rewrites.

## Open Questions

- [x] Add vitest as a dev dependency so `folderTree.ts` gets tests? Resolved: yes (see Post-design decisions).
- [x] Skipping symlinks hides root-level symlinked overlays. Resolved: acceptable.

## Post-design decisions

1. Symlinks: the user does not use them. Discovery skips symlinks; the change from `is_dir()` is accepted.
2. Add `vitest` as a frontend devDependency, with a `test` script, to unit-test `src/lib/folderTree.ts` test-first (per-level contents, folders-first ordering, direct counts, empty folders, breadcrumb segments). Running frontend tests in the `pre-push` hook is optional and needs a user decision.
3. Home has no search, so the search decision is N/A. Depth limit = 5. Duplicate ids: alphabetical depth-first walk, first wins, `eprintln` warning (root `x` vs `a/x`: `a/x` wins).
4. Performance is out of scope.
