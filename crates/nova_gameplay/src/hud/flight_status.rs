//! Minimal flight readouts: a one-line text status (manual vs engaged
//! maneuver, phase, speed, GOTO distance) and a projected marker on the GOTO
//! destination. The autopilot would be invisible without them; anything
//! richer is the diegetic-instruments task (20260709-103454).

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{camera_controller::SpaceshipCameraController, flight::prelude::*};

pub mod prelude {
    pub use super::{
        autopilot_destination_hud, flight_status_hud, AutopilotDestinationHudConfig,
        AutopilotDestinationHudMarker, FlightStatusHudConfig, FlightStatusHudMarker,
        FlightStatusHudPlugin, FlightStatusHudTargetEntity,
    };
}

/// Fixed on-screen size (px) of the destination marker. Unlike the target
/// reticle it does not track apparent size - it marks a nav point, not a
/// silhouette.
const DESTINATION_MARKER_PX: f32 = 24.0;

#[derive(Component, Debug, Clone, Reflect)]
pub struct FlightStatusHudMarker;

/// The ship whose flight state this readout shows.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct FlightStatusHudTargetEntity(pub Entity);

#[derive(Clone, Debug)]
pub struct FlightStatusHudConfig {
    pub target: Entity,
}

/// UI bundle for the readout: a small fixed text node in the lower-left,
/// clear of the velocity sphere and the health bar.
pub fn flight_status_hud(config: FlightStatusHudConfig) -> impl Bundle {
    debug!("flight_status_hud: config {:?}", config);

    (
        Name::new("FlightStatusHUD"),
        FlightStatusHudMarker,
        FlightStatusHudTargetEntity(config.target),
        Text::new(""),
        TextFont::from_font_size(14.0),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    )
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct AutopilotDestinationHudMarker;

/// Marker for the inner, absolutely-positioned marker node.
#[derive(Component, Debug, Clone, Reflect)]
struct AutopilotDestinationUIMarker;

/// The ship whose engaged GOTO destination this marker projects.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
struct AutopilotDestinationShipEntity(Entity);

#[derive(Clone, Debug)]
pub struct AutopilotDestinationHudConfig {
    pub ship: Entity,
    pub marker_sprite: Handle<Image>,
}

impl AutopilotDestinationHudConfig {
    pub fn new(ship: Entity, marker_sprite: Handle<Image>) -> Self {
        Self {
            ship,
            marker_sprite,
        }
    }
}

/// UI bundle for the destination marker: the same full-screen click-through
/// layer + projected child the torpedo-target reticle uses, but fixed-size
/// and tinted, visible only while a GOTO is engaged.
pub fn autopilot_destination_hud(config: AutopilotDestinationHudConfig) -> impl Bundle {
    debug!("autopilot_destination_hud: config {:?}", config);

    (
        Name::new("AutopilotDestinationHUD"),
        AutopilotDestinationHudMarker,
        AutopilotDestinationShipEntity(config.ship),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Pickable::IGNORE,
        children![(
            Name::new("AutopilotDestinationUI"),
            AutopilotDestinationUIMarker,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(DESTINATION_MARKER_PX),
                height: Val::Px(DESTINATION_MARKER_PX),
                ..default()
            },
            // Reuse the target sprite, tinted toward "nav" cyan so it never
            // reads as a weapons lock.
            ImageNode::new(config.marker_sprite.clone())
                .with_color(Color::srgba(0.3, 0.9, 1.0, 0.9)),
            Pickable::IGNORE,
            Visibility::Hidden,
        )],
    )
}

#[derive(Default)]
pub struct FlightStatusHudPlugin;

impl Plugin for FlightStatusHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("FlightStatusHudPlugin: build");

        app.add_systems(
            Update,
            (update_flight_status_text, update_destination_marker).in_set(super::NovaHudSystems),
        );
    }
}

fn update_flight_status_text(
    mut q_hud: Query<(&FlightStatusHudTargetEntity, &mut Text), With<FlightStatusHudMarker>>,
    q_ship: Query<(&GlobalTransform, &LinearVelocity, Option<&Autopilot>)>,
    q_target: Query<&GlobalTransform>,
) {
    for (target, mut text) in &mut q_hud {
        let Ok((ship_transform, velocity, autopilot)) = q_ship.get(**target) else {
            // The ship can die a frame before the HUD is despawned.
            continue;
        };

        // Distance to an engaged GOTO destination, when it still exists.
        let goto_distance = autopilot.and_then(|ap| match ap.action {
            AutopilotAction::Goto { target } => q_target
                .get(target)
                .ok()
                .map(|t| t.translation().distance(ship_transform.translation())),
            AutopilotAction::Stop => None,
        });

        **text = crate::flight::flight_status_line(velocity.length(), autopilot, goto_distance);
    }
}

/// Project the engaged GOTO destination to the screen; hidden in manual mode,
/// during STOP, or while the destination is off-screen/behind the camera.
fn update_destination_marker(
    q_hud: Query<&AutopilotDestinationShipEntity, With<AutopilotDestinationHudMarker>>,
    mut q_ui: Query<(&mut Node, &mut Visibility, &ChildOf), With<AutopilotDestinationUIMarker>>,
    q_ship: Query<&Autopilot>,
    q_transform: Query<&GlobalTransform>,
    main_camera: Single<(&GlobalTransform, &Camera), With<SpaceshipCameraController>>,
) {
    let (camera_transform, camera) = main_camera.into_inner();

    for (mut node, mut visibility, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };

        let destination = q_ship.get(**ship).ok().and_then(|ap| match ap.action {
            AutopilotAction::Goto { target } => Some(target),
            AutopilotAction::Stop => None,
        });
        let center = destination
            .and_then(|d| q_transform.get(d).ok())
            .and_then(|t| {
                camera
                    .world_to_viewport(camera_transform, t.translation())
                    .ok()
            });

        match center {
            Some(center) => {
                *visibility = Visibility::Visible;
                node.left = Val::Px(center.x - DESTINATION_MARKER_PX / 2.0);
                node.top = Val::Px(center.y - DESTINATION_MARKER_PX / 2.0);
            }
            None => {
                *visibility = Visibility::Hidden;
            }
        }
    }
}
