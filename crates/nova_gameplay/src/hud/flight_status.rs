//! Diegetic flight readouts (task 20260710-231926, spike
//! docs/spikes/20260710-234019-diegetic-flight-status.md): the old
//! bottom-left status text rehomed onto the ship - a speed chip parked
//! beside the velocity sphere and a mode chip (verb + phase) shown only
//! while the autopilot is engaged; manual flight keeps a quiet HUD. Plus
//! the projected marker on the GOTO destination.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{screen_indicator::prelude::*, NAV_CYAN};
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

/// On-screen size of the ship status chips (px).
const CHIP_SIZE: Vec2 = Vec2::new(120.0, 16.0);

/// The speed chip parks to the right of the ship, clear of the velocity
/// sphere (world radius 5.6 u for the outer gravity shell) at typical
/// chase-camera distance. Fixed px in v1; a projected-radius offset is the
/// richer option if the fixed one misbehaves at extreme zooms.
const SPEED_CHIP_OFFSET: Vec2 = Vec2::new(120.0, 0.0);

/// The mode chip stacks one row above the speed chip (screen y grows
/// downward).
const MODE_CHIP_OFFSET: Vec2 = Vec2::new(120.0, -18.0);

#[derive(Component, Debug, Clone, Reflect)]
pub struct FlightStatusHudMarker;

/// The ship whose flight state this readout shows.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct FlightStatusHudTargetEntity(pub Entity);

/// Marker for the speed chip.
#[derive(Component, Debug, Clone, Reflect)]
struct SpeedChipUIMarker;

/// Marker for the autopilot mode (verb + phase) chip.
#[derive(Component, Debug, Clone, Reflect)]
struct ModeChipUIMarker;

#[derive(Clone, Debug)]
pub struct FlightStatusHudConfig {
    pub target: Entity,
}

/// UI bundle for the ship status chips: one indicator layer with the speed
/// chip (anchored to the ship from spawn - it is always on) and the mode
/// chip (anchor driven at runtime; it spawns hidden exactly like the
/// disengaged state it starts in).
pub fn flight_status_hud(config: FlightStatusHudConfig) -> impl Bundle {
    debug!("flight_status_hud: config {:?}", config);

    let chip = |anchor: Option<ScreenIndicatorAnchorKind>, offset: Vec2| {
        screen_indicator(ScreenIndicatorConfig {
            anchor,
            size: ScreenIndicatorSize::Fixed(CHIP_SIZE),
            offset,
            offscreen: ScreenIndicatorOffscreen::Hide,
        })
    };

    (
        Name::new("FlightStatusHUD"),
        FlightStatusHudMarker,
        FlightStatusHudTargetEntity(config.target),
        screen_indicator_layer(),
        children![
            (
                Name::new("SpeedChipUI"),
                SpeedChipUIMarker,
                chip(
                    Some(ScreenIndicatorAnchorKind::Entity(config.target)),
                    SPEED_CHIP_OFFSET,
                ),
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                TextColor(NAV_CYAN),
            ),
            (
                Name::new("ModeChipUI"),
                ModeChipUIMarker,
                chip(None, MODE_CHIP_OFFSET),
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                TextColor(NAV_CYAN),
            ),
        ],
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
            ImageNode::new(config.marker_sprite.clone()).with_color(NAV_CYAN),
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
            (drive_speed_chip, drive_mode_chip, drive_destination_anchor)
                .before(ScreenIndicatorSystems)
                .in_set(super::NovaHudSystems),
        );
    }
}

/// The mode chip's label for an engaged autopilot: the verb and its phase.
fn mode_chip_label(autopilot: &Autopilot) -> String {
    let verb = match autopilot.action {
        AutopilotAction::Stop => "STOP",
        AutopilotAction::Goto { .. } | AutopilotAction::GotoPos { .. } => "GOTO",
        AutopilotAction::Orbit { .. } => "ORBIT",
    };
    let phase = match autopilot.phase {
        AutopilotPhase::Align => "ALIGN",
        AutopilotPhase::Burn => "BURN",
        AutopilotPhase::Hold => "HOLD",
    };
    format!("AP {verb} - {phase}")
}

/// The ship's speed beside the velocity sphere, always on. A dead ship
/// clears the anchor so the chip hides in the frame gap before the HUD
/// observer despawns the layer.
fn drive_speed_chip(
    q_hud: Query<&FlightStatusHudTargetEntity, With<FlightStatusHudMarker>>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Text, &ChildOf), With<SpeedChipUIMarker>>,
    q_ship: Query<&LinearVelocity>,
) {
    for (mut anchor, mut text, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };

        match q_ship.get(**ship) {
            Ok(velocity) => {
                // Re-assert the anchor so a transient query miss cannot
                // leave the chip dark while its text keeps updating.
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(**ship));
                **text = format!("{:5.1} u/s", velocity.length());
            }
            Err(_) => {
                **anchor = None;
                text.clear();
            }
        }
    }
}

/// The engaged maneuver's verb and phase above the speed chip; manual
/// flight (no [`Autopilot`]) shows nothing - a quiet HUD is the manual
/// look.
fn drive_mode_chip(
    q_hud: Query<&FlightStatusHudTargetEntity, With<FlightStatusHudMarker>>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Text, &ChildOf), With<ModeChipUIMarker>>,
    q_ship: Query<&Autopilot>,
) {
    for (mut anchor, mut text, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };

        match q_ship.get(**ship) {
            Ok(autopilot) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(**ship));
                **text = mode_chip_label(autopilot);
            }
            Err(_) => {
                **anchor = None;
                text.clear();
            }
        }
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

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn spawn_status_hud(world: &mut World, ship: Entity) -> (Entity, Entity) {
        let layer = world
            .spawn(flight_status_hud(FlightStatusHudConfig { target: ship }))
            .id();
        let children = world.entity(layer).get::<Children>().unwrap();
        (children[0], children[1])
    }

    fn anchor_of(world: &World, entity: Entity) -> Option<ScreenIndicatorAnchorKind> {
        **world.entity(entity).get::<ScreenIndicatorAnchor>().unwrap()
    }

    fn text_of(world: &World, entity: Entity) -> String {
        world.entity(entity).get::<Text>().unwrap().0.clone()
    }

    #[test]
    fn speed_chip_tracks_the_ship_and_hides_when_it_dies() {
        let mut world = World::new();
        let ship = world.spawn(LinearVelocity(Vec3::new(3.0, 0.0, 4.0))).id();
        let (speed, _) = spawn_status_hud(&mut world, ship);

        // Anchored to the ship from spawn: the chip is always on.
        assert_eq!(
            anchor_of(&world, speed),
            Some(ScreenIndicatorAnchorKind::Entity(ship))
        );

        world.run_system_once(drive_speed_chip).unwrap();
        assert_eq!(text_of(&world, speed), "  5.0 u/s");

        // The ship dies a frame before the HUD observer sweeps the layer.
        world.despawn(ship);
        world.run_system_once(drive_speed_chip).unwrap();
        assert_eq!(anchor_of(&world, speed), None);
        assert!(text_of(&world, speed).is_empty());
    }

    #[test]
    fn mode_chip_spawns_hidden_and_follows_engagement() {
        let mut world = World::new();
        let ship = world.spawn(LinearVelocity(Vec3::ZERO)).id();
        let (_, mode) = spawn_status_hud(&mut world, ship);

        // Manual from frame zero: hidden at spawn, hidden after a run.
        assert_eq!(anchor_of(&world, mode), None);
        world.run_system_once(drive_mode_chip).unwrap();
        assert_eq!(anchor_of(&world, mode), None);
        assert!(text_of(&world, mode).is_empty());

        // Engaging shows verb + phase.
        world
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        world.run_system_once(drive_mode_chip).unwrap();
        assert_eq!(
            anchor_of(&world, mode),
            Some(ScreenIndicatorAnchorKind::Entity(ship))
        );
        assert_eq!(text_of(&world, mode), "AP STOP - ALIGN");

        // Disengaging (component removed) hides it again.
        world.entity_mut(ship).remove::<Autopilot>();
        world.run_system_once(drive_mode_chip).unwrap();
        assert_eq!(anchor_of(&world, mode), None);
        assert!(text_of(&world, mode).is_empty());
    }

    #[test]
    fn mode_chip_labels_every_verb_and_phase() {
        let goto = Autopilot::engage(AutopilotAction::GotoPos {
            position: Vec3::ZERO,
        });
        assert_eq!(mode_chip_label(&goto), "AP GOTO - ALIGN");

        let mut orbit = Autopilot::engage(AutopilotAction::Orbit {
            well: Entity::PLACEHOLDER,
            plan: None,
        });
        orbit.phase = AutopilotPhase::Burn;
        assert_eq!(mode_chip_label(&orbit), "AP ORBIT - BURN");
        orbit.phase = AutopilotPhase::Hold;
        assert_eq!(mode_chip_label(&orbit), "AP ORBIT - HOLD");
    }

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
