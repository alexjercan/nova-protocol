//! Production-faithful behavior + geometry rig for The Ledger chapter 3, THE
//! QUIET CHANNEL (task 20260722-214105, deepening the campaign's thinnest
//! chapter). Loads the ACTUAL shipped
//! `webmods/the-ledger/ledger_ch3.content.ron`, registers its real handlers
//! the way the loader does, and drives the gate machine with the same event
//! infos the engine emits - plus a scenario-clock pump for the new
//! clock-gated opening cascade + breathers.
//!
//! What this pins (the ch3-depth deliverables):
//!
//! 1. OnStart seeds the sequencer (open_step/nav_posted/beat_gate + the
//!    per-beat one-shots) and spawns the cast (player, four NAV beacons, the
//!    two invulnerable pinch boulders + the NARROWS trigger, the debris
//!    field), and posts only a HOLDING objective - never the real NAV
//!    objective (the objective-shares-a-frame-with-conversation ban).
//! 2. The opening conversation cascade reaches the first NAV objective only
//!    after the clock is pumped past its thresholds (the deferred-objective
//!    lazy-post; a blind burn cannot skip it).
//! 3. Each gate transition advances via the real OnEnter (NAV-1 -> 2 -> 3 ->
//!    YARD), strictly sequenced by the gate == N guards.
//! 4. The debris pinch: the two straddling boulders are invulnerable and
//!    leave a threadable gap on the NAV-1 -> NAV-2 leg (a computed geometry
//!    pin, mirroring the ch2 cover-corridor style - there IS a gap wider than
//!    the tug).
//! 5. Reaching YARD chains Victory -> ledger_ch4_the_buyer; player death
//!    before arrival retries ledger_ch3_quiet_channel.
//!
//! Harness mirrors ledger_ch2_encounter.rs (mod content stays out of the
//! deep core-CI suite; nova_assets unifies the serde feature so this compiles
//! standalone: `cargo test -p nova_assets --test ledger_ch3_channel`).

use bevy::{ecs::system::RunSystemOnce, math::Vec3, prelude::*};
use bevy_common_systems::prelude::{
    CommandsGameEventExt, EventHandler, GameEventsPlugin, GameObjectives,
};
use nova_events::prelude::{
    OnDestroyedEvent, OnDestroyedEventInfo, OnEnterEvent, OnEnterEventInfo, OnUpdateEvent,
    OnUpdateEventInfo,
};
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

const CH3_RON: &str = include_str!("../../../webmods/the-ledger/ledger_ch3.content.ron");

/// Worst-case body multiplier for an asteroid's nominal radius (the pinch
/// gap must survive the fattest possible boulders).
/// See `ASTEROID_GEOMETRIC_FACTOR_MAX` in nova_scenario.
const ROCK_WORST: f32 = ASTEROID_GEOMETRIC_FACTOR_MAX;
/// The tug's approximate collision half-width (a 3x3-ish cube ship is ~4u
/// across; this is the body that must fit through the pinch gap with room).
const SHIP_BODY: f32 = 5.0;
/// Comfortable clear margin the gap must leave beyond the ship body.
const GAP_MARGIN: f32 = 6.0;

// --- content plumbing (mirrors ledger_ch2_encounter.rs) ---------------------

fn scenario_from(ron_str: &str) -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(ron_str).expect("content RON parses");
    items
        .into_iter()
        .find_map(|c| match c {
            Content::Scenario(s) => Some(s),
            Content::Section(_) => None,
        })
        .expect("content contains a Scenario")
}

fn on_start(scenario: &ScenarioConfig) -> &ScenarioEventConfig {
    scenario
        .events
        .iter()
        .find(|e| matches!(e.name, EventConfig::OnStart))
        .expect("has OnStart")
}

fn spawns(event: &ScenarioEventConfig) -> Vec<&ScenarioObjectConfig> {
    event
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::SpawnScenarioObject(config) => Some(config),
            _ => None,
        })
        .collect()
}

fn spawn_by_id<'a>(event: &'a ScenarioEventConfig, id: &str) -> &'a ScenarioObjectConfig {
    spawns(event)
        .into_iter()
        .find(|s| s.base.id == id)
        .unwrap_or_else(|| panic!("OnStart spawns '{id}'"))
}

fn asteroid_at<'a>(event: &'a ScenarioEventConfig, id: &str) -> (Vec3, &'a AsteroidConfig) {
    let obj = spawn_by_id(event, id);
    match &obj.kind {
        ScenarioObjectKind::Asteroid(rock) => (obj.base.position, rock),
        _ => panic!("'{id}' is an asteroid"),
    }
}

fn seeded_keys(event: &ScenarioEventConfig) -> Vec<&str> {
    event
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::VariableSet(set) => Some(set.key.as_str()),
            _ => None,
        })
        .collect()
}

// --- app harness (mirrors ledger_ch2_encounter.rs) --------------------------

fn slice_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // The debris-pinch beat now carries a real SetSkybox accent (task
    // 20260722-214115); its command reads the AssetServer to start the cubemap
    // load, exactly as in production. Register the asset plumbing so the shipped
    // handler runs to completion in the rig rather than panicking on a missing
    // resource (no scenario camera is present, so the swap no-ops after the
    // load kicks off - which is all this behavior rig needs).
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
    app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
    app.init_resource::<NovaEventWorld>();
    app.init_resource::<GameObjectives>();
    app.init_resource::<CurrentOutcome>();
    app.add_systems(Update, |mut commands: Commands| {
        commands.fire::<OnUpdateEvent>(OnUpdateEventInfo);
    });
    app
}

fn seed_var(app: &mut App, key: &str, value: f64) {
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .insert_variable(key.to_string(), VariableLiteral::Number(value));
}

fn register_non_start_handlers(app: &mut App, scenario: &ScenarioConfig) {
    for event in scenario
        .events
        .iter()
        .filter(|e| !matches!(e.name, EventConfig::OnStart))
    {
        let mut handler = EventHandler::<NovaEventWorld>::from(event.name);
        for filter in event.filters.iter() {
            handler.add_filter(filter.clone());
        }
        for action in event.actions.iter() {
            handler.add_action(action.clone());
        }
        app.world_mut().spawn(handler);
    }
}

fn enter(app: &mut App, area_id: &str, other_id: &str) {
    let info = OnEnterEventInfo {
        id: area_id.to_string(),
        other_id: other_id.to_string(),
        other_type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnEnterEvent>(info.clone());
        })
        .expect("fire OnEnter");
    app.update();
    app.update();
}

fn destroy(app: &mut App, id: &str) {
    let info = OnDestroyedEventInfo {
        id: id.to_string(),
        type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnDestroyedEvent>(info.clone());
        })
        .expect("fire OnDestroyed");
    app.update();
    app.update();
}

/// Pump the scenario clock to a value and tick, so the clock-gated opening
/// cascade + breather handlers actually fire. The loader fires OnStart before
/// the first tick and this rig sets no time, so `scenario_elapsed` reads 0
/// until stamped (the time-gated-content-needs-a-clock-pump lesson, task
/// 20260721-211506).
fn pump_clock(app: &mut App, to_secs: f64) {
    seed_var(app, "scenario_elapsed", to_secs);
    app.update();
    app.update();
}

fn number_var(app: &App, key: &str) -> Option<f64> {
    match app.world().resource::<NovaEventWorld>().get_variable(key) {
        Some(VariableLiteral::Number(n)) => Some(*n),
        _ => None,
    }
}

fn outcome_kind(app: &App) -> Option<ScenarioOutcomeKind> {
    app.world()
        .resource::<CurrentOutcome>()
        .0
        .as_ref()
        .map(|outcome| outcome.outcome)
}

fn queued_next(app: &App) -> Option<(String, bool)> {
    app.world()
        .resource::<NovaEventWorld>()
        .next_scenario
        .as_ref()
        .map(|next| (next.scenario_id.clone(), next.linger))
}

fn has_objective(app: &App, id: &str) -> bool {
    app.world()
        .resource::<GameObjectives>()
        .objectives
        .iter()
        .any(|o| o.id == id)
}

/// A rig with the gate machine + sequencer seeded the way OnStart does; the
/// OnStart wiring itself is pinned structurally in `on_start_seeds_...`
/// (rig-supplies-precondition). Every one-shot guard OnStart seeds to 0 is
/// seeded here too, or the deferred handlers read undefined and fail closed.
fn armed_app() -> App {
    let scenario = scenario_from(CH3_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    for (k, v) in [
        ("act", 1.0),
        ("gate", 1.0),
        ("open_step", 0.0),
        ("nav_posted", 0.0),
        ("beat_gate", 0.0),
        ("arrive1_said", 0.0),
        ("arrive3_said", 0.0),
        ("pinch_gate", 0.0),
        ("pinch_warn_said", 0.0),
        ("pinch_clear_said", 0.0),
        ("scenario_elapsed", 0.0),
    ] {
        seed_var(&mut app, k, v);
    }
    app
}

// --- structural pins --------------------------------------------------------

#[test]
fn on_start_seeds_the_sequencer_and_spawns_the_cast() {
    let scenario = scenario_from(CH3_RON);
    let start = on_start(&scenario);
    let keys = seeded_keys(start);

    for key in [
        "act",
        "gate",
        "open_step",
        "nav_posted",
        "beat_gate",
        "arrive1_said",
        "arrive3_said",
        "pinch_gate",
        "pinch_warn_said",
        "pinch_clear_said",
    ] {
        assert!(
            keys.contains(&key),
            "OnStart must seed '{key}' (an undefined gate fails closed forever)"
        );
    }

    // The cast every walk assumes.
    spawn_by_id(start, "player_spaceship");
    for beacon in ["nav_1", "nav_2", "nav_3", "vesh_yard", "pinch_clear"] {
        spawn_by_id(start, beacon);
    }
    spawn_by_id(start, "pinch_boulder_port");
    spawn_by_id(start, "pinch_boulder_starboard");
}

#[test]
fn on_start_posts_only_the_holding_objective_not_the_nav_goal() {
    // The beat sheet ban: no objective shares a frame with the opening
    // conversation. OnStart posts the stand-by recap and NOTHING else; the
    // real NAV objective lazy-posts only after the cascade hands off.
    let scenario = scenario_from(CH3_RON);
    let start = on_start(&scenario);
    let posted: Vec<&str> = start
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::Objective(o) => Some(o.id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        posted,
        vec!["obj_ch3_recap"],
        "OnStart posts only the holding recap; the NAV goal must lazy-post later"
    );
}

// --- the debris-pinch geometry pin ------------------------------------------

#[test]
fn the_pinch_boulders_are_invulnerable_and_leave_a_threadable_gap() {
    let scenario = scenario_from(CH3_RON);
    let start = on_start(&scenario);

    let (port_pos, port) = asteroid_at(start, "pinch_boulder_port");
    let (star_pos, star) = asteroid_at(start, "pinch_boulder_starboard");

    assert!(
        port.invulnerable && star.invulnerable,
        "the pinch boulders are invulnerable - the hazard is FLOWN, not shot away \
         (preserving the fighting-is-optional contract, no lock will save the player)"
    );

    // The clear gap between the two worst-case boulder bodies must fit the
    // tug with comfortable margin - a careful pilot threads it.
    let centre_gap = port_pos.distance(star_pos);
    let worst_bodies = (port.radius + star.radius) * ROCK_WORST;
    let clear_gap = centre_gap - worst_bodies;
    let need = SHIP_BODY + GAP_MARGIN;
    assert!(
        clear_gap >= need,
        "the pinch gap is only {clear_gap:.1}u clear at the 6x worst case \
         (centres {centre_gap:.1}u apart, bodies {worst_bodies:.1}u); a threadable \
         lane needs >= {need:.1}u (ship {SHIP_BODY}u + {GAP_MARGIN}u margin)"
    );

    // The gap straddles the NAV-1 -> NAV-2 leg: the lane midpoint sits inside
    // the gap, so the pinch is ON the line, not off to the side.
    let nav1 = spawn_by_id(start, "nav_1").base.position;
    let nav2 = spawn_by_id(start, "nav_2").base.position;
    let gap_centre = (port_pos + star_pos) * 0.5;
    let (dist_to_leg, t) = point_to_segment(gap_centre, nav1, nav2);
    assert!(
        dist_to_leg <= 6.0 && t > 0.1 && t < 0.9,
        "the gap centre must sit ON the NAV-1 -> NAV-2 leg (found {dist_to_leg:.1}u off, \
         progress {t:.2}); the pinch is on the lane the player already flies"
    );

    // The NARROWS trigger sits past the gap so the "you're through" confirm
    // fires only after threading it, not before.
    let narrows = spawn_by_id(start, "pinch_clear").base.position;
    assert!(
        narrows.distance(gap_centre) > 22.0,
        "the NARROWS trigger must sit clear of the gap so its confirm fires AFTER \
         the player threads the pinch (found {:.1}u)",
        narrows.distance(gap_centre)
    );
}

/// Distance from `p` to segment `a`->`b`, plus normalized progress of the
/// closest point (mirrors ledger_ch2_encounter.rs).
fn point_to_segment(p: Vec3, a: Vec3, b: Vec3) -> (f32, f32) {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= f32::EPSILON {
        return (p.distance(a), 0.0);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p.distance(a + ab * t), t)
}

// --- behavior walks ---------------------------------------------------------

#[test]
fn the_opening_cascade_lazy_posts_the_nav_objective_after_the_clock_pump() {
    let mut app = armed_app();

    // Before any clock time: the holding recap only, NEVER the NAV goal.
    app.update();
    assert!(
        has_objective(&app, "obj_ch3_recap") || !has_objective(&app, "obj_gates"),
        "before the cascade, the NAV goal must not be posted"
    );
    assert!(
        !has_objective(&app, "obj_gates"),
        "a blind burn cannot start threading before the run is called"
    );
    assert_eq!(number_var(&app, "open_step"), Some(0.0));

    // Walk the clock through every cascade threshold (2, 11, 20, 30s), one
    // pump at a time so each open_step advance actually fires.
    for t in [3.0, 12.0, 21.0, 31.0, 32.0] {
        pump_clock(&mut app, t);
    }
    assert_eq!(
        number_var(&app, "open_step"),
        Some(4.0),
        "the cascade reaches its final step (the hand-off)"
    );
    assert_eq!(
        number_var(&app, "nav_posted"),
        Some(1.0),
        "the hand-off one-shot fired"
    );
    assert!(
        has_objective(&app, "obj_gates"),
        "the first NAV objective lazy-posts once the briefing hands off"
    );
}

#[test]
fn the_gate_machine_advances_in_order_to_the_yard() {
    let mut app = armed_app();

    enter(&mut app, "nav_1", "player_spaceship");
    assert_eq!(
        number_var(&app, "gate"),
        Some(2.0),
        "NAV-1 advances to gate 2"
    );
    // NAV-1 also arms the pinch-warning breather.
    assert!(
        number_var(&app, "pinch_gate").unwrap() > 0.0,
        "NAV-1 stamps the pinch breather gate"
    );

    enter(&mut app, "nav_2", "player_spaceship");
    assert_eq!(
        number_var(&app, "gate"),
        Some(3.0),
        "NAV-2 advances to gate 3"
    );

    enter(&mut app, "nav_3", "player_spaceship");
    assert_eq!(
        number_var(&app, "gate"),
        Some(4.0),
        "NAV-3 advances to gate 4"
    );
    assert_eq!(
        outcome_kind(&app),
        None,
        "reaching NAV-3 does not win; the YARD does"
    );
}

#[test]
fn the_pinch_warning_and_confirm_fire_in_order() {
    let mut app = armed_app();

    // Reach NAV-1: stamps pinch_gate.
    enter(&mut app, "nav_1", "player_spaceship");
    seed_var(&mut app, "scenario_elapsed", 5.0); // NAV-1 stamped pinch_gate ~4s
    let pinch_gate = number_var(&app, "pinch_gate").unwrap();

    // Before the breather elapses, the warning has not fired.
    assert_eq!(number_var(&app, "pinch_warn_said"), Some(0.0));
    // The confirm cannot pre-empt the warning even if the player reaches the
    // NARROWS early.
    enter(&mut app, "pinch_clear", "player_spaceship");
    assert_eq!(
        number_var(&app, "pinch_clear_said"),
        Some(0.0),
        "the far-side confirm is guarded on the warning having played"
    );

    // Pump past the breather: the warning fires.
    pump_clock(&mut app, pinch_gate + 1.0);
    assert_eq!(
        number_var(&app, "pinch_warn_said"),
        Some(1.0),
        "the 'thread it slow' warning lands a beat after NAV-1"
    );

    // Now clearing the NARROWS fires the confirm.
    enter(&mut app, "pinch_clear", "player_spaceship");
    assert_eq!(
        number_var(&app, "pinch_clear_said"),
        Some(1.0),
        "clearing the far side confirms the thread"
    );
}

#[test]
fn reaching_the_yard_wins_into_chapter_four() {
    let mut app = armed_app();

    // March the gate machine to the yard.
    enter(&mut app, "nav_1", "player_spaceship");
    enter(&mut app, "nav_2", "player_spaceship");
    enter(&mut app, "nav_3", "player_spaceship");
    assert_eq!(number_var(&app, "gate"), Some(4.0));
    assert_eq!(outcome_kind(&app), None);

    enter(&mut app, "vesh_yard", "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "reaching the yard wins the chapter"
    );
    assert_eq!(number_var(&app, "act"), Some(2.0), "the win closes the act");
    let (next, linger) = queued_next(&app).expect("victory chains on");
    assert_eq!(next, "ledger_ch4_the_buyer");
    assert!(linger, "the player chooses when to fly into chapter four");
}

#[test]
fn player_death_before_arrival_retries_the_channel() {
    let mut app = armed_app();

    destroy(&mut app, "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Defeat),
        "dying in the channel loses the run"
    );
    let (next, linger) = queued_next(&app).expect("a retry is queued");
    assert_eq!(next, "ledger_ch3_quiet_channel", "retry is THIS chapter");
    assert!(linger);
}

#[test]
fn death_after_the_win_declares_nothing() {
    // A post-victory death must not flip the earned win (the act-gating
    // lesson); the Defeat handler is gated act < 2 and the YARD sets act = 2.
    let scenario = scenario_from(CH3_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 2.0);
    seed_var(&mut app, "gate", 4.0);

    destroy(&mut app, "player_spaceship");
    assert_eq!(outcome_kind(&app), None, "no Defeat over the earned win");
    assert_eq!(queued_next(&app), None, "no retry queued over the win");
}
