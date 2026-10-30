# Archive Report: overlay-folder-explorer

**Change**: overlay-folder-explorer
**Project**: overlays
**Branch**: feat/overlay-folder-explorer
**Artifact store**: hybrid
**Archive date**: 2026-10-29
**Archive path**: `openspec/changes/archive/2026-10-29-overlay-folder-explorer/` (folder move pending, see Pending steps)
**Archive verdict**: Complete, with warnings. No CRITICAL issues. No override used.

## Summary

The home page now works as a file explorer over the overlays directory. Overlays are discovered recursively (depth 5, hidden dirs and symlinks skipped, overlay dirs are leaves). Templates carry an additive `folder` field and the catalog exposes `folders`, including empty ones. Nested overlay assets resolve from their real directory. The home page shows folder cards, a breadcrumb, and direct overlay counts. Duplicate ids keep the first match in traversal order and log a warning.

## Commits

| Commit | Message | Work unit |
|--------|---------|-----------|
| 597038c | `feat(templates): discover overlays recursively in nested folders` | WU1 (domain and recursive discovery) |
| df3411d | `fix(templates): resolve assets from the template's real directory` | WU2 (nested asset resolution) |
| 1c36e36 | `test(frontend): add vitest and folder tree helpers` | WU3 (vitest and folderTree) |
| a8cc10f | `feat(home): browse overlays as a folder explorer` | WU4 (store, components, i18n, docs) |
| d9c5016 | `chore(hooks): run frontend tests on pre-push` | Task 3.6 (user-approved) |

## Verification

**Verdict**: PASS WITH WARNINGS (0 CRITICAL, 3 WARNING, 3 SUGGESTION), from `openspec/changes/overlay-folder-explorer/verify-report.md`.

Execution evidence (from verify-report):
- `cargo test`: 203 passed, 0 failed
- `cargo clippy -- -D warnings`: clean
- `pnpm test`: 7 passed
- `pnpm check`: clean
- `pnpm build`: green

Warnings carried into archive:
1. Explorer UI scenarios (direct counts, empty-folder card and state, breadcrumb navigation, restart reset, flat setup, es/en strings) have no component or store test. Only `folderTree` helpers are covered. Covered by manual check 4.6 only.
2. No test for `..` escaping the template directory over HTTP. `resolve_asset_path` rejection is tested at the function level.
3. The duplicate-id warning is `eprintln` only and no test asserts it.

Suggestions carried into archive (non-blocking): `overlayExplorer.up()` is unused (remove or wire it); `folderExists` is kept as the ancestor fallback; no dedicated Manifest compatibility test.

## Manual checks

Task 4.6 (`pnpm tauri dev` check: nested navigation, OBS URL of a nested overlay, file field in a nested overlay) was confirmed by the user as passed. This is user confirmation, not a stale-checkbox reconciliation.

## Task completion gate

`openspec/changes/overlay-folder-explorer/tasks.md` was read in full before the sync. Every implementation task (1.1–4.7, plus 3.6) is `[x]`. No unchecked tasks remain. No exceptional reconciliation was needed.

Note: Engram observation #1021 (`sdd/overlay-folder-explorer/tasks`) is the pre-apply snapshot (all tasks unchecked at save time). The filesystem `tasks.md` is the source of truth for completion and is final.

## Artifact traceability

| Artifact | Engram observation | Filesystem source |
|----------|--------------------|-------------------|
| proposal | #1018 (`sdd/overlay-folder-explorer/proposal`) | `openspec/changes/overlay-folder-explorer/proposal.md` |
| spec | #1019 (`sdd/overlay-folder-explorer/spec`) | `openspec/changes/overlay-folder-explorer/specs/overlay-catalog-browsing/spec.md` |
| design | #1020 (`sdd/overlay-folder-explorer/design`) | `openspec/changes/overlay-folder-explorer/design.md` |
| tasks | #1021 (`sdd/overlay-folder-explorer/tasks`, pre-apply snapshot) | `openspec/changes/overlay-folder-explorer/tasks.md` (final, all `[x]`) |
| apply-progress | #1023 (`sdd/overlay-folder-explorer/apply-progress`) | n/a |
| verify-report | #1024 (`sdd/overlay-folder-explorer/verify-report`) | `openspec/changes/overlay-folder-explorer/verify-report.md` |
| archive-report | this document, saved as `sdd/overlay-folder-explorer/archive-report` | `openspec/changes/archive/2026-10-29-overlay-folder-explorer/archive-report.md` (after move) |

## Main specs updated

- **Created**: `openspec/specs/overlay-catalog-browsing/spec.md` (new capability; 7 requirements, copied from the delta). The delta header and `(new capability)` label were kept to match the convention used by `group-preset-instance-identity`.

## Archive contents (after move)

- `proposal.md`: present
- `specs/overlay-catalog-browsing/spec.md`: present
- `design.md`: present
- `tasks.md`: present, all implementation tasks checked
- `verify-report.md`: present
- `exploration.md`: present
- `archive-report.md`: present

## Pending steps

- Move the change folder with `git mv openspec/changes/overlay-folder-explorer openspec/changes/archive/2026-10-29-overlay-folder-explorer`. The archive executor had no shell access and could not run `git mv`. Its tracked and untracked files (including this report) move together.
- Commit the new main spec, the moved folder, and this report (orchestrator).
