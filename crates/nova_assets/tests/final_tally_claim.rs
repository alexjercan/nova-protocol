//! Production-faithful behavior + layout rig for Final Tally, chapter three's
//! finale. Loads the ACTUAL shipped
//! `final_tally.content.ron`, registers its real handlers, and drives the
//! machine with the same event infos the engine emits - plus layout pins
//! COMPUTED from the production gravity and geometry constants
//! (authored-vs-derived-values), so the staging cannot rot:
//!
//! 1. the coast-in: the player spawns outside the SOI of the claim's well
//!    (derived from the authored mass and
//!    GravitySettings::default().soi_cutoff_accel, never hand-copied);
//! 2. the survey is a one-shot travel-lock gate on the anchorage bow;
//! 3. the cast-off waits on ALL of: survey + both pickets + the breathe
//!    clock (flag-based - a pre-survey picket kill cannot deadlock it);
//! 4. the flagship kill opens a paced epilogue (act 4 locks the win; the
//!    banner lands at +9s with NOTHING queued - the campaign's designed
//!    end);
//! 5. the player's death is terminal only while the fight is LIVE
//!    (LESSONS: outcome-is-last-write-wins-close-the-act).
//!
//! Harness mirrors lifeline_convoy.rs / broadside_assault.rs
//! (`cargo test -p nova_assets --test final_tally_claim`).

use bevy::{ecs::system::RunSystemOnce, math::Vec3, prelude::*};
use nova_events::prelude::{
    CommandsGameEventExt, EntityId, EventHandler, GameEventsPlugin, LockEventInfo, OnDefeatedEvent,
    OnDefeatedEventInfo, OnDestroyedEvent, OnDestroyedEventInfo, OnTravelLockStartEvent,
    OnUpdateEvent, OnUpdateEventInfo,
};
use nova_gameplay::prelude::{GameObjectives, GravitySettings};
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

const FINAL_TALLY_RON: &str =
    include_str!("../../../assets/base/scenarios/final_tally.content.ron");
const BASE_BUNDLE_RON: &str = include_str!("../../../assets/base/base.bundle.ron");

/// The design floor (u) between hostile spawns and the player spawn (the
/// lifeline_convoy convention); the flagship additionally keeps its own
/// 1000u torpedo envelope clear of the player spawn (the balance audit's
/// derived number - pinned here so the berth cannot creep back inside it).
const HOSTILE_SPAWN_MIN_RANGE: f32 = 700.0;
const FLAGSHIP_TORPEDO_ENVELOPE: f32 = 1000.0;

/// Two frames, then run the world out to LIVE.
///
/// The two frames dispatch the event just fired; `settle_spawns` is what makes
/// the NEXT one land. This scenario spawns from handlers the rig registers, so
/// firing an event can leave the world settling - and the dispatcher holds
/// every handler until those objects are in. A fixed frame count would pass or
/// fail on machine load rather than on logic.
fn step(app: &mut App) {
    app.update();
    app.update();
    nova_scenario::test_support::settle_spawns(app);
}

fn scenario_from(ron_str: &str) -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(ron_str).expect("content RON parses");
    items
        .into_iter()
        .find_map(|c| match c {
            Content::Scenario(s) => Some(s),
            Content::Section(_) | Content::Campaign(_) | Content::Style(_) | Content::Ship(_) => {
                None
            }
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

fn triggered_spawn<'a>(scenario: &'a ScenarioConfig, id: &str) -> &'a ScenarioObjectConfig {
    // A lazily spawned ship arrives on a BEAT of a chain (the cast-off), so the
    // search has to walk into the steps, not just the handler's own actions.
    let mut found = None;
    for action in scenario.events.iter().flat_map(|e| e.actions.iter()) {
        action.walk(&mut |action| {
            if let EventActionConfig::SpawnScenarioObject(spawn) = action {
                if spawn.base.id == id && found.is_none() {
                    found = Some(spawn);
                }
            }
        });
    }
    found.unwrap_or_else(|| panic!("a handler spawns '{id}'"))
}

// --- app harness (mirrors lifeline_convoy.rs) -------------------------------

fn slice_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
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
        app.world_mut().spawn(event.build_handler());
        for gate in event.gate_handlers() {
            app.world_mut().spawn(gate);
        }
    }
}

/// Seed the whole OnStart variable block the way OnStart would. Must stay in
/// lockstep with the OnStart `VariableSet` block in `final_tally.content.ron` -
/// a slice test that omits a variable leaves the handler reading `None`, which
/// never passes, so the beat it guards silently never fires.
fn seed_live_claim(app: &mut App) {
    for (key, value) in [
        ("act", 1.0),
        ("surveyed", 0.0),
        ("picket_a_down", 0.0),
        ("picket_b_down", 0.0),
    ] {
        seed_var(app, key, value);
    }
}

/// Advance the scenario clock and deliver whatever that makes due - a beat of a
/// chain, a keyed timer's end - then pulse. The rig runs no clock systems of
/// its own, so this is the only thing that moves scenario time.
fn advance(app: &mut App, seconds: f64) {
    nova_scenario::test_support::advance_scenario_clock(app, seconds);
    pump(app);
}

/// How many handlers are still registered. A `once` handler despawns the
/// moment its filters pass, so this is what a spent beat looks like from
/// outside: the entity is gone, and the dispatcher never walks it again.
fn live_handlers(app: &mut App) -> usize {
    app.world_mut()
        .query::<&EventHandler<NovaEventWorld>>()
        .iter(app.world())
        .count()
}

fn destroy(app: &mut App, id: &str) {
    let info = OnDestroyedEventInfo {
        id: id.to_string(),
        type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnDefeatedEvent>(OnDefeatedEventInfo {
                id: info.id.clone(),
                type_name: info.type_name.clone(),
            });
            commands.fire::<OnDestroyedEvent>(info.clone());
        })
        .expect("fire OnDestroyed");
    step(app);
}

/// Fire the scenario travel-lock event the engine bridge emits when the
/// player's TRAVEL lock lands on a scenario object.
fn travel_lock(app: &mut App, target: &str) {
    let info = LockEventInfo {
        id: target.to_string(),
        other_id: "player_spaceship".to_string(),
        other_type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnTravelLockStartEvent>(info.clone());
        })
        .expect("fire OnTravelLockStart");
    step(app);
}

fn pump(app: &mut App) {
    step(app);
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

fn outcome_message(app: &App) -> String {
    app.world()
        .resource::<CurrentOutcome>()
        .0
        .as_ref()
        .and_then(|outcome| outcome.message.clone())
        .unwrap_or_default()
}

fn ship_in_world(app: &mut App, id: &str) -> bool {
    let mut q = app.world_mut().query::<&EntityId>();
    q.iter(app.world()).any(|e| **e == *id)
}

// --- the stage --------------------------------------------------------------

/// OnStart stages the claim: a hidden continuation (the gunship precedent)
/// with a thumbnail, the gravity-authored invulnerable planetoid at the
/// world origin (the Ring scatter's requirement), the two invulnerable
/// anchorage wrecks (the bow carrying the long-range survey signature),
/// two orbit-directed graced pickets, the Ring belt, one comms line, and
/// the survey objective.
#[test]
fn on_start_stages_the_claim() {
    let scenario = scenario_from(FINAL_TALLY_RON);
    assert!(scenario.hidden, "the finale never appears in the picker");
    assert!(
        scenario.thumbnail.is_some(),
        "thumbnail for the details pane"
    );

    let start = on_start(&scenario);

    let anchor = spawn_by_id(start, "claim_anchor");
    let ScenarioObjectKind::Asteroid(anchor_rock) = &anchor.kind else {
        panic!("the claim anchor is an asteroid");
    };
    assert_eq!(anchor.base.position, Vec3::new(0.0, -20.0, 0.0));
    assert!(anchor_rock.invulnerable, "the well survives the fight");
    assert_eq!(
        anchor_rock.mass,
        Some(45_000.0),
        "the claim is a REAL gravity well (the chain's first combat one)"
    );

    for (id, has_signature) in [("anchorage_bow", true), ("anchorage_stern", false)] {
        let wreck = spawn_by_id(start, id);
        let ScenarioObjectKind::Asteroid(rock) = &wreck.kind else {
            panic!("{id} is an asteroid");
        };
        assert!(rock.invulnerable, "{id} is set dressing + hard cover");
        assert_eq!(
            rock.lock_signature.is_some(),
            has_signature,
            "{id}: only the bow carries the survey signature"
        );
    }

    for id in ["picket_a", "picket_b"] {
        let ship = spawn_by_id(start, id);
        let ScenarioObjectKind::Spaceship(config) = &ship.kind else {
            panic!("{id} is a spaceship");
        };
        let SpaceshipController::AI(ai) = &config.controller else {
            panic!("{id} is AI-flown");
        };
        assert_eq!(
            ai.orbit.as_deref(),
            Some("claim_anchor"),
            "{id} is a guard on rails (the orbit directive's first combat use)"
        );
        assert!(ai.engage_delay.unwrap_or(0.0) > 0.0, "{id} is graced");
    }

    let ring = start
        .actions
        .iter()
        .find_map(|a| match a {
            EventActionConfig::ScatterObjects(s) => Some(s),
            _ => None,
        })
        .expect("the claim belt scatters on start");
    assert!(
        matches!(ring.region, ScatterRegion::Ring { .. }),
        "the belt is a RING around the origin well (first combat use)"
    );

    let stories: Vec<_> = start
        .actions
        .iter()
        .filter(|a| matches!(a, EventActionConfig::StoryMessage(_)))
        .collect();
    assert_eq!(stories.len(), 1, "one comms line per beat: the dispatch");
}

/// The base bundle registers the finale.
#[test]
fn base_bundle_ships_final_tally() {
    assert!(
        BASE_BUNDLE_RON.contains("scenarios/final_tally.content.ron"),
        "base.bundle.ron lists the generated final_tally file"
    );
}

/// Layout pins COMPUTED from the production constants
/// (authored-vs-derived-values): the coast-in spawn sits outside even the
/// worst-seed SOI; every hostile spawn keeps the design floor from the
/// player spawn; the flagship's berth keeps its own torpedo envelope clear
/// of the player spawn (the balance audit found 952u once - never again).
#[test]
fn layout_clearances_derive_from_the_measured_constants() {
    let scenario = scenario_from(FINAL_TALLY_RON);
    let start = on_start(&scenario);
    let player = spawn_by_id(start, "player_spaceship").base.position;
    let anchor = spawn_by_id(start, "claim_anchor");
    let ScenarioObjectKind::Asteroid(anchor_rock) = &anchor.kind else {
        panic!("anchor is an asteroid");
    };

    // The SOI, from the PRODUCTION constants: it falls out of the authored
    // mass alone, so unlike the body below it is the same on every seed.
    let soi = (anchor_rock.mass.expect("the claim anchor authors a mass")
        / GravitySettings::default().soi_cutoff_accel)
        .sqrt();
    assert!(
        player.distance(anchor.base.position) >= soi,
        "the player coasts IN from outside the SOI \
         ({:.0}u >= {soi:.0}u)",
        player.distance(anchor.base.position)
    );

    // Nothing spawns inside the planetoid's worst-case body.
    let worst_body = anchor_rock.radius * ASTEROID_GEOMETRIC_FACTOR_MAX;
    for event in &scenario.events {
        for s in spawns(event) {
            if s.base.id == "claim_anchor" {
                continue;
            }
            assert!(
                s.base.position.distance(anchor.base.position) >= worst_body,
                "'{}' spawns clear of the planetoid's worst-case body",
                s.base.id
            );
        }
    }

    // Hostile floors from the player spawn.
    for id in ["picket_a", "picket_b", "escort"] {
        let pos = triggered_spawn(&scenario, id).base.position;
        assert!(
            pos.distance(player) >= HOSTILE_SPAWN_MIN_RANGE,
            "'{id}' keeps the {HOSTILE_SPAWN_MIN_RANGE}u design floor"
        );
    }
    let flagship = triggered_spawn(&scenario, "flagship").base.position;
    assert!(
        flagship.distance(player) >= FLAGSHIP_TORPEDO_ENVELOPE,
        "the flagship's berth keeps its own torpedo envelope clear of the \
         player spawn ({:.0}u >= {FLAGSHIP_TORPEDO_ENVELOPE}u)",
        flagship.distance(player)
    );
}

// --- the machine ------------------------------------------------------------

/// The survey is a one-shot travel-lock gate: the first lock confirms the
/// claim and duplicate acquisition events are no-ops.
#[test]
fn the_survey_is_a_one_shot_travel_lock_gate() {
    let scenario = scenario_from(FINAL_TALLY_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_live_claim(&mut app);

    travel_lock(&mut app, "anchorage_bow");
    assert_eq!(number_var(&app, "surveyed"), Some(1.0), "the lock surveys");

    // The picket objective lands a BREATHE after the survey, not the same
    // frame: the survey starts a keyed timer and a separate OnTimerEnd handler
    // posts the objective when it ends (the "announce, breathe, arrive"
    // deferral). It stays a HANDLER rather than a beat because it must be
    // ABANDONED if both pickets die inside the gap. Advance past the timer.
    advance(&mut app, 30.0);
    assert!(
        app.world()
            .resource::<GameObjectives>()
            .objectives
            .iter()
            .any(|o| o.id == "picket"),
        "the survey posts the picket objective after the breathe"
    );

    // The 5s re-fire is a no-op (the one-shot flag gates the handler).
    travel_lock(&mut app, "anchorage_bow");
    assert_eq!(number_var(&app, "surveyed"), Some(1.0));

    // A lock on the STERN never surveys (only the bow carries the beat).
    let mut fresh = slice_app();
    register_non_start_handlers(&mut fresh, &scenario);
    seed_live_claim(&mut fresh);
    travel_lock(&mut fresh, "anchorage_stern");
    assert_eq!(
        number_var(&fresh, "surveyed"),
        Some(0.0),
        "the survey target is the bow, not any wreck"
    );
}

/// The cast-off waits for ALL of survey + both pickets + the breathe -
/// flag-based, so a pre-survey picket clear cannot deadlock it, and the
/// flagship arrives with its escort and the break objective.
#[test]
fn the_cast_off_waits_for_survey_pickets_and_the_breathe() {
    let scenario = scenario_from(FINAL_TALLY_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_live_claim(&mut app);

    // Kill the picket BEFORE surveying: the taunt beat starts the cast-off
    // chain, but no flagship (the chain's step also owes the survey).
    destroy(&mut app, "picket_a");
    destroy(&mut app, "picket_b");
    assert_eq!(
        app.world()
            .resource::<NovaEventWorld>()
            .sequence_step("cast_off"),
        Some(0),
        "the taunt beat arms the cast-off chain"
    );
    advance(&mut app, 30.0);
    assert!(
        !ship_in_world(&mut app, "flagship"),
        "no cast-off without the survey"
    );

    // Survey late: the cast-off arrives (no deadlock) - and the late survey
    // takes its pickets-down variant: no picket objective is left open for
    // ships that are already drift.
    travel_lock(&mut app, "anchorage_bow");
    // The gate opens on the next pulse the driver sees, so the beat lands on
    // the following advance rather than in the lock's own frame.
    advance(&mut app, 1.0);
    assert!(
        app.world()
            .resource::<GameObjectives>()
            .objectives
            .iter()
            .all(|o| o.id != "picket"),
        "a late survey never posts the picket objective"
    );
    assert!(
        ship_in_world(&mut app, "flagship"),
        "the flagship casts off"
    );
    assert!(ship_in_world(&mut app, "escort"), "with its escort");

    // The break objective also lands a breathe after the cast-off: the cast-off
    // beat starts a one-step chain of its own and the ENGINE holds the delay
    // (same announce-breathe-arrive deferral). Advance past it, then it posts.
    advance(&mut app, 30.0);
    assert!(
        app.world()
            .resource::<GameObjectives>()
            .objectives
            .iter()
            .any(|o| o.id == "break_flagship"),
        "the break objective posts after the breathe"
    );

    // And the breathe clock is real: a fresh run with survey FIRST holds
    // the flagship until the cast_at mark passes.
    let mut fresh = slice_app();
    register_non_start_handlers(&mut fresh, &scenario);
    seed_live_claim(&mut fresh);
    travel_lock(&mut fresh, "anchorage_bow");
    destroy(&mut fresh, "picket_a");
    destroy(&mut fresh, "picket_b");
    // The beat owes a 6s breathe from the kill; the clock has not moved yet.
    assert!(
        !ship_in_world(&mut fresh, "flagship"),
        "the breathe holds the cast-off"
    );
    advance(&mut fresh, 30.0);
    assert!(ship_in_world(&mut fresh, "flagship"));
}

/// The flagship kill opens the paced epilogue: act 4 immediately (the win
/// locks), the banner only at +9s - with the campaign-complete message and
/// NOTHING queued (the designed end).
#[test]
fn the_epilogue_paces_the_campaign_end() {
    let scenario = scenario_from(FINAL_TALLY_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_live_claim(&mut app);
    seed_var(&mut app, "surveyed", 1.0);

    let before = live_handlers(&mut app);
    destroy(&mut app, "flagship");
    assert_eq!(number_var(&app, "act"), Some(4.0), "the kill locks the win");
    assert_eq!(outcome_kind(&app), None, "no banner yet - the epilogue");

    advance(&mut app, 5.0);
    assert_eq!(
        outcome_kind(&app),
        None,
        "the close line beat, still no banner"
    );
    // The kill beat is `once`: it retired itself rather than latching a
    // said-flag. The two beats behind it are STEPS of the chain it started,
    // which cost no handler at all.
    assert_eq!(
        live_handlers(&mut app),
        before - 1,
        "a spent beat leaves the world instead of latching a flag"
    );

    advance(&mut app, 6.0);
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Victory));
    assert!(
        outcome_message(&app).contains("End of the base campaign"),
        "the designed end says so: {}",
        outcome_message(&app)
    );
    assert!(
        app.world()
            .resource::<NovaEventWorld>()
            .next_scenario
            .is_none(),
        "nothing queued - the chain ends here by design"
    );
}

/// The player's death is terminal only while the fight is LIVE: act 1
/// death declares Defeat + retries THIS scenario and closes every gate
/// (terminal act 3); a death during the epilogue (act 4) declares nothing
/// and the Victory still lands.
#[test]
fn player_death_is_terminal_only_while_live() {
    let scenario = scenario_from(FINAL_TALLY_RON);

    // Live death: Defeat, retry, gates closed for good.
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_live_claim(&mut app);
    destroy(&mut app, "player_spaceship");
    assert_eq!(number_var(&app, "act"), Some(3.0), "terminal act");
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Defeat));
    let world = app.world().resource::<NovaEventWorld>();
    let next = world.next_scenario.as_ref().expect("a retry is queued");
    assert_eq!(
        next.scenario_id, "final_tally",
        "the retry is THIS scenario"
    );
    assert!(next.linger);
    // The trade: the flagship dying after the player must not overwrite.
    destroy(&mut app, "flagship");
    advance(&mut app, 100.0);
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Defeat),
        "no win gate opens after the player's death"
    );

    // Epilogue death: the win is locked; the banner still lands.
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_live_claim(&mut app);
    seed_var(&mut app, "surveyed", 1.0);
    destroy(&mut app, "flagship");
    destroy(&mut app, "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        None,
        "a post-kill death declares nothing (the win is locked)"
    );
    advance(&mut app, 5.0);
    advance(&mut app, 6.0);
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "the epilogue completes over the wreckage"
    );
}
