use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

pub mod prelude {
    pub use super::{ScenarioAreaMarker, ScenarioAreaPlugin};
}

#[derive(Component, Debug, Clone, Reflect)]
#[require(Collider, Sensor)]
pub struct ScenarioAreaMarker;

pub struct ScenarioAreaPlugin;

impl Plugin for ScenarioAreaPlugin {
    fn build(&self, app: &mut App) {
        debug!("AreaPlugin: build");

        app.add_observer(insert_collision_events);
        app.add_observer(on_collision_start_event);
        app.add_observer(on_collision_end_event);
    }
}

fn insert_collision_events(add: On<Add, ScenarioAreaMarker>, mut commands: Commands) {
    let entity = add.entity;
    trace!("insert_collision_events: entity {:?}", entity);

    commands.entity(entity).insert(CollisionEventsEnabled);
}

fn on_collision_start_event(
    collision: On<CollisionStart>,
    mut commands: Commands,
    q_area: Query<&EntityId, With<ScenarioAreaMarker>>,
    q_other: Query<(&EntityId, &EntityTypeName)>,
) {
    trace!(
        "on_collision_start_event: collision between {:?} and {:?}",
        collision.body1,
        collision.body2
    );

    let Some(body) = collision.body1 else {
        return;
    };
    let Some(other) = collision.body2 else {
        return;
    };

    let Ok(area_id) = q_area.get(body) else {
        return;
    };

    let Ok((other_id, other_type_name)) = q_other.get(other) else {
        return;
    };

    commands.fire::<OnEnterEvent>(OnEnterEventInfo {
        id: area_id.0.clone(),
        other_id: other_id.0.clone(),
        other_type_name: other_type_name.0.clone(),
    });
}

fn on_collision_end_event(
    collision: On<CollisionEnd>,
    mut commands: Commands,
    q_area: Query<&EntityId, With<ScenarioAreaMarker>>,
    q_other: Query<(&EntityId, &EntityTypeName)>,
) {
    trace!(
        "on_collision_end_event: collision between {:?} and {:?}",
        collision.body1,
        collision.body2
    );

    let Some(body) = collision.body1 else {
        return;
    };
    let Some(other) = collision.body2 else {
        return;
    };

    let Ok(area_id) = q_area.get(body) else {
        return;
    };

    let Ok((other_id, other_type_name)) = q_other.get(other) else {
        return;
    };

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
    use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};

    use super::*;
    use crate::prelude::*;

    /// An area spawned AROUND an already-present body fires OnEnter - IF it
    /// carries the full production bundle: during this pin's discovery
    /// (task 20260713-150343) a Collider WITHOUT a RigidBody registered no
    /// contact pair at all, silently. With `RigidBody::Static` (what
    /// CreateScenarioArea spawns) avian starts the fresh overlapping pair
    /// even at full containment, so a scenario may create a trigger at a
    /// player already inside it and the beat still advances instead of
    /// soft-locking (the shakedown coast ring's sizing leans on this).
    #[test]
    fn an_area_spawned_around_a_body_fires_on_enter() {
        // The proven salvage-pipeline rig shape (20260712-093044): zero
        // gravity, manual fixed steps, ScenarioAreaPlugin only.
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
}
