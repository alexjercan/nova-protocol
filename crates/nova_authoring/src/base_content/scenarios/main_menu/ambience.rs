//! The planetoid-orbiter main-menu backdrop.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::{backdrop_rig, planetoid_glow};
use crate::base_content::{scenarios::SCATTER_SEED, ships};

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
            sections: ships::racer_sections(ships::ShipGrade::Player, vec![]),
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
        let catalog = crate::base_content::sections::section_catalog(
            &crate::base_content::assets::BaseContentAssets::from_paths(),
        );
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
