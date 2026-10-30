use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use super::ports::{InstanceEvents, OverlayBus, TemplateLookup};
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::instance::{
    defaults_for, fields_for, EventSource, Instance, InstancesSnapshot, Registry,
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
    ) -> Self {
        Self {
            lookup,
            bus,
            events,
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
        self.send_fields(id, OverlayAction::Show, source)
    }

    pub fn update(&self, id: &str, source: EventSource) -> DomainResult<InstancesSnapshot> {
        self.send_fields(id, OverlayAction::Update, source)
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
        let message = payload(instance, action, fields_for(instance, template));
        if action == OverlayAction::Show {
            instance.visible = true;
        }
        Ok(self.commit(&mut state, &manifest, &[message], source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        manifest: Manifest,
        reads: AtomicUsize,
    }

    impl TemplateLookup for FakeLookup {
        fn manifest(&self) -> DomainResult<Manifest> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            Ok(self.manifest.clone())
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
    }

    fn fixture() -> Fixture {
        let events = Arc::new(FakeEvents(Mutex::new(Vec::new())));
        let bus = Arc::new(FakeBus(Mutex::new(Vec::new())));
        let lookup = Arc::new(FakeLookup {
            manifest: manifest(),
            reads: AtomicUsize::new(0),
        });
        let svc = InstanceService::new(lookup, bus.clone(), events.clone());
        Fixture { svc, events, bus }
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
}
