# AGENTS.md

## What is this

Tauri v2 desktop app (Rust backend + Vue 3 frontend) that serves HTML overlays to OBS via a local HTTP+WebSocket server (`127.0.0.1:4848`, fallback `4849–4851`).

## Key commands

| Task               | Command                                                |
| ------------------ | ------------------------------------------------------ |
| Dev                | `pnpm tauri dev` (starts Vite on :1420 + Rust backend) |
| Lint + format      | `pnpm check` / auto-fix with `pnpm check:fix` (Biome)  |
| Frontend typecheck | `vue-tsc --noEmit`                                     |
| Frontend build     | `pnpm build` (runs Biome check, typecheck, vite build) |
| Rust tests         | `cd src-tauri && cargo test`                           |

## Git hooks (lefthook)

Hooks managed with [lefthook](https://lefthook.dev) (`lefthook.yml`, installed via pnpm postinstall):

- `pre-commit` (parallel): `pnpm check` (front), `cargo fmt --check --all` + `cargo clippy -- -D warnings` (back, only when `.rs` files are staged).
- `pre-push`: `cargo test` (back).

After changing `lefthook.yml`, run `pnpm exec lefthook install` to re-sync hooks.

## Project structure

- `src/` — Vue 3 frontend (TypeScript strict, no JSX, no router lib)
  - `src/components/` — single-file components (pages + panels)
  - `src/services/` — Tauri IPC wrappers (one module per domain: templates, presets, config, dialog)
  - `src/stores/` — Pinia stores (templates, instances, presets, config); components never call `invoke` directly
  - `src/types/` — shared TS types incl. `commandError.ts`
  - `src/i18n/` — Spanish and English locales (`locales/es.json`, `locales/en.json`)
- `src-tauri/src/` — Rust backend in layered architecture:
  - `domain/` — core models (template, preset, overlay message, config) and `CommandError`
  - `application/` — services (config, presets, template catalog) and ports
  - `infrastructure/` — Axum HTTP+WS server (`http/`), Tauri commands (`tauri/commands.rs`), JSON persistence (`json_store.rs`), filesystem template source (`fs_template_source.rs`)
- `examples/` — Overlay templates (lower-third-basico, titulo-centrado). The real runtime dir defaults to `src-tauri/overlays/` but is configurable via Settings.
- `examples/*/overlay.json` — Template manifest (no top-level `id` field — the directory name IS the template id)

## IPC error contract

Tauri commands return structured errors (`CommandError`, mirrored by `src/types/commandError.ts`). The UI maps error codes to i18n messages — don't return raw strings from new commands.

## Overlay template contract

Each overlay is a directory with `overlay.json`, `index.html`, `style.css`, `script.js`. The JS must:

- Filter WS messages by `TEMPLATE_ID` (matches dir name) + `instance` query param
- Implement `show(fields)`, `update(fields)`, `hide()` handlers
- Reconnect WS on close (2s backoff)
- Use transparent `body` background (OBS requirement)

WS payload shape: `{ instance_id, template, action: "show"|"update"|"hide", fields }`

## Gotchas

- First `pnpm tauri dev` compiles Rust from scratch — slow initial build.
- `overlay.json` has no `id` field; the **directory name** is the template identifier.
- Overlay discovery is filesystem-based (no central manifest) — add a template by creating a folder in the overlays dir.
- Biome handles frontend lint + format (`biome.json`); Vue SFC full support is enabled via `html.experimentalFullSupportEnabled` (still experimental upstream). SVG assets are excluded from linting. The Rust backend has no linter — `cargo test` and clippy are the only Rust gates.
- CSP is disabled (`null`) in `tauri.conf.json` — needed for local WS connections.
- `bundle.resources` is not set in tauri.conf.json — overlay files are NOT bundled in builds yet (a CI release workflow builds installers, but they lack the overlay templates).
- UI text goes through i18n (`src/i18n/locales/{es,en}.json`) — add new strings to both locales.
- Only `tauri-plugin-dialog` is in use; don't add plugins without need.
