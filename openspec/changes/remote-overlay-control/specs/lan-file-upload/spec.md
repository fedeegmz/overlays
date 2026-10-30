# Delta Spec: lan-file-upload (modified capability)

## Purpose

The network exposure rule of `lan-file-upload` stated that only `/upload*` is reachable from non-loopback peers. This change adds a second LAN-reachable surface, `/remote*`, gated by the remote-access toggle and token authentication defined in `remote-control-access`. Upload behavior itself is unchanged; all other requirements of the existing spec remain in force unchanged.

## MODIFIED Requirements

### Requirement: Network exposure and loopback guard
The HTTP server MUST accept connections on all network interfaces (same port range as today: 4848, fallback 4849-4851). Requests to `/ws`, `/api/*` and `/overlay/*` MUST be served only when the peer address is loopback: IPv4 `127.0.0.0/8`, IPv6 `::1`, or IPv4-mapped loopback (`::ffff:127.0.0.1`). Any other peer MUST be rejected with `403 Forbidden` and MUST NOT reach the handler, even when it presents a valid remote access token. Only `/upload*` and `/remote*` routes SHALL be reachable from non-loopback peers. `/remote*` routes MUST be reachable from non-loopback peers only while remote access is enabled; the `/remote` REST API and state WebSocket MUST additionally require a valid remote access token, as defined by `remote_auth` in `remote-control-access`, while the remote page and its static assets (`/remote/`, `/remote/assets/*`) require only that remote access is enabled. While remote access is disabled, every `/remote*` request MUST be rejected, loopback included.
(Previously: only `/upload*` routes were reachable from non-loopback peers.)

#### Scenario: Local OBS keeps working
- GIVEN the server is running
- WHEN a request to `/overlay/<template>/index.html` or `/ws` arrives from `127.0.0.1`
- THEN it is served as before

#### Scenario: IPv6 loopback allowed
- WHEN a request to `/api/templates` arrives from `::1`
- THEN it is served

#### Scenario: IPv4-mapped loopback allowed
- WHEN a request to `/ws` arrives from `::ffff:127.0.0.1`
- THEN it is served

#### Scenario: LAN peer blocked on guarded routes
- WHEN a request to `/ws`, `/api/templates` or `/overlay/x/index.html` arrives from `192.168.1.50`
- THEN the response is `403` and no overlay data is returned

#### Scenario: Non-loopback IPv6 and mapped LAN peers blocked
- WHEN a guarded route is requested from `fe80::1`, `2001:db8::1` or `::ffff:192.168.1.50`
- THEN the response is `403`

#### Scenario: Upload routes open to LAN
- WHEN `GET /upload` arrives from `192.168.1.50`
- THEN it is served (subject to the disabled behavior below)

#### Scenario: Remote token does not open guarded routes
- GIVEN remote access is enabled and a LAN device holds a valid remote access token
- WHEN it requests `/ws`, `/api/templates` or `/overlay/x/index.html` from `192.168.1.50` presenting that token
- THEN the response is `403`

#### Scenario: Remote API open to LAN only when enabled and authenticated
- GIVEN remote access is enabled
- WHEN `GET /remote/api/state` arrives from `192.168.1.50` with a valid token
- THEN it is served
- AND WHEN the same request arrives without a token or with an invalid token
- THEN it is rejected with `401`

#### Scenario: Remote surface closed while disabled
- GIVEN remote access is disabled
- WHEN any `/remote*` request arrives, from a LAN peer or from loopback
- THEN it is rejected with `403`
