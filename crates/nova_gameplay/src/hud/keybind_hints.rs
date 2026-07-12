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

/// The HUD-level cycle row (task 20260711-180501). Not a flight verb, so it
/// carries its own marker, but it obeys the cluster's "no rig, no keys, no
/// hints" rule: blank until the flight rig exists.
#[derive(Component, Debug, Clone, Reflect)]
struct HudLevelHintRow;

/// The verb names, in cluster display order (top to bottom). The two cycle
/// rows document the wheel gestures (task 20260708-165705): plain scroll
/// steps the component fine-lock, CTRL+scroll steps the ship lock through
/// the tracked candidates.
const ROW_VERBS: [&str; 6] = ["STOP", "GOTO", "ORBIT", "CANCEL", "COMPONENT", "TARGET"];

/// Emphasis pulse rate and depth: the emphasized row's color lerps between
/// its availability color and objective gold on this wave. ~1 Hz - present
/// in peripheral vision, not a strobe.
const EMPHASIS_PERIOD_SECS: f32 = 1.0;
const EMPHASIS_LERP_MAX: f32 = 0.85;

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
        children![
            row(0),
            row(1),
            row(2),
            row(3),
            row(4),
            row(5),
            // Discoverability row for the HUD level cycle, driven by
            // update_hint_cluster (blank without a rig). It is chrome
            // itself: at Minimal the whole cluster is hidden, which is
            // exactly the point.
            (
                Name::new("HudLevelHintRow"),
                HudLevelHintRow,
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(DIM_COLOR),
            ),
        ],
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
        4 => &hints.component_cycle,
        _ => &hints.target_cycle,
    }
}

/// `[KEY] VERB` per row: cyan when available, dim when not, empty until
/// the flight rig exists (no rig, no keys, no hints).
fn update_hint_cluster(
    hints: Res<FlightVerbHints>,
    mut q_row: Query<(&KeybindHintRow, &mut Text, &mut TextColor)>,
    q_added: Query<(), Added<KeybindHintRow>>,
    mut q_hud_row: Query<&mut Text, (With<HudLevelHintRow>, Without<KeybindHintRow>)>,
    q_hud_added: Query<(), Added<HudLevelHintRow>>,
) {
    // Skip quiet frames, but never skip freshly spawned rows (a respawned
    // HUD may appear while the resource is unchanged).
    if !hints.is_changed() && q_added.is_empty() && q_hud_added.is_empty() {
        return;
    }
    for (row, mut text, mut color) in &mut q_row {
        let hint = row_hint(&hints, **row);
        if hint.key.is_empty() {
            text.clear();
            continue;
        }
        **text = format!(
            "[{}] {}",
            hint.key,
            ROW_VERBS[(**row).min(ROW_VERBS.len() - 1)]
        );
        **color = if hint.available { NAV_CYAN } else { DIM_COLOR };
    }
    // The HUD-cycle row follows the same no-rig rule; an empty STOP key is
    // the established "rig missing" signal (see cycle_label in
    // input/player.rs).
    let rig_exists = !hints.stop.key.is_empty();
    for mut text in &mut q_hud_row {
        if rig_exists {
            if text.0 != "[`] HUD" {
                **text = "[`] HUD".to_string();
            }
        } else {
            text.clear();
        }
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

/// Pulse the emphasized rows' color toward objective gold (~1 Hz). Runs
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
    let lerp = wave * EMPHASIS_LERP_MAX;

    for (row, mut color) in &mut q_row {
        let hint = row_hint(&hints, **row);
        let verb = ROW_VERBS[(**row).min(ROW_VERBS.len() - 1)];
        // Key-empty rows (no flight rig) have cleared text and never pulse,
        // but they still take the restore below - a row whose key empties
        // MID-pulse (rig despawn) must not keep its gold, and the key
        // emptying is a HINTS change, not an emphasis change (review R1.4).
        let next = if !hint.key.is_empty() && emphasis.contains(verb) {
            base_row_color(hint).mix(&OBJECTIVE_GOLD, lerp)
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
            target_cycle: VerbHint {
                key: "CTRL+SCROLL".into(),
                available: true,
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
    fn cluster_rows_show_labels_and_availability() {
        let mut world = World::new();
        let well = world.spawn_empty().id();
        world.insert_resource(hints(true, false, Some(well)));
        let rows = cluster_rows(&mut world);

        world.run_system_once(update_hint_cluster).unwrap();

        let text = |e: Entity| world.entity(e).get::<Text>().unwrap().0.clone();
        let color = |e: Entity| world.entity(e).get::<TextColor>().unwrap().0;
        assert_eq!(text(rows[0]), "[X] STOP");
        assert_eq!(color(rows[0]), NAV_CYAN, "available verbs light up");
        assert_eq!(text(rows[1]), "[G] GOTO");
        assert_eq!(color(rows[1]), DIM_COLOR, "no lock, GOTO stays dim");
        assert_eq!(text(rows[2]), "[O] ORBIT");
        assert_eq!(text(rows[3]), "[Z] CANCEL");
        assert_eq!(color(rows[3]), DIM_COLOR, "nothing engaged");
        assert_eq!(text(rows[4]), "[SCROLL] COMPONENT");
        assert_eq!(color(rows[4]), DIM_COLOR, "no focus, component cycle dim");
        assert_eq!(text(rows[5]), "[CTRL+SCROLL] TARGET");
        assert_eq!(color(rows[5]), NAV_CYAN, "candidates tracked, cycle lit");
    }

    #[test]
    fn cluster_rows_stay_empty_without_a_flight_rig() {
        let mut world = World::new();
        world.insert_resource(FlightVerbHints::default());
        let rows = cluster_rows(&mut world);

        world.run_system_once(update_hint_cluster).unwrap();

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

    /// The emphasized row leaves its availability color toward gold; the
    /// other rows keep theirs. Emphasis is a spotlight, not a state: the
    /// pulse departs from the row's own base (lit or dim).
    #[test]
    fn emphasized_row_pulses_toward_gold() {
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
        // GOTO is unavailable in this fixture: the pulse departs from DIM.
        assert_ne!(
            color(&world, rows[1]),
            DIM_COLOR,
            "the emphasized row left its base color"
        );
        assert_ne!(
            color(&world, rows[1]),
            OBJECTIVE_GOLD,
            "the pulse lerps toward gold, never fully replacing the row color"
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
