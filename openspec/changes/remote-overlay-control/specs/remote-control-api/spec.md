# Delta Spec: remote-control-api (new capability)

## Purpose

Authenticated `/remote` REST endpoints and a state WebSocket let a remote client observe and control existing instances.

## ADDED Requirements

### Requirement: Authenticated surface
All `/remote` REST routes and the remote state WebSocket MUST pass `remote_auth` (see `remote-control-access`). Error responses MUST use the `CommandError` JSON shape.

#### Scenario: Unauthenticated REST
- WHEN a non-loopback request without token calls `/remote` REST
- THEN it is rejected with a `CommandError` body

#### Scenario: Unauthenticated WS
- WHEN a non-loopback client opens the state WS without a valid token
- THEN the connection is refused

### Requirement: Read endpoints
The API MUST expose: the list of live instances (id, template, display name, shown state, fields, order, revision), template metadata needed to render field editors, per-template presets, and group presets. Responses MUST reflect the registry at request time.

#### Scenario: List instances
- GIVEN two live instances
- WHEN a client requests the instance list
- THEN both are returned in registry order with their current fields and shown state

#### Scenario: Presets listed
- WHEN a client requests presets for a template
- THEN the saved presets of that template are returned

### Requirement: Control endpoints
The API MUST expose show, update (fields), and hide for an existing instance; apply a per-template preset to an existing instance; and start a group preset. Each MUST delegate to the instance service and be attributed with a remote source id. Targeting an unknown instance, preset, or group MUST return a `CommandError` with a not-found style code and HTTP 404.

#### Scenario: Show remotely
- GIVEN live instance "abc"
- WHEN an authenticated client calls show
- THEN the instance is shown, the overlay receives `show`, and the desktop receives a change event with the remote source

#### Scenario: Update fields
- WHEN an authenticated client updates fields of "abc"
- THEN the registry and overlay clients reflect the new values

#### Scenario: Apply preset
- WHEN an authenticated client applies a preset of the instance's template
- THEN the instance fields equal the preset values

#### Scenario: Start group
- GIVEN a saved group
- WHEN an authenticated client starts it
- THEN instances are created or updated per `group-preset-instance-identity` and the conflict report is returned

#### Scenario: Unknown instance
- WHEN a client calls hide for a nonexistent id
- THEN a 404 `CommandError` is returned

### Requirement: Restricted capabilities
The remote API MUST NOT expose creating, removing, renaming, or reordering instances, nor creating, editing, or deleting presets or group presets. Such requests MUST be unroutable or rejected.

#### Scenario: No remote create
- WHEN a client attempts to create an instance via `/remote`
- THEN no such route exists or the request is rejected and the registry is unchanged

#### Scenario: No preset CRUD
- WHEN a client attempts to save or delete a preset or group
- THEN the request is rejected and stored data is unchanged

### Requirement: File fields read-only
Update requests MUST NOT change fields of file type. An update that includes a file field MUST ignore or reject that field, leaving its current value unchanged.

#### Scenario: File field in update
- GIVEN an instance with a file field value F
- WHEN a remote update includes a different value for that field
- THEN the field remains F

### Requirement: State WebSocket
An authenticated state WS MUST send an initial snapshot of all live instances upon connection and then a message for each registry change event, including its source id and revision. The WS MUST be one-way for state (clients send control via REST). Sessions MUST be closed on token rotation or when remote access is turned off. A newly created or removed desktop instance MUST be broadcast so remote views refresh without reload.

#### Scenario: Initial snapshot
- WHEN a client connects
- THEN the first message contains all current instances and the current revision

#### Scenario: Desktop change propagates
- GIVEN a connected remote client
- WHEN the desktop updates an instance field
- THEN the client receives a change message with desktop source and a higher revision

#### Scenario: Instance appears
- GIVEN a connected remote client with an empty registry
- WHEN an instance is created on the desktop
- THEN the client receives a message including the new instance

#### Scenario: Session closed on rotation
- GIVEN a connected remote client
- WHEN the token is rotated
- THEN the server closes its WS session

### Requirement: Concurrent clients
Multiple remote clients and the desktop MAY operate concurrently. Writes MUST be last-write-wins and every connected client MUST receive every change.

#### Scenario: Two phones
- GIVEN two connected remote clients
- WHEN one updates a field
- THEN the other receives the change event with the first client's source id
