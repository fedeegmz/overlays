# Tasks: Preset Instance Identity

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~280-360 (Rust ~170 incl. tests, frontend ~120, i18n/CHANGELOG ~20) |
| 400-line budget risk | Medium |
| Chained PRs recommended | No |
| Suggested split | Single PR, 3 work-unit commits |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Medium

Note: if the frontend WU grows past budget, split WU3 into its own PR (stacked-to-main).

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| WU1 | Backend fields + duplicate validation + error code | PR 1 | base main; Rust tests included |
| WU2 | Persistence compat tests + service test | PR 1 | tests only plus helper literals |
| WU3 | Frontend save/load reconcile, i18n, CHANGELOG | PR 1 | gate: pnpm check + vue-tsc + manual |

## Phase 1: Backend domain (WU1, strict TDD)

- [x] 1.1 RED: in `src-tauri/src/domain/group_preset.rs` tests, add: duplicate ids rejected with index of 2nd; multiple `None` allowed; blank id/name normalized to `None`; name trimmed.
- [x] 1.2 GREEN: add `instance_id`/`custom_name` (`Option`, `serde(default, skip_serializing_if)`) to `GroupPresetEntry`; normalize and dup-check in `GroupPreset::new`.
- [x] 1.3 RED: extend `maps_group_preset_errors_to_codes` in `src-tauri/src/infrastructure/error.rs` for `group_preset.duplicate_instance_id` with `index` param.
- [x] 1.4 GREEN: add `GroupPresetDuplicateInstanceId { index }` + Display in `src-tauri/src/domain/error.rs`; map `GROUP_PRESET_DUPLICATE_INSTANCE_ID`.
- [x] 1.5 Fix entry literals in existing tests/helpers; run `cargo test`, `cargo fmt --check --all`, `cargo clippy -- -D warnings`. Commit `feat(group-presets): persist instance identity in entries`.

## Phase 2: Persistence and service tests (WU2)

- [x] 2.1 RED/GREEN: `src-tauri/src/infrastructure/json_store.rs` tests: legacy JSON without keys loads as `None`; id/name round trip; `None` keys omitted on write.
- [x] 2.2 `src-tauri/src/application/group_preset_service.rs`: test duplicate ids rejected with `InMemoryRepo.saves == 0` (create and update).
- [x] 2.3 Run `cargo test` + clippy. Commit `test(group-presets): cover legacy load and duplicate rejection`.

## Phase 3: Frontend (WU3, gate `pnpm check && pnpm exec vue-tsc --noEmit`)

- [x] 3.1 `src/types/groupPreset.ts`: add `instance_id?`, `custom_name?`; `idConflict: boolean` on success `GroupStartResult`.
- [x] 3.2 `src/stores/instances.ts`: `createInstance(templateId, opts?: { id?, customName? })`; throw if `opts.id` is live.
- [x] 3.3 `src/stores/groupPresets.ts`: save maps `instance_id: i.id`, `custom_name: i.customName`.
- [x] 3.4 `src/stores/groupPresets.ts` `startGroup`: per-entry reconcile (no id / free / same-template update / other-template or claimed to fresh id + `idConflict`); update keeps live name unless `entry.custom_name`; claimed set; on send failure remove only newly created.
- [x] 3.5 `src/components/GroupPresetsSection.vue`: render localized conflict notice per `idConflict` result only.
- [x] 3.6 `src/i18n/locales/{es,en}.json`: add `groups.idConflict`, `errors.group_preset.duplicate_instance_id`.
- [x] 3.7 `CHANGELOG.md`: add entry under Unreleased.
- [x] 3.8 Run gate; manual checklist below. Commit `feat(groups): restore instance id and name on group load`.

## Phase 4: Manual verification

- [x] 4.1 Save group, reload app, load group: OBS URL id unchanged, custom name restored.
- [x] 4.2 Load same group twice: no duplicates; renamed live instance keeps name when entry has none.
- [x] 4.3 Different-template collision: fresh id, localized notice; no notice otherwise.
- [x] 4.4 Legacy `groups.json` loads; re-save adds ids; duplicate-id error shows localized in es and en.
