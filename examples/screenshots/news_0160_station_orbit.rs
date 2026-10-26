//! news_0160_station_orbit: the v0.16.0 news loop of a generated ring
//! station riding a planetoid's well with two generated hulls docked at its
//! ports, its turrets tracking and firing on a hostile hull that passes it
//! (`news-0160-station-orbit.webm`).
//!
//! A STAGED fixture, not a seeded world cluster. The station is the open
//! world's generator run as a ring at [`STATION_ADVANCEMENT`], which gives it
//! three docking ports; it is handed two hulls for sale, each the open world's
//! ship generator at that advancement, and parks and docks them as a world
//! station parks its catalogue, one port kept free. The planetoid is the one
//! `system_station_orbit` scales for a short walk: 200 m with its mass at the
//! surface-gravity cap, so the 550 m orbit takes about 40 s.
//!
//! The docked assembly is unusual: two hulls rigidly docked to a small ring
//! load its root far off the centre of mass, so the root does not keep its
//! post exactly. Over the loop it strays 9% to 10% in radius and sweeps 6% to
//! 8% ahead of the circular rate. The walk logs those departures and does not
//! assert a circular orbit.
//!
//! The station flies the player's side, and the contact is an undriven
//! Enemy hull from the same ship generator. It waits 40 km out, past every AI
//! sensor's reach, until the loop opens; then it is written onto a circular
//! orbit [`CONTACT_ORBIT_RADIUS`] out, tilted [`CONTACT_INCLINATION_RAD`] off
//! the station's plane, at the phase that brings it past the station
//! [`PASS_AT_SECS`] later. From there the station's own sensor pass, target
//! pick, turret aim and fire do the work; the walk only reads them.
//!
//! The loop opens when the station reaches [`loop_open_phase`], so the arc it
//! rides in the loop is the one centred on the camera, whatever the load
//! took. The camera stands still outside the contact's orbit and above the
//! station's plane, the planetoid mid-frame and the station crossing below it.
//!
//! | # | claim |
//! | - | - |
//! | 1 | the station wears the ring style and is posted on an orbit of the planetoid at the radius it was placed at |
//! | 2 | its two generated hulls are parked and docked, each through a connection to a different station port |
//! | 3 | over the loop the station sweeps forward about the planetoid and no sampled tick's sweep runs back |
//! | 4 | every sampled tick both hulls stay docked at two distinct ports |
//! | 5 | a station turret selects the contact ([`TurretSectionTargetEntity`]) and its muzzle holds within [`TRACK_CONE_RAD`] of the contact while it swings at least [`TRACK_SWING_RAD`] |
//! | 6 | turret fire strips the contact: at the loop's close it has fewer live sections than at the open |
//!
//! The contact is neutralized ([`NeutralizedMarker`]) about 4 s into the loop
//! on the smoke path, but under the capture clock it survives the loop
//! damaged. The walk logs a neutralization when it sees one and does not
//! assert it.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write
//!   `news-0160-station-orbit.webm` (staged under `NOVA_CAPTURE_DIR`).
//!
//! Either way the wait for the loop's phase can take up to one orbit of
//! world time, past the 120 s completion backstop; raise
//! `NOVA_AUTOPILOT_DEADLINE` above the 1620 s the steps' own deadlines sum to,
//! so a stalled step fails under its own name:
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example news_0160_station_orbit --features debug
//! # look for: `station orbit: the ring is posted`,
//! #           `station orbit: the pass held`
//! ```

use std::{
    collections::{BTreeMap, BTreeSet},
    f32::consts::{FRAC_PI_2, PI, TAU},
    sync::Arc,
};

use avian3d::prelude::{AngularVelocity, ComputedCenterOfMass, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use clap::Parser;
use generated_station::generated_station_design;
use nova_gameplay::prelude::CivilizationId;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[path = "../shared/generated_station.rs"]
mod generated_station;

#[derive(Parser)]
#[command(name = "news_0160_station_orbit")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 station loop: a generated ring station in orbit with two docked hulls, its turrets on a passing hull",
    long_about = None
)]
struct Cli;

/// The loop this producer writes.
const LOOP_NAME: &str = "news-0160-station-orbit";

/// The scenario id the stage loads under.
const SCENARIO_ID: &str = "news_0160_station_orbit";

/// The planetoid's mean radius, mass in engine units and seed: the body
/// `system_station_orbit` scales for a short walk.
const PLANET_RADIUS: Meters = Meters(200.0);
const PLANET_MASS: f32 = 4000.0;
const PLANET_SEED: u32 = 7;

/// The planetoid's outer surface: BarrenRock relief on [`PLANET_RADIUS`], as
/// `system_station_orbit` measures it.
const PLANET_OUTER_RADIUS: Meters = Meters(211.0);

/// Where the well's pull starts to fade: past it an orbit is not circular.
const WELL_FADE_RADIUS: Meters = Meters(1075.0);

/// The station's orbit radius: past the planetoid's surface by more than the
/// station and its docked hulls reach.
const ORBIT_RADIUS: Meters = Meters(550.0);

/// The advancement the ring is generated at: `2 + floor(6 * 0.25)` is three
/// docking ports, two hulls for sale and the player's free one.
const STATION_ADVANCEMENT: f32 = 0.25;

/// The seed the ring and its hulls are generated from.
const STATION_SEED: u32 = 3;

/// The hulls the station sells: one at each port but one.
const TENDERS: usize = 2;

/// The roles of the hulls for sale, in park order. Both are roles every
/// civilization builds.
const TENDER_ROLES: [ShipRoleType; TENDERS] = [ShipRoleType::Civilian, ShipRoleType::Industrial];

/// The contact's role: the same always-buildable hull as the second tender,
/// laid out on a seed of its own.
const CONTACT_ROLE: ShipRoleType = ShipRoleType::Industrial;

/// The first hull seed each generated hull tries, and how many it tries.
/// A hull takes the first seed that lays out within [`HULL_CLEARANCE`].
const FIRST_HULL_SEED: u32 = 1;
const HULL_SEED_SEARCH: u32 = 32;

/// The largest clearance radius a generated hull here may have: small hulls
/// keep the docked assembly clear of the planetoid and of the contact. The
/// generator lays out no civilian or industrial hull under 80 m at
/// [`STATION_ADVANCEMENT`], and each role's seed slot holds one under this.
const HULL_CLEARANCE: Meters = Meters(87.0);

/// The contact's orbit radius and its tilt off the station's orbit plane.
/// The tilt carries it over the station's equator at the pass, where the
/// turrets on the ring's upper face can reach it.
const CONTACT_ORBIT_RADIUS: Meters = Meters(900.0);
const CONTACT_INCLINATION_RAD: f32 = 0.35;

/// Where the contact waits before the loop: 40 km below the planetoid,
/// past the 20 km AI sensor reach and out of the well.
const CONTACT_PARK: Meters3 = Meters3::new(0.0, -40_000.0, 0.0);

/// World seconds from the loop's open to the contact's pass of the station.
const PASS_AT_SECS: f32 = 6.0;

/// World seconds the loop records: 420 frames at the 30 fps capture clock,
/// under the 600-frame cap. The turrets fire on the contact for most of it.
const LOOP_SECS: f32 = 14.0;

/// How far past the loop's phase the station may be when the loop opens,
/// radians. Wider than one frame's sweep at the quarter-second clamp.
const OPEN_WINDOW_RAD: f32 = 0.15;

/// Where the player ship is parked: above the orbit plane, outside the
/// well's sphere of influence and out of frame. It only watches.
const PLAYER_POSITION: Meters3 = Meters3::new(0.0, 1500.0, 0.0);

/// The camera: on +Z, where the three-point rig is lit from, outside the
/// contact's orbit and about 30 degrees above the station's plane, so the
/// ring and its two docked hulls read apart instead of edge-on. The
/// planetoid sits mid-frame with the station crossing below it and the
/// contact above.
const CAMERA_POSITION: Meters3 = Meters3::new(0.0, 500.0, 1400.0);
const CAMERA_LOOK_AT: Meters3 = Meters3::new(0.0, 80.0, 220.0);

/// How far the station's posted orbit radius may differ from the authored
/// [`ORBIT_RADIUS`], as a fraction.
const POST_TOLERANCE: f32 = 0.02;

/// The fewest ticks the loop must sample.
const MIN_SAMPLES: u32 = 30;

/// How far a muzzle may point off the contact and still count as on it,
/// radians: wide enough for the lead the turret solves at the pass's
/// closing speed.
const TRACK_CONE_RAD: f32 = 0.15;

/// How far a muzzle must swing while on the contact, radians: a measurable
/// aim change, not a set arc.
const TRACK_SWING_RAD: f32 = 0.1;

/// Real seconds the load and the docking each get on a software rasterizer.
const SETUP_SECS: f32 = 120.0;

/// Real seconds the station gets to reach the loop's phase: at most one
/// orbit of world time.
const PHASE_SECS: f32 = 300.0;

/// Real seconds the loop gets to record its world time.
const LOOP_STEP_SECS: f32 = 900.0;

/// Real seconds the loop gets to encode once its last frame is in.
const LOOP_CLOSE_SECS: f32 = 60.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_systems(Update, watch_the_pass.run_if(resource_exists::<PassWatch>));
    app.add_plugins(orbit_script());
    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_stage);
    app.add_observer(offer_the_tenders);
}

/// The station's angular rate on its orbit, radians per world second.
fn station_rate() -> f32 {
    let radius = ORBIT_RADIUS.to_engine();
    circular_orbit_speed(PLANET_MASS, radius) / radius
}

/// The orbit phase the loop opens at, radians from +X toward +Z: the loop's
/// arc is centred on +Z, under the camera.
fn loop_open_phase() -> f32 {
    FRAC_PI_2 - station_rate() * LOOP_SECS * 0.5
}

/// The unit radial at orbit phase `phase` in the station's plane.
fn radial_at(phase: f32) -> Vec3 {
    Vec3::new(phase.cos(), 0.0, phase.sin())
}

/// The orbit phase of `radial`, radians from +X toward +Z.
fn phase_of(radial: Vec3) -> f32 {
    radial.z.atan2(radial.x)
}

/// The hulls the station sells, laid out at setup and handed over once the
/// station root is up.
#[derive(Resource)]
struct TenderOffers(Vec<StationHullOffer>);

/// The first hull seed that lays out `role` for `civilization` at
/// [`STATION_ADVANCEMENT`] within [`HULL_CLEARANCE`].
///
/// # Panics
///
/// When no seed in the search does: the stage cannot stand its hulls.
fn small_hull(
    snapshot: &ShipPartSnapshot,
    civilization: CivilizationId,
    role: ShipRoleType,
    seeds: std::ops::Range<u32>,
) -> ShipLayout {
    let mut last = None;
    for seed in seeds.clone() {
        let request = ShipLayoutRequest {
            seed,
            civilization,
            role,
            advancement: STATION_ADVANCEMENT,
        };
        match generate_ship(snapshot, request) {
            Ok(layout) if layout.clearance.0 <= HULL_CLEARANCE.0 => {
                info!(
                    "station orbit: {} hull seed {seed}, {:.0} m clearance",
                    role.label(),
                    layout.clearance.0
                );
                return layout;
            }
            Ok(layout) => last = Some(format!("seed {seed}: {:.0} m", layout.clearance.0)),
            Err(failure) => last = Some(format!("seed {seed}: {failure}")),
        }
    }
    panic!(
        "station orbit: no {} hull seed in {seeds:?} lays out within {:.0} m (last {})",
        role.label(),
        HULL_CLEARANCE.0,
        last.unwrap_or_default()
    );
}

/// Generate the ring, its two hulls for sale and the contact, check that
/// the stage's orbits clear what they carry, and load the stage.
fn setup_stage(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    loaded: Res<LoadedSectionPacks>,
    items: Res<GameItems>,
) {
    let (layout, ring) = generated_station_design(
        &loaded,
        &items,
        StationArchetypeType::Ring,
        STATION_ADVANCEMENT,
        STATION_SEED,
    );
    assert_eq!(
        layout.ports,
        TENDERS + 1,
        "station orbit: the ring has {} ports, not one per hull for sale and a free one",
        layout.ports
    );
    let packs: Vec<ShipPartPack> = loaded
        .packs
        .iter()
        .map(|pack| ShipPartPack {
            id: pack.id.clone(),
            dependencies: pack.dependencies.clone(),
            sections: pack.sections.clone(),
        })
        .collect();
    let snapshot = ShipPartSnapshot::build(&packs)
        .unwrap_or_else(|faults| panic!("station orbit: the loaded parts are refused: {faults:?}"));
    // The civilization `generated_station_design` builds the ring for.
    let civilization = CivilizationId {
        world_seed: STATION_SEED,
        node: [0, 0, 0],
    };
    let seeds = |slot: u32| {
        let first = FIRST_HULL_SEED + slot * HULL_SEED_SEARCH;
        first..first + HULL_SEED_SEARCH
    };
    let tenders: Vec<ShipLayout> = TENDER_ROLES
        .iter()
        .zip(0..)
        .map(|(role, slot)| small_hull(&snapshot, civilization, *role, seeds(slot)))
        .collect();
    let contact = small_hull(&snapshot, civilization, CONTACT_ROLE, seeds(TENDERS as u32));

    // A hull docked at a port inside the station's clearance reaches at most
    // its own diameter past it.
    let reach = layout.clearance.0
        + 2.0
            * tenders
                .iter()
                .map(|tender| tender.clearance.0)
                .fold(0.0, f32::max);
    assert!(
        ORBIT_RADIUS.0 - reach > PLANET_OUTER_RADIUS.0,
        "station orbit: the docked ring reaches {reach:.0} m, into the planetoid from a {:.0} m orbit",
        ORBIT_RADIUS.0
    );
    let pass = (ORBIT_RADIUS.0.powi(2) + CONTACT_ORBIT_RADIUS.0.powi(2)
        - 2.0 * ORBIT_RADIUS.0 * CONTACT_ORBIT_RADIUS.0 * CONTACT_INCLINATION_RAD.cos())
    .sqrt();
    assert!(
        pass > reach + contact.clearance.0,
        "station orbit: the contact passes {pass:.0} m from the station, inside the {:.0} m the \
         two reach",
        reach + contact.clearance.0
    );
    assert!(
        CONTACT_ORBIT_RADIUS.0 + contact.clearance.0 < WELL_FADE_RADIUS.0,
        "station orbit: the contact's orbit reaches the well's fade"
    );
    info!(
        "station orbit: the ring has {} ports, {} turrets and a {:.0} m clearance; docked it \
         reaches {reach:.0} m and the contact passes {pass:.0} m out",
        layout.ports, layout.turrets, layout.clearance.0
    );

    let civilization_name = civilization.name();
    commands.insert_resource(TenderOffers(
        TENDER_ROLES
            .iter()
            .zip(tenders)
            .map(|(role, tender)| StationHullOffer {
                name: format!("{civilization_name} {} Mk 1", role.label()),
                terms: HullOfferType::Sale,
                design: tender.design,
            })
            .collect(),
    ));
    commands.trigger(LoadScenario(orbit_stage(
        &game_assets,
        &sections,
        ring,
        contact.design,
    )));
}

/// Hand the station its hulls for sale as the world's spawn does, so it
/// parks and docks one at each port but one once its sections are live.
fn offer_the_tenders(
    add: On<Add, StationMarker>,
    offers: Res<TenderOffers>,
    mut commands: Commands,
) {
    let hulls = offers
        .0
        .iter()
        .map(|offer| PendingParkedHull::Offer(Box::new(offer.clone())))
        .collect();
    commands
        .entity(add.entity)
        .insert(PendingParkedHulls(hulls));
}

/// Where the station spawns, engine units: half an orbit before the loop's
/// phase, so a load shorter than that opens the loop on the first pass.
fn station_start() -> Vec3 {
    radial_at(loop_open_phase() - PI) * ORBIT_RADIUS.to_engine()
}

/// The stage: the planetoid at the origin, the ring on its orbit, the player
/// above, the contact parked far below, lit by the three-point rig.
fn orbit_stage(
    game_assets: &GameAssets,
    sections: &GameSections,
    ring: StationDesign,
    contact: ShipDesign,
) -> ScenarioConfig {
    let planet = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "planet".to_string(),
            name: "PLANETOID".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Planet(PlanetConfig::new(
            PlanetType::BarrenRock,
            PLANET_RADIUS,
            PLANET_SEED,
            PLANET_MASS,
        )),
    };
    let initial_velocity = station_orbit_velocity(PLANET_MASS, Vec3::ZERO, station_start())
        .expect("station orbit: a station off the planetoid's centre gets an orbit");
    let station = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "ring".to_string(),
            name: "RING".to_string(),
            position: Meters3::from_engine(station_start()),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Station(StationConfig {
            design: StationDesignSource::Inline(ring),
            allegiance: Allegiance::Player,
            derelict: false,
            initial_velocity,
        }),
    };
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_spaceship".to_string(),
            name: "Player".to_string(),
            position: PLAYER_POSITION,
            rotation: Quat::from_rotation_x(-FRAC_PI_2),
        },
        kind: ScenarioObjectKind::Spaceship(player_ship(sections)),
    };
    let contact = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "contact".to_string(),
            name: "CONTACT".to_string(),
            position: CONTACT_PARK,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            design: ShipDesignSource::Inline(contact),
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Enemy),
            ..default()
        }),
    };
    ScenarioConfig {
        description: "A generated ring station in orbit of a planetoid with two hulls docked and \
                      a hostile hull passing."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    EventActionConfig::SpawnScenarioObject(planet),
                    EventActionConfig::SpawnScenarioObject(station),
                    EventActionConfig::SpawnScenarioObject(player),
                    EventActionConfig::SpawnScenarioObject(contact),
                ],
                ThreePointRig::around("rig", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "News 0.16.0 Station Orbit".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player's hull: a controller and a piece of armor out of the base
/// catalog. It only watches; the subject is the station.
fn player_ship(sections: &GameSections) -> SpaceshipConfig {
    fixtures::ship(
        sections,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
        }),
        &[
            SectionSpec::new("controller", "basic_controller_section", Vec3::ZERO),
            SectionSpec::new("hull", "reinforced_hull_section", Vec3::new(0.0, 0.0, 1.0)),
        ],
    )
}

fn when(
    check: impl Fn(&World) -> bool + Send + Sync + 'static,
) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(check)
}

/// The live station root, if one is up.
fn the_station(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<Entity, With<StationMarker>>()?
        .iter(world)
        .next()
}

/// The contact: the one Enemy ship root.
fn the_contact(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &Allegiance), (With<SpaceshipRootMarker>, Without<StationMarker>)>()?
        .iter(world)
        .find(|(_, allegiance)| **allegiance == Allegiance::Enemy)
        .map(|(entity, _)| entity)
}

/// The planetoid's well strength and centre.
fn the_well(world: &World) -> Option<(f32, Vec3)> {
    world
        .try_query_filtered::<(&GravityWell, &Position), With<PlanetMarker>>()?
        .iter(world)
        .next()
        .map(|(well, position)| (well.mu, position.0))
}

/// The hulls parked at `station`, in park order.
fn parked_hulls(world: &World, station: Entity) -> Vec<Entity> {
    world
        .get::<ParkedHulls>(station)
        .map(|parked| parked.iter().collect())
        .unwrap_or_default()
}

/// The station ports `station`'s parked hulls are docked at, one per hull
/// that is docked with it through a live connection.
fn docked_ports(world: &World, station: Entity) -> Vec<Entity> {
    parked_hulls(world, station)
        .into_iter()
        .filter_map(|hull| {
            let connection =
                world.get::<DockingConnection>(world.get::<DockedShip>(hull)?.connection)?;
            if connection.first_ship == station {
                Some(connection.first_section)
            } else if connection.second_ship == station {
                Some(connection.second_section)
            } else {
                None
            }
        })
        .collect()
}

/// Both hulls for sale are docked with the station, at two different ports.
fn two_distinct_docks(world: &World, station: Entity) -> bool {
    let ports = docked_ports(world, station);
    parked_hulls(world, station).len() == TENDERS
        && ports.len() == TENDERS
        && ports.iter().collect::<BTreeSet<_>>().len() == TENDERS
}

/// The station has parked its hulls and both docked at their own ports.
fn hulls_docked(world: &World) -> bool {
    the_station(world).is_some_and(|station| {
        world.get::<PendingParkedHulls>(station).is_none() && two_distinct_docks(world, station)
    })
}

/// The sections of `ship` still in its tree and not disabled.
fn live_sections(world: &World, ship: Entity) -> usize {
    world
        .try_query_filtered::<&ChildOf, (With<SectionMarker>, Without<IntegrityDisabledMarker>)>()
        .map_or(0, |mut sections| {
            sections
                .iter(world)
                .filter(|child_of| child_of.parent() == ship)
                .count()
        })
}

/// The station's physics position and velocity.
fn station_motion(world: &World, station: Entity) -> Option<(Vec3, Vec3)> {
    Some((
        world.get::<Position>(station)?.0,
        world.get::<LinearVelocity>(station)?.0,
    ))
}

/// The station is moving at more than half the circular speed: the spawn's
/// velocity reached the physics body.
fn station_under_way(world: &World) -> bool {
    let expected = circular_orbit_speed(PLANET_MASS, ORBIT_RADIUS.to_engine());
    the_station(world)
        .and_then(|station| station_motion(world, station))
        .is_some_and(|(_, velocity)| velocity.length() > expected * 0.5)
}

/// The station is at the loop's phase, or at most [`OPEN_WINDOW_RAD`] past it.
fn at_loop_phase(world: &World) -> bool {
    let (Some(station), Some((_, centre))) = (the_station(world), the_well(world)) else {
        return false;
    };
    station_motion(world, station).is_some_and(|(position, _)| {
        let past = (phase_of(position - centre) - loop_open_phase()).rem_euclid(TAU);
        past <= OPEN_WINDOW_RAD
    })
}

/// Invariants 1 and 2: the ring is posted on the orbit it was placed on and
/// both its generated hulls are docked at their own ports.
fn assert_ring_orbits(world: &mut World) {
    let station = the_station(world).expect("station orbit: the station must be up");
    let (mu, centre) = the_well(world).expect("station orbit: the planetoid must be up");
    assert!(
        (mu - PLANET_MASS).abs() < f32::EPSILON,
        "station orbit: the planetoid's well is {mu}, not the authored {PLANET_MASS}"
    );
    let ring = StationArchetypeType::Ring.style_id();
    let style = world
        .get::<ShipStyle>(station)
        .and_then(|style| style.0.clone());
    assert_eq!(
        style.as_deref(),
        Some(ring),
        "station orbit: the station wears style {style:?}, not the ring's '{ring}'"
    );
    let (position, velocity) =
        station_motion(world, station).expect("station orbit: the station has a physics body");
    let radius = (position - centre).length();
    let speed = velocity.length();
    let expected_radius = ORBIT_RADIUS.to_engine();
    let post = world
        .get::<StationPost>(station)
        .copied()
        .expect("station orbit: the station is posted by its first fixed tick");
    let StationPost::Orbit {
        centre: post_centre,
        radius: post_radius,
        ..
    } = post
    else {
        panic!("station orbit: a station in a well is posted on an orbit, got {post:?}");
    };
    assert!(
        post_centre.distance(centre) < f32::EPSILON
            && ((post_radius - expected_radius) / expected_radius).abs() < POST_TOLERANCE,
        "station orbit: the station is posted on {post:?}, not the {expected_radius:.2} u orbit"
    );
    assert!(
        two_distinct_docks(world, station),
        "station orbit: the station's {} parked hull(s) are not docked at {TENDERS} distinct \
         ports ({:?})",
        parked_hulls(world, station).len(),
        docked_ports(world, station)
    );
    info!(
        "station orbit: the ring is posted on a {post_radius:.2} u orbit, at {radius:.2} u and \
         {speed:.3} u/s, with {TENDERS} hulls docked at ports {:?}",
        docked_ports(world, station)
    );
}

/// Pose the camera and clear the chrome out of shot.
fn frame_the_orbit(world: &mut World) {
    hide_dev_overlays(world);
    hide_hud(world);
    hide_status_bar(world);
    pose_camera(world, CAMERA_POSITION, CAMERA_LOOK_AT);
}

/// Write the contact onto its tilted circular orbit at the phase that
/// brings it past the station [`PASS_AT_SECS`] from now, and start watching.
fn place_the_contact(world: &mut World) {
    let station = the_station(world).expect("station orbit: the station must be up");
    let contact = the_contact(world).expect("station orbit: the contact must be up");
    let (mu, centre) = the_well(world).expect("station orbit: the planetoid must be up");
    let (position, velocity) =
        station_motion(world, station).expect("station orbit: the station has a physics body");
    let radial = position - centre;
    let radius = radial.length();
    let station_out = radial / radius;
    let station_along = station_out.cross(Vec3::Y);
    assert!(
        velocity.dot(station_along) > 0.0,
        "station orbit: the station runs against the orbit sense the stage is laid out in"
    );
    // Where the station will be at the pass, and the contact's orbit plane:
    // the station's plane turned about the pass's line of motion, so the
    // contact stands highest over the station as it passes.
    let station_rate = velocity.length() / radius;
    let swept = station_rate * PASS_AT_SECS;
    let pass_out = station_out * swept.cos() + station_along * swept.sin();
    let pass_along = pass_out.cross(Vec3::Y);
    let tilted_out =
        pass_out * CONTACT_INCLINATION_RAD.cos() + Vec3::Y * CONTACT_INCLINATION_RAD.sin();
    let contact_radius = CONTACT_ORBIT_RADIUS.to_engine();
    let contact_speed = circular_orbit_speed(mu, contact_radius);
    let lag = -(contact_speed / contact_radius) * PASS_AT_SECS;
    let out = tilted_out * lag.cos() + pass_along * lag.sin();
    let along = pass_along * lag.cos() - tilted_out * lag.sin();
    world.entity_mut(contact).insert((
        Position(centre + out * contact_radius),
        Rotation(Transform::IDENTITY.looking_to(along, Vec3::Y).rotation),
        LinearVelocity(along * contact_speed),
        AngularVelocity(Vec3::ZERO),
    ));
    info!(
        "station orbit: the contact joins a {:.0} m orbit tilted {CONTACT_INCLINATION_RAD} rad, \
         {PASS_AT_SECS} s before its pass",
        CONTACT_ORBIT_RADIUS.0
    );
    let sections = live_sections(world, contact);
    world.insert_resource(PassWatch::new(
        world.resource::<Time>().elapsed_secs(),
        station,
        contact,
        sections,
        centre,
        radial,
        velocity,
    ));
}

/// One station turret's hold on the contact.
#[derive(Default)]
struct MountTrack {
    /// The muzzle's forward the first tick it was on the contact.
    first_on: Option<Vec3>,
    /// The widest angle the muzzle swung from there while on the contact.
    swing: f32,
    /// The ticks the turret had the contact selected.
    selected: u32,
    /// The ticks its muzzle was on the contact.
    on: u32,
}

/// What the loop has seen, tick by tick.
#[derive(Resource)]
struct PassWatch {
    station: Entity,
    contact: Entity,
    well: Vec3,
    /// World seconds at the open and at the last sample.
    opened: f32,
    last_sampled: f32,
    /// The orbit at the open: radius, speed and the plane's normal.
    radius: f32,
    speed: f32,
    normal: Vec3,
    last_radial: Vec3,
    /// The angle swept about the well since the open, radians.
    swept: f32,
    /// The most a tick's sweep ran back, radians.
    reversal: f32,
    /// The widest radius and speed departures from the open, as fractions.
    /// Logged, not asserted: the docked assembly does not hold a circle.
    radius_off: f32,
    speed_off: f32,
    /// The contact's live sections at the open and at the last sample.
    contact_sections_open: usize,
    contact_sections: usize,
    /// World seconds after the open the contact was first seen neutralized.
    neutralized_at: Option<f32>,
    /// The ticks sampled, and those that did not see both hulls docked at
    /// two distinct ports.
    samples: u32,
    loose_ticks: u32,
    mounts: BTreeMap<Entity, MountTrack>,
}

impl PassWatch {
    fn new(
        now: f32,
        station: Entity,
        contact: Entity,
        contact_sections: usize,
        well: Vec3,
        radial: Vec3,
        velocity: Vec3,
    ) -> Self {
        Self {
            station,
            contact,
            well,
            opened: now,
            last_sampled: now,
            radius: radial.length(),
            speed: velocity.length(),
            normal: radial.cross(velocity).normalize(),
            last_radial: radial.normalize(),
            swept: 0.0,
            reversal: 0.0,
            radius_off: 0.0,
            speed_off: 0.0,
            contact_sections_open: contact_sections,
            contact_sections,
            neutralized_at: None,
            samples: 0,
            loose_ticks: 0,
            mounts: BTreeMap::new(),
        }
    }
}

/// Sample the station's orbit, its docks and every one of its turrets
/// against the contact.
fn watch_the_pass(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs();
    let (station, contact, well) = {
        let watch = world.resource::<PassWatch>();
        (watch.station, watch.contact, watch.well)
    };
    let Some((position, velocity)) = station_motion(world, station) else {
        return;
    };
    let docks_held = two_distinct_docks(world, station);
    let neutralized = world.get::<NeutralizedMarker>(contact).is_some();
    let contact_sections = live_sections(world, contact);
    let anchor = match (
        world.get::<Position>(contact),
        world.get::<Rotation>(contact),
        world.get::<ComputedCenterOfMass>(contact),
    ) {
        (Some(position), Some(rotation), Some(center)) => Some(position.0 + rotation.0 * center.0),
        _ => None,
    };
    let mounts: Vec<(Entity, bool, Option<(Vec3, Vec3)>)> = world
        .query_filtered::<(
            Entity,
            &ChildOf,
            &TurretSectionTargetEntity,
            &TurretSectionMuzzleEntity,
        ), With<TurretSectionMarker>>()
        .iter(world)
        .filter(|(_, child_of, ..)| child_of.parent() == station)
        .map(|(turret, _, target, muzzle)| (turret, **target == Some(contact), **muzzle))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(turret, selected, muzzle)| {
            let pose = world
                .get::<GlobalTransform>(muzzle)
                .map(|pose| (pose.translation(), pose.forward().as_vec3()));
            (turret, selected, pose)
        })
        .collect();

    let mut watch = world.resource_mut::<PassWatch>();
    let radial = position - well;
    let radius = radial.length();
    let direction = radial / radius;
    let normal = watch.normal;
    let from = (watch.last_radial - normal * watch.last_radial.dot(normal)).normalize_or_zero();
    let to = (direction - normal * direction.dot(normal)).normalize_or_zero();
    let step = from.cross(to).dot(normal).atan2(from.dot(to));
    watch.swept += step;
    watch.reversal = watch.reversal.max(-step);
    watch.last_radial = direction;
    watch.last_sampled = now;
    watch.radius_off = watch
        .radius_off
        .max(((radius - watch.radius) / watch.radius).abs());
    watch.speed_off = watch
        .speed_off
        .max(((velocity.length() - watch.speed) / watch.speed).abs());
    watch.samples += 1;
    if !docks_held {
        watch.loose_ticks += 1;
    }
    watch.contact_sections = contact_sections;
    if neutralized && watch.neutralized_at.is_none() {
        watch.neutralized_at = Some(now - watch.opened);
    }
    for (turret, selected, pose) in mounts {
        let track = watch.mounts.entry(turret).or_default();
        if !selected {
            continue;
        }
        track.selected += 1;
        let (Some(anchor), Some((muzzle, forward))) = (anchor, pose) else {
            continue;
        };
        if muzzle_aim_error(forward, muzzle, anchor) < TRACK_CONE_RAD {
            track.on += 1;
            let first = *track.first_on.get_or_insert(forward);
            track.swing = track.swing.max(first.angle_between(forward));
        }
    }
}

/// Invariants 3 to 6: over the loop the ring swept forward and held both
/// docks, a turret selected the contact and swung with it, and the contact
/// lost sections.
fn assert_the_pass(world: &mut World) {
    let watch = world
        .remove_resource::<PassWatch>()
        .expect("station orbit: the loop was watched");
    assert!(
        watch.samples >= MIN_SAMPLES,
        "station orbit: the loop sampled {} tick(s), under {MIN_SAMPLES}",
        watch.samples
    );
    assert!(
        watch.swept > 0.0,
        "station orbit: the station swept {:.4} rad over the loop, not forward",
        watch.swept
    );
    assert!(
        watch.reversal < 1e-4,
        "station orbit: a tick's sweep ran {:.5} rad back",
        watch.reversal
    );
    let circular = watch.speed / watch.radius * (watch.last_sampled - watch.opened);
    assert_eq!(
        watch.loose_ticks, 0,
        "station orbit: {} of {} ticks did not see both hulls docked at two distinct ports",
        watch.loose_ticks, watch.samples
    );
    let selected = watch
        .mounts
        .values()
        .filter(|track| track.selected > 0)
        .count();
    assert!(
        selected > 0,
        "station orbit: no station turret of {} selected the contact",
        watch.mounts.len()
    );
    let (best, track) = watch
        .mounts
        .iter()
        .max_by(|(_, a), (_, b)| a.swing.total_cmp(&b.swing))
        .expect("station orbit: the station has turrets");
    assert!(
        track.swing >= TRACK_SWING_RAD,
        "station orbit: the best turret {best:?} swung {:.3} rad on the contact ({} of {} \
         selected ticks on it), under {TRACK_SWING_RAD} rad",
        track.swing,
        track.on,
        track.selected
    );
    assert!(
        watch.contact_sections < watch.contact_sections_open,
        "station orbit: the contact kept {} of its {} live sections over the loop",
        watch.contact_sections,
        watch.contact_sections_open
    );
    let neutralized = watch.neutralized_at.map_or_else(
        || "not neutralized".to_string(),
        |at| format!("neutralized {at:.2} s in"),
    );
    let tracking = watch
        .mounts
        .values()
        .filter(|track| track.swing >= TRACK_SWING_RAD)
        .count();
    info!(
        "station orbit: the pass held - {:.3} rad swept forward (a circular orbit sweeps \
         {circular:.3}) over {} ticks, radius strayed up to {:.4} and speed up to {:.4}, both \
         hulls docked every tick, {selected} of {} turrets selected the contact and {tracking} \
         swung on it (best {best:?}, {:.3} rad), the contact kept {} of {} live sections, \
         {neutralized}",
        watch.swept,
        watch.samples,
        watch.radius_off,
        watch.speed_off,
        watch.mounts.len(),
        track.swing,
        watch.contact_sections,
        watch.contact_sections_open
    );
}

/// The walk: load, dock, check the orbit, frame it, wait for the loop's
/// phase, then record the pass.
fn orbit_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the planetoid, the ring, the player and the contact")
        .enter(GameStates::Loading)
        .until(when(|world| {
            the_station(world).is_some()
                && the_well(world).is_some()
                && the_contact(world).is_some()
        }))
        .deadline(SETUP_SECS)
        .add()
        .step("wait for the ring's hulls to dock")
        .until(when(hulls_docked))
        .deadline(SETUP_SECS)
        .add()
        .step("wait for the ring to move on its orbit")
        .until(when(station_under_way))
        .deadline(SETUP_SECS)
        .add()
        .step("assert the ring is posted on its orbit with both hulls docked")
        .on_enter(assert_ring_orbits)
        .add()
        .step("frame the orbit")
        .on_enter(frame_the_orbit)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("wait for the ring to reach the loop's phase")
        .until(when(at_loop_phase))
        .deadline(PHASE_SECS)
        .add()
        .step("send the contact past and open the station-orbit loop")
        .on_enter(|world: &mut World| {
            place_the_contact(world);
            loop_start(world, LOOP_NAME);
        })
        .add()
        .step("ride the orbit while the contact passes")
        .until(elapsed(LOOP_SECS))
        .deadline(LOOP_STEP_SECS)
        .add()
        .step("close the station-orbit loop")
        .on_enter(|world: &mut World| {
            assert_the_pass(world);
            loop_end(world, LOOP_NAME);
        })
        .until(loop_written(LOOP_NAME))
        .deadline(LOOP_CLOSE_SECS)
        .add()
}
