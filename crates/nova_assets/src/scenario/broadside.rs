//! Broadside - the capital-combat vertical slice (task 20260708-203659),
//! act-split for checkpointed retries by the difficulty rework
//! (task 20260717-112639, spike tasks/20260717-111808/SPIKE.md F4/F7).
//!
//! Chapter two of the base storyline: the scavenger driven off in Shakedown
//! Run was a scout. Its gang comes back in force to strip the belt - and a
//! neutral hauler is caught in the middle. The chapter now plays as TWO
//! scenarios so a death retries the current act, never the whole chapter:
//!
//! - `broadside` (part one): act 0 (contact) - answer the hauler's distress
//!   call across the cover field; act 1 (escalation) - two scavenger
//!   corvettes jump the player at the hauler. Breaking the pair is the
//!   chapter's CHECKPOINT: a Victory beat chains (lingering) into part two.
//! - `broadside_gunship` (part two, hidden): the gang's GUNSHIP burns in
//!   from the dark - a capital with turrets and torpedo tubes. Screen its
//!   torpedoes with the PDC, then break it section by section. Dying here
//!   retries HERE.
//!
//! Win: gunship destroyed -> Victory overlay whose lingering chain enters
//! chapter three (`lifeline`, task 20260721-160957). Lose: player destroyed
//! -> Defeat + lingering retry of the current part. The hauler is a NEUTRAL ship (the
//! `SpaceshipConfig.allegiance` override): nobody targets it, but stray
//! blast damage can kill it - a flavor beat reacts, the mission continues.
//!
//! Cover comes in two tiers since the AI line-of-fire gate (2d006707):
//! five fixed INVULNERABLE boulders anchor the hauler fight and the gunship
//! lane (real pressure relief - the AI holds fire when one blocks the
//! shot), while the seeded 24-rock scatter stays destructible chaff.
//!
//! Distances are authored against the measured AI constants
//! (crates/nova_gameplay/src/input/ai.rs): engage range 800u, torpedo
//! envelope [3 x blast_radius, 1000u] with a 10s per-bay cadence and the
//! first launch immediate, standoff orbit ~250u. The gunship spawns ~720u
//! from the hauler fight so it engages on arrival and its tubes are open
//! through the whole approach.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::{
    cast::{BELT_RELAY, CAPTAIN_HALLORAN, RUST_TALLY},
    craft::{self, ShipGrade},
    pacing::{self, mark_clock, open_gate, BEAT_GAP},
    shakedown::{
        complete, destroyed, emphasize, eq_num, lt_num, mark, num, objective, player_enters, set,
        spawn, story, unmark,
    },
    SCATTER_SEED,
};

/// Ships spawn with -Z forward; the fight sits +Z of both the corvette and
/// gunship spawns, so every combatant is authored with this about-face -
/// the gunship's torpedo alignment gate (cos > 0.5 on the hull bearing)
/// opens on arrival instead of after a 180-degree slew (review R1.4).
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
/// (broadside_gunship): 1 the capital fight, 2 won. Every gate filter
/// checks it, so beats fire once and in order within each part.
const VAR_ACT: &str = "act";
/// Per-corvette kill flags: two independent OnDestroyed handlers set them,
/// and the act-2 escalation gates on BOTH - no arithmetic counter, so a
/// double-fire cannot skip the gate (count-gate-use-gt-not-eq by
/// construction).
const VAR_CORVETTE_A_DOWN: &str = "corvette_a_down";
const VAR_CORVETTE_B_DOWN: &str = "corvette_b_down";
/// Whether the Ceres Queen died to stray fire this part (0/1). Seeded 0 on
/// start; the soft-fail beat raises it, and the Victory beat reads it to
/// pick its banner variant - protecting her finally gets acknowledged
/// (voice pass, task 20260721-160929). Scenario-scoped like every variable:
/// each part tracks its OWN hauler (state does not cross the checkpoint).
const VAR_HAULER_LOST: &str = "hauler_lost";

/// Pacing (task 20260722-092421): objectives post a beat AFTER the comms line
/// that introduces them, never the same frame. Each gate variable holds a
/// `mark_clock` deadline; the paired `_posted` flag latches the one-shot
/// `gated_once` that posts the objective once the clock passes it. Part one:
/// the contact objective (after the distress call) and the defend objective
/// (after the ambush line). Part two reuses its own pair in a separate scope.
const VAR_CONTACT_GATE: &str = "contact_gate";
const VAR_CONTACT_POSTED: &str = "contact_posted";
const VAR_DEFEND_GATE: &str = "defend_gate";
const VAR_DEFEND_POSTED: &str = "defend_posted";
const VAR_GUN_OBJ_GATE: &str = "gun_obj_gate";
const VAR_GUN_OBJ_POSTED: &str = "gun_obj_posted";

/// The hauler drifts here; the fight happens around it.
const HAULER_POS: Vec3 = Vec3::new(0.0, 10.0, -450.0);
/// Player spawn, looking down the lane toward the hauler.
const PLAYER_SPAWN: Vec3 = Vec3::new(0.0, 0.0, 40.0);
/// Corvettes jump the player from the hauler's flanks.
const CORVETTE_A_SPAWN: Vec3 = Vec3::new(140.0, 30.0, -560.0);
const CORVETTE_B_SPAWN: Vec3 = Vec3::new(-150.0, -20.0, -540.0);
/// The gunship burns in from deep field: ~720u past the hauler, inside its
/// own engage range (800u) of the fight the moment it spawns, torpedo
/// envelope (<= 1000u) open through the whole approach.
const GUNSHIP_SPAWN: Vec3 = Vec3::new(80.0, 60.0, -1170.0);

/// The player's chapter-two ship: shakedown's trainer plus a second hull
/// and the better turret. NO torpedo bay - torpedoes are the ENEMY's
/// weapon this chapter (story: not unlocked yet), which keeps the
/// PDC-screening fantasy pure: you shoot torpedoes down, you don't trade
/// them. Finite ammo: catalog weapons auto-reload (task 20260717-085640),
/// so a dry magazine is a pacing beat, not a fail state.
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
                // Both racer turret cubes fire on LMB / right trigger.
                input_mapping: craft::RACER_TURRET_IDS
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
                // Finite ammo: catalog weapons auto-reload (task 20260717-085640),
                // so the PDC screen-and-brawl plays with real magazines and the
                // diegetic ammo gauge instead of unlimited fire.
                infinite_ammo: false,
                lock_refire_secs: None,
            }),
            allegiance: None,
            // The racer. RCS is off in the mainline campaign until the rework
            // (task 20260718-175502); no other verb is gated this chapter.
            sections: craft::racer_sections(
                ShipGrade::Player,
                vec![SectionModification::DisableVerb(FlightVerb::Rcs)],
            ),
        }),
    }
}

/// The neutral hauler: drive stripped, adrift by the derelict field. No
/// controller (it cannot fly), NEUTRAL allegiance (nobody's AI targets it),
/// but real sections with real health - stray blast damage can kill it and
/// the story notices.
fn hauler_ship() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_HAULER.to_string(),
            name: "Hauler Ceres Queen".to_string(),
            position: HAULER_POS,
            rotation: Quat::from_rotation_y(0.6),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Neutral),
            // The Ceres Queen is the cargoa hauler - a wide, unarmed cargo hull
            // that reads as a civilian freighter caught in the crossfire.
            sections: craft::cargoa_sections(),
        }),
    }
}

/// A scavenger corvette: shakedown's pirate silhouette, flown in a pair.
/// Leashed to the hauler fight so the duel stays in the derelict field.
fn corvette(id: &str, spawn_pos: Vec3) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: "Scavenger Corvette".to_string(),
            position: spawn_pos,
            rotation: facing_the_fight(),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: vec![spawn_pos, HAULER_POS + Vec3::new(0.0, 40.0, 60.0)],
                leash: Some(420.0),
                // Arrival grace (beat-sheet pass, task 20260717-163058):
                // "drop off the rocks" is readable before the tracers.
                engage_delay: Some(5.0),
                ..Default::default()
            }),
            allegiance: None,
            // A scavenger-grade racer: weaker turrets, squishier hull.
            sections: craft::racer_sections(ShipGrade::Enemy, vec![]),
        }),
    }
}

/// The gang's gunship: the capital the slice exists for. Two PDC turrets,
/// two torpedo tubes, an armored spine of reinforced hulls. No leash - it
/// came here to end the fight, and it chases.
fn gunship() -> ScenarioObjectConfig {
    // The Rust Tally is the cargob (moved into base from the craft_cargoB
    // example mod): a 42-cube capital with two PDC turrets, two torpedo tubes
    // and a core controller. No leash - it came here to end the fight, and it
    // chases.
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
            sections: craft::cargob_sections(),
        }),
    }
}

/// The destructible chaff field along the approach lane. A Box region (the
/// Ring variant is origin-centred; sample() REPLACES the template position,
/// it does not offset it) with margins that keep the player spawn (z=40)
/// and the hauler (z=-450) themselves clear. Shared by both parts, same
/// seed, so the chapter's arena reads as one place.
fn cover_scatter(asteroid_texture: &AssetRef<Image>) -> EventActionConfig {
    EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "cover_rock_".to_string(),
        count: 24,
        seed: SCATTER_SEED,
        region: ScatterRegion::Box {
            min: Vec3::new(-200.0, -45.0, -430.0),
            max: Vec3::new(200.0, 45.0, -80.0),
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "cover_rock_".to_string(),
                name: "Derelict Field Rock".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
                radius: 1.0,
                texture: asteroid_texture.clone(),
                health: 100.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.5, 4.0)),
    })
}

/// The hard-cover boulders (both parts, same layout): INVULNERABLE, so
/// they survive better-turret fire and the AI line-of-fire gate treats
/// them as real occluders - the pressure-relief tier above the chaff.
/// Nominal radii are small on purpose: asteroid bodies run 3.5x-6x nominal
/// (ASTEROID_GEOMETRIC_FACTOR_MIN/MAX), so nominal 3.5-5 is a 12-30u
/// boulder. Three anchor the corvette fight north of the hauler
/// (z -520..-575), two sit on the gunship's approach lane (z -700..-750);
/// all are outside the scatter box (z >= -430) and clear of every spawn at
/// the 6x worst case (pinned by broadside_assault.rs).
fn hard_cover(asteroid_texture: &AssetRef<Image>) -> Vec<ScenarioObjectConfig> {
    let boulder = |id: &str, position: Vec3, radius: f32| ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: "Derelict Boulder".to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
            destroy_sound: None,
            radius,
            texture: asteroid_texture.clone(),
            health: 1000.0,
            surface_gravity: None,
            invulnerable: true,
            lock_signature: None,
        }),
    };
    vec![
        boulder("cover_boulder_1", Vec3::new(90.0, 20.0, -520.0), 4.0),
        boulder("cover_boulder_2", Vec3::new(-110.0, 0.0, -530.0), 4.0),
        boulder("cover_boulder_3", Vec3::new(20.0, -15.0, -575.0), 5.0),
        boulder("cover_boulder_4", Vec3::new(130.0, 40.0, -700.0), 3.5),
        boulder("cover_boulder_5", Vec3::new(-70.0, 30.0, -750.0), 3.5),
    ]
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
        set(VAR_ACT, num(0.0)),
        set(VAR_CORVETTE_A_DOWN, num(0.0)),
        set(VAR_CORVETTE_B_DOWN, num(0.0)),
        set(VAR_HAULER_LOST, num(0.0)),
        set(VAR_CONTACT_POSTED, num(0.0)),
        set(VAR_DEFEND_POSTED, num(0.0)),
        // Seed the defend gate so its gated_once filter reads a defined 0 before
        // the ambush stamps it, not an undefined var (bug 20260722-114541).
        set(VAR_DEFEND_GATE, num(0.0)),
        spawn(player_ship()),
        spawn(hauler_ship()),
        cover_scatter,
    ];
    opening.extend(boulders.into_iter().map(spawn));
    opening.extend([
        EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
            id: ID_HAULER_AREA.to_string(),
            name: "Hauler Approach".to_string(),
            position: HAULER_POS,
            rotation: Quat::IDENTITY,
            radius: 130.0,
        }),
        // The voice pass (task 20260721-160929): the distress call the
        // shakedown banner promised is now HEARD - the announce beat's one
        // comms line; the objective shrinks to the goal.
        // Pacing pass (task 20260722-092421): the objective no longer shares
        // this frame with the distress call - the deadline is stamped here and
        // OBJ_CONTACT posts a beat later (the gated_once handler below).
        story(
            CAPTAIN_HALLORAN,
            "Ceres Queen to any ship in the belt - drive's stripped, and \
             they're coming back for the hull.",
        ),
        open_gate(VAR_CONTACT_GATE, BEAT_GAP),
    ]);

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: opening,
        },
        // The contact objective posts a beat after the distress call (pacing
        // pass): still act 0, so a player who somehow reaches the hauler inside
        // the beat springs the ambush without a stale objective appearing after.
        pacing::gated_once(
            VAR_CONTACT_POSTED,
            VAR_CONTACT_GATE,
            vec![eq_num(VAR_ACT, 0.0)],
            vec![
                objective(OBJ_CONTACT, "Find the hauler Ceres Queen."),
                mark(ID_HAULER, "CERES QUEEN"),
            ],
        ),
        // Act 0 -> 1: reaching the hauler springs the ambush. The threats spawn
        // and the warning lands now; the DEFEND objective posts a beat later
        // (gated_once below) so "contact done" and "drive them off" never share
        // a frame.
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![player_enters(ID_HAULER_AREA), eq_num(VAR_ACT, 0.0)],
            actions: vec![
                set(VAR_ACT, num(1.0)),
                complete(OBJ_CONTACT),
                mark_clock(VAR_DEFEND_GATE, BEAT_GAP),
                spawn(corvette(ID_CORVETTE_A, CORVETTE_A_SPAWN)),
                spawn(corvette(ID_CORVETTE_B, CORVETTE_B_SPAWN)),
                story(
                    CAPTAIN_HALLORAN,
                    "They're here - two of them, off the rocks. They were \
                     waiting for someone to answer.",
                ),
                unmark(ID_HAULER),
                mark(ID_CORVETTE_A, "CORVETTE"),
                mark(ID_CORVETTE_B, "CORVETTE"),
                emphasize("RADAR"),
            ],
        },
        pacing::gated_once(
            VAR_DEFEND_POSTED,
            VAR_DEFEND_GATE,
            vec![eq_num(VAR_ACT, 1.0)],
            vec![objective(
                OBJ_DEFEND,
                "Drive the corvettes off the Ceres Queen.",
            )],
        ),
        // Corvette kills raise their flags (separate handlers, no counter
        // arithmetic - a double OnDestroyed cannot overshoot a flag).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_CORVETTE_A)],
            actions: vec![set(VAR_CORVETTE_A_DOWN, num(1.0)), unmark(ID_CORVETTE_A)],
        },
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_CORVETTE_B)],
            actions: vec![set(VAR_CORVETTE_B_DOWN, num(1.0)), unmark(ID_CORVETTE_B)],
        },
        // First-kill beat (voice pass): one line when the FIRST corvette
        // dies, whichever it is. Each handler gates on the OTHER flag still
        // being down so the pair is mutually exclusive - the second kill
        // goes straight to the checkpoint beat, no second line. Separate
        // from the flag handlers so the flag-set stays unconditional.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![
                destroyed(ID_CORVETTE_A),
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_CORVETTE_B_DOWN, 0.0),
            ],
            actions: vec![story(
                CAPTAIN_HALLORAN,
                "One picker's venting out. The other one is swinging onto you.",
            )],
        },
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![
                destroyed(ID_CORVETTE_B),
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_CORVETTE_A_DOWN, 0.0),
            ],
            actions: vec![story(
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
        // Two variants of the same beat, gated on the hauler's fate
        // (mutually exclusive on VAR_HAULER_LOST), so protecting her is
        // acknowledged in the banner. The overlay's own message carries the
        // closing line per the beat-sheet convention.
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_CORVETTE_A_DOWN, 1.0),
                eq_num(VAR_CORVETTE_B_DOWN, 1.0),
                eq_num(VAR_HAULER_LOST, 0.0),
            ],
            actions: vec![
                set(VAR_ACT, num(2.0)),
                complete(OBJ_DEFEND),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Victory,
                    "The pickers break off, hulls venting - and the Ceres \
                     Queen is still in one piece. On the deep scan: a capital \
                     burn, closing fast. The Rust Tally is coming to finish \
                     what its pickers started.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: BROADSIDE_GUNSHIP_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_CORVETTE_A_DOWN, 1.0),
                eq_num(VAR_CORVETTE_B_DOWN, 1.0),
                eq_num(VAR_HAULER_LOST, 1.0),
            ],
            actions: vec![
                set(VAR_ACT, num(2.0)),
                complete(OBJ_DEFEND),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Victory,
                    "The pickers break off, hulls venting - too late for the \
                     Ceres Queen. On the deep scan: a capital burn, closing \
                     fast. The Rust Tally is coming to finish what its \
                     pickers started.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: BROADSIDE_GUNSHIP_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
        // Flavor, not failure: the hauler dies to stray fire and the story
        // notices - but only while the fight is on; after the win nothing
        // pushes fresh objectives under the Victory overlay (review R1.5).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_HAULER), lt_num(VAR_ACT, 2.0)],
            actions: vec![
                set(VAR_HAULER_LOST, num(1.0)),
                unmark(ID_HAULER),
                story(
                    BELT_RELAY,
                    "The Ceres Queen's beacon just went dark. Make it cost them.",
                ),
            ],
        },
        // Lose: the Defeat overlay offers Retry (lingering restart) and
        // Main Menu. Gated to the live acts: a death AFTER the win (a
        // drifting rock under the gold banner) must not overwrite the
        // earned Victory with Defeat (review R1.3).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_PLAYER), lt_num(VAR_ACT, 2.0)],
            actions: vec![
                // Terminal act FIRST (review R1.1 class, task 20260721-182034):
                // CurrentOutcome is last-write-wins, so a mutual-destruction
                // trade - the player's blast killing the last corvette on the
                // same beat the player dies - could let the checkpoint win
                // (gated act == 1) overwrite this Defeat over the queued retry.
                // Act 3 closes every win gate.
                set(VAR_ACT, num(3.0)),
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
    ];

    ScenarioConfig {
        id: BROADSIDE_SCENARIO_ID.to_string(),
        name: "Broadside".to_string(),
        description: "The scavengers come back in force: answer a hauler's \
                      distress call and break the ambush at the Ceres Queen. \
                      Chapter two of the base storyline, part one."
            .to_string(),
        cubemap,
        // Placeholder thumbnail (real per-scenario art: task 20260715-220011).
        thumbnail: Some(AssetRef::from("self://banner.png")),
        hidden: false,
        menu_backdrop: false,
        events,
    }
}

/// Part two: the capital fight, entered only through part one's checkpoint
/// (hidden from the Scenarios picker). The gunship spawns at OnStart - its
/// ~720u burn toward the hauler IS the act's pacing, torpedo tubes open
/// through the whole approach - and dying here retries HERE.
pub(crate) fn broadside_gunship(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    // Same arena as part one: hauler, chaff scatter (same seed), hard
    // boulders - the chapter reads as one place across the split.
    let mut opening = vec![
        set(VAR_ACT, num(1.0)),
        set(VAR_HAULER_LOST, num(0.0)),
        set(VAR_GUN_OBJ_POSTED, num(0.0)),
        spawn(player_ship()),
        spawn(hauler_ship()),
        cover_scatter(&asteroid_texture),
    ];
    opening.extend(hard_cover(&asteroid_texture).into_iter().map(spawn));
    opening.extend([
        spawn(gunship()),
        // The capital gets a voice (task 20260721-160929): the announce
        // beat's one comms line, while the objectives shrink to goals. Pacing
        // pass (task 20260722-092421): the objectives post a beat after the
        // taunt (the gated_once handler below), not the same frame.
        story(
            RUST_TALLY,
            "You cost me two pickers, belt rat. The Rust Tally pays its \
             debts in torpedoes.",
        ),
        open_gate(VAR_GUN_OBJ_GATE, BEAT_GAP),
        mark(ID_GUNSHIP, "GUNSHIP"),
        emphasize("RADAR"),
    ]);

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: opening,
        },
        pacing::gated_once(
            VAR_GUN_OBJ_POSTED,
            VAR_GUN_OBJ_GATE,
            vec![eq_num(VAR_ACT, 1.0)],
            vec![
                objective(
                    OBJ_SCREEN,
                    "Lock the incoming torpedoes and screen them with your PDC.",
                ),
                objective(OBJ_BREAK, "Break the Rust Tally, section by section."),
            ],
        ),
        // Win: the gunship comes apart - and the deep scan keeps the door
        // open: the lingering chain rides into chapter three (Lifeline,
        // task 20260721-160957).
        // Two variants on the hauler's fate (mutually exclusive on
        // VAR_HAULER_LOST) - each part tracks its OWN hauler, since
        // variables are scenario-scoped and the arena restages across the
        // checkpoint.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![
                destroyed(ID_GUNSHIP),
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_HAULER_LOST, 0.0),
            ],
            actions: vec![
                set(VAR_ACT, num(2.0)),
                complete(OBJ_SCREEN),
                complete(OBJ_BREAK),
                unmark(ID_GUNSHIP),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Victory,
                    "The Rust Tally breaks apart - and the Ceres Queen is \
                     still whole to see it. But the deep scan does not go \
                     quiet: the gang's traffic keeps moving, and all of it \
                     is inbound to the freight lane.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: super::lifeline::LIFELINE_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![
                destroyed(ID_GUNSHIP),
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_HAULER_LOST, 1.0),
            ],
            actions: vec![
                set(VAR_ACT, num(2.0)),
                complete(OBJ_SCREEN),
                complete(OBJ_BREAK),
                unmark(ID_GUNSHIP),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Victory,
                    "The Rust Tally breaks apart - too late for the Ceres \
                     Queen. And the deep scan does not go quiet: the gang's \
                     traffic keeps moving, and all of it is inbound to the \
                     freight lane.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: super::lifeline::LIFELINE_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
        // Flavor, not failure: same soft-fail beat as part one.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_HAULER), lt_num(VAR_ACT, 2.0)],
            actions: vec![
                set(VAR_HAULER_LOST, num(1.0)),
                unmark(ID_HAULER),
                story(
                    BELT_RELAY,
                    "The Ceres Queen's beacon just went dark. Make it cost them.",
                ),
            ],
        },
        // Lose: retry THIS part - the checkpoint's whole point (spike F7).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_PLAYER), lt_num(VAR_ACT, 2.0)],
            actions: vec![
                // Terminal act FIRST (review R1.1 class, task 20260721-182034):
                // last-write-wins CurrentOutcome means a trade - the player's
                // blast breaking the gunship on the same beat the player dies -
                // could let the win (gated act == 1) overwrite this Defeat over
                // the queued retry. Act 3 closes every win gate.
                set(VAR_ACT, num(3.0)),
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
    ];

    ScenarioConfig {
        id: BROADSIDE_GUNSHIP_SCENARIO_ID.to_string(),
        name: "Broadside: Rust Tally".to_string(),
        description: "The gang's gunship burns in to finish the job: screen \
                      its torpedoes with your PDC and break it section by \
                      section. Chapter two of the base storyline, part two."
            .to_string(),
        cubemap,
        // Placeholder thumbnail (real per-scenario art: task 20260715-220011).
        thumbnail: Some(AssetRef::from("self://banner.png")),
        hidden: true,
        menu_backdrop: false,
        events,
    }
}
