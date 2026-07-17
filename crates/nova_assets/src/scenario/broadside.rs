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
//! Win: gunship destroyed -> Victory overlay (end of the base story so far;
//! Enter/Main Menu). Lose: player destroyed -> Defeat + lingering retry of
//! the current part. The hauler is a NEUTRAL ship (the
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

use bevy::{platform::collections::HashMap, prelude::*};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::{
    shakedown::{
        complete, destroyed, emphasize, eq_num, lt_num, mark, num, objective, player_enters,
        section, set, spawn, unmark,
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
const OBJ_HAULER_LOST: &str = "hauler_lost";

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
    let turret = SpaceshipSectionConfig {
        id: "turret".to_string(),
        position: Vec3::new(0.0, 0.0, -2.0),
        rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        source: SectionSource::Prototype("better_turret_section".to_string()),
        modifications: vec![],
    };
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PLAYER.to_string(),
            name: "Player Spaceship".to_string(),
            position: PLAYER_SPAWN,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: HashMap::from([(
                    "turret".to_string(),
                    vec![
                        MouseButton::Left.into(),
                        GamepadButton::RightTrigger2.into(),
                    ],
                )]),
                // Post-tutorial: unbounded burn.
                speed_cap: None,
                // Finite ammo: catalog weapons auto-reload (task 20260717-085640),
                // so the PDC screen-and-brawl plays with real magazines and the
                // diegetic ammo gauge instead of unlimited fire.
                infinite_ammo: false,
                lock_refire_secs: None,
            }),
            allegiance: None,
            sections: vec![
                section("controller", "basic_controller_section", Vec3::ZERO),
                section("hull_front", "reinforced_hull_section", Vec3::Z),
                section("hull_back", "reinforced_hull_section", Vec3::NEG_Z),
                section("thruster", "basic_thruster_section", Vec3::Z * 2.0),
                turret,
            ],
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
            sections: vec![
                section("hull_bow", "light_hull_section", Vec3::Z),
                section("hull_mid", "light_hull_section", Vec3::ZERO),
                section("hull_stern", "light_hull_section", Vec3::NEG_Z),
            ],
        }),
    }
}

/// A scavenger corvette: shakedown's pirate silhouette, flown in a pair.
/// Leashed to the hauler fight so the duel stays in the derelict field.
fn corvette(id: &str, spawn_pos: Vec3) -> ScenarioObjectConfig {
    let turret = SpaceshipSectionConfig {
        id: "turret".to_string(),
        position: Vec3::new(0.0, 0.0, -1.0),
        rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        source: SectionSource::Prototype("light_turret_section".to_string()),
        modifications: vec![],
    };
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
            sections: vec![
                section("controller", "basic_controller_section", Vec3::ZERO),
                section("hull_front", "light_hull_section", Vec3::Z),
                section("thruster", "basic_thruster_section", Vec3::Z * 2.0),
                turret,
            ],
        }),
    }
}

/// The gang's gunship: the capital the slice exists for. Two PDC turrets,
/// two torpedo tubes, an armored spine of reinforced hulls. No leash - it
/// came here to end the fight, and it chases.
fn gunship() -> ScenarioObjectConfig {
    // Side-mount rolls (task 20260717-151214): a section's mount base is
    // its local -Y (verified against the GLBs in 20260717-151208's
    // review), so a mount hanging off the spine's +X flank rolls Rz(-90)
    // to seat base-to-hull (-Y -> -X) and a -X mount rolls the mirror
    // Rz(+90). Rz leaves local -Z untouched, so a bay's launch/spawn axis
    // still points ship-forward, and its +Y hatch turns outboard. The
    // bow-mount roll (the player ships' Rx(-90)) stays correct only for
    // spine-end mounts. Ship-local forward is -Z with up +Y, so
    // STARBOARD is +X and PORT is -X - the tube ids used to be swapped.
    let starboard_roll = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    let port_roll = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let turret = |id: &str, offset: Vec3, roll: Quat| SpaceshipSectionConfig {
        id: id.to_string(),
        position: offset,
        rotation: roll,
        source: SectionSource::Prototype("better_turret_section".to_string()),
        modifications: vec![],
    };
    let tube = |id: &str, offset: Vec3, roll: Quat| SpaceshipSectionConfig {
        id: id.to_string(),
        position: offset,
        rotation: roll,
        source: SectionSource::Prototype("torpedo_section".to_string()),
        modifications: vec![],
    };
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
            sections: vec![
                section("controller", "basic_controller_section", Vec3::ZERO),
                section("hull_bow", "reinforced_hull_section", Vec3::Z),
                section("hull_mid", "reinforced_hull_section", Vec3::NEG_Z),
                section("hull_aft", "reinforced_hull_section", Vec3::NEG_Z * 2.0),
                section("thruster", "basic_thruster_section", Vec3::Z * 2.0),
                turret("turret_starboard", Vec3::new(1.0, 0.0, 0.0), starboard_roll),
                turret("turret_port", Vec3::new(-1.0, 0.0, -1.0), port_roll),
                tube("tube_starboard", Vec3::new(1.0, 0.0, -2.0), starboard_roll),
                tube("tube_port", Vec3::new(-1.0, 0.0, -2.0), port_roll),
            ],
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
        objective(
            OBJ_CONTACT,
            "A distress call crackles out of the derelict field: \
             \"Ceres Queen, drive's stripped, they're coming back for \
             the hull.\" Find the hauler.",
        ),
        mark(ID_HAULER, "CERES QUEEN"),
    ]);

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: opening,
        },
        // Act 0 -> 1: reaching the hauler springs the ambush.
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![player_enters(ID_HAULER_AREA), eq_num(VAR_ACT, 0.0)],
            actions: vec![
                set(VAR_ACT, num(1.0)),
                complete(OBJ_CONTACT),
                spawn(corvette(ID_CORVETTE_A, CORVETTE_A_SPAWN)),
                spawn(corvette(ID_CORVETTE_B, CORVETTE_B_SPAWN)),
                objective(
                    OBJ_DEFEND,
                    "Two scavenger corvettes drop off the rocks - the gang's \
                     pickers. Drive them off the Ceres Queen.",
                ),
                unmark(ID_HAULER),
                mark(ID_CORVETTE_A, "CORVETTE"),
                mark(ID_CORVETTE_B, "CORVETTE"),
                emphasize("RADAR"),
            ],
        },
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
        // Act 1 -> 2: both corvettes down - the chapter's CHECKPOINT. The
        // gunship fight is its own scenario now, so the Victory beat here
        // means a death against the capital retries the capital, never
        // this ambush (spike F7). OnUpdate gated on the act makes this a
        // one-shot regardless of which kill lands last; Continue rides the
        // lingering chain into part two.
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_CORVETTE_A_DOWN, 1.0),
                eq_num(VAR_CORVETTE_B_DOWN, 1.0),
            ],
            actions: vec![
                set(VAR_ACT, num(2.0)),
                complete(OBJ_DEFEND),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Victory,
                    "The pickers break off, hulls venting. On the deep scan: \
                     a capital burn, closing fast. The Rust Tally is coming \
                     to finish what its pickers started.",
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
                unmark(ID_HAULER),
                objective(
                    OBJ_HAULER_LOST,
                    "The Ceres Queen is gone. Make it cost them.",
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
        spawn(player_ship()),
        spawn(hauler_ship()),
        cover_scatter(&asteroid_texture),
    ];
    opening.extend(hard_cover(&asteroid_texture).into_iter().map(spawn));
    opening.extend([
        spawn(gunship()),
        objective(
            OBJ_SCREEN,
            "The gang's gunship burns in from the dark, tubes open. \
             Lock the incoming torpedoes and screen them with your \
             PDC.",
        ),
        objective(OBJ_BREAK, "Break the Rust Tally, section by section."),
        mark(ID_GUNSHIP, "GUNSHIP"),
        emphasize("RADAR"),
    ]);

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: opening,
        },
        // Win: the gunship comes apart. End of the base story so far - no
        // queued next scenario, so the overlay offers Main Menu (and Enter
        // exits there too, per the outcome frame).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_GUNSHIP), eq_num(VAR_ACT, 1.0)],
            actions: vec![
                set(VAR_ACT, num(2.0)),
                complete(OBJ_SCREEN),
                complete(OBJ_BREAK),
                unmark(ID_GUNSHIP),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Victory,
                    "The Rust Tally breaks apart. The gang is done picking \
                     this belt clean.",
                )),
            ],
        },
        // Flavor, not failure: same soft-fail beat as part one.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_HAULER), lt_num(VAR_ACT, 2.0)],
            actions: vec![
                unmark(ID_HAULER),
                objective(
                    OBJ_HAULER_LOST,
                    "The Ceres Queen is gone. Make it cost them.",
                ),
            ],
        },
        // Lose: retry THIS part - the checkpoint's whole point (spike F7).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_PLAYER), lt_num(VAR_ACT, 2.0)],
            actions: vec![
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
