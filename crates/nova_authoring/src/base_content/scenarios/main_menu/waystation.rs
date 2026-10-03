//! The freight-waystation main-menu backdrop.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::*;
/// The original seeded scatter's positions, sizes, materials and silhouette
/// seeds. Each cargo rock needs its own tangent to orbit the menu planet;
/// one ScatterObjects template velocity cannot orbit a full ring.
const CARGO_ROCKS: [(Meters3, Meters, &str, u32); 18] = [
    (
        Meters3::new(-1864.7748, -429.0432, -1188.4957),
        Meters(12.49941),
        KIND_ROCK,
        3297592068,
    ),
    (
        Meters3::new(-826.9489, -369.04984, -2089.9993),
        Meters(17.77287),
        KIND_ROCK,
        3864994695,
    ),
    (
        Meters3::new(-1476.7327, -599.35236, -1286.2676),
        Meters(23.92167),
        KIND_ICE,
        1947664723,
    ),
    (
        Meters3::new(1556.9567, -546.707, -1562.3416),
        Meters(18.56644),
        KIND_ROCK,
        2168599307,
    ),
    (
        Meters3::new(552.37494, -518.097, 2003.2517),
        Meters(12.65928),
        KIND_ROCK,
        259259152,
    ),
    (
        Meters3::new(-1663.1521, -453.0343, -790.8806),
        Meters(22.51752),
        KIND_ICE,
        3850304159,
    ),
    (
        Meters3::new(-1232.1969, -579.1598, -1599.8951),
        Meters(13.62021),
        KIND_ROCK,
        1763786347,
    ),
    (
        Meters3::new(-1769.8716, -490.6195, 623.7265),
        Meters(23.70276),
        KIND_ICE,
        3003914376,
    ),
    (
        Meters3::new(-1131.583, -301.3091, -1502.2435),
        Meters(10.33877),
        KIND_ICE,
        1617577883,
    ),
    (
        Meters3::new(1263.1505, -556.8329, 1823.899),
        Meters(22.22169),
        KIND_ROCK,
        4137734546,
    ),
    (
        Meters3::new(-272.80994, -439.71454, 1866.3633),
        Meters(17.50573),
        KIND_ICE,
        140415615,
    ),
    (
        Meters3::new(2147.3362, -584.1902, 708.62964),
        Meters(17.2011),
        KIND_ICE,
        4257068043,
    ),
    (
        Meters3::new(2228.1375, -407.58734, 411.87048),
        Meters(12.69876),
        KIND_ROCK,
        1106598922,
    ),
    (
        Meters3::new(468.61432, -536.1453, 1779.8889),
        Meters(17.29959),
        KIND_ROCK,
        2518937871,
    ),
    (
        Meters3::new(-1077.8923, -403.44324, 1768.6757),
        Meters(16.07594),
        KIND_ROCK,
        356607876,
    ),
    (
        Meters3::new(-2225.7043, -596.21747, 172.80734),
        Meters(19.17878),
        KIND_ROCK,
        1045848126,
    ),
    (
        Meters3::new(2015.9606, -550.9034, 549.0353),
        Meters(12.09588),
        KIND_ROCK,
        304946624,
    ),
    (
        Meters3::new(-1820.5232, -456.99384, -602.89606),
        Meters(18.09421),
        KIND_ROCK,
        652128792,
    ),
];

/// The cargo prefix, and the rotation limit this
/// endless scene hands off on.
const CARGO_ID_PREFIX: &str = "waystation_cargo_";
const TIMER_ROTATE: &str = "waystation_rotate";

pub(crate) fn menu_waystation(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    // Lighter pull for the two heavy haulers to hold their orbit.
    let planet = backdrop_planetoid(30_000.0);
    let ScenarioObjectKind::Planet(planet_config) = &planet.kind else {
        unreachable!("the menu landmark is authored as a planet");
    };
    let well = GravityWell::from_mass(
        planet_config.mass,
        planet_config.body_radius().to_engine(),
        &GravitySettings::default(),
    );
    let mut objects = vec![
        planet,
        backdrop_orbiter(
            "waystation_hauler_a",
            "Hauler Biscuit",
            Meters3::new(1_400.0, 0.0, 0.0),
            true,
        ),
        backdrop_orbiter(
            "waystation_hauler_b",
            "Hauler Kettle",
            Meters3::new(-1_400.0, 0.0, 0.0),
            true,
        ),
        backdrop_beacon(
            "waystation_dock_a",
            "DOCK-A",
            Meters3::new(1_700.0, -250.0, 600.0),
            Color::srgb(1.0, 0.7, 0.2),
        ),
        backdrop_beacon(
            "waystation_dock_b",
            "DOCK-B",
            Meters3::new(1_500.0, -300.0, -900.0),
            Color::srgb(1.0, 0.7, 0.2),
        ),
        backdrop_beacon(
            "waystation_traffic",
            "TRAFFIC",
            Meters3::new(-1_800.0, -200.0, 400.0),
            Color::srgb(0.3, 0.9, 1.0),
        ),
    ];
    objects.extend(backdrop_rig("waystation").objects());

    // Snapshot the original seeded lane so the cargo keeps its first-frame
    // layout, while each rock authors the velocity its own radius needs.
    let cargo = CARGO_ROCKS
        .iter()
        .enumerate()
        .map(|(index, (position, radius, kind, seed))| {
            let radial = position.to_engine();
            let tangent = radial.cross(Vec3::Y).normalize();
            let velocity = MetersPerSecond3::from_engine(
                tangent * circular_orbit_speed(well.mu, radial.length()),
            );
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: format!("{CARGO_ID_PREFIX}{index}"),
                    name: "Cargo Rock".to_string(),
                    position: *position,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    kind: (*kind).into(),
                    destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
                    radius: *radius,
                    texture: asteroid_texture.clone(),
                    initial_velocity: velocity,
                    seed: Some(*seed),
                    lock_signature: None,
                }),
            })
        });

    let events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                // The scene poses its own camera: a fixed mid-range shot on
                // the planetoid (the old well-derived pose averaged ~here;
                // the noise mesh runs to ~1.2 km, safely inside the frame).
                .chain([
                    // HELD at 3,350 m: the planetoid at the origin IS the
                    // shot, and the traffic works its flanks at +-1.4..1.8 km
                    // - so the near arc is already inside the rolloff and the
                    // far arc cannot be brought in without putting the camera
                    // through the rock. Distance is doing the right thing here.
                    backdrop_camera(Meters3::new(0.0, 1_000.0, 3_350.0)),
                ])
                .chain(cargo)
                .chain([
                    // The carousel's rotation limit: the waystation's day
                    // never ends on its own, so after a couple of freight
                    // laps the menu turns to the next backdrop.
                    EventActionConfig::TimerStart(TimerStartActionConfig {
                        key: TIMER_ROTATE.to_string(),
                        seconds: crate::scenario_helpers::number(150.0),
                    }),
                ])
                .collect::<_>(),
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: TIMER_ROTATE.to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: super::MENU_GAUNTLET_SCENARIO_ID.to_string(),
                linger: false,
                delay: Some(1.0),
            })],
        },
    ];

    ScenarioConfig {
        description: "A freight waystation going about its day.".to_string(),
        role: ScenarioRole::Backdrop,
        events,
        ..ScenarioConfig::new(
            super::MENU_WAYSTATION_SCENARIO_ID.to_string(),
            "Waystation Traffic".to_string(),
            cubemap,
        )
    }
}
