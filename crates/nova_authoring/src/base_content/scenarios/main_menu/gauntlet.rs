//! The torpedo-gauntlet main-menu backdrop: a station-keeping racer's point
//! defense against scripted torpedo batteries on both flanks - a doomed
//! stand. The racer's PDC magazines are HARD (no reload): it swats torpedoes
//! until the guns run dry, the stream overruns it, and the blast ends the
//! act; after a beat the carousel turns to the next backdrop.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::{backdrop_beacon, backdrop_camera, backdrop_rig, planetoid_glow};
use crate::{
    base_content::{scenarios::SCATTER_SEED, ships},
    scenario_helpers::{entity, number},
};

/// The racer's HARD magazine per PDC turret (SetAmmo strips the auto-reload).
/// Sized for roughly a TEN-torpedo defense across the pair: the first cut
/// shipped 1800 per turret and the stand never fell inside a menu visit.
const RACER_ROUNDS_PER_TURRET: u32 = 400;

/// The racer's station-keeping circuit, just left of frame center (the menu
/// panel owns the right half): a six-point ring with ~55 u legs and a little
/// vertical wander - enough motion to read as a ship on watch, small enough
/// that the guns, not the flying, stay the show. Its centroid anchors the
/// leash. RAISED ~25 u above the fight plane: at y 0 the ship parked in
/// front of the far rock cluster and vanished into the low contrast; up
/// here it reads against black sky, above the band's sightline.
const HOLD_LOOP: [Vec3; 6] = [
    Vec3::new(-5.0, 25.0, 0.0),
    Vec3::new(-32.5, 33.0, 48.0),
    Vec3::new(-87.5, 20.0, 48.0),
    Vec3::new(-115.0, 25.0, 0.0),
    Vec3::new(-87.5, 35.0, -48.0),
    Vec3::new(-32.5, 18.0, -48.0),
];

/// The racer's authored detection range. Two geometry facts hang off it: the
/// batteries park ~720 u from the circuit's centroid, so the DEFAULT 800
/// would pull the racer off station toward whichever battery its acquisition
/// found; and a leaked hit still overrides the passive gate (damage memory
/// ignores detection range), which is why the batteries ALSO stay beyond
/// leash + turret reach (250 + 450) of the centroid - a lunging racer can
/// never bring its guns into range before the leash walks it home.
const RACER_ENGAGE_RANGE: f32 = 300.0;

/// Battery parks: far off both flanks of the frame (~+-230 u visible at
/// origin depth), at slightly scattered y/z so the inbound lanes fan instead
/// of stacking. All are >700 u from the circuit centroid (see
/// [`RACER_ENGAGE_RANGE`]); with launches SCRIPTED there is no AI launch
/// envelope to stay inside of.
const BATTERY_PARKS: [(&str, Vec3, f64, f64); 4] = [
    // (id, park, first launch at, relaunch every)
    (
        "gauntlet_battery_w1",
        Vec3::new(-800.0, 20.0, 150.0),
        3.0,
        15.0,
    ),
    (
        "gauntlet_battery_w2",
        Vec3::new(-780.0, -30.0, -170.0),
        8.0,
        18.0,
    ),
    (
        "gauntlet_battery_e1",
        Vec3::new(660.0, 25.0, 140.0),
        12.0,
        21.0,
    ),
    (
        "gauntlet_battery_e2",
        Vec3::new(640.0, -20.0, -160.0),
        16.0,
        24.0,
    ),
];

/// One battery: a Spaceship in kind only - a single standard torpedo bay,
/// no thrusters, no controller. `ForceTorpedoLaunch` is its whole brain;
/// Enemy allegiance is what its torpedoes inherit, which is what makes them
/// point-defense targets for the Player-allegiance racer. Rotation stays
/// identity: the bay launches along its +Y like a vertical cell and PN
/// guidance arcs the torpedo onto the lane.
fn battery(id: &str, park: Vec3) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: "Gauntlet Battery".to_string(),
            position: park,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Enemy),
            controller: SpaceshipController::None,
            sections: vec![SpaceshipSectionConfig {
                id: "bay".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("torpedo_section".to_string()),
                modifications: vec![],
            }],
        }),
    }
}

/// A doomed stand behind the menu: the racer (player-grade PDC turrets)
/// holds a station circuit while four dumb batteries, parked far off BOTH
/// flanks, launch standard torpedoes at it on staggered scenario timers.
/// Torpedoes stream in from both sides of the frame and the racer swats
/// them mid-shot - but its magazines are hard (SetAmmo, no reload), so the
/// defense eventually runs dry and the stream wins. The racer's death
/// starts a short aftermath linger, then the carousel turns to the next
/// backdrop (the menu's Factorio-style rotation). A battery whose target is
/// already gone skips its launch (no dud ordnance).
pub(crate) fn menu_gauntlet(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut stage = Vec::new();

    stage.extend(backdrop_rig("gauntlet").objects());
    stage.push(planetoid_glow("gauntlet_lamp"));

    // Waypoint markers by the station circuit: intangible, and parked off
    // every torpedo lane so the dressing cannot eat an inbound round.
    stage.push(backdrop_beacon(
        "gauntlet_nav_a",
        "KP-7",
        Vec3::new(-150.0, -10.0, 20.0),
        Color::srgb(1.0, 0.75, 0.4),
    ));
    stage.push(backdrop_beacon(
        "gauntlet_nav_b",
        "KP-12",
        Vec3::new(-30.0, -18.0, -110.0),
        Color::srgb(1.0, 0.75, 0.4),
    ));

    for (id, park, _, _) in BATTERY_PARKS {
        stage.push(battery(id, park));
    }

    // Depth dressing, below the station circuit and the torpedo lanes (which
    // run at y ~ +-30 from the flanks to the circuit - the rocks cannot
    // block launches or eat torpedoes).
    let rock_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "gauntlet_rock_".to_string(),
        count: 26,
        seed: SCATTER_SEED ^ 0x3,
        region: ScatterRegion::Ring {
            center: Vec3::ZERO,
            inner: 140.0,
            outer: 230.0,
            y_min: -75.0,
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
            position: HOLD_LOOP[0],
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            // Player allegiance is what makes the Enemy batteries' torpedoes
            // hostile - AI-vs-AI combat needs one side on the player's team.
            allegiance: Some(Allegiance::Player),
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: HOLD_LOOP.to_vec(),
                // The arrival grace lands the ship on its ROUTINE: ships
                // spawn in the Engage state and hold on any acquired target,
                // so an ungraced spawn would charge a battery instead of
                // taking up station.
                engage_delay: Some(2.0),
                // If a torpedo leaks through, the hit overrides the passive
                // gate and the racer lunges; the leash walks it back onto
                // the circuit once the damage memory fades.
                leash: Some(250.0),
                engage_range: Some(RACER_ENGAGE_RANGE),
                // Hold the intercepts for the camera: the default 400 u PD
                // ring kills inbound torpedoes at (or past) the frame edge;
                // 130 u waits until the ordnance is well inside even the
                // narrow 4:3 shot before the tracer stream opens up. The
                // shorter window means more leaks - which is drama, and the
                // fall of the stand is the scene's ending anyway.
                pd_range: Some(130.0),
                ..Default::default()
            }),
            // Player-grade racer with HARD magazines: the full 100-rounds/s
            // PDC turrets are the show, and their finite rounds are the
            // scene's clock - when they run dry the stand falls.
            sections: {
                let mut sections = ships::racer_sections(ships::ShipGrade::Player, vec![]);
                for section in &mut sections {
                    if ships::RACER_TURRET_IDS.contains(&section.id.as_str()) {
                        section
                            .modifications
                            .push(SectionModification::SetAmmo(RACER_ROUNDS_PER_TURRET));
                    }
                }
                sections
            },
        }),
    });

    let timer = |key: &str, seconds: f64| {
        EventActionConfig::TimerStart(TimerStartActionConfig {
            key: key.to_string(),
            seconds: number(seconds),
        })
    };

    let mut events = vec![
        // One spawn site per ship id: OnStart only arms the timers, and each
        // id's single handler performs the first spawn/launch and every
        // later one.
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: stage
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                // Closer than the reference shot: the racer IS this scene,
                // and at 300 u it was a handful of pixels. From (0, 80, 260)
                // the 4:3 half-frame is ~150 u at origin depth - the raised
                // circuit and its intercepts stay in shot, bigger.
                .chain([
                    backdrop_camera(Vec3::new(0.0, 80.0, 260.0)),
                    rock_scatter,
                    spawn_ship,
                    // Stall watchdog (the duel's idiom): a racer crippled
                    // without counting as DEFEATED would freeze the cycle;
                    // healthy stands reload long before this fires.
                    timer("gauntlet_watchdog", 360.0),
                ])
                .chain(
                    BATTERY_PARKS
                        .iter()
                        .map(|(id, _, first, _)| timer(&format!("{id}_fire"), *first)),
                )
                .collect(),
        },
        // The fall of the stand: the racer's death (dry guns, leaked
        // torpedo) starts an aftermath linger - the wreck drifts, the
        // batteries fire into the dark and skip - then the FULL reset.
        ScenarioEventConfig {
            name: EventConfig::OnDefeated,
            filters: vec![entity("gauntlet_ship")],
            actions: vec![timer("gauntlet_reset", 8.0)],
        },
        // The fall of the stand ends the act: teardown clears the wreck,
        // debris and in-flight ordnance, and the carousel turns to the next
        // backdrop. Own handler + short delay, per the NextScenario
        // same-flush rule.
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "gauntlet_reset".to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "menu_weave".to_string(),
                linger: false,
                delay: Some(1.0),
            })],
        },
        // The watchdog's own reset (see OnStart).
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "gauntlet_watchdog".to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "menu_weave".to_string(),
                linger: false,
                delay: Some(1.0),
            })],
        },
    ];

    // Each battery fires on its own self-restarting clock; the staggered
    // periods keep the two flanks trading lanes instead of salvoing. A
    // launch during the aftermath linger (racer gone) is skipped by the
    // action itself.
    for (id, _, _, period) in BATTERY_PARKS {
        events.push(ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: format!("{id}_fire"),
            })],
            actions: vec![
                EventActionConfig::ForceTorpedoLaunch(ForceTorpedoLaunchActionConfig {
                    id: id.to_string(),
                    target: "gauntlet_ship".to_string(),
                }),
                timer(&format!("{id}_fire"), period),
            ],
        });
    }

    ScenarioConfig {
        description: "A racer's doomed point-defense stand against batteries on both flanks."
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
