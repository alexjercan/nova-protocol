//! The salvage-yard main-menu backdrop.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::*;
use crate::base_content::scenarios::SCATTER_SEED;

pub(crate) fn menu_scrapyard(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut objects = vec![
        backdrop_planetoid(asteroid_texture.clone(), 45_000.0),
        backdrop_orbiter(
            "scrapyard_tug",
            "Tug Pebble",
            Vec3::new(0.0, 0.0, 140.0),
            false,
        ),
        // Two derelict hulks: plain big rocks at fixed spots, breakable (they
        // are dressing; nothing in the backdrop shoots).
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "scrapyard_wreck_a".to_string(),
                name: "Wreck".to_string(),
                position: Vec3::new(200.0, -18.0, 80.0),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
                radius: 6.0,
                texture: asteroid_texture.clone(),
                health: 400.0,
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "scrapyard_wreck_b".to_string(),
                name: "Wreck".to_string(),
                position: Vec3::new(-180.0, -30.0, -120.0),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
                radius: 8.0,
                texture: asteroid_texture.clone(),
                health: 400.0,
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        backdrop_beacon(
            "scrapyard_marker",
            "YARD",
            Vec3::new(165.0, -12.0, -55.0),
            Color::srgb(1.0, 0.55, 0.15),
        ),
    ];
    objects.extend(backdrop_rig("scrapyard").objects());
    // The yard's work lamp: warm falloff over the drifting crate band, which a
    // parallel-ray rig alone leaves reading uniformly lit.
    objects.push(planetoid_glow("scrapyard_lamp"));

    // The drifting cargo: on-rails salvage crates (visual tumble, no
    // physics), scattered in the same safe band as the rocks would be.
    let crate_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "scrapyard_crate_".to_string(),
        count: 10,
        seed: SCATTER_SEED ^ 0x2,
        region: ScatterRegion::Ring {
            center: Vec3::ZERO,
            inner: 150.0,
            outer: 210.0,
            y_min: -50.0,
            y_max: -10.0,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "scrapyard_crate_".to_string(),
                name: "Drifting Crate".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::SalvageCrate(SalvageCrateConfig {
                size: 2.0,
                area_radius: 3.0,
                pickup_sound: Some(AssetRef::from("self://sounds/salvage_pickup.wav")),
            }),
        },
        asteroid_radius: None,
        min_separation: None,
    });

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .chain([crate_scatter])
            .collect::<_>(),
    }];

    ScenarioConfig {
        description: "A quiet salvage yard tumbling in orbit.".to_string(),
        hidden: true,
        menu_backdrop: true,
        events,
        ..ScenarioConfig::new(
            "menu_scrapyard".to_string(),
            "Scrapyard Drift".to_string(),
            cubemap,
        )
    }
}
