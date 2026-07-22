//! Production-faithful behavior rig for The Ledger chapter 4's DIVERGING
//! endings (task 20260722-214110). Loads the ACTUAL shipped
//! `webmods/the-ledger/ledger_ch4.content.ron`, registers its real handlers
//! the way the loader does, and drives the act machine with the same event
//! infos the engine emits (plus a clock pump for the deferred burn overlay).
//! The test IS the divergence contract - it pins that:
//!
//! 1. OnStart seeds `act = 1` / `choice = 0` and spawns both branch beacons
//!    (`handoff_berth`, `burn_buoy`) plus the player;
//! 2. the SELL branch (HANDOFF berth) sets `choice = 1` / `act = 2` AND spawns
//!    the `auditor` gunship WITH an `engage_delay` telegraph;
//! 3. the BURN branch (buoy) sets `choice = 2`, does NOT spawn the Auditor,
//!    latches `act = 3` synchronously, and reaches a terminal Victory a beat
//!    later WITHOUT any fight;
//! 4. the two paths land DISTINCT terminal outcomes (both Victory - the only
//!    win kind - but distinct MESSAGES, and structurally one has no fight);
//! 5. Defeat (player death) is reachable ONLY on the sell path (act == 2) and
//!    is inert once the burn path has latched act = 3;
//! 6. neither terminal ending chains a NextScenario off this last chapter
//!    (only the Defeat retry requeues the finale).
//!
//! Harness mirrors `ledger_ch2_encounter.rs` (mod content stays out of the
//! deep core-CI behavior suite; nova_assets unifies the serde feature so this
//! compiles standalone:
//! `cargo test -p nova_assets --test ledger_ch4_ending`).

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use bevy_common_systems::prelude::{
    CommandsGameEventExt, EventHandler, GameEventsPlugin, GameObjectives,
};
use nova_events::prelude::{
    OnDestroyedEvent, OnDestroyedEventInfo, OnEnterEvent, OnEnterEventInfo, OnUpdateEvent,
    OnUpdateEventInfo,
};
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

const CH4_RON: &str = include_str!("../../../webmods/the-ledger/ledger_ch4.content.ron");
const LEDGER_BUNDLE_RON: &str = include_str!("../../../webmods/the-ledger/the-ledger.bundle.ron");

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

fn spawn_by_id<'a>(event: &'a ScenarioEventConfig, id: &str) -> Option<&'a ScenarioObjectConfig> {
    spawns(event).into_iter().find(|s| s.base.id == id)
}

// --- app harness (mirrors ledger_ch2_encounter.rs) --------------------------

fn slice_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // The Auditor-arrival beat (sell path) now carries a real SetSkybox accent
    // (task 20260722-214115); its command reads the AssetServer to start the
    // cubemap load, exactly as in production. Register the asset plumbing so the
    // shipped handoff_berth handler runs to completion in the rig rather than
    // panicking on a missing resource (no scenario camera is present, so the
    // swap no-ops after the load kicks off - all this behavior rig needs).
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

/// Fire an OnEnter of `beacon` by the player (the choice beacons carry the
/// area id as subject, the entrant as other party - the loader's pairing).
fn enter(app: &mut App, beacon: &str) {
    let info = OnEnterEventInfo {
        id: beacon.to_string(),
        other_id: "player_spaceship".to_string(),
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

/// Pump the scenario clock past a deadline and tick, so the deferred burn
/// overlay (gated `scenario_elapsed > burn_gate`) actually fires. The rig
/// sets no time, so `scenario_elapsed` reads 0 until we stamp it here (the
/// time-gated-content-needs-a-clock-pump lesson, task 20260721-211506).
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

fn outcome_message(app: &App) -> Option<String> {
    app.world()
        .resource::<CurrentOutcome>()
        .0
        .as_ref()
        .and_then(|outcome| outcome.message.clone())
}

fn queued_next(app: &App) -> Option<(String, bool)> {
    app.world()
        .resource::<NovaEventWorld>()
        .next_scenario
        .as_ref()
        .map(|next| (next.scenario_id.clone(), next.linger))
}

/// The finale's act machine seeded the way OnStart does (act = 1,
/// choice = 0, the burn one-shots). OnStart's own wiring is pinned
/// structurally in `on_start_seeds_the_branch`.
fn armed_app(scenario: &ScenarioConfig) -> App {
    let mut app = slice_app();
    register_non_start_handlers(&mut app, scenario);
    seed_var(&mut app, "act", 1.0);
    seed_var(&mut app, "choice", 0.0);
    seed_var(&mut app, "burn_gate", 0.0);
    seed_var(&mut app, "burn_said", 0.0);
    app
}

// --- structural pin: OnStart seeds the branch and spawns the cast ----------

#[test]
fn on_start_seeds_the_branch() {
    let scenario = scenario_from(CH4_RON);
    let start = on_start(&scenario);

    let seeded: Vec<(&str, &VariableExpressionNode)> = start
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::VariableSet(set) => Some((set.key.as_str(), &set.expression)),
            _ => None,
        })
        .collect();
    let seeds = |k: &str| seeded.iter().find(|(key, _)| *key == k).map(|(_, e)| *e);
    assert!(seeds("act").is_some(), "OnStart seeds 'act'");
    assert!(seeds("choice").is_some(), "OnStart seeds 'choice'");
    assert!(
        seeds("burn_gate").is_some() && seeds("burn_said").is_some(),
        "OnStart seeds the burn one-shots so the deferred overlay reads defined values"
    );

    // Both branch beacons and the player must be present for the walks below.
    assert!(
        spawn_by_id(start, "player_spaceship").is_some(),
        "spawns the player"
    );
    assert!(
        spawn_by_id(start, "handoff_berth").is_some(),
        "spawns the SELL beacon"
    );
    assert!(
        spawn_by_id(start, "burn_buoy").is_some(),
        "spawns the BURN beacon"
    );
    // The Auditor is NEVER spawned at OnStart - it is a SELL-branch arrival.
    assert!(
        spawn_by_id(start, "auditor").is_none(),
        "the Auditor is not an OnStart spawn - it arrives on the sell branch only"
    );
}

// --- SELL branch: the fight arrives, telegraphed ---------------------------

#[test]
fn sell_branch_spawns_a_telegraphed_auditor() {
    let scenario = scenario_from(CH4_RON);

    // Find the HANDOFF OnEnter handler and read its Auditor spawn directly, so
    // the telegraph pin does not depend on the runtime spawn path.
    let handoff = scenario
        .events
        .iter()
        .find(|e| {
            matches!(e.name, EventConfig::OnEnter)
                && e.filters.iter().any(|f| {
                    matches!(f, EventFilterConfig::Entity(ent)
                        if ent.id.as_deref() == Some("handoff_berth"))
                })
        })
        .expect("has a HANDOFF OnEnter handler");
    let auditor = spawn_by_id(handoff, "auditor").expect("the SELL branch spawns the Auditor");
    let ScenarioObjectKind::Spaceship(ship) = &auditor.kind else {
        panic!("the Auditor is a spaceship");
    };
    let SpaceshipController::AI(ai) = &ship.controller else {
        panic!("the Auditor is AI-controlled");
    };
    let grace = ai
        .engage_delay
        .expect("the Auditor now telegraphs with an engage_delay grace");
    assert!(
        grace >= 8.0,
        "the Auditor's engage_delay ({grace}) matches the ch2/ch2b/ch3 telegraph (>= 8s)"
    );

    // Behavior: entering the berth flips the sell act machine and arms the fight.
    let mut app = armed_app(&scenario);
    enter(&mut app, "handoff_berth");
    assert_eq!(
        number_var(&app, "choice"),
        Some(1.0),
        "SELL sets choice = 1"
    );
    assert_eq!(
        number_var(&app, "act"),
        Some(2.0),
        "SELL flips to the fight act"
    );
    assert_eq!(
        outcome_kind(&app),
        None,
        "the sale is not a win yet - the Auditor still has to die"
    );
}

#[test]
fn sell_branch_wins_by_breaking_the_auditor() {
    let scenario = scenario_from(CH4_RON);
    let mut app = armed_app(&scenario);

    enter(&mut app, "handoff_berth");
    assert_eq!(outcome_kind(&app), None);

    destroy(&mut app, "auditor");
    assert_eq!(
        number_var(&app, "act"),
        Some(3.0),
        "the kill closes the act"
    );
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "breaking the Auditor wins the SELL ending"
    );
    assert!(
        outcome_message(&app).unwrap().contains("SOLD"),
        "the SELL ending is the payday message"
    );
    assert_eq!(
        queued_next(&app),
        None,
        "the finale's win does not chain a NextScenario"
    );
}

// --- BURN branch: no fight, distinct terminal outcome ----------------------

#[test]
fn burn_branch_closes_without_a_fight() {
    let scenario = scenario_from(CH4_RON);
    let mut app = armed_app(&scenario);
    // The buoy stamps burn_gate = scenario_elapsed + 3; give the clock a base.
    seed_var(&mut app, "scenario_elapsed", 30.0);

    enter(&mut app, "burn_buoy");
    assert_eq!(
        number_var(&app, "choice"),
        Some(2.0),
        "BURN sets choice = 2"
    );
    // The terminal latch is SYNCHRONOUS with the choice - act = 3 immediately,
    // so no death window ever opens on this path.
    assert_eq!(
        number_var(&app, "act"),
        Some(3.0),
        "BURN latches the terminal act immediately (no death window)"
    );
    // The overlay is DEFERRED a beat behind the burn line - not up yet.
    assert_eq!(
        outcome_kind(&app),
        None,
        "the burn comms line plays first; the overlay is a beat behind"
    );

    // Pump past burn_gate: the deferred BURN overlay fires.
    pump_clock(&mut app, 100.0);
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "the burn escape is a Victory (the only win kind) once the breather elapses"
    );
    assert!(
        outcome_message(&app).unwrap().contains("BURNED"),
        "the BURN ending is the safe-but-broke message"
    );
    assert_eq!(
        queued_next(&app),
        None,
        "the burn ending does not chain a NextScenario off the last chapter"
    );
}

#[test]
fn burn_branch_never_spawns_the_auditor() {
    let scenario = scenario_from(CH4_RON);
    let mut app = armed_app(&scenario);
    seed_var(&mut app, "scenario_elapsed", 30.0);

    enter(&mut app, "burn_buoy");

    // No handler in the whole scenario spawns the Auditor on the burn path.
    // Structurally: only the HANDOFF (sell) handler carries an auditor spawn.
    let auditor_spawn_sites = scenario
        .events
        .iter()
        .filter(|e| spawn_by_id(e, "auditor").is_some())
        .count();
    assert_eq!(
        auditor_spawn_sites, 1,
        "exactly ONE handler spawns the Auditor (the sell branch); the burn branch has none"
    );
}

// --- the divergence: distinct terminal outcomes ----------------------------

#[test]
fn the_two_endings_are_distinct_terminal_outcomes() {
    let scenario = scenario_from(CH4_RON);

    // Drive SELL to its terminal.
    let mut sell = armed_app(&scenario);
    enter(&mut sell, "handoff_berth");
    destroy(&mut sell, "auditor");
    let sell_kind = outcome_kind(&sell).expect("sell reaches an outcome");
    let sell_msg = outcome_message(&sell).expect("sell has a message");
    assert_eq!(number_var(&sell, "act"), Some(3.0), "sell is terminal");

    // Drive BURN to its terminal.
    let mut burn = armed_app(&scenario);
    seed_var(&mut burn, "scenario_elapsed", 30.0);
    enter(&mut burn, "burn_buoy");
    pump_clock(&mut burn, 100.0);
    let burn_kind = outcome_kind(&burn).expect("burn reaches an outcome");
    let burn_msg = outcome_message(&burn).expect("burn has a message");
    assert_eq!(number_var(&burn, "act"), Some(3.0), "burn is terminal");

    // Both are Victory (the only win kind), but the endings are DISTINCT: the
    // messages differ, AND the structural fact that the burn path never
    // fought is pinned by `burn_branch_never_spawns_the_auditor`.
    assert_eq!(sell_kind, ScenarioOutcomeKind::Victory);
    assert_eq!(burn_kind, ScenarioOutcomeKind::Victory);
    assert_ne!(
        sell_msg, burn_msg,
        "the two endings must carry DISTINCT terminal messages, not the same text"
    );
    assert!(sell_msg.contains("SOLD") && burn_msg.contains("BURNED"));
}

// --- Defeat: sell-path only, inert once the burn has closed ----------------

#[test]
fn defeat_is_reachable_only_on_the_sell_path() {
    let scenario = scenario_from(CH4_RON);

    // Sell path, mid-fight (act == 2): player death is a Defeat + retry.
    let mut sell = armed_app(&scenario);
    enter(&mut sell, "handoff_berth");
    assert_eq!(number_var(&sell, "act"), Some(2.0));
    destroy(&mut sell, "player_spaceship");
    assert_eq!(
        outcome_kind(&sell),
        Some(ScenarioOutcomeKind::Defeat),
        "dying to the Auditor loses the sell ending"
    );
    let (next, linger) = queued_next(&sell).expect("Defeat queues the retry");
    assert_eq!(
        next, "ledger_ch4_the_buyer",
        "the retry is the finale itself"
    );
    assert!(linger);
}

#[test]
fn burn_path_has_no_death_window() {
    let scenario = scenario_from(CH4_RON);
    let mut app = armed_app(&scenario);
    seed_var(&mut app, "scenario_elapsed", 30.0);

    // Latch the burn (act -> 3) BEFORE any death.
    enter(&mut app, "burn_buoy");
    assert_eq!(number_var(&app, "act"), Some(3.0));

    // A player death AFTER the burn latch (debris, a stray) must not flip the
    // earned escape into a Defeat: the Defeat gate is act < 3.
    destroy(&mut app, "player_spaceship");
    assert_ne!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Defeat),
        "no Defeat can fire once the burn path has latched act = 3"
    );

    // And the deferred burn Victory still lands.
    pump_clock(&mut app, 100.0);
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Victory));
}

#[test]
fn a_settled_outcome_is_not_overwritten() {
    // Both endings latch a terminal act (== 3); no later handler may overwrite
    // a settled Outcome (the outcome-is-last-write-wins-close-the-act lesson).
    let scenario = scenario_from(CH4_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    // Post-win state: sold, closed.
    seed_var(&mut app, "act", 3.0);
    seed_var(&mut app, "choice", 1.0);
    seed_var(&mut app, "burn_gate", 0.0);
    seed_var(&mut app, "burn_said", 0.0);

    destroy(&mut app, "player_spaceship");
    assert_eq!(outcome_kind(&app), None, "no Defeat over a closed act");
    assert_eq!(queued_next(&app), None, "no retry queued over a closed act");
}

// --- the bundle still ships the chapter ------------------------------------

#[test]
fn the_bundle_ships_chapter_four() {
    assert!(
        LEDGER_BUNDLE_RON.contains("ledger_ch4.content.ron"),
        "the bundle lists chapter four"
    );
}
