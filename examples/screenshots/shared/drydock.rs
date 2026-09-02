//! "Drydock drift": the beauty set the website's pure-3D shots are taken from.
//!
//! A hero Kenney racer posed at the origin, a near-field rock belt close enough
//! to be IN the frame, two neutral cargo hulls flying slow loops through the
//! yard, and a planetoid far enough back that its surface reads as a body rather
//! than a wall. The photo kit lights it (key + rim + fill) instead of the
//! scenario's single top-down key.
//!
//! Included by each drydock producer with
//! `#[path = "shared/drydock.rs"] mod drydock;`. It pulls in `shared/kit.rs`
//! ITSELF, so a producer that includes this must not also include the kit by
//! `#[path]` - two path copies of one file are two distinct modules with two
//! distinct `NearField` types.
//!
//! THE SET DRIFTS, on purpose and in the name: the planetoid is a real well
//! (these shots illustrate gravity, so the body in them had better be one), and
//! at this range it pulls the yard at about 0.01 u/s^2 - meters of drift over a
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
pub const PLANETOID_POSITION: Vec3 = Vec3::new(170.0, -95.0, -560.0);
/// Big enough for its surface to read at that distance (the old reel's was 4
/// units across and read as a pebble). The generated rock reaches well past its
/// nominal radius, so 30 units here draws a body roughly 120 across.
pub const PLANETOID_RADIUS: f32 = 30.0;
/// The body's mass parameter (mu, u^3/s^2), deliberately weak: these shots
/// illustrate gravity, so the body in them is a real well - but at the default
/// 45 000 it would haul the posed set out of frame during a long look. Reach
/// follows strength now, so a weak well is also a SHORT one: the SOI is 156u,
/// and nothing posed further out than that is inside it.
pub const PLANETOID_MASS: f32 = 6_000.0;

/// The set: planetoid, near-field belt, hero racer, two drifting hulls.
pub fn drydock_drift(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    // The hero: a Kenney racer at the origin, turned three-quarters and tipped
    // off the horizontal so it reads as a ship parked in a yard rather than a
    // model on a shelf.
    let hero = ship(
        "drydock_hero",
        "Hero Racer",
        Vec3::ZERO,
        Quat::from_rotation_y(-0.55) * Quat::from_rotation_x(0.10) * Quat::from_rotation_z(0.18),
        SpaceshipController::None,
        None,
        kit::kenney_hull(ships, "racer"),
    );

    // The yard traffic: two cargo hulls flying slow loops around the hero,
    // NEUTRAL so they stay bystanders (nothing in this scene is meant to shoot
    // anything). Their routes run down opposite flanks at different heights, so
    // no framing catches them in a row.
    let hauler_a = ship(
        "drydock_hauler_a",
        "Hauler",
        Vec3::new(-52.0, 9.0, -64.0),
        Quat::from_rotation_y(0.9),
        patrolling(vec![
            Vec3::new(-52.0, 9.0, -64.0),
            Vec3::new(-96.0, 22.0, 30.0),
            Vec3::new(-30.0, 4.0, 86.0),
        ]),
        Some(Allegiance::Neutral),
        kit::kenney_hull(ships, "cargoa"),
    );
    let hauler_b = ship(
        "drydock_hauler_b",
        "Hauler",
        Vec3::new(74.0, -16.0, -98.0),
        Quat::from_rotation_y(-2.1),
        patrolling(vec![
            Vec3::new(74.0, -16.0, -98.0),
            Vec3::new(120.0, -30.0, -10.0),
            Vec3::new(58.0, -8.0, 60.0),
        ]),
        Some(Allegiance::Neutral),
        kit::kenney_hull(ships, "cargob"),
    );

    let belt = kit::NearField {
        id_prefix: "drydock_rock_",
        count: 18,
        seed: 90210,
        distance: (60.0, 165.0),
        radius: (1.5, 5.0),
        y_spread: 70.0,
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
                ThreePointRig::around("photo", Vec3::ZERO, 1.0).actions(),
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
            material: None,
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
pub fn patrolling(route: Vec<Vec3>) -> SpaceshipController {
    SpaceshipController::AI(AIControllerConfig {
        patrol: route,
        ..default()
    })
}

/// One posed ship in the set.
pub fn ship(
    id: &str,
    name: &str,
    position: Vec3,
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
