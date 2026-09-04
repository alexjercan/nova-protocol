//! The belt both mainline chapters are set in.
//!
//! Chapter two is chapter one's map an hour later, so the two scenarios must
//! agree on every fixed thing in it down to the metre: the same two planetoids
//! with the same masses, the same rock plate, the same far dressing, and the
//! carrier's position - which chapter one spawns a ship at and chapter two
//! spawns a grave at. Authoring that twice would let the wreck field drift off
//! the rocks it is supposed to be tangled in, so it is authored ONCE here and
//! both chapters read it.
//!
//! Layout provenance: `examples/playable/first_shift_map.rs` and
//! `second_shift_map.rs`, the spatial benches this stage was reviewed in.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

/// The carrier's berth, and later the centre of its wreck.
pub(crate) const CARRIER_POS: Meters3 = Meters3::new(-1_000.0, 0.0, 2_500.0);

/// The small planetoid the first shift's inspection round is flown against.
pub(crate) const INSPECTION_POS: Meters3 = Meters3::new(-4_500.0, -400.0, -6_500.0);
/// Its MEAN radius, and now a real one.
///
/// As a rock this read 200 m and meant nothing directly: the noise mesh
/// reached 3.5-6.0 times past it, and the geometric body the sim measured from
/// came out at 1 000.1 m. A planet's mesh stands only `1 + relief` off its
/// radius, so the number has to carry the size itself. 950 m of mean radius
/// puts the surface at 997.5 m - within 0.3% of the body this scene was tuned
/// against, so the well, the 3.29 km sphere of influence and the ORBIT
/// autopilot's ring are the ones the chapters were authored for.
/// `the_belt_planets_keep_the_body_radius_their_rocks_published` pins it.
pub(crate) const INSPECTION_RADIUS: Meters = Meters(950.0);
/// A dust world: ochre and mundane, the sort of body a maintenance shift is
/// sent to inspect, and instantly distinct from the dark rock across the belt.
pub(crate) const INSPECTION_TYPE: PlanetType = PlanetType::DustWorld;
/// Draws a wide ochre plain under a pale frost cap, with the basin turned
/// toward the approach lane the chapter flies in on.
pub(crate) const INSPECTION_SEED: u32 = 7;
/// Its mass parameter (mu, u^3/s^2): the shakedown's proven tutorial well,
/// a 3.29 km sphere of influence with an escapable surface pull.
pub(crate) const INSPECTION_MASS: f32 = 27_000.0;

/// The large planetoid on the far side of the belt. It exists to HIDE
/// something: it is two and a half times the inspection body's nominal radius,
/// so its 1.75-3.0 km hull is opaque to anything parked behind it.
pub(crate) const CONCEALMENT_POS: Meters3 = Meters3::new(4_500.0, 300.0, -6_500.0);
/// Its MEAN radius, on the same footing as [`INSPECTION_RADIUS`]: 2 250 m puts
/// the surface at 2 373.8 m, within 0.2% of the 2 377.0 m body the rock
/// published, so what it hides it still hides.
pub(crate) const CONCEALMENT_RADIUS: Meters = Meters(2_250.0);
/// Barren rock: airless grey stone, no cap, no colour. It exists to be a wall,
/// and it reads as one beside the dust world.
pub(crate) const CONCEALMENT_TYPE: PlanetType = PlanetType::BarrenRock;
/// Draws dark mare against pale highland, so the silhouette stays legible at
/// belt range without the body ever looking inviting.
pub(crate) const CONCEALMENT_SEED: u32 = 3;
/// Deliberately WEAKER than the inspection body despite being larger: the
/// first shift's navigation lesson is authored against one well, and a second
/// one reaching into the route would teach the wrong thing about both.
pub(crate) const CONCEALMENT_MASS: f32 = 20_000.0;

/// The rock plate between the carrier and both planetoids: broad enough that
/// the cutter has several lines through it, and tight enough that nothing
/// capital-sized would try. Chapter one's crates sit in it; chapter two's
/// wreckage is scattered across it.
pub(crate) const SALVAGE_ROCKS: [(Meters3, Meters); 40] = [
    (Meters3::new(400.0, 220.0, -1_200.0), Meters(32.0)),
    (Meters3::new(1_000.0, -260.0, -1_000.0), Meters(22.0)),
    (Meters3::new(1_700.0, 320.0, -1_050.0), Meters(35.0)),
    (Meters3::new(2_300.0, -220.0, -1_250.0), Meters(26.0)),
    (Meters3::new(2_800.0, 240.0, -1_700.0), Meters(30.0)),
    (Meters3::new(3_000.0, -280.0, -2_300.0), Meters(20.0)),
    (Meters3::new(3_050.0, 300.0, -2_900.0), Meters(34.0)),
    (Meters3::new(2_900.0, -220.0, -3_500.0), Meters(24.0)),
    (Meters3::new(2_600.0, 280.0, -4_100.0), Meters(30.0)),
    (Meters3::new(2_100.0, -260.0, -4_500.0), Meters(22.0)),
    (Meters3::new(1_400.0, 220.0, -4_700.0), Meters(35.0)),
    (Meters3::new(700.0, -240.0, -4_500.0), Meters(20.0)),
    (Meters3::new(200.0, 300.0, -4_100.0), Meters(28.0)),
    (Meters3::new(-100.0, -200.0, -3_500.0), Meters(25.0)),
    (Meters3::new(-200.0, 260.0, -2_800.0), Meters(35.0)),
    (Meters3::new(-50.0, -220.0, -2_100.0), Meters(24.0)),
    (Meters3::new(600.0, 300.0, -1_800.0), Meters(30.0)),
    (Meters3::new(1_300.0, -250.0, -1_600.0), Meters(18.0)),
    (Meters3::new(2_000.0, 240.0, -1_750.0), Meters(28.0)),
    (Meters3::new(2_500.0, -300.0, -2_200.0), Meters(22.0)),
    (Meters3::new(2_600.0, 260.0, -2_900.0), Meters(31.0)),
    (Meters3::new(2_350.0, -220.0, -3_500.0), Meters(19.0)),
    (Meters3::new(1_800.0, 300.0, -3_950.0), Meters(26.0)),
    (Meters3::new(1_100.0, -260.0, -4_000.0), Meters(22.0)),
    (Meters3::new(500.0, 260.0, -3_600.0), Meters(30.0)),
    (Meters3::new(300.0, -280.0, -3_000.0), Meters(20.0)),
    (Meters3::new(450.0, 320.0, -2_400.0), Meters(32.0)),
    (Meters3::new(1_200.0, -300.0, -2_700.0), Meters(24.0)),
    (Meters3::new(850.0, 80.0, -2_100.0), Meters(24.0)),
    (Meters3::new(1_450.0, -60.0, -2_100.0), Meters(20.0)),
    (Meters3::new(1_900.0, 100.0, -2_300.0), Meters(26.0)),
    (Meters3::new(750.0, -100.0, -2_700.0), Meters(18.0)),
    (Meters3::new(1_550.0, 80.0, -2_700.0), Meters(28.0)),
    (Meters3::new(2_050.0, -80.0, -2_850.0), Meters(20.0)),
    (Meters3::new(800.0, 120.0, -3_250.0), Meters(22.0)),
    (Meters3::new(1_400.0, -100.0, -3_350.0), Meters(20.0)),
    (Meters3::new(1_950.0, 100.0, -3_300.0), Meters(24.0)),
    (Meters3::new(1_050.0, 60.0, -2_400.0), Meters(18.0)),
    (Meters3::new(1_650.0, -120.0, -2_450.0), Meters(22.0)),
    (Meters3::new(1_150.0, 120.0, -3_000.0), Meters(19.0)),
];

/// The far dressing: bigger rocks strung around the whole playable volume, so
/// the belt reads as a belt from anywhere in it. None of them is on a route.
pub(crate) const AMBIENT_ROCKS: [(Meters3, Meters); 20] = [
    (Meters3::new(-6_000.0, 1_000.0, -1_000.0), Meters(55.0)),
    (Meters3::new(-4_200.0, -900.0, -2_500.0), Meters(40.0)),
    (Meters3::new(-2_500.0, 1_300.0, -1_500.0), Meters(65.0)),
    (Meters3::new(-1_800.0, -1_100.0, -3_800.0), Meters(35.0)),
    (Meters3::new(500.0, 1_000.0, -2_500.0), Meters(45.0)),
    (Meters3::new(1_600.0, -900.0, -1_200.0), Meters(60.0)),
    (Meters3::new(3_200.0, 1_300.0, -2_700.0), Meters(38.0)),
    (Meters3::new(5_200.0, -1_000.0, -2_000.0), Meters(70.0)),
    (Meters3::new(7_000.0, 700.0, -3_500.0), Meters(42.0)),
    (Meters3::new(8_000.0, -1_200.0, -5_000.0), Meters(55.0)),
    (Meters3::new(-8_000.0, 1_500.0, -3_500.0), Meters(48.0)),
    (Meters3::new(-8_500.0, -1_000.0, -8_000.0), Meters(75.0)),
    (Meters3::new(-6_500.0, 1_800.0, -10_000.0), Meters(44.0)),
    (Meters3::new(-2_500.0, -1_500.0, -10_000.0), Meters(62.0)),
    (Meters3::new(1_000.0, 1_700.0, -9_500.0), Meters(40.0)),
    (Meters3::new(3_500.0, -1_400.0, -10_000.0), Meters(68.0)),
    (Meters3::new(6_500.0, 1_600.0, -9_500.0), Meters(50.0)),
    (Meters3::new(8_500.0, -800.0, -8_000.0), Meters(72.0)),
    (Meters3::new(9_000.0, 1_200.0, -6_000.0), Meters(46.0)),
    (Meters3::new(7_000.0, -1_600.0, -7_000.0), Meters(58.0)),
];

/// Scenario ids for the two fixed bodies. Both chapters spawn them under these
/// names, so a marker, an orbit order or a lock reads the same in either.
pub(crate) const ID_INSPECTION: &str = "inspection_planetoid";
pub(crate) const ID_CONCEALMENT: &str = "concealment_planetoid";

/// The beacon ink both chapters navigate by.
pub(crate) const BEACON_COLOR: Color = Color::srgb(0.3, 0.9, 1.0);

/// Beacon trigger radius. It MUST contain the autopilot's park point: GOTO
/// stops 500 m short of an unsized target, and a smaller trigger would leave
/// a ship parked outside the objective it just flew to.
pub(crate) const BEACON_AREA_RADIUS: Meters = Meters(700.0);

/// A lit navigation mark with a trigger volume around it, sized for a leg the
/// autopilot flies.
pub(crate) fn beacon(id: &str, label: &str, position: Meters3) -> ScenarioObjectConfig {
    sized_beacon(id, label, position, BEACON_AREA_RADIUS, None)
}

/// The same mark with the trigger volume and the radar signature spelled out.
///
/// A hand-flown mark wants a TIGHT volume - a lesson that asks the player to
/// place the hull cannot be passed by drifting through a 700 m sphere. A long
/// authored leg can also override the global beacon radar signature.
pub(crate) fn sized_beacon(
    id: &str,
    label: &str,
    position: Meters3,
    area_radius: Meters,
    lock_signature: Option<Meters>,
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
            radius: Meters(20.0),
            color: BEACON_COLOR,
            area_radius: Some(area_radius),
            lock_signature,
        }),
    }
}

/// One belt rock: collidable, destructible, and carrying no well of its own.
pub(crate) fn rock(
    id: &str,
    name: &str,
    position: Meters3,
    radius: Meters,
    texture: &AssetRef<Image>,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            material: None,
            destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
            radius,
            texture: texture.clone(),
            mass: None,
            invulnerable: false,
            seed: None,
            lock_signature: None,
        }),
    }
}

/// One of the two planetoids: a gravity source, and indestructible, because
/// both chapters are authored against it still being there.
///
/// A PLANET, not a big rock. The ids, the masses and the invulnerable flag are
/// unchanged, and the radii are chosen to reproduce the body radius each rock
/// published (see [`INSPECTION_RADIUS`]), so this is a look change and not a
/// gameplay one.
pub(crate) fn planetoid(
    id: &str,
    name: &str,
    position: Meters3,
    radius: Meters,
    mass: f32,
    planet_type: PlanetType,
    seed: u32,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Planet(
            PlanetConfig::new(planet_type, radius, seed).anchored(mass),
        ),
    }
}

/// The GEOMETRIC surface of an authored planetoid: the radius the sim
/// measures from, and the one every clearance rule is written against.
///
/// A rock only ever offered a RANGE here, because its noise mesh reaches
/// 3.5-6.0 times past its nominal radius depending on the seed, so a route had
/// to clear `nominal * ASTEROID_GEOMETRIC_FACTOR_MAX` and hope. A planet is
/// exact: `radius * (1 + relief)`, whatever the seed. Corridor and well tests
/// read this instead of guessing at a worst case.
#[cfg(test)]
pub(crate) fn planetoid_body_radius(planet_type: PlanetType, radius: Meters) -> Meters {
    PlanetConfig::new(planet_type, radius, 0).body_radius()
}

/// The inspection body's geometric surface (see [`planetoid_body_radius`]).
#[cfg(test)]
pub(crate) fn inspection_body_radius() -> Meters {
    planetoid_body_radius(INSPECTION_TYPE, INSPECTION_RADIUS)
}

/// The concealment body's geometric surface (see [`planetoid_body_radius`]).
#[cfg(test)]
pub(crate) fn concealment_body_radius() -> Meters {
    planetoid_body_radius(CONCEALMENT_TYPE, CONCEALMENT_RADIUS)
}

/// Every fixed body of the belt: both planetoids, the rock plate, the far
/// dressing. The chapter adds its own ships, marks and cargo on top.
pub(crate) fn belt(texture: &AssetRef<Image>) -> Vec<ScenarioObjectConfig> {
    let mut objects = vec![
        planetoid(
            ID_INSPECTION,
            "Inspection Planetoid",
            INSPECTION_POS,
            INSPECTION_RADIUS,
            INSPECTION_MASS,
            INSPECTION_TYPE,
            INSPECTION_SEED,
        ),
        planetoid(
            ID_CONCEALMENT,
            "Concealment Planetoid",
            CONCEALMENT_POS,
            CONCEALMENT_RADIUS,
            CONCEALMENT_MASS,
            CONCEALMENT_TYPE,
            CONCEALMENT_SEED,
        ),
    ];
    for (index, (position, radius)) in SALVAGE_ROCKS.into_iter().enumerate() {
        objects.push(rock(
            &format!("salvage_rock_{index}"),
            &format!("Salvage Rock {}", index + 1),
            position,
            radius,
            texture,
        ));
    }
    for (index, (position, radius)) in AMBIENT_ROCKS.into_iter().enumerate() {
        objects.push(rock(
            &format!("ambient_rock_{index}"),
            &format!("Belt Rock {}", index + 1),
            position,
            radius,
            texture,
        ));
    }
    objects
}
