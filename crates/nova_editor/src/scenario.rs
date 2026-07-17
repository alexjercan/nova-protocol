//! The scenario the editor hands off to on Play. Baseline slice (task
//! 20260714-204219): an asteroid field with a single large PLANETOID backdrop
//! and the PLAYER ship only - no enemy, no objective. The enemy ship, the
//! destroy objective, and richer authoring live in "the rest" (task
//! 20260714-081703).

use bevy::prelude::*;
use nova_assets::prelude::*;
use nova_gameplay::prelude::AssetRef;
use nova_scenario::prelude::*;
use rand::prelude::*;

use crate::config::PlayerSpaceshipConfig;

pub(crate) fn setup_scenario(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    player_config: Res<PlayerSpaceshipConfig>,
) {
    commands.trigger(LoadScenario(test_scenario(&game_assets, &player_config)));
}

/// Build the sandbox scenario: a scattered asteroid field, one big planetoid as
/// a backdrop/gravity well, and the player's built ship. Deliberately
/// combat-free - the sandbox is for building and flying, not fighting.
pub(crate) fn test_scenario(
    game_assets: &GameAssets,
    player_config: &PlayerSpaceshipConfig,
) -> ScenarioConfig {
    let objects = sandbox_objects(player_config, game_assets.asteroid_texture.clone());

    ScenarioConfig {
        id: "test_scenario".to_string(),
        name: "Test Scenario".to_string(),
        description: "A sandbox scenario: an asteroid field and a planetoid.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events: sandbox_events(objects),
        ..Default::default()
    }
}

/// The scenario objects: a random asteroid field, one big invulnerable planetoid
/// (a large asteroid - every asteroid mesh is a PlanetHeight-displaced sphere,
/// see nova_scenario::objects::asteroid) as a backdrop/gravity well, and the
/// player's built ship. No enemy ship - the sandbox is combat-free.
fn sandbox_objects(
    player_config: &PlayerSpaceshipConfig,
    asteroid_texture: Handle<Image>,
) -> Vec<ScenarioObjectConfig> {
    let mut rng = rand::rng();
    let mut objects = Vec::new();

    for i in 0..20 {
        let pos = Vec3::new(
            rng.random_range(-100.0..100.0),
            rng.random_range(-20.0..20.0),
            rng.random_range(-100.0..100.0),
        );
        let radius = rng.random_range(1.0..3.0);

        objects.push(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: format!("asteroid_{}", i),
                name: format!("Asteroid {}", i),
                position: pos,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                // DIRECT paths, not dep://: the editor sandbox is built at
                // runtime outside the mod merge (its texture is a raw
                // GameAssets handle), so scheme refs would never rewrite.
                impact_sound: Some(AssetRef::from("base/sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("base/sounds/explosion.wav")),
                radius,
                texture: AssetRef::from(asteroid_texture.clone()),
                health: 100.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        });
    }

    // The planetoid: a large, invulnerable asteroid with an explicit surface
    // gravity so it reads as a proper well and as scenery rather than a target.
    // Parked below and behind the field, well clear of where the player spawns.
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "planetoid".to_string(),
            name: "Planetoid".to_string(),
            position: Vec3::new(80.0, -90.0, -240.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            // DIRECT paths, not dep://: the editor sandbox is built at
            // runtime outside the mod merge (its texture is a raw
            // GameAssets handle), so scheme refs would never rewrite.
            impact_sound: Some(AssetRef::from("base/sounds/impact.wav")),
            destroy_sound: Some(AssetRef::from("base/sounds/explosion.wav")),
            radius: 55.0,
            texture: AssetRef::from(asteroid_texture),
            health: 100.0,
            surface_gravity: Some(40.0),
            invulnerable: true,
            lock_signature: None,
        }),
    });

    let player_spaceship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: player_config
                .inputs
                .iter()
                .map(|(entity, key)| (entity.to_string(), key.clone()))
                .collect(),

            speed_cap: None,
            // The editor sandbox keeps normal finite magazines.
            infinite_ammo: false,
            lock_refire_secs: None,
        }),
        sections: player_config.sections.values().cloned().collect(),
    };
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_spaceship".to_string(),
            name: "Player's Spaceship".to_string(),
            position: Vec3::new(0.0, 0.0, 50.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(player_spaceship),
    });

    objects
}

/// The scenario events: spawn every object on start, and a debug message if the
/// player is destroyed. No objective wiring - that (and the enemy) is "the rest".
fn sandbox_events(objects: Vec<ScenarioObjectConfig>) -> Vec<ScenarioEventConfig> {
    vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .collect::<_>(),
        },
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some("player_spaceship".to_string()),
                type_name: None,
                ..default()
            })],
            actions: vec![EventActionConfig::DebugMessage(DebugMessageActionConfig {
                message: "The player's spaceship was destroyed!".to_string(),
            })],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The baseline sandbox is combat-free: an asteroid field plus one
    /// invulnerable planetoid and the player only. This would fail against the
    /// pre-rework scenario, which spawned an "other_spaceship" enemy.
    #[test]
    fn sandbox_has_a_planetoid_and_no_enemy_ship() {
        let objects = sandbox_objects(&PlayerSpaceshipConfig::default(), Handle::default());

        let ids: Vec<&str> = objects.iter().map(|o| o.base.id.as_str()).collect();
        assert!(
            !ids.contains(&"other_spaceship"),
            "the sandbox must not spawn the enemy ship"
        );
        assert!(ids.contains(&"player_spaceship"), "the player still spawns");
        assert_eq!(
            ids.iter().filter(|id| id.starts_with("asteroid_")).count(),
            20,
            "the asteroid field is intact"
        );

        // Exactly one planetoid, and it is a large invulnerable gravity well.
        let planetoids: Vec<&ScenarioObjectConfig> = objects
            .iter()
            .filter(|o| o.base.id == "planetoid")
            .collect();
        assert_eq!(planetoids.len(), 1, "one planetoid backdrop");
        match &planetoids[0].kind {
            ScenarioObjectKind::Asteroid(a) => {
                assert!(a.invulnerable, "the planetoid is scenery, not a target");
                assert!(a.radius >= 40.0, "the planetoid is large");
                assert_eq!(
                    a.surface_gravity,
                    Some(40.0),
                    "the planetoid is an explicit gravity well"
                );
            }
            other => panic!("planetoid should be an asteroid, got {other:?}"),
        }
    }

    /// No objective wiring in the baseline: the old scenario set a
    /// "destroy the other spaceship" objective and completed it on the enemy's
    /// death. Both must be gone.
    #[test]
    fn sandbox_has_no_objective_wiring() {
        let objects = sandbox_objects(&PlayerSpaceshipConfig::default(), Handle::default());
        let events = sandbox_events(objects);

        let has_objective_action = events.iter().flat_map(|e| &e.actions).any(|a| {
            matches!(
                a,
                EventActionConfig::Objective(_) | EventActionConfig::ObjectiveComplete(_)
            )
        });
        assert!(
            !has_objective_action,
            "the sandbox must not wire any objective in the baseline"
        );
    }
}
