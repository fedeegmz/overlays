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
Applying a per-template preset to a live instance MUST set the instance fields to the preset values and, if the instance is shown, deliver an `update` to overlay clients. A preset for a different template MUST be rejected with a `CommandError`.

#### Scenario: Preset applied
- GIVEN live instance of template T and a preset of T
- WHEN the preset is applied
- THEN the instance fields equal the preset values

#### Scenario: Template mismatch
- GIVEN a preset of template B and an instance of template A
- WHEN the preset is applied to the instance
- THEN a `CommandError` is returned and the instance is unchanged

### Requirement: Change events
Every registry mutation MUST publish a change event through a port consumed by (a) the Tauri event emitter and (b) the remote state broadcast. Each event MUST carry a source identifier (desktop or remote client) and a monotonically increasing revision id. Concurrent writes MUST resolve as last-write-wins by arrival order at the registry.

#### Scenario: Event on mutation
- WHEN any instance is created, updated, shown, hidden, renamed, removed, or reordered
- THEN one change event with source and revision is published to both consumers

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

### Requirement: Group start in backend
Starting a group preset MUST be executed by the registry service (see `group-preset-instance-identity`), callable from the desktop and from remote clients, and MUST return the conflict report.

#### Scenario: Group start via service
- GIVEN a saved group
- WHEN a group start is requested from any client
- THEN the service creates or updates instances and returns the result with conflicts

### Requirement: Desktop behavior parity
After this change desktop and OBS behavior MUST be unchanged: the same operations produce the same overlay messages and UI outcomes as before, and the Pinia stores act as event-fed thin clients.

#### Scenario: Pinia reflects events
- WHEN a change event is received by the desktop
- THEN the Pinia instances store updates from it without owning independent state
