//! Contextual keybind hints (task 20260710-174646, spike
//! docs/spikes/20260710-174523-diegetic-instruments-keybind-hints.md):
//! nobody memorizes X/G/O/Z. Everything renders from the input layer's
//! [`FlightVerbHints`] resource - availability and key labels are resolved
//! where the verbs live, this module is a dumb view.
//!
//! - **Hint cluster**: a small column docked in the lower-left corner,
//!   one `[KEY] VERB` row per flight verb, nav-cyan when pressing it would
//!   do something, dimmed when not.
//! - **Anchored cues**: the hint sits on the thing you would act on -
//!   `[O] ORBIT` projected on the dominant well (absorbed from the
//!   hand-placed v1 cue in flight_status.rs), `[G] GOTO` on the aim lock
//!   while no maneuver is engaged.

use bevy::{platform::collections::HashSet, prelude::*};

use super::{screen_indicator::prelude::*, NAV_CYAN, OBJECTIVE_GOLD};
use crate::input::prelude::*;

pub mod prelude {
    pub use super::{
        keybind_hint_cluster_hud, verb_cues_hud, HintEmphasis, KeybindHintClusterMarker,
        KeybindHintsPlugin, VerbCuesHudMarker,
    };
}

/// Dimmed row color for a verb that is present but not currently available.
const DIM_COLOR: Color = Color::srgba(0.5, 0.55, 0.6, 0.5);

/// On-screen size of an anchored cue chip (px).
const CUE_SIZE: Vec2 = Vec2::new(96.0, 16.0);

/// The cue sits below its object so it reads as a caption, not a lock.
const CUE_OFFSET: Vec2 = Vec2::new(0.0, 48.0);

#[derive(Component, Debug, Clone, Reflect)]
pub struct KeybindHintClusterMarker;

/// A row of the cluster; the index picks the verb (see [`row_hint`]).
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
struct KeybindHintRow(usize);

/// The verb names, in cluster display order (top to bottom). The two cycle
/// rows document the wheel gestures (task 20260708-165705): plain scroll
/// steps the component fine-lock, CTRL+scroll steps the ship lock through
/// the tracked candidates.
const ROW_VERBS: [&str; 7] = [
    "STOP",
    "GOTO",
    "ORBIT",
    "CANCEL",
    "RADAR",
    "COMPONENT",
    "RCS",
];

/// Emphasis pulse rate and the alpha bands it sweeps. The emphasized row
/// renders PURE OBJECTIVE_GOLD hue at all times and only its alpha
/// pulses: the first cut cross-mixed the availability color toward gold,
/// and a lit row's cyan->gold RGB path passes through a washed
/// near-white blend every cycle - unreadable at 12 px (playtest
/// 2026-07-12, task 20260712-152340). Availability still reads from the
/// band: available rows sweep the bright band, unavailable rows a dim one
/// (spotlight, not a state change). ~1 Hz - present in peripheral vision,
/// not a strobe.
const EMPHASIS_PERIOD_SECS: f32 = 1.0;
const EMPHASIS_ALPHA_AVAILABLE: (f32, f32) = (0.7, 1.0);
const EMPHASIS_ALPHA_UNAVAILABLE: (f32, f32) = (0.3, 0.5);

/// The verb rows the scenario wants the player's eyes on (task
/// 20260712-093831): names from [`ROW_VERBS`], set/cleared by the
/// `HintEmphasisSet`/`HintEmphasisClear` scenario actions and cleared
/// wholesale on scenario teardown. Emphasis is a SPOTLIGHT, not a state
/// change - it never alters availability; an unavailable row pulses from
/// dim, still clearly "not yet".
#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource)]
pub struct HintEmphasis {
    verbs: HashSet<String>,
}

impl HintEmphasis {
    /// Emphasize `verb` (a [`ROW_VERBS`] name). Unknown verbs are refused
    /// with a warning - a typo in scenario data should be loud, not a
    /// silently dead handler.
    pub fn set(&mut self, verb: &str) -> bool {
        if !ROW_VERBS.contains(&verb) {
            warn!(
                "HintEmphasis: '{}' is not a cluster verb (rows: {:?})",
                verb, ROW_VERBS
            );
            return false;
        }
        self.verbs.insert(verb.to_string());
        true
    }

    /// Drop the emphasis on `verb` (no-op when it was not emphasized).
    pub fn clear(&mut self, verb: &str) {
        self.verbs.remove(verb);
    }

    /// Drop every emphasis (scenario teardown).
    pub fn clear_all(&mut self) {
        self.verbs.clear();
    }

    /// Whether `verb` is currently emphasized.
    pub fn contains(&self, verb: &str) -> bool {
        self.verbs.contains(verb)
    }

    /// Whether nothing is emphasized (the pulse system's early-out).
    pub fn is_empty(&self) -> bool {
        self.verbs.is_empty()
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct VerbCuesHudMarker;

/// Marker for the orbit cue chip (anchored on the dominant well).
#[derive(Component, Debug, Clone, Reflect)]
struct OrbitCueUIMarker;

/// Marker for the goto cue chip (anchored on the aim lock).
#[derive(Component, Debug, Clone, Reflect)]
struct GotoCueUIMarker;

/// UI bundle for the hint cluster: a fixed column in the lower-left
/// corner (the flight status that used to sit under it moved onto the
/// ship as chips, task 20260710-231926).
pub fn keybind_hint_cluster_hud() -> impl Bundle {
    let row = |index: usize| {
        (
            KeybindHintRow(index),
            Text::new(""),
            TextFont::from_font_size(12.0),
            TextColor(DIM_COLOR),
        )
    };

    (
        Name::new("KeybindHintClusterHUD"),
        KeybindHintClusterMarker,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        },
        Pickable::IGNORE,
        children![row(0), row(1), row(2), row(3), row(4), row(5), row(6),],
    )
}

/// UI bundle for the anchored cues: one indicator layer with the orbit and
/// goto chips.
pub fn verb_cues_hud() -> impl Bundle {
    let cue = || {
        screen_indicator(ScreenIndicatorConfig {
            anchor: None,
            size: ScreenIndicatorSize::Fixed(CUE_SIZE),
            offset: CUE_OFFSET,
            offscreen: ScreenIndicatorOffscreen::Hide,
        })
    };

    (
        Name::new("VerbCuesHUD"),
        VerbCuesHudMarker,
        screen_indicator_layer(),
        children![
            (
                Name::new("OrbitCueUI"),
                OrbitCueUIMarker,
                cue(),
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(NAV_CYAN),
            ),
            (
                Name::new("GotoCueUI"),
                GotoCueUIMarker,
                cue(),
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(NAV_CYAN),
            ),
        ],
    )
}

#[derive(Default)]
pub struct KeybindHintsPlugin;

impl Plugin for KeybindHintsPlugin {
    fn build(&self, app: &mut App) {
        debug!("KeybindHintsPlugin: build");

        app.init_resource::<HintEmphasis>();
        app.register_type::<HintEmphasis>();

        app.add_systems(
            Update,
            (
                update_hint_cluster,
                // The pulse layers OVER the availability coloring, so it
                // must run downstream of the writer inside the frame.
                pulse_emphasized_rows.after(update_hint_cluster),
                (drive_orbit_cue, drive_goto_cue),
            )
                .in_set(super::NovaHudSystems),
        );
    }
}

fn row_hint(hints: &FlightVerbHints, index: usize) -> &VerbHint {
    match index {
        0 => &hints.stop,
        1 => &hints.goto,
        2 => &hints.orbit,
        3 => &hints.cancel,
        4 => &hints.radar,
        5 => &hints.component_cycle,
        _ => &hints.rcs,
    }
}

/// `[KEY] VERB` per row - CONTEXTUAL (playtest 2026-07-13, Arma-style): a
/// row renders only while its verb is actionable; a verb that cannot do
/// anything right now is not shown at all (the old grey rows read as
/// noise). The one exception is an EMPHASIZED verb: the tutorial spotlight
/// must be able to point at a key just before it becomes actionable, so an
/// emphasized row shows (and pulses) even while unavailable. No rig, no
/// keys, no rows, as always.
fn update_hint_cluster(
    hints: Res<FlightVerbHints>,
    emphasis: Res<HintEmphasis>,
    mut q_row: Query<(&KeybindHintRow, &mut Text, &mut TextColor, &mut Node)>,
    q_added: Query<(), Added<KeybindHintRow>>,
) {
    // Skip quiet frames, but never skip freshly spawned rows (a respawned
    // HUD may appear while the resource is unchanged).
    if !hints.is_changed() && !emphasis.is_changed() && q_added.is_empty() {
        return;
    }
    for (row, mut text, mut color, mut node) in &mut q_row {
        let hint = row_hint(&hints, **row);
        let verb = ROW_VERBS[(**row).min(ROW_VERBS.len() - 1)];
        let shown = !hint.key.is_empty() && (hint.available || emphasis.contains(verb));
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        if !shown {
            text.clear();
            continue;
        }
        **text = format!("[{}] {}", hint.key, verb);
        **color = if hint.available { NAV_CYAN } else { DIM_COLOR };
    }
}

/// The availability color [`update_hint_cluster`] gives a row - the base
/// the emphasis pulse departs from and returns to.
fn base_row_color(hint: &VerbHint) -> Color {
    if hint.available {
        NAV_CYAN
    } else {
        DIM_COLOR
    }
}

/// The emphasized row's color at `wave` in [0,1]: pure gold hue, alpha
/// swept over the availability band - never a cross-hue blend (see the
/// EMPHASIS_* docs).
fn emphasis_color(available: bool, wave: f32) -> Color {
    let (lo, hi) = if available {
        EMPHASIS_ALPHA_AVAILABLE
    } else {
        EMPHASIS_ALPHA_UNAVAILABLE
    };
    OBJECTIVE_GOLD.with_alpha(lo + (hi - lo) * wave)
}

/// Pulse the emphasized rows in objective gold (~1 Hz alpha breath). Runs
/// after [`update_hint_cluster`] so the availability coloring stays the
/// base; on any emphasis change every row's base is restored first, so a
/// cleared emphasis cannot leave a row stuck mid-pulse.
fn pulse_emphasized_rows(
    time: Res<Time>,
    emphasis: Res<HintEmphasis>,
    hints: Res<FlightVerbHints>,
    mut q_row: Query<(&KeybindHintRow, &mut TextColor)>,
) {
    if emphasis.is_empty() && !emphasis.is_changed() {
        return;
    }

    let t = time.elapsed_secs() * std::f32::consts::TAU / EMPHASIS_PERIOD_SECS;
    let wave = 0.5 + 0.5 * t.sin();

    for (row, mut color) in &mut q_row {
        let hint = row_hint(&hints, **row);
        let verb = ROW_VERBS[(**row).min(ROW_VERBS.len() - 1)];
        // Key-empty rows (no flight rig) have cleared text and never pulse,
        // but they still take the restore below - a row whose key empties
        // MID-pulse (rig despawn) must not keep its gold, and the key
        // emptying is a HINTS change, not an emphasis change (review R1.4).
        let next = if !hint.key.is_empty() && emphasis.contains(verb) {
            emphasis_color(hint.available, wave)
        } else if emphasis.is_changed() || hints.is_changed() {
            // Restore the base exactly once per change; steady state
            // leaves unemphasized rows to update_hint_cluster (whose own
            // write is identical, so the diffed write below is a no-op).
            base_row_color(hint)
        } else {
            continue;
        };
        if color.0 != next {
            color.0 = next;
        }
    }
}

/// `[O] ORBIT` on the dominant well while parking is on offer (the
/// resolver already retires the offer while orbiting).
fn drive_orbit_cue(
    hints: Res<FlightVerbHints>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Text), With<OrbitCueUIMarker>>,
    q_added: Query<(), Added<OrbitCueUIMarker>>,
) {
    // Same guard shape as the cluster: the resource is the only input, so
    // quiet frames write nothing (an unconditional Text deref would
    // re-layout the chip every frame).
    if !hints.is_changed() && q_added.is_empty() {
        return;
    }
    for (mut anchor, mut text) in &mut q_ui {
        match (hints.orbit.available, hints.orbit.anchor) {
            (true, Some(well)) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(well));
                **text = format!("[{}] ORBIT", hints.orbit.key);
            }
            _ => {
                **anchor = None;
                text.clear();
            }
        }
    }
}

/// `[G] GOTO` on the aim lock while nothing is engaged - once a maneuver
/// runs the destination marker takes over and the cue would be noise.
fn drive_goto_cue(
    hints: Res<FlightVerbHints>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Text), With<GotoCueUIMarker>>,
    q_added: Query<(), Added<GotoCueUIMarker>>,
) {
    if !hints.is_changed() && q_added.is_empty() {
        return;
    }
    for (mut anchor, mut text) in &mut q_ui {
        match (hints.goto.available && !hints.engaged, hints.goto.anchor) {
            (true, Some(lock)) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(lock));
                **text = format!("[{}] GOTO", hints.goto.key);
            }
            _ => {
                **anchor = None;
                text.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn hints(orbit_available: bool, engaged: bool, well: Option<Entity>) -> FlightVerbHints {
        FlightVerbHints {
            stop: VerbHint {
                key: "X".into(),
                available: true,
                anchor: None,
            },
            goto: VerbHint {
                key: "G".into(),
                available: false,
                anchor: None,
            },
            orbit: VerbHint {
                key: "O".into(),
                available: orbit_available,
                anchor: well,
            },
            cancel: VerbHint {
                key: "Z".into(),
                available: engaged,
                anchor: None,
            },
            component_cycle: VerbHint {
                key: "SCROLL".into(),
                available: false,
                anchor: None,
            },
            radar: VerbHint {
                key: "CTRL".into(),
                available: true,
                anchor: None,
            },
            rcs: VerbHint {
                key: "SHIFT".into(),
                available: false,
                anchor: None,
            },
            engaged,
        }
    }

    fn cluster_rows(world: &mut World) -> Vec<Entity> {
        let cluster = world.spawn(keybind_hint_cluster_hud()).id();
        world.entity(cluster).get::<Children>().unwrap().to_vec()
    }

    #[test]
    fn only_actionable_rows_show_and_emphasis_overrides() {
        // Contextual cluster (playtest 2026-07-13, Arma-style): unavailable
        // verbs are NOT rendered - no grey noise; an emphasized verb is the
        // exception (the tutorial spotlight may precede availability).
        let mut world = World::new();
        world.init_resource::<HintEmphasis>();
        let well = world.spawn_empty().id();
        world.insert_resource(hints(true, false, Some(well)));
        let rows = cluster_rows(&mut world);

        world.run_system_once(update_hint_cluster).unwrap();

        let text = |w: &World, e: Entity| w.entity(e).get::<Text>().unwrap().0.clone();
        let color = |w: &World, e: Entity| w.entity(e).get::<TextColor>().unwrap().0;
        let display = |w: &World, e: Entity| w.entity(e).get::<Node>().unwrap().display;
        // Available: shown, cyan.
        assert_eq!(text(&world, rows[0]), "[X] STOP");
        assert_eq!(display(&world, rows[0]), Display::Flex);
        assert_eq!(color(&world, rows[0]), NAV_CYAN, "available verbs light up");
        assert_eq!(text(&world, rows[2]), "[O] ORBIT");
        assert_eq!(display(&world, rows[2]), Display::Flex);
        assert_eq!(text(&world, rows[4]), "[CTRL] RADAR");
        assert_eq!(color(&world, rows[4]), NAV_CYAN, "the computer grants Lock");
        // Unavailable: gone entirely, not greyed.
        for (index, name) in [(1, "GOTO"), (3, "CANCEL"), (5, "COMPONENT")] {
            assert_eq!(
                display(&world, rows[index]),
                Display::None,
                "unavailable {name} is not rendered"
            );
            assert_eq!(text(&world, rows[index]), "", "and carries no text");
        }

        // Emphasize the unavailable GOTO: the row comes back (dim base;
        // the gold pulse rides on top) so the tutorial can point at it.
        world.resource_mut::<HintEmphasis>().set("GOTO");
        world.run_system_once(update_hint_cluster).unwrap();
        assert_eq!(
            display(&world, rows[1]),
            Display::Flex,
            "an emphasized verb shows even while unavailable"
        );
        assert_eq!(text(&world, rows[1]), "[G] GOTO");
        assert_eq!(
            color(&world, rows[1]),
            DIM_COLOR,
            "base stays dim; gold is the pulse's"
        );

        // Clearing the emphasis hides it again (delivery guard).
        world.resource_mut::<HintEmphasis>().clear("GOTO");
        world.run_system_once(update_hint_cluster).unwrap();
        assert_eq!(display(&world, rows[1]), Display::None);
    }

    #[test]
    fn cluster_rows_stay_empty_without_a_flight_rig() {
        let mut world = World::new();
        world.init_resource::<HintEmphasis>();
        world.insert_resource(FlightVerbHints::default());
        let rows = cluster_rows(&mut world);

        world.run_system_once(update_hint_cluster).unwrap();

        for &row in &rows {
            assert_eq!(
                world.entity(row).get::<Node>().unwrap().display,
                Display::None,
                "no rig, no rows"
            );
        }

        for row in rows {
            assert!(
                world.entity(row).get::<Text>().unwrap().0.is_empty(),
                "no rig, no keys, no hint rows"
            );
        }
    }

    fn spawn_cues(world: &mut World) -> (Entity, Entity) {
        let layer = world.spawn(verb_cues_hud()).id();
        let children = world.entity(layer).get::<Children>().unwrap();
        (children[0], children[1])
    }

    fn anchor_of(world: &World, entity: Entity) -> Option<ScreenIndicatorAnchorKind> {
        **world.entity(entity).get::<ScreenIndicatorAnchor>().unwrap()
    }

    #[test]
    fn orbit_cue_follows_the_resolvers_offer() {
        let mut world = World::new();
        let well = world.spawn_empty().id();
        world.insert_resource(hints(true, false, Some(well)));
        let (orbit_cue, _) = spawn_cues(&mut world);

        world.run_system_once(drive_orbit_cue).unwrap();
        assert_eq!(
            anchor_of(&world, orbit_cue),
            Some(ScreenIndicatorAnchorKind::Entity(well))
        );
        assert_eq!(
            world.entity(orbit_cue).get::<Text>().unwrap().0,
            "[O] ORBIT"
        );

        // Orbiting (the resolver retires the offer): the cue hides.
        world.insert_resource(hints(false, true, Some(well)));
        world.run_system_once(drive_orbit_cue).unwrap();
        assert_eq!(anchor_of(&world, orbit_cue), None);
    }

    /// Only cluster verbs are addressable - a scenario typo is refused
    /// loudly instead of becoming a silently dead emphasis.
    #[test]
    fn emphasis_rejects_non_cluster_verbs() {
        let mut emphasis = HintEmphasis::default();
        assert!(!emphasis.set("ALT"), "ALT is not a cluster row");
        assert!(!emphasis.contains("ALT"));
        assert!(emphasis.set("GOTO"));
        assert!(emphasis.contains("GOTO"));
    }

    /// The emphasized row renders PURE gold hue at every point of the
    /// wave - only its alpha moves; a cross-hue mix (lit cyan -> gold)
    /// passes through a washed near-white blend that killed readability
    /// in playtest (task 20260712-152340). Availability reads from the
    /// alpha band: an unavailable row pulses in the dim band, below the
    /// available band (spotlight, not a state change). Unemphasized rows
    /// keep their availability color.
    #[test]
    fn emphasized_rows_pulse_pure_gold_alpha_only() {
        let gold = OBJECTIVE_GOLD.to_srgba();
        // The invariant across the whole wave, not just one sample.
        for i in 0..=10 {
            let wave = i as f32 / 10.0;
            for available in [true, false] {
                let sample = emphasis_color(available, wave).to_srgba();
                assert_eq!(
                    (sample.red, sample.green, sample.blue),
                    (gold.red, gold.green, gold.blue),
                    "wave {wave}: hue must stay gold, never a white-ish blend"
                );
            }
            let bright = emphasis_color(true, wave).to_srgba().alpha;
            let dim = emphasis_color(false, wave).to_srgba().alpha;
            assert!(
                dim < bright,
                "wave {wave}: unavailable stays below available ({dim} vs {bright})"
            );
        }
        let (lo, hi) = EMPHASIS_ALPHA_AVAILABLE;
        assert!(
            emphasis_color(true, 1.0).to_srgba().alpha - emphasis_color(true, 0.0).to_srgba().alpha
                >= 0.9 * (hi - lo),
            "the alpha actually sweeps its band (a flat pulse is dead code)"
        );

        // And through the real system: the emphasized (unavailable) GOTO
        // row carries gold hue, the unemphasized STOP row keeps cyan.
        let mut world = World::new();
        world.init_resource::<Time>();
        world.insert_resource(hints(true, false, None));
        let mut emphasis = HintEmphasis::default();
        emphasis.set("GOTO");
        world.insert_resource(emphasis);
        let rows = cluster_rows(&mut world);

        world.run_system_once(update_hint_cluster).unwrap();
        world.run_system_once(pulse_emphasized_rows).unwrap();

        let color = |world: &World, e: Entity| world.entity(e).get::<TextColor>().unwrap().0;
        let goto = color(&world, rows[1]).to_srgba();
        assert_eq!(
            (goto.red, goto.green, goto.blue),
            (gold.red, gold.green, gold.blue),
            "the emphasized row is gold-hued on screen"
        );
        assert_eq!(
            color(&world, rows[0]),
            NAV_CYAN,
            "unemphasized rows keep their availability color"
        );
    }

    /// Clearing the emphasis restores the availability color - a cleared
    /// row must not stay stuck mid-pulse.
    #[test]
    fn cleared_emphasis_restores_the_base_color() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.insert_resource(hints(true, false, None));
        let mut emphasis = HintEmphasis::default();
        emphasis.set("STOP");
        world.insert_resource(emphasis);
        let rows = cluster_rows(&mut world);

        world.run_system_once(update_hint_cluster).unwrap();
        world.run_system_once(pulse_emphasized_rows).unwrap();
        let color = |world: &World, e: Entity| world.entity(e).get::<TextColor>().unwrap().0;
        assert_ne!(color(&world, rows[0]), NAV_CYAN, "delivery guard: pulsing");

        world.resource_mut::<HintEmphasis>().clear("STOP");
        world.run_system_once(pulse_emphasized_rows).unwrap();
        assert_eq!(
            color(&world, rows[0]),
            NAV_CYAN,
            "the cleared row returns to its availability color"
        );
    }

    /// The change-detection gates across REAL frames (run_system_once
    /// makes Res::is_changed always true, so the run_system_once tests
    /// above cannot exercise them - review R1.3): pulsing runs every
    /// frame while emphasized, the clear restores the base on the next
    /// frame, and the restored color then STAYS at base over further
    /// quiet frames.
    #[test]
    fn emphasis_gates_behave_across_real_frames() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(hints(true, false, None));
        app.init_resource::<HintEmphasis>();
        app.add_systems(
            Update,
            (
                update_hint_cluster,
                pulse_emphasized_rows.after(update_hint_cluster),
            ),
        );
        let cluster = app.world_mut().spawn(keybind_hint_cluster_hud()).id();
        let stop_row = app.world().entity(cluster).get::<Children>().unwrap()[0];
        let color = |app: &App| app.world().entity(stop_row).get::<TextColor>().unwrap().0;

        app.world_mut().resource_mut::<HintEmphasis>().set("STOP");
        app.update();
        assert_ne!(color(&app), NAV_CYAN, "delivery guard: the row pulses");

        app.world_mut().resource_mut::<HintEmphasis>().clear("STOP");
        app.update();
        assert_eq!(color(&app), NAV_CYAN, "the clear restores the base");

        // Quiet frames: nothing rewrites the row, and it stays at base
        // (a regressed gate would resume pulsing or stick a stale color).
        app.update();
        app.update();
        assert_eq!(color(&app), NAV_CYAN, "steady state holds at base");
    }

    /// A rig despawn (key empties) while a row is mid-pulse must restore
    /// the base color even though the EMPHASIS never changed - the key
    /// emptying is a hints change (review R1.4).
    #[test]
    fn rig_despawn_mid_pulse_restores_the_base_color() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(hints(true, false, None));
        app.init_resource::<HintEmphasis>();
        app.add_systems(
            Update,
            (
                update_hint_cluster,
                pulse_emphasized_rows.after(update_hint_cluster),
            ),
        );
        let cluster = app.world_mut().spawn(keybind_hint_cluster_hud()).id();
        let stop_row = app.world().entity(cluster).get::<Children>().unwrap()[0];
        let color = |app: &App| app.world().entity(stop_row).get::<TextColor>().unwrap().0;

        app.world_mut().resource_mut::<HintEmphasis>().set("STOP");
        app.update();
        assert_ne!(color(&app), NAV_CYAN, "delivery guard: the row pulses");

        // The rig despawns: every key label empties, emphasis untouched.
        app.insert_resource(FlightVerbHints::default());
        app.update();
        assert_eq!(
            color(&app),
            DIM_COLOR,
            "the keyless row returns to its base instead of freezing gold"
        );
    }

    #[test]
    fn goto_cue_shows_on_the_lock_and_hides_while_engaged() {
        let mut world = World::new();
        let lock = world.spawn_empty().id();
        let mut resource = hints(false, false, None);
        resource.goto = VerbHint {
            key: "G".into(),
            available: true,
            anchor: Some(lock),
        };
        world.insert_resource(resource.clone());
        let (_, goto_cue) = spawn_cues(&mut world);

        world.run_system_once(drive_goto_cue).unwrap();
        assert_eq!(
            anchor_of(&world, goto_cue),
            Some(ScreenIndicatorAnchorKind::Entity(lock))
        );
        assert_eq!(world.entity(goto_cue).get::<Text>().unwrap().0, "[G] GOTO");

        // A maneuver engages: the destination marker takes over.
        resource.cancel.available = true;
        resource.engaged = true;
        world.insert_resource(resource);
        world.run_system_once(drive_goto_cue).unwrap();
        assert_eq!(anchor_of(&world, goto_cue), None);
    }
}
