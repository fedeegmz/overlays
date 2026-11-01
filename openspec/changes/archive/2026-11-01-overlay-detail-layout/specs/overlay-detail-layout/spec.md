# Delta Spec: overlay-detail-layout (new capability)

## Purpose

Layout and link-copy behaviour of the overlay detail screen: no horizontal scroll from long titles or paths, copy-only access to the OBS URL, a preview that stays visible while settings scroll, and a preview that remains usable in short windows.

Verification legend: [Auto] = automated (vitest mount test); [Manual] = checked by hand in the Tauri app, because jsdom does no layout.

## ADDED Requirements

### Requirement: No horizontal overflow
The detail screen MUST NOT scroll horizontally at any supported window size (minimum width 900px), regardless of how long the overlay title, the instance label or the overlay's nested folder path is. Long header text MUST shrink or wrap within the header instead of widening the page.

#### Scenario: Long nested path [Manual]
- GIVEN an overlay nested several folders deep, with a long path and long title
- WHEN its detail screen is shown in a 900px wide window
- THEN no horizontal scrollbar appears and the whole header is within the visible area

#### Scenario: Long instance label [Manual]
- GIVEN an instance with a very long label
- WHEN its detail screen is shown
- THEN the label does not push any header control outside the visible area

### Requirement: Copy-only URL access
The detail screen MUST NOT render the overlay URL as text. It MUST provide a copy-link button that copies the full URL to the clipboard. The button MUST keep its accessible name and tooltip (`overlayDetail.copyUrl`, or `overlayDetail.copied` while in the copied state), its 2-second copied state with a distinct icon and colour, and its visible error message (`overlayDetail.copyFailed`) when copying fails.

#### Scenario: URL text absent [Auto]
- GIVEN the detail screen is mounted for an instance
- THEN the URL string appears nowhere in the rendered text
- AND a button with the copy-link accessible name and tooltip is present

#### Scenario: Copy succeeds [Auto]
- GIVEN the clipboard write resolves
- WHEN the user activates the copy button
- THEN the full URL is written to the clipboard
- AND the button shows the copied state (label `overlayDetail.copied`)
- AND after 2 seconds it returns to its default state

#### Scenario: Copy fails [Auto]
- GIVEN the clipboard write rejects
- WHEN the user activates the copy button
- THEN the `overlayDetail.copyFailed` message is shown
- AND the button does not show the copied state

### Requirement: Copy button placement
The copy button MUST sit at the right end of the page header, with visible spacing between the icon and the right edge of the content area (not flush against the edge or the scrollbar). This spacing MUST apply to this screen only: other pages and the shared icon-button style MUST NOT change.

#### Scenario: Button at right end with gap [Manual]
- GIVEN the detail screen at any supported width
- THEN the copy button is aligned to the right end of the header
- AND a visible gap separates the icon from the right edge

#### Scenario: Other screens unaffected [Manual]
- GIVEN any other screen that uses icon buttons
- THEN its icon-button spacing is the same as before the change

### Requirement: Pinned preview
While the page scrolls vertically, the preview MUST stay visible at the top of its column. Only the settings column MUST visibly move. The page header MAY scroll out of view. The pinned preview MUST NOT overlap or hide settings content.

#### Scenario: Preview stays while settings scroll [Manual]
- GIVEN an instance whose settings are taller than the window
- WHEN the user scrolls down the page
- THEN the preview remains fully visible in its column
- AND the settings content scrolls past without being covered

#### Scenario: Header scrolls away [Manual]
- GIVEN the same screen
- WHEN the user scrolls down
- THEN the header scrolls out of view while the preview stays

#### Scenario: Short settings [Manual]
- GIVEN settings that fit in the window
- THEN the page does not scroll and the preview stays in its normal position

### Requirement: Preview usable in short windows
At the minimum window height (550px), every control of the preview panel MUST be reachable. The preview MUST NOT be taller than the visible area: when the window is too short to show it whole, its content MUST remain accessible (for example by scrolling within the preview) rather than being clipped.

#### Scenario: Minimum height [Manual]
- GIVEN a 550px high window at 900px width
- WHEN the detail screen is shown
- THEN all preview controls can be reached and used

#### Scenario: Preview taller than window [Manual]
- GIVEN a window shorter than the preview panel
- THEN no part of the preview is permanently unreachable

### Requirement: No dead styles or strings
Styles made unused by this change MUST be removed. All `overlayDetail` i18n keys MUST remain, since each is still used; no string may be added or removed in either locale. The change MUST be recorded in `CHANGELOG.md`.

#### Scenario: Locales unchanged [Auto]
- GIVEN the `en` and `es` locale files
- THEN the `overlayDetail` key set is identical to before the change

#### Scenario: Changelog entry [Manual]
- WHEN the change is reviewed
- THEN `CHANGELOG.md` contains an entry describing it
