//! The freight-waystation main-menu backdrop.

use bevy::prelude::*;
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
            Vec3::new(140.0, 0.0, 0.0),
            true,
        ),
        backdrop_orbiter(
            "waystation_hauler_b",
            "Hauler Kettle",
            Vec3::new(-140.0, 0.0, 0.0),
            true,
        ),
        backdrop_beacon(
            "waystation_dock_a",
            "DOCK-A",
            Vec3::new(170.0, -25.0, 60.0),
            Color::srgb(1.0, 0.7, 0.2),
        ),
        backdrop_beacon(
            "waystation_dock_b",
            "DOCK-B",
            Vec3::new(150.0, -30.0, -90.0),
            Color::srgb(1.0, 0.7, 0.2),
        ),
        backdrop_beacon(
            "waystation_traffic",
            "TRAFFIC",
            Vec3::new(-180.0, -20.0, 40.0),
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
            center: Vec3::ZERO,
            inner: 180.0,
            outer: 230.0,
            y_min: -60.0,
            y_max: -25.0,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "waystation_cargo_".to_string(),
                name: "Cargo Rock".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
                radius: 1.0,
                texture: asteroid_texture,
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.0, 2.5)),
        min_separation: None,
    });

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                // The scene poses its own camera: a fixed mid-range shot on
                // the planetoid (the old well-derived pose averaged ~here;
                // the noise mesh runs to ~120 u, safely inside the frame).
                .chain([
                    backdrop_camera(Vec3::new(0.0, 100.0, 335.0)),
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
            name: EventConfig::OnTimerEnd,
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
