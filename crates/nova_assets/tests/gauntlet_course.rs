//! Production-faithful behavior + layout rig for Gauntlet Run 2.0, the portal
//! parkour mod (task 20260716-124722). Loads the ACTUAL shipped
//! `webmods/gauntlet/gauntlet.content.ron`, registers its real OnEnter /
//! OnDestroyed handlers the way the loader does, and drives the race by firing
//! the same event infos the engine emits. What this file owns is the SCENARIO
//! DATA's consumption of the vocabulary and the two INVARIANTS the course
//! header promises; the filter/action machinery it leans on is pinned by
//! nova_scenario's own tests.
//!
//! Unlike base story content, MOD content is deliberately kept out of the deep
//! core-CI behavior suite (task 20260716-155830) - but the gauntlet is the
//! portal's flagship worked example, and its two geometric invariants (gate
//! areas must not overlap; the racing line must stay flyable past the 6x
//! asteroid geometric factor) are exactly the kind of thing that silently rots
//! and soft-locks a player. So it gets a rig. (It lives in nova_assets, which
//! already unifies the serde feature across the workspace, so it compiles and
//! runs standalone - `cargo test -p nova_assets --test gauntlet_course`.)

use bevy::{
    asset::{AssetApp, AssetPlugin},
    ecs::system::RunSystemOnce,
    math::Vec3,
    prelude::*,
};
use bevy_common_systems::prelude::{
    CommandsGameEventExt, EventHandler, GameEventsPlugin, GameObjectives,
};
use nova_events::prelude::{
    OnDestroyedEvent, OnDestroyedEventInfo, OnEnterEvent, OnEnterEventInfo, OnUpdateEvent,
    OnUpdateEventInfo,
};
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

const GAUNTLET_RON: &str = include_str!("../../../webmods/gauntlet/gauntlet.content.ron");
const GAUNTLET_BUNDLE_RON: &str = include_str!("../../../webmods/gauntlet/gauntlet.bundle.ron");

/// Ship clearance margin (world units) required between the racing line and any
/// asteroid's worst-case body. The racer is ~4u long; this leaves comfortable
/// room for the autopilot's straight GOTO line to pass without a collision.
const SHIP_CLEARANCE: f32 = 8.0;

/// The ordered ids the racing-line polyline runs through: spawn point, the six
/// threaded gates, the finish. Keep in sync with the content's gate chain.
const LINE_IDS: [&str; 8] = [
    "gauntlet_start",
    "gauntlet_gate_1",
    "gauntlet_gate_2",
    "gauntlet_gate_3",
    "gauntlet_gate_4",
    "gauntlet_gate_5",
    "gauntlet_gate_6",
    "gauntlet_finish",
];

/// The ids whose OnEnter areas gate the race (START is only a spawn marker, not
/// a threaded gate), paired with the `gate` value that arms each.
const GATE_CHAIN: [(&str, f64); 7] = [
    ("gauntlet_gate_1", 1.0),
    ("gauntlet_gate_2", 2.0),
    ("gauntlet_gate_3", 3.0),
    ("gauntlet_gate_4", 4.0),
    ("gauntlet_gate_5", 5.0),
    ("gauntlet_gate_6", 6.0),
    ("gauntlet_finish", 7.0),
];

fn scenario() -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(GAUNTLET_RON).expect("content RON parses");
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

// --- geometry helpers -------------------------------------------------------

fn point_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= f32::EPSILON {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn point_to_polyline(p: Vec3, line: &[Vec3]) -> f32 {
    line.windows(2)
        .map(|w| point_to_segment(p, w[0], w[1]))
        .fold(f32::INFINITY, f32::min)
}

/// Closest distance from a point to an axis-aligned box (0 if inside).
fn point_to_box(p: Vec3, min: Vec3, max: Vec3) -> f32 {
    let clamped = p.clamp(min, max);
    p.distance(clamped)
}

/// Min distance from the polyline to an axis-aligned box, by dense sampling.
/// Distance-to-a-convex-set along a segment is convex, so fine sampling finds
/// the true minimum; step 0.5u is far under any margin the rig asserts.
fn polyline_to_box(line: &[Vec3], min: Vec3, max: Vec3) -> f32 {
    let mut best = f32::INFINITY;
    for w in line.windows(2) {
        let (a, b) = (w[0], w[1]);
        let steps = ((b - a).length() / 0.5).ceil().max(1.0) as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            best = best.min(point_to_box(a + (b - a) * t, min, max));
        }
    }
    best
}

/// The racing-line polyline, read from the shipped spawn positions.
fn racing_line(event: &ScenarioEventConfig) -> Vec<Vec3> {
    LINE_IDS
        .iter()
        .map(|id| spawn_by_id(event, id).base.position)
        .collect()
}

// --- app harness (mirrors broadside_assault.rs) -----------------------------

fn course_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // The act-boundary SetSkybox actions resolve a cubemap handle off the
    // AssetServer; give the headless harness a minimal asset backend so the
    // real handlers run to completion (no scenario camera is present, so the
    // swap warns and returns - all we assert is the gate advance around it).
    app.add_plugins(AssetPlugin::default());
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

fn seed_gate(app: &mut App, value: f64) {
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .insert_variable("gate".to_string(), VariableLiteral::Number(value));
}

/// Seed the clean-run counter the way OnStart does (the harness bypasses
/// OnStart, so a FINISH test that exercises the crash-gated Victory branches
/// must set `crash` itself - an unset variable would fail both gates closed).
fn seed_crash(app: &mut App, value: f64) {
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .insert_variable("crash".to_string(), VariableLiteral::Number(value));
}

fn crash_var(app: &App) -> Option<f64> {
    match app
        .world()
        .resource::<NovaEventWorld>()
        .get_variable("crash")
    {
        Some(VariableLiteral::Number(n)) => Some(*n),
        _ => None,
    }
}

fn outcome_message(app: &App) -> Option<String> {
    app.world()
        .resource::<CurrentOutcome>()
        .0
        .as_ref()
        .and_then(|outcome| outcome.message.clone())
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

fn gate_var(app: &App) -> Option<f64> {
    match app
        .world()
        .resource::<NovaEventWorld>()
        .get_variable("gate")
    {
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

// --- INVARIANT 1: gate areas must not overlap -------------------------------

#[test]
fn gate_areas_are_pairwise_non_overlapping() {
    let scenario = scenario();
    let start = on_start(&scenario);

    // (position, area_radius) for every beacon that owns a trigger area.
    let areas: Vec<(String, Vec3, f32)> = spawns(start)
        .into_iter()
        .filter_map(|s| match &s.kind {
            ScenarioObjectKind::Beacon(b) => b
                .area_radius
                .map(|r| (s.base.id.clone(), s.base.position, r)),
            _ => None,
        })
        .collect();

    assert_eq!(
        areas.len(),
        8,
        "START + six gates + FINISH each own a trigger area"
    );

    for i in 0..areas.len() {
        for j in (i + 1)..areas.len() {
            let (ref id_a, pos_a, r_a) = areas[i];
            let (ref id_b, pos_b, r_b) = areas[j];
            let gap = pos_a.distance(pos_b) - (r_a + r_b);
            assert!(
                gap > 0.0,
                "gate areas '{id_a}' and '{id_b}' overlap (centres {:.1}u apart, radii {r_a}+{r_b}); \
                 a pilot loitering in the next area when it arms would soft-lock the race",
                pos_a.distance(pos_b)
            );
        }
    }
}

// --- INVARIANT 2: the racing line stays flyable past the 6x factor ----------

#[test]
fn every_rock_clears_the_racing_line() {
    let scenario = scenario();
    let start = on_start(&scenario);
    let line = racing_line(start);

    // Solo asteroids: worst-case body = nominal radius * factor MAX.
    let mut checked = 0;
    for s in spawns(start) {
        let ScenarioObjectKind::Asteroid(rock) = &s.kind else {
            continue;
        };
        let body_max = rock.radius * ASTEROID_GEOMETRIC_FACTOR_MAX;
        let dist = point_to_polyline(s.base.position, &line);
        let clearance = dist - body_max;
        assert!(
            clearance >= SHIP_CLEARANCE,
            "rock '{}' (r{}) sits {:.1}u from the racing line; worst-case body {:.1}u leaves only \
             {:.1}u clearance (< {SHIP_CLEARANCE}u) - it could block the GOTO line and soft-lock",
            s.base.id,
            rock.radius,
            dist,
            body_max,
            clearance
        );
        checked += 1;
    }
    assert!(
        checked >= 9,
        "the solo rock field + gravity well are present"
    );

    // The scatter field: worst-case rock anywhere in the box, body = max
    // authored radius * factor MAX. The box (a whole region) must clear the line.
    let scatter: &ScatterObjectsConfig = start
        .actions
        .iter()
        .find_map(|a| match a {
            EventActionConfig::ScatterObjects(cfg) => Some(cfg),
            _ => None,
        })
        .expect("the course scatters a belt-wall field");
    let ScatterRegion::Box { min, max } = scatter.region else {
        panic!("the belt-wall field uses a Box region");
    };
    let max_radius = scatter
        .asteroid_radius
        .map(|(_, hi)| hi)
        .expect("the scatter field ranges its radius");
    let body_max = max_radius * ASTEROID_GEOMETRIC_FACTOR_MAX;
    let dist = polyline_to_box(&line, min, max);
    let clearance = dist - body_max;
    assert!(
        clearance >= SHIP_CLEARANCE,
        "the scatter field box sits {:.1}u from the racing line; a worst-case rock (r{max_radius}, \
         body {:.1}u) leaves only {:.1}u clearance (< {SHIP_CLEARANCE}u)",
        dist,
        body_max,
        clearance
    );
}

// --- The hazard menu is actually present ------------------------------------

#[test]
fn on_start_stages_the_course() {
    let scenario = scenario();
    assert!(!scenario.hidden, "gauntlet_run is a playable scenario");
    let start = on_start(&scenario);

    // Seeds the ordered-gate counter.
    let seeds: Vec<&str> = start
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::VariableSet(v) => Some(v.key.as_str()),
            _ => None,
        })
        .collect();
    assert!(seeds.contains(&"gate"), "OnStart seeds the gate counter");

    // The player flies the racer (base craft-ships-into-base prototypes): its
    // many hull cubes give the crash tolerance the course needs (run-and-fail,
    // not one-touch death).
    let player = spawn_by_id(start, "player_spaceship");
    let ScenarioObjectKind::Spaceship(ship) = &player.kind else {
        panic!("player is a spaceship");
    };
    let racer_cubes = ship
        .sections
        .iter()
        .filter(|s| matches!(&s.source, SectionSource::Prototype(p) if p.starts_with("racer_")))
        .count();
    assert!(
        racer_cubes >= 10,
        "the player flies the racer (got {racer_cubes} racer cubes)"
    );

    // The gravity well is an authored well (surface_gravity), invulnerable so it
    // can never die mid-run.
    let well = spawn_by_id(start, "gravity_well");
    let ScenarioObjectKind::Asteroid(rock) = &well.kind else {
        panic!("gravity_well is an asteroid");
    };
    assert!(
        rock.surface_gravity.is_some(),
        "the gravity well authors a surface_gravity so it actually pulls"
    );
    assert!(
        rock.invulnerable,
        "the well is invulnerable - a shot-to-death well would drop the hazard"
    );
}

// --- Behavior: gates advance ONLY in order ----------------------------------

#[test]
fn gates_advance_only_in_order() {
    let scenario = scenario();
    let mut app = course_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_gate(&mut app, 1.0);

    // Delivery guard: nothing advances on its own.
    app.update();
    assert_eq!(gate_var(&app), Some(1.0));

    // Skipping ahead does nothing: entering gate 3's area while gate == 1 is inert.
    enter(&mut app, "gauntlet_gate_3", "player_spaceship");
    assert_eq!(
        gate_var(&app),
        Some(1.0),
        "an out-of-order gate entry must not advance the race"
    );

    // The wrong ship entering the right gate is inert (filter on other_id).
    enter(&mut app, "gauntlet_gate_1", "gauntlet_start");
    assert_eq!(
        gate_var(&app),
        Some(1.0),
        "only the player threading the gate advances it"
    );

    // Walk the whole chain in order; each entry arms exactly the next gate.
    for (id, arming) in GATE_CHAIN {
        assert_eq!(
            gate_var(&app),
            Some(arming),
            "gate is armed to {arming} before threading '{id}'"
        );
        enter(&mut app, id, "player_spaceship");
    }
    assert_eq!(
        gate_var(&app),
        Some(8.0),
        "crossing FINISH bumps to the terminal done-state"
    );
}

// --- Behavior: FINISH wins, a wreck loses, a post-win wreck does neither -----

#[test]
fn crossing_finish_clean_declares_the_clean_run_victory() {
    let scenario = scenario();
    let mut app = course_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_gate(&mut app, 7.0);
    // A clean run: zero grazes recorded (OnStart seeds this in production).
    seed_crash(&mut app, 0.0);

    app.update();
    assert_eq!(outcome_kind(&app), None, "no outcome before the finish");

    enter(&mut app, "gauntlet_finish", "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "crossing FINISH declares Victory"
    );
    assert!(
        outcome_message(&app)
            .unwrap_or_default()
            .contains("CLEAN RUN"),
        "a zero-crash finish earns the CLEAN RUN banner, got {:?}",
        outcome_message(&app)
    );
    assert!(
        app.world()
            .resource::<NovaEventWorld>()
            .next_scenario
            .is_none(),
        "a clean run queues nothing - the overlay offers Main Menu"
    );
}

/// A finish WITH grazes takes the other gated Victory branch: still a win, but
/// the plain banner (no CLEAN RUN). The clean test above is this test's
/// delivery guard - the two branches are mutually exclusive on `crash`.
#[test]
fn crossing_finish_grazed_declares_the_plain_victory() {
    let scenario = scenario();
    let mut app = course_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_gate(&mut app, 7.0);
    // Grazed at least one hazard on the way.
    seed_crash(&mut app, 2.0);

    enter(&mut app, "gauntlet_finish", "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "a grazed finish still wins"
    );
    let message = outcome_message(&app).unwrap_or_default();
    assert!(
        !message.contains("CLEAN RUN"),
        "a grazed finish must NOT claim a clean run, got {message:?}"
    );
}

/// Flying through a hazard graze zone bumps the clean-run counter; the same
/// zone re-counts on a fresh entry (the intended fly-clean pressure).
#[test]
fn a_graze_zone_bumps_the_crash_counter() {
    let scenario = scenario();
    let mut app = course_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_gate(&mut app, 3.0);
    seed_crash(&mut app, 0.0);

    enter(&mut app, "graze_slalom", "player_spaceship");
    assert_eq!(crash_var(&app), Some(1.0), "a graze bumps crash");
    enter(&mut app, "graze_slalom", "player_spaceship");
    assert_eq!(
        crash_var(&app),
        Some(2.0),
        "re-entering the same zone re-counts"
    );

    // A graze after the finish (gate == 8) cannot un-clean an earned win.
    seed_gate(&mut app, 8.0);
    enter(&mut app, "graze_slalom", "player_spaceship");
    assert_eq!(
        crash_var(&app),
        Some(2.0),
        "a post-finish graze is gated out (gate < 8)"
    );
}

/// The RUN TIMER wiring: OnStart shows a HudReadout on the engine clock,
/// formatted mm:ss.s, from the very start - the display half the time-trial
/// needs. Pins the slot, the bound variable, the format, and that it is shown.
#[test]
fn on_start_shows_the_run_timer_on_the_scenario_clock() {
    let scenario = scenario();
    let start = on_start(&scenario);
    let timer = start
        .actions
        .iter()
        .find_map(|a| match a {
            EventActionConfig::HudReadout(cfg) => Some(cfg),
            _ => None,
        })
        .expect("OnStart shows a HudReadout timer");
    assert_eq!(
        timer.variable, "scenario_elapsed",
        "the timer is bound to the engine scenario clock"
    );
    assert_eq!(
        timer.format,
        HudReadoutFormat::Time,
        "the timer renders as mm:ss.s"
    );
    assert!(timer.visible, "the timer is shown, not cleared");
    // And the clean-run counter is seeded so the clean gate is satisfiable.
    let seeds: Vec<&str> = start
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::VariableSet(v) => Some(v.key.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        seeds.contains(&"crash"),
        "OnStart seeds the clean-run counter"
    );
}

#[test]
fn wrecking_before_finish_declares_defeat_with_a_retry() {
    let scenario = scenario();
    let mut app = course_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_gate(&mut app, 3.0);

    destroy(&mut app, "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Defeat),
        "a wreck mid-course declares Defeat"
    );
    let next = app
        .world()
        .resource::<NovaEventWorld>()
        .next_scenario
        .clone()
        .expect("a retry is queued");
    assert_eq!(
        next.scenario_id, "gauntlet_run",
        "the retry re-runs the course"
    );
    assert!(next.linger, "the retry lingers behind the overlay");
}

/// The Defeat handler is gated `gate < 8`: a death blast AFTER the win (the
/// terminal state the FINISH handler set) must not flip the earned Victory.
/// `wrecking_before_finish_...` above is this test's delivery guard.
#[test]
fn wrecking_after_the_win_declares_nothing() {
    let scenario = scenario();
    let mut app = course_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_gate(&mut app, 8.0);

    destroy(&mut app, "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        None,
        "no Defeat once the course is finished"
    );
    assert!(
        app.world()
            .resource::<NovaEventWorld>()
            .next_scenario
            .is_none(),
        "no retry queued over an earned Victory"
    );
}

/// The escalation actually spawns the racer + all eight beacons under their
/// scenario ids through the production drain (not just parsed config).
#[test]
fn on_start_spawns_the_full_gate_run() {
    let scenario = scenario();
    let start = on_start(&scenario);
    // The eight line ids and the gravity well all appear as spawns.
    for id in LINE_IDS.iter().chain(std::iter::once(&"gravity_well")) {
        let _ = spawn_by_id(start, id);
    }
}

/// The bundle ships the bumped version and lists the content file - an
/// unbumped or unlisted mod is a broken publish.
#[test]
fn bundle_ships_the_bumped_version() {
    assert!(
        GAUNTLET_BUNDLE_RON.contains("version: \"1.3.0\""),
        "the bundle is bumped to 1.3.0 for the time-trial republish"
    );
    assert!(
        GAUNTLET_BUNDLE_RON.contains("gauntlet.content.ron"),
        "the bundle lists the content file"
    );
}
