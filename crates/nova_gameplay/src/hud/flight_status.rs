//! Minimal flight readouts: a one-line text status (manual vs engaged
//! maneuver, phase, speed, GOTO distance) and a projected marker on the GOTO
//! destination. The autopilot would be invisible without them; anything
//! richer is the diegetic-instruments task (20260709-103454).

use avian3d::prelude::*;
use bevy::prelude::*;

use super::screen_indicator::prelude::*;
use crate::flight::prelude::*;

pub mod prelude {
    pub use super::{
        autopilot_destination_hud, flight_status_hud, AutopilotDestinationHudConfig,
        AutopilotDestinationHudMarker, AutopilotDestinationUIMarker, FlightStatusHudConfig,
        FlightStatusHudMarker, FlightStatusHudPlugin, FlightStatusHudTargetEntity,
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

/// Marker for the inner, screen-projected marker node. Public so range
/// examples can assert on the marker's node state.
#[derive(Component, Debug, Clone, Reflect)]
pub struct AutopilotDestinationUIMarker;

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

/// UI bundle for the destination marker: a screen-projected indicator on the
/// engaged GOTO destination, fixed-size and tinted, visible only while a GOTO
/// is engaged. The screen_indicator widget owns projection and visibility;
/// this module only drives the anchor from the ship's [`Autopilot`].
pub fn autopilot_destination_hud(config: AutopilotDestinationHudConfig) -> impl Bundle {
    debug!("autopilot_destination_hud: config {:?}", config);

    (
        Name::new("AutopilotDestinationHUD"),
        AutopilotDestinationHudMarker,
        AutopilotDestinationShipEntity(config.ship),
        screen_indicator_layer(),
        children![(
            Name::new("AutopilotDestinationUI"),
            AutopilotDestinationUIMarker,
            screen_indicator(ScreenIndicatorConfig {
                anchor: None,
                size: ScreenIndicatorSize::Fixed(Vec2::splat(DESTINATION_MARKER_PX)),
                offset: Vec2::ZERO,
                offscreen: ScreenIndicatorOffscreen::Hide,
            }),
            // Reuse the target sprite, tinted toward "nav" cyan so it never
            // reads as a weapons lock.
            ImageNode::new(config.marker_sprite.clone())
                .with_color(Color::srgba(0.3, 0.9, 1.0, 0.9)),
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
            (
                update_flight_status_text,
                drive_destination_anchor.before(ScreenIndicatorSystems),
            )
                .in_set(super::NovaHudSystems),
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
            AutopilotAction::GotoPos { position } => {
                Some(position.distance(ship_transform.translation()))
            }
            AutopilotAction::Stop => None,
        });

        **text = crate::flight::flight_status_line(velocity.length(), autopilot, goto_distance);
    }
}

/// Anchor the destination marker to the engaged GOTO destination; manual
/// mode, STOP, or a vanished destination clear the anchor, and the widget
/// hides the marker (including while the destination is behind the camera).
fn drive_destination_anchor(
    q_hud: Query<&AutopilotDestinationShipEntity, With<AutopilotDestinationHudMarker>>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &ChildOf), With<AutopilotDestinationUIMarker>>,
    q_ship: Query<&Autopilot>,
) {
    for (mut anchor, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };

        **anchor = q_ship.get(**ship).ok().and_then(|ap| match ap.action {
            AutopilotAction::Goto { target } => Some(ScreenIndicatorAnchorKind::Entity(target)),
            AutopilotAction::GotoPos { position } => {
                Some(ScreenIndicatorAnchorKind::Point(position))
            }
            AutopilotAction::Stop => None,
        });
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn spawn_destination_hud(world: &mut World, ship: Entity) -> Entity {
        let layer = world
            .spawn(autopilot_destination_hud(
                AutopilotDestinationHudConfig::new(ship, Handle::default()),
            ))
            .id();
        world
            .entity(layer)
            .get::<Children>()
            .expect("layer has the marker child")[0]
    }

    #[test]
    fn destination_anchor_follows_the_engaged_goto() {
        let mut world = World::new();
        let destination = world.spawn_empty().id();
        let ship = world
            .spawn(Autopilot::engage(AutopilotAction::Goto {
                target: destination,
            }))
            .id();
        let marker = spawn_destination_hud(&mut world, ship);

        world.run_system_once(drive_destination_anchor).unwrap();
        assert_eq!(
            **world.entity(marker).get::<ScreenIndicatorAnchor>().unwrap(),
            Some(ScreenIndicatorAnchorKind::Entity(destination))
        );

        // STOP has no destination: the anchor clears and the widget hides.
        world
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        world.run_system_once(drive_destination_anchor).unwrap();
        assert_eq!(
            **world.entity(marker).get::<ScreenIndicatorAnchor>().unwrap(),
            None
        );
    }

    #[test]
    fn destination_anchor_clears_in_manual_mode() {
        let mut world = World::new();
        let destination = world.spawn_empty().id();
        let ship = world
            .spawn(Autopilot::engage(AutopilotAction::Goto {
                target: destination,
            }))
            .id();
        let marker = spawn_destination_hud(&mut world, ship);

        world.run_system_once(drive_destination_anchor).unwrap();
        assert!(world
            .entity(marker)
            .get::<ScreenIndicatorAnchor>()
            .unwrap()
            .is_some());

        // Disengaging the autopilot removes the component entirely.
        world.entity_mut(ship).remove::<Autopilot>();
        world.run_system_once(drive_destination_anchor).unwrap();
        assert_eq!(
            **world.entity(marker).get::<ScreenIndicatorAnchor>().unwrap(),
            None
        );
    }
}
