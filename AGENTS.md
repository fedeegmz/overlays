# AGENTS.md

## What is this

Tauri v2 desktop app (Rust backend + Vue 3 frontend) that serves HTML overlays to OBS via a local HTTP+WebSocket server (port `4848`, fallback `4849–4851`). The server binds all interfaces (`0.0.0.0`); see the network-exposure note below.

## Network exposure

The server listens on every interface (`src-tauri/src/infrastructure/http/mod.rs`), so route access is enforced per route:

- `/overlay/*`, `/ws` and `/api/*` are loopback-only (`http/guard.rs`). A non-loopback peer gets `403` even with a valid remote token.
- `/upload*` is reachable from the LAN (subject to its own toggle).
- `/remote/api/*` is reachable from the LAN only while remote access is enabled, and non-loopback peers also need a token (`http/remote_auth.rs`). While disabled, every `/remote*` request gets `403`, loopback included. Pair with a 6-digit PIN at `POST /remote/api/pair`; each pairing mints its own token, stored only as a SHA-256 hash in `config.json`.
- The phone page (`/remote/`) and its files (`/remote/assets/*`) need only the toggle, not a token (`http/static_assets.rs`). They are read through the `StaticAssets` port, backed in the app by Tauri's `asset_resolver` (`infrastructure/tauri_assets.rs`), so they exist only in a built app (`pnpm tauri build`). In `pnpm tauri dev` Vite serves the phone page on `:1420`; `http/dev_cors.rs` (debug builds only) lets that origin call `/remote/api`.

## Key commands

| Task               | Command                                                |
| ------------------ | ------------------------------------------------------ |
| Dev                | `pnpm tauri dev` (starts Vite on :1420 + Rust backend) |
| Lint + format      | `pnpm check` / auto-fix with `pnpm check:fix` (Biome)  |
| Frontend typecheck | `vue-tsc --noEmit`                                     |
| Frontend build     | `pnpm build` (runs Biome check, typecheck, vite build) |
| Frontend tests     | `pnpm test` (vitest, pure logic in `src/lib/` and colocated store tests `src/stores/*.test.ts` with mocked `src/services/*`)         |
| Rust tests         | `cd src-tauri && cargo test`                           |

## Git hooks (lefthook)

Hooks managed with [lefthook](https://lefthook.dev) (`lefthook.yml`, installed via pnpm postinstall):

- `pre-commit` (parallel): `pnpm check` (front), `cargo fmt --check --all` + `cargo clippy -- -D warnings` (back, only when `.rs` files are staged).
- `pre-push`: `pnpm test` (front) + `cargo test` (back).

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
  - `application/` — services (config, presets, template catalog, instance registry, remote access) and ports
  - `infrastructure/` — Axum HTTP+WS server (`http/`: `routes.rs`, `guard.rs` loopback guard, `remote.rs` + `remote_auth.rs` remote control API and auth, `static_assets.rs` phone page), Tauri commands (`tauri/commands.rs`), JSON persistence (`json_store.rs`), filesystem template source (`fs_template_source.rs`)
- `examples/` — Overlay templates (lower-third-basico, titulo-centrado). The real runtime dir defaults to `src-tauri/overlays/` but is configurable via Settings.
- Overlays dir may contain nested folders (max depth 5; hidden dirs and symlinks skipped). `list_templates` returns flat `templates` (each with `folder`, `""` = root, `/`-separated) plus `folders` (includes empty ones); the UI builds the explorer from these via `src/lib/folderTree.ts` and `stores/overlayExplorer.ts`.
- `examples/*/overlay.json` — Template manifest (no top-level `id` field — the directory name IS the template id)

## Config exposure

`get_config` and the `set_*` config commands return `AppConfigDto`, never `AppConfig`: `remote_token_hashes` must not reach the webview. Remote state goes through `get_remote_status` / `start_pairing` (Settings > Remote control, `stores/remote.ts`).

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

- Vite is multipage (`index.html` desktop, `remote.html` phone page) with `base: "./"`. `FieldEditor.vue` is shared by both and must stay free of Tauri imports; the desktop supplies `FileField` through its `#file` slot.
- First `pnpm tauri dev` compiles Rust from scratch — slow initial build.
- `overlay.json` has no `id` field; the **directory name** is the template identifier.
- Overlay discovery is filesystem-based (no central manifest) — add a template by creating a folder in the overlays dir.
- Biome handles frontend lint + format (`biome.json`); Vue SFC full support is enabled via `html.experimentalFullSupportEnabled` (still experimental upstream). SVG assets are excluded from linting. The Rust backend has no linter — `cargo test` and clippy are the only Rust gates.
- CSP is disabled (`null`) in `tauri.conf.json` — needed for local WS connections.
- `bundle.resources` is not set in tauri.conf.json — overlay files are NOT bundled in builds yet (a CI release workflow builds installers, but they lack the overlay templates).
- UI text goes through i18n (`src/i18n/locales/{es,en}.json`) — add new strings to both locales.
- Only `tauri-plugin-dialog` is in use; don't add plugins without need.
