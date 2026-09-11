//! Diegetic flight readouts: the old bottom-left status text rehomed onto
//! the ship - a speed chip parked beside the velocity sphere and a mode chip
//! (verb + phase) shown only while the autopilot is engaged; manual flight
//! keeps a quiet HUD. Plus the projected marker on the GOTO destination.
//!
//! Anything measured here is an ENGINE figure - a world unit is 10 m, a speed
//! is world units per second - because it comes off a bevy transform or an
//! avian `LinearVelocity`. Readouts cross into meters once, through
//! `Meters::from_engine` / `MetersPerSecond::from_engine`, and `nova_ui::units`
//! does the formatting; nothing here converts by hand.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_ship::{flight::prelude::*, prelude::CameraAuthoritySystems};
use nova_ui::hud::{chip_node, chip_paint, ChipTone};

use super::{
    emphasis::prelude::*, hull_shell::prelude::*, screen_indicator::prelude::*,
    situation::prelude::*, NAV_CYAN,
};

/// The flight-status and autopilot-destination spawners with their configs, markers and
/// `FlightStatusHudPlugin`.
pub mod prelude {
    pub use super::{
        autopilot_destination_hud, flight_status_hud, AutopilotDestinationHudConfig,
        AutopilotDestinationHudMarker, AutopilotDestinationUIMarker, FlightStatusHudConfig,
        FlightStatusHudMarker, FlightStatusHudPlugin, FlightStatusHudTargetEntity,
        ModeChipUIMarker, SpeedChipUIMarker, DESTINATION_MARKER_PX,
    };
}

/// Fixed on-screen size (px) of the destination marker. Unlike the target
/// reticle it does not track apparent size - it marks a nav point, not a
/// silhouette. Public because the widgets that sit AROUND the marker derive
/// their placement from it rather than hand-matching a second number.
pub const DESTINATION_MARKER_PX: f32 = 24.0;

/// The speed chip is the biggest readout on the flight HUD - it is the number
/// you fly by (demo 2 `.speed`, 15 px against the family's 12).
const SPEED_FONT_PX: f32 = 15.0;

/// Every other chip's text size (px).
const CHIP_FONT_PX: f32 = 12.0;

/// The clear space between the projected edge of the outer gravity shell and
/// the NEAR EDGE of a flight chip (px). A fixed pixel gap rather than a world
/// distance: this is the space between a drawn sphere and a text chip, and that
/// reads the same whatever the hull behind it measures.
const CHIP_SHELL_GAP_PX: f32 = 12.0;

/// How far the speed chip rides above the ship's centre of mass (px; screen y
/// grows downward). Lifted clear of the bottom-centre keybind dock: the ship
/// sits low-centre under the chase camera, so a chip level with it landed on
/// the dock's chips. This is the demo's `.speed` band (~120 px off the bottom)
/// expressed as a ship-relative offset, so the readout stays parked on the ship
/// rather than becoming screen furniture.
const SPEED_CHIP_LIFT_PX: f32 = -90.0;

/// The mode chip stacks one row above the speed chip, keeping the same 24 px
/// gap after the lift above.
const MODE_CHIP_LIFT_PX: f32 = -114.0;

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

/// Marker for the speed chip. Public so range examples can assert on where the
/// chip ended up.
#[derive(Component, Debug, Clone, Reflect)]
pub struct SpeedChipUIMarker;

/// Marker for the autopilot mode (verb + phase) chip. Public for the same
/// reason as [`SpeedChipUIMarker`].
#[derive(Component, Debug, Clone, Reflect)]
pub struct ModeChipUIMarker;

/// Which row a flight chip rides on (px above its ship's centre of mass) and,
/// by carrying it, that the chip is one [`anchor_flight_chips`] places. The
/// horizontal half of the placement is not authored here - it is whatever this
/// frame's outer gravity shell projects to.
#[derive(Component, Debug, Clone, Copy, Deref, Reflect)]
struct FlightChipLift(f32);

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
    trace!("flight_status_hud: config {:?}", config);

    // The chips hug their text (`Content`): a fixed box would either clip
    // "1.24 km/s" or pad "0 m/s" into an empty slab now that the chip has a
    // visible fill and border.
    //
    // The spawn offset is the bare gap on the chip's own row: until the hull
    // has published an envelope there is no shell edge to stand outside of, and
    // hugging the centre of mass for the frame or two a hull takes to assemble
    // is better than parking at a guessed radius.
    let chip = |anchor: Option<ScreenIndicatorAnchorKind>, lift: f32| {
        (
            FlightChipLift(lift),
            screen_indicator_node(
                ScreenIndicatorConfig {
                    anchor,
                    size: ScreenIndicatorSize::Content,
                    offset: Vec2::new(CHIP_SHELL_GAP_PX, lift),
                    offscreen: ScreenIndicatorOffscreen::Hide,
                },
                chip_node(),
            ),
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
                    SPEED_CHIP_LIFT_PX,
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
                chip(None, MODE_CHIP_LIFT_PX),
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
    trace!("autopilot_destination_hud: config {:?}", config);

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
            ImageNode::new(config.marker_sprite).with_color(NAV_CYAN),
        )],
    )
}

/// Drives the diegetic flight readouts: the speed chip, the autopilot mode
/// chip, and the destination marker anchor.
/// Adds `drive_speed_chip`, `emphasize_speed_on_burn`, `drive_mode_chip` and
/// `drive_destination_anchor` in Update within [`super::NovaHudSystems`], plus
/// `anchor_flight_chips` in PostUpdate between the chase camera and the
/// indicator projection.
#[derive(Default)]
pub struct FlightStatusHudPlugin;

impl Plugin for FlightStatusHudPlugin {
    fn build(&self, app: &mut App) {
        trace!("FlightStatusHudPlugin: build");

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
        // Placement needs the camera pose this frame RENDERS with, so it sits
        // in the same PostUpdate slot the projection does: after the last
        // camera writer, before the indicators read the anchors. In Update the
        // camera has not moved yet and a chip parked off a projected world
        // radius would lag every zoom by a frame.
        app.add_systems(
            PostUpdate,
            anchor_flight_chips
                .after(CameraAuthoritySystems::Override)
                .before(ScreenIndicatorSystems),
        );
    }
}

/// Park the live flight chips just outside the ship's outer gravity shell.
///
/// The chips used to sit a fixed 120 px right of the ship ROOT, which is beside
/// the shell only while the shell is a constant too. Both ends of that are now
/// live: the shell is sized off the hull, and the hull's centre of mass - not
/// its root origin, which on an asymmetric build is somewhere inside the
/// structure - is what both spheres are centred on.
///
/// So the chip is anchored on the centre of mass and pushed out by the shell's
/// own projected radius, plus [`CHIP_SHELL_GAP_PX`] and half the chip's own
/// width: a projected world radius tracks hull size and camera zoom together,
/// which a pixel constant cannot.
/// The OUTER radius is used even in flat space, where the gravity shell is
/// hidden, so the readouts do not jump sideways when a ship leaves a well.
///
/// Whether a chip is shown at all stays with its driver: this only moves the
/// ones that are already on.
fn anchor_flight_chips(
    envelopes: Res<HudShellEnvelopes>,
    q_hud: Query<&FlightStatusHudTargetEntity, With<FlightStatusHudMarker>>,
    mut q_chip: Query<(
        &mut ScreenIndicatorAnchor,
        &mut ScreenIndicatorOffset,
        &FlightChipLift,
        &ComputedNode,
        &ChildOf,
    )>,
    q_ship: Query<Option<&ComputedCenterOfMass>>,
    // UI layout runs before this frame's transform propagation, so
    // `GlobalTransform` is still last frame's. Composing the camera and the
    // ship fresh is what the indicator pass itself does, and the chip has to
    // agree with it to the pixel.
    transform_helper: TransformHelper,
    q_camera: Query<(Entity, &Camera), With<ScreenIndicatorCamera>>,
) {
    let Some((camera_entity, camera)) = q_camera.iter().next() else {
        return;
    };
    let Ok(camera_transform) = transform_helper.compute_global_transform(camera_entity) else {
        return;
    };

    for (mut anchor, mut offset, lift, computed, &ChildOf(parent)) in &mut q_chip {
        if anchor.is_none() {
            continue;
        }
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };
        let Ok(center_of_mass) = q_ship.get(**ship) else {
            continue;
        };
        let Ok(ship_transform) = transform_helper.compute_global_transform(**ship) else {
            continue;
        };
        let com = match center_of_mass {
            Some(center_of_mass) => ship_transform.transform_point(center_of_mass.0),
            None => ship_transform.translation(),
        };
        anchor.set_if_neq(ScreenIndicatorAnchor(Some(
            ScreenIndicatorAnchorKind::Point(com),
        )));

        // No envelope yet (a hull still assembling): the chip keeps the offset
        // it has rather than snapping to a radius nobody has measured.
        let Some(radius) = outer_shell_radius(&envelopes, **ship) else {
            continue;
        };
        // One radius along the camera's own right vector, so the projected
        // distance is the shell's on-screen radius at this zoom whatever the
        // ship's attitude.
        let edge = com + camera_transform.right() * radius;
        let (Ok(projected_com), Ok(projected_edge)) = (
            camera.world_to_viewport(&camera_transform, com),
            camera.world_to_viewport(&camera_transform, edge),
        ) else {
            continue;
        };
        // The widget CENTRES the node on the offset point, so the gap is a
        // gap only once half the chip is added to it. `ComputedNode::size` is
        // physical and is last frame's measurement - the same one the widget
        // centres `Content` chips with - so a chip that changes length is off
        // by half that change for exactly one frame.
        let half_width = computed.size().x * computed.inverse_scale_factor() / 2.0;
        offset.set_if_neq(ScreenIndicatorOffset(Vec2::new(
            projected_edge.x - projected_com.x + CHIP_SHELL_GAP_PX + half_width,
            **lift,
        )));
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
///
/// A ship flown under a [`FlightSpeedCap`] reads `current / rated`: a held
/// burn that levels off is the cap working, and without the second number it
/// reads as a drive that stopped pulling. Ships without the component burn
/// unbounded and have no rating to print.
fn drive_speed_chip(
    q_hud: Query<&FlightStatusHudTargetEntity, With<FlightStatusHudMarker>>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Text, &ChildOf), With<SpeedChipUIMarker>>,
    q_ship: Query<(&LinearVelocity, Option<&FlightSpeedCap>)>,
) {
    for (mut anchor, mut text, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };

        match q_ship.get(**ship) {
            Ok((velocity, cap)) => {
                // Re-assert the anchor so a transient query miss cannot
                // leave the chip dark while its text keeps updating.
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(**ship));
                **text = match cap {
                    Some(cap) => nova_ui::units::speed_rated(
                        MetersPerSecond::from_engine(velocity.length()),
                        MetersPerSecond::from_engine(**cap),
                    ),
                    None => nova_ui::units::speed(MetersPerSecond::from_engine(velocity.length())),
                };
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
    use std::f32::consts::FRAC_PI_2;

    use bevy::{
        camera::{ComputedCameraValues, RenderTargetInfo},
        ecs::system::RunSystemOnce,
    };
    use nova_ship::sections::controller_section::prelude::FlightVerb;

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

    /// The capped ship says what it is capped AT. A held burn levelling off at
    /// 80 is the cap doing its job; with only the left number on the chip it
    /// reads as a drive that quit.
    #[test]
    fn the_speed_chip_reads_current_over_rated_only_where_there_is_a_rating() {
        let mut world = World::new();
        let ship = world
            .spawn((
                LinearVelocity(Vec3::new(3.0, 0.0, 4.0)),
                FlightSpeedCap(8.0),
            ))
            .id();
        let (speed, _) = spawn_status_hud(&mut world, ship);

        world.run_system_once(drive_speed_chip).unwrap();
        assert_eq!(text_of(&world, speed), "50.0 / 80.0 m/s");

        // The scenario lifts the cap: the chip drops back to the bare number
        // rather than printing a rating the ship no longer has.
        world.entity_mut(ship).remove::<FlightSpeedCap>();
        world.run_system_once(drive_speed_chip).unwrap();
        assert_eq!(text_of(&world, speed), "50.0 m/s");
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

    // -- chip placement against a fabricated camera --

    /// Viewport width of the test camera (px).
    const RIG_WIDTH: f32 = 800.0;

    /// Viewport height of the test camera (px).
    const RIG_HEIGHT: f32 = 600.0;

    /// How far down -Z the test ship is parked (world units).
    const RIG_RANGE: f32 = 100.0;

    /// A camera whose computed values are filled in by hand, since no render
    /// backend runs in tests: 90 degree vertical FOV, 800x600, at the origin
    /// looking down -Z.
    fn spawn_test_camera(world: &mut World) {
        world.spawn((
            Transform::IDENTITY,
            Camera {
                computed: ComputedCameraValues {
                    clip_from_view: Mat4::perspective_infinite_reverse_rh(
                        FRAC_PI_2,
                        RIG_WIDTH / RIG_HEIGHT,
                        0.1,
                    ),
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::new(RIG_WIDTH as u32, RIG_HEIGHT as u32),
                        scale_factor: 1.0,
                    }),
                    ..default()
                },
                ..default()
            },
            ScreenIndicatorCamera,
        ));
    }

    /// The on-screen radius (px) of a world radius `radius` at `RIG_RANGE` down
    /// the camera's axis, derived from the rig's own optics rather than from the
    /// projection the system under test uses.
    ///
    /// A 90 degree VERTICAL fov puts the half-height of the view at one range,
    /// so the half-width is `aspect` ranges; a world offset of `radius` is that
    /// fraction of the half-width, in half-viewport pixels.
    fn projected_px(radius: f32) -> f32 {
        let half_width_world = RIG_RANGE * (RIG_WIDTH / RIG_HEIGHT);
        radius / half_width_world * (RIG_WIDTH / 2.0)
    }

    /// A ship at `RIG_RANGE` in front of the camera with `center_of_mass` in its
    /// own frame, a shell envelope of `envelope` world units, and its chips
    /// switched on as their drivers would leave them.
    fn spawn_chip_rig(world: &mut World, center_of_mass: Vec3, envelope: f32) -> (Entity, Entity) {
        spawn_test_camera(world);
        let ship = world
            .spawn((
                Transform::from_xyz(0.0, 0.0, -RIG_RANGE),
                ComputedCenterOfMass(center_of_mass),
            ))
            .id();
        world
            .resource_mut::<HudShellEnvelopes>()
            .insert(ship, envelope);

        let (speed, mode) = spawn_status_hud(world, ship);
        for chip in [speed, mode] {
            **world
                .entity_mut(chip)
                .get_mut::<ScreenIndicatorAnchor>()
                .unwrap() = Some(ScreenIndicatorAnchorKind::Entity(ship));
        }
        (speed, mode)
    }

    fn offset_of(world: &World, entity: Entity) -> Vec2 {
        **world.entity(entity).get::<ScreenIndicatorOffset>().unwrap()
    }

    /// The chips clear the OUTER shell by the authored pixel gap, on their own
    /// authored rows - whatever the hull under it measures. The 20 u hull and
    /// the 200 u hull are the bug: a fixed offset put the second one's chip
    /// inside the ship.
    #[test]
    fn the_chips_park_twelve_pixels_outside_the_projected_gravity_shell() {
        for envelope in [20.0, 200.0] {
            let mut world = World::new();
            world.init_resource::<HudShellEnvelopes>();
            let (speed, mode) = spawn_chip_rig(&mut world, Vec3::ZERO, envelope);

            world.run_system_once(anchor_flight_chips).unwrap();

            // The shell the chip stands outside of is the gravity one: 5 m of
            // hull clearance plus the 6 m between the shells. No UI layout runs
            // in a bare world, so the chips measure zero wide and the offset is
            // the bare gap; the half-width term is what the live range grades.
            let outer = envelope + Meters(11.0).to_engine();
            let expected = projected_px(outer) + 12.0;
            assert!(
                (offset_of(&world, speed).x - expected).abs() < 0.5,
                "{envelope} u hull: speed chip at {} px, expected {expected}",
                offset_of(&world, speed).x
            );
            assert!(
                (offset_of(&world, mode).x - expected).abs() < 0.5,
                "{envelope} u hull: mode chip at {} px, expected {expected}",
                offset_of(&world, mode).x
            );
            // The rows are untouched: this moves the chips out, not up.
            assert_eq!(offset_of(&world, speed).y, -90.0);
            assert_eq!(offset_of(&world, mode).y, -114.0);
        }
    }

    /// The chips hang off the same point the shells are centred on - the live
    /// centre of mass, not the ship's root origin, which on an asymmetric build
    /// is metres away from the middle of the hull.
    #[test]
    fn the_chips_hang_off_the_live_centre_of_mass() {
        let mut world = World::new();
        world.init_resource::<HudShellEnvelopes>();
        let center_of_mass = Vec3::new(4.0, -2.0, 7.0);
        let (speed, _) = spawn_chip_rig(&mut world, center_of_mass, 20.0);

        world.run_system_once(anchor_flight_chips).unwrap();

        assert_eq!(
            anchor_of(&world, speed),
            Some(ScreenIndicatorAnchorKind::Point(
                center_of_mass + Vec3::new(0.0, 0.0, -RIG_RANGE)
            )),
        );
    }

    /// Placement moves the chips that are ON. A chip its driver has cleared -
    /// the mode chip in manual flight, either chip on a dead ship - stays
    /// cleared, so the widget still hides it.
    #[test]
    fn a_chip_its_driver_turned_off_is_left_alone() {
        let mut world = World::new();
        world.init_resource::<HudShellEnvelopes>();
        let (speed, mode) = spawn_chip_rig(&mut world, Vec3::ZERO, 20.0);
        **world
            .entity_mut(mode)
            .get_mut::<ScreenIndicatorAnchor>()
            .unwrap() = None;
        let parked = offset_of(&world, mode);

        world.run_system_once(anchor_flight_chips).unwrap();

        assert_eq!(anchor_of(&world, mode), None);
        assert_eq!(offset_of(&world, mode), parked);
        assert!(
            offset_of(&world, speed).x > parked.x,
            "the chip that is on was still placed"
        );
    }

    /// Before a hull has published an envelope there is no edge to stand
    /// outside of, so the chip keeps the offset it has instead of snapping onto
    /// the hull at a guessed radius.
    #[test]
    fn an_unmeasured_hull_leaves_its_chips_where_they_are() {
        let mut world = World::new();
        world.init_resource::<HudShellEnvelopes>();
        let (speed, _) = spawn_chip_rig(&mut world, Vec3::ZERO, 20.0);
        world.resource_mut::<HudShellEnvelopes>().clear();
        let parked = offset_of(&world, speed);

        world.run_system_once(anchor_flight_chips).unwrap();

        assert_eq!(offset_of(&world, speed), parked);
    }
}
