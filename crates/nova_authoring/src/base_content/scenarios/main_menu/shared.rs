//! Shared stage builders for private main-menu backdrop scenarios.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use crate::base_content::ships;

pub(super) fn backdrop_planetoid(
    asteroid_texture: AssetRef<Image>,
    mass: f32,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "menu_planetoid".to_string(),
            name: "Menu Planetoid".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
            destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
            radius: 20.0,
            texture: asteroid_texture,
            health: 2000.0,
            mass: Some(mass),
            invulnerable: true,
            lock_signature: None,
        }),
    }
}

/// A small AI ship on the orbit directive around the backdrop planetoid -
/// the proven menu actor (the ORBIT autopilot plans its ring from the
/// well's runtime geometry). `extra_hull` adds a mid hull segment for a
/// longer, hauler-ish silhouette. Never `SpaceshipController::Player`: the
/// spaceship input sets are LIVE in MainMenu (see menu_ambience's warning).
pub(super) fn backdrop_orbiter(
    id: &str,
    name: &str,
    position: Vec3,
    // The hauler silhouette knob: `true` flies the wide unarmed cargoa (the
    // waystation freighters), `false` the racer (the scrapyard tug).
    cargo: bool,
) -> ScenarioObjectConfig {
    let sections = if cargo {
        ships::cargoa_sections()
    } else {
        ships::racer_sections(ships::ShipGrade::Player, vec![])
    };
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::AI(AIControllerConfig {
                orbit: Some("menu_planetoid".to_string()),
                ..Default::default()
            }),
            sections,
        }),
    }
}

/// The shared backdrop rig: the standard three-point key/rim/fill, aimed at the
/// planetoid every menu scene frames, scaled to the backdrop's ~200u stage.
///
/// Every backdrop carries one - deleting the engine's hardcoded key light made
/// lighting authored content, and a menu scene that authors none renders black.
pub(super) fn backdrop_rig(prefix: &str) -> ThreePointRig {
    ThreePointRig::around(prefix, Vec3::ZERO, 20.0)
}

/// A warm positional lamp just off the planetoid's limb: the falloff a
/// directional light cannot give, so near dressing reads brighter than far.
pub(super) fn planetoid_glow(id: &str) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: "Planetoid Glow".to_string(),
            position: Vec3::new(-60.0, 20.0, 90.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Light(LightConfig::Point {
            // Lumens at backdrop scale: the lamp sits ~110u from the planetoid
            // and must still register against an 11000 lux key.
            intensity: 2_500_000.0,
            range: 400.0,
            radius: 12.0,
            color: Color::srgb(1.0, 0.82, 0.6),
            shadows: false,
        }),
    }
}

/// A static dressing beacon (label + warm little light). Below the orbit
/// plane and outside the planetoid's geometric radius, like everything
/// else in a backdrop.
pub(super) fn backdrop_beacon(
    id: &str,
    label: &str,
    position: Vec3,
    color: Color,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: label.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: label.to_string(),
            radius: 2.0,
            color,
            area_radius: None,
            lock_signature: None,
        }),
    }
}
