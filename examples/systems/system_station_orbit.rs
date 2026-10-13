//! system_station_orbit: a station is a body in a planet's well.
//!
//! The base waystation is spawned in a circular orbit of a small planet with
//! the velocity `station_orbit_velocity` gives it, and a player ship parked
//! outside the well fires one torpedo at it. The warhead goes off on the
//! station and takes hit points off it without killing it; a kick the size of
//! a ram spins the hull and its own flight computer levels it under gravity;
//! then the station rides three full orbits without its radius drifting.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: station rides its orbit` | the station moves at the circular speed of the planet's well, on the radius it was placed at |
//! | 2 | `outcome: station takes a torpedo` | a warhead detonates on the station and its hull pool drops below the hull it was built with; the root is not collapsed, marked at zero, disabled, neutralized or derelict |
//! | 3 | `outcome: station levels after a hit under gravity` | spun and shoved, the hull comes to rest within `STATION_LEVEL_TOLERANCE_RAD` of its parked attitude while the well pulls on it |
//! | 4 | `outcome: station holds its orbit` | after three orbits and the hit the radius never left `STATION_ORBIT_TOLERANCE` of where it started, the hull is still level and no pool changed since the torpedo |
//!
//! The planet is scaled for the walk, not for the world. A generated
//! planetoid is 500-1600 m with the mass that reaches 3.5 radii of influence
//! (`clusters.rs`); this one is 200 m with its mass at the surface-gravity
//! cap, which puts a 450 m orbit at a 30 s period so three orbits fit a
//! software-rendered run. The tolerances are the shipped constants, unscaled.
//!
//! A torpedo blast deals pressure and no impulse (`apply_blast_damage`), so
//! the hit that has to be levelled out is authored: the same 0.5 rad turn and
//! 0.3 rad/s spin `system_station_body` uses, plus a tangential delta-v of one
//! percent of orbital speed, which is what a ram leaves behind.
//!
//! The orbit the proof measures is not the kick alone. The pull is a force
//! held constant over each fixed step, so an orbit gains energy every step:
//! about 1.5 % of its binding energy per orbit here (the gain scales with the
//! angle swept per step, so a generated planetoid's slower orbit gains less).
//! Three orbits of that plus the kick's four percent is where the orbit
//! tolerance's budget goes.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`). The
//! walk is three orbits long, so raise the harness backstop:
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example system_station_orbit --features debug
//! # look for: `station probe: the waystation rides its orbit`,
//! #           `station probe: the waystation takes the torpedo`,
//! #           `station probe: the waystation levels under gravity`,
//! #           `station probe: the waystation holds its orbit`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use std::sync::Arc;
use std::{collections::BTreeMap, f32::consts::FRAC_PI_2};

#[cfg(feature = "debug")]
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_station_orbit")]
#[command(version = "1.0.0")]
#[command(about = "A station orbits a planet, takes a torpedo and a shove, levels and holds its orbit. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario id the range loads under.
const SCENARIO_ID: &str = "station_orbit_rig";

/// The planet's mean radius. A tenth of a small generated planetoid, see the
/// module doc for the scaling.
const PLANET_RADIUS: Meters = Meters(200.0);

/// The planet's well strength, engine units, as `PlanetConfig::mass` takes
/// it. Under the surface-gravity cap for this body (211 m outer radius at
/// BarrenRock relief gives `10 * 21.1^2 = 4452`), so the well keeps the
/// number whole and the orbit below is computed from the mass the pull uses.
const PLANET_MASS: f32 = 4000.0;

/// Which world the planet is. Required by the config; the terrain plays no
/// part here.
const PLANET_SEED: u32 = 7;

/// The orbit's radius from the planet's centre: 240 m above the outer
/// surface, inside the well's unfaded core (its sphere of influence is
/// 1265 m, the fade starts at 1075 m).
const ORBIT_RADIUS: Meters = Meters(450.0);

/// Where the player ship is parked: above the orbit plane, outside the
/// sphere of influence so it neither falls nor orbits, aimed down at the
/// planet so the bay's muzzle faces the station's whole orbit.
const PLAYER_POSITION: Meters3 = Meters3::new(0.0, 1500.0, 0.0);

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(nova_screenshot(orbit_script()));
    }

    app.run()
}

/// The scripted run. Every beat waits on a world value - the station moving,
/// the safety going hot, the warhead going off, the spin dying, the sweep
/// closing - so a slow frame delays the walk instead of truncating it. The
/// deadlines are real seconds on a software rasterizer and sized for it.
#[cfg(feature = "debug")]
fn orbit_script() -> Script {
    Script::new()
        // Every step re-presses whatever the script is holding down: the
        // safety reads a HELD button, and a press applied once on a step's
        // entry would be a press, not a hold.
        .input(hold_inputs)
        .step("load the planet, the station and the player")
        .enter(GameStates::Loading)
        .until(and(station_present(), planet_present()))
        .deadline(120.0)
        .add()
        .step("wait for the station to move on its orbit")
        .until(station_under_way())
        .deadline(60.0)
        .add()
        .step("assert the station rides its orbit")
        .on_enter(assert_station_orbits)
        .add()
        .step("raise the weapons")
        .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().combat = true)
        .until(weapons_are_hot())
        .deadline(60.0)
        .add()
        .step("fire one torpedo at the station")
        .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().fire = true)
        .until(torpedo_present())
        .deadline(60.0)
        .add()
        .step("release the trigger and wait for the warhead")
        .on_enter(release_the_trigger)
        .until(blast_seen())
        .deadline(600.0)
        .add()
        // The blast's pressure is queued and spent on later frames; give the
        // health store time to hear every hit before reading it.
        .step("let the pressure land")
        .until(frames(20))
        .add()
        .step("assert the station takes the torpedo")
        .on_enter(assert_station_damaged)
        .add()
        .step("kick the station as a ram would")
        .on_enter(kick_station)
        .until(station_turned())
        .deadline(60.0)
        .add()
        .step("wait for the spin to die")
        .until(station_settled())
        .deadline(600.0)
        .add()
        .step("assert the station levels under gravity")
        .on_enter(assert_station_level)
        .add()
        .step("ride three orbits")
        .until(three_orbits_swept())
        .deadline(1500.0)
        .add()
        // The script's last beat: `nova_screenshot` appends the capture beat
        // behind it, so the run ends on the assertion.
        .step("assert the station holds its orbit")
        .on_enter(assert_station_holds_orbit)
        .add()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    #[cfg(feature = "debug")]
    {
        app.init_resource::<OrbitEpoch>();
        app.init_resource::<HeldInput>();
        app.init_resource::<BlastLog>();
        app.add_systems(First, hold_the_walk_clock.before(bevy::time::TimeSystems));
        app.add_systems(Update, (aim_torpedoes_at_the_station, track_orbit));
        app.add_observer(note_the_blast);
    }
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(orbit_range(&game_assets, &sections)));
}

/// Where the station starts, engine units: on +X of the planet, which puts
/// its seeded tangent on +Z.
fn station_start() -> Vec3 {
    Vec3::new(ORBIT_RADIUS.to_engine(), 0.0, 0.0)
}

/// The range: the planet at the origin, the catalog waystation on its orbit
/// and the player ship above, lit by a three-point rig because the engine
/// spawns no light.
fn orbit_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let planet = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "planet".to_string(),
            name: "PLANET".to_string(),
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
        .expect("station probe: a station off the planet's centre gets an orbit");
    let station_position = Meters3::from_engine(station_start());
    let station = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "waystation".to_string(),
            name: "WAYSTATION".to_string(),
            position: station_position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Station(StationConfig {
            design: BLOCK_WAYSTATION_STATION_ID.into(),
            allegiance: Allegiance::Neutral,
            derelict: false,
            initial_velocity,
        }),
    };
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_spaceship".to_string(),
            name: "Player".to_string(),
            position: PLAYER_POSITION,
            // The hull's forward is -Z; this turns it onto -Y, at the planet.
            rotation: Quat::from_rotation_x(-FRAC_PI_2),
        },
        kind: ScenarioObjectKind::Spaceship(torpedo_ship(sections)),
    };
    ScenarioConfig {
        description: "The waystation in orbit of a small planet, under the player's bay."
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
                ],
                ThreePointRig::around("rig", station_position, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Station Orbit Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player's hull: a controller, a piece of armor and one Serpent bay out
/// of the base catalog, firing on Space. The bay carries no magazine: the
/// subject is the station, not the ammunition.
fn torpedo_ship(sections: &GameSections) -> SpaceshipConfig {
    fixtures::ship(
        sections,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::from([(
                "torpedo".to_string(),
                vec![KeyCode::Space.into(), GamepadButton::RightTrigger.into()],
            )]),
        }),
        &[
            SectionSpec::new("controller", "basic_controller_section", Vec3::ZERO),
            SectionSpec::new("hull", "reinforced_hull_section", Vec3::new(0.0, 0.0, 1.0)),
            // 1x1x2, centred at -1.5 so its two cells sit on the grid and its
            // aft socket mates the controller.
            SectionSpec::new("torpedo", "torpedo_section", Vec3::new(0.0, 0.0, -1.5)).unlimited(),
        ],
    )
}

/// How far the kick turns the hull off its parked attitude, radians, and the
/// angular speed it leaves it with, radians per second: the nudge
/// `system_station_body` grades the hold on, so the two proofs ask the same
/// recovery of the computer, here with the well pulling.
#[cfg(feature = "debug")]
const KICK_RAD: f32 = 0.5;
#[cfg(feature = "debug")]
const KICK_RAD_PER_SEC: f32 = 0.3;

/// The tangential delta-v the kick adds, as a fraction of orbital speed. One
/// percent swings the radius about four percent, inside the orbit tolerance
/// with room for the integrator, and is the order a ram leaves behind.
#[cfg(feature = "debug")]
const KICK_SPEED_FRACTION: f32 = 0.01;

/// How far off the parked attitude the kick must take the hull before the
/// hold is asked to recover it. Above [`STATION_LEVEL_TOLERANCE_RAD`], so a
/// hull that never moved cannot pass the recovery assert for free.
#[cfg(feature = "debug")]
const TURNED_RAD: f32 = 0.1;

/// The angular speed under which the hull counts as at rest.
#[cfg(feature = "debug")]
const REST_RAD_PER_SEC: f32 = 0.01;

/// How far the measured orbit may differ from the computed one at the start,
/// as a fraction: the speed the spawn wrote against the first fixed steps of
/// the pull, and the radius against the first steps of travel.
#[cfg(feature = "debug")]
const START_TOLERANCE: f32 = 0.02;

/// The Serpent's authored blast radius, engine units (300 m): how far from
/// the station's centre the warhead may go off and still be a hit on it.
#[cfg(feature = "debug")]
const BLAST_RADIUS: f32 = 30.0;

/// How much world time one frame may carry, seconds. A software rasterizer
/// takes seconds per frame; Bevy's quarter-second clamp would stretch three
/// 30 s orbits into a quarter-hour of frames. One second is 64 fixed steps,
/// well within what a frame can run, and nothing here reads a frame.
#[cfg(feature = "debug")]
const WALK_CLOCK_SECS: f32 = 1.0;

/// Hold the virtual clock's clamp at [`WALK_CLOCK_SECS`].
#[cfg(feature = "debug")]
fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
    let cell = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
    if time.max_delta() != cell {
        time.set_max_delta(cell);
    }
}

/// The inputs the script is holding down.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HeldInput {
    /// The combat stance the safety gates firing on.
    combat: bool,
    /// The trigger.
    fire: bool,
}

/// Re-press the held inputs for the frame.
#[cfg(feature = "debug")]
fn hold_inputs(world: &mut World, _elapsed: f32, _frame: u32) {
    let held = world.resource::<HeldInput>();
    let (combat, fire) = (held.combat, held.fire);
    if combat {
        drive_action(world, "combat_stance", InputPhase::Press);
    }
    if fire {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
    }
}

/// Let go of the trigger: both halves, because `hold_inputs` re-presses the
/// key every frame and `ButtonInput` keeps a pressed key down until it is
/// released, so clearing the hold alone left the bay firing at its rate for
/// the rest of the walk.
#[cfg(feature = "debug")]
fn release_the_trigger(world: &mut World) {
    world.resource_mut::<HeldInput>().fire = false;
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Space);
}

/// What the walk remembers between beats: the station, the orbit it was
/// placed on, the attitude it was parked at, the health of everything under
/// it, and what the orbit tracker has seen.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct OrbitEpoch {
    /// The station root the scenario spawned.
    spawned: Option<Entity>,
    /// The planet's centre, engine units.
    well: Vec3,
    /// The attitude the station was parked at.
    spawn_rotation: Quat,
    /// The orbit radius at the start, engine units.
    radius: f32,
    /// The orbit plane's normal: radial crossed with velocity at the start.
    normal: Vec3,
    /// Every pool under the station root: as it stood at spawn, then as the
    /// torpedo left it.
    health: BTreeMap<Entity, f32>,
    /// The radial direction at the last tracked frame.
    last_radial: Option<Vec3>,
    /// The angle swept about the well since the start, radians.
    swept: f32,
    /// The widest and the narrowest radius seen since the start.
    radius_min: f32,
    radius_max: f32,
}

/// Every warhead that went off, by how far from the station's centre it did.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct BlastLog {
    blasts: Vec<f32>,
}

/// A station root exists.
#[cfg(feature = "debug")]
fn station_present() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    any_entity::<With<StationMarker>>()
}

/// The planet is up with its well.
#[cfg(feature = "debug")]
fn planet_present() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    any_entity::<(With<PlanetMarker>, With<GravityWell>)>()
}

/// The live station root, or `None` while none is up.
#[cfg(feature = "debug")]
fn live_station(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, With<StationMarker>>()
        .iter(world)
        .next()
}

/// The planet's well and centre.
#[cfg(feature = "debug")]
fn live_well(world: &mut World) -> Option<(GravityWell, Vec3)> {
    world
        .query_filtered::<(&GravityWell, &Position), With<PlanetMarker>>()
        .iter(world)
        .next()
        .map(|(well, position)| (well.clone(), position.0))
}

/// The station is moving at more than half the circular speed: the spawn's
/// velocity reached the physics body.
#[cfg(feature = "debug")]
fn station_under_way() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(mut stations) = world.try_query_filtered::<&LinearVelocity, With<StationMarker>>()
        else {
            return false;
        };
        let expected = circular_orbit_speed(PLANET_MASS, ORBIT_RADIUS.to_engine());
        stations
            .iter(world)
            .any(|velocity| velocity.0.length() > expected * 0.5)
    })
}

/// Every pool under `root`, including the root's own, keyed by entity.
#[cfg(feature = "debug")]
fn health_under(world: &World, root: Entity) -> BTreeMap<Entity, f32> {
    let mut pools = BTreeMap::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if let Some(health) = world.get::<Health>(node) {
            pools.insert(node, health.current);
        }
        if let Some(children) = world.get::<Children>(node) {
            stack.extend(children.iter());
        }
    }
    pools
}

/// Invariant 1: the station rides the orbit it was placed on - the well is
/// the authored one, the speed is the circular speed for it and the radius
/// is where the spawn put it.
#[cfg(feature = "debug")]
fn assert_station_orbits(world: &mut World) {
    let station = live_station(world).expect("station probe: the station must be up");
    let (well, centre) = live_well(world).expect("station probe: the planet must be up");
    assert!(
        (well.mu - PLANET_MASS).abs() < f32::EPSILON,
        "station probe: the planet's well is {} not the authored {PLANET_MASS}; the cap bit",
        well.mu
    );
    let root = world.entity(station);
    let position = root
        .get::<Position>()
        .map(|p| p.0)
        .expect("station probe: the station has a physics position");
    let velocity = root
        .get::<LinearVelocity>()
        .map(|v| v.0)
        .expect("station probe: the station has a physics velocity");
    let rotation = root
        .get::<Rotation>()
        .map(|r| r.0)
        .expect("station probe: the station has a physics rotation");
    let radial = position - centre;
    let radius = radial.length();
    let expected_radius = ORBIT_RADIUS.to_engine();
    let expected_speed = circular_orbit_speed(well.mu, expected_radius);
    let speed = velocity.length();
    assert!(
        ((radius - expected_radius) / expected_radius).abs() < START_TOLERANCE,
        "station probe: the station orbits at {radius:.2} u, not {expected_radius:.2} u"
    );
    assert!(
        ((speed - expected_speed) / expected_speed).abs() < START_TOLERANCE,
        "station probe: the station moves at {speed:.3} u/s, not the circular {expected_speed:.3} u/s"
    );
    assert!(
        velocity.dot(radial).abs() / (speed * radius) < START_TOLERANCE * 2.0,
        "station probe: the station's velocity {velocity:?} is not tangent to its radial {radial:?}"
    );
    let health = health_under(world, station);
    assert!(
        !health.is_empty(),
        "station probe: nothing under the station carries health"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!(
        "station probe: the waystation rides its orbit at {radius:.2} u and {speed:.3} u/s \
         with {} pools under it",
        health.len()
    );
    nova_probe::probe_marker(
        world,
        "outcome: station rides its orbit",
        serde_json::json!({ "t": elapsed, "radius": radius, "speed": speed, "mu": well.mu }),
    );
    let mut epoch = world.resource_mut::<OrbitEpoch>();
    epoch.spawned = Some(station);
    epoch.well = centre;
    epoch.spawn_rotation = rotation;
    epoch.radius = radius;
    epoch.normal = radial.cross(velocity).normalize();
    epoch.health = health;
    epoch.last_radial = Some(radial / radius);
    epoch.swept = 0.0;
    epoch.radius_min = radius;
    epoch.radius_max = radius;
}

/// Follow the station round the well from the epoch on: the angle it has
/// swept and the widest and narrowest radius it has had. Nothing here reads
/// a frame, so a slow frame samples a longer arc, not a different orbit.
#[cfg(feature = "debug")]
fn track_orbit(mut epoch: ResMut<OrbitEpoch>, q_station: Query<&Position, With<StationMarker>>) {
    let Some(station) = epoch.spawned else {
        return;
    };
    let Ok(position) = q_station.get(station) else {
        return;
    };
    let radial = position.0 - epoch.well;
    let radius = radial.length();
    let Some(direction) = radial.try_normalize() else {
        return;
    };
    if let Some(last) = epoch.last_radial {
        // The angle about the plane normal, so a wobble out of plane counts
        // for nothing and a reversal counts against the sweep.
        let normal = epoch.normal;
        let from = (last - normal * last.dot(normal)).normalize_or_zero();
        let to = (direction - normal * direction.dot(normal)).normalize_or_zero();
        epoch.swept += from.cross(to).dot(normal).atan2(from.dot(to));
    }
    epoch.last_radial = Some(direction);
    epoch.radius_min = epoch.radius_min.min(radius);
    epoch.radius_max = epoch.radius_max.max(radius);
}

/// The player ship's weapons safety has gone hot.
#[cfg(feature = "debug")]
fn weapons_are_hot() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<&WeaponsHot, With<PlayerSpaceshipMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|hot| hot.0))
    })
}

/// A torpedo is in flight.
#[cfg(feature = "debug")]
fn torpedo_present() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    any_entity::<With<TorpedoProjectileMarker>>()
}

/// Hand every fresh torpedo the station as its target. The player's own
/// targeting would mark a shot with no lock as dumb-fire, and a dumb-fire
/// torpedo never fuzes; the station is the one thing on this range to hit.
#[cfg(feature = "debug")]
fn aim_torpedoes_at_the_station(
    mut commands: Commands,
    q_torpedo: Query<Entity, (With<TorpedoProjectileMarker>, Without<TorpedoTargetChosen>)>,
    q_station: Query<Entity, With<StationMarker>>,
) {
    let Some(station) = q_station.iter().next() else {
        return;
    };
    for torpedo in &q_torpedo {
        commands
            .entity(torpedo)
            .insert((TorpedoTargetEntity(station), TorpedoTargetChosen));
    }
}

/// Record every warhead that goes off, and how far from the station it did.
#[cfg(feature = "debug")]
fn note_the_blast(
    add: On<Add, NovaBlast>,
    q_transform: Query<&Transform>,
    q_station: Query<&Position, With<StationMarker>>,
    mut log: ResMut<BlastLog>,
) {
    let at = q_transform
        .get(add.entity)
        .map_or(Vec3::NAN, |transform| transform.translation);
    let from_station = q_station
        .iter()
        .next()
        .map_or(f32::INFINITY, |position| position.0.distance(at));
    info!("station probe: a warhead went off at {at:?}, {from_station:.2} u from the station");
    log.blasts.push(from_station);
}

/// A warhead has gone off.
#[cfg(feature = "debug")]
fn blast_seen() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .get_resource::<BlastLog>()
            .is_some_and(|log| !log.blasts.is_empty())
    })
}

/// Invariant 2: the warhead went off on the station and took hit points off
/// it - the root's pool stands below the hull it was built with - but the
/// station is neither collapsed nor marked at zero, and it is still a living
/// station with a flight computer to level it. The pools as they stand now
/// are what the orbit beat later checks against.
#[cfg(feature = "debug")]
fn assert_station_damaged(world: &mut World) {
    let station = live_station(world).expect("station probe: the station must still be up");
    let blasts = world.resource::<BlastLog>().blasts.clone();
    let nearest = blasts.iter().copied().fold(f32::INFINITY, f32::min);
    assert!(
        nearest < BLAST_RADIUS,
        "station probe: no warhead went off on the station; the nearest was {nearest:.2} u \
         away of {:?}",
        blasts
    );
    let pool = world
        .get::<Health>(station)
        .map(|health| Health {
            current: health.current,
            max: health.max,
        })
        .expect("station probe: the station root carries the hull pool");
    assert!(
        pool.current < pool.max,
        "station probe: the torpedo took nothing off the station ({} of {})",
        pool.current,
        pool.max
    );
    let root = world.entity(station);
    assert!(
        !root.contains::<HealthZeroMarker>() && !root.contains::<StructuralCollapseMarker>(),
        "station probe: the station root was collapsed or marked at zero by the torpedo"
    );
    assert!(
        !root.contains::<IntegrityDisabledMarker>()
            && !root.contains::<NeutralizedMarker>()
            && !root.contains::<DerelictShipMarker>(),
        "station probe: the torpedo killed the station rather than damaging it"
    );
    assert!(
        world
            .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
            .iter(world)
            .next()
            .is_some(),
        "station probe: the player ship is gone"
    );
    let after = health_under(world, station);
    let lost = world
        .resource::<OrbitEpoch>()
        .health
        .keys()
        .filter(|node| !after.contains_key(node))
        .count();
    world.resource_mut::<OrbitEpoch>().health = after;
    let elapsed = world.resource::<Time>().elapsed_secs();
    let share = pool.current / pool.max;
    info!(
        "station probe: the waystation takes the torpedo ({} blast(s), nearest {nearest:.2} u, \
         pool {:.1} of {:.1} = {share:.3}, {lost} pool(s) gone)",
        blasts.len(),
        pool.current,
        pool.max
    );
    nova_probe::probe_marker(
        world,
        "outcome: station takes a torpedo",
        serde_json::json!({
            "t": elapsed,
            "blasts": blasts.len(),
            "nearest": nearest,
            "pool": pool.current,
            "pool_max": pool.max,
            "share": share,
            "pools_lost": lost,
        }),
    );
}

/// Turn the station, leave it spinning and shove it along its orbit: what a
/// ram leaves behind, written the way the engine's own teleports write it.
#[cfg(feature = "debug")]
fn kick_station(world: &mut World) {
    let station = live_station(world).expect("station probe: the station must be up");
    let spawn_rotation = world.resource::<OrbitEpoch>().spawn_rotation;
    let velocity = world
        .get::<LinearVelocity>(station)
        .map(|v| v.0)
        .expect("station probe: the station has a physics velocity");
    let shove = velocity * KICK_SPEED_FRACTION;
    world.entity_mut(station).insert((
        Rotation(spawn_rotation * Quat::from_rotation_y(KICK_RAD)),
        AngularVelocity(Vec3::new(0.0, KICK_RAD_PER_SEC, 0.0)),
        LinearVelocity(velocity + shove),
    ));
    info!(
        "station probe: kicked the waystation {KICK_RAD} rad off its attitude at \
         {KICK_RAD_PER_SEC} rad/s and {:.3} u/s along its orbit",
        shove.length()
    );
}

/// Angle between the live station's attitude and the one it was parked at.
#[cfg(feature = "debug")]
fn attitude_error(world: &World) -> Option<f32> {
    let epoch = world.get_resource::<OrbitEpoch>()?;
    let rotation = world.get::<Rotation>(epoch.spawned?)?;
    Some(rotation.0.angle_between(epoch.spawn_rotation))
}

/// The live station's angular speed.
#[cfg(feature = "debug")]
fn angular_speed(world: &World) -> Option<f32> {
    let epoch = world.get_resource::<OrbitEpoch>()?;
    Some(world.get::<AngularVelocity>(epoch.spawned?)?.0.length())
}

/// The kick took: the hull is clear of its parked attitude.
#[cfg(feature = "debug")]
fn station_turned() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| attitude_error(world).is_some_and(|error| error > TURNED_RAD))
}

/// The spin died. Where the hull came to rest is [`assert_station_level`]'s
/// question.
#[cfg(feature = "debug")]
fn station_settled() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| angular_speed(world).is_some_and(|speed| speed < REST_RAD_PER_SEC))
}

/// Invariant 3: the hull came to rest on its parked attitude, under the pull.
#[cfg(feature = "debug")]
fn assert_station_level(world: &mut World) {
    let error = attitude_error(world).expect("station probe: the station must be up");
    assert!(
        error < STATION_LEVEL_TOLERANCE_RAD,
        "station probe: the hull came to rest {error:.3} rad off its parked attitude \
         (tolerance {STATION_LEVEL_TOLERANCE_RAD}); the computer is not holding under gravity"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!("station probe: the waystation levels under gravity ({error:.4} rad off its attitude)");
    nova_probe::probe_marker(
        world,
        "outcome: station levels after a hit under gravity",
        serde_json::json!({ "t": elapsed, "error_rad": error }),
    );
}

/// The station has swept three full turns about the well since the epoch.
#[cfg(feature = "debug")]
fn three_orbits_swept() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .get_resource::<OrbitEpoch>()
            .is_some_and(|epoch| epoch.swept >= 3.0 * std::f32::consts::TAU)
    })
}

/// Invariant 4: over three orbits and the hit the radius stayed within
/// [`STATION_ORBIT_TOLERANCE`] of where it started, the hull is still level
/// and no pool under it changed since the torpedo.
#[cfg(feature = "debug")]
fn assert_station_holds_orbit(world: &mut World) {
    let station = live_station(world).expect("station probe: the station must still be up");
    let (radius, radius_min, radius_max, swept, before) = {
        let epoch = world.resource::<OrbitEpoch>();
        (
            epoch.radius,
            epoch.radius_min,
            epoch.radius_max,
            epoch.swept,
            epoch.health.clone(),
        )
    };
    let drift = ((radius_max - radius) / radius).max((radius - radius_min) / radius);
    assert!(
        drift < STATION_ORBIT_TOLERANCE,
        "station probe: the orbit drifted {drift:.3} of its radius ({radius_min:.2}..{radius_max:.2} \
         u about {radius:.2} u) over {swept:.2} rad (tolerance {STATION_ORBIT_TOLERANCE})"
    );
    let error = attitude_error(world).expect("station probe: the station must be up");
    assert!(
        error < STATION_LEVEL_TOLERANCE_RAD,
        "station probe: the hull is {error:.3} rad off its parked attitude after three orbits"
    );
    assert_eq!(
        health_under(world, station),
        before,
        "station probe: a pool under the station changed over the orbits after the torpedo"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!(
        "station probe: the waystation holds its orbit ({swept:.2} rad swept, radius \
         {radius_min:.2}..{radius_max:.2} u about {radius:.2} u, drift {drift:.4}, \
         {error:.4} rad off its attitude)"
    );
    nova_probe::probe_marker(
        world,
        "outcome: station holds its orbit",
        serde_json::json!({
            "t": elapsed,
            "swept_rad": swept,
            "radius": radius,
            "radius_min": radius_min,
            "radius_max": radius_max,
            "drift": drift,
            "error_rad": error,
        }),
    );
}
