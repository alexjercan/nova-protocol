//! Trigger volumes: a collider that reports enter/exit as scenario events.
//!
//! Occupancy is refcounted per (area, body) pair because a compound body
//! reports one collision per child collider - a bare start/stop would fire
//! `on_enter` once per part.
//!
//! Touch this module when changing when an area counts as entered or left.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::{CommandsGameEventExt, *};

/// `ScenarioAreaMarker` and `ScenarioAreaPlugin`.
pub mod prelude {
    pub use super::{ScenarioAreaMarker, ScenarioAreaPlugin};
}

/// Marks a scenario trigger volume: a sensor collider whose overlaps the area
/// plugin turns into `OnEnter`/`OnExit` events under the area's scenario id.
/// Inserted by `CreateScenarioArea` (and by crates/beacons doubling as their own
/// trigger); requires a [`Collider`] and [`Sensor`].
#[derive(Component, Debug, Clone, Reflect)]
#[require(Collider, Sensor)]
pub struct ScenarioAreaMarker;

/// Per-(area, body) SET of the colliders currently overlapping, so a compound
/// body entering an area fires exactly one `OnEnter` and one `OnExit`.
///
/// A spaceship is ONE rigid body wearing many section colliders, so avian fires
/// a separate `CollisionStart`/`CollisionEnd` per section collider that touches
/// an area sensor (empirically 3+ for the old trainer, 18 for a racer, 90+ for a
/// skinned block ship). Without this, an `OnEnter` handler that is not
/// idempotent - the salvage crate's `despawn + crates_recovered += 1` - runs
/// once PER section collider, despawning a crate several times and over-counting
/// the tally. Collapsing the burst: `OnEnter` fires when the set fills, `OnExit`
/// when it empties.
///
/// A SET, not a count. A ship loses colliders while it is inside an area - a
/// destroyed section despawns, and so does every skin plate riding it - and
/// avian fires no `CollisionEnd` for a collider that no longer exists. A counter
/// therefore only ever climbed: measured over one menu duel, a block gunship
/// took 270 starts against 149 ends, so its tally stood at 121 and the area
/// could never report it leaving. Membership is repairable where a count is not:
/// [`forget_collider_occupancy`] drops a dead collider from every set it is in,
/// and the surviving sections still drive the set to empty when the ship
/// finally leaves.
#[derive(Resource, Default)]
struct AreaOccupancy(
    bevy::platform::collections::HashMap<
        (Entity, Entity),
        bevy::platform::collections::HashSet<Entity>,
    >,
);

/// Turns [`ScenarioAreaMarker`] sensor overlaps into scenario `OnEnter`/`OnExit`
/// events, deduping a compound body's many section colliders to one enter/exit.
/// Adds the per-area wiring observer plus the occupancy-cleanup observer (all
/// observer-driven, no scheduled systems).
pub struct ScenarioAreaPlugin;

impl Plugin for ScenarioAreaPlugin {
    fn build(&self, app: &mut App) {
        trace!("AreaPlugin: build");

        app.init_resource::<AreaOccupancy>();
        app.add_observer(wire_area_collisions);
        app.add_observer(forget_body_occupancy);
        app.add_observer(forget_collider_occupancy);
    }
}

/// Drop every occupancy row an entity is part of when it leaves the world,
/// from EITHER side of the pair: an area despawned on pickup, or a body
/// destroyed while still inside a live area. avian fires no `CollisionEnd` for
/// a despawned collider, so without this the pair keeps a non-zero count
/// forever and the next body to occupy that area never drives it back to zero.
///
/// Keyed on [`EntityId`] because that is what both collision handlers require
/// of the non-area side, and areas carry it too (`q_area` requires it) - so one
/// observer covers both sides. Scenario teardown despawns every scoped entity,
/// so this is also what clears the table between scenarios.
///
/// `Despawn`, NOT `Remove`: this is now reachable from every entity in the game
/// rather than from areas only, and a bare `remove::<EntityId>()` on a LIVE
/// body would drop its row while it is still physically inside the sensor -
/// after which [`on_collision_end_event`]'s missing-row guard swallows the real
/// exit and the area never fires `OnExit` for it.
///
/// PRUNE ONLY: the row is dropped silently, so a body destroyed inside an area
/// fires no `OnExit` for ITSELF. The only `OnExitEvent` is on the 1 -> 0
/// transition in [`on_collision_end_event`]. What this restores is the NEXT
/// body's ability to reach zero at all.
fn forget_body_occupancy(despawn: On<Despawn, EntityId>, mut occupancy: ResMut<AreaOccupancy>) {
    occupancy
        .0
        .retain(|(area, other), _| *area != despawn.entity && *other != despawn.entity);
}

/// Drop a dead COLLIDER from every occupancy set it is in - the sub-body twin of
/// [`forget_body_occupancy`], and what keeps a damaged ship able to leave an
/// area at all.
///
/// A ship sheds colliders while it flies: a destroyed section despawns, and its
/// skin plates go with it. avian fires no `CollisionEnd` for a collider that no
/// longer exists, so without this the set keeps entries that nothing can ever
/// remove and the body never reads as having left. The menu duel is the case
/// that found it - two ships trading fire inside a trigger volume, neither able
/// to trip its own exit.
///
/// PRUNE ONLY, like its sibling: an emptied set is dropped silently rather than
/// reported as an `OnExit`, because a body that lost its last collider inside an
/// area did not leave it.
///
/// Global, but it declines on an empty table first: a scenario with no areas
/// pays one resource read per collider despawn.
fn forget_collider_occupancy(despawn: On<Despawn, Collider>, mut occupancy: ResMut<AreaOccupancy>) {
    if occupancy.0.is_empty() {
        return;
    }

    occupancy.0.retain(|_, colliders| {
        colliders.remove(&despawn.entity);
        !colliders.is_empty()
    });
}

/// Arm a fresh area for collision reporting and bind its two handlers TO THAT
/// AREA.
///
/// Bound to the area, never a global `add_observer`: a global one dispatches
/// every collision anywhere in the world into this crate - 23,363 invocations
/// in four seconds of a headless duel that contains no areas at all, declined
/// on the first query. An entity observer costs nothing in a scenario with no
/// areas and scales with the areas, not with the world.
///
/// Scoping is also what makes `collider1` meaningful below. avian fires the
/// event once per side that has [`CollisionEventsEnabled`], with that side as
/// the target, so an observer bound to the area only ever sees the arm where
/// the area IS `collider1`.
fn wire_area_collisions(add: On<Add, ScenarioAreaMarker>, mut commands: Commands) {
    let entity = add.entity;
    trace!("wire_area_collisions: entity {:?}", entity);

    commands
        .entity(entity)
        .insert(CollisionEventsEnabled)
        .observe(on_collision_start_event)
        .observe(on_collision_end_event);
}

fn on_collision_start_event(
    collision: On<CollisionStart>,
    mut commands: Commands,
    mut occupancy: ResMut<AreaOccupancy>,
    q_area: Query<&EntityId, With<ScenarioAreaMarker>>,
    q_other: Query<(&EntityId, &EntityTypeName)>,
) {
    trace!(
        "on_collision_start_event: collision between {:?} and {:?}",
        collision.collider1,
        collision.body2
    );

    // Bound to the area by `wire_area_collisions`, so the event target IS the
    // area; the other side is whatever body owns `collider2`.
    let area = collision.collider1;
    let Ok(area_id) = q_area.get(area) else {
        return;
    };
    let Some(other) = collision.body2 else {
        return;
    };
    let Ok((other_id, other_type_name)) = q_other.get(other) else {
        return;
    };

    // One rigid body can present many colliders (a ship's sections), so avian
    // fires a CollisionStart per collider pair. Only the FIRST contact for this
    // (area, body) pair is a real entry - record the rest without re-firing.
    let colliders = occupancy.0.entry((area, other)).or_default();
    if !colliders.insert(collision.collider2) || colliders.len() > 1 {
        return;
    }

    commands.fire::<OnEnterEvent>(OnEnterEventInfo {
        id: area_id.0.clone(),
        other_id: other_id.0.clone(),
        other_type_name: other_type_name.0.clone(),
    });
}

fn on_collision_end_event(
    collision: On<CollisionEnd>,
    mut commands: Commands,
    mut occupancy: ResMut<AreaOccupancy>,
    q_area: Query<&EntityId, With<ScenarioAreaMarker>>,
    q_other: Query<(&EntityId, &EntityTypeName)>,
) {
    trace!(
        "on_collision_end_event: collision between {:?} and {:?}",
        collision.collider1,
        collision.body2
    );

    // Bound to the area, like the start handler.
    let area = collision.collider1;
    let Ok(area_id) = q_area.get(area) else {
        return;
    };
    let Some(other) = collision.body2 else {
        return;
    };
    let Ok((other_id, other_type_name)) = q_other.get(other) else {
        return;
    };

    // Mirror the start handler: only the LAST collider leaving is a real exit.
    // If we have no record (a start we never saw, or a collider already pruned
    // by its despawn), stay silent.
    let Some(colliders) = occupancy.0.get_mut(&(area, other)) else {
        return;
    };
    if !colliders.remove(&collision.collider2) || !colliders.is_empty() {
        return;
    }
    occupancy.0.remove(&(area, other));

    commands.fire::<OnExitEvent>(OnExitEventInfo {
        id: area_id.0.clone(),
        other_id: other_id.0.clone(),
        other_type_name: other_type_name.0.clone(),
    });
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use avian3d::prelude::{ColliderDensity, Gravity, LinearVelocity, PhysicsPlugins};
    use bevy::time::TimeUpdateStrategy;
    use nova_events::prelude::{EventHandler, GameEventsPlugin};
    use nova_gameplay::prelude::GameObjectives;

    use super::*;
    use crate::prelude::*;

    /// An area spawned AROUND an already-present body fires OnEnter - IF it
    /// carries the full production bundle: during this pin's discovery a
    /// Collider WITHOUT a RigidBody registered no contact pair at all,
    /// silently. With `RigidBody::Static` (what CreateScenarioArea spawns)
    /// avian starts the fresh overlapping pair even at full containment, so a
    /// scenario may create a trigger at a player already inside it and the beat
    /// still advances instead of soft-locking (the shakedown coast ring's
    /// sizing leans on this).
    #[test]
    fn an_area_spawned_around_a_body_fires_on_enter() {
        // The proven salvage-pipeline rig shape: zero gravity, manual fixed
        // steps, ScenarioAreaPlugin only.
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.02,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_plugins(ScenarioAreaPlugin);
        app.finish();

        let mut handler = EventHandler::<NovaEventWorld>::from(crate::events::EventConfig::OnEnter);
        handler.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
            id: Some("ring".to_string()),
            other_id: Some("ship".to_string()),
            ..Default::default()
        }));
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "entered".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(handler);
        let entered = |app: &App| -> bool {
            matches!(
                app.world()
                    .resource::<NovaEventWorld>()
                    .get_variable("entered"),
                Some(VariableLiteral::Boolean(true))
            )
        };

        // The body exists FIRST, settled, at what will be the area's
        // CENTER (full containment - the hardest case).
        app.world_mut().spawn((
            EntityId::new("ship".to_string()),
            EntityTypeName::new(SPACESHIP_TYPE_NAME),
            RigidBody::Dynamic,
            Collider::sphere(0.5),
            ColliderDensity(1.0),
            Transform::IDENTITY,
        ));
        for _ in 0..5 {
            app.update();
        }
        assert!(!entered(&app), "delivery guard: nothing before the spawn");

        // The exact production bundle CreateScenarioArea spawns.
        app.world_mut().spawn((
            ScenarioAreaMarker,
            EntityId::new("ring".to_string()),
            RigidBody::Static,
            Collider::sphere(50.0),
            Sensor,
            Transform::IDENTITY,
        ));
        for _ in 0..25 {
            app.update();
        }
        assert!(
            entered(&app),
            "spawning a trigger around a body must fire OnEnter (fresh contact pair)"
        );
    }

    /// A body that DESPAWNS inside a live area must take its occupancy row with
    /// it. avian fires no `CollisionEnd` for a despawned collider, so the row
    /// would otherwise stay at its non-zero count forever and the area could
    /// never be driven back to empty - the `OnExit` a scenario gates on would
    /// never fire again for that area.
    #[test]
    fn a_body_despawned_inside_an_area_drops_its_occupancy() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.02,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_plugins(ScenarioAreaPlugin);
        app.finish();

        let ship = app
            .world_mut()
            .spawn((
                EntityId::new("ship".to_string()),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
                RigidBody::Dynamic,
                Collider::sphere(0.5),
                ColliderDensity(1.0),
                Transform::IDENTITY,
            ))
            .id();
        app.world_mut().spawn((
            ScenarioAreaMarker,
            EntityId::new("ring".to_string()),
            RigidBody::Static,
            Collider::sphere(50.0),
            Sensor,
            Transform::IDENTITY,
        ));
        for _ in 0..25 {
            app.update();
        }
        assert!(
            !app.world().resource::<AreaOccupancy>().0.is_empty(),
            "delivery guard: the body must be counted as inside first"
        );

        app.world_mut().entity_mut(ship).despawn();
        app.update();

        assert!(
            app.world().resource::<AreaOccupancy>().0.is_empty(),
            "the despawned body's row must not outlive it"
        );
    }

    /// A compound body that LOSES a collider inside an area must still be able
    /// to leave it. A ship sheds colliders as it is shot apart - destroyed
    /// sections despawn, and their skin plates go with them - and avian fires no
    /// `CollisionEnd` for a collider that no longer exists. The counter this
    /// replaced could only climb: a block gunship fighting inside a menu-duel
    /// trigger volume took 270 starts against 149 ends, so the area never
    /// reported it leaving and an out-of-bounds rule built on `OnExit` never
    /// fired.
    #[test]
    fn a_compound_body_that_loses_a_collider_can_still_leave() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.02,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_plugins(ScenarioAreaPlugin);
        app.finish();

        let mut handler = EventHandler::<NovaEventWorld>::from(crate::events::EventConfig::OnExit);
        handler.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
            id: Some("ring".to_string()),
            other_id: Some("ship".to_string()),
            ..Default::default()
        }));
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "left".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(handler);
        let left = |app: &App| -> bool {
            matches!(
                app.world()
                    .resource::<NovaEventWorld>()
                    .get_variable("left"),
                Some(VariableLiteral::Boolean(true))
            )
        };

        let ship = app
            .world_mut()
            .spawn((
                EntityId::new("ship".to_string()),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
                RigidBody::Dynamic,
                Transform::IDENTITY,
            ))
            .id();
        let sections: Vec<Entity> = [-0.4_f32, 0.0, 0.4]
            .into_iter()
            .map(|dx| {
                app.world_mut()
                    .spawn((
                        Collider::sphere(0.5),
                        ColliderDensity(1.0),
                        Transform::from_xyz(dx, 0.0, 0.0),
                        ChildOf(ship),
                    ))
                    .id()
            })
            .collect();
        app.world_mut().spawn((
            ScenarioAreaMarker,
            EntityId::new("ring".to_string()),
            RigidBody::Static,
            Collider::sphere(50.0),
            Sensor,
            Transform::IDENTITY,
        ));
        for _ in 0..25 {
            app.update();
        }
        assert!(
            !app.world().resource::<AreaOccupancy>().0.is_empty(),
            "delivery guard: the whole body must be counted as inside first"
        );

        // One section is shot off INSIDE the area: its collider despawns and
        // avian will never report an end for it.
        app.world_mut().entity_mut(sections[0]).despawn();
        app.update();
        assert!(!left(&app), "losing a section is not leaving");

        // The rest of the ship flies out.
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(200.0, 0.0, 0.0)));
        for _ in 0..60 {
            app.update();
        }

        assert!(
            left(&app),
            "a body that lost a collider inside the area must still fire OnExit"
        );
    }

    /// A COMPOUND body - one rigid body wearing many section colliders, like a
    /// spaceship - must fire exactly ONE OnEnter, not one per collider. Regression
    /// for the racer's 18-section hull triple-triggering the salvage crate pickup
    /// (despawning a crate several times and over-counting the tally). Counts
    /// OnEnter deliveries by incrementing a variable each fire.
    #[test]
    fn a_compound_body_fires_one_on_enter() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.02,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_plugins(ScenarioAreaPlugin);
        app.finish();

        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("enters".to_string(), VariableLiteral::Number(0.0));
        let mut handler = EventHandler::<NovaEventWorld>::from(crate::events::EventConfig::OnEnter);
        handler.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
            id: Some("ring".to_string()),
            other_id: Some("ship".to_string()),
            ..Default::default()
        }));
        // enters = enters + 1 on every OnEnter delivery.
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "enters".to_string(),
            expression: VariableExpressionNode::new_add(
                VariableTermNode::new_factor(VariableFactorNode::new_name("enters".to_string())),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                )),
            ),
        }));
        app.world_mut().spawn(handler);
        let enters = |app: &App| -> f64 {
            match app
                .world()
                .resource::<NovaEventWorld>()
                .get_variable("enters")
            {
                Some(VariableLiteral::Number(n)) => *n,
                _ => -1.0,
            }
        };

        // A compound body: ONE rigid body wearing three section colliders (as a
        // ship's sections all share the ship's rigid body).
        let ship = app
            .world_mut()
            .spawn((
                EntityId::new("ship".to_string()),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
                RigidBody::Dynamic,
                Transform::IDENTITY,
            ))
            .id();
        for dx in [-0.4_f32, 0.0, 0.4] {
            app.world_mut().spawn((
                Collider::sphere(0.5),
                ColliderDensity(1.0),
                Transform::from_xyz(dx, 0.0, 0.0),
                ChildOf(ship),
            ));
        }
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(enters(&app), 0.0, "no area yet");

        app.world_mut().spawn((
            ScenarioAreaMarker,
            EntityId::new("ring".to_string()),
            RigidBody::Static,
            Collider::sphere(50.0),
            Sensor,
            Transform::IDENTITY,
        ));
        for _ in 0..25 {
            app.update();
        }
        assert_eq!(
            enters(&app),
            1.0,
            "a compound body fires exactly one OnEnter, not one per section collider"
        );
    }
}
