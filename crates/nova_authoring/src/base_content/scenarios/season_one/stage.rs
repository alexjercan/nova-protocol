//! Everything chapter one puts in the sky: the two ships, the working lane
//! and its marks, the rock the lane runs through, and the moons it runs
//! between.
//!
//! The place is Saturn's inner network, between Aquila and Baikal, and it is
//! drawn with three things and no station: a lane of small rock, two icy
//! bodies standing off it, and a hazy one further out. Baikal and Aquila are
//! in the dialogue and nowhere else - the chapter is the hour between them.
//!
//! The lane is the chapter's teaching ground. Four marks, each one offset
//! from the last across two axes, with rock thick enough between them that
//! swinging the whole hull around costs more than sliding it: the RCS lesson
//! is the LAYOUT, not a line of dialogue about thrusters.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use super::super::{marks::Mark, SCATTER_SEED};
use crate::base_content::ships;

// --- the two ships -----------------------------------------------------------

/// The player's hull, and the id every helm, area and docking event names.
pub(crate) const ID_KAVERI: &str = "kaveri";
/// The stranded hull the chapter is flown out to.
pub(crate) const ID_GANTRY: &str = "gantry";

/// What the two hulls are called on the HUD and in the objective text.
pub(crate) const KAVERI_NAME: &str = "Kaveri";
pub(crate) const GANTRY_NAME: &str = "Gantry";

/// Soft manual-speed cap for the whole chapter.
///
/// A working ship with an outsized load strapped to an open cradle is flown
/// deliberately, and the lane is a few kilometres of rock: the cap is what
/// makes threading it a matter of placing the hull rather than of reflexes.
pub(crate) const KAVERI_SPEED_CAP: MetersPerSecond = MetersPerSecond(120.0);

/// Kaveri's flight computer and docking collar, armoured for the chapter.
///
/// Neither is armour for a fight - nothing here shoots. They are armour
/// against the ROCK: a hull that clips a boulder hard enough to lose its
/// collar could still fly the rest of the chapter and never be able to finish
/// it, and a rescue that cannot be finished is worse than one that is lost
/// outright. Losing the ship is still a defeat, and still ends the chapter.
const KAVERI_BRIDGE_HEALTH: f32 = 1_800.0;
const KAVERI_COLLAR_HEALTH: f32 = 1_800.0;

/// Gantry's surviving collar, armoured for the same reason and one more: it
/// is the port the story says survived the raid, and the only way home for
/// three people.
const GANTRY_COLLAR_HEALTH: f32 = 1_800.0;

/// Where Gantry drifts, and the pose it drifts in.
///
/// Off the far end of the lane and well to starboard, so reaching it is a
/// course change out of the rock rather than more of the same lane. Its
/// heading is the lane's, which puts its PORT collar on the side the rescue
/// arrives from: Kaveri comes about, stands off that flank, and the two hulls
/// end up facing opposite ways with their two hatches looking at each other -
/// the arrangement the story docks in.
const GANTRY_POSITION: Meters3 = Meters3::new(2_400.0, 0.0, -6_000.0);

/// Kaveri on the lane, loaded and running home.
pub(crate) fn kaveri() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_KAVERI.to_string(),
            name: KAVERI_NAME.to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Player),
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: Default::default(),
                speed_cap: Some(KAVERI_SPEED_CAP),
            }),
            // Every verb is in the player's hands from the first frame. The
            // crew are professionals who have flown this ship for years, and
            // the chapter is the job rather than the lesson: what it teaches,
            // it teaches by asking for it.
            capabilities: ShipCapabilities::default(),
            design: ships::patched_design(
                ships::BLOCK_WORKSHIP_SHIP_ID,
                [
                    ships::on_section(
                        ships::BLOCK_BRIDGE_SECTION_ID,
                        ships::section_health(KAVERI_BRIDGE_HEALTH),
                    ),
                    ships::on_section(
                        ships::BLOCK_PORT_COLLAR_SECTION_ID,
                        ships::section_health(KAVERI_COLLAR_HEALTH),
                    ),
                ],
            ),
        }),
    }
}

/// Gantry, stranded: nobody at the helm, nothing hostile aboard, and the hull
/// intact from outside. The damage the story gives it is to its main drive
/// and its distribution - neither of which is a thing a hull shows - so what
/// the player reads is a working ship going nowhere.
pub(crate) fn gantry() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_GANTRY.to_string(),
            name: GANTRY_NAME.to_string(),
            position: GANTRY_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Neutral),
            controller: SpaceshipController::None,
            capabilities: ShipCapabilities::default(),
            design: ships::patched_design(
                ships::BLOCK_FRAME_TENDER_SHIP_ID,
                [ships::on_section(
                    ships::BLOCK_PORT_COLLAR_SECTION_ID,
                    ships::section_health(GANTRY_COLLAR_HEALTH),
                )],
            ),
        }),
    }
}

// --- the working lane --------------------------------------------------------

/// The four marks of the lane home, in order.
///
/// Each one is offset from the last across BOTH the lateral axes and never by
/// much: a few hundred metres of sidestep at a walking pace, which is a
/// thruster job. The volumes are tight for the same reason - a wide sphere
/// would pass a ship that merely drifted past the mark, and the point of the
/// lane is that the hull is put where it is meant to be.
pub(crate) const LANE: [Mark; 4] = [
    Mark {
        id: "lane_one",
        label: "LANE-1",
        position: Meters3::new(0.0, 0.0, -1_200.0),
        area: Meters(220.0),
    },
    Mark {
        id: "lane_two",
        label: "LANE-2",
        position: Meters3::new(-340.0, 90.0, -2_300.0),
        area: Meters(180.0),
    },
    Mark {
        id: "lane_three",
        label: "LANE-3",
        position: Meters3::new(300.0, -110.0, -3_400.0),
        area: Meters(180.0),
    },
    Mark {
        id: "lane_four",
        label: "LANE-4",
        position: Meters3::new(-140.0, 60.0, -4_500.0),
        area: Meters(200.0),
    },
];

/// The volume Gantry sits in the middle of: the chapter's last arrival gate.
///
/// A gate and nothing else - no beacon, because a stranded ship a kilometre
/// across the sky IS the landmark, and a lit buoy hung next to it would say
/// the scenario put it there. The radius is wide enough that a captain who
/// stands off to look at the hull has already arrived.
pub(crate) const APPROACH: Mark = Mark {
    id: "gantry_approach",
    label: GANTRY_NAME,
    position: GANTRY_POSITION,
    area: Meters(700.0),
};

/// The rock the lane runs through: one box of small bodies around the whole
/// run, seeded, so the field is the same field every time the chapter is
/// flown and a route that worked once works again.
///
/// Small and many rather than few and large, which is what a working lane in
/// a ring system is: nothing here is a landmark, and nothing here is worth
/// steering wide of by a kilometre. `min_separation` is what keeps the gaps
/// flyable - the widest bodies reach about six times their nominal radius, so
/// two neighbours at their worst still leave better than a hull's width
/// between them.
const LANE_ROCK_COUNT: u32 = 96;
const LANE_ROCK_RADIUS: (Meters, Meters) = (Meters(6.0), Meters(16.0));
const LANE_ROCK_SEPARATION: Meters = Meters(300.0);
const LANE_ROCK_MIN: Meters3 = Meters3::new(-900.0, -420.0, -5_200.0);
const LANE_ROCK_MAX: Meters3 = Meters3::new(900.0, 420.0, -600.0);

/// A second, thinner drift standing off the lane between its far end and
/// Gantry, so the course change out to the wreck is flown through something
/// rather than across empty sky.
const DRIFT_ROCK_COUNT: u32 = 34;
const DRIFT_ROCK_RADIUS: (Meters, Meters) = (Meters(8.0), Meters(22.0));
const DRIFT_ROCK_SEPARATION: Meters = Meters(400.0);
const DRIFT_ROCK_MIN: Meters3 = Meters3::new(700.0, -500.0, -6_600.0);
const DRIFT_ROCK_MAX: Meters3 = Meters3::new(1_900.0, 500.0, -4_800.0);

/// Both fields as scatter actions, each reproducible from its own seed.
pub(crate) fn rock(texture: &AssetRef<Image>) -> Vec<EventActionConfig> {
    vec![
        scatter(
            "lane_rock_",
            "Lane Rock",
            LANE_ROCK_COUNT,
            SCATTER_SEED ^ 0x1A4E_0001,
            (LANE_ROCK_MIN, LANE_ROCK_MAX),
            LANE_ROCK_RADIUS,
            LANE_ROCK_SEPARATION,
            texture,
        ),
        scatter(
            "drift_rock_",
            "Drift Rock",
            DRIFT_ROCK_COUNT,
            SCATTER_SEED ^ 0x1A4E_0002,
            (DRIFT_ROCK_MIN, DRIFT_ROCK_MAX),
            DRIFT_ROCK_RADIUS,
            DRIFT_ROCK_SEPARATION,
            texture,
        ),
    ]
}

#[expect(
    clippy::too_many_arguments,
    reason = "one scatter field's whole authored description, named at both call sites"
)]
fn scatter(
    id_prefix: &str,
    name: &str,
    count: u32,
    seed: u64,
    region: (Meters3, Meters3),
    radius: (Meters, Meters),
    min_separation: Meters,
    texture: &AssetRef<Image>,
) -> EventActionConfig {
    EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: id_prefix.to_string(),
        count,
        seed,
        region: ScatterRegion::Box {
            min: region.0,
            max: region.1,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id_prefix.to_string(),
                name: name.to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                kind: KIND_ICE.to_string(),
                destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
                radius: radius.0,
                texture: texture.clone(),
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some(radius),
        // Saturn's rock is mostly water ice, with stone and carbon in it. The
        // mix is the region's geology, and it is why the lane reads white
        // rather than as another grey belt.
        asteroid_kinds: vec![
            (KIND_ICE.to_string(), 10),
            (KIND_ROCK.to_string(), 4),
            (KIND_CARBON.to_string(), 2),
        ],
        min_separation: Some(min_separation),
    })
}

// --- the neighbourhood -------------------------------------------------------

/// One body standing off the lane, and what it is made of.
struct Moon {
    id: &'static str,
    name: &'static str,
    kind: PlanetType,
    position: Meters3,
    radius: Meters,
    seed: u32,
}

/// Three moons, at the distances that make them scenery rather than
/// destinations.
///
/// Their mass is the same small figure for all three, and it is deliberate:
/// the sphere of influence a mass that size buys (about 1.3 km) is smaller
/// than any of these bodies, so each well floors at its own surface and NONE
/// of them pulls on the lane. The chapter is flown between them, not around
/// them, and a rescue that drifted off course because a decorative moon had
/// a grip on it would be a bug wearing a story's clothes.
const MOON_MASS: f32 = 4_000.0;

const MOONS: [Moon; 3] = [
    // The near one: bright water ice, standing off the lane's port side,
    // close enough to light the rock and read as somewhere.
    Moon {
        id: "moon_near",
        name: "Ice Moon",
        kind: PlanetType::IceWorld,
        position: Meters3::new(-7_800.0, -900.0, -4_200.0),
        radius: Meters(1_350.0),
        seed: 20_780_101,
    },
    // The far one: older, cratered, grey against the ice.
    Moon {
        id: "moon_far",
        name: "Grey Moon",
        kind: PlanetType::BarrenRock,
        position: Meters3::new(9_600.0, 1_400.0, -11_000.0),
        radius: Meters(1_900.0),
        seed: 20_780_102,
    },
    // The horizon: a hazy body with almost no silhouette, which is what a
    // thick atmosphere looks like from a working lane.
    Moon {
        id: "moon_hazy",
        name: "Hazy Moon",
        kind: PlanetType::Greenhouse,
        position: Meters3::new(-2_600.0, 2_600.0, -17_500.0),
        radius: Meters(2_600.0),
        seed: 20_780_103,
    },
];

/// The three moons as spawns.
pub(crate) fn moons() -> Vec<ScenarioObjectConfig> {
    MOONS
        .iter()
        .map(|moon| ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: moon.id.to_string(),
                name: moon.name.to_string(),
                position: moon.position,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Planet(
                PlanetConfig::new(moon.kind, moon.radius, moon.seed).anchored(MOON_MASS),
            ),
        })
        .collect()
}

/// The light on the chapter: one distant sun off the lane's shoulder, and the
/// fill that keeps the shadowed side of a hull readable.
///
/// `key` prefixes the rig's object ids, the way the training range's does.
pub(crate) fn lights(key: &str) -> Vec<ScenarioObjectConfig> {
    ThreePointRig::around(key, Meters3::new(0.0, 0.0, -3_000.0), 25.0).objects()
}
