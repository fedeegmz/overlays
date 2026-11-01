# Archive Report: overlay-detail-layout

**Project**: overlays
**Branch**: fix/overlay-detail-layout
**Archive date**: 2026-11-01
**Artifact store**: openspec
**Archived to**: `openspec/changes/archive/2026-11-01-overlay-detail-layout/`
**Commits**: Phase 6 checkmarks and verify report in `7b6136a`; the archive is committed on top of it.

## Verification gate

- verify-report verdict read: **PASS WITH WARNINGS**, 0 CRITICAL, 2 warnings, 2 suggestions.
- Archive was approved by the user despite the warnings. Warnings are recorded below, not resolved.
- Commands recorded by the verifier: `pnpm check` exit 0; `vue-tsc --noEmit` exit 0; `pnpm test` 8 files / 64 tests passed; `pnpm build` OK.

### Warnings carried into archive

1. **Phase 6 manual checks are user-reported, not independently verified.** Tasks 6.1 to 6.9 cover layout behaviour that jsdom cannot exercise. The verifier recorded them as "todo ok" from the user. They are marked as user-reported in this report and were not reproduced by the archive or the verifier.
2. **`tasks.md` Phase 6 checkmarks were uncommitted at verify time.** Resolved: committed in `7b6136a`.

### Suggestions addressed or carried

- Suggestion 2 (long instance label is defensive): addressed. The main spec scenario "Long instance label [Manual]" now carries a NOTE that the label is `id.slice(0, 6)` today, so the case cannot occur currently and is kept as a defensive guard. The requirement is kept.
- Suggestion 2 (`openspec/config.yaml` stale on `unit_runner`): NOT changed, per instruction. Follow-up only.
- Suggestion 1 (locale test hardcodes key list rather than comparing to main): accepted by the verifier; no action.

## Task gate

- `tasks.md`: 24 of 24 items checked (`[x]`), 0 unchecked. Phases 1 to 5 are 15 items; Phase 6 is 9 items (6.1 to 6.9).
- No stale-checkbox reconciliation was needed or performed.
- Note: the Phase 6 heading still reads "Left unchecked until a person runs them." while its items are checked. Those checks are the user-reported results above. The heading text is stale and was not edited here.

## Specs synced to `openspec/specs/`

| Capability | Action | Details |
|------------|--------|---------|
| `overlay-detail-layout` | Created | New capability; 6 requirements and 14 scenarios copied from the ADDED delta. Scenario "Long instance label" carries the defensive-guard note. |

No existing main spec was modified. The change is additive and destructive-free.

Format note: the new main spec uses the title line `# Delta Spec: overlay-detail-layout (new capability)` and a `## Requirements` section, matching the existing main specs (for example `overlay-catalog-browsing` and `instance-registry`). The `## ADDED Requirements` heading from the delta was converted to `## Requirements`.

## Spec conflicts

- None. The capability has no overlap with `overlay-catalog-browsing`, `instance-registry`, or the remote-control specs.

## Artifact traceability

Artifacts are in openspec mode; there are no Engram observation IDs for this change in this archive step.

| Artifact | Path (before archive) |
|----------|-----------------------|
| exploration | `openspec/changes/overlay-detail-layout/exploration.md` |
| proposal | `openspec/changes/overlay-detail-layout/proposal.md` |
| spec (delta) | `openspec/changes/overlay-detail-layout/specs/overlay-detail-layout/spec.md` |
| design | `openspec/changes/overlay-detail-layout/design.md` |
| tasks | `openspec/changes/overlay-detail-layout/tasks.md` |
| verify-report | `openspec/changes/overlay-detail-layout/verify-report.md` |
| archive-report | `openspec/changes/overlay-detail-layout/archive-report.md` (this file) |

Main spec: `openspec/specs/overlay-detail-layout/spec.md`.

## Archive steps

1. Task completion gate: passed (24 of 24 checked).
2. Delta spec merged into `openspec/specs/overlay-detail-layout/spec.md` (created; new capability).
3. Folder move: `openspec/changes/overlay-detail-layout/` to `openspec/changes/archive/2026-11-01-overlay-detail-layout/`. Done.
4. Archive committed (conventional commit, no AI attribution).
5. Engram archive report saved as `sdd/overlay-detail-layout/archive-report` (launcher contract).

## Follow-ups (not blocking)

- Run the Phase 6 manual checks again if the layout code changes; they are user-reported only.
- Update `openspec/config.yaml` (`unit_runner: none` is stale; frontend tests run via `pnpm test`).
- Clean the stale Phase 6 heading in `tasks.md` in a later change if the archived file is ever edited (the archive itself is not edited).

## Status

Specs synced, change archived. Verdict PASS WITH WARNINGS, user-approved.
