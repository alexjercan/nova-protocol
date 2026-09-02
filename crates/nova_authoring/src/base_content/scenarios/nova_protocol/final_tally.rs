//! Final Tally - chapter three, part two: the finale at the gang's claim
//! (spike).
//!
//! The intercepted burn from Lifeline traces to a dead claim: a cracked
//! megahauler anchorage berthed deep in a planetoid's gravity well - the
//! base chain's FIRST combat gravity well, ringed by a scattered belt (the
//! Ring region's first combat use). The player coasts into the pull (the
//! tutorial's gravity-coast beat, now with stakes), SURVEYS the anchorage
//! by holding a travel lock on it (the lock verb reused narratively), breaks
//! the two-ship orbital picket - guards on rails, the orbit directive's
//! first combat use (pinned by ai.rs orbit_directive_tests) - and then the
//! Final Tally itself casts off with an escort: the campaign's only
//! simultaneous capital + escort fight, and its peak.
//!
//! The ending is a proper close, not an omission: the flagship kill opens a
//! clock-gated epilogue (confirm line, then the guild's close, then the
//! Victory overlay) and the campaign completes with NOTHING queued - by
//! design this time, stated in the banner.
//!
//! Structure notes: the planetoid sits at WORLD ORIGIN because ScatterObjects'
//! Ring region is origin-centred (sample replaces the template position);
//! everything else is authored around it. Gates are FLAG-based (surveyed,
//! picket kills), not act-sequenced, so killing the picket before surveying
//! cannot deadlock the cast-off. Terminal acts close every outcome gate the
//! moment any outcome is declared (LESSONS:
//! outcome-is-last-write-wins-close-the-act): act 1 live, 4 epilogue (the win
//! is locked - a post-kill death declares nothing), 2 won, 3 lost.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use super::{
    cast::{BELT_RELAY, CAPTAIN_HALLORAN, TALLYMAN},
    pacing::{self, MID_GAP, REVEAL_GAP},
    ships, SCATTER_SEED, SCENARIO_ELAPSED_VAR,
};
use crate::scenario_helpers::prelude::*;

pub(crate) const FINAL_TALLY_SCENARIO_ID: &str = "final_tally";

const ID_PLAYER: &str = "player_spaceship";
/// The gravity well the claim hides in. The id doubles as the pickets'
/// orbit-directive target.
const ID_ANCHOR: &str = "claim_anchor";
/// The cracked megahauler's two hull sections - invulnerable set dressing,
/// hard cover, and the SURVEY target (the bow carries the long-range lock
/// signature).
const ID_WRECK_BOW: &str = "anchorage_bow";
const ID_WRECK_STERN: &str = "anchorage_stern";
const ID_PICKET_A: &str = "picket_a";
const ID_PICKET_B: &str = "picket_b";
const ID_FLAGSHIP: &str = "flagship";
const ID_ESCORT: &str = "escort";

const OBJ_SURVEY: &str = "survey";
const OBJ_PICKET: &str = "picket";
const OBJ_BREAK: &str = "break_flagship";

/// Story act: 1 = live (approach, survey, both fights), 4 = the epilogue
/// (flagship dead, the win locked - no outcome can overwrite it), 2 = won,
/// 3 = lost. Terminal acts per the ledger lesson.
const VAR_ACT: &str = "act";
/// The epilogue act: the flagship is dead and the win locked, but the banner
/// has not landed. It sits OUTSIDE the defeat gates (`act == 1`), so a death
/// during the outro beats cannot overwrite the win.
const ACT_EPILOGUE: f64 = 4.0;
const ACT_WON: f64 = 2.0;
/// One-shot: the anchorage has been surveyed (travel lock held on the bow).
const VAR_SURVEYED: &str = "surveyed";
/// Per-picket kill flags (the broadside pattern: flags, not counters).
const VAR_PICKET_A_DOWN: &str = "picket_a_down";
const VAR_PICKET_B_DOWN: &str = "picket_b_down";
/// Pacing: objectives post a beat after the comms line that introduces them.
/// The keys name the one-step sequences the introducing beats start; the ENGINE
/// holds the delay, so there is no gate variable to seed.
const SEQ_OPENING: &str = "opening";
const SEQ_BREAK: &str = "break_objective";
/// The cast-off chain: started by the pickets-down beat, it owes BOTH the
/// breathe and the survey. The survey can still be outstanding when the last
/// picket dies, so the step carries an `until` gate as well as its delay - and
/// a deadline, because a cast-off that never arrives strands the scenario with
/// no flagship to break.
const SEQ_CAST_OFF: &str = "cast_off";
/// The picket objective's beat is a TIMER, not a sequence step: it must be
/// ABANDONED if both pickets die inside the gap, or the objective would post
/// pointing at two dead ships with nothing left to complete it. A step runs when
/// its delay elapses; only a handler can still ask whether the beat is current.
const TIMER_PICKET_GATE: &str = "picket_gate";

/// Halloran's sendoff, one breath behind the opening dispatch.
const HELLO_AT: f64 = 9.0;
/// Breathe between pickets-down and the cast-off.
const CAST_OFF_DELAY: f64 = 6.0;
/// The cast-off's backstop. The survey is player-paced and untimed, so this is
/// far longer than any play of the beat: it exists so a cast-off that can never
/// arrive fails loudly instead of leaving the claim empty.
const CAST_OFF_DEADLINE: f64 = 600.0;

/// The planetoid: nominal 20u, surface gravity 6 - the shakedown
/// planetoid's proven numbers (geometric body 70-120u, SOI 560-960u, from
/// the measured ASTEROID_GEOMETRIC_FACTOR range; the harness pins the
/// derived clearances).
const ANCHOR_POS: Vec3 = Vec3::new(0.0, -20.0, 0.0);
const ANCHOR_RADIUS: f32 = 20.0;
/// Player spawn: outside even the worst-seed SOI (960u from the well), so
/// the approach COASTS into the pull - the tutorial callback.
const PLAYER_SPAWN: Vec3 = Vec3::new(0.0, 20.0, 1150.0);
/// The anchorage: two big invulnerable hull-section rocks off the
/// planetoid's shoulder, clear of its worst-case body.
const WRECK_BOW_POS: Vec3 = Vec3::new(200.0, 20.0, 140.0);
const WRECK_STERN_POS: Vec3 = Vec3::new(-90.0, -40.0, 230.0);
/// Picket spawns: on the well's wire, opposite shoulders, both outside the
/// raider design floor (700u) of the player spawn.
const PICKET_A_SPAWN: Vec3 = Vec3::new(300.0, 0.0, 100.0);
const PICKET_B_SPAWN: Vec3 = Vec3::new(-280.0, 40.0, -120.0);
/// The cast-off berth: the flagship and its escort emerge from BEHIND the
/// anchorage bow (triggered spawns, kept outside the flagship's own 1000u
/// torpedo envelope of the player SPAWN - 952u tripped the balance WARN at
/// z=210, so the berth sits deeper; the audit stays clean with zero acks).
const FLAGSHIP_SPAWN: Vec3 = Vec3::new(150.0, -10.0, 90.0);
const ESCORT_SPAWN: Vec3 = Vec3::new(60.0, 30.0, 280.0);
/// The long-range survey signature on the anchorage bow: lockable from the
/// coast-in (default beacon signature 20 reads ~600u; the bow reads ~1350u).
const WRECK_SURVEY_SIGNATURE: f32 = 45.0;

fn facing_the_approach() -> Quat {
    Quat::from_rotation_y(std::f32::consts::PI)
}

/// The player's finale ship: unchanged from Lifeline/Broadside.
fn player_ship() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PLAYER.to_string(),
            name: "Player Spaceship".to_string(),
            position: PLAYER_SPAWN,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: ships::CARGOA_TURRET_IDS
                    .iter()
                    .map(|id| {
                        (
                            id.to_string(),
                            vec![
                                MouseButton::Left.into(),
                                GamepadButton::RightTrigger2.into(),
                            ],
                        )
                    })
                    .collect(),
                speed_cap: None,
            }),
            allegiance: None,
            hull: ships::hull(ships::CARGOA_SHIP_ID),
            modifications: vec![ships::on_section(
                ships::FUSELAGE_SECTION_ID,
                vec![SectionModification::DisableVerb(FlightVerb::Rcs)],
            )],
        }),
    }
}

/// The claim's planetoid: invulnerable, gravity-authored - the well the
/// whole finale is staged in, and the pickets' orbit target.
fn claim_anchor(asteroid_texture: &AssetRef<Image>) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_ANCHOR.to_string(),
            name: "The Claim".to_string(),
            position: ANCHOR_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            material: None,
            destroy_sound: None,
            radius: ANCHOR_RADIUS,
            texture: asteroid_texture.clone(),
            mass: Some(45_000.0),
            invulnerable: true,
            seed: None,
            lock_signature: None,
        }),
    }
}

/// An anchorage hull section: a big invulnerable rock as set dressing - hard
/// cover in the well, and the bow carries the survey signature.
fn anchorage_wreck(
    id: &str,
    name: &str,
    position: Vec3,
    radius: f32,
    lock_signature: Option<f32>,
    asteroid_texture: &AssetRef<Image>,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            material: None,
            destroy_sound: None,
            radius,
            texture: asteroid_texture.clone(),
            mass: None,
            invulnerable: true,
            seed: None,
            lock_signature,
        }),
    }
}

/// A picket guard: scavenger-grade cargoa corvette holding an ORBIT directive around
/// the claim - a guard on rails (combat pulls it off the orbit, calm
/// returns it; ai.rs orbit_directive_tests). Graced like every telegraphed
/// hostile; leashed to the well so the fight stays in the pull.
fn picket(id: &str, spawn_pos: Vec3) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: "Tally Picket".to_string(),
            position: spawn_pos,
            rotation: facing_the_approach(),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                orbit: Some(ID_ANCHOR.to_string()),
                leash: Some(600.0),
                engage_delay: Some(8.0),
                ..Default::default()
            }),
            allegiance: None,
            hull: ships::hull(ships::CARGOA_RAIDER_SHIP_ID),
            ..Default::default()
        }),
    }
}

/// The Final Tally: the gang's flagship - the cargob capital at full grade
/// (two PDC turrets, two torpedo tubes), no leash: it casts off to end it.
///
/// The SERPENT cargo-B, against the Lances the chapter-two gunship opened with.
/// That is the escalation, and it costs the fight nothing else: the same hull,
/// the same twelve-torpedo alpha strike, the same warhead - flown so a lead
/// solution cannot hold it. The player has had two chapters of screening the
/// straight ones to earn it.
fn flagship() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_FLAGSHIP.to_string(),
            name: "Flagship Final Tally".to_string(),
            position: FLAGSHIP_SPAWN,
            rotation: facing_the_approach(),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig::default()),
            allegiance: None,
            hull: ships::hull(ships::CARGOB_SHIP_ID),
            ..Default::default()
        }),
    }
}

/// The flagship's escort: a scavenger-grade cargoa corvette screening the capital
/// (first-pass grade; the playtest tunable is one word).
fn escort() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_ESCORT.to_string(),
            name: "Tally Escort".to_string(),
            position: ESCORT_SPAWN,
            rotation: facing_the_approach(),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: vec![ESCORT_SPAWN, FLAGSHIP_SPAWN + Vec3::new(0.0, 40.0, 80.0)],
                leash: Some(700.0),
                engage_delay: Some(4.0),
                ..Default::default()
            }),
            allegiance: None,
            hull: ships::hull(ships::CARGOA_RAIDER_SHIP_ID),
            ..Default::default()
        }),
    }
}

/// The belt ring around the claim: the Ring region's first combat use -
/// destructible chaff orbiting the well's plane.
fn claim_belt(asteroid_texture: &AssetRef<Image>) -> EventActionConfig {
    EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "belt_rock_".to_string(),
        count: 16,
        seed: SCATTER_SEED,
        region: ScatterRegion::Ring {
            center: Vec3::ZERO,
            inner: 260.0,
            outer: 420.0,
            y_min: -70.0,
            y_max: -10.0,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "belt_rock_".to_string(),
                name: "Claim Belt Rock".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                material: None,
                destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
                radius: 1.0,
                texture: asteroid_texture.clone(),
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.5, 3.5)),
        min_separation: None,
    })
}

/// Filter: the player's travel lock landed on `target` (OnTravelLockStart:
/// id = the locked object, other = the locking ship).
fn player_travel_locks(target: &str) -> EventFilterConfig {
    EventFilterConfig::Entity(EntityFilterConfig {
        id: Some(target.to_string()),
        other_id: Some(ID_PLAYER.to_string()),
        ..default()
    })
}

/// The CAST-OFF chain, started by the pickets-down beat: the Final Tally and
/// its escort emerge from behind the anchorage, and the break objective posts a
/// beat behind the reveal.
///
/// The step owes BOTH waits. The breathe is the pacing beat between the taunt
/// and the reveal; the gate is the survey, which can still be outstanding when
/// the last picket dies. The two run together, so a player who surveyed first
/// waits only the breathe.
fn cast_off() -> EventActionConfig {
    sequence(
        SEQ_CAST_OFF,
        vec![until_step(
            CAST_OFF_DELAY,
            EventConfig::OnUpdate,
            vec![number_equals(VAR_SURVEYED, 1.0)],
            CAST_OFF_DEADLINE,
            vec![
                story_message(
                    BELT_RELAY,
                    "Capital burn off the anchorage - tubes open. That's \
                     the flagship.",
                ),
                spawn_object(flagship()),
                spawn_object(escort()),
                // Threat reveal (the capital ship emerges): full absorb beat -
                // the flagship's approach IS the peak-fight framing. The marker
                // is set with the reveal.
                pacing::beat_later(
                    SEQ_BREAK,
                    REVEAL_GAP,
                    vec![post_objective(OBJ_BREAK, "Break the Final Tally.")],
                ),
                attach_objective_marker(ID_FLAGSHIP, "FINAL TALLY"),
            ],
        )],
    )
}

/// The epilogue chain: the guild's close, then the banner. Both flagship-kill
/// variants start the same cursor - only one of them can ever fire.
///
/// The campaign ends here by design, so nothing is queued behind the banner -
/// the banner says so.
fn epilogue() -> EventActionConfig {
    pacing::outro_sequence(
        VAR_ACT,
        ACT_WON,
        CAPTAIN_HALLORAN,
        "Quota's settled, pilot. The guild will not forget whose guns held \
         the line.",
        "The claim is quiet. The Tallyman's ledger is closed, his flagship is \
         drift, and the belt's lanes are open. End of the base campaign - for \
         now.",
        vec![],
        None,
    )
}

pub(crate) fn final_tally(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut opening = vec![
        set_variable(VAR_ACT, number(1.0)),
        set_variable(VAR_SURVEYED, number(0.0)),
        set_variable(VAR_PICKET_A_DOWN, number(0.0)),
        set_variable(VAR_PICKET_B_DOWN, number(0.0)),
        spawn_object(player_ship()),
        spawn_object(claim_anchor(&asteroid_texture)),
        spawn_object(anchorage_wreck(
            ID_WRECK_BOW,
            "Anchorage Bow",
            WRECK_BOW_POS,
            8.0,
            Some(WRECK_SURVEY_SIGNATURE),
            &asteroid_texture,
        )),
        spawn_object(anchorage_wreck(
            ID_WRECK_STERN,
            "Anchorage Stern",
            WRECK_STERN_POS,
            6.5,
            None,
            &asteroid_texture,
        )),
        spawn_object(picket(ID_PICKET_A, PICKET_A_SPAWN)),
        spawn_object(picket(ID_PICKET_B, PICKET_B_SPAWN)),
        claim_belt(&asteroid_texture),
        story_message(
            BELT_RELAY,
            "The raiders' burn traces to a dead claim: a cracked megahauler \
             berthed deep in a planetoid's pull. Confirm what's hiding there.",
        ),
        // The opening chain. Reveal-then-instruct: "confirm what's hiding
        // there" sets up, the objective explains the travel-lock mechanic - a
        // mid gap, so the objective never shares a frame with the dispatch.
        // Halloran's sendoff follows a breath later. The anchorage marker is
        // already up (below).
        sequence(
            SEQ_OPENING,
            vec![
                step(
                    MID_GAP,
                    vec![post_objective(
                        OBJ_SURVEY,
                        "Survey the anchorage - hold a travel lock on the wreck's bow.",
                    )],
                ),
                step(
                    HELLO_AT - MID_GAP,
                    vec![story_message(
                        CAPTAIN_HALLORAN,
                        "Whatever is berthed in that pull, pilot - the guild \
                         settles its debts. So does he.",
                    )],
                ),
            ],
        ),
        attach_objective_marker(ID_WRECK_BOW, "ANCHORAGE"),
    ];
    // Scale 20, not the usual 10: the claim's planetoid reaches ANCHOR_RADIUS *
    // ASTEROID_GEOMETRIC_FACTOR_MAX (~113u) on its worst seed, and a scale-10
    // key light would sit inside that body. Cosmetic for a directional light -
    // only its direction is read - but `final_tally_claim` asserts that nothing
    // spawns inside the planetoid, and a sun inside the planet is a lie anyway.
    opening.extend(ThreePointRig::around("anchorage", ANCHOR_POS, 20.0).actions());

    let events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: opening,
        },
        // The SURVEY: the travel lock lands on the bow. TWO fate variants
        // (the lifeline banner pattern): the pickets may already be drift when
        // the survey lands - that path must not post a picket objective nothing
        // will ever complete, nor mark two dead ships.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTravelLockStart,
            once: true,
            filters: vec![
                player_travel_locks(ID_WRECK_BOW),
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_SURVEYED, 0.0),
                EventFilterConfig::Conditional(ConditionalFilterConfig::Or(
                    Box::new(number_equals(VAR_PICKET_A_DOWN, 0.0)),
                    Box::new(number_equals(VAR_PICKET_B_DOWN, 0.0)),
                )),
            ],
            actions: vec![
                set_variable(VAR_SURVEYED, number(1.0)),
                complete_objective(OBJ_SURVEY),
                detach_objective_marker(ID_WRECK_BOW),
                story_message(
                    BELT_RELAY,
                    "Confirmed: the Final Tally, berthed hot behind the \
                     wreck. Two pickets riding the well.",
                ),
                // The confirm line reveals the pickets (already on-screen
                // orbiting), so the reveal is short - a mid gap lands "break
                // the picket" snappier without stepping on the line.
                start_timer(TIMER_PICKET_GATE, MID_GAP),
            ],
        },
        // The picket objective, a beat after the survey confirm. Guarded on at
        // least one picket still live: if BOTH die inside the beat, the objective
        // never posts (the pickets-down beat below drives on), so nothing is left
        // pointing at dead ships. The timer is only started on the pickets-live
        // survey path, so this cannot fire on the already-drift variant.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: true,
            filters: vec![
                timer(TIMER_PICKET_GATE),
                EventFilterConfig::Conditional(ConditionalFilterConfig::Or(
                    Box::new(number_equals(VAR_PICKET_A_DOWN, 0.0)),
                    Box::new(number_equals(VAR_PICKET_B_DOWN, 0.0)),
                )),
            ],
            actions: vec![
                post_objective(OBJ_PICKET, "Break the orbital picket."),
                attach_objective_marker(ID_PICKET_A, "PICKET"),
                attach_objective_marker(ID_PICKET_B, "PICKET"),
            ],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTravelLockStart,
            once: true,
            filters: vec![
                player_travel_locks(ID_WRECK_BOW),
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_SURVEYED, 0.0),
                number_equals(VAR_PICKET_A_DOWN, 1.0),
                number_equals(VAR_PICKET_B_DOWN, 1.0),
            ],
            actions: vec![
                set_variable(VAR_SURVEYED, number(1.0)),
                complete_objective(OBJ_SURVEY),
                detach_objective_marker(ID_WRECK_BOW),
                story_message(
                    BELT_RELAY,
                    "Confirmed: the Final Tally, berthed hot behind the \
                     wreck - and its pickets are already drift.",
                ),
            ],
        },
        // Picket defeat flags (unconditional, one handler each).
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDefeated,
            once: true,
            filters: vec![entity(ID_PICKET_A)],
            actions: vec![
                set_variable(VAR_PICKET_A_DOWN, number(1.0)),
                detach_objective_marker(ID_PICKET_A),
            ],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDefeated,
            once: true,
            filters: vec![entity(ID_PICKET_B)],
            actions: vec![
                set_variable(VAR_PICKET_B_DOWN, number(1.0)),
                detach_objective_marker(ID_PICKET_B),
            ],
        },
        // Both pickets down: the Tallyman's last taunt, and the cast-off chain
        // starts. Flag-gated, not act-sequenced, so a pre-survey picket kill
        // cannot deadlock - the chain's step owes the survey as well as the
        // breathe, and only the ENGINE holds that wait.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_PICKET_A_DOWN, 1.0),
                number_equals(VAR_PICKET_B_DOWN, 1.0),
            ],
            actions: vec![
                complete_objective(OBJ_PICKET),
                story_message(
                    TALLYMAN,
                    "You counted my pickets, pilot. Now count the tubes on \
                     my flagship.",
                ),
                cast_off(),
            ],
        },
        // The KILL: the epilogue opens. Act 4 locks the win (a post-kill
        // player death declares nothing; the escort's fate is its own -
        // it runs, narratively). The confirm line fires now; the close
        // and the banner ride the epilogue clock.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_FLAGSHIP), number_equals(VAR_ACT, 1.0)],
            actions: pacing::open_outro(
                VAR_ACT,
                ACT_EPILOGUE,
                epilogue(),
                vec![
                    complete_objective(OBJ_BREAK),
                    detach_objective_marker(ID_FLAGSHIP),
                    story_message(
                        BELT_RELAY,
                        "The Final Tally is breaking up. The claim is going dark.",
                    ),
                ],
            ),
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnNeutralized,
            once: true,
            filters: vec![entity(ID_FLAGSHIP), number_equals(VAR_ACT, 1.0)],
            actions: pacing::open_outro(
                VAR_ACT,
                ACT_EPILOGUE,
                epilogue(),
                vec![
                    complete_objective(OBJ_BREAK),
                    detach_objective_marker(ID_FLAGSHIP),
                    story_message(
                        BELT_RELAY,
                        "The Final Tally hangs dead - guns cold, engines dark. \
                         The claim is going dark.",
                    ),
                ],
            ),
        },
        // Lose: the player dies while the fight is LIVE (act 1 only - the
        // epilogue's act 4 locks the win; terminal act 3 closes every gate
        // per the ledger lesson). Retry THIS scenario.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_PLAYER), number_equals(VAR_ACT, 1.0)],
            actions: vec![
                set_variable(VAR_ACT, number(3.0)),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "The claim keeps its secret, and the Tallyman keeps \
                     the belt.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: FINAL_TALLY_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnNeutralized,
            once: true,
            filters: vec![entity(ID_PLAYER), number_equals(VAR_ACT, 1.0)],
            actions: vec![
                set_variable(VAR_ACT, number(3.0)),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Nothing left to fight with - you drift, and the Tallyman keeps the belt.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: FINAL_TALLY_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
    ];

    ScenarioConfig {
        id: FINAL_TALLY_SCENARIO_ID.to_string(),
        name: "Final Tally".to_string(),
        description: "The trace ends at the gang's claim: survey the \
                      anchorage, break the orbital picket, and finish the \
                      Final Tally in its own gravity well. Chapter three of \
                      the base storyline, part two."
            .to_string(),
        cubemap,
        // A mid-story continuation reached from Lifeline's victory chain (the
        // Broadside-gunship precedent).
        // Generated placeholder art (scripts/gen-scenario-thumbnails.py);
        // real art overwrites this same path with no code change.
        thumbnail: Some(AssetRef::from("self://thumbnails/final_tally.png")),
        // Hidden from the flat picker, but a member of the `nova_protocol`
        // campaign mapping so the finale is replayable from the campaign
        // header.
        hidden: true,
        menu_backdrop: false,
        skybox_brightness: DEFAULT_SKYBOX_BRIGHTNESS,
        watches: vec![scenario_elapsed_watch(SCENARIO_ELAPSED_VAR)],
        events,
    }
}
