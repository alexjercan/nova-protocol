//! The asteroid-weave main-menu backdrop: an AI ship threads a dense rock
//! band on patrol waypoints, steering around whatever the scatter placed.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::{backdrop_beacon, backdrop_camera, backdrop_rig, planetoid_glow};
use crate::base_content::{scenarios::SCATTER_SEED, ships};

/// The circuit's center, nudged left of the frame center: the menu panel
/// owns the right half of the shot, so the loop leans into the open side.
const LOOP_CENTER: Vec3 = Vec3::new(-40.0, 0.0, 0.0);

/// The patrol circuit: TEN waypoints on a 140 u ring around [`LOOP_CENTER`],
/// through the middle of the rock band (radius band 100-190, y -60..-28).
/// Short ~87 u legs mean the autopilot spends its time decelerating, turning
/// and detouring instead of cruising - a slow, deliberate thread. The
/// waypoints are authored mid-band on purpose: the route is NOT threaded
/// through measured gaps; the field is deterministic (seeded scatter, seeded
/// silhouettes) but the pilot, not the author, does the dodging. The whole
/// loop (plus worst-case detours, ~45 u) stays inside the ~230 u half-frame.
const WEAVE_LOOP: [Vec3; 10] = [
    Vec3::new(100.0, -38.0, 0.0),
    Vec3::new(73.3, -50.0, 82.3),
    Vec3::new(3.3, -42.0, 133.1),
    Vec3::new(-83.3, -55.0, 133.1),
    Vec3::new(-153.3, -35.0, 82.3),
    Vec3::new(-180.0, -48.0, 0.0),
    Vec3::new(-153.3, -40.0, -82.3),
    Vec3::new(-83.3, -52.0, -133.1),
    Vec3::new(3.3, -36.0, -133.1),
    Vec3::new(73.3, -46.0, -82.3),
];

/// A navigation drill behind the menu: a racer flies a ten-waypoint loop
/// straight through the backdrop's rock band. There is nothing hostile
/// anywhere; the scene is the passive pilot itself - GOTO legs, waypoint
/// turns, and the avoidance detours around every rock the seeded scatter
/// put on the line.
pub(crate) fn menu_weave(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut objects = Vec::new();

    objects.extend(backdrop_rig("weave").objects());
    objects.push(planetoid_glow("weave_lamp"));

    // Nav beacons at every other corner: the circuit, made visible without
    // ten blinking orbs of clutter. Intangible dressing - beacon volumes
    // stop nothing, so the ship can fly through its own markers.
    for (index, corner) in WEAVE_LOOP.iter().enumerate().step_by(2) {
        objects.push(backdrop_beacon(
            &format!("weave_nav_{index}"),
            &format!("NAV-{}", index / 2 + 1),
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
                // Press in close to each mark: the DEFAULTS turn 75 u out
                // (50 u autopilot standoff + 25 u slack) - most of an 87 u
                // leg. Standoff 10 parks the computer nearly on the beacon
                // and slack 5 turns at ~15 u, so the runner visibly REACHES
                // each mark before rolling onto the next.
                waypoint_slack: Some(5.0),
                arrival_standoff: Some(10.0),
                ..Default::default()
            }),
            sections: ships::racer_sections(ships::ShipGrade::Player, vec![]),
        }),
    });

    // The band itself: a tight, genuinely dense shell in the SAME altitude
    // slab as the patrol loop - the route runs through it, not above it.
    // min_separation keeps spawned dynamic bodies from being
    // penetration-shoved into each other (worst-case geometric extent is
    // radius * 6, so 3 u nominal rocks need ~36 u plus ship room).
    let band = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "weave_rock_".to_string(),
        count: 40,
        seed: SCATTER_SEED ^ 0x4,
        region: ScatterRegion::Ring {
            center: LOOP_CENTER,
            inner: 100.0,
            outer: 190.0,
            y_min: -60.0,
            y_max: -28.0,
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

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                // Pulled further back than the reference shot: the loop's
                // far rim plus its avoidance detours kept escaping the
                // default frame. From (0, 127, 425) a 4:3 window sees
                // ~+-245 u at origin depth; the loop's worst case is ~225.
                .chain([backdrop_camera(Vec3::new(0.0, 127.0, 425.0)), band])
                .collect(),
        },
        // Failsafe: the runner ramming a rock must not leave the menu
        // staring at a pilotless band. A short aftermath linger, then the
        // scenario reloads itself - same full-reset idiom as the other
        // backdrops (destroyed rocks come back too; the scatter is seeded).
        ScenarioEventConfig {
            name: EventConfig::OnDefeated,
            filters: vec![crate::scenario_helpers::entity("weave_runner")],
            actions: vec![EventActionConfig::TimerStart(TimerStartActionConfig {
                key: "weave_reset".to_string(),
                seconds: crate::scenario_helpers::number(6.0),
            })],
        },
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "weave_reset".to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "menu_weave".to_string(),
                linger: false,
                delay: Some(1.0),
            })],
        },
    ];

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
