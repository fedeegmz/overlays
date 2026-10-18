# Delta Spec: upload-settings (new capability)

## Purpose

Persisted configuration and Settings UI for the LAN upload feature: on/off switch, destination template, and the LAN upload URL.

## Requirements

### Requirement: Uploads-enabled setting
The configuration MUST include an "uploads enabled" boolean that defaults to ON. A configuration file that lacks the field (legacy `config.json`) MUST load successfully with uploads enabled. An absent configuration file MUST also yield uploads enabled. Persisted `false` MUST be honored. Unknown/extra fields MUST NOT break loading.

#### Scenario: Legacy config
- GIVEN a `config.json` containing only `overlays_dir` and `language`
- WHEN the app loads configuration
- THEN uploads are enabled and existing values are preserved

#### Scenario: Fresh install
- GIVEN no config file exists
- THEN uploads are enabled

#### Scenario: Persisted off
- GIVEN `config.json` has uploads disabled
- WHEN the app restarts
- THEN uploads remain disabled

#### Scenario: Toggle takes effect immediately
- WHEN the user flips the switch in Settings
- THEN the setting is persisted and the running server honors it on the next request without restart

#### Scenario: Other settings preserved
- WHEN the uploads setting or template changes
- THEN `overlays_dir` and `language` are unchanged (and vice versa)

### Requirement: Destination template setting
The configuration MUST store an optional destination path template (string). When empty, whitespace-only, or unset, the effective destination MUST be `<OS Downloads directory>/Overlays`. If the OS Downloads directory cannot be determined, uploads MUST fail with a `5xx` error and write nothing (the Settings UI SHOULD surface the problem rather than silently choosing another location). The stored template is saved verbatim (unexpanded); expansion happens per upload.

#### Scenario: Unset uses default
- GIVEN no template is stored
- WHEN an upload succeeds
- THEN the file is in `<Downloads>/Overlays`

#### Scenario: Whitespace-only treated as unset
- WHEN the user saves `"   "`
- THEN the stored value is cleared and the default applies

#### Scenario: Template stored unexpanded
- WHEN the user saves `/data/%CCYY/%MM`
- THEN `config.json` holds the literal string and each upload expands it with its own time

#### Scenario: Downloads dir unavailable
- GIVEN the template is unset and no Downloads directory can be resolved
- WHEN an upload is attempted
- THEN it fails with an error and nothing is written

### Requirement: Template validation
Saving a template MUST validate it: it MUST be an absolute path (after trimming), MUST NOT contain `..` components, and MUST NOT contain NUL characters. Invalid input MUST be rejected with a structured `CommandError` carrying a stable error code, the previous value MUST remain stored, and the UI MUST display a localized message mapped from that code. Variables are not validated for existence (unknown sequences are legal and left as-is).

#### Scenario: Relative path rejected
- WHEN the user saves `uploads/%DD`
- THEN a `CommandError` is returned, the stored value is unchanged, and the UI shows a localized error

#### Scenario: Traversal rejected
- WHEN the user saves `/data/../etc`
- THEN a `CommandError` is returned and nothing changes

#### Scenario: Valid template accepted
- WHEN the user saves `/mnt/share/%CCYY-%MM`
- THEN it is stored and the success state is shown

#### Scenario: Unknown variable accepted
- WHEN the user saves `/mnt/%Q`
- THEN it is accepted (unknown sequences are left as-is at expansion)

### Requirement: Settings UI rows
The Settings page MUST add, following the existing settings-row pattern:
1. A switch for enabling/disabling uploads (reflecting the persisted value).
2. A text input for the destination template with a visible hint listing the supported variables (`%CCYY %YY %MM %DD %hh %mm %ss %%`) and stating the default destination when empty.
3. A read-only display of the LAN upload URL.

Components MUST NOT call IPC directly; changes go through the existing store/service layers. Failures MUST surface as localized messages, not raw strings.

#### Scenario: Switch reflects state
- GIVEN uploads are disabled in config
- WHEN the user opens Settings
- THEN the switch shows off

#### Scenario: Template hint visible
- WHEN the user opens Settings
- THEN the template row displays the supported variables and the default-destination note

#### Scenario: Save failure shown
- GIVEN the backend returns a `CommandError` when saving the template
- THEN the UI shows the localized message for that code

### Requirement: LAN upload URL display
Settings MUST display the URL where LAN devices reach the upload page, composed of the machine's LAN IPv4 address, the actual port the server bound (4848 or a fallback 4849-4851), and the `/upload` path. The URL SHOULD be easy to copy. If no LAN address can be determined, the UI MUST show a localized "not available" message instead of a bogus URL. The URL SHOULD remain visible (with an indication of the current state) when uploads are disabled.

#### Scenario: Normal URL
- GIVEN LAN IP `192.168.1.20` and bound port 4848
- THEN Settings shows `http://192.168.1.20:4848/upload`

#### Scenario: Fallback port
- GIVEN the server bound to 4849
- THEN the URL shows port 4849

#### Scenario: No network
- GIVEN no LAN interface is available
- THEN a localized "unavailable" message is shown

### Requirement: Localization
Every new user-visible string in the Settings UI (labels, hints, switch text, URL display, errors) MUST exist in both `es.json` and `en.json` with matching keys. The upload web page strings are served to remote clients and SHALL be English-only unless design decides otherwise.

#### Scenario: Locale parity
- WHEN comparing `es.json` and `en.json`
- THEN every new key exists in both

#### Scenario: Language switch
- WHEN the user switches the app language
- THEN the new Settings rows update accordingly
