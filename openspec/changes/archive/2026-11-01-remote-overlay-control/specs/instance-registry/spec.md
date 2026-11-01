# Delta Spec: instance-registry (new capability)

## Purpose

The backend owns the live overlay instances in memory. Desktop and remote clients are thin views that read the registry and issue commands to it. Every change emits an event so all clients stay in sync.

## ADDED Requirements

### Requirement: Backend source of truth
The backend MUST hold all live (non-preview) instances in an in-memory registry. Frontend stores MUST NOT own instance state or duplicate defaults, numbering, or group-start logic. Preview instances and the active selection MUST remain frontend-local and MUST NOT be registered.

#### Scenario: Desktop reads registry
- GIVEN instances exist in the registry
- WHEN the desktop UI loads
- THEN it renders the instances reported by the backend, not locally derived state

#### Scenario: Preview stays local
- GIVEN the user opens a preview instance on the desktop
- WHEN the registry is listed
- THEN the preview instance is not present

### Requirement: Instance lifecycle
The registry MUST support creating an instance (optional explicit id and optional custom name), updating fields, showing, hiding, renaming, removing, and reordering. A new instance without explicit id MUST receive a freshly generated unique id. Creating with an id already live MUST be rejected with a `CommandError`. Instance order MUST be stable and reflect creation or reorder operations.

#### Scenario: Create with generated id
- GIVEN no explicit id is supplied
- WHEN an instance is created
- THEN it receives a unique id and appears last in order

#### Scenario: Duplicate id rejected
- GIVEN instance "abc" is live
- WHEN another instance is created with explicit id "abc"
- THEN the operation fails with a `CommandError` and the registry is unchanged

#### Scenario: Reorder
- GIVEN instances A, B, C in that order
- WHEN C is moved to the first position
- THEN listing returns C, A, B

### Requirement: Template resolution
The registry MUST resolve templates by catalog id through the template catalog, as defined by `overlay-catalog-browsing`: the id is `meta.id` or the overlay directory name and never includes folder segments, and when several overlays resolve to the same id the first in the deterministic traversal order wins. Operations that need template data (create, apply preset, group start) MUST read the catalog manifest once per operation rather than once per instance or entry. Creating an instance of a template id not in the catalog MUST fail with `template.not_found`.

#### Scenario: Duplicate id across folders
- GIVEN overlays `<root>/a/intro` and `<root>/b/intro` both resolve to id `intro`
- WHEN an instance of template `intro` is created
- THEN its defaults come from `a/intro` and its overlay payloads carry `template` `intro`

#### Scenario: Unknown template
- WHEN an instance is created for a template id absent from the catalog
- THEN a `template.not_found` `CommandError` is returned and the registry is unchanged

### Requirement: Defaults and auto-numbering
When an instance is created, its fields MUST be initialized from the template defaults (the logic previously in the frontend). An instance without a user-set custom name MUST expose an auto-numbered default label per template; labels MUST NOT be frozen or persisted.

#### Scenario: Defaults applied
- GIVEN a template declares default field values
- WHEN an instance of it is created without field values
- THEN the instance fields equal the defaults

#### Scenario: Auto numbering
- GIVEN two instances of template T without custom names
- WHEN the registry is listed
- THEN they carry distinct auto-numbered labels and no custom name

### Requirement: Overlay actions
`show`, `update`, and `hide` on a live instance MUST update registry state and MUST deliver the matching overlay WebSocket message (`{ instance_id, template, action, fields }`) to the overlay clients, with payload shape unchanged. Acting on an unknown id MUST fail with a `CommandError`.

#### Scenario: Show broadcasts
- GIVEN live instance "abc"
- WHEN show is invoked with fields
- THEN the registry marks it shown and an overlay `show` message for "abc" is emitted

#### Scenario: Unknown instance
- WHEN update is invoked for an id not in the registry
- THEN a `CommandError` is returned and nothing is emitted

### Requirement: Apply preset
Applying a per-template preset to a live instance MUST match the current desktop behavior: it MUST set the instance fields to the preset values, deliver a `show` overlay message with those fields to overlay clients (whether or not the instance was shown before), and mark the instance visible. The preset MUST be looked up by name among the presets of the instance's template; a preset of a different template or an unknown name MUST be rejected with `preset.not_found`. When the instance's template is not present in the catalog, the operation MUST be rejected with `template.not_found`. A rejected operation MUST leave the instance unchanged and emit nothing.

#### Scenario: Preset applied
- GIVEN a hidden live instance of template T and a preset of T
- WHEN the preset is applied
- THEN the instance fields equal the preset values, an overlay `show` message with those fields is emitted, and the instance is marked visible

#### Scenario: Template mismatch
- GIVEN a preset of template B and an instance of template A
- WHEN the preset is applied to the instance
- THEN a `preset.not_found` `CommandError` is returned and the instance is unchanged

#### Scenario: Template missing from catalog
- GIVEN a live instance whose template is no longer in the catalog
- WHEN a preset is applied to it
- THEN a `template.not_found` `CommandError` is returned, the instance is unchanged, and nothing is emitted

### Requirement: Change events
Every registry operation that mutates state MUST publish exactly one change event, a full snapshot of the registry, through a port consumed by (a) the Tauri event emitter and (b) the remote state broadcast. A compound operation that touches several instances (group start) MUST still publish a single snapshot with a single revision bump. Each event MUST carry a source identifier (desktop or remote client) and a monotonically increasing revision id. Concurrent writes MUST resolve as last-write-wins by arrival order at the registry.

#### Scenario: Event on mutation
- WHEN any instance is created, updated, shown, hidden, renamed, removed, or reordered
- THEN one change event with source and revision is published to both consumers

#### Scenario: Compound operation emits one event
- GIVEN a group start that creates one instance and updates another
- WHEN it completes
- THEN exactly one snapshot event containing both instances is published and the revision increases by exactly one

#### Scenario: Revision increases
- GIVEN two consecutive mutations
- WHEN their events are compared
- THEN the second revision is greater than the first

#### Scenario: Last write wins
- GIVEN desktop and a remote client update the same field near-simultaneously
- WHEN both are applied
- THEN the registry holds the value of the later-applied write and the final event reflects it

#### Scenario: Echo recognition
- GIVEN a client issued a write with its source id
- WHEN the resulting event arrives back
- THEN the client can identify it as its own via source and revision and avoid overwriting in-progress local input

### Requirement: No persistence
Registry contents MUST NOT be persisted across app restarts.

#### Scenario: Restart
- GIVEN instances exist
- WHEN the app restarts
- THEN the registry is empty

### Requirement: Group start and save in backend
Starting a group preset MUST be executed by the registry service (see `group-preset-instance-identity`), callable from the desktop and from remote clients, and MUST return the conflict report. Saving a group from live instances MUST build the entries in the backend from the registry; the desktop MUST supply only the group name.

#### Scenario: Group start via service
- GIVEN a saved group
- WHEN a group start is requested from any client
- THEN the service creates or updates instances and returns the result with conflicts

#### Scenario: Group save via service
- GIVEN live instances in the registry
- WHEN the desktop saves a group by name
- THEN the stored entries are built from the registry, not from client-supplied entries

### Requirement: Desktop behavior parity
After this change desktop and OBS behavior MUST be unchanged: the same operations produce the same overlay messages and UI outcomes as before, and the Pinia stores act as event-fed thin clients.

#### Scenario: Pinia reflects events
- WHEN a change event is received by the desktop
- THEN the Pinia instances store updates from it without owning independent state
