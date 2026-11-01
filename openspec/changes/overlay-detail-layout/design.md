# Design: Overlay detail layout

## Technical Approach

CSS and template changes in `OverlayDetailPage.vue`, plus one scoped rule in `PreviewPanel.vue`. No store, IPC, i18n, or data-flow change. Implements `specs/overlay-detail-layout/spec.md`. No sequence diagram: no complex flow.

## Architecture Decisions

| Decision | Choice | Rejected | Rationale |
|---|---|---|---|
| Header | Button becomes a direct `.page-header` child; `copyError` stays under the title | Error beside button | The header's only child today is the left column, so `space-between` then pushes the button right. The message has room to wrap |
| Overflow | `min-width: 0` on left column; `overflow-wrap: anywhere` on `.page-title`, `.breadcrumb` | `min-width: 0` alone | An unbreakable name would still overflow. The breadcrumb renders the same name |
| Right gap | `margin-right: 8px` on scoped `.copy-btn` | Global `.icon-btn` | User decided on own spacing; screen-local |
| Sticky location | `<PreviewPanel class="preview-column" />`, styled in the parent | Wrapper div; rules in `PreviewPanel.vue` | Parent scoped styles reach a child's root, the grid owner keeps layout, DOM unchanged |
| Sticky `top` | `34px` | `0`, `16px` | See Geometry |
| Short windows | `max-height: calc(100vh - 68px); overflow-y: auto` | Shrinking `.stage` | Simplest bound; a safety net that rarely engages |
| Narrow actions | `flex-wrap: wrap` on `.action-row` | `min-width: min-content` | Prevents a regression, see Geometry |
| Test environment | `jsdom` + `@vue/test-utils` dev deps, per file | SSR plus a composable extracted for tests | See Testing |

## Geometry

**Offset.** Sticky insets are measured from the scrollport, the padding box of `.content`, so its 34px padding does not offset them. `top: 0` would glue the panel to the window edge; `34px` matches the inset pages start from.

**Ancestors.** Only `.detail-page` and `.detail-layout` sit between `.content` and the panel; neither sets `overflow`, and `App.vue` adds no wrapper. `align-items: start` leaves the panel room to travel.

**Panel height** (estimates):

- Fixed parts: border 2 + header 44 + body padding 36 + action row 52 = 134.
- Widest (980 cap): column 526, stage 274 high, panel about 408.
- At `minWidth` 900 (sidebar 232, padding 88): column 306, stage 151 high, panel about 285, or 330 with a wrapped action row.
- A `sendError` adds up to about 40.

Worst case is about 448, against a bound of 550 - 68 = 482 at `minHeight`, so controls fit without inner scroll at supported sizes. If WebKitGTK measured `top` from the content edge (stuck at 68px), the bound would still end exactly at the window bottom.

**Action row.** `overflow-y: auto` makes the panel a scroll container, so its grid automatic minimum width becomes 0. Today the row's min-content widens the track. Spanish labels need about 300px against 268px available at 900 wide, which would add a horizontal scrollbar inside the panel. Wrapping avoids it and changes nothing where the row fits.

## File Changes

| File | Action | Description |
|---|---|---|
| `src/components/OverlayDetailPage.vue` | Modify | Template and CSS below |
| `src/components/PreviewPanel.vue` | Modify | `flex-wrap: wrap` on `.action-row` |
| `src/components/OverlayDetailPage.test.ts` | Create | Spec `[Auto]` scenarios |
| `package.json`, `pnpm-lock.yaml` | Modify | Dev deps |
| `CHANGELOG.md` | Modify | `[Unreleased]` > `### Fixed` entry |

## Interfaces / Contracts

```html
<div class="page-header">
  <div class="page-header-main">
    <div class="page-title">...</div>
    <p v-if="copyError" class="copy-error">...</p>
  </div>
  <button v-if="overlayUrl" class="icon-btn copy-btn" ...><!-- unchanged --></button>
</div>
<div class="detail-layout">
  <PreviewPanel class="preview-column" />
  <ContentPanel />
</div>
```

```css
.page-header { /* existing */ gap: 16px; }
.page-header-main { min-width: 0; }
.page-title, .breadcrumb { overflow-wrap: anywhere; }
.page-id { flex-shrink: 0; }
.copy-btn { flex-shrink: 0; margin-right: 8px; }
.preview-column {
  position: sticky;
  top: 34px;
  max-height: calc(100vh - 68px);
  overflow-y: auto;
}
```

Remove the `.page-subtitle` div, the URL span, and the `.page-subtitle` and `.page-subtitle-url` rules from this file only; other pages scope their own. The script stays unchanged.

## Testing Strategy

Correction to the exploration: `FieldEditor.test.ts` uses `renderToString`, not mounting, and the repo has no DOM environment. SSR cannot click, and the spec's copy scenarios are `[Auto]` with an activation step. Dev deps test the real button wiring without refactoring production code. `// @vitest-environment jsdom` keeps other tests on Node.

Mounting is practical:

- `vi.mock` `./PreviewPanel.vue` and `./ContentPanel.vue` with empty components, avoiding their store and Tauri imports.
- `vi.mock("../stores/instances")` returning `reactive({ activeInstance, activeTemplate, overlayUrl })` with refs.
- `vi.mock("../services/clipboardApi")`, real i18n via `createAppI18n("en")`, fake timers.

| Layer | What | Approach |
|---|---|---|
| Auto | URL absent from `html()`; button name and title | `pnpm test` |
| Auto | Copy success with full URL, label change, 2s reset | Click, `advanceTimersByTimeAsync` |
| Auto | Copy rejection shows `copyFailed`, no copied state | Click |
| Auto | `overlayDetail` keys in `en` and `es` | Import JSON |
| Gate | `pnpm check`, `vue-tsc --noEmit` | CLI |
| Manual | Tauri app at 900x550: long nested path, unbroken long title, sticky scroll, reachable controls, right gap, Spanish action row, short settings, another page's icon buttons | By hand |

`openspec/config.yaml` is stale (`unit_runner: none`); use `pnpm test`.

## Migration / Rollout

No migration required.

## Open Questions

None blocking.
