# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0] - 2026-10-25

### Added

- Language selector (Español / English) on the LAN upload page. The initial language still follows the device (Spanish or English); the visitor's choice is remembered per browser in `localStorage` (falling back to the device language when storage is unavailable) and applied instantly without reloading. Result messages already shown keep the language they were created in.
- Checklists: a new "Checklists" page in the sidebar to create any number of named checklists with ordered items and tick them off while you work through them (handy for repetitive stream setup steps). The list view shows one card per checklist with its "done of total" progress and a completed style; opening one shows checkboxes, the progress counter, a completed state, and an "Uncheck all" button. The editor creates, renames and deletes lists and adds, edits and removes items; items are reordered by dragging them or by focusing one and pressing Alt+↑ / Alt+↓ (reordering is only available while editing). Names are trimmed and must be unique ignoring case, a list needs at least one item, and blank items are rejected (duplicate item texts are allowed); the editor highlights the offending row and shows localized backend errors. Checks are never saved: they live in memory only, so every list starts unchecked each time the app starts, and checks survive editing because items keep a stable id (removed items simply drop out). Lists persist in `checklists.json` in the app data directory; like group presets, a corrupt file is reported as an error instead of being treated as empty, so it is never overwritten silently. New `list_checklists`, `save_checklist`, and `delete_checklist` commands return structured `CommandError`s (`checklist.invalid_id`, `checklist.empty_name`, `checklist.no_items`, `checklist.empty_item`, `checklist.duplicate_name`, `checklist.not_found`, `checklist.load_failed`, `checklist.save_failed`) with localized messages in Spanish and English.

### Changed

- Group presets now remember each instance's ID and user-set name. Saving a group records them (default numbered labels are not stored), and starting it restores the same IDs so OBS browser-source URLs keep working: an instance that is already open with the same ID and overlay is updated in place instead of duplicated (a saved name replaces its name; without one, its current name is kept), a free ID is recreated as-is, and an ID held by a different overlay gets a new ID plus a localized notice to update the OBS URL. Groups saved by earlier versions still load and gain IDs when saved again. A group with a repeated instance ID is rejected with the new `group_preset.duplicate_instance_id` error.

- Spanish strings on the LAN upload page now use voseo, matching the rest of the app.
- Frontend page types are now defined once in `src/types/navigation.ts` (`NavPage` for sidebar pages, `Page` for every rendered page) instead of being repeated as inline unions in `App.vue`, `AppShell.vue`, and `SideNav.vue`. No user-visible change.
- The sidebar's drag-and-drop reorder logic now lives in a shared `useDragReorder` composable (`src/composables/useDragReorder.ts`), also used by the checklist editor, instead of being implemented inline in `SideNav.vue`. Dragging and Alt+↑ / Alt+↓ keep working as before. No user-visible change.

## [0.6.0] - 2026-10-18

### Added

- LAN file upload: other devices on the same network (a phone, another PC) can send files to this computer through a self-contained page at `http://<LAN IP>:<port>/upload` (default port 4848, fallback 4849-4851). The page lets the visitor pick one or more files and optionally type a file name, shows per-file progress, and reports success or errors (too large, uploads disabled, bad request). It follows the browser language (Spanish or English) and shows a notice instead of the form while uploads are off. Files go to `Downloads/Overlays` by default and never overwrite existing files (repeated names become `name (1).ext`); writes are atomic and each file is capped at 2 GiB. New Settings rows control the feature: an on/off switch (on by default, applied immediately without restart), a destination folder template (absolute path; supports `%CCYY %YY %MM %DD %hh %mm %ss %%`, unknown sequences are kept as-is, empty restores the default), and a copyable read-only display of the LAN upload URL. New `set_uploads_enabled` and `set_upload_dir` commands return structured `CommandError`s (`upload_dir.invalid`) with localized messages; `get_server_status` now also returns `lan_urls`.
- Security note: the local server now binds `0.0.0.0` instead of `127.0.0.1` so LAN devices can reach `/upload`. Everything else (`/overlay/*`, `/api/*`, `/ws`) is restricted to loopback peers and answers `403` to any other address, so OBS and the app keep working unchanged.
- Reorder overlay instances in the sidebar: drag a tab and drop it above or below another (an accent line marks the drop position), or focus a tab and press Alt+↑ / Alt+↓ to move it one step. The active instance and custom names are kept. The order lives in memory only, like the instances themselves.
- Group presets: save every open overlay instance (hidden ones included) as a named group and start them all together later. A new "Groups" section on the Home page lists saved groups with Start and Delete actions. Saving snapshots each instance's template and current field values in sidebar order (duplicate templates are kept); saving under an existing name asks for a two-step confirmation and replaces that group, and deletion uses the same two-step confirmation. Starting a group creates and shows a new instance per entry (saved fields merged over the template defaults) and keeps the previously active instance selected; entries whose template no longer exists or whose overlay could not be sent are skipped and reported ("Started X of Y instances") without aborting the rest. Groups persist in `group_presets.json` in the app data directory. Unlike per-template presets, a corrupt groups file is reported as an error instead of being treated as empty, so it is never overwritten silently. New `list_group_presets`, `save_group_preset`, and `delete_group_preset` commands return structured `CommandError`s (empty name, no entries, invalid entry, not found, load/save failure) with localized messages.
- Confirmation dialog when closing the app while overlays are live in OBS: if any instance is currently shown, a native warning asks before quitting (closing stops the local server and takes those overlays off the air). With no live instances the app closes immediately. Requires the `core:window:allow-destroy` capability on the main window.

### Changed

- Because the server now listens on all interfaces, the operating system may show a firewall prompt the first time the app starts; allow it on private networks for LAN uploads to work (OBS on the same machine is unaffected). On Windows, another process already bound to `127.0.0.1:4848` can shadow the `0.0.0.0:4848` bind and receive loopback traffic; close that process or free the port. Downgrading to an older build may drop the new `uploads_enabled` and `upload_dir` settings if that build rewrites `config.json`.
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

[Unreleased]: https://github.com/fedeegmz/overlays/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/fedeegmz/overlays/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/fedeegmz/overlays/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/fedeegmz/overlays/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/fedeegmz/overlays/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/fedeegmz/overlays/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/fedeegmz/overlays/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/fedeegmz/overlays/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/fedeegmz/overlays/releases/tag/v0.1.0
