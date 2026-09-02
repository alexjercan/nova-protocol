//! The freight-waystation main-menu backdrop.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::*;
use crate::base_content::scenarios::SCATTER_SEED;

pub(crate) fn menu_waystation(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut objects = vec![
        // Lighter pull for the two heavy haulers to hold their orbit.
        backdrop_planetoid(asteroid_texture.clone(), 30_000.0),
        backdrop_orbiter(
            "waystation_hauler_a",
            "Hauler Biscuit",
            Meters3::new(1_400.0, 0.0, 0.0),
            true,
        ),
        backdrop_orbiter(
            "waystation_hauler_b",
            "Hauler Kettle",
            Meters3::new(-1_400.0, 0.0, 0.0),
            true,
        ),
        backdrop_beacon(
            "waystation_dock_a",
            "DOCK-A",
            Meters3::new(1_700.0, -250.0, 600.0),
            Color::srgb(1.0, 0.7, 0.2),
        ),
        backdrop_beacon(
            "waystation_dock_b",
            "DOCK-B",
            Meters3::new(1_500.0, -300.0, -900.0),
            Color::srgb(1.0, 0.7, 0.2),
        ),
        backdrop_beacon(
            "waystation_traffic",
            "TRAFFIC",
            Meters3::new(-1_800.0, -200.0, 400.0),
            Color::srgb(0.3, 0.9, 1.0),
        ),
    ];
    objects.extend(backdrop_rig("waystation").objects());

    // The shipping lane: a flatter, slightly denser band than menu_ambience's
    // ring, same safety floor (inner past any plausible geometric radius,
    // whole band below the orbit plane).
    let lane_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "waystation_cargo_".to_string(),
        count: 18,
        seed: SCATTER_SEED ^ 0x1,
        region: ScatterRegion::Ring {
            center: Meters3::ZERO,
            inner: Meters(1_800.0),
            outer: Meters(2_300.0),
            y_min: Meters(-600.0),
            y_max: Meters(-250.0),
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "waystation_cargo_".to_string(),
                name: "Cargo Rock".to_string(),
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
        asteroid_radius: Some((Meters(10.0), Meters(25.0))),
        min_separation: None,
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
                // The scene poses its own camera: a fixed mid-range shot on
                // the planetoid (the old well-derived pose averaged ~here;
                // the noise mesh runs to ~1.2 km, safely inside the frame).
                .chain([
                    // HELD at 3,350 m: the planetoid at the origin IS the
                    // shot, and the traffic works its flanks at +-1.4..1.8 km
                    // - so the near arc is already inside the rolloff and the
                    // far arc cannot be brought in without putting the camera
                    // through the rock. Distance is doing the right thing here.
                    backdrop_camera(Meters3::new(0.0, 1_000.0, 3_350.0)),
                    lane_scatter,
                    // The carousel's rotation limit: the waystation's day
                    // never ends on its own, so after a couple of freight
                    // laps the menu turns to the next backdrop.
                    EventActionConfig::TimerStart(TimerStartActionConfig {
                        key: "waystation_rotate".to_string(),
                        seconds: crate::scenario_helpers::number(150.0),
                    }),
                ])
                .collect::<_>(),
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "waystation_rotate".to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "menu_gauntlet".to_string(),
                linger: false,
                delay: Some(1.0),
            })],
        },
    ];

    ScenarioConfig {
        description: "A freight waystation going about its day.".to_string(),
        hidden: true,
        menu_backdrop: true,
        events,
        ..ScenarioConfig::new(
            "menu_waystation".to_string(),
            "Waystation Traffic".to_string(),
            cubemap,
        )
    }
}
