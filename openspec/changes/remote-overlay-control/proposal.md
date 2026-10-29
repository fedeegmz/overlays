# Proposal: Remote Overlay Control

## Intent
Operators can only drive overlays from the desktop window. Live instance state exists only in Pinia (`src/stores/instances.ts`), so nothing else can control it. Goal: let a phone on the LAN show, update, or hide existing instances and apply presets or group presets, safely, without breaking desktop or OBS behavior.

## Scope
### In Scope
- Rust `InstanceService` as the in-memory source of truth (registry, order, defaults/number, show/update/hide, apply_preset, start_group port), with a change-event port feeding Tauri emit and a remote broadcast
- Pinia becomes an event-fed thin client; TS startGroup and defaults logic deleted; minimal vitest for stores
- Global remote toggle (default off), QR/PIN pairing, token rotation, hashed token in config
- `remote_auth` middleware (loopback OR valid token, enabled per request); authenticated `/remote` REST plus state WS
- `remote.html` second Vite entry, shared `FieldEditor`, mobile UI

### Out of Scope
- Remote create/remove/rename/reorder of instances; preset/group CRUD remotely
- Persisting instances across restarts
- Editing file fields remotely (read-only in v1)
- HTTPS/TLS, per-instance remote filter, multi-user roles
- Changing `/overlay`, `/ws`, `/api/templates` (stay loopback-only) or `/upload`

## Capabilities
### New Capabilities
- `instance-registry`: backend-owned live instances, lifecycle, ordering, change events
- `remote-control-access`: toggle, pairing, token rotation, auth middleware
- `remote-control-api`: `/remote` REST and state WS contract
- `remote-control-ui`: mobile page served at `/remote`

### Modified Capabilities
- `group-preset-instance-identity`: load rules run in the backend and are callable remotely; observable behavior unchanged

## Approach
Approach C in two chained phases.

- S1 moves the state to the backend:
  - S1a-1: domain plus service core
  - S1a-2: start_group / apply_preset
  - S1b: commands, emit adapter and wiring
  - S1c: Pinia refactor plus vitest
- S2 adds remote access:
  - S2a: config, RemoteSettingsHandle and remote_auth
  - S2b: REST/WS
  - S2c: Settings UI with QR/PIN/i18n
  - S2d-1: Vite multi-page plus FieldEditor
  - S2d-2: mobile UI plus serving

About 2.4–2.7k lines across about 9 PRs. Strict TDD on the Rust side.

Assumptions:
- Serve via Tauri `asset_resolver` (no embed crate).
- The QR uses the first LAN URL, with a selector when there are several.
- Preview instances and active selection stay frontend-local.
- Last write wins; source/revision ids prevent echo overwriting typing.

## Affected Areas
| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/{domain,application}/` | New/Modified | instance model, service, ports, config fields |
| `src-tauri/src/infrastructure/{http,tauri}/` | Modified | auth, /remote routes, commands, emit |
| `src-tauri/src/lib.rs` | Modified | wiring |
| `src/stores/*`, `src/services/` | Modified | thin client, transport |
| `src/components/` | Modified/New | FieldEditor, Settings, mobile UI |
| `vite.config.ts`, `remote.html`, `package.json` | Modified/New | multi-page, vitest |

## Risks
| Risk | Likelihood | Mitigation |
|------|------------|------------|
| startGroup parity regression | Med | port with tests first (S1a-2) |
| Typing lag or echo overwrite | Med | local draft, debounce, revision ids |
| Token sniffed on plain-HTTP LAN | Med | default off, rotation, hashed storage, PIN rate limit |
| Prod asset serving or dev origin mismatch | Med | asset_resolver; dev API base |
| Multi-NIC or firewall confusion | Low | URL selector, docs |

## Rollback Plan
Each slice is its own PR, revertible in reverse order. S2 is gated by a default-off toggle, so turning it off disables remote access. Reverting S1c and S1b restores Pinia ownership. No data migration: instances aren't persisted, and the added config fields have serde defaults.

## Dependencies
- Crates: rand, sha2, subtle. Frontend: QR library, vitest.

## Success Criteria
- [ ] Desktop and OBS behavior unchanged after S1 (cargo test, vitest, manual)
- [ ] Phone controls existing instances; changes appear on desktop in real time
- [ ] Unauthenticated, wrong-token, or disabled requests are rejected (test matrix)
- [ ] Loopback-only routes stay loopback-only

## Proposal question round
1. Phone view with no desktop instances: **resolved** — show an empty-state message ("No instances are open on the desktop") that refreshes live via the state WS when an instance appears; no manual reload.
2. Token rotation: **resolved** — paired phones are disconnected immediately (open remote WS sessions are closed and later requests with the old token are rejected); they must re-pair via QR/PIN.
3. Paired devices: **resolved** — one token per pairing: each successful PIN/QR pairing issues a fresh token and only its hash is stored. Several devices may be paired at once; rotation (revoke all) clears every hash and disconnects all of them. Per-device revocation is out of scope for v1.
