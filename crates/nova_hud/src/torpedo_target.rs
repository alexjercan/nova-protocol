//! The torpedo-lock reticle and the locked-target info readout: a
//! screen-projected indicator on the entity the player's aim-assist currently
//! locks (the ship-root `CombatLock`), with distance, closing speed
//! and a health bar riding its edge.
//!
//! A thin consumer of the [`screen_indicator`](mod@super::screen_indicator)
//! widget: the widget owns projection, sizing and visibility; this module
//! spawns the reticle, drives its anchor from the lock resource, and fills
//! the readout content. The readout is a child of the reticle node, so
//! visibility inheritance hides it with the reticle. Its slot is chosen by
//! `fit_lock_readout` after the indicator pass: beside the reticle, on
//! whichever side keeps it inside the view and clear of the target inset.
//!
//! The lock's range and closing speed arrive as ENGINE figures (world units,
//! one of which is 10 m, off a transform and an avian `LinearVelocity`). They
//! cross into meters once, through `Meters::from_engine` /
//! `MetersPerSecond::from_engine`, and `nova_ui::units` formats them.

use avian3d::prelude::*;
use bevy::{prelude::*, ui::UiSystems};
use nova_events::units::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    hud::{self as chip, chip_paint, ChipTone},
    theme::combat,
};

use crate::{prelude::*, screen_indicator::fit_box, target_inset::TargetInsetKeepOut};

/// Minimum on-screen size (px) of the target reticle. This is its historical
/// fixed size: the reticle grows to match larger targets but never shrinks
/// below this, so small or distant targets still show a clearly visible,
/// clickable marker.
const MIN_RETICLE_PX: f32 = 32.0;

/// Font size (px) of the readout lines, matching the flight-status readout.
const READOUT_FONT_PX: f32 = 14.0;

/// Gap (px) between the reticle edge and the readout's near edge while the
/// readout holds [`LOCK_READOUT_EMPHASIS`]. The hold grows the readout about
/// its own centre, so a gap measured at rest let the hot readout touch the
/// reticle bracket.
const READOUT_GAP_PX: f32 = 8.0;

/// The least clear space between the readout and every edge of the view (px).
const READOUT_VIEW_MARGIN_PX: f32 = 12.0;

/// The least clear space between the readout and the target inset panel (px).
const READOUT_INSET_GAP_PX: f32 = 12.0;

/// Peak scale of the lock readout while the weapons are hot - demo 2's
/// `.lock-read.emph`.
const LOCK_READOUT_EMPHASIS: f32 = 1.12;

/// Peak scale and full cycle of the reticle's firing pulse - demo 2's
/// `retpulse 0.28s ... alternate`, which is a 0.56 s there-and-back.
const RETICLE_FIRING_PULSE: f32 = 1.12;
const RETICLE_PULSE_PERIOD_SECS: f32 = 0.56;

/// The two COMPOSE: the readout is a child of the reticle node, so with the
/// safety off AND the trigger down it renders at
/// `LOCK_READOUT_EMPHASIS * RETICLE_FIRING_PULSE` = 1.2544 at the pulse peak,
/// not at 1.12. That is deliberate - the lock reads as one instrument reacting,
/// rather than a readout sitting still on a breathing reticle - and it is the
/// number to quote if the playtest calls the motion too busy (the fix would be
/// dropping the readout's own hold while firing, not restructuring the tree).
/// Pinned by `the_composed_peak_while_firing_is_the_product`, which is its only
/// consumer - hence `cfg(test)`: it documents a value the RENDERER derives from
/// the two constants above, it does not feed it.
#[cfg(test)]
const LOCK_COMPOSED_FIRING_PEAK: f32 = LOCK_READOUT_EMPHASIS * RETICLE_FIRING_PULSE;

/// Health bar size (px): a small underline below the text lines.
const HEALTH_BAR_SIZE: Vec2 = Vec2::new(64.0, 6.0);

/// Health bar backdrop (the "missing health" part).
const HEALTH_BAR_BACKDROP: Color = nova_ui::theme::semantic::BACKDROP;

/// Focus meter size (px): a thin underline below the reticle that fills
/// while the focus dwell accumulates (the component-lock arc).
const FOCUS_METER_SIZE: Vec2 = Vec2::new(48.0, 4.0);

/// Focus meter backdrop.
const FOCUS_METER_BACKDROP: Color = nova_ui::theme::semantic::BACKDROP;

/// Focus meter fill: hot-metal red, matching the component markers it
/// unlocks.
const FOCUS_METER_COLOR: Color = combat::at(combat::LOCK, 0.9);

/// The combat reticle is ALWAYS combat-red: the on-object lock language is
/// purely slot-colored - red bracket = combat lock, white bracket = travel
/// lock. This retires the relation tint and the reticle's four armed corner
/// pips: a visible combat reticle already IMPLIES weapons-hot (lock => hot,
/// the safety truth table).
const RETICLE_COMBAT_COLOR: Color = nova_ui::theme::semantic::THREAT;

/// The `torpedo_target_hud` spawner, its config, the reticle, health-bar, focus-meter and readout
/// markers, and `TorpedoTargetHudPlugin`.
pub mod prelude {
    pub use super::{
        torpedo_target_hud, TorpedoTargetFocusFillMarker, TorpedoTargetFocusMeterMarker,
        TorpedoTargetHealthBarMarker, TorpedoTargetHealthFillMarker, TorpedoTargetHudConfig,
        TorpedoTargetHudMarker, TorpedoTargetHudPlugin, TorpedoTargetReadoutLine,
        TorpedoTargetReadoutMarker, TorpedoTargetReticleMarker,
    };
}

/// Marker for the full-screen reticle layer (the root the HUD setup spawns).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetHudMarker;

/// Marker for the reticle indicator node itself. Public so other HUD pieces
/// (e.g. the locked-target readout) can attach content to it.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetReticleMarker;

/// Marker for the readout column beside the reticle.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetReadoutMarker;

/// Which readout line a `Text` node shows. One enum component instead of one
/// marker type per line, so a single query updates all lines without filter
/// gymnastics.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum TorpedoTargetReadoutLine {
    /// `DST <range>` - range to the locked target (m below 1 km, km above).
    Distance,
    /// `CLS <closing>` - signed closing speed in m/s, positive when
    /// approaching.
    ClosingSpeed,
    /// `TTI <seconds> s` - time until the player's soonest torpedo reaches the
    /// locked hull. Hidden while [`PlayerTorpedoEta`] holds no estimate.
    TimeToImpact,
}

/// Marker for the health bar backdrop node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetHealthBarMarker;

/// Marker for the health bar fill node (width = health fraction).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetHealthFillMarker;

/// Marker for the focus meter backdrop below the reticle.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetFocusMeterMarker;

/// Marker for the focus meter fill node (width = focus fraction).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetFocusFillMarker;

/// Spawn-time settings for a [`torpedo_target_hud`] reticle: the target sprite
/// the combat reticle draws. Not a component - consumed by
/// [`torpedo_target_hud`].
#[derive(Clone, Debug, Default)]
pub struct TorpedoTargetHudConfig {
    /// The reticle sprite the combat lock draws.
    pub target_sprite: Handle<Image>,
}

/// UI bundle for the torpedo-lock reticle: a full-screen click-through layer
/// whose child is a screen-projected indicator sized to the locked target's
/// on-screen extent, carrying the info readout on its right edge.
pub fn torpedo_target_hud(config: TorpedoTargetHudConfig) -> impl Bundle {
    trace!("torpedo_target_hud: config {:?}", config);

    (
        Name::new("TorpedoTargetHUD"),
        TorpedoTargetHudMarker,
        screen_indicator_layer(),
        children![(
            Name::new("TorpedoTargetReticle"),
            TorpedoTargetReticleMarker,
            screen_indicator(ScreenIndicatorConfig {
                anchor: None,
                size: ScreenIndicatorSize::ApparentSize {
                    min_px: MIN_RETICLE_PX,
                    // The travel crosshair rides the same anchors at 1.35;
                    // the combat reticle tracks true apparent size so the
                    // pair stays concentric.
                    scale: 1.0,
                },
                offset: Vec2::ZERO,
                offscreen: ScreenIndicatorOffscreen::Hide,
            }),
            ImageNode::new(config.target_sprite).with_color(RETICLE_COMBAT_COLOR),
            // Pulses while the trigger is down (demo 2's `retpulse`). The
            // readout below is a CHILD of this node, so it breathes along with
            // the reticle while firing - one instrument reacting, which is why
            // its own emphasis is the smaller, steadier hold.
            HudEmphasis::pulse(RETICLE_FIRING_PULSE, RETICLE_PULSE_PERIOD_SECS),
            children![
                (
                    Name::new("TorpedoTargetFocusMeter"),
                    TorpedoTargetFocusMeterMarker,
                    Node {
                        position_type: PositionType::Absolute,
                        // Centered under the reticle, tracking its scaled edge
                        // via UI layout like the readout does.
                        top: Val::Percent(100.0),
                        left: Val::Percent(50.0),
                        margin: UiRect {
                            left: Val::Px(-FOCUS_METER_SIZE.x / 2.0),
                            top: Val::Px(4.0),
                            ..default()
                        },
                        width: Val::Px(FOCUS_METER_SIZE.x),
                        height: Val::Px(FOCUS_METER_SIZE.y),
                        ..default()
                    },
                    BackgroundColor(FOCUS_METER_BACKDROP),
                    Pickable::IGNORE,
                    Visibility::Hidden,
                    children![(
                        Name::new("TorpedoTargetFocusFill"),
                        TorpedoTargetFocusFillMarker,
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(FOCUS_METER_COLOR),
                        Pickable::IGNORE,
                    )],
                ),
                (
                    Name::new("TorpedoTargetReadout"),
                    TorpedoTargetReadoutMarker,
                    Node {
                        position_type: PositionType::Absolute,
                        // Beside the reticle, relative to its top-left corner.
                        // `fit_lock_readout` writes both every frame the
                        // reticle is shown.
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::FlexStart,
                        row_gap: Val::Px(2.0),
                        padding: UiRect::axes(Val::Px(9.0), Val::Px(4.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(chip::CHIP_RADIUS)),
                        ..default()
                    },
                    // The lock readout is the THREAT member of the chip family
                    // (demo 2 `.lock-read`): a red-bordered slab so the numbers
                    // stay legible over a lit hull and never read as nav data.
                    chip_paint(ChipTone::Threat),
                    // Grows while the weapons are hot: with the safety off,
                    // range and closing speed are the numbers you are shooting
                    // by (demo 2's `.lock-read.emph`).
                    HudEmphasis::settle(LOCK_READOUT_EMPHASIS),
                    Pickable::IGNORE,
                    children![
                        (
                            Name::new("TorpedoTargetReadoutDistance"),
                            TorpedoTargetReadoutLine::Distance,
                            Text::new(""),
                            TextFont::from_font_size(READOUT_FONT_PX),
                            // The chip hugs its text, so a wrapping line would
                            // fold "DST 1.50 km" into two ragged rows.
                            TextLayout {
                                linebreak: LineBreak::NoWrap,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ),
                        (
                            Name::new("TorpedoTargetReadoutClosing"),
                            TorpedoTargetReadoutLine::ClosingSpeed,
                            Text::new(""),
                            TextFont::from_font_size(READOUT_FONT_PX),
                            TextLayout {
                                linebreak: LineBreak::NoWrap,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ),
                        (
                            Name::new("TorpedoTargetReadoutTimeToImpact"),
                            TorpedoTargetReadoutLine::TimeToImpact,
                            Text::new(""),
                            TextFont::from_font_size(READOUT_FONT_PX),
                            TextLayout {
                                linebreak: LineBreak::NoWrap,
                                ..default()
                            },
                            Node {
                                display: Display::None,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ),
                        (
                            Name::new("TorpedoTargetHealthBar"),
                            TorpedoTargetHealthBarMarker,
                            Node {
                                width: Val::Px(HEALTH_BAR_SIZE.x),
                                height: Val::Px(HEALTH_BAR_SIZE.y),
                                ..default()
                            },
                            BackgroundColor(HEALTH_BAR_BACKDROP),
                            Pickable::IGNORE,
                            children![(
                                Name::new("TorpedoTargetHealthFill"),
                                TorpedoTargetHealthFillMarker,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(health_color(1.0)),
                                Pickable::IGNORE,
                            )],
                        ),
                    ],
                )
            ],
        )],
    )
}

/// Drives the combat-lock reticle: anchors it to the current [`CombatLock`]
/// target and updates its distance/closing-speed readout, health bar and focus
/// meter.
/// Adds `drive_reticle_anchor`, `update_target_readout`, `update_focus_meter`
/// and `emphasize_lock_on_weapons_hot` in Update within
/// [`super::NovaHudSystems`], and `fit_lock_readout` in PostUpdate after
/// `ScreenIndicatorSystems` and before UI layout.
#[derive(Default)]
pub struct TorpedoTargetHudPlugin;

impl Plugin for TorpedoTargetHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                drive_reticle_anchor,
                update_target_readout,
                update_focus_meter,
                emphasize_lock_on_weapons_hot,
            )
                .in_set(super::NovaHudSystems),
        );
        // The slot needs the reticle box the indicator pass just wrote, and
        // must land before layout reads the readout's `left`/`top`.
        app.add_systems(
            PostUpdate,
            fit_lock_readout
                .after(ScreenIndicatorSystems)
                .before(UiSystems::Layout),
        );
    }
}

/// The gap (px) a `width` readout stands off the reticle at rest, so that its
/// near edge is [`READOUT_GAP_PX`] off at the hold emphasis.
fn readout_rest_gap(width: f32) -> f32 {
    READOUT_GAP_PX + (LOCK_READOUT_EMPHASIS - 1.0) * width / 2.0
}

/// The readout's `left`/`top` (px, from the reticle's top-left corner) that
/// keeps it inside the view and clear of `keep_out`.
///
/// Slots in order: right of the reticle, then left of it. The first whose
/// swept box needs no move wins; with neither free the right slot takes its
/// displacement, so the view still holds the readout whole.
///
/// The swept box is the rest box united with the box at the composed peak. The
/// readout is a child of the reticle, and a node's `UiTransform` scale acts
/// about its own centre while its parent's scales the child's offset from the
/// PARENT's centre. So at the peak the readout's centre sits at
/// `reticle_centre + RETICLE_FIRING_PULSE * (rest_centre - reticle_centre)`
/// and its size is scaled by both. Every in-between box lies in the union, so
/// a hot, firing lock clears the inset at its largest.
///
/// A non-zero displacement moves the rest box one-to-one but the peak box by
/// the reticle's pulse scale; the extra 12 % of the move lands away from the
/// keep-out, and into the view margin by at most 12 % of the move.
fn lock_readout_origin(reticle: Rect, size: Vec2, view: Vec2, keep_out: Option<Rect>) -> Vec2 {
    let centre = reticle.center();
    let displacement = |min: Vec2| {
        let rest = Rect::from_corners(min, min + size);
        let peak = Rect::from_center_size(
            centre + (rest.center() - centre) * RETICLE_FIRING_PULSE,
            size * (RETICLE_FIRING_PULSE * LOCK_READOUT_EMPHASIS),
        );
        fit_box(
            rest.union(peak),
            view,
            READOUT_VIEW_MARGIN_PX,
            keep_out,
            READOUT_INSET_GAP_PX,
        )
    };
    let gap = readout_rest_gap(size.x);
    let right = Vec2::new(reticle.max.x + gap, reticle.min.y);
    let left = Vec2::new(reticle.min.x - gap - size.x, reticle.min.y);
    let origin = if displacement(right) == Vec2::ZERO {
        right
    } else if displacement(left) == Vec2::ZERO {
        left
    } else {
        right + displacement(right)
    };
    origin - reticle.min
}

/// Place the lock readout beside the reticle, on the side that stays in view
/// and clear of the target inset.
///
/// Reads the reticle box the indicator pass wrote this frame (px) and the
/// readout's last measured size, so a readout that changes length is off by
/// that change for one frame. Writes only on change.
fn fit_lock_readout(
    inset: TargetInsetKeepOut,
    q_camera: Query<&Camera, With<ScreenIndicatorCamera>>,
    ui_scale: Res<UiScale>,
    q_reticle: Query<
        (&Node, &Visibility),
        (
            With<TorpedoTargetReticleMarker>,
            Without<TorpedoTargetReadoutMarker>,
        ),
    >,
    mut q_readout: Query<
        (&mut Node, &ComputedNode, &ChildOf),
        (
            With<TorpedoTargetReadoutMarker>,
            Without<TargetInsetHudMarker>,
        ),
    >,
) {
    let Some(view) = q_camera
        .iter()
        .next()
        .and_then(|camera| camera.logical_viewport_size())
        .map(|size| size / ui_scale.0)
    else {
        return;
    };
    let keep_out = inset.rect(view);
    for (mut node, computed, &ChildOf(parent)) in &mut q_readout {
        let Ok((reticle, visibility)) = q_reticle.get(parent) else {
            continue;
        };
        if *visibility == Visibility::Hidden {
            continue;
        }
        let (Val::Px(left), Val::Px(top), Val::Px(width), Val::Px(height)) =
            (reticle.left, reticle.top, reticle.width, reticle.height)
        else {
            continue;
        };
        let reticle = Rect::new(left, top, left + width, top + height);
        let size = computed.size() * computed.inverse_scale_factor();
        let origin = lock_readout_origin(reticle, size, view, keep_out);
        let (want_left, want_top) = (Val::Px(origin.x), Val::Px(origin.y));
        if node.left != want_left {
            node.left = want_left;
        }
        if node.top != want_top {
            node.top = want_top;
        }
    }
}

/// The combat emphasis: the lock readout grows while the
/// weapons are hot, and the reticle pulses while the trigger is actually down.
/// Two nodes, one situation source, so they cannot disagree about what "in
/// combat" means.
fn emphasize_lock_on_weapons_hot(
    situations: Res<HudSituations>,
    mut q_readout: Query<&mut HudEmphasis, With<TorpedoTargetReadoutMarker>>,
    mut q_reticle: Query<
        &mut HudEmphasis,
        (
            With<TorpedoTargetReticleMarker>,
            Without<TorpedoTargetReadoutMarker>,
        ),
    >,
) {
    for mut emphasis in &mut q_readout {
        emphasis.set_held(situations.weapons_hot);
    }
    for mut emphasis in &mut q_reticle {
        emphasis.set_held(situations.firing);
    }
}

/// Point the reticle indicator at the current lock; `None` (no lock) hides it
/// via the widget's anchor handling.
fn drive_reticle_anchor(
    q_player: Query<&CombatLock, With<PlayerSpaceshipMarker>>,
    mut q_reticle: Query<&mut ScreenIndicatorAnchor, With<TorpedoTargetReticleMarker>>,
) {
    let lock = q_player.iter().next().and_then(|lock| lock.0);
    for mut anchor in &mut q_reticle {
        **anchor = lock.map(ScreenIndicatorAnchorKind::Entity);
    }
}

/// Closing speed (positive when approaching) of `target` relative to `ship`
/// along the line of sight, or `None` when the two positions coincide (no
/// line of sight to project onto).
pub(crate) fn closing_speed(
    ship_pos: Vec3,
    ship_vel: Vec3,
    target_pos: Vec3,
    target_vel: Vec3,
) -> Option<f32> {
    let los_dir = (target_pos - ship_pos).try_normalize()?;
    Some(-(target_vel - ship_vel).dot(los_dir))
}

/// The `DST` line. The distance arrives in world units off a Bevy transform
/// and crosses here, once; the shared policy formats the meters it is handed
/// (meters below 1 km, kilometers above) and converts nothing.
pub(crate) fn distance_line(distance: f32) -> String {
    format!(
        "DST {}",
        nova_ui::units::distance(Meters::from_engine(distance))
    )
}

/// The `CLS` line: the same crossing, then the shared closing-speed policy,
/// which prints an explicit sign - positive closing, negative opening. `None`
/// (no velocity data on either body) renders a placeholder.
pub(crate) fn closing_line(closing: Option<f32>) -> String {
    match closing {
        Some(closing) => format!(
            "CLS {}",
            nova_ui::units::closing_speed(MetersPerSecond::from_engine(closing))
        ),
        None => "CLS   ---".to_string(),
    }
}

/// The `TTI` line: seconds to one decimal, from [`PlayerTorpedoEta`].
pub(crate) fn tti_line(seconds: f32) -> String {
    format!("TTI {seconds:.1} s")
}

/// Health as a fraction in [0, 1]; a non-positive `max` reads as empty.
fn health_fraction(health: &Health) -> f32 {
    if health.max <= 0.0 {
        return 0.0;
    }
    (health.current / health.max).clamp(0.0, 1.0)
}

/// Fill color for a health fraction: green at full, through amber, to red
/// near death.
fn health_color(fraction: f32) -> Color {
    let fraction = fraction.clamp(0.0, 1.0);
    Color::srgba(1.0 - fraction * 0.8, 0.2 + fraction * 0.7, 0.15, 0.9)
}

/// Fill the readout from the locked target: distance and closing speed from
/// the transforms and `LinearVelocity`s, health bar from the target root's
/// `Health`, time to impact from [`PlayerTorpedoEta`]. Degrades gracefully:
/// missing velocity on either body blanks the closing line, a target without
/// `Health` hides the bar, and no estimate takes the time-to-impact line out of
/// the column rather than showing a number nothing measured. With no lock the
/// readout is not updated at all - it is a child of the reticle indicator,
/// which the widget already hides.
#[expect(clippy::type_complexity, reason = "one query per readout part")]
fn update_target_readout(
    ship: Single<
        (&GlobalTransform, Option<&LinearVelocity>, &CombatLock),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_target: Query<(&GlobalTransform, Option<&LinearVelocity>, Option<&Health>)>,
    eta: Res<PlayerTorpedoEta>,
    mut q_lines: Query<
        (&mut Text, &mut Node, &TorpedoTargetReadoutLine),
        Without<TorpedoTargetHealthFillMarker>,
    >,
    mut q_bar: Query<&mut Visibility, With<TorpedoTargetHealthBarMarker>>,
    mut q_fill: Query<(&mut Node, &mut BackgroundColor), With<TorpedoTargetHealthFillMarker>>,
) {
    let (ship_transform, ship_vel, lock) = ship.into_inner();
    let Some(target) = lock.0 else {
        return;
    };
    let Ok((target_transform, target_vel, target_health)) = q_target.get(target) else {
        // The lock can outlive its entity by a frame; the reticle (and the
        // readout with it) is already hidden by the widget.
        return;
    };

    let ship_pos = ship_transform.translation();
    let target_pos = target_transform.translation();
    let distance = ship_pos.distance(target_pos);
    let closing = match (ship_vel, target_vel) {
        (Some(ship_vel), Some(target_vel)) => {
            closing_speed(ship_pos, **ship_vel, target_pos, **target_vel)
        }
        _ => None,
    };

    for (mut text, mut node, line) in &mut q_lines {
        let content = match line {
            TorpedoTargetReadoutLine::Distance => Some(distance_line(distance)),
            TorpedoTargetReadoutLine::ClosingSpeed => Some(closing_line(closing)),
            TorpedoTargetReadoutLine::TimeToImpact => eta.0.map(|eta| tti_line(eta.seconds)),
        };
        let display = if content.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        if let Some(content) = content {
            if **text != content {
                **text = content;
            }
        }
    }

    for mut visibility in &mut q_bar {
        visibility.set_if_neq(match target_health {
            Some(_) => Visibility::Inherited,
            None => Visibility::Hidden,
        });
    }
    if let Some(health) = target_health {
        let fraction = health_fraction(health);
        for (mut node, mut color) in &mut q_fill {
            let width = Val::Percent(fraction * 100.0);
            if node.width != width {
                node.width = width;
            }
            let fill_color = health_color(fraction);
            if color.0 != fill_color {
                color.0 = fill_color;
            }
        }
    }
}

/// Drive the focus meter: visible with a partial fill while a lock is held
/// and the dwell is still accumulating, gone before a lock exists and once
/// focus completes (the component markers appearing is the completion
/// signal).
fn update_focus_meter(
    q_player: Query<(&CombatLock, &LockFocus), With<PlayerSpaceshipMarker>>,
    mut q_meter: Query<&mut Visibility, With<TorpedoTargetFocusMeterMarker>>,
    mut q_fill: Query<&mut Node, With<TorpedoTargetFocusFillMarker>>,
) {
    let fraction = q_player.iter().next().and_then(|(lock, focus)| {
        matches!(lock.0, Some(target) if focus.target == Some(target) && !focus.focused_on(target))
            .then(|| focus.fraction())
    });
    let filling = fraction.is_some();

    for mut visibility in &mut q_meter {
        visibility.set_if_neq(if filling {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    if let Some(fraction) = fraction {
        let width = Val::Percent(fraction * 100.0);
        for mut node in &mut q_fill {
            if node.width != width {
                node.width = width;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn torpedo_target_hud_spawns_the_reticle_indicator() {
        let mut world = World::new();
        let layer = world
            .spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()))
            .id();

        let children = world
            .entity(layer)
            .get::<Children>()
            .expect("layer has the reticle child");
        assert_eq!(children.len(), 1);
        let reticle = world.entity(children[0]);
        assert!(reticle.contains::<TorpedoTargetReticleMarker>());
        assert!(reticle.contains::<ScreenIndicatorMarker>());
        assert_eq!(
            **reticle.get::<ScreenIndicatorAnchor>().unwrap(),
            None,
            "the reticle starts unanchored (hidden) until a lock exists"
        );
    }

    #[test]
    fn readout_rides_the_reticle_node() {
        // The readout must be a child of the reticle indicator (not the
        // layer): that is what makes it track the scaled edge and inherit
        // the reticle's visibility.
        let mut world = World::new();
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));

        let ChildOf(parent) = world
            .query_filtered::<&ChildOf, With<TorpedoTargetReadoutMarker>>()
            .iter(&world)
            .next()
            .expect("readout spawned");
        assert!(world
            .entity(*parent)
            .contains::<TorpedoTargetReticleMarker>());

        let lines = world
            .query::<&TorpedoTargetReadoutLine>()
            .iter(&world)
            .count();
        assert_eq!(lines, 3, "distance, closing-speed and time-to-impact lines");
    }

    #[test]
    fn reticle_anchor_follows_the_combat_lock() {
        let mut world = World::new();
        let player = world.spawn((PlayerSpaceshipMarker, CombatLock(None))).id();
        let reticle = world
            .spawn((
                TorpedoTargetReticleMarker,
                screen_indicator(ScreenIndicatorConfig::default()),
            ))
            .id();

        world.run_system_once(drive_reticle_anchor).unwrap();
        assert_eq!(
            **world
                .entity(reticle)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            None
        );

        let target = world.spawn_empty().id();
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(target);
        world.run_system_once(drive_reticle_anchor).unwrap();
        assert_eq!(
            **world
                .entity(reticle)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            Some(ScreenIndicatorAnchorKind::Entity(target))
        );

        world.get_mut::<CombatLock>(player).unwrap().0 = None;
        world.run_system_once(drive_reticle_anchor).unwrap();
        assert_eq!(
            **world
                .entity(reticle)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            None,
            "dropping the lock clears the anchor so the widget hides the reticle"
        );
    }

    // -- readout math and formatting --

    #[test]
    fn closing_speed_sign_convention() {
        let ship = Vec3::ZERO;
        let target = Vec3::new(0.0, 0.0, -100.0);
        // Target flying toward the ship (+z): closing, positive.
        assert_eq!(
            closing_speed(ship, Vec3::ZERO, target, Vec3::new(0.0, 0.0, 50.0)),
            Some(50.0)
        );
        // Target flying away (-z): opening, negative.
        assert_eq!(
            closing_speed(ship, Vec3::ZERO, target, Vec3::new(0.0, 0.0, -50.0)),
            Some(-50.0)
        );
        // Pure crossing motion: no closing component.
        assert_eq!(
            closing_speed(ship, Vec3::ZERO, target, Vec3::new(50.0, 0.0, 0.0)),
            Some(0.0)
        );
        // The ship chasing the target closes too.
        assert_eq!(
            closing_speed(ship, Vec3::new(0.0, 0.0, -30.0), target, Vec3::ZERO),
            Some(30.0)
        );
        // Coincident positions: no line of sight.
        assert_eq!(closing_speed(ship, Vec3::ZERO, ship, Vec3::ZERO), None);
    }

    #[test]
    fn readout_lines_use_the_shared_unit_policy() {
        // 1 u = 10 m: 150.4 u = 1504 m -> km; 1234.6 u = 12346 m -> km.
        assert_eq!(distance_line(150.4), "DST 1.50 km");
        assert_eq!(distance_line(1234.6), "DST 12.35 km");
        assert_eq!(closing_line(Some(12.34)), "CLS +123.4 m/s");
        assert_eq!(closing_line(Some(-3.21)), "CLS -32.1 m/s");
        assert_eq!(closing_line(None), "CLS   ---");
        assert_eq!(tti_line(4.21), "TTI 4.2 s");
        assert_eq!(tti_line(93.0), "TTI 93.0 s");
    }

    #[test]
    fn health_fraction_clamps() {
        let health = |current, max| Health { current, max };
        assert_eq!(health_fraction(&health(50.0, 100.0)), 0.5);
        assert_eq!(health_fraction(&health(150.0, 100.0)), 1.0);
        assert_eq!(health_fraction(&health(-5.0, 100.0)), 0.0);
        assert_eq!(health_fraction(&health(5.0, 0.0)), 0.0);
    }

    // -- readout slot --

    /// The reproduced failure at 1280x720 with a 150 % interface: a view of
    /// 853.33 x 480 UI px, the inset in its corner, the reticle left of it, and
    /// the readout's right slot running under the inset's left edge.
    #[test]
    fn the_lock_readout_takes_the_free_side_of_the_reticle_when_the_inset_holds_its_slot() {
        use bevy::camera::{ComputedCameraValues, RenderTargetInfo};

        let mut world = World::new();
        world.insert_resource(UiScale(1.5));
        world.spawn((
            Camera {
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::new(1280, 720),
                        scale_factor: 1.0,
                    }),
                    ..default()
                },
                ..default()
            },
            ScreenIndicatorCamera,
        ));
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));
        let inset = world
            .spawn(target_inset_hud(Handle::<Image>::default()))
            .id();
        let reticle_entity = world
            .query_filtered::<Entity, With<TorpedoTargetReticleMarker>>()
            .single(&world)
            .unwrap();
        let readout_entity = world
            .query_filtered::<Entity, With<TorpedoTargetReadoutMarker>>()
            .single(&world)
            .unwrap();

        let view = Vec2::new(1280.0, 720.0) / 1.5;
        let reticle = Rect::new(314.7, 113.3, 314.7 + 147.3, 113.3 + 146.7);
        {
            let mut node = world.get_mut::<Node>(reticle_entity).unwrap();
            node.left = Val::Px(reticle.min.x);
            node.top = Val::Px(reticle.min.y);
            node.width = Val::Px(reticle.width());
            node.height = Val::Px(reticle.height());
        }
        world.entity_mut(reticle_entity).insert(Visibility::Visible);
        let readout_size = Vec2::new(122.0, 65.0);
        let gap = readout_rest_gap(readout_size.x);
        // What the hold emphasis adds to each side of the readout.
        let hold_growth = (LOCK_READOUT_EMPHASIS - 1.0) * readout_size.x / 2.0;
        world.entity_mut(readout_entity).insert(ComputedNode {
            size: readout_size * 1.5,
            inverse_scale_factor: 1.0 / 1.5,
            ..default()
        });

        let readout_rest = |world: &World| {
            let node = world.get::<Node>(readout_entity).unwrap();
            let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
                panic!(
                    "the readout is not placed in px: {:?} {:?}",
                    node.left, node.top
                );
            };
            Rect::from_corners(
                reticle.min + Vec2::new(left, top),
                reticle.min + Vec2::new(left, top) + readout_size,
            )
        };
        let overlaps = |a: Rect, b: Rect| {
            a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
        };

        // Shown: the panel is the inset's real rect.
        world.entity_mut(inset).insert(Visibility::Visible);
        let panel = Rect::new(view.x - 12.0 - 256.0, 12.0, view.x - 12.0, 12.0 + 256.0);
        assert!(
            overlaps(
                Rect::from_corners(
                    Vec2::new(reticle.max.x + gap, reticle.min.y),
                    Vec2::new(reticle.max.x + gap, reticle.min.y) + readout_size,
                ),
                panel
            ),
            "delivery guard: the right slot is under the inset"
        );
        world.run_system_once(fit_lock_readout).unwrap();
        let rest = readout_rest(&world);
        // The same box at the composed firing peak: the reticle scales the
        // readout's offset from its own centre, the readout scales itself.
        let peak = Rect::from_center_size(
            reticle.center() + (rest.center() - reticle.center()) * RETICLE_FIRING_PULSE,
            readout_size * (RETICLE_FIRING_PULSE * LOCK_READOUT_EMPHASIS),
        );
        let room = Rect::new(12.0, 12.0, view.x - 12.0, view.y - 12.0);
        for (what, shape) in [("rest", rest), ("peak", peak)] {
            assert!(
                !overlaps(shape, panel.inflate(12.0)),
                "the {what} readout {shape:?} is under the inset {panel:?}"
            );
            assert!(
                shape.min.cmpge(room.min - 0.01).all() && shape.max.cmple(room.max + 0.01).all(),
                "the {what} readout {shape:?} leaves the view {room:?}"
            );
        }
        assert!(
            rest.max.x <= reticle.min.x,
            "the readout moved to the left of the reticle"
        );
        assert!(
            (reticle.min.x - (rest.max.x + hold_growth) - READOUT_GAP_PX).abs() < 0.01,
            "the held readout {rest:?} is not {READOUT_GAP_PX} px off the reticle {reticle:?}"
        );

        // Hidden: nothing holds the right slot, so the readout keeps it.
        world.entity_mut(inset).insert(Visibility::Hidden);
        world.run_system_once(fit_lock_readout).unwrap();
        let node = world.get::<Node>(readout_entity).unwrap();
        // Its near edge is the gap off the reticle once the hold has grown it.
        assert_eq!(
            node.left,
            Val::Px(reticle.width() + READOUT_GAP_PX + hold_growth)
        );
        assert_eq!(node.top, Val::Px(0.0));
    }

    // -- readout system behavior --

    fn spawn_readout_world(world: &mut World) -> Entity {
        world.init_resource::<PlayerTorpedoEta>();
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));
        world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                GlobalTransform::IDENTITY,
                LinearVelocity(Vec3::ZERO),
                CombatLock(None),
            ))
            .id()
    }

    fn line_text(world: &mut World, which: TorpedoTargetReadoutLine) -> String {
        world
            .query::<(&Text, &TorpedoTargetReadoutLine)>()
            .iter(world)
            .find(|(_, line)| **line == which)
            .map(|(text, _)| text.0.clone())
            .expect("line exists")
    }

    #[test]
    fn readout_fills_from_the_locked_target() {
        let mut world = World::new();
        let player = spawn_readout_world(&mut world);
        // 150 world units dead ahead (1.50 km displayed), flying toward the
        // ship at 20 u/s (200.0 m/s closing), half health.
        let target = world
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -150.0)),
                LinearVelocity(Vec3::new(0.0, 0.0, 20.0)),
                Health {
                    current: 50.0,
                    max: 100.0,
                },
            ))
            .id();
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(target);

        world.run_system_once(update_target_readout).unwrap();

        assert_eq!(
            line_text(&mut world, TorpedoTargetReadoutLine::Distance),
            "DST 1.50 km"
        );
        assert_eq!(
            line_text(&mut world, TorpedoTargetReadoutLine::ClosingSpeed),
            "CLS +200.0 m/s"
        );
        let bar_visibility = *world
            .query_filtered::<&Visibility, With<TorpedoTargetHealthBarMarker>>()
            .iter(&world)
            .next()
            .expect("bar exists");
        assert_eq!(bar_visibility, Visibility::Inherited);
        let fill = world
            .query_filtered::<&Node, With<TorpedoTargetHealthFillMarker>>()
            .iter(&world)
            .next()
            .expect("fill exists");
        assert_eq!(fill.width, Val::Percent(50.0));
    }

    #[test]
    fn readout_degrades_without_velocity_or_health() {
        let mut world = World::new();
        let player = spawn_readout_world(&mut world);
        // A bare drifting body: transform only, no velocity, no health.
        // 80 world units = 800 m, still below the 1 km threshold.
        let target = world
            .spawn(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -80.0,
            )))
            .id();
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(target);

        world.run_system_once(update_target_readout).unwrap();

        assert_eq!(
            line_text(&mut world, TorpedoTargetReadoutLine::Distance),
            "DST 800 m"
        );
        assert_eq!(
            line_text(&mut world, TorpedoTargetReadoutLine::ClosingSpeed),
            "CLS   ---"
        );
        let bar_visibility = *world
            .query_filtered::<&Visibility, With<TorpedoTargetHealthBarMarker>>()
            .iter(&world)
            .next()
            .expect("bar exists");
        assert_eq!(bar_visibility, Visibility::Hidden);
    }

    #[test]
    fn the_time_to_impact_line_shows_only_while_a_torpedo_eta_exists() {
        fn line_display(world: &mut World) -> Display {
            world
                .query::<(&Node, &TorpedoTargetReadoutLine)>()
                .iter(world)
                .find(|(_, line)| **line == TorpedoTargetReadoutLine::TimeToImpact)
                .map(|(node, _)| node.display)
                .expect("line exists")
        }

        let mut world = World::new();
        let player = spawn_readout_world(&mut world);
        let target = world.spawn(GlobalTransform::IDENTITY).id();
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(target);

        world.run_system_once(update_target_readout).unwrap();
        assert_eq!(line_display(&mut world), Display::None);

        let torpedo = world.spawn_empty().id();
        world.resource_mut::<PlayerTorpedoEta>().0 = Some(TorpedoEta {
            torpedo,
            seconds: 4.2,
        });
        world.run_system_once(update_target_readout).unwrap();
        assert_eq!(line_display(&mut world), Display::Flex);
        assert_eq!(
            line_text(&mut world, TorpedoTargetReadoutLine::TimeToImpact),
            "TTI 4.2 s"
        );

        world.resource_mut::<PlayerTorpedoEta>().0 = None;
        world.run_system_once(update_target_readout).unwrap();
        assert_eq!(line_display(&mut world), Display::None);
    }

    // -- focus meter --

    fn meter_state(world: &mut World) -> (Visibility, Val) {
        let visibility = *world
            .query_filtered::<&Visibility, With<TorpedoTargetFocusMeterMarker>>()
            .iter(world)
            .next()
            .expect("meter exists");
        let width = world
            .query_filtered::<&Node, With<TorpedoTargetFocusFillMarker>>()
            .iter(world)
            .next()
            .expect("fill exists")
            .width;
        (visibility, width)
    }

    #[test]
    fn focus_meter_fills_while_the_dwell_accumulates() {
        let mut world = World::new();
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));
        let target = world.spawn_empty().id();
        let player = world
            .spawn((
                PlayerSpaceshipMarker,
                CombatLock(Some(target)),
                LockFocus::default(),
            ))
            .id();
        // Halfway through the dwell: drive through the public API by finding
        // the seconds that yield fraction 0.5.
        {
            let mut focus = world.get_mut::<LockFocus>(player).unwrap();
            focus.target = Some(target);
            let mut lo = 0.0_f32;
            let mut hi = 60.0_f32;
            for _ in 0..40 {
                let mid = (lo + hi) / 2.0;
                focus.seconds = mid;
                if focus.fraction() < 0.5 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
        }

        world.run_system_once(update_focus_meter).unwrap();

        let (visibility, width) = meter_state(&mut world);
        assert_eq!(visibility, Visibility::Inherited);
        match width {
            Val::Percent(percent) => {
                assert!((percent - 50.0).abs() < 1.0, "width {percent}")
            }
            other => panic!("expected Val::Percent, got {other:?}"),
        }
    }

    #[test]
    fn focus_meter_hides_without_a_lock_and_once_focused() {
        let mut world = World::new();
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));
        let player = world
            .spawn((
                PlayerSpaceshipMarker,
                CombatLock(None),
                LockFocus::default(),
            ))
            .id();

        world.run_system_once(update_focus_meter).unwrap();
        assert_eq!(meter_state(&mut world).0, Visibility::Hidden);

        // Focused: the meter yields to the component markers.
        let target = world.spawn_empty().id();
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(target);
        *world.get_mut::<LockFocus>(player).unwrap() = LockFocus {
            target: Some(target),
            seconds: f32::MAX,
        };
        world.run_system_once(update_focus_meter).unwrap();
        assert_eq!(meter_state(&mut world).0, Visibility::Hidden);
    }

    // -- reticle slot color --

    #[test]
    fn the_reticle_is_always_combat_red() {
        // The on-object lock language is slot-colored: red bracket = combat
        // lock, white bracket = travel lock. No relation tint - the red is
        // baked into the bundle; this pins the contract so a re-added tint
        // system shows up as a failing diff here. Nothing moves the reticle
        // colour per frame any more: the lock has no clock to wind down.
        let mut world = World::new();
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));
        let color = world
            .query_filtered::<&ImageNode, With<TorpedoTargetReticleMarker>>()
            .iter(&world)
            .next()
            .expect("reticle exists")
            .color;
        assert_eq!(color, RETICLE_COMBAT_COLOR);
    }

    /// The combat emphasis: the readout grows with the
    /// SAFETY (weapons hot) and the reticle pulses with the TRIGGER (firing).
    /// Pinned as two different situations so the pair cannot be collapsed into
    /// one flag by a later edit.
    #[test]
    fn the_readout_follows_the_safety_and_the_reticle_follows_the_trigger() {
        let mut world = World::new();
        world.init_resource::<HudSituations>();
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));

        let held = |world: &mut World, reticle: bool| {
            let mut q = world.query::<(&HudEmphasis, Has<TorpedoTargetReticleMarker>)>();
            q.iter(world)
                .find(|(_, is_reticle)| *is_reticle == reticle)
                .expect("the emphasis node exists")
                .0
                .held()
        };

        world
            .run_system_once(emphasize_lock_on_weapons_hot)
            .unwrap();
        assert!(!held(&mut world, false), "safe: the readout is at rest");
        assert!(!held(&mut world, true), "safe: the reticle is at rest");

        world.resource_mut::<HudSituations>().weapons_hot = true;
        world
            .run_system_once(emphasize_lock_on_weapons_hot)
            .unwrap();
        assert!(held(&mut world, false), "hot: the readout grows");
        assert!(
            !held(&mut world, true),
            "hot but not shooting: the reticle is still steady"
        );

        world.resource_mut::<HudSituations>().firing = true;
        world
            .run_system_once(emphasize_lock_on_weapons_hot)
            .unwrap();
        assert!(held(&mut world, true), "trigger down: the reticle pulses");

        world.resource_mut::<HudSituations>().firing = false;
        world.resource_mut::<HudSituations>().weapons_hot = false;
        world
            .run_system_once(emphasize_lock_on_weapons_hot)
            .unwrap();
        assert!(!held(&mut world, false), "safety back on: readout settles");
        assert!(!held(&mut world, true), "trigger released: pulse stops");
    }

    /// The readout rides INSIDE the reticle, so the two emphases MULTIPLY
    /// rather than the readout showing its own 1.12 while firing. Pinned so the
    /// number a playtest complaint would be about is written down, and so a
    /// later re-parent that changes it shows up here.
    #[test]
    fn the_composed_peak_while_firing_is_the_product() {
        // The doc comment on LOCK_COMPOSED_FIRING_PEAK quotes 1.2544 as the
        // number to take to a playtest; pin it to the constants so retuning
        // either emphasis cannot leave that prose silently stale.
        assert!(
            (LOCK_COMPOSED_FIRING_PEAK - 1.2544).abs() < 1e-6,
            "the documented composed peak is {LOCK_COMPOSED_FIRING_PEAK}, not 1.2544"
        );

        let mut world = World::new();
        let layer = world
            .spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()))
            .id();
        let reticle = world.entity(layer).get::<Children>().unwrap()[0];
        let readout = world
            .entity(reticle)
            .get::<Children>()
            .unwrap()
            .iter()
            .find(|&child| {
                world
                    .entity(child)
                    .get::<TorpedoTargetReadoutMarker>()
                    .is_some()
            })
            .expect("the readout is a child of the reticle");

        let peak =
            |world: &World, entity: Entity| world.entity(entity).get::<HudEmphasis>().unwrap().peak;
        let composed = peak(&world, readout) * peak(&world, reticle);
        assert!(
            (composed - LOCK_COMPOSED_FIRING_PEAK).abs() < 1e-6,
            "hot AND firing renders the readout at {composed}, not at its own \
             {LOCK_READOUT_EMPHASIS}"
        );
    }
}
