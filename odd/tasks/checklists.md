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

- [ ] 2.1 `ChecklistRepository` port (strict load)
- [ ] 2.2 RED: service tests with in-memory repo
- [ ] 2.3 GREEN: `ChecklistService::{list,save,delete}` (validate → load → mutate → write)
- [ ] 2.4 RED → GREEN: `JsonChecklistRepository` (missing, round trip, no check fields, corrupt untouched)

### WU3 — `feat(checklists): expose checklist commands and frontend store` (route: delegated)

- [ ] 3.1 Tauri commands + `lib.rs` wiring
- [ ] 3.2 TS types + `checklistsApi`
- [ ] 3.3 Pinia store with ephemeral checks and `pruneChecked`
- [ ] 3.4 `errors.checklist.*` i18n keys (es voseo, en)

### WU4 — `feat(checklists): add checklist page with list and run views` (route: delegated)

- [ ] 4.1 `NavPage` + SideNav entry + `App.vue` page
- [ ] 4.2 `ChecklistsPage.vue` container
- [ ] 4.3 `ChecklistListView.vue`
- [ ] 4.4 `ChecklistRunView.vue`
- [ ] 4.5 i18n page strings

### WU5 — `refactor(dnd): extract useDragReorder composable` (route: delegated)

- [ ] 5.1 `src/composables/useDragReorder.ts` from `SideNav.vue`

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
| WU1 | recorded in the WU2 commit | cargo test 171 pass (9 new), clippy -D warnings, fmt OK | RED: missing-symbol compile errors; temporary `#[allow(dead_code)]` on `domain::checklist` and `DomainError` (lib.rs mods are private) |

## Next step

WU2.
