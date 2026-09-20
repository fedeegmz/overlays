# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- App version shown in the Settings page via the Tauri app API (`@tauri-apps/api/app`).
- Biome for frontend linting and formatting, wired into the build gate (`pnpm check` / `pnpm check:fix`).
- Git hooks managed with lefthook: `pre-commit` runs Biome (frontend) plus `cargo fmt --check` and `cargo clippy -- -D warnings` (backend, in parallel), and `pre-push` runs `cargo test`.
- CI workflow (GitHub Actions) running on push and pull requests to `develop` and `main`: frontend job (Biome + `vue-tsc`) and backend job (`cargo fmt`, `clippy`, `test`).
- Progress field type for overlay templates: configurable slider + numeric input in the content panel. Defined in `overlay.json` with required `min`/`max` (fields with missing or inverted bounds are discarded at discovery) and an optional `default` that falls back to `min`. Field values keep traveling as strings over IPC, presets, and WebSocket.
- Example overlay template `barra-progreso` demonstrating the progress field.
- Reload button on the Overlays page to re-scan the overlays directory without restarting the app; shows a spinning loader while refreshing.
- Example overlay template `marco-camara`: full-canvas camera frame with transparent background and configurable color, thickness, and corner radius (outer corners stay square while only the interior hole rounds via an SVG overlay).
- Live preview in the overlay detail page: the real overlay HTML is loaded in an embedded preview that reflects configuration changes (typed fields, color picker, progress, applied presets) in real time over the WebSocket bus, without clicking the Update button. A dedicated preview instance ID keeps the live updates from touching the instance shown in OBS. Includes an "Open in window" fallback that renders the overlay as a first-class webview (reliable across Linux, macOS, and Windows).

### Fixed

- Closing the last overlay instance from the sidebar no longer leaves a blank detail page — the app now returns to the overlay list.

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

[Unreleased]: https://github.com/fedeegmz/overlays/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/fedeegmz/overlays/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/fedeegmz/overlays/releases/tag/v0.1.0
