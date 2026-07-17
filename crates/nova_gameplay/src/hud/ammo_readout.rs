//! Diegetic per-weapon ammo readouts: a small chunked gauge drawn ON each
//! player weapon that still carries a finite [`SectionAmmo`], so the player
//! can see a turret or torpedo bay running dry without reading a corner panel
//! (task 20260712-131348; direction settled in
//! docs/spikes/20260712-143113-diegetic-ammo-readout.md).
//!
//! A thin consumer of the [`screen_indicator`](super::screen_indicator)
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
//!   remaining rounds lit.
//!
//! While a weapon is reloading (it carries a [`SectionReload`] mid-cycle) the
//! pips above the live-round level fill as a reload sweep in the same hue at a
//! dimmer [`RELOAD_ALPHA`], proportional to cycle progress: a spent turret ring
//! fills from empty back to full, and a rearming torpedo bar lights the rounds
//! coming back above the ones still loaded (task 20260716-123556).
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
//! together via [`AmmoReadoutDebug`].
//!
//! Like the other combat overlays the layer is `HudTier::Instrument` and is
//! spawned/despawned with the player ship by the hud/mod.rs observers.

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::prelude::*;

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

pub mod prelude {
    pub use super::{
        ammo_readout_hud, AmmoReadoutHudMarker, AmmoReadoutKind, AmmoReadoutMarker, AmmoReadoutPip,
        AmmoReadoutPlugin, AmmoReadoutSection, RING_SEGMENTS,
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
}

/// A single chunk of a gauge, carrying its position in the lit order.
#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Reflect)]
pub struct AmmoReadoutPip(pub usize);

/// The debug `rounds/capacity` text child of a readout. Debug-only: only
/// compiled under the `debug` feature.
#[cfg(feature = "debug")]
#[derive(Component, Debug, Clone, Reflect)]
pub struct AmmoReadoutNumber;

/// Whether the debug ammo number is shown (toggled with F11). On by default so
/// it starts in phase with nova_debug's `DebugEnabled(true)`: the number then
/// tracks debug mode (shown while on, hidden once F11 switches debug off)
/// instead of inverting it. The gauge itself is always on. Debug-only: only
/// compiled under the `debug` feature, so release builds have no numeric
/// readout at all.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone, Copy, Deref, DerefMut, PartialEq, Eq, Reflect)]
#[reflect(Resource)]
pub struct AmmoReadoutDebug(pub bool);

/// Starts on to match nova_debug's `DebugEnabled(true)` default. Both toggle on
/// F11, so matching defaults keeps the ammo number in phase with debug mode; a
/// mismatch here is what makes the number show in normal play and vanish in
/// debug mode.
#[cfg(feature = "debug")]
impl Default for AmmoReadoutDebug {
    fn default() -> Self {
        Self(true)
    }
}

/// UI bundle for the readout layer. Readouts are spawned under it by
/// [`sync_ammo_readouts`], one per player weapon section with ammo.
pub fn ammo_readout_hud() -> impl Bundle {
    (
        Name::new("AmmoReadoutHUD"),
        AmmoReadoutHudMarker,
        screen_indicator_layer(),
    )
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

/// Spawn one torpedo bar readout under `layer` for `torpedo` with `capacity`
/// pips.
fn spawn_torpedo_readout(commands: &mut Commands, layer: Entity, torpedo: Entity, capacity: u32) {
    let pips = capacity.max(1);
    let width = pips as f32 * BAR_PIP_W + (pips.saturating_sub(1)) as f32 * BAR_PIP_GAP;
    commands.entity(layer).with_children(|layer_children| {
        layer_children
            .spawn((
                Name::new("AmmoReadout(Torpedo)"),
                AmmoReadoutMarker,
                AmmoReadoutSection(torpedo),
                AmmoReadoutKind::Torpedo,
                readout_indicator(torpedo, Vec2::new(width, BAR_PIP_H)),
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
                        Node {
                            width: Val::Px(BAR_PIP_W),
                            height: Val::Px(BAR_PIP_H),
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

/// Keep exactly one readout per player weapon section that carries a
/// [`SectionAmmo`]. A reconcile system (like `sync_turret_pips`): weapon
/// sections are destroyed mid-fight, ships gain their sections after the
/// player marker, and a section can lose its ammo component, so one idempotent
/// pass covers every ordering. Sections without `SectionAmmo` (infinite ammo)
/// never match, so they draw nothing.
#[allow(clippy::type_complexity)]
fn sync_ammo_readouts(
    mut commands: Commands,
    q_layer: Query<Entity, With<AmmoReadoutHudMarker>>,
    q_turrets: Query<(Entity, &ChildOf), (With<TurretSectionMarker>, With<SectionAmmo>)>,
    q_torpedoes: Query<(Entity, &ChildOf, &SectionAmmo), With<TorpedoSectionMarker>>,
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
            spawn_torpedo_readout(&mut commands, layer, torpedo, ammo.capacity);
        }
    }
}

/// Alpha of a lit / spent chunk. The hue now comes from the loaded round's
/// [`damage_type_color`]; these are the lit-vs-dim alphas applied over it (the
/// old `LIT_COLOR`/`DIM_COLOR` were this alpha over the Kinetic amber).
const LIT_ALPHA: f32 = 0.95;
const DIM_ALPHA: f32 = 0.16;
/// Alpha of a pip the reload sweep has filled - between dim and lit, so a
/// reloading track reads as "coming back" without being mistaken for live
/// rounds. Task 20260716-123556.
const RELOAD_ALPHA: f32 = 0.5;

/// How many pips above the `steady_lit` level the reload sweep has filled, given
/// the cycle `progress` (0..=1). The sweep fills the remaining track - from the
/// steady level up to full - as progress runs 0->1, so a discrete reload of an
/// empty magazine fills the whole gauge and a continuous regen lights just the
/// round being restored. Pure and gauge-agnostic (turret ring or torpedo bar):
/// the caller passes the pip count and the steady lit count. Task 20260716-123556.
fn reload_fill_segments(segment_count: usize, steady_lit: usize, progress: f32) -> usize {
    let remaining = segment_count.saturating_sub(steady_lit);
    ((progress.clamp(0.0, 1.0) * remaining as f32).round() as usize).min(remaining)
}

/// Light each readout's chunks from its section's current `rounds/capacity`, in
/// the color of the loaded round's damage type (task 20260712-133349). Turret
/// readouts read the section's [`LoadedBullet`] slot; torpedo readouts are
/// Explosive (a torpedo always detonates an Explosive `NovaBlast`).
///
/// While the section is reloading (it carries a [`SectionReload`] mid-cycle) the
/// pips above the steady lit level fill as a reload sweep in the same hue at
/// [`RELOAD_ALPHA`], so a spent turret ring fills from empty to full and a
/// rearming torpedo bar lights the round being restored (task 20260716-123556).
/// This is the single point that reads ammo/reload state, so growing to
/// per-bullet-type magazines later stays a local change.
fn drive_ammo_readouts(
    q_readouts: Query<(&AmmoReadoutSection, &AmmoReadoutKind, &Children), With<AmmoReadoutMarker>>,
    q_ammo: Query<&SectionAmmo>,
    q_reload: Query<&SectionReload>,
    q_loaded: Query<&LoadedBullet>,
    mut q_pips: Query<(&AmmoReadoutPip, &mut BackgroundColor)>,
) {
    for (section, kind, children) in &q_readouts {
        let Ok(ammo) = q_ammo.get(**section) else {
            continue;
        };
        // Total pips in this gauge: the fixed ring for a turret, one bar pip per
        // round of capacity for a torpedo bay.
        let (segment_count, steady_lit, damage_type) = match kind {
            AmmoReadoutKind::Turret => (
                RING_SEGMENTS,
                turret_lit_segments(ammo.rounds, ammo.capacity),
                // The turret's loaded round; default Kinetic if the slot is
                // somehow absent (production turrets always carry one).
                q_loaded
                    .get(**section)
                    .map(|loaded| loaded.kind)
                    .unwrap_or(DamageType::Kinetic),
            ),
            AmmoReadoutKind::Torpedo => (
                ammo.capacity as usize,
                ammo.rounds as usize,
                DamageType::Explosive,
            ),
        };
        // The reload sweep: pips filled above the steady level while a reload
        // cycle is in flight. Absent/at-rest reload leaves this at `steady_lit`,
        // so the steady lit/dim rendering is byte-identical to before.
        let reload_end = match q_reload.get(**section) {
            Ok(reload) if reload.is_reloading(ammo) => {
                steady_lit + reload_fill_segments(segment_count, steady_lit, reload.progress())
            }
            _ => steady_lit,
        };
        let hue = damage_type_color(damage_type);
        let lit_color = hue.with_alpha(LIT_ALPHA);
        let reload_color = hue.with_alpha(RELOAD_ALPHA);
        let dim_color = hue.with_alpha(DIM_ALPHA);
        for &child in children {
            if let Ok((pip, mut color)) = q_pips.get_mut(child) {
                color.0 = if **pip < steady_lit {
                    lit_color
                } else if **pip < reload_end {
                    reload_color
                } else {
                    dim_color
                };
            }
        }
    }
}

/// Write `rounds/capacity` onto each readout's debug number child and show it
/// while [`AmmoReadoutDebug`] is on. Debug-only: compiled out of release builds
/// so the exact count is never a gameplay affordance.
#[cfg(feature = "debug")]
#[allow(clippy::type_complexity)]
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

#[derive(Default)]
pub struct AmmoReadoutPlugin;

impl Plugin for AmmoReadoutPlugin {
    fn build(&self, app: &mut App) {
        debug!("AmmoReadoutPlugin: build");

        app.register_type::<AmmoReadoutHudMarker>();
        app.register_type::<AmmoReadoutMarker>();
        app.register_type::<AmmoReadoutSection>();
        app.register_type::<AmmoReadoutKind>();
        app.register_type::<AmmoReadoutPip>();

        // Reconcile then light the chunks before the indicator projection
        // places the nodes, mirroring TurretLeadPlugin's slot.
        app.add_systems(
            PostUpdate,
            (sync_ammo_readouts, drive_ammo_readouts)
                .chain()
                .before(ScreenIndicatorSystems),
        );

        // The numeric readout is debug-only (never compiled into release): its
        // resource, F11 toggle and driver all live behind the `debug` feature.
        #[cfg(feature = "debug")]
        {
            app.init_resource::<AmmoReadoutDebug>();
            app.register_type::<AmmoReadoutDebug>();
            app.register_type::<AmmoReadoutNumber>();
            // UNGATED on purpose (task 20260712-173928): this mirrors
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

    #[test]
    fn driver_lights_turret_chunks_by_fraction() {
        let mut world = World::new();
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

    #[test]
    fn driver_lights_one_torpedo_pip_per_remaining_round() {
        let mut world = World::new();
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
        // with EMP reads EMP-colored (differs from Kinetic amber), and a torpedo
        // reads Explosive.
        let mut world = World::new();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(8)));
        world.entity_mut(turret).insert(LoadedBullet {
            kind: DamageType::Emp,
            damage: 5.0,
        });
        let torpedo = spawn_torpedo(&mut world, player, Some(SectionAmmo::new(4)));
        world.run_system_once(sync_ammo_readouts).unwrap();
        world.run_system_once(drive_ammo_readouts).unwrap();

        let turret_lit = first_lit_pip_color(&mut world, turret);
        assert_eq!(
            turret_lit,
            damage_type_color(DamageType::Emp).with_alpha(LIT_ALPHA),
            "an EMP-loaded turret reads in the EMP hue"
        );
        assert_ne!(
            turret_lit,
            damage_type_color(DamageType::Kinetic).with_alpha(LIT_ALPHA),
            "EMP must read differently from the Kinetic amber (the point of color-coding)"
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
        world.init_resource::<AmmoReadoutDebug>(); // true by default
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(DEBUG_TOGGLE_KEY);
        world.insert_resource(input);

        world.run_system_once(toggle_ammo_readout_debug).unwrap();
        assert!(
            !**world.resource::<AmmoReadoutDebug>(),
            "F11 turns the ammo number off"
        );

        // A fresh press flips it back. (A new ButtonInput, not clear()+press():
        // clear() keeps F11 in the `pressed` set, so a re-press would not raise a
        // new just_pressed edge.)
        let mut next = ButtonInput::<KeyCode>::default();
        next.press(DEBUG_TOGGLE_KEY);
        world.insert_resource(next);
        world.run_system_once(toggle_ammo_readout_debug).unwrap();
        assert!(
            **world.resource::<AmmoReadoutDebug>(),
            "a second F11 turns it back on"
        );
    }

    // -- reload sweep --

    #[test]
    fn reload_fill_segments_fills_the_remaining_track_with_progress() {
        // Empty gauge fills whole with progress; clamps to the remaining track.
        assert_eq!(
            reload_fill_segments(8, 0, 0.0),
            0,
            "no progress fills nothing"
        );
        assert_eq!(
            reload_fill_segments(8, 0, 0.5),
            4,
            "half fills half the ring"
        );
        assert_eq!(
            reload_fill_segments(8, 0, 1.0),
            8,
            "full progress fills all"
        );
        // Above a partial steady level, only the gap fills.
        assert_eq!(
            reload_fill_segments(4, 1, 0.5),
            2,
            "fills the 3 remaining by half -> 2 (round)"
        );
        assert_eq!(
            reload_fill_segments(4, 4, 1.0),
            0,
            "a full gauge has nothing to sweep"
        );
        // Progress is clamped, so an overshoot never exceeds the track.
        assert_eq!(reload_fill_segments(8, 0, 5.0), 8);
    }

    /// Count pips rendered in the reload-sweep alpha (between dim and lit).
    fn reload_pip_count(world: &mut World, section: Entity) -> usize {
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
            .filter(|color| (color.0.alpha() - RELOAD_ALPHA).abs() < 1e-3)
            .count()
    }

    /// A reload state seeded from config with the cycle advanced to `progress`.
    fn reload_at(reload_time: f32, only_when_empty: bool, progress: f32) -> SectionReload {
        let mut reload = SectionReload::from_config(SectionReloadConfig {
            reload_time,
            rounds_per_cycle: 1,
            only_when_empty,
        });
        reload.elapsed = reload_time * progress;
        reload
    }

    #[test]
    fn driver_sweeps_the_ring_while_a_turret_reloads() {
        // An empty, discretely-reloading turret shows a reload sweep proportional
        // to cycle progress - no steady-lit rounds, half the ring in reload hue -
        // which is visibly different from a plain empty magazine (nothing lit).
        let mut world = World::new();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(8)));
        world
            .entity_mut(turret)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 0;
        world.entity_mut(turret).insert(reload_at(2.0, true, 0.5));
        world.run_system_once(sync_ammo_readouts).unwrap();
        world.run_system_once(drive_ammo_readouts).unwrap();

        assert_eq!(
            lit_pip_count(&mut world, turret),
            0,
            "no live rounds while empty"
        );
        assert_eq!(
            reload_pip_count(&mut world, turret),
            4,
            "a half-done reload sweeps half the ring"
        );

        // A/B: the same empty turret with NO reload shows nothing - the sweep is
        // what makes reload visible.
        world.entity_mut(turret).remove::<SectionReload>();
        world.run_system_once(drive_ammo_readouts).unwrap();
        assert_eq!(reload_pip_count(&mut world, turret), 0);
        assert_eq!(lit_pip_count(&mut world, turret), 0);
    }

    #[test]
    fn driver_sweeps_the_torpedo_bar_above_the_live_rounds_while_rearming() {
        // A bay with one live torpedo, continuously rearming: the live round stays
        // lit and the sweep lights the rounds coming back above it.
        let mut world = World::new();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let torpedo = spawn_torpedo(&mut world, player, Some(SectionAmmo::new(4)));
        world
            .entity_mut(torpedo)
            .get_mut::<SectionAmmo>()
            .unwrap()
            .rounds = 1;
        world.entity_mut(torpedo).insert(reload_at(4.0, false, 0.5));
        world.run_system_once(sync_ammo_readouts).unwrap();
        world.run_system_once(drive_ammo_readouts).unwrap();

        assert_eq!(
            lit_pip_count(&mut world, torpedo),
            1,
            "the live round stays lit"
        );
        // 3 remaining pips, half swept (round(1.5)) -> 2 in reload hue.
        assert_eq!(
            reload_pip_count(&mut world, torpedo),
            2,
            "the rearming rounds show in the reload hue above the live one"
        );
    }

    #[test]
    fn driver_at_rest_reload_is_identical_to_no_reload() {
        // A full magazine that carries a SectionReload is not reloading, so the
        // gauge is byte-identical to the shipped steady rendering (no regression
        // to loaded-type/count).
        let mut world = World::new();
        world.spawn(ammo_readout_hud());
        let player = spawn_player(&mut world);
        let turret = spawn_turret(&mut world, player, Some(SectionAmmo::new(8)));
        world.entity_mut(turret).insert(reload_at(2.0, true, 0.0));
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
            "a rested reload sweeps nothing"
        );
    }
}
