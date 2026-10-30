# Delta Spec: group-preset-instance-identity (modified capability)

## Purpose

Observable behavior of group identity is unchanged. The load and save rules now execute in the backend `InstanceService` instead of the frontend, and group start is callable by remote clients. All other requirements of the existing spec (entry identity fields, legacy compatibility, duplicate id validation, conflict reporting, out-of-scope behavior) remain in force unchanged.

## MODIFIED Requirements

### Requirement: Save rules
When saving a group from live instances, the backend MUST read the live instances from the registry and each entry MUST record the live instance's id as `instance_id`. `custom_name` MUST be recorded only when the user explicitly set a custom name on the instance; auto-generated default labels MUST NOT be persisted.
(Previously: the frontend assembled entries from its own instance state.)

#### Scenario: User-set name persisted
- GIVEN a live instance in the registry with a user-set custom name "Main title"
- WHEN the group is saved
- THEN the entry has that `instance_id` and `custom_name` "Main title"

#### Scenario: Default label not persisted
- GIVEN a live instance with no user-set custom name (shown with an auto-numbered label)
- WHEN the group is saved
- THEN the entry has the `instance_id` and no `custom_name`

### Requirement: Load rules per entry
When a group is started, the backend instance service MUST process each entry by the first matching rule, and the operation MUST be invocable from both the desktop and remote clients with identical results:
1. Saved id is live and belongs to the same template: the existing instance MUST be updated in place (fields and show), and no new instance MUST be created. Its custom name MUST be replaced only when the entry has a saved custom name; when the entry has none, the live instance's current name MUST be preserved.
2. Saved id is not live: a new instance MUST be created using that exact id and the saved custom name (if any).
3. Saved id is live and belongs to a different template: the existing instance MUST remain untouched, a new instance MUST be created with a freshly generated id, and the conflict MUST be reported (see Conflict reporting).
4. No saved id (legacy): a new instance MUST be created with a freshly generated id (pre-change behavior).

Instance creation MUST accept an optional explicit id and optional custom name. Each resulting registry change MUST publish change events (see `instance-registry`). No group-start logic MUST remain in the frontend.
(Previously: the rules ran in the frontend `startGroup`.)

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

#### Scenario: Remote start parity
- GIVEN a saved group and an identical registry state
- WHEN the group is started from the desktop and, separately, from an authenticated remote client
- THEN the resulting instances, ids, names, fields, and conflict report are identical

#### Scenario: Events emitted
- WHEN a group start creates or updates instances
- THEN change events are published for each affected instance so desktop and remote views update

### Requirement: Conflict reporting
The group-start result MUST report each entry that was assigned a fresh id because its saved id was held by a different template. The result MUST be returned to the caller whether it is the desktop (Tauri command) or a remote client (REST response). The desktop and remote UIs MUST present conflicts using localized strings present in both `es.json` and `en.json`. A start with no conflicts MUST NOT show a conflict message.
(Previously: only the desktop UI received the result.)

#### Scenario: Conflict surfaced
- GIVEN a group start in which one entry hit a different-template id collision
- WHEN the start completes
- THEN the result identifies that entry and the UI displays a localized conflict notice

#### Scenario: Remote conflict surfaced
- GIVEN a remote group start with a different-template collision
- WHEN the REST response is returned
- THEN it identifies the entry and the remote UI shows a localized conflict notice

#### Scenario: No conflict
- GIVEN a group start with no collisions
- WHEN the start completes
- THEN no conflict notice is shown
