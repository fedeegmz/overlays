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
- Frontend: `pnpm check && pnpm exec vue-tsc --noEmit` (+ `pnpm exec vitest run` from S1c on)

---

## S1a-1: Instance domain + service core (~350 lines)

- [ ] 1.1 RED: `src-tauri/src/application/instance_service.rs` tests with fake `InstanceEvents` and fake template source: `create_assigns_defaults_from_template`, `create_with_custom_id_and_name`, `create_duplicate_id_returns_instance_id_taken`, `auto_numbering_labels_second_instance_of_same_template`, `display_name_ports_instance_default_name`, `rename_sets_custom_name`, `remove_unknown_returns_instance_not_found`, `move_reorders_and_clamps`, `every_mutation_bumps_revision_monotonically`, `every_mutation_emits_full_snapshot_with_source`, `set_fields_updates_state_only_no_overlay_publish`.
- [ ] 1.2 RED: `show_publishes_unchanged_ws_payload`, `update_publishes_payload`, `hide_publishes_payload` (shape `{instance_id, template, action, fields}`) using a fake `OverlayBus`.
- [ ] 1.3 Create `src-tauri/src/domain/instance.rs`: `Instance`, `InstancesSnapshot {revision, instances, source}`, `EventSource {kind: desktop|remote, client_id}` (camelCase serde).
- [ ] 1.4 Add `InstanceEvents` port (`application/ports.rs`) and `CommandError` codes `instance.not_found`, `instance.id_taken` in `domain/`.
- [ ] 1.5 GREEN: implement `InstanceService` (single `Mutex`, in-memory, revision counter) in `application/instance_service.rs`; port `instanceDefaultName` numbering from `src/stores/instances.ts`; register module in `application/mod.rs`.
- [ ] 1.6 REFACTOR + verify (backend commands above).

## S1a-2: apply_preset + start_group (~380 lines)

Depends on S1a-1.

- [ ] 2.1 RED: `apply_preset` tests: `apply_preset_sets_fields_by_name_and_template`, `apply_preset_template_mismatch_returns_preset_not_found`, `apply_preset_unknown_name_returns_preset_not_found`, `apply_preset_unknown_instance_returns_instance_not_found`.
- [ ] 2.2 RED: `start_group` parity table derived from `src/stores/groupPresets.ts:67-141`: `load_rule_1_matching_id_reuses_instance`, `load_rule_2_...`, `load_rule_3_...`, `load_rule_4_creates_new_with_saved_id_and_name` (4-rule load order), `preserves_saved_display_name`, `id_collision_reports_conflict_per_entry`, `legacy_entry_without_id_creates_new`, `cross_group_overwrite`, `unknown_template_returns_template_not_found`, `start_group_atomic_single_snapshot_single_revision`, `result_has_no_send_failed_reason`.
- [ ] 2.3 Add `CommandError` codes `template.not_found`, `preset.not_found`; define `GroupStartResult` without `sendFailed`.
- [ ] 2.4 GREEN: `apply_preset` and `start_group` computed under one lock, then published (OverlayBus::publish is infallible).
- [ ] 2.5 Verify (backend commands).

## S1b: Events fan-out + Tauri commands (~300 lines)

Depends on S1a-2.

- [ ] 3.1 RED: `src-tauri/src/infrastructure/instance_events.rs` unit tests: `fanout_calls_tauri_and_broadcast`, `broadcast_lagged_subscriber_can_resubscribe_and_get_latest`, `fanout_without_subscribers_does_not_error`.
- [ ] 3.2 GREEN: `TauriInstanceEvents` (AppHandle.emit `instances-changed`), `BroadcastInstanceEvents` (tokio broadcast), `FanOut`.
- [ ] 3.3 Add commands in `infrastructure/tauri/commands.rs` (`client_id` arg; return snapshot): `list_instances`, `create_instance` -> `{id, snapshot}`, `remove_instance`, `rename_instance`, `move_instance`, `set_instance_fields`, `show_instance`, `update_instance`, `hide_instance`, `apply_preset`, `start_group_preset` -> `{results, snapshot}`; keep `send_overlay_update` for preview.
- [ ] 3.4 Wire `InstanceService` into `lib.rs` and `infrastructure/http/state.rs` (share bus/broadcast).
- [ ] 3.5 Manual check via `pnpm tauri dev` (command invocable, event emitted); verify (backend commands).

## S1c: Frontend thin client + vitest (~400 lines)

Depends on S1b.

- [ ] 4.1 Add vitest devDependency + `vitest.config.ts` + `package.json` script; ensure Biome/vue-tsc cover tests.
- [ ] 4.2 RED: `src/stores/instances.test.ts` with `vi.mock('@/services/instancesApi')` and fake timers: `drops_snapshot_with_revision_le_last`, `applies_newer_snapshot`, `invoke_result_snapshot_then_duplicate_event_dropped`, `dirty_draft_not_overwritten_by_snapshot`, `set_fields_debounced_250ms`, `own_client_id_echo_recognized`, `activeInstanceId_and_previews_stay_local`.
- [ ] 4.3 Create `src/services/instancesApi.ts` (invoke wrappers + `instances-changed` listener + per-window `clientId`).
- [ ] 4.4 Refactor `src/stores/instances.ts` to event-fed client with revision guard and local drafts; refactor `src/stores/groupPresets.ts` to call `start_group_preset`; delete TS `startGroup`, `getDefaults`, numbering logic.
- [ ] 4.5 Adjust components consuming removed APIs (`src/components/*`) minimally; map `GroupStartResult` conflicts to existing i18n strings (update `es.json`/`en.json` only if a string is removed or added).
- [ ] 4.6 GREEN + verify (frontend commands incl. `pnpm exec vitest run`); manual desktop parity check against OBS overlay.

## S2a: Remote access core + auth middleware (~400 lines)

Depends on S1b.

- [ ] 5.1 RED: `src-tauri/src/infrastructure/http/remote_auth.rs` tests (oneshot + `MockConnectInfo`): `disabled_returns_403_even_for_loopback`, `loopback_enabled_no_token_allowed`, `lan_enabled_no_token_401`, `lan_bad_token_401`, `lan_good_token_ok`, `two_concurrent_tokens_both_valid`, `revoke_all_invalidates_old_tokens_401`, `revoke_all_closes_ws_with_4001`, `disable_closes_ws_keeps_hashes_and_tokens_work_on_reenable`, `ws_query_token_accepted`, `pair_route_skips_token_check`, `overlay_ws_templates_stay_loopback_only_even_with_valid_token`.
- [ ] 5.2 RED: `application/remote_access_service.rs` tests with fake `MonotonicClock`: `pin_is_6_digits`, `pin_expires_after_120s`, `pin_single_use`, `five_failures_burn_pin`, `rate_limit_10_per_min_per_ip_returns_429_code`, `pairing_mints_distinct_tokens_and_stores_sha256_only`, `plaintext_never_persisted`, `revoke_clears_all_hashes_and_bumps_epoch`.
- [ ] 5.3 Extend config with `remote_enabled` (default false) and `remote_token_hashes: Vec<String>` (serde defaults, no migration); add test `legacy_config_loads_with_defaults`.
- [ ] 5.4 Create `domain/remote.rs`, ports `MonotonicClock`, `SessionRevoker` (tokio `watch<u64>` epoch); add `CommandError` codes `remote.disabled`, `remote.unauthorized`, `remote.pairing_rate_limited`.
- [ ] 5.5 GREEN: `RemoteAccessService` (toggle, PIN pairing, SHA-256 hashes, `subtle` ct_eq against every hash, revoke) and `remote_auth` middleware.
- [ ] 5.6 Add `subtle`, `sha2`, `base64`/`rand` deps as needed in `src-tauri/Cargo.toml`; verify (backend commands).

## S2b: /remote REST + WS routes (~400 lines)

Depends on S2a.

- [ ] 6.1 RED: `infrastructure/http/remote.rs` route tests: `pair_with_valid_pin_returns_token`, `get_state_returns_snapshot`, `get_templates_and_presets_and_groups`, `show_update_hide_emit_overlay_payload`, `update_drops_file_type_and_unknown_keys_when_remote`, `preset_unknown_returns_404_command_error`, `group_start_returns_results`, `no_create_remove_rename_move_routes_404_405`, `body_over_64kib_rejected`, `ws_sends_snapshot_on_connect`, `ws_ignores_incoming_and_limits_4kib`, `ws_closes_4001_on_revoke`, `ws_receives_change_events_with_source_and_revision`.
- [ ] 6.2 GREEN: implement `/remote/api` router (`POST /pair`, `GET /state|/templates|/presets?template=|/groups`, `POST /instances/{id}/show|update|hide|preset`, `POST /groups/{name}/start`, `GET /ws?token=`) with `X-Client-Id` -> `EventSource{remote}`.
- [ ] 6.3 Mount as separate router layered with `remote_auth` in `infrastructure/http/routes.rs`; keep `/overlay`, `/ws`, `/api/templates` loopback_only.
- [ ] 6.4 Docs: update `AGENTS.md` loopback rule (loopback-only routes vs `/remote` behind toggle + token) and project structure notes.
- [ ] 6.5 Verify (backend commands).

## S2c: Desktop remote settings (~300 lines)

Depends on S2a.

- [ ] 7.1 Add commands in `commands.rs`: `get_remote_status` -> `{enabled, pairedDevices, lanUrls}`, `set_remote_enabled`, `start_pairing` -> `{pin, expiresAtMs, urls}`, `cancel_pairing`, `revoke_remote_access`; register in `lib.rs`. RED tests for command-level service calls where covered by S2a fakes (`start_pairing_returns_urls_and_expiry`).
- [ ] 7.2 Create `src/services/remoteApi.ts` and `src/stores/remote.ts` (no direct `invoke` in components); add vitest `remote.test.ts` for store state.
- [ ] 7.3 Create `src/components/RemoteSettingsSection.vue` (toggle, QR of `http://<lan>:<port>/remote/#pin=NNNNNN`, PIN display, URL selector, revoke with confirmation); mount in Settings; add QR dependency only if none present.
- [ ] 7.4 i18n: add all new strings to `src/i18n/locales/es.json` and `en.json` (error codes `remote.*` included).
- [ ] 7.5 Verify (frontend + backend commands); manual: toggle, pair, revoke.

## S2d-1: Vite multipage + shared FieldEditor (~250 lines)

Depends on S1c.

- [ ] 8.1 Update `vite.config.ts`: multipage inputs (`index.html`, `remote.html`), `base: "./"`; create `remote.html` entry stub.
- [ ] 8.2 Create `src/components/FieldEditor.vue` (field rendering + `#file` slot, no Tauri imports); refactor `src/components/ContentPanel.vue` to use it, passing `FileField` in the slot.
- [ ] 8.3 Verify `pnpm build` emits both entries and desktop behavior unchanged (manual); frontend commands.

## S2d-2: Remote phone app + static serving (~450 lines)

Depends on S2b, S2d-1. Forecast may exceed budget; split static serving into its own PR if so.

- [ ] 9.1 RED: `infrastructure/http/static_assets.rs` tests with fake `StaticAssets`: `remote_root_serves_remote_html`, `assets_path_maps_to_assets`, `missing_asset_404`, `page_requires_only_toggle_not_token`, `page_403_when_disabled`.
- [ ] 9.2 GREEN: `StaticAssets` port + Tauri `asset_resolver` adapter; add `/remote/` and `/remote/assets/*` routes; debug-only CORS middleware on `/remote/api` (allow `http://*:1420`).
- [ ] 9.3 Create `src/services/transport.ts` (absolute API base in dev, Bearer from localStorage, WS `?token=`, reconnect + resnapshot).
- [ ] 9.4 Create `src/remote/` (main.ts, App.vue, PairView, ControlView): fragment `#pin` pairing, re-pair on 401, per-instance show/update/hide, preset apply, group start, read-only file fields, empty-state message, echo does not overwrite typing; vitest for revision guard/transport.
- [ ] 9.5 i18n es/en strings for the remote app; verify (all commands); manual on phone: pair, control, revoke -> returns to pairing.
