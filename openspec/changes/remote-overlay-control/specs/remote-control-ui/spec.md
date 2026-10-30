# Delta Spec: remote-control-ui (new capability)

## Purpose

A mobile-first page served at `/remote` lets a paired phone control existing instances, plus the desktop Settings section that manages remote access.

## ADDED Requirements

### Requirement: Serving the remote page
The app MUST serve a dedicated `remote.html` entry at `/remote` in production builds (via Tauri asset resolution) and in development. The page MUST be reachable from the LAN when remote access is on and the device is authenticated or can pair; when remote access is off, non-loopback requests MUST be rejected.

#### Scenario: Page served
- GIVEN remote access is on
- WHEN a phone opens the QR URL
- THEN the remote page loads

#### Scenario: Disabled
- GIVEN remote access is off
- WHEN a phone opens `/remote`
- THEN it is rejected and no page content is served

### Requirement: Pairing flow in UI
An unpaired device MUST be shown a PIN entry (or auto-pair from the QR payload). The received token MUST be kept on the device and used for REST and WS. After a failed token check (e.g. rotation) the page MUST return to the pairing view with a localized message.

#### Scenario: Enter PIN
- WHEN the user enters the correct PIN
- THEN the control view opens

#### Scenario: Token invalidated
- GIVEN a paired phone
- WHEN the token is rotated on the desktop
- THEN the phone shows the pairing view and a localized "re-pair" message

### Requirement: Instance control view
The page MUST list the desktop's live instances in registry order and, for each, let the user show, hide, edit non-file field values (applied via update), apply a per-template preset, and start a group preset. It MUST NOT offer create, remove, rename, reorder, or preset/group CRUD actions.

#### Scenario: Show and hide
- GIVEN a listed instance
- WHEN the user taps show then hide
- THEN the corresponding API calls are made and the state reflects each

#### Scenario: Edit text field
- WHEN the user edits a text field
- THEN an update is sent (debounced) and the overlay reflects the value

#### Scenario: Apply preset
- WHEN the user chooses a preset for an instance
- THEN the preset is applied and the fields refresh

#### Scenario: Start group
- WHEN the user starts a group preset
- THEN the group is started and any reported conflicts are shown with a localized notice

#### Scenario: No management actions
- WHEN the page is rendered
- THEN no control for creating, removing, renaming, reordering instances or editing presets is present

### Requirement: File fields read-only
File-type fields MUST be shown read-only (current value displayed) and MUST NOT be editable.

#### Scenario: File field
- GIVEN an instance with a file field
- WHEN the control view renders it
- THEN the value is visible and no input to change it is available

### Requirement: Live state without reload
The page MUST keep itself synchronized through the state WS. Remote and desktop changes MUST appear without a manual reload. Incoming events that echo the page's own writes (by source id and revision) MUST NOT overwrite text the user is currently typing.

#### Scenario: Desktop change visible
- GIVEN the page is open
- WHEN the desktop changes an instance field
- THEN the page shows the new value without reload

#### Scenario: Typing not overwritten
- GIVEN the user is typing in a field and an echo of an earlier own write arrives
- WHEN the event is processed
- THEN the in-progress input is not replaced

#### Scenario: Reconnect
- GIVEN the WS drops
- WHEN it reconnects
- THEN the page replaces its state with the fresh snapshot

### Requirement: Empty state
When there are no desktop instances, the page MUST show a localized empty-state message ("No instances are open on the desktop"). When an instance appears via the state WS the message MUST be replaced by the instance list without reload.

#### Scenario: Empty
- GIVEN the registry has no instances
- WHEN the page is open
- THEN the empty-state message is displayed

#### Scenario: Instance appears
- GIVEN the empty state is displayed
- WHEN an instance is created on the desktop
- THEN the instance appears and the message disappears without reload

### Requirement: Shared field editor and i18n
Field editing MUST reuse a shared `FieldEditor` component used by the desktop and remote pages. All user-facing text MUST come from i18n with entries in both `es.json` and `en.json`.

#### Scenario: Locale strings
- WHEN the remote page or the Settings remote section renders
- THEN all text is resolved from the active locale, with keys present in both locales

### Requirement: Desktop Settings remote section
Settings MUST provide: the remote toggle (default off), the pairing QR and PIN (when on), a LAN URL selector (when several), and a rotate-token action with confirmation that warns that all paired devices will be disconnected.

#### Scenario: Toggle on shows pairing
- WHEN the user turns the toggle on
- THEN the QR and PIN are shown

#### Scenario: Rotate confirmation
- WHEN the user chooses rotate
- THEN a confirmation warns that all paired devices will be disconnected, and rotation happens only after confirming
