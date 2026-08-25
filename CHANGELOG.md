# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Biome for frontend linting and formatting, wired into the build gate (`pnpm check` / `pnpm check:fix`).
- Git hooks managed with lefthook: `pre-commit` runs Biome (frontend) plus `cargo fmt --check` and `cargo clippy -- -D warnings` (backend, in parallel), and `pre-push` runs `cargo test`.

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
