# Verification Report: remote-overlay-control

**Overall verdict: PASS WITH WARNINGS**

Verified at commit `370240f` on `feat/remote-overlay-control`. 0 critical issues, 3 warnings, 3 suggestions. All automated checks pass, all 50 tasks are done, and every security requirement has a passing covering test. The warnings are stale documents and missing UI component tests, not code defects.

## Completeness

50 of 50 tasks are checked and the code backs the main ones. The 5 manual tasks (3.5, 4.6, 7.5, 8.3, 9.5) are reported as passed by the user, who re-tested the phone page after `bcd46fc`. The verifier could not reproduce GUI, OBS or phone checks.

## Execution evidence

| Command | Result |
|---|---|
| `pnpm test` | 60/60 passed, 7 files |
| `pnpm check` | Biome clean, 113 files |
| `pnpm exec vue-tsc --noEmit` | exit 0, no output |
| `pnpm build` | built; `git status -s` still empty afterwards |
| `cargo test` | 341 passed, 0 failed |
| `cargo fmt --check --all` | exit 0 |
| `cargo clippy -- -D warnings` | clean |
| `cargo clippy --release -- -D warnings` | clean |

## Spec compliance matrix

Statuses: COMPLIANT, PARTIAL (behavior implemented, coverage incomplete), MANUAL-USER-REPORTED.

### remote-control-access

| Requirement / scenario | Code | Test | Status |
|---|---|---|---|
| Default off, missing field | `domain/config.rs` | `domain/config.rs` tests | COMPLIANT |
| Disabled rejects valid token (403) | `remote_auth.rs` `remote_enabled` | `disabled_returns_403_even_for_loopback`, `lan_good_token_ok` | COMPLIANT |
| Toggle off at runtime closes WS and rejects | `remote_access_service.rs::set_enabled`, revoker | `disable_closes_ws_keeps_hashes_and_tokens_work_on_reenable`, `disable_revokes_sessions_keeps_hashes_and_reenable_restores_tokens` | COMPLIANT |
| Tokens survive off and on | hashes kept on disable | same two tests | COMPLIANT |
| Successful pairing stores hash | `pair()` | `pairing_mints_distinct_tokens_and_stores_sha256_only`, `pair_with_valid_pin_returns_token` | COMPLIANT |
| Wrong PIN returns `CommandError`, no token | `pair()`, `pin_matches` constant-time | `wrong_pin_issues_nothing`, `five_failures_burn_pin`, `pair_with_wrong_pin_is_401_and_floods_are_429` | COMPLIANT |
| PIN expiry and single use | `PIN_TTL_MS` 120 s, `state.pin=None` | `pin_expires_after_120s`, `pin_single_use` | COMPLIANT |
| Rate limit: correct PIN during cooldown is rejected | `check_rate`, 10 per minute per IP, before the PIN check | `rate_limit_10_per_min_per_ip_returns_429_code` | COMPLIANT |
| Pairing while off, loopback included | `RemoteDisabled`; `/pair` sits behind `remote_enabled` | `disabled_rejects_pairing_and_pin_creation`, `pair_while_disabled_is_403` | COMPLIANT |
| LAN URL selection re-encodes the QR | `RemoteSettingsSection.vue`, `remote.ts` `selectUrl` | `qr_payload_uses_selected_url_with_pin_fragment`; the `<select>` is not tested | MANUAL-USER-REPORTED (7.5) |
| Hash at rest, no plaintext | `hash_token`, 32-byte rand | `plaintext_never_persisted`, `hash_is_sha256_hex` | COMPLIANT |
| Constant-time compare | `matches_any` uses `subtle::ct_eq` over every hash | `matches_any_checks_every_hash`, `lan_bad_token_401` | COMPLIANT |
| Two devices | one token per pairing | `two_concurrent_tokens_both_valid` | COMPLIANT |
| Rotation closes WS with 4001 and old tokens fail | `revoke_all`, `REVOKED_CLOSE_CODE` | `revoke_all_closes_ws_with_4001`, `ws_closes_4001_on_revoke`, `revoke_all_invalidates_old_tokens_401`, `revoke_clears_all_hashes_and_bumps_epoch` | COMPLIANT |
| Re-pair after rotation | `pair()` | `pairing_mints_distinct_tokens_and_stores_sha256_only`, `pair_route_skips_token_check`; no rotate-then-pair test | PARTIAL |
| Loopback needs remote on (403) | `remote_enabled` | `disabled_returns_403_even_for_loopback` | COMPLIANT |
| Loopback allowed when on, without token | `remote_auth` | `loopback_enabled_no_token_allowed` | COMPLIANT |
| Missing or wrong token gives 401 with `CommandError` body | `error_response` | `lan_enabled_no_token_401`, `lan_bad_token_401` | COMPLIANT |
| `/ws`, `/api/templates`, `/overlay` stay loopback-only with a valid token | `guard.rs` | `overlay_ws_templates_stay_loopback_only_even_with_valid_token`, `guarded_routes_reject_lan_and_non_loopback_peers` | COMPLIANT |
| Upload unchanged | `upload.rs` | `upload_routes_are_open_to_lan_peers` and the 18 upload tests | COMPLIANT |

### remote-control-api

| Requirement / scenario | Code | Test | Status |
|---|---|---|---|
| Unauthenticated REST | `secure()` | `control_routes_require_a_token` | COMPLIANT |
| Unauthenticated WS refused | `presented_token`; `?token=` only on `/ws` | `ws_refuses_peers_without_a_valid_token`, `ws_query_token_accepted`, `query_token_is_refused_outside_the_ws_route` | COMPLIANT |
| List instances, presets, groups, templates with folders | `remote.rs` get handlers | `get_state_returns_snapshot`, `get_presets_and_groups`, `get_templates_returns_manifest_with_folders` | COMPLIANT |
| Show, update, hide emit the overlay payload with remote source | `remote.rs`, `InstanceService` | `show_update_hide_emit_overlay_payload`, `client_id_header_becomes_the_remote_source` | COMPLIANT |
| Apply preset | `apply_preset` | `preset_applies_and_unknown_returns_404_preset_not_found` | COMPLIANT |
| Start group | `start_group` | `group_start_returns_results` | COMPLIANT |
| 404 for unknown instance, preset, group | `status_for` | `instance_unknown_returns_404_instance_not_found`, `group_unknown_returns_404_group_preset_not_found` | COMPLIANT |
| Group name with reserved characters | Axum `Path`, `encodeURIComponent` in the client | `group_start_decodes_name_with_spaces_and_slash`, `group_name_with_space_and_slash_is_encoded_in_the_path` | COMPLIANT |
| No create, remove, rename, reorder or preset CRUD | no such routes | `no_create_remove_rename_move_routes_404_405` | COMPLIANT |
| File fields read-only on the server | remote patch drops file and unknown keys | `update_drops_file_type_and_unknown_keys_when_remote`, `remote_patch_drops_file_type_and_unknown_keys` | COMPLIANT |
| WS initial snapshot and change events | `remote.rs::session` | `ws_sends_snapshot_on_connect`, `ws_receives_change_events_with_source_and_revision` | COMPLIANT |
| WS is one-way for state | incoming ignored, 4 KiB limit | `ws_ignores_incoming_and_limits_4kib` | COMPLIANT |
| Instance appears via WS | snapshot broadcast | `ws_receives_change_events_with_source_and_revision`, `empty_registry_shows_empty_state_and_clears_when_an_instance_appears` | COMPLIANT |
| Two phones see each other's source id | source id in the event | `client_id_header_becomes_the_remote_source`, `own_client_id_echo_recognized`; no two-client WS test | PARTIAL |

### lan-file-upload

| Scenario | Test | Status |
|---|---|---|
| Local OBS, IPv6 loopback, mapped loopback allowed | `guarded_routes_allow_loopback_peers`, `loopback_addresses_are_allowed` | COMPLIANT |
| LAN, non-loopback IPv6 and mapped LAN blocked | `guarded_routes_reject_lan_and_non_loopback_peers`, `non_loopback_addresses_are_rejected` | COMPLIANT |
| Upload open to LAN | `upload_routes_are_open_to_lan_peers` | COMPLIANT |
| Remote token does not open guarded routes | `overlay_ws_templates_stay_loopback_only_even_with_valid_token` | COMPLIANT |
| Remote API open only when enabled and authenticated | `lan_good_token_ok`, `lan_enabled_no_token_401` | COMPLIANT |
| Remote surface closed while disabled | `disabled_returns_403_even_for_loopback`, `page_403_when_disabled` | COMPLIANT |

### remote-control-ui

| Requirement / scenario | Code | Test | Status |
|---|---|---|---|
| Page served when on, 403 when off | `static_assets.rs`, `tauri_assets.rs` | `remote_root_serves_remote_html`, `page_403_when_disabled`, `assets_path_maps_to_assets`, `traversal_segments_are_404` | COMPLIANT |
| Page served from a built app | `tauri_assets.rs` | one unit test only | MANUAL-USER-REPORTED (9.5) |
| Pairing flow | `session.ts`, `PairView.vue` | `pin_in_the_fragment_pairs_automatically`, `bad_pin_stays_on_pairing_with_the_server_error`, `malformed_pin_is_rejected_locally`, `unauthorized_returns_to_pairing_with_a_repair_flag`, `revoked_close_4001_clears_token_and_does_not_reconnect` | COMPLIANT (state), MANUAL-USER-REPORTED (view) |
| Show and hide | `session.ts`, `InstanceCard.vue` | `show_carries_the_pending_edit`; hide has no phone-side test | PARTIAL |
| Collapsed by default, tap to expand | `InstanceCard.vue` | none | MANUAL-USER-REPORTED |
| Edit makes no API call and shows the pending indicator | `session.ts`, `InstanceCard.vue` | `edits_never_call_update_on_their_own`, `has_pending_edits_after_an_edit_until_update_succeeds` | COMPLIANT (state), MANUAL-USER-REPORTED (visual) |
| Apply edits sends only edited keys and clears the indicator | `session.ts` | `update_sends_only_edited_keys_with_the_latest_value`, `has_pending_edits_until_show_succeeds`, `failed_update_keeps_the_draft_and_reports_the_error` | COMPLIANT |
| Apply preset | `session.ts` | no phone-side test; backend covered | PARTIAL |
| Start group shows conflicts | `session.ts` | `group_start_reports_conflicts_and_missing_templates` | COMPLIANT |
| No management actions | components have none | no UI test; route tests cover the backend | PARTIAL |
| File field read-only | `FieldEditor.vue` | `file_field_defaults_to_read_only_value_without_input`, `file_fields_are_read_only` | COMPLIANT |
| Live state, typing not overwritten, reconnect | `session.ts`, `transport.ts` | `echo_of_own_write_does_not_overwrite_text_being_typed`, `untouched_fields_follow_the_server_while_another_is_typed`, `drops_snapshot_with_revision_le_last`, `fresh_snapshot_replaces_state_even_with_a_lower_revision`, `first_state_after_each_connect_is_fresh`, `reconnects_with_growing_backoff_and_resets_after_open` | COMPLIANT |
| Empty state and instance appears | `session.ts` | `empty_registry_shows_empty_state_and_clears_when_an_instance_appears` | COMPLIANT |
| i18n keys in both locales | `es.json`, `en.json` | key-set diff run during verification returned no differences; no automated test | COMPLIANT |
| Settings: toggle, QR and PIN, URL selector, rotate confirmation | `RemoteSettingsSection.vue`, `useConfirmClick` | `remote.test.ts` store tests; confirmation UI untested | MANUAL-USER-REPORTED (7.5) |

### instance-registry

| Requirement / scenario | Test | Status |
|---|---|---|
| Source of truth, desktop reads registry | `instances.test.ts` | COMPLIANT |
| Preview stays local | `activeInstanceId_and_previews_stay_local` | COMPLIANT |
| Create with generated id, duplicate id rejected, reorder | `create_without_id_generates_distinct_ids_appended_in_order`, `create_duplicate_id_returns_instance_id_taken`, `move_reorders_and_clamps` | COMPLIANT |
| Template resolution: unknown template, duplicate id across folders | `create_unknown_template_returns_template_not_found`; no test for a duplicate id across folders | PARTIAL |
| Defaults and auto numbering | `create_assigns_defaults_from_template`, `auto_numbering_labels_second_instance_of_same_template` | COMPLIANT |
| Overlay actions, unknown id | `show_publishes_unchanged_ws_payload`, `update_publishes_payload`, `hide_publishes_payload`, `actions_on_unknown_instance_fail_and_emit_nothing` | COMPLIANT |
| Apply preset: applied, mismatch, missing template | `apply_preset_sets_fields_by_name_and_template`, `apply_preset_publishes_show_and_marks_visible_even_when_hidden`, `apply_preset_template_mismatch_returns_preset_not_found`, `apply_preset_missing_template_returns_template_not_found_and_emits_nothing` | COMPLIANT |
| Change events | `every_mutation_emits_full_snapshot_with_source`, `every_mutation_bumps_revision_monotonically`, `start_group_atomic_single_snapshot_single_revision` | COMPLIANT |
| Last write wins | writes serialize under one lock; no dedicated concurrency test | PARTIAL |
| Echo recognition | `own_client_id_echo_recognized` | COMPLIANT |
| No persistence | no persistence code; no restart test | PARTIAL |
| Group start and save in the backend, Pinia as a thin client | `save_group_calls_backend_with_name_only`, `save_group_builds_entries_in_registry_order_including_hidden`, `applies_newer_snapshot` | COMPLIANT |

### group-preset-instance-identity

| Requirement / scenario | Test | Status |
|---|---|---|
| Save rules | `save_group_records_custom_name_only_when_user_set`, `save_group_records_current_fields_and_instance_id`, `save_group_builds_entries_in_registry_order_including_hidden` | COMPLIANT |
| Missing template keeps stored fields (`370240f`) | `save_group_keeps_stored_fields_when_template_is_missing` | COMPLIANT |
| Load rules | `preserves_saved_display_name`, `id_collision_reports_conflict_per_entry`, `cross_group_overwrite`; the other group-start tests were not read in detail | PARTIAL |
| Remote start parity | both callers use the same service method (`group_start_returns_results`); no explicit parity test | PARTIAL |
| Events emitted | `start_group_atomic_single_snapshot_single_revision` | COMPLIANT |
| Conflict reporting | `id_collision_reports_conflict_per_entry`, `group_start_reports_conflicts_and_missing_templates` | COMPLIANT |
| Overlay protocol unchanged | `show_publishes_unchanged_ws_payload` | COMPLIANT |
| Restart leaves no instances | no restart test | PARTIAL |

## Design coherence

No deviations found in the decisions checked: full snapshot plus monotonic revision, commands returning the snapshot, source id, apply_preset parity with the desktop, server-side file-key filtering, REST for mutations with a server-to-client WS, localStorage plus Bearer with `?token=` only on `/ws`, QR with a `#pin` fragment, one token per pairing, constant-time compare, and the revoke epoch with close code 4001.

## Issues

### Warnings

1. **Engram mirrors are stale.** The spec mirror (#897) has no `lan-file-upload` section, while the openspec change does. The design (#898), tasks (#899) and proposal (#895) mirrors were not compared. The openspec files are authoritative.
2. **Change documents do not describe the `bcd46fc` phone behavior.** `design.md` ("Echo vs typing" row and the sequence diagram around line 104) describes only the desktop draft with its 250 ms debounce, which is still correct for the desktop. `tasks.md` 9.4 and `proposal.md` line 70 do not mention collapsible cards or apply-on-demand edits. The `remote-control-ui` spec is correct and the implementation matches it.
3. **No automated test covers the Vue components** (`InstanceCard`, `PairView`, `ControlView`, `RemoteSettingsSection`). Their scenarios rest on the user's manual reports; the underlying state logic is tested in `session.test.ts` and `remote.test.ts`.

### Suggestions

1. Add a component test for `InstanceCard` (collapsed by default, pending indicator while collapsed).
2. Add small tests for the PARTIAL rows: re-pair after rotation, a two-client WS echo, a duplicate template id across folders, and remote vs desktop group-start parity.
3. Update `design.md` and `proposal.md` for the phone's apply-on-demand drafts.

## Resolved before archive

Warning 2 and suggestion 3 were resolved by updating `design.md`, `proposal.md` and `tasks.md` 9.4. Warning 1 was resolved by refreshing the Engram mirrors. Warning 3 is accepted: component tests are left for a later change.

## Next

Recommended: `sdd-archive`.
