## Verification Report

**Change**: preset-instance-identity | **Mode**: Strict TDD, hybrid | **Branch**: feat/preset-instance-identity (bc09fcf, 86c6358, d097a8f)

**Verdict**: PASS WITH WARNINGS — 0 CRITICAL, 5 WARNING, 3 SUGGESTION.

### Completeness
| Item | Result |
|---|---|
| Tasks 1.1–3.8 | 21/21 checked |
| Phase 4 (4.1–4.4) | 0/4 — manual user checks, pending (not failed) |
| Implementation matches tasks | Yes |

### Execution evidence
| Command | Result |
|---|---|
| `cd src-tauri && cargo test` | 191 passed, 0 failed |
| `cargo fmt --check --all` | OK |
| `cargo clippy -- -D warnings` | clean |
| `pnpm check` (Biome) | clean, 86 files |
| `pnpm exec vue-tsc --noEmit` | clean |
| Coverage tool | none detected |

### TDD compliance
| Check | Result | Details |
|---|---|---|
| TDD evidence table in apply-progress | ⚠️ | Prose only, no table |
| Tests exist for backend tasks | ✅ | All pass |
| RED observed | ⚠️ | WU1 plausible; WU2 tests were characterization (passed immediately) |
| GREEN | ✅ | 191/191 |
| Triangulation | ✅ | Duplicate index, blank, trimmed, `None` cases |

### Spec compliance matrix
| Requirement / scenario | Evidence | Status |
|---|---|---|
| Entry fields round trip / without identity | `json_store::group_identity_round_trips_and_absent_keys_are_omitted` | ✅ |
| snake_case keys | serde fields, `src/types/groupPreset.ts:4-5`, JSON assertions | ✅ |
| Legacy file loads | `json_store::legacy_group_file_without_identity_keys_loads_as_none` | ✅ |
| Legacy entry gains id on re-save | `src/stores/groupPresets.ts:40-41` | ⚠️ code only |
| Duplicate ids rejected | `rejects_duplicate_instance_ids_with_index_of_second`, `save_rejects_duplicate_instance_ids_on_create_and_update_without_writing` | ✅ |
| Multiple absent ids allowed | `allows_multiple_absent_instance_ids` | ✅ |
| Dedicated CommandError | `maps_group_preset_errors_to_codes` (`group_preset.duplicate_instance_id`, param `index`) | ✅ |
| Localized message | key in `en.json` and `es.json` | ✅ |
| Save: only user-set name persisted | `groupPresets.ts:41`, `instances.ts:106,128` | ⚠️ code only |
| Load 1: same template → update in place, keep live name if none saved | `groupPresets.ts:96-100` | ⚠️ code only |
| Load 2: free id → create with id | `groupPresets.ts` free branch | ⚠️ code only |
| Load 3: different template → fresh id + `idConflict` | `groupPresets.ts` conflict branch | ⚠️ code only |
| Load 4: legacy → fresh id | `groupPresets.ts` | ⚠️ code only |
| Idempotent re-load / cross-group overwrite | same-template branch | ⚠️ code only |
| `createInstance` optional id/name, throws on live id | `instances.ts:131-143` | ✅ code |
| Conflict reporting UI | `GroupStartResult.idConflict`, `groups.idConflict` (es/en) | ⚠️ code only |
| Out of scope unchanged | no HTTP/WS or per-template preset changes | ✅ |

### Design conformance
All decisions match (keys, serde attributes, domain validation, error code, claimed set, `createInstance` signature, name rule, i18n wording, no `invoke` in components, CHANGELOG). One deviation: on send failure, an updated live instance has its previous `fields` and `customName` restored (`groupPresets.ts:124-128`) — beyond the design's "remove only newly created".

### Issues
**WARNING**
1. apply-progress lacks a formal TDD Cycle Evidence table.
2. WU2 tests had no observed RED (characterization).
3. Undocumented rollback of fields/name on failed in-place update.
4. Frontend save/load scenarios verified by code reading only (no runner).
5. Phase 4 manual checks pending with the user.

**SUGGESTION**
1. Extract per-entry reconcile into a pure function for future unit testing.
2. Load re-trims `instance_id`/`custom_name`; harmless and handles hand-edited blanks.
3. `idConflict` notice uses class `muted`; confirm readability during manual check.
