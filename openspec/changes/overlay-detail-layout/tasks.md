# Tasks: Overlay detail layout

## Review Workload Forecast

| Item | Estimate |
|---|---|
| Estimated authored changed lines (excluding `pnpm-lock.yaml`) | about 150-180 (test about 110-130, `OverlayDetailPage.vue` about 35, `package.json` 2, `PreviewPanel.vue` 1, `CHANGELOG.md` 3) |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Decision needed before apply | No (dev dependencies already approved by the user) |

Delivery strategy: `ask-on-risk`. Single PR; one work-unit commit per phase (tests and docs alongside behavior).

Verification legend: [Auto] = vitest, [Manual] = by hand in the Tauri app at 900x550. Manual items stay unchecked until a person runs them.

## Phase 1: Test infrastructure

- [x] 1.1 Add `jsdom` and `@vue/test-utils` as dev dependencies (`pnpm add -D jsdom @vue/test-utils`); updates `package.json` and `pnpm-lock.yaml`. Environment is enabled per file only, so existing tests stay on Node. Run `pnpm test` to confirm the existing suite is unaffected. (Design: Test environment)

## Phase 2: Tests first (RED)

- [ ] 2.1 Create `src/components/OverlayDetailPage.test.ts` with `// @vitest-environment jsdom`. Set up mocks: `./PreviewPanel.vue` and `./ContentPanel.vue` as empty components, `../stores/instances` returning `reactive({ activeInstance, activeTemplate, overlayUrl })`, `../services/clipboardApi`, real i18n via `createAppI18n("en")`, fake timers. (Design: Testing Strategy)
- [ ] 2.2 Add test: URL string absent from `html()`, and a button with the `overlayDetail.copyUrl` name and title is present. Run `pnpm test` and observe RED. (Spec: URL text absent [Auto])
- [ ] 2.3 Add test: clipboard resolves, click writes the full URL, label becomes `overlayDetail.copied`, and after `advanceTimersByTimeAsync(2000)` it returns to default. (Spec: Copy succeeds [Auto]) This may already pass, since the script is unchanged; record the observed result honestly.
- [ ] 2.4 Add test: clipboard rejects, click shows `overlayDetail.copyFailed` and no copied state. (Spec: Copy fails [Auto]) Same note as 2.3.
- [ ] 2.5 Add test: `overlayDetail` key set is identical in `en` and `es` and matches the pre-change set (import both JSON files). (Spec: Locales unchanged [Auto])

## Phase 3: Implementation (GREEN)

- [ ] 3.1 `OverlayDetailPage.vue` template: remove the `.page-subtitle` div and URL span; move the copy button to be a direct `.page-header` child; keep `copyError` under the title inside `.page-header-main`; give `<PreviewPanel>` `class="preview-column"`. (Spec: Copy-only URL access, Copy button placement)
- [ ] 3.2 `OverlayDetailPage.vue` CSS: `.page-header` `gap: 16px`; `.page-header-main { min-width: 0 }`; `overflow-wrap: anywhere` on `.page-title` and `.breadcrumb`; `.page-id` and `.copy-btn` `flex-shrink: 0`; `.copy-btn { margin-right: 8px }` scoped; `.preview-column` sticky `top: 34px`, `max-height: calc(100vh - 68px)`, `overflow-y: auto`. Remove `.page-subtitle` and `.page-subtitle-url` rules from this file only. (Spec: No horizontal overflow, Pinned preview, Preview usable in short windows, No dead styles)
- [ ] 3.3 `PreviewPanel.vue`: add `flex-wrap: wrap` to `.action-row` so the scroll container does not add a horizontal scrollbar at 900px wide. (Design: Narrow actions)
- [ ] 3.4 Run `pnpm test` and observe GREEN for all Phase 2 tests.

## Phase 4: Documentation

- [ ] 4.1 `CHANGELOG.md`: add an entry under `[Unreleased]` > `### Fixed` describing the detail screen layout (no horizontal overflow, copy-only URL, pinned preview). (Spec: Changelog entry)

## Phase 5: Gates

- [ ] 5.1 `pnpm check` passes (Biome; run `pnpm check:fix` first if formatting is flagged).
- [ ] 5.2 `pnpm exec vue-tsc --noEmit` passes.
- [ ] 5.3 `pnpm test` passes.
- [ ] 5.4 `pnpm build` passes.

## Phase 6: Manual verification (Tauri app, `pnpm tauri dev`, 900x550)

Left unchecked until a person runs them.

- [ ] 6.1 Long nested path and long title: no horizontal scrollbar, header fully visible. (No horizontal overflow)
- [ ] 6.2 Unbroken long title and very long instance label wrap and do not push controls out. (No horizontal overflow)
- [ ] 6.3 Copy button at the right end of the header with a visible gap from the edge. (Copy button placement)
- [ ] 6.4 Another page's icon buttons (for example Settings) look unchanged. (Other screens unaffected)
- [ ] 6.5 Tall settings: scrolling keeps the preview fully visible and settings are not covered; the header scrolls away. (Pinned preview)
- [ ] 6.6 Short settings: page does not scroll and the preview stays in its normal position. (Short settings)
- [ ] 6.7 At 550px height all preview controls are reachable, including with a `sendError` shown; shorter windows scroll inside the preview. (Preview usable in short windows)
- [ ] 6.8 Spanish locale: action row wraps without a horizontal scrollbar inside the preview. (Design: Narrow actions)
- [ ] 6.9 Confirm sticky `top` behaves as designed in WebKitGTK (no 68px stuck offset surprises). (Risk: sticky in WebKitGTK)

## Dependencies

- 1.1 before all of Phase 2. Phase 2 before Phase 3. 3.1-3.3 are independent of each other but touch shared files, so apply sequentially. Phase 4 can run any time after 3.2. Phase 5 after Phases 3 and 4. Phase 6 after 5.4.
