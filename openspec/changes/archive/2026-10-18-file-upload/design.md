# Design: LAN File Upload

## Technical Approach

One Axum server, bound to `0.0.0.0` (4848-4851 fallback), serves two route groups: a **loopback-only** group (`/overlay/{*path}`, `/api/*`, `/ws`) behind a `ConnectInfo` middleware, and a **LAN** group (`/upload*`). Pure naming logic lives in `domain/`, upload planning in `application/`, streaming and filesystem work in `infrastructure/`. Runtime settings are shared through a handle (the `OverlaysDirHandle` pattern), so no restart is needed. Exploration decisions 1-10 are final; this design only covers how they are built.

## Architecture Decisions

| # | Topic | Options | Tradeoff | Decision |
|---|-------|---------|----------|----------|
| D1 | Loopback guard | per-handler check / `route_layer(from_fn)` on a sub-router / tower `Service` | per-handler checks are easy to forget; a custom Service is more code | `middleware::from_fn(loopback_only)` as `route_layer` on the guarded sub-router, then `.merge(upload_router)`. Serve with `into_make_service_with_connect_info::<SocketAddr>()`. Reads `req.extensions().get::<ConnectInfo<SocketAddr>>()`. **A missing ConnectInfo is denied (fail closed)**. |
| D2 | Loopback test | `ip.is_loopback()` only | misses `::ffff:127.0.0.1` | V4 `is_loopback()` (whole 127/8); V6 `is_loopback()` (`::1`) OR `to_ipv4_mapped()` is loopback. |
| D3 | Rejected peer status | 404 (hide) / 403 | 404 hides routes but makes debugging confusing | `403 Forbidden`, empty body. |
| D4 | Bind | `[::]` dual-stack / `0.0.0.0` | dual-stack works differently per OS | `0.0.0.0`, same 4848-4851 loop. Webview iframe (`http://localhost:{port}/overlay/...`) and OBS reach it as `127.0.0.1` (`::1` is refused and the client falls back to IPv4, the same as today), so both pass the guard. The app UI uses Tauri IPC, not HTTP. |
| D5 | Clock / local time | own impl / `time` / `chrono` | std has no local timezone; `time` local offset is unreliable on multithreaded Linux | `chrono` direct dep, `default-features=false, features=["clock"]` (0.4.45 is already in the lock). Domain stays chrono-free: the formatter takes `DateParts{year,month,day,hour,minute,second}`, and an application `Clock` port has a `ChronoClock` infra impl. |
| D6 | Sanitizer | reject / strip | rejecting would fail guests over harmless input | **Strip/transform, applied after expansion**: `/ \` and `<>:"|?*` become `_`; control chars (incl. NUL) are removed; `..` runs collapse to `.`; leading dots and trailing dots/spaces are trimmed; Windows reserved stems (CON, PRN, AUX, NUL, COM1-9, LPT1-9, case-insensitive) get a `_` prefix; the result is truncated to 255 bytes on a UTF-8 boundary, keeping the extension. Returns `Option<String>`, `None` when empty. |
| D7 | Name choice | — | — | The typed name is expanded and sanitized. If it is `None`, fall back to the sanitized basename of the original name (taken after the last `/` or `\`). If that is `None` too, use `upload`. If the typed name has no extension, the original extension is appended. |
| D8 | No overwrite plus temp file | write straight to the final file / `hard_link` / reserve and rename | a direct write leaves a partial file visible; `hard_link` fails on FAT/exFAT | **Reserve the final name** with `create_new` (empty placeholder), trying `stem.ext`, `stem (1).ext` … `stem (9999).ext` (domain `candidate_name(base, n)`). Stream into `.{final}.part-{pid}-{nanos}` (`create_new`) in the **same dir**, then `rename` it over our own placeholder (atomic on the same FS, and it replaces on Windows too). |
| D9 | Abort cleanup | explicit cleanup in error branches | dropping the future on client disconnect skips that code | RAII `PendingUpload` whose `Drop` removes the temp file and the placeholder unless `commit()` ran. |
| D10 | Size limit | global limit / per route | a global limit would raise `/api` too | `DefaultBodyLimit::max(2 GiB + 1 MiB)` on the `POST /upload` method route only (it is the multipart envelope limit). A per-file byte counter enforces exactly `2 GiB = 2_147_483_648` → `413`. A multer limit error maps through `MultipartError::status()`. |
| D11 | Enabled switch | read config per request / handle | the handle matches the existing pattern | `UploadSettingsHandle(Arc<RwLock<UploadSettings{enabled, dir_template}>>)`, set at startup from config and updated by `ConfigService` setters. Checked **before** the body is read → `503`. |
| D12 | Default dir | `dirs` crate / Tauri path API | `dirs` is in the lock but would be a new direct dep | `app.path().download_dir()` in `setup`, injected as `Option<PathBuf>` into `UploadService`. `<downloads>/Overlays` is joined and `create_dir_all`'d **at request time** (no folder unless used). `None` → `500 upload.destination_unavailable`. |
| D13 | Dest template | — | — | A trusted config string (`Option<String>`, not `PathBuf`, so `%` survives). Expanded, then it must be absolute. `set_upload_dir` rejects non-absolute → `upload_dir.invalid`. Empty → `None`. |
| D14 | Page language | English only / app language / browser | the app language is the operator's choice, not the guest's | **Browser language**: an inline JS dictionary EN/ES, picking `es` when `navigator.language` starts with `es`, otherwise English. About 20 lines; the app is bilingual. |
| D15 | Page state | template substitution / status endpoint | an endpoint is testable and works with `include_str!` | `GET /upload` (HTML via `include_str!("upload.html")`), `GET /upload/status` → `{enabled}`, `POST /upload` (one file per request; the `name` text field goes before the `file` field). The page uploads files one by one with XHR progress. |
| D16 | LAN IP | `if-addrs` / `local-ip-address` | `if-addrs` only lists interfaces; `local-ip-address` also gives the default-route IP | `local-ip-address`: `local_ip()` first, then the other IPv4 entries from `list_afinet_netifas()` that are not loopback and not link-local, deduplicated. `ServerStatus` gains `lan_urls: Vec<String>` (`http://{ip}:{port}/upload`). |
| D17 | Config compat | — | — | **Confirmed: no `deny_unknown_fields`** anywhere, so rollback loads fine. `#[derive(Default)]` must be **replaced by a manual `Default`** (otherwise `bool` = false). Note: an older build rewriting config drops the new fields, and `JsonConfigRepository::load` resets to default on any parse error. |

## Data Flow (POST /upload)

```
Browser(LAN)      Router            upload handler        UploadService     PendingUpload(fs)
   |--POST------->| (no guard)        |                      |                  |
   |              |--BodyLimit------->| settings.enabled?    |                  |
   |<----503------|<-- no ------------|                      |                  |
   |              |                   |--plan(orig,name,now)>| expand+sanitize  |
   |              |                   |<--UploadTarget{dir,base}               |
   |              |                   |--reserve(target)---------------------->| create_dir_all, create_new loop, temp
   |              |                   |--chunks (count<=2GiB)----------------->| write
   |              |                   |--commit()----------------------------->| rename temp→final
   |<--201 {name}-|<------------------|   (drop on error/abort → cleanup)      |
```

Guarded routes: `peer → loopback_only → 403 | handler`.

## Interfaces / Contracts

```rust
// domain/path_template.rs
pub struct DateParts { pub year: i32, pub month: u32, pub day: u32, pub hour: u32, pub minute: u32, pub second: u32 }
pub fn expand(template: &str, at: &DateParts) -> String;     // %CCYY %YY %MM %DD %hh %mm %ss %%
// domain/file_name.rs
pub fn sanitize_file_name(raw: &str) -> Option<String>;
pub fn candidate_name(base: &str, n: u32) -> String;          // n=0 → base
// application/upload_service.rs
pub trait Clock: Send + Sync { fn now(&self) -> DateParts; }
impl UploadService { pub fn plan(&self, original: &str, typed: Option<&str>) -> DomainResult<UploadTarget>; }
```

HTTP errors use a `CommandError` JSON body: `upload.disabled` 503, `upload.too_large` 413, `upload.missing_file`/`upload.bad_request` 400, `upload.name_exhausted` 409, `upload.destination_unavailable`/`upload.write_failed` 500. Success: `201 {"file_name": "..."}`.
New DomainError variants map to codes in `infrastructure/error.rs`. IPC: `set_uploads_enabled(enabled) -> AppConfig`, `set_upload_dir(template) -> AppConfig` (error `upload_dir.invalid`). TS: `AppConfig += uploads_enabled: boolean; upload_dir: string | null`; `ServerStatus += lan_urls: string[]`.
i18n (es+en): `settings.uploads.{enabled.label,enabled.description,dir.label,dir.description,dir.placeholder,dir.choose,dir.variablesHint,lanUrl.label,lanUrl.description,lanUrl.unavailable,lanUrl.copy}`, `errors.upload_dir.invalid`.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/Cargo.toml` | Modify | axum `multipart`; `chrono` (clock); `local-ip-address` |
| `src-tauri/src/domain/{path_template,file_name}.rs`, `domain/mod.rs` | Create/Modify | formatter, sanitizer, candidates |
| `src-tauri/src/domain/{config,error}.rs` | Modify | fields, manual Default, variants |
| `src-tauri/src/application/upload_service.rs`, `application/{mod,config_service}.rs` | Create/Modify | plan, Clock port, `UploadSettingsHandle`, setters |
| `src-tauri/src/infrastructure/{clock,upload_sink,lan}.rs` | Create | ChronoClock, PendingUpload, LAN IPs |
| `src-tauri/src/infrastructure/http/{mod,routes,state,guard,upload}.rs` | Create/Modify | bind, connect_info, guard, upload routes |
| `src-tauri/src/infrastructure/http/upload.html` | Create | page |
| `src-tauri/src/infrastructure/{error.rs,tauri/commands.rs}`, `src-tauri/src/lib.rs` | Modify | codes, commands, wiring |
| `src/types/config.ts`, `src/services/{configApi,overlayApi}.ts`, `src/stores/config.ts`, `src/components/SettingsPage.vue`, `src/i18n/locales/{es,en}.json` | Modify | Settings UI |

## Testing Strategy (strict TDD, backend)

| Layer | What | Approach |
|-------|------|----------|
| Unit | expand (every var, `%%`, unknown kept, trailing `%`), sanitizer (separators, `..`, NUL, reserved, empty, 255-byte), candidates | pure tests with fixed `DateParts` |
| Unit | config legacy → enabled=true, missing dir → None; service plan/setters | in-memory repo, fake Clock |
| Integration | guard: 127.0.0.1, ::1, ::ffff:127.0.0.1 pass; 192.168.x, fe80::, ::ffff:10.x → 403; missing ConnectInfo → 403; `/upload` open to LAN | `router.layer(MockConnectInfo(addr)).oneshot` |
| Integration | upload: success, collision → `(1)`, disabled → 503 + empty dir, over-limit → 413 + no leftovers, missing file → 400 | oneshot with a hand-built multipart body in a temp dir; the limit is injectable (a test-only small max) |
| Existing | WS tests | switch to `into_make_service_with_connect_info` |

## Migration / Rollout

No migration: serde defaults keep legacy configs working (uploads ON). Runtime kill switch: toggle OFF.

## Slice Plan (feature-branch chain)

| Slice | Scope | Est. lines |
|-------|-------|-----------|
| 1 | domain formatter/sanitizer/candidates, Clock+ChronoClock, AppConfig fields + manual Default, DomainError variants + tests | ~350 |
| 2 | bind 0.0.0.0, guard, connect_info, existing test updates, UploadService, PendingUpload, `/upload*` handlers (page stub), lib.rs wiring + tests | ~420 |
| 3 | upload.html, ConfigService setters + commands + codes, LAN URLs/ServerStatus, frontend types/service/store/Settings rows, i18n | ~400 |

## Open Questions

- [ ] None blocking. Assumptions to align with spec: D7 (append the original extension), one file per request, 2 GiB = 2^31 bytes.
- [ ] Windows risk: if another process binds `127.0.0.1:4848` specifically, a `0.0.0.0:4848` bind can still succeed and loopback traffic goes to that process. To be documented.
