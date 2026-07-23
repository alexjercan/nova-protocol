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
//! 5. The stealth rework (task 20260723-000320): the two channel Magpies
//!    spawn at OnStart as NEUTRAL patrols (no engage_delay) flanking the
//!    pinch; the picket-watch OnEnter zones and the per-Magpie OnCombatLock
//!    paint both fire SetAllegiance -> Enemy on BOTH ships (asserted on the
//!    live Allegiance COMPONENT, spawned through the real spawn action and
//!    flushed by the production state_to_world sync) and stamp `spotted`.
//! 5b. The overspeed provocation (task 20260723-143603): burning over 8 u/s
//!    is the fifth wake path, warn-then-trip on the reserved `player_speed`
//!    readout - the first breach only WARNS (pickets stay Neutral), and
//!    tripping needs a FRESH breach after slowing under the 7 u/s rearm band,
//!    so one continuous burn cannot warn-and-trip. All three OnUpdate handlers
//!    gate `spotted == 0`, so a prior wake disarms them and a trip disarms the
//!    other four - the same one-shot contract.
//! 6. Watch-zone geometry: every detection bubble stays clear of the pinch
//!    safe lane (the NAV-1 -> NAV-2 leg) AND covers the wide swing around
//!    its boulder - sneaking through the pinch is how you stay unseen.
//! 7. Reaching YARD with `spotted == 0` lands Vesh's payoff line first and
//!    the Victory a beat later (the ch2b deferred-victory idiom) with the
//!    Magpies STILL Neutral; `spotted == 1` wins on the spot (the shipped
//!    flow). Both chain Victory -> ledger_ch4_the_buyer; player death
//!    before arrival retries ledger_ch3_quiet_channel on both paths.
//!
//! Harness mirrors ledger_ch2_encounter.rs (mod content stays out of the
//! deep core-CI suite; nova_assets unifies the serde feature so this compiles
//! standalone: `cargo test -p nova_assets --test ledger_ch3_channel`).

use bevy::{ecs::system::RunSystemOnce, math::Vec3, prelude::*};
use bevy_common_systems::prelude::{
    CommandsGameEventExt, EventAction, EventHandler, GameEventInfo, GameEventsPlugin,
    GameObjectives,
};
use nova_events::prelude::{
    EntityId, OnCombatLockEvent, OnCombatLockEventInfo, OnDestroyedEvent, OnDestroyedEventInfo,
    OnEnterEvent, OnEnterEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_gameplay::prelude::Allegiance;
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

fn spaceship_at<'a>(event: &'a ScenarioEventConfig, id: &str) -> &'a SpaceshipConfig {
    let obj = spawn_by_id(event, id);
    match &obj.kind {
        ScenarioObjectKind::Spaceship(ship) => ship,
        _ => panic!("'{id}' is a spaceship"),
    }
}

fn areas(event: &ScenarioEventConfig) -> Vec<&ScenarioAreaConfig> {
    event
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::CreateScenarioArea(config) => Some(config),
            _ => None,
        })
        .collect()
}

fn area_by_id<'a>(event: &'a ScenarioEventConfig, id: &str) -> &'a ScenarioAreaConfig {
    areas(event)
        .into_iter()
        .find(|a| a.id == id)
        .unwrap_or_else(|| panic!("OnStart creates area '{id}'"))
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

fn combat_lock(app: &mut App, id: &str, other_id: &str) {
    let info = OnCombatLockEventInfo {
        id: id.to_string(),
        other_id: other_id.to_string(),
        other_type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnCombatLockEvent>(info.clone());
        })
        .expect("fire OnCombatLock");
    app.update();
    app.update();
}

/// Spawn the two channel Magpies into the rig world through their REAL
/// OnStart `SpawnScenarioObject` configs (the same `EventAction` the loader
/// runs), then tick so the production `state_to_world` sync flushes the
/// deferred spawn commands - the ships exist as scoped entities carrying the
/// authored `Allegiance` component, exactly as in the game.
fn spawn_magpies(app: &mut App, scenario: &ScenarioConfig) {
    let start = on_start(scenario);
    for id in ["channel_magpie_1", "channel_magpie_2"] {
        let config = spawn_by_id(start, id).clone();
        let mut event_world = app.world_mut().resource_mut::<NovaEventWorld>();
        config.action(&mut event_world, &GameEventInfo::default());
    }
    app.update();
    app.update();
}

/// Read the live `Allegiance` COMPONENT off a spawned scenario ship - the
/// value the AI targeting actually reads, not a rig-side variable.
fn ship_allegiance(app: &mut App, id: &str) -> Allegiance {
    let mut query = app.world_mut().query::<(&EntityId, &Allegiance)>();
    query
        .iter(app.world())
        .find(|(entity_id, _)| entity_id.0 == id)
        .map(|(_, allegiance)| *allegiance)
        .unwrap_or_else(|| panic!("ship '{id}' exists with an Allegiance"))
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

/// Set the reserved `player_speed` readout (units/second) and tick, so the
/// overspeed provocation handlers see it - the same idiom as `pump_clock` for
/// the clock. In production `track_player_speed` (nova_scenario) writes this
/// off the player ship's LinearVelocity every live tick; the tracker is
/// unit-pinned there, so this rig injects the value directly and tests the
/// CONTENT handlers that consume it (task 20260723-143603).
fn pump_speed(app: &mut App, to_units: f64) {
    seed_var(app, "player_speed", to_units);
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
        ("spotted", 0.0),
        ("speed_warned", 0.0),
        ("overspeed_deadline", 0.0),
        ("win_gate", 0.0),
        ("win_said", 0.0),
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
        "spotted",
        "speed_warned",
        "overspeed_deadline",
        "win_gate",
        "win_said",
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
    // The stealth cast: both pickets and their watch zones are present from
    // the first frame - the player sees the patrols ahead and plans the
    // sneak, no jump-scare spawn on NAV-2.
    spawn_by_id(start, "channel_magpie_1");
    spawn_by_id(start, "channel_magpie_2");
    area_by_id(start, "picket_watch_starboard");
    area_by_id(start, "picket_watch_port");
}

#[test]
fn the_magpies_spawn_neutral_on_patrol_without_engage_delay() {
    // The stealth contract (task 20260723-000320): the pickets are NEUTRAL
    // bystanders on a patrol loop, not hostiles on an arrival grace. An
    // engage_delay would be a lie (a Neutral ship needs no grace), and a
    // missing patrol would leave them station-keeping instead of walking
    // the lane the player must sneak past.
    let scenario = scenario_from(CH3_RON);
    let start = on_start(&scenario);

    for id in ["channel_magpie_1", "channel_magpie_2"] {
        let ship = spaceship_at(start, id);
        assert_eq!(
            ship.allegiance,
            Some(Allegiance::Neutral),
            "'{id}' must spawn Neutral (neutral-until-provoked)"
        );
        match &ship.controller {
            SpaceshipController::AI(ai) => {
                assert!(
                    ai.patrol.len() >= 2,
                    "'{id}' needs a patrol route (found {} waypoints)",
                    ai.patrol.len()
                );
                assert_eq!(
                    ai.engage_delay, None,
                    "'{id}' must carry no engage_delay - that was the hostile-arrival \
                     grace, meaningless on a Neutral patrol"
                );
            }
            other => panic!("'{id}' is AI-driven, got {other:?}"),
        }
    }
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

#[test]
fn the_picket_watch_zones_spare_the_safe_lane_and_cover_the_wide_swing() {
    // The stealth geometry pin (task 20260723-000320), computed from the
    // authored positions like the pinch-gap pin above: the detection bubbles
    // must (a) leave the whole pinch safe lane - the NAV-1 -> NAV-2 leg and
    // its worst-case clear gap - undetected, so threading the pinch IS the
    // sneak; (b) cover the wide swing around their flanking boulder, so
    // skipping the pinch flies into the watch; and (c) stay clear of every
    // beacon's arrival sphere, so simply flying the objectives never trips
    // detection.
    let scenario = scenario_from(CH3_RON);
    let start = on_start(&scenario);

    let nav1 = spawn_by_id(start, "nav_1").base.position;
    let nav2 = spawn_by_id(start, "nav_2").base.position;
    let (port_pos, port) = asteroid_at(start, "pinch_boulder_port");
    let (star_pos, star) = asteroid_at(start, "pinch_boulder_starboard");

    // Worst-case half-width of the clear gap around the leg (the corridor a
    // threading pilot may legitimately occupy).
    let centre_gap = port_pos.distance(star_pos);
    let worst_bodies = (port.radius + star.radius) * ROCK_WORST;
    let half_gap = (centre_gap - worst_bodies) * 0.5;

    for id in ["picket_watch_starboard", "picket_watch_port"] {
        let area = area_by_id(start, id);

        // (a) The safe lane stays OUTSIDE the bubble, with margin: even a
        // pilot at the very edge of the worst-case gap is not detected.
        let (dist_to_leg, _) = point_to_segment(area.position, nav1, nav2);
        let clearance = dist_to_leg - area.radius;
        let need = half_gap + GAP_MARGIN;
        assert!(
            clearance >= need,
            "'{id}' edge is only {clearance:.1}u off the NAV-1 -> NAV-2 leg \
             (centre {dist_to_leg:.1}u, radius {:.1}u); the safe lane needs \
             >= {need:.1}u (worst-case half-gap {half_gap:.1}u + {GAP_MARGIN}u margin)",
            area.radius
        );

        // (b) The bubble sits on the wide swing: it contains its flanking
        // boulder's centre, so any arc around that boulder's far side crosses
        // the watch - the go-around is never free.
        let nearest_boulder = port_pos
            .distance(area.position)
            .min(star_pos.distance(area.position));
        assert!(
            nearest_boulder < area.radius,
            "'{id}' must sit on its boulder's wide swing (nearest boulder centre \
             {nearest_boulder:.1}u away, radius {:.1}u) - otherwise skipping the \
             pinch costs nothing",
            area.radius
        );

        // (c) No beacon arrival sphere overlaps a watch zone: touching an
        // objective is never what wakes the pickets.
        for beacon_id in ["nav_1", "nav_2", "nav_3", "vesh_yard", "pinch_clear"] {
            let beacon = spawn_by_id(start, beacon_id);
            let arrival = match &beacon.kind {
                ScenarioObjectKind::Beacon(b) => b.area_radius.unwrap_or(0.0),
                _ => panic!("'{beacon_id}' is a beacon"),
            };
            let gap = beacon.base.position.distance(area.position) - arrival - area.radius;
            assert!(
                gap > 0.0,
                "'{id}' overlaps the '{beacon_id}' arrival sphere by {:.1}u - \
                 flying the line would trip detection",
                -gap
            );
        }
    }
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
fn entering_a_picket_watch_zone_wakes_both_magpies() {
    let scenario = scenario_from(CH3_RON);
    let mut app = armed_app();
    spawn_magpies(&mut app, &scenario);

    // The pickets spawn asleep: the live component the AI targets by.
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Neutral
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_2"),
        Allegiance::Neutral
    );

    // Blunder into the starboard watch: SetAllegiance flips BOTH pickets.
    enter(&mut app, "picket_watch_starboard", "player_spaceship");
    assert_eq!(
        number_var(&app, "spotted"),
        Some(1.0),
        "the watch zone stamps spotted"
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Enemy,
        "the starboard picket wakes"
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_2"),
        Allegiance::Enemy,
        "its wingmate wakes with it"
    );

    // The other zone is disqualified (spotted == 0 was the one-shot gate).
    enter(&mut app, "picket_watch_port", "player_spaceship");
    assert_eq!(number_var(&app, "spotted"), Some(1.0));
}

#[test]
fn painting_a_sleeping_magpie_wakes_both() {
    let scenario = scenario_from(CH3_RON);
    let mut app = armed_app();
    spawn_magpies(&mut app, &scenario);

    // Red-lock the port picket: the paint provocation.
    combat_lock(&mut app, "channel_magpie_2", "player_spaceship");
    assert_eq!(
        number_var(&app, "spotted"),
        Some(1.0),
        "painting stamps spotted"
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Enemy
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_2"),
        Allegiance::Enemy
    );

    // Once awake, the existing fight wiring holds: dying to them is the
    // shipped Defeat + retry.
    destroy(&mut app, "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Defeat),
        "dying to the woken pickets loses the run"
    );
    let (next, linger) = queued_next(&app).expect("a retry is queued");
    assert_eq!(next, "ledger_ch3_quiet_channel");
    assert!(linger);
}

/// The fifth provocation (task 20260723-143603, reaction window
/// 20260723-182850): burning too hot wakes the pickets, but the SECOND strike
/// gives the player a sustained-overspeed grace before it lands. The rig
/// injects `player_speed` via `pump_speed` and the clock via `pump_clock`
/// (independent reserved-variable seeds; the engine writes `player_speed` off
/// the ship every tick in production, the tracker is unit-pinned in
/// nova_scenario, so here the CONTENT handlers are under test). Hysteresis:
/// warn > 8 u/s, rearm < 7; the armed breach STARTS a 3.5s countdown and the
/// wake lands only if the burn is held past the deadline - so one continuous
/// burn cannot warn-and-trip, and an armed breach cannot instantly trip.
#[test]
fn overspeed_warns_then_a_held_breach_wakes_both_magpies_after_the_window() {
    let scenario = scenario_from(CH3_RON);
    let mut app = armed_app();
    spawn_magpies(&mut app, &scenario);

    // Baseline: both pickets asleep, nobody warned.
    assert_eq!(number_var(&app, "speed_warned"), Some(0.0));
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Neutral
    );

    // First breach over 8 u/s: WARNS only. speed_warned -> 1, the pickets stay
    // Neutral and spotted stays 0 (a warning is not a spotting).
    pump_speed(&mut app, 9.0);
    assert_eq!(
        number_var(&app, "speed_warned"),
        Some(1.0),
        "the first overspeed only warns"
    );
    assert_eq!(
        number_var(&app, "spotted"),
        Some(0.0),
        "a warning does not wake the pickets"
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Neutral
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_2"),
        Allegiance::Neutral
    );

    // A SINGLE CONTINUOUS burn above 8 never advances past the warning: the
    // countdown needs speed_warned == 2, which needs a rearm below 7 first.
    // Hold hot; nothing advances.
    for _ in 0..3 {
        pump_speed(&mut app, 12.0);
    }
    assert_eq!(
        number_var(&app, "speed_warned"),
        Some(1.0),
        "a continuous burn never rearms, so it never arms the countdown"
    );
    assert_eq!(number_var(&app, "spotted"), Some(0.0));
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Neutral
    );

    // Slow back under the rearm band (< 7): ARMS the countdown (speed_warned ->
    // 2), silently - still Neutral, still unspotted.
    pump_speed(&mut app, 6.0);
    assert_eq!(
        number_var(&app, "speed_warned"),
        Some(2.0),
        "slowing under 7 arms the countdown"
    );
    assert_eq!(number_var(&app, "spotted"), Some(0.0));
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Neutral
    );

    // A FRESH breach over 8 once armed: STARTS the 3.5s countdown (speed_warned
    // -> 3) and stamps overspeed_deadline = scenario_elapsed + 3.5. It does NOT
    // wake yet - this is the reaction window the player asked for.
    pump_speed(&mut app, 9.0);
    assert_eq!(
        number_var(&app, "speed_warned"),
        Some(3.0),
        "the armed breach starts the countdown, it does not trip"
    );
    assert_eq!(
        number_var(&app, "overspeed_deadline"),
        Some(3.5),
        "the deadline is stamped scenario_elapsed (0) + 3.5"
    );
    assert_eq!(
        number_var(&app, "spotted"),
        Some(0.0),
        "the countdown has not elapsed, so the pickets stay asleep"
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Neutral
    );

    // Still hot but BEFORE the deadline (clock at 2.0 < 3.5): no wake. The burn
    // persists (player_speed holds at 9 across the clock pump).
    pump_clock(&mut app, 2.0);
    assert_eq!(
        number_var(&app, "spotted"),
        Some(0.0),
        "before the deadline the run is still dark"
    );
    assert_eq!(
        number_var(&app, "speed_warned"),
        Some(3.0),
        "the countdown is still running"
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Neutral
    );

    // Held past the deadline (clock at 4.0 > 3.5) with the burn still > 8: WAKE.
    // spotted -> 1, BOTH pickets Enemy on the live Allegiance component - the
    // same outcome as the zone/paint wakes.
    pump_clock(&mut app, 4.0);
    assert_eq!(
        number_var(&app, "spotted"),
        Some(1.0),
        "holding the burn past the window wakes them"
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Enemy
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_2"),
        Allegiance::Enemy
    );
}

/// The reaction window is real: easing off the throttle BEFORE the deadline
/// cancels the wake and re-arms, and the run stays dark. A later held breach
/// past a fresh deadline still wakes them - so the cancel is a reprieve, not an
/// immunity (task 20260723-182850).
#[test]
fn easing_off_during_the_countdown_cancels_the_wake() {
    let scenario = scenario_from(CH3_RON);
    let mut app = armed_app();
    spawn_magpies(&mut app, &scenario);

    // Warn, rearm, then a fresh breach starts the countdown (deadline 3.5).
    pump_speed(&mut app, 9.0);
    pump_speed(&mut app, 6.0);
    pump_speed(&mut app, 9.0);
    assert_eq!(number_var(&app, "speed_warned"), Some(3.0));
    assert_eq!(number_var(&app, "overspeed_deadline"), Some(3.5));

    // Ease off under the rearm band BEFORE the deadline: the countdown CANCELS
    // back to armed (speed_warned -> 2), silently, and the pickets stay asleep.
    pump_clock(&mut app, 2.0); // clock past 0 but before 3.5
    pump_speed(&mut app, 6.0);
    assert_eq!(
        number_var(&app, "speed_warned"),
        Some(2.0),
        "easing off in time cancels the countdown and re-arms"
    );
    assert_eq!(number_var(&app, "spotted"), Some(0.0));
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Neutral
    );

    // Even letting the clock run well past the OLD deadline: no wake - the state
    // is armed (2), not counting (3).
    pump_clock(&mut app, 10.0);
    assert_eq!(number_var(&app, "spotted"), Some(0.0));
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Neutral
    );

    // A fresh breach starts a NEW countdown (deadline 10 + 3.5 = 13.5); held
    // past it, they wake. The reprieve did not grant immunity.
    pump_speed(&mut app, 9.0);
    assert_eq!(number_var(&app, "speed_warned"), Some(3.0));
    assert_eq!(
        number_var(&app, "overspeed_deadline"),
        Some(13.5),
        "the new countdown is stamped off the current clock"
    );
    pump_clock(&mut app, 14.0);
    assert_eq!(
        number_var(&app, "spotted"),
        Some(1.0),
        "a held breach after the reprieve still wakes them"
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Enemy
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_2"),
        Allegiance::Enemy
    );
}

/// The overspeed provocation composes one-shot with the other four: once any
/// wake has stamped `spotted`, the speed handlers (all gated spotted == 0) are
/// inert - no second warning, no re-flip. Pins the one-shot contract from the
/// speed side (task 20260723-143603).
#[test]
fn a_prior_wake_disarms_the_overspeed_provocation() {
    let scenario = scenario_from(CH3_RON);
    let mut app = armed_app();
    spawn_magpies(&mut app, &scenario);

    // A picket-watch zone entry wakes them first.
    enter(&mut app, "picket_watch_starboard", "player_spaceship");
    assert_eq!(
        number_var(&app, "spotted"),
        Some(1.0),
        "the zone wakes them"
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Enemy
    );

    // Now burn hot: the speed handlers gate spotted == 0, so they never fire -
    // speed_warned stays 0 (no second Vesh warning) and spotted stays 1.
    pump_speed(&mut app, 20.0);
    assert_eq!(
        number_var(&app, "speed_warned"),
        Some(0.0),
        "an already-spotted run never warns on speed"
    );
    assert_eq!(number_var(&app, "spotted"), Some(1.0));
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Enemy
    );
}

#[test]
fn a_clean_run_earns_the_payoff_line_then_the_deferred_victory() {
    let scenario = scenario_from(CH3_RON);
    let mut app = armed_app();
    spawn_magpies(&mut app, &scenario);

    // Walk the corridor without ever touching a watch zone or painting.
    enter(&mut app, "nav_1", "player_spaceship");
    enter(&mut app, "nav_2", "player_spaceship");
    enter(&mut app, "nav_3", "player_spaceship");
    assert_eq!(number_var(&app, "gate"), Some(4.0));
    assert_eq!(number_var(&app, "spotted"), Some(0.0), "never seen");

    // Arrival: the run closes and the payoff beat arms, but the Victory
    // overlay does NOT land in the same frame as Vesh's line (StoryMessage
    // never beside Outcome - the ch2b deferred-victory idiom).
    enter(&mut app, "vesh_yard", "player_spaceship");
    assert_eq!(
        number_var(&app, "act"),
        Some(2.0),
        "the arrival closes the act"
    );
    assert_eq!(
        outcome_kind(&app),
        None,
        "the payoff line speaks first; the overlay waits its beat"
    );
    let win_gate = number_var(&app, "win_gate").expect("win_gate stamped");
    assert!(
        win_gate > 0.0,
        "the clean arrival arms the deferred overlay"
    );

    // The payoff line is authored on the clean arrival handler itself.
    let clean_yard = scenario
        .events
        .iter()
        .find(|e| {
            matches!(e.name, EventConfig::OnEnter) && e.filters.iter().any(|f| {
                matches!(
                    f,
                    EventFilterConfig::Entity(entity) if entity.id.as_deref() == Some("vesh_yard")
                )
            })
                && e.actions
                    .iter()
                    .any(|a| matches!(a, EventActionConfig::StoryMessage(_)))
        })
        .expect("the clean yard arrival carries the payoff line");
    assert!(
        !clean_yard
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::Outcome(_))),
        "the payoff line and the Outcome never share a handler"
    );

    // A beat later: the Victory overlay, the ch4 chain, the pickets STILL
    // asleep - stealth delivered.
    pump_clock(&mut app, win_gate + 1.0);
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "the clean run wins the chapter a beat after the payoff line"
    );
    assert_eq!(number_var(&app, "win_said"), Some(1.0));
    let (next, linger) = queued_next(&app).expect("victory chains on");
    assert_eq!(next, "ledger_ch4_the_buyer");
    assert!(linger, "the player chooses when to fly into chapter four");
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_1"),
        Allegiance::Neutral,
        "the pickets never woke"
    );
    assert_eq!(
        ship_allegiance(&mut app, "channel_magpie_2"),
        Allegiance::Neutral,
        "the pickets never woke"
    );
}

#[test]
fn a_provoked_run_still_wins_on_the_spot_into_chapter_four() {
    let scenario = scenario_from(CH3_RON);
    let mut app = armed_app();
    spawn_magpies(&mut app, &scenario);

    // Wake the pickets mid-run, then finish the corridor anyway (the
    // fighting-is-optional contract: run or fight, the gates are the job).
    enter(&mut app, "nav_1", "player_spaceship");
    enter(&mut app, "picket_watch_port", "player_spaceship");
    assert_eq!(number_var(&app, "spotted"), Some(1.0));
    enter(&mut app, "nav_2", "player_spaceship");
    enter(&mut app, "nav_3", "player_spaceship");
    assert_eq!(number_var(&app, "gate"), Some(4.0));
    assert_eq!(outcome_kind(&app), None);

    // The provoked arrival is the shipped flow: Victory on the spot.
    enter(&mut app, "vesh_yard", "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "reaching the yard wins the chapter even after being made"
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
