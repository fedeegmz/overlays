# Delta Spec: overlay-catalog-browsing (new capability)

## Purpose

Recursive discovery of overlay templates under the overlays directory, folder metadata exposed additively on the catalog, deterministic id-collision handling, correct asset resolution for nested overlays, and a file-explorer style home page.

## Requirements

### Requirement: Recursive discovery
The template source MUST discover overlays at any depth up to a maximum depth of 5 directory levels below the overlays root. A directory containing `overlay.json` MUST be treated as a leaf overlay: its subdirectories MUST NOT be scanned. Directories whose name starts with `.` MUST be skipped. Symbolic links MUST NOT be followed. Traversal order MUST be deterministic: depth-first, entries sorted alphabetically by relative path.

#### Scenario: Nested overlay discovered
- GIVEN `<root>/streams/intro/overlay.json` exists
- WHEN the catalog is listed
- THEN a template for `intro` is returned with folder `streams`

#### Scenario: Flat overlay unchanged
- GIVEN `<root>/lower-third/overlay.json` exists
- WHEN the catalog is listed
- THEN the template is returned with an empty (root) folder and the same id as before the change

#### Scenario: Leaf directory is not descended
- GIVEN `<root>/a/overlay.json` and `<root>/a/b/overlay.json` both exist
- WHEN the catalog is listed
- THEN only the template at `a` is returned

#### Scenario: Hidden directories skipped
- GIVEN `<root>/.git/x/overlay.json` exists
- WHEN the catalog is listed
- THEN no template from `.git` is returned

#### Scenario: Symlinks not followed
- GIVEN `<root>/link` is a symlink to a directory containing overlays
- WHEN the catalog is listed
- THEN nothing under `link` is returned

#### Scenario: Depth limit
- GIVEN an overlay located 6 directory levels below the root
- WHEN the catalog is listed
- THEN that overlay is not returned, while one at 5 levels is

### Requirement: Template identity
The template id MUST remain `meta.id` when present, otherwise the overlay directory name; it MUST NOT include folder segments. Existing presets, instances, WS payloads (`{ instance_id, template, action, fields }`) and OBS URLs MUST keep working unchanged.

#### Scenario: Id independent of folder
- GIVEN an overlay in `<root>/streams/intro` without `meta.id`
- THEN its id is `intro`

#### Scenario: Explicit meta id
- GIVEN an overlay whose directory is `<root>/x/dir-name` and whose manifest sets `id` to `custom`
- THEN its id is `custom`

### Requirement: Duplicate id handling
When two or more overlays resolve to the same id, the first found in the deterministic traversal order MUST be kept and the others MUST be skipped. Each skipped duplicate MUST produce a warning in the log naming the id and the skipped path. The UI MUST NOT indicate skipped duplicates.

#### Scenario: Duplicate across folders
- GIVEN `<root>/a/intro` and `<root>/b/intro` both resolve to id `intro`
- WHEN the catalog is listed
- THEN only the overlay at `a/intro` is returned and a warning mentions the skipped `b/intro`

#### Scenario: Root vs nested duplicate ordering
- GIVEN duplicates at `<root>/intro` and `<root>/a/intro`
- THEN the one winning is the first by alphabetical relative-path order, depth-first, and the result is identical across runs

### Requirement: Additive catalog metadata
Each template in the catalog MUST carry a relative `folder` path (forward-slash separated, empty for root). The catalog MUST also expose the list of known folders so that folders without any overlay (empty folders) are represented. Existing `Manifest` fields MUST NOT be removed or renamed; new fields MUST be additive so existing consumers continue to work. A folder is any non-hidden, non-leaf directory reached within the depth limit.

#### Scenario: Empty folder listed
- GIVEN `<root>/empty/` contains no overlays
- WHEN the catalog is listed
- THEN `empty` appears in the known-folders list

#### Scenario: Manifest consumer compatibility
- GIVEN a consumer that reads only pre-existing `Manifest` fields
- WHEN the catalog contains nested overlays
- THEN that consumer still parses the response successfully

### Requirement: Nested asset resolution
Asset requests for an overlay MUST resolve from that overlay's real directory, regardless of nesting and regardless of whether its id differs from its directory name. Paths escaping the overlay directory MUST be rejected as before.

#### Scenario: Nested asset served
- GIVEN overlay `intro` in `<root>/streams/intro`
- WHEN `style.css` is requested for template `intro`
- THEN the file `<root>/streams/intro/style.css` is served

#### Scenario: Id differs from directory
- GIVEN an overlay with `meta.id` `custom` in `<root>/x/dir-name`
- WHEN its assets are requested
- THEN they are served from `<root>/x/dir-name`

#### Scenario: Flat asset regression
- GIVEN a flat overlay at `<root>/lower-third`
- WHEN its assets are requested
- THEN they are served exactly as before

#### Scenario: Traversal rejected
- WHEN an asset path contains `..` segments escaping the overlay directory
- THEN the request is rejected

### Requirement: Explorer home page
The home page MUST render the contents of a current folder, starting at the root, showing folder cards followed by overlay cards. Folder cards MUST precede loose overlays; each group MUST be sorted alphabetically. Empty folders MUST be shown. Each folder card MUST display the count of overlays directly inside it (not recursive). Activating a folder card MUST make it the current folder. A breadcrumb MUST show the path from the root to the current folder, and activating any breadcrumb segment MUST navigate to that folder. The current folder is not persisted across app restarts. The home page currently has no search or filter; this change MUST NOT add one.

#### Scenario: Root view ordering
- GIVEN root contains folders `b`, `a` and overlays `z`, `m`
- THEN the order shown is `a`, `b`, `m`, `z`

#### Scenario: Direct count
- GIVEN folder `a` directly holds 2 overlays and its subfolder `a/sub` holds 3
- THEN the `a` card shows 2

#### Scenario: Empty folder card
- GIVEN an empty folder `empty`
- THEN a card for `empty` is shown with count 0, and opening it shows an empty state

#### Scenario: Navigate in and via breadcrumb
- GIVEN the user opens `a` then `a/sub`
- THEN the breadcrumb shows root > a > sub and the grid shows the contents of `a/sub`
- WHEN the user activates `a` in the breadcrumb
- THEN the grid shows the contents of `a`

#### Scenario: Restart resets location
- GIVEN the user is inside `a/sub`
- WHEN the app restarts
- THEN the home page shows the root

#### Scenario: Flat setup unchanged
- GIVEN only flat overlays and no folders
- THEN the home page shows the overlay cards at the root with no folder cards

### Requirement: Localization
All new UI strings (breadcrumb root label, empty folder state, folder count label, navigation labels) MUST be provided in both `es` and `en` locales.

#### Scenario: Both locales
- WHEN the language is switched between Spanish and English
- THEN all new explorer strings are translated

## Out of Scope
- Startup lag and performance
- Folder creation, move, or rename from the UI; expandable tree view; persisting the current folder
- UI indication of skipped duplicate ids; search or filtering
