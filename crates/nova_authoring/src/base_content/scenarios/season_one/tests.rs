//! Structural pins for chapter one.
//!
//! These enforce the SHAPE the chapter is built to - the lane arms one mark at
//! a time, every card posted is completed, the evacuation cannot narrate
//! itself while the clamp is off, the moons never touch the lane, and nothing
//! aboard either hull is a weapon - rather than a transcript of the script. A
//! dialogue or pacing pass should be able to move every line and every delay
//! without touching an assertion below.

use std::collections::BTreeSet;

use nova_events::prelude::Meters;
use nova_gameplay::prelude::{Allegiance, GravitySettings, GravityWell};

use super::*;
use crate::base_content::{scenarios::marks::Mark, ships};

fn config() -> ScenarioConfig {
    chapter_one(AssetRef::default(), AssetRef::default())
}

/// Every action in the chapter, flattened through every chain.
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

/// Filters and actions carry no `PartialEq` (they are configuration trees
/// with asset handles in them), so a pin that wants "this exact filter" or
/// "this exact action" compares the DEBUG rendering. It is the whole tree,
/// field for field, and it fails loudly when either shape changes.
fn same(left: impl std::fmt::Debug, right: impl std::fmt::Debug) -> bool {
    format!("{left:?}") == format!("{right:?}")
}

fn has_filter(event: &ScenarioEventConfig, wanted: &EventFilterConfig) -> bool {
    event.filters.iter().any(|filter| same(filter, wanted))
}

fn has_action(actions: &[EventActionConfig], wanted: &EventActionConfig) -> bool {
    actions.iter().any(|action| same(action, wanted))
}

/// The beat number one handler is gated on, if it is gated on one at all.
fn gated_beat(event: &ScenarioEventConfig) -> Option<f64> {
    let beats = [
        LANE_BEATS[0],
        LANE_BEATS[1],
        LANE_BEATS[2],
        LANE_BEATS[3],
        BEAT_CALL,
        BEAT_REACH,
        BEAT_DOCK,
        BEAT_REGRIP,
        BEAT_HOLD,
        BEAT_TRANSFER,
        BEAT_RELEASE,
    ];
    beats
        .into_iter()
        .find(|beat| has_filter(event, &in_beat(*beat)))
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
                "handler #{idx} ({:?}) posts an objective in the same frame as a line - \
                 give the objective a beat of its own (pacing::beat_later)",
                event.name,
            );
        }
    }
}

#[test]
fn the_opening_scene_hands_back_everything_it_takes() {
    let config = config();
    let start = config
        .events
        .iter()
        .find(|event| matches!(event.name, EventConfig::OnStart))
        .expect("the chapter has an OnStart handler");
    assert!(
        start
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::SuspendPlayerControl(_)))
            && start
                .actions
                .iter()
                .any(|a| matches!(a, EventActionConfig::SetCameraAnchor(_))),
        "the opening takes the camera and the helm"
    );
    assert!(
        !start
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::Objective(_))),
        "OnStart posts no objective over the scene"
    );

    let finish = config
        .events
        .iter()
        .find(|event| matches!(event.name, EventConfig::OnCinematicFinished))
        .expect("the scene has a finish handler");
    assert!(
        finish.filters.len() == 1 && has_filter(finish, &scene(SCENE_OPEN)),
        "the finish handler matches the scene by key, so a skip lands here too"
    );
    assert!(
        finish
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::ReleaseCamera(_)))
            && finish
                .actions
                .iter()
                .any(|a| matches!(a, EventActionConfig::ResumePlayerControl(_))),
        "every way out of the scene gives the camera and the helm back"
    );
}

#[test]
fn the_lane_arms_one_mark_at_a_time() {
    let config = config();
    for (index, mark) in LANE.iter().enumerate() {
        let handler = config
            .events
            .iter()
            .find(|event| has_filter(event, &mark.entered_by(ID_KAVERI)))
            .unwrap_or_else(|| panic!("mark {} has an arrival handler", mark.label));
        assert_eq!(
            gated_beat(handler),
            Some(LANE_BEATS[index]),
            "{} is heard only in its own beat",
            mark.label
        );
        assert!(handler.once, "a mark is arrived at once");

        // The gate a handler arms belongs to the NEXT mark, never its own: a
        // captain cannot be standing in a volume that does not exist yet.
        let raised: BTreeSet<String> = handler
            .action_groups()
            .into_iter()
            .flatten()
            .filter_map(|action| match action {
                EventActionConfig::CreateScenarioArea(area) => Some(area.id.clone()),
                _ => None,
            })
            .collect();
        // The last mark has no next mark: what it arms, at the end of the
        // distress chain, is the volume Gantry sits in.
        let expected: BTreeSet<String> = [LANE
            .get(index + 1)
            .map_or_else(|| APPROACH.gate_id(), Mark::gate_id)]
        .into_iter()
        .collect();
        assert_eq!(
            raised, expected,
            "{} raises exactly the one gate that follows it",
            mark.label
        );
    }
}

#[test]
fn every_mark_and_gate_raised_is_taken_back_down() {
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
            EventActionConfig::CreateScenarioArea(area) => Some(area.id.clone()),
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
    assert!(!raised.is_empty(), "the lane has marks");
    let left_up: Vec<_> = raised.difference(&lowered).collect();
    assert!(
        left_up.is_empty(),
        "every mark and gate raised comes down again; still up: {left_up:?}"
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
fn every_beat_of_the_evacuation_is_gated_on_the_clamp() {
    let config = config();
    let beats: Vec<&ScenarioEventConfig> = config
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnTimerEnd))
        .filter(|event| {
            TRANSFER_KEYS
                .iter()
                .any(|key| has_filter(event, &timer(*key)))
        })
        .collect();
    assert_eq!(
        beats.len(),
        TRANSFER_KEYS.len(),
        "one handler per authored transfer beat"
    );
    for (event, key) in beats.iter().zip(TRANSFER_KEYS) {
        assert!(
            has_filter(event, &timer(key)),
            "the '{key}' beat is matched by its own timer"
        );
        let beat = gated_beat(event).expect("a transfer beat is gated on a beat");
        assert!(
            beat == BEAT_HOLD || beat == BEAT_TRANSFER,
            "the '{key}' beat only fires while the clamp is on"
        );
        assert!(
            !event.once,
            "the '{key}' beat runs again for a captain who re-docks"
        );
    }
}

#[test]
fn letting_go_early_asks_for_the_collar_again() {
    let config = config();
    let releases: Vec<&ScenarioEventConfig> = config
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnUndocked))
        .collect();
    assert_eq!(
        releases.len(),
        3,
        "a release lands in one of three beats: before the card, during the \
         transfer, or with all three aboard"
    );
    for event in &releases {
        assert!(
            has_filter(event, &clamp()),
            "a release is heard only from Kaveri letting go of Gantry"
        );
    }

    for beat in [BEAT_HOLD, BEAT_TRANSFER] {
        let early = releases
            .iter()
            .find(|event| gated_beat(event) == Some(beat))
            .expect("an early release is handled in every clamped beat");
        assert!(
            has_action(&early.actions, &set_variable(VAR_BEAT, number(BEAT_REGRIP))),
            "an early release parks the chapter on the regrip beat"
        );
        assert!(
            has_action(&early.actions, &start_timer(TIMER_REGRIP, REGRIP_GAP)),
            "an early release arms the ask that follows it"
        );
        assert!(
            !early
                .action_groups()
                .into_iter()
                .flatten()
                .any(|action| matches!(action, EventActionConfig::Objective(_))),
            "the ask belongs to the regrip beat, not to the release itself"
        );
    }

    let regrip = config
        .events
        .iter()
        .find(|event| has_filter(event, &timer(TIMER_REGRIP)))
        .expect("the regrip ask has a handler");
    assert_eq!(
        gated_beat(regrip),
        Some(BEAT_REGRIP),
        "the ask is only made while the collar is still empty"
    );
    assert!(
        has_action(
            &regrip.actions,
            &post_objective(OBJ_DOCK, script::OBJ_TEXT_DOCK_AGAIN)
        ),
        "the collar goes back on the card"
    );

    // Only the beat that completes the hold card may complete it.
    let during = releases
        .iter()
        .find(|event| gated_beat(event) == Some(BEAT_TRANSFER))
        .expect("the transfer beat handles a release");
    let before = releases
        .iter()
        .find(|event| gated_beat(event) == Some(BEAT_HOLD))
        .expect("the clamped beat handles a release");
    assert!(
        has_action(&during.actions, &complete_objective(OBJ_HOLD)),
        "a release during the transfer takes the hold card down"
    );
    assert!(
        !has_action(&before.actions, &complete_objective(OBJ_HOLD)),
        "a release BEFORE the card lands must not complete an objective that was \
         never posted"
    );
}

#[test]
fn the_docking_card_and_the_docking_beat_arrive_together() {
    let config = config();
    for (idx, event) in config.events.iter().enumerate() {
        for group in event.action_groups() {
            let sets_beat = group.iter().any(|action| same(action, &advance(BEAT_DOCK)));
            let posts_card = group.iter().any(|action| {
                matches!(action, EventActionConfig::Objective(objective) if objective.id == OBJ_DOCK)
            });
            assert_eq!(
                sets_beat, posts_card,
                "handler #{idx} ({:?}) moves the docking beat and the docking card apart - \
                 the clamp completes that card on sight, so the two travel together",
                event.name,
            );
        }
    }
}

#[test]
fn a_clamp_inside_the_regrip_breath_is_heard() {
    let config = config();
    let clamps: Vec<&ScenarioEventConfig> = config
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnDocked))
        .collect();
    assert_eq!(
        clamps.len(),
        2,
        "a clamp lands either on the ask or inside the breath after a release"
    );
    let fast = clamps
        .iter()
        .find(|event| gated_beat(event) == Some(BEAT_REGRIP))
        .expect("the regrip beat hears a clamp of its own");
    assert!(
        !fast
            .action_groups()
            .into_iter()
            .flatten()
            .any(|action| matches!(
                action,
                EventActionConfig::Objective(_) | EventActionConfig::ObjectiveComplete(_)
            )),
        "no card was posted in that breath, so none is posted or taken down"
    );
    assert!(
        has_action(&fast.actions, &advance(BEAT_HOLD))
            && has_action(
                &fast.actions,
                &start_timer(TRANSFER_KEYS[0], TRANSFER_CARD_AFTER)
            ),
        "the evacuation simply starts again"
    );
}

#[test]
fn the_two_hulls_are_the_only_ships_and_neither_carries_a_gun() {
    let config = config();
    let ships_spawned: Vec<String> = all_actions(&config)
        .into_iter()
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object)
                if matches!(object.kind, ScenarioObjectKind::Spaceship(_)) =>
            {
                Some(object.base.id)
            }
            _ => None,
        })
        .collect();
    assert_eq!(ships_spawned, vec![ID_KAVERI, ID_GANTRY]);

    let assets = crate::base_content::assets::BaseContentAssets::from_paths();
    let catalog = ships::ship_catalog(&assets);
    for id in [
        ships::BLOCK_WORKSHIP_SHIP_ID,
        ships::BLOCK_FRAME_TENDER_SHIP_ID,
    ] {
        let design = catalog
            .iter()
            .find(|entry| entry.id == id)
            .unwrap_or_else(|| panic!("'{id}' is a catalog ship"));
        for section in &design.design.sections {
            let SectionSource::Prototype { id: prototype, .. } = &section.source else {
                continue;
            };
            assert!(
                !prototype.contains("turret")
                    && !prototype.contains("railgun")
                    && !prototype.contains("bay"),
                "'{id}' carries '{prototype}' - the chapter has no weapons in it"
            );
        }
        assert!(
            design
                .design
                .sections
                .iter()
                .any(|section| section.id == ships::BLOCK_PORT_COLLAR_SECTION_ID),
            "'{id}' carries the collar the rescue docks on"
        );
    }
}

#[test]
fn no_moon_can_reach_the_lane_it_stands_off() {
    let settings = GravitySettings::default();
    // Everything the lane is flown through: the marks, both ends of the run,
    // and the wreck at the end of it.
    let mut lane: Vec<Meters3> = LANE.iter().map(|mark| mark.position).collect();
    lane.push(Meters3::ZERO);
    lane.push(APPROACH.position);

    for moon in moons() {
        let ScenarioObjectKind::Planet(planet) = &moon.kind else {
            panic!("a moon is a planet");
        };
        let mass = planet.mass.expect("every moon authors its own mass");
        // Engine units: one world unit is ten metres, and the well is built
        // from the body's radius in those units.
        let well = GravityWell::from_mass(mass, planet.radius.to_engine(), &settings);
        let soi = Meters::from_engine(well.soi_radius);
        assert!(
            (soi.0 - planet.radius.0).abs() < 1.0,
            "'{}' is massed so its well floors at its own surface, not past it",
            moon.base.id
        );
        for point in &lane {
            let gap = Meters((point.0 - moon.base.position.0).length());
            assert!(
                gap.0 > soi.0,
                "'{}' pulls on the lane at {gap:?} (its reach is {soi:?})",
                moon.base.id
            );
        }
    }
}

#[test]
fn both_rock_fields_are_seeded_and_kept_apart() {
    let config = config();
    let fields: Vec<ScatterObjectsConfig> = all_actions(&config)
        .into_iter()
        .filter_map(|action| match action {
            EventActionConfig::ScatterObjects(scatter) => Some(scatter),
            _ => None,
        })
        .collect();
    assert_eq!(
        fields.len(),
        2,
        "the lane's rock, and the drift out to Gantry"
    );
    let mut seeds = BTreeSet::new();
    for field in &fields {
        assert!(field.count > 0, "a field has rock in it");
        assert!(
            field.min_separation.is_some(),
            "'{}' keeps its rocks apart, or the lane is not flyable",
            field.id_prefix
        );
        assert!(
            seeds.insert(field.seed),
            "'{}' reuses another field's seed",
            field.id_prefix
        );
    }
}

#[test]
fn a_loss_offers_the_same_chapter_again() {
    let config = config();
    let losses: Vec<&ScenarioEventConfig> = config
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnDestroyed))
        .collect();
    let lost: Vec<Option<&str>> = losses
        .iter()
        .map(|event| {
            assert!(
                event.actions.iter().any(|a| matches!(
                    a,
                    EventActionConfig::Outcome(outcome)
                        if outcome.outcome == ScenarioOutcomeKind::Defeat
                )),
                "losing a hull is a defeat"
            );
            event.actions.iter().find_map(|a| match a {
                EventActionConfig::NextScenario(next) => Some(next.scenario_id.as_str()),
                _ => None,
            })
        })
        .collect();
    assert_eq!(lost, vec![Some(CHAPTER_ONE_SCENARIO_ID); 2]);
}

#[test]
fn the_chapter_is_won_once_and_hands_off_to_nothing() {
    let config = config();
    let wins: Vec<_> = all_actions(&config)
        .into_iter()
        .filter(|action| {
            matches!(
                action,
                EventActionConfig::Outcome(outcome)
                    if outcome.outcome == ScenarioOutcomeKind::Victory
            )
        })
        .collect();
    assert_eq!(wins.len(), 1);
    let chained: Vec<String> = all_actions(&config)
        .into_iter()
        .filter_map(|action| match action {
            EventActionConfig::NextScenario(next) => Some(next.scenario_id),
            _ => None,
        })
        .filter(|id| id != CHAPTER_ONE_SCENARIO_ID)
        .collect();
    assert!(
        chained.is_empty(),
        "chapter one ends the campaign it opens, for now"
    );
}

#[test]
fn gantry_is_stranded_rather_than_hostile() {
    let config = config();
    let gantry = spawned(&config, ID_GANTRY).expect("Gantry is spawned");
    let ScenarioObjectKind::Spaceship(ship) = &gantry.kind else {
        panic!("Gantry is a ship");
    };
    assert!(
        matches!(ship.controller, SpaceshipController::None),
        "nobody is flying Gantry"
    );
    assert_eq!(ship.allegiance, Some(Allegiance::Neutral));
}
