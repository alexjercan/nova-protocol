//! The duel-cycle main-menu backdrop: two racers fight, the winner is
//! erased by a siege torpedo, and after a beat two fresh ships fly in.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::{backdrop_planetoid, backdrop_rig, planetoid_glow};
use crate::{
    base_content::{scenarios::SCATTER_SEED, ships},
    scenario_helpers::{despawn_object, entity, number},
};

/// The finisher battery's park: far outside the ~+-240 u camera frame, the
/// planetoid's 424 u SOI (it has no thrusters to fight the pull with), AND
/// the winner's default 800 u detection pull - the victor must never break
/// its victory lap to charge the battery. The battery's own widened
/// `engage_range` is what lets it wake from this distance once flipped
/// hostile; the arena stays inside the 1000 u torpedo launch envelope.
const BATTERY_POS: Vec3 = Vec3::new(-950.0, 0.0, 0.0);
/// The altitude the whole duel is staged at. Engage-state flight has NO
/// obstacle avoidance (only passive patrol legs do), and the first cut of
/// this scene proved it: two ships entering from +-420 on the y=0 plane
/// chase along a line straight through the planetoid, pin themselves
/// against its surface, and stall the fight forever with the line-of-fire
/// gate holding both triggers. The planetoid's geometric radius runs to
/// ~91 u across seeds; a fight plane at 110 keeps every chase line - and
/// the head-on entrance line - clear of the rock, floating the duel above
/// the planetoid in the frame.
const DUEL_PLANE_Y: f32 = 110.0;
/// Off-screen entrances, one per side.
const VICTOR_SPAWN: Vec3 = Vec3::new(-420.0, DUEL_PLANE_Y, 100.0);
const RIVAL_SPAWN: Vec3 = Vec3::new(420.0, DUEL_PLANE_Y, -100.0);

/// One duelist: a racer that flies in from off-screen onto an in-frame
/// patrol triangle. The arrival grace holds the entrance (ships spawn in
/// the Engage state and hold on ANY acquired target, so an ungraced spawn
/// would turn and burn from the spawn point instead of flying in); the
/// leash, anchored on the patrol centroid near the planetoid, keeps the
/// fight from drifting out of frame.
fn duelist(
    id: &str,
    name: &str,
    spawn: Vec3,
    patrol: [Vec3; 3],
    grade: ships::ShipGrade,
    allegiance: Option<Allegiance>,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position: spawn,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance,
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: patrol.to_vec(),
                leash: Some(300.0),
                engage_delay: Some(6.0),
                ..Default::default()
            }),
            sections: ships::racer_sections(grade, vec![]),
        }),
    }
}

/// A repeating three-act battle behind the menu. Act one: a player-grade
/// racer (Player allegiance) and an enemy-grade racer fly in from opposite
/// sides and fight; the gun gap makes the outcome all but certain. Act two:
/// the rival's defeat starts a short beat, then the off-screen siege
/// battery goes hostile and feeds armored ship-killing torpedoes at the
/// winner until one connects - point defense hammers the ordnance and
/// loses. Act three: the stage is cleared, the battery stands down, and a
/// timer flies in the next pair. Every ship id has exactly ONE spawning
/// handler (the respawn timer), so the cycle repeats forever without
/// duplicate-spawn lint noise.
pub(crate) fn menu_duel(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut stage = Vec::new();

    // A LIGHT planetoid: the camera contract only needs the well's id and
    // geometry, but its pull is real - at the default 45 000 the arena
    // (110-180 u from the center) sits deep inside the 424 u SOI, and two
    // dogfighting ships under a constant ~2 u/s^2 drag spiral onto the rock
    // and pin there (the first cut of this scene). 6 000 shrinks the SOI to
    // ~155 u, leaving the duel plane on its edge where the drift is noise.
    stage.push(backdrop_planetoid(asteroid_texture.clone(), 6_000.0));
    stage.extend(backdrop_rig("duel").objects());
    stage.push(planetoid_glow("duel_lamp"));

    // The finisher: one siege bay, nothing else. Neutral until scripted
    // hostile - a Neutral ship never acquires and is never point-defensed,
    // so it is scenery until the flip. No controller section: the authored
    // rotation (-Z toward the arena, within the launch gate's 60 degrees)
    // IS the aim, and nothing can knock it off it.
    stage.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "duel_finisher".to_string(),
            name: "Duel Finisher".to_string(),
            position: BATTERY_POS,
            rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Neutral),
            controller: SpaceshipController::AI(AIControllerConfig {
                // Sitting Neutral, the battery is passive (Idle) when the
                // allegiance flip lands; a passive ship only wakes for a
                // hostile inside its detection range, and the arena runs
                // 810-1092 u out - past the 800 u default. The widened range
                // is the whole trick: the battery sees the winner, the
                // winner's default range never sees far enough to charge
                // the battery back.
                engage_range: Some(1600.0),
                ..Default::default()
            }),
            sections: vec![SpaceshipSectionConfig {
                id: "siege_bay".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("heavy_torpedo_section".to_string()),
                modifications: vec![],
            }],
        }),
    });

    // Sparse dressing ring, below the y=0 fight plane.
    let rock_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "duel_rock_".to_string(),
        count: 12,
        seed: SCATTER_SEED ^ 0x5,
        region: ScatterRegion::Ring {
            center: Vec3::ZERO,
            inner: 175.0,
            outer: 235.0,
            y_min: -70.0,
            y_max: -35.0,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "duel_rock_".to_string(),
                name: "Duel Rock".to_string(),
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

    let spawn_victor = EventActionConfig::SpawnScenarioObject(duelist(
        "duel_victor",
        "Duel Victor",
        VICTOR_SPAWN,
        [
            Vec3::new(-140.0, DUEL_PLANE_Y, 100.0),
            Vec3::new(0.0, DUEL_PLANE_Y, -100.0),
            Vec3::new(140.0, DUEL_PLANE_Y, 100.0),
        ],
        ships::ShipGrade::Player,
        // The relation model only makes Player<->Enemy hostile: one duelist
        // must fly the player's colors for AI-vs-AI combat to exist.
        Some(Allegiance::Player),
    ));
    let spawn_rival = EventActionConfig::SpawnScenarioObject(duelist(
        "duel_rival",
        "Duel Rival",
        RIVAL_SPAWN,
        [
            Vec3::new(140.0, DUEL_PLANE_Y, -100.0),
            Vec3::new(0.0, DUEL_PLANE_Y, 100.0),
            Vec3::new(-140.0, DUEL_PLANE_Y, -100.0),
        ],
        ships::ShipGrade::Enemy,
        None,
    ));

    let battery_allegiance = |allegiance: Allegiance| {
        EventActionConfig::SetAllegiance(SetAllegianceActionConfig {
            id: "duel_finisher".to_string(),
            allegiance,
        })
    };
    let timer = |key: &str, seconds: f64| {
        EventActionConfig::TimerStart(TimerStartActionConfig {
            key: key.to_string(),
            seconds: number(seconds),
        })
    };

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: stage
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain([rock_scatter, timer("duel_respawn", 0.5)])
                .collect(),
        },
        // The single spawn site for both duelists; also re-safeties the
        // battery, so a cycle that ended weirdly (both ships dead at once,
        // the beat timer firing into an empty arena) self-heals here.
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "duel_respawn".to_string(),
            })],
            actions: vec![
                battery_allegiance(Allegiance::Neutral),
                spawn_victor,
                spawn_rival,
            ],
        },
        // Act two, armed by the rival's defeat (destroyed OR neutralized -
        // AI stops shooting a neutralized wreck, so waiting for full
        // destruction could wait forever).
        ScenarioEventConfig {
            name: EventConfig::OnDefeated,
            filters: vec![entity("duel_rival")],
            actions: vec![timer("duel_finisher_beat", 4.0)],
        },
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "duel_finisher_beat".to_string(),
            })],
            actions: vec![battery_allegiance(Allegiance::Enemy)],
        },
        // Act three: clear the stage (the rival's wreck too, if it ended
        // neutralized rather than destroyed - despawning an id that already
        // fully despawned is a warn-and-skip, not an error) and fly in the
        // next pair.
        ScenarioEventConfig {
            name: EventConfig::OnDefeated,
            filters: vec![entity("duel_victor")],
            actions: vec![
                battery_allegiance(Allegiance::Neutral),
                despawn_object("duel_victor"),
                despawn_object("duel_rival"),
                timer("duel_respawn", 10.0),
            ],
        },
    ];

    ScenarioConfig {
        description: "Two racers duel; a siege torpedo erases the winner; repeat.".to_string(),
        hidden: true,
        menu_backdrop: true,
        events,
        ..ScenarioConfig::new("menu_duel".to_string(), "Duel Cycle".to_string(), cubemap)
    }
}
