//! Broadside - the capital-combat vertical slice (task 20260708-203659).
//!
//! Chapter two of the base storyline: the scavenger driven off in Shakedown
//! Run was a scout. Its gang comes back in force to strip the belt - and a
//! neutral hauler is caught in the middle. Three acts under an explicit
//! win/lose frame:
//!
//! - Act 0 (contact): answer the hauler's distress call across an asteroid
//!   cover field.
//! - Act 1 (escalation): two scavenger corvettes jump the player at the
//!   hauler - a guns dogfight.
//! - Act 2 (the twist / climax): the gang's GUNSHIP burns in from the dark -
//!   a capital with turrets and torpedo tubes. Screen its torpedoes with the
//!   PDC, then break it section by section.
//!
//! Win: gunship destroyed -> Victory overlay (end of the base story so far;
//! Enter/Main Menu). Lose: player destroyed -> Defeat + lingering retry.
//! The hauler is a NEUTRAL ship (the `SpaceshipConfig.allegiance` override):
//! nobody targets it, but stray blast damage can kill it - a flavor beat
//! reacts, the mission continues.
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

/// Story act (0 contact, 1 corvettes, 2 gunship, 3 won). Every gate filter
/// checks it, so beats fire once and in order.
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
/// them. Infinite turret ammo like chapter one - there is no resupply
/// mechanic yet, so a dry magazine mid-gunship-fight is frustration, not
/// pressure (playtest verdict, task 20260716-160159).
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
                infinite_ammo: true,
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
    let turret = |id: &str, offset: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position: offset,
        rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        source: SectionSource::Prototype("better_turret_section".to_string()),
        modifications: vec![],
    };
    let tube = |id: &str, offset: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position: offset,
        rotation: Quat::IDENTITY,
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
                turret("turret_dorsal", Vec3::new(1.0, 0.0, 0.0)),
                turret("turret_ventral", Vec3::new(-1.0, 0.0, -1.0)),
                tube("tube_port", Vec3::new(1.0, 0.0, -2.0)),
                tube("tube_starboard", Vec3::new(-1.0, 0.0, -2.0)),
            ],
        }),
    }
}

pub(crate) fn broadside(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    // Asteroid cover along the lane between the player and the hauler, so
    // the approach weaves through rock and the fights have cover. A Box
    // region (the Ring variant is origin-centred; sample() REPLACES the
    // template position, it does not offset it) with margins that keep the
    // player spawn (z=40) and the hauler (z=-450) themselves clear.
    let cover_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
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
                radius: 1.0,
                texture: asteroid_texture.clone(),
                health: 100.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.5, 4.0)),
    });

    let events = vec![
        // Act 0: the stage and the hook.
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![
                set(VAR_ACT, num(0.0)),
                set(VAR_CORVETTE_A_DOWN, num(0.0)),
                set(VAR_CORVETTE_B_DOWN, num(0.0)),
                spawn(player_ship()),
                spawn(hauler_ship()),
                cover_scatter,
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
            ],
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
        // Act 1 -> 2: both corvettes down - the twist. OnUpdate gated on the
        // act makes this a one-shot regardless of which kill lands last.
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
            ],
        },
        // Win: the gunship comes apart. End of the base story so far - no
        // queued next scenario, so the overlay offers Main Menu (and Enter
        // exits there too, per the outcome frame).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_GUNSHIP), eq_num(VAR_ACT, 2.0)],
            actions: vec![
                set(VAR_ACT, num(3.0)),
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
        // Flavor, not failure: the hauler dies to stray fire and the story
        // notices - but only while the fight is on; after the win nothing
        // pushes fresh objectives under the Victory overlay (review R1.5).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_HAULER), lt_num(VAR_ACT, 3.0)],
            actions: vec![
                unmark(ID_HAULER),
                objective(
                    OBJ_HAULER_LOST,
                    "The Ceres Queen is gone. Make it cost them.",
                ),
            ],
        },
        // Lose: the Defeat overlay offers Retry (lingering restart) and
        // Main Menu. Gated to the live acts: a death AFTER the win (the
        // gunship's own death blast, a drifting rock) must not overwrite
        // the earned Victory with Defeat (review R1.3).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_PLAYER), lt_num(VAR_ACT, 3.0)],
            actions: vec![
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "The scavengers strip your wreck for parts.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: BROADSIDE_SCENARIO_ID.to_string(),
                    linger: true,
                }),
            ],
        },
    ];

    ScenarioConfig {
        id: BROADSIDE_SCENARIO_ID.to_string(),
        name: "Broadside".to_string(),
        description: "The scavengers come back in force: answer a hauler's \
                      distress call, break an ambush, and screen the gang \
                      gunship's torpedoes with your PDC. Chapter two of the \
                      base storyline."
            .to_string(),
        cubemap,
        // Placeholder thumbnail (real per-scenario art: task 20260715-220011).
        thumbnail: Some(AssetRef::from("banner.png")),
        hidden: false,
        menu_backdrop: false,
        events,
    }
}
