# Delta Spec: group-preset-instance-identity (new capability)

## Purpose

Group presets persist each entry's instance id and user-set custom name so that loading a group restores the same instance identity. OBS browser sources reference `?instance=<id>`, so stable ids keep OBS wiring working across loads.

## Requirements

### Requirement: Entry identity fields
A group preset entry MUST support an optional instance id and an optional custom name. Both fields MUST default to absent when missing from persisted JSON. Field names in persisted JSON and over IPC MUST remain snake_case (`instance_id`, `custom_name`).

#### Scenario: Entry with identity round-trips
- GIVEN an entry with `instance_id` "abc" and `custom_name` "Main title"
- WHEN the group is saved and read back
- THEN both values are preserved unchanged

#### Scenario: Entry without identity
- GIVEN an entry with neither field
- WHEN the group is saved and read back
- THEN both fields remain absent

### Requirement: Legacy compatibility
A `groups.json` written before this change (entries lacking `instance_id` and `custom_name`) MUST load successfully with all existing data preserved. Entries without an id MUST be treated as legacy entries on load. Unknown fields MUST NOT break loading. Re-saving a group after it was loaded MUST persist stable ids for its entries (see Save rules).

#### Scenario: Legacy file loads
- GIVEN a `groups.json` whose entries contain only the pre-change fields
- WHEN the app loads group presets
- THEN loading succeeds and every group and entry is unchanged

#### Scenario: Legacy entry gains id on re-save
- GIVEN a legacy group was loaded, creating instances with fresh ids
- WHEN the user saves the group again from those live instances
- THEN each entry now carries the live instance's id

### Requirement: Duplicate instance id validation
Domain validation MUST reject a group in which two or more entries share the same non-absent instance id. The rejection MUST be a structured `CommandError` with a dedicated code, mirrored in `src/types/commandError.ts`, and the UI MUST map it to localized messages in both `es.json` and `en.json`. Entries with absent ids MUST NOT be considered duplicates of each other. A rejected group MUST NOT be persisted.

#### Scenario: Duplicate ids rejected
- GIVEN a group with two entries both having `instance_id` "abc"
- WHEN the group is created or updated
- THEN the operation fails with the duplicate-instance-id `CommandError` and nothing is persisted

#### Scenario: Multiple legacy entries allowed
- GIVEN a group with several entries lacking `instance_id`
- WHEN the group is created or updated
- THEN validation passes

#### Scenario: Localized message
- WHEN the UI receives the duplicate-instance-id error
- THEN it shows a message from the active locale (es or en), not a raw string

### Requirement: Save rules
When saving a group from live instances, each entry MUST record the live instance's id as `instance_id`. `custom_name` MUST be recorded only when the user explicitly set a custom name on the instance; auto-generated default labels MUST NOT be persisted.

#### Scenario: User-set name persisted
- GIVEN a live instance with a user-set custom name "Main title"
- WHEN the group is saved
- THEN the entry has that `instance_id` and `custom_name` "Main title"

#### Scenario: Default label not persisted
- GIVEN a live instance with no user-set custom name (shown with an auto-numbered label)
- WHEN the group is saved
- THEN the entry has the `instance_id` and no `custom_name`

### Requirement: Load rules per entry
When a group is started, each entry MUST be processed by the first matching rule:
1. Saved id is live and belongs to the same template: the existing instance MUST be updated in place (fields and show), and no new instance MUST be created. Its custom name MUST be replaced only when the entry has a saved custom name; when the entry has none, the live instance's current name MUST be preserved.
2. Saved id is not live: a new instance MUST be created using that exact id and the saved custom name (if any).
3. Saved id is live and belongs to a different template: the existing instance MUST remain untouched, a new instance MUST be created with a freshly generated id, and the conflict MUST be reported (see Conflict reporting).
4. No saved id (legacy): a new instance MUST be created with a freshly generated id (pre-change behavior).

Instance creation MUST accept an optional explicit id and optional custom name.

#### Scenario: Free id restored
- GIVEN an entry with `instance_id` "abc" and no live instance "abc"
- WHEN the group is started
- THEN an instance with id "abc" and the saved custom name is created and shown

#### Scenario: Idempotent re-load
- GIVEN the group was started and instance "abc" (same template) is live
- WHEN the same group is started again
- THEN instance "abc" is updated in place and the instance count does not grow

#### Scenario: Update applies saved name and fields
- GIVEN live instance "abc" of template T with different fields and name
- WHEN a group entry for "abc" with template T is started
- THEN the instance keeps id "abc" and takes the entry's fields and custom name

#### Scenario: Update keeps live name when entry has none
- GIVEN live instance "abc" of template T renamed by the user to "Session name"
- AND a group entry for "abc" with template T and no `custom_name`
- WHEN the group is started
- THEN instance "abc" takes the entry's fields and keeps the name "Session name"

#### Scenario: Different template collision
- GIVEN live instance "abc" of template A
- WHEN a group entry with `instance_id` "abc" and template B is started
- THEN instance "abc" of template A is unchanged, a new B instance with a fresh id is created, and the conflict is reported

#### Scenario: Legacy entry
- GIVEN an entry without `instance_id`
- WHEN the group is started
- THEN a new instance with a fresh id is created

#### Scenario: Cross-group same-template overwrite
- GIVEN groups G1 and G2 each contain an entry with id "abc" of the same template, and G1 was started
- WHEN G2 is started
- THEN instance "abc" is updated in place with G2's entry values (expected behavior, since the id is the OBS URL identity)

### Requirement: Conflict reporting
The group-start result MUST report each entry that was assigned a fresh id because its saved id was held by a different template. The UI MUST present this to the user using localized strings present in both `es.json` and `en.json`. A start with no conflicts MUST NOT show a conflict message.

#### Scenario: Conflict surfaced
- GIVEN a group start in which one entry hit a different-template id collision
- WHEN the start completes
- THEN the result identifies that entry and the UI displays a localized conflict notice

#### Scenario: No conflict
- GIVEN a group start with no collisions
- WHEN the start completes
- THEN no conflict notice is shown

### Requirement: Out-of-scope behavior unchanged
Instances MUST NOT be persisted across app restarts by this change. The HTTP/WebSocket server protocol and per-template presets MUST be unchanged. Auto-numbered labels MUST NOT be frozen.

#### Scenario: Restart
- GIVEN instances were created from a group
- WHEN the app restarts
- THEN no instances exist until a group is started again, and saved ids/names remain in `groups.json`
