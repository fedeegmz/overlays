# Exploration: file-upload

Date: 2026-10-18
Sources (engram): `sdd/file-upload/explore` (#850), `sdd/file-upload/decisions` (#851), `sdd/file-upload/decisions-security` (#852).
Where decisions differ from the original exploration recommendation, the user DECISIONS below take precedence.

## Goal

Let people on the LAN upload files to the machine running the app through a styled web page served by the app. The destination is configured in Settings, supports OBS-style variables, and the feature has an on/off switch (default ON).

## Decisions (authoritative)

1. **Single server bound to `0.0.0.0`.** No second listener. The existing Axum server (ports 4848, fallback 4849-4851) changes its bind from `127.0.0.1` to `0.0.0.0`. (Supersedes the exploration recommendation of a separate upload listener.)
2. **Loopback guard.** `/ws` and `/api/*` are restricted to loopback (`127.0.0.1` / `::1`) using Axum `ConnectInfo`. `/overlay/{*path}` is also loopback-only (OBS runs on the same machine). Only the `/upload` routes are open to the LAN.
3. **Destination directory comes only from config**, with OBS-style variables. The uploader may optionally type a file name; if empty, the original uploaded file name is used. No subfolder choice from the web.
4. **Config**: an "uploads enabled" switch (default ON) and a destination path template.
5. **Never overwrite.** When the target file exists, append an automatic numeric suffix before the extension (`photo.jpg` -> `photo (1).jpg`, `photo (2).jpg`, ...). Use `create_new` to avoid races.
6. **Size limit: fixed 2 GB per file**, not configurable. Enforced with `DefaultBodyLimit` on the upload route only; files stream to disk in chunks.
7. **Disabled switch**: checked per request (no server restart). The upload page still loads and shows an "uploads are disabled" message; upload requests respond `503 Service Unavailable` and write nothing.
8. **Variables in the name field**: the uploader's optional file name is expanded with the same OBS-style formatter as the destination template. Path separators (`/`, `\`) and `..` coming from the name (before or after expansion) are always rejected or stripped.
9. **Supported variables (launch set)**, local time: `%CCYY` (4-digit year), `%YY` (2-digit year), `%MM` (month), `%DD` (day), `%hh` (hour 00-23), `%mm` (minute), `%ss` (second), `%%` (literal `%`). Unknown `%` sequences are left as-is. Locale-dependent names (`%A`, `%B`, ...) are out of scope.
10. **Default destination**: when the destination template is empty/unset, uploads go to `<OS Downloads dir>/Overlays`.

## Current state (findings)

- `src-tauri/src/domain/config.rs`: `AppConfig { overlays_dir: Option<PathBuf>, language: Option<String> }`, derives `Default`; a legacy-config test exists.
- `src-tauri/src/application/config_service.rs`: load-mutate-save pattern.
- `src-tauri/src/infrastructure/http/mod.rs:12-36`: binds `127.0.0.1` only, ports 4848-4851, no restart/shutdown handle.
- `src-tauri/src/infrastructure/http/routes.rs:57-63`: Router with `/overlay/{*path}`, `/api/templates`, `/ws`; `serve_overlay` uses canonicalize + `starts_with` traversal guard.
- `src-tauri/src/infrastructure/http/state.rs`: `HttpState`.
- `src-tauri/src/lib.rs:67-71`: single spawn of `start_server`.
- `src-tauri/src/infrastructure/tauri/commands.rs`: `get_config`, `set_language`, `set_overlays_dir`.
- `src-tauri/Cargo.toml`: axum 0.8 with feature `ws` only; no multipart; no direct chrono. `Cargo.lock` has chrono 0.4.45 and time 0.3.55 transitively only.
- Frontend: `SettingsPage.vue` (settings-row pattern), `stores/config.ts`, `services/configApi.ts`, i18n `es.json`/`en.json`, `AppConfig` type in `src/types`.

## Implications

- **Exposure**: binding `0.0.0.0` would expose `/ws` (overlay control bus), `/api/templates` and `/overlay` to the LAN; the loopback guard (decision 2) mitigates this. Requires serving with `into_make_service_with_connect_info::<SocketAddr>()` and a middleware/layer on the guarded routes. Needs tests with non-loopback peer addresses.
- **Config default**: `AppConfig` derives `Default`, so a plain `bool` defaults to `false`. Use a manual `Default` or a serde default fn returning `true`, keeping legacy configs (without the field) loading as enabled.
- **Toggle**: the server has no shutdown/restart, so the switch must be checked per request (shared handle such as `AtomicBool`, similar to `OverlaysDirHandle`).
- **Upload handling**: add axum feature `multipart` (multer); override `DefaultBodyLimit` (2 MB default); stream field chunks to disk with `tokio::fs`, write to a temp file then rename; use `create_new` where overwrite must be avoided (depends on pending overwrite policy).
- **OBS-style variables** (OBS filename formatting): `%CCYY %YY %MM %DD %hh %mm %ss %a %A %b %B %d %H %I %m %M %p %S %y %Y %z %Z %%`. Mapping to chrono: CCYY->%Y, hh->%H, mm->%M, MM->%m, ss->%S, YY->%y. Options: a small own formatter, or add chrono with `default-features = false` + `clock`.
- **Security**: sanitize the uploaded/typed name (basename only; strip separators and `..`); destination fixed by config; consider optional PIN/token since LAN access has no auth; show LAN IP in Settings (`local-ip-address` or `if-addrs` crate).
- **Upload page**: embed the HTML via `include_str!` (not the overlays dir, which is user-configurable and may be unset).
- **Frontend**: new settings rows (enabled switch, destination template) with strings in both `es.json` and `en.json`; IPC errors via `CommandError`; component -> store -> service layering.

## Out of scope (future improvements)

- Optional PIN/token to authorize uploads.
- Configurable size limit.
- Subfolder selection from the upload page.

## Open questions

None. All scope questions resolved on 2026-10-18. Assumed in scope: Settings shows the LAN upload URL (LAN IP + port) so users know where to connect.

## Risks

- LAN exposure of an unauthenticated upload endpoint writing to disk (mitigated by config-fixed destination, sanitization, size limit).
- Behavior change: the app now listens on all interfaces; loopback guard must be correct for IPv4, IPv6 and IPv4-mapped IPv6 addresses.
- Backward compatibility of existing `config.json` files.

## Recommended next step

`sdd-propose` once the pending open questions are resolved or explicitly deferred to design.
