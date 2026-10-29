## Exploration: remote-overlay-control

### Current State
- Server binds `0.0.0.0` on 4848–4851 (`src-tauri/src/infrastructure/http/mod.rs:19-24`). `/overlay/*`, `/api/templates` and `/ws` sit behind the fail-closed `loopback_only` layer (`http/routes.rs:66`, `http/guard.rs:21-32`); only `/upload*` is LAN-open (`routes.rs:70`), with a per-request toggle check (`upload.rs:57`, `UploadSettingsHandle`).
- LAN detection already exists: `infrastructure/lan.rs:36`, exposed as `lan_urls` by `get_server_status` (`tauri/commands.rs:24-65`). Persisted toggle precedent: `set_uploads_enabled` (`config_service.rs:53-59`, serde-default pattern at `domain/config.rs:12-22`).
- Broadcast: `BroadcastOverlayBus` (`infrastructure/overlay_bus.rs:34-47`) = tokio broadcast (capacity 128) + last payload per instance; `ws.rs:25,44` replays the snapshot on connect and on `Lagged`, and ignores incoming messages.
- Show/update/hide: Vue `sendOverlayUpdate` -> Tauri `send_overlay_update` (`commands.rs:33`) -> `bus.publish` only.
- Live instance state (id, templateId, customName, number, fields, visible, order) exists ONLY in Pinia (`src/stores/instances.ts:17`), in memory, unknown to the backend. `visible` is set on the frontend after the send (`instances.ts:188-213`).
- Store API surface: state `instances`, `activeInstanceId`, `serverPort`; computed `activeInstance`, `activeTemplate`, `overlayUrl`, `previewInstanceId`, `previewUrl`; functions `instanceDefaultName`, `instanceDisplayName`, `renameInstance`, `createInstance(templateId, {id?, customName?})`, `removeInstance`, `moveInstance`, `selectInstance`, `fieldsFor`, `buildFields`, `showInstance`, `sendPreviewShow`, `openPreviewWindow`, `show`, `update`, `hide`, `toggleVisibility`, `fetchServerPort`.
- Business logic in TS that must move to Rust: `startGroup` restore algorithm (`src/stores/groupPresets.ts:67-141`: live match by id+template, `claimed` set, `idConflict` fresh id, defaults merge, rollback, restore active id), `applyPreset` (`stores/presets.ts:50-67`), `getDefaults` / `nextInstanceNumber` (`stores/templates.ts:26`).
- Other readers/writers: `ContentPanel.vue:39` mutates `activeInstance.fields[key]` directly; `PreviewPanel.vue:25-36` deep-watches fields (200ms debounce) on `preview-<id>`; `SideNav.vue:23-79` (list/rename/select/remove/reorder); `OverlaysPage.vue:23` (create); `App.vue:19-54` (`liveCount`, close confirm); `GroupPresetsSection.vue:27,154`; `OverlayDetailPage.vue:11`; `bootstrap.ts` (`fetchServerPort`).
- Wiring: `lib.rs:29-105` builds repos/services, `HttpState::new(bus, catalog, overlays_dir, upload)` (`http/state.rs:23`), `app.manage(Arc<…>)`, spawns server (`lib.rs:99`), registers commands (`lib.rs:107`). No `app.emit` / `Emitter` usage exists anywhere.
- Frontend: single Vite entry (`vite.config.ts`, `frontendDist = ../dist`). `ColorField`, `BooleanField`, `ProgressField` have no Tauri imports; `FileField.vue` imports `dialogApi`, `assetsApi` and the config store.
- Tests: Rust only. Patterns: service tests with `InMemoryRepo` (`config_service.rs:91+`, `group_preset_service.rs:50+`), `json_store.rs:144+` tempfile round-trip, HTTP tests with `tower::oneshot` + `MockConnectInfo` and real-socket `tokio_tungstenite` (`http/mod.rs:47-259`), `guard.rs:34+`, `HttpState::for_test` (`state.rs:41`). No frontend test runner.

### Affected Areas
- Backend new: `domain/instance.rs`, `application/instance_service.rs`, `InstanceEvents` port in `application/ports.rs` + Tauri emit adapter, `RemoteSettingsHandle`, `http/remote.rs` (REST + state WS), `remote_auth` middleware.
- Backend changed: `lib.rs`, `http/state.rs`, `http/routes.rs`, `http/guard.rs`, `tauri/commands.rs`, `domain/config.rs`, `application/config_service.rs`, `Cargo.toml` (random bytes + hashing crates).
- Frontend changed: `src/stores/{instances,groupPresets,presets,templates}.ts`, `src/services/` (transport abstraction), `ContentPanel.vue` (extract shared `FieldEditor`), `PreviewPanel.vue`, `SideNav.vue`, `OverlaysPage.vue`, `App.vue`, `GroupPresetsSection.vue`, `OverlayDetailPage.vue`, Settings UI, `src/i18n/locales/{es,en}.json`.
- Frontend new: `remote.html` + mobile entry, `vite.config.ts` multi-page input, vitest setup.
- Docs: `AGENTS.md` (the "everything but `/upload` is loopback" rule changes).

### Approaches
1. Per-overlay "remote enabled" flag + hand-written HTML control page. Quick and the upload page proves the pattern, but duplicates field rendering and cannot list or sync instances because state lives in Pinia. Effort: Medium.
2. Serve the full Vue app remotely behind an IPC/HTTP transport abstraction. Maximum reuse, but exposes all admin features (settings, paths, uploads) to the LAN, desktop UI is not mobile-ready, large attack surface. Effort: High.
3. Backend instance registry as source of truth + authenticated `/remote` REST/WS API + mobile page as a second Vite entry sharing field components. Desktop and remote stay in sync via events; small exposed surface. Effort: Medium-High.

### Recommendation
Approach 3, in two phases:
1. `InstanceService` in Rust owns the registry (in memory). Tauri commands + change event; Pinia becomes an event-fed thin client. Port `startGroup`, `applyPreset`, defaults and numbering.
2. `/remote` route group on the existing router behind "loopback OR valid token" middleware (constant-time compare, only while `remote_enabled`). `/overlay`, `/ws`, `/api/templates` stay loopback-only. Global toggle in Settings (default off), QR/PIN pairing, token rotation, token stored hashed. Mobile UI served via Tauri `asset_resolver()`.

Proposed endpoints: `GET /remote/state`, `GET /remote/templates`, `POST /remote/instances/{id}/show|update|hide`, `POST /remote/instances/{id}/preset`, `GET /remote/presets`, `GET /remote/groups`, `POST /remote/groups/{name}/start`, `GET /remote/ws?token=`. Errors reuse the `CommandError` JSON.

### Confirmed Decisions
1. Approach 3 accepted.
2. Remote clients only control existing instances (show/update/hide, apply presets, start group presets). No create/remove/rename/reorder; presets are apply-only remotely.
3. Instances are not persisted across restarts.
4. File fields are read-only on the remote UI in v1.
5. Minimal vitest for the Pinia stores, added in the first frontend slice.

### Defaults (to confirm in proposal)
- Serve `remote.html` via `asset_resolver()` (no embed crate). In dev, the phone opens `http://<lan-ip>:1420/remote.html` via `TAURI_DEV_HOST`, which needs an absolute API base.
- QR shows the first LAN URL with a selector when several interfaces exist.
- Preview instances and active selection stay frontend-local.
- Last-write-wins between desktop and remote; events carry a source/revision id to avoid echo overwriting typing (local draft + debounced commit).

### Risks
- `startGroup` port is the largest parity risk (id conflicts, rollback, defaults merge).
- Event echo overwriting in-progress typing with per-keystroke IPC.
- Middleware bug could expose OBS endpoints on the shared `0.0.0.0` listener; mitigate with route-level tests and a separate `/remote` router.
- Plain HTTP on the LAN: token sniffable on shared Wi-Fi. Default off, rate-limit PIN attempts, rotation.
- Remote WS accepts input today and ignores it; cap message size.
- `bundle.resources` is unset; mobile assets must come from `frontendDist` via the asset resolver.

### Slice Forecast (authored lines)
| Slice | Content | Lines |
|---|---|---|
| S1a-1 | Domain + `InstanceService` core with tests | ~300 |
| S1a-2 | `start_group` + `apply_preset` with tests | ~220 |
| S1b | Tauri commands, emit adapter, `lib.rs` wiring | ~150 |
| S1c | Pinia thin-client refactor + vitest | ~350 |
| S2a | Config, `RemoteSettingsHandle`, `remote_auth` with tests | ~350 |
| S2b | `/remote` REST + state WS with tests | ~350 |
| S2c | Settings UI (QR/PIN, rotate) + i18n | ~250 |
| S2d | Vite multi-page, `FieldEditor` extraction, mobile UI + serving | ~550 |

Total ~2.4–2.7k lines; chained PRs, phase 1 first.

### Ready for Proposal
Yes. All product questions answered.
