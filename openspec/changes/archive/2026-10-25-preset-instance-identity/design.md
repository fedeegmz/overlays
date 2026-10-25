# Design: Preset Instance Identity

## Technical Approach

Extend `GroupPresetEntry` with two optional, serde-defaulted fields; normalize and validate them in the domain constructor (`GroupPreset::new`), which the service already calls on every save. The frontend snapshots `id`/`customName` on save and reconciles per entry on `startGroup`. No server/WS change; `commands.rs` is unchanged (it deserializes the struct as-is).

## Architecture Decisions

| Topic | Options | Tradeoff | Decision |
|---|---|---|---|
| JSON keys | snake_case (`instance_id`, `custom_name`) vs `rename_all = "camelCase"` | camelCase would be the only renamed group field; Tauri converts top-level command args only, nested struct fields follow serde names | **snake_case** across IPC and `groups.json`; TS mirror uses the same keys |
| Serde shape | `Option<String>` + `#[serde(default)]` vs `String` with `""` | `""` sentinel is ambiguous | **`Option<String>`, `#[serde(default, skip_serializing_if = "Option::is_none")]`**: legacy files load, files written without identity keep the legacy shape |
| Duplicate check | domain vs service vs frontend | Domain covers every constructor; frontend cannot be trusted alone | **`GroupPreset::new`**, after normalization (trim; blank becomes `None`). Many `None` ids are allowed |
| Error | new variant `GroupPresetDuplicateInstanceId { index }` (index of the 2nd occurrence) | Mirrors `invalid_entry` (index param); a UUID in the message is noise | Code **`group_preset.duplicate_instance_id`**, param `index` |
| Data loaded from file | Validate on load vs reconcile defensively | Validating on load could make a hand-edited file unusable | Load stays unvalidated; `startGroup` treats an id already claimed earlier in the same run as a conflict (gets a fresh id) |
| `createInstance` | new function vs optional arg | Optional arg keeps a single creation path | `createInstance(templateId, opts?: { id?: string; customName?: string })`; throws if `opts.id` is already live (programming error; `startGroup` checks first) |
| `number` | persist vs recompute | Freezing labels is out of scope | Always `nextInstanceNumber(templateId)` |
| Update-in-place name | keep live name vs apply saved | User decision: a saved name wins, but an absent saved name never erases a live one | `if (entry.custom_name) customName = entry.custom_name`; otherwise the live `customName` is kept |
| Result shape | new `ok:false` reason vs flag on success | The instance does start, so it counts as a success with a warning | Success variant gains `idConflict: boolean` |

## Data Flow

Save: `instances.map(i => ({ template, fields, instance_id: i.id, custom_name: i.customName }))` goes through `save_group_preset`, then `GroupPreset::new` (normalize + validate), then the JSON store. `customName` is already `undefined` unless the user renamed the instance.

Load (`startGroup`, per entry):

```
entry ──► template missing? ──yes──► {ok:false, templateMissing}
            │no
            ▼
   id = normalized entry.instance_id
   claimed = Set<string> (ids used in this run)
            │
   ┌────────┼─────────────────────────────┬──────────────────────────────┐
   no id    id free & !claimed             live, same template, !claimed   live other template OR claimed
   │        │                              │                               │
 create()  create({id, customName})     update in place:              create({customName}) fresh id
                                         fields = {...defaults,        idConflict = true
                                         ...entry.fields},
                                         customName = saved ?? live
   └────────┴──────────────┬───────────────┴───────────────────────────────┘
                           ▼
            claimed.add(resultId); showInstance(resultId)
            fail ─► newly created? removeInstance : keep live one ─► {ok:false, sendFailed}
            ok   ─► {ok:true, instanceId, idConflict}
```

`activeInstanceId` is restored afterwards, as it is today. Loading a second group that shares a same-template id overwrites that instance. This is accepted behavior because the id is the OBS URL.

## File Changes

| File | Action | Description |
|---|---|---|
| `src-tauri/src/domain/group_preset.rs` | Modify | New fields, normalization, duplicate check, tests |
| `src-tauri/src/domain/error.rs` | Modify | `GroupPresetDuplicateInstanceId { index }` + Display |
| `src-tauri/src/infrastructure/error.rs` | Modify | `GROUP_PRESET_DUPLICATE_INSTANCE_ID` mapping + test |
| `src-tauri/src/application/group_preset_service.rs` | Modify | Test helper literals; rejects-without-writing test |
| `src-tauri/src/infrastructure/json_store.rs` | Modify | Helper literal; legacy-load and round-trip tests |
| `src/types/groupPreset.ts` | Modify | `instance_id?`, `custom_name?`; `idConflict` on the success variant |
| `src/stores/instances.ts` | Modify | `createInstance` options |
| `src/stores/groupPresets.ts` | Modify | Save mapping; reconcile algorithm |
| `src/components/GroupPresetsSection.vue` | Modify | Render conflict notices |
| `src/i18n/locales/{es,en}.json` | Modify | `groups.idConflict`, `errors.group_preset.duplicate_instance_id` |

`src/types/commandError.ts` needs no change because codes are plain strings.

## Interfaces / Contracts

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupPresetEntry {
    pub template: String,
    pub fields: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
}
```

```ts
export type GroupStartResult =
  | { template: string; ok: true; instanceId: string; idConflict: boolean }
  | { template: string; ok: false; reason: "templateMissing" | "sendFailed" };
```

i18n:
- en: `groups.idConflict`: "{template}: its saved ID is used by another overlay. Started with a new ID; update the OBS source URL." and `errors.group_preset.duplicate_instance_id`: "Group entry {index} repeats an instance ID."
- es: "{template}: su ID guardado lo usa otro overlay. Se inició con un ID nuevo; actualiza la URL de la fuente en OBS." and "La entrada {index} del grupo repite un ID de instancia."

## Testing Strategy

| Layer | What | Approach |
|---|---|---|
| Unit (domain) | Duplicate ids rejected (index of the 2nd); multiple `None` allowed; blank values normalized to `None`; custom name trimmed | Strict TDD, RED first |
| Unit (error) | Duplicate code + `index` param | Extend `maps_group_preset_errors_to_codes` |
| Service | Duplicate rejected without writing | `InMemoryRepo.saves == 0` |
| Integration (store) | Legacy JSON without the keys loads as `None`; id/name round trip; `None` keys omitted on write | Temp-file tests in `json_store.rs` |
| Frontend | Types, lint | `pnpm check && pnpm exec vue-tsc --noEmit`; manual: save, load twice (no duplicates), different-template conflict |

## Migration / Rollout

No migration is required. Legacy entries get fresh ids until the group is re-saved. Rollback: revert the commits. Pre-change code ignores the extra keys (no `deny_unknown_fields`) and drops them on the next save.

## Open Questions

None blocking.
