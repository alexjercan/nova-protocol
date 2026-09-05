//! Structural pins for First Shift.
//!
//! These enforce the SHAPE the chapter was rebuilt for - one thing on the
//! panel at a time, the thrusters taught in open space, the orbit as a detour,
//! the attack held until the cutter is parked where the shot is, the camera
//! handed back -
//! rather than a transcript of the current script. A dialogue or pacing pass
//! should be able to move every line and every delay in this chapter without
//! touching a single assertion below.

use nova_scenario::prelude::ASTEROID_GEOMETRIC_FACTOR_MAX;

use super::*;

fn portraits() -> CampaignPortraits {
    crate::base_content::assets::BaseContentAssets::from_paths().portraits
}

fn config() -> ScenarioConfig {
    first_shift(AssetRef::default(), AssetRef::default(), &portraits())
}

#[test]
fn reusable_scenes_keep_preview_positions_out_of_production_code() {
    let scenes = [
        FirstShiftScene::Departure,
        FirstShiftScene::Rcs,
        FirstShiftScene::Salvage,
        FirstShiftScene::Navigation,
        FirstShiftScene::Orbit,
        FirstShiftScene::Return,
        FirstShiftScene::AttackApproach,
        FirstShiftScene::AttackSalvo,
        FirstShiftScene::Aftermath,
    ];
    for scene in scenes {
        let preview = first_shift_scene(
            scene,
            AssetRef::default(),
            AssetRef::default(),
            &portraits(),
        );
        let cutter = spawned_ship(&preview, ID_CUTTER)
            .unwrap_or_else(|| panic!("{scene:?} preview does not spawn Cutter"));
        assert_eq!(
            cutter.base.position, CUTTER_START_POS,
            "{scene:?} embeds a preview-only Cutter position; examples must own it"
        );
        let spawned_ids: Vec<&str> = preview
            .events
            .iter()
            .flat_map(|event| &event.actions)
            .filter_map(|action| match action {
                EventActionConfig::SpawnScenarioObject(object) => Some(object.base.id.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            spawned_ids.contains(&stage::ID_INSPECTION)
                && spawned_ids.contains(&stage::ID_CONCEALMENT),
            "{scene:?} does not reuse the production stage"
        );
        assert!(
            all_actions(&preview).iter().any(|action| {
                matches!(action, EventActionConfig::StoryMessage(message)
                    if message.speaker == "PREVIEW"
                        && message.text.starts_with("Here is where First Shift"))
            }),
            "{scene:?} has no explicit preview end message"
        );
        if scene != FirstShiftScene::Departure {
            for verb in [FlightVerb::Rcs, FlightVerb::Lock] {
                assert!(
                    all_actions(&preview).iter().any(|action| {
                        matches!(action, EventActionConfig::SetControllerVerb(grant)
                            if grant.id == ID_CUTTER && grant.verb == verb && grant.enabled)
                    }),
                    "{scene:?} does not enable {verb:?} for standalone review"
                );
            }
        }
        if scene == FirstShiftScene::AttackApproach {
            assert!(
                spawned_ids.contains(&ID_WARSHIP),
                "attack scene does not carry its warship fixture: {spawned_ids:?}"
            );
        }
        if scene == FirstShiftScene::Return {
            assert!(
                all_actions(&preview).iter().any(|action| {
                    matches!(action, EventActionConfig::SpawnScenarioObject(object)
                        if object.base.id == crate_id(3))
                }),
                "return scene does not carry its crate fixture"
            );
        }
    }
}

/// Every action the script can run, chain steps included.
fn all_actions(config: &ScenarioConfig) -> Vec<EventActionConfig> {
    let mut found = Vec::new();
    for action in config.events.iter().flat_map(|event| event.actions.iter()) {
        action.walk(&mut |action| found.push(action.clone()));
    }
    found
}

/// The beats in the order the shift runs them.
const BEATS: [f64; 21] = [
    BEAT_LAUNCH,
    BEAT_STOP,
    BEAT_TRIM_LATERAL,
    BEAT_TRIM_VERTICAL,
    BEAT_TRIM_RETURN_LATERAL,
    BEAT_TRIM_RETURN_VERTICAL,
    BEAT_CRATE_FIRST,
    BEAT_CRATE_SECOND,
    BEAT_LOCK,
    BEAT_GOTO,
    BEAT_TRANSIT,
    BEAT_DETOUR,
    BEAT_ORBIT,
    BEAT_ORBIT_RETURN_VIEW,
    BEAT_RETURN,
    BEAT_SEARCH,
    BEAT_VANTAGE,
    BEAT_ATTACK,
    BEAT_DISTRESS,
    BEAT_OUTRO,
    BEAT_WON,
];

/// The beat gate a handler runs under, read back by rebuilding the filter the
/// graph would have written. OnStart and the defeat pair carry none.
fn beat_of(event: &ScenarioEventConfig) -> Option<f64> {
    BEATS.iter().copied().find(|beat| {
        let gate = format!("{:?}", number_equals(VAR_BEAT, *beat));
        event
            .filters
            .iter()
            .any(|filter| format!("{filter:?}") == gate)
    })
}

/// The beat a handler MOVES the shift to, if it transitions.
fn beat_set_by(event: &ScenarioEventConfig) -> Option<f64> {
    BEATS.iter().copied().find(|beat| {
        let write = format!("{:?}", set_variable(VAR_BEAT, number(*beat)));
        let mut found = false;
        for action in &event.actions {
            action.walk(&mut |action| {
                found |= format!("{action:?}") == write;
            });
        }
        found
    })
}

/// Handlers in the order the shift can reach them: by the beat they are gated
/// on, then by their position in the graph.
fn handlers_in_beat_order(config: &ScenarioConfig) -> Vec<&ScenarioEventConfig> {
    let mut events: Vec<(f64, usize, &ScenarioEventConfig)> = config
        .events
        .iter()
        .enumerate()
        .map(|(index, event)| (beat_of(event).unwrap_or(0.0), index, event))
        .collect();
    events.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    events.into_iter().map(|(_, _, event)| event).collect()
}

/// The ship the scenario spawns under an id, if it spawns one.
fn spawned_ship(config: &ScenarioConfig, id: &str) -> Option<ScenarioObjectConfig> {
    all_actions(config)
        .into_iter()
        .find_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object)
                if object.base.id == id
                    && matches!(object.kind, ScenarioObjectKind::Spaceship(_)) =>
            {
                Some(object)
            }
            _ => None,
        })
}

/// The campaign's two named ships are named, and the player's hull is no
/// longer the generic runtime id. Everything downstream addresses them by
/// these strings: chapter two flies the same cutter, the console names it, and
/// the set piece fires the warship's guns by ship id.
#[test]
fn the_cutter_and_the_carrier_carry_their_own_names() {
    let config = config();

    let cutter = spawned_ship(&config, ID_CUTTER).expect("the shift spawns no cutter");
    assert_eq!(cutter.base.name, CUTTER_NAME);
    assert!(
        matches!(&cutter.kind, ScenarioObjectKind::Spaceship(ship)
            if matches!(ship.controller, SpaceshipController::Player(_))),
        "'{ID_CUTTER}' is not the ship the player flies"
    );

    let carrier = spawned_ship(&config, ID_CARRIER).expect("the shift spawns no carrier");
    assert!(
        carrier.base.name.contains(CARRIER_NAME),
        "the carrier is called '{}', not the campaign's '{CARRIER_NAME}'",
        carrier.base.name
    );

    for action in all_actions(&config) {
        if let EventActionConfig::SpawnScenarioObject(object) = action {
            assert_ne!(
                object.base.id, "player_spaceship",
                "the chapter still spawns something under the generic runtime id"
            );
        }
    }
}

/// A finished mark DISAPPEARS. Dropping the HUD chip is not enough: a beacon
/// left burning in the belt is a mark the player keeps flying to, and by the
/// end of the shift the route would be a line of them. The wreck's distress
/// beacon is the one exception - it is the chapter's last image and is meant
/// to still be there.
#[test]
fn every_temporary_mark_the_shift_raises_is_taken_back_down_again() {
    let config = config();
    let mut raised: Vec<String> = Vec::new();
    let mut cleared: Vec<String> = Vec::new();
    for action in all_actions(&config) {
        match action {
            EventActionConfig::SpawnScenarioObject(object) => match object.kind {
                ScenarioObjectKind::Beacon(_) | ScenarioObjectKind::SalvageCrate(_) => {
                    raised.push(object.base.id);
                }
                _ => {}
            },
            EventActionConfig::CreateScenarioArea(area) => raised.push(area.id),
            EventActionConfig::DespawnScenarioObject(despawn) => cleared.push(despawn.id.clone()),
            _ => {}
        }
    }
    for id in &raised {
        if id == ID_DISTRESS {
            continue;
        }
        assert!(
            cleared.contains(id),
            "'{id}' is put up and never taken down - it stays lit in the belt \
             for the rest of the chapter"
        );
    }
    assert!(
        !cleared.contains(&ID_DISTRESS.to_string()),
        "the distress beacon is the chapter's last image and must survive it"
    );
}

/// A beat is never armed before the card it clears exists.
///
/// This is the orphaned-objective failure, stated structurally. A beat is
/// closed by a `once` handler on an arrival gate; if that gate is live before
/// the objective the handler completes has been posted, a player who arrives
/// early spends the handler on a `complete_objective` for an id that is not
/// there. The completion warns and no-ops, and the card lands afterwards with
/// nothing left to clear it - a dead objective riding the panel to the end of
/// the chapter.
///
/// So: walk the shift in beat order, and by the frame a gate is RAISED, every
/// objective the handlers it arms complete must already have been posted -
/// including in that same frame, which is the idiom the RCS box uses.
#[test]
fn no_arrival_gate_is_raised_before_the_objective_it_clears() {
    use std::collections::HashSet;

    let config = config();
    assert_eq!(
        objectives_completed_on_entering(&config, &TRIM_VERTICAL.gate_id()),
        vec![OBJ_TRIM_VERTICAL.to_string()],
        "the walk below reads nothing if a gate's completion handler cannot be found"
    );

    let mut posted: HashSet<String> = HashSet::new();
    for event in handlers_in_beat_order(&config) {
        for group in event.action_groups() {
            for action in &group {
                if let EventActionConfig::Objective(objective) = action {
                    posted.insert(objective.id.clone());
                }
            }
            for action in &group {
                let EventActionConfig::CreateScenarioArea(area) = action else {
                    continue;
                };
                for closed in objectives_completed_on_entering(&config, &area.id) {
                    assert!(
                        posted.contains(&closed),
                        "{:?} raises '{}' before '{closed}' is posted - a player who \
                         reaches it early spends the beat on an objective that does \
                         not exist yet",
                        event.name,
                        area.id,
                    );
                }
            }
        }
    }
}

/// Every objective completed by a handler that fires on ENTERING `area`.
fn objectives_completed_on_entering(config: &ScenarioConfig, area: &str) -> Vec<String> {
    config
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnEnter))
        .filter(|event| {
            event.filters.iter().any(|filter| match filter {
                EventFilterConfig::Entity(entity) => entity.id.as_deref() == Some(area),
                _ => false,
            })
        })
        .flat_map(|event| event.actions.iter())
        .filter_map(|action| match action {
            EventActionConfig::ObjectiveComplete(complete) => Some(complete.id.clone()),
            _ => None,
        })
        .collect()
}

/// ONE thing on the panel at a time. The old script put three crate chips up
/// together, which turns a lesson in flying the plate into a shopping list;
/// walking the graph in beat order proves no two work markers are ever lit at
/// once, whatever the marks themselves are moved to.
#[test]
fn the_panel_never_points_at_two_places_at_once() {
    let config = config();
    let mut lit: Vec<String> = Vec::new();
    for event in handlers_in_beat_order(&config) {
        for group in event.action_groups() {
            for action in group {
                match action {
                    EventActionConfig::ObjectiveMarkerAttach(attach) => {
                        lit.push(attach.target_id.clone());
                    }
                    EventActionConfig::ObjectiveMarkerDetach(detach) => {
                        lit.retain(|id| *id != detach.target_id);
                    }
                    _ => {}
                }
            }
            assert!(
                lit.len() <= 1,
                "{:?} lights {lit:?} at once - reveal one work marker at a time",
                event.name,
            );
        }
    }
    assert_eq!(
        lit,
        vec![ID_DISTRESS.to_string()],
        "the chapter should end pointing at the wreck's beacon and nothing else"
    );
}

/// The thrusters are taught in OPEN SPACE, before the plate. All four trim
/// marks must be clear of every rock's worst-case rendered body by more than
/// their own trigger volume, and every lesson must run at a beat BELOW the first
/// crate - the old script's first RCS translation was inside the densest rock
/// in the field, which is the last place to learn a new control.
#[test]
fn the_thrusters_are_taught_in_open_space_before_the_plate() {
    for mark in [&WORK_MARK].into_iter().chain(TRIM_ROUTE) {
        for (rock, radius) in stage::SALVAGE_ROCKS {
            let separation = (mark.position - rock).length().0;
            let required = radius.0 * ASTEROID_GEOMETRIC_FACTOR_MAX + mark.area.0;
            assert!(
                separation > required,
                "'{}' is {separation:.0} m from the worst-case rock at {rock:?}, \
                 inside its {required:.0} m envelope - the thruster lesson is \
                 not in open space",
                mark.id
            );
        }
    }
    const {
        assert!(
            BEAT_TRIM_LATERAL < BEAT_CRATE_FIRST
                && BEAT_TRIM_VERTICAL < BEAT_CRATE_FIRST
                && BEAT_TRIM_RETURN_LATERAL < BEAT_CRATE_FIRST
                && BEAT_TRIM_RETURN_VERTICAL < BEAT_CRATE_FIRST,
            "the plate opens before the thrusters are taught"
        );
    }

    // ...and the control itself is handed over before the field work, not
    // with it: the grant lands on a beat below the first crate.
    let config = config();
    let granted_at = config
        .events
        .iter()
        .find(|event| {
            let mut grants = false;
            for action in &event.actions {
                action.walk(&mut |action| {
                    grants |= matches!(action, EventActionConfig::SetControllerVerb(verb)
                        if verb.verb == FlightVerb::Rcs && verb.enabled);
                });
            }
            grants
        })
        .and_then(beat_set_by)
        .expect("nothing in the shift grants RCS");
    assert!(
        granted_at < BEAT_CRATE_FIRST,
        "RCS is granted at beat {granted_at}, at or after the plate opens at \
         {BEAT_CRATE_FIRST}"
    );
}

/// STOP is a physical tutorial step and the manual governor stays visible in
/// its behavior for the full shift. An elapsed delay must not grant RCS while
/// Cutter is still drifting.
#[test]
fn the_rcs_lesson_waits_for_stop_and_keeps_the_manual_governor() {
    let config = config();
    let cutter = spawned_ship(&config, ID_CUTTER).expect("the shift spawns no cutter");
    let ScenarioObjectKind::Spaceship(ship) = cutter.kind else {
        panic!("Cutter is not a spaceship");
    };
    assert!(
        matches!(ship.controller, SpaceshipController::Player(PlayerControllerConfig {
            speed_cap: Some(cap), ..
        }) if cap == CUTTER_SPEED_CAP),
        "Cutter does not retain the authored manual speed cap"
    );
    assert!(
        !all_actions(&config).iter().any(
            |action| matches!(action, EventActionConfig::SetSpeedCap(cap)
                if cap.id == ID_CUTTER && cap.cap.is_none())
        ),
        "First Shift silently removes Cutter's manual governor"
    );

    let stop = config
        .events
        .iter()
        .find(|event| {
            matches!(event.name, EventConfig::OnStopComplete) && beat_of(event) == Some(BEAT_STOP)
        })
        .expect("no physical STOP completion opens the RCS lesson");
    assert_eq!(
        beat_set_by(stop),
        Some(BEAT_TRIM_LATERAL),
        "STOP completion does not advance to the first RCS translation"
    );
}

#[test]
fn the_rcs_briefing_shows_the_complete_four_mark_box_before_control_returns() {
    let config = config();
    let stop = config
        .events
        .iter()
        .find(|event| {
            matches!(event.name, EventConfig::OnStopComplete) && beat_of(event) == Some(BEAT_STOP)
        })
        .expect("no physical STOP completion opens the RCS lesson");
    let spawned: Vec<&str> = stop
        .actions
        .iter()
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object)
                if TRIM_ROUTE.iter().any(|mark| mark.id == object.base.id) =>
            {
                Some(object.base.id.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        spawned,
        TRIM_ROUTE.map(|mark| mark.id).to_vec(),
        "all four route marks must spawn together in route order"
    );
    assert_eq!(
        TRIM_ROUTE.map(|mark| mark.position),
        [
            Meters3::new(-200.0, 80.0, 900.0),
            Meters3::new(-200.0, 300.0, 900.0),
            Meters3::new(-500.0, 300.0, 900.0),
            WORK_MARK.position,
        ],
        "the route no longer closes its four-mark box"
    );

    let mut actions = Vec::new();
    for action in &stop.actions {
        action.walk(&mut |action| actions.push(action.clone()));
    }
    assert!(
        actions.iter().any(|action| {
            matches!(action, EventActionConfig::SetCameraAnchor(shot)
                if shot.anchor == ID_CUTTER
                    && shot.offset == CINEMA_TRIM_OFFSET
                    && matches!(&shot.look_at, CameraLookAtConfig::Point(position)
                        if *position == TRIM_ROUTE_CENTRE))
        }),
        "the complete route is never framed from Cutter"
    );
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, EventActionConfig::ReleaseCamera(_))),
        "the briefing never returns the chase camera"
    );
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, EventActionConfig::ResumePlayerControl(_))),
        "the briefing never restores player control"
    );
}

/// The plate itself gets HARDER. The first crate sits on the open edge where a
/// mistake costs nothing and the second is well inside the rocks, so the field
/// teaches itself in the right order.
#[test]
fn the_field_work_runs_from_the_plate_edge_inward() {
    let visible_half_diagonal = CRATE_SIZE.0 * 3.0_f32.sqrt() / 2.0;
    let contact_tolerance = CRATE_AREA_RADIUS.0 - visible_half_diagonal;
    assert!(
        (0.0..=2.1).contains(&contact_tolerance),
        "the pickup sphere leaves {contact_tolerance:.1} m beyond the tumbling \
         crate; it must enclose the visible box without collecting at a distance"
    );

    let clearance = |position: Meters3| {
        stage::SALVAGE_ROCKS
            .into_iter()
            .map(|(rock, radius)| {
                (position - rock).length().0 - radius.0 * ASTEROID_GEOMETRIC_FACTOR_MAX
            })
            .fold(f32::INFINITY, f32::min)
    };
    let first = clearance(CRATE_POSITIONS[0]);
    let second = clearance(CRATE_POSITIONS[1]);
    assert!(
        first > second,
        "the first crate has {first:.0} m of room and the second {second:.0} m - \
         the plate is supposed to close in, not open up"
    );
    for (index, position) in CRATE_POSITIONS.into_iter().enumerate() {
        let room = clearance(position);
        assert!(
            room > CRATE_AREA_RADIUS.0,
            "crate {} has {room:.0} m of room, inside its own {:.0} m pickup \
             volume - it cannot be flown to",
            index + 1,
            CRATE_AREA_RADIUS.0
        );
    }
}

/// The orbit is a DETOUR, not the job. It happens with work still outstanding,
/// and the shift goes back to that same work afterwards - which is the whole
/// reason the Meridian's answer to it lands.
#[test]
fn the_orbit_is_a_detour_the_crew_comes_back_from() {
    const {
        assert!(
            BEAT_ORBIT < BEAT_ORBIT_RETURN_VIEW
                && BEAT_ORBIT_RETURN_VIEW < BEAT_RETURN
                && BEAT_RETURN < BEAT_SEARCH,
            "the orbit does not sit before the return to the field work"
        );
        assert!(
            BEAT_CRATE_SECOND < BEAT_DETOUR,
            "the detour is proposed before any of the field work is done - it has \
             to be time the crew is stealing from a job in progress"
        );
    }

    let config = config();
    let lap = config
        .events
        .iter()
        .find(|event| matches!(event.name, EventConfig::OnOrbitLap))
        .expect("the detour waits for physical angular travel");
    assert_eq!(
        beat_set_by(lap),
        Some(BEAT_ORBIT_RETURN_VIEW),
        "one lap must lead to the return-view beat, not directly end the orbit"
    );
    let return_view = config
        .events
        .iter()
        .find(|event| beat_set_by(event) == Some(BEAT_RETURN))
        .expect("crossing the near-side gate starts the return beat");
    assert!(matches!(return_view.name, EventConfig::OnEnter));

    let cover = distance_to_segment(
        stage::INSPECTION_POS,
        stage::CARRIER_POS,
        TRANSIT_TWO.position,
    );
    assert!(
        cover < stage::inspection_body_radius().0,
        "TRANSIT 2 is visible around the inspection body: {cover:.0} m"
    );
    let return_view_clearance = distance_to_segment(
        stage::INSPECTION_POS,
        stage::CARRIER_POS,
        ORBIT_RETURN_GATE_POS,
    );
    assert!(
        return_view_clearance > stage::inspection_body_radius().0,
        "the return gate is still hidden behind the inspection body"
    );

    let closes_the_return = config
        .events
        .iter()
        .find(|event| {
            beat_of(event) == Some(BEAT_RETURN)
                && event.actions.iter().any(|action| {
                    matches!(action, EventActionConfig::ObjectiveComplete(objective)
                        if objective.id == OBJ_RETURN)
                })
        })
        .expect("nothing completes the return to the field work");
    assert!(
        matches!(closes_the_return.name, EventConfig::OnGotoComplete),
        "the return completes on {:?} - it must wait for GOTO to settle at the \
         work site, not merely cross its trigger",
        closes_the_return.name
    );
    let gate = format!("{:?}", cutter_completes_goto(WORK_SITE.id));
    assert!(
        closes_the_return
            .filters
            .iter()
            .any(|filter| format!("{filter:?}") == gate),
        "the return completes after GOTO settles at something other than the work site"
    );
}

/// The attack waits for the cutter to be PARKED WHERE THE SHOT IS. The shift's
/// last errand is the run home, and the warship comes out of cover on arrival
/// at the hold - not on the last crate, and not while the player is still
/// picking their way out of the rocks.
#[test]
fn the_attack_waits_for_the_cutter_to_reach_the_hold() {
    let config = config();
    let opens = config
        .events
        .iter()
        .find(|event| {
            event.actions.iter().any(|action| {
                matches!(action, EventActionConfig::SpawnScenarioObject(object)
                    if object.base.id == ID_WARSHIP)
            })
        })
        .expect("nothing in the shift spawns the warship");
    assert_eq!(
        beat_of(opens),
        Some(BEAT_VANTAGE),
        "the warship comes out at the wrong beat - it must wait for the run home"
    );
    assert!(
        matches!(opens.name, EventConfig::OnGotoComplete),
        "the warship comes out on {:?} rather than after the player settles at \
         the hold",
        opens.name
    );
    let gate = format!("{:?}", cutter_completes_goto(HOME_MARK.id));
    assert!(
        opens
            .filters
            .iter()
            .any(|filter| format!("{filter:?}") == gate),
        "the warship comes out after GOTO settles at something other than the hold"
    );
    const {
        assert!(
            BEAT_SEARCH < BEAT_VANTAGE && BEAT_VANTAGE < BEAT_ATTACK,
            "the run home does not sit between the last crate and the attack"
        );
    }
}

/// The hold IS the composition. Every number that decides whether the set piece
/// reads is pinned here rather than left to a playtest: how big the Meridian is
/// in the frame, how far the player is from the ordnance crossing between the
/// two ships, and how much rock is near enough to hit while they watch.
#[test]
fn the_hold_frames_the_set_piece_without_standing_in_it() {
    let hold = HOME_MARK.position;

    let to_carrier = (hold - stage::CARRIER_POS).length().0;
    assert!(
        (2_000.0..4_000.0).contains(&to_carrier),
        "the hold is {to_carrier:.0} m off the Meridian - the carrier is either \
         a shape in the distance or on top of the camera"
    );

    // A Breaker's blast reaches 450 m: `sections/standard.rs`, the heavy bay's
    // `blast_radius`. The player can arrive anywhere in the mark's own volume,
    // so the whole sphere has to sit outside that.
    const BLAST_RADIUS: f32 = 450.0;
    let lane = distance_to_segment(hold, WARSHIP_FIRING_POS, stage::CARRIER_POS);
    assert!(
        lane > HOME_MARK.area.0 + BLAST_RADIUS,
        "the hold is {lane:.0} m off the torpedo lane and {:.0} m across - a \
         player parked on the near side of it is inside a {BLAST_RADIUS:.0} m \
         blast",
        HOME_MARK.area.0,
    );

    for (rock, radius) in stage::SALVAGE_ROCKS {
        let separation = (hold - rock).length().0;
        let required = radius.0 * ASTEROID_GEOMETRIC_FACTOR_MAX + HOME_MARK.area.0;
        assert!(
            separation > required,
            "the hold is {separation:.0} m from the worst-case rock at {rock:?}, \
             inside its {required:.0} m envelope - a player watching the sky is \
             in immediate collision danger"
        );
    }
}

/// Every fixed GOTO corridor authored for Cutter clears the complete shared
/// stage. The direct autopilot has no obstacle avoidance, so the route owns a
/// conservative 55 m Cutter sphere plus 100 m of flight margin beyond every
/// worst-case rock mesh.
#[test]
fn every_fixed_cutter_goto_corridor_clears_the_stage() {
    const CUTTER_RADIUS: f32 = 55.0;
    const FLIGHT_MARGIN: f32 = 100.0;
    // Geometric radii, not nominal ones. A rock has to be widened to its
    // worst-case mesh; a planet already publishes its exact surface.
    let bodies: Vec<(Meters3, Meters)> = stage::SALVAGE_ROCKS
        .into_iter()
        .chain(stage::AMBIENT_ROCKS)
        .map(|(position, nominal)| (position, Meters(nominal.0 * ASTEROID_GEOMETRIC_FACTOR_MAX)))
        .chain([
            (stage::INSPECTION_POS, stage::inspection_body_radius()),
            (stage::CONCEALMENT_POS, stage::concealment_body_radius()),
        ])
        .collect();
    let legs = [
        (
            "second crate to transit 1",
            CRATE_POSITIONS[1],
            TRANSIT_ONE.position,
        ),
        (
            "transit 1 to transit 2",
            TRANSIT_ONE.position,
            TRANSIT_TWO.position,
        ),
        (
            "orbit return gate to work site",
            ORBIT_RETURN_GATE_POS,
            WORK_SITE.position,
        ),
        (
            "last crate to Meridian hold",
            CRATE_POSITIONS[2],
            HOME_MARK.position,
        ),
    ];
    for (name, from, to) in legs {
        for (body, radius) in &bodies {
            let separation = distance_to_segment(*body, from, to);
            let required = radius.0 + CUTTER_RADIUS + FLIGHT_MARGIN;
            assert!(
                separation > required,
                "Cutter's '{name}' corridor passes {separation:.0} m from the \
                 body at {body:?}, inside its {required:.0} m flight envelope"
            );
        }
    }
}

/// The two prescribed transit legs route around the inspection body's gravity
/// well, not merely around its visible rock. A player following GOTO should not
/// cross the well on the way to beacons that were moved outside it.
#[test]
fn the_transit_route_clears_the_inspection_gravity_well() {
    const CUTTER_RADIUS: Meters = Meters(55.0);
    const FLIGHT_MARGIN: Meters = Meters(100.0);
    let settings = GravitySettings::default();
    let surface = stage::inspection_body_radius().to_engine();
    let soi = Meters::from_engine(
        GravityWell::from_mass(stage::INSPECTION_MASS, surface, &settings).soi_radius,
    );
    let required = soi + CUTTER_RADIUS + FLIGHT_MARGIN;
    for (name, from, to) in [
        (
            "second crate to transit 1",
            CRATE_POSITIONS[1],
            TRANSIT_ONE.position,
        ),
        (
            "transit 1 to transit 2",
            TRANSIT_ONE.position,
            TRANSIT_TWO.position,
        ),
    ] {
        let separation = distance_to_segment(stage::INSPECTION_POS, from, to);
        assert!(
            separation > required.0,
            "Cutter's '{name}' route passes {separation:.0} m from the inspection \
             body, inside its {:.0} m gravity-flight envelope",
            required.0,
        );
    }
}

/// Every leg the warship is ordered to fly is FLYABLE. A `MoveShipTo` is a
/// straight line with no avoidance of its own, so a mark on the far side of a
/// body is a hull grinding into it - which is how the warship used to die on
/// its way out, taking the last shot of the chapter with it.
#[test]
fn the_warship_never_flies_a_leg_through_a_body() {
    let bodies: Vec<(Meters3, Meters)> = stage::SALVAGE_ROCKS
        .into_iter()
        .chain(stage::AMBIENT_ROCKS)
        .map(|(position, nominal)| (position, Meters(nominal.0 * ASTEROID_GEOMETRIC_FACTOR_MAX)))
        .chain([
            (stage::INSPECTION_POS, stage::inspection_body_radius()),
            (stage::CONCEALMENT_POS, stage::concealment_body_radius()),
        ])
        .collect();
    let legs = [
        ("hide to emergence", WARSHIP_HIDE_POS, WARSHIP_EMERGE_POS),
        (
            "emergence to firing",
            WARSHIP_EMERGE_POS,
            WARSHIP_FIRING_POS,
        ),
        ("firing to exit", WARSHIP_FIRING_POS, WARSHIP_EXIT_POS),
    ];
    for (name, from, to) in legs {
        for (body, radius) in &bodies {
            let separation = distance_to_segment(*body, from, to);
            let required = radius.0;
            assert!(
                separation > required,
                "the warship's '{name}' leg passes {separation:.0} m from the \
                 body at {body:?}, inside its {required:.0} m worst-case mesh"
            );
        }
    }
}

/// Shortest distance from `point` to the segment `from`-`to`.
fn distance_to_segment(point: Meters3, from: Meters3, to: Meters3) -> f32 {
    let leg = to - from;
    let along = (leg.get().dot((point - from).get()) / leg.get().length_squared()).clamp(0.0, 1.0);
    (point - (from + leg * along)).length().0
}

/// The opening and RCS briefing return to gameplay. The attack does not:
/// Cutter frames the approach, the warship frames launch, Meridian frames the
/// lances, and Cutter frames both the torpedo kill and aftermath until teardown.
#[test]
fn the_cinematic_runs_its_shots_in_order_without_returning_attack_control() {
    let config = config();
    let mut shots: Vec<String> = Vec::new();
    let mut released = 0_usize;
    for action in all_actions(&config) {
        match action {
            EventActionConfig::SetCameraAnchor(shot) => {
                assert!(
                    spawned_ship(&config, &shot.anchor).is_some(),
                    "a shot is anchored to '{}', which the shift never spawns - \
                     the camera would have nothing to hang off",
                    shot.anchor
                );
                shots.push(shot.anchor);
            }
            EventActionConfig::ReleaseCamera(_) => released += 1,
            _ => {}
        }
    }
    assert_eq!(
        shots,
        vec![
            ID_CUTTER.to_string(),
            ID_CUTTER.to_string(),
            ID_CUTTER.to_string(),
            ID_WARSHIP.to_string(),
            ID_CARRIER.to_string(),
            ID_CUTTER.to_string(),
            ID_CUTTER.to_string(),
        ],
        "the shot list must run opening, RCS, approach, launch, rail impact, \
         torpedo impact, aftermath without an intermediate departure angle"
    );
    assert_eq!(
        released, 2,
        "the opening and RCS lesson return to gameplay; the attack stays cinematic"
    );

    // Inside the salvo chain, each weapon and movement runs under its shot.
    let salvo = all_actions(&config)
        .into_iter()
        .find_map(|action| match action {
            EventActionConfig::Sequence(chain) if chain.key == SEQ_SALVO => Some(chain),
            _ => None,
        })
        .expect("the salvo chain is gone");
    let step_with = |pred: &dyn Fn(&EventActionConfig) -> bool| {
        salvo
            .steps
            .iter()
            .position(|step| step.actions.iter().any(pred))
    };
    let on_warship = step_with(&|action| {
        matches!(action, EventActionConfig::SetCameraAnchor(shot) if shot.anchor == ID_WARSHIP)
    })
    .expect("the tubes are never filmed");
    let on_carrier = step_with(&|action| {
        matches!(action, EventActionConfig::SetCameraAnchor(shot) if shot.anchor == ID_CARRIER)
    })
    .expect("the impacts are never filmed");
    let first_bay = step_with(&|action| matches!(action, EventActionConfig::ForceTorpedoFire(_)))
        .expect("the salvo never launches");
    let first_lance = step_with(&|action| matches!(action, EventActionConfig::ForceRailgunFire(_)))
        .expect("the salvo never fires a lance");
    let lance_steps: Vec<Vec<&str>> = salvo
        .steps
        .iter()
        .filter_map(|step| {
            let sections: Vec<&str> = step
                .actions
                .iter()
                .filter_map(|action| match action {
                    EventActionConfig::ForceRailgunFire(fire) => Some(fire.section.as_str()),
                    _ => None,
                })
                .collect();
            (!sections.is_empty()).then_some(sections)
        })
        .collect();
    assert_eq!(
        lance_steps,
        vec![ships::BLOCK_WARSHIP_RAILGUN_IDS.to_vec()],
        "both spinal lances must fire in one step so neither hit is hidden by \
         debris from the other"
    );
    assert!(
        on_warship <= first_bay,
        "the bays open at step {first_bay} and the camera reaches the warship at \
         {on_warship} - the launch happens off screen"
    );
    assert!(
        on_carrier < first_lance,
        "the first lance fires at step {first_lance} and the camera reaches the \
         Meridian at {on_carrier} - the slug lands off screen"
    );

    let last_lance = salvo
        .steps
        .iter()
        .rposition(|step| {
            step.actions
                .iter()
                .any(|action| matches!(action, EventActionConfig::ForceRailgunFire(_)))
        })
        .expect("the salvo never fires a lance");
    let on_cutter = step_with(&|action| {
        matches!(action, EventActionConfig::SetCameraAnchor(shot) if shot.anchor == ID_CUTTER)
    })
    .expect("the salvo never comes back to the cutter");
    assert!(
        last_lance < on_cutter,
        "the camera leaves the Meridian at step {on_cutter} and the second lance \
         fires at {last_lance} - the guns land off screen"
    );

    let exit = step_with(&|action| {
        matches!(action, EventActionConfig::MoveShipTo(order) if order.order == ORDER_EXIT)
    })
    .expect("the warship never starts away");
    let aftermath_cutter = salvo
        .steps
        .iter()
        .rposition(|step| {
            step.actions.iter().any(|action| {
                matches!(action, EventActionConfig::SetCameraAnchor(shot)
                    if shot.anchor == ID_CUTTER)
            })
        })
        .expect("the aftermath never returns to Cutter");
    assert!(
        on_cutter < exit && exit < aftermath_cutter,
        "the Cutter view must hold through torpedo impacts at {on_cutter}, the \
         warship starting away at {exit}, and aftermath at {aftermath_cutter}"
    );
    assert!(
        salvo
            .steps
            .iter()
            .all(|step| step.actions.iter().all(|action| {
                !matches!(
                    action,
                    EventActionConfig::StoryMessage(_)
                        | EventActionConfig::ReleaseCamera(_)
                        | EventActionConfig::ResumePlayerControl(_)
                )
            })),
        "the destruction scene must stay silent and keep cinematic authority"
    );
    let distress = salvo
        .steps
        .iter()
        .position(|step| {
            let write = format!("{:?}", set_variable(VAR_BEAT, number(BEAT_DISTRESS)));
            step.actions
                .iter()
                .any(|action| format!("{action:?}") == write)
        })
        .expect("the salvo never opens the distress act");
    assert_eq!(
        aftermath_cutter, distress,
        "the distress act must open on the final Cutter aftermath shot"
    );
}

/// Cinematic suspension blocks the human controls but never installs a
/// scripted order on Cutter: when control returns, its helm is still its own.
#[test]
fn the_script_never_flies_the_cutter_for_the_player() {
    for action in all_actions(&config()) {
        let commanded = match &action {
            EventActionConfig::MoveShipTo(order) => Some(&order.ship),
            EventActionConfig::StopShip(order) => Some(&order.ship),
            EventActionConfig::ForceAlign(order) => Some(&order.ship),
            EventActionConfig::OrbitShip(order) => Some(&order.ship),
            EventActionConfig::PatrolShip(order) => Some(&order.ship),
            _ => None,
        };
        assert_ne!(
            commanded,
            Some(&ID_CUTTER.to_string()),
            "the script flies the player's ship for them: {action:?}"
        );
    }
}

#[test]
fn only_the_conversation_holds_return_control_before_teardown() {
    let config = config();
    let actions = all_actions(&config);
    let suspended = actions
        .iter()
        .filter(|action| matches!(action, EventActionConfig::SuspendPlayerControl(_)))
        .count();
    let resumed = actions
        .iter()
        .filter(|action| matches!(action, EventActionConfig::ResumePlayerControl(_)))
        .count();
    let released = actions
        .iter()
        .filter(|action| matches!(action, EventActionConfig::ReleaseCamera(_)))
        .count();

    assert_eq!(
        suspended, 3,
        "opening, RCS briefing and attack entry each own one control interval"
    );
    assert_eq!(
        resumed, 2,
        "the opening and RCS briefing each return player control"
    );
    assert_eq!(
        released, 2,
        "the opening and RCS briefing each return the chase camera"
    );
}

/// The cutter is unarmed and stays that way, and the helm verbs it starts
/// without are all handed back before the chapter ends. A verb withheld and
/// never granted is a control the player is taught about and never given.
#[test]
fn every_withheld_control_is_handed_back() {
    let config = config();
    let cutter = spawned_ship(&config, ID_CUTTER).expect("the shift spawns no cutter");
    let ScenarioObjectKind::Spaceship(ship) = &cutter.kind else {
        unreachable!("the cutter is a ship")
    };
    let withheld: Vec<FlightVerb> = ship
        .modifications
        .iter()
        .flat_map(|modification| modification.modifications.iter())
        .filter_map(|modification| match modification {
            SectionModification::DisableVerb(verb) => Some(*verb),
            _ => None,
        })
        .collect();
    assert!(!withheld.is_empty(), "the shift teaches nothing");

    for verb in withheld {
        let granted = all_actions(&config).into_iter().any(|action| {
            matches!(action, EventActionConfig::SetControllerVerb(set)
                if set.id == ID_CUTTER && set.verb == verb && set.enabled)
        });
        assert!(
            granted,
            "{verb:?} is withheld at spawn and never handed back"
        );
    }
}

/// Every beacon and salvage objective the player must physically reach stands
/// outside every gravity well on the map.
///
/// A beacon is not outside a gravity well merely because its centre clears the
/// numerical boundary. Its complete trigger volume must clear the well with a
/// visible authored buffer. Otherwise the marker still sits against a
/// planetoid, and part of the place it asks the player to occupy remains in
/// gravity.
///
/// GOTO also finishes by coming to rest, and inside a well the pull never stops:
/// the arrival cannot settle, the `OnGotoComplete` the beat waits on never
/// fires, and the shift stops dead. The well is built here exactly the way the
/// asteroid loader builds it - `GravityWell::from_mass` against the widest
/// geometric radius.
#[test]
fn no_beacon_or_salvage_objective_stands_in_a_gravity_well() {
    const VISIBLE_GRAVITY_BUFFER: Meters = Meters(500.0);
    let settings = GravitySettings::default();
    let mut wells = vec![
        (
            "Inspection Planetoid".to_string(),
            stage::INSPECTION_POS,
            stage::INSPECTION_MASS,
            stage::inspection_body_radius(),
        ),
        (
            "Concealment Planetoid".to_string(),
            stage::CONCEALMENT_POS,
            stage::CONCEALMENT_MASS,
            stage::concealment_body_radius(),
        ),
    ];
    wells.extend(
        stage::SALVAGE_ROCKS
            .into_iter()
            .enumerate()
            .filter(|(_, (_, radius))| radius.to_engine() >= settings.min_well_radius)
            .map(|(index, (position, radius))| {
                (
                    format!("Salvage Rock {}", index + 1),
                    position,
                    settings.default_mass,
                    radius,
                )
            }),
    );
    wells.extend(
        stage::AMBIENT_ROCKS
            .into_iter()
            .enumerate()
            .filter(|(_, (_, radius))| radius.to_engine() >= settings.min_well_radius)
            .map(|(index, (position, radius))| {
                (
                    format!("Belt Rock {}", index + 1),
                    position,
                    settings.default_mass,
                    radius,
                )
            }),
    );
    let marks = [
        &WORK_MARK,
        &TRIM_LATERAL,
        &TRIM_VERTICAL,
        &TRIM_RETURN_LATERAL,
        &TRIM_RETURN_VERTICAL,
        &TRANSIT_ONE,
        &TRANSIT_TWO,
        &WORK_SITE,
        &HOME_MARK,
    ];
    for (name, centre, mass, surface) in wells {
        let soi = Meters::from_engine(
            GravityWell::from_mass(mass, surface.to_engine(), &settings).soi_radius,
        );
        for mark in marks {
            let separation = (mark.position - centre).length();
            let required = soi + mark.area + VISIBLE_GRAVITY_BUFFER;
            assert!(
                separation.0 > required.0,
                "'{}' stands {:.0} m from {name}; its full beacon volume and \
                 500 m gravity buffer require {:.0} m - no beacon may stand \
                 against or inside a gravity well",
                mark.id,
                separation.0,
                required.0,
            );
        }
        for (index, position) in CRATE_POSITIONS.into_iter().enumerate() {
            let separation = (position - centre).length();
            let required = soi + CRATE_AREA_RADIUS + VISIBLE_GRAVITY_BUFFER;
            assert!(
                separation.0 > required.0,
                "crate {} stands {:.0} m from {name}; its pickup volume and \
                 500 m gravity buffer require {:.0} m - no salvage objective \
                 may stand against or inside a gravity well",
                index + 1,
                separation.0,
                required.0,
            );
        }
    }
}

#[test]
fn every_prescribed_beacon_is_acquirable_from_the_previous_goal() {
    let settings = TargetingSettings::default();
    let lock_range =
        Meters::from_engine(TRANSIT_SIGNATURE.to_engine() * settings.signature_range_per_unit);
    let legs = [
        (
            "crate 2 to transit 1",
            CRATE_POSITIONS[1],
            TRANSIT_ONE.position,
        ),
        (
            "transit 1 to transit 2",
            TRANSIT_ONE.position,
            TRANSIT_TWO.position,
        ),
        (
            "orbit return to work site",
            ORBIT_RETURN_GATE_POS,
            WORK_SITE.position,
        ),
        (
            "crate 3 to Meridian hold",
            CRATE_POSITIONS[2],
            HOME_MARK.position,
        ),
    ];
    for (name, from, to) in legs {
        let distance = (to - from).length();
        assert!(
            distance.0 < lock_range.0,
            "{name} is {:.0} m but the beacon is only acquirable within {:.0} m",
            distance.0,
            lock_range.0,
        );
    }
}

/// The torpedo kill and aftermath are filmed at a POINT, not at the ship being
/// killed.
///
/// An `Object` aim falls back to the anchor when that entity dies. Meridian
/// dies during the first Cutter shot and is absent in the second, so both must
/// retain the berth as a stable world-space subject.
#[test]
fn the_kill_is_filmed_at_a_point_that_outlives_the_carrier() {
    let death_shots: Vec<SetCameraAnchorActionConfig> = all_actions(&config())
        .into_iter()
        .filter_map(|action| match action {
            EventActionConfig::SetCameraAnchor(shot) => Some(shot),
            _ => None,
        })
        .filter(|shot| shot.offset == CINEMA_DEATH_OFFSET)
        .collect();
    assert_eq!(
        death_shots.len(),
        2,
        "the chapter needs one Cutter shot for the kill and one for aftermath"
    );
    for shot in death_shots {
        assert!(
            matches!(shot.look_at, CameraLookAtConfig::Point(_)),
            "a Cutter destruction shot follows a carrier that can no longer exist"
        );
        assert_eq!(
            shot.anchor, ID_CUTTER,
            "the destruction and aftermath stay on the player's own hull"
        );
    }
}

/// The orbit beat completes by crossing a gate, and the ring the player is on
/// is whatever the ORBIT verb clamped their approach radius into. A gate
/// narrower than that band is invisible to anyone flying a legal higher ring:
/// they orbit exactly as instructed, forever, with nothing on screen to
/// correct them. Derived from the engine's own band rather than restated, so
/// a gravity or flight retune fails here instead of in a playtest.
#[test]
fn the_orbit_return_gate_intercepts_every_ring_the_verb_can_plan() {
    use nova_gameplay::prelude::{GravitySettings, GravityWell};
    use nova_ship::prelude::{orbit_radius_band, FlightSettings};

    let gravity = GravitySettings::default();
    let well = GravityWell::from_mass(
        stage::INSPECTION_MASS,
        stage::inspection_body_radius().to_engine(),
        &gravity,
    );
    let (floor, ceiling) = orbit_radius_band(&well, &gravity, &FlightSettings::default())
        .expect("the inspection planetoid is orbitable");
    let (floor, ceiling) = (Meters::from_engine(floor), Meters::from_engine(ceiling));

    // A ring of radius R centred on the body reaches the gate sphere exactly
    // when |R - stand_off| <= gate radius.
    let stand_off = (ORBIT_RETURN_GATE_POS - stage::INSPECTION_POS).length();
    let reached = |ring: Meters| (ring.0 - stand_off.0).abs() <= ORBIT_RETURN_GATE_RADIUS.0;
    assert!(
        reached(floor) && reached(ceiling),
        "the gate spans {:.0}-{:.0} m of orbital radius but the verb plans rings from \
         {:.0} m to {:.0} m - a ring outside it can never complete the beat",
        stand_off.0 - ORBIT_RETURN_GATE_RADIUS.0,
        stand_off.0 + ORBIT_RETURN_GATE_RADIUS.0,
        floor.0,
        ceiling.0,
    );
}
