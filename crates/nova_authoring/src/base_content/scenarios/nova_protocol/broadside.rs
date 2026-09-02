//! Broadside - the capital-combat vertical slice, act-split for checkpointed
//! retries by the difficulty rework (spike F4/F7).
//!
//! Chapter two of the base storyline: the scavenger driven off in Shakedown
//! Run was a scout. Its gang comes back in force to strip the belt - and a
//! neutral yacht is caught in the middle. The chapter now plays as TWO
//! scenarios so a death retries the current act, never the whole chapter:
//!
//! - `broadside` (part one): act 0 (contact) - answer the yacht's distress
//!   call across the cover field; act 1 (escalation) - two scavenger
//!   corvettes jump the player at the yacht. Breaking the pair is the
//!   chapter's CHECKPOINT: a Victory beat chains (lingering) into part two.
//! - `broadside_gunship` (part two, hidden): the gang's GUNSHIP burns in
//!   from the dark - a capital with turrets and torpedo tubes. Screen its
//!   torpedoes with the PDC, then break it section by section. Dying here
//!   retries HERE.
//!
//! Win: gunship destroyed -> Victory overlay whose lingering chain enters
//! chapter three (`lifeline`). Lose: player destroyed -> Defeat + lingering
//! retry of the current part. The yacht is a NEUTRAL ship (the
//! `SpaceshipConfig.allegiance` override): nobody targets it, but stray blast
//! damage can kill it - a flavor beat reacts, the mission continues.
//!
//! Cover comes in two tiers since the AI line-of-fire gate (2d006707):
//! five fixed INVULNERABLE boulders anchor the yacht fight and the gunship
//! lane (real pressure relief - the AI holds fire when one blocks the
//! shot), while the seeded 24-rock scatter stays destructible chaff.
//!
//! Distances are authored against the measured AI constants
//! (crates/nova_ship/src/input/ai): engage range 8 km, torpedo envelope [3
//! x blast_radius, 10 km] with a 10s per-bay cadence and the first launch
//! immediate, standoff orbit ~2.5 km. The gunship spawns ~7.2 km from the
//! yacht fight so it engages on arrival and its tubes are open through the
//! whole approach.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use super::{
    cast::{BELT_RELAY, CAPTAIN_HALLORAN, RUST_TALLY},
    pacing::{self, MID_GAP, REVEAL_GAP},
    ships, SCATTER_SEED, SCENARIO_ELAPSED_VAR,
};
use crate::scenario_helpers::prelude::*;

/// Ships spawn with -Z forward; the fight sits +Z of both the corvette and
/// gunship spawns, so every combatant is authored with this about-face - the
/// gunship's torpedo alignment gate (cos > 0.5 on the hull bearing) opens on
/// arrival instead of after a 180-degree slew.
fn facing_the_fight() -> Quat {
    Quat::from_rotation_y(std::f32::consts::PI)
}

pub(crate) const BROADSIDE_SCENARIO_ID: &str = "broadside";
pub(crate) const BROADSIDE_GUNSHIP_SCENARIO_ID: &str = "broadside_gunship";

const ID_PLAYER: &str = "player_spaceship";
const ID_HAULER: &str = "hauler";
const ID_HAULER_AREA: &str = "hauler_area";
const ID_CORVETTE_A: &str = "corvette_a";
const ID_CORVETTE_B: &str = "corvette_b";
const ID_GUNSHIP: &str = "gunship";

const OBJ_CONTACT: &str = "contact";
const OBJ_DEFEND: &str = "defend";
const OBJ_SCREEN: &str = "screen";
const OBJ_BREAK: &str = "break";

/// Story act. Part one: 0 contact, 1 corvettes, 2 checkpoint won. Part two
/// (broadside_gunship): 1 the capital fight, 2 won. Both parts pass through
/// 4 - the OUTRO, where the fight is decided and the win locked but the
/// banner has not landed yet. Every gate filter checks it, so beats fire once
/// and in order within each part.
const VAR_ACT: &str = "act";

/// The outro act: the fight is won, the overlay has not shown. It sits
/// OUTSIDE every defeat gate (they read `act < 2` or `act == 1`), so a death
/// during the outro beats cannot overwrite the win.
const ACT_OUTRO: f64 = 4.0;
const ACT_WON: f64 = 2.0;

/// Per-corvette kill flags: two independent OnDestroyed handlers set them,
/// and the act-2 escalation gates on BOTH - no arithmetic counter, so a
/// double-fire cannot skip the gate (count-gate-use-gt-not-eq by
/// construction).
const VAR_CORVETTE_A_DOWN: &str = "corvette_a_down";
const VAR_CORVETTE_B_DOWN: &str = "corvette_b_down";
/// Whether the Ceres Queen died to stray fire this part (0/1). Seeded 0 on
/// start; the soft-fail beat raises it, and the Victory beat reads it to pick
/// its banner variant - protecting her finally gets acknowledged (voice pass).
/// Scenario-scoped like every variable: each part tracks its OWN yacht (state
/// does not cross the checkpoint).
const VAR_HAULER_LOST: &str = "hauler_lost";

/// Pacing: objectives post a beat AFTER the comms line that introduces them,
/// never the same frame. Each key names the one-step sequence the introducing
/// beat starts; the ENGINE holds the delay, so there is no gate variable to
/// seed and no act guard to prove the beat is still current. Part one: the
/// contact objective (after the distress call) and the defend objective (after
/// the ambush line). Part two names its own in a separate scope.
const SEQ_CONTACT: &str = "contact";
const SEQ_DEFEND: &str = "defend";
const SEQ_GUN_OBJECTIVE: &str = "gun_objective";

/// The yacht drifts here; the fight happens around it.
const HAULER_POS: Meters3 = Meters3::new(0.0, 100.0, -4_500.0);
/// Player spawn, looking down the lane toward the yacht.
const PLAYER_SPAWN: Meters3 = Meters3::new(0.0, 0.0, 400.0);
/// Corvettes jump the player from the yacht's flanks.
const CORVETTE_A_SPAWN: Meters3 = Meters3::new(1_400.0, 300.0, -5_600.0);
const CORVETTE_B_SPAWN: Meters3 = Meters3::new(-1_500.0, -200.0, -5_400.0);
/// The gunship burns in from deep field: ~7.2 km past the yacht, inside its
/// own engage range (8 km) of the fight the moment it spawns, torpedo
/// envelope (<= 10 km) open through the whole approach.
const GUNSHIP_SPAWN: Meters3 = Meters3::new(800.0, 600.0, -11_700.0);

/// The player's chapter-two ship: shakedown's trainer plus a second hull and
/// the better turret. NO torpedo bay - torpedoes are the ENEMY's weapon this
/// chapter (story: not unlocked yet), which keeps the PDC-screening fantasy
/// pure: you shoot torpedoes down, you don't trade them. Finite ammo: catalog
/// weapons auto-reload, so a dry magazine is a pacing beat, not a fail state.
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
                // Both corvette turret cubes fire on LMB / right trigger.
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
                // Post-tutorial: unbounded burn.
                speed_cap: None,
                // Finite ammo: catalog weapons auto-reload, so the PDC
                // screen-and-brawl plays with real magazines and the diegetic
                // ammo gauge instead of unlimited fire.
            }),
            allegiance: None,
            // The cargoa corvette. RCS is off in the mainline campaign until
            // the rework; no other verb is gated this chapter.
            hull: ships::hull(ships::CARGOA_SHIP_ID),
            modifications: vec![ships::on_section(
                ships::FUSELAGE_SECTION_ID,
                vec![SectionModification::DisableVerb(FlightVerb::Rcs)],
            )],
        }),
    }
}

/// The neutral yacht: drive stripped, adrift by the derelict field. No
/// controller (it cannot fly), NEUTRAL allegiance (nobody's AI targets it),
/// but real sections with real health - stray blast damage can kill it and
/// the story notices.
fn yacht_ship() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_HAULER.to_string(),
            name: "Yacht Ceres Queen".to_string(),
            position: HAULER_POS,
            rotation: Quat::from_rotation_y(0.6),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Neutral),
            // The Ceres Queen is the racer hull - a sleek, unarmed pleasure
            // yacht caught in the crossfire, the client the chapter protects.
            hull: ships::hull(ships::RACER_SHIP_ID),
            ..Default::default()
        }),
    }
}

/// A scavenger corvette: shakedown's pirate silhouette, flown in a pair.
/// Leashed to the yacht fight so the duel stays in the derelict field.
fn corvette(id: &str, spawn_pos: Meters3) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: "Scavenger Corvette".to_string(),
            position: spawn_pos,
            rotation: facing_the_fight(),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: vec![spawn_pos, HAULER_POS + Meters3::new(0.0, 400.0, 600.0)],
                leash: Some(Meters(4_200.0)),
                // Arrival grace (beat-sheet pass): "drop off the rocks" is
                // readable before the tracers.
                engage_delay: Some(5.0),
                ..Default::default()
            }),
            allegiance: None,
            // A scavenger-grade corvette: weaker turrets, squishier hull.
            hull: ships::hull(ships::CARGOA_RAIDER_SHIP_ID),
            ..Default::default()
        }),
    }
}

/// The gang's gunship: the capital the slice exists for. Two PDC turrets,
/// two torpedo tubes, an armored spine of reinforced hulls. No leash - it
/// came here to end the fight, and it chases.
///
/// The LANCE cargo-B, and that is this chapter's difficulty setting. This is
/// the first torpedo the campaign throws at the player (shakedown and part one
/// field none), and the player meets it with two hand-aimed PDCs and no
/// autonomous point defense at all. Measured against one PERFECT defender
/// across the shipped 1.5 km point-defense envelope, a Serpent costs ~370 rounds
/// to stop and is only killed ~400 m out - barely outside its own 300 m blast
/// radius, with nothing left over for a human's aim - while a Lance costs ~120
/// and dies ~1150 m out. The gunship opens with twelve of them, so screening is
/// the fight - one whose answer exists. The Serpent is what `final_tally`
/// escalates to.
fn gunship() -> ScenarioObjectConfig {
    // The Rust Tally is the cargob: a 42-cube capital with two PDC turrets, two
    // torpedo tubes and a core controller. No leash - it came here to end the
    // fight, and it chases.
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_GUNSHIP.to_string(),
            name: "Gunship Rust Tally".to_string(),
            position: GUNSHIP_SPAWN,
            rotation: facing_the_fight(),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig::default()),
            allegiance: None,
            hull: ships::hull(ships::CARGOB_LANCE_SHIP_ID),
            ..Default::default()
        }),
    }
}

/// The destructible chaff field along the approach lane. A Box region (the
/// Ring variant is origin-centred; sample() REPLACES the template position,
/// it does not offset it) with margins that keep the player spawn (z=400)
/// and the yacht (z=-4500) themselves clear. Shared by both parts, same
/// seed, so the chapter's arena reads as one place.
fn cover_scatter(asteroid_texture: &AssetRef<Image>) -> EventActionConfig {
    EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "cover_rock_".to_string(),
        count: 24,
        seed: SCATTER_SEED,
        region: ScatterRegion::Box {
            min: Meters3::new(-2_000.0, -450.0, -4_300.0),
            max: Meters3::new(2_000.0, 450.0, -800.0),
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "cover_rock_".to_string(),
                name: "Derelict Field Rock".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                material: None,
                destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
                radius: Meters(10.0),
                texture: asteroid_texture.clone(),
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((Meters(15.0), Meters(40.0))),
        min_separation: None,
    })
}

/// The hard-cover boulders (both parts, same layout): INVULNERABLE, so
/// they survive better-turret fire and the AI line-of-fire gate treats
/// them as real occluders - the pressure-relief tier above the chaff.
/// Nominal radii are small on purpose: asteroid bodies run 3.5x-6x nominal
/// (ASTEROID_GEOMETRIC_FACTOR_MIN/MAX), so nominal 35-50 m is a 120-300 m
/// boulder. Three anchor the corvette fight north of the yacht
/// (z -5200..-5750), two sit on the gunship's approach lane
/// (z -7000..-7500); all are outside the scatter box (z >= -4300) and clear
/// of every spawn at the 6x worst case (pinned by broadside_assault.rs).
fn hard_cover(asteroid_texture: &AssetRef<Image>) -> Vec<ScenarioObjectConfig> {
    let boulder = |id: &str, position: Meters3, radius: Meters| ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: "Derelict Boulder".to_string(),
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
            lock_signature: None,
        }),
    };
    vec![
        boulder(
            "cover_boulder_1",
            Meters3::new(900.0, 200.0, -5_200.0),
            Meters(40.0),
        ),
        boulder(
            "cover_boulder_2",
            Meters3::new(-1_100.0, 0.0, -5_300.0),
            Meters(40.0),
        ),
        boulder(
            "cover_boulder_3",
            Meters3::new(200.0, -150.0, -5_750.0),
            Meters(50.0),
        ),
        boulder(
            "cover_boulder_4",
            Meters3::new(1_300.0, 400.0, -7_000.0),
            Meters(35.0),
        ),
        boulder(
            "cover_boulder_5",
            Meters3::new(-700.0, 300.0, -7_500.0),
            Meters(35.0),
        ),
    ]
}

/// Part one's outro: both ambush-cleared variants differ only in the yacht's
/// fate, which their own handlers said - the tease and the banner are one
/// shared chain. Only one variant can ever fire, so they all start this cursor.
fn ambush_outro() -> EventActionConfig {
    pacing::outro_sequence(
        VAR_ACT,
        ACT_WON,
        BELT_RELAY,
        "Deep scan is not going quiet: a capital burn, closing fast. The \
         Rust Tally is coming to finish what its pickers started.",
        "The ambush at the Ceres Queen is broken - and the gang's capital \
         is already on its way.",
        vec![],
        Some(BROADSIDE_GUNSHIP_SCENARIO_ID.to_string()),
    )
}

pub(crate) fn broadside(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let cover_scatter = cover_scatter(&asteroid_texture);
    let boulders = hard_cover(&asteroid_texture);

    // Act 0: the stage and the hook (the boulders splice in after the
    // chaff so the field reads chaff-then-anchors in the data too).
    let mut opening = vec![
        set_variable(VAR_ACT, number(0.0)),
        set_variable(VAR_CORVETTE_A_DOWN, number(0.0)),
        set_variable(VAR_CORVETTE_B_DOWN, number(0.0)),
        set_variable(VAR_HAULER_LOST, number(0.0)),
        spawn_object(player_ship()),
        spawn_object(yacht_ship()),
        cover_scatter,
    ];
    opening.extend(boulders.into_iter().map(spawn_object));
    opening.extend([
        EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
            id: ID_HAULER_AREA.to_string(),
            name: "Yacht Approach".to_string(),
            position: HAULER_POS,
            rotation: Quat::IDENTITY,
            radius: Meters(1_300.0),
        }),
        // The voice pass: the distress call the shakedown banner promised is
        // now HEARD - the announce beat's one comms line; the objective shrinks
        // to the goal. Pacing pass: the objective does not share this frame
        // with the distress call.
        story_message(
            CAPTAIN_HALLORAN,
            "Ceres Queen to any ship in the belt - drive's stripped, and \
             they're coming back for the hull.",
        ),
        // Reveal-then-navigate: the distress call sets up, "find the yacht" is
        // a soft instruction - a mid gap. The gold marker rides the yacht from
        // OnStart (NOT withheld inside the sequence), so the player has a nav
        // target during the call; only the objective TEXT waits, matching
        // shakedown/final_tally.
        pacing::beat_later(
            SEQ_CONTACT,
            MID_GAP,
            vec![post_objective(OBJ_CONTACT, "Find the yacht Ceres Queen.")],
        ),
        attach_objective_marker(ID_HAULER, "CERES QUEEN"),
    ]);
    opening.extend(ThreePointRig::around("broadside", Meters3::ZERO, 10.0).actions());

    let events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: opening,
        },
        // Act 0 -> 1: reaching the yacht springs the ambush. The threats spawn
        // and the warning lands now; the DEFEND objective posts a beat later so
        // "contact done" and "drive them off" never share a frame.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![
                entity_pair(ID_HAULER_AREA, ID_PLAYER),
                number_equals(VAR_ACT, 0.0),
            ],
            actions: vec![
                set_variable(VAR_ACT, number(1.0)),
                complete_objective(OBJ_CONTACT),
                // Threat reveal (the ambush springs): full absorb beat; the
                // corvette markers appear at the transition (below), so the
                // threats are visible while the objective text waits.
                pacing::beat_later(
                    SEQ_DEFEND,
                    REVEAL_GAP,
                    vec![post_objective(
                        OBJ_DEFEND,
                        "Drive the corvettes off the Ceres Queen.",
                    )],
                ),
                spawn_object(corvette(ID_CORVETTE_A, CORVETTE_A_SPAWN)),
                spawn_object(corvette(ID_CORVETTE_B, CORVETTE_B_SPAWN)),
                story_message(
                    CAPTAIN_HALLORAN,
                    "They're here - two of them, off the rocks. They were \
                     waiting for someone to answer.",
                ),
                detach_objective_marker(ID_HAULER),
                attach_objective_marker(ID_CORVETTE_A, "CORVETTE"),
                attach_objective_marker(ID_CORVETTE_B, "CORVETTE"),
                show_hint_emphasis("RADAR"),
            ],
        },
        // Corvette defeats raise their flags once for either terminal path.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDefeated,
            once: true,
            filters: vec![entity(ID_CORVETTE_A)],
            actions: vec![
                set_variable(VAR_CORVETTE_A_DOWN, number(1.0)),
                detach_objective_marker(ID_CORVETTE_A),
            ],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDefeated,
            once: true,
            filters: vec![entity(ID_CORVETTE_B)],
            actions: vec![
                set_variable(VAR_CORVETTE_B_DOWN, number(1.0)),
                detach_objective_marker(ID_CORVETTE_B),
            ],
        },
        // First-kill beat (voice pass): one line when the FIRST corvette
        // dies, whichever it is. Each handler gates on the OTHER flag still
        // being down so the pair is mutually exclusive - the second kill
        // goes straight to the checkpoint beat, no second line. Separate
        // from the flag handlers so the flag-set stays unconditional.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![
                entity(ID_CORVETTE_A),
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_CORVETTE_B_DOWN, 0.0),
            ],
            actions: vec![story_message(
                CAPTAIN_HALLORAN,
                "One picker's venting out. The other one is swinging onto you.",
            )],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![
                entity(ID_CORVETTE_B),
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_CORVETTE_A_DOWN, 0.0),
            ],
            actions: vec![story_message(
                CAPTAIN_HALLORAN,
                "One picker's venting out. The other one is swinging onto you.",
            )],
        },
        // Act 1 -> 2: both corvettes down - the chapter's CHECKPOINT. The
        // gunship fight is its own scenario now, so the Victory beat here
        // means a death against the capital retries the capital, never
        // this ambush (spike F7). OnUpdate gated on the act makes this a
        // one-shot regardless of which kill lands last; Continue rides the
        // lingering chain into part two.
        // Two variants of the same beat, gated on the yacht's fate
        // (mutually exclusive on VAR_HAULER_LOST), so protecting her is
        // acknowledged in the banner. The overlay's own message carries the
        // closing line per the beat-sheet convention.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_CORVETTE_A_DOWN, 1.0),
                number_equals(VAR_CORVETTE_B_DOWN, 1.0),
                number_equals(VAR_HAULER_LOST, 0.0),
            ],
            actions: pacing::open_outro(
                VAR_ACT,
                ACT_OUTRO,
                ambush_outro(),
                vec![
                    complete_objective(OBJ_DEFEND),
                    story_message(
                        BELT_RELAY,
                        "The pickers break off, hulls venting - and the Ceres \
                     Queen is still in one piece.",
                    ),
                ],
            ),
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_CORVETTE_A_DOWN, 1.0),
                number_equals(VAR_CORVETTE_B_DOWN, 1.0),
                number_equals(VAR_HAULER_LOST, 1.0),
            ],
            actions: pacing::open_outro(
                VAR_ACT,
                ACT_OUTRO,
                ambush_outro(),
                vec![
                    complete_objective(OBJ_DEFEND),
                    story_message(
                        BELT_RELAY,
                        "The pickers break off, hulls venting - too late for the \
                     Ceres Queen.",
                    ),
                ],
            ),
        },
        // Flavor, not failure: the yacht dies to stray fire and the story
        // notices - but only while the fight is on; after the win nothing
        // pushes fresh objectives under the Victory overlay.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_HAULER), number_less_than(VAR_ACT, 2.0)],
            actions: vec![
                set_variable(VAR_HAULER_LOST, number(1.0)),
                detach_objective_marker(ID_HAULER),
                story_message(
                    BELT_RELAY,
                    "The Ceres Queen's beacon just went dark. Make it cost them.",
                ),
            ],
        },
        // Lose: the Defeat overlay offers Retry (lingering restart) and Main
        // Menu. Gated to the live acts: a death AFTER the win (a drifting rock
        // under the gold banner) must not overwrite the earned Victory with
        // Defeat.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_PLAYER), number_less_than(VAR_ACT, 2.0)],
            actions: vec![
                // Terminal act FIRST: CurrentOutcome is last-write-wins, so a
                // mutual-destruction trade - the player's blast killing the
                // last corvette on the same beat the player dies - could let
                // the checkpoint win (gated act == 1) overwrite this Defeat
                // over the queued retry. Act 3 closes every win gate.
                set_variable(VAR_ACT, number(3.0)),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "The scavengers strip your wreck for parts.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: BROADSIDE_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnNeutralized,
            once: true,
            filters: vec![entity(ID_PLAYER), number_less_than(VAR_ACT, 2.0)],
            actions: vec![
                set_variable(VAR_ACT, number(3.0)),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Nothing left to fight with - you drift, and the scavengers close in.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: BROADSIDE_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
    ];

    ScenarioConfig {
        id: BROADSIDE_SCENARIO_ID.to_string(),
        name: "Broadside".to_string(),
        description: "The scavengers come back in force: answer a stranded \
                      yacht's distress call and break the ambush at the Ceres Queen. \
                      Chapter two of the base storyline, part one."
            .to_string(),
        cubemap,
        // Generated placeholder art (scripts/gen-scenario-thumbnails.py);
        // real art overwrites this same path with no code change.
        thumbnail: Some(AssetRef::from("self://thumbnails/broadside.png")),
        hidden: false,
        menu_backdrop: false,
        skybox_brightness: DEFAULT_SKYBOX_BRIGHTNESS,
        watches: vec![scenario_elapsed_watch(SCENARIO_ELAPSED_VAR)],
        // Chapter two of the Nova Protocol campaign. Membership + order now
        // live in the `nova_protocol` campaign mapping, which also lists the
        // hidden part-two wave (`broadside_gunship`) so it is replayable from
        // the campaign header.
        events,
    }
}

/// Part two: the capital fight, entered only through part one's checkpoint
/// (hidden from the Scenarios picker). The gunship spawns at OnStart - its
/// ~7.2 km burn toward the yacht IS the act's pacing, torpedo tubes open
/// through the whole approach - and dying here retries HERE.
/// Part two's outro: all four win variants (destroyed / neutralized x the
/// yacht's fate) said their own line already - one shared chain carries the
/// tease into chapter three and the banner.
fn gunship_outro() -> EventActionConfig {
    pacing::outro_sequence(
        VAR_ACT,
        ACT_WON,
        BELT_RELAY,
        "The deep scan still is not quiet: the gang's traffic keeps moving, \
         and all of it is inbound to the freight lane.",
        "The Rust Tally is finished. The gang still holds the lane - and the \
         belt's convoys are the next thing it reaches for.",
        vec![],
        Some(super::lifeline::LIFELINE_SCENARIO_ID.to_string()),
    )
}

pub(crate) fn broadside_gunship(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    // Same arena as part one: yacht, chaff scatter (same seed), hard
    // boulders - the chapter reads as one place across the split.
    let mut opening = vec![
        set_variable(VAR_ACT, number(1.0)),
        set_variable(VAR_HAULER_LOST, number(0.0)),
        spawn_object(player_ship()),
        spawn_object(yacht_ship()),
        cover_scatter(&asteroid_texture),
    ];
    opening.extend(hard_cover(&asteroid_texture).into_iter().map(spawn_object));
    opening.extend([
        spawn_object(gunship()),
        // The capital gets a voice: the announce beat's one comms line, while
        // the objectives shrink to goals. Pacing pass: the objectives post a
        // beat after the taunt, not the same frame.
        story_message(
            RUST_TALLY,
            "You cost me two pickers, belt rat. The Rust Tally pays its \
             debts in torpedoes.",
        ),
        // Threat reveal (the gunship taunts and burns in): full absorb beat.
        // The gunship and its RADAR marker are already up (this is OnStart), so
        // only the objective text waits.
        pacing::beat_later(
            SEQ_GUN_OBJECTIVE,
            REVEAL_GAP,
            vec![
                post_objective(
                    OBJ_SCREEN,
                    "Lock the incoming torpedoes and screen them with your PDC.",
                ),
                post_objective(OBJ_BREAK, "Break the Rust Tally, section by section."),
            ],
        ),
        attach_objective_marker(ID_GUNSHIP, "GUNSHIP"),
        show_hint_emphasis("RADAR"),
    ]);
    opening.extend(ThreePointRig::around("gunship", Meters3::ZERO, 10.0).actions());

    let events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: opening,
        },
        // Win: the gunship comes apart - and the deep scan keeps the door open:
        // the lingering chain rides into chapter three (Lifeline). Two variants
        // on the yacht's fate (mutually exclusive on VAR_HAULER_LOST) - each
        // part tracks its OWN yacht, since variables are scenario-scoped and
        // the arena restages across the checkpoint.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![
                entity(ID_GUNSHIP),
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_HAULER_LOST, 0.0),
            ],
            actions: pacing::open_outro(
                VAR_ACT,
                ACT_OUTRO,
                gunship_outro(),
                vec![
                    complete_objective(OBJ_SCREEN),
                    complete_objective(OBJ_BREAK),
                    detach_objective_marker(ID_GUNSHIP),
                    story_message(
                        BELT_RELAY,
                        "The Rust Tally breaks apart - and the Ceres Queen is \
                     still whole to see it.",
                    ),
                ],
            ),
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![
                entity(ID_GUNSHIP),
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_HAULER_LOST, 1.0),
            ],
            actions: pacing::open_outro(
                VAR_ACT,
                ACT_OUTRO,
                gunship_outro(),
                vec![
                    complete_objective(OBJ_SCREEN),
                    complete_objective(OBJ_BREAK),
                    detach_objective_marker(ID_GUNSHIP),
                    story_message(
                        BELT_RELAY,
                        "The Rust Tally breaks apart - too late for the Ceres \
                     Queen.",
                    ),
                ],
            ),
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnNeutralized,
            once: true,
            filters: vec![
                entity(ID_GUNSHIP),
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_HAULER_LOST, 0.0),
            ],
            actions: pacing::open_outro(
                VAR_ACT,
                ACT_OUTRO,
                gunship_outro(),
                vec![
                    complete_objective(OBJ_SCREEN),
                    complete_objective(OBJ_BREAK),
                    detach_objective_marker(ID_GUNSHIP),
                    story_message(
                        BELT_RELAY,
                        "The Rust Tally hangs dead in the void, guns cold and \
                     engines dark - and the Ceres Queen is still whole to \
                     see it.",
                    ),
                ],
            ),
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnNeutralized,
            once: true,
            filters: vec![
                entity(ID_GUNSHIP),
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_HAULER_LOST, 1.0),
            ],
            actions: pacing::open_outro(
                VAR_ACT,
                ACT_OUTRO,
                gunship_outro(),
                vec![
                    complete_objective(OBJ_SCREEN),
                    complete_objective(OBJ_BREAK),
                    detach_objective_marker(ID_GUNSHIP),
                    story_message(
                        BELT_RELAY,
                        "The Rust Tally hangs dead in the void, guns cold and \
                     engines dark - too late for the Ceres Queen.",
                    ),
                ],
            ),
        },
        // Flavor, not failure: same soft-fail beat as part one.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_HAULER), number_less_than(VAR_ACT, 2.0)],
            actions: vec![
                set_variable(VAR_HAULER_LOST, number(1.0)),
                detach_objective_marker(ID_HAULER),
                story_message(
                    BELT_RELAY,
                    "The Ceres Queen's beacon just went dark. Make it cost them.",
                ),
            ],
        },
        // Lose: retry THIS part - the checkpoint's whole point (spike F7).
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_PLAYER), number_less_than(VAR_ACT, 2.0)],
            actions: vec![
                // Terminal act FIRST: last-write-wins CurrentOutcome means a
                // trade - the player's blast breaking the gunship on the same
                // beat the player dies - could let the win (gated act == 1)
                // overwrite this Defeat over the queued retry. Act 3 closes
                // every win gate.
                set_variable(VAR_ACT, number(3.0)),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "The Rust Tally walks its torpedoes onto your wreck.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: BROADSIDE_GUNSHIP_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnNeutralized,
            once: true,
            filters: vec![entity(ID_PLAYER), number_less_than(VAR_ACT, 2.0)],
            actions: vec![
                set_variable(VAR_ACT, number(3.0)),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Nothing left to fight with - the Rust Tally finishes you at leisure.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: BROADSIDE_GUNSHIP_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
    ];

    ScenarioConfig {
        id: BROADSIDE_GUNSHIP_SCENARIO_ID.to_string(),
        name: "Broadside: Rust Tally".to_string(),
        description: "The gang's gunship burns in to finish the job: screen \
                      its torpedoes with your PDC and break it section by \
                      section. Chapter two of the base storyline, part two."
            .to_string(),
        cubemap,
        // Generated placeholder art (scripts/gen-scenario-thumbnails.py);
        // real art overwrites this same path with no code change.
        thumbnail: Some(AssetRef::from("self://thumbnails/broadside_gunship.png")),
        // Hidden from the flat picker, but a member of the `nova_protocol`
        // campaign mapping so it is replayable from the campaign header.
        hidden: true,
        menu_backdrop: false,
        skybox_brightness: DEFAULT_SKYBOX_BRIGHTNESS,
        watches: vec![scenario_elapsed_watch(SCENARIO_ELAPSED_VAR)],
        events,
    }
}
