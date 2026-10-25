# Archive Report: preset-instance-identity

**Change**: preset-instance-identity
**Project**: overlays
**Branch**: feat/preset-instance-identity
**Artifact store**: hybrid
**Archived on**: 2026-10-25
**Archive path**: `openspec/changes/archive/2026-10-25-preset-instance-identity/` (folder move performed by the orchestrator)
**Archive verdict**: Intentional-with-warnings. No CRITICAL issues.

## Summary

Group presets now persist each entry's instance id (`instance_id`) and user-set custom name (`custom_name`). Loading a group restores the same instance identity, so OBS browser sources using `?instance=<id>` keep working across loads. Legacy `groups.json` files load unchanged, and re-saving a group adds stable ids. Duplicate instance ids within a group are rejected with a structured `CommandError` (`group_preset.duplicate_instance_id`) and mapped to localized messages in `es.json` and `en.json`.

## Commits

| Commit | Message | Work unit |
|--------|---------|-----------|
| bc09fcf | `feat(group-presets): persist instance identity in entries` | WU1 (backend domain + error code) |
| 86c6358 | `test(group-presets): cover legacy load and duplicate rejection` | WU2 (persistence and service tests) |
| d097a8f | `feat(groups): restore instance id and name on group load` | WU3 (frontend reconcile, i18n, CHANGELOG) |
| 55c8b88 | `docs(openspec): add preset-instance-identity verify report` | Verify report |

## Verification

**Verdict**: PASS WITH WARNINGS (0 CRITICAL, 5 WARNING, 3 SUGGESTION), from `openspec/changes/preset-instance-identity/verify-report.md`.

Execution evidence:
- `cd src-tauri && cargo test`: 191 passed, 0 failed
- `cargo fmt --check --all`: OK
- `cargo clippy -- -D warnings`: clean
- `pnpm check` (Biome): clean, 86 files
- `pnpm exec vue-tsc --noEmit`: clean

Warnings carried into archive:
1. The apply-progress artifact has no formal TDD Cycle Evidence table (prose only).
2. WU2 tests were characterization tests and had no observed RED.
3. Undocumented rollback on failed in-place update (see accepted deviation below).
4. Frontend save/load scenarios are verified by code reading only (no frontend test runner).
5. Phase 4 manual checks were pending. Now confirmed (see below).

## Manual checks

Phase 4 checks 4.1–4.4 were confirmed by the user as performed successfully on 2026-10-25 and are marked `[x]` in `openspec/changes/preset-instance-identity/tasks.md`. This is user confirmation, not a stale-checkbox reconciliation. Coverage:
- 4.1 Save, reload, load: OBS URL id unchanged, custom name restored.
- 4.2 Load same group twice: no duplicates; renamed live instance keeps its name when the entry has none.
- 4.3 Different-template collision: fresh id and localized notice; no notice otherwise.
- 4.4 Legacy `groups.json` loads; re-save adds ids; duplicate-id error is localized in es and en.

## Accepted deviation

On a failed in-place update, `startGroup` restores the instance's previous `fields` and `customName` (`src/stores/groupPresets.ts:124-128`). The design only specified "remove only newly created" instances on send failure. This rollback goes beyond the design, is not documented in the design, and was accepted as part of this archive. It is a benign extension that keeps a failed load from leaving a live instance half-updated.

## Follow-up suggestions (not blocking)

- Extract the per-entry reconcile logic in `startGroup` into a pure function so it can be unit-tested without the Vue/Pinia stack. This addresses warning 4 directly.
- Optional: load re-trims `instance_id` and `custom_name` (harmless; handles hand-edited blanks).
- Optional: confirm the `muted` class used by the `idConflict` notice is readable (verify suggestion 3).

## Decisions recorded during the change

Engram observation #887 (`decision`, topic `sdd/preset-instance-identity/decisions`) records the user decision on 2026-10-25: when a same-template live instance is updated in place, its custom name is replaced only if the entry has a saved `custom_name`; otherwise the live name is kept. This overrides the design's "saved state wins" wording. The filesystem spec and design are authoritative for this point.

## Artifact traceability

| Artifact | Engram observation | Filesystem source |
|----------|--------------------|-------------------|
| proposal | #884 (`sdd/preset-instance-identity/proposal`) | `openspec/changes/preset-instance-identity/proposal.md` |
| spec | #885 (`sdd/preset-instance-identity/spec`) | `openspec/changes/preset-instance-identity/specs/group-preset-instance-identity/spec.md` |
| design | #886 (`sdd/preset-instance-identity/design`, marked superseded by #887) | `openspec/changes/preset-instance-identity/design.md` |
| decision | #887 (`sdd/preset-instance-identity/decisions`) | n/a |
| tasks | #888 (`sdd/preset-instance-identity/tasks`) | `openspec/changes/preset-instance-identity/tasks.md` |
| verify-report | #891 (`sdd/preset-instance-identity/verify-report`) | `openspec/changes/preset-instance-identity/verify-report.md` |
| archive-report | this document, saved as `sdd/preset-instance-identity/archive-report` | `openspec/changes/preset-instance-identity/archive-report.md` |

## Main specs updated

- **Created**: `openspec/specs/group-preset-instance-identity/spec.md` (new capability; 7 requirements, all copied from the delta). The delta header and `(new capability)` label were kept to match the existing main specs (`upload-settings`, `path-template-variables`).

## Archive contents

- `proposal.md`: present
- `specs/group-preset-instance-identity/spec.md`: present
- `design.md`: present
- `tasks.md`: present, 20/20 checked (16 implementation tasks 1.1–3.8 plus 4 manual checks 4.1–4.4). No unchecked tasks remain. Note: verify-report.md states "21/21" for tasks 1.1–3.8, but the file contains 16 tasks in that range. The count in the verify report is a documentation error and does not affect the verdict.
- `verify-report.md`: present
- `archive-report.md`: present

## Pending steps

- The orchestrator moves `openspec/changes/preset-instance-identity/` to `openspec/changes/archive/2026-10-25-preset-instance-identity/`. The archive executor did not perform this move.
- The orchestrator commits the archive, including the new main spec, the updated `tasks.md`, and the moved folder.
