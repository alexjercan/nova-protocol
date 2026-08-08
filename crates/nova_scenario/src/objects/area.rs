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

/// Per-(area, body) count of overlapping collider pairs, so a compound body
/// entering an area fires exactly one `OnEnter` and one `OnExit`.
///
/// A spaceship is ONE rigid body wearing many section colliders, so avian fires
/// a separate `CollisionStart`/`CollisionEnd` per section collider that touches
/// an area sensor (empirically 3+ for the old trainer, 18 for a racer). Without
/// this, an `OnEnter` handler that is not idempotent - the salvage crate's
/// `despawn + crates_recovered += 1` - runs once PER section collider, despawning
/// a crate several times and over-counting the tally. Counting contacts collapses
/// the burst: `OnEnter` fires on the 0 -> 1 transition, `OnExit` on 1 -> 0.
#[derive(Resource, Default)]
struct AreaOccupancy(bevy::platform::collections::HashMap<(Entity, Entity), u32>);

/// Turns [`ScenarioAreaMarker`] sensor overlaps into scenario `OnEnter`/`OnExit`
/// events, deduping a compound body's many section colliders to one enter/exit.
/// Adds the collision-events setup observer plus the collision-start/end and
/// occupancy-cleanup observer (all observer-driven, no scheduled systems).
pub struct ScenarioAreaPlugin;

impl Plugin for ScenarioAreaPlugin {
    fn build(&self, app: &mut App) {
        debug!("AreaPlugin: build");

        app.init_resource::<AreaOccupancy>();
        app.add_observer(insert_collision_events);
        app.add_observer(on_collision_start_event);
        app.add_observer(on_collision_end_event);
        app.add_observer(forget_body_occupancy);
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

fn insert_collision_events(add: On<Add, ScenarioAreaMarker>, mut commands: Commands) {
    let entity = add.entity;
    trace!("insert_collision_events: entity {:?}", entity);

    commands.entity(entity).insert(CollisionEventsEnabled);
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
        collision.body1,
        collision.body2
    );

    // avian does not guarantee which of body1/body2 is the area, so resolve it
    // from either side (matches the crate-pickup SFX observer).
    let (Some(a), Some(b)) = (collision.body1, collision.body2) else {
        return;
    };
    let (body, other) = if q_area.get(a).is_ok() {
        (a, b)
    } else if q_area.get(b).is_ok() {
        (b, a)
    } else {
        return;
    };
    let Ok(area_id) = q_area.get(body) else {
        return;
    };
    let Ok((other_id, other_type_name)) = q_other.get(other) else {
        return;
    };

    // One rigid body can present many colliders (a ship's sections), so avian
    // fires a CollisionStart per collider pair. Only the FIRST contact for this
    // (area, body) pair is a real entry - count the rest without re-firing.
    let count = occupancy.0.entry((body, other)).or_insert(0);
    *count += 1;
    if *count > 1 {
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
        collision.body1,
        collision.body2
    );

    // Resolve the area from either body, like the start handler.
    let (Some(a), Some(b)) = (collision.body1, collision.body2) else {
        return;
    };
    let (body, other) = if q_area.get(a).is_ok() {
        (a, b)
    } else if q_area.get(b).is_ok() {
        (b, a)
    } else {
        return;
    };
    let Ok(area_id) = q_area.get(body) else {
        return;
    };
    let Ok((other_id, other_type_name)) = q_other.get(other) else {
        return;
    };

    // Mirror the start counter: only the LAST collider pair leaving is a real
    // exit. If we have no record (a start we never saw), stay silent.
    let Some(count) = occupancy.0.get_mut(&(body, other)) else {
        return;
    };
    *count = count.saturating_sub(1);
    if *count > 0 {
        return;
    }
    occupancy.0.remove(&(body, other));

    commands.fire::<OnExitEvent>(OnExitEventInfo {
        id: area_id.0.clone(),
        other_id: other_id.0.clone(),
        other_type_name: other_type_name.0.clone(),
    });
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use avian3d::prelude::{ColliderDensity, Gravity, PhysicsPlugins};
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
            EntityTypeName::new("spaceship".to_string()),
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
                EntityTypeName::new("spaceship".to_string()),
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
                EntityTypeName::new("spaceship".to_string()),
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
