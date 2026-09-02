//! Diegetic per-weapon ammo readouts: a small chunked gauge drawn ON each
//! player weapon that still carries a finite [`SectionAmmo`], so the player
//! can see a turret or torpedo bay running dry without reading a corner
//! panel.
//!
//! A thin consumer of the [`screen_indicator`](mod@super::screen_indicator)
//! widget with `Entity` anchors: a reconcile system keeps one readout per
//! player weapon section that has a `SectionAmmo`, anchored to that section so
//! the gauge rides on the weapon in screen space, and a driver reads
//! `rounds/capacity` each frame and lights the matching chunks. The gauge is
//! deliberately quantized, not a number:
//!
//! - a turret shows a ring of [`RING_SEGMENTS`] pips that drains from a full
//!   `o` toward an empty `c` as rounds deplete (at least one pip stays lit
//!   while any round remains, so "nearly empty" never reads as "empty");
//! - a torpedo bay shows a `||||` row of one pip per round of capacity, the
//!   remaining rounds lit;
//! - a railgun shows the same row in Pierce blue, which for the shipped lance
//!   is a single pip: loaded or spent.
//!
//! While a weapon reloads, only the next authored batch pulses above the live
//! rounds. Its alpha rises with delay progress, then the pips become solid when
//! the batch lands. A spent PDC previews its incoming ring segments; a torpedo
//! bay previews exactly one returning torpedo, and a spent lance previews the
//! one shell coming back across its long reload.
//!
//! A BAR pip also FILLS from its floor with the wait ([`AmmoReadoutPipFill`]).
//! Brightness can say a shell is coming; it cannot say how much of twelve
//! seconds is left, and on a one-shell lance that wait is most of what the
//! weapon feels like. A ring segment gets no column: it previews a batch of two
//! hundred rounds rather than a moment worth counting down.
//!
//! A weapon with no `SectionAmmo` fires without limit (the `infinite_ammo`
//! path forces `ammo_capacity = None`, so the component is simply absent):
//! the reconcile filter skips it and it gets no readout at all, which is the
//! intended "don't even show it" behavior for infinite ammo.
//!
//! The exact count is a debug-only overlay, never a gameplay affordance: the
//! `rounds/capacity` `Text` child, its resource and its toggle only compile
//! under the `debug` cargo feature (`--features debug`), so a release build has
//! no numeric readout at all. Under that feature the number tracks debug mode:
//! it is shown while debug mode is on (which nova_debug starts on) and hidden
//! once debug mode is switched off, F11 (the shared debug toggle) flipping both
//! together via `AmmoReadoutDebug` (only present under the `debug` feature).
//!
//! Like the other combat overlays the layer is `HudTier::Instrument` and is
//! spawned/despawned with the player ship by the hud/mod.rs observers.
//!
//! CONTEXTUAL: the gauges are not on whenever a weapon
//! has ammo - the layer carries a [`HudContextGate`] driven by
//! `sync_ammo_gate`, so they surface while weapons are hot, a group is nearly
//! dry, or a batch is reloading. Reload keeps its gauge visible through the
//! quiet interval; a full magazine lets it leave idle cruise again.

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::prelude::*;

/// Number of chunks in a turret's ring gauge. Fixed (not the magazine size):
/// turret magazines are large, so the ring conveys a coarse fraction, not an
/// exact count - the exact count is the debug number.
pub const RING_SEGMENTS: usize = 8;

/// On-screen size (px) of a turret ring gauge. Small: it is a status mark on
/// the weapon, not a reticle.
const RING_PX: f32 = 28.0;
/// Diameter (px) of one ring pip.
const RING_PIP_PX: f32 = 6.0;

/// Width, height and gap (px) of one torpedo bar pip.
const BAR_PIP_W: f32 = 3.0;
const BAR_PIP_H: f32 = 12.0;
const BAR_PIP_GAP: f32 = 2.0;

/// Key that toggles the debug ammo number. F11 mirrors the nova_debug toggle
/// (`DebugEnabled`); nova_gameplay cannot depend on nova_debug (that crate
/// depends on this one), so the readout owns its own F11-driven flag, kept in
/// sync by watching the same key. Only exists under the `debug` feature.
#[cfg(feature = "debug")]
const DEBUG_TOGGLE_KEY: KeyCode = KeyCode::F11;

/// A spent chunk's initial color at spawn: the Kinetic amber, dimmed.
/// `drive_ammo_readouts` overwrites this each frame in the loaded round's hue;
/// this is just the neutral pre-drive fill (the ring exists a frame before the
/// driver runs). The lit/dim HUES now come from [`damage_type_color`]; the
/// alphas are `LIT_ALPHA`/`DIM_ALPHA` on the driver.
const DIM_COLOR: Color = Color::srgba(1.0, 0.75, 0.2, 0.16);

/// A thin dark outline around every pip so the amber gauge holds contrast on
/// light or same-hue backgrounds (grey hull, orange nebula) - the way a
/// dark-edged cursor stays visible on any desktop. Applied to lit and dim pips
/// alike so the whole track reads regardless of what is behind it.
const PIP_OUTLINE_PX: f32 = 1.0;
const PIP_OUTLINE_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.85);

/// The `ammo_readout_hud` spawner, the readout components, `RING_SEGMENTS` and `AmmoReadoutPlugin`;
/// the numeric debug overlay names arrive with the `debug` feature.
pub mod prelude {
    pub use super::{
        ammo_readout_hud, AmmoReadoutHudMarker, AmmoReadoutKind, AmmoReadoutMarker, AmmoReadoutPip,
        AmmoReadoutPipFill, AmmoReadoutPlugin, AmmoReadoutSection, RING_SEGMENTS,
    };
    #[cfg(feature = "debug")]
    pub use super::{AmmoReadoutDebug, AmmoReadoutNumber};
}

/// Marker for the full-screen readout layer (the root the HUD setup spawns).
#[derive(Component, Debug, Clone, Reflect)]
pub struct AmmoReadoutHudMarker;

/// Marker for one weapon's readout node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct AmmoReadoutMarker;

/// The weapon section entity this readout renders the ammo of.
#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Reflect)]
pub struct AmmoReadoutSection(pub Entity);

/// Which gauge shape a readout draws, and thus how a fraction maps to lit
/// chunks.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum AmmoReadoutKind {
    /// A ring of [`RING_SEGMENTS`] pips lit by the coarse fill fraction.
    Turret,
    /// A `||||` row of one pip per round of capacity, `rounds` of them lit.
    Torpedo,
    /// The same bar, in the slug's Pierce blue. A separate kind rather than a
    /// one-pip torpedo because the hue is the point: a spinal magazine of one
    /// says almost nothing by its length, and the colour is what tells you
    /// which of your two loaded weapons just went dark.
    Railgun,
}

/// A single chunk of a gauge, carrying its position in the lit order.
#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Reflect)]
pub struct AmmoReadoutPip(pub usize);

/// The reload COLUMN inside one bar pip: a child node whose height is the
/// fraction of the current wait that has run.
///
/// A bar pip is the entire gauge on a one-shell lance, so the pulse's
/// brightness was carrying twelve seconds on its own - and brightness cannot
/// say "four seconds left". A column can, and it reads at the same glance the
/// pip does.
///
/// Only bar pips carry one. A ring segment is a coarse fraction of a large
/// magazine, where the thing that is coming back is a batch of two hundred
/// rounds rather than a moment worth waiting for.
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct AmmoReadoutPipFill;

/// The debug `rounds/capacity` text child of a readout. Debug-only: only
/// compiled under the `debug` feature.
#[cfg(feature = "debug")]
#[derive(Component, Debug, Clone, Reflect)]
pub struct AmmoReadoutNumber;

/// Whether the debug ammo number is shown (toggled with F11). Off by default so
/// it starts in phase with nova_debug's `DebugEnabled(false)`: the number then
/// tracks debug mode (hidden while off, shown once F11 switches debug on)
/// instead of inverting it. The gauge itself is always on. Debug-only: only
/// compiled under the `debug` feature, so release builds have no numeric
/// readout at all.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone, Copy, Deref, DerefMut, PartialEq, Eq, Reflect)]
#[reflect(Resource)]
pub struct AmmoReadoutDebug(pub bool);

/// Starts off to match nova_debug's `DebugEnabled(false)` default: the whole
/// debug layer boots off and F11 raises it as one.
/// Both toggle on F11, so matching defaults keeps the ammo number in phase with
/// debug mode; a mismatch here is what inverts the number relative to the rest
/// of the debug layer.
#[cfg(feature = "debug")]
impl Default for AmmoReadoutDebug {
    fn default() -> Self {
        Self(false)
    }
}

/// UI bundle for the readout layer. Readouts are spawned under it by
/// `sync_ammo_readouts`, one per player weapon section with ammo.
pub fn ammo_readout_hud() -> impl Bundle {
    (
        Name::new("AmmoReadoutHUD"),
        AmmoReadoutHudMarker,
        // Contextual: shut until the weapons are hot or a group runs low. The
        // readouts underneath are projected indicators, so the gate has to be
        // enforced downstream of the projection - which is exactly where
        // `apply_hud_visibility` reads it from this layer.
        HudContextGate(false),
        screen_indicator_layer(),
    )
}

/// Open the ammo layer's gate while the gauges are relevant. Separate from
/// `drive_ammo_readouts` (which paints pips) because this decides whether the
/// gauges are on screen AT ALL, and it must keep answering while they are off.
fn sync_ammo_gate(
    situations: Res<HudSituations>,
    mut q_layer: Query<&mut HudContextGate, With<AmmoReadoutHudMarker>>,
) {
    for mut gate in &mut q_layer {
        gate.set_if_neq(HudContextGate(situations.ammo_relevant()));
    }
}

/// How many of a turret ring's [`RING_SEGMENTS`] pips are lit for the given
/// magazine. Empty lights none; any remaining round lights at least one, so a
/// nearly-spent turret never reads as fully empty; a full magazine lights them
/// all. A zero-capacity magazine (degenerate) lights none.
pub fn turret_lit_segments(rounds: u32, capacity: u32) -> usize {
    if rounds == 0 || capacity == 0 {
        return 0;
    }
    let fraction = rounds as f32 / capacity as f32;
    let lit = (fraction * RING_SEGMENTS as f32).round() as usize;
    lit.clamp(1, RING_SEGMENTS)
}

/// Absolute position (left, top in px) of ring pip `index` within a `RING_PX`
/// node: evenly spaced around a circle, pip 0 at the top, going clockwise.
fn ring_pip_pos(index: usize) -> (f32, f32) {
    let center = RING_PX / 2.0;
    let radius = (RING_PX - RING_PIP_PX) / 2.0;
    let angle = index as f32 / RING_SEGMENTS as f32 * TAU - FRAC_PI_2;
    let left = center + radius * angle.cos() - RING_PIP_PX / 2.0;
    let top = center + radius * angle.sin() - RING_PIP_PX / 2.0;
    (left, top)
}

/// The shared screen-projected node for a readout, anchored to `section`.
fn readout_indicator(section: Entity, size: Vec2) -> impl Bundle {
    screen_indicator(ScreenIndicatorConfig {
        anchor: Some(ScreenIndicatorAnchorKind::Entity(section)),
        size: ScreenIndicatorSize::Fixed(size),
        // Sit just up-right of the weapon so the gauge reads as attached to,
        // not painted over, the barrel.
        offset: Vec2::new(RING_PX * 0.6, -RING_PX * 0.6),
        offscreen: ScreenIndicatorOffscreen::Hide,
    })
}

/// The debug number child (hidden until [`AmmoReadoutDebug`] is on). Debug-only.
#[cfg(feature = "debug")]
fn readout_number() -> impl Bundle {
    (
        Name::new("AmmoReadoutNumber"),
        AmmoReadoutNumber,
        Text::new(""),
        TextFont::from_font_size(9.0),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(100.0),
            ..default()
        },
        Visibility::Hidden,
    )
}

/// Spawn one turret ring readout under `layer` for `turret`.
fn spawn_turret_readout(commands: &mut Commands, layer: Entity, turret: Entity) {
    commands.entity(layer).with_children(|layer_children| {
        layer_children
            .spawn((
                Name::new("AmmoReadout(Turret)"),
                AmmoReadoutMarker,
                AmmoReadoutSection(turret),
                AmmoReadoutKind::Turret,
                readout_indicator(turret, Vec2::splat(RING_PX)),
            ))
            .with_children(|readout| {
                for index in 0..RING_SEGMENTS {
                    let (left, top) = ring_pip_pos(index);
                    readout.spawn((
                        AmmoReadoutPip(index),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(left),
                            top: Val::Px(top),
                            width: Val::Px(RING_PIP_PX),
                            height: Val::Px(RING_PIP_PX),
                            // Round the pip so the ring reads as dots, not a
                            // grid of squares.
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BackgroundColor(DIM_COLOR),
                        Outline::new(Val::Px(PIP_OUTLINE_PX), Val::ZERO, PIP_OUTLINE_COLOR),
                    ));
                }
                #[cfg(feature = "debug")]
                readout.spawn(readout_number());
            });
    });
}

/// Spawn one bar readout under `layer` for `section` with `capacity` pips.
///
/// Shared by the torpedo bay and the lance: both are magazines you count, and
/// `kind` is what the driver reads back to pick the hue.
fn spawn_bar_readout(
    commands: &mut Commands,
    layer: Entity,
    section: Entity,
    capacity: u32,
    kind: AmmoReadoutKind,
) {
    let pips = capacity.max(1);
    let width = pips as f32 * BAR_PIP_W + (pips.saturating_sub(1)) as f32 * BAR_PIP_GAP;
    commands.entity(layer).with_children(|layer_children| {
        layer_children
            .spawn((
                Name::new(format!("AmmoReadout({kind:?})")),
                AmmoReadoutMarker,
                AmmoReadoutSection(section),
                kind,
                readout_indicator(section, Vec2::new(width, BAR_PIP_H)),
            ))
            // Replace the widget's plain Node with a flex row so the bar pips
            // lay out left-to-right; the widget still writes size/position each
            // frame (insert-on-existing replaces, never a second Node - the
            // duplicate-Node panic from hud/mod.rs).
            .insert(Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(BAR_PIP_GAP),
                ..default()
            })
            .with_children(|readout| {
                for index in 0..pips as usize {
                    readout.spawn((
                        AmmoReadoutPip(index),
                        // A column of its own so the reload fill can grow from
                        // the pip's floor; clipped so a rounding error at the
                        // top of the wait cannot spill past the outline.
                        Node {
                            width: Val::Px(BAR_PIP_W),
                            height: Val::Px(BAR_PIP_H),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexEnd,
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(DIM_COLOR),
                        Outline::new(Val::Px(PIP_OUTLINE_PX), Val::ZERO, PIP_OUTLINE_COLOR),
                        children![(
                            AmmoReadoutPipFill,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(0.0),
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                        )],
                    ));
                }
                #[cfg(feature = "debug")]
                readout.spawn(readout_number());
            });
    });
}

/// Keep exactly one readout per player weapon section that carries a
/// [`SectionAmmo`]. A reconcile system (like `sync_turret_pips`): weapon
/// sections are destroyed mid-fight, ships gain their sections after the
/// player marker, and a section can lose its ammo component, so one idempotent
/// pass covers every ordering. Sections without `SectionAmmo` (infinite ammo)
/// never match, so they draw nothing.
#[expect(
    clippy::type_complexity,
    reason = "one query per weapon kind plus the readout layer"
)]
fn sync_ammo_readouts(
    mut commands: Commands,
    q_layer: Query<Entity, With<AmmoReadoutHudMarker>>,
    q_turrets: Query<(Entity, &ChildOf), (With<TurretSectionMarker>, With<SectionAmmo>)>,
    q_torpedoes: Query<(Entity, &ChildOf, &SectionAmmo), With<TorpedoSectionMarker>>,
    q_railguns: Query<(Entity, &ChildOf, &SectionAmmo), With<RailgunSectionMarker>>,
    q_readouts: Query<(Entity, &AmmoReadoutSection), With<AmmoReadoutMarker>>,
    q_player: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let Ok(layer) = q_layer.single() else {
        // No layer means no player HUD; the layer's despawn took its readouts.
        return;
    };
    let Ok(player) = q_player.single() else {
        // Player gone but HUD teardown has not run; the removal observer
        // despawns the layer (and its readouts).
        return;
    };

    // Despawn readouts whose section died, left the player, or lost its ammo
    // (turned infinite). A section that no longer matches either weapon query
    // as a player child is stale.
    for (readout, section) in &q_readouts {
        let alive = q_turrets
            .get(**section)
            .is_ok_and(|(_, ChildOf(parent))| *parent == player)
            || q_torpedoes
                .get(**section)
                .is_ok_and(|(_, ChildOf(parent), _)| *parent == player)
            || q_railguns
                .get(**section)
                .is_ok_and(|(_, ChildOf(parent), _)| *parent == player);
        if !alive {
            commands.entity(readout).despawn();
        }
    }

    // Spawn readouts for player weapon sections that have ammo but none yet.
    let has_readout = |section: Entity| q_readouts.iter().any(|(_, s)| **s == section);
    for (turret, ChildOf(parent)) in &q_turrets {
        if *parent == player && !has_readout(turret) {
            spawn_turret_readout(&mut commands, layer, turret);
        }
    }
    for (torpedo, ChildOf(parent), ammo) in &q_torpedoes {
        if *parent == player && !has_readout(torpedo) {
            spawn_bar_readout(
                &mut commands,
                layer,
                torpedo,
                ammo.capacity,
                AmmoReadoutKind::Torpedo,
            );
        }
    }
    for (railgun, ChildOf(parent), ammo) in &q_railguns {
        if *parent == player && !has_readout(railgun) {
            spawn_bar_readout(
                &mut commands,
                layer,
                railgun,
                ammo.capacity,
                AmmoReadoutKind::Railgun,
            );
        }
    }
}

/// Alpha of a lit / spent chunk. The hue now comes from the loaded round's
/// [`damage_type_color`]; these are the lit-vs-dim alphas applied over it (the
/// old `LIT_COLOR`/`DIM_COLOR` were this alpha over the Kinetic amber).
const LIT_ALPHA: f32 = 0.95;
const DIM_ALPHA: f32 = 0.16;
/// At or below this fraction of capacity a weapon group is NEARLY DRY and
/// switches to the warn state: amber pips with a slow breath (demo 2
/// `.grp.low`). A quarter magazine is the point where "top up before the next
/// pass" stops being optional; above it the gauge stays in its damage-type hue
/// so the normal readout never nags.
const LOW_AMMO_FRACTION: f32 = 0.25;

/// The warn breath: ~1.1 Hz, sweeping the lit alpha over this band. Slow enough
/// to read as a state, not an alarm strobe - it shares the emphasis-pulse
/// philosophy of the keybind dock (hue constant, alpha moves).
const WARN_PERIOD_SECS: f32 = 0.9;
// The band's floor stays ABOVE the lit/dim midpoint so a warning pip still
// reads (and still counts) as LIT at every phase of the breath.
const WARN_ALPHA: (f32, f32) = (0.62, LIT_ALPHA);

/// Alpha of the reload COLUMN inside an incoming bar pip. Above the pulse's
/// own peak, so the level reads against the pip it is filling; below
/// [`LIT_ALPHA`], so a column cannot be mistaken for a loaded round even at
/// the top of its travel - and the top of its travel is the tick the shell
/// lands. `the_reload_column_sits_between_the_pulse_and_a_live_round` pins
/// both ends.
const RELOAD_FILL_ALPHA: f32 = 0.70;

/// Percent steps the reload column is quantized to. The column is a `Node`
/// height, so every distinct value is a UI layout pass: at 100 steps a twelve
/// second reload relayouts about eight times a second and still reads as
/// continuous.
const RELOAD_FILL_STEPS: f32 = 100.0;

/// Incoming-batch pulse. Both ends brighten with progress, but the peak stays
/// below the live-pip threshold so incoming rounds never read as usable.
const RELOAD_PERIOD_SECS: f32 = 0.65;
const RELOAD_ALPHA_START: (f32, f32) = (0.24, 0.36);
const RELOAD_ALPHA_END: (f32, f32) = (0.40, 0.54);

/// Whether a weapon group is nearly dry (see [`LOW_AMMO_FRACTION`]). A group at
/// zero rounds counts as low too: an empty gauge with a dark track is easy to
/// mistake for "no weapon", and the amber tells you it is a weapon out of ammo.
pub(super) fn is_low_ammo(ammo: &SectionAmmo) -> bool {
    ammo.capacity > 0 && (ammo.rounds as f32) <= LOW_AMMO_FRACTION * ammo.capacity as f32
}

/// The warn breath's lit alpha at `elapsed` seconds.
fn warn_alpha(elapsed: f32) -> f32 {
    let (lo, hi) = WARN_ALPHA;
    let wave = 0.5 + 0.5 * (elapsed * std::f32::consts::TAU / WARN_PERIOD_SECS).sin();
    lo + (hi - lo) * wave
}

/// Alpha for an incoming batch: a visible pulse whose whole band brightens as
/// the delay approaches completion.
fn reload_alpha(elapsed: f32, progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    let lo = RELOAD_ALPHA_START.0 + (RELOAD_ALPHA_END.0 - RELOAD_ALPHA_START.0) * progress;
    let hi = RELOAD_ALPHA_START.1 + (RELOAD_ALPHA_END.1 - RELOAD_ALPHA_START.1) * progress;
    let wave = 0.5 + 0.5 * (elapsed * std::f32::consts::TAU / RELOAD_PERIOD_SECS).sin();
    lo + (hi - lo) * wave
}

/// Light each readout's chunks from its section's current `rounds/capacity`, in
/// the color of the loaded round's damage type. Turret
/// readouts read the section's [`LoadedBullet`] slot; torpedo readouts are
/// Explosive (a torpedo always detonates an Explosive `NovaBlast`).
///
/// While the section reloads, the pips in its next batch pulse in the same hue
/// and brighten with progress. Live rounds remain solid; later missing rounds
/// remain dark. A BAR pip also fills from its floor with the wait, which is how
/// a one-shell lance says how long is left rather than only that something is
/// coming.
/// This is the single point that reads ammo/reload state, so growing to
/// per-bullet-type magazines later stays a local change.
fn drive_ammo_readouts(
    time: Res<Time>,
    q_readouts: Query<(&AmmoReadoutSection, &AmmoReadoutKind, &Children), With<AmmoReadoutMarker>>,
    q_ammo: Query<&SectionAmmo>,
    q_reload: Query<&SectionReload>,
    q_loaded: Query<&LoadedBullet>,
    mut q_pips: Query<
        (&AmmoReadoutPip, &mut BackgroundColor, Option<&Children>),
        Without<AmmoReadoutPipFill>,
    >,
    mut q_fills: Query<(&mut Node, &mut BackgroundColor), With<AmmoReadoutPipFill>>,
) {
    for (section, kind, children) in &q_readouts {
        let Ok(ammo) = q_ammo.get(**section) else {
            continue;
        };
        // Total pips in this gauge: the fixed ring for a turret, one bar pip per
        // round of capacity for a torpedo bay.
        let (steady_lit, damage_type) = match kind {
            AmmoReadoutKind::Turret => (
                turret_lit_segments(ammo.rounds, ammo.capacity),
                // The turret's loaded round; default Kinetic if the slot is
                // somehow absent (production turrets always carry one).
                q_loaded
                    .get(**section)
                    .map(|loaded| loaded.kind)
                    .unwrap_or(DamageType::Kinetic),
            ),
            AmmoReadoutKind::Torpedo => (ammo.rounds as usize, DamageType::Explosive),
            // Pierce, and not read off a `LoadedBullet`: a lance has no
            // magazine of types to choose from, it authors the one slug.
            AmmoReadoutKind::Railgun => (ammo.rounds as usize, DamageType::Pierce),
        };
        // Preview only the next batch. Progress changes its pulse brightness,
        // not its size, so the gauge says both how much is coming and how near.
        let active_reload = q_reload
            .get(**section)
            .ok()
            .filter(|reload| reload.is_reloading(ammo));
        let reload_end = active_reload.map_or(steady_lit, |reload| match kind {
            AmmoReadoutKind::Turret => {
                turret_lit_segments(reload.incoming_rounds(ammo), ammo.capacity)
            }
            AmmoReadoutKind::Torpedo | AmmoReadoutKind::Railgun => {
                reload.incoming_rounds(ammo) as usize
            }
        });
        // Nearly dry: the whole group goes amber and breathes, so a magazine
        // about to run out is visible without reading a number (demo 2
        // `.grp.low`). A group in a reload cycle is deliberately NOT warned -
        // it is already coming back.
        let low = is_low_ammo(ammo) && active_reload.is_none();
        let hue = if low {
            nova_ui::theme::AMBER_NOVA
        } else {
            damage_type_color(damage_type)
        };
        let lit_alpha = if low {
            warn_alpha(time.elapsed_secs())
        } else {
            LIT_ALPHA
        };
        let lit_color = hue.with_alpha(lit_alpha);
        let reload_color = hue.with_alpha(active_reload.map_or(DIM_ALPHA, |reload| {
            reload_alpha(time.elapsed_secs(), reload.progress())
        }));
        let dim_color = hue.with_alpha(DIM_ALPHA);
        // Quantized so a pip that is not moving writes no `Node` at all, and a
        // pip that is moving writes one a hundred times over the whole wait
        // instead of once a frame.
        let fill_height = active_reload.map_or(0.0, |reload| {
            (reload.progress() * RELOAD_FILL_STEPS).round() / RELOAD_FILL_STEPS
        });
        let fill_color = hue.with_alpha(RELOAD_FILL_ALPHA);
        for &child in children {
            let Ok((pip, mut color, pip_children)) = q_pips.get_mut(child) else {
                continue;
            };
            let incoming = **pip >= steady_lit && **pip < reload_end;
            color.0 = if **pip < steady_lit {
                lit_color
            } else if incoming {
                reload_color
            } else {
                dim_color
            };
            for fill in pip_children.map(Children::iter).into_iter().flatten() {
                let Ok((mut node, mut fill_background)) = q_fills.get_mut(fill) else {
                    continue;
                };
                let height = Val::Percent(if incoming { fill_height * 100.0 } else { 0.0 });
                let color = if incoming { fill_color } else { Color::NONE };
                if node.height != height {
                    node.height = height;
                }
                if fill_background.0 != color {
                    fill_background.0 = color;
                }
            }
        }
    }
}

/// Write `rounds/capacity` onto each readout's debug number child and show it
/// while [`AmmoReadoutDebug`] is on. Debug-only: compiled out of release builds
/// so the exact count is never a gameplay affordance.
#[cfg(feature = "debug")]
fn drive_ammo_readout_numbers(
    debug: Res<AmmoReadoutDebug>,
    q_readouts: Query<(&AmmoReadoutSection, &Children), With<AmmoReadoutMarker>>,
    q_ammo: Query<&SectionAmmo>,
    mut q_number: Query<(&mut Text, &mut Visibility), With<AmmoReadoutNumber>>,
) {
    let number_visibility = if **debug {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    for (section, children) in &q_readouts {
        let Ok(ammo) = q_ammo.get(**section) else {
            continue;
        };
        for &child in children {
            if let Ok((mut text, mut visibility)) = q_number.get_mut(child) {
                let wanted = format!("{}/{}", ammo.rounds, ammo.capacity);
                if text.0 != wanted {
                    text.0 = wanted;
                }
                visibility.set_if_neq(number_visibility);
            }
        }
    }
}

/// Toggle the debug ammo number on F11 (gameplay only). Debug-only.
#[cfg(feature = "debug")]
fn toggle_ammo_readout_debug(mut debug: ResMut<AmmoReadoutDebug>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(DEBUG_TOGGLE_KEY) {
        **debug = !**debug;
    }
}

/// Draws the diegetic per-weapon ammo gauges (turret ring, torpedo bar) on
/// each player weapon section that carries a finite [`SectionAmmo`], with a
/// incoming-batch pulse and a debug-only numeric readout.
/// Registers the readout marker/kind/pip types, runs `sync_ammo_readouts` then
/// `drive_ammo_readouts` (chained) in PostUpdate before `ScreenIndicatorSystems`;
/// under the `debug` feature also inits `AmmoReadoutDebug` and adds the F11
/// toggle plus the numeric driver.
#[derive(Default)]
pub struct AmmoReadoutPlugin;

impl Plugin for AmmoReadoutPlugin {
    fn build(&self, app: &mut App) {
        trace!("AmmoReadoutPlugin: build");

        app.register_type::<AmmoReadoutHudMarker>();
        app.register_type::<AmmoReadoutMarker>();
        app.register_type::<AmmoReadoutSection>();
        app.register_type::<AmmoReadoutKind>();
        app.register_type::<AmmoReadoutPip>();
        app.register_type::<AmmoReadoutPipFill>();

        // Reconcile then light the chunks before the indicator projection
        // places the nodes, mirroring TurretLeadPlugin's slot.
        app.add_systems(
            PostUpdate,
            (sync_ammo_readouts, drive_ammo_readouts)
                .chain()
                .before(ScreenIndicatorSystems),
        );
        // The contextual gate rides the normal HUD drivers: written in Update
        // from this frame's situations, enforced in PostUpdate by
        // `apply_hud_visibility` after the projection.
        app.add_systems(Update, sync_ammo_gate.in_set(super::NovaHudSystems));

        // The numeric readout is debug-only (never compiled into release): its
        // resource, F11 toggle and driver all live behind the `debug` feature.
        #[cfg(feature = "debug")]
        {
            app.init_resource::<AmmoReadoutDebug>();
            app.register_type::<AmmoReadoutDebug>();
            app.register_type::<AmmoReadoutNumber>();
            // UNGATED on purpose - this mirrors
            // nova_debug's `toggle_debug_mode`, which is also ungated, so the two
            // F11 flags stay in phase from their shared `true` default. Gating
            // this to `Playing` (the old bug) let an F11 press in the menu/editor
            // flip `DebugEnabled` but not this mirror, leaving the ammo number
            // visible with debug off. Do not re-add a state gate here.
            app.add_systems(Update, toggle_ammo_readout_debug);
            app.add_systems(
                PostUpdate,
                drive_ammo_readout_numbers
                    .after(drive_ammo_readouts)
                    .before(ScreenIndicatorSystems),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn spawn_player(world: &mut World) -> Entity {
        world
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id()
    }

    fn spawn_turret(world: &mut World, parent: Entity, ammo: Option<SectionAmmo>) -> Entity {
        let mut ec = world.spawn((TurretSectionMarker, ChildOf(parent)));
        if let Some(ammo) = ammo {
            ec.insert(ammo);
        }
        ec.id()
    }

    fn spawn_torpedo(world: &mut World, parent: Entity, ammo: Option<SectionAmmo>) -> Entity {
        let mut ec = world.spawn((TorpedoSectionMarker, ChildOf(parent)));
        if let Some(ammo) = ammo {
            ec.insert(ammo);
        }
        ec.id()
    }

    fn spawn_railgun(world: &mut World, parent: Entity, ammo: Option<SectionAmmo>) -> Entity {
        let mut ec = world.spawn((RailgunSectionMarker, ChildOf(parent)));
        if let Some(ammo) = ammo {
            ec.insert(ammo);
        }
        ec.id()
    }

    fn readout_sections(world: &mut World) -> Vec<Entity> {
        let mut sections: Vec<Entity> = world
            .query_filtered::<&AmmoReadoutSection, With<AmmoReadoutMarker>>()
            .iter(world)
            .map(|section| **section)
            .collect();
        sections.sort();
        sections
    }

    // -- pure helper --

    #[test]
    fn turret_lit_segments_buckets_full_partial_empty() {
        assert_eq!(turret_lit_segments(0, 8), 0, "empty lights none");
        assert_eq!(turret_lit_segments(8, 8), RING_SEGMENTS, "full lights all");
        assert_eq!(turret_lit_segments(4, 8), 4, "half lights half");
        // Any remaining round lights at least one chunk, even far below 1/8.
        assert_eq!(turret_lit_segments(1, 100), 1, "one round still lit");
        // Degenerate zero-capacity magazine never divides by zero.
        assert_eq!(turret_lit_segments(0, 0), 0);
    }

    // -- reconcile --

    #[test]
    fn sync_spawns_one_readout_per_player_weapon_with_ammo() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(8)));
        let torpedo = spawn_torpedo(&mut world, player, Some(SectionAmmo::new(4)));

        world.run_system_once(sync_ammo_readouts).unwrap();

        let mut expected = vec![turret, torpedo];
        expected.sort();
        assert_eq!(readout_sections(&mut world), expected);

        // Idempotent: a second pass adds nothing.
        world.run_system_once(sync_ammo_readouts).unwrap();
        assert_eq!(readout_sections(&mut world), expected);
    }

    #[test]
    fn sync_ignores_infinite_ammo_weapons() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        // No SectionAmmo == infinite ammo: no readout at all.
        spawn_turret(&mut world, player, None);
        let finite = spawn_torpedo(&mut world, player, Some(SectionAmmo::new(4)));

        world.run_system_once(sync_ammo_readouts).unwrap();

        assert_eq!(readout_sections(&mut world), vec![finite]);
    }

    #[test]
    fn sync_ignores_other_ships_weapons() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        spawn_player(&mut world);
        let enemy = world.spawn(SpaceshipRootMarker).id();
        spawn_turret(&mut world, enemy, Some(SectionAmmo::new(8)));

        world.run_system_once(sync_ammo_readouts).unwrap();

        assert!(readout_sections(&mut world).is_empty());
    }

    #[test]
    fn sync_despawns_readout_of_a_dead_weapon() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(8)));
        let torpedo = spawn_torpedo(&mut world, player, Some(SectionAmmo::new(4)));
        world.run_system_once(sync_ammo_readouts).unwrap();

        world.despawn(turret);
        world.run_system_once(sync_ammo_readouts).unwrap();

        assert_eq!(readout_sections(&mut world), vec![torpedo]);
    }

    #[test]
    fn sync_despawns_readout_when_ammo_becomes_infinite() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(8)));
        world.run_system_once(sync_ammo_readouts).unwrap();
        assert_eq!(readout_sections(&mut world), vec![turret]);

        // Dropping the component (a reload-to-infinite, say) removes the gauge.
        world.entity_mut(turret).remove::<SectionAmmo>();
        world.run_system_once(sync_ammo_readouts).unwrap();
        assert!(readout_sections(&mut world).is_empty());
    }

    // -- driver --

    /// Count lit pips (by color) among a readout's pip children.
    fn lit_pip_count(world: &mut World, section: Entity) -> usize {
        let readout = world
            .query_filtered::<(Entity, &AmmoReadoutSection), With<AmmoReadoutMarker>>()
            .iter(world)
            .find(|(_, s)| ***s == section)
            .map(|(entity, _)| entity)
            .expect("readout exists");
        let children: Vec<Entity> = world
            .entity(readout)
            .get::<Children>()
            .map(|children| children.iter().collect())
            .unwrap_or_default();
        children
            .into_iter()
            .filter_map(|child| world.entity(child).get::<BackgroundColor>().copied())
            // Lit pips carry LIT_ALPHA, dim pips DIM_ALPHA, regardless of the
            // per-type hue - count by alpha so this works for any ammo type.
            .filter(|color| color.0.alpha() > (LIT_ALPHA + DIM_ALPHA) / 2.0)
            .count()
    }

    /// The colour of a LIT pip in `section`'s readout (the first one found).
    fn lit_pip_color(world: &mut World, section: Entity) -> Option<Color> {
        let readout = world
            .query_filtered::<(Entity, &AmmoReadoutSection), With<AmmoReadoutMarker>>()
            .iter(world)
            .find(|(_, s)| ***s == section)
            .map(|(entity, _)| entity)?;
        let children: Vec<Entity> = world
            .entity(readout)
            .get::<Children>()
            .map(|children| children.iter().collect())
            .unwrap_or_default();
        children
            .into_iter()
            .filter_map(|child| world.entity(child).get::<BackgroundColor>().copied())
            .find(|color| color.0.alpha() > (LIT_ALPHA + DIM_ALPHA) / 2.0)
            .map(|color| color.0)
    }

    #[test]
    fn driver_lights_turret_chunks_by_fraction() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(8)));
        world.run_system_once(sync_ammo_readouts).unwrap();

        // Full magazine: all segments lit.
        world.run_system_once(drive_ammo_readouts).unwrap();
        assert_eq!(lit_pip_count(&mut world, turret), RING_SEGMENTS);

        // Spend to half: half the ring.
        world
            .entity_mut(turret)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 4;
        world.run_system_once(drive_ammo_readouts).unwrap();
        assert_eq!(lit_pip_count(&mut world, turret), 4);

        // Empty: nothing lit.
        world
            .entity_mut(turret)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 0;
        world.run_system_once(drive_ammo_readouts).unwrap();
        assert_eq!(lit_pip_count(&mut world, turret), 0);
    }

    /// The lance gets a gauge like any other finite magazine, and it is the
    /// HUE that carries the message: one pip is one pip either way, so a bar
    /// in Explosive orange and a bar in Pierce blue is how the player tells a
    /// spent bay from a spent lance without counting anything.
    #[test]
    fn a_spent_lance_darkens_its_one_pierce_blue_pip() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let lance = spawn_railgun(&mut world, player, Some(SectionAmmo::new(1)));
        world.run_system_once(sync_ammo_readouts).unwrap();

        assert_eq!(
            readout_sections(&mut world),
            vec![lance],
            "a lance with a finite magazine draws a readout"
        );

        world.run_system_once(drive_ammo_readouts).unwrap();
        assert_eq!(lit_pip_count(&mut world, lance), 1, "loaded");
        let lit = first_lit_pip_color(&mut world, lance);
        assert_eq!(
            lit,
            damage_type_color(DamageType::Pierce).with_alpha(LIT_ALPHA),
            "the slug is Pierce, and the gauge says so"
        );
        assert_ne!(
            lit,
            damage_type_color(DamageType::Explosive).with_alpha(LIT_ALPHA),
            "a lance must not read as a torpedo bay - the hue is the whole tell"
        );

        world
            .entity_mut(lance)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 0;
        world.run_system_once(drive_ammo_readouts).unwrap();
        assert_eq!(lit_pip_count(&mut world, lance), 0, "spent");
    }

    #[test]
    fn driver_lights_one_torpedo_pip_per_remaining_round() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let torpedo = spawn_torpedo(&mut world, player, Some(SectionAmmo::new(4)));
        world.run_system_once(sync_ammo_readouts).unwrap();

        world.run_system_once(drive_ammo_readouts).unwrap();
        assert_eq!(lit_pip_count(&mut world, torpedo), 4);

        world
            .entity_mut(torpedo)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 1;
        world.run_system_once(drive_ammo_readouts).unwrap();
        assert_eq!(lit_pip_count(&mut world, torpedo), 1);
    }

    /// The color of the first lit pip of `section`'s readout.
    fn first_lit_pip_color(world: &mut World, section: Entity) -> Color {
        let readout = world
            .query_filtered::<(Entity, &AmmoReadoutSection), With<AmmoReadoutMarker>>()
            .iter(world)
            .find(|(_, s)| ***s == section)
            .map(|(entity, _)| entity)
            .expect("readout exists");
        let children: Vec<Entity> = world
            .entity(readout)
            .get::<Children>()
            .map(|children| children.iter().collect())
            .unwrap_or_default();
        children
            .into_iter()
            .filter_map(|child| world.entity(child).get::<BackgroundColor>().copied())
            .map(|c| c.0)
            .find(|c| c.alpha() > (LIT_ALPHA + DIM_ALPHA) / 2.0)
            .expect("at least one lit pip")
    }

    #[test]
    fn driver_colors_pips_by_loaded_ammo_type() {
        // The readout hue tracks the loaded round's DamageType: a turret loaded
        // with Pierce reads in the penetrator blue (differs from the Kinetic
        // amber), and a torpedo reads Explosive.
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(8)));
        world.entity_mut(turret).insert(LoadedBullet {
            kind: DamageType::Pierce,
            damage: 5.0,
        });
        let torpedo = spawn_torpedo(&mut world, player, Some(SectionAmmo::new(4)));
        world.run_system_once(sync_ammo_readouts).unwrap();
        world.run_system_once(drive_ammo_readouts).unwrap();

        let turret_lit = first_lit_pip_color(&mut world, turret);
        assert_eq!(
            turret_lit,
            damage_type_color(DamageType::Pierce).with_alpha(LIT_ALPHA),
            "a Pierce-loaded turret reads in the penetrator hue"
        );
        assert_ne!(
            turret_lit,
            damage_type_color(DamageType::Kinetic).with_alpha(LIT_ALPHA),
            "Pierce must read differently from the Kinetic amber (the point of color-coding)"
        );

        // Torpedoes always detonate an Explosive blast, so their readout is
        // Explosive-colored even though they carry no LoadedBullet slot.
        assert_eq!(
            first_lit_pip_color(&mut world, torpedo),
            damage_type_color(DamageType::Explosive).with_alpha(LIT_ALPHA),
            "a torpedo bay reads Explosive"
        );
    }

    #[cfg(feature = "debug")]
    #[test]
    fn driver_debug_number_follows_the_toggle() {
        let mut world = World::new();
        world.init_resource::<AmmoReadoutDebug>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(8)));
        world.run_system_once(sync_ammo_readouts).unwrap();
        world
            .entity_mut(turret)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 5;

        // Debug off: the number is hidden.
        **world.resource_mut::<AmmoReadoutDebug>() = false;
        world.run_system_once(drive_ammo_readout_numbers).unwrap();
        let (text, visibility) = world
            .query_filtered::<(&Text, &Visibility), With<AmmoReadoutNumber>>()
            .single(&world)
            .unwrap();
        assert_eq!(text.0, "5/8");
        assert_eq!(*visibility, Visibility::Hidden);

        // Debug on: the number shows.
        **world.resource_mut::<AmmoReadoutDebug>() = true;
        world.run_system_once(drive_ammo_readout_numbers).unwrap();
        let visibility = world
            .query_filtered::<&Visibility, With<AmmoReadoutNumber>>()
            .single(&world)
            .unwrap();
        assert_eq!(*visibility, Visibility::Inherited);
    }

    #[cfg(feature = "debug")]
    #[test]
    fn f11_flips_the_ammo_debug_flag() {
        // The toggle must flip on F11 so the number tracks debug mode. (The
        // desync bug this guards against was in the REGISTRATION - a `Playing`
        // state gate that let the flag fall out of phase with nova_debug's
        // ungated toggle; keep this system ungated, see AmmoReadoutPlugin.)
        let mut world = World::new();
        world.init_resource::<AmmoReadoutDebug>();
        // Default OFF, in phase with nova_debug's `DebugEnabled(false)`: the
        // whole debug layer boots off and F11 raises it as one.
        assert!(
            !**world.resource::<AmmoReadoutDebug>(),
            "the ammo number defaults off, matching the rest of the debug layer"
        );
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(DEBUG_TOGGLE_KEY);
        world.insert_resource(input);

        world.run_system_once(toggle_ammo_readout_debug).unwrap();
        assert!(
            **world.resource::<AmmoReadoutDebug>(),
            "F11 turns the ammo number on"
        );

        // A fresh press flips it back. (A new ButtonInput, not clear()+press():
        // clear() keeps F11 in the `pressed` set, so a re-press would not raise a
        // new just_pressed edge.)
        let mut next = ButtonInput::<KeyCode>::default();
        next.press(DEBUG_TOGGLE_KEY);
        world.insert_resource(next);
        world.run_system_once(toggle_ammo_readout_debug).unwrap();
        assert!(
            !**world.resource::<AmmoReadoutDebug>(),
            "a second F11 turns it back off"
        );
    }

    // -- reload batch pulse --

    #[test]
    fn reload_alpha_brightens_with_progress_and_pulses() {
        let peak = RELOAD_PERIOD_SECS * 0.25;
        let trough = RELOAD_PERIOD_SECS * 0.75;
        assert!(reload_alpha(peak, 1.0) > reload_alpha(peak, 0.0));
        assert!(reload_alpha(peak, 0.5) > reload_alpha(trough, 0.5));
        assert!(reload_alpha(peak, 1.0) < (LIT_ALPHA + DIM_ALPHA) / 2.0);
    }

    /// Count pips rendered as incoming: brighter than missing, dimmer than live.
    fn reload_pip_count(world: &mut World, section: Entity) -> usize {
        let readout = world
            .query_filtered::<(Entity, &AmmoReadoutSection), With<AmmoReadoutMarker>>()
            .iter(world)
            .find(|(_, s)| ***s == section)
            .map(|(entity, _)| entity)
            .expect("readout exists");
        world
            .entity(readout)
            .get::<Children>()
            .map(|children| children.iter().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|child| world.entity(child).get::<BackgroundColor>().copied())
            .filter(|color| {
                color.0.alpha() > DIM_ALPHA + 0.02
                    && color.0.alpha() < (LIT_ALPHA + DIM_ALPHA) / 2.0
            })
            .count()
    }

    fn reload_at(delay: f32, amount: u32, progress: f32) -> SectionReload {
        let mut reload = SectionReload::from_config(SectionReloadConfig { delay, amount });
        reload.elapsed = delay * progress;
        reload
    }

    #[test]
    fn driver_pulses_only_the_next_pdc_batch() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(500)));
        world
            .entity_mut(turret)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 0;
        world.entity_mut(turret).insert(reload_at(3.0, 200, 0.5));
        world.run_system_once(sync_ammo_readouts).unwrap();
        world.run_system_once(drive_ammo_readouts).unwrap();

        assert_eq!(lit_pip_count(&mut world, turret), 0);
        assert_eq!(
            reload_pip_count(&mut world, turret),
            3,
            "200 of 500 previews three coarse ring segments"
        );

        world.entity_mut(turret).remove::<SectionReload>();
        world.run_system_once(drive_ammo_readouts).unwrap();
        assert_eq!(reload_pip_count(&mut world, turret), 0);
    }

    #[test]
    fn driver_pulses_one_incoming_torpedo_above_live_rounds() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let torpedo = spawn_torpedo(&mut world, player, Some(SectionAmmo::new(4)));
        world
            .entity_mut(torpedo)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 1;
        world.entity_mut(torpedo).insert(reload_at(10.0, 1, 0.5));
        world.run_system_once(sync_ammo_readouts).unwrap();
        world.run_system_once(drive_ammo_readouts).unwrap();

        assert_eq!(lit_pip_count(&mut world, torpedo), 1);
        assert_eq!(reload_pip_count(&mut world, torpedo), 1);
    }

    /// A nearly-dry group goes AMBER and breathes (demo 2
    /// `.grp.low`), while a healthy group keeps its damage-type hue. Without
    /// this the only "you are out" signal was counting dark pips.
    #[test]
    fn driver_warns_amber_on_a_nearly_dry_group() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(8)));
        world.run_system_once(sync_ammo_readouts).unwrap();

        // Healthy magazine: the damage-type hue, no warning.
        world.run_system_once(drive_ammo_readouts).unwrap();
        let healthy = lit_pip_color(&mut world, turret).expect("a lit pip");
        assert_ne!(
            healthy.to_srgba().to_vec3(),
            nova_ui::theme::AMBER_NOVA.to_srgba().to_vec3(),
            "a full magazine does not nag"
        );

        // Down to a quarter: amber.
        world
            .entity_mut(turret)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 2;
        world.run_system_once(drive_ammo_readouts).unwrap();
        let low = lit_pip_color(&mut world, turret).expect("a lit pip");
        assert_eq!(
            low.to_srgba().to_vec3(),
            nova_ui::theme::AMBER_NOVA.to_srgba().to_vec3(),
            "a nearly-dry group warns in amber"
        );
        assert_eq!(
            lit_pip_count(&mut world, turret),
            2,
            "the warn state does not change WHICH pips are lit"
        );

        // The warn state actually breathes: the alpha moves across the cycle
        // (a flat pulse would be dead code), and never drops out of `lit`.
        let alphas: Vec<f32> = (0..=8)
            .map(|i| warn_alpha(i as f32 * WARN_PERIOD_SECS / 8.0))
            .collect();
        let (min, max) = alphas
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &a| (lo.min(a), hi.max(a)));
        assert!(max - min > 0.2, "the warn breath sweeps its band");
        assert!(
            min > (LIT_ALPHA + DIM_ALPHA) / 2.0,
            "a warning pip stays clearly lit at every phase"
        );

        // A group that is rearming is NOT warned - its incoming batch already
        // communicates recovery.
        let torpedo = spawn_torpedo(&mut world, player, Some(SectionAmmo::new(4)));
        world
            .entity_mut(torpedo)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 1;
        world.entity_mut(torpedo).insert(reload_at(4.0, 1, 0.5));
        world.run_system_once(sync_ammo_readouts).unwrap();
        world.run_system_once(drive_ammo_readouts).unwrap();
        let reloading = lit_pip_color(&mut world, torpedo).expect("a lit pip");
        assert_ne!(
            reloading.to_srgba().to_vec3(),
            nova_ui::theme::AMBER_NOVA.to_srgba().to_vec3(),
            "a rearming group shows its batch pulse, not the warning"
        );
    }

    #[test]
    fn driver_at_rest_reload_is_identical_to_no_reload() {
        // A full magazine that carries a SectionReload is not reloading, so the
        // gauge is byte-identical to the shipped steady rendering (no regression
        // to loaded-type/count).
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(8)));
        world.entity_mut(turret).insert(reload_at(2.0, 8, 0.0));
        world.run_system_once(sync_ammo_readouts).unwrap();
        world.run_system_once(drive_ammo_readouts).unwrap();

        assert_eq!(
            lit_pip_count(&mut world, turret),
            RING_SEGMENTS,
            "full mag all lit"
        );
        assert_eq!(
            reload_pip_count(&mut world, turret),
            0,
            "a rested reload pulses nothing"
        );
    }

    /// The height of the reload column in `section`'s FIRST bar pip, as a
    /// percentage. `None` when that pip carries no column (a ring segment).
    fn fill_percent(world: &mut World, section: Entity) -> Option<f32> {
        let readout = world
            .query_filtered::<(Entity, &AmmoReadoutSection), With<AmmoReadoutMarker>>()
            .iter(world)
            .find(|(_, s)| ***s == section)
            .map(|(entity, _)| entity)
            .expect("readout exists");
        let pips: Vec<Entity> = world
            .entity(readout)
            .get::<Children>()
            .map(|children| children.iter().collect())
            .unwrap_or_default();
        let fills: Vec<Entity> = pips
            .into_iter()
            .filter(|pip| world.entity(*pip).contains::<AmmoReadoutPip>())
            .filter_map(|pip| world.entity(pip).get::<Children>().map(Children::iter))
            .flatten()
            .filter(|child| world.entity(*child).contains::<AmmoReadoutPipFill>())
            .collect();
        let fill = *fills.first()?;
        match world.entity(fill).get::<Node>()?.height {
            Val::Percent(percent) => Some(percent),
            other => panic!("the reload column must be a percentage, not {other:?}"),
        }
    }

    /// The lance is one pip and a twelve-second wait, so the pulse alone said
    /// only "something is coming". The column says how much of the wait is
    /// gone, which is most of what the weapon feels like.
    #[test]
    fn a_reloading_lance_fills_its_one_pip_as_the_wait_runs() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let lance = spawn_railgun(&mut world, player, Some(SectionAmmo::new(1)));
        world
            .entity_mut(lance)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 0;
        world.entity_mut(lance).insert(reload_at(12.0, 1, 0.25));
        world.run_system_once(sync_ammo_readouts).unwrap();
        world.run_system_once(drive_ammo_readouts).unwrap();

        assert_eq!(
            fill_percent(&mut world, lance),
            Some(25.0),
            "a quarter of the wait is a quarter of the pip"
        );

        world.entity_mut(lance).insert(reload_at(12.0, 1, 0.75));
        world.run_system_once(drive_ammo_readouts).unwrap();
        assert_eq!(
            fill_percent(&mut world, lance),
            Some(75.0),
            "the column tracks the wait, not the pulse"
        );

        world.entity_mut(lance).remove::<SectionReload>();
        world
            .entity_mut(lance)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 1;
        world.run_system_once(drive_ammo_readouts).unwrap();
        assert_eq!(
            fill_percent(&mut world, lance),
            Some(0.0),
            "a loaded lance shows no column at all"
        );
        assert_eq!(lit_pip_count(&mut world, lance), 1, "and reads as loaded");
    }

    /// A live round is a solid pip. A column is not one, at any height, or the
    /// gauge would tell you a shell is back before it is.
    #[test]
    fn the_reload_column_sits_between_the_pulse_and_a_live_round() {
        assert!(
            RELOAD_ALPHA_END.1 < RELOAD_FILL_ALPHA,
            "the column must read against the pulse it fills"
        );
        assert!(
            RELOAD_FILL_ALPHA < LIT_ALPHA,
            "a full column must still not read as a loaded round"
        );
    }

    /// A ring segment previews a batch of two hundred rounds, not a moment
    /// worth counting down, so the turret gauge is unchanged.
    #[test]
    fn a_turret_ring_segment_carries_no_reload_column() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(500)));
        world
            .entity_mut(turret)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 0;
        world.entity_mut(turret).insert(reload_at(3.0, 200, 0.5));
        world.run_system_once(sync_ammo_readouts).unwrap();
        world.run_system_once(drive_ammo_readouts).unwrap();

        assert_eq!(fill_percent(&mut world, turret), None);
        assert_eq!(
            reload_pip_count(&mut world, turret),
            3,
            "the ring still previews its incoming batch by pulsing"
        );
    }

    // -- contextual gate --

    fn gate(world: &mut World) -> bool {
        world
            .query_filtered::<&HudContextGate, With<AmmoReadoutHudMarker>>()
            .single(world)
            .expect("one ammo layer")
            .0
    }

    /// The situation drives the gate BOTH ways: gauges appear while the weapons
    /// are hot and go away again when the safety goes back on.
    #[test]
    fn the_ammo_gate_opens_on_weapons_hot_and_shuts_again() {
        let mut world = World::new();
        world.init_resource::<HudSituations>();
        world.spawn(ammo_readout_hud());
        assert!(
            !gate(&mut world),
            "idle cruise keeps the gauges out of sight"
        );

        world.resource_mut::<HudSituations>().weapons_hot = true;
        world.run_system_once(sync_ammo_gate).unwrap();
        assert!(gate(&mut world), "weapons hot shows the gauges");

        world.resource_mut::<HudSituations>().weapons_hot = false;
        world.run_system_once(sync_ammo_gate).unwrap();
        assert!(!gate(&mut world), "safety back on hides them again");
    }

    /// Low ammo forces the gauges up on its own: a dry magazine is news before
    /// you pull the trigger, not after.
    #[test]
    fn low_ammo_alone_opens_the_ammo_gate() {
        let mut world = World::new();
        world.init_resource::<HudSituations>();
        world.spawn(ammo_readout_hud());

        world.resource_mut::<HudSituations>().low_ammo = true;
        world.run_system_once(sync_ammo_gate).unwrap();
        assert!(
            gate(&mut world),
            "a nearly-dry group shows the gauges even with the safety on"
        );
    }

    #[test]
    fn active_reload_alone_opens_the_ammo_gate() {
        let mut world = World::new();
        world.init_resource::<HudSituations>();
        world.spawn(ammo_readout_hud());

        world.resource_mut::<HudSituations>().reloading = true;
        world.run_system_once(sync_ammo_gate).unwrap();
        assert!(gate(&mut world), "an active reload keeps its gauge visible");
    }
}
