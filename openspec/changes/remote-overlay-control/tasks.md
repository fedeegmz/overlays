# Tasks: Remote Overlay Control

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~3,100-3,400 total (9 slices, ~250-450 each) |
| 400-line budget risk | High (as one PR); Low-Medium per slice |
| Chained PRs recommended | Yes |
| Suggested split | S1a-1 -> S1a-2 -> S1b -> S1c -> S2a -> S2b -> S2c -> S2d-1 -> S2d-2 |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Forecast | Depends on |
|------|------|-----------|----------|------------|
| S1a-1 | Domain + InstanceService CRUD/order/numbering/events port (Rust, TDD) | PR 1 | ~350 | none |
| S1a-2 | apply_preset + start_group + errors (Rust, TDD) | PR 2 | ~380 | S1a-1 |
| S1b | FanOut events, Tauri commands, AppState wiring | PR 3 | ~300 | S1a-2 |
| S1c | Frontend thin client: instancesApi, store refactor, vitest | PR 4 | ~400 | S1b |
| S2a | Remote config, RemoteAccessService, remote_auth (Rust, TDD) | PR 5 | ~400 | S1b |
| S2b | /remote REST + WS router, AGENTS.md rule | PR 6 | ~400 | S2a |
| S2c | Desktop RemoteSettingsSection, remoteApi.ts, i18n | PR 7 | ~300 | S2a (commands in S2b not required) |
| S2d-1 | Vite multipage, remote.html, shared FieldEditor | PR 8 | ~250 | S1c |
| S2d-2 | src/remote/* app, transport, static_assets, page routes | PR 9 | ~450 | S2b, S2d-1 |

Base boundaries (feature-branch-chain if chosen): PR 1 base = tracker; PR N base = PR N-1 branch. With stacked-to-main each merges to main in order. Revert in reverse order. S2 is inert until the toggle is on.

Per-slice verification (all slices):
- Backend: `cd src-tauri && cargo test` and `cargo fmt --check --all && cargo clippy -- -D warnings`
- Frontend: `pnpm check && pnpm exec vue-tsc --noEmit && pnpm test` (vitest already exists; store tests are added from S1c on)

---

## S1a-1: Instance domain + service core (~350 lines)

- [x] 1.1 RED: `src-tauri/src/application/instance_service.rs` tests with fake `InstanceEvents` and a fake `TemplateLookup` returning a fixed `Manifest { templates, folders }`: `create_assigns_defaults_from_template`, `create_unknown_template_returns_template_not_found`, `create_with_custom_id_and_name`, `create_duplicate_id_returns_instance_id_taken`, `auto_numbering_labels_second_instance_of_same_template`, `display_name_ports_instance_default_name`, `rename_sets_custom_name`, `remove_unknown_returns_instance_not_found`, `move_reorders_and_clamps`, `every_mutation_bumps_revision_monotonically`, `every_mutation_emits_full_snapshot_with_source`, `set_fields_updates_state_only_no_overlay_publish`.
- [x] 1.2 RED: `show_publishes_unchanged_ws_payload`, `update_publishes_payload`, `hide_publishes_payload` (shape `{instance_id, template, action, fields}`) using a fake `OverlayBus`.
- [x] 1.3 Create `src-tauri/src/domain/instance.rs`: `Instance`, `InstancesSnapshot {revision, instances, source}`, `EventSource {kind: desktop|remote, client_id}` (camelCase serde).
- [x] 1.4 Add `InstanceEvents` and `TemplateLookup` (`fn manifest(&self) -> DomainResult<Manifest>`) ports in `application/ports.rs`; implement `TemplateLookup` for `TemplateCatalog` (`application/template_catalog.rs`, delegating to `manifest()`); add `CommandError` codes `instance.not_found`, `instance.id_taken`, `template.not_found` in `domain/` and `infrastructure/error.rs`.
- [x] 1.5 GREEN: implement `InstanceService` (single `Mutex`, in-memory, revision counter, `Arc<dyn TemplateLookup>` read once per operation because `TemplateCatalog::manifest()` re-walks the overlays tree on every call) in `application/instance_service.rs`; port `instanceDefaultName` numbering from `src/stores/instances.ts`; register module in `application/mod.rs`.
- [x] 1.6 REFACTOR + verify (backend commands above).

## S1a-2: apply_preset + start_group (~380 lines)

Depends on S1a-1.

- [x] 2.1 RED: `apply_preset` tests (parity with `src/stores/presets.ts:50-67`): `apply_preset_sets_fields_by_name_and_template`, `apply_preset_publishes_show_and_marks_visible_even_when_hidden`, `apply_preset_template_mismatch_returns_preset_not_found`, `apply_preset_unknown_name_returns_preset_not_found`, `apply_preset_unknown_instance_returns_instance_not_found`, `apply_preset_missing_template_returns_template_not_found_and_emits_nothing`.
- [x] 2.2 RED: `start_group` parity table derived from `src/stores/groupPresets.ts:67-141`: `load_rule_1_matching_id_reuses_instance`, `load_rule_2_...`, `load_rule_3_...`, `load_rule_4_creates_new_with_saved_id_and_name` (4-rule load order), `preserves_saved_display_name`, `id_collision_reports_conflict_per_entry`, `legacy_entry_without_id_creates_new`, `cross_group_overwrite`, `unknown_template_returns_template_not_found`, `start_group_atomic_single_snapshot_single_revision`, `start_group_reads_manifest_once` (counting fake `TemplateLookup`), `result_has_no_send_failed_reason`.
- [x] 2.3 Add `CommandError` code `preset.not_found` (`template.not_found` comes from 1.4); define `GroupStartResult` without `sendFailed`.
- [x] 2.4 GREEN: `apply_preset` (set fields, publish `show`, mark visible) and `start_group` computed under one lock against a single manifest read, then published (OverlayBus::publish is infallible).
- [x] 2.6 RED + GREEN: save-from-registry in `InstanceService` (e.g. `save_group_from_instances(name)` delegating to `GroupPresetService::save`), replacing the entry assembly in `src/stores/groupPresets.ts:35-43`. Tests: `save_group_builds_entries_in_registry_order_including_hidden`, `save_group_records_custom_name_only_when_user_set`, `save_group_records_current_fields_and_instance_id`, `save_group_with_no_instances_returns_group_preset_no_entries`.
- [x] 2.5 Verify (backend commands).

## S1b: Events fan-out + Tauri commands (~300 lines)

Depends on S1a-2.

- [x] 3.1 RED: `src-tauri/src/infrastructure/instance_events.rs` unit tests: `fanout_calls_tauri_and_broadcast`, `broadcast_lagged_subscriber_can_resubscribe_and_get_latest`, `fanout_without_subscribers_does_not_error`.
- [x] 3.2 GREEN: `TauriInstanceEvents` (AppHandle.emit `instances-changed`), `BroadcastInstanceEvents` (tokio broadcast), `FanOut`.
- [x] 3.3 Add commands in `infrastructure/tauri/commands.rs` (`client_id` arg; return snapshot): `list_instances`, `create_instance` -> `{id, snapshot}`, `remove_instance`, `rename_instance`, `move_instance`, `set_instance_fields`, `show_instance`, `update_instance`, `hide_instance`, `apply_preset`, `start_group_preset` -> `{results, snapshot}`; keep `send_overlay_update` for preview.
- [x] 3.6 Add Tauri command `save_group_from_instances(name) -> GroupPreset[]` in `infrastructure/tauri/commands.rs` calling the 2.6 service method; register in `lib.rs`; keep the existing `save_group_preset(name, entries)` (`commands.rs:121`). Rust test for the command-to-service mapping where feasible (feasible part: result DTO serialization tests; `State`-based commands are thin delegations and are covered by the manual check).
- [x] 3.4 Wire `InstanceService` into `lib.rs` and `infrastructure/http/state.rs` (share bus/broadcast; pass the same `Arc<TemplateCatalog>` already held by `HttpState.catalog` as its `TemplateLookup`).
- [x] 3.5 Manual check via `pnpm tauri dev` (command invocable, event emitted); verify (backend commands). Backend commands verified (cargo test 261 passed, fmt, clippy); manual check passed, reported by the user.

## S1c: Frontend thin client + vitest (~400 lines)

Depends on S1b.

- [x] 4.1 Confirm the existing vitest setup (`package.json` `"test": "vitest run"`, config from `vite.config.ts`, colocated `*.test.ts`, CI `.github/workflows/ci.yml:46-47`, lefthook pre-push) picks up new `src/stores/<name>.test.ts` files; no new config or dependency.
- [x] 4.2 RED: `src/stores/instances.test.ts` with `vi.mock('@/services/instancesApi')` and fake timers: `drops_snapshot_with_revision_le_last`, `applies_newer_snapshot`, `invoke_result_snapshot_then_duplicate_event_dropped`, `dirty_draft_not_overwritten_by_snapshot`, `set_fields_debounced_250ms`, `own_client_id_echo_recognized`, `activeInstanceId_and_previews_stay_local`.
- [x] 4.3 Create `src/services/instancesApi.ts` (invoke wrappers + `instances-changed` listener + per-window `clientId`).
- [x] 4.4 Refactor `src/stores/instances.ts` to event-fed client with revision guard and local drafts (`createInstance` routed via `create_instance`; delete `nextInstanceNumber`); refactor `src/stores/groupPresets.ts` to call `start_group_preset`; delete TS `startGroup`. In `src/stores/templates.ts` remove only `getDefaults` and preserve `templates`, `folders`, `refreshTemplates` and `getById`, which `src/stores/overlayExplorer.ts:13` and other consumers depend on.
- [x] 4.5 Adjust consumers of removed or changed APIs minimally, including `src/stores/overlayExplorer.ts` (must keep working unchanged) and `src/components/OverlaysPage.vue` (`createInstance(id)`, lines 16 and 26-29), plus other `src/components/*`; map `GroupStartResult` conflicts to existing i18n strings (update `es.json`/`en.json` only if a string is removed or added).
- [x] 4.6 GREEN + verify (frontend commands incl. `pnpm test`); Automated: biome, vue-tsc and vitest (17 tests) pass. Manual desktop parity check against OBS overlay and the overlay folder explorer: passed, reported by the user.
- [x] 4.7 RED + GREEN: switch `saveFromInstances` in `src/stores/groupPresets.ts` to the `save_group_from_instances` command (entries no longer assembled in TS); test in colocated `src/stores/groupPresets.test.ts` (`save_calls_backend_with_name_only`).
- [x] 4.8 Docs: update the AGENTS.md "Frontend tests" row (currently "pure logic in `src/lib/`") to mention colocated store tests.

## S2a: Remote access core + auth middleware (~400 lines)

Depends on S1b.

- [x] 5.1 RED: `src-tauri/src/infrastructure/http/remote_auth.rs` tests (oneshot + `MockConnectInfo`): `disabled_returns_403_even_for_loopback`, `loopback_enabled_no_token_allowed`, `lan_enabled_no_token_401`, `lan_bad_token_401`, `lan_good_token_ok`, `two_concurrent_tokens_both_valid`, `revoke_all_invalidates_old_tokens_401`, `revoke_all_closes_ws_with_4001`, `disable_closes_ws_keeps_hashes_and_tokens_work_on_reenable`, `ws_query_token_accepted`, `pair_route_skips_token_check`, `overlay_ws_templates_stay_loopback_only_even_with_valid_token`.
- [x] 5.2 RED: `application/remote_access_service.rs` tests with fake `MonotonicClock`: `pin_is_6_digits`, `pin_expires_after_120s`, `pin_single_use`, `five_failures_burn_pin`, `rate_limit_10_per_min_per_ip_returns_429_code`, `pairing_mints_distinct_tokens_and_stores_sha256_only`, `plaintext_never_persisted`, `revoke_clears_all_hashes_and_bumps_epoch`.
- [x] 5.3 Extend config with `remote_enabled` (default false) and `remote_token_hashes: Vec<String>` (serde defaults, no migration); add test `legacy_config_loads_with_defaults`.
- [x] 5.4 Create `domain/remote.rs`, ports `MonotonicClock`, `SessionRevoker` (tokio `watch<u64>` epoch); add `CommandError` codes `remote.disabled`, `remote.unauthorized`, `remote.pairing_rate_limited`.
- [x] 5.5 GREEN: `RemoteAccessService` (toggle, PIN pairing, SHA-256 hashes, `subtle` ct_eq against every hash, revoke) and `remote_auth` middleware.
- [x] 5.6 Add `subtle`, `sha2`, `base64`/`rand` deps as needed in `src-tauri/Cargo.toml`; verify (backend commands). Verified: cargo test 296 passed, fmt, clippy; pnpm check and pnpm test pass.

## S2b: /remote REST + WS routes (~400 lines)

Depends on S2a.

- [x] 6.1 RED: `infrastructure/http/remote.rs` route tests: `pair_with_valid_pin_returns_token`, `get_state_returns_snapshot`, `get_templates_returns_manifest_with_folders`, `get_presets_and_groups`, `show_update_hide_emit_overlay_payload`, `update_drops_file_type_and_unknown_keys_when_remote`, `instance_unknown_returns_404_instance_not_found`, `preset_unknown_returns_404_preset_not_found`, `group_unknown_returns_404_group_preset_not_found`, `group_start_returns_results`, `group_start_decodes_name_with_spaces_and_slash` (e.g. `Intro%20%2F%20Day%201`), `no_create_remove_rename_move_routes_404_405`, `body_over_64kib_rejected`, `ws_sends_snapshot_on_connect`, `ws_ignores_incoming_and_limits_4kib`, `ws_closes_4001_on_revoke`, `ws_receives_change_events_with_source_and_revision`.
- [x] 6.2 GREEN: implement `/remote/api` router (`POST /pair`, `GET /state|/templates|/presets?template=|/groups`, `POST /instances/{id}/show|update|hide|preset`, `POST /groups/{name}/start`, `GET /ws?token=`) with `X-Client-Id` -> `EventSource{remote}`.
- [x] 6.3 Mount as separate router layered with `remote_auth` in `infrastructure/http/routes.rs`; keep `/overlay`, `/ws`, `/api/templates` loopback_only.
- [x] 6.4 Docs: add a network-exposure note to `AGENTS.md` (the server binds all interfaces; only `/upload*` and, when enabled, `/remote*` are reachable from the LAN; `/overlay`, `/ws`, `/api/*` stay loopback-only) and fix line 5, which wrongly says it serves on `127.0.0.1` (it binds `0.0.0.0`, `src-tauri/src/infrastructure/http/mod.rs:19`); update project structure notes.
- [x] 6.5 Verify (backend commands). Verified: cargo test 324 passed, fmt, clippy; pnpm check and pnpm test pass. Manual phone/GUI checks arrive with S2c/S2d.

## S2c: Desktop remote settings (~300 lines)

Depends on S2a.

- [x] 7.1 Add commands in `commands.rs`: `get_remote_status` -> `{enabled, pairedDevices, lanUrls}`, `set_remote_enabled`, `start_pairing` -> `{pin, expiresAtMs, urls}`, `cancel_pairing`, `revoke_remote_access`; register in `lib.rs`. RED tests for command-level service calls where covered by S2a fakes (`start_pairing_returns_urls_and_expiry`). Also `get_config`/`set_*` now return `AppConfigDto` without `remote_token_hashes` (test `config_dto_never_serializes_remote_token_hashes`); dead_code allowances removed.
- [x] 7.2 Create `src/services/remoteApi.ts` and `src/stores/remote.ts` (no direct `invoke` in components); add colocated `src/stores/remote.test.ts` for store state.
- [x] 7.3 Create `src/components/RemoteSettingsSection.vue` (toggle, QR of `http://<lan>:<port>/remote/#pin=NNNNNN`, PIN display, URL selector, revoke with confirmation); mount in Settings; add QR dependency only if none present (added `qrcode-generator`).
- [x] 7.4 i18n: add all new strings to `src/i18n/locales/es.json` and `en.json` (error codes `remote.*` included).
- [x] 7.5 Verify (frontend + backend commands); manual: toggle, pair, revoke. Automated verified (cargo test 328 passed, fmt, clippy, biome, vue-tsc, vitest 25 passed, pnpm build); manual toggle/pair/revoke check passed, reported by the user.

## S2d-1: Vite multipage + shared FieldEditor (~250 lines)

Depends on S1c.

- [x] 8.1 Update `vite.config.ts`: multipage inputs (`index.html`, `remote.html`), `base: "./"`; create `remote.html` entry stub.
- [x] 8.2 Create `src/components/FieldEditor.vue` (field rendering + `#file` slot, no Tauri imports); refactor `src/components/ContentPanel.vue` to use it, passing `FileField` in the slot.
- [x] 8.3 Verify `pnpm build` emits both entries, `pnpm test` still discovers and passes all tests with the multipage `vite.config.ts`, and desktop behavior is unchanged (manual); frontend commands. Automated verified: `pnpm build` emits `dist/index.html` and `dist/remote.html` (relative asset URLs), vitest 28 passed, biome, vue-tsc, vite dev serves both pages; manual desktop parity check (ContentPanel fields, file picker) passed, reported by the user.

## S2d-2: Remote phone app + static serving (~450 lines)

Depends on S2b, S2d-1. Forecast may exceed budget; split static serving into its own PR if so.

- [x] 9.1 RED: `infrastructure/http/static_assets.rs` tests with fake `StaticAssets`: `remote_root_serves_remote_html`, `assets_path_maps_to_assets`, `missing_asset_404`, `page_requires_only_toggle_not_token`, `page_403_when_disabled` (plus redirect and path-traversal tests).
- [x] 9.2 GREEN: `StaticAssets` port + Tauri `asset_resolver` adapter (treats Tauri's `index.html` fallback as a miss); add `/remote/` and `/remote/assets/*` routes; debug-only CORS middleware on `/remote/api` (allow `http://*:1420`, compiled out of release; `cargo clippy --release` passes).
- [x] 9.3 Create `src/services/transport.ts` (absolute API base in dev, Bearer from localStorage, WS `?token=`, reconnect + resnapshot, `encodeURIComponent` on path segments such as group names).
- [x] 9.4 Create `src/remote/` (main.ts, App.vue, PairView, ControlView): fragment `#pin` pairing, re-pair on 401, per-instance show/update/hide, preset apply, group start, read-only file fields, empty-state message, echo does not overwrite typing; colocated vitest (`*.test.ts`) for revision guard/transport, including group-name encoding.
- [x] 9.5 i18n es/en strings for the remote app (done, `remote.*` and `errors.remote.{unreachable,invalid_pin}`); verify (all commands). Automated verified: cargo test 340 passed, fmt, clippy (debug and `--release`), biome, vue-tsc, vitest 56 passed, pnpm build (remote chunks contain no Tauri code). Manual on phone (pair, control, revoke -> returns to pairing) and a built-app check that `/remote/` is served from the embedded assets: passed, reported by the user.
