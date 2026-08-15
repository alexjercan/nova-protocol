//! The duel-cycle main-menu backdrop: two corvettes fight, the winner is
//! erased by a siege torpedo, and after a beat two fresh ships fly in.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::{backdrop_camera, backdrop_rig, planetoid_glow};
use crate::{
    base_content::{scenarios::SCATTER_SEED, ships},
    scenario_helpers::{entity, number},
};

/// The finisher battery's park: far outside the ~+-230 u camera frame AND
/// beyond the winner's leash + turret reach (250 + 200) of the arena center,
/// so a victor lunging off a blast hit can never bring its guns onto the
/// battery before the leash walks it home. With the launch SCRIPTED there is
/// no AI envelope to stay inside of.
const BATTERY_POS: Vec3 = Vec3::new(-950.0, 0.0, 0.0);
/// Off-screen entrances, one per side, with a little vertical split so the
/// approach lines cross instead of meeting head-on.
const VICTOR_SPAWN: Vec3 = Vec3::new(-420.0, 25.0, 100.0);
const RIVAL_SPAWN: Vec3 = Vec3::new(420.0, -15.0, -100.0);

/// One duelist: a corvette that flies in from off-screen onto an in-frame
/// patrol triangle. The arrival grace holds the entrance (ships spawn in
/// the Engage state and hold on ANY acquired target, so an ungraced spawn
/// would turn and burn from the spawn point instead of flying in); the
/// leash, anchored on the patrol centroid at the frame's center, keeps the
/// fight from drifting out of shot. With no rock and no gravity anywhere in
/// the scene, every chase line through the center is clear - the fight
/// happens IN the middle of the frame, not pinned against a planetoid.
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
            collapse_threshold: None,
            skin: false,
            style: None,
            allegiance,
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: patrol.to_vec(),
                // The leash anchors on center-hugging patrol centroids, so
                // the fight gravitates to the middle of the frame. Wide
                // enough for real chases, tight enough that the act plays
                // over the same ground every wave.
                leash: Some(250.0),
                engage_delay: Some(6.0),
                ..Default::default()
            }),
            // Hardened flight computers on BOTH duelists: the tight rings
            // make the merge a nose-to-nose joust, and a stock controller
            // dies to the opening burst - which under the brain-death
            // defeat rule would end the act seconds after it starts. The
            // 500 keeps the DOGFIGHT on screen; however the loser finally
            // cripples (guns, computer, or full destruction), the defeat
            // chain fires and the finale plays.
            sections: ships::cargoa_sections(grade, vec![SectionModification::SetHealth(500.0)]),
        }),
    }
}

/// A repeating three-act battle behind the menu. Act one: a player-grade
/// corvette (Player allegiance) and an enemy-grade corvette fly in from opposite
/// sides and dogfight through the open center of the frame; the gun gap
/// makes the outcome all but certain. Act two: the rival's defeat starts a
/// short beat, then the off-screen siege battery is SCRIPTED to launch an
/// armored ship-killing torpedo at the winner - point defense hammers the
/// ordnance and loses - re-firing on a slow clock until one connects. Act
/// three: the aftermath drifts for a beat, then the carousel turns to the
/// next backdrop - the scenario switch is a genuine full reset that clears
/// wrecks, debris and in-flight ordnance.
pub(crate) fn menu_duel(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut stage = Vec::new();

    // An OPEN arena: no planetoid (the first cut proved chase lines pin
    // against a central rock and its SOI drags the fight onto it), just a
    // sparse dressing ring well below the fight plane. A crippled loser
    // drifting into the rocks is harmless now - brain-death neutralizes it
    // the moment its computer (or last gun) goes, rocks or no rocks.
    stage.extend(backdrop_rig("duel").objects());
    stage.push(planetoid_glow("duel_lamp"));

    // The finisher: one siege bay, nothing else. No controller - the launch
    // is scripted, so the battery needs no AI and no detection range. It
    // sits NEUTRAL until the beat flips it Enemy: an Enemy battery would be
    // a live acquisition target, and the freshly-victorious ship - still in
    // its combat hold, which keeps ANY acquired hostile - would break off
    // its victory lap and drift toward the frame's left edge chasing it
    // (the observed end-of-duel drift). Neutral is invisible to
    // acquisition; the flip lands only for the kill window, when the
    // ordnance must inherit Enemy so the victor's PD engages (and loses).
    stage.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "duel_finisher".to_string(),
            name: "Duel Finisher".to_string(),
            position: BATTERY_POS,
            rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            collapse_threshold: None,
            skin: false,
            style: None,
            allegiance: Some(Allegiance::Neutral),
            controller: SpaceshipController::None,
            sections: vec![SpaceshipSectionConfig {
                id: "siege_bay".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("heavy_torpedo_section".to_string()),
                modifications: vec![],
            }],
        }),
    });

    // Sparse dressing ring, below the fight plane - depth parallax the
    // rockless first cut of this arena visibly missed.
    let rock_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "duel_rock_".to_string(),
        count: 12,
        seed: SCATTER_SEED ^ 0x5,
        region: ScatterRegion::Ring {
            center: Vec3::ZERO,
            inner: 150.0,
            outer: 220.0,
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

    // The victor's routine is a TIGHT ring on the frame center: it is the
    // fight's center of gravity while the duel runs (the leash anchors on
    // the patrol centroid), and after the win it parks the victory lap right
    // where the finisher's torpedo will land - mid-shot, not drifting at the
    // frame edge.
    let spawn_victor = EventActionConfig::SpawnScenarioObject(duelist(
        "duel_victor",
        "Duel Victor",
        VICTOR_SPAWN,
        [
            Vec3::new(-70.0, 10.0, 50.0),
            Vec3::new(70.0, 15.0, -50.0),
            Vec3::new(0.0, 5.0, 70.0),
        ],
        ships::ShipGrade::Player,
        // The relation model only makes Player<->Enemy hostile: one duelist
        // must fly the player's colors for AI-vs-AI combat to exist. It also
        // makes the Enemy finisher's ordnance hostile to the winner.
        Some(Allegiance::Player),
    ));
    // The rival's ring mirrors the victor's tight center ring, so the
    // fight's whole geometry - approach, merge, chase - happens in the
    // middle of the frame instead of wandering to the edges.
    let spawn_rival = EventActionConfig::SpawnScenarioObject(duelist(
        "duel_rival",
        "Duel Rival",
        RIVAL_SPAWN,
        [
            Vec3::new(70.0, -5.0, -50.0),
            Vec3::new(-70.0, -10.0, 50.0),
            Vec3::new(0.0, -15.0, -70.0),
        ],
        ships::ShipGrade::Enemy,
        None,
    ));

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
                // The scene poses its own camera: the reference backdrop
                // shot, dead on the arena center the duel fights over.
                .chain([
                    backdrop_camera(Vec3::new(0.0, 90.0, 300.0)),
                    rock_scatter,
                    timer("duel_respawn", 0.5),
                    // Stall watchdog: a duelist can end up crippled without
                    // ever counting as DEFEATED (observed live: a rival lost
                    // its flight computer, drifted out of the victor's leash
                    // reach, and the cycle sat frozen for 11 minutes). Every
                    // healthy cycle reloads the scenario long before this
                    // fires - and the reload re-arms it - so the watchdog
                    // only ever catches a wedged state.
                    timer("duel_watchdog", 300.0),
                ])
                .collect(),
        },
        // The single spawn site for both duelists.
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "duel_respawn".to_string(),
            })],
            actions: vec![spawn_victor, spawn_rival],
        },
        // Act two, armed by the rival's defeat (destroyed OR neutralized -
        // AI stops shooting a neutralized wreck, so waiting for full
        // destruction could wait forever).
        ScenarioEventConfig {
            name: EventConfig::OnDefeated,
            filters: vec![entity("duel_rival")],
            actions: vec![timer("duel_finisher_beat", 4.0)],
        },
        // The finisher clock: launch at the winner and re-arm itself, so a
        // miss (or a launch skipped because the victor just died to a wreck
        // collision) retries instead of stalling the cycle. The re-arm is
        // LONGER than the ~16 s flight from the park, so exactly one siege
        // torpedo is ever in the air - the first cut re-armed at 12 s and
        // doubled up. Expired keys are removed before dispatch, so the
        // self-restart is legal.
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "duel_finisher_beat".to_string(),
            })],
            actions: vec![
                // Hostile only for the kill window (see the battery's spawn
                // comment); the full reset restores the authored Neutral.
                EventActionConfig::SetAllegiance(SetAllegianceActionConfig {
                    id: "duel_finisher".to_string(),
                    allegiance: Allegiance::Enemy,
                }),
                EventActionConfig::ForceTorpedoLaunch(ForceTorpedoLaunchActionConfig {
                    id: "duel_finisher".to_string(),
                    target: "duel_victor".to_string(),
                }),
                timer("duel_finisher_beat", 20.0),
            ],
        },
        // Act three: stop the finisher clock and let the aftermath drift for
        // a beat - the wrecks stay in shot - then the carousel turns.
        ScenarioEventConfig {
            name: EventConfig::OnDefeated,
            filters: vec![entity("duel_victor")],
            actions: vec![
                EventActionConfig::TimerCancel(TimerCancelActionConfig {
                    key: "duel_finisher_beat".to_string(),
                }),
                timer("duel_reset", 8.0),
            ],
        },
        // The hand-off: teardown despawns every scoped entity (wrecks,
        // debris, in-flight ordnance - runtime projectiles are
        // scenario-scoped too) and the next backdrop starts fresh. In its
        // own handler with a short delay: an instant switch consumed in the
        // same flush would discard sibling handlers' queued commands.
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "duel_reset".to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "menu_waystation".to_string(),
                linger: false,
                delay: Some(1.0),
            })],
        },
        // The watchdog's own reset (see OnStart).
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "duel_watchdog".to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "menu_waystation".to_string(),
                linger: false,
                delay: Some(1.0),
            })],
        },
    ];

    ScenarioConfig {
        description: "Two corvettes duel; a siege torpedo erases the winner; repeat.".to_string(),
        hidden: true,
        menu_backdrop: true,
        events,
        ..ScenarioConfig::new("menu_duel".to_string(), "Duel Cycle".to_string(), cubemap)
    }
}
