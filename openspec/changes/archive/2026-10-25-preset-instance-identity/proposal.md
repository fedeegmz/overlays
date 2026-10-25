# Proposal: Preset Instance Identity

## Intent

Loading a saved group (`GroupPreset`) creates instances with fresh UUIDs and default names. OBS browser sources point at `?instance=<id>`, so every load breaks OBS wiring and forces manual renaming. Persist each entry's instance id and custom name so a group load restores the same identity.

## Scope

### In Scope
- `GroupPresetEntry` gains optional `instance_id` and `custom_name` (`Option` + `#[serde(default)]`).
- Domain validation: reject duplicate instance ids within one group (new structured `CommandError` + es/en i18n).
- Save maps live instance `id` and `customName` (only if user-set) into entries.
- Load rules per entry:
  - saved id live + same template → update in place (fields, name, show); idempotent re-load;
  - saved id free → create instance with that id;
  - saved id live + different template → leave it untouched, create with fresh id, report in group-start result;
  - no saved id (legacy) → fresh id (current behavior).
- `createInstance` accepts optional id and custom name.

### Out of Scope
- Persisting instances across app restart.
- Server / WebSocket changes.
- Per-template `Preset` changes.
- Freezing auto-numbered labels.

## Capabilities

### New Capabilities
- `group-preset-instance-identity`: persisting and restoring instance id + custom name in group presets, load conflict rules, duplicate-id validation, legacy compatibility.

### Modified Capabilities
- None

## Approach

Exploration Approach 1. Backend: extend entry model with serde defaults, add duplicate-id check in domain validation, map to a new `CommandError` code. Frontend: extend `src/types/groupPreset.ts`, save/load logic in `groupPresets.ts`, optional id/name in `instances.ts`, conflict line in the group-start report. Strict TDD on Rust; frontend gate `pnpm check && pnpm exec vue-tsc --noEmit`.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/domain/group_preset.rs` | Modified | Optional fields, duplicate-id validation |
| `src-tauri/src/application/group_preset_service.rs` | Modified | Validation call, tests |
| `src-tauri/src/infrastructure/json_store.rs` | Modified | Legacy JSON load test |
| `src-tauri/src/infrastructure/error.rs` | Modified | New error code |
| `src/types/groupPreset.ts`, `src/types/commandError.ts` | Modified | Mirror fields / code |
| `src/stores/groupPresets.ts`, `src/stores/instances.ts` | Modified | Save/load identity rules |
| `src/components/GroupPresetsSection.vue` | Modified | Conflict reporting |
| `src/i18n/locales/{es,en}.json` | Modified | New strings |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Existing `groups.json` fails to load | Med | `#[serde(default)]` + legacy fixture test |
| IPC key casing mismatch (`instance_id` vs `instanceId`) | Med | Keep snake_case (no `rename_all` on group presets today); typecheck mirror |
| Saved id hijacks a different template's instance | Low | Template-match guard + report |
| Two groups sharing ids collide | Low | Inherent (id = OBS URL); same-template update is defined behavior |

## Rollback Plan

Revert the change commits. No migration is needed: the backend uses no `deny_unknown_fields`, so pre-change code ignores the new optional fields in `groups.json` and loads groups as before (ids/names are simply dropped again).

## Dependencies

- None

## Success Criteria

- [ ] Legacy `groups.json` loads unchanged.
- [ ] Save → load restores same instance ids and custom names; OBS URLs keep working.
- [ ] Loading the same group twice creates no duplicates.
- [ ] Different-template id collision creates a fresh instance and is reported.
- [ ] Duplicate ids in a group are rejected with a localized error.
- [ ] `cargo test`, clippy, `pnpm check`, `vue-tsc` pass.
