# Proposal: LAN File Upload

## Intent

People on the LAN (crew, guests) have no simple way to send files to the streaming machine. Add a styled upload page served by the app, writing to a config-defined destination, with an on/off switch. Decisions 1-10 in `exploration.md` are final.

## Scope

### In Scope
- Bind the Axum server to `0.0.0.0` (ports 4848-4851); `/ws`, `/api/*`, `/overlay/*` loopback-only (IPv4, IPv6, IPv4-mapped); only `/upload*` open to LAN.
- Embedded (`include_str!`) styled upload page; optional name field; English UI strings (final call in design).
- Streaming multipart upload, fixed 2 GB/file, temp file + rename, never overwrite (`name (1).ext`, `create_new`).
- OBS-style formatter (`%CCYY %YY %MM %DD %hh %mm %ss %%`, local time; unknown left as-is) for destination template and name; separators and `..` stripped/rejected.
- Config: `uploads_enabled` (default ON, legacy configs ON) + destination template; per-request check; disabled -> page shows notice, POST `503`, nothing written.
- Settings: switch, destination template, LAN upload URL; es/en strings; `CommandError` IPC.

### Out of Scope
- PIN/token auth, configurable size limit, subfolder selection, locale-dependent variables.

## Capabilities

### New Capabilities
- `lan-file-upload`: upload page, endpoint, limits, naming, collision, disabled behavior.
- `path-template-variables`: OBS-style variable expansion and name sanitization.
- `upload-settings`: config fields, defaults, Settings UI, LAN URL display.

### Modified Capabilities
- None (no existing specs). New network-exposure rule (loopback guard) is specified inside `lan-file-upload`.

## Approach

Layered: formatter + sanitizer in `domain/`; upload service (resolve dir, unique name, write) behind a port in `application/`; Axum route + loopback middleware via `into_make_service_with_connect_info::<SocketAddr>()` in `infrastructure/http/`. Enabled flag via shared handle (`AtomicBool`, like `OverlaysDirHandle`). Add axum `multipart`; small LAN-IP crate.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/Cargo.toml` | Modified | axum `multipart`, LAN-IP crate |
| `src-tauri/src/domain/config.rs` | Modified | new fields, default-true |
| `src-tauri/src/domain/` (new module) | New | formatter, sanitizer |
| `src-tauri/src/application/` | New/Modified | upload service, config service |
| `src-tauri/src/infrastructure/http/{mod,routes,state}.rs` | Modified | bind, guard, upload routes |
| `src-tauri/src/infrastructure/http/upload.html` | New | embedded page |
| `src-tauri/src/infrastructure/tauri/commands.rs`, `src-tauri/src/lib.rs` | Modified | commands, wiring |
| `src/components/SettingsPage.vue`, `src/stores/config.ts`, `src/services/configApi.ts`, `src/types/` | Modified | settings UI |
| `src/i18n/locales/{es,en}.json` | Modified | strings |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Guard misses an address form, exposing `/ws` | Med | Tests with v4/v6/mapped non-loopback peers |
| Unauthenticated LAN writes | Med | Fixed dir, sanitization, 2 GB cap, switch |
| Path traversal via name/template | Low | Basename-only, reject separators/`..` post-expansion |
| Partial files on failure | Low | Temp file, rename on success, cleanup |
| Legacy config breaks | Low | serde default + legacy test |
| Firewall prompt on `0.0.0.0` | Med | Document behavior |

## Rollback Plan

Revert the change commits; bind returns to `127.0.0.1`, upload routes disappear. Extra config fields are ignored on load, provided `AppConfig` does not use `deny_unknown_fields` (to be confirmed in design). Runtime mitigation: switch OFF (no restart).

## Dependencies

- axum `multipart` feature; one LAN-IP crate (`local-ip-address` or `if-addrs`).

## Success Criteria

- [ ] LAN device uploads a file; it lands in the expanded destination without overwriting.
- [ ] Non-loopback requests to `/ws`, `/api/*`, `/overlay/*` are rejected; OBS still works locally.
- [ ] >2 GB rejected; switch OFF yields `503` and no file.
- [ ] Legacy config loads with uploads enabled.
- [ ] `cargo test`, clippy, `pnpm build` pass.

## Resolved Question

- Destination template empty/unset: uploads go to the default `<OS Downloads dir>/Overlays` (resolved 2026-10-18). Design decides how the Downloads dir is resolved (Tauri path API or `dirs`).

## Line Forecast

~900-1100 authored lines (backend+tests ~650-750, HTML ~150, frontend+i18n ~150). Exceeds 400; expect 3 slices: (1) formatter + config, (2) bind/guard + upload endpoint/page, (3) Settings UI.
