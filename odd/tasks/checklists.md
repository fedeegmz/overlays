# Feature: checklists

## Objective

Add an in-app "Checklists" page: create N named checklists with ordered string items, and tick items off while working through them.

## Problem / why

Repetitive stream setup steps have no in-app place to be tracked. Checks must start unchecked on every app start.

## Scope

- In: new sidebar page (list view + run view + editor), full CRUD of lists and items, drag-and-drop reorder in edit mode only, "Uncheck all", progress counter and completed state, validation (name required + unique case-insensitive, at least one item, no blank items, duplicate texts allowed, trimmed), id-based check preservation across edits, `checklists.json` with strict load, structured `CommandError`s, es (voseo) + en.
- Out: OBS/overlay integration, persisted check state, user-visible ids, import/export.

## Constraints

- Mirror the group-presets stack (domain → service + port → strict JSON repo → Tauri commands → TS api → Pinia store).
- Check state lives only in the Pinia store.
- No new crates or Tauri plugins; ids generated in the frontend with `crypto.randomUUID()`.

## References

- Engram (project `overlays`): `sdd/checklists/{explore,scope,proposal,spec,spec-assumptions,design,tasks}`.
- Spec: 15 requirements / 52 scenarios.

## Delivery

- Strategy: all work-unit commits on `feat/checklists` (user decision, 2026-10-19). PR slicing deferred to the user.
- Forecast: ~1450 authored changed lines across 7 work units.
- Receipt-driven development: off (global) — ordinary checks only.

## Checks

- Rust: `cd src-tauri && cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check --all`.
- Frontend: `pnpm exec vue-tsc --noEmit`, `pnpm check` (no frontend test runner — manual smoke in `pnpm tauri dev`).

## Tasks

### WU1 — `feat(checklists): add checklist domain and error codes` (route: delegated, writer trigger: 3 Rust files)

- [x] 1.1 RED: domain tests for `Checklist::new` (trim, invalid ids, empty name, no items, blank item index, order and duplicate texts kept)
- [x] 1.2 GREEN: `ChecklistItem`, `Checklist::new`, 8 `DomainError` variants
- [x] 1.3 RED → GREEN: `checklist.*` error-code mapping test and implementation

### WU2 — `feat(checklists): add checklist service and JSON repository` (route: delegated)

- [x] 2.1 `ChecklistRepository` port (strict load)
- [x] 2.2 RED: service tests with in-memory repo
- [x] 2.3 GREEN: `ChecklistService::{list,save,delete}` (validate → load → mutate → write)
- [x] 2.4 RED → GREEN: `JsonChecklistRepository` (missing, round trip, no check fields, corrupt untouched)

### WU3 — `feat(checklists): expose checklist commands and frontend store` (route: delegated)

- [x] 3.1 Tauri commands + `lib.rs` wiring
- [x] 3.2 TS types + `checklistsApi`
- [x] 3.3 Pinia store with ephemeral checks and `pruneChecked`
- [x] 3.4 `errors.checklist.*` i18n keys (es voseo, en)

### WU4 — `feat(checklists): add checklist page with list and run views` (route: delegated)

- [x] 4.1 `NavPage` + SideNav entry + `App.vue` page
- [x] 4.2 `ChecklistsPage.vue` container
- [x] 4.3 `ChecklistListView.vue`
- [x] 4.4 `ChecklistRunView.vue`
- [x] 4.5 i18n page strings

### WU5 — `refactor(dnd): extract useDragReorder composable` (route: delegated)

- [x] 5.1 `src/composables/useDragReorder.ts` from `SideNav.vue`

### WU6 — `feat(checklists): add checklist editor with drag reorder` (route: delegated)

- [ ] 6.1 `ChecklistEditor.vue`
- [ ] 6.2 Wire create/edit/rename in the page
- [ ] 6.3 Editor i18n strings
- [ ] 6.4 `CHANGELOG.md` entry under [Unreleased] → Added

### WU7 (optional) — `refactor(nav): use useDragReorder in SideNav` (route: delegated)

- [ ] 7.1 Migrate SideNav drag code to the composable

## Progress

| WU | Commit | Checks | Notes |
|---|---|---|---|
| WU1 | ca40248 | cargo test 171 pass (9 new), clippy -D warnings, fmt OK | RED: missing-symbol compile errors; temporary dead_code allows later removed in WU2 |
| WU2 | b82e99d | cargo test 185 pass (14 new; parent spot check 185 pass), clippy -D warnings, fmt OK | Assess: medium (RDD off → writer self-verification). 4 dead_code allows with `TODO(checklists WU3)` remain |
| WU3 | e247649 | cargo test 185 pass, clippy -D warnings, fmt OK; vue-tsc, pnpm check OK | Commands + lib.rs wiring; 4 dead_code allows removed; `empty_item` copy omits the 0-based `index` param |
| WU4 | 32e04ee | vue-tsc, pnpm check OK; cargo test 185 pass, clippy, fmt OK | Create/Edit emit to no-op handlers in `ChecklistsPage` until WU6. Manual smoke pending (user) |
| WU5 | see git log | vue-tsc, pnpm check OK; cargo test 185 pass | Composable only; SideNav untouched (WU7 optional); not yet imported (Biome/tsc accept it). No Rust change |

## Next step

WU5 (`useDragReorder` composable), then WU6 (editor; replace the `noop` create/edit handlers in `ChecklistsPage.vue`).
