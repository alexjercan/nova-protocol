//! The torpedo-gauntlet main-menu backdrop: a lone racer holds its orbit
//! while an off-screen battery feeds torpedoes into its point defense.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::{backdrop_planetoid, backdrop_rig, planetoid_glow};
use crate::base_content::{scenarios::SCATTER_SEED, ships};

/// Where the racer respawns and anchors its leash.
const SHIP_SPAWN: Vec3 = Vec3::new(140.0, 0.0, 0.0);
/// The off-screen battery position. Three constraints meet here: the ship's
/// PASSIVE re-engage pull reaches 800 u, so the battery must stay beyond
/// 800 u of every point on the orbit ring (ring radius ~120-131 u across
/// planetoid seeds -> keep the battery past ~950 u) or the racer abandons
/// its orbit to charge it; the AI torpedo envelope tops out at 1000 u, so
/// the near side of the ring must stay inside that; and the menu panel
/// owns the frame's RIGHT half, so the battery parks on -X - inbound
/// torpedoes and their intercepts cross the open LEFT half of the shot
/// instead of sliding in behind the UI. The camera frames roughly +-300 u
/// around the planetoid, so the park itself is far off-screen.
const BATTERY_POS: Vec3 = Vec3::new(-950.0, 0.0, 0.0);

/// A living point-defense drill behind the menu: the racer (player-grade
/// PDC turrets) flies the proven orbit directive while a thrusterless
/// torpedo battery, parked outside the frame, launches standard torpedoes
/// at it. AI point defense runs in every behavior state, so the racer
/// swats the inbound ordnance mid-frame without ever leaving its ring. If
/// one leaks through and the racer dies, a timer respawns it - the scene
/// re-arms itself instead of decaying into an empty shot.
pub(crate) fn menu_gauntlet(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut stage = Vec::new();

    // The camera contract: the planetoid well `stage_menu_camera` frames.
    stage.push(backdrop_planetoid(asteroid_texture.clone(), 45_000.0));
    stage.extend(backdrop_rig("gauntlet").objects());
    stage.push(planetoid_glow("gauntlet_lamp"));

    // The battery: a Spaceship in kind only - two standard torpedo bays and
    // nothing else. No thrusters (it holds its park outside the planetoid's
    // 424 u SOI), no controller (nothing swings the hull; the authored
    // rotation is the aim), no turrets. Default AI allegiance (Enemy) makes
    // the Player-allegiance racer its one hostile; ships spawn in the
    // Engage state and combat states hold on ANY acquired target (<=2000 u),
    // so it keeps launching from beyond the racer's passive 800 u pull. The
    // torpedo launch gate needs hull-forward within 60 degrees of the
    // target: -90 degrees about Y points the hull's -Z down +X, at the ring.
    stage.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "gauntlet_battery".to_string(),
            name: "Gauntlet Battery".to_string(),
            position: BATTERY_POS,
            rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::AI(AIControllerConfig {
                // The long watch: the battery idles whenever its target is
                // gone (a respawn gap leaves it passive), and a passive ship
                // only re-engages inside its detection range. The ring sits
                // 819-1081 u out, so the default 800 would leave the battery
                // dozing forever; the widened range is what re-arms the
                // scene after every racer respawn - while the RACER's
                // default range is exactly what keeps it from being pulled
                // toward the battery in return.
                engage_range: Some(1600.0),
                ..Default::default()
            }),
            sections: vec![
                SpaceshipSectionConfig {
                    id: "bay_a".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Prototype("torpedo_section".to_string()),
                    modifications: vec![],
                },
                SpaceshipSectionConfig {
                    id: "bay_b".to_string(),
                    position: Vec3::X,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Prototype("torpedo_section".to_string()),
                    modifications: vec![],
                },
            ],
        }),
    });

    // Depth dressing, below the orbit plane like every backdrop ring (and
    // below the torpedo lane, which runs at y ~ 0 from the battery to the
    // ring - the rocks cannot block launches or eat torpedoes).
    let rock_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "gauntlet_rock_".to_string(),
        count: 16,
        seed: SCATTER_SEED ^ 0x3,
        region: ScatterRegion::Ring {
            center: Vec3::ZERO,
            inner: 170.0,
            outer: 240.0,
            y_min: -70.0,
            y_max: -30.0,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "gauntlet_rock_".to_string(),
                name: "Gauntlet Rock".to_string(),
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
        min_separation: None,
    });

    let spawn_ship = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "gauntlet_ship".to_string(),
            name: "Gauntlet Racer".to_string(),
            position: SHIP_SPAWN,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            // Player allegiance is what makes the Enemy battery hostile -
            // AI-vs-AI combat needs one side on the player's team.
            allegiance: Some(Allegiance::Player),
            controller: SpaceshipController::AI(AIControllerConfig {
                orbit: Some("menu_planetoid".to_string()),
                // The arrival grace lands the ship on its ROUTINE: ships
                // spawn in the Engage state and hold on any acquired target,
                // so an ungraced spawn would charge the battery instead of
                // taking up its orbit.
                engage_delay: Some(2.0),
                // If a torpedo leaks through, the hit overrides the passive
                // gate and the racer lunges toward the battery; the leash
                // walks it back onto the ring once the damage memory fades.
                leash: Some(250.0),
                ..Default::default()
            }),
            // Player-grade racer: the full 100-rounds/s PDC turrets are the
            // whole point of the scene.
            sections: ships::racer_sections(ships::ShipGrade::Player, vec![]),
        }),
    });

    let events = vec![
        // One spawn site per ship id: OnStart only arms the respawn timer,
        // and the SAME timer handler performs the first spawn and every
        // later one (spawning the same id from two handlers is a lint warn).
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: stage
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain([
                    rock_scatter,
                    EventActionConfig::TimerStart(TimerStartActionConfig {
                        key: "gauntlet_respawn".to_string(),
                        seconds: crate::scenario_helpers::number(0.5),
                    }),
                ])
                .collect(),
        },
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "gauntlet_respawn".to_string(),
            })],
            actions: vec![spawn_ship],
        },
        // The re-arm: a leaked torpedo (or a lucky blast) killing the racer
        // must not leave the menu staring at an empty ring. Expired timer
        // keys are removed before dispatch, so a handler restarting its own
        // key is a legal self-restarting loop.
        ScenarioEventConfig {
            name: EventConfig::OnDefeated,
            filters: vec![crate::scenario_helpers::entity("gauntlet_ship")],
            actions: vec![
                crate::scenario_helpers::despawn_object("gauntlet_ship"),
                EventActionConfig::TimerStart(TimerStartActionConfig {
                    key: "gauntlet_respawn".to_string(),
                    seconds: crate::scenario_helpers::number(8.0),
                }),
            ],
        },
    ];

    ScenarioConfig {
        description: "A lone racer's point defense against an off-screen torpedo battery."
            .to_string(),
        hidden: true,
        menu_backdrop: true,
        events,
        ..ScenarioConfig::new(
            "menu_gauntlet".to_string(),
            "Torpedo Gauntlet".to_string(),
            cubemap,
        )
    }
}
