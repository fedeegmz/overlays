# Archive Report: remote-overlay-control

**Project**: overlays
**Branch**: feat/remote-overlay-control (verified at commit 370240f)
**Archive date**: 2026-11-01
**Archived to**: `openspec/changes/archive/2026-11-01-remote-overlay-control` (folder move performed by the parent with `git mv` after this report is written; this report moves with the folder)
**Commits**: none created by archive

## Verification gate

- verify-report verdict read: **PASS WITH WARNINGS**, 0 CRITICAL, 3 warnings, 3 suggestions.
- Warnings 1 and 2 were resolved before archive (Engram mirrors refreshed; `design.md`, `proposal.md`, `tasks.md` 9.4 updated). Warning 3 (no automated Vue component tests) was accepted by the user for a later change.
- Task gate: `tasks.md` has 50 of 50 items checked (`[x]`), 0 unchecked. No stale-checkbox reconciliation was needed or performed.
- Dependencies: proposal, specs, design, tasks, apply, and verify all done.

## Artifact traceability (Engram observation IDs)

| Artifact | Engram observation | Topic key |
|----------|--------------------|-----------|
| proposal | #895 | `sdd/remote-overlay-control/proposal` |
| spec | #897 | `sdd/remote-overlay-control/spec` |
| design | #898 | `sdd/remote-overlay-control/design` |
| tasks | #899 | `sdd/remote-overlay-control/tasks` |
| verify-report | #1030 | `sdd/remote-overlay-control/verify-report` |

The Engram observations are condensed mirrors. The openspec files in `openspec/changes/remote-overlay-control/` are authoritative, and the specs below were synced from those files.

## Specs synced to `openspec/specs/`

| Capability | Action | Details |
|------------|--------|---------|
| `instance-registry` | Created | 10 requirements, copied from the ADDED delta |
| `remote-control-access` | Created | 8 requirements, copied from the ADDED delta |
| `remote-control-api` | Created | 7 requirements, copied from the ADDED delta |
| `remote-control-ui` | Created | 8 requirements, copied from the ADDED delta |
| `lan-file-upload` | Updated (MODIFIED) | Replaced `Network exposure and loopback guard` (3 scenarios added). The other 8 requirements are unchanged. |
| `group-preset-instance-identity` | Updated (MODIFIED) | Replaced `Save rules`, `Load rules per entry`, `Conflict reporting`, and `Out-of-scope behavior unchanged`. Added 5 scenarios: `Entries built from the registry`, `Remote start parity`, `Events emitted`, `Remote conflict surfaced`, and `Overlay protocol unchanged with remote surface`. The other 3 requirements are unchanged (`Entry identity fields`, `Legacy compatibility`, `Duplicate instance id validation`). |

Read-back: every synced file was read after writing, and all requirements and modified scenarios are present. The `(Previously: ...)` annotations from the delta files were not copied into the main specs.

## Spec conflicts

- None found during the merge. The `Template resolution` requirement in `instance-registry` is consistent with `overlay-catalog-browsing` (id is `meta.id` or directory name, no folder segments, first in traversal order wins).
- Note: the existing main specs use the title line `# Delta Spec: <capability> (new capability)`. The four new main specs follow that shape for consistency. This is a pre-existing convention, not changed here.

## Follow-ups (not blocking)

- Accepted by the user for a later change: automated component tests for `InstanceCard`, `PairView`, `ControlView`, and `RemoteSettingsSection`.
- Partial coverage rows from verify-report, not blocking: re-pair after rotation, two-client WS echo, duplicate template id across folders, remote vs desktop group-start parity test, and restart-leaves-no-instances test.
- Manual checks (tasks 3.5, 4.6, 7.5, 8.3, 9.5) were reported as passed by the user. The verifier did not reproduce them.

## Archive steps

1. Task completion gate: passed.
2. Delta specs merged into `openspec/specs/` (see table above).
3. Folder move: `openspec/changes/remote-overlay-control/` to `openspec/changes/archive/2026-11-01-remote-overlay-control/`, performed by the parent. Nothing was copied, and nothing was written inside `openspec/changes/archive/`.
4. Engram archive report saved as `sdd/remote-overlay-control/archive-report`.

## Status

Archived and closed. Ready for the next change.
