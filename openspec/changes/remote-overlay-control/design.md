# Design: Remote Overlay Control

## Technical Approach

Approach C in two phases. **Phase 1** moves live instance state into a Rust `InstanceService` (application layer, in memory, behind one `Mutex`). It owns the registry, ordering, numbering, defaults, display names, show/update/hide, `apply_preset` and `start_group`. Every mutation bumps a global `revision` and pushes a full snapshot through the `InstanceEvents` port. The fan-out adapter sends it to Tauri (`AppHandle::emit`) and to a tokio broadcast that feeds the remote WS. Pinia becomes a thin client fed by snapshots. Preview instances (`preview-<id>`) and `activeInstanceId` stay local to the frontend.

**Phase 2** adds `RemoteAccessService` (toggle, pairing, token hashes, revocation) and a `/remote` router behind the `remote_auth` middleware. It also adds the `remote.html` Vite entry, which reuses a shared `FieldEditor`.

## Architecture Decisions

| Topic | Options | Tradeoff | Decision |
|---|---|---|---|
| Event payload | deltas vs full snapshot | Deltas are smaller but can be missed or reordered. There are few instances | **Full snapshot + monotonic `revision`**. Clients drop `revision <= last`; `Lagged` resends the latest |
| Command return | event only vs snapshot in the response | Event-only races the invoke response | Mutating commands **return the snapshot** (plus the id or results). The duplicate event is dropped by revision |
| Echo vs typing | per-keystroke IPC vs draft | Per-keystroke IPC can overwrite text the user is still typing | Desktop keeps a **local draft** per instance and commits after **250 ms** of inactivity through `set_instance_fields`. Snapshot fields never overwrite a dirty draft. Last write wins |
| Source id | none vs `source{kind,client_id}` | Lets the UI recognize its own echoes | Snapshot carries `source: {kind:"desktop"\|"remote", client_id}`. `client_id` is random per window or tab |
| DTO casing | snake vs camel | `src/types/instance.ts` is already camelCase | **camelCase** (`rename_all`) for instance DTOs, IPC and REST |
| Display name | TS vs Rust | The remote needs it too | Port `instanceDefaultName` to the domain; snapshot carries `displayName` |
| start_group rollback | port rollback vs atomic | `OverlayBus::publish` is infallible in process | Compute everything under one lock, then publish. `sendFailed` is removed from `GroupStartResult` because it cannot occur |
| apply_preset lookup | object from client vs `(instance_id, name)` | Remote input is untrusted | Lookup by `name` + the instance's template (the UI already filters by template, `ContentPanel.vue:28`). Missing → `preset.not_found` |
| Remote file fields | client-only read-only vs server-enforced | Never trust the client | When `source=remote`, the server drops `file` type keys and unknown keys |
| Remote mutations | REST vs WS messages | REST reuses the auth and error paths | **REST only**. WS is server→client only; incoming frames are ignored, max 4 KiB |
| Phone token storage | HttpOnly cookie vs localStorage + Bearer | Cookie breaks the cross-origin dev setup and needs CSRF care. The WS must use `?token=` anyway | **localStorage + `Authorization: Bearer`**; WS `?token=` |
| QR content | raw token vs URL + PIN | A raw token in the QR means the desktop must keep the plaintext | QR = `http://<lan>:<port>/remote/#pin=NNNNNN`. The fragment is never sent to the server; the page POSTs the PIN |
| Token model | one shared token vs one token per pairing | A shared token stored only as a hash cannot be handed to a new device after a restart | **One token per pairing**: each successful PIN/QR pairing mints a fresh token and appends its hash. Several devices can be paired at once. Revoke clears every hash. No per-device revoke in v1 |
| Token check | `==` vs constant time | Timing leak | SHA-256 of a 32-byte base64url token, compared with `subtle::ConstantTimeEq` against every stored hash |
| Revocation | re-check per frame vs epoch | Per-frame checks are wasteful | `SessionRevoker` port backed by a `tokio::sync::watch<u64>` epoch. Revoke or disable bumps it and every remote WS closes with code `4001` |
| Static serving | embed crate vs `asset_resolver` | asset_resolver adds no dependency | `StaticAssets` port with a Tauri `asset_resolver` adapter. Vite `base: "./"`, so `/remote/` and `/remote/assets/*` resolve to `remote.html` and `assets/*` |
| Dev API base | Vite proxy vs absolute base | Proxied requests look like loopback, which would bypass auth | **Absolute base** `http://<host>:4848` in DEV, plus a debug-only CORS middleware on `/remote/api` that allows `http://*:1420` |
| FieldEditor file field | prop vs slot | FileField imports Tauri | `#file` **slot**: desktop passes `FileField`, remote renders read-only text. The remote bundle stays free of Tauri imports |

## Module Layout and File Changes

| File | Action | Slice |
|---|---|---|
| `src-tauri/src/domain/instance.rs` | Create: `Instance`, `Registry` (ordering, `next_number`, `display_name`, `defaults_for`) | S1a-1 |
| `src-tauri/src/application/instance_service.rs` | Create: service + tests (fake events, fake template source) | S1a-1 |
| same + `domain/error.rs`, `infrastructure/error.rs` | `apply_preset`, `start_group`; errors `instance.not_found`, `instance.id_taken`, `template.not_found`, `preset.not_found` | S1a-2 |
| `application/ports.rs` | `InstanceEvents`, later `SessionRevoker`, `StaticAssets`, `TokenRng` | S1a-1/S2a |
| `infrastructure/instance_events.rs` | Create: `TauriInstanceEvents`, `BroadcastInstanceEvents`, `FanOut` | S1b |
| `infrastructure/tauri/commands.rs`, `lib.rs`, `http/state.rs` | New commands and wiring; `HttpState` gains `instances`, `presets`, `group_presets`, `remote`, `state_tx`, `assets` | S1b/S2a |
| `src/services/instancesApi.ts`, `src/stores/{instances,presets,groupPresets,templates}.ts` | Thin client; delete TS `startGroup`, `getDefaults` and numbering logic | S1c |
| `vitest.config.ts`, `src/stores/__tests__/*.spec.ts`, `package.json` | vitest | S1c |
| `domain/config.rs`, `application/config_service.rs` | `remote_enabled` (default false), `remote_token_hashes: Vec<String>` (default empty; one SHA-256 hash per pairing, plaintext never persisted) | S2a |
| `domain/remote.rs`, `application/remote_access_service.rs`, `http/remote_auth.rs` | Token hashing, PIN, rate limiter, middleware | S2a |
| `http/remote.rs`, `http/routes.rs` | `/remote` router (REST + state WS), merged separately from `guarded` | S2b |
| `src/components/RemoteSettingsSection.vue`, `services/remoteApi.ts`, i18n | Toggle, URL selector, QR + PIN, revoke | S2c |
| `vite.config.ts`, `remote.html`, `src/components/FieldEditor.vue`, `ContentPanel.vue` | Multi-page, `base:"./"`, extraction | S2d-1 |
| `src/remote/{main.ts,RemoteApp.vue,store.ts}`, `services/transport.ts`, `http/remote.rs` page routes, `infrastructure/static_assets.rs` | Mobile UI + serving | S2d-2 |
| `AGENTS.md` | "Only `/upload` is LAN-open" becomes "`/upload` and `/remote`" | S2b |

## Interfaces / Contracts

```rust
pub trait InstanceEvents: Send + Sync { fn changed(&self, snap: &InstancesSnapshot); }
pub trait SessionRevoker: Send + Sync { fn revoke_all(&self); }
pub trait StaticAssets: Send + Sync { fn get(&self, path: &str) -> Option<(Vec<u8>, String)>; }

#[serde(rename_all = "camelCase")]
pub struct InstanceDto { id, template_id, number: u32, custom_name: Option<String>,
                         display_name: String, fields: HashMap<String,String>, visible: bool }
#[serde(rename_all = "camelCase")]
pub struct InstancesSnapshot { revision: u64, source: Source, instances: Vec<InstanceDto> }
```

**Tauri commands** (all take `client_id`; all return `InstancesSnapshot` unless noted):
`list_instances`, `create_instance(template_id, id?, custom_name?) -> {id, snapshot}`, `remove_instance(id)`, `rename_instance(id, name)`, `move_instance(from, to)`, `set_instance_fields(id, fields)` (state only, no publish), `show_instance(id)`, `update_instance(id)`, `hide_instance(id)`, `apply_preset(id, name)`, `start_group_preset(name) -> {results: GroupStartResult[], snapshot}`. `send_overlay_update` stays for preview only. **Event**: `instances-changed` carrying an `InstancesSnapshot`.
S2 commands: `get_remote_status -> {enabled, pairedDevices, lanUrls}`, `set_remote_enabled(bool)`, `start_pairing -> {pin, expiresAtMs, urls}`, `cancel_pairing`, `revoke_remote_access`.

**REST** (`/remote/api`, JSON, errors are `CommandError`; 403 `remote.disabled`, 401 `remote.unauthorized`, 429 `remote.pairing_rate_limited`, 404 `instance.not_found`):

| Method | Path | Body → Response |
|---|---|---|
| POST | `/pair` (no token) | `{pin}` → `{token}` |
| GET | `/state` | → `InstancesSnapshot` |
| GET | `/templates` | → `Manifest` |
| GET | `/presets?template=` | → `Preset[]` |
| GET | `/groups` | → `[{name, entries}]` |
| POST | `/instances/{id}/show\|update` | `{fields?}` → snapshot (fields merged, file and unknown keys dropped) |
| POST | `/instances/{id}/hide` | → snapshot |
| POST | `/instances/{id}/preset` | `{name}` → snapshot |
| POST | `/groups/{name}/start` | → `{results, snapshot}` |
| GET | `/ws?token=` | server→client `{"type":"state", ...snapshot}`; close `4001` on revoke |

Page: `GET /remote/` and `/remote/assets/{*path}` (needs the toggle on, no token). Request body limit: 64 KiB. Clients send an `X-Client-Id` header.

**Middleware order**: `remote_enabled?` (else 403) → loopback, or Bearer/`?token=` matching a stored hash (else 401). `/pair` skips only the token step. `/overlay`, `/ws` and `/api/templates` keep `loopback_only`. Their tests are extended with "remote enabled + valid token from LAN → still 403".

**Pairing**: one active 6-digit PIN with a 120 s TTL. It is single use, and 5 failures burn it. Each IP gets at most 10 attempts per minute through a `MonotonicClock` port. PINs are compared in constant time.

## Sequence Diagrams

```
Desktop edit
User─►ContentPanel: type ─► store.draft[id] (dirty) ─► 250ms ─► set_instance_fields
InstanceService: rev++ ─► FanOut ─► emit(instances-changed) + state_tx
store: response snapshot applied; draft kept if still dirty, cleared if equal
Update button ─► update_instance(id) ─► bus.publish(update) ─► OBS /ws
Phone WS ◄─ state snapshot (revision n)
```

```
Remote edit
Phone─►POST /instances/{id}/update {fields} ─► remote_auth ─► InstanceService(source=remote)
 ─► filter file/unknown keys ─► rev++ ─► bus.publish ─► OBS
 ─► FanOut ─► desktop event (applied unless draft dirty) + phone WS
Phone ◄─ 200 snapshot
```

```
Group start
caller ─► start_group(name) ─► GroupPresetService.find ─► lock registry
 per entry: template? ─► match live id+template / free id / conflict→fresh id
           fields = defaults ⊕ entry.fields; custom_name rule unchanged; claimed.insert
 unlock ─► publish show per ok entry ─► one rev++ snapshot ─► FanOut
 ─► {results, snapshot}   (active selection untouched: frontend-local)
```

```
Pairing
Desktop: start_pairing ─► PIN + QR(url#pin)
Phone scans ─► /remote/ (page) ─► reads #pin ─► POST /pair {pin}
RemoteAccessService: rate limit ─► ct-compare ─► mint fresh token ─► append sha256 hash to remote_token_hashes ─► burn PIN
Phone ◄─ {token} ─► localStorage ─► GET /state + WS ?token=
```

```
Rotation / revoke
Desktop: revoke_remote_access ─► clear ALL hashes, persist ─► SessionRevoker.revoke_all (epoch++)
remote WS tasks: epoch_rx.changed() ─► Close(4001) ─► phone: drop token, show "pair again"
next REST with any old token ─► 401
set_remote_enabled(false) ─► same revoke_all (hashes kept; only revoke clears them)
set_remote_enabled(true)  ─► previously paired tokens authenticate again
```

## Testing Strategy

| Slice | What | Approach |
|---|---|---|
| S1a-1 | create (id taken), remove, rename trim, move bounds, numbering gaps, defaults (progress `min`, boolean `"false"`), display name, revision++ and event per mutation | Strict TDD, fake `InstanceEvents` + fake `TemplateSource` |
| S1a-2 | start_group parity matrix (no id, free id, live same template, other template, claimed, missing template, saved name vs live name); apply_preset template scoping | Test table derived from `groupPresets.ts:67-141` |
| S1b | Command → service mapping, FanOut | Unit test on the FanOut adapter; manual `pnpm tauri dev` |
| S1c | Revision guard, draft preserved, debounce commit, active id after remove | vitest + `vi.mock(instancesApi)` + fake timers |
| S2a | Middleware matrix: disabled, no token, wrong token, valid token, loopback (on and off); PIN TTL, burn, rate limit; one hash appended per pairing, two tokens valid concurrently, revoke clears all, disable keeps hashes; legacy config defaults | `oneshot` + `MockConnectInfo`, fake clock and RNG |
| S2b | Every REST route; file-key filtering; WS snapshot on connect; close 4001 after revoke; loopback routes still 403 from LAN | Real socket with `tokio_tungstenite` |
| S2c/S2d | Types, lint, build; manual phone test | `pnpm build`; in dev, `TAURI_DEV_HOST` |

## Migration / Rollout

No data migration: instances are not persisted, and the new config fields have serde defaults (`remote_enabled=false`). Slices merge as a chained series in the order S1a-1 → S2d-2, and each one can be reverted in reverse order. After S1, desktop and OBS behavior is unchanged. S2 does nothing until the toggle is on, and disabling it closes all remote sessions.

## Open Questions

- [x] **Shared token vs hashed storage — resolved (user decision): one token per pairing.** Each successful PIN/QR pairing mints a fresh random token; config stores a list of SHA-256 hashes compared in constant time. Revoke/rotate clears all hashes at once and closes every remote WS with code `4001`. Several devices may be paired concurrently; no per-device revoke in v1. Plaintext tokens are never persisted.
- [x] Loopback requests to `/remote` also require `remote_enabled` on — resolved: yes (fail closed).
- [x] Token hashes survive disabling remote — resolved: disable closes sessions but keeps hashes, so paired phones reconnect when it is re-enabled; only revoke clears them.
- [x] `GroupStartResult.reason="sendFailed"` — resolved: removed, since `OverlayBus::publish` cannot fail.
