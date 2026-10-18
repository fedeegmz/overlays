# Tasks: LAN File Upload (`file-upload`)

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~1170 (S1 ~350, S2 ~420, S3 ~400) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (domain) -> PR 2 (server + upload) -> PR 3 (UI + settings) |
| Delivery strategy | ask-on-risk |
| Chain strategy | feature-branch-chain |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: feature-branch-chain
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| S1 | Pure domain + config + Clock | PR 1 | base = tracker branch `feat/file-upload`; tests included |
| S2 | Bind 0.0.0.0, guard, upload backend | PR 2 | base = PR 1 branch; depends on S1 |
| S3 | Page, commands, LAN URLs, Settings UI, i18n | PR 3 | base = PR 2 branch; depends on S2 |

Conventions: backend tasks are strict TDD (RED test first, then GREEN, then refactor). Frontend has no unit runner; its gate is `pnpm check`, `pnpm exec vue-tsc --noEmit`, `pnpm build`. Spec refs: LFU = lan-file-upload, UST = upload-settings, PTV = path-template-variables.

---

## Slice 1: Domain foundation (PR 1, ~350 lines)

- [x] 1.1 Add `chrono` (`default-features=false`, `features=["clock"]`) to `src-tauri/Cargo.toml`. Verify: `cd src-tauri && cargo build`.
- [x] 1.2 `domain/path_template.rs` (+ `domain/mod.rs`): `DateParts`, `expand(template, &DateParts)`. RED first with fixed `DateParts`. Accept: PTV "Supported variables" and "Unknown sequences left as-is" (full timestamp, `%YY`, case sensitivity, `100%%`, `%%CCYY`, `x-%Q-y`, trailing `%`, `%A`, `%CCYYx`).
- [x] 1.3 `domain/file_name.rs`: `sanitize_file_name(&str) -> Option<String>` per D6. RED first. Accept: PTV "Name sanitization" (`../../evil.sh`, `a\b\c.txt`, `C:\Users\x\photo.jpg`, `..`/`.` -> None, NUL/newline removed, reserved stems prefixed `_`, 255-byte UTF-8-safe truncation keeping extension, post-expansion re-sanitize).
- [x] 1.4 `domain/file_name.rs`: `candidate_name(base, n)` (n=0 -> base; `stem (n).ext`; no-ext -> `notes (1)`; multi-dot -> `archive.tar (1).gz`). RED first. Accept: LFU "Never overwrite" naming scenarios.
- [x] 1.5 `domain/error.rs` + `infrastructure/error.rs`: add variants (`UploadDisabled`, `UploadTooLarge`, `UploadMissingFile`, `UploadBadRequest`, `UploadNameExhausted`, `UploadDestinationUnavailable`, `UploadWriteFailed`, `UploadDirInvalid`) mapped to stable codes (`upload.*`, `upload_dir.invalid`). RED: code-mapping unit test. Accept: design "Interfaces" error list.
- [x] 1.6 `domain/config.rs`: add `uploads_enabled: bool` (`serde(default = true)`) and `upload_dir: Option<String>`; replace derived `Default` with manual impl (uploads ON). RED first: legacy JSON (`overlays_dir`+`language` only) -> enabled; empty/absent -> enabled; persisted `false` honored; unknown fields ignored. Accept: UST "Uploads-enabled setting".
- [x] 1.7 `application/upload_service.rs` (new, trait only for now; register in `application/mod.rs`): `Clock` port; `infrastructure/clock.rs`: `ChronoClock` building `DateParts` from `chrono::Local::now()`. Test: `ChronoClock` returns plausible ranges (month 1-12, hour 0-23). Accept: PTV "local time captured once per expansion".
- [x] 1.8 **Slice gate + commit.** Run `cd src-tauri && cargo test && cargo clippy -- -D warnings && cargo fmt --check --all`.
  Commit: `feat(upload): add path template, file name sanitizer and upload config fields`

## Slice 2: Server exposure and upload backend (PR 2, ~420 lines)

- [ ] 2.1 `Cargo.toml`: enable axum `multipart` feature. Verify: `cargo build`.
- [ ] 2.2 `infrastructure/http/guard.rs` (new): `is_loopback(IpAddr)` + `loopback_only` middleware; missing `ConnectInfo` -> 403. RED first, `MockConnectInfo` + `oneshot`. Accept: LFU "Network exposure" (127.0.0.1, ::1, ::ffff:127.0.0.1 pass; 192.168.1.50, fe80::1, 2001:db8::1, ::ffff:192.168.1.50, missing ConnectInfo -> 403, empty body).
- [ ] 2.3 `infrastructure/http/{mod,routes,state}.rs`: bind `0.0.0.0` (same 4848-4851 loop); split router into guarded group (`/overlay/{*path}`, `/api/*`, `/ws` via `route_layer(from_fn(loopback_only))`) and `/upload*` group; serve with `into_make_service_with_connect_info::<SocketAddr>()`. Update existing WS/HTTP tests to the connect-info service. RED: guarded routes 403 from LAN peer, `/upload` reachable from LAN peer. Accept: LFU "Local OBS keeps working", "LAN peer blocked", "Upload routes open to LAN".
- [ ] 2.4 `application/upload_service.rs` + `application/mod.rs`: `UploadSettings`/`UploadSettingsHandle` (`Arc<RwLock<..>>`), `UploadService::plan(original, typed) -> UploadTarget{dir, base}` per D7/D12/D13 (typed -> expand+sanitize -> original basename -> `upload`; append original extension; default `<downloads>/Overlays`; `None` downloads -> `UploadDestinationUnavailable`). RED first with fake `Clock`. Accept: LFU "Optional file name" (all 5 scenarios), PTV "Applies to destination template and name" (incl. `100%%.txt` literal), UST "Destination template setting" scenarios.
- [ ] 2.5 `infrastructure/upload_sink.rs` (new): `PendingUpload` — `create_dir_all`, reserve final name via `create_new` loop (`candidate_name`, up to 9999 -> `UploadNameExhausted`), temp `.{final}.part-{pid}-{nanos}` in same dir, `commit()` rename, RAII `Drop` cleanup of temp + placeholder. RED first in temp dirs. Accept: LFU "Never overwrite" (first/repeated collision, concurrent same-name -> distinct files), "Atomic writes" (drop without commit leaves no files).
- [ ] 2.6 `infrastructure/http/upload.rs` (new) + `upload.html` stub (`include_str!`): `GET /upload`, `GET /upload/status` -> `{enabled}`, `POST /upload` (check enabled before reading body -> 503; `name` field before `file`; stream chunks; per-file counter 2 GiB = 2_147_483_648 -> 413; `DefaultBodyLimit::max(2 GiB + 1 MiB)` on POST only; multer limit error -> 413; missing file -> 400; 201 `{"file_name"}`; `CommandError` JSON body; limit injectable for tests). Accept: LFU "Upload endpoint", "Size limit", "Disabled switch".
- [ ] 2.7 Handler integration tests (`oneshot`, hand-built multipart, temp dir): success with original name; custom name `stage-%hh%mm`; `../../etc/passwd` name stays inside dir; collision -> `(1)`; disabled -> 503 and dir untouched; live toggle -> next POST 201; over-limit -> 413 with no leftovers; missing file -> 400 with nothing written; GET /upload 200 with no overlays dir. Write alongside 2.6 (RED before GREEN). Accept: LFU scenarios above.
- [ ] 2.8 `src-tauri/src/lib.rs`: in `setup`, resolve `app.path().download_dir()` -> `Option<PathBuf>`, build `ChronoClock`, `UploadSettingsHandle` from loaded config, `UploadService`, pass into HTTP state. Verify: `cargo build`; manual `pnpm tauri dev` smoke (curl `/upload` from loopback). Accept: design D11/D12.
- [ ] 2.9 **Slice gate + commit.** Run `cd src-tauri && cargo test && cargo clippy -- -D warnings && cargo fmt --check --all`.
  Commit: `feat(upload): expose server on LAN with loopback guard and upload endpoint`

## Slice 3: Page, settings and UI (PR 3, ~400 lines)

- [ ] 3.1 `application/config_service.rs`: `set_uploads_enabled(bool)` and `set_upload_dir(Option<String>)` (trim, empty -> None, must be absolute, no `..` component, no NUL -> `UploadDirInvalid`, previous value kept); both update `UploadSettingsHandle` and preserve `overlays_dir`/`language`. RED first with in-memory repo. Accept: UST "Template validation", "Toggle takes effect immediately", "Other settings preserved", whitespace-only cleared, `/mnt/%Q` accepted, `/data/%CCYY/%MM` stored verbatim.
- [ ] 3.2 `infrastructure/tauri/commands.rs` + `lib.rs` handler registration: `set_uploads_enabled(enabled) -> AppConfig`, `set_upload_dir(template) -> AppConfig`. Verify: `cargo build` + clippy.
- [ ] 3.3 `Cargo.toml` (`local-ip-address`), `infrastructure/lan.rs` (new), `ServerStatus.lan_urls: Vec<String>`: `local_ip()` first, then non-loopback non-link-local IPv4 from `list_afinet_netifas()`, deduplicated -> `http://{ip}:{port}/upload`. RED: pure filter/format fn test (loopback and link-local dropped, dedup, bound fallback port 4849 used). Accept: UST "LAN upload URL display".
- [ ] 3.4 `infrastructure/http/upload.html`: full self-contained page: file picker, optional name field, XHR progress per file (one request each), success/error messages, disabled notice from `/upload/status` (no usable form), inline ES/EN dictionary chosen by `navigator.language`. Test: `GET /upload` body contains form markers and both dictionaries; manual smoke from a phone/LAN browser. Accept: LFU "Upload page", "Disabled page"; D14.
- [ ] 3.5 `src/types/config.ts`, `src/types/` ServerStatus type, `src/services/{configApi,overlayApi}.ts`, `src/stores/config.ts`: add `uploads_enabled`, `upload_dir`, `lan_urls`; service wrappers for the 2 commands; store actions; no `invoke` in components. Verify: `pnpm exec vue-tsc --noEmit`.
- [ ] 3.6 `src/components/SettingsPage.vue`: three rows in existing settings-row pattern: uploads switch; template input with variables hint + default note and localized `CommandError` display (`errors.upload_dir.invalid`); read-only LAN URL with copy button and "unavailable" state, visible when disabled. Verify: `pnpm check:fix && pnpm check`. Accept: UST "Settings UI rows" and "LAN upload URL display" scenarios.
- [ ] 3.7 `src/i18n/locales/{es,en}.json`: add `settings.uploads.{enabled.label,enabled.description,dir.label,dir.description,dir.placeholder,dir.choose,dir.variablesHint,lanUrl.label,lanUrl.description,lanUrl.unavailable,lanUrl.copy}` and `errors.upload_dir.invalid` in both locales with matching keys. Accept: UST "Localization" (parity, language switch).
- [ ] 3.8 **Slice gate + commit.** Run `cd src-tauri && cargo test && cargo clippy -- -D warnings && cargo fmt --check --all`, then `pnpm check && pnpm exec vue-tsc --noEmit && pnpm build`. Add a CHANGELOG entry if the project keeps one (recent commits show it does).
  Commit: `feat(upload): add upload page, settings rows and LAN URL display`

---

## Dependencies

- 1.2/1.3/1.4 -> 2.4 (plan); 1.5 -> 2.6, 3.1; 1.6 -> 2.4, 3.1; 1.7 -> 2.4, 2.8.
- 2.2 -> 2.3 (router split); 2.4 + 2.5 -> 2.6/2.7; 2.8 needs 2.3-2.6.
- 3.1 -> 3.2 -> 3.5; 3.3 -> 3.5; 3.5 -> 3.6 -> 3.7 (strings can be drafted with 3.6 but land together).
- Parallelizable within a slice: 1.2/1.3/1.5/1.6 (disjoint files); 2.2 with 2.4/2.5; 3.3 with 3.4.

## Known risks (documented, not tasks)

- Windows: another process bound to `127.0.0.1:4848` can shadow the `0.0.0.0:4848` bind; document in README/CHANGELOG during 3.8.
- An older build rewriting `config.json` drops the new fields (rollback caveat, D17).
