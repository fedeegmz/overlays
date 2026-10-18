# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Reorder overlay instances in the sidebar: drag a tab and drop it above or below another (an accent line marks the drop position), or focus a tab and press Alt+↑ / Alt+↓ to move it one step. The active instance and custom names are kept. The order lives in memory only, like the instances themselves.
- Group presets: save every open overlay instance (hidden ones included) as a named group and start them all together later. A new "Groups" section on the Home page lists saved groups with Start and Delete actions. Saving snapshots each instance's template and current field values in sidebar order (duplicate templates are kept); saving under an existing name asks for a two-step confirmation and replaces that group, and deletion uses the same two-step confirmation. Starting a group creates and shows a new instance per entry (saved fields merged over the template defaults) and keeps the previously active instance selected; entries whose template no longer exists or whose overlay could not be sent are skipped and reported ("Started X of Y instances") without aborting the rest. Groups persist in `group_presets.json` in the app data directory. Unlike per-template presets, a corrupt groups file is reported as an error instead of being treated as empty, so it is never overwritten silently. New `list_group_presets`, `save_group_preset`, and `delete_group_preset` commands return structured `CommandError`s (empty name, no entries, invalid entry, not found, load/save failure) with localized messages.
- Confirmation dialog when closing the app while overlays are live in OBS: if any instance is currently shown, a native warning asks before quitting (closing stops the local server and takes those overlays off the air). With no live instances the app closes immediately. Requires the `core:window:allow-destroy` capability on the main window.

### Changed

- Tauri's native drag and drop handling (`dragDropEnabled`) is now disabled on the main window so HTML5 drag and drop works in the webview (required on Windows). Files dropped from the OS onto the window are no longer captured by Tauri; the app did not use them.
- Closing an overlay instance from the sidebar now requires a two-step confirmation, matching preset deletion: the first click on the `×` arms it (highlighted in red and kept visible), and a second click within 2 seconds closes the instance. Clicking the `×` no longer triggers the row's double-click rename.
- Default overlay instance labels (`Template name #n`) now keep a fixed number assigned when the instance is created (the lowest free number for that template) instead of being renumbered by their position in the list. Closing an instance frees its number for the next one created from the same template.
- The "Overlays" tab in the sidebar is now labeled "Inicio" in Spanish and "Home" in English.

## [0.5.0] - 2026-10-09

### Added

- Optional `step` for progress fields in `overlay.json` (defaults to `1`), applied to both the slider and the numeric input. Enables decimal values such as relative percentages (`"step": 0.1`). Fields with a non-positive `step` are discarded at discovery.
- Custom names for overlay instances in the sidebar: double-click a tab (or press F2 while it is focused) to rename it inline. Enter or clicking away saves, Escape cancels, and an empty name restores the default `Template name #n` label. Custom names live in memory only, like the instances themselves, and are lost on restart.

## [0.4.0] - 2026-09-22

### Added

- Copy-to-clipboard button next to the overlay URL in the detail page (works on Linux, Windows, and macOS via `tauri-plugin-clipboard-manager`). The button shows a checkmark confirmation for a couple of seconds after copying, and the URL truncates with an ellipsis while remaining fully reachable for copy.
- File field type for overlay templates: a picker that opens the native file explorer restricted to the configured overlays directory. Defined in `overlay.json` with an optional `accept` list of extensions (without dots, normalized at discovery) that filters the picker. The stored value is the path **relative to the template folder**, so overlays keep referencing files the local HTTP server can serve; files outside the overlays directory are rejected by the backend.

## [0.3.1] - 2026-09-20

### Fixed

- Windows release builds failing on the CI runner: git checked out sources with CRLF line endings, which the Biome format gate (expects LF) reported as errors in every file. Added `.gitattributes` to normalize line endings to LF on every platform.

## [0.3.0] - 2026-09-20

### Added

- App version shown in the Settings page via the Tauri app API (`@tauri-apps/api/app`).
- Biome for frontend linting and formatting, wired into the build gate (`pnpm check` / `pnpm check:fix`).
- Git hooks managed with lefthook: `pre-commit` runs Biome (frontend) plus `cargo fmt --check` and `cargo clippy -- -D warnings` (backend, in parallel), and `pre-push` runs `cargo test`.
- CI workflow (GitHub Actions) running on push and pull requests to `develop` and `main`: frontend job (Biome + `vue-tsc`) and backend job (`cargo fmt`, `clippy`, `test`).
- Progress field type for overlay templates: configurable slider + numeric input in the content panel. Defined in `overlay.json` with required `min`/`max` (fields with missing or inverted bounds are discarded at discovery) and an optional `default` that falls back to `min`. Field values keep traveling as strings over IPC, presets, and WebSocket.
- Boolean field type for overlay templates: a switch in the content panel to activate or deactivate overlay features. Defined in `overlay.json` with an optional `default` (`"true"`/`"false"`, discarded otherwise — unknown defaults make the field fall back to off at instance creation). Values travel as the strings `"true"`/`"false"` over IPC, presets, and WebSocket; overlay scripts read them with `fields.key === "true"`.
- Example overlay template `barra-progreso` demonstrating the progress field.
- Reload button on the Overlays page to re-scan the overlays directory without restarting the app; shows a spinning loader while refreshing.
- Example overlay template `marco-camara`: full-canvas camera frame with transparent background and configurable color, thickness, and corner radius (outer corners stay square while only the interior hole rounds via an SVG overlay).
- `lower-third-basico` demonstrates the boolean field: a "Mostrar subtítulo" switch that shows or hides the subtitle line, on by default.
- Live preview in the overlay detail page: the real overlay HTML is loaded in an embedded preview that reflects configuration changes (typed fields, color picker, progress, applied presets) in real time over the WebSocket bus, without clicking the Update button. A dedicated preview instance ID keeps the live updates from touching the instance shown in OBS. Includes an "Open in window" fallback that renders the overlay as a first-class webview (reliable across Linux, macOS, and Windows).

### Fixed

- Closing the last overlay instance from the sidebar no longer leaves a blank detail page — the app now returns to the overlay list.
- The HTTP manifest test (`templates_endpoint_returns_manifest`) no longer hardcodes a template count from the `examples/` directory — it now validates the manifest shape and non-empty collection, so adding example templates won't break `cargo test`.

## [0.2.0] - 2026-08-24

### Added

- Internationalization (i18n) with Spanish and English support — the UI language can be switched from Settings.
- Color field type with alpha channel support for overlay templates, including a color picker component.
- Custom application logo replacing the default Tauri icons.

### Changed

- Backend restructured into a layered architecture (`domain`, `application`, `infrastructure`).
- Frontend restructured into `services` (IPC wrappers), `stores` (Pinia), and typed modules.
- Tauri commands now return structured errors (`CommandError`) instead of raw strings, with error codes mapped to localized UI messages.

## [0.1.0] - 2026-08-18

### Added

- Initial desktop app built with Tauri v2 (Rust backend + Vue 3 frontend).
- Local HTTP + WebSocket server (Axum) that serves overlay HTML pages and pushes show/update/hide commands to OBS browser sources.
- Overlay instance management: multiple instances per template, preview panel, and live control of visible fields.
- Template catalog discovered from the filesystem — each overlay is a directory with `overlay.json`, `index.html`, `style.css`, and `script.js`.
- Example overlay templates: `lower-third-basico` and `titulo-centrado`.
- Presets for saving and reusing field configurations per template.
- Configurable overlays directory and server port via Settings, persisted as JSON.
- Release workflow (GitHub Actions) building installers for Linux, Windows, and macOS.

[Unreleased]: https://github.com/fedeegmz/overlays/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/fedeegmz/overlays/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/fedeegmz/overlays/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/fedeegmz/overlays/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/fedeegmz/overlays/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/fedeegmz/overlays/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/fedeegmz/overlays/releases/tag/v0.1.0
