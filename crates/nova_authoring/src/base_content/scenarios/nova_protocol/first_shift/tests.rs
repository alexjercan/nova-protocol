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

fn config() -> ScenarioConfig {
    first_shift(AssetRef::default(), AssetRef::default())
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
const BEATS: [f64; 17] = [
    BEAT_LAUNCH,
    BEAT_TRIM_LATERAL,
    BEAT_TRIM_VERTICAL,
    BEAT_CRATE_FIRST,
    BEAT_CRATE_SECOND,
    BEAT_LOCK,
    BEAT_GOTO,
    BEAT_TRANSIT,
    BEAT_DETOUR,
    BEAT_ORBIT,
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

/// The thrusters are taught in OPEN SPACE, before the plate. Both trim marks
/// must be clear of every rock's worst-case rendered body by more than their
/// own trigger volume, and both lessons must run at beats BELOW the first
/// crate - the old script's first RCS translation was inside the densest rock
/// in the field, which is the last place to learn a new control.
#[test]
fn the_thrusters_are_taught_in_open_space_before_the_plate() {
    for mark in [&WORK_MARK, &TRIM_LATERAL, &TRIM_VERTICAL] {
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
    assert!(
        BEAT_TRIM_LATERAL < BEAT_CRATE_FIRST && BEAT_TRIM_VERTICAL < BEAT_CRATE_FIRST,
        "the plate opens before the thrusters are taught"
    );

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

/// The plate itself gets HARDER. The first crate sits on the open edge where a
/// mistake costs nothing and the second is well inside the rocks, so the field
/// teaches itself in the right order.
#[test]
fn the_field_work_runs_from_the_plate_edge_inward() {
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
    assert!(
        BEAT_ORBIT < BEAT_RETURN && BEAT_RETURN < BEAT_SEARCH,
        "the orbit does not sit before the return to the field work"
    );
    assert!(
        BEAT_CRATE_SECOND < BEAT_DETOUR,
        "the detour is proposed before any of the field work is done - it has \
         to be time the crew is stealing from a job in progress"
    );

    let config = config();
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
        matches!(closes_the_return.name, EventConfig::OnEnter),
        "the return completes on {:?} - it must complete on ARRIVING at the \
         work site, not on leaving the planetoid",
        closes_the_return.name
    );
    let gate = format!("{:?}", WORK_SITE.entered());
    assert!(
        closes_the_return
            .filters
            .iter()
            .any(|filter| format!("{filter:?}") == gate),
        "the return completes on entering something other than the work site"
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
        matches!(opens.name, EventConfig::OnEnter),
        "the warship comes out on {:?} rather than on the player reaching the \
         hold",
        opens.name
    );
    let gate = format!("{:?}", HOME_MARK.entered());
    assert!(
        opens
            .filters
            .iter()
            .any(|filter| format!("{filter:?}") == gate),
        "the warship comes out on entering something other than the hold"
    );
    assert!(
        BEAT_SEARCH < BEAT_VANTAGE && BEAT_VANTAGE < BEAT_ATTACK,
        "the run home does not sit between the last crate and the attack"
    );
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

/// Every leg the warship is ordered to fly is FLYABLE. A `MoveShipTo` is a
/// straight line with no avoidance of its own, so a mark on the far side of a
/// body is a hull grinding into it - which is how the warship used to die on
/// its way out, taking the last shot of the chapter with it.
#[test]
fn the_warship_never_flies_a_leg_through_a_body() {
    let bodies: Vec<(Meters3, Meters)> = stage::SALVAGE_ROCKS
        .into_iter()
        .chain(stage::AMBIENT_ROCKS)
        .chain([
            (stage::INSPECTION_POS, stage::INSPECTION_RADIUS),
            (stage::CONCEALMENT_POS, stage::CONCEALMENT_RADIUS),
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
            let required = radius.0 * ASTEROID_GEOMETRIC_FACTOR_MAX;
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

/// The cinematic is four shots in one order, and it comes home twice. The
/// entrance is filmed from the CUTTER (the player is in their own set piece),
/// the launch from the WARSHIP, the guns from the MERIDIAN, and the kill from
/// the CUTTER again - and the chase rig is handed back once across the long
/// middle leg and once BEFORE the aftermath, because the approach and the
/// distress beat are both the player's own view rather than shots. Losing a
/// release leaves the player watching the chapter from a pose they cannot fly
/// out of.
#[test]
fn the_cinematic_runs_its_shots_in_order_and_hands_the_camera_back() {
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
            ID_WARSHIP.to_string(),
            ID_CARRIER.to_string(),
            ID_CUTTER.to_string(),
        ],
        "the shot list is out of order: the entrance is the cutter's, the \
         tubes are the warship's, the guns are the Meridian's, and the kill is \
         the cutter's again"
    );
    assert_eq!(
        released, 2,
        "the camera is handed back {released} times - it comes home once across \
         the warship's second leg and once before the aftermath"
    );

    // Inside the salvo chain: the guns fire under the shot they belong to, and
    // the release step lands before the step that opens the distress act.
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
            .position(|step| step.actions.iter().any(|action| pred(action)))
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

    let release = step_with(&|action| matches!(action, EventActionConfig::ReleaseCamera(_)))
        .expect("the salvo never releases the camera");
    assert!(
        on_cutter < release,
        "the camera is anchored to the carrier until step {release} - it would \
         be handed back by the carrier's own destruction, on the frame the \
         chapter is about"
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
    assert!(
        release < distress,
        "the camera is handed back at step {release} and the aftermath opens at \
         {distress} - the player would watch it from the cinematic pose"
    );
}

/// Camera authority is not helm authority. Nothing in the chapter steers or
/// stops the cutter: the player flies their own ship through the whole set
/// piece, and the composition is bought by putting the work where the shot
/// wants it rather than by taking the controls away.
#[test]
fn the_shift_never_takes_the_helm_off_the_player() {
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
