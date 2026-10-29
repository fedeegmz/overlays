# Exploration: overlay-folder-explorer

## Goal

Replace the flat home grid with a file-explorer view that mirrors the overlays directory (folders + loose overlays at any level), and investigate the slow/laggy startup with 18 overlays (observed in `pnpm tauri dev`).

## Current state

- **Discovery is flat**: `src-tauri/src/infrastructure/fs_template_source.rs:151-223` reads one directory level. A directory is an overlay iff it has `overlay.json` (:169) and `index.html` (:174). `id = meta.id || dir name` (:204); `path = "{folder}/index.html"` (:216) — id and path are already decoupled.
- **Latent bug**: `resolve_asset_path` uses `canonical_dir.join(template_id)` (`fs_template_source.rs:100`), assuming id == dir name. Breaks when `meta.id` differs from the dir name, and for nested overlays. Should derive the template dir from `TemplateInfo.path`.
- **Serving**: `http/routes.rs:63` exposes `/overlay/{*path}` (wildcard) with a canonical `starts_with` guard (:23-27) — nested paths already work.
- **Ids are load-bearing**: overlay scripts hardcode `const TEMPLATE_ID = "<dirname>"` and filter WS messages by it (`examples/*/script.js:3,58`). The WS payload `template` and presets/group presets store the id string (`stores/instances.ts:63,182`). Using a relative path as id would break all existing overlays and presets.
- **Frontend**: `OverlaysPage.vue:77` → `OverlayGrid.vue:16` → `OverlayCard.vue`. Cards are pure CSS mock previews — no iframe, no per-card WS, no per-card `invoke`. The only iframe is `PreviewPanel.vue:106` (detail page). Templates load via a single `listTemplates` call (`stores/templates.ts:15`, from `bootstrap.ts:13`).

## Startup lag — hypotheses (unmeasured)

- `backdrop-filter: blur` on every card (`OverlayCard.vue:64`) ×18 plus hover transforms — plausible WebKitGTK GPU cost on Linux.
- `list_templates` is a sync `#[tauri::command]` (`tauri/commands.rs:50`) → runs on the main thread in Tauri v2 (I/O for 18 dirs is small, though).
- Dev-only factors: Rust debug build, Vite unbundled modules.
- No code evidence of a heavy home-level cost. Measure with `pnpm tauri build --debug` / release before claiming a fix.

## Specs and overlapping changes

- `openspec/specs/` has only `lan-file-upload`, `path-template-variables`, `upload-settings`, `group-preset-instance-identity` — no templates/home spec → new capability spec.
- `remote-overlay-control` (planned, tasks not started) touches `OverlaysPage.vue` and `stores/templates.ts`; `GET /remote/templates` returns the `Manifest` (`design.md:76`). Adding fields to `TemplateInfo` is additive; keep `Manifest` backward-compatible. Merge-conflict risk → sequence the changes.

## Approaches

| Approach | Tradeoff | Verdict |
| --- | --- | --- |
| Flat list + `folder` field (+ optional folder list for empty folders); frontend builds tree | Minimal, `Manifest`-compatible, testable | Recommended |
| Backend returns nested tree | Breaks `Manifest` consumers (incl. remote API) | Rejected |
| Breadcrumb / current-folder navigation | Fits existing grid | Recommended |
| Expandable tree | More UI, little benefit for a grid home | Rejected |
| Live/lazy previews | Cards are already static | Not needed |

## Recommendation

- Keep template id unchanged (`meta.id || dir name`); add a relative `folder` field to `TemplateInfo` (`domain/template.rs:21-27`).
- Recursive walk: depth limit (~5), skip hidden dirs, don't follow symlinks, a dir with `overlay.json` is a leaf.
- Detect id collisions across folders (skip duplicate + warn, matching existing `eprintln` style).
- Fix `resolve_asset_path` to use the template's real dir.
- Make `list_templates` async / `spawn_blocking`.
- Measure lag in a release build; treat the perf fix as evidence-driven.

## Risks

- Lag root cause unproven; the explorer itself may not fix it.
- Id collisions across folders.
- `resolve_asset_path` bug.
- Conflicts with `remote-overlay-control` in the same frontend files.
- Current-folder UI state not persisted across navigation.
