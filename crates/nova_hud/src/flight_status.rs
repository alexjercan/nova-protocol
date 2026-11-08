//! Diegetic flight readouts: the old bottom-left status text rehomed onto
//! the ship - a speed chip parked beside the velocity sphere, a mode chip
//! (verb + phase) shown only while the autopilot is engaged, `HELM FAULT` on a
//! docked hull whose pair cannot be measured, `STATION HELM` on a hull docked
//! to a living station, `NEUTRAL` on a docked hull that does not drive its
//! pair, a station's refusal line while the travel lock is on a station that
//! refuses a dock, and an RCS magazine chip shown only while RCS is commanded,
//! pushing, or short of full; manual flight keeps a quiet HUD.
//! Plus the projected marker on the GOTO destination.
//!
//! Anything measured here is an ENGINE figure - a world unit is 10 m, a speed
//! is world units per second - because it comes off a bevy transform or an
//! avian `LinearVelocity`. Readouts cross into meters once, through
//! `Meters::from_engine` / `MetersPerSecond::from_engine`, and `nova_ui::units`
//! does the formatting; nothing here converts by hand.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::prelude::HullStripPauseType;
use nova_ship::{
    flight::prelude::*,
    prelude::{
        CameraAuthoritySystems, DockedHelmType, DockedShip, DockingConnection, HullStripJob,
    },
};
use nova_ui::hud::{chip_node, chip_paint, ChipText, ChipTone};

use super::{
    emphasis::prelude::*,
    hull_shell::prelude::*,
    screen_indicator::{fit_box, prelude::*, UiProjection},
    situation::prelude::*,
    target_inset::TargetInsetKeepOut,
    velocity::prelude::*,
    NAV_CYAN,
};

/// The flight-status and autopilot-destination spawners with their configs, markers and
/// `FlightStatusHudPlugin`.
pub mod prelude {
    pub use super::{
        autopilot_destination_hud, flight_status_hud, AutopilotDestinationHudConfig,
        AutopilotDestinationHudMarker, AutopilotDestinationUIMarker, FlightStatusHudConfig,
        FlightStatusHudMarker, FlightStatusHudPlugin, FlightStatusHudTargetEntity,
        ModeChipUIMarker, RcsBudgetChipUIMarker, RcsBudgetPipUIMarker, SpeedChipUIMarker,
        DESTINATION_MARKER_PX,
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

/// The least clear space between the flight chip stack and every edge of the
/// view (px): a shell too wide to stand outside of, or a stack riding low or
/// high on the screen, holds the stack this far inside the view.
const CHIP_VIEW_MARGIN_PX: f32 = 12.0;

/// The least clear space between the flight chip stack and the target inset
/// panel (px). The inset is a keep-out: a chip under it is unreadable.
const CHIP_INSET_GAP_PX: f32 = 12.0;

/// How far the speed chip rides above the ship's centre of mass (px; screen y
/// grows downward). Lifted clear of the bottom-centre keybind dock: the ship
/// sits low-centre under the chase camera, so a chip level with it landed on
/// the dock's chips. This is the demo's `.speed` band (~120 px off the bottom)
/// expressed as a ship-relative offset, so the readout stays parked on the ship
/// rather than becoming screen furniture.
const SPEED_CHIP_LIFT_PX: f32 = -90.0;

/// Row pitch of the chip stack (px). The speed chip is 28 px tall and grows
/// by [`SPEED_CHIP_EMPHASIS`] in a burn; this pitch keeps the 25 px mode chip
/// clear of it at that peak. The stack's box spans half a pitch past the outer
/// rows, which is also the room a burn-grown chip needs.
const CHIP_ROW_PITCH_PX: f32 = 30.0;

/// The mode chip stacks one row above the speed chip.
const MODE_CHIP_LIFT_PX: f32 = SPEED_CHIP_LIFT_PX - CHIP_ROW_PITCH_PX;

/// The RCS magazine chip stacks one row below the speed chip.
const RCS_CHIP_LIFT_PX: f32 = SPEED_CHIP_LIFT_PX + CHIP_ROW_PITCH_PX;

/// Pips on the RCS magazine chip; each is a tenth of the magazine.
const RCS_PIPS: usize = 10;

/// Width, height and gap (px) of one RCS magazine pip.
const RCS_PIP_W: f32 = 4.0;
const RCS_PIP_H: f32 = 10.0;
const RCS_PIP_GAP: f32 = 2.0;

/// Alpha of a spent RCS pip, so the empty track still reads as ten slots.
const RCS_PIP_SPENT_ALPHA: f32 = 0.2;

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

/// Marker for the RCS magazine chip. Public for the same reason as
/// [`SpeedChipUIMarker`].
#[derive(Component, Debug, Clone, Reflect)]
pub struct RcsBudgetChipUIMarker;

/// One pip of the RCS magazine chip, by its index from the left.
#[derive(Component, Debug, Clone, Copy, Deref, Reflect)]
pub struct RcsBudgetPipUIMarker(pub usize);

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
/// chip (anchored to the ship from spawn - it is always on), and the mode and
/// RCS magazine chips (anchors driven at runtime; they spawn hidden exactly
/// like the idle state they start in).
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
                chip_paint(ChipTone::Readout),
                ChipText::value(ChipTone::Readout),
                TextColor(Color::NONE),
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
                ChipText::value(ChipTone::Amber),
                TextColor(Color::NONE),
            ),
            (
                Name::new("RcsBudgetChipUI"),
                RcsBudgetChipUIMarker,
                chip(None, RCS_CHIP_LIFT_PX),
                chip_paint(ChipTone::Readout),
                children![(
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(RCS_PIP_GAP),
                        ..default()
                    },
                    Children::spawn(SpawnIter((0..RCS_PIPS).map(|index| {
                        (
                            RcsBudgetPipUIMarker(index),
                            Node {
                                width: Val::Px(RCS_PIP_W),
                                height: Val::Px(RCS_PIP_H),
                                ..default()
                            },
                            BackgroundColor(rcs_pip_color(true)),
                        )
                    }))),
                )],
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
/// chip, the RCS magazine chip, and the destination marker anchor.
/// Adds `drive_speed_chip`, `emphasize_speed_on_burn`, `drive_mode_chip`,
/// `drive_rcs_budget_chip` and `drive_destination_anchor` in Update within
/// [`super::NovaHudSystems`], plus
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
                drive_rcs_budget_chip,
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
/// So the chips are anchored on the centre of mass and pushed out by the
/// shell's own projected radius, plus [`CHIP_SHELL_GAP_PX`] and half the chip's
/// own width: a projected world radius tracks hull size and camera zoom
/// together, which a pixel constant cannot.
/// The OUTER radius is used even in flat space, where the gravity shell is
/// hidden, so the readouts do not jump sideways when a ship leaves a well.
///
/// The chips are one stack that moves as a unit. Its box spans the three
/// rows - always, shown or not, so it does not hop when the mode chip appears -
/// and [`fit_box`] holds that box inside the view by [`CHIP_VIEW_MARGIN_PX`] on
/// both axes and clear of the target inset by [`CHIP_INSET_GAP_PX`]. Every chip
/// takes the same displacement, so the rows keep their pitch and one left edge.
/// The fit applies only while the centre of mass is on screen; off it the
/// indicator pass hides the chips, and a fit would only chase a dead point.
///
/// Whether a chip is shown at all stays with its driver: this only moves the
/// ones that are already on.
fn anchor_flight_chips(
    envelopes: Res<HudShellEnvelopes>,
    q_hud: Query<&FlightStatusHudTargetEntity, With<FlightStatusHudMarker>>,
    mut q_chip: Query<(
        Entity,
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
    ui_scale: Res<UiScale>,
    inset: TargetInsetKeepOut,
) {
    let Some((camera_entity, camera)) = q_camera.iter().next() else {
        return;
    };
    let Ok(camera_transform) = transform_helper.compute_global_transform(camera_entity) else {
        return;
    };
    let projection = UiProjection::new(camera, &camera_transform, &ui_scale);

    // One chip's unfitted placement. The stack's box needs every chip's width
    // before any chip can be written, hence two passes.
    struct Placed {
        chip: Entity,
        hud: Entity,
        lift: f32,
        half_width: f32,
        /// Projected outer shell radius (UI px) plus the gap: the stack's
        /// left edge relative to the centre of mass.
        stand_off: f32,
        com: Vec2,
        view: Option<Vec2>,
    }
    let mut placed: Vec<Placed> = Vec::new();

    for (chip, mut anchor, _, lift, computed, &ChildOf(hud)) in &mut q_chip {
        if anchor.is_none() {
            continue;
        }
        let Ok(ship) = q_hud.get(hud) else {
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
        let edge = com + projection.right() * radius;
        let (Some(projected_com), Some(projected_edge)) =
            (projection.point(com), projection.point(edge))
        else {
            continue;
        };
        // The widget CENTRES the node on the offset point, so the gap is a
        // gap only once half the chip is added to it. `ComputedNode::size` is
        // physical and is last frame's measurement - the same one the widget
        // centres `Content` chips with - so a chip that changes length is off
        // by half that change for exactly one frame.
        let half_width = computed.size().x * computed.inverse_scale_factor() / 2.0;
        placed.push(Placed {
            chip,
            hud,
            lift: **lift,
            half_width,
            stand_off: projected_edge.x - projected_com.x + CHIP_SHELL_GAP_PX,
            com: projected_com,
            view: projection.view_size(),
        });
    }

    for hud in placed
        .iter()
        .map(|p| p.hud)
        .collect::<std::collections::BTreeSet<_>>()
    {
        let stack: Vec<&Placed> = placed.iter().filter(|p| p.hud == hud).collect();
        let first = stack[0];
        let widest = stack
            .iter()
            .map(|p| p.half_width * 2.0)
            .fold(0.0_f32, f32::max);
        let delta = match first.view {
            Some(view) if first.com.cmpge(Vec2::ZERO).all() && first.com.cmple(view).all() => {
                let left = first.com.x + first.stand_off;
                let stack_box = Rect::new(
                    left,
                    first.com.y + MODE_CHIP_LIFT_PX - CHIP_ROW_PITCH_PX / 2.0,
                    left + widest,
                    first.com.y + RCS_CHIP_LIFT_PX + CHIP_ROW_PITCH_PX / 2.0,
                );
                fit_box(
                    stack_box,
                    view,
                    CHIP_VIEW_MARGIN_PX,
                    inset.rect(view),
                    CHIP_INSET_GAP_PX,
                )
            }
            _ => Vec2::ZERO,
        };
        for p in stack {
            if let Ok((_, _, mut offset, ..)) = q_chip.get_mut(p.chip) {
                offset.set_if_neq(ScreenIndicatorOffset(Vec2::new(
                    p.stand_off + p.half_width + delta.x,
                    p.lift + delta.y,
                )));
            }
        }
    }
}

/// The RCS violet the velocity sphere wears while RCS has the ship, full for
/// a pip with delta-v behind it and dimmed for a spent one.
fn rcs_pip_color(lit: bool) -> Color {
    let violet = VelocityHudPalette::RCS_ACTIVE.indicator;
    if lit {
        violet
    } else {
        violet.with_alpha(RCS_PIP_SPENT_ALPHA)
    }
}

/// The ship's RCS magazine beneath the speed chip: one pip per tenth left,
/// any remainder rounding up so a magazine that is not empty never reads as
/// empty. Shown only while RCS is commanded, pushing, or short of full, so a
/// full idle magazine keeps the manual HUD quiet.
fn drive_rcs_budget_chip(
    q_hud: Query<&FlightStatusHudTargetEntity, With<FlightStatusHudMarker>>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &Children, &ChildOf), With<RcsBudgetChipUIMarker>>,
    q_ship: Query<(&RcsBudget, Option<&RcsIntent>)>,
    q_children: Query<&Children>,
    mut q_pips: Query<(&RcsBudgetPipUIMarker, &mut BackgroundColor)>,
) {
    for (mut anchor, rows, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };
        let Ok((budget, intent)) = q_ship.get(**ship) else {
            **anchor = None;
            continue;
        };
        let commanded = intent.is_some_and(|intent| intent.0 != Vec3::ZERO);
        if !commanded && budget.spent <= 0.0 && budget.applied <= 0.0 {
            **anchor = None;
            continue;
        }
        **anchor = Some(ScreenIndicatorAnchorKind::Entity(**ship));

        let lit = (budget.fraction() * RCS_PIPS as f32).ceil() as usize;
        for pip in rows
            .iter()
            .filter_map(|row| q_children.get(row).ok())
            .flat_map(|pips| pips.iter())
        {
            if let Ok((index, mut color)) = q_pips.get_mut(pip) {
                color.set_if_neq(BackgroundColor(rcs_pip_color(**index < lit)));
            }
        }
    }
}

/// The mode chip's label for an engaged autopilot: the verb and its phase.
fn mode_chip_label(autopilot: &Autopilot) -> String {
    let verb = match autopilot.action {
        AutopilotAction::Stop => "STOP",
        AutopilotAction::Goto { .. } | AutopilotAction::GotoPos { .. } => "GOTO",
        AutopilotAction::Orbit { .. } => "ORBIT",
        AutopilotAction::MatchVelocity { .. } | AutopilotAction::MatchTarget { .. } => "MATCH",
        AutopilotAction::DockApproach { .. } => "DOCK",
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
/// The chip is one number: manual flight is Newtonian, so there is no rating
/// a burn levels off against.
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
                **text = nova_ui::units::speed(MetersPerSecond::from_engine(velocity.length()));
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
/// look. A docked hull that does not drive its pair reads `NEUTRAL`, because
/// its flight verbs are blocked rather than idle. That wins over a maneuver
/// left frozen off the helm, which would otherwise read as live. A pair that
/// cannot be measured reads `HELM FAULT` over both, from
/// [`DockingConnection::measurement_fault`]: neither root drives then, and
/// `drives` alone would show that as an ordinary `NEUTRAL`. A pair a living
/// station holds reads `STATION HELM` ([`DockedHelmType::Station`]), because
/// `NEUTRAL` would promise a helm the player can take. A station's dock
/// refusal is a notice, not a mode (`hud_notice`). A hull
/// stripping a docked wreck reads `STRIP <percent>%` from its
/// [`HullStripJob`], or `STRIP HELD: HOLD FULL` while a full hold holds it,
/// so the strip shows in flight with the interface shut.
fn drive_mode_chip(
    q_hud: Query<&FlightStatusHudTargetEntity, With<FlightStatusHudMarker>>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Text, &ChildOf), With<ModeChipUIMarker>>,
    q_ship: Query<(
        Option<&Autopilot>,
        Option<&DockedShip>,
        Option<&HullStripJob>,
    )>,
    q_connections: Query<&DockingConnection>,
) {
    for (mut anchor, mut text, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };

        let connection = |docked: &DockedShip| q_connections.get(docked.connection).ok();
        let faulted = |docked: &DockedShip| connection(docked).is_some_and(|c| c.measurement_fault);
        let station_held = |docked: &DockedShip| {
            connection(docked).is_some_and(|c| matches!(c.helm, DockedHelmType::Station(_)))
        };
        match q_ship.get(**ship) {
            Ok((_, Some(docked), ..)) if faulted(docked) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(**ship));
                **text = "HELM FAULT".to_string();
            }
            Ok((.., Some(job))) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(**ship));
                **text = match job.paused {
                    Some(HullStripPauseType::HoldFull { .. }) => {
                        "STRIP HELD: HOLD FULL".to_string()
                    }
                    None => format!("STRIP {:.0}%", job.progress() * 100.0),
                };
            }
            Ok((_, Some(docked), ..)) if station_held(docked) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(**ship));
                **text = "STATION HELM".to_string();
            }
            Ok((_, Some(docked), ..)) if !docked.drives => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(**ship));
                **text = "NEUTRAL".to_string();
            }
            Ok((Some(autopilot), ..)) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(**ship));
                **text = mode_chip_label(autopilot);
            }
            Ok(_) | Err(_) => {
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
            // None has a destination to anchor on: a brake, a velocity to
            // hold and a velocity to match. A dock approach has one, but only
            // an AI helm order flies it, and the player ship takes no helm
            // order.
            AutopilotAction::Stop
            | AutopilotAction::MatchVelocity { .. }
            | AutopilotAction::MatchTarget { .. }
            | AutopilotAction::DockApproach { .. } => None,
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

    use super::*;
    use crate::prelude::ManeuverChip;

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

    /// The RCS chip reads the magazine a tenth per pip, rounding a remainder
    /// up, and stays hidden while the magazine is full and idle.
    #[test]
    fn the_rcs_chip_lights_one_pip_per_tenth_of_the_magazine_left() {
        let mut world = World::new();
        let capacity = RcsBudget::default().capacity;
        let ship = world
            .spawn((LinearVelocity(Vec3::ZERO), RcsBudget::default()))
            .id();
        let layer = world
            .spawn(flight_status_hud(FlightStatusHudConfig { target: ship }))
            .id();
        let chip = world.entity(layer).get::<Children>().unwrap()[2];
        let lit = |world: &mut World| {
            let mut pips = world.query::<(&RcsBudgetPipUIMarker, &BackgroundColor)>();
            let colors: Vec<_> = pips.iter(world).map(|(_, color)| color.0).collect();
            assert_eq!(colors.len(), RCS_PIPS, "the chip carries ten pips");
            colors
                .iter()
                .filter(|&&color| color == rcs_pip_color(true))
                .count()
        };

        world.run_system_once(drive_rcs_budget_chip).unwrap();
        assert_eq!(anchor_of(&world, chip), None, "full and idle: hidden");

        // 65% left rounds up to seven pips.
        world.get_mut::<RcsBudget>(ship).unwrap().spent = 0.35 * capacity;
        world.run_system_once(drive_rcs_budget_chip).unwrap();
        assert_eq!(
            anchor_of(&world, chip),
            Some(ScreenIndicatorAnchorKind::Entity(ship))
        );
        assert_eq!(lit(&mut world), 7);

        // A crumb left still shows one pip; an empty magazine shows none.
        world.get_mut::<RcsBudget>(ship).unwrap().spent = 0.99 * capacity;
        world.run_system_once(drive_rcs_budget_chip).unwrap();
        assert_eq!(lit(&mut world), 1);
        world.get_mut::<RcsBudget>(ship).unwrap().spent = capacity;
        world.run_system_once(drive_rcs_budget_chip).unwrap();
        assert_eq!(lit(&mut world), 0);

        // Full again but commanded: shown with every pip lit.
        world
            .entity_mut(ship)
            .insert((RcsBudget::default(), RcsIntent(Vec3::X)));
        world.run_system_once(drive_rcs_budget_chip).unwrap();
        assert!(anchor_of(&world, chip).is_some(), "a held command shows it");
        assert_eq!(lit(&mut world), RCS_PIPS);
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

    /// The chip follows the connection's fault first and `drives` second,
    /// never the helm: a pair that cannot be measured flies nothing, so it
    /// must not read as a live burn or as an ordinary hand-off.
    #[test]
    fn a_frozen_maneuver_reads_helm_fault_then_neutral_until_its_hull_drives() {
        let mut world = World::new();
        let ship = world.spawn(LinearVelocity(Vec3::ZERO)).id();
        let connection = world
            .spawn(DockingConnection {
                first_ship: ship,
                first_section: Entity::PLACEHOLDER,
                second_ship: Entity::PLACEHOLDER,
                second_section: Entity::PLACEHOLDER,
                helm: DockedHelmType::Held(ship),
                measurement_fault: true,
            })
            .id();
        let mut goto = Autopilot::engage(AutopilotAction::GotoPos {
            position: Vec3::ZERO,
        });
        goto.phase = AutopilotPhase::Burn;
        world.entity_mut(ship).insert((
            goto,
            DockedShip {
                connection,
                helm: Quat::IDENTITY,
                drives: false,
            },
        ));
        let (_, mode) = spawn_status_hud(&mut world, ship);

        world.run_system_once(drive_mode_chip).unwrap();
        assert_eq!(
            text_of(&world, mode),
            "HELM FAULT",
            "an unmeasured pair is a fault, not a hand-off"
        );

        world
            .get_mut::<DockingConnection>(connection)
            .unwrap()
            .measurement_fault = false;
        world.run_system_once(drive_mode_chip).unwrap();
        assert_eq!(
            text_of(&world, mode),
            "NEUTRAL",
            "no live BURN off the helm"
        );

        world.get_mut::<DockedShip>(ship).unwrap().drives = true;
        world.run_system_once(drive_mode_chip).unwrap();
        assert_eq!(
            text_of(&world, mode),
            "AP GOTO - BURN",
            "driving resumes it"
        );
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

        world.resource_mut::<HudSituations>().maneuver = Some(ManeuverChip::Goto);
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
                    clip_from_view: bevy::math::proj::perspective_infinite_reverse(
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
        world.init_resource::<UiScale>();
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
    /// the 100 u hull are the bug: a fixed offset put the second one's chip
    /// inside the ship.
    #[test]
    fn the_chips_park_twelve_pixels_outside_the_projected_gravity_shell() {
        for envelope in [20.0, 100.0] {
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
            assert_eq!(offset_of(&world, mode).y, -120.0);
        }
    }

    /// A shell wider than the view cannot have a chip outside it: the chip
    /// stays inside the window, where the speed can still be read. The view
    /// is the window in UI px, so a 150% interface has two thirds as many.
    #[test]
    fn a_shell_wider_than_the_view_keeps_the_chips_inside_the_window() {
        for ui_scale in [1.0, 1.5] {
            let mut world = World::new();
            world.init_resource::<HudShellEnvelopes>();
            let (speed, mode) = spawn_chip_rig(&mut world, Vec3::ZERO, 400.0);
            world.resource_mut::<UiScale>().0 = ui_scale;
            assert!(
                projected_px(400.0) > RIG_WIDTH / 2.0,
                "delivery guard: the shell leaves the view"
            );

            world.run_system_once(anchor_flight_chips).unwrap();

            // The ship is at the view centre and the chips measure zero wide
            // here.
            let expected = RIG_WIDTH / ui_scale / 2.0 - CHIP_VIEW_MARGIN_PX;
            for chip in [speed, mode] {
                assert!(
                    (offset_of(&world, chip).x - expected).abs() < 0.5,
                    "{ui_scale}x: chip at {} px, expected {expected}",
                    offset_of(&world, chip).x
                );
            }
        }
    }

    /// One placed stack: the chips' boxes in UI px (speed, mode, RCS), their
    /// rows, the view, and the inset rect while the panel is shown.
    struct PlacedStack {
        boxes: Vec<(Rect, f32)>,
        offsets: Vec<Vec2>,
        view: Vec2,
        com: Vec2,
        panel: Rect,
    }

    /// Place the three chips of a ship whose centre of mass projects to
    /// `com_window` (window px) on a `window`-sized view at `ui_scale`, with
    /// the target inset panel shown or hidden at its corner.
    fn place_stack(
        window: UVec2,
        ui_scale: f32,
        com_window: Vec2,
        envelope: f32,
        inset_shown: bool,
    ) -> PlacedStack {
        use crate::target_inset::target_inset_hud;

        let mut world = World::new();
        world.init_resource::<HudShellEnvelopes>();
        world.insert_resource(UiScale(ui_scale));
        world.spawn((
            Transform::IDENTITY,
            Camera {
                computed: ComputedCameraValues {
                    clip_from_view: bevy::math::proj::perspective_infinite_reverse(
                        FRAC_PI_2,
                        window.x as f32 / window.y as f32,
                        0.1,
                    ),
                    target_info: Some(RenderTargetInfo {
                        physical_size: window,
                        scale_factor: 1.0,
                    }),
                    ..default()
                },
                ..default()
            },
            ScreenIndicatorCamera,
        ));
        // A 90 degree vertical fov puts the half-height of the view at one
        // range, so a window offset from the centre is that fraction of it.
        let half = window.as_vec2() / 2.0;
        let offset = (com_window - half) / half.y * RIG_RANGE;
        let ship = world
            .spawn(Transform::from_xyz(offset.x, -offset.y, -RIG_RANGE))
            .id();
        world
            .resource_mut::<HudShellEnvelopes>()
            .insert(ship, envelope);

        let layer = world
            .spawn(flight_status_hud(FlightStatusHudConfig { target: ship }))
            .id();
        let chips: Vec<Entity> = world.entity(layer).get::<Children>().unwrap()[..3].to_vec();
        // Speed, mode, RCS: the widths and heights a live layout gives them.
        let sizes = [
            Vec2::new(83.0, 28.0),
            Vec2::new(120.0, 25.0),
            Vec2::new(70.0, 22.0),
        ];
        for (&chip, size) in chips.iter().zip(sizes) {
            world.entity_mut(chip).insert(ComputedNode {
                size: size * ui_scale,
                inverse_scale_factor: 1.0 / ui_scale,
                ..default()
            });
            **world
                .entity_mut(chip)
                .get_mut::<ScreenIndicatorAnchor>()
                .unwrap() = Some(ScreenIndicatorAnchorKind::Entity(ship));
        }
        let inset = world
            .spawn(target_inset_hud(Handle::<Image>::default()))
            .id();
        world.entity_mut(inset).insert(if inset_shown {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });

        world.run_system_once(anchor_flight_chips).unwrap();

        let view = window.as_vec2() / ui_scale;
        let com = com_window / ui_scale;
        let offsets: Vec<Vec2> = chips.iter().map(|&chip| offset_of(&world, chip)).collect();
        let boxes = chips
            .iter()
            .zip(sizes)
            .zip(&offsets)
            .map(|((&chip, size), offset)| {
                let lift = **world.entity(chip).get::<FlightChipLift>().unwrap();
                (Rect::from_center_size(com + offset, size), lift)
            })
            .collect();
        // The panel is a corner node: 256 px square, 12 px off the top-right.
        let panel = Rect::new(view.x - 12.0 - 256.0, 12.0, view.x - 12.0, 12.0 + 256.0);
        PlacedStack {
            boxes,
            offsets,
            view,
            com,
            panel,
        }
    }

    /// The reproduced failure: at 1280x720 with a 150 % interface the stack of
    /// a ship with a shell wider than the view was held on the view's right
    /// edge, under the inset. The stack must clear the inset as ONE unit, and
    /// the scales that worked keep working untouched.
    #[test]
    fn the_flight_chip_stack_clears_the_target_inset_as_one_unit() {
        let overlaps = |a: Rect, b: Rect| {
            a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
        };

        // 1280x720 at 150 %: the centre of mass is UI (426.7, 369).
        let com_window = Vec2::new(640.0, 553.5);
        let shown = place_stack(UVec2::new(1280, 720), 1.5, com_window, 400.0, true);
        let hidden = place_stack(UVec2::new(1280, 720), 1.5, com_window, 400.0, false);
        let held_in_view = hidden.boxes.iter().fold(
            Rect::new(f32::MAX, f32::MAX, f32::MIN, f32::MIN),
            |all, (rect, _)| all.union(*rect),
        );
        assert!(
            overlaps(held_in_view, shown.panel),
            "delivery guard: held at the view edge the stack sits under the inset"
        );
        let room = Rect::new(
            CHIP_VIEW_MARGIN_PX,
            CHIP_VIEW_MARGIN_PX,
            shown.view.x - CHIP_VIEW_MARGIN_PX,
            shown.view.y - CHIP_VIEW_MARGIN_PX,
        );
        for (rect, lift) in &shown.boxes {
            assert!(
                !overlaps(*rect, shown.panel.inflate(CHIP_INSET_GAP_PX)),
                "the chip on row {lift} at {rect:?} is under the inset {:?}",
                shown.panel
            );
            assert!(
                rect.min.cmpge(room.min - 0.01).all() && rect.max.cmple(room.max + 0.01).all(),
                "the chip on row {lift} at {rect:?} leaves the view {room:?}"
            );
        }
        // One rigid move: the unfitted stack shares one left edge and the row
        // pitch, so the fitted one must too.
        let left = shown.boxes[0].0.min.x;
        let first_centre = shown.boxes[0].0.center().y;
        for ((rect, lift), offset) in shown.boxes.iter().zip(&shown.offsets) {
            assert!(
                (rect.min.x - left).abs() < 0.01,
                "the chip on row {lift} starts at x = {}, the stack at {left}",
                rect.min.x
            );
            assert!(
                (rect.center().y - first_centre - (lift - shown.boxes[0].1)).abs() < 0.01,
                "the row pitch broke at {offset:?}"
            );
        }

        // The scales that work today: nothing is near the inset or an edge, so
        // the chips stand exactly where the shell puts them.
        for (ui_scale, com_window) in [
            (0.75, Vec2::new(960.0, 831.0)),
            (1.0, Vec2::new(960.0, 831.0)),
        ] {
            let window = UVec2::new(1920, 1080);
            let placed = place_stack(window, ui_scale, com_window, 20.0, true);
            // The shell the chips stand outside of: 5 m of clearance plus 6 m
            // between the shells, in world units, projected at the rig range.
            let outer = 20.0 + Meters(11.0).to_engine();
            let shell_px = outer * (window.y as f32 / 2.0) / RIG_RANGE / ui_scale;
            for ((rect, lift), offset) in placed.boxes.iter().zip(&placed.offsets) {
                let half_width = rect.width() / 2.0;
                let expected = Vec2::new(shell_px + CHIP_SHELL_GAP_PX + half_width, *lift);
                assert!(
                    (*offset - expected).abs().max_element() < 0.5,
                    "{ui_scale}x: the chip on row {lift} is at {offset:?}, expected {expected:?}"
                );
            }
            assert!(placed.com.x > 0.0, "{ui_scale}x: the rig is on screen");
        }
    }

    /// The speed chip is the tallest chip and grows while the autopilot burns.
    /// The mode row above it and the magazine row under it must clear it at
    /// its peak scale.
    #[test]
    fn the_chip_rows_clear_the_speed_chip_at_its_peak_emphasis() {
        use crate::chip_layout_rig::{chip_layout_app, measure, only_descendant_with, settle};

        let mut app = chip_layout_app();
        let ship = app.world_mut().spawn_empty().id();
        let hud = app
            .world_mut()
            .spawn(flight_status_hud(FlightStatusHudConfig { target: ship }))
            .id();
        let speed = only_descendant_with::<SpeedChipUIMarker>(&mut app, hud);
        let mode = only_descendant_with::<ModeChipUIMarker>(&mut app, hud);
        let rcs = only_descendant_with::<RcsBudgetChipUIMarker>(&mut app, hud);
        app.world_mut()
            .entity_mut(speed)
            .insert(Text::new("11.7 m/s"));
        app.world_mut()
            .entity_mut(mode)
            .insert(Text::new("AP MATCH - BURN"));
        settle(&mut app);

        let half_height = |chip: Entity| measure(&app, chip).size.y / 2.0;
        assert!(
            half_height(mode) > 0.0 && half_height(rcs) > 0.0,
            "delivery guard: the chips were laid out"
        );
        let speed_half = half_height(speed) * SPEED_CHIP_EMPHASIS;
        let mode_bottom = MODE_CHIP_LIFT_PX + half_height(mode);
        let speed_top = SPEED_CHIP_LIFT_PX - speed_half;
        assert!(
            mode_bottom <= speed_top,
            "the mode chip ends at {mode_bottom} px, the speed chip starts at {speed_top} px"
        );
        let rcs_top = RCS_CHIP_LIFT_PX - half_height(rcs);
        let speed_bottom = SPEED_CHIP_LIFT_PX + speed_half;
        assert!(
            rcs_top >= speed_bottom,
            "the magazine chip starts at {rcs_top} px, the speed chip ends at {speed_bottom} px"
        );
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
