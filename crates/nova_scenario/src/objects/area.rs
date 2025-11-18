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
