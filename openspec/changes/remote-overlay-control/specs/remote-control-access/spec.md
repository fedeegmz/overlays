# Delta Spec: remote-control-access (new capability)

## Purpose

Controls who may use the remote control surface: a global toggle, QR/PIN pairing that issues one token per pairing, revoke-all rotation, and per-request authentication.

## ADDED Requirements

### Requirement: Global remote toggle
A single global remote-access setting MUST exist and MUST default to off (including for configs lacking the field). Its state MUST be enforced on every request, not only at startup: turning it off MUST immediately cause requests to remote routes to be rejected (loopback included) and MUST close open remote WebSocket sessions. Turning it off MUST NOT clear stored token hashes; previously paired tokens MUST authenticate again once it is turned back on.

#### Scenario: Default off
- GIVEN a fresh or pre-existing config without the field
- WHEN the app starts
- THEN remote access is disabled

#### Scenario: Disabled rejects valid token
- GIVEN remote access is off and a device holds a valid token
- WHEN it calls a remote route from a non-loopback address
- THEN the request is rejected

#### Scenario: Toggle off at runtime
- GIVEN remote access is on with a connected remote WS session
- WHEN the user turns the toggle off
- THEN the session is closed and the next request is rejected without restart

#### Scenario: Tokens survive toggle off and on
- GIVEN a paired device and remote access turned off and then on again
- WHEN the device calls a remote route with its existing token
- THEN it is authenticated without re-pairing

### Requirement: Pairing via QR and PIN
When remote access is on, the desktop Settings MUST present a pairing QR encoding a LAN URL and a PIN. Each successful pairing (valid PIN or QR payload presented to the pairing endpoint) MUST issue a freshly generated token unique to that pairing. PIN attempts MUST be rate-limited; after the limit the endpoint MUST reject further attempts for a cooldown period regardless of PIN correctness. A wrong PIN MUST NOT issue or reveal any token.

#### Scenario: Successful pairing
- GIVEN remote access is on
- WHEN a device submits the correct PIN
- THEN it receives a newly generated token
- AND the hash of that token is added to the stored token hashes

#### Scenario: Wrong PIN
- WHEN a device submits an incorrect PIN
- THEN it is rejected with a `CommandError` and receives no token

#### Scenario: Rate limit
- GIVEN the maximum number of failed attempts was reached
- WHEN a device submits the correct PIN during the cooldown
- THEN it is rejected as rate-limited

#### Scenario: Pairing while off
- GIVEN remote access is off
- WHEN any device, loopback included, attempts pairing
- THEN it is rejected

### Requirement: LAN URL selection
The QR MUST use the first detected LAN URL. When multiple LAN addresses exist, the Settings UI MUST let the user select which URL the QR encodes.

#### Scenario: Multiple interfaces
- GIVEN the host has several LAN addresses
- WHEN the user selects another URL
- THEN the QR re-encodes the selected URL

### Requirement: Token storage and comparison
Each token MUST be generated with a cryptographically secure source. Config MUST store a list of token hashes (SHA-256), one per pairing. Token verification MUST use constant-time comparison against the stored hashes. Plaintext tokens MUST NOT be persisted.

#### Scenario: Hash at rest
- WHEN config is written after one or more pairings
- THEN it contains one hash per issued token and no plaintext token

#### Scenario: Valid and invalid token
- WHEN a request presents a token whose hash is stored
- THEN it is authenticated
- AND WHEN it presents any other value
- THEN it is rejected

### Requirement: One token per pairing
Multiple devices MAY be paired and connected concurrently, each holding the distinct token issued by its own pairing. Per-device revocation is not provided in v1.

#### Scenario: Two devices
- GIVEN two devices paired separately, each holding its own token
- WHEN both call remote routes
- THEN both are authenticated

### Requirement: Token rotation
The user MUST be able to rotate (revoke all) tokens from Settings. On rotation, all stored token hashes MUST be cleared at once, all open remote WebSocket sessions MUST be closed immediately with close code `4001`, every previously issued token MUST be rejected on subsequent requests, and devices MUST re-pair via QR/PIN to obtain a new token.

#### Scenario: Rotation disconnects
- GIVEN two paired devices with open state WS sessions
- WHEN the user rotates (revokes all) tokens
- THEN both sessions are closed with code `4001` and requests with either old token are rejected

#### Scenario: Re-pair
- GIVEN a device whose token was rotated out
- WHEN it pairs with a new PIN
- THEN it receives a new token and is authenticated

### Requirement: Per-request authentication middleware
A `remote_auth` middleware MUST guard remote routes. A request MUST be accepted if and only if remote access is enabled and the request either comes from loopback or presents a valid token. Loopback requests MUST also be rejected while remote access is off. Failures MUST return a `CommandError` JSON body with an HTTP 401 (missing/invalid token) or 403 (remote disabled) status.

#### Scenario: Loopback requires remote enabled
- GIVEN remote access is off
- WHEN a loopback request calls a remote route
- THEN it is rejected with 403

#### Scenario: Loopback allowed when enabled
- GIVEN remote access is on
- WHEN a loopback request without a token calls a remote route
- THEN it is accepted

#### Scenario: Missing token
- GIVEN remote access is on
- WHEN a non-loopback request lacks a token
- THEN it is rejected with 401 and a `CommandError` body

#### Scenario: Wrong token
- GIVEN remote access is on
- WHEN a non-loopback request presents a wrong token
- THEN it is rejected with 401

#### Scenario: Disabled
- GIVEN remote access is off
- WHEN a non-loopback request presents a valid token
- THEN it is rejected with 403

### Requirement: Loopback-only and unchanged routes
`/overlay`, `/ws`, and `/api/templates` MUST remain loopback-only and unchanged, and MUST NOT be reachable by remote clients even with a valid token. `/upload` behavior MUST be unchanged.

#### Scenario: Remote token on loopback-only route
- GIVEN a valid token
- WHEN a non-loopback request calls `/api/templates`
- THEN it is rejected

#### Scenario: Upload unchanged
- WHEN `/upload` is used as before
- THEN its behavior and access rules are identical to before this change
