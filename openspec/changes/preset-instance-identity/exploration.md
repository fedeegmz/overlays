## Exploration: preset-instance-identity

### Current State
- "Groups of overlays" = `GroupPreset` (not the per-template `Preset`). Domain: `src-tauri/src/domain/group_preset.rs:6-16` -> `GroupPresetEntry{template, fields}`, `GroupPreset{name, entries}`. No id/name per entry.
- Service upserts by trimmed group name: `src-tauri/src/application/group_preset_service.rs:20-34`. Persistence: `JsonGroupPresetRepository` `src-tauri/src/infrastructure/json_store.rs:53-80` (plain serde_json, no `#[serde(default)]` anywhere). Commands: `infrastructure/tauri/commands.rs:104-123`.
- Frontend save: `src/stores/groupPresets.ts:35-47` maps instances -> `{template, fields}` only (drops `instance.id`, `customName`, `number`). Load: `startGroup` `groupPresets.ts:63-96` calls `instanceStore.createInstance(template)` -> fresh `crypto.randomUUID()` (`instances.ts:13-15,131-144`); always appends, never replaces.
- Instance model `src/types/instance.ts:1-9`: `id` (uuid), `templateId`, `number`, `customName?`, `fields`, `visible`. Display name = `customName ?? "<template name>[ #number]"` (`instances.ts:99-122`).
- OBS URL: `instances.ts:34-38` -> `http://localhost:{port}/overlay/{template.path}?instance={id}`. `instance_id` is opaque to the server; any non-empty string works.
- Instances are in-memory only; group presets are the only persistent store of identity.
- Existing tests (`group_preset.rs`, `group_preset_service.rs`, `json_store.rs`, `error.rs`) build `GroupPresetEntry` literals and must be updated. No frontend unit runner.

### Affected Areas
- `src-tauri/src/domain/group_preset.rs`, `application/group_preset_service.rs`, `infrastructure/json_store.rs`, `infrastructure/error.rs`, `infrastructure/tauri/commands.rs` (only if a new error).
- `src/types/groupPreset.ts`, `src/stores/groupPresets.ts`, `src/stores/instances.ts`, `src/i18n/locales/{es,en}.json`, `src/components/GroupPresetsSection.vue`.

### Approaches
1. Persist id + name per entry; reuse saved id on load (create if free, handle collision if live). Backward compatible via `#[serde(default)]`. Effort: Low-Medium.
2. Persist id + name; loading replaces all current instances. Destructive, surprising. Effort: Medium.
3. User-chosen slug ids with fresh UUIDs underneath. New concept, does not meet the request. Effort: Medium.

### Recommendation
Approach 1. `Option<String>` fields with `#[serde(default)]`; reject duplicate ids within a group; `createInstance` accepts optional id/customName; never hijack a live instance of a different template. Strict TDD on Rust (legacy JSON loads, id/name round trip, duplicate-id rejection); frontend via `pnpm check` + `vue-tsc`.

### Risks
- Missing serde defaults would break loading existing `groups.json`.
- Duplicate ids in hand-edited JSON.
- Saved id colliding with a live instance of a different template.
- Two groups sharing ids cannot run simultaneously (inherent: OBS URL = id).
- Deleting and recreating an instance changes its id; user must re-save.
- JSON key casing (`instanceId` vs `instance_id`) across IPC.

### Open product questions
1. Saved id already open on load: update in place, skip, or fresh id?
2. Persist only custom names, or freeze the displayed name for every entry?
3. Loading the same group twice: idempotent?
4. Legacy groups without ids get fresh ids until re-saved: acceptable?

### Ready for Proposal
Yes, once questions 1-2 are answered.
