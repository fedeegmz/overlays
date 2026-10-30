# Verify Report: overlay-folder-explorer

Verdict: PASS WITH WARNINGS (strict TDD, backend-scoped)

## Commands
- cargo test: 203 passed, 0 failed
- cargo clippy -- -D warnings: clean
- pnpm test: 1 file, 7 passed
- pnpm check: clean (91 files)
- pnpm build: green (biome, vue-tsc, vite)

## Tasks
All checked except 4.6 (manual pnpm tauri dev check): PENDING (user).

## Findings
CRITICAL: none.
WARNING:
- Explorer UI scenarios (direct count rendering, empty-folder card/state, breadcrumb nav, restart reset, flat setup, es/en strings) have no component/store test; only folderTree helpers are covered (src/lib/folderTree.test.ts). Covered by manual 4.6 only.
- Spec "Traversal rejected" / "Nested asset served": resolve_asset_path covers rejection of files outside overlays dir (fs_template_source.rs:691), but no test of `..` escaping the template dir for HTTP serving; serving itself is unchanged static /overlay/*.
- Spec "duplicate warning in log": eprintln only; no test asserts the warning.
SUGGESTION:
- folderExists (src/lib/folderTree.ts:62) is used by overlayExplorer fallback; not drift, it implements the design's ancestor fallback. Keep.
- overlayExplorer.up() (src/stores/overlayExplorer.ts:24) unused; remove or wire to UI.
- No dedicated Manifest compat test; additive serde fields are plain (folder/folders), low risk.

## Conventions
- No invoke in components/stores (rg: none in new code).
- es.json and en.json both have rootFolder, emptyFolder, folderCount, breadcrumbLabel.
- TemplateInfo.folder and Manifest.folders additive.
