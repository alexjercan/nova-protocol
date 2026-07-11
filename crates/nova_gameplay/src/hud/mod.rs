use bevy::prelude::*;

use crate::prelude::*;

pub mod component_lock;
pub mod flight_status;
pub mod holo_instruments;
pub mod keybind_hints;
pub mod maneuver_instruments;
pub mod screen_indicator;
pub mod torpedo_target;
pub mod turret_lead;
pub mod velocity;

pub mod prelude {
    pub use super::{
        component_lock::prelude::*, flight_status::prelude::*, holo_instruments::prelude::*,
        keybind_hints::prelude::*, maneuver_instruments::prelude::*, screen_indicator::prelude::*,
        torpedo_target::prelude::*, turret_lead::prelude::*, velocity::prelude::*, NovaHudAssets,
        NovaHudPlugin, NovaHudSystems,
    };
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaHudSystems;

/// Nav cyan, the family color of every flight-computer projection (the
/// destination marker tint, the orbit cue, the maneuver chips, the holo
/// ring).
pub(crate) const NAV_CYAN: Color = Color::srgba(0.3, 0.9, 1.0, 0.9);

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
        app.add_plugins(maneuver_instruments::ManeuverInstrumentsPlugin);
        app.add_plugins(keybind_hints::KeybindHintsPlugin);
        app.add_plugins(holo_instruments::HoloInstrumentsPlugin);
        // The health and objectives HUDs are now the generic bevy_common_systems widgets.
        app.add_plugins(HealthDisplayPlugin);
        app.add_plugins(ObjectivesPlugin);
        app.add_plugins(screen_indicator::ScreenIndicatorPlugin);
        app.add_plugins(torpedo_target::TorpedoTargetHudPlugin);
        app.add_plugins(turret_lead::TurretLeadPlugin);
        app.add_plugins(component_lock::ComponentLockHudPlugin);

        // Keep the generic HUD widgets inside nova's HUD ordering slot, as the local ones were.
        // ScreenIndicatorSystems is NOT in this Update slot anymore: the
        // projection runs in PostUpdate after the chase camera's final move
        // (task 20260710-231928), and the Update-schedule driver systems
        // precede it by schedule order alone.
        app.configure_sets(
            Update,
            (
                HealthDisplayPluginSystems::Sync,
                ObjectivesPluginSystems::Sync,
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
        app.add_observer(setup_hud_turret_lead);
        app.add_observer(remove_hud_turret_lead);
        app.add_observer(setup_hud_component_lock);
        app.add_observer(remove_hud_component_lock);
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
        ..default()
    }),));
    // The gravity indicator: same widget, yellow, pointing down the
    // dominant well's pull, hidden in flat space. Nested slightly outside
    // the velocity sphere so the two shells never z-fight.
    commands.spawn((velocity_hud(VelocityHudConfig {
        radius: 5.6,
        sharpness: 20.0,
        target: spaceship,
        source: VelocityHudSource::Gravity,
        palette: VelocityHudPalette::GRAVITY,
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
    q_existing_cluster: Query<(), With<KeybindHintClusterMarker>>,
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
    commands.spawn((maneuver_instruments_hud(ManeuverInstrumentsHudConfig {
        ship: spaceship,
    }),));
    // The cluster and cues are global singletons, not ship-targeted
    // widgets: one player, one set (same guard as the flight input rig).
    if q_existing_cluster.is_empty() {
        commands.spawn((keybind_hint_cluster_hud(),));
        commands.spawn((verb_cues_hud(),));
    }
}

fn remove_hud_flight_status(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<(Entity, &FlightStatusHudTargetEntity), With<FlightStatusHudMarker>>,
    q_destination: Query<Entity, With<AutopilotDestinationHudMarker>>,
    q_cluster: Query<Entity, With<KeybindHintClusterMarker>>,
    q_cues: Query<Entity, With<VerbCuesHudMarker>>,
    q_instruments: Query<Entity, With<ManeuverInstrumentsHudMarker>>,
    q_ring: Query<Entity, With<OrbitRingMarker>>,
    q_spoke: Query<Entity, With<RadiusSpokeMarker>>,
    q_ribbon: Query<Entity, With<TrajectoryRibbonSegment>>,
    q_gate: Query<Entity, With<FlipGateMarker>>,
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
    for hud_entity in &q_cluster {
        commands.entity(hud_entity).despawn();
    }
    for hud_entity in &q_cues {
        commands.entity(hud_entity).despawn();
    }
    for hud_entity in &q_instruments {
        commands.entity(hud_entity).despawn();
    }
    for hud_entity in &q_ring {
        commands.entity(hud_entity).despawn();
    }
    for hud_entity in q_spoke.iter().chain(q_ribbon.iter()).chain(&q_gate) {
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

fn setup_hud_turret_lead(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_turret_lead: entity {:?}", entity);

    let Ok(_spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_turret_lead: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn(turret_lead_hud());
}

fn remove_hud_turret_lead(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<TurretLeadHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_turret_lead: entity {:?}", entity);

    for hud_entity in &q_hud {
        commands.entity(hud_entity).despawn();
    }
}

fn setup_hud_component_lock(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_component_lock: entity {:?}", entity);

    let Ok(_spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_component_lock: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn(component_lock_hud());
}

fn remove_hud_component_lock(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<ComponentLockHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_component_lock: entity {:?}", entity);

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
