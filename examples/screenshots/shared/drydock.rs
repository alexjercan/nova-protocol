//! "Drydock drift": the beauty set the website's pure-3D shots are taken from.
//!
//! A hero gunship posed at the origin, a near-field rock belt close enough to be
//! IN the frame, two neutral working hulls flying slow loops through the yard,
//! and a planetoid far enough back that its surface reads as a body rather than
//! a wall. The photo kit lights it (key + rim + fill) instead of the scenario's
//! single top-down key.
//!
//! Included by each drydock producer with
//! `#[path = "shared/drydock.rs"] mod drydock;`. It pulls in `shared/kit.rs`
//! ITSELF, so a producer that includes this must not also include the kit by
//! `#[path]` - two path copies of one file are two distinct modules with two
//! distinct `NearField` types.
//!
//! THE SET DRIFTS, on purpose and in the name: the planetoid is a real well
//! (these shots illustrate gravity, so the body in them had better be one), and
//! at this range it pulls the yard at about 0.1 m/s^2 - meters of drift over a
//! long free-fly look, nothing over a capture, which freezes the scene anyway
//! (`freeze_bodies`).

// Each producer includes the whole set and uses the part its shot needs; the
// unused half is not dead code, it is another shot's tool.
#![allow(
    dead_code,
    reason = "one source, many example targets: what one producer leaves unused another needs, so no single build can fulfil an expectation"
)]

#[path = "kit.rs"]
mod kit;

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// The planetoid's scenario id.
pub const PLANETOID_ID: &str = "drydock_planetoid";
/// Where the planetoid sits: down and well behind the yard, so a wide shot has
/// a body in it and the hero still has open space around it.
pub const PLANETOID_POSITION: Meters3 = Meters3::new(1_700.0, -950.0, -5_600.0);
/// Big enough for its surface to read at that distance (the old reel's was 40 m
/// across and read as a pebble). The generated rock reaches well past its
/// nominal radius, so 300 m here draws a body roughly 1.2 km across.
pub const PLANETOID_RADIUS: Meters = Meters(300.0);
/// The body's mass parameter (mu), deliberately weak: these shots illustrate
/// gravity, so the body in them is a real well - but at the default 45 000 it
/// would haul the posed set out of frame during a long look. A gravitational
/// parameter, not a length: it stays in the engine's own scale (world units
/// cubed per second squared), which is why it carries no meter type. Reach
/// follows strength now, so a weak well is also a SHORT one: the SOI is 156
/// world units, 1.56 km, and nothing posed further out than that is inside it.
pub const PLANETOID_MASS: f32 = 6_000.0;

/// The set: planetoid, near-field belt, hero gunship, two drifting hulls.
pub fn drydock_drift(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    // The hero: a patrol gunship at the origin, turned three-quarters and tipped
    // off the horizontal so it reads as a ship parked in a yard rather than a
    // model on a shelf. The fleet's most detailed silhouette, which is what a
    // beauty shot wants.
    let hero = ship(
        "drydock_hero",
        "Hero Gunship",
        Meters3::ZERO,
        Quat::from_rotation_y(-0.55) * Quat::from_rotation_x(0.10) * Quat::from_rotation_z(0.18),
        SpaceshipController::None,
        None,
        kit::catalog_hull(ships, "block_gunship"),
    );

    // The yard traffic: two working hulls flying slow loops around the hero,
    // NEUTRAL so they stay bystanders (nothing in this scene is meant to shoot
    // anything). Their routes run down opposite flanks at different heights, so
    // no framing catches them in a row.
    let hauler_a = ship(
        "drydock_hauler_a",
        "Hauler",
        Meters3::new(-520.0, 90.0, -640.0),
        Quat::from_rotation_y(0.9),
        patrolling(vec![
            Meters3::new(-520.0, 90.0, -640.0),
            Meters3::new(-960.0, 220.0, 300.0),
            Meters3::new(-300.0, 40.0, 860.0),
        ]),
        Some(Allegiance::Neutral),
        kit::catalog_hull(ships, "block_hauler"),
    );
    let hauler_b = ship(
        "drydock_hauler_b",
        "Hauler",
        Meters3::new(740.0, -160.0, -980.0),
        Quat::from_rotation_y(-2.1),
        patrolling(vec![
            Meters3::new(740.0, -160.0, -980.0),
            Meters3::new(1_200.0, -300.0, -100.0),
            Meters3::new(580.0, -80.0, 600.0),
        ]),
        Some(Allegiance::Neutral),
        kit::catalog_hull(ships, "block_cutter"),
    );

    let belt = kit::NearField {
        id_prefix: "drydock_rock_",
        count: 18,
        seed: 90210,
        distance: (Meters(600.0), Meters(1_650.0)),
        radius: (Meters(15.0), Meters(50.0)),
        y_spread: Meters(700.0),
    };

    ScenarioConfig {
        description: "A salvage yard adrift over a planetoid - the website's beauty set."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            // The photo rig, authored content rather than an example-side
            // observer swap: scale 1.0 around the origin reproduces the kit's
            // exact key/rim/fill numbers, so the captured frames are unchanged.
            actions: [
                vec![
                    planetoid(game_assets),
                    belt.action(game_assets),
                    hero,
                    hauler_a,
                    hauler_b,
                ],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "drydock_drift".to_string(),
            "Drydock Drift".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The planetoid: the set's backdrop body and its only gravity source.
/// Invulnerable - nothing should be able to shoot the scenery out of a shot.
pub fn planetoid(game_assets: &GameAssets) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLANETOID_ID.to_string(),
            name: "Planetoid".to_string(),
            position: PLANETOID_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: PLANETOID_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
            material: KIND_ROCK.to_string(),
            destroy_sound: None,
            mass: Some(PLANETOID_MASS),
            invulnerable: true,
            seed: None,
            lock_signature: None,
        }),
    })
}

/// A hull's routine: a slow loop around the yard.
///
/// Patrol, not `orbit`: the planetoid sits far enough back to read as a body
/// rather than a wall, which puts the yard outside the orbit band the AI flies
/// to - an orbiting hull would leave the set for the rock. A waypoint loop
/// keeps the traffic where the camera is.
pub fn patrolling(route: Vec<Meters3>) -> SpaceshipController {
    SpaceshipController::AI(AIControllerConfig {
        patrol: route,
        ..default()
    })
}

/// One posed ship in the set.
pub fn ship(
    id: &str,
    name: &str,
    position: Meters3,
    rotation: Quat,
    controller: SpaceshipController,
    allegiance: Option<Allegiance>,
    sections: Vec<SpaceshipSectionConfig>,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller,
            allegiance,
            hull: ShipSource::Inline(ShipHull {
                sections,
                ..default()
            }),
            ..default()
        }),
    })
}
