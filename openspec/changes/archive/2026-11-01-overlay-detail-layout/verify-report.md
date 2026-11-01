# Verify Report: overlay-detail-layout

Verdict: PASS WITH WARNINGS (0 CRITICAL, 2 WARNING, 2 SUGGESTION)

## Commands
- pnpm check: Checked 114 files, no fixes, exit 0
- pnpm exec vue-tsc --noEmit: no output, exit 0
- pnpm test: 8 files, 64 tests passed
- pnpm build: built OK (vite, 343ms)

## Matrix
| Requirement / scenario | Evidence | Status |
|---|---|---|
| No overflow: long path, long label | OverlayDetailPage.vue CSS (min-width:0, overflow-wrap:anywhere, gap, flex-shrink); user-reported 6.1, 6.2 | User-reported, not independently verified |
| URL text absent | test "does not render the URL as text..." | PASS (auto) |
| Copy succeeds | test "copies the full URL..." | PASS (auto) |
| Copy fails | test "shows the failure message..." | PASS (auto) |
| Button placement / gap | .copy-btn margin-right:8px, direct .page-header child; user-reported 6.3 | User-reported |
| Other screens unaffected | src/styles/components.css unchanged vs main; user-reported 6.4 | Verified by diff plus user-reported |
| Pinned preview (3 scenarios) | .preview-column sticky top:34px; user-reported 6.5, 6.6, 6.9 | User-reported |
| Short windows | max-height calc(100vh - 68px), overflow-y:auto, action-row flex-wrap; user-reported 6.7, 6.8 | User-reported |
| Locales unchanged | test keys + src/i18n has no diff vs main | PASS |
| Changelog | CHANGELOG.md Fixed entry | PASS |
| Dead styles removed | .page-subtitle(-url) removed here; other pages scope their own; no remaining refs in this file | PASS |

## Design constraints
- FieldEditor.vue, AppShell.vue, global .icon-btn (styles/components.css), src/i18n: no diff vs main. OK
- Only jsdom and @vue/test-utils added to package.json. OK
- jsdom env only in OverlayDetailPage.test.ts (rg vitest-environment); other tests on node. OK
- No Co-Authored-By in the 3 commits. OK
- tasks.md edit (Phase 6 checked) is uncommitted.

## Test validity
- URL-absent test fails for the right reason: pre-change the URL span rendered URL_VALUE in html() (observed RED 1 failed / 63 passed).
- Copy tests assert on real click wiring (copyText call arg, aria-label, copy-success class, 2s timer); would fail if wiring broke.
- Phase 6 results are user-reported ("todo ok"); not verified by me.

## Findings
WARNING 1: Phase 6 manual scenarios (all layout requirements) have no independent verification; jsdom does no layout. Rely on user report.
WARNING 2: tasks.md Phase 6 checkmarks are uncommitted; commit before archive.
SUGGESTION 1: Locale test hardcodes the key list rather than comparing to main; acceptable since the diff vs main is empty.
SUGGESTION 2: Scenario "long instance label" cannot occur in practice (label is id.slice(0,6)); consider noting in the spec. Also openspec/config.yaml is stale on unit_runner.
