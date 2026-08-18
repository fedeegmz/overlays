# AGENTS.md

## What is this

Tauri v2 desktop app (Rust backend + Vue 3 frontend) that serves HTML overlays to OBS via a local HTTP+WebSocket server (`127.0.0.1:4848`, fallback `4849–4851`).

## Key commands

| Task               | Command                                                |
| ------------------ | ------------------------------------------------------ |
| Dev                | `pnpm tauri dev` (starts Vite on :1420 + Rust backend) |
| Frontend typecheck | `vue-tsc --noEmit` (no separate lint configured)       |
| Frontend build     | `pnpm build` (runs typecheck then vite build)          |
| Rust tests         | `cd src-tauri && cargo test`                           |

## Project structure

- `src/` — Vue 3 frontend (TypeScript strict, no JSX, no router lib — single-file components)
- `src-tauri/src/` — Rust backend: Axum server (`server/`), Tauri commands (`commands.rs`), template discovery (`templates.rs`), persistence (`storage.rs`, `config.rs`)
- `examples/` — Overlay templates (lower-third-basico, titulo-centrado). The real runtime dir defaults to `src-tauri/overlays/` but is configurable via Settings.
- `examples/*/overlay.json` — Template manifest (no top-level `id` field — the directory name IS the template id)

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
- No linting tool is configured for frontend or backend. Typecheck is the only gate.
- CSP is disabled (`null`) in `tauri.conf.json` — needed for local WS connections.
- `bundle.resources` is not set in tauri.conf.json — overlay files are NOT bundled in builds yet.
