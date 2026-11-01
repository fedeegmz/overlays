use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};

use super::group_preset_service::GroupPresetService;
use super::ports::{InstanceEvents, OverlayBus, TemplateLookup};
use super::preset_service::PresetService;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::group_preset::{GroupPreset, GroupPresetEntry};
use crate::domain::instance::{
    defaults_for, fields_for, EventSource, GroupStartFailure, GroupStartResult, Instance,
    InstancesSnapshot, Registry, SourceKind,
};
use crate::domain::overlay::{OverlayAction, OverlayPayload};
use crate::domain::template::{Manifest, TemplateInfo};

struct State {
    registry: Registry,
    revision: u64,
}

/// Owner of the live overlay instances (in memory, never persisted).
///
/// Every mutation runs under one lock: it bumps the revision, publishes the
/// overlay payloads it implies and then the full snapshot. Publishing while
/// still holding the lock keeps delivery order equal to revision order; both
/// ports are non-blocking and must not call back into the service.
///
/// The template catalog is read once per operation (the real one re-walks the
/// overlays tree on every call), before the lock is taken.
pub struct InstanceService {
    lookup: Arc<dyn TemplateLookup>,
    bus: Arc<dyn OverlayBus>,
    events: Arc<dyn InstanceEvents>,
    presets: Arc<PresetService>,
    groups: Arc<GroupPresetService>,
    state: Mutex<State>,
}

fn find_template<'a>(manifest: &'a Manifest, id: &str) -> Option<&'a TemplateInfo> {
    manifest.templates.iter().find(|t| t.id == id)
}

fn payload(
    instance: &Instance,
    action: OverlayAction,
    fields: HashMap<String, String>,
) -> OverlayPayload {
    OverlayPayload {
        instance_id: instance.id.clone(),
        template: instance.template_id.clone(),
        action,
        fields,
    }
}

fn normalize(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl InstanceService {
    pub fn new(
        lookup: Arc<dyn TemplateLookup>,
        bus: Arc<dyn OverlayBus>,
        events: Arc<dyn InstanceEvents>,
        presets: Arc<PresetService>,
        groups: Arc<GroupPresetService>,
    ) -> Self {
        Self {
            lookup,
            bus,
            events,
            presets,
            groups,
            state: Mutex::new(State {
                registry: Registry::new(),
                revision: 0,
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// For operations that only need template names for display: an
    /// unreadable catalog degrades labels to template ids instead of failing.
    fn manifest_lossy(&self) -> Manifest {
        self.lookup.manifest().unwrap_or(Manifest {
            templates: Vec::new(),
            folders: Vec::new(),
        })
    }

    fn snapshot(state: &State, manifest: &Manifest, source: EventSource) -> InstancesSnapshot {
        InstancesSnapshot {
            revision: state.revision,
            source,
            instances: state.registry.dtos(manifest),
        }
    }

    /// Bumps the revision, publishes `payloads` to the overlays and the new
    /// snapshot to the event consumers.
    fn commit(
        &self,
        state: &mut State,
        manifest: &Manifest,
        payloads: &[OverlayPayload],
        source: EventSource,
    ) -> InstancesSnapshot {
        state.revision += 1;
        for payload in payloads {
            self.bus.publish(payload);
        }
        let snapshot = Self::snapshot(state, manifest, source);
        self.events.changed(&snapshot);
        snapshot
    }

    pub fn list(&self, source: EventSource) -> InstancesSnapshot {
        let manifest = self.manifest_lossy();
        Self::snapshot(&self.lock(), &manifest, source)
    }

    pub fn create(
        &self,
        template_id: &str,
        id: Option<String>,
        custom_name: Option<String>,
        source: EventSource,
    ) -> DomainResult<(String, InstancesSnapshot)> {
        let manifest = self.lookup.manifest()?;
        let template =
            find_template(&manifest, template_id).ok_or_else(|| DomainError::TemplateNotFound {
                template: template_id.to_string(),
            })?;
        let mut state = self.lock();
        let id = match id.filter(|i| !i.is_empty()) {
            Some(id) if state.registry.contains(&id) => {
                return Err(DomainError::InstanceIdTaken { id });
            }
            Some(id) => id,
            None => uuid::Uuid::new_v4().to_string(),
        };
        let number = state.registry.next_number(template_id);
        state.registry.push(Instance {
            id: id.clone(),
            template_id: template_id.to_string(),
            number,
            custom_name: normalize(custom_name),
            fields: defaults_for(template),
            visible: false,
        });
        let snapshot = self.commit(&mut state, &manifest, &[], source);
        Ok((id, snapshot))
    }

    /// Removes the instance, hiding it on the overlays first when visible.
    pub fn remove(&self, id: &str, source: EventSource) -> DomainResult<InstancesSnapshot> {
        let manifest = self.manifest_lossy();
        let mut state = self.lock();
        let removed = state
            .registry
            .remove(id)
            .ok_or_else(|| DomainError::InstanceNotFound { id: id.to_string() })?;
        let payloads: Vec<_> = removed
            .visible
            .then(|| payload(&removed, OverlayAction::Hide, HashMap::new()))
            .into_iter()
            .collect();
        Ok(self.commit(&mut state, &manifest, &payloads, source))
    }

    /// A blank name clears the custom name, restoring the auto-numbered label.
    pub fn rename(
        &self,
        id: &str,
        name: &str,
        source: EventSource,
    ) -> DomainResult<InstancesSnapshot> {
        let manifest = self.manifest_lossy();
        let mut state = self.lock();
        let instance = state
            .registry
            .get_mut(id)
            .ok_or_else(|| DomainError::InstanceNotFound { id: id.to_string() })?;
        instance.custom_name = normalize(Some(name.to_string()));
        Ok(self.commit(&mut state, &manifest, &[], source))
    }

    /// Moves an instance; `to` is clamped. An out-of-range `from` or a move
    /// that changes nothing is not a mutation: no revision bump, no event.
    pub fn move_instance(&self, from: usize, to: usize, source: EventSource) -> InstancesSnapshot {
        let manifest = self.manifest_lossy();
        let mut state = self.lock();
        if state.registry.move_to(from, to) {
            self.commit(&mut state, &manifest, &[], source)
        } else {
            Self::snapshot(&state, &manifest, source)
        }
    }

    /// Replaces the stored fields. State only: nothing reaches the overlays
    /// until `show` or `update`.
    pub fn set_fields(
        &self,
        id: &str,
        fields: HashMap<String, String>,
        source: EventSource,
    ) -> DomainResult<InstancesSnapshot> {
        let manifest = self.manifest_lossy();
        let mut state = self.lock();
        let instance = state
            .registry
            .get_mut(id)
            .ok_or_else(|| DomainError::InstanceNotFound { id: id.to_string() })?;
        instance.fields = fields;
        Ok(self.commit(&mut state, &manifest, &[], source))
    }

    pub fn show(&self, id: &str, source: EventSource) -> DomainResult<InstancesSnapshot> {
        self.send_fields(id, OverlayAction::Show, HashMap::new(), source)
    }

    pub fn update(&self, id: &str, source: EventSource) -> DomainResult<InstancesSnapshot> {
        self.send_fields(id, OverlayAction::Update, HashMap::new(), source)
    }

    /// `show` after merging `patch` over the stored fields, in one revision.
    /// A remote source may only touch fields the template declares and that
    /// are not of type `file`; other keys are dropped.
    pub fn show_with(
        &self,
        id: &str,
        patch: HashMap<String, String>,
        source: EventSource,
    ) -> DomainResult<InstancesSnapshot> {
        self.send_fields(id, OverlayAction::Show, patch, source)
    }

    /// `update` after merging `patch`; see `show_with` for the filtering.
    pub fn update_with(
        &self,
        id: &str,
        patch: HashMap<String, String>,
        source: EventSource,
    ) -> DomainResult<InstancesSnapshot> {
        self.send_fields(id, OverlayAction::Update, patch, source)
    }

    pub fn hide(&self, id: &str, source: EventSource) -> DomainResult<InstancesSnapshot> {
        let manifest = self.manifest_lossy();
        let mut state = self.lock();
        let instance = state
            .registry
            .get_mut(id)
            .ok_or_else(|| DomainError::InstanceNotFound { id: id.to_string() })?;
        instance.visible = false;
        let message = payload(instance, OverlayAction::Hide, HashMap::new());
        Ok(self.commit(&mut state, &manifest, &[message], source))
    }

    /// `show` and `update`: publishes the stored fields, keyed by every field
    /// the template declares. `show` also marks the instance visible.
    fn send_fields(
        &self,
        id: &str,
        action: OverlayAction,
        patch: HashMap<String, String>,
        source: EventSource,
    ) -> DomainResult<InstancesSnapshot> {
        let manifest = self.lookup.manifest()?;
        let mut state = self.lock();
        let instance = state
            .registry
            .get_mut(id)
            .ok_or_else(|| DomainError::InstanceNotFound { id: id.to_string() })?;
        let template = find_template(&manifest, &instance.template_id).ok_or_else(|| {
            DomainError::TemplateNotFound {
                template: instance.template_id.clone(),
            }
        })?;
        let remote = source.kind == SourceKind::Remote;
        instance.fields.extend(patch.into_iter().filter(|(key, _)| {
            !remote
                || template
                    .fields
                    .iter()
                    .any(|f| f.key == *key && f.field_type != "file")
        }));
        let message = payload(instance, action, fields_for(instance, template));
        if action == OverlayAction::Show {
            instance.visible = true;
        }
        Ok(self.commit(&mut state, &manifest, &[message], source))
    }

    /// Applies a per-template preset like the desktop always did: the stored
    /// fields become exactly the preset's, a `show` goes to the overlays and
    /// the instance is marked visible, whether or not it was shown before.
    /// The preset is looked up by name among those of the instance's template.
    pub fn apply_preset(
        &self,
        id: &str,
        name: &str,
        source: EventSource,
    ) -> DomainResult<InstancesSnapshot> {
        let manifest = self.lookup.manifest()?;
        let presets = self.presets.list();
        let mut state = self.lock();
        let instance = state
            .registry
            .get_mut(id)
            .ok_or_else(|| DomainError::InstanceNotFound { id: id.to_string() })?;
        if find_template(&manifest, &instance.template_id).is_none() {
            return Err(DomainError::TemplateNotFound {
                template: instance.template_id.clone(),
            });
        }
        let preset = presets
            .iter()
            .find(|p| p.name == name && p.template == instance.template_id)
            .ok_or_else(|| DomainError::PresetNotFound {
                name: name.to_string(),
            })?;
        instance.fields = preset.fields.clone();
        instance.visible = true;
        let message = payload(instance, OverlayAction::Show, preset.fields.clone());
        Ok(self.commit(&mut state, &manifest, &[message], source))
    }

    /// Starts a saved group. Each entry follows the first matching rule:
    /// 1. its saved id is live with the same template: update in place (the
    ///    saved name replaces the live one only when present);
    /// 2. its saved id is free: create with that id and the saved name;
    /// 3. its saved id is live with another template (or already claimed by
    ///    an earlier entry): create with a fresh id and flag the conflict;
    /// 4. no saved id: create with a fresh id.
    ///
    /// Everything is resolved against one catalog read under one lock and
    /// ends in a single revision bump and snapshot. An entry whose template
    /// is not in the catalog is reported and skipped; if no entry started,
    /// nothing is bumped or emitted.
    pub fn start_group(
        &self,
        name: &str,
        source: EventSource,
    ) -> DomainResult<(Vec<GroupStartResult>, InstancesSnapshot)> {
        let group = self
            .groups
            .list()?
            .into_iter()
            .find(|g| g.name == name)
            .ok_or_else(|| DomainError::GroupPresetNotFound {
                name: name.to_string(),
            })?;
        let manifest = self.lookup.manifest()?;
        let mut state = self.lock();
        let mut results = Vec::with_capacity(group.entries.len());
        let mut payloads = Vec::new();
        let mut claimed: HashSet<String> = HashSet::new();

        for entry in &group.entries {
            let Some(template) = find_template(&manifest, &entry.template) else {
                results.push(GroupStartResult::failed(
                    &entry.template,
                    GroupStartFailure::TemplateMissing,
                ));
                continue;
            };
            let mut fields = defaults_for(template);
            fields.extend(entry.fields.clone());
            let saved_id = entry
                .instance_id
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty());
            let saved_name = entry
                .custom_name
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty());

            let live = saved_id.and_then(|i| state.registry.get(i));
            let live_exists = live.is_some();
            let reuse = !saved_id.is_some_and(|i| claimed.contains(i))
                && live.is_some_and(|l| l.template_id == entry.template);

            let (id, id_conflict) = match (reuse, saved_id) {
                (true, Some(saved)) => {
                    if let Some(live) = state.registry.get_mut(saved) {
                        live.fields = fields;
                        if let Some(n) = saved_name {
                            live.custom_name = Some(n.to_string());
                        }
                    }
                    (saved.to_string(), false)
                }
                _ => {
                    let free = saved_id.filter(|i| !live_exists && !claimed.contains(*i));
                    let id = free
                        .map(str::to_string)
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                    let number = state.registry.next_number(&entry.template);
                    state.registry.push(Instance {
                        id: id.clone(),
                        template_id: entry.template.clone(),
                        number,
                        custom_name: saved_name.map(str::to_string),
                        fields,
                        visible: false,
                    });
                    (id, saved_id.is_some() && free.is_none())
                }
            };
            claimed.insert(id.clone());

            if let Some(instance) = state.registry.get_mut(&id) {
                instance.visible = true;
                payloads.push(payload(
                    instance,
                    OverlayAction::Show,
                    fields_for(instance, template),
                ));
            }
            results.push(GroupStartResult::started(&entry.template, &id, id_conflict));
        }

        let snapshot = if payloads.is_empty() {
            Self::snapshot(&state, &manifest, source)
        } else {
            self.commit(&mut state, &manifest, &payloads, source)
        };
        Ok((results, snapshot))
    }

    /// Saves every live instance (hidden included, registry order) as a
    /// group. Only the user-set custom name is recorded, never the
    /// auto-numbered label. Does not touch the registry.
    pub fn save_group_from_instances(&self, name: &str) -> DomainResult<Vec<GroupPreset>> {
        let manifest = self.lookup.manifest()?;
        let entries = self
            .lock()
            .registry
            .instances()
            .iter()
            .map(|i| GroupPresetEntry {
                template: i.template_id.clone(),
                fields: find_template(&manifest, &i.template_id)
                    .map(|t| fields_for(i, t))
                    .unwrap_or_else(|| i.fields.clone()),
                instance_id: Some(i.id.clone()),
                custom_name: i.custom_name.clone(),
            })
            .collect();
        self.groups.save(name.to_string(), entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{GroupPresetRepository, PresetRepository};
    use crate::domain::preset::Preset;
    use crate::domain::template::OverlayField;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeEvents(Mutex<Vec<InstancesSnapshot>>);

    impl InstanceEvents for FakeEvents {
        fn changed(&self, snapshot: &InstancesSnapshot) {
            self.0.lock().unwrap().push(snapshot.clone());
        }
    }

    struct FakeBus(Mutex<Vec<OverlayPayload>>);

    impl OverlayBus for FakeBus {
        fn publish(&self, payload: &OverlayPayload) {
            self.0.lock().unwrap().push(payload.clone());
        }

        fn snapshot(&self) -> Vec<OverlayPayload> {
            self.0.lock().unwrap().clone()
        }
    }

    struct FakeLookup {
        manifest: Mutex<Manifest>,
        reads: AtomicUsize,
    }

    impl TemplateLookup for FakeLookup {
        fn manifest(&self) -> DomainResult<Manifest> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            Ok(self.manifest.lock().unwrap().clone())
        }
    }

    struct MemPresets(Mutex<Vec<Preset>>);

    impl PresetRepository for MemPresets {
        fn load(&self) -> Vec<Preset> {
            self.0.lock().unwrap().clone()
        }

        fn save(&self, presets: &[Preset]) -> DomainResult<()> {
            *self.0.lock().unwrap() = presets.to_vec();
            Ok(())
        }
    }

    struct MemGroups(Mutex<Vec<GroupPreset>>);

    impl GroupPresetRepository for MemGroups {
        fn load(&self) -> DomainResult<Vec<GroupPreset>> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn save(&self, groups: &[GroupPreset]) -> DomainResult<()> {
            *self.0.lock().unwrap() = groups.to_vec();
            Ok(())
        }
    }

    fn field(key: &str, field_type: &str, default: Option<&str>, min: Option<f64>) -> OverlayField {
        OverlayField {
            key: key.into(),
            label: key.into(),
            field_type: field_type.into(),
            default: default.map(str::to_string),
            min,
            max: None,
            step: None,
            accept: None,
        }
    }

    fn template(id: &str, name: &str, fields: Vec<OverlayField>) -> TemplateInfo {
        TemplateInfo {
            id: id.into(),
            name: name.into(),
            path: format!("{id}/index.html"),
            folder: String::new(),
            fields,
        }
    }

    fn manifest() -> Manifest {
        Manifest {
            templates: vec![
                template(
                    "title",
                    "Title",
                    vec![
                        field("text", "text", Some("Hello"), None),
                        field("pct", "progress", None, Some(5.0)),
                        field("flag", "boolean", None, None),
                        field("note", "text", None, None),
                    ],
                ),
                template("lower", "Lower", vec![field("text", "text", None, None)]),
            ],
            folders: vec![],
        }
    }

    struct Fixture {
        svc: InstanceService,
        events: Arc<FakeEvents>,
        bus: Arc<FakeBus>,
        lookup: Arc<FakeLookup>,
        presets: Arc<PresetService>,
        groups: Arc<GroupPresetService>,
    }

    fn fixture() -> Fixture {
        let events = Arc::new(FakeEvents(Mutex::new(Vec::new())));
        let bus = Arc::new(FakeBus(Mutex::new(Vec::new())));
        let lookup = Arc::new(FakeLookup {
            manifest: Mutex::new(manifest()),
            reads: AtomicUsize::new(0),
        });
        let presets = Arc::new(PresetService::new(Arc::new(MemPresets(Mutex::new(
            Vec::new(),
        )))));
        let groups = Arc::new(GroupPresetService::new(Arc::new(MemGroups(Mutex::new(
            Vec::new(),
        )))));
        let svc = InstanceService::new(
            lookup.clone(),
            bus.clone(),
            events.clone(),
            presets.clone(),
            groups.clone(),
        );
        Fixture {
            svc,
            events,
            bus,
            lookup,
            presets,
            groups,
        }
    }

    fn desktop() -> EventSource {
        EventSource::desktop("win-1")
    }

    fn create(f: &Fixture, template: &str) -> String {
        f.svc.create(template, None, None, desktop()).unwrap().0
    }

    fn ids(snap: &InstancesSnapshot) -> Vec<&str> {
        snap.instances.iter().map(|i| i.id.as_str()).collect()
    }

    fn patch(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn update_with_fields_merges_patch_publishes_once_and_bumps_once() {
        let f = fixture();
        let id = create(&f, "title");

        let snap = f
            .svc
            .update_with(&id, patch(&[("text", "Live")]), EventSource::remote("ph"))
            .unwrap();

        assert_eq!(snap.revision, 2);
        assert_eq!(snap.source, EventSource::remote("ph"));
        assert_eq!(snap.instances[0].fields["text"], "Live");
        assert_eq!(
            snap.instances[0].fields["pct"], "5",
            "untouched fields stay"
        );
        let published = f.bus.0.lock().unwrap();
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].action, OverlayAction::Update);
        assert_eq!(published[0].fields["text"], "Live");
    }

    #[test]
    fn show_with_fields_marks_visible_and_publishes_show() {
        let f = fixture();
        let id = create(&f, "title");

        let snap = f
            .svc
            .show_with(&id, patch(&[("note", "n")]), EventSource::remote("ph"))
            .unwrap();

        assert!(snap.instances[0].visible);
        let published = f.bus.0.lock().unwrap();
        assert_eq!(published[0].action, OverlayAction::Show);
        assert_eq!(published[0].fields["note"], "n");
    }

    #[test]
    fn remote_patch_drops_file_type_and_unknown_keys() {
        let f = fixture();
        f.lookup.manifest.lock().unwrap().templates.push(template(
            "media",
            "Media",
            vec![
                field("caption", "text", None, None),
                field("photo", "file", None, None),
            ],
        ));
        let id = create(&f, "media");
        f.svc
            .set_fields(&id, patch(&[("photo", "/a.png")]), desktop())
            .unwrap();

        let snap = f
            .svc
            .update_with(
                &id,
                patch(&[("caption", "c"), ("photo", "/evil.png"), ("ghost", "x")]),
                EventSource::remote("ph"),
            )
            .unwrap();

        let fields = &snap.instances[0].fields;
        assert_eq!(fields["caption"], "c");
        assert_eq!(fields["photo"], "/a.png");
        assert!(!fields.contains_key("ghost"));
        let published = f.bus.0.lock().unwrap();
        assert_eq!(published[0].fields["photo"], "/a.png");
        assert!(!published[0].fields.contains_key("ghost"));
    }

    #[test]
    fn patch_for_unknown_instance_returns_instance_not_found() {
        let f = fixture();
        let err = f
            .svc
            .update_with("nope", HashMap::new(), EventSource::remote("ph"))
            .unwrap_err();
        assert!(matches!(err, DomainError::InstanceNotFound { .. }));
    }

    #[test]
    fn create_assigns_defaults_from_template() {
        let f = fixture();
        let (id, snap) = f.svc.create("title", None, None, desktop()).unwrap();

        let created = &snap.instances[0];
        assert_eq!(created.id, id);
        assert!(!id.is_empty());
        assert_eq!(created.fields["text"], "Hello");
        assert_eq!(created.fields["pct"], "5");
        assert_eq!(created.fields["flag"], "false");
        assert!(!created.fields.contains_key("note"));
        assert!(!created.visible);
        assert_eq!(created.number, 1);
    }

    #[test]
    fn create_unknown_template_returns_template_not_found() {
        let f = fixture();
        let err = f.svc.create("nope", None, None, desktop()).unwrap_err();

        assert!(matches!(err, DomainError::TemplateNotFound { template } if template == "nope"));
        assert!(f.svc.list(desktop()).instances.is_empty());
        assert!(f.events.0.lock().unwrap().is_empty());
    }

    #[test]
    fn create_with_custom_id_and_name() {
        let f = fixture();
        let (id, snap) = f
            .svc
            .create(
                "title",
                Some("abc".into()),
                Some("  Main  ".into()),
                desktop(),
            )
            .unwrap();

        assert_eq!(id, "abc");
        assert_eq!(snap.instances[0].custom_name.as_deref(), Some("Main"));
        assert_eq!(snap.instances[0].display_name, "Main");
    }

    #[test]
    fn create_without_id_generates_distinct_ids_appended_in_order() {
        let f = fixture();
        let a = create(&f, "title");
        let b = create(&f, "lower");

        assert_ne!(a, b);
        assert_eq!(ids(&f.svc.list(desktop())), [a.as_str(), b.as_str()]);
    }

    #[test]
    fn create_duplicate_id_returns_instance_id_taken() {
        let f = fixture();
        f.svc
            .create("title", Some("abc".into()), None, desktop())
            .unwrap();
        let before = f.events.0.lock().unwrap().len();

        let err = f
            .svc
            .create("lower", Some("abc".into()), None, desktop())
            .unwrap_err();

        assert!(matches!(err, DomainError::InstanceIdTaken { id } if id == "abc"));
        assert_eq!(f.svc.list(desktop()).instances.len(), 1);
        assert_eq!(f.events.0.lock().unwrap().len(), before);
    }

    #[test]
    fn auto_numbering_labels_second_instance_of_same_template() {
        let f = fixture();
        create(&f, "title");
        let second = create(&f, "title");

        let snap = f.svc.list(desktop());
        let names: Vec<_> = snap
            .instances
            .iter()
            .map(|i| i.display_name.as_str())
            .collect();
        assert_eq!(names, ["Title #1", "Title #2"]);
        let last = snap.instances.iter().find(|i| i.id == second).unwrap();
        assert_eq!(last.number, 2);
        assert!(last.custom_name.is_none());
    }

    #[test]
    fn display_name_ports_instance_default_name() {
        let f = fixture();
        let a = create(&f, "title");
        let b = create(&f, "title");
        let other = create(&f, "lower");

        // Removing the first leaves #2 alone, so the label drops its number.
        let snap = f.svc.remove(&a, desktop()).unwrap();
        let name = |id: &str| {
            snap.instances
                .iter()
                .find(|i| i.id == id)
                .unwrap()
                .display_name
                .clone()
        };
        assert_eq!(name(&b), "Title");
        assert_eq!(name(&other), "Lower");

        // The freed number is reused.
        let (c, snap) = f.svc.create("title", None, None, desktop()).unwrap();
        assert_eq!(snap.instances.iter().find(|i| i.id == c).unwrap().number, 1);
    }

    #[test]
    fn rename_sets_custom_name() {
        let f = fixture();
        let id = create(&f, "title");

        let snap = f.svc.rename(&id, "  Headline ", desktop()).unwrap();
        assert_eq!(snap.instances[0].custom_name.as_deref(), Some("Headline"));
        assert_eq!(snap.instances[0].display_name, "Headline");

        let snap = f.svc.rename(&id, "   ", desktop()).unwrap();
        assert!(snap.instances[0].custom_name.is_none());
        assert_eq!(snap.instances[0].display_name, "Title");

        assert!(matches!(
            f.svc.rename("ghost", "x", desktop()),
            Err(DomainError::InstanceNotFound { .. })
        ));
    }

    #[test]
    fn remove_unknown_returns_instance_not_found() {
        let f = fixture();
        create(&f, "title");
        let before = f.events.0.lock().unwrap().len();

        let err = f.svc.remove("ghost", desktop()).unwrap_err();

        assert!(matches!(err, DomainError::InstanceNotFound { id } if id == "ghost"));
        assert_eq!(f.svc.list(desktop()).instances.len(), 1);
        assert_eq!(f.events.0.lock().unwrap().len(), before);
    }

    #[test]
    fn remove_hides_a_visible_instance_before_dropping_it() {
        let f = fixture();
        let shown = create(&f, "title");
        let hidden = create(&f, "title");
        f.svc.show(&shown, desktop()).unwrap();
        f.bus.0.lock().unwrap().clear();

        f.svc.remove(&hidden, desktop()).unwrap();
        assert!(f.bus.0.lock().unwrap().is_empty());

        let snap = f.svc.remove(&shown, desktop()).unwrap();
        assert!(snap.instances.is_empty());
        let published = f.bus.0.lock().unwrap();
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].action, OverlayAction::Hide);
        assert_eq!(published[0].instance_id, shown);
    }

    #[test]
    fn move_reorders_and_clamps() {
        let f = fixture();
        let a = create(&f, "title");
        let b = create(&f, "title");
        let c = create(&f, "title");

        let snap = f.svc.move_instance(2, 0, desktop());
        assert_eq!(ids(&snap), [c.as_str(), a.as_str(), b.as_str()]);

        let snap = f.svc.move_instance(0, 99, desktop());
        assert_eq!(ids(&snap), [a.as_str(), b.as_str(), c.as_str()]);
    }

    #[test]
    fn move_without_effect_does_not_bump_or_emit() {
        let f = fixture();
        create(&f, "title");
        create(&f, "title");
        let before_events = f.events.0.lock().unwrap().len();
        let before_rev = f.svc.list(desktop()).revision;

        let snap = f.svc.move_instance(5, 0, desktop());
        let snap2 = f.svc.move_instance(1, 1, desktop());

        assert_eq!(snap.revision, before_rev);
        assert_eq!(snap2.revision, before_rev);
        assert_eq!(f.events.0.lock().unwrap().len(), before_events);
    }

    #[test]
    fn every_mutation_bumps_revision_monotonically() {
        let f = fixture();
        let (id, s1) = f.svc.create("title", None, None, desktop()).unwrap();
        let s2 = f.svc.rename(&id, "x", desktop()).unwrap();
        let s3 = f.svc.set_fields(&id, HashMap::new(), desktop()).unwrap();
        let s4 = f.svc.show(&id, desktop()).unwrap();
        let s5 = f.svc.update(&id, desktop()).unwrap();
        let s6 = f.svc.hide(&id, desktop()).unwrap();
        let (_, s7) = f.svc.create("lower", None, None, desktop()).unwrap();
        let s8 = f.svc.move_instance(1, 0, desktop());
        let s9 = f.svc.remove(&id, desktop()).unwrap();

        let revisions: Vec<_> = [s1, s2, s3, s4, s5, s6, s7, s8, s9]
            .iter()
            .map(|s| s.revision)
            .collect();
        assert_eq!(revisions, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn every_mutation_emits_full_snapshot_with_source() {
        let f = fixture();
        let remote = EventSource::remote("phone-1");
        let (id, _) = f.svc.create("title", None, None, desktop()).unwrap();
        f.svc.create("lower", None, None, desktop()).unwrap();
        let returned = f.svc.rename(&id, "Named", remote.clone()).unwrap();

        let events = f.events.0.lock().unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].source, desktop());
        let last = events.last().unwrap();
        assert_eq!(last.source, remote);
        assert_eq!(last.instances.len(), 2);
        assert_eq!(*last, returned);
    }

    #[test]
    fn list_does_not_bump_or_emit() {
        let f = fixture();
        create(&f, "title");

        let a = f.svc.list(desktop());
        let b = f.svc.list(desktop());

        assert_eq!(a.revision, 1);
        assert_eq!(b.revision, 1);
        assert_eq!(f.events.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn set_fields_updates_state_only_no_overlay_publish() {
        let f = fixture();
        let id = create(&f, "title");
        let fields = HashMap::from([("text".to_string(), "Edited".to_string())]);

        let snap = f.svc.set_fields(&id, fields.clone(), desktop()).unwrap();

        assert_eq!(snap.instances[0].fields, fields);
        assert!(!snap.instances[0].visible);
        assert!(f.bus.0.lock().unwrap().is_empty());
        assert!(matches!(
            f.svc.set_fields("ghost", HashMap::new(), desktop()),
            Err(DomainError::InstanceNotFound { .. })
        ));
    }

    fn payload_fields(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn show_publishes_unchanged_ws_payload() {
        let f = fixture();
        let id = create(&f, "title");
        f.svc
            .set_fields(
                &id,
                payload_fields(&[("text", "Hi"), ("stale", "x")]),
                desktop(),
            )
            .unwrap();

        let snap = f.svc.show(&id, desktop()).unwrap();

        assert!(snap.instances[0].visible);
        let published = f.bus.0.lock().unwrap();
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].instance_id, id);
        assert_eq!(published[0].template, "title");
        assert_eq!(published[0].action, OverlayAction::Show);
        // Every declared key is present ("" when unset); undeclared keys are not.
        assert_eq!(
            published[0].fields,
            payload_fields(&[("text", "Hi"), ("pct", ""), ("flag", ""), ("note", "")])
        );
        assert_eq!(
            serde_json::to_value(&published[0]).unwrap()["action"],
            serde_json::json!("show")
        );
    }

    #[test]
    fn update_publishes_payload() {
        let f = fixture();
        let id = create(&f, "lower");
        f.svc
            .set_fields(&id, payload_fields(&[("text", "v2")]), desktop())
            .unwrap();

        let snap = f.svc.update(&id, desktop()).unwrap();

        // Update does not change visibility.
        assert!(!snap.instances[0].visible);
        let published = f.bus.0.lock().unwrap();
        assert_eq!(published[0].action, OverlayAction::Update);
        assert_eq!(published[0].template, "lower");
        assert_eq!(published[0].fields, payload_fields(&[("text", "v2")]));
    }

    #[test]
    fn hide_publishes_payload() {
        let f = fixture();
        let id = create(&f, "title");
        f.svc.show(&id, desktop()).unwrap();

        let snap = f.svc.hide(&id, desktop()).unwrap();

        assert!(!snap.instances[0].visible);
        let published = f.bus.0.lock().unwrap();
        assert_eq!(published.len(), 2);
        assert_eq!(published[1].action, OverlayAction::Hide);
        assert_eq!(published[1].instance_id, id);
        assert_eq!(published[1].template, "title");
        assert!(published[1].fields.is_empty());
    }

    #[test]
    fn actions_on_unknown_instance_fail_and_emit_nothing() {
        let f = fixture();
        assert!(matches!(
            f.svc.show("ghost", desktop()),
            Err(DomainError::InstanceNotFound { .. })
        ));
        assert!(matches!(
            f.svc.update("ghost", desktop()),
            Err(DomainError::InstanceNotFound { .. })
        ));
        assert!(matches!(
            f.svc.hide("ghost", desktop()),
            Err(DomainError::InstanceNotFound { .. })
        ));
        assert!(f.bus.0.lock().unwrap().is_empty());
        assert!(f.events.0.lock().unwrap().is_empty());
    }

    // ---- apply_preset ----

    fn save_preset(f: &Fixture, name: &str, template: &str, fields: &[(&str, &str)]) {
        f.presets
            .save(name.into(), template.into(), payload_fields(fields))
            .unwrap();
    }

    fn clear_outputs(f: &Fixture) {
        f.bus.0.lock().unwrap().clear();
        f.events.0.lock().unwrap().clear();
    }

    fn remove_template_from_catalog(f: &Fixture, id: &str) {
        f.lookup
            .manifest
            .lock()
            .unwrap()
            .templates
            .retain(|t| t.id != id);
    }

    #[test]
    fn apply_preset_sets_fields_by_name_and_template() {
        let f = fixture();
        let id = create(&f, "title");
        save_preset(&f, "Intro", "title", &[("text", "From preset")]);

        let snap = f.svc.apply_preset(&id, "Intro", desktop()).unwrap();

        // Fields are exactly the preset's, not merged with the old ones.
        assert_eq!(
            snap.instances[0].fields,
            payload_fields(&[("text", "From preset")])
        );
    }

    #[test]
    fn apply_preset_publishes_show_and_marks_visible_even_when_hidden() {
        let f = fixture();
        let id = create(&f, "title");
        save_preset(&f, "Intro", "title", &[("text", "P")]);
        assert!(!f.svc.list(desktop()).instances[0].visible);

        let snap = f.svc.apply_preset(&id, "Intro", desktop()).unwrap();

        assert!(snap.instances[0].visible);
        let published = f.bus.0.lock().unwrap();
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].action, OverlayAction::Show);
        assert_eq!(published[0].instance_id, id);
        assert_eq!(published[0].template, "title");
        assert_eq!(published[0].fields, payload_fields(&[("text", "P")]));
        assert_eq!(f.events.0.lock().unwrap().len(), 2);
    }

    #[test]
    fn apply_preset_template_mismatch_returns_preset_not_found() {
        let f = fixture();
        let id = create(&f, "title");
        save_preset(&f, "Other", "lower", &[("text", "x")]);
        clear_outputs(&f);

        let err = f.svc.apply_preset(&id, "Other", desktop()).unwrap_err();

        assert!(matches!(err, DomainError::PresetNotFound { name } if name == "Other"));
        assert_eq!(f.svc.list(desktop()).instances[0].fields["text"], "Hello");
        assert!(f.bus.0.lock().unwrap().is_empty());
        assert!(f.events.0.lock().unwrap().is_empty());
    }

    #[test]
    fn apply_preset_unknown_name_returns_preset_not_found() {
        let f = fixture();
        let id = create(&f, "title");

        assert!(matches!(
            f.svc.apply_preset(&id, "Nope", desktop()),
            Err(DomainError::PresetNotFound { .. })
        ));
    }

    #[test]
    fn apply_preset_unknown_instance_returns_instance_not_found() {
        let f = fixture();
        save_preset(&f, "Intro", "title", &[]);

        assert!(matches!(
            f.svc.apply_preset("ghost", "Intro", desktop()),
            Err(DomainError::InstanceNotFound { id }) if id == "ghost"
        ));
    }

    #[test]
    fn apply_preset_missing_template_returns_template_not_found_and_emits_nothing() {
        let f = fixture();
        let id = create(&f, "title");
        save_preset(&f, "Intro", "title", &[("text", "P")]);
        remove_template_from_catalog(&f, "title");
        clear_outputs(&f);

        let err = f.svc.apply_preset(&id, "Intro", desktop()).unwrap_err();

        assert!(matches!(err, DomainError::TemplateNotFound { template } if template == "title"));
        let snap = f.svc.list(desktop());
        assert_eq!(snap.instances[0].fields["text"], "Hello");
        assert!(!snap.instances[0].visible);
        assert!(f.bus.0.lock().unwrap().is_empty());
        assert!(f.events.0.lock().unwrap().is_empty());
    }

    // ---- start_group ----

    fn entry(
        template: &str,
        id: Option<&str>,
        name: Option<&str>,
        fields: &[(&str, &str)],
    ) -> GroupPresetEntry {
        GroupPresetEntry {
            template: template.into(),
            fields: payload_fields(fields),
            instance_id: id.map(str::to_string),
            custom_name: name.map(str::to_string),
        }
    }

    fn save_group(f: &Fixture, name: &str, entries: Vec<GroupPresetEntry>) {
        f.groups.save(name.into(), entries).unwrap();
    }

    fn instance_of<'a>(
        snap: &'a InstancesSnapshot,
        id: &str,
    ) -> &'a crate::domain::instance::InstanceDto {
        snap.instances.iter().find(|i| i.id == id).unwrap()
    }

    #[test]
    fn load_rule_1_matching_id_reuses_instance() {
        let f = fixture();
        f.svc
            .create("title", Some("abc".into()), Some("Mine".into()), desktop())
            .unwrap();
        save_group(
            &f,
            "G",
            vec![entry("title", Some("abc"), None, &[("text", "Grp")])],
        );

        let (results, snap) = f.svc.start_group("G", desktop()).unwrap();

        assert_eq!(results, [GroupStartResult::started("title", "abc", false)]);
        assert_eq!(snap.instances.len(), 1);
        let live = instance_of(&snap, "abc");
        // Defaults are merged under the entry fields, like the old frontend.
        assert_eq!(live.fields["text"], "Grp");
        assert_eq!(live.fields["pct"], "5");
        assert_eq!(live.fields["flag"], "false");
        assert!(live.visible);
        // Rule 1 without a saved name keeps the live name.
        assert_eq!(live.custom_name.as_deref(), Some("Mine"));
    }

    #[test]
    fn load_rule_1_replaces_name_only_when_entry_has_one() {
        let f = fixture();
        f.svc
            .create("title", Some("abc".into()), Some("Mine".into()), desktop())
            .unwrap();
        save_group(
            &f,
            "G",
            vec![entry("title", Some("abc"), Some(" Saved "), &[])],
        );

        let (_, snap) = f.svc.start_group("G", desktop()).unwrap();

        assert_eq!(
            instance_of(&snap, "abc").custom_name.as_deref(),
            Some("Saved")
        );
    }

    #[test]
    fn load_rule_2_free_id_creates_with_saved_id_and_name() {
        let f = fixture();
        save_group(
            &f,
            "G",
            vec![entry(
                "title",
                Some("abc"),
                Some("Main title"),
                &[("text", "T")],
            )],
        );

        let (results, snap) = f.svc.start_group("G", desktop()).unwrap();

        assert_eq!(results, [GroupStartResult::started("title", "abc", false)]);
        let created = instance_of(&snap, "abc");
        assert_eq!(created.custom_name.as_deref(), Some("Main title"));
        assert_eq!(created.fields["text"], "T");
        assert!(created.visible);
    }

    #[test]
    fn load_rule_3_other_template_keeps_existing_and_creates_fresh_id_with_conflict() {
        let f = fixture();
        f.svc
            .create("lower", Some("abc".into()), None, desktop())
            .unwrap();
        save_group(
            &f,
            "G",
            vec![entry("title", Some("abc"), Some("New"), &[("text", "T")])],
        );

        let (results, snap) = f.svc.start_group("G", desktop()).unwrap();

        assert_eq!(snap.instances.len(), 2);
        let untouched = instance_of(&snap, "abc");
        assert_eq!(untouched.template_id, "lower");
        assert!(!untouched.visible);
        let GroupStartResult {
            instance_id,
            id_conflict,
            ok,
            ..
        } = results[0].clone();
        assert!(ok);
        assert_eq!(id_conflict, Some(true));
        let fresh = instance_id.unwrap();
        assert_ne!(fresh, "abc");
        assert_eq!(instance_of(&snap, &fresh).template_id, "title");
        assert_eq!(
            instance_of(&snap, &fresh).custom_name.as_deref(),
            Some("New")
        );
    }

    #[test]
    fn load_rule_4_legacy_entry_without_id_creates_new() {
        let f = fixture();
        save_group(
            &f,
            "G",
            vec![
                entry("title", None, None, &[]),
                entry("title", None, None, &[]),
            ],
        );

        let (results, snap) = f.svc.start_group("G", desktop()).unwrap();

        assert_eq!(snap.instances.len(), 2);
        assert!(results.iter().all(|r| r.ok && r.id_conflict == Some(false)));
        assert_ne!(results[0].instance_id, results[1].instance_id);
        assert!(snap.instances.iter().all(|i| i.visible));
    }

    #[test]
    fn preserves_saved_display_name() {
        let f = fixture();
        save_group(
            &f,
            "G",
            vec![
                entry("title", Some("a"), Some("Headline"), &[]),
                entry("title", Some("b"), None, &[]),
            ],
        );

        let (_, snap) = f.svc.start_group("G", desktop()).unwrap();

        assert_eq!(instance_of(&snap, "a").display_name, "Headline");
        assert_eq!(instance_of(&snap, "b").display_name, "Title #2");
    }

    #[test]
    fn id_collision_reports_conflict_per_entry() {
        let f = fixture();
        f.svc
            .create("lower", Some("x".into()), None, desktop())
            .unwrap();
        f.svc
            .create("lower", Some("y".into()), None, desktop())
            .unwrap();
        save_group(
            &f,
            "G",
            vec![
                entry("title", Some("x"), None, &[]),
                entry("title", Some("free"), None, &[]),
                entry("title", Some("y"), None, &[]),
            ],
        );

        let (results, _) = f.svc.start_group("G", desktop()).unwrap();

        let conflicts: Vec<_> = results.iter().map(|r| r.id_conflict).collect();
        assert_eq!(conflicts, [Some(true), Some(false), Some(true)]);
        assert_eq!(results[1].instance_id.as_deref(), Some("free"));
    }

    #[test]
    fn cross_group_overwrite() {
        let f = fixture();
        save_group(
            &f,
            "G1",
            vec![entry("title", Some("abc"), None, &[("text", "one")])],
        );
        save_group(
            &f,
            "G2",
            vec![entry("title", Some("abc"), None, &[("text", "two")])],
        );

        f.svc.start_group("G1", desktop()).unwrap();
        let (results, snap) = f.svc.start_group("G2", desktop()).unwrap();

        assert_eq!(snap.instances.len(), 1);
        assert_eq!(instance_of(&snap, "abc").fields["text"], "two");
        assert_eq!(results[0].id_conflict, Some(false));
    }

    #[test]
    fn unknown_template_is_reported_per_entry_without_aborting() {
        let f = fixture();
        save_group(
            &f,
            "G",
            vec![
                entry("nope", Some("n"), None, &[]),
                entry("title", Some("ok"), None, &[]),
            ],
        );

        let (results, snap) = f.svc.start_group("G", desktop()).unwrap();

        assert_eq!(
            results[0],
            GroupStartResult::failed("nope", GroupStartFailure::TemplateMissing)
        );
        assert!(results[1].ok);
        assert_eq!(ids(&snap), ["ok"]);
    }

    #[test]
    fn start_group_with_nothing_started_does_not_bump_or_emit() {
        let f = fixture();
        save_group(&f, "G", vec![entry("nope", None, None, &[])]);

        let (results, snap) = f.svc.start_group("G", desktop()).unwrap();

        assert!(!results[0].ok);
        assert_eq!(snap.revision, 0);
        assert!(f.events.0.lock().unwrap().is_empty());
        assert!(f.bus.0.lock().unwrap().is_empty());
    }

    #[test]
    fn start_group_unknown_group_is_not_found() {
        let f = fixture();

        assert!(matches!(
            f.svc.start_group("Nope", desktop()),
            Err(DomainError::GroupPresetNotFound { name }) if name == "Nope"
        ));
        assert!(f.events.0.lock().unwrap().is_empty());
    }

    #[test]
    fn start_group_atomic_single_snapshot_single_revision() {
        let f = fixture();
        f.svc
            .create("title", Some("live".into()), None, desktop())
            .unwrap();
        save_group(
            &f,
            "G",
            vec![
                entry("title", Some("live"), None, &[("text", "u")]),
                entry("lower", Some("new"), None, &[]),
                entry("lower", None, None, &[]),
            ],
        );
        clear_outputs(&f);
        let before = f.svc.list(desktop()).revision;

        let (_, snap) = f.svc.start_group("G", EventSource::remote("p")).unwrap();

        let events = f.events.0.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], snap);
        assert_eq!(snap.revision, before + 1);
        assert_eq!(snap.source, EventSource::remote("p"));
        assert_eq!(snap.instances.len(), 3);
        // One show per started entry, in entry order.
        let published = f.bus.0.lock().unwrap();
        assert_eq!(published.len(), 3);
        assert!(published.iter().all(|p| p.action == OverlayAction::Show));
        assert_eq!(published[0].instance_id, "live");
        assert_eq!(published[1].instance_id, "new");
    }

    #[test]
    fn start_group_reads_manifest_once() {
        let f = fixture();
        save_group(
            &f,
            "G",
            vec![
                entry("title", Some("a"), None, &[]),
                entry("lower", Some("b"), None, &[]),
                entry("title", None, None, &[]),
            ],
        );
        let before = f.lookup.reads.load(Ordering::SeqCst);

        f.svc.start_group("G", desktop()).unwrap();

        assert_eq!(f.lookup.reads.load(Ordering::SeqCst) - before, 1);
    }

    #[test]
    fn result_has_no_send_failed_reason() {
        let f = fixture();
        save_group(
            &f,
            "G",
            vec![
                entry("nope", None, None, &[]),
                entry("title", Some("a"), None, &[]),
            ],
        );

        let (results, _) = f.svc.start_group("G", desktop()).unwrap();
        let json = serde_json::to_value(&results).unwrap();

        assert_eq!(
            json,
            serde_json::json!([
                { "template": "nope", "ok": false, "reason": "templateMissing" },
                { "template": "title", "ok": true, "instanceId": "a", "idConflict": false }
            ])
        );
        assert!(!json.to_string().contains("sendFailed"));
    }

    // ---- save_group_from_instances ----

    #[test]
    fn save_group_builds_entries_in_registry_order_including_hidden() {
        let f = fixture();
        let a = create(&f, "title");
        let b = create(&f, "lower");
        f.svc.show(&a, desktop()).unwrap();

        let groups = f.svc.save_group_from_instances("G").unwrap();

        let entries = &groups[0].entries;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].instance_id.as_deref(), Some(a.as_str()));
        assert_eq!(entries[0].template, "title");
        assert_eq!(entries[1].instance_id.as_deref(), Some(b.as_str()));
        assert_eq!(entries[1].template, "lower");

        f.svc.move_instance(1, 0, desktop());
        let groups = f.svc.save_group_from_instances("G").unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].entries[0].template, "lower");
    }

    #[test]
    fn save_group_records_custom_name_only_when_user_set() {
        let f = fixture();
        let named = create(&f, "title");
        create(&f, "title");
        f.svc.rename(&named, "Main title", desktop()).unwrap();

        let groups = f.svc.save_group_from_instances("G").unwrap();

        assert_eq!(
            groups[0].entries[0].custom_name.as_deref(),
            Some("Main title")
        );
        // The second shows "Title #2" but that label is not persisted.
        assert_eq!(groups[0].entries[1].custom_name, None);
    }

    #[test]
    fn save_group_records_current_fields_and_instance_id() {
        let f = fixture();
        let id = create(&f, "title");
        f.svc
            .set_fields(
                &id,
                payload_fields(&[("text", "Now"), ("stale", "x")]),
                desktop(),
            )
            .unwrap();
        let before = f.svc.list(desktop()).revision;

        let groups = f.svc.save_group_from_instances("G").unwrap();

        let saved = &groups[0].entries[0];
        assert_eq!(saved.instance_id.as_deref(), Some(id.as_str()));
        assert_eq!(
            saved.fields,
            payload_fields(&[("text", "Now"), ("pct", ""), ("flag", ""), ("note", "")])
        );
        // Saving does not touch the registry.
        assert_eq!(f.svc.list(desktop()).revision, before);
    }

    #[test]
    fn save_group_keeps_stored_fields_when_template_is_missing() {
        let f = fixture();
        let id = create(&f, "title");
        let stored = payload_fields(&[("text", "Now"), ("stale", "x")]);
        f.svc.set_fields(&id, stored.clone(), desktop()).unwrap();
        remove_template_from_catalog(&f, "title");

        let groups = f.svc.save_group_from_instances("G").unwrap();

        assert_eq!(groups[0].entries[0].template, "title");
        assert_eq!(groups[0].entries[0].fields, stored);
    }

    #[test]
    fn save_group_with_no_instances_returns_group_preset_no_entries() {
        let f = fixture();

        assert!(matches!(
            f.svc.save_group_from_instances("G"),
            Err(DomainError::GroupPresetNoEntries)
        ));
    }
}
