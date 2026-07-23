//! Production-faithful behavior rig for The Ledger chapter 5, THE RAID - the
//! reward finale (task 20260723-182855). Loads the ACTUAL shipped
//! `webmods/the-ledger/ledger_ch5_the_raid.content.ron`, registers its real
//! handlers the way the loader does, and drives the raid with the same event
//! infos the engine emits. The test IS the reward contract - it pins that:
//!
//! 1. OnStart seeds the counters (`act = 1`, `raiders_left = 4`, `base_down =
//!    0`, the one-shot guards) and spawns the cast: the player gunship, two
//!    escorts, four Magpie fighters and the base station.
//! 2. the player flies a TORPEDO gunship - a Player-controlled ship that
//!    actually carries torpedo bays (the one time the campaign hands the player
//!    one) with the tubes bound to a fire input, plus its guns;
//! 3. the two escorts are Player-allegiance AI; the four raiders are Enemy
//!    (AI default) and the base station is Enemy;
//! 4. the raid is WON only when the base AND all four raiders are down - a
//!    partial clear does not win - and the win is TERMINAL (no NextScenario,
//!    this is the campaign's end);
//! 5. player death is a Defeat that retries ch5, and once the raid is won a
//!    late player death cannot overwrite the Victory (the
//!    outcome-is-last-write-wins-close-the-act lesson);
//! 6. reachability: ch4's SELL (fight) win chains a NextScenario into this
//!    raid, and exactly that one ch4 handler does so (the BURN path does not).
//!
//! Harness mirrors `ledger_ch4_ending.rs`; mod content stays out of the deep
//! core-CI behavior suite (`cargo test -p nova_assets --test ledger_ch5_raid`).

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use bevy_common_systems::prelude::{
    CommandsGameEventExt, EventHandler, GameEventsPlugin, GameObjectives,
};
use nova_events::prelude::{
    OnDestroyedEvent, OnDestroyedEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_gameplay::prelude::{Allegiance, SectionConfig, SectionKind};
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

const CH5_RON: &str = include_str!("../../../webmods/the-ledger/ledger_ch5_the_raid.content.ron");
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

fn spaceship_at<'a>(event: &'a ScenarioEventConfig, id: &str) -> &'a SpaceshipConfig {
    match &spawn_by_id(event, id)
        .unwrap_or_else(|| panic!("OnStart spawns '{id}'"))
        .kind
    {
        ScenarioObjectKind::Spaceship(ship) => ship,
        _ => panic!("'{id}' is a spaceship"),
    }
}

/// Resolve a section entry to its kind through the shipped catalog (prototypes)
/// or its inline config (the broadside_assault idiom).
fn section_kind(
    section: &SpaceshipSectionConfig,
    catalog: &[SectionConfig],
) -> Option<SectionKind> {
    match &section.source {
        SectionSource::Inline(c) => Some(c.kind.clone()),
        SectionSource::Prototype(id) => catalog
            .iter()
            .find(|c| c.base.id == *id)
            .map(|c| c.kind.clone()),
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

// --- app harness (mirrors ledger_ch4_ending.rs) -----------------------------

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

/// The raid's counters seeded the way OnStart does. OnStart's own wiring is
/// pinned structurally in `on_start_spawns_the_raid_cast_and_seeds_counters`.
fn armed_app(scenario: &ScenarioConfig) -> App {
    let mut app = slice_app();
    register_non_start_handlers(&mut app, scenario);
    seed_var(&mut app, "act", 1.0);
    seed_var(&mut app, "raiders_left", 4.0);
    seed_var(&mut app, "base_down", 0.0);
    seed_var(&mut app, "win_said", 0.0);
    seed_var(&mut app, "base_said", 0.0);
    app
}

/// Break the whole defence: the base plus all four fighters.
fn clear_the_raid(app: &mut App) {
    destroy(app, "magpie_base");
    for raider in ["raider_1", "raider_2", "raider_3", "raider_4"] {
        destroy(app, raider);
    }
    // One more pulse so the OnUpdate victory gate (which reads the counters
    // only after the kills land) is guaranteed to have evaluated.
    app.update();
}

// --- structural pin: OnStart seeds the counters and spawns the cast ---------

#[test]
fn on_start_spawns_the_raid_cast_and_seeds_counters() {
    let scenario = scenario_from(CH5_RON);
    let start = on_start(&scenario);

    for key in ["act", "raiders_left", "base_down", "win_said", "base_said"] {
        assert!(
            seeded_keys(start).contains(&key),
            "OnStart must seed '{key}' (an undefined counter fails the raid open)"
        );
    }

    // The whole cast every walk assumes.
    assert!(
        spawn_by_id(start, "player_spaceship").is_some(),
        "spawns the player gunship"
    );
    assert!(spawn_by_id(start, "wing_1").is_some(), "spawns escort 1");
    assert!(spawn_by_id(start, "wing_2").is_some(), "spawns escort 2");
    for raider in ["raider_1", "raider_2", "raider_3", "raider_4"] {
        assert!(spawn_by_id(start, raider).is_some(), "spawns {raider}");
    }
    assert!(
        spawn_by_id(start, "magpie_base").is_some(),
        "spawns the base station"
    );
    // Some planetoid scenery for cover and scale.
    assert!(
        spawn_by_id(start, "planetoid_1").is_some()
            && spawn_by_id(start, "planetoid_2").is_some()
            && spawn_by_id(start, "planetoid_3").is_some(),
        "spawns the planetoids"
    );
}

// --- the reward: the player finally flies a torpedo ship --------------------

#[test]
fn the_player_flies_a_torpedo_gunship() {
    let scenario = scenario_from(CH5_RON);
    let start = on_start(&scenario);
    let player = spaceship_at(start, "player_spaceship");
    let catalog = nova_assets::scenario_generation::build_section_catalog();

    // The whole point of the reward: unlike every prior chapter, the player's
    // ship actually CARRIES torpedo bays (and guns).
    let torpedo_ids: Vec<&str> = player
        .sections
        .iter()
        .filter(|s| matches!(section_kind(s, &catalog), Some(SectionKind::Torpedo(_))))
        .map(|s| s.id.as_str())
        .collect();
    assert!(
        torpedo_ids.len() >= 2,
        "the reward gunship carries torpedo bays (found {torpedo_ids:?})"
    );
    assert!(
        player
            .sections
            .iter()
            .any(|s| matches!(section_kind(s, &catalog), Some(SectionKind::Turret(_)))),
        "the gunship also carries guns"
    );

    // It is Player-driven, with the torpedo bays bound to a fire input and
    // infinite ammo so the hero moment is not magazine-gated.
    let SpaceshipController::Player(cfg) = &player.controller else {
        panic!("the gunship is player-controlled");
    };
    for tube in &torpedo_ids {
        let binds = cfg
            .input_mapping
            .get(*tube)
            .unwrap_or_else(|| panic!("torpedo bay '{tube}' is bound to a fire input"));
        // The tubes fire on the R KEY (playtest rebind, task 20260723-200643) -
        // round-trip each Binding back to its authorable form and look for R.
        assert!(
            binds
                .iter()
                .any(|b| BindingInput::try_from(b).ok()
                    == Some(BindingInput::Keyboard(KeyCode::KeyR))),
            "torpedo bay '{tube}' fires on the R key"
        );
    }
    assert!(cfg.infinite_ammo, "infinite ammo for the victory lap");
}

// --- allegiances: two friends, four foes, one hostile base ------------------

#[test]
fn the_wing_is_friendly_and_the_defenders_are_hostile() {
    let scenario = scenario_from(CH5_RON);
    let start = on_start(&scenario);

    // The escorts fight on the player's side.
    for wing in ["wing_1", "wing_2"] {
        let ship = spaceship_at(start, wing);
        assert_eq!(
            ship.allegiance,
            Some(Allegiance::Player),
            "{wing} is a Player-allegiance escort"
        );
        assert!(
            matches!(ship.controller, SpaceshipController::AI(_)),
            "{wing} is AI-flown"
        );
    }

    // The four raiders carry no authored allegiance, so they take the AI
    // marker's Enemy default (authored_allegiance_overrides_the_controller_default).
    for raider in ["raider_1", "raider_2", "raider_3", "raider_4"] {
        let ship = spaceship_at(start, raider);
        assert_eq!(
            ship.allegiance, None,
            "{raider} takes the AI Enemy default (no authored allegiance)"
        );
        assert!(
            matches!(ship.controller, SpaceshipController::AI(_)),
            "{raider} is AI-flown"
        );
    }

    // The base is explicitly Enemy and holds station (AI, no thrusters).
    let base = spaceship_at(start, "magpie_base");
    assert_eq!(
        base.allegiance,
        Some(Allegiance::Enemy),
        "the base is hostile"
    );
}

// --- the base holds station: RCS thrusters + a tight leash, fewer turrets ----

/// Playtest tuning (task 20260723-200643): the base gets RCS thrusters and a
/// tight leash so it station-keeps against the (reduced) gravity instead of
/// being dragged off, and its turret load is trimmed from four to two.
#[test]
fn the_base_holds_station_with_rcs_and_a_tight_leash() {
    let scenario = scenario_from(CH5_RON);
    let start = on_start(&scenario);
    let base = spaceship_at(start, "magpie_base");
    let catalog = nova_assets::scenario_generation::build_section_catalog();

    let count = |pred: fn(&SectionKind) -> bool| {
        base.sections
            .iter()
            .filter(|s| {
                section_kind(s, &catalog)
                    .as_ref()
                    .map(pred)
                    .unwrap_or(false)
            })
            .count()
    };
    let turrets = count(|k| matches!(k, SectionKind::Turret(_)));
    let thrusters = count(|k| matches!(k, SectionKind::Thruster(_)));
    assert_eq!(turrets, 2, "the base turret load is trimmed to two");
    assert!(
        thrusters >= 2,
        "the base carries RCS thrusters to station-keep (found {thrusters})"
    );

    // The tight leash keeps it from chasing the player off its post.
    let SpaceshipController::AI(ai) = &base.controller else {
        panic!("the base is AI-flown");
    };
    assert_eq!(
        ai.leash,
        Some(15.0),
        "the base is leashed tight to its post so it holds station"
    );
}

/// The finale is temporarily un-hidden (playtest convenience) so it can be
/// launched straight from the Scenarios picker. RE-HIDE before release.
#[test]
fn the_raid_is_launchable_for_testing() {
    let scenario = scenario_from(CH5_RON);
    assert!(
        !scenario.hidden,
        "ch5 is un-hidden for playtesting (re-hide before release)"
    );
}

// --- the win: base AND all four defenders, and it is terminal ---------------

#[test]
fn the_raid_wins_only_when_the_base_and_all_defenders_are_down() {
    let scenario = scenario_from(CH5_RON);
    let mut app = armed_app(&scenario);

    clear_the_raid(&mut app);

    assert_eq!(
        number_var(&app, "raiders_left"),
        Some(0.0),
        "all four fighters down"
    );
    assert_eq!(number_var(&app, "base_down"), Some(1.0), "the base is down");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "base + all defenders down wins the raid"
    );
    assert!(
        outcome_message(&app).unwrap().contains("account is closed"),
        "the win carries the raid payoff message"
    );
    assert_eq!(
        number_var(&app, "act"),
        Some(2.0),
        "the win latches the act"
    );
    // Terminal: the raid is the campaign's end, it does not chain onward.
    assert_eq!(
        queued_next(&app),
        None,
        "the raid win is terminal - no NextScenario off the last chapter"
    );
}

#[test]
fn a_partial_clear_does_not_win() {
    let scenario = scenario_from(CH5_RON);

    // Base down but a fighter still up: no win.
    let mut app = armed_app(&scenario);
    destroy(&mut app, "magpie_base");
    for raider in ["raider_1", "raider_2", "raider_3"] {
        destroy(&mut app, raider);
    }
    app.update();
    assert_eq!(
        number_var(&app, "raiders_left"),
        Some(1.0),
        "one fighter left"
    );
    assert_eq!(
        outcome_kind(&app),
        None,
        "a surviving fighter denies the win"
    );

    // All fighters down but the base still standing: also no win.
    let mut app = armed_app(&scenario);
    for raider in ["raider_1", "raider_2", "raider_3", "raider_4"] {
        destroy(&mut app, raider);
    }
    app.update();
    assert_eq!(
        number_var(&app, "raiders_left"),
        Some(0.0),
        "all fighters down"
    );
    assert_eq!(
        number_var(&app, "base_down"),
        Some(0.0),
        "the base still stands"
    );
    assert_eq!(outcome_kind(&app), None, "the base must fall too");
}

// --- the loss: player death retries the raid, and cannot un-win it ----------

#[test]
fn player_death_is_a_defeat_that_retries_the_raid() {
    let scenario = scenario_from(CH5_RON);
    let mut app = armed_app(&scenario);

    destroy(&mut app, "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Defeat),
        "dying in the raid is a Defeat"
    );
    assert!(
        outcome_message(&app)
            .unwrap()
            .contains("account stays open"),
        "the loss carries the raid Defeat message"
    );
    assert_eq!(
        number_var(&app, "act"),
        Some(3.0),
        "the loss latches the act"
    );
    let (next, linger) = queued_next(&app).expect("Defeat queues a retry");
    assert_eq!(next, "ledger_ch5_the_raid", "the retry is the raid itself");
    assert!(linger);
}

#[test]
fn a_won_raid_is_not_overwritten_by_a_late_defeat() {
    let scenario = scenario_from(CH5_RON);
    let mut app = armed_app(&scenario);

    clear_the_raid(&mut app);
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Victory));

    // A stray death after the win (debris, a last enemy shot in flight) must
    // NOT flip the earned Victory into a Defeat: the Defeat gate is act == 1,
    // and the win latched act = 2.
    destroy(&mut app, "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "a late death cannot overwrite the settled Victory"
    );
}

// --- reachability: only the ch4 SELL win leads here -------------------------

#[test]
fn the_ch4_sell_win_chains_into_this_raid() {
    let ch4 = scenario_from(CH4_RON);

    // Count the ch4 handlers that chain a NextScenario into this raid: exactly
    // one - the Auditor-death SELL win. Pin the structural reachability fact
    // (review-rig-can-false-green: pin that ONE path chains, not just that a
    // chain exists somewhere).
    let chains_to_raid = ch4
        .events
        .iter()
        .filter(|e| {
            e.actions.iter().any(|a| {
                matches!(
                    a,
                    EventActionConfig::NextScenario(next) if next.scenario_id == "ledger_ch5_the_raid"
                )
            })
        })
        .count();
    assert_eq!(
        chains_to_raid, 1,
        "exactly one ch4 handler (the SELL/fight win) chains into the raid"
    );

    // And that handler is the one gated on the Auditor's death - it also filters
    // the auditor entity, so the chain rides the fight win, not the burn ending.
    let sell_win_chains = ch4.events.iter().any(|e| {
        let filters_auditor = e.filters.iter().any(|f| {
            matches!(
                f,
                EventFilterConfig::Entity(entity) if entity.id.as_deref() == Some("auditor")
            )
        });
        let chains = e.actions.iter().any(|a| {
            matches!(
                a,
                EventActionConfig::NextScenario(next) if next.scenario_id == "ledger_ch5_the_raid"
            )
        });
        filters_auditor && chains
    });
    assert!(
        sell_win_chains,
        "the raid is chained off the Auditor-death (fight) win, not the burn ending"
    );
}

// --- the bundle ships the raid and the bumped version -----------------------

#[test]
fn the_bundle_ships_the_raid_and_bumps_the_version() {
    assert!(
        LEDGER_BUNDLE_RON.contains("ledger_ch5_the_raid.content.ron"),
        "the bundle lists the raid finale"
    );
    assert!(
        LEDGER_BUNDLE_RON.contains("version: \"1.11.0\""),
        "the bundle version is bumped for the raid finale + its tuning"
    );
}
