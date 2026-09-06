//! The fixed belt the First Shift map bench is laid out in: two planetoids, a
//! working rock plate between them and the carrier, and the far dressing
//! around all of it.
//!
//! This is BENCH content, not shipped content. It sits here so the layout is
//! reviewed and iterated beside the example that draws it, rather than as a
//! scenario the game loads.

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// The carrier's berth: the near end of every sight line the bench frames.
pub const CARRIER_POS: Meters3 = Meters3::new(-1_000.0, 0.0, 2_500.0);

/// The small planetoid the inspection round is flown against.
pub const INSPECTION_POS: Meters3 = Meters3::new(-4_500.0, -400.0, -6_500.0);
/// Its MEAN radius.
///
/// A planet's mesh stands only `1 + relief` off its radius, so the number
/// carries the size itself: 950 m of mean radius puts the surface at 997.5 m,
/// which is the body the route, the 3.29 km sphere of influence and the ORBIT
/// autopilot's ring were all framed against.
pub const INSPECTION_RADIUS: Meters = Meters(950.0);
/// A dust world: ochre and mundane, and instantly distinct from the dark rock
/// across the belt.
pub const INSPECTION_TYPE: PlanetType = PlanetType::DustWorld;
/// Draws a wide ochre plain under a pale frost cap, with the basin turned
/// toward the approach lane the bench flies in on.
pub const INSPECTION_SEED: u32 = 7;
/// Its mass parameter (mu, u^3/s^2): a 3.29 km sphere of influence with an
/// escapable surface pull.
pub const INSPECTION_MASS: f32 = 27_000.0;

/// The large planetoid on the far side of the belt. It exists to HIDE
/// something: at two and a half times the inspection body's radius, its hull
/// is opaque to anything parked behind it.
pub const CONCEALMENT_POS: Meters3 = Meters3::new(4_500.0, 300.0, -6_500.0);
/// Its MEAN radius, on the same footing as [`INSPECTION_RADIUS`]: 2 250 m puts
/// the surface at 2 373.8 m.
pub const CONCEALMENT_RADIUS: Meters = Meters(2_250.0);
/// Barren rock: airless grey stone, no cap, no colour. It exists to be a wall,
/// and it reads as one beside the dust world.
pub const CONCEALMENT_TYPE: PlanetType = PlanetType::BarrenRock;
/// Draws dark mare against pale highland, so the silhouette stays legible at
/// belt range without the body ever looking inviting.
pub const CONCEALMENT_SEED: u32 = 3;
/// Deliberately WEAKER than the inspection body despite being larger: the
/// navigation leg is framed against ONE well, and a second one reaching into
/// the route would read as noise.
pub const CONCEALMENT_MASS: f32 = 20_000.0;

/// The rock plate between the carrier and both planetoids: broad enough that a
/// cutter has several lines through it, and tight enough that nothing
/// capital-sized would try. The bench's crates sit in it.
pub const SALVAGE_ROCKS: [(Meters3, Meters); 40] = [
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
    // Fill the former bowl so the field reads as a plate, not a ring.
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

/// The far dressing: bodies strung kilometres out to give the belt depth,
/// outside any line the bench flies.
pub const AMBIENT_ROCKS: [(Meters3, Meters); 20] = [
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

/// What the working field is MADE of, as relative parts.
///
/// This is the plate a salvage run happens in, so it is the one that has to
/// carry an ore fiction: mostly ordinary stone, a solid minority of dark
/// carbonaceous bodies, some ice, and metal at one part in twenty. Rare is the
/// point of the metal - a belt where every fourth rock is nickel-iron has
/// nothing left to find.
pub const SALVAGE_MIX: [(&str, u32); 4] = [
    (KIND_ROCK, 12),
    (KIND_CARBON, 5),
    (KIND_ICE, 2),
    (KIND_METAL, 1),
];

/// What the FAR dressing is made of.
///
/// Kilometres out, so hue differences wash out and only the two ends of the
/// value range survive: stone against near-black carbon, with the odd bright
/// ice body to keep the far ring from reading as one grey band. No metal - a
/// metal body strung outside the playable volume is a promise nothing keeps.
pub const AMBIENT_MIX: [(&str, u32); 3] = [(KIND_ROCK, 6), (KIND_CARBON, 3), (KIND_ICE, 1)];

/// Scenario ids for the two fixed bodies, so a marker or an orbit order in the
/// bench names the same thing the belt spawns.
pub const ID_INSPECTION: &str = "inspection_planetoid";
pub const ID_CONCEALMENT: &str = "concealment_planetoid";

/// One belt rock: collidable, destructible, and carrying no well of its own.
///
/// `kind` is what it is made of - the id that decides how it is shaded. Drawn
/// per body from a mix rather than authored per rock, because the belt is 60
/// bodies and the interesting fact about it is the PROPORTION, not which
/// particular rock is ice.
fn rock(
    id: &str,
    name: &str,
    position: Meters3,
    radius: Meters,
    kind: &str,
    texture: &Handle<Image>,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            material: kind.to_string(),
            destroy_sound: Some(AssetRef::from("base/sounds/destroy_rock.wav")),
            radius,
            texture: texture.clone().into(),
            mass: None,
            invulnerable: false,
            seed: None,
            lock_signature: None,
        }),
    }
}

/// One of the two planetoids: a gravity source, and anchored, because the map
/// is laid out against it still being there.
///
/// A PLANET, not a big rock: the radius is exact, so `radius * (1 + relief)`
/// is the surface every clearance rule is written against.
fn planetoid(
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

/// Every fixed body of the belt: both planetoids, the rock plate, the far
/// dressing. The bench adds its own ships, marks and cargo on top.
pub fn belt(texture: &Handle<Image>) -> Vec<ScenarioObjectConfig> {
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
            asteroid_kind_at(&SALVAGE_MIX, index).expect("the salvage mix has weight"),
            texture,
        ));
    }
    for (index, (position, radius)) in AMBIENT_ROCKS.into_iter().enumerate() {
        objects.push(rock(
            &format!("ambient_rock_{index}"),
            &format!("Belt Rock {}", index + 1),
            position,
            radius,
            asteroid_kind_at(&AMBIENT_MIX, index).expect("the ambient mix has weight"),
            texture,
        ));
    }
    objects
}
