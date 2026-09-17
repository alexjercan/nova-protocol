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
#[cfg(doc)]
use nova_hud::prelude::COMMS_DWELL_SECS;

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

    finish_hands_back(&config, SCENE_OPEN);
}

/// The finish handler for one scene, checked by its own key.
///
/// By KEY, never by position: the chapter has two scenes now, and a pin that
/// took the first `OnCinematicFinished` it found would grade the opening twice
/// and the distress call never.
fn finish_hands_back(config: &ScenarioConfig, key: &str) -> Vec<EventActionConfig> {
    let finish = config
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnCinematicFinished))
        .find(|event| has_filter(event, &scene(key)))
        .unwrap_or_else(|| panic!("scene '{key}' has a finish handler"));
    assert!(
        finish.filters.len() == 1,
        "scene '{key}' matches on its key alone, so a skip lands here too"
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
        "every way out of scene '{key}' gives the camera and the helm back"
    );
    finish.actions.clone()
}

/// The distress call is a SCENE, and the ship is parked for it.
///
/// The pin is the pairing rather than the shot: a scene that takes the helm
/// and cuts the camera away has to take the ship's speed off as well, or a
/// skip would hand the player back a hull still running at the manual cap
/// through the end of a rock lane. The course change is the scene's exit, so
/// it lands however the player leaves.
#[test]
fn the_call_stops_the_ship_before_it_looks_away() {
    let config = config();
    let call = config
        .events
        .iter()
        .find(|event| {
            event.actions.iter().any(
                |a| matches!(a, EventActionConfig::Cinematic(scene) if scene.key == SCENE_CALL),
            )
        })
        .expect("the call is a cinematic");
    assert!(
        call.actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::SuspendPlayerControl(_)))
            && call.actions.iter().any(
                |a| matches!(a, EventActionConfig::ZeroShipMotion(stop) if stop.id == ID_KAVERI)
            ),
        "the call suspends the helm and brings Kaveri to rest"
    );
    let anchored = call
        .actions
        .iter()
        .any(|a| matches!(a, EventActionConfig::SetCameraAnchor(shot) if shot.anchor == ID_GANTRY));
    assert!(
        anchored,
        "the shot is of Gantry, which is what the call is about"
    );

    let out = finish_hands_back(&config, SCENE_CALL);
    assert!(
        has_action(&out, &post_objective(OBJ_REACH, script::OBJ_TEXT_REACH))
            && has_action(&out, &APPROACH.raise_gate()),
        "leaving the scene posts the course change and the volume it ends in, a skip included"
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

        // The gate a handler arms belongs to the NEXT mark, never its own:
        // the player cannot stand in a volume that does not exist yet.
        let raised: BTreeSet<String> = handler
            .action_groups()
            .into_iter()
            .flatten()
            .filter_map(|action| match action {
                EventActionConfig::CreateScenarioArea(area) => Some(area.id.clone()),
                _ => None,
            })
            .collect();
        // The last mark has no next mark: it starts the distress call, and the
        // volume Gantry sits in is raised on the way OUT of that scene - see
        // `the_call_stops_the_ship_before_it_looks_away` - so that a player
        // who walks out of the call still gets the gate.
        let expected: BTreeSet<String> =
            LANE.get(index + 1).map(Mark::gate_id).into_iter().collect();
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
            "the '{key}' beat runs again when the player re-docks"
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

/// Every beat a clamp can land in has a handler.
///
/// DOCK is in the player's hands from the first frame, so the chapter does not
/// get to decide when the collar is taken - only what happens when it is. The
/// three beats are: before the card was ever posted, on the card, and inside
/// the breath after a release. A beat with no handler is a clamp that does
/// nothing and a chapter with no way forward.
/// The approach is two rings, and the talking belongs to the inner one.
///
/// Arriving is a milestone and happens where the hull fills the canopy; the
/// four cards' worth of docking talk happens 250 m further in, where the ship
/// is nearly stopped. A pass that moved the conversation back out to the
/// arrival gate would hand it to a player who is still braking.
#[test]
fn the_approach_talks_from_the_inner_ring() {
    let config = config();
    let handler = |mark: &Mark| {
        config
            .events
            .iter()
            .find(|event| has_filter(event, &mark.entered_by(ID_KAVERI)))
            .unwrap_or_else(|| panic!("'{}' has an arrival handler", mark.id))
    };
    let lines = |event: &ScenarioEventConfig| {
        event
            .action_groups()
            .into_iter()
            .flatten()
            .filter(|action| matches!(action, EventActionConfig::NarrativeCue(_)))
            .count()
    };

    assert!(
        STANDOFF.area.0 < APPROACH.area.0 && STANDOFF.position == APPROACH.position,
        "the standoff ring is inside the arrival gate, on the same place"
    );

    let outer = handler(&APPROACH);
    assert_eq!(
        lines(outer),
        1,
        "arriving says one thing: they can see the ship"
    );
    assert!(
        has_action(&outer.actions, &complete_objective(OBJ_REACH))
            && has_action(&outer.actions, &STANDOFF.raise_gate()),
        "arriving takes the course-change card down and arms the ring inside it"
    );

    let inner = handler(&STANDOFF);
    assert!(
        lines(inner) > 1,
        "the docking conversation runs from the inner ring"
    );
    assert!(
        !inner
            .action_groups()
            .into_iter()
            .flatten()
            .any(|action| matches!(action, EventActionConfig::Objective(_))),
        "the card is armed on a timer behind the conversation, not posted inside it"
    );
    assert_eq!(
        gated_beat(inner),
        Some(BEAT_REACH),
        "the ring is only heard while the chapter is still on its way in"
    );
}

/// Nothing the chapter puts on screen asks for more than one thought.
///
/// Cards land on top of each other while the ship is being flown, so a comms
/// line that needs two cards' worth of reading, or an objective carrying three
/// instructions, pushes a readable card off the stack. The comms panel holds a
/// card for [`COMMS_DWELL_SECS`] and shows three at once, which is the budget
/// these two ceilings are drawn from: an objective is read at a glance, a line
/// is read once.
#[test]
fn no_card_asks_for_more_than_one_thought() {
    const LINE_CEILING: usize = 90;
    const OBJECTIVE_CEILING: usize = 60;

    let config = config();
    for action in all_actions(&config) {
        match action {
            EventActionConfig::NarrativeCue(cue) => assert!(
                cue.text.chars().count() <= LINE_CEILING,
                "'{}' says {} characters - split it at the full stop it already has",
                cue.speaker,
                cue.text.chars().count(),
            ),
            EventActionConfig::Objective(objective) => assert!(
                objective.message.chars().count() <= OBJECTIVE_CEILING,
                "the '{}' card is {} characters - an objective is the goal, and how \
                 to do it belongs to a crew line or the keybind chip",
                objective.id,
                objective.message.chars().count(),
            ),
            _ => {}
        }
    }
}

#[test]
fn every_beat_a_clamp_can_land_in_is_heard() {
    let config = config();
    let clamps: Vec<&ScenarioEventConfig> = config
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnDocked))
        .collect();
    let heard: BTreeSet<String> = clamps
        .iter()
        .map(|event| format!("{:?}", gated_beat(event)))
        .collect();
    let wanted: BTreeSet<String> = [BEAT_REACH, BEAT_DOCK, BEAT_REGRIP]
        .into_iter()
        .map(|beat| format!("{:?}", Some(beat)))
        .collect();
    assert_eq!(
        heard, wanted,
        "a clamp lands before the card, on the card, or inside the breath after \
         a release - and each is heard"
    );
    for event in &clamps {
        assert!(
            has_filter(event, &clamp()),
            "a clamp is heard only from Kaveri taking Gantry's collar"
        );
    }

    // The early one starts the evacuation without posting or completing a card:
    // none was ever up, and the pending docking timer lands in a beat that has
    // moved and says nothing.
    let early = clamps
        .iter()
        .find(|event| gated_beat(event) == Some(BEAT_REACH))
        .expect("a clamp before the card is heard");
    assert!(
        !early
            .action_groups()
            .into_iter()
            .flatten()
            .any(|action| matches!(
                action,
                EventActionConfig::Objective(_) | EventActionConfig::ObjectiveComplete(_)
            )),
        "no docking card existed yet, so none is posted or taken down"
    );
    assert!(
        has_action(&early.actions, &advance(BEAT_HOLD))
            && has_action(
                &early.actions,
                &start_timer(TRANSFER_KEYS[0], TRANSFER_CARD_AFTER)
            ),
        "an early clamp starts the evacuation rather than dead-ending the chapter"
    );

    let fast = clamps
        .iter()
        .find(|event| gated_beat(event) == Some(BEAT_REGRIP))
        .expect("the regrip beat hears a clamp of its own");
    assert!(
        !fast.once,
        "the player may let go and come back more than once"
    );
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
    // The two crewed hulls, and then Gantry's own plating: the debris is a
    // hull in the engine's eyes, so it is listed here, but nothing flies it
    // and it is the only thing that may join them.
    let (crewed, debris) = ships_spawned.split_at(2);
    assert_eq!(crewed, [ID_KAVERI, ID_GANTRY]);
    assert!(
        debris.iter().all(|id| id.starts_with("gantry_debris")),
        "the only other hulls in the chapter are Gantry's own wreckage: {debris:?}"
    );

    let assets = crate::base_content::assets::BaseContentAssets::from_paths();
    let catalog = ships::ship_catalog(&assets);
    for id in [
        ships::BLOCK_WORKSHIP_SHIP_ID,
        ships::BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID,
        ships::BLOCK_WRECK_PLATE_SHIP_ID,
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
    }

    for id in [
        ships::BLOCK_WORKSHIP_SHIP_ID,
        ships::BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID,
    ] {
        let design = catalog
            .iter()
            .find(|entry| entry.id == id)
            .unwrap_or_else(|| panic!("'{id}' is a catalog ship"));
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

/// Gantry flies the DAMAGED tender, and the damage is all behind the collar.
///
/// The chapter asks for a hull that reads as hit and docks as easily as an
/// intact one, which is two claims about the same cell plan: the drive and the
/// stern are gone, and the port collar sits exactly where the whole ship's
/// does. A future pass that moves the collar to dress the wreck up would make
/// the rescue harder without saying so.
#[test]
fn gantrys_damage_is_aft_of_everything_the_rescue_touches() {
    let assets = crate::base_content::assets::BaseContentAssets::from_paths();
    let catalog = ships::ship_catalog(&assets);
    let section = |ship: &str, id: &str| {
        catalog
            .iter()
            .find(|entry| entry.id == ship)
            .unwrap_or_else(|| panic!("'{ship}' is a catalog ship"))
            .design
            .sections
            .iter()
            .find(|section| section.id == id)
            .cloned()
    };

    let whole = section(
        ships::BLOCK_FRAME_TENDER_SHIP_ID,
        ships::BLOCK_PORT_COLLAR_SECTION_ID,
    )
    .expect("the tender carries a collar");
    let hurt = section(
        ships::BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID,
        ships::BLOCK_PORT_COLLAR_SECTION_ID,
    )
    .expect("the damaged tender still carries it");
    assert!(
        whole.position == hurt.position && whole.rotation == hurt.rotation,
        "the collar has not moved: {:?} vs {:?}",
        whole.position,
        hurt.position
    );

    assert!(
        section(ships::BLOCK_FRAME_TENDER_SHIP_ID, "main_drive").is_some()
            && section(ships::BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID, "main_drive").is_none(),
        "the raid took the main drive, which is what the mayday says it took"
    );
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
