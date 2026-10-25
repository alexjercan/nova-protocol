//! system_station_body: a station is content and its own body.
//!
//! The base waystation is parked from the station catalog in a void beside a
//! player ship. It stands where it was placed with a collar, a bridge and no
//! thruster; a nudge spins it and its own flight computer turns it back; a
//! shove carries it off the post it stands on and, mid-return, the same
//! `freeze_body` and `thaw_record` a retiring cell runs take it out and bring
//! it back as the same station, which finishes the return on its RCS.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: station spawns from content` | the catalog station is up with a Docking and a Controller section and no Thruster |
//! | 2 | `outcome: station stands and is seen` | it stands at rest where it was placed, posted there, and the player's sensors hold it |
//! | 3 | `outcome: station holds its attitude` | a nudged hull comes to rest back on its spawn attitude |
//! | 4 | `outcome: station freezes as a station` | `freeze_body` on the shoved station yields `FrozenBodyType::Station` carrying the catalog design's services and the post |
//! | 5 | `outcome: station thaws whole` | the thawed station keeps its services, pose, motion, section set, storage and post |
//! | 6 | `outcome: station rides back onto its post` | the thawed station comes to rest inside the post and hold deadbands of where it was placed, with its RCS magazine not emptied |
//!
//! The settle beat waits on the angular SPEED alone; whether the hull came
//! back to its spawn attitude is the assertion's question, so a dead computer
//! parks the hull off-axis and fails instead of stalling a beat that shares
//! the assert's tolerance.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_station_body --features debug
//! # look for: `station probe: the waystation stands`,
//! #           `station probe: the waystation holds its attitude`,
//! #           `station probe: the thawed waystation matches`,
//! #           `station probe: the waystation rides back onto its post`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use std::{collections::BTreeSet, sync::Arc};

#[cfg(feature = "debug")]
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_world::prelude::{
    freeze_body, thaw_record, DockPolicy, FrozenBody, FrozenBodyType, PreparedFrozenBody,
    SectorCoord, WorldClock,
};

#[derive(Parser)]
#[command(name = "system_station_body")]
#[command(version = "1.0.0")]
#[command(about = "A station spawns from content, holds its attitude, freezes and thaws. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// Where the station is parked: ahead of the player, far enough that its 30 m
/// block clears the player's hull and close enough that the sensors hold it.
const STATION_POSITION: Meters3 = Meters3::new(0.0, 0.0, -150.0);

/// The scenario id the void loads under.
const SCENARIO_ID: &str = "station_body_rig";

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(nova_screenshot(station_script()));
    }

    app.run()
}

/// The scripted run. Every beat waits on a world value - the station being
/// up, the player seeing it, the spin dying - so a slow load delays the walk
/// instead of truncating it; the deadlines name the beat that stalled and sum
/// well under the 120 s harness backstop.
#[cfg(feature = "debug")]
fn station_script() -> Script {
    Script::new()
        .step("load the void with the station and the player")
        .enter(GameStates::Loading)
        .until(station_present())
        .deadline(30.0)
        .add()
        .step("wait for the player's sensors to hold the station")
        .until(player_sees_station())
        .deadline(15.0)
        .add()
        .step("assert the station stands")
        .on_enter(assert_station_stands)
        .add()
        .step("nudge the station off its spawn attitude")
        .on_enter(nudge_station)
        .until(station_turned())
        .deadline(10.0)
        .add()
        .step("wait for the spin to die")
        .until(station_settled())
        .deadline(40.0)
        .add()
        .step("assert the station holds its attitude")
        .on_enter(assert_station_holds)
        .add()
        .step("shove the station off its post")
        .on_enter(shove_station)
        .until(station_displaced())
        .deadline(10.0)
        .add()
        .step("freeze the station mid-return and thaw it")
        .on_enter(freeze_and_thaw_station)
        .until(thawed_station_built())
        .deadline(20.0)
        .add()
        .step("assert the thawed station matches")
        .on_enter(assert_thawed_station)
        .add()
        .step("wait for the thawed station to ride back onto its post")
        .until(station_on_post())
        .deadline(300.0)
        .add()
        // The script's last beat: `nova_screenshot` appends the capture beat
        // behind it, so the run ends on the assertion.
        .step("assert the station holds its post")
        .on_enter(assert_station_holds_post)
        .add()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_void);
    #[cfg(feature = "debug")]
    {
        app.init_resource::<StationEpoch>();
    }
}

fn setup_void(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(station_void(&game_assets, &sections)));
}

/// The void: the catalog waystation parked ahead and a player ship behind it,
/// lit by a three-point rig because the engine spawns no light.
fn station_void(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let station = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "waystation".to_string(),
            name: "WAYSTATION".to_string(),
            position: STATION_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Station(StationConfig {
            design: StationDesignSource::Prototype(BLOCK_WAYSTATION_STATION_ID.into()),
            allegiance: Allegiance::Neutral,
            derelict: false,
            initial_velocity: MetersPerSecond3::ZERO,
        }),
    };
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_spaceship".to_string(),
            name: "Player".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(player_ship(sections)),
    };
    ScenarioConfig {
        description: "The waystation parked in a void beside the player.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    EventActionConfig::SpawnScenarioObject(station),
                    EventActionConfig::SpawnScenarioObject(player),
                ],
                ThreePointRig::around("rig", STATION_POSITION, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Station Body Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player's hull: a controller and a piece of armor out of the base
/// catalog, so the fixture is held to the grammar the game ships.
fn player_ship(sections: &GameSections) -> SpaceshipConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig::default()),
        design: ShipDesignSource::Inline(ShipDesign {
            sections: vec![
                SpaceshipSectionConfig {
                    id: "controller".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section("basic_controller_section")),
                },
                SpaceshipSectionConfig {
                    id: "hull".to_string(),
                    position: Vec3::new(0.0, 0.0, 1.0),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section("reinforced_hull_section")),
                },
            ],
            ..default()
        }),
        ..default()
    }
}

/// What the walk remembers between beats: which root is the live station,
/// the attitude it was parked at, and what it froze as.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct StationEpoch {
    /// The station root the scenario spawned.
    spawned: Option<Entity>,
    /// The attitude it was parked at.
    spawn_rotation: Quat,
    /// The section ids it stood with, read before the freeze.
    sections: BTreeSet<String>,
    /// The store it stood with, read before the freeze.
    storage: Option<StationStorage>,
    /// The record `freeze_body` returned, kept for the thaw comparison.
    frozen: Option<FrozenBody>,
    /// World seconds when the record was thawed: the frozen motion carries
    /// the station on from there while its sections build.
    thawed_at: f32,
}

/// How far the nudge turns the hull off its spawn attitude, radians, and the
/// angular speed it leaves it with, radians per second. A velocity alone is
/// not a nudge: the computer kills 0.3 rad/s inside a frame, before the hull
/// has turned a hundredth of a radian, so the displacement is what asks the
/// hold to recover anything.
#[cfg(feature = "debug")]
const NUDGE_RAD: f32 = 0.5;
#[cfg(feature = "debug")]
const NUDGE_RAD_PER_SEC: f32 = 0.3;

/// How far off the spawn attitude the nudge must take the hull before the
/// hold is asked to recover it. Above [`STATION_LEVEL_TOLERANCE_RAD`], the
/// contract the hold is graded on, so a hull that never moved cannot pass the
/// recovery assert for free.
#[cfg(feature = "debug")]
const TURNED_RAD: f32 = 0.1;

/// The angular speed under which the hull counts as at rest.
#[cfg(feature = "debug")]
const REST_RAD_PER_SEC: f32 = 0.01;

/// The thawed pose and velocity may differ from the record carried forward
/// on its own motion by one physics step of integration, the RCS return's
/// pull over the frames the thaw took and float noise, not by a cell or a
/// turn.
#[cfg(feature = "debug")]
const THAW_TOLERANCE: f32 = 0.01;

/// How far the shove carries the hull off its post, engine units, and the
/// speed it leaves it moving away at, engine units per second: 20 m out at
/// 2 m/s, four times the post deadband, so the return has to be flown and
/// a hull that never moved cannot pass the hold for free.
#[cfg(feature = "debug")]
const SHOVE_OFFSET: Vec3 = Vec3::new(2.0, 0.0, 0.0);
#[cfg(feature = "debug")]
const SHOVE_SPEED: Vec3 = Vec3::new(0.2, 0.0, 0.0);

/// A station root exists.
#[cfg(feature = "debug")]
fn station_present() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    any_entity::<With<StationMarker>>()
}

/// The live station root, or `None` while none is up.
#[cfg(feature = "debug")]
fn live_station(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, With<StationMarker>>()
        .iter(world)
        .next()
}

/// The player ship's sensors list the station.
#[cfg(feature = "debug")]
fn player_sees_station() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(mut stations) = world.try_query_filtered::<Entity, With<StationMarker>>() else {
            return false;
        };
        let Some(station) = stations.iter(world).next() else {
            return false;
        };
        let Some(mut sensors) =
            world.try_query_filtered::<&SensorContacts, With<PlayerSpaceshipMarker>>()
        else {
            return false;
        };
        sensors
            .iter(world)
            .any(|contacts| contacts.get(station).is_some())
    })
}

/// The section ids under `root`, as the spawn named them.
#[cfg(feature = "debug")]
fn section_ids(world: &World, root: Entity) -> BTreeSet<String> {
    world
        .entity(root)
        .get::<Children>()
        .into_iter()
        .flat_map(|children| children.iter())
        .filter(|child| world.entity(*child).contains::<SectionBuildConfig>())
        .filter_map(|child| world.entity(child).get::<EntityId>())
        .map(|id| id.0.clone())
        .collect()
}

/// Which section kinds stand under `root`: whether a Docking, a Controller
/// and a Thruster section each exist.
#[cfg(feature = "debug")]
fn section_kinds(world: &World, root: Entity) -> (bool, bool, bool) {
    let mut found = (false, false, false);
    for child in world
        .entity(root)
        .get::<Children>()
        .into_iter()
        .flat_map(|children| children.iter())
    {
        if let Some(config) = world.entity(child).get::<SectionBuildConfig>() {
            match config.0.kind {
                SectionKind::Docking(_) => found.0 = true,
                SectionKind::Controller(_) => found.1 = true,
                SectionKind::Thruster(_) => found.2 = true,
                _ => {}
            }
        }
    }
    found
}

/// The services the catalog waystation design offers.
#[cfg(feature = "debug")]
fn waystation_services(world: &World) -> Vec<StationServiceType> {
    world
        .resource::<GameStationDesigns>()
        .get_design(&BLOCK_WAYSTATION_STATION_ID.into())
        .expect("station probe: the catalog holds the waystation")
        .design
        .services
        .clone()
}

/// Invariants 1 and 2: the catalog station is up with the right section
/// kinds, at rest where it was placed, and the player's sensors hold it.
#[cfg(feature = "debug")]
fn assert_station_stands(world: &mut World) {
    let station = live_station(world).expect("station probe: the station must be up");
    assert_eq!(
        world
            .entity(station)
            .get::<StationServices>()
            .map(|services| services.0.clone()),
        Some(waystation_services(world)),
        "station probe: the spawned station carries the catalog design's services"
    );
    let (docking, controller, thruster) = section_kinds(world, station);
    assert!(
        docking,
        "station probe: the waystation has no Docking section"
    );
    assert!(
        controller,
        "station probe: the waystation has no Controller section"
    );
    assert!(!thruster, "station probe: the waystation grew a Thruster");
    let sections = section_ids(world, station);
    assert!(
        sections.contains("bridge") && sections.contains("port_collar"),
        "station probe: the waystation's sections are {sections:?}"
    );
    assert!(
        !world.entity(station).contains::<PlayerSpaceshipMarker>(),
        "station probe: a station is nobody's ship"
    );
    let position = world.entity(station).get::<Position>().map(|p| p.0);
    let linear = world.entity(station).get::<LinearVelocity>().map(|v| v.0);
    let rotation = world
        .entity(station)
        .get::<Rotation>()
        .map(|r| r.0)
        .expect("station probe: the station has a physics rotation");
    let expected = STATION_POSITION.to_engine();
    let drift = position.map_or(f32::INFINITY, |p| p.distance(expected));
    assert!(
        drift < THAW_TOLERANCE,
        "station probe: the station stands {drift:.3} units off {expected}"
    );
    let speed = linear.map_or(f32::INFINITY, Vec3::length);
    assert!(
        speed < REST_RAD_PER_SEC,
        "station probe: the station is moving at {speed:.3} units/s with no thruster"
    );
    let post = world
        .entity(station)
        .get::<StationPost>()
        .copied()
        .expect("station probe: the station is posted by its first fixed tick");
    assert!(
        matches!(post, StationPost::Fixed { position } if position.distance(expected) < THAW_TOLERANCE),
        "station probe: a station at rest in a void is posted where it stands, got {post:?}"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!(
        "station probe: the waystation stands at {expected} with {} sections, posted {post:?}",
        sections.len()
    );
    nova_probe::probe_marker(
        world,
        "outcome: station spawns from content",
        serde_json::json!({ "t": elapsed, "sections": sections.len() }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: station stands and is seen",
        serde_json::json!({ "t": elapsed, "speed": speed, "drift": drift }),
    );
    let storage = world.entity(station).get::<StationStorage>().cloned();
    let mut epoch = world.resource_mut::<StationEpoch>();
    epoch.spawned = Some(station);
    epoch.spawn_rotation = rotation;
    epoch.sections = sections;
    epoch.storage = storage;
}

/// Turn the station about +Y and leave it spinning: the stimulus the hold has
/// to undo. The pose is written the way the engine's own teleports write it,
/// through avian's `Rotation`.
#[cfg(feature = "debug")]
fn nudge_station(world: &mut World) {
    let station = live_station(world).expect("station probe: the station must be up");
    let spawn_rotation = world.resource::<StationEpoch>().spawn_rotation;
    world.entity_mut(station).insert((
        Rotation(spawn_rotation * Quat::from_rotation_y(NUDGE_RAD)),
        AngularVelocity(Vec3::new(0.0, NUDGE_RAD_PER_SEC, 0.0)),
    ));
    info!("station probe: nudged the waystation {NUDGE_RAD} rad off spawn at {NUDGE_RAD_PER_SEC} rad/s");
}

/// Angle between the live station's attitude and the one it was parked at.
#[cfg(feature = "debug")]
fn attitude_error(world: &World) -> Option<f32> {
    let epoch = world.get_resource::<StationEpoch>()?;
    let rotation = world.get::<Rotation>(epoch.spawned?)?;
    Some(rotation.0.angle_between(epoch.spawn_rotation))
}

/// The live station's angular speed.
#[cfg(feature = "debug")]
fn angular_speed(world: &World) -> Option<f32> {
    let epoch = world.get_resource::<StationEpoch>()?;
    Some(world.get::<AngularVelocity>(epoch.spawned?)?.0.length())
}

/// The nudge took: the hull is clear of its spawn attitude.
#[cfg(feature = "debug")]
fn station_turned() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| attitude_error(world).is_some_and(|error| error > TURNED_RAD))
}

/// The spin died. Nothing here about WHERE the hull came to rest: that is
/// [`assert_station_holds`]'s question.
#[cfg(feature = "debug")]
fn station_settled() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| angular_speed(world).is_some_and(|speed| speed < REST_RAD_PER_SEC))
}

/// Invariant 3: the hull came to rest back on its spawn attitude.
#[cfg(feature = "debug")]
fn assert_station_holds(world: &mut World) {
    let error = attitude_error(world).expect("station probe: the station must be up");
    assert!(
        error < STATION_LEVEL_TOLERANCE_RAD,
        "station probe: the hull came to rest {error:.3} rad off its spawn attitude \
         (tolerance {STATION_LEVEL_TOLERANCE_RAD}); the computer is not holding"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!("station probe: the waystation holds its attitude ({error:.4} rad off spawn)");
    nova_probe::probe_marker(
        world,
        "outcome: station holds its attitude",
        serde_json::json!({ "t": elapsed, "error_rad": error }),
    );
}

/// Carry the station [`SHOVE_OFFSET`] off its post and leave it moving away
/// at [`SHOVE_SPEED`]: what a ram leaves behind, written the way the engine's
/// own teleports write it, through avian's `Position` and `LinearVelocity`.
#[cfg(feature = "debug")]
fn shove_station(world: &mut World) {
    let station = live_station(world).expect("station probe: the station must be up");
    let position = world
        .get::<Position>(station)
        .map(|p| p.0)
        .expect("station probe: the station has a physics position");
    world.entity_mut(station).insert((
        Position(position + SHOVE_OFFSET),
        LinearVelocity(SHOVE_SPEED),
    ));
    info!(
        "station probe: shoved the waystation {} u off its post at {} u/s",
        SHOVE_OFFSET.length(),
        SHOVE_SPEED.length()
    );
}

/// The live station's distance from where it was placed and its speed. The
/// live root, not the epoch's: after the thaw the station is a fresh root.
#[cfg(feature = "debug")]
fn post_error(world: &World) -> Option<(f32, f32)> {
    let mut stations = world.try_query_filtered::<Entity, With<StationMarker>>()?;
    let station = stations.iter(world).next()?;
    let position = world.get::<Position>(station)?.0;
    let velocity = world.get::<LinearVelocity>(station)?.0;
    Some((
        position.distance(STATION_POSITION.to_engine()),
        velocity.length(),
    ))
}

/// The shove took: the hull is outside its post deadband.
#[cfg(feature = "debug")]
fn station_displaced() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let deadband = world.resource::<FlightSettings>().station_post_deadband;
        post_error(world).is_some_and(|(offset, _)| offset > deadband)
    })
}

/// The hull is back inside both deadbands of its post. Where it is posted
/// and what the return cost are [`assert_station_holds_post`]'s questions.
#[cfg(feature = "debug")]
fn station_on_post() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let settings = world.resource::<FlightSettings>();
        post_error(world).is_some_and(|(offset, speed)| {
            offset < settings.station_post_deadband && speed < settings.station_hold_deadband
        })
    })
}

/// Invariant 6: the thawed station rode back onto the post it was placed on
/// with its own RCS, and its magazine was not emptied doing it.
#[cfg(feature = "debug")]
fn assert_station_holds_post(world: &mut World) {
    let station = live_station(world).expect("station probe: the thawed station must be up");
    let settings = world.resource::<FlightSettings>().clone();
    let (offset, speed) = post_error(world).expect("station probe: the station has a pose");
    assert!(
        offset < settings.station_post_deadband && speed < settings.station_hold_deadband,
        "station probe: the station rests {offset:.3} u off its post at {speed:.3} u/s \
         (deadbands {} u, {} u/s)",
        settings.station_post_deadband,
        settings.station_hold_deadband
    );
    let budget = world
        .get::<RcsBudget>(station)
        .copied()
        .expect("station probe: a station root carries its RCS magazine");
    assert!(
        !budget.is_empty(),
        "station probe: the return emptied the station's RCS magazine"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!(
        "station probe: the waystation rides back onto its post ({offset:.4} u off at \
         {speed:.4} u/s, {:.3} u/s of RCS spent)",
        budget.spent
    );
    nova_probe::probe_marker(
        world,
        "outcome: station rides back onto its post",
        serde_json::json!({
            "t": elapsed,
            "offset": offset,
            "speed": speed,
            "rcs_spent": budget.spent,
        }),
    );
}

/// Invariant 4, then the thaw: freeze the live station mid-return as a
/// retiring cell would, check the record is a station carrying the services
/// and the post, despawn the live one and thaw the record under a fresh root.
#[cfg(feature = "debug")]
fn freeze_and_thaw_station(world: &mut World) {
    let station = live_station(world).expect("station probe: the station must be up");
    let frozen = freeze_body(
        world,
        station,
        DockPolicy::Refuse,
        world.resource::<WorldClock>(),
    )
    .expect("station probe: a parked station is settled");
    let FrozenBodyType::Station(record) = frozen.body() else {
        panic!(
            "station probe: the station froze as {:?}, not a station",
            frozen.body()
        );
    };
    assert_eq!(
        record.services(),
        waystation_services(world).as_slice(),
        "station probe: the frozen record carries the design's services"
    );
    assert_eq!(
        frozen.id(),
        Some("waystation"),
        "station probe: the frozen record carries the object id"
    );
    assert!(
        record.ship().has_section("bridge") && record.ship().has_section("port_collar"),
        "station probe: the frozen hull lost its bridge or collar"
    );
    let live_post = world
        .get::<StationPost>(station)
        .copied()
        .expect("station probe: the live station is posted");
    assert_eq!(
        record.post(),
        live_post,
        "station probe: the frozen record does not carry the live post"
    );
    let (offset, speed) = post_error(world).expect("station probe: the station has a pose");
    assert!(
        offset > world.resource::<FlightSettings>().station_post_deadband,
        "station probe: the freeze did not catch the station mid-return ({offset:.3} u off at \
         {speed:.3} u/s)"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!(
        "station probe: froze the waystation at {} with motion {:?}",
        frozen.transform().translation,
        frozen.motion()
    );
    nova_probe::probe_marker(
        world,
        "outcome: station freezes as a station",
        serde_json::json!({ "t": elapsed, "services": record.services().len() }),
    );

    world.entity_mut(station).despawn();
    let root = world
        .spawn((
            Name::new("thaw root"),
            Transform::IDENTITY,
            Visibility::default(),
        ))
        .id();
    let prepared = PreparedFrozenBody::prepare(SectorCoord::new(0, 0, 0), frozen.clone(), &[], &[]);
    let clock = *world.resource::<WorldClock>();
    let mut commands = world.commands();
    thaw_record(&mut commands, root, vec![prepared], &clock);
    world.flush();
    let now = world.resource::<Time>().elapsed_secs();
    let mut epoch = world.resource_mut::<StationEpoch>();
    epoch.frozen = Some(frozen);
    epoch.thawed_at = now;
}

/// A station root other than the one the freeze despawned stands with every
/// section the record names.
#[cfg(feature = "debug")]
fn thawed_station_built() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(epoch) = world.get_resource::<StationEpoch>() else {
            return false;
        };
        let Some(mut stations) = world.try_query_filtered::<Entity, With<StationMarker>>() else {
            return false;
        };
        stations
            .iter(world)
            .any(|root| Some(root) != epoch.spawned && section_ids(world, root) == epoch.sections)
    })
}

/// Invariant 5: the thawed station is the frozen one.
#[cfg(feature = "debug")]
fn assert_thawed_station(world: &mut World) {
    let station = live_station(world).expect("station probe: the thawed station must be up");
    let (spawned, spawn_rotation, sections, storage, frozen, thawed_at) = {
        let epoch = world.resource::<StationEpoch>();
        (
            epoch.spawned,
            epoch.spawn_rotation,
            epoch.sections.clone(),
            epoch.storage.clone(),
            epoch.frozen.clone().expect("station probe: the freeze ran"),
            epoch.thawed_at,
        )
    };
    // The record was mid-return: carry its motion forward over the time the
    // thaw took, and allow the return's pull over that time on top.
    let since_thaw = world.resource::<Time>().elapsed_secs() - thawed_at;
    let return_accel = world.resource::<FlightSettings>().station_rcs_accel;
    assert_ne!(
        Some(station),
        spawned,
        "station probe: the thawed station is a fresh root"
    );
    let root = world.entity(station);
    assert_eq!(
        root.get::<StationServices>()
            .map(|services| services.0.clone()),
        Some(waystation_services(world)),
        "station probe: the thawed station carries the design's services"
    );
    assert_eq!(
        root.get::<EntityId>().map(|id| id.0.as_str()),
        Some("waystation"),
        "station probe: the thawed station carries the object id"
    );
    assert_eq!(
        root.get::<StationStorage>(),
        storage.as_ref(),
        "station probe: the thawed station's storage differs"
    );
    let position = root
        .get::<Position>()
        .map(|p| p.0)
        .expect("station probe: the thawed station has a physics position");
    let rotation = root
        .get::<Rotation>()
        .map(|r| r.0)
        .expect("station probe: the thawed station has a physics rotation");
    let (linear, angular) = frozen
        .motion()
        .expect("station probe: a physics body froze with motion");
    let live_linear = root.get::<LinearVelocity>().map_or(Vec3::NAN, |v| v.0);
    let live_angular = root.get::<AngularVelocity>().map_or(Vec3::NAN, |v| v.0);
    let carried = frozen.transform().translation + linear * since_thaw;
    let drift = position.distance(carried);
    let turn = rotation.angle_between(frozen.transform().rotation);
    assert!(
        drift < THAW_TOLERANCE + 0.5 * return_accel * since_thaw * since_thaw,
        "station probe: the thawed station stands {drift:.3} units off where its record \
         carried it in {since_thaw:.2} s"
    );
    assert!(
        turn < THAW_TOLERANCE,
        "station probe: the thawed station is {turn:.3} rad off the attitude it froze at"
    );
    assert!(
        rotation.angle_between(spawn_rotation) < STATION_LEVEL_TOLERANCE_RAD + THAW_TOLERANCE,
        "station probe: the thawed station is off the attitude the live one was parked at"
    );
    assert!(
        live_linear.distance(linear) < THAW_TOLERANCE + return_accel * since_thaw
            && live_angular.distance(angular) < THAW_TOLERANCE,
        "station probe: the thawed motion {live_linear:?}/{live_angular:?} differs from the \
         frozen {linear:?}/{angular:?} after {since_thaw:.2} s"
    );
    let (docking, controller, thruster) = section_kinds(world, station);
    assert!(
        docking && controller && !thruster,
        "station probe: the thawed station's section kinds changed"
    );
    let thawed_sections = section_ids(world, station);
    assert_eq!(
        thawed_sections, sections,
        "station probe: the thawed station's sections differ"
    );
    let FrozenBodyType::Station(record) = frozen.body() else {
        unreachable!("station probe: the record was checked to be a station at the freeze");
    };
    assert_eq!(
        root.get::<StationPost>().copied(),
        Some(record.post()),
        "station probe: the thawed station does not hold the post it froze with"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!(
        "station probe: the thawed waystation matches ({} sections, {drift:.4} units and \
         {turn:.4} rad off the record)",
        thawed_sections.len()
    );
    nova_probe::probe_marker(
        world,
        "outcome: station thaws whole",
        serde_json::json!({
            "t": elapsed,
            "sections": thawed_sections.len(),
            "drift": drift,
            "turn_rad": turn,
        }),
    );
}
