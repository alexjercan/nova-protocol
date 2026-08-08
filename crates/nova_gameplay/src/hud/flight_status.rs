//! Diegetic flight readouts: the old bottom-left status text rehomed onto
//! the ship - a speed chip parked beside the velocity sphere and a mode chip
//! (verb + phase) shown only while the autopilot is engaged; manual flight
//! keeps a quiet HUD. Plus the projected marker on the GOTO destination.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_ui::hud::{chip_node, chip_paint, ChipTone};

use super::{emphasis::prelude::*, screen_indicator::prelude::*, situation::prelude::*, NAV_CYAN};
use crate::flight::prelude::*;

/// The flight-status and autopilot-destination spawners with their configs, markers and
/// `FlightStatusHudPlugin`.
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

/// The speed chip is the biggest readout on the flight HUD - it is the number
/// you fly by (demo 2 `.speed`, 15 px against the family's 12).
const SPEED_FONT_PX: f32 = 15.0;

/// Every other chip's text size (px).
const CHIP_FONT_PX: f32 = 12.0;

/// The speed chip parks to the right of the ship, clear of the velocity
/// sphere (world radius 5.6 u for the outer gravity shell) at typical
/// chase-camera distance. Fixed px in v1; a projected-radius offset is the
/// richer option if the fixed one misbehaves at extreme zooms.
/// Lifted clear of the bottom-centre keybind dock: the
/// ship sits low-centre under the chase camera, so a chip level with it landed
/// on the dock's chips. This is the demo's `.speed` band (~120 px off the
/// bottom) expressed as a ship-relative offset, so the readout stays parked on
/// the ship rather than becoming screen furniture.
const SPEED_CHIP_OFFSET: Vec2 = Vec2::new(120.0, -90.0);

/// The mode chip stacks one row above the speed chip (screen y grows
/// downward), keeping the same 24 px gap after the lift above.
const MODE_CHIP_OFFSET: Vec2 = Vec2::new(120.0, -114.0);

/// Peak scale of the speed chip while the autopilot flies - demo 2's
/// `.speed.emph`.
const SPEED_CHIP_EMPHASIS: f32 = 1.14;

/// Marker for the ship-status chip layer (speed chip + autopilot mode chip);
/// spawned by [`flight_status_hud`] and carried by the layer the drive systems
/// query.
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

/// Spawn-time settings for a [`flight_status_hud`] layer: the ship whose speed
/// and autopilot mode the chips report. Not a component - consumed by
/// [`flight_status_hud`].
#[derive(Clone, Debug)]
pub struct FlightStatusHudConfig {
    /// The ship whose speed and autopilot mode the chips report.
    pub target: Entity,
}

/// UI bundle for the ship status chips: one indicator layer with the speed
/// chip (anchored to the ship from spawn - it is always on) and the mode
/// chip (anchor driven at runtime; it spawns hidden exactly like the
/// disengaged state it starts in).
pub fn flight_status_hud(config: FlightStatusHudConfig) -> impl Bundle {
    debug!("flight_status_hud: config {:?}", config);

    // The chips hug their text (`Content`): a fixed box would either clip
    // "1.24 km/s" or pad "0 m/s" into an empty slab now that the chip has a
    // visible fill and border.
    let chip = |anchor: Option<ScreenIndicatorAnchorKind>, offset: Vec2| {
        screen_indicator_node(
            ScreenIndicatorConfig {
                anchor,
                size: ScreenIndicatorSize::Content,
                offset,
                offscreen: ScreenIndicatorOffscreen::Hide,
            },
            chip_node(),
        )
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
                TextFont::from_font_size(SPEED_FONT_PX),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                chip_paint(ChipTone::Phosphor),
                TextColor(ChipTone::Phosphor.text()),
                // Grows while the autopilot is burning - the readout you are
                // actually watching during a maneuver (demo 2's `.speed.emph`).
                HudEmphasis::settle(SPEED_CHIP_EMPHASIS),
            ),
            (
                Name::new("ModeChipUI"),
                ModeChipUIMarker,
                chip(None, MODE_CHIP_OFFSET),
                Text::new(""),
                TextFont::from_font_size(CHIP_FONT_PX),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                // The autopilot mode is an amber "the computer is flying"
                // statement, not a nav readout (demo 2 `.mode`).
                chip_paint(ChipTone::Amber),
                TextColor(ChipTone::Amber.text()),
            ),
        ],
    )
}

/// Marker for the autopilot-destination marker layer (the projected pip on the
/// engaged GOTO/ORBIT destination); spawned by [`autopilot_destination_hud`].
#[derive(Component, Debug, Clone, Reflect)]
pub struct AutopilotDestinationHudMarker;

/// Marker for the inner, screen-projected marker node. Public so range
/// examples can assert on the marker's node state.
#[derive(Component, Debug, Clone, Reflect)]
pub struct AutopilotDestinationUIMarker;

/// The ship whose engaged GOTO destination this marker projects.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
struct AutopilotDestinationShipEntity(Entity);

/// Spawn-time settings for an [`autopilot_destination_hud`] layer: the ship
/// whose engaged destination the pip projects, and the marker sprite to draw.
/// Not a component - consumed by [`autopilot_destination_hud`].
#[derive(Clone, Debug)]
pub struct AutopilotDestinationHudConfig {
    /// The ship whose engaged destination the pip projects.
    pub ship: Entity,
    /// The marker sprite to draw at the destination.
    pub marker_sprite: Handle<Image>,
}

impl AutopilotDestinationHudConfig {
    /// Builds a config for the given ship and destination-marker sprite.
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

/// Drives the diegetic flight readouts: the speed chip, the autopilot mode
/// chip, and the destination marker anchor.
/// Adds `drive_speed_chip`, `emphasize_speed_on_burn`, `drive_mode_chip` and
/// `drive_destination_anchor` in Update within [`super::NovaHudSystems`].
#[derive(Default)]
pub struct FlightStatusHudPlugin;

impl Plugin for FlightStatusHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("FlightStatusHudPlugin: build");

        app.add_systems(
            Update,
            (
                drive_speed_chip,
                emphasize_speed_on_burn,
                drive_mode_chip,
                drive_destination_anchor,
            )
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
                **text = nova_ui::units::speed(velocity.length());
            }
            Err(_) => {
                **anchor = None;
                text.clear();
            }
        }
    }
}

/// Emphasize the speed chip while a maneuver is engaged:
/// during an autopilot burn the speed is the number the player is watching, so
/// it grows and settles back the moment the maneuver ends.
fn emphasize_speed_on_burn(
    situations: Res<HudSituations>,
    mut q_ui: Query<&mut HudEmphasis, With<SpeedChipUIMarker>>,
) {
    for mut emphasis in &mut q_ui {
        emphasis.set_held(situations.maneuver.is_some());
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
    use crate::sections::controller_section::prelude::FlightVerb;

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
        // Ship velocity (3,0,4) has length 5.0 world u/s; at 1 u = 10 m the
        // live chip must read 50.0 m/s (would fail if the system no-opped or
        // skipped the x10 unit conversion).
        assert_eq!(text_of(&world, speed), "50.0 m/s");

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

    /// The burn emphasis: the speed chip is held large
    /// while a maneuver flies and lets go the moment it ends. Asserted on the
    /// HELD flag rather than the eased scale, so it pins the RULE; the easing
    /// itself is pinned in `hud::emphasis`.
    #[test]
    fn the_speed_chip_is_emphasized_only_while_a_maneuver_is_engaged() {
        let mut world = World::new();
        world.init_resource::<HudSituations>();
        let ship = world.spawn(LinearVelocity(Vec3::ZERO)).id();
        let (speed, _) = spawn_status_hud(&mut world, ship);

        world.run_system_once(emphasize_speed_on_burn).unwrap();
        assert!(
            !world.entity(speed).get::<HudEmphasis>().unwrap().held(),
            "manual flight leaves the speed chip at rest"
        );

        world.resource_mut::<HudSituations>().maneuver = Some(FlightVerb::Goto);
        world.run_system_once(emphasize_speed_on_burn).unwrap();
        assert!(
            world.entity(speed).get::<HudEmphasis>().unwrap().held(),
            "a GOTO burn emphasizes the number you are flying by"
        );

        world.resource_mut::<HudSituations>().maneuver = None;
        world.run_system_once(emphasize_speed_on_burn).unwrap();
        assert!(
            !world.entity(speed).get::<HudEmphasis>().unwrap().held(),
            "disengaging settles it back"
        );
    }
}
