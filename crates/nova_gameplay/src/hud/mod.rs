use bevy::prelude::*;

use crate::prelude::*;

pub mod health;
pub mod objectives;
pub mod torpedo_target;
pub mod velocity;

pub mod prelude {
    pub use super::{
        health::prelude::*, objectives::prelude::*, torpedo_target::prelude::*,
        velocity::prelude::*, NovaHudAssets, NovaHudPlugin, NovaHudSystems,
    };
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaHudSystems;

#[derive(Resource, Clone, Default, Debug)]
pub struct NovaHudAssets {
    pub target_sprite: Handle<Image>,
}

#[derive(Default)]
pub struct NovaHudPlugin;

impl Plugin for NovaHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("HudPlugin: build");

        app.init_resource::<NovaHudAssets>();

        app.add_plugins(velocity::VelocityHudPlugin);
        app.add_plugins(health::HealthHudPlugin);
        app.add_plugins(objectives::ObjectivesHudPlugin);
        app.add_plugins(torpedo_target::TorpedoTargetHudPlugin);

        // Setup and remove HUDs when player spaceship is added/removed
        app.add_observer(setup_hud_velocity);
        app.add_observer(remove_hud_velocity);
        app.add_observer(setup_hud_health);
        app.add_observer(remove_hud_health);
        app.add_observer(setup_hud_objectives);
        app.add_observer(remove_hud_objectives);
        app.add_observer(setup_hud_torpedo_target);
        app.add_observer(remove_hud_torpedo_target);
    }
}

fn setup_hud_velocity(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_velocity: entity {:?}", entity);

    let Ok(spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_velocity: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((velocity_hud(VelocityHudConfig {
        radius: 5.0,
        sharpness: 20.0,
        target: spaceship,
    }),));
}

fn remove_hud_velocity(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<(Entity, &VelocityHudTargetEntity), With<VelocityHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_velocity: entity {:?}", entity);

    for (hud_entity, target) in &q_hud {
        if **target == entity {
            commands.entity(hud_entity).despawn();
        }
    }
}

fn setup_hud_health(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_health: entity {:?}", entity);

    let Ok(spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_health: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((health_hud(HealthHudConfig {
        target: Some(spaceship),
    }),));
}

fn remove_hud_health(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<(Entity, &HealthHudTargetEntity), With<HealthHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_health: entity {:?}", entity);

    for (hud_entity, target) in &q_hud {
        if let Some(target_entity) = **target {
            if target_entity == entity {
                commands.entity(hud_entity).despawn();
            }
        }
    }
}

fn setup_hud_objectives(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_objectives: entity {:?}", entity);

    let Ok(_) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_objectives: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((objectives_hud(ObjectiveRootHudConfig {}),));
}

fn remove_hud_objectives(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<ObjectiveRootHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_objectives: entity {:?}", entity);

    for hud_entity in &q_hud {
        commands.entity(hud_entity).despawn();
    }
}

fn setup_hud_torpedo_target(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    assets: Res<NovaHudAssets>,
) {
    let entity = add.entity;
    debug!("setup_hud_torpedo_target: entity {:?}", entity);

    let Ok(_spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_torpedo_target: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((torpedo_target_hud(TorpedoTargetHudConfig {
        target_sprite: assets.target_sprite.clone(),
    }),));
}

fn remove_hud_torpedo_target(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<TorpedoTargetHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_torpedo_target: entity {:?}", entity);

    for hud_entity in &q_hud {
        commands.entity(hud_entity).despawn();
    }
}
