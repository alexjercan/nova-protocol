//! The torpedo-gauntlet main-menu backdrop: a station-keeping gunship's point
//! defense against scripted torpedo batteries on both flanks - a doomed
//! stand. The gunship's PDC magazines are HARD (no reload): it swats torpedoes
//! until the guns run dry, the stream overruns it, and the blast ends the
//! act; after a beat the carousel turns to the next backdrop.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::{backdrop_beacon, backdrop_camera, backdrop_rig, planetoid_glow};
use crate::{
    base_content::{scenarios::SCATTER_SEED, ships},
    scenario_helpers::{entity, number},
};

/// The gunship's HARD magazine per PDC turret (SetAmmo strips the auto-reload).
/// Sized so the stand SHOWS about four intercepts before the guns run dry and
/// the next torpedo ends it - six mounts at 100 rounds/s spend this in roughly
/// six seconds of battery fire, and an intercept costs about a second and a
/// half of it.
///
/// PER TURRET, not per ship, so recasting the stand from a two-turret corvette
/// onto the six-mount block gunship does NOT divide the old number six ways.
/// That was tried at 135: every bearing mount ran dry three seconds before
/// impact, the first torpedo through killed the ship, and the scene was over
/// in half a minute with nothing shot down. What a mount needs is enough
/// rounds to FINISH the intercept it opened.
const GUNSHIP_ROUNDS_PER_TURRET: u32 = 600;

/// The gunship's station-keeping circuit, just left of frame center (the menu
/// panel owns the right half): a six-point ring with ~550 m legs and a little
/// vertical wander - enough motion to read as a ship on watch, small enough
/// that the guns, not the flying, stay the show. Its centroid anchors the
/// leash. RAISED ~250 m above the fight plane: at y 0 the ship parked in
/// front of the far rock cluster and vanished into the low contrast; up
/// here it reads against black sky, above the band's sightline.
const HOLD_LOOP: [Meters3; 6] = [
    Meters3::new(-50.0, 250.0, 0.0),
    Meters3::new(-325.0, 330.0, 480.0),
    Meters3::new(-875.0, 200.0, 480.0),
    Meters3::new(-1_150.0, 250.0, 0.0),
    Meters3::new(-875.0, 350.0, -480.0),
    Meters3::new(-325.0, 180.0, -480.0),
];

/// The gunship's authored detection range. Two geometry facts hang off it: the
/// batteries park ~7.2 km from the circuit's centroid, so a detection range that
/// reached them would pull the gunship off station toward whichever battery its
/// acquisition found; and a leaked hit still overrides the passive gate (damage
/// memory ignores detection range), which is why the batteries ALSO stay beyond
/// leash + turret reach (2.5 km + 2 km) of the centroid - a lunging gunship can
/// never bring its guns into range before the leash walks it home. Authored
/// rather than left to the 4 km default so the framing survives a retune of
/// the engine constants.
const GUNSHIP_ENGAGE_RANGE: Meters = Meters(3_000.0);

/// Battery parks: far off both flanks of the frame (~+-2.3 km visible at
/// origin depth), at slightly scattered y/z so the inbound lanes fan instead
/// of stacking. All are >7 km from the circuit centroid (see
/// [`GUNSHIP_ENGAGE_RANGE`]); with launches SCRIPTED there is no AI launch
/// envelope to stay inside of.
const BATTERY_PARKS: [(&str, Meters3, f64, f64); 4] = [
    // (id, park, first launch at, relaunch every)
    (
        "gauntlet_battery_w1",
        Meters3::new(-8_000.0, 200.0, 1_500.0),
        3.0,
        15.0,
    ),
    (
        "gauntlet_battery_w2",
        Meters3::new(-7_800.0, -300.0, -1_700.0),
        8.0,
        18.0,
    ),
    (
        "gauntlet_battery_e1",
        Meters3::new(6_600.0, 250.0, 1_400.0),
        12.0,
        21.0,
    ),
    (
        "gauntlet_battery_e2",
        Meters3::new(6_400.0, -200.0, -1_600.0),
        16.0,
        24.0,
    ),
];

/// One battery: a Spaceship in kind only - a single standard torpedo bay,
/// no thrusters, no controller. `ForceTorpedoFire` on its `bay` is its whole
/// brain;
/// Enemy allegiance is what its torpedoes inherit, which is what makes them
/// point-defense targets for the Player-allegiance gunship. Rotation stays
/// identity: the bay launches along its +Y like a vertical cell and PN
/// guidance arcs the torpedo onto the lane.
fn battery(id: &str, park: Meters3) -> ScenarioObjectConfig {
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
            hull: ships::inline_hull(vec![SpaceshipSectionConfig {
                id: "bay".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("torpedo_section".to_string()),
                modifications: vec![],
            }]),
            ..Default::default()
        }),
    }
}

/// A doomed stand behind the menu: the block gunship (six PDC turrets) holds
/// a station circuit while four dumb batteries, parked far off BOTH flanks,
/// launch standard torpedoes at it on staggered scenario timers. Torpedoes
/// stream in from both sides of the frame and the gunship swats them
/// mid-shot - but its magazines are hard (SetAmmo, no reload), so the
/// defense eventually runs dry and the stream wins. The gunship's death
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
        Meters3::new(-1_500.0, -100.0, 200.0),
        Color::srgb(1.0, 0.75, 0.4),
    ));
    stage.push(backdrop_beacon(
        "gauntlet_nav_b",
        "KP-12",
        Meters3::new(-300.0, -180.0, -1_100.0),
        Color::srgb(1.0, 0.75, 0.4),
    ));

    for (id, park, _, _) in BATTERY_PARKS {
        stage.push(battery(id, park));
    }

    // Depth dressing, below the station circuit and the torpedo lanes (which
    // run at y ~ +-300 m from the flanks to the circuit - the rocks cannot
    // block launches or eat torpedoes).
    let rock_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "gauntlet_rock_".to_string(),
        count: 26,
        seed: SCATTER_SEED ^ 0x3,
        region: ScatterRegion::Ring {
            center: Meters3::ZERO,
            inner: Meters(1_400.0),
            outer: Meters(2_300.0),
            y_min: Meters(-750.0),
            y_max: Meters(-300.0),
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "gauntlet_rock_".to_string(),
                name: "Gauntlet Rock".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                material: None,
                destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
                radius: Meters(10.0),
                texture: asteroid_texture,
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((Meters(10.0), Meters(30.0))),
        min_separation: None,
    });

    let spawn_ship = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "gauntlet_ship".to_string(),
            name: "Gauntlet Gunship".to_string(),
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
                // gate and the gunship lunges; the leash walks it back onto
                // the circuit once the damage memory fades.
                leash: Some(Meters(2_500.0)),
                engage_range: Some(GUNSHIP_ENGAGE_RANGE),
                // Hold the intercepts for the camera: the default 1.5 km PD
                // ring kills inbound torpedoes at the frame edge; 1.3 km
                // waits until the ordnance is well inside even the narrow 4:3
                // shot before the tracer stream opens up. The shorter window
                // means more leaks - which is drama, and the fall of the stand
                // is the scene's ending anyway.
                pd_range: Some(Meters(1_300.0)),
                ..Default::default()
            }),
            // The armoured block gunship with HARD magazines: the full
            // 100-rounds/s PDC turrets are the show, and their finite rounds
            // are the scene's clock - when they run dry the stand falls. Six
            // mounts cover both hemispheres, so a torpedo arriving from
            // either flank meets guns without the ship having to turn.
            hull: ships::hull(ships::BLOCK_GUNSHIP_SHIP_ID),
            modifications: ships::BLOCK_GUNSHIP_TURRET_IDS
                .iter()
                .map(|turret| {
                    ships::on_section(
                        turret,
                        vec![SectionModification::SetAmmo(GUNSHIP_ROUNDS_PER_TURRET)],
                    )
                })
                .collect(),
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
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: stage
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                // Closer than the reference shot: the gunship IS this scene,
                // and at 3 km it was a handful of pixels. From (0, 800, 2,600)
                // the 4:3 half-frame is ~1.5 km at origin depth - the raised
                // circuit and its intercepts stay in shot, bigger.
                .chain([
                    // HELD at 2,600 m against the audio pass that pulled the
                    // duel's camera in: KP-7 sits at x -1,500 m and a 4:3
                    // frame sees +-0.55 x the camera distance, so this pose
                    // already has the beacon exactly on the left edge. Coming
                    // in far enough to matter for the rolloff (~2 km) crops
                    // it. The gunship reads at ~0.03 of full volume from
                    // here, which is the cost of keeping the shot.
                    backdrop_camera(Meters3::new(0.0, 800.0, 2_600.0)),
                    rock_scatter,
                    spawn_ship,
                    // Stall watchdog (the duel's idiom): a gunship crippled
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
        // The fall of the stand: the gunship's death (dry guns, leaked
        // torpedo) starts an aftermath linger - the wreck drifts, the
        // batteries fire into the dark and skip - then the FULL reset.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDefeated,
            once: false,
            filters: vec![entity("gauntlet_ship")],
            actions: vec![timer("gauntlet_reset", 8.0)],
        },
        // The fall of the stand ends the act: teardown clears the wreck,
        // debris and in-flight ordnance, and the carousel turns to the next
        // backdrop. Own handler + short delay, per the NextScenario
        // same-flush rule.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
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
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
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
    // launch during the aftermath linger (gunship gone) is skipped by the
    // action itself.
    for (id, _, _, period) in BATTERY_PARKS {
        events.push(ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: format!("{id}_fire"),
            })],
            actions: vec![
                EventActionConfig::ForceTorpedoFire(ForceTorpedoFireActionConfig {
                    ship: id.to_string(),
                    section: "bay".to_string(),
                    target: "gauntlet_ship".to_string(),
                }),
                timer(&format!("{id}_fire"), period),
            ],
        });
    }

    ScenarioConfig {
        description: "A gunship's doomed point-defense stand against batteries on both flanks."
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
