# Exploration: overlay-detail-layout

## Request

Two corrections to the overlay detail screen:

1. Remove the horizontal scroll caused by the long overlay URL (longer since nested directories). Show only the copy-link button instead of the full URL, and widen the small gap between the copy icon and the right edge.
2. Keep the preview always visible, with vertical scroll affecting only the settings.

## Findings

### Components

- `src/components/OverlayDetailPage.vue` renders the header (URL at line 79, copy button at lines 80-119) and `.detail-layout` (lines 221-226), a grid with `1.1fr 0.9fr` and `align-items: start`.
- The grid contains `PreviewPanel.vue` and `ContentPanel.vue`. `ContentPanel.vue` wraps the shared `FieldEditor.vue` and the presets section.

### Horizontal scroll root cause

- The URL span already has `overflow: hidden`, `text-overflow: ellipsis`, `white-space: nowrap` and `min-width: 0` (`OverlayDetailPage.vue:189-205`).
- The `<div>` at line 73 is a flex item of `.page-header` (line 163) without `min-width: 0`. Its automatic minimum width equals its min-content width, which is the full nowrap URL, so the header and `.content` overflow.
- The URL is the only trigger found on this screen. `FieldEditor` uses `overflow-wrap: anywhere` (line 85). `.detail-page` has `max-width: 980px` and no overflow rule.

### Small gap at the right of the copy icon

- Hypothesis, not verified at runtime: the button is `.icon-btn` (`styles/components.css:130`, 26px wide with a 13px icon) and nothing adds right padding to the header. Once the header is wider than the viewport, the button sits against the clipped edge of `.content` and its scrollbar.
- Fixing the overflow probably fixes most of the gap. Add padding or margin on the button if more room is needed.

### Scroll chain

- `body`: `height: 100vh; overflow: hidden` (`styles/variables.css:37-45`).
- `.app-shell`: flex row, `height: 100vh` (`AppShell.vue:29-33`).
- `main.content`: `flex: 1; overflow-y: auto; padding: 34px 44px 60px` (`AppShell.vue:35-39`). It is the only scroller, so the preview scrolls away with the settings.
- `PreviewPanel.vue` has no overflow rule. Its `.stage` uses `aspect-ratio: 16/9` (line 191), so its height follows its width.

### Copy link behaviour

- A `copied` state lasts 2 seconds and swaps in a check icon and a green colour (lines 83-101, 211-213).
- The button has `title` and `aria-label` from `overlayDetail.copied` / `overlayDetail.copyUrl`. A `copyError` paragraph uses `overlayDetail.copyFailed`.
- The i18n keys exist in both locales (`en.json:84-88`, `es.json:84`).
- Removing the visible URL removes the only place the URL is displayed on this screen.

### Blast radius

- `OverlayDetailPage` is used only in `App.vue:64`. `PreviewPanel` and `ContentPanel` are used only by the detail page.
- `FieldEditor` is also used by the phone page (`src/remote/InstanceCard.vue:74`): do not touch it.
- `.icon-btn` is global and `.content` is shared by every page: do not change them globally.

### Specs

No spec in `openspec/specs/` covers the detail screen. `overlay-catalog-browsing` covers the explorer only. A small new capability is the natural fit.

## Approaches for the pinned preview

| Approach | Pros | Cons | Effort |
| --- | --- | --- | --- |
| A. `position: sticky; top: 0` on the preview column inside the existing `.content` scroller | Minimal change, no height maths, scrollbar stays on the right edge | The page still scrolls as a whole, so the header scrolls away. A preview taller than the viewport is clipped unless bounded with `max-height` | Low |
| B. Page-level flex column of `height: 100%`, `.content` not scrolling on this page, settings column with its own `overflow-y: auto; min-height: 0` | Only the settings scroll, literally. The header stays pinned too | Needs a per-page override of the shared `.content` and a height chain. Padding and breadcrumb consume vertical space | Medium |

## Testing

- Layout is not unit-testable in vitest (jsdom does no layout).
- A mount test can assert the URL text is absent and the button keeps its `aria-label` and `title`. `FieldEditor.test.ts` shows mount tests are possible.
- Real gates: `pnpm check`, `vue-tsc --noEmit`, and a manual check at small windows with a long nested path.

## Risks

- Sticky and `height` behaviour in the Tauri webview (WebKitGTK) is unverified.
- Short windows with a tall preview panel.
- The right gap may depend on the `.content` scrollbar width.
