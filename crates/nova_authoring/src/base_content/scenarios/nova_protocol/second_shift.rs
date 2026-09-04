//! "Second Shift" - the same belt, an hour later.
//!
//! The cutter comes back to the Meridian's position and finds a field instead
//! of a ship. The chapter is a search and then a run: recover three pieces of
//! evidence out of the wreck, and get out of the belt when the people who did
//! it send a cleanup group in behind you.
//!
//! The cutter is still unarmed, so the group is not a fight - it is a reason
//! to be somewhere else. Each searcher flies a `PatrolShip` MISSION over the
//! wreck field with a deliberately short detection range and permission to
//! interrupt its own orders: it sweeps its lane, and if it sees the cutter it
//! drops the sweep, fights, and picks the lane back up when the sky clears.
//! Being seen therefore costs the player the quiet route rather than the run,
//! which is the whole point - detection changes the escape, it does not end
//! it.
//!
//! The stage is chapter one's, shared metre for metre through `stage`.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use super::{
    cast::{apply_portraits, CARRIER_NAME, CLEANUP_LEADER, CUTTER_NAME, PLAYER},
    pacing::{self, INSTRUCTION_GAP, REVEAL_GAP},
    ships, stage, SCENARIO_ELAPSED_VAR,
};
use crate::scenario_helpers::prelude::*;

/// The scenario id, shared with chapter one's hand-off.
pub const SECOND_SHIFT_SCENARIO_ID: &str = "second_shift";

// --- layout ------------------------------------------------------------------
//
// Reviewed in `examples/playable/second_shift_map.rs`.

/// Where chapter one left them: the outer mark off the Meridian's starboard
/// quarter, the hold they watched the carrier die from. Chapter two opens on
/// the same coordinates under the same sky, so the cut between the two is a
/// cut in TIME and nothing else.
///
/// It IS chapter one's hold, not a copy of it. The one number the two chapters
/// share, so restaging the set piece cannot leave chapter two opening somewhere
/// the player never was.
const PLAYER_START_POS: Meters3 = super::first_shift::HOME_HOLD_POS;
/// The mark at the edge of the debris, so the approach has somewhere to be.
/// Far enough out that the player must fly to it: a mark whose trigger volume
/// reached back to the spawn would complete the approach before the opening
/// had finished posting it.
const APPROACH_MARK_POS: Meters3 = Meters3::new(900.0, 250.0, -4_200.0);
/// Where the cutter runs to. Five hundred metres off the start: the whole
/// chapter is a loop out into the wreck and back, and the way out is the way
/// you came - out past the Meridian's grave, away from the corner the cleanup
/// crew arrived from.
const EXTRACTION_POS: Meters3 = Meters3::new(2_300.0, -600.0, 2_800.0);
/// The lit line through the middle of the rock plate. It is not the fastest
/// way home; it is the way home with rock between you and the sweep.
const QUIET_ROUTE_POS: Meters3 = Meters3::new(1_400.0, 500.0, -2_800.0);

/// The three evidence marks, in the pieces they came out of. The bridge
/// recorder sits at the Meridian's own position, on top of the biggest thing
/// left of it.
const EVIDENCE: [(&str, &str, Meters3); 3] = [
    (
        "evidence_relay",
        "DISTRESS RELAY",
        Meters3::new(2_300.0, 700.0, -3_650.0),
    ),
    (
        "evidence_engineering",
        "ENGINEERING LOG",
        Meters3::new(700.0, 800.0, -1_800.0),
    ),
    (
        "evidence_bridge",
        "BRIDGE RECORDER",
        Meters3::new(-1_000.0, 500.0, 2_500.0),
    ),
];
/// Evidence pickup radius: a little wider than chapter one's crates, because
/// the pieces sit among wreckage the cutter has to pick its way around.
const EVIDENCE_AREA_RADIUS: Meters = Meters(120.0);

/// How the wreck is scattered: the Meridian broke over the rock plate, so its
/// pieces sit ON the plate's rocks with a small offset each. Fragment zero is
/// the exception - the bridge tower is still where the ship was.
const WRECK_SCATTER: [Meters3; 4] = [
    Meters3::new(-140.0, 420.0, 90.0),
    Meters3::new(180.0, -380.0, -120.0),
    Meters3::new(110.0, 500.0, -160.0),
    Meters3::new(-190.0, -440.0, 140.0),
];
/// How many pieces the field is made of: the bridge tower, plus one beside
/// each of the first 27 plate rocks.
const WRECK_COUNT: usize = 28;

/// The extraction trigger. Wider than a beacon's own volume so the run ends on
/// arrival rather than on a precise park.
const EXTRACTION_RADIUS: Meters = Meters(900.0);

/// How far a searcher looks. Short on purpose: the group is sweeping a debris
/// field for salvage, not running a picket line, and at this range a cutter
/// that keeps rock between itself and the lanes gets through. Widened to
/// [`PURSUIT_ENGAGE_RANGE`] the moment one of them sees something.
const SEARCH_ENGAGE_RANGE: Meters = Meters(900.0);
/// How far they look once the group knows the belt is not empty.
const PURSUIT_ENGAGE_RANGE: Meters = Meters(6_000.0);
/// Arrival grace, so the group flies in readably before anything is hot.
const SEARCH_ENGAGE_DELAY: f32 = 4.0;

/// One of the cleanup group. `armed` is the whole difference that matters to
/// the script: an unarmed hull never acquires a target, so it can neither see
/// the cutter nor interrupt its own sweep, and the two of them are pressure
/// rather than threat.
struct Searcher {
    id: &'static str,
    name: &'static str,
    ship: &'static str,
    armed: bool,
    /// The lane it sweeps, first mark first. `PatrolShip` flies one loop and
    /// reports, and the handler below sends it round again - so every lap is a
    /// beat the scenario could act on, instead of a route nothing can count.
    route: [Meters3; 3],
}

/// The group, entering together from behind the large planetoid - the same
/// body the warship came out from, which is the point.
fn cleanup_group() -> [Searcher; 5] {
    [
        Searcher {
            id: "cleanup_skiff",
            name: "Cleanup Skiff",
            ship: ships::BLOCK_SKIFF_SHIP_ID,
            armed: false,
            route: [
                Meters3::new(7_700.0, 450.0, -6_100.0),
                Meters3::new(2_800.0, 200.0, -3_700.0),
                Meters3::new(1_500.0, 200.0, -3_100.0),
            ],
        },
        Searcher {
            id: "cleanup_tug",
            name: "Cleanup Tug",
            ship: ships::BLOCK_TUG_SHIP_ID,
            armed: false,
            route: [
                Meters3::new(8_050.0, -100.0, -6_200.0),
                Meters3::new(2_500.0, -200.0, -2_600.0),
                Meters3::new(900.0, 0.0, -1_500.0),
            ],
        },
        Searcher {
            id: "cleanup_picket",
            name: "Cleanup Picket",
            ship: ships::BLOCK_PICKET_SHIP_ID,
            armed: true,
            route: [
                Meters3::new(7_750.0, -350.0, -6_750.0),
                Meters3::new(3_050.0, -200.0, -2_900.0),
                Meters3::new(1_200.0, 200.0, -4_300.0),
            ],
        },
        Searcher {
            id: "cleanup_claw",
            name: "Cleanup Claw",
            ship: ships::BLOCK_CLAW_SHIP_ID,
            armed: true,
            route: [
                Meters3::new(8_250.0, 500.0, -6_850.0),
                Meters3::new(2_150.0, 400.0, -1_200.0),
                Meters3::new(0.0, 300.0, 0.0),
            ],
        },
        Searcher {
            id: "cleanup_leader",
            name: "Cleanup Leader",
            ship: ships::BLOCK_CLEANUP_LEADER_SHIP_ID,
            armed: true,
            // A kilometre behind the rest, and deliberately: the leader's
            // Serpent bay reaches ten kilometres, and the group must arrive
            // with the cutter OUTSIDE it rather than under it.
            route: [
                Meters3::new(9_400.0, 100.0, -6_450.0),
                Meters3::new(2_600.0, 250.0, -3_500.0),
                Meters3::new(200.0, 200.0, -2_000.0),
            ],
        },
    ]
}

// Scenario entity ids.
/// The same cutter chapter one flew, so a save, a console command and a log
/// line name one ship across the campaign.
const ID_PLAYER: &str = "cutter";
const ID_APPROACH_MARK: &str = "approach_mark";
const ID_QUIET_ROUTE: &str = "quiet_route";
const ID_EXTRACTION: &str = "extraction";

// Objective ids.
const OBJ_APPROACH: &str = "approach";
const OBJ_SEARCH: &str = "search";
const OBJ_ESCAPE: &str = "escape";
const OBJ_DONE: &str = "done";

// Script variables.
const VAR_BEAT: &str = "beat";
const VAR_EVIDENCE: &str = "evidence_recovered";
const VAR_SETUP_LAST: &str = "setup_last";
/// Whether the group has seen the cutter. It changes the run and it changes
/// the ending line, and it latches - once seen, always seen.
const VAR_SEEN: &str = "seen";
/// The outro act: every defeat gate sits below it.
const BEAT_OUTRO: f64 = 5.0;
const BEAT_WON: f64 = 6.0;

const SEQ_OPENING: &str = "opening";
const OPEN_1_AT: f64 = 2.0;
const OPEN_GAP: f64 = 4.0;

/// The order key one searcher's sweep runs under.
fn sweep_order(searcher: &Searcher) -> String {
    format!("{}_sweep", searcher.id)
}

fn player_enters(area: &str) -> EventFilterConfig {
    entity_pair(area, ID_PLAYER)
}

fn open_line(after: f64, speaker: &str, line: &str) -> SequenceStepConfig {
    step(after, vec![story_message(speaker, line)])
}

fn beat_setup(beat: f64, delay: f64, actions: Vec<EventActionConfig>) -> EventActionConfig {
    let mut all = vec![set_variable(VAR_SETUP_LAST, number(beat))];
    all.extend(actions);
    pacing::beat_later(&format!("beat_{beat}"), delay, all)
}

/// The cutter, one shift older. Every helm verb chapter one handed over is
/// still granted: the progression is that the ship the player learned is the
/// ship they now have, and the chapter can be about the belt instead.
fn player_ship() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PLAYER.to_string(),
            name: CUTTER_NAME.to_string(),
            position: PLAYER_START_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            hull: ships::hull(ships::BLOCK_CUTTER_SHIP_ID),
            ..Default::default()
        }),
    }
}

/// One piece of the Meridian: an industrial hull with nothing left in it that
/// works, and Neutral, so the cleanup group has no more reason to shoot the
/// wreck than the player does.
fn wreck_fragment(index: usize) -> ScenarioObjectConfig {
    // Most of a debris field is small pieces. The bridge tower is the one
    // recognizable thing left, and it is still where the ship was.
    let hull = if index == 0 {
        ships::BLOCK_WRECK_BRIDGE_SHIP_ID
    } else {
        match index % 4 {
            0 => ships::BLOCK_WRECK_SPINE_SHIP_ID,
            1 => ships::BLOCK_WRECK_SHOULDER_SHIP_ID,
            _ => ships::BLOCK_WRECK_PLATE_SHIP_ID,
        }
    };
    let position = if index == 0 {
        stage::CARRIER_POS
    } else {
        stage::SALVAGE_ROCKS[index - 1].0 + WRECK_SCATTER[index % WRECK_SCATTER.len()]
    };
    // Tumble derived from the index rather than hand-typed per piece: it is
    // reproducible, it is a diff of one line when the field is retuned, and no
    // reader has to check twenty-eight quaternions for a typo.
    let turn = index as f32;
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("wreck_{index}"),
            name: format!("{CARRIER_NAME} Fragment {}", index + 1),
            position,
            rotation: Quat::from_euler(EulerRot::XYZ, turn * 0.7, turn * 1.1, turn * 0.37),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Neutral),
            hull: ships::hull(hull),
            ..Default::default()
        }),
    }
}

fn evidence_object(id: &str, label: &str, position: Meters3) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: label.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::SalvageCrate(SalvageCrateConfig {
            size: Meters(15.0),
            area_radius: EVIDENCE_AREA_RADIUS,
            pickup_sound: Some(AssetRef::from("self://sounds/salvage_pickup.wav")),
        }),
    }
}

/// One searcher, on its own hull and its own lane.
fn searcher_object(searcher: &Searcher) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: searcher.id.to_string(),
            name: searcher.name.to_string(),
            position: searcher.route[0],
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                engage_range: Some(SEARCH_ENGAGE_RANGE),
                engage_delay: Some(SEARCH_ENGAGE_DELAY),
                // The sweep is a job, not an order from on high: a searcher
                // that sees something drops the lane, deals with it, and picks
                // the lane back up where it left off.
                order_interruption: Some(AIOrderInterruption::OnHostileContact),
                ..Default::default()
            }),
            hull: ships::hull(searcher.ship),
            ..Default::default()
        }),
    }
}

fn sweep(searcher: &Searcher) -> EventActionConfig {
    EventActionConfig::PatrolShip(PatrolShipActionConfig {
        order: sweep_order(searcher),
        ship: searcher.id.to_string(),
        waypoints: searcher.route.to_vec(),
    })
}

/// The lap handler: one loop finished, send it round again. Without this the
/// group would sweep once and then station-keep in the middle of the wreck.
fn relap(searcher: &Searcher) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnShipOrderComplete,
        once: false,
        filters: vec![EventFilterConfig::ShipOrder(ShipOrderFilterConfig {
            order: Some(sweep_order(searcher)),
            ship: Some(searcher.id.to_string()),
            kind: None,
        })],
        actions: vec![sweep(searcher)],
    }
}

/// The escalation, once per run: a searcher broke off its own sweep, which
/// means it has the cutter. The armed hulls go wide-eyed and lose their lanes
/// entirely; the two unarmed ones keep sweeping, because they still cannot see
/// anything and would only get in the way.
fn detected() -> ScenarioEventConfig {
    let mut actions = vec![
        set_variable(VAR_SEEN, number(1.0)),
        story_message(
            CLEANUP_LEADER,
            "Contact in the debris. Small hull, no transponder. It has been listening to \
             all of it. Bring it down.",
        ),
    ];
    for searcher in cleanup_group().iter().filter(|s| s.armed) {
        actions.push(EventActionConfig::SetAIEngageRange(
            SetAIEngageRangeActionConfig {
                ship: searcher.id.to_string(),
                range: Some(PURSUIT_ENGAGE_RANGE),
            },
        ));
        // Release the lane as well as widening the eyes: a hunt that still
        // owed a waypoint would drag the hull back off the chase every time
        // the contact broke.
        actions.push(EventActionConfig::ClearShipOrder(
            ClearShipOrderActionConfig {
                ship: searcher.id.to_string(),
            },
        ));
    }
    actions.push(pacing::beat_later(
        "detected",
        REVEAL_GAP,
        vec![
            complete_objective(OBJ_ESCAPE),
            post_objective(OBJ_ESCAPE, "They have you. Run for the extraction point."),
        ],
    ));
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnShipOrderInterrupted,
        once: true,
        filters: vec![
            EventFilterConfig::ShipOrder(ShipOrderFilterConfig {
                order: None,
                ship: None,
                kind: Some(ShipOrderKind::Patrol),
            }),
            number_equals(VAR_BEAT, 3.0),
        ],
        actions,
    }
}

/// The epilogue. There is no chapter three yet, so the hand-off is to the
/// player: the evidence is aboard and nobody has heard it.
fn outro() -> EventActionConfig {
    pacing::outro_sequence(
        VAR_BEAT,
        BEAT_WON,
        PLAYER,
        "Three recordings and a hull that should not exist. Somebody is going to want \
         to hear this. Somebody else is going to want me not to.",
        "Clear of the belt with the Meridian's recorders aboard.",
        vec![post_objective(
            OBJ_DONE,
            "Second shift complete. The evidence is aboard.",
        )],
        None,
    )
}

/// The win, in the two ways it can be reached.
fn victory(seen: bool, line: &str) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnEnter,
        once: true,
        filters: vec![
            player_enters(ID_EXTRACTION),
            number_equals(VAR_BEAT, 3.0),
            number_equals(VAR_SEEN, if seen { 1.0 } else { 0.0 }),
        ],
        actions: pacing::open_outro(
            VAR_BEAT,
            BEAT_OUTRO,
            outro(),
            vec![
                complete_objective(OBJ_ESCAPE),
                detach_objective_marker(ID_EXTRACTION),
                story_message(PLAYER, line),
            ],
        ),
    }
}

fn defeat(message: &str, event: EventConfig) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: event,
        once: true,
        filters: vec![entity(ID_PLAYER), number_less_than(VAR_BEAT, BEAT_OUTRO)],
        actions: vec![
            EventActionConfig::Outcome(OutcomeActionConfig::new(
                ScenarioOutcomeKind::Defeat,
                message,
            )),
            EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: SECOND_SHIFT_SCENARIO_ID.to_string(),
                linger: true,
                delay: None,
            }),
        ],
    }
}

pub(crate) fn second_shift(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut start_spawns = vec![player_ship()];
    start_spawns.extend(stage::belt(&asteroid_texture));
    start_spawns.extend((0..WRECK_COUNT).map(wreck_fragment));
    start_spawns.push(stage::beacon(
        ID_APPROACH_MARK,
        "WRECK FIELD",
        APPROACH_MARK_POS,
    ));
    start_spawns.extend(
        ThreePointRig::around("second_shift", Meters3::new(500.0, 0.0, -2_000.0), 25.0).objects(),
    );

    let group = cleanup_group();
    let mut arrival: Vec<EventActionConfig> = group
        .iter()
        .map(|searcher| spawn_object(searcher_object(searcher)))
        .collect();
    arrival.extend(group.iter().map(sweep));

    let mut events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: start_spawns
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain([
                    set_variable(VAR_BEAT, number(1.0)),
                    set_variable(VAR_EVIDENCE, number(0.0)),
                    set_variable(VAR_SETUP_LAST, number(0.0)),
                    set_variable(VAR_SEEN, number(0.0)),
                    // Nobody talks back this time. The opening is one voice in
                    // an empty channel, which is the difference between the two
                    // chapters stated before anything is flown.
                    sequence(
                        SEQ_OPENING,
                        vec![
                            open_line(
                                OPEN_1_AT,
                                PLAYER,
                                "Meridian, cutter one. I have your beacon. I am coming in.",
                            ),
                            open_line(OPEN_GAP, PLAYER, "Meridian, respond."),
                            open_line(
                                OPEN_GAP,
                                PLAYER,
                                "...All right. Recorders, then. Bridge, engineering, the \
                                 relay - whatever is left of them.",
                            ),
                            step(
                                0.0,
                                vec![
                                    post_objective(OBJ_APPROACH, "Fly in to the wreck field."),
                                    attach_objective_marker(ID_APPROACH_MARK, "WRECK FIELD"),
                                ],
                            ),
                        ],
                    ),
                ])
                .collect(),
        },
        // Beat 1 -> 2: at the edge of the debris. The search begins.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![
                player_enters(ID_APPROACH_MARK),
                number_equals(VAR_BEAT, 1.0),
            ],
            actions: vec![
                set_variable(VAR_BEAT, number(2.0)),
                complete_objective(OBJ_APPROACH),
                story_message(
                    PLAYER,
                    "She is all over the plate. Three marks reading on the recorder band - \
                     I will take them one at a time.",
                ),
                beat_setup(
                    2.0,
                    INSTRUCTION_GAP,
                    EVIDENCE
                        .iter()
                        .map(|(id, label, position)| {
                            spawn_object(evidence_object(id, label, *position))
                        })
                        .chain(
                            EVIDENCE
                                .iter()
                                .map(|(id, label, _)| attach_objective_marker(*id, *label)),
                        )
                        .chain([
                            detach_objective_marker(ID_APPROACH_MARK),
                            post_objective(OBJ_SEARCH, "Recover the 3 recorders from the wreck."),
                        ])
                        .collect(),
                ),
            ],
        },
        // Beat 2 -> 3: the evidence is aboard, and the belt is not empty any
        // more. The group flies in from behind the large body - the same place
        // the warship came from, which the player does not have to be told.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![
                number_equals(VAR_BEAT, 2.0),
                number_equals(VAR_EVIDENCE, 3.0),
            ],
            actions: {
                let mut actions = vec![
                    set_variable(VAR_BEAT, number(3.0)),
                    complete_objective(OBJ_SEARCH),
                ];
                actions.extend(arrival);
                actions.push(story_message(
                    PLAYER,
                    "Drives. Five of them, coming around the big body. They are not here \
                     for survivors.",
                ));
                actions.push(beat_setup(
                    3.0,
                    REVEAL_GAP,
                    vec![
                        spawn_object(stage::beacon(ID_QUIET_ROUTE, "ROCK LINE", QUIET_ROUTE_POS)),
                        EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
                            id: ID_EXTRACTION.to_string(),
                            name: "Extraction".to_string(),
                            position: EXTRACTION_POS,
                            rotation: Quat::IDENTITY,
                            radius: EXTRACTION_RADIUS,
                        }),
                        post_objective(
                            OBJ_ESCAPE,
                            "Get out of the belt. Keep rock between you and them.",
                        ),
                        attach_objective_marker(ID_QUIET_ROUTE, "ROCK LINE"),
                    ],
                ));
                actions
            },
        },
        detected(),
        victory(
            false,
            "Clear. They are still turning over rocks back there, and they never saw me.",
        ),
        victory(
            true,
            "Clear of them. They know there was a witness now - and they know what it \
             looks like.",
        ),
        defeat(
            "Your cutter broke apart in the wreck field.",
            EventConfig::OnDestroyed,
        ),
        defeat(
            "Nothing left to fly with - you drift among the pieces of your ship.",
            EventConfig::OnNeutralized,
        ),
    ];
    events.extend(EVIDENCE.iter().map(|(id, _, _)| evidence_pickup(id)));
    events.push(evidence_tally(1.0));
    events.push(evidence_tally(2.0));
    events.extend(group.iter().map(relap));
    apply_portraits(&mut events);

    ScenarioConfig {
        description: "Search the Meridian's wreck, and get out before the cleanup group finds you."
            .to_string(),
        thumbnail: Some(AssetRef::from("self://thumbnails/second_shift.png")),
        watches: vec![scenario_elapsed_watch(SCENARIO_ELAPSED_VAR)],
        events,
        ..ScenarioConfig::new(
            SECOND_SHIFT_SCENARIO_ID.to_string(),
            "Second Shift".to_string(),
            cubemap,
        )
    }
}

fn evidence_pickup(id: &str) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnEnter,
        once: true,
        filters: vec![
            player_enters(id),
            number_equals(VAR_BEAT, 2.0),
            number_equals(VAR_SETUP_LAST, 2.0),
        ],
        actions: vec![despawn_object(id), increment_variable(VAR_EVIDENCE)],
    }
}

fn evidence_tally(count: f64) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnUpdate,
        once: true,
        filters: vec![
            number_equals(VAR_BEAT, 2.0),
            number_equals(VAR_EVIDENCE, count),
        ],
        actions: vec![
            complete_objective(OBJ_SEARCH),
            post_objective(OBJ_SEARCH, format!("Recorders aboard: {count}/3.")),
        ],
    }
}

#[cfg(test)]
mod tests {
    use nova_scenario::prelude::ASTEROID_GEOMETRIC_FACTOR_MAX;

    use super::*;

    fn config() -> ScenarioConfig {
        second_shift(AssetRef::default(), AssetRef::default())
    }

    fn all_actions(config: &ScenarioConfig) -> Vec<EventActionConfig> {
        let mut found = Vec::new();
        for action in config.events.iter().flat_map(|event| event.actions.iter()) {
            action.walk(&mut |action| found.push(action.clone()));
        }
        found
    }

    /// `PatrolShip` flies ONE loop and reports. Every searcher's sweep must
    /// therefore have a handler that sends it round again, or the group would
    /// arrive, cross the wreck once, and then station-keep in the middle of it
    /// for the rest of the chapter.
    #[test]
    fn every_sweep_is_sent_round_again_when_it_finishes() {
        let config = config();
        for searcher in cleanup_group() {
            let key = sweep_order(&searcher);
            let issued = all_actions(&config).into_iter().any(|action| {
                matches!(action, EventActionConfig::PatrolShip(patrol)
                    if patrol.order == key && patrol.ship == searcher.id)
            });
            assert!(issued, "'{}' is never given its sweep", searcher.id);

            let relapped = config.events.iter().any(|event| {
                matches!(event.name, EventConfig::OnShipOrderComplete)
                    && !event.once
                    && event.filters.iter().any(|filter| {
                        matches!(filter, EventFilterConfig::ShipOrder(order)
                            if order.order.as_deref() == Some(key.as_str()))
                    })
                    && event.actions.iter().any(|action| {
                        matches!(action, EventActionConfig::PatrolShip(patrol)
                            if patrol.order == key)
                    })
            });
            assert!(
                relapped,
                "'{}' finishes its lap and is never sent round again",
                searcher.id
            );
        }
    }

    /// Being seen must cost the quiet route, not the run.
    ///
    /// The escalation widens the eyes of exactly the hulls that can act on it
    /// and releases their lanes, and it does NOT touch the two unarmed ones -
    /// a searcher that cannot acquire a target would only be dragged off its
    /// sweep for nothing. It also must not declare an outcome: detection is a
    /// complication, and the only thing that ends this chapter is the player
    /// arriving or dying.
    #[test]
    fn detection_turns_the_sweep_into_a_hunt_without_ending_the_chapter() {
        let handler = detected();
        for searcher in cleanup_group() {
            let widened = handler.actions.iter().any(|action| {
                matches!(action, EventActionConfig::SetAIEngageRange(set)
                    if set.ship == searcher.id && set.range == Some(PURSUIT_ENGAGE_RANGE))
            });
            let released = handler.actions.iter().any(|action| {
                matches!(action, EventActionConfig::ClearShipOrder(clear)
                    if clear.ship == searcher.id)
            });
            assert_eq!(
                widened,
                searcher.armed,
                "'{}' (armed: {}) is wrongly {} by the escalation",
                searcher.id,
                searcher.armed,
                if widened {
                    "widened"
                } else {
                    "left short-sighted"
                }
            );
            assert_eq!(
                released, searcher.armed,
                "'{}' (armed: {}) has the wrong lane state after detection",
                searcher.id, searcher.armed
            );
        }
        assert!(
            !handler
                .actions
                .iter()
                .any(|action| matches!(action, EventActionConfig::Outcome(_))),
            "being seen is not a loss"
        );
    }

    /// The group arrives OUTSIDE the reach of the only ordnance it carries.
    /// The leader's Serpent bay launches from ten kilometres, so a formation
    /// authored any closer would put a torpedo in the air before the player
    /// has seen the drives that fired it. The leader is the only bay in the
    /// group; the block fleet's own tests pin which hull carries what.
    #[test]
    fn the_leader_enters_outside_its_own_launch_envelope() {
        // The launch envelope the content lint measures a spawn against.
        const LAUNCH_ENVELOPE: f32 = 10_000.0;
        let leader = cleanup_group()
            .into_iter()
            .find(|searcher| searcher.ship == ships::BLOCK_CLEANUP_LEADER_SHIP_ID)
            .expect("the group has a leader");
        let range = (leader.route[0] - PLAYER_START_POS).length().0;
        assert!(
            range > LAUNCH_ENVELOPE,
            "'{}' enters {range:.0} m from the player spawn, inside its own \
             {LAUNCH_ENVELOPE:.0} m envelope",
            leader.id
        );
    }

    /// No searcher is sent to a mark inside something solid. A lane is flown by
    /// the real autopilot with no avoidance of its own, so a mark inside a body
    /// is a hull grinding against it for the rest of the chapter - and the
    /// plate rocks are as fatal as the planetoids, because the sweep crosses
    /// the plate by design.
    #[test]
    fn no_sweep_mark_sits_inside_something_solid() {
        // Enough room for the hull itself around the worst-case surface.
        const HULL_PAD: f32 = 100.0;
        let bodies = [
            (
                "the inspection planetoid",
                stage::INSPECTION_POS,
                stage::INSPECTION_RADIUS,
            ),
            (
                "the concealment planetoid",
                stage::CONCEALMENT_POS,
                stage::CONCEALMENT_RADIUS,
            ),
        ]
        .into_iter()
        .chain(
            stage::SALVAGE_ROCKS
                .into_iter()
                .map(|(position, radius)| ("a plate rock", position, radius)),
        );
        let bodies: Vec<_> = bodies.collect();
        for searcher in cleanup_group() {
            for mark in searcher.route {
                for (name, centre, radius) in &bodies {
                    let clearance = (mark - *centre).length().0;
                    let required = radius.0 * ASTEROID_GEOMETRIC_FACTOR_MAX + HULL_PAD;
                    assert!(
                        clearance > required,
                        "'{}' is sent to {mark:?}, {clearance:.0} m from {name} at \
                         {centre:?}, inside its {required:.0} m envelope",
                        searcher.id
                    );
                }
            }
        }
    }

    /// Every evidence mark is reachable. A mark sits deliberately close to the
    /// piece it came out of, but the plate underneath is dense enough that one
    /// authored by eye can end up inside a rock's worst-case surface, where the
    /// pickup volume can never be entered.
    #[test]
    fn every_evidence_mark_sits_in_open_space() {
        for (id, _, position) in EVIDENCE {
            for (rock, radius) in stage::SALVAGE_ROCKS {
                let separation = (position - rock).length().0;
                let required = radius.0 * ASTEROID_GEOMETRIC_FACTOR_MAX + EVIDENCE_AREA_RADIUS.0;
                assert!(
                    separation > required,
                    "'{id}' is {separation:.0} m from the worst-case rock at {rock:?}, \
                     inside its {required:.0} m envelope"
                );
            }
        }
    }

    /// Twenty-eight pieces, each in its own place. A field built by indexing
    /// into a shorter table would silently stack hulls on top of each other,
    /// which is both a physics explosion and a much smaller-looking wreck.
    #[test]
    fn no_two_wreck_fragments_share_a_position() {
        let mut seen: Vec<Meters3> = Vec::new();
        for index in 0..WRECK_COUNT {
            let position = wreck_fragment(index).base.position;
            assert!(
                seen.iter()
                    .all(|other| (*other - position).length().0 > 1.0),
                "wreck fragment {index} lands on top of another piece"
            );
            seen.push(position);
        }
        assert_eq!(seen.len(), WRECK_COUNT);
    }
}
