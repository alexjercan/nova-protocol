//! Production-faithful behavior tests for Broadside, the chapter-two slice
//! (task 20260708-203659). Loads the ACTUAL shipped
//! `assets/base/scenarios/broadside.content.ron`, registers its real
//! `OnDestroyed`/`OnUpdate` handlers the way the loader does, and drives the
//! act machine by firing `OnDestroyedEvent` - the same info the integrity
//! bridge emits when a ship root dies (the physical bridge itself is pinned
//! in nova_scenario/nova_gameplay; what this file owns is the SCENARIO
//! DATA's consumption of it, and the filter/action machinery the data leans
//! on is pinned by nova_scenario's filters.rs tests). BASE story content
//! gets this depth of coverage in core CI; MOD content deliberately does
//! not (task 20260716-155830).
//!
//! Structural pins ride along for what the behavior rig seeds for itself
//! (rig-supplies-precondition): the OnStart stage - player loadout, the
//! NEUTRAL hauler, the trigger area, the act/flag seeds - plus the
//! base-bundle membership (a scenario missing from base.bundle.ron ships as
//! dead data: declared-but-not-loaded) and the story chain into and out of
//! the neighboring scenarios.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use bevy_common_systems::prelude::{
    CommandsGameEventExt, EventHandler, GameEventsPlugin, GameObjectives,
};
use nova_events::prelude::{
    EntityId, OnDestroyedEvent, OnDestroyedEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_gameplay::prelude::{Allegiance, SectionKind};
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

const BROADSIDE_RON: &str = include_str!("../../../assets/base/scenarios/broadside.content.ron");
const BROADSIDE_GUNSHIP_RON: &str =
    include_str!("../../../assets/base/scenarios/broadside_gunship.content.ron");
const SHAKEDOWN_RON: &str =
    include_str!("../../../assets/base/scenarios/shakedown_run.content.ron");
const ASTEROID_FIELD_RON: &str =
    include_str!("../../../assets/base/scenarios/asteroid_field.content.ron");
const BASE_BUNDLE_RON: &str = include_str!("../../../assets/base/base.bundle.ron");

fn scenario_from(ron: &str) -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(ron).expect("content RON parses");
    items
        .into_iter()
        .find_map(|c| match c {
            Content::Scenario(s) => Some(s),
            Content::Section(_) => None,
        })
        .expect("content contains a Scenario")
}

/// Headless app with the scenario event plumbing plus the per-frame
/// `OnUpdate` pulse (production's `fire_on_update` is crate-private to
/// nova_scenario). Act-machine variables are seeded per test the way
/// OnStart would; the OnStart wiring itself is pinned structurally below.
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

/// Fire the scenario `OnDestroyed` for a ship root id - the same info the
/// integrity bridge emits - and pump the handlers + queued commands through.
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

#[test]
fn breaking_both_corvettes_declares_the_chapter_checkpoint() {
    let scenario = scenario_from(BROADSIDE_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 1.0);
    seed_var(&mut app, "corvette_a_down", 0.0);
    seed_var(&mut app, "corvette_b_down", 0.0);

    // Delivery guard: nothing advances on its own.
    app.update();
    assert_eq!(number_var(&app, "act"), Some(1.0));

    destroy(&mut app, "corvette_a");
    assert_eq!(
        number_var(&app, "corvette_a_down"),
        Some(1.0),
        "the kill handler ran (delivery guard for the act assert below)"
    );
    assert_eq!(
        number_var(&app, "act"),
        Some(1.0),
        "one corvette is not enough to escalate"
    );

    // Double OnDestroyed for the same ship (multi-collider bodies can): the
    // flag is idempotent, the act must not skip.
    destroy(&mut app, "corvette_a");
    assert_eq!(number_var(&app, "corvette_a_down"), Some(1.0));
    assert_eq!(number_var(&app, "act"), Some(1.0));

    destroy(&mut app, "corvette_b");
    assert_eq!(
        number_var(&app, "act"),
        Some(2.0),
        "both corvettes down wins part one"
    );

    // The act-split checkpoint (spike F7): part one ends in a Victory beat
    // whose lingering chain enters the gunship scenario - the capital
    // fight retries THERE, never back through this ambush.
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "the broken ambush is a Victory beat"
    );
    let world = app.world().resource::<NovaEventWorld>();
    let next = world.next_scenario.as_ref().expect("the checkpoint chains");
    assert_eq!(next.scenario_id, "broadside_gunship");
    assert!(next.linger, "Continue rides the lingering chain");

    // No gunship in part one, in the drained world OR anywhere in the data.
    let mut q = app.world_mut().query::<&EntityId>();
    assert!(
        !q.iter(app.world()).any(|id| **id == *"gunship"),
        "part one never spawns the gunship"
    );
    assert!(
        !scenario.events.iter().flat_map(|e| e.actions.iter()).any(
            |a| matches!(a, EventActionConfig::SpawnScenarioObject(c) if c.base.id == "gunship")
        ),
        "no gunship spawn action survives in part one's data"
    );
}

#[test]
fn killing_the_gunship_declares_victory_with_no_queued_next() {
    let scenario = scenario_from(BROADSIDE_GUNSHIP_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 1.0);

    app.update();
    assert_eq!(outcome_kind(&app), None, "no outcome before the kill");

    destroy(&mut app, "gunship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "the gunship kill wins the slice"
    );
    assert!(
        app.world()
            .resource::<NovaEventWorld>()
            .next_scenario
            .is_none(),
        "end of the base story: nothing queued, the overlay offers Main Menu"
    );
}

#[test]
fn player_death_retries_the_current_part_only() {
    // The checkpoint's contract (spike F7): each part's Defeat requeues
    // ITSELF - a gunship death never re-earns the corvette ambush.
    for (ron, own_id) in [
        (BROADSIDE_RON, "broadside"),
        (BROADSIDE_GUNSHIP_RON, "broadside_gunship"),
    ] {
        let scenario = scenario_from(ron);
        let mut app = slice_app();
        register_non_start_handlers(&mut app, &scenario);
        seed_var(&mut app, "act", 1.0);

        destroy(&mut app, "player_spaceship");
        assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Defeat));
        let world = app.world().resource::<NovaEventWorld>();
        let next = world.next_scenario.as_ref().expect("a retry is queued");
        assert_eq!(next.scenario_id, own_id, "the retry is the CURRENT part");
        assert!(next.linger, "the retry lingers behind the overlay");
    }
}

/// Review R1.3 (original slice) + split review R1.3: a death AFTER the win
/// (act 2 in either part - a death blast, a rock under the gold banner)
/// declares NOTHING and pushes NOTHING - the earned Victory must not flip
/// to Defeat, and the hauler's soft-fail objective must not appear under
/// the overlay. The act-1 tests above are the delivery guards: the same
/// destroys on a live act do declare/push.
#[test]
fn player_death_after_the_win_declares_nothing() {
    for ron in [BROADSIDE_RON, BROADSIDE_GUNSHIP_RON] {
        let scenario = scenario_from(ron);
        let mut app = slice_app();
        register_non_start_handlers(&mut app, &scenario);
        seed_var(&mut app, "act", 2.0);

        destroy(&mut app, "player_spaceship");
        assert_eq!(
            outcome_kind(&app),
            None,
            "no Defeat after the win (the real flow holds Victory here)"
        );
        assert!(
            app.world()
                .resource::<NovaEventWorld>()
                .next_scenario
                .is_none(),
            "no retry queued over the earned Victory"
        );

        // The hauler's soft-fail gate is act < 2 too: no fresh objective
        // may land under the Victory overlay (split review R1.3).
        destroy(&mut app, "hauler");
        assert!(
            !app.world()
                .resource::<GameObjectives>()
                .objectives
                .iter()
                .any(|o| o.id == "hauler_lost"),
            "no hauler_lost objective under the earned Victory"
        );
    }
}

/// Delivery guard for the post-win hauler assert above: on a LIVE act the
/// same destroy does push the soft-fail beat.
#[test]
fn hauler_death_on_a_live_act_pushes_the_soft_fail_beat() {
    for ron in [BROADSIDE_RON, BROADSIDE_GUNSHIP_RON] {
        let scenario = scenario_from(ron);
        let mut app = slice_app();
        register_non_start_handlers(&mut app, &scenario);
        seed_var(&mut app, "act", 1.0);

        destroy(&mut app, "hauler");
        assert!(
            app.world()
                .resource::<GameObjectives>()
                .objectives
                .iter()
                .any(|o| o.id == "hauler_lost"),
            "the live-act hauler death pushes 'Make it cost them'"
        );
        assert_eq!(
            outcome_kind(&app),
            None,
            "the hauler is flavor, not failure - no Defeat"
        );
    }
}

/// The OnStart stage the behavior rigs seed for themselves, pinned on the
/// shipped data (rig-supplies-precondition): variables, the player loadout,
/// the neutral hauler, and the trigger area all come from OnStart.
#[test]
fn on_start_stages_the_slice() {
    let scenario = scenario_from(BROADSIDE_RON);
    assert!(!scenario.hidden, "broadside is a Scenarios-picker entry");

    let on_start = scenario
        .events
        .iter()
        .find(|e| matches!(e.name, EventConfig::OnStart))
        .expect("broadside has an OnStart");

    let seeded: Vec<&str> = on_start
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::VariableSet(v) => Some(v.key.as_str()),
            _ => None,
        })
        .collect();
    for key in ["act", "corvette_a_down", "corvette_b_down"] {
        assert!(seeded.contains(&key), "OnStart seeds '{key}'");
    }

    let ships: Vec<&ScenarioObjectConfig> = on_start
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::SpawnScenarioObject(config) => Some(config),
            _ => None,
        })
        .collect();

    let player = ships
        .iter()
        .find(|s| s.base.id == "player_spaceship")
        .expect("OnStart spawns the player");
    let ScenarioObjectKind::Spaceship(player_ship) = &player.kind else {
        panic!("player is a spaceship");
    };
    let SpaceshipController::Player(player_controller) = &player_ship.controller else {
        panic!("player-controlled");
    };
    // Playtest tuning (task 20260716-160159): torpedoes are the ENEMY's
    // weapon this chapter - the player screens them, not trades them - and
    // the magazine is finite with auto-reload (task 20260717-085640).
    assert!(
        !player_ship.sections.iter().any(|s| matches!(
            &s.source,
            SectionSource::Inline(c) if matches!(c.kind, SectionKind::Torpedo(_))
        )),
        "the player carries NO torpedo bay in chapter two"
    );
    assert!(
        player_ship.sections.iter().any(|s| matches!(
            &s.source,
            SectionSource::Inline(c) if matches!(c.kind, SectionKind::Turret(_))
        )),
        "the PDC turret is the player's weapon"
    );
    assert!(
        !player_controller.infinite_ammo,
        "finite auto-reloading ammo (matches Shakedown's chapter-one precedent, \
         now finite since task 20260717-085640)"
    );

    let hauler = ships
        .iter()
        .find(|s| s.base.id == "hauler")
        .expect("OnStart spawns the hauler");
    let ScenarioObjectKind::Spaceship(hauler_ship) = &hauler.kind else {
        panic!("hauler is a spaceship");
    };
    assert_eq!(
        hauler_ship.allegiance,
        Some(Allegiance::Neutral),
        "the hauler is authored NEUTRAL - no AI may target it"
    );
    assert!(
        matches!(hauler_ship.controller, SpaceshipController::None),
        "the hauler drifts (no controller)"
    );

    assert!(
        on_start.actions.iter().any(
            |a| matches!(a, EventActionConfig::CreateScenarioArea(area) if area.id == "hauler_area")
        ),
        "OnStart creates the ambush trigger area"
    );
}

/// The other half of the playtest tuning (task 20260716-160159): torpedoes
/// stay the GUNSHIP's weapon - the screening beat needs tubes on the enemy.
#[test]
fn the_gunship_keeps_its_torpedo_tubes() {
    let scenario = scenario_from(BROADSIDE_GUNSHIP_RON);
    let gunship = scenario
        .events
        .iter()
        .flat_map(|e| e.actions.iter())
        .find_map(|a| match a {
            EventActionConfig::SpawnScenarioObject(config) if config.base.id == "gunship" => {
                Some(config)
            }
            _ => None,
        })
        .expect("part two's OnStart spawns the gunship");
    let ScenarioObjectKind::Spaceship(ship) = &gunship.kind else {
        panic!("gunship is a spaceship");
    };
    let tubes = ship
        .sections
        .iter()
        .filter(|s| matches!(
            &s.source,
            SectionSource::Inline(c) if matches!(c.kind, SectionKind::Torpedo(_))
        ))
        .count();
    assert!(
        tubes >= 2,
        "the gunship fields its torpedo tubes (got {tubes})"
    );
}

/// A scenario absent from base.bundle.ron is dead data the game never loads
/// (declared-but-not-loaded).
#[test]
fn base_bundle_ships_broadside() {
    assert!(
        BASE_BUNDLE_RON.contains("scenarios/broadside.content.ron"),
        "base.bundle.ron lists broadside"
    );
    assert!(
        BASE_BUNDLE_RON.contains("scenarios/broadside_gunship.content.ron"),
        "base.bundle.ron lists the gunship part (declared-but-not-loaded otherwise)"
    );
}

/// The story chain, pinned at both ends: shakedown's chapter win chains into
/// broadside behind a Victory overlay, and the asteroid_field retrofits
/// (outcome review R1.8) declare their outcomes instead of switching
/// silently.
#[test]
fn story_chain_declares_outcomes_at_both_ends() {
    let shakedown = scenario_from(SHAKEDOWN_RON);
    let chapter_win = shakedown
        .events
        .iter()
        .find(|e| {
            e.actions.iter().any(|a| {
                matches!(
                    a,
                    EventActionConfig::NextScenario(next) if next.scenario_id == "broadside"
                )
            })
        })
        .expect("shakedown chains into broadside");
    assert!(
        chapter_win.actions.iter().any(|a| matches!(
            a,
            EventActionConfig::Outcome(o) if o.outcome == ScenarioOutcomeKind::Victory
        )),
        "the chain rides a Victory overlay (Continue)"
    );
    let chained = chapter_win
        .actions
        .iter()
        .find_map(|a| match a {
            EventActionConfig::NextScenario(next) => Some(next),
            _ => None,
        })
        .expect("chain action present");
    assert!(chained.linger, "the chain lingers behind the overlay");

    let field = scenario_from(ASTEROID_FIELD_RON);
    let field_death = field
        .events
        .iter()
        .find(|e| {
            matches!(e.name, EventConfig::OnDestroyed)
                && e.filters.iter().any(|f| matches!(
                    f,
                    EventFilterConfig::Entity(entity) if entity.id.as_deref() == Some("player_spaceship")
                ))
        })
        .expect("asteroid_field has a player-death handler");
    assert!(
        field_death.actions.iter().any(|a| matches!(
            a,
            EventActionConfig::Outcome(o) if o.outcome == ScenarioOutcomeKind::Defeat
        )),
        "the sandbox death restart declares Defeat (R1.8 retrofit)"
    );

    // The OTHER half of the retrofit (slice review R1.1 - the first
    // application was lost in a retry): the zone-clear switch to
    // asteroid_next declares Victory.
    let zone_clear = field
        .events
        .iter()
        .find(|e| {
            e.actions.iter().any(|a| {
                matches!(
                    a,
                    EventActionConfig::NextScenario(next) if next.scenario_id == "asteroid_next"
                )
            })
        })
        .expect("asteroid_field has the zone-clear switch");
    assert!(
        zone_clear.actions.iter().any(|a| matches!(
            a,
            EventActionConfig::Outcome(o) if o.outcome == ScenarioOutcomeKind::Victory
        )),
        "zone-clear declares Victory (R1.8's second half, review R1.1)"
    );
}

/// Part two is entered only through part one's checkpoint: hidden from the
/// picker, and its OnStart stages the whole capital fight (gunship spawned
/// immediately - the ~720u burn is the act's pacing).
#[test]
fn the_gunship_part_is_hidden_and_stages_itself() {
    let scenario = scenario_from(BROADSIDE_GUNSHIP_RON);
    assert!(scenario.hidden, "part two never appears in the picker");

    let on_start = scenario
        .events
        .iter()
        .find(|e| matches!(e.name, EventConfig::OnStart))
        .expect("part two has an OnStart");
    for id in ["player_spaceship", "hauler", "gunship"] {
        assert!(
            on_start
                .actions
                .iter()
                .any(|a| matches!(a, EventActionConfig::SpawnScenarioObject(c) if c.base.id == id)),
            "part two's OnStart spawns '{id}'"
        );
    }
}

/// The hard-cover tier (spike F4): five invulnerable boulders shared by
/// both parts, at least two inside each threat corridor (within 120u of the
/// hauler-to-threat axis), every worst-case 6x body clear of the stations,
/// the fixed spawns, and each other - computed from the shipped data, not
/// eyeballed (authored-vs-derived-values).
#[test]
fn hard_cover_anchors_both_threat_lanes() {
    // Distance from `p` to segment `a`->`b` plus the clamped progress of
    // the closest point (mirrors ledger_ch2_encounter.rs).
    fn point_to_segment(p: Vec3, a: Vec3, b: Vec3) -> (f32, f32) {
        let ab = b - a;
        let len2 = ab.length_squared();
        if len2 <= f32::EPSILON {
            return (p.distance(a), 0.0);
        }
        let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
        (p.distance(a + ab * t), t)
    }
    fn spawn_pos(scenario: &ScenarioConfig, id: &str) -> Vec3 {
        scenario
            .events
            .iter()
            .flat_map(|e| e.actions.iter())
            .find_map(|a| match a {
                EventActionConfig::SpawnScenarioObject(c) if c.base.id == id => {
                    Some(c.base.position)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("spawns '{id}'"))
    }
    fn boulders(scenario: &ScenarioConfig) -> Vec<(String, Vec3, f32)> {
        scenario
            .events
            .iter()
            .flat_map(|e| e.actions.iter())
            .filter_map(|a| match a {
                EventActionConfig::SpawnScenarioObject(c) => match &c.kind {
                    ScenarioObjectKind::Asteroid(rock) if rock.invulnerable => {
                        Some((c.base.id.clone(), c.base.position, rock.radius))
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    let part_one = scenario_from(BROADSIDE_RON);
    let part_two = scenario_from(BROADSIDE_GUNSHIP_RON);
    let hauler = spawn_pos(&part_one, "hauler");
    let corvette_mid =
        (spawn_pos(&part_one, "corvette_a") + spawn_pos(&part_one, "corvette_b")) / 2.0;
    let gunship = spawn_pos(&part_two, "gunship");

    for (part, scenario, threat) in [
        ("part one", &part_one, corvette_mid),
        ("part two", &part_two, gunship),
    ] {
        let field = boulders(scenario);
        assert!(
            field.len() >= 5,
            "{part}: the five hard boulders are staged (found {})",
            field.len()
        );

        let in_corridor = field
            .iter()
            .filter(|(_, pos, _)| {
                let (dist, t) = point_to_segment(*pos, hauler, threat);
                dist <= 120.0 && t > 0.05 && t < 0.95
            })
            .count();
        assert!(
            in_corridor >= 2,
            "{part}: needs >= 2 invulnerable boulders in the hauler-to-threat \
             corridor, found {in_corridor} - hard cover must sit on the attack \
             lane (spike F4)"
        );

        // Worst-case 6x bodies overlap nothing they must not.
        let stations = [
            ("hauler", hauler),
            ("player spawn", spawn_pos(scenario, "player_spaceship")),
        ];
        for (id, pos, radius) in &field {
            let body = radius * ASTEROID_GEOMETRIC_FACTOR_MAX;
            for (station, spot) in stations {
                let clearance = pos.distance(spot) - body;
                assert!(
                    clearance >= 20.0,
                    "{part}: boulder '{id}' worst-case body ({body:.0}u) leaves \
                     {clearance:.0}u at the {station}"
                );
            }
            // Outside the destructible scatter box (z in [-430, -80]): the
            // seeded chaff can then never merge with an anchor at the 6x
            // worst case.
            assert!(
                pos.z < -430.0,
                "{part}: boulder '{id}' sits inside the chaff scatter box"
            );
        }
        for i in 0..field.len() {
            for j in (i + 1)..field.len() {
                let (id_a, pos_a, r_a) = &field[i];
                let (id_b, pos_b, r_b) = &field[j];
                let gap = pos_a.distance(*pos_b) - (r_a + r_b) * ASTEROID_GEOMETRIC_FACTOR_MAX;
                assert!(
                    gap > 0.0,
                    "{part}: boulders '{id_a}' and '{id_b}' can merge at the 6x factor"
                );
            }
        }
    }

    // The fixed hostile spawns clear the boulders too (a corvette born
    // inside a rock is a soft-lock).
    for (id, spawn) in [
        ("corvette_a", spawn_pos(&part_one, "corvette_a")),
        ("corvette_b", spawn_pos(&part_one, "corvette_b")),
        ("gunship", gunship),
    ] {
        for (rock_id, pos, radius) in boulders(&part_one) {
            let clearance = pos.distance(spawn) - radius * ASTEROID_GEOMETRIC_FACTOR_MAX;
            assert!(
                clearance >= 20.0,
                "'{id}' spawns {clearance:.0}u clear of boulder '{rock_id}' (< 20u)"
            );
        }
    }
}
