//! Chapter-one object and layout configuration.
//!
//! Four lane marks are offset across two axes. The intervening rocks make a
//! full hull rotation cost more distance than lateral translation, so the
//! layout exercises RCS placement without a scripted control restriction.

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
/// The target hull referenced by approach and docking events.
pub(crate) const ID_GANTRY: &str = "gantry";

/// What the two hulls are called on the HUD and in the objective text.
pub(crate) const KAVERI_NAME: &str = "Kaveri";
pub(crate) const GANTRY_NAME: &str = "Gantry";

/// Health for the player ship's required controller and docking collar.
///
/// Both survive incidental rock collisions so the scenario cannot remain
/// active after its required docking section is destroyed. Ship destruction
/// still triggers defeat.
const KAVERI_BRIDGE_HEALTH: f32 = 1_800.0;
const KAVERI_COLLAR_HEALTH: f32 = 1_800.0;

/// Health for the scenario's only target docking collar.
const GANTRY_COLLAR_HEALTH: f32 = 1_800.0;

/// Target position and orientation.
///
/// The lateral offset requires a course change after the lane. Matching its
/// heading places the port collar on the player's approach side and aligns the
/// two docking faces.
const GANTRY_POSITION: Meters3 = Meters3::new(2_400.0, 0.0, -6_000.0);

/// Player ship at the lane entrance.
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
            }),
            // Every verb is in the player's hands from the first frame: what
            // the chapter teaches, it teaches by asking for it.
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

/// Gantry, stranded: nobody at the helm, nothing hostile aboard, and a stern
/// that is not there any more.
///
/// It flies the DAMAGED tender: the same hull with its drive, its transom,
/// its service stack and half its aft arch missing. Nothing drives it and
/// nothing is authored to move it, so the hull the player closes on is exactly
/// as still as the intact one: it LOOKS worse and the clamp is no harder.
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
                ships::BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID,
                [ships::on_section(
                    ships::BLOCK_PORT_COLLAR_SECTION_ID,
                    ships::section_health(GANTRY_COLLAR_HEALTH),
                )],
            ),
        }),
    }
}

/// The plating that came off Gantry, still hanging where it was thrown.
///
/// Three pieces, hand-placed rather than scattered: they belong to the stern
/// and the starboard arch the hull is missing, so they sit off those two faces
/// and nowhere else. The port side - the flank Kaveri comes about onto, and
/// the one the collar is on - is deliberately clear. A field of loose rock
/// around the one place the player has to fly precisely would make the dock a
/// collision test.
pub(crate) fn wreckage() -> Vec<ScenarioObjectConfig> {
    const PIECES: [(&str, Meters3, f32, f32); 3] = [
        (
            "gantry_debris_transom",
            Meters3::new(60.0, 20.0, 90.0),
            0.9,
            0.4,
        ),
        (
            "gantry_debris_stack",
            Meters3::new(5.0, -35.0, 130.0),
            2.3,
            -0.7,
        ),
        (
            "gantry_debris_arch",
            Meters3::new(85.0, 5.0, 55.0),
            1.6,
            1.2,
        ),
    ];

    PIECES
        .into_iter()
        .map(|(id, offset, yaw, pitch)| ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: "Debris".to_string(),
                position: GANTRY_POSITION + offset,
                rotation: Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch),
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                allegiance: Some(Allegiance::Neutral),
                controller: SpaceshipController::None,
                capabilities: ShipCapabilities::default(),
                design: ships::design(ships::BLOCK_WRECK_PLATE_SHIP_ID),
            }),
        })
        .collect()
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

/// The final arrival gate, centered on the target hull.
///
/// Its 700 m radius registers a player who stops outside docking range to
/// inspect the target. No separate beacon is required.
pub(crate) const APPROACH: Mark = Mark {
    id: "gantry_approach",
    label: GANTRY_NAME,
    position: GANTRY_POSITION,
    area: Meters(700.0),
};

/// A second ring inside the first, on the same place: the gate the collar
/// dialogue runs on.
///
/// Two rings separate arrival from the final approach. ARRIVING belongs where
/// the hull fills the canopy: the card comes down and the marker comes off.
/// The collar dialogue is four cards' worth of reading, and at 700 m the player
/// is still braking out of a course change while they land. Inside this ring
/// the approach is nearly over - the ship is slow and the flying left is the
/// last hundred metres of it.
///
/// A gate and nothing else, like the ring outside it.
pub(crate) const STANDOFF: Mark = Mark {
    id: "gantry_standoff",
    label: GANTRY_NAME,
    position: GANTRY_POSITION,
    area: Meters(450.0),
};

/// The rock the lane runs through: one box of small bodies around the whole
/// run, seeded, so the field is the same field every time the chapter is
/// flown and a route that worked once works again.
///
/// Small, numerous rocks avoid creating a single dominant landmark.
/// `min_separation` keeps the gaps flyable: the widest bodies reach about six
/// times their nominal radius, so worst-case neighbours still leave more than
/// one hull width between them.
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
        // Mostly water ice, with stone and carbon in it: the mix is what
        // makes the lane read white rather than as another grey belt.
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
/// of them pulls on the lane, so a decorative body can never drag the flight
/// off course.
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
