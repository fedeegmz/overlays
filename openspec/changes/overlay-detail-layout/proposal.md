# Proposal: Overlay detail layout

## Intent

Long overlay URLs (longer since nested folders) make the detail screen scroll horizontally and push the copy button against the right edge. The preview also scrolls away while editing settings, so changes cannot be seen as they are made.

## Scope

### In Scope
- Remove the visible URL; keep only the copy button, with its existing states and error message.
- Add `min-width: 0` to the header's flex item, so a long title or instance label cannot overflow.
- Widen the copy button's right gap, locally to this screen.
- Pin the preview column while the page scrolls.
- Keep the preview usable in short windows.
- Remove CSS left unused; add a `CHANGELOG.md` entry.

### Out of Scope
- `FieldEditor.vue`, global `.icon-btn`, the shared `.content` rule.
- Pinning the header: it scrolls away, as decided.
- Button tooltip changes.
- i18n removal: all `overlayDetail` keys stay used (`copied` is shared with `SettingsPage.vue:187`).

## Capabilities

### New Capabilities
- `overlay-detail-layout`: no horizontal scroll, copy-only URL access, pinned preview, short-window behavior.

### Modified Capabilities
None

## Approach

- Sticky positioning on the preview column inside the existing `.content` scroller (exploration approach A). `.detail-layout` already uses `align-items: start`, which sticky needs.
- Bounded `max-height` with a fallback; design picks the mechanism.
- Gap cause is an unverified hypothesis (overflow pushes the button to the clipped edge), so verify manually before adding spacing.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src/components/OverlayDetailPage.vue` | Modified | Template, scoped CSS |
| `src/components/OverlayDetailPage.test.ts` | New | URL absent, button labels kept |
| `CHANGELOG.md` | Modified | Entry |

Forecast: about 50-80 changed lines. No backend or IPC impact.

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| URL unreadable on this screen (accepted) | Med | Copy still works; revisit if users complain |
| Sticky misbehaves in WebKitGTK | Low | Manual check in the Tauri app |
| Preview taller than window | Low | Estimated panel about 410px against `minHeight: 550`; bound it anyway |
| Gap persists after overflow fix | Med | Local spacing on the button |
| Grid columns overflow at `minWidth: 900` | Low | Manual check; exploration found no other trigger |

## Rollback Plan

Revert the commit. There is no data, config, or API change.

## Dependencies

None

## Success Criteria

- [ ] No horizontal scroll at 900px width with a long nested path.
- [ ] URL text absent; copy still works and shows success and error states.
- [ ] Visible gap between the copy icon and the right edge.
- [ ] Preview stays visible while settings scroll.
- [ ] Preview controls reachable at 550px height.
- [ ] `pnpm check`, `vue-tsc --noEmit`, `pnpm test` pass.

## Proposal question round

Direct questions were not available in this phase; please review:

1. Without the URL, should the button sit at the header's right end? Assumed yes, since the request mentions the right edge. The current markup would otherwise leave it left-aligned under the title.
2. Is losing the readable URL acceptable for checking which instance a link targets? Assumed yes.
