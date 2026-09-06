//! Structural pins for Basic Training.
//!
//! These enforce the SHAPE the card is built to - a lesson's verb, card and
//! beat land together and a beat after the line, every mark comes down, every
//! withheld verb comes back, a cadet who works ahead of the card is never
//! stalled - rather than a transcript of the script. A dialogue or pacing
//! pass should be able to move every line and every delay without touching
//! an assertion below.

use std::collections::{BTreeSet, HashSet};

use super::*;
use crate::base_content::{assets::BaseContentAssets, ships};

fn config() -> ScenarioConfig {
    tutorial(AssetRef::default(), AssetRef::default())
}

/// Every action in the scenario, flattened through every chain.
fn all_actions(config: &ScenarioConfig) -> Vec<EventActionConfig> {
    let mut actions = Vec::new();
    for action in config.events.iter().flat_map(|event| event.actions.iter()) {
        action.walk(&mut |action| actions.push(action.clone()));
    }
    actions
}

fn spawned(config: &ScenarioConfig, id: &str) -> Option<ScenarioObjectConfig> {
    all_actions(config)
        .into_iter()
        .find_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) if object.base.id == id => Some(object),
            _ => None,
        })
}

#[test]
fn no_handler_posts_an_objective_alongside_a_conversation() {
    let config = config();
    for (idx, event) in config.events.iter().enumerate() {
        for group in event.action_groups() {
            let has_line = group
                .iter()
                .any(|a| matches!(a, EventActionConfig::NarrativeCue(_)));
            let has_objective = group
                .iter()
                .any(|a| matches!(a, EventActionConfig::Objective(_)));
            assert!(
                !(has_line && has_objective),
                "handler #{idx} ({:?}) posts an objective in the same frame as a comms \
                 line - give the objective a beat of its own (pacing::beat_later)",
                event.name,
            );
        }
    }
}

#[test]
fn the_opening_panel_stays_empty_until_the_briefing_hands_over() {
    let config = config();
    let start = config
        .events
        .iter()
        .find(|event| matches!(event.name, EventConfig::OnStart))
        .expect("the range has an OnStart handler");
    assert!(
        !start
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::Objective(_))),
        "OnStart posts an objective in its own frame"
    );
    let deferred = start
        .action_groups()
        .into_iter()
        .skip(1)
        .filter(|group| {
            group
                .iter()
                .any(|a| matches!(a, EventActionConfig::Objective(_)))
        })
        .count();
    assert_eq!(
        deferred, 1,
        "the burn card posts on one deferred beat of the briefing"
    );
}

#[test]
fn every_lesson_grants_its_verb_in_the_same_step_as_its_card() {
    // A verb granted at the line arms the lesson's handler before its card
    // exists; a verb granted after the card leaves a cadet reading an order
    // they cannot follow. The grant rides the card's step.
    let config = config();
    let mut grants_beside_cards = 0;
    for event in &config.events {
        for group in event.action_groups() {
            let grants = group
                .iter()
                .filter(|a| matches!(a, EventActionConfig::SetControllerVerb(_)))
                .count();
            if grants == 0 {
                continue;
            }
            assert!(
                group
                    .iter()
                    .any(|a| matches!(a, EventActionConfig::Objective(_))),
                "a verb is granted in a step that posts no card"
            );
            grants_beside_cards += grants;
        }
    }
    assert_eq!(
        grants_beside_cards, 5,
        "STOP, RCS, LOCK, GOTO and ORBIT are each granted once"
    );
}

#[test]
fn every_withheld_verb_the_card_teaches_is_handed_back() {
    let config = config();
    let trainer = spawned(&config, ID_TRAINER).expect("the trainer spawns");
    let ScenarioObjectKind::Spaceship(ship) = trainer.kind else {
        panic!("the trainer is a spaceship");
    };
    let withheld: BTreeSet<String> = ship
        .modifications
        .iter()
        .flat_map(|section| section.modifications.iter())
        .filter_map(|m| match m {
            SectionModification::DisableVerb(verb) => Some(format!("{verb:?}")),
            _ => None,
        })
        .collect();
    let granted: BTreeSet<String> = all_actions(&config)
        .into_iter()
        .filter_map(|action| match action {
            EventActionConfig::SetControllerVerb(grant) if grant.enabled => {
                Some(format!("{:?}", grant.verb))
            }
            _ => None,
        })
        .collect();
    let taught: BTreeSet<String> = ["Stop", "Rcs", "Lock", "Goto", "Orbit"]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(
        granted, taught,
        "the card grants exactly the verbs it teaches"
    );
    assert_eq!(
        withheld, taught,
        "every verb withheld at spawn is taught, and every taught verb is withheld so its \
         lesson cannot be finished early"
    );
}

/// The autopilot drill: a travel lock on the planetoid unlocks GOTO, parking
/// off it unlocks ORBIT, a stable orbit raises the mark home, and the mark's
/// gate opens the gun. The two places the drill parks are pinned with the
/// engine's own rules: the planetoid's reach contains GOTO's park point, and
/// CHARLIE's gate contains it too.
#[test]
fn the_autopilot_is_taught_out_to_the_planetoid_and_home() {
    let config = config();
    let filtered_on = |event: &ScenarioEventConfig, filter: &EventFilterConfig| {
        let wanted = format!("{filter:?}");
        event.filters.iter().any(|f| format!("{f:?}") == wanted)
    };
    let grants = |event: &ScenarioEventConfig| -> Vec<FlightVerb> {
        let mut verbs = Vec::new();
        for action in &event.actions {
            action.walk(&mut |action| {
                if let EventActionConfig::SetControllerVerb(grant) = action {
                    if grant.enabled {
                        verbs.push(grant.verb);
                    }
                }
            });
        }
        verbs
    };
    let handler = |name: &str, filter: EventFilterConfig| -> ScenarioEventConfig {
        config
            .events
            .iter()
            .find(|event| format!("{:?}", event.name) == name && filtered_on(event, &filter))
            .cloned()
            .unwrap_or_else(|| panic!("a {name} handler filtered on {filter:?}"))
    };

    let travel_lock = handler("OnTravelLockStart", trainer_at(ID_PLANETOID));
    assert_eq!(grants(&travel_lock), vec![FlightVerb::Goto]);
    let parked = handler("OnGotoComplete", trainer_at(ID_PLANETOID));
    assert_eq!(grants(&parked), vec![FlightVerb::Orbit]);
    let stable = handler("OnOrbitStable", trainer_at(ID_PLANETOID));
    let raises_charlie = all_actions(&ScenarioConfig {
        events: vec![stable],
        ..config.clone()
    })
    .into_iter()
    .any(|action| matches!(action, EventActionConfig::SpawnScenarioObject(object) if object.base.id == MARK_CHARLIE.id));
    assert!(raises_charlie, "a stable orbit raises the mark home");
    let home = handler("OnEnter", MARK_CHARLIE.gate_entered());
    let opens_the_gun = all_actions(&ScenarioConfig {
        events: vec![home],
        ..config.clone()
    })
    .into_iter()
    .any(|action| matches!(action, EventActionConfig::Objective(card) if card.id == OBJ_LOCK));
    assert!(opens_the_gun, "CHARLIE's gate opens the gun's lock");

    let planetoid = spawned(&config, ID_PLANETOID).expect("the planetoid spawns");
    let ScenarioObjectKind::Planet(planet) = planetoid.kind else {
        panic!("the planetoid is a planet");
    };
    let mass = planet.mass.expect("the planetoid is anchored, so it pulls");
    let surface = planet.body_radius().to_engine();
    let gravity = GravitySettings::default();
    let flight = FlightSettings::default();
    let well = GravityWell::from_mass(mass, surface, &gravity);
    let standoff = flight.arrival_standoff;
    let park = surface + standoff;
    // The card sends the cadet from GOTO's park point straight into ORBIT, so
    // the park point has to be a ring ORBIT will actually fly. The band is the
    // engine's own rule (a floor over the surface, a ceiling inside the well's
    // unfaded core), and it is published for exactly this: content that gates
    // progress on a ring must be sized against the rings the verb hands out,
    // not against a re-derivation of the arithmetic.
    let (min, max) = orbit_radius_band(&well, &gravity, &flight)
        .expect("the planetoid must be orbitable at all");
    assert!(
        park > min && park < max,
        "GOTO parks {park} u from the planetoid's centre; ORBIT's band is {min}..{max} u"
    );

    // And the cadet's clock. GOTO hands the trainer back at the park point with
    // ORBIT still withheld, so the trainer is falling while Range Control talks
    // it through the key: the mass has to leave enough fall to hear the line,
    // read the card and answer. Radial free-fall from rest, closed form.
    let floor = surface + gravity.surface_margin;
    let ratio = floor / park;
    let fall_seconds = (park.powi(3) / (2.0 * well.mu)).sqrt()
        * (ratio.sqrt().acos() + (ratio * (1.0 - ratio)).sqrt());
    assert!(
        fall_seconds > 8.0,
        "a cadet parked off the planetoid has {fall_seconds} s before the rock; the ORBIT \
         line and its card need more room than that"
    );
    assert!(
        MARK_CHARLIE.area.to_engine() > standoff + Meters(100.0).to_engine(),
        "CHARLIE's gate must contain the park point an arrival standoff short of it"
    );
}

#[test]
fn every_mark_the_range_raises_is_taken_back_down_with_its_gate() {
    let config = config();
    let actions = all_actions(&config);
    let raised: BTreeSet<String> = actions
        .iter()
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object)
                if matches!(object.kind, ScenarioObjectKind::Beacon(_)) =>
            {
                Some(object.base.id.clone())
            }
            EventActionConfig::CreateScenarioArea(area) if area.id != ID_RANGE_BOUNDARY => {
                Some(area.id.clone())
            }
            _ => None,
        })
        .collect();
    let lowered: BTreeSet<String> = actions
        .iter()
        .filter_map(|action| match action {
            EventActionConfig::DespawnScenarioObject(despawn) => Some(despawn.id.clone()),
            _ => None,
        })
        .collect();
    assert!(!raised.is_empty(), "the pattern has marks");
    let left_up: Vec<_> = raised.difference(&lowered).collect();
    assert!(
        left_up.is_empty(),
        "every mark and gate raised is despawned again; still up: {left_up:?}"
    );
    assert!(
        !lowered.contains(ID_RANGE_BOUNDARY),
        "the boundary is the range's edge and stays up for the whole card"
    );
}

#[test]
fn every_card_posted_is_completed() {
    let config = config();
    let actions = all_actions(&config);
    let posted: BTreeSet<String> = actions
        .iter()
        .filter_map(|action| match action {
            EventActionConfig::Objective(objective) => Some(objective.id.clone()),
            _ => None,
        })
        .collect();
    let completed: BTreeSet<String> = actions
        .iter()
        .filter_map(|action| match action {
            EventActionConfig::ObjectiveComplete(objective) => Some(objective.id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(posted, completed);
}

#[test]
fn a_target_shot_apart_ahead_of_its_lesson_moves_the_card_on() {
    // Target 1 can die in the lock lesson or the fire lesson; either way the
    // line's card follows, and the tally that closes the line counts a target
    // whenever it died.
    let config = config();
    let gate = format!("{:?}", number_greater_than(VAR_TARGET_1_DOWN, 0.5));
    let target_1_down = |event: &ScenarioEventConfig| {
        event
            .filters
            .iter()
            .any(|filter| format!("{filter:?}") == gate)
    };
    let posts_line_card = |event: &ScenarioEventConfig| {
        let mut posts = false;
        for action in &event.actions {
            action.walk(&mut |action| {
                if let EventActionConfig::Objective(objective) = action {
                    posts |= objective.id == OBJ_LINE;
                }
            });
        }
        posts
    };
    let hand_offs = config
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnUpdate) && target_1_down(event))
        .filter(|event| posts_line_card(event))
        .count();
    assert_eq!(
        hand_offs, 2,
        "both fire lessons hand off to the line's card"
    );

    let tallies = config
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnDestroyed))
        .filter(|event| {
            event.filters.len() == 1
                && event
                    .actions
                    .iter()
                    .any(|a| matches!(a, EventActionConfig::VariableSet(_)))
        })
        .count();
    assert_eq!(
        tallies, TARGET_COUNT,
        "every target is tallied without a beat gate"
    );
}

#[test]
fn the_drones_start_dormant_and_are_woken_by_the_script() {
    let config = config();
    for (id, _, _) in DRONES {
        let drone = spawned(&config, id).unwrap_or_else(|| panic!("{id} spawns"));
        let ScenarioObjectKind::Spaceship(ship) = drone.kind else {
            panic!("{id} is a spaceship");
        };
        assert_eq!(ship.allegiance, Some(Allegiance::Neutral));
        assert!(matches!(ship.controller, SpaceshipController::AI(_)));
        let woken = all_actions(&config).into_iter().any(|action| {
            matches!(action, EventActionConfig::SetAllegiance(set) if set.id == id && set.allegiance == Allegiance::Enemy)
        });
        assert!(woken, "{id} is woken by the script");
    }
}

/// The drone fight is a first fight: every drone section is fragile and the
/// drone's gun is a hard magazine, while the trainer's gun and bridge outlast
/// everything both drones can fire. The numbers are the range's own; this
/// pins that they cover the whole hull and that the arithmetic holds.
#[test]
fn the_drones_are_handicapped_and_the_trainer_cannot_be_shot_dry_of_its_gun() {
    let config = config();
    let hull_sections: BTreeSet<String> = ships::picket_section_ids().into_iter().collect();
    for (id, _, _) in DRONES {
        let drone = spawned(&config, id).unwrap_or_else(|| panic!("{id} spawns"));
        let ScenarioObjectKind::Spaceship(ship) = drone.kind else {
            panic!("{id} is a spaceship");
        };
        let mut softened = BTreeSet::new();
        let mut magazines = Vec::new();
        for modification in &ship.modifications {
            for delta in &modification.modifications {
                match delta {
                    SectionModification::SetHealth(health) => {
                        assert!(
                            *health <= DRONE_SECTION_HEALTH,
                            "{id}: '{}' is softened to {health}",
                            modification.section
                        );
                        softened.insert(modification.section.clone());
                    }
                    SectionModification::SetAmmo(rounds) => {
                        magazines.push((modification.section.clone(), *rounds));
                    }
                    other => panic!("{id}: unexpected spawn delta {other:?}"),
                }
            }
        }
        assert_eq!(
            softened, hull_sections,
            "{id}: every section of the hull is softened"
        );
        assert_eq!(
            magazines,
            vec![(TRAINER_GUN.to_string(), DRONE_MAGAZINE)],
            "{id}: exactly its gun carries the hard magazine"
        );
    }

    let trainer = spawned(&config, ID_TRAINER).expect("the trainer spawns");
    let ScenarioObjectKind::Spaceship(ship) = trainer.kind else {
        panic!("the trainer is a spaceship");
    };
    let armour = |section: &str| -> f32 {
        ship.modifications
            .iter()
            .filter(|m| m.section == section)
            .flat_map(|m| m.modifications.iter())
            .find_map(|delta| match delta {
                SectionModification::SetHealth(health) => Some(*health),
                _ => None,
            })
            .unwrap_or_else(|| panic!("the trainer's '{section}' is armoured"))
    };
    let catalog = ships::ship_catalog(&BaseContentAssets::from_paths());
    let sections = crate::base_content::sections::section_catalog(&BaseContentAssets::from_paths());
    let gun_prototype = catalog
        .iter()
        .find(|entry| entry.id == ships::BLOCK_PICKET_SHIP_ID)
        .and_then(|entry| entry.hull.sections.iter().find(|s| s.id == TRAINER_GUN))
        .map(|s| match &s.source {
            SectionSource::Prototype(id) => id.clone(),
            other => panic!("the picket's gun is a prototype, not {other:?}"),
        })
        .expect("the picket carries the gun");
    let per_hit = sections
        .iter()
        .find(|s| s.base.id == gun_prototype)
        .and_then(|s| match &s.kind {
            SectionKind::Turret(turret) => Some(turret.bullet_damage),
            _ => None,
        })
        .expect("the gun is a turret with an authored per-hit damage");
    let worst_case = DRONES.len() as f32 * DRONE_MAGAZINE as f32 * per_hit;
    assert!(
        armour(TRAINER_GUN) > worst_case,
        "both drones' magazines ({worst_case}) cannot take the trainer's gun off"
    );
    assert!(
        armour(ships::BLOCK_BRIDGE_SECTION_ID) > worst_case,
        "both drones' magazines ({worst_case}) cannot take the trainer's bridge off"
    );
}

/// A drone that loses its drive or its flight computer keeps its speed, and
/// a cadet cannot always run it down. The range boundary catches it: a drone
/// under control can never reach the boundary, a drone that coasts out is
/// counted and taken away, and no drone is counted twice however it goes.
#[test]
fn a_drone_that_coasts_off_the_range_is_counted_once_and_taken_away() {
    let config = config();
    let boundary = all_actions(&config)
        .into_iter()
        .find_map(|action| match action {
            EventActionConfig::CreateScenarioArea(area) if area.id == ID_RANGE_BOUNDARY => {
                Some(area)
            }
            _ => None,
        })
        .expect("the range raises its boundary");
    let counted = format!("{:?}", increment_variable(VAR_DRONES_DOWN));
    for (id, _, station) in DRONES {
        let reach = (station.0 - boundary.position.0).length() + DRONE_LEASH.0;
        assert!(
            reach < boundary.radius.0,
            "{id}: a drone under control (station plus leash, {reach} m) can reach the \
             boundary ({} m)",
            boundary.radius.0
        );

        let fresh = format!("{:?}", number_equals(drone_down_var(id), 0.0));
        let flagged = format!("{:?}", set_number(drone_down_var(id), 1.0));
        let tallies: Vec<&ScenarioEventConfig> = config
            .events
            .iter()
            .filter(|event| event.actions.iter().any(|a| format!("{a:?}") == flagged))
            .collect();
        assert_eq!(
            tallies.len(),
            2,
            "{id} is tallied by its defeat and by the boundary"
        );
        for event in &tallies {
            assert!(
                event.filters.iter().any(|f| format!("{f:?}") == fresh),
                "{id}: a tally runs only while the drone is uncounted"
            );
            assert!(
                event.actions.iter().any(|a| format!("{a:?}") == counted),
                "{id}: a tally counts the drone"
            );
        }
        assert!(
            tallies
                .iter()
                .any(|event| matches!(event.name, EventConfig::OnDefeated)),
            "{id}: a defeat counts"
        );
        let adrift = tallies
            .iter()
            .find(|event| matches!(event.name, EventConfig::OnExit))
            .unwrap_or_else(|| panic!("{id}: leaving the range counts"));
        let left = format!("{:?}", drone_left_range(id));
        assert!(
            adrift.filters.iter().any(|f| format!("{f:?}") == left),
            "{id}: the exit is the range boundary's"
        );
        assert!(
            adrift.actions.iter().any(
                |a| matches!(a, EventActionConfig::DespawnScenarioObject(gone) if gone.id == id)
            ),
            "{id}: the boundary takes the drone away"
        );
        assert!(
            adrift
                .actions
                .iter()
                .any(|a| matches!(a, EventActionConfig::NarrativeCue(_))),
            "{id}: Range Control calls the drone adrift"
        );
    }
    let counting = config
        .events
        .iter()
        .filter(|event| event.actions.iter().any(|a| format!("{a:?}") == counted))
        .count();
    assert_eq!(
        counting,
        DRONES.len() * 2,
        "no handler counts a drone without flagging it"
    );
}

#[test]
fn the_trainer_fires_a_gun_its_hull_actually_carries() {
    let config = config();
    let trainer = spawned(&config, ID_TRAINER).expect("the trainer spawns");
    let ScenarioObjectKind::Spaceship(ship) = trainer.kind else {
        panic!("the trainer is a spaceship");
    };
    let SpaceshipController::Player(player) = ship.controller else {
        panic!("the trainer is the player's");
    };
    let ShipSource::Prototype(hull_id) = &ship.hull else {
        panic!("the trainer flies a catalog hull");
    };
    let catalog = ships::ship_catalog(&BaseContentAssets::from_paths());
    let hull = catalog
        .iter()
        .find(|entry| entry.id == *hull_id)
        .expect("the trainer's hull is in the catalog");
    for section in player.input_mapping.keys() {
        assert!(
            hull.hull.sections.iter().any(|s| s.id == *section),
            "input mapping names section '{section}' which '{hull_id}' does not carry"
        );
    }
    assert!(
        player.input_mapping.contains_key(TRAINER_GUN),
        "the fire lesson's gun is bound"
    );
}

#[test]
fn every_voice_on_the_channel_has_a_face_of_its_own() {
    let config = config();
    let mut faces: HashSet<String> = HashSet::new();
    let mut speakers: HashSet<String> = HashSet::new();
    for action in all_actions(&config) {
        let EventActionConfig::NarrativeCue(cue) = action else {
            continue;
        };
        speakers.insert(cue.speaker.clone());
        let icon = cue
            .icon
            .as_ref()
            .unwrap_or_else(|| panic!("'{}' speaks without a portrait", cue.speaker));
        faces.insert(icon.path().expect("authored as a path").to_string());
    }
    assert_eq!(speakers.len(), 2, "Range Control and the cadet");
    assert_eq!(faces.len(), speakers.len(), "each voice has its own face");
}

#[test]
fn a_defeat_offers_the_same_range_again() {
    let config = config();
    let retries = config
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.name,
                EventConfig::OnDestroyed | EventConfig::OnNeutralized
            ) && event.filters.iter().any(|f| {
                matches!(f, EventFilterConfig::Entity(e) if e.id.as_deref() == Some(ID_TRAINER))
            })
        })
        .map(|event| {
            let retry = event.actions.iter().find_map(|a| match a {
                EventActionConfig::NextScenario(next) => Some(next.scenario_id.as_str()),
                _ => None,
            });
            assert!(
                event.actions.iter().any(|a| matches!(
                    a,
                    EventActionConfig::Outcome(outcome) if outcome.outcome == ScenarioOutcomeKind::Defeat
                )),
                "a trainer loss is a defeat"
            );
            retry
        })
        .collect::<Vec<_>>();
    assert_eq!(retries, vec![Some(TUTORIAL_SCENARIO_ID); 2]);
}

#[test]
fn the_card_is_won_once_and_hands_off_to_nothing() {
    let config = config();
    let wins: Vec<_> = all_actions(&config)
        .into_iter()
        .filter_map(|action| match action {
            EventActionConfig::Outcome(outcome)
                if outcome.outcome == ScenarioOutcomeKind::Victory =>
            {
                Some(outcome)
            }
            _ => None,
        })
        .collect();
    assert_eq!(wins.len(), 1);
    let chained: Vec<_> = all_actions(&config)
        .into_iter()
        .filter_map(|action| match action {
            EventActionConfig::NextScenario(next) => Some(next.scenario_id),
            _ => None,
        })
        .filter(|id| id != TUTORIAL_SCENARIO_ID)
        .collect();
    assert!(chained.is_empty(), "the range chains to no other scenario");
}
