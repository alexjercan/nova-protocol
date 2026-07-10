//! Minimal flight readouts: a one-line text status (manual vs engaged
//! maneuver, phase, speed, GOTO distance) and a projected marker on the GOTO
//! destination. The autopilot would be invisible without them; anything
//! richer is the diegetic-instruments task (20260709-103454).

use avian3d::prelude::*;
use bevy::prelude::*;

use super::screen_indicator::prelude::*;
use crate::{
    flight::{prelude::*, GravStatus},
    gravity::prelude::*,
};

pub mod prelude {
    pub use super::{
        autopilot_destination_hud, flight_status_hud, orbit_available_hud,
        AutopilotDestinationHudConfig, AutopilotDestinationHudMarker, AutopilotDestinationUIMarker,
        FlightStatusHudConfig, FlightStatusHudMarker, FlightStatusHudPlugin,
        FlightStatusHudTargetEntity, OrbitAvailableHudConfig, OrbitAvailableHudMarker,
        OrbitAvailableUIMarker,
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
                (drive_destination_anchor, drive_orbit_available_anchor)
                    .before(ScreenIndicatorSystems),
            )
                .in_set(super::NovaHudSystems),
        );
    }
}

fn update_flight_status_text(
    mut q_hud: Query<(&FlightStatusHudTargetEntity, &mut Text), With<FlightStatusHudMarker>>,
    q_ship: Query<(
        &GlobalTransform,
        &LinearVelocity,
        Option<&Autopilot>,
        Option<&DominantWell>,
    )>,
    q_target: Query<&GlobalTransform>,
    q_well: Query<(&GlobalTransform, Option<&Name>), With<GravityWell>>,
) {
    for (target, mut text) in &mut q_hud {
        let Ok((ship_transform, velocity, autopilot, dominant)) = q_ship.get(**target) else {
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
            AutopilotAction::Stop | AutopilotAction::Orbit { .. } => None,
        });

        // The well the line reports on: while ORBIT is engaged it is the
        // action's well (dominance can flip to another rock mid-orbit, and
        // an insertion overshoot can drop DominantWell entirely - the
        // readout must track what the computer actually flies); otherwise
        // the dominant well, when the ship is inside one. Either can be
        // dead for a flush, so the lookup degrades gracefully.
        let grav_well = match autopilot.map(|ap| ap.action) {
            Some(AutopilotAction::Orbit { well, .. }) => Some(well),
            _ => dominant.map(|d| **d),
        };
        let grav =
            grav_well
                .and_then(|well| q_well.get(well).ok())
                .map(|(well_transform, name)| GravStatus {
                    name: name.map(|n| n.as_str()).unwrap_or("WELL"),
                    radius: well_transform
                        .translation()
                        .distance(ship_transform.translation()),
                });

        **text =
            crate::flight::flight_status_line(velocity.length(), autopilot, goto_distance, grav);
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
            // The orbited well is the maneuver's destination.
            AutopilotAction::Orbit { well, .. } => Some(ScreenIndicatorAnchorKind::Entity(well)),
            AutopilotAction::Stop => None,
        });
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct OrbitAvailableHudMarker;

/// Marker for the inner, screen-projected cue node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct OrbitAvailableUIMarker;

/// The ship whose dominant well this cue points at.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
struct OrbitAvailableShipEntity(Entity);

#[derive(Clone, Debug)]
pub struct OrbitAvailableHudConfig {
    pub ship: Entity,
}

/// UI bundle for the orbit-available cue: while the ship coasts inside an
/// SOI (dominant well present, no ORBIT engaged), a small `[O] ORBIT` label
/// projects onto the well - the one-key hint that parking is on offer
/// (spike decision 7). The full diegetic keybind-hint system is task
/// 20260709-103454; this label is its first, hand-placed instance.
pub fn orbit_available_hud(config: OrbitAvailableHudConfig) -> impl Bundle {
    debug!("orbit_available_hud: config {:?}", config);

    (
        Name::new("OrbitAvailableHUD"),
        OrbitAvailableHudMarker,
        OrbitAvailableShipEntity(config.ship),
        screen_indicator_layer(),
        children![(
            Name::new("OrbitAvailableUI"),
            OrbitAvailableUIMarker,
            screen_indicator(ScreenIndicatorConfig {
                anchor: None,
                size: ScreenIndicatorSize::Fixed(Vec2::new(80.0, 16.0)),
                // Sit below the rock's center so the label reads as a
                // caption, not a lock.
                offset: Vec2::new(0.0, 48.0),
                offscreen: ScreenIndicatorOffscreen::Hide,
            }),
            Text::new("[O] ORBIT"),
            TextFont::from_font_size(12.0),
            TextColor(Color::srgba(0.3, 0.9, 1.0, 0.9)),
        )],
    )
}

/// Anchor the orbit-available cue to the ship's dominant well while no ORBIT
/// is engaged; leaving the SOI, losing the well, or engaging the maneuver
/// clears the anchor and the widget hides the label.
fn drive_orbit_available_anchor(
    q_hud: Query<&OrbitAvailableShipEntity, With<OrbitAvailableHudMarker>>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &ChildOf), With<OrbitAvailableUIMarker>>,
    q_ship: Query<(Option<&DominantWell>, Option<&Autopilot>)>,
) {
    for (mut anchor, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };

        **anchor = q_ship.get(**ship).ok().and_then(|(dominant, autopilot)| {
            let orbiting = matches!(
                autopilot.map(|ap| ap.action),
                Some(AutopilotAction::Orbit { .. })
            );
            match dominant {
                Some(well) if !orbiting => Some(ScreenIndicatorAnchorKind::Entity(**well)),
                _ => None,
            }
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

    fn spawn_orbit_cue(world: &mut World, ship: Entity) -> Entity {
        let layer = world
            .spawn(orbit_available_hud(OrbitAvailableHudConfig { ship }))
            .id();
        world
            .entity(layer)
            .get::<Children>()
            .expect("layer has the cue child")[0]
    }

    #[test]
    fn orbit_cue_shows_in_a_well_and_hides_while_orbiting() {
        let mut world = World::new();
        let well = world.spawn_empty().id();
        let ship = world.spawn(DominantWell(well)).id();
        let cue = spawn_orbit_cue(&mut world, ship);

        // Coasting inside the SOI: the cue anchors to the well.
        world.run_system_once(drive_orbit_available_anchor).unwrap();
        assert_eq!(
            **world.entity(cue).get::<ScreenIndicatorAnchor>().unwrap(),
            Some(ScreenIndicatorAnchorKind::Entity(well))
        );

        // Engaging ORBIT retires the offer.
        world
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));
        world.run_system_once(drive_orbit_available_anchor).unwrap();
        assert_eq!(
            **world.entity(cue).get::<ScreenIndicatorAnchor>().unwrap(),
            None
        );

        // Back to manual but outside every SOI: nothing to offer.
        world
            .entity_mut(ship)
            .remove::<Autopilot>()
            .remove::<DominantWell>();
        world.run_system_once(drive_orbit_available_anchor).unwrap();
        assert_eq!(
            **world.entity(cue).get::<ScreenIndicatorAnchor>().unwrap(),
            None
        );
    }

    #[test]
    fn destination_anchor_follows_the_orbited_well() {
        let mut world = World::new();
        let well = world.spawn_empty().id();
        let ship = world
            .spawn(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }))
            .id();
        let marker = spawn_destination_hud(&mut world, ship);

        world.run_system_once(drive_destination_anchor).unwrap();
        assert_eq!(
            **world.entity(marker).get::<ScreenIndicatorAnchor>().unwrap(),
            Some(ScreenIndicatorAnchorKind::Entity(well))
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
