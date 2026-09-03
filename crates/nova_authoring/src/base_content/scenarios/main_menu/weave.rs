//! The asteroid-weave main-menu backdrop: an AI ship threads a dense rock
//! band on patrol waypoints, steering around whatever the scatter placed.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::{backdrop_beacon, backdrop_camera, backdrop_rig, planetoid_glow};
use crate::base_content::{scenarios::SCATTER_SEED, ships};

/// The circuit's center, nudged left of the frame center: the menu panel
/// owns the right half of the shot, so the loop leans into the open side.
const LOOP_CENTER: Meters3 = Meters3::new(-400.0, 0.0, 0.0);

/// The patrol circuit: TEN waypoints on a 1.4 km ring around [`LOOP_CENTER`],
/// through the middle of the rock band (radius band 1,000-1,900 m, y -600..-280
/// m). Short ~870 m legs mean the autopilot spends its time decelerating,
/// turning and detouring instead of cruising - a slow, deliberate thread. The
/// waypoints are authored mid-band on purpose: the route is NOT threaded
/// through measured gaps; the field is deterministic (seeded scatter, seeded
/// silhouettes) but the pilot, not the author, does the dodging. The whole
/// loop (plus worst-case detours, ~450 m) stays inside the ~2.3 km half-frame.
const WEAVE_LOOP: [Meters3; 10] = [
    Meters3::new(1_000.0, -380.0, 0.0),
    Meters3::new(733.0, -500.0, 823.0),
    Meters3::new(33.0, -420.0, 1_331.0),
    Meters3::new(-833.0, -550.0, 1_331.0),
    Meters3::new(-1_533.0, -350.0, 823.0),
    Meters3::new(-1_800.0, -480.0, 0.0),
    Meters3::new(-1_533.0, -400.0, -823.0),
    Meters3::new(-833.0, -520.0, -1_331.0),
    Meters3::new(33.0, -360.0, -1_331.0),
    Meters3::new(733.0, -460.0, -823.0),
];

/// A navigation drill behind the menu: a cutter flies a ten-waypoint loop
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

    // The ship: an unarmed block cutter on the patrol loop. No hostiles exist
    // here, so it never leaves Patrol; the avoidance layer (AIAvoidanceDetour)
    // rounds whatever rock blocks the current leg. The cutter's two bell
    // drives make it the slowest hull in the fleet, which is the point - the
    // thread reads as deliberate rather than as a fly-through.
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
                // Press in close to each mark: the DEFAULTS turn 750 m out
                // (500 m autopilot standoff + 250 m slack) - most of an 870 m
                // leg. Standoff 100 m parks the computer nearly on the beacon
                // and slack 50 m turns at ~150 m, so the runner visibly
                // REACHES each mark before rolling onto the next.
                waypoint_slack: Some(Meters(50.0)),
                arrival_standoff: Some(Meters(100.0)),
                ..Default::default()
            }),
            hull: ships::hull(ships::BLOCK_CUTTER_SHIP_ID),
            ..Default::default()
        }),
    });

    // The band itself: a tight, genuinely dense shell in the SAME altitude
    // slab as the patrol loop - the route runs through it, not above it.
    // min_separation keeps spawned dynamic bodies from being
    // penetration-shoved into each other (worst-case geometric extent is
    // radius * 6, so 30 m nominal rocks need ~360 m plus ship room).
    let band = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "weave_rock_".to_string(),
        count: 40,
        seed: SCATTER_SEED ^ 0x4,
        region: ScatterRegion::Ring {
            center: LOOP_CENTER,
            inner: Meters(1_000.0),
            outer: Meters(1_900.0),
            y_min: Meters(-600.0),
            y_max: Meters(-280.0),
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "weave_rock_".to_string(),
                name: "Weave Rock".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                material: None,
                destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
                radius: Meters(10.0),
                texture: asteroid_texture,
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((Meters(10.0), Meters(30.0))),
        min_separation: Some(Meters(450.0)),
    });

    let events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                // Pulled further back than the reference shot: the loop's
                // far rim plus its avoidance detours kept escaping the
                // default frame. From (0, 1,270, 4,250) m a 4:3 window sees
                // ~+-2.45 km at origin depth; the loop's worst case is ~2.25 km.
                .chain([
                    backdrop_camera(Meters3::new(0.0, 1_010.0, 3_380.0)),
                    band,
                    // The carousel's rotation limit: the weave has no natural
                    // ending, so after a couple of laps the menu turns to
                    // the next backdrop.
                    EventActionConfig::TimerStart(TimerStartActionConfig {
                        key: "weave_rotate".to_string(),
                        seconds: crate::scenario_helpers::number(150.0),
                    }),
                ])
                .collect(),
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "weave_rotate".to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "menu_duel".to_string(),
                linger: false,
                delay: Some(1.0),
            })],
        },
        // Failsafe: the runner ramming a rock must not leave the menu
        // staring at a pilotless band - a short aftermath linger, then the
        // carousel turns early.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDefeated,
            once: false,
            filters: vec![crate::scenario_helpers::entity("weave_runner")],
            actions: vec![EventActionConfig::TimerStart(TimerStartActionConfig {
                key: "weave_reset".to_string(),
                seconds: crate::scenario_helpers::number(6.0),
            })],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "weave_reset".to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "menu_duel".to_string(),
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
