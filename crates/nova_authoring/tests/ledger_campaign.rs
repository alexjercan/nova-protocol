//! Contracts for the portal activity pack, read from its real RON files.
//! These check authored structure. Flight and appearance need separate plays.

use std::{collections::BTreeSet, path::PathBuf};

use nova_mod_format::BundleManifest;
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

const ACTIVITIES: [&str; 6] = [
    "ledger_01_drift_run",
    "ledger_02_rock_garden",
    "ledger_03_blue_survey",
    "ledger_04_cold_patrol",
    "ledger_05_freight_lane",
    "ledger_06_siege_line",
];

fn content() -> Vec<Content> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../webmods/the-ledger");
    let manifest: BundleManifest =
        ron::de::from_bytes(&std::fs::read(root.join("the-ledger.bundle.ron")).unwrap()).unwrap();
    for resource in &manifest.resources {
        assert!(
            root.join(resource).is_file(),
            "missing resource: {resource}"
        );
    }
    manifest
        .content
        .iter()
        .flat_map(|file| {
            ron::de::from_bytes::<Vec<Content>>(&std::fs::read(root.join(file)).unwrap())
                .unwrap_or_else(|error| panic!("{file}: {error}"))
        })
        .collect()
}

fn activities(content: &[Content]) -> Vec<&ScenarioConfig> {
    ACTIVITIES
        .iter()
        .map(|id| {
            content
                .iter()
                .find_map(|item| match item {
                    Content::Scenario(scenario) if scenario.id == *id => Some(scenario),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing activity {id}"))
        })
        .collect()
}

fn walk<'a>(actions: &'a [EventActionConfig], output: &mut Vec<&'a EventActionConfig>) {
    for action in actions {
        output.push(action);
        match action {
            EventActionConfig::Sequence(chain) => {
                for step in &chain.steps {
                    walk(&step.actions, output);
                }
            }
            EventActionConfig::Cinematic(chain) => {
                for step in &chain.steps {
                    walk(&step.actions, output);
                }
            }
            _ => {}
        }
    }
}

fn actions(scenario: &ScenarioConfig) -> Vec<&EventActionConfig> {
    let mut output = Vec::new();
    for event in &scenario.events {
        walk(&event.actions, &mut output);
    }
    output
}

#[test]
fn every_activity_is_replayable_and_continues_only_to_its_neighbor() {
    let content = content();
    let campaign = content
        .iter()
        .find_map(|item| match item {
            Content::Campaign(campaign) => Some(campaign),
            _ => None,
        })
        .unwrap();
    assert_eq!(campaign.scenarios, ACTIVITIES);
    assert_eq!(
        content
            .iter()
            .filter(|c| matches!(c, Content::Scenario(_)))
            .count(),
        6
    );
    for (index, scenario) in activities(&content).iter().enumerate() {
        assert!(
            scenario.role.picker_lists(),
            "a campaign member is a chapter the picker can list"
        );
        let actions = actions(scenario);
        let transitions: BTreeSet<_> = actions
            .iter()
            .filter_map(|action| match action {
                EventActionConfig::NextScenario(next) => {
                    assert!(next.linger, "preserve the outcome banner");
                    Some(next.scenario_id.as_str())
                }
                _ => None,
            })
            .collect();
        let expected: BTreeSet<_> = ACTIVITIES[index..=(index + 1).min(5)]
            .iter()
            .copied()
            .collect();
        assert_eq!(
            transitions, expected,
            "{}: retry and continuation only",
            scenario.id
        );
        assert_eq!(
            actions
                .iter()
                .filter(|a| matches!(a,
                    EventActionConfig::Outcome(o) if o.outcome == ScenarioOutcomeKind::Victory
                ))
                .count(),
            1
        );
        assert!(
            scenario.events.iter().any(|event| {
                event.actions.iter().any(|a| {
                    matches!(a, EventActionConfig::Outcome(o)
                if o.outcome == ScenarioOutcomeKind::Defeat)
                }) && event.actions.iter().any(|a| {
                    matches!(a, EventActionConfig::NextScenario(n)
                if n.scenario_id == scenario.id)
                })
            }),
            "{} has a local retry",
            scenario.id
        );
    }
}

#[test]
fn activities_open_with_an_objective_and_without_a_story_scene() {
    let content = content();
    for scenario in activities(&content) {
        let start = scenario
            .events
            .iter()
            .find(|e| matches!(e.name, EventConfig::OnStart))
            .unwrap();
        assert!(
            start
                .actions
                .iter()
                .any(|a| matches!(a, EventActionConfig::Objective(_))),
            "{} must be playable immediately",
            scenario.id
        );
        assert!(
            actions(scenario).iter().all(|a| !matches!(
                a,
                EventActionConfig::Cinematic(_)
                    | EventActionConfig::SuspendPlayerControl(_)
                    | EventActionConfig::NarrativeCue(_)
            )),
            "{} must not interrupt play for a story scene",
            scenario.id
        );
        assert_eq!(
            actions(scenario)
                .iter()
                .filter(|a| matches!(a, EventActionConfig::Objective(_)))
                .count(),
            1,
            "one concise objective, not a stream of instructions"
        );
        for action in actions(scenario) {
            if let EventActionConfig::SpawnScenarioObject(object) = action {
                if let ScenarioObjectKind::Beacon(beacon) = &object.kind {
                    assert!(
                        beacon.label.len() <= 16,
                        "short navigation labels: {}",
                        beacon.label
                    );
                    let label = beacon.label.to_ascii_lowercase();
                    for hint in ["needed", "hold", "seconds", "upload", "no kill"] {
                        assert!(!label.contains(hint), "instructions belong in objectives");
                    }
                }
            }
        }
    }
}

#[test]
fn every_map_contains_a_dense_reproducible_field_planets_and_moving_traffic() {
    let content = content();
    for scenario in activities(&content) {
        let (mut rocks, mut planets, mut moving) = (0, 0, 0);
        let mut seeds = BTreeSet::new();
        for action in actions(scenario) {
            match action {
                EventActionConfig::ScatterObjects(field) => {
                    assert!(seeds.insert(field.seed), "distinct seeded fields");
                    assert!(field.asteroid_radius.is_some());
                    assert!(field.asteroid_kinds.len() >= 3, "mixed materials");
                    let ScenarioObjectKind::Asteroid(rock) = &field.template.kind else {
                        panic!("the counted field must actually contain rocks")
                    };
                    assert_eq!(rock.mass, Some(0.0));
                    assert!(!rock.invulnerable, "terrain remains carvable");
                    rocks += field.count;
                }
                EventActionConfig::SpawnScenarioObject(object) => match &object.kind {
                    ScenarioObjectKind::Planet(_) => planets += 1,
                    ScenarioObjectKind::Asteroid(_) => rocks += 1,
                    ScenarioObjectKind::Spaceship(ship)
                        if object.base.id.starts_with("traffic_") =>
                    {
                        let SpaceshipController::AI(ai) = &ship.controller else {
                            panic!("traffic needs a driver")
                        };
                        assert!(ai.patrol.len() >= 2);
                        moving += 1;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        assert!(
            (50..=100).contains(&rocks),
            "{}: bounded dense terrain",
            scenario.id
        );
        assert!(planets >= 2, "{}: planetary backdrop", scenario.id);
        assert!(moving >= 2, "{}: real moving traffic", scenario.id);
    }
}

#[test]
fn every_player_ship_uses_the_raised_speed_governor() {
    let content = content();
    for scenario in activities(&content) {
        let player = actions(scenario)
            .into_iter()
            .find_map(|action| match action {
                EventActionConfig::SpawnScenarioObject(object)
                    if object.base.id == "player_spaceship" =>
                {
                    let ScenarioObjectKind::Spaceship(ship) = &object.kind else {
                        panic!("player ship")
                    };
                    Some(ship)
                }
                _ => None,
            })
            .unwrap();
        let SpaceshipController::Player(controller) = &player.controller else {
            panic!("player driver")
        };
        assert_eq!(
            controller.speed_cap,
            Some(nova_events::prelude::MetersPerSecond(500.0)),
            "{} uses the campaign governor",
            scenario.id
        );
    }
}

#[test]
fn the_racer_course_arms_only_the_next_gate_and_freezes_its_clock_at_finish() {
    let content = content();
    let scenario = activities(&content)[0];
    let actions = actions(scenario);
    let player = actions
        .iter()
        .find_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object)
                if object.base.id == "player_spaceship" =>
            {
                let ScenarioObjectKind::Spaceship(ship) = &object.kind else {
                    panic!("player ship")
                };
                Some(ship)
            }
            _ => None,
        })
        .unwrap();
    assert!(matches!(&player.design, ShipDesignSource::Prototype { id, .. } if id == "racer"));
    let SpaceshipController::Player(controller) = &player.controller else {
        panic!("player driver")
    };
    assert!(
        controller.input_mapping.is_empty(),
        "race with an unarmed Racer"
    );
    let start = actions
        .iter()
        .find_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) if object.base.id == "start" => {
                Some(object)
            }
            _ => None,
        })
        .expect("the start gate");
    let ScenarioObjectKind::Beacon(start_beacon) = &start.kind else {
        panic!("start beacon")
    };
    assert_eq!(
        start_beacon.area_radius,
        Some(nova_events::prelude::Meters(50.0))
    );
    for (id, next) in [
        ("start", "gate_1"),
        ("gate_1", "gate_2"),
        ("gate_2", "gate_3"),
        ("gate_3", "gate_4"),
        ("gate_4", "finish"),
    ] {
        let gate = scenario
            .events
            .iter()
            .find(|event| {
                matches!(event.name, EventConfig::OnEnter)
                    && event.filters.iter().any(|f| {
                        matches!(f, EventFilterConfig::Entity(e)
                if e.id.as_deref() == Some(id))
                    })
            })
            .unwrap();
        assert!(gate.once);
        assert!(gate
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::DespawnScenarioObject(o)
            if o.id == id)));
        assert!(
            gate.actions.iter().any(|action| {
                matches!(action, EventActionConfig::PlaySound(cue)
                if cue.sound.path() == Some("dep://base/sounds/radar_retarget.wav")
                    && cue.route == SoundRouteConfig::Interface
                    && cue.volume == Some(0.7))
            }),
            "{id} crossing has one cockpit checkpoint cue"
        );
        let spawned: Vec<_> = gate
            .actions
            .iter()
            .filter_map(|a| match a {
                EventActionConfig::SpawnScenarioObject(o) => Some(o.base.id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(spawned, [next]);
        let next_beacon = gate
            .actions
            .iter()
            .find_map(|action| match action {
                EventActionConfig::SpawnScenarioObject(object) if object.base.id == next => {
                    Some(object)
                }
                _ => None,
            })
            .expect("the next gate");
        let ScenarioObjectKind::Beacon(next_beacon) = &next_beacon.kind else {
            panic!("next beacon")
        };
        assert_eq!(
            next_beacon.area_radius,
            Some(nova_events::prelude::Meters(50.0)),
            "{next} requires a direct pass"
        );
    }
    let finish = scenario
        .events
        .iter()
        .find(|event| {
            event.filters.iter().any(
                |f| matches!(f, EventFilterConfig::Entity(e) if e.id.as_deref() == Some("finish")),
            )
        })
        .unwrap();
    assert!(finish
        .actions
        .iter()
        .any(|a| matches!(a, EventActionConfig::VariableSet(v)
        if v.key == "racing"
            && ron::to_string(&v.expression).unwrap() == "Term(Factor(Literal(Number(0.0))))")));
    assert!(finish
        .actions
        .iter()
        .any(|a| matches!(a, EventActionConfig::VariableSet(v)
        if v.key == "lap_time")));
}

#[test]
fn survey_sites_can_restart_without_repeating_completed_sites() {
    let content = content();
    let scenario = activities(&content)[2];
    for id in ["site_a", "site_b", "site_c"] {
        for kind in [EventConfig::OnEnter, EventConfig::OnExit] {
            let event = scenario
                .events
                .iter()
                .find(|event| {
                    event.name == kind
                        && event.filters.iter().any(|f| {
                            matches!(f, EventFilterConfig::Entity(e)
                    if e.id.as_deref() == Some(id))
                        })
                })
                .unwrap();
            assert!(
                !event.once,
                "site {id} permits an interrupted scan to restart"
            );
            assert!(event
                .filters
                .iter()
                .any(|f| matches!(f, EventFilterConfig::Expression(_))));
            if kind == EventConfig::OnExit {
                assert!(event
                    .actions
                    .iter()
                    .any(|a| matches!(a, EventActionConfig::TimerCancel(t)
                    if t.key == id)));
            }
        }
        let timer = scenario
            .events
            .iter()
            .find(|event| {
                matches!(event.name, EventConfig::OnTimerEnd)
                    && event
                        .filters
                        .iter()
                        .any(|f| matches!(f, EventFilterConfig::Timer(t) if t.key == id))
            })
            .unwrap();
        assert!(timer.once);
        assert!(timer
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::DespawnScenarioObject(o)
            if o.id == id)));
    }
}

#[test]
fn convoy_victory_requires_two_real_arrivals_and_either_loss_has_a_retry() {
    let content = content();
    let scenario = activities(&content)[4];
    let escort = actions(scenario)
        .into_iter()
        .find_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) if object.base.id == "escort" => {
                Some(object)
            }
            _ => None,
        })
        .expect("the convoy keeps its escort");
    assert!(
        escort.base.position.0.z >= 2_000.0,
        "the escort starts behind the player: {:?}",
        escort.base.position
    );
    let ScenarioObjectKind::Spaceship(escort_ship) = &escort.kind else {
        panic!("escort ship")
    };
    let SpaceshipController::AI(escort_ai) = &escort_ship.controller else {
        panic!("escort driver")
    };
    assert!(
        escort_ai
            .patrol
            .first()
            .is_some_and(|point| point.0.z >= 2_000.0),
        "the escort route starts behind the player"
    );
    let departure = scenario
        .events
        .iter()
        .find(|event| {
            matches!(event.name, EventConfig::OnEnter)
                && event.filters.iter().any(|filter| {
                    matches!(filter, EventFilterConfig::Entity(entity)
                        if entity.id.as_deref() == Some("departure"))
                })
        })
        .expect("the departure trigger");
    let mut departure_actions = Vec::new();
    walk(&departure.actions, &mut departure_actions);
    let opening_interceptors: BTreeSet<_> = departure_actions
        .iter()
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object)
                if object.base.id.starts_with("interceptor_") =>
            {
                let ScenarioObjectKind::Spaceship(ship) = &object.kind else {
                    panic!("interceptor ship")
                };
                let SpaceshipController::AI(ai) = &ship.controller else {
                    panic!("interceptor driver")
                };
                assert!(ai.engage_delay.is_some_and(|delay| delay <= 5.0));
                assert!(object.base.position.0.z > -5_000.0);
                Some(object.base.id.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        opening_interceptors,
        BTreeSet::from(["interceptor_a", "interceptor_b"]),
        "both blockers arrive before the convoy midpoint"
    );
    let lane_cluster = actions(scenario)
        .into_iter()
        .find_map(|action| match action {
            EventActionConfig::ScatterObjects(scatter) if scatter.id_prefix == "lane_cluster_" => {
                Some(scatter)
            }
            _ => None,
        })
        .expect("the lane has a seeded depth cluster");
    assert_eq!(lane_cluster.count, 12);
    assert_eq!(
        lane_cluster.asteroid_radius,
        Some((
            nova_events::prelude::Meters(10.0),
            nova_events::prelude::Meters(43.0)
        ))
    );
    assert!(lane_cluster.asteroid_kinds.len() >= 3);
    for order in ["a_late", "b_late"] {
        assert!(
            actions(scenario).iter().any(|action| {
                matches!(action, EventActionConfig::MoveShipTo(move_ship)
                    if move_ship.order == order)
            }),
            "the longer route includes {order}"
        );
    }
    for order in ["a_dock", "b_dock"] {
        let berth = actions(scenario)
            .into_iter()
            .find_map(|action| match action {
                EventActionConfig::MoveShipTo(move_ship) if move_ship.order == order => {
                    Some(move_ship)
                }
                _ => None,
            })
            .expect("the final berth order");
        assert!(
            berth.position.0.z <= -8_900.0,
            "{order} completes an 8 km route"
        );
    }
    let arrivals: Vec<_> = scenario
        .events
        .iter()
        .filter(|event| {
            matches!(event.name, EventConfig::OnShipOrderComplete)
                && event.actions.iter().any(|a| {
                    matches!(a, EventActionConfig::VariableSet(v)
            if v.key == "arrived")
                })
        })
        .collect();
    assert_eq!(arrivals.len(), 2);
    for id in ["transport_a", "transport_b"] {
        assert!(scenario.events.iter().any(|event| {
            matches!(event.name, EventConfig::OnDefeated)
                && event.filters.iter().any(|f| {
                    matches!(f, EventFilterConfig::Entity(e)
                if e.id.as_deref() == Some(id))
                })
                && event.actions.iter().any(|a| {
                    matches!(a, EventActionConfig::Outcome(o)
                if o.outcome == ScenarioOutcomeKind::Defeat)
                })
        }));
    }
    let victory = scenario
        .events
        .iter()
        .find(|event| {
            event.actions.iter().any(|a|
        matches!(a, EventActionConfig::Outcome(o) if o.outcome == ScenarioOutcomeKind::Victory))
        })
        .unwrap();
    assert!(matches!(victory.name, EventConfig::OnUpdate));
    let filters = ron::to_string(&victory.filters).unwrap();
    assert!(filters.contains("arrived") && filters.contains("GreaterThan"));
    assert!(
        !filters.contains("cleared"),
        "an escort ends on arrivals, not kills"
    );
}

#[test]
fn the_platform_stays_within_its_encounter_health_budget() {
    let content = content();
    let ship = content
        .iter()
        .find_map(|item| match item {
            Content::Ship(ship) if ship.id == "ledger_platform" => Some(ship),
            _ => None,
        })
        .unwrap();
    let health: f32 = ship
        .design
        .sections
        .iter()
        .map(|section| {
            let SectionSource::Prototype { id, patch } = &section.source else {
                panic!("catalog part")
            };
            patch.health.unwrap_or_else(|| {
                content
                    .iter()
                    .find_map(|item| match item {
                        Content::Section(config) if config.base.id == *id => {
                            Some(config.base.health)
                        }
                        _ => None,
                    })
                    .expect("unpatched platform part resolves in the mod")
            })
        })
        .sum();
    assert!(
        health <= 3500.0,
        "platform health {health} exceeds its encounter budget"
    );
}

#[test]
fn the_player_corvette_protects_the_nose_that_connects_both_guns() {
    let content = content();
    for index in [1, 3, 4] {
        let scenario = activities(&content)[index];
        let player = actions(scenario)
            .into_iter()
            .find_map(|a| match a {
                EventActionConfig::SpawnScenarioObject(object)
                    if object.base.id == "player_spaceship" =>
                {
                    let ScenarioObjectKind::Spaceship(ship) = &object.kind else {
                        panic!("player ship")
                    };
                    Some(ship)
                }
                _ => None,
            })
            .unwrap();
        let ShipDesignSource::Prototype {
            section_patches, ..
        } = &player.design
        else {
            panic!("the player flies a catalog design")
        };
        assert_eq!(
            section_patches
                .get("nose")
                .and_then(|patch| patch.config.health),
            Some(1200.0),
            "{}: do not lose both guns through a lightly plated nose",
            scenario.id
        );
    }
}

fn muzzle_rate(joint: &TurretJoint) -> f32 {
    joint.muzzle.as_ref().map_or(0.0, |m| m.fire_rate)
        + joint.children.iter().map(muzzle_rate).sum::<f32>()
}

#[test]
fn modelled_critical_sections_and_campaign_guns_have_a_reaction_margin() {
    let content = content();
    let sections: Vec<_> = content
        .iter()
        .filter_map(|item| match item {
            Content::Section(section) => Some(section),
            _ => None,
        })
        .collect();
    let get = |id: &str| *sections.iter().find(|s| s.base.id == id).unwrap();
    // Fixed content margins, not timing assertions or a proxy for pilot aim.
    assert_eq!(get("cargoa_engine_port").base.health, 140.0);
    assert_eq!(get("cargoa_fuselage").base.health, 700.0);
    for (id, rate, damage) in [("ledger_pdc", 35.0, 4.0), ("ledger_pdc_patrol", 25.0, 2.0)] {
        let SectionKind::Turret(gun) = &get(id).kind else {
            panic!("{id} is a turret")
        };
        assert_eq!(muzzle_rate(&gun.root), rate);
        assert_eq!(gun.bullet_damage, damage);
        assert!(
            gun.ammunition.rounds().is_some(),
            "finite magazines remain gameplay"
        );
    }
    for item in &content {
        if let Content::Ship(ship) = item {
            for section in &ship.design.sections {
                if let SectionSource::Prototype { id, .. } = &section.source {
                    assert_ne!(
                        id, "pdc_kinetic_turret_section",
                        "{} owns its gun balance",
                        ship.id
                    );
                }
            }
        }
    }
}
