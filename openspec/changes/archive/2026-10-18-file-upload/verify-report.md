# Verify Report: file-upload

Verdict: PASS WITH WARNINGS (0 CRITICAL, 2 WARNING, 2 SUGGESTION)
Commits: 7e7d23c, acc82e8, 5583c0f (base a554fba)

## Commands
- cargo test: 162 passed, 0 failed
- cargo clippy -- -D warnings: clean
- cargo fmt --check --all: clean
- pnpm check (Biome): clean (85 files)
- pnpm exec vue-tsc --noEmit: clean
- pnpm build: not run

## Security (HIGH tier)
- Guard: infrastructure/http/guard.rs:10-32 (127/8, ::1, v4-mapped pass; missing ConnectInfo -> empty 403). Applied via route_layer to /overlay/{*path}, /api/templates, /ws (routes.rs:62-66). No fallback/static route exists outside the guard. Tests cover LAN/fe80/2001:db8/mapped-LAN/missing -> 403.
- Prod serving uses into_make_service_with_connect_info::<SocketAddr>() (http/mod.rs:25-30); WS tests use the same service.
- Traversal: typed name expanded then reduced to last component and sanitized (upload_service.rs:80-98); original name literal basename; dir template must be absolute, no `..`, no NUL at save (config_service.rs:79-89) and absolute re-check after expansion (upload_service.rs:100-118).
- Writes: create_new reservation, temp in same dir, rename commit, Drop cleanup (upload_sink.rs:28-118). 2 GiB streaming counter + DefaultBodyLimit on POST only -> 413 (upload.rs:77-83, 25-33). Disabled check precedes multipart parsing -> 503 (upload.rs:57).
- uploads_enabled default true incl. legacy (domain/config.rs:17, tested). Same UploadSettingsHandle cloned into ConfigService and UploadService (lib.rs:67-80); persisted before handle update.
- Frontend: no invoke in components; es/en have 98 matching keys each; CommandError mapped (errors.upload_dir.invalid).

## Findings
WARNING 1 (RESOLVED 2026-10-18): Manual smoke (pnpm tauri dev, phone/LAN browser) passed, confirmed by the user.
WARNING 2: Name field ignored if sent after the file part (upload.rs:68-73); by design (D-name-before-file) but silent. Upload page must keep name first.
SUGGESTION 1: pnpm build (task 3.8 gate) not re-run here; run before PR.
SUGGESTION 2: Windows 127.0.0.1 shadow-bind risk is documented in tasks only; confirm it landed in README/CHANGELOG (CHANGELOG +3 lines present).

## Spec compliance
lan-file-upload, upload-settings, path-template-variables: all scenarios have matching implementation and tests; all tasks checked and match code.
