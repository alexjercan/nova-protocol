//! The asteroid-weave main-menu backdrop: an AI ship threads a dense rock
//! band on patrol waypoints, steering around whatever the scatter placed.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::{backdrop_beacon, backdrop_planetoid, backdrop_rig, planetoid_glow};
use crate::base_content::{scenarios::SCATTER_SEED, ships};

/// The patrol loop: a square-ish circuit through the middle of the rock
/// band (radius band 175-235, y -65..-30), so every leg has rocks ON it and
/// the passive pilot's avoidance detours are what the camera watches. The
/// waypoints are authored mid-band on purpose - the route is NOT threaded
/// through measured gaps; the field is deterministic (seeded scatter, seeded
/// silhouettes) but the pilot, not the author, does the dodging.
const WEAVE_LOOP: [Vec3; 4] = [
    Vec3::new(205.0, -48.0, 0.0),
    Vec3::new(0.0, -40.0, 205.0),
    Vec3::new(-205.0, -55.0, 0.0),
    Vec3::new(0.0, -45.0, -205.0),
];

/// A navigation drill behind the menu: a racer flies a four-waypoint loop
/// straight through the backdrop's rock band. There is nothing hostile
/// anywhere; the scene is the passive pilot itself - GOTO legs, waypoint
/// turns, and the avoidance detours around every rock the seeded scatter
/// put on the line.
pub(crate) fn menu_weave(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut objects = Vec::new();

    // The camera contract well, plus the shared rig and lamp.
    objects.push(backdrop_planetoid(asteroid_texture.clone(), 45_000.0));
    objects.extend(backdrop_rig("weave").objects());
    objects.push(planetoid_glow("weave_lamp"));

    // Nav beacons at the circuit's corners: the waypoints, made visible.
    // Intangible dressing - beacon volumes stop nothing, so the ship can fly
    // through its own markers.
    for (index, corner) in WEAVE_LOOP.iter().enumerate() {
        objects.push(backdrop_beacon(
            &format!("weave_nav_{index}"),
            &format!("NAV-{}", index + 1),
            *corner,
            Color::srgb(0.55, 0.85, 1.0),
        ));
    }

    // The ship: a racer on the patrol loop. No hostiles exist here, so it
    // never leaves Patrol; the avoidance layer (AIAvoidanceDetour) rounds
    // whatever rock blocks the current leg.
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "weave_runner".to_string(),
            name: "Weave Runner".to_string(),
            position: WEAVE_LOOP[0],
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: WEAVE_LOOP.to_vec(),
                ..Default::default()
            }),
            sections: ships::racer_sections(ships::ShipGrade::Player, vec![]),
        }),
    });

    // The band itself: denser than the other backdrops' dressing rings, in
    // the SAME altitude slab as the patrol loop - the route runs through it,
    // not above it. min_separation keeps spawned dynamic bodies from being
    // penetration-shoved into each other (worst-case geometric extent is
    // radius * 6, so 3 u nominal rocks need ~36 u plus ship room).
    let band = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "weave_rock_".to_string(),
        count: 28,
        seed: SCATTER_SEED ^ 0x4,
        region: ScatterRegion::Ring {
            center: Vec3::ZERO,
            inner: 175.0,
            outer: 235.0,
            y_min: -65.0,
            y_max: -30.0,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "weave_rock_".to_string(),
                name: "Weave Rock".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
                radius: 1.0,
                texture: asteroid_texture,
                health: 100.0,
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.0, 3.0)),
        min_separation: Some(45.0),
    });

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .chain([band])
            .collect(),
    }];

    ScenarioConfig {
        description: "An AI ship weaves a waypoint circuit through a dense rock band.".to_string(),
        hidden: true,
        menu_backdrop: true,
        events,
        ..ScenarioConfig::new(
            "menu_weave".to_string(),
            "Asteroid Weave".to_string(),
            cubemap,
        )
    }
}
