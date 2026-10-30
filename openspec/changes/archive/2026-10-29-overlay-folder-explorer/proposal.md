# Proposal: Overlay Folder Explorer

## Intent
The home page is a flat grid that reads one directory level. Users with many overlays (18+) can't group them, and nested overlays aren't discovered. Goal: the home page works like a file explorer that mirrors the overlays dir on disk (folders plus loose overlays at any level), without breaking template ids, presets, OBS sources, or the `Manifest` API.

## Scope
### In Scope
- Recursive discovery: depth limit (~5), skip hidden dirs, don't follow symlinks, a dir with `overlay.json` is a leaf
- Additive relative `folder` field on `TemplateInfo`, plus a folder list so empty folders show
- Fix `resolve_asset_path` (`fs_template_source.rs:100`) to resolve from the template's real dir
- Home page with breadcrumb/current-folder navigation (folder cards plus overlay cards) and i18n strings in es/en

### Out of Scope
- Startup lag / performance (separate future investigation, measured in a release build)
- Changing the template id or the WS payload shape
- Creating, moving, or renaming folders from the UI; expandable tree; persisting the current folder

## Capabilities
### New Capabilities
- `overlay-catalog-browsing`: recursive discovery rules, folder metadata, id-collision handling, explorer navigation

### Modified Capabilities
- None

## Approach
Backend keeps returning a flat list (`Manifest`-compatible) and adds `folder` to each template plus a known-folders list. The frontend builds the tree and renders the current folder. Id = `meta.id || dir name`. Duplicate ids across folders: the first one found wins, the duplicate is skipped, and an `eprintln` warning is logged. Strict TDD on the Rust walk and asset resolution.

**Assumptions (defaults, pending user review):** collision = skip + warn; empty folders are shown; breadcrumb navigation, not a tree; depth ~5.

## Affected Areas
| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/infrastructure/fs_template_source.rs` | Modified | recursive walk, asset path fix |
| `src-tauri/src/domain/template.rs` | Modified | `folder` field, folder list |
| `src/stores/templates.ts`, `src/types/` | Modified | tree derivation, current folder |
| `src/components/OverlaysPage.vue`, `OverlayGrid.vue`, new folder card/breadcrumb | Modified/New | explorer UI |
| `src/i18n/locales/{es,en}.json` | Modified | new strings |

## Risks
| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Merge conflicts with `remote-overlay-control` (`OverlaysPage.vue`, `stores/templates.ts`, `Manifest`) | High | sequence the changes; additive-only `Manifest` fields |
| Id collisions hide an overlay silently | Med | skip and warn; spec the deterministic order |
| Asset resolution regression for flat overlays | Low | tests for flat, nested, and `meta.id != dir` |
| Deep or large trees slow the walk | Low | depth limit, no symlinks |

## Rollback Plan
Revert the PRs in reverse order. The new fields are additive and unused by OBS/WS, so reverting restores the flat grid. Overlays placed in subfolders would stop being discovered until they are moved back to the root. No data migration.

## Dependencies
- Coordinate merge order with `remote-overlay-control`

## Success Criteria
- [ ] Nested overlays are discovered and served (`/overlay/{folder}/.../index.html` and their assets)
- [ ] Existing flat overlays, presets, and OBS URLs work unchanged
- [ ] Home page lets you navigate folders via breadcrumb; empty folders appear
- [ ] Duplicate ids are skipped with a warning (cargo test)
- [ ] `Manifest` consumers keep working (no fields removed or renamed)

## Proposal question round
Pending user review:
1. Collisions: is "first found wins + log warning" enough, or should the UI surface skipped duplicates?
2. Should a folder show a count of the overlays it contains (recursive or direct)?
3. Should search or filtering span all folders, or only the current folder?
4. Should "loose" overlays at the root stay mixed with the folders, or be listed after them?
