use bevy::prelude::*;

use crate::prelude::*;

pub mod flight_status;
pub mod screen_indicator;
pub mod torpedo_target;
pub mod velocity;

pub mod prelude {
    pub use super::{
        flight_status::prelude::*, screen_indicator::prelude::*, torpedo_target::prelude::*,
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
        app.add_plugins(flight_status::FlightStatusHudPlugin);
        // The health and objectives HUDs are now the generic bevy_common_systems widgets.
        app.add_plugins(HealthDisplayPlugin);
        app.add_plugins(ObjectivesPlugin);
        app.add_plugins(screen_indicator::ScreenIndicatorPlugin);
        app.add_plugins(torpedo_target::TorpedoTargetHudPlugin);

        // Keep the generic HUD widgets inside nova's HUD ordering slot, as the local ones were.
        app.configure_sets(
            Update,
            (
                HealthDisplayPluginSystems::Sync,
                ObjectivesPluginSystems::Sync,
                ScreenIndicatorSystems,
            )
                .in_set(NovaHudSystems),
        );

        // Screen indicators project through the spaceship chase camera. The
        // widget is camera-agnostic (its own marker keeps it promotable), so
        // nova tags the camera whenever the controller hands it over.
        app.add_observer(add_screen_indicator_camera);
        app.add_observer(remove_screen_indicator_camera);

        // Setup and remove HUDs when player spaceship is added/removed
        app.add_observer(setup_hud_velocity);
        app.add_observer(remove_hud_velocity);
        app.add_observer(setup_hud_flight_status);
        app.add_observer(remove_hud_flight_status);
        app.add_observer(setup_hud_health);
        app.add_observer(remove_hud_health);
        app.add_observer(setup_hud_objectives);
        app.add_observer(remove_hud_objectives);
        app.add_observer(setup_hud_torpedo_target);
        app.add_observer(remove_hud_torpedo_target);
    }
}

/// Tag the spaceship chase camera as the projection camera for screen
/// indicators.
fn add_screen_indicator_camera(
    add: On<Add, crate::camera_controller::SpaceshipCameraController>,
    mut commands: Commands,
) {
    debug!("add_screen_indicator_camera: entity {:?}", add.entity);
    commands.entity(add.entity).insert(ScreenIndicatorCamera);
}

/// Untag the camera when the spaceship controller releases it (e.g. back to
/// the WASD camera after the player ship dies), so indicators hide instead of
/// projecting through a free camera.
fn remove_screen_indicator_camera(
    remove: On<Remove, crate::camera_controller::SpaceshipCameraController>,
    mut commands: Commands,
) {
    debug!("remove_screen_indicator_camera: entity {:?}", remove.entity);
    if let Ok(mut camera) = commands.get_entity(remove.entity) {
        camera.remove::<ScreenIndicatorCamera>();
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

fn setup_hud_flight_status(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    assets: Res<NovaHudAssets>,
) {
    let entity = add.entity;
    debug!("setup_hud_flight_status: entity {:?}", entity);

    let Ok(spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_flight_status: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((flight_status_hud(FlightStatusHudConfig {
        target: spaceship,
    }),));
    commands.spawn((autopilot_destination_hud(
        AutopilotDestinationHudConfig::new(spaceship, assets.target_sprite.clone()),
    ),));
}

fn remove_hud_flight_status(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<(Entity, &FlightStatusHudTargetEntity), With<FlightStatusHudMarker>>,
    q_destination: Query<Entity, With<AutopilotDestinationHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_flight_status: entity {:?}", entity);

    for (hud_entity, target) in &q_hud {
        if **target == entity {
            commands.entity(hud_entity).despawn();
        }
    }
    for hud_entity in &q_destination {
        commands.entity(hud_entity).despawn();
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

    commands.spawn((health_display(HealthDisplayConfig {
        target: Some(spaceship),
    }),));
}

fn remove_hud_health(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<(Entity, &HealthDisplayTarget), With<HealthDisplayMarker>>,
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

    commands.spawn((objectives_panel(ObjectivesPanelConfig {}),));
}

fn remove_hud_objectives(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<ObjectivesPanelMarker>>,
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
