use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        base_scenario_object, BaseScenarioObjectConfig, DebugMessageActionConfig,
        DespawnScenarioObjectActionConfig, EventActionConfig, HintEmphasisClearActionConfig,
        HintEmphasisSetActionConfig, NextScenarioActionConfig, ObjectiveActionConfig,
        ObjectiveCompleteActionConfig, ObjectiveMarkerAttachActionConfig,
        ObjectiveMarkerDetachActionConfig, ScenarioAreaConfig, ScenarioObjectConfig,
        ScenarioObjectKind, SetControllerVerbActionConfig, SetSpeedCapActionConfig,
        VariableSetActionConfig,
    };
}

#[derive(Clone, Debug)]
pub enum EventActionConfig {
    DebugMessage(DebugMessageActionConfig),
    VariableSet(VariableSetActionConfig),
    Objective(ObjectiveActionConfig),
    ObjectiveComplete(ObjectiveCompleteActionConfig),
    ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig),
    ObjectiveMarkerDetach(ObjectiveMarkerDetachActionConfig),
    HintEmphasisSet(HintEmphasisSetActionConfig),
    HintEmphasisClear(HintEmphasisClearActionConfig),
    SpawnScenarioObject(ScenarioObjectConfig),
    DespawnScenarioObject(DespawnScenarioObjectActionConfig),
    SetSpeedCap(SetSpeedCapActionConfig),
    SetControllerVerb(SetControllerVerbActionConfig),
    CreateScenarioArea(ScenarioAreaConfig),
    NextScenario(NextScenarioActionConfig),
}

impl EventAction<NovaEventWorld> for EventActionConfig {
    fn action(&self, world: &mut NovaEventWorld, info: &GameEventInfo) {
        match self {
            EventActionConfig::DebugMessage(config) => {
                config.action(world, info);
            }
            EventActionConfig::VariableSet(config) => {
                config.action(world, info);
            }
            EventActionConfig::Objective(config) => {
                config.action(world, info);
            }
            EventActionConfig::ObjectiveComplete(config) => {
                config.action(world, info);
            }
            EventActionConfig::ObjectiveMarkerAttach(config) => {
                config.action(world, info);
            }
            EventActionConfig::ObjectiveMarkerDetach(config) => {
                config.action(world, info);
            }
            EventActionConfig::HintEmphasisSet(config) => {
                config.action(world, info);
            }
            EventActionConfig::HintEmphasisClear(config) => {
                config.action(world, info);
            }
            EventActionConfig::SpawnScenarioObject(config) => {
                config.action(world, info);
            }
            EventActionConfig::DespawnScenarioObject(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetSpeedCap(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetControllerVerb(config) => {
                config.action(world, info);
            }
            EventActionConfig::CreateScenarioArea(config) => {
                config.action(world, info);
            }
            EventActionConfig::NextScenario(config) => {
                config.action(world, info);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct VariableSetActionConfig {
    pub key: String,
    pub expression: VariableExpressionNode,
}

impl EventAction<NovaEventWorld> for VariableSetActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        match self.expression.evaluate(world) {
            Ok(literal) => {
                world.insert_variable(self.key.clone(), literal);
            }
            Err(e) => {
                error!(
                    "VariableSetActionConfig: failed to evaluate expression for key '{}': {:?}",
                    self.key, e
                );
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct DebugMessageActionConfig {
    pub message: String,
}

impl EventAction<NovaEventWorld> for DebugMessageActionConfig {
    fn action(&self, _: &mut NovaEventWorld, _: &GameEventInfo) {
        debug!("Event Action Message: {}", self.message);
    }
}

#[derive(Clone, Debug, Default)]
pub struct NextScenarioActionConfig {
    pub scenario_id: String,
    pub linger: bool,
}

impl EventAction<NovaEventWorld> for NextScenarioActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        debug!(
            "NextScenario: queuing scenario '{}' (linger: {})",
            self.scenario_id, self.linger
        );
        world.next_scenario = Some(self.clone());
    }
}

/// A scenario action that adds an objective to the HUD.
///
/// The objective *data* (id + message) is the generic `bevy_common_systems` `Objective`, but
/// this scenario-action wrapper stays nova-local because it implements the (foreign)
/// `EventAction` trait - which the orphan rule forbids implementing on the foreign
/// `Objective` type directly.
#[derive(Clone, Debug)]
pub struct ObjectiveActionConfig {
    /// Opaque identifier, used to complete/remove the objective later.
    pub id: String,
    /// The text shown in the objectives HUD.
    pub message: String,
}

impl ObjectiveActionConfig {
    /// Construct from string slices.
    pub fn new(id: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            message: message.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ObjectiveActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.push_objective(self.clone());
    }
}

#[derive(Clone, Debug)]
pub struct ObjectiveCompleteActionConfig {
    pub id: String,
}

impl EventAction<NovaEventWorld> for ObjectiveCompleteActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.remove_objective(&self.id);
    }
}

/// Despawn the scenario object whose [`EntityId`] matches `id` (recursive,
/// so the object's whole child hierarchy goes with it). The complement of
/// `SpawnScenarioObject`, e.g. a salvage crate the script removes on pickup.
#[derive(Clone, Debug)]
pub struct DespawnScenarioObjectActionConfig {
    pub id: String,
}

impl DespawnScenarioObjectActionConfig {
    /// Construct from a string slice.
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl EventAction<NovaEventWorld> for DespawnScenarioObjectActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        debug!("DespawnScenarioObject: despawning '{}'", id);

        // The id -> Entity lookup needs world access, which push_command's
        // `&mut Commands` does not have - so the command queues a Command
        // closure that resolves and despawns in one step. The lookup is
        // gated on ScenarioScopedMarker: spaceship SECTIONS also carry
        // EntityId (their per-ship section ids like "controller"), and an
        // unscoped match on such an id would rip that section out of every
        // ship in the scene.
        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query =
                    world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
                let matches: Vec<Entity> = query
                    .iter(world)
                    .filter(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                    .collect();
                if matches.is_empty() {
                    warn!(
                        "DespawnScenarioObject: no entity with id '{}'; check the scenario \
                         for a typo or a double despawn",
                        id
                    );
                }
                for entity in matches {
                    // get_entity_mut, not entity_mut: an earlier recursive
                    // despawn in this loop may have taken a matched
                    // descendant with it (review R1.1).
                    if let Ok(entity_mut) = world.get_entity_mut(entity) {
                        entity_mut.despawn();
                    }
                }
            });
        });
    }
}

/// Attach the gold objective marker (task 20260712-093831) to the scenario
/// object whose [`EntityId`] matches `target_id`: inserts
/// [`ObjectiveMarkerTarget`] with `label`, and the HUD's objective-markers
/// observer grows the chip. Scoped-only lookup, same rule as
/// DespawnScenarioObject. Attaching to an already-marked entity just
/// updates the label.
#[derive(Clone, Debug)]
pub struct ObjectiveMarkerAttachActionConfig {
    pub target_id: String,
    /// The short name the marker chip shows next to the distance.
    pub label: String,
}

impl ObjectiveMarkerAttachActionConfig {
    /// Construct from string slices.
    pub fn new(target_id: &str, label: &str) -> Self {
        Self {
            target_id: target_id.to_string(),
            label: label.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ObjectiveMarkerAttachActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.target_id.clone();
        let label = self.label.clone();
        debug!("ObjectiveMarkerAttach: '{}' <- '{}'", id, label);

        // Same shape as DespawnScenarioObject: the id lookup needs world
        // access, so the queued command resolves and inserts in one step -
        // which also means an attach ordered after a spawn in the same
        // handler sees the freshly spawned entity.
        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query =
                    world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
                let matches: Vec<Entity> = query
                    .iter(world)
                    .filter(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                    .collect();
                if matches.is_empty() {
                    warn!(
                        "ObjectiveMarkerAttach: no scoped entity with id '{}'; check the \
                         scenario for a typo or an attach before the spawn",
                        id
                    );
                }
                for entity in matches {
                    if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                        entity_mut.insert(ObjectiveMarkerTarget::new(&label));
                    }
                }
            });
        });
    }
}

/// Detach the objective marker from the scenario object whose [`EntityId`]
/// matches `target_id` (no-op with a warning when nothing matches; a
/// marker whose entity despawned is already detached - the chip died with
/// it).
#[derive(Clone, Debug)]
pub struct ObjectiveMarkerDetachActionConfig {
    pub target_id: String,
}

impl ObjectiveMarkerDetachActionConfig {
    /// Construct from a string slice.
    pub fn new(target_id: &str) -> Self {
        Self {
            target_id: target_id.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ObjectiveMarkerDetachActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.target_id.clone();
        debug!("ObjectiveMarkerDetach: '{}'", id);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query =
                    world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
                let matches: Vec<Entity> = query
                    .iter(world)
                    .filter(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                    .collect();
                if matches.is_empty() {
                    // Quieter than attach: detaching an entity that already
                    // despawned (crate picked up) is a legitimate script
                    // shape, not necessarily a typo.
                    debug!("ObjectiveMarkerDetach: no scoped entity with id '{}'", id);
                }
                for entity in matches {
                    if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                        entity_mut.remove::<ObjectiveMarkerTarget>();
                    }
                }
            });
        });
    }
}

/// Emphasize one keybind-hint row (task 20260712-093831): pushes `verb`
/// into nova_gameplay's [`HintEmphasis`] resource, so the cluster pulses
/// that row toward objective gold until a `HintEmphasisClear` (or scenario
/// teardown) drops it. Only [`ROW_VERBS`] names are valid; the resource
/// refuses unknown verbs with a warning.
#[derive(Clone, Debug)]
pub struct HintEmphasisSetActionConfig {
    pub verb: String,
}

impl HintEmphasisSetActionConfig {
    /// Construct from a string slice.
    pub fn new(verb: &str) -> Self {
        Self {
            verb: verb.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for HintEmphasisSetActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let verb = self.verb.clone();
        debug!("HintEmphasisSet: '{}'", verb);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                // get_resource_mut, not resource_mut: headless rigs that
                // exercise scenario scripts without the HUD plugins have no
                // emphasis resource, and the action must not panic there.
                let Some(mut emphasis) = world.get_resource_mut::<HintEmphasis>() else {
                    warn!("HintEmphasisSet: no HintEmphasis resource (HUD not loaded)");
                    return;
                };
                emphasis.set(&verb);
            });
        });
    }
}

/// Drop the emphasis on one keybind-hint row (see [`HintEmphasisSetActionConfig`]).
#[derive(Clone, Debug)]
pub struct HintEmphasisClearActionConfig {
    pub verb: String,
}

impl HintEmphasisClearActionConfig {
    /// Construct from a string slice.
    pub fn new(verb: &str) -> Self {
        Self {
            verb: verb.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for HintEmphasisClearActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let verb = self.verb.clone();
        debug!("HintEmphasisClear: '{}'", verb);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(mut emphasis) = world.get_resource_mut::<HintEmphasis>() else {
                    return;
                };
                emphasis.clear(&verb);
            });
        });
    }
}

/// Set or clear the manual [`FlightSpeedCap`] on a scenario ship by id
/// (the shakedown training governor releases at beacon 1; playtest round
/// 2 finding 3). Scoped-only lookup, same rule as DespawnScenarioObject.
#[derive(Clone, Debug)]
pub struct SetSpeedCapActionConfig {
    pub id: String,
    /// `Some(cap)` installs/updates the cap (u/s); `None` removes it.
    pub cap: Option<f32>,
}

impl EventAction<NovaEventWorld> for SetSpeedCapActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        let cap = self.cap;
        debug!("SetSpeedCap: '{}' -> {:?}", id, cap);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query = world.query_filtered::<(Entity, &EntityId), (
                    With<ScenarioScopedMarker>,
                    With<SpaceshipRootMarker>,
                )>();
                let Some(ship) = query
                    .iter(world)
                    .find(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                else {
                    warn!("SetSpeedCap: no scoped ship with id '{}'", id);
                    return;
                };
                match cap {
                    Some(cap) => {
                        world.entity_mut(ship).insert(FlightSpeedCap(cap));
                    }
                    None => {
                        world.entity_mut(ship).remove::<FlightSpeedCap>();
                    }
                }
            });
        });
    }
}

/// Enable or disable one flight verb on a scenario ship's controller
/// section(s) by id. Flight verbs (STOP/GOTO/ORBIT) are a capability the
/// controller grants; this flips a single verb at runtime - the shakedown
/// withholds GOTO until the first objective is complete
/// (spike docs/spikes/20260712-143551-controller-provided-verb-flags.md).
/// Scoped-only lookup, same rule as SetSpeedCap; writes every controller
/// section on the ship so the union the input layer reads matches.
#[derive(Clone, Debug)]
pub struct SetControllerVerbActionConfig {
    pub id: String,
    pub verb: FlightVerb,
    pub enabled: bool,
}

impl EventAction<NovaEventWorld> for SetControllerVerbActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        let verb = self.verb;
        let enabled = self.enabled;
        debug!("SetControllerVerb: '{}' {:?} -> {}", id, verb, enabled);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut ships = world.query_filtered::<(Entity, &EntityId), (
                    With<ScenarioScopedMarker>,
                    With<SpaceshipRootMarker>,
                )>();
                let Some(ship) = ships
                    .iter(world)
                    .find(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                else {
                    warn!("SetControllerVerb: no scoped ship with id '{}'", id);
                    return;
                };

                // Every controller section on this ship (active or not - the
                // flag persists across (de)activation), so the union the hint
                // pass and observers read reflects the change.
                let mut controllers =
                    world.query_filtered::<(Entity, &ChildOf), With<ControllerSectionMarker>>();
                let targets: Vec<Entity> = controllers
                    .iter(world)
                    .filter(|(_, &ChildOf(parent))| parent == ship)
                    .map(|(entity, _)| entity)
                    .collect();
                if targets.is_empty() {
                    warn!("SetControllerVerb: ship '{}' has no controller section", id);
                    return;
                }
                for controller in targets {
                    if let Some(mut verbs) = world.get_mut::<ControllerVerbs>(controller) {
                        verbs.set(verb, enabled);
                    }
                }
            });
        });
    }
}

#[derive(Clone, Debug)]
pub struct ScenarioObjectConfig {
    pub base: BaseScenarioObjectConfig,
    pub kind: ScenarioObjectKind,
}

#[derive(Clone, Debug)]
pub struct BaseScenarioObjectConfig {
    pub id: String,
    pub name: String,
    pub position: Vec3,
    pub rotation: Quat,
}

pub fn base_scenario_object(config: &BaseScenarioObjectConfig) -> impl Bundle {
    (
        ScenarioScopedMarker,
        Name::new(config.name.clone()),
        EntityId::new(config.id.clone()),
        Transform::from_translation(config.position).with_rotation(config.rotation),
        RigidBody::Dynamic,
        // Physics advances Transform only on fixed ticks (64 Hz by
        // default); everything
        // watched by the render-rate camera must interpolate between them or
        // it stair-steps. Invisible while the chase camera was bolted rigidly
        // to the ship (both stepped together), but the camera smoothing from
        // the flight-feel retune eases at render rate and exposed the steps
        // as twitch (task 20260709-160753).
        TransformInterpolation,
        Visibility::Visible,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The behavior the component buys (task 20260709-160753): a moving
    /// scenario body's Transform advances on EVERY render frame, not just on
    /// fixed physics ticks. 4 ms frames against the 15.6 ms tick mean at
    /// most one tick lands inside any 3-frame span - without easing at
    /// least two consecutive frames would show identical translations.
    #[test]
    fn scenario_bodies_move_between_fixed_ticks() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        // Mirrors the integrity physics harness: MeshPlugin because avian's
        // collider-from-mesh backend reads AssetEvent<Mesh> at startup.
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.004,
        )));
        app.finish();

        let body = app
            .world_mut()
            .spawn((
                base_scenario_object(&BaseScenarioObjectConfig {
                    id: "mover".to_string(),
                    name: "Mover".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                }),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
                LinearVelocity(Vec3::X * 10.0),
            ))
            .id();

        // Warm up past two fixed ticks so the easing has start+end states.
        for _ in 0..10 {
            app.update();
        }

        // Four consecutive 4 ms frames: with easing every frame advances the
        // translation; stair-stepping would repeat a value.
        let mut positions = Vec::new();
        for _ in 0..4 {
            app.update();
            positions.push(app.world().get::<Transform>(body).unwrap().translation.x);
        }
        for pair in positions.windows(2) {
            assert!(
                pair[1] > pair[0],
                "translation must advance every render frame, got {positions:?}"
            );
        }
    }

    /// The despawn action removes exactly the scenario object whose id
    /// matches - and ONLY scenario-scoped entities: spaceship sections
    /// carry EntityId too (per-ship ids like "controller"), and an
    /// unscoped match would rip that section out of every ship.
    #[test]
    fn despawn_action_removes_the_scoped_object_by_id() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let crate_1 = world
            .spawn((ScenarioScopedMarker, EntityId::new("crate_1".to_string())))
            .id();
        let crate_2 = world
            .spawn((ScenarioScopedMarker, EntityId::new("crate_2".to_string())))
            .id();
        // An unscoped entity with a colliding id - a stand-in for a ship
        // section - must survive.
        let section = world.spawn(EntityId::new("crate_1".to_string())).id();

        let action = DespawnScenarioObjectActionConfig::new("crate_1");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());

        // The action only queues; the drain in state_to_world applies it.
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(
            world.get_entity(crate_1).is_err(),
            "the matching scoped object despawns"
        );
        assert!(
            world.get_entity(crate_2).is_ok(),
            "other scoped objects survive"
        );
        assert!(
            world.get_entity(section).is_ok(),
            "an unscoped entity with the same id (a ship section) survives"
        );
    }

    /// A missing id is a warning, not a crash: the drain must complete and
    /// unrelated entities survive (double-despawn / typo path).
    #[test]
    fn despawn_action_with_missing_id_is_harmless() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let bystander = world
            .spawn((ScenarioScopedMarker, EntityId::new("beacon_1".to_string())))
            .id();

        let action = DespawnScenarioObjectActionConfig::new("no_such_id");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(world.get_entity(bystander).is_ok());
    }

    /// The marker attach/detach pair drives the [`ObjectiveMarkerTarget`]
    /// component on exactly the scoped object with the id - unscoped
    /// entities with colliding ids (ship sections) are never marked, and a
    /// re-attach updates the label in place.
    #[test]
    fn objective_marker_attach_and_detach_drive_the_component() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let beacon = world
            .spawn((ScenarioScopedMarker, EntityId::new("beacon_1".to_string())))
            .id();
        let section = world.spawn(EntityId::new("beacon_1".to_string())).id();

        let attach = ObjectiveMarkerAttachActionConfig::new("beacon_1", "BEACON 1");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        attach.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert_eq!(
            world
                .get::<ObjectiveMarkerTarget>(beacon)
                .map(|marker| marker.label.as_str()),
            Some("BEACON 1"),
            "the scoped object is marked"
        );
        assert!(
            world.get::<ObjectiveMarkerTarget>(section).is_none(),
            "an unscoped entity with the same id (a ship section) is never marked"
        );

        // Re-attach updates the label in place (no detach needed between).
        let relabel = ObjectiveMarkerAttachActionConfig::new("beacon_1", "NEXT");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        relabel.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert_eq!(
            world
                .get::<ObjectiveMarkerTarget>(beacon)
                .map(|marker| marker.label.as_str()),
            Some("NEXT")
        );

        let detach = ObjectiveMarkerDetachActionConfig::new("beacon_1");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        detach.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(
            world.get::<ObjectiveMarkerTarget>(beacon).is_none(),
            "detach removes the marker"
        );
    }

    /// Attach/detach against a missing id must warn and complete, not
    /// crash - the detach-after-despawn shape is legitimate script data
    /// (crate picked up before its detach action runs).
    #[test]
    fn objective_marker_actions_with_missing_id_are_harmless() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        let bystander = world
            .spawn((ScenarioScopedMarker, EntityId::new("beacon_1".to_string())))
            .id();

        for action in [
            EventActionConfig::ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig::new(
                "no_such_id",
                "GHOST",
            )),
            EventActionConfig::ObjectiveMarkerDetach(ObjectiveMarkerDetachActionConfig::new(
                "no_such_id",
            )),
        ] {
            let mut event_world = world.resource_mut::<NovaEventWorld>();
            action.action(&mut event_world, &GameEventInfo::default());
            NovaEventWorld::state_to_world_system(&mut world);
        }

        assert!(world.get_entity(bystander).is_ok());
        assert!(world.get::<ObjectiveMarkerTarget>(bystander).is_none());
    }

    /// The emphasis pair mutates nova_gameplay's HintEmphasis resource
    /// through the queued-command drain; without the resource (headless
    /// scenario rigs) both are warn-and-continue no-ops.
    #[test]
    fn hint_emphasis_actions_drive_the_resource() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        // Without the resource: harmless.
        let set = HintEmphasisSetActionConfig::new("GOTO");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        set.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        // With it: set lands, clear drops.
        world.init_resource::<HintEmphasis>();
        let set = HintEmphasisSetActionConfig::new("GOTO");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        set.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(world.resource::<HintEmphasis>().contains("GOTO"));

        let clear = HintEmphasisClearActionConfig::new("GOTO");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        clear.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(!world.resource::<HintEmphasis>().contains("GOTO"));
    }

    /// SetControllerVerb flips exactly the addressed ship's controller verb,
    /// leaving other verbs on that controller and other ships untouched; and
    /// re-enabling restores it. If the action did not scope by ship id, the
    /// bystander ship's controller would flip too and this test would fail.
    #[test]
    fn set_controller_verb_flips_only_the_scoped_ship() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        // The target ship and a bystander ship, each a scoped root with a
        // controller section carrying all verbs.
        let player = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("player".to_string()),
            ))
            .id();
        let player_ctrl = world
            .spawn((
                ChildOf(player),
                ControllerSectionMarker,
                ControllerVerbs::default(),
            ))
            .id();
        let bystander = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("bystander".to_string()),
            ))
            .id();
        let bystander_ctrl = world
            .spawn((
                ChildOf(bystander),
                ControllerSectionMarker,
                ControllerVerbs::default(),
            ))
            .id();

        // Disable GOTO on the player only.
        let disable = SetControllerVerbActionConfig {
            id: "player".to_string(),
            verb: FlightVerb::Goto,
            enabled: false,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        disable.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        let pv = *world.get::<ControllerVerbs>(player_ctrl).unwrap();
        assert!(!pv.goto, "GOTO disabled on the addressed ship");
        assert!(
            pv.stop && pv.orbit,
            "other verbs on that controller untouched"
        );
        assert!(
            world.get::<ControllerVerbs>(bystander_ctrl).unwrap().goto,
            "the bystander ship's controller is untouched"
        );

        // Re-enable restores it.
        let enable = SetControllerVerbActionConfig {
            id: "player".to_string(),
            verb: FlightVerb::Goto,
            enabled: true,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        enable.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(
            world.get::<ControllerVerbs>(player_ctrl).unwrap().goto,
            "GOTO re-enabled on the addressed ship"
        );
    }

    /// SetControllerVerb writes EVERY controller section on the ship, so the
    /// union the input layer reads (verb available if ANY live controller
    /// grants it) reflects the change no matter which controller it samples.
    #[test]
    fn set_controller_verb_writes_all_controllers_on_the_ship() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let ship = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("twin".to_string()),
            ))
            .id();
        let ctrl_a = world
            .spawn((
                ChildOf(ship),
                ControllerSectionMarker,
                ControllerVerbs::default(),
            ))
            .id();
        let ctrl_b = world
            .spawn((
                ChildOf(ship),
                ControllerSectionMarker,
                ControllerVerbs::default(),
            ))
            .id();

        let disable = SetControllerVerbActionConfig {
            id: "twin".to_string(),
            verb: FlightVerb::Stop,
            enabled: false,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        disable.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(
            !world.get::<ControllerVerbs>(ctrl_a).unwrap().stop,
            "first controller written"
        );
        assert!(
            !world.get::<ControllerVerbs>(ctrl_b).unwrap().stop,
            "second controller written too"
        );
    }

    /// Every dynamic scenario body must interpolate its Transform between
    /// fixed physics ticks, or it stair-steps under the smoothed chase
    /// camera (task 20260709-160753).
    #[test]
    fn scenario_objects_interpolate_their_transforms() {
        let mut world = World::new();
        let entity = world
            .spawn(base_scenario_object(&BaseScenarioObjectConfig {
                id: "test".to_string(),
                name: "Test".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            }))
            .id();
        assert!(world.get::<TransformInterpolation>(entity).is_some());
    }
}

#[derive(Clone, Debug)]
pub enum ScenarioObjectKind {
    Asteroid(AsteroidConfig),
    Spaceship(SpaceshipConfig),
    Beacon(BeaconConfig),
    SalvageCrate(SalvageCrateConfig),
}

impl EventAction<NovaEventWorld> for ScenarioObjectConfig {
    fn action(&self, world: &mut NovaEventWorld, _info: &GameEventInfo) {
        let config = self.clone();
        debug!("SpawnScenarioObject: spawning '{}'", config.base.id);

        world.push_command(move |commands| {
            let mut entity_commands = commands.spawn(base_scenario_object(&config.base));

            match &config.kind {
                ScenarioObjectKind::Asteroid(config) => {
                    entity_commands.insert(asteroid_scenario_object(config.clone()));
                }
                ScenarioObjectKind::Spaceship(config) => {
                    entity_commands.insert(spaceship_scenario_object(config.clone()));
                }
                ScenarioObjectKind::Beacon(config) => {
                    entity_commands.insert(beacon_scenario_object(config.clone()));
                }
                ScenarioObjectKind::SalvageCrate(config) => {
                    entity_commands.insert(salvage_crate_scenario_object(config.clone()));
                }
            }
        });
    }
}

#[derive(Clone, Debug)]
pub struct ScenarioAreaConfig {
    pub id: String,
    pub name: String,
    pub position: Vec3,
    pub rotation: Quat,
    pub radius: f32,
}

impl EventAction<NovaEventWorld> for ScenarioAreaConfig {
    fn action(&self, world: &mut NovaEventWorld, _info: &GameEventInfo) {
        let config = self.clone();
        debug!(
            "CreateScenarioArea: creating area '{}' (radius: {})",
            config.id, config.radius
        );

        world.push_command(move |commands| {
            commands.spawn((
                ScenarioScopedMarker,
                ScenarioAreaMarker,
                Name::new(config.name.clone()),
                EntityId::new(config.id.clone()),
                Transform::from_translation(config.position).with_rotation(config.rotation),
                RigidBody::Static,
                Collider::sphere(config.radius),
                Sensor,
                Visibility::Visible,
            ));
        });
    }
}
