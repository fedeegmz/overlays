# Delta Spec: lan-file-upload (new capability)

## Purpose

Let people on the LAN upload files to the machine running the app through a web page served by the app, while keeping the overlay control surface reachable only from the local machine.

## Requirements

### Requirement: Network exposure and loopback guard
The HTTP server MUST accept connections on all network interfaces (same port range as today: 4848, fallback 4849-4851). Requests to `/ws`, `/api/*` and `/overlay/*` MUST be served only when the peer address is loopback: IPv4 `127.0.0.0/8`, IPv6 `::1`, or IPv4-mapped loopback (`::ffff:127.0.0.1`). Any other peer MUST be rejected with `403 Forbidden` and MUST NOT reach the handler. Only `/upload*` routes SHALL be reachable from non-loopback peers.

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

### Requirement: Upload page
`GET /upload` MUST return a styled, self-contained HTML page (no dependency on the user-configurable overlays directory) with a file picker, an optional file-name field, and an upload action showing progress/result to the user. The page SHALL work even when no overlays directory is configured.

#### Scenario: Page loads with no overlays dir
- GIVEN no overlays directory is configured and uploads are enabled
- WHEN a LAN client requests `GET /upload`
- THEN it receives the upload page with `200`

#### Scenario: Page reports outcome
- WHEN an upload succeeds or fails
- THEN the page shows a clear success or error message to the user

### Requirement: Upload endpoint
`POST /upload` MUST accept a `multipart/form-data` request with one file part and an optional name field, and write the file to the resolved destination directory. On success it MUST respond `2xx` and report the final stored file name. The file content MUST be streamed to disk without being fully buffered in memory.

#### Scenario: Successful upload with original name
- GIVEN uploads enabled and destination resolves to directory D
- WHEN a client uploads `photo.jpg` with an empty name field
- THEN `D/photo.jpg` exists with identical bytes and the response is `2xx`

#### Scenario: Successful upload with custom name
- WHEN a client uploads `a.jpg` with name `stage-%hh%mm`
- THEN the file is stored under the expanded name (see path-template-variables)

#### Scenario: Missing file part
- WHEN the request has no file part
- THEN the response is `400` and nothing is written

### Requirement: Destination directory
The destination directory MUST come only from server configuration (expanded destination template, or the default when unset). The destination directory SHALL be created if it does not exist. Clients MUST NOT be able to choose or influence the directory.

#### Scenario: Directory auto-created
- GIVEN the expanded destination directory does not exist
- WHEN an upload succeeds
- THEN the directory is created and contains the file

#### Scenario: Client cannot pick directory
- WHEN the name field is `../../etc/passwd` or `sub/dir/file.txt`
- THEN the file is still written inside the destination directory only (see sanitization requirements)

### Requirement: Optional file name
If the name field is non-empty (after trimming), it MUST be used as the stored file name after variable expansion and sanitization. If empty, the original uploaded file name (sanitized) MUST be used. If the typed name sanitizes to an empty name, the original file name MUST be used instead; if that is also empty, the fallback name `upload` MUST be used. If the resulting typed name has no extension and the original file name has one, the original extension MUST be appended.

#### Scenario: Typed name without extension gets original extension
- WHEN name is `clip` and original is `video.mp4`
- THEN stored name is `clip.mp4`

#### Scenario: Typed name with extension is kept
- WHEN name is `clip.mov` and original is `video.mp4`
- THEN stored name is `clip.mov`

#### Scenario: Empty name uses original
- WHEN name is `""` and original is `report.pdf`
- THEN stored name is `report.pdf`

#### Scenario: Whitespace-only name treated as empty
- WHEN name is `"   "`
- THEN the original file name is used

#### Scenario: Name reduces to nothing
- WHEN name is `..` or `/`, and the sanitized result is empty, and original is `report.pdf`
- THEN stored name is `report.pdf`

### Requirement: Never overwrite
If the target file already exists, the server MUST NOT overwrite it. It MUST insert ` (N)` (N starting at 1, incrementing until free) before the final extension. Names without extension get the suffix appended. Existing-file detection MUST be race-safe (two concurrent uploads of the same name MUST both succeed with distinct names).

#### Scenario: First collision
- GIVEN `photo.jpg` exists
- WHEN `photo.jpg` is uploaded
- THEN it is stored as `photo (1).jpg` and the original is untouched

#### Scenario: Repeated collisions
- GIVEN `photo.jpg` and `photo (1).jpg` exist
- WHEN `photo.jpg` is uploaded
- THEN it is stored as `photo (2).jpg`

#### Scenario: No extension
- GIVEN `notes` exists
- WHEN `notes` is uploaded
- THEN it is stored as `notes (1)`

#### Scenario: Multi-dot name
- GIVEN `archive.tar.gz` exists
- WHEN it is uploaded again
- THEN it is stored as `archive.tar (1).gz`

#### Scenario: Concurrent same-name uploads
- WHEN two uploads of `a.txt` are in flight simultaneously
- THEN two distinct files result and neither is truncated or lost

### Requirement: Size limit
Each uploaded file MUST NOT exceed 2 GB (fixed, not configurable). The limit MUST apply to the upload route only; other routes keep their existing behavior. A request exceeding the limit MUST be rejected with `413 Payload Too Large` and no file (final or partial) MUST remain.

#### Scenario: File within limit
- WHEN a file of exactly the limit or less is uploaded
- THEN it is accepted

#### Scenario: File over limit
- WHEN a file larger than 2 GB is uploaded
- THEN the response is `413` and nothing remains on disk

### Requirement: Atomic writes (no partial files)
Data MUST be written to a temporary file in the destination filesystem and moved to its final name only after the full upload succeeds. On any failure (client disconnect, size limit, I/O error, malformed multipart) the temporary file MUST be removed and no file with the final name SHALL appear. Temporary files MUST NOT be mistaken for completed uploads by the collision logic.

#### Scenario: Client disconnects mid-upload
- WHEN the connection drops before the body completes
- THEN no final file exists and the temp file is cleaned up

#### Scenario: Disk error
- WHEN writing fails (e.g. disk full or directory not writable)
- THEN the response is a `5xx` error, no final file exists, and the temp file is cleaned up

### Requirement: Disabled switch
Whether uploads are enabled MUST be evaluated on every request, so toggling takes effect without restarting the server. When disabled, `GET /upload` MUST still return `200` with a page that shows an "uploads are disabled" notice and no usable upload form, and `POST /upload` MUST respond `503 Service Unavailable` and write nothing (no final, temp, or created directory).

#### Scenario: Disabled page
- GIVEN uploads are disabled
- WHEN a LAN client requests `GET /upload`
- THEN the page shows the disabled notice

#### Scenario: Disabled POST
- GIVEN uploads are disabled
- WHEN a client POSTs a valid file
- THEN the response is `503` and the destination holds no new file or directory

#### Scenario: Live toggle
- GIVEN uploads were disabled and a POST returned `503`
- WHEN the user enables uploads in Settings (no restart)
- THEN the next POST succeeds

#### Scenario: Toggle during request
- GIVEN an upload already accepted before the switch is turned off
- WHEN the switch is turned off mid-transfer
- THEN that in-flight upload MAY complete; new requests are rejected
