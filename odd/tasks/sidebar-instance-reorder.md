# Sidebar instance reorder

Locator: `odd/tasks/sidebar-instance-reorder.md` · Engram topic: `odd/sidebar-instance-reorder/tasks`
Branch: `feat/sidebar-instance-reorder` · Delivery strategy: `ask-on-risk` · Forecast: ~150 authored changed lines

## Objective

Let the user reorder active overlay instances (tabs) in the sidebar.

## Problem / why

Instances are listed in creation order with no way to rearrange them. Default labels (`Template #n`) are derived from list position, so any reorder would renumber them.

## Scope

- Each instance gets a fixed number at creation (user decision 2026-10-11: number must stay fixed, not renumber on move).
- Drag and drop reorder in the sidebar, plus keyboard reorder (Alt+ArrowUp / Alt+ArrowDown on a focused row).

## Constraints

- Instances are in-memory only (no persistence).
- Tauri `dragDropEnabled` must be `false` on the main window for HTML5 drag and drop to work on Windows; nothing in the app consumes OS file drops.
- UI strings go to both `es.json` and `en.json`.
- No frontend test runner exists (only Biome + vue-tsc), so test-first does not apply: checks are `pnpm check` and `vue-tsc --noEmit` plus manual run.

## Tasks

- [x] T1 — Fixed instance number: add `number` to `OverlayInstance`, assign the lowest free positive number per template at creation, use it in `instanceDefaultName` (still no suffix when the template has a single instance). Route: delegated (writer trigger with T2).
- [x] T2 — Reorder: `moveInstance` in the store, drag and drop + Alt+Arrow keyboard in `SideNav.vue`, `dragDropEnabled: false` in `tauri.conf.json`, hint strings in both locales, CHANGELOG entries. Route: delegated (writer trigger: store + SideNav non-trivial).

## Acceptance criteria

- Dragging a row onto another moves it to that position; active instance and custom names are preserved.
- Default labels keep their number after reordering.
- Alt+ArrowUp/Down moves the focused row and keeps focus on it.
- Clicking, rename (double-click/F2) and two-step close keep working.

## Checks

`pnpm check`, `pnpm exec vue-tsc --noEmit`.

## Progress

- 2026-10-11: document created; branch created.
- 2026-10-11: T1 done (commit `2a0cb16`). Checks: `pnpm check` clean (73 files), `vue-tsc --noEmit` exit 0.
- 2026-10-11: T2 done (commit: `feat: reorder overlay instances in the sidebar`, SHA recorded in the next update). Checks: `pnpm check` clean (73 files), `vue-tsc --noEmit` exit 0. Manual run in the app still pending.
