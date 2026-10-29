# Tasks: Overlay Folder Explorer

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~750-950 (Rust ~400, frontend ~350, tests/i18n ~150) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (WU1+WU2 backend) -> PR 2 (WU3+WU4 frontend) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Domain + recursive discovery (Rust) | PR 1 | `feat(discovery)`; cargo tests included |
| 2 | Asset resolution via catalog lookup | PR 1 | `fix(assets)`; depends on WU1 |
| 3 | vitest + folderTree (TDD) | PR 2 | `feat(explorer)`; independent of Rust, needs only the type shape |
| 4 | Store, components, i18n, docs | PR 2 | `feat(ui)`; depends on WU1+WU3 |

## WU1: Domain and recursive discovery

- [x] 1.1 RED: in `src-tauri/src/infrastructure/fs_template_source.rs` tests, add temp-dir cases: nested discovery with `folder`/`path`, empty folders listed, hidden dirs skipped, depth 5 cutoff, leaf with nested `overlay.json` not walked, `meta.id != dir`.
- [x] 1.2 RED: add duplicate-id test (root `x` vs `a/x`: `a/x` wins) and `#[cfg(unix)]` symlink-skip test.
- [x] 1.3 Add `TemplateInfo.folder: String` and `Manifest.folders: Vec<String>` in `src-tauri/src/domain/template.rs`.
- [x] 1.4 Change `discover` to return `DomainResult<Manifest>` in `application/ports.rs`; pass through in `application/template_catalog.rs`; update `FakeSource`.
- [x] 1.5 GREEN: implement the sorted recursive walk (`MAX_DEPTH=5`, `file_type()` symlink skip, `eprintln` on duplicate/subdir error) in `fs_template_source.rs`.
- [x] 1.6 Run `cargo test`, `cargo fmt --check --all`, `cargo clippy -- -D warnings`; commit.

## WU2: Nested asset resolution

- [x] 2.1 RED: tests for `TemplateCatalog::template_dir(id)` (FakeSource; strips `/index.html`; unknown -> None) in `template_catalog.rs`.
- [x] 2.2 RED: tests for `resolve_asset_path(overlays_dir, template_rel_dir, abs)` covering flat, nested, `meta.id != dir`, outside-dir rejection.
- [x] 2.3 GREEN: implement `template_dir` and the new `resolve_asset_path` signature.
- [x] 2.4 Update `resolve_overlay_asset_path` in `infrastructure/tauri/commands.rs` to look up the id via `http.catalog`; unknown id -> `AssetResolveFailed`.
- [x] 2.5 Run Rust gates; commit.

## WU3: Frontend test runner and folderTree

- [ ] 3.1 Add `vitest` devDependency and `"test": "vitest run"` to `package.json`; add minimal config if needed; confirm `pnpm check` and `vue-tsc --noEmit` still pass.
- [ ] 3.2 Update `src/types/template.ts` with `folder: string` and `folders: string[]`.
- [ ] 3.3 RED: write `src/lib/folderTree.test.ts` (contents per level, folders-first data, direct counts, empty folders, breadcrumb segments, root).
- [ ] 3.4 GREEN: implement `src/lib/folderTree.ts` (`folderContents`, `breadcrumbSegments`, `FolderEntry`).
- [ ] 3.5 Run `pnpm test`, `pnpm check`; commit.
- [ ] 3.6 OPTIONAL (user decision, not default): add a `pnpm test` command to `lefthook.yml` `pre-push`, then `pnpm exec lefthook install`.

## WU4: Explorer UI

- [ ] 4.1 Store `folders` in `src/stores/templates.ts` (minimal edit).
- [ ] 4.2 Create `src/stores/overlayExplorer.ts`: `currentFolder`, `contents`, `open`, `up`, fallback to nearest existing ancestor after reload.
- [ ] 4.3 Create `FolderCard.vue` and `FolderBreadcrumb.vue` (presentational; emit `open` / `navigate`).
- [ ] 4.4 Update `OverlayGrid.vue` (folders first) and `OverlaysPage.vue` (breadcrumb, grid, empty-folder message).
- [ ] 4.5 Add root label, plural overlay count, empty-folder strings to `src/i18n/locales/{es,en}.json`.
- [ ] 4.6 Run `pnpm build`; manual check in `pnpm tauri dev`: nested navigation, OBS URL of a nested overlay, file field in a nested overlay.
- [ ] 4.7 Update `AGENTS.md` (nested discovery, `folder`/`folders` fields, vitest command); commit.
