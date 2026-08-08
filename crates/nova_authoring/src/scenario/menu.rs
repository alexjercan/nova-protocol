//! The main menu's living-backdrop scenarios.
//!
//! Ambience scenes shown behind the menu - no player, no objectives - each a
//! planetoid with a real gravity well and AI ships flying thruster/RCS orbits
//! around it. Extracted from the scenario catalog into their own module (like
//! the campaign scenarios `broadside`/`shakedown`) so the menu backdrops live
//! together. Registered in `nova_assets/src/lib.rs`; the shared scatter seed
//! stays in the parent module.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::{craft, SCATTER_SEED};

/// The main menu's living backdrop: a big planetoid with a real gravity well, a
/// scatter of rocks, and one AI ship flying a thruster-driven orbit around the
/// planetoid (orbit directive). No player, no objectives, no areas - the scene
/// exists to be looked at.
pub(crate) fn menu_ambience(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut objects = Vec::new();

    // The stage: a nominally-20u planetoid at the origin carrying the default
    // mass. That mass alone fixes the pull and the SOI (424u); the GEOMETRIC
    // collider radius (observed ~80-91u across seeds;
    // insert_asteroid_gravity_well) only says where the surface clamp starts,
    // so the orbiter below sees the same well on every seed.
    objects.push(ScenarioObjectConfig {
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
            texture: asteroid_texture.clone(),
            health: 2000.0,
            mass: Some(45_000.0),
            invulnerable: true,
            lock_signature: None,
        }),
    });

    // A loose ring of small rocks for depth, kept strictly out of harm's way:
    // the planetoid's GEOMETRIC radius runs several times its nominal 20u
    // (observed ~80-91u across seeds), and rocks that spawn inside that mesh
    // get penetration-resolved with impulses whose collision damage destroyed
    // the planetoid (and its gravity well) within a second - twice, in two
    // different ring layouts. So the ring starts past any plausible geometric
    // radius AND sits below the orbit plane (the orbiter circles at y=0 at
    // roughly body_radius + 40), keeping it clear of the orbit across collider
    // seeds (worst-case clearance is on the order of 10u, not unbounded - if
    // the planetoid's nominal radius grows, regrow this ring floor with it).
    //
    // This is now a single seeded ScatterObjects action (below, in the OnStart
    // event) rather than a per-launch RNG loop: the layout is deterministic
    // content, reproducible across loads.
    let menu_rock_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "menu_rock_".to_string(),
        count: 14,
        seed: SCATTER_SEED,
        region: ScatterRegion::Ring {
            center: Vec3::ZERO,
            inner: 170.0,
            outer: 240.0,
            y_min: -70.0,
            y_max: -30.0,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "menu_rock_".to_string(),
                name: "Menu Rock".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
                radius: 1.0,
                texture: asteroid_texture.clone(),
                health: 100.0,
                mass: None,
                invulnerable: false,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.0, 3.0)),
        min_separation: None,
    });

    // The scene's own lighting. There is no engine light any more, so a backdrop
    // that authors nothing renders black. Scale 20 puts the rig ~190u out, well
    // clear of the planetoid's geometric radius - cosmetic for a directional
    // light, but it keeps the numbers readable as a physical rig.
    objects.extend(backdrop_rig("menu").objects());
    // The lamp the rig cannot be: a warm positional glow riding just off the
    // planetoid's limb, so the scatter ring falls off with distance instead of
    // reading uniformly lit. Also the shipped proof that Point lights work.
    objects.push(planetoid_glow("menu_lamp"));

    // The actor: an AI ship directed to orbit the planetoid on its own
    // thrusters - the ORBIT autopilot plans its ring from the well's runtime
    // geometry, so no staging math lives here or in nova_menu. It spawns
    // comfortably outside the planetoid's geometric surface (the noise mesh
    // reaches several times past the nominal 20u) and inside its SOI, and flies
    // itself in from there. WARNING: the spaceship input/section sets ARE live
    // in MainMenu - this scenario is a loaded scenario like any other
    // (scenario_is_live gating, nova_scenario) - so keep ambience ships off
    // SpaceshipController::Player.
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "menu_orbiter".to_string(),
            name: "Menu Orbiter".to_string(),
            position: Vec3::new(140.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::AI(AIControllerConfig {
                orbit: Some("menu_planetoid".to_string()),
                ..Default::default()
            }),
            // The menu orbiter flies the racer (craft-ships-into-base) - a
            // detailed silhouette drifting the backdrop reads far cooler than the
            // old trainer cube.
            sections: craft::racer_sections(craft::ShipGrade::Player, vec![]),
        }),
    });

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .chain([menu_rock_scatter])
            .collect::<_>(),
    }];

    ScenarioConfig {
        description: "The main menu's living backdrop.".to_string(),
        // The menu backdrop is never a player-facing scenario (hidden from
        // the picker) but IS in the menu's backdrop rotation (menu_backdrop):
        // the menu picks one flagged scenario at random on entry.
        hidden: true,
        menu_backdrop: true,
        events,
        ..ScenarioConfig::new(
            "menu_ambience".to_string(),
            "Menu Ambience".to_string(),
            cubemap,
        )
    }
}

/// The shared backdrop stage: the camera-framing planetoid every menu
/// backdrop must carry (id `menu_planetoid` - the contract
/// `stage_menu_camera` frames by; see the scenario authoring guide). Nominal
/// 20u, invulnerable, with the caller's authored mass parameter - which sets
/// both the pull and the SOI (the GEOMETRIC collider radius, observed ~80-91u
/// across seeds, only bounds the surface clamp). The mass is per-scene: the
/// waystation carries two heavy haulers and needs a lighter pull to hold its
/// orbit (30 000: 1.9-3.8 u/s^2 at the surface, SOI 346u), while the
/// single-ship ambience/scrapyard scenes take the default 45 000 (2.9-5.7
/// u/s^2, SOI 424u).
fn backdrop_planetoid(asteroid_texture: AssetRef<Image>, mass: f32) -> ScenarioObjectConfig {
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
fn backdrop_orbiter(
    id: &str,
    name: &str,
    position: Vec3,
    // The hauler silhouette knob: `true` flies the wide unarmed cargoa (the
    // waystation freighters), `false` the racer (the scrapyard tug).
    cargo: bool,
) -> ScenarioObjectConfig {
    let sections = if cargo {
        craft::cargoa_sections()
    } else {
        craft::racer_sections(craft::ShipGrade::Player, vec![])
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
fn backdrop_rig(prefix: &str) -> ThreePointRig {
    ThreePointRig::around(prefix, Vec3::ZERO, 20.0)
}

/// A warm positional lamp just off the planetoid's limb: the falloff a
/// directional light cannot give, so near dressing reads brighter than far.
fn planetoid_glow(id: &str) -> ScenarioObjectConfig {
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
fn backdrop_beacon(id: &str, label: &str, position: Vec3, color: Color) -> ScenarioObjectConfig {
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

/// Menu backdrop two: a freight waystation going about its day. Two named
/// haulers circle the planetoid in convoy (opposite phases on the same
/// autopilot ring, so they chase and never meet), dock beacons glow below the
/// lane, and a flat band of cargo rocks reads as the shipping lane. Same safety
/// envelope as menu_ambience: everything static sits past the geometric radius
/// and below the orbit plane.
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
                health: 100.0,
                mass: None,
                invulnerable: false,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.0, 2.5)),
        min_separation: None,
    });

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .chain([lane_scatter])
            .collect::<_>(),
    }];

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

/// Menu backdrop three: a quiet salvage yard. One lazy tug circles the
/// planetoid, salvage crates tumble in a drifting band (crates are on-rails
/// statics with a render-child tumble - zero physics risk), two big wreck rocks
/// anchor the yard, and a single warm beacon marks it. Cozy, not busy.
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

#[cfg(test)]
mod tests {
    use nova_ship::prelude::*;

    use super::*;

    /// The menu backdrop's contract: the orbiter is an AI ship directed to
    /// orbit the planetoid on its own thrusters - controller + thruster
    /// sections aboard, directive pointing at an object that actually exists in
    /// the same scenario and carries an authored surface gravity (so it gets a
    /// well at spawn).
    #[test]
    fn menu_orbiter_is_an_ai_ship_directed_at_the_planetoid() {
        let scenario = menu_ambience(AssetRef::default(), AssetRef::default());

        let spawns: Vec<_> = scenario
            .events
            .iter()
            .flat_map(|event| event.actions.iter())
            .filter_map(|action| match action {
                EventActionConfig::SpawnScenarioObject(object) => Some(object),
                _ => None,
            })
            .collect();

        let orbiter = spawns
            .iter()
            .find(|object| object.base.id == "menu_orbiter")
            .expect("the backdrop spawns the orbiter");
        let ScenarioObjectKind::Spaceship(ship) = &orbiter.kind else {
            panic!("the orbiter is a spaceship");
        };
        let SpaceshipController::AI(ai) = &ship.controller else {
            panic!("the orbiter is AI-controlled, got {:?}", ship.controller);
        };
        assert_eq!(
            ai.orbit.as_deref(),
            Some("menu_planetoid"),
            "the directive targets the planetoid"
        );
        // The orbiter flies the racer now, whose section prototypes are named by
        // cut-cube id, so resolve each ref's KIND against the base catalog.
        let catalog =
            crate::sections::build_sections(&crate::sections::SectionMeshRefs::from_paths());
        let has_kind = |want: fn(&SectionKind) -> bool| {
            ship.sections.iter().any(|section| match &section.source {
                SectionSource::Prototype(id) => catalog
                    .iter()
                    .find(|c| c.base.id == *id)
                    .is_some_and(|c| want(&c.kind)),
                SectionSource::Inline(c) => want(&c.kind),
            })
        };
        assert!(
            has_kind(|k| matches!(k, SectionKind::Controller(_))),
            "a controller section flies the autopilot's attitude commands"
        );
        assert!(
            has_kind(|k| matches!(k, SectionKind::Thruster(_))),
            "a thruster section provides the burn"
        );

        // The directive's target exists and gets a gravity well at spawn
        // (authored surface gravity), so the ORBIT autopilot can engage.
        let planetoid = spawns
            .iter()
            .find(|object| object.base.id == "menu_planetoid")
            .expect("the backdrop spawns the planetoid the directive names");
        let ScenarioObjectKind::Asteroid(rock) = &planetoid.kind else {
            panic!("the planetoid is an asteroid body");
        };
        assert!(
            rock.mass.is_some(),
            "an authored mass is what spawns the planetoid's well"
        );
    }
}
