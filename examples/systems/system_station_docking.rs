//! system_station_docking: who flies a pair docked at a station, and which
//! stations let one in.
//!
//! Three base waystations stand in a void: a living Neutral one, a dead one
//! (spawned `derelict: true`) and a living Enemy one. The player's hull is a
//! controller, a port and a drive, parked collar to collar with each in turn.
//! At the living station the dock opens on the station's helm: `HELM` is
//! refused and a full burn held on the player's hull moves nothing. At the
//! dead station the dock opens on the player's helm and the same burn tows
//! the pair. At the Enemy station the travel lock alone puts the station's
//! refusal on the HUD, and `DOCK` builds nothing.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a living station holds the helm` | docked, the helm is `Station(living)`, `HELM` leaves it there, a held burn leaves both roots at rest, and the HUD reads `STATION HELM` |
//! | 2 | `outcome: a dead station hands the helm to the player` | docked, the helm is `Held(player)` from the capture and the held burn moves both roots on one joint |
//! | 3 | `outcome: an enemy station refuses docking on the HUD` | with the travel lock on the Enemy station the hints carry `<NAME> REFUSES DOCKING`, the mode chip shows it, and a dock request builds no connection |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_station_docking --features debug
//! # look for: `station docking probe: the living station holds the helm`,
//! #           `station docking probe: the dead station hands the helm to the player`,
//! #           `station docking probe: the enemy station refuses on the HUD`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use std::sync::Arc;

#[cfg(feature = "debug")]
use avian3d::prelude::{AngularVelocity, FixedJoint, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_station_docking")]
#[command(version = "1.0.0")]
#[command(about = "A living station holds a docked pair's helm, a dead one hands it over, an Enemy one refuses on the HUD. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The living Neutral waystation, the dead one and the Enemy one, 300 m
/// apart down -Z so no hull ever sees two collars at once.
const LIVING_POSITION: Meters3 = Meters3::new(0.0, 0.0, -150.0);
const DEAD_POSITION: Meters3 = Meters3::new(0.0, 0.0, -450.0);
const ENEMY_POSITION: Meters3 = Meters3::new(0.0, 0.0, -750.0);

/// The three station object ids and names; the names are what the HUD line
/// carries.
const LIVING_ID: &str = "living_station";
const LIVING_NAME: &str = "WAYSTATION";
const DEAD_ID: &str = "dead_station";
const ENEMY_ID: &str = "enemy_station";
const ENEMY_NAME: &str = "ENEMY POST";

/// The scenario id the void loads under.
const SCENARIO_ID: &str = "station_docking_rig";

/// Every station is turned a quarter turn about +Y, so the waystation's
/// collar, which stands off its -X flank, faces +Z: back up the line toward
/// the player. The player's own computer holds its hull on the identity
/// attitude, so the hull is never turned; the stations are.
const STATION_TURN: f32 = std::f32::consts::FRAC_PI_2;

/// Where the player's hull parks relative to a station, engine units: the
/// turned collar's face stands 2.5 cells along +Z from the station origin;
/// the player's port is the cell ahead of its controller, facing -Z, so its
/// face sits 1.5 ahead of the hull origin. Half a cell of gap between the two
/// faces, inside the one-cell capture distance.
const BERTH_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 4.5);

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

/// How fast either root may still be moving after a burn held off the helm,
/// engine units per second. The pair sits at rest on a joint the solver
/// settles, so not zero; an ungated drive would be past this inside a frame.
#[cfg(feature = "debug")]
const HELD_SPEED_CEILING: f32 = 0.05;

/// How far either root may stand from where it was at the start of the held
/// burn, engine units: a tenth of a cell, where a live drive would have towed
/// the pair cells away.
#[cfg(feature = "debug")]
const HELD_DRIFT_CEILING: f32 = 0.1;

/// How fast both roots must be going once the player's helm tows the pair,
/// engine units per second. Twenty times the held ceiling, so the two
/// readings cannot be confused.
#[cfg(feature = "debug")]
const TOWED_SPEED_FLOOR: f32 = 1.0;

/// How long the burn is held off the helm before the rest is read, in driven
/// frames: long enough that a drive the gate let through would be past every
/// ceiling above many times over.
#[cfg(feature = "debug")]
const HOLD_FRAMES: u32 = 90;

/// The throttle under which a drive counts as cold. The throttle decays
/// toward zero and never lands on it; at this fraction of full thrust a
/// parked hull does not leave its berth inside the walk.
#[cfg(feature = "debug")]
const COLD_THROTTLE: f32 = 1e-4;

/// How close a teleported hull must settle to its berth before a dock is
/// asked for, engine units, and how slow it must be.
#[cfg(feature = "debug")]
const BERTH_TOLERANCE: f32 = 0.05;
#[cfg(feature = "debug")]
const BERTH_REST: f32 = 0.01;

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

/// The scripted run. Every beat waits on a world value - a station being up,
/// a dock made or let go, a hull settled on its berth, the HUD line - so a
/// slow load delays the walk instead of truncating it.
#[cfg(feature = "debug")]
fn station_script() -> Script {
    Script::new()
        .step("load the void with the three stations and the player")
        .enter(GameStates::Loading)
        .until(stations_present())
        .deadline(30.0)
        .add()
        .step("settle the player on the living station's berth")
        .on_enter(|world| berth_player(world, LIVING_ID))
        .until(player_berthed(LIVING_ID))
        .deadline(15.0)
        .add()
        .step("dock at the living station")
        .each(|world, _, _| request_dock(world, LIVING_ID))
        .until(player_docked())
        .deadline(15.0)
        .diagnose(describe_ports)
        .add()
        .step("ask for the helm and hold a burn against the living station")
        .on_enter(|world| assert_living_station_helm(world))
        .each(|world, _, _| hold_burn(world))
        .until(frames(HOLD_FRAMES))
        .deadline(20.0)
        .add()
        .step("assert the living station still holds the helm and the pair stood still")
        .on_enter(assert_living_station_pair_at_rest)
        .until(player_free())
        .deadline(15.0)
        .add()
        .step("settle the player on the dead station's berth")
        .on_enter(|world| berth_player(world, DEAD_ID))
        .until(player_berthed(DEAD_ID))
        .deadline(15.0)
        .add()
        .step("dock at the dead station")
        .each(|world, _, _| request_dock(world, DEAD_ID))
        .until(player_docked())
        .deadline(15.0)
        .add()
        .step("hold a burn on the player's helm against the dead station")
        .on_enter(assert_dead_station_helm)
        .each(|world, _, _| hold_burn(world))
        .until(pair_towed(DEAD_ID))
        .deadline(20.0)
        .add()
        .step("assert the player's helm towed the pair on one joint")
        .on_enter(assert_dead_station_pair_towed)
        .until(player_free())
        .deadline(15.0)
        .add()
        // The throttle ramps down rather than cutting: a hull parked while
        // its drive is still warm coasts off the berth and never settles.
        .step("let the player's drive go cold")
        .until(drive_cold())
        .deadline(15.0)
        .diagnose(describe_ports)
        .add()
        .step("settle the player on the enemy station's berth and lock it")
        .on_enter(|world| berth_player(world, ENEMY_ID))
        .until(player_berthed(ENEMY_ID))
        .deadline(15.0)
        .diagnose(describe_ports)
        .add()
        .step("wait for the refusal line")
        .on_enter(lock_enemy_station)
        .until(refusal_line_shown())
        .deadline(15.0)
        .add()
        .step("ask for a dock the enemy station refuses")
        .on_enter(|world| request_dock(world, ENEMY_ID))
        .until(frames(10))
        .deadline(15.0)
        .add()
        // The script's last beat: `nova_screenshot` appends the capture beat
        // behind it, so the run ends on the assertion.
        .step("assert the enemy station refused on the HUD")
        .on_enter(assert_enemy_station_refused)
        .add()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_void);
    #[cfg(feature = "debug")]
    {
        app.init_resource::<DockingEpoch>();
    }
}

fn setup_void(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(station_void(&game_assets, &sections)));
}

/// One base waystation parked at `position` as `allegiance` and dead or not.
fn station(
    id: &str,
    name: &str,
    position: Meters3,
    allegiance: Allegiance,
    derelict: bool,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::from_rotation_y(STATION_TURN),
        },
        kind: ScenarioObjectKind::Station(StationConfig {
            design: StationDesignSource::Prototype(BLOCK_WAYSTATION_STATION_ID.into()),
            allegiance,
            derelict,
            initial_velocity: MetersPerSecond3::ZERO,
        }),
    }
}

/// The void: three waystations down -Z and the player parked on the living
/// one's berth, lit by a three-point rig because the engine spawns no light.
fn station_void(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let berth = Meters3::from_engine(LIVING_POSITION.to_engine() + BERTH_OFFSET);
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_spaceship".to_string(),
            name: "Player".to_string(),
            position: berth,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(player_ship(sections)),
    };
    ScenarioConfig {
        description: "Three waystations in a void: living, dead and Enemy.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    EventActionConfig::SpawnScenarioObject(station(
                        LIVING_ID,
                        LIVING_NAME,
                        LIVING_POSITION,
                        Allegiance::Neutral,
                        false,
                    )),
                    EventActionConfig::SpawnScenarioObject(station(
                        DEAD_ID,
                        "DEAD STATION",
                        DEAD_POSITION,
                        Allegiance::Neutral,
                        true,
                    )),
                    EventActionConfig::SpawnScenarioObject(station(
                        ENEMY_ID,
                        ENEMY_NAME,
                        ENEMY_POSITION,
                        Allegiance::Enemy,
                        false,
                    )),
                    EventActionConfig::SpawnScenarioObject(player),
                ],
                ThreePointRig::around("rig", LIVING_POSITION, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Station Docking Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player's hull: a controller, a port on its nose and a drive on its
/// tail, out of the base catalog, so the fixture is held to the grammar the
/// game ships and the burn claims are made against a ship that can burn.
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
                    source: SectionSource::Inline(section(BASIC_CONTROLLER_SECTION_ID)),
                },
                SpaceshipSectionConfig {
                    id: "port".to_string(),
                    position: Vec3::new(0.0, 0.0, -1.0),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section(DOCKING_PORT_SECTION_ID)),
                },
                SpaceshipSectionConfig {
                    id: "drive".to_string(),
                    position: Vec3::new(0.0, 0.0, 1.0),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section(BASIC_THRUSTER_SECTION_ID)),
                },
            ],
            ..default()
        }),
        ..default()
    }
}

/// What the walk remembers between beats: where the two roots stood when the
/// burn off the helm began.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct DockingEpoch {
    /// The player's and the station's positions at the start of the held
    /// burn, engine units.
    rest: Option<[Vec3; 2]>,
}

/// The station root the scenario spawned under `id`, or `None` while none
/// is up.
#[cfg(feature = "debug")]
fn station_root(world: &World, id: &str) -> Option<Entity> {
    let mut stations = world.try_query_filtered::<(Entity, &EntityId), With<StationMarker>>()?;
    stations
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

/// The player's ship root, or `None` while none is up.
#[cfg(feature = "debug")]
fn player_root(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    players.iter(world).next()
}

/// All three stations stand with a physics pose, and so does the player.
#[cfg(feature = "debug")]
fn stations_present() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        [LIVING_ID, DEAD_ID, ENEMY_ID].iter().all(|id| {
            station_root(world, id).is_some_and(|station| world.get::<Position>(station).is_some())
        }) && player_root(world).is_some_and(|player| world.get::<Position>(player).is_some())
    })
}

/// Where the player's hull parks for the station `id`, engine units.
#[cfg(feature = "debug")]
fn berth_of(world: &World, id: &str) -> Vec3 {
    let station = station_root(world, id).expect("station docking probe: the station is up");
    world
        .get::<Position>(station)
        .expect("station docking probe: the station has a physics position")
        .0
        + BERTH_OFFSET
}

/// Park the player's hull on the berth of station `id`, at rest, on the
/// attitude its computer holds. Written the way the engine's own teleports
/// write a pose: through avian's `Position` and `Rotation`.
#[cfg(feature = "debug")]
fn berth_player(world: &mut World, id: &str) {
    let berth = berth_of(world, id);
    let player = player_root(world).expect("station docking probe: the player is up");
    world.entity_mut(player).insert((
        Position(berth),
        Rotation(Quat::IDENTITY),
        LinearVelocity(Vec3::ZERO),
        AngularVelocity(Vec3::ZERO),
    ));
    info!("station docking probe: parked the player on the {id} berth at {berth}");
}

/// The player's hull stands on the berth of station `id`, at rest.
#[cfg(feature = "debug")]
fn player_berthed(id: &'static str) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        let Some(player) = player_root(world) else {
            return false;
        };
        let (Some(position), Some(velocity)) = (
            world.get::<Position>(player),
            world.get::<LinearVelocity>(player),
        ) else {
            return false;
        };
        position.0.distance(berth_of(world, id)) < BERTH_TOLERANCE
            && velocity.0.length() < BERTH_REST
    })
}

/// Every live port's world face and axis, with its root's pose, and the
/// player's motion state: what a stall on a berth or dock beat needs to say.
#[cfg(feature = "debug")]
fn describe_ports(world: &World) -> String {
    let mut lines = Vec::new();
    if let Some(player) = player_root(world) {
        let root = world.entity(player);
        let throttles: Vec<f32> = world
            .try_query::<(&ChildOf, &ThrusterSectionInput)>()
            .map(|mut q| {
                q.iter(world)
                    .filter(|(parent, _)| parent.0 == player)
                    .map(|(_, input)| **input)
                    .collect()
            })
            .unwrap_or_default();
        lines.push(format!(
            "player {player:?}: pos {:?} vel {:?} intent {:?} throttles {throttles:?} docked {} autopilot {}",
            root.get::<Position>().map(|p| p.0),
            root.get::<LinearVelocity>().map(|v| v.0),
            root.get::<FlightIntent>(),
            root.contains::<DockedShip>(),
            root.contains::<Autopilot>()
        ));
    }
    let Some(mut ports) = world
        .try_query_filtered::<(Entity, &ChildOf, &GlobalTransform), With<DockingSectionMarker>>()
    else {
        return "no ports".to_string();
    };
    for (port, parent, global) in ports.iter(world) {
        let (_, rotation, translation) = global.to_scale_rotation_translation();
        let root = parent.0;
        let name = world.get::<Name>(root).map_or("?", |name| name.as_str());
        let position = world.get::<Position>(root).map(|p| p.0);
        let body_rotation = world.get::<Rotation>(root).map(|r| r.0);
        let inactive = world.entity(port).contains::<SectionInactiveMarker>();
        lines.push(format!(
            "port {port:?} on {name} ({root:?}): face {:?} axis {:?} inactive {inactive}; root pos {position:?} rot {body_rotation:?}",
            translation + rotation * Vec3::new(0.0, 0.0, -0.5),
            rotation * Vec3::NEG_Z
        ));
    }
    lines.join("\n")
}

/// Ask for a dock between the player and station `id`, as `DOCK` does. Asked
/// every frame of a dock beat, so the frame the connection lands is skipped
/// rather than asking an occupied station again.
#[cfg(feature = "debug")]
fn request_dock(world: &mut World, id: &str) {
    let player = player_root(world).expect("station docking probe: the player is up");
    if world.get::<DockedShip>(player).is_some() {
        return;
    }
    let station = station_root(world, id).expect("station docking probe: the station is up");
    world.trigger(DockingConnectionRequest {
        entity: player,
        target: station,
    });
}

/// The player's hull holds a connection.
#[cfg(feature = "debug")]
fn player_docked() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        player_root(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
    })
}

/// The player's hull holds no connection.
#[cfg(feature = "debug")]
fn player_free() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        player_root(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
    })
}

/// Hold a full burn on the player's hull, re-asserted every frame so the
/// input layer's own writer cannot clear it.
#[cfg(feature = "debug")]
fn hold_burn(world: &mut World) {
    let player = player_root(world).expect("station docking probe: the player is up");
    world.entity_mut(player).insert(FlightIntent { burn: 1.0 });
}

/// Drop the burn. The intent is written to zero rather than removed: the
/// manual balancer reads the intent, and a hull with none keeps the throttle
/// it last had.
#[cfg(feature = "debug")]
fn drop_burn(world: &mut World) {
    let player = player_root(world).expect("station docking probe: the player is up");
    world.entity_mut(player).insert(FlightIntent { burn: 0.0 });
}

/// Every throttle on the player's hull is at zero.
#[cfg(feature = "debug")]
fn drive_cold() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(player) = player_root(world) else {
            return false;
        };
        world
            .try_query::<(&ChildOf, &ThrusterSectionInput)>()
            .is_some_and(|mut throttles| {
                throttles
                    .iter(world)
                    .filter(|(parent, _)| parent.0 == player)
                    .all(|(_, input)| **input < COLD_THROTTLE)
            })
    })
}

/// The connection holding the player, with its record.
#[cfg(feature = "debug")]
fn player_connection(world: &World) -> (Entity, DockedShip, DockingConnection) {
    let player = player_root(world).expect("station docking probe: the player is up");
    let docked = *world
        .get::<DockedShip>(player)
        .expect("station docking probe: the player is docked");
    let connection = *world
        .get::<DockingConnection>(docked.connection)
        .expect("station docking probe: the connection is live");
    (player, docked, connection)
}

/// The speed of a root, engine units per second.
#[cfg(feature = "debug")]
fn speed_of(world: &World, root: Entity) -> f32 {
    world
        .get::<LinearVelocity>(root)
        .map_or(f32::NAN, |velocity| velocity.0.length())
}

/// The position of a root, engine units.
#[cfg(feature = "debug")]
fn position_of(world: &World, root: Entity) -> Vec3 {
    world
        .get::<Position>(root)
        .map_or(Vec3::NAN, |position| position.0)
}

/// How many fixed joints are live.
#[cfg(feature = "debug")]
fn joint_count(world: &mut World) -> usize {
    world.query::<&FixedJoint>().iter(world).count()
}

/// What the mode chip reads on the player's HUD, empty while no chip is up.
#[cfg(feature = "debug")]
fn mode_chip_text(world: &mut World) -> String {
    world
        .query_filtered::<&Text, With<ModeChipUIMarker>>()
        .iter(world)
        .map(|text| text.0.clone())
        .next()
        .unwrap_or_default()
}

/// Claim 1, first half: the dock opened on the station's helm, and `HELM`
/// leaves it there. Records where the two roots stand for the rest check.
#[cfg(feature = "debug")]
fn assert_living_station_helm(world: &mut World) {
    let (player, docked, connection) = player_connection(world);
    let station = station_root(world, LIVING_ID).expect("station docking probe: the station is up");
    assert_eq!(
        connection.helm,
        DockedHelmType::Station(station),
        "station docking probe: the living station must hold the helm at capture"
    );
    assert!(
        !docked.drives,
        "station docking probe: the player's hull must not drive the pair"
    );
    world.trigger(DockingHelmRequest { entity: player });
    world.flush();
    let (_, _, connection) = player_connection(world);
    assert_eq!(
        connection.helm,
        DockedHelmType::Station(station),
        "station docking probe: HELM must be refused at a living station"
    );
    world.resource_mut::<DockingEpoch>().rest =
        Some([position_of(world, player), position_of(world, station)]);
    info!(
        "station docking probe: docked at the living station, helm {:?}, HELM refused",
        connection.helm
    );
}

/// Claim 1, second half: after the held burn the station still holds the
/// helm, the station drives, both roots are at rest where they were, and the
/// HUD says so. Then the burn is dropped and the dock let go.
#[cfg(feature = "debug")]
fn assert_living_station_pair_at_rest(world: &mut World) {
    let (player, docked, connection) = player_connection(world);
    let station = station_root(world, LIVING_ID).expect("station docking probe: the station is up");
    assert_eq!(connection.helm, DockedHelmType::Station(station));
    assert!(
        !docked.drives,
        "station docking probe: the player's hull took the helm"
    );
    assert!(
        world
            .get::<DockedStation>(station)
            .is_some_and(|docked| docked.drives),
        "station docking probe: the living station must drive its pair"
    );
    let rest = world
        .resource::<DockingEpoch>()
        .rest
        .expect("station docking probe: the rest pose was recorded");
    let speeds = [speed_of(world, player), speed_of(world, station)];
    let drifts = [
        position_of(world, player).distance(rest[0]),
        position_of(world, station).distance(rest[1]),
    ];
    assert!(
        speeds.iter().all(|speed| *speed < HELD_SPEED_CEILING),
        "station docking probe: a burn held off the helm moved the pair: speeds {speeds:?}"
    );
    assert!(
        drifts.iter().all(|drift| *drift < HELD_DRIFT_CEILING),
        "station docking probe: a burn held off the helm towed the pair: drifts {drifts:?}"
    );
    let hints = world.resource::<FlightVerbHints>().clone();
    assert!(
        hints.helm_station && !hints.helm.available,
        "station docking probe: the HELM chip must read the station's helm: {hints:?}"
    );
    let chip = mode_chip_text(world);
    assert_eq!(
        chip, "STATION HELM",
        "station docking probe: the mode chip must read STATION HELM"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!(
        "station docking probe: the living station holds the helm (speeds {speeds:?}, drifts {drifts:?}, chip {chip})"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a living station holds the helm",
        serde_json::json!({ "t": elapsed, "speeds": speeds, "drifts": drifts, "chip": chip }),
    );
    drop_burn(world);
    world.trigger(DockingReleaseRequest { entity: player });
}

/// Claim 2, first half: the dock at the dead station opened on the player's
/// helm.
#[cfg(feature = "debug")]
fn assert_dead_station_helm(world: &mut World) {
    let (player, docked, connection) = player_connection(world);
    assert_eq!(
        connection.helm,
        DockedHelmType::Held(player),
        "station docking probe: a dead station must hand the helm to the player at capture"
    );
    assert!(
        docked.drives,
        "station docking probe: the player must drive the pair at a dead station"
    );
    info!(
        "station docking probe: docked at the dead station, helm {:?}",
        connection.helm
    );
}

/// Both roots of the pair docked at station `id` are under way.
#[cfg(feature = "debug")]
fn pair_towed(id: &'static str) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        let (Some(player), Some(station)) = (player_root(world), station_root(world, id)) else {
            return false;
        };
        speed_of(world, player) > TOWED_SPEED_FLOOR && speed_of(world, station) > TOWED_SPEED_FLOOR
    })
}

/// Claim 2, second half: the held burn towed both roots and the joint held.
/// Then the burn is dropped and the dock let go.
#[cfg(feature = "debug")]
fn assert_dead_station_pair_towed(world: &mut World) {
    let (player, docked, connection) = player_connection(world);
    let station = station_root(world, DEAD_ID).expect("station docking probe: the station is up");
    assert_eq!(connection.helm, DockedHelmType::Held(player));
    assert!(docked.drives);
    assert!(
        world
            .get::<DockedStation>(station)
            .is_some_and(|docked| !docked.drives),
        "station docking probe: a dead station must not drive the pair"
    );
    let speeds = [speed_of(world, player), speed_of(world, station)];
    assert!(
        speeds.iter().all(|speed| *speed > TOWED_SPEED_FLOOR),
        "station docking probe: the player's helm must tow both roots: {speeds:?}"
    );
    let joints = joint_count(world);
    assert_eq!(
        joints, 1,
        "station docking probe: the tow must ride one joint"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!(
        "station docking probe: the dead station hands the helm to the player (speeds {speeds:?})"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a dead station hands the helm to the player",
        serde_json::json!({ "t": elapsed, "speeds": speeds, "joints": joints }),
    );
    drop_burn(world);
    world
        .entity_mut(player)
        .insert((LinearVelocity(Vec3::ZERO), AngularVelocity(Vec3::ZERO)));
    world
        .entity_mut(station)
        .insert((LinearVelocity(Vec3::ZERO), AngularVelocity(Vec3::ZERO)));
    world.trigger(DockingReleaseRequest { entity: player });
}

/// Put the player's travel lock on the Enemy station, as the targeting layer
/// would on a lock.
#[cfg(feature = "debug")]
fn lock_enemy_station(world: &mut World) {
    let player = player_root(world).expect("station docking probe: the player is up");
    let station = station_root(world, ENEMY_ID).expect("station docking probe: the station is up");
    world.entity_mut(player).insert(TravelLock(Some(station)));
}

/// The line the Enemy station's refusal carries.
#[cfg(feature = "debug")]
fn refusal_line() -> String {
    format!("{ENEMY_NAME} REFUSES DOCKING")
}

/// The hints carry the refusal line and the mode chip shows it.
#[cfg(feature = "debug")]
fn refusal_line_shown() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(hints) = world.get_resource::<FlightVerbHints>() else {
            return false;
        };
        hints.dock_refusal.as_deref() == Some(refusal_line().as_str())
            && world
                .try_query_filtered::<&Text, With<ModeChipUIMarker>>()
                .is_some_and(|mut chips| chips.iter(world).any(|text| text.0 == refusal_line()))
    })
}

/// Claim 3: the refusal is on the HUD, the DOCK chip is dark, and the dock
/// request built nothing.
#[cfg(feature = "debug")]
fn assert_enemy_station_refused(world: &mut World) {
    let player = player_root(world).expect("station docking probe: the player is up");
    let hints = world.resource::<FlightVerbHints>().clone();
    assert_eq!(
        hints.dock_refusal.as_deref(),
        Some(refusal_line().as_str()),
        "station docking probe: the hints must carry the station's refusal"
    );
    assert!(
        !hints.dock.available,
        "station docking probe: DOCK must not be offered on a refusing station"
    );
    let chip = mode_chip_text(world);
    assert_eq!(
        chip,
        refusal_line(),
        "station docking probe: the mode chip must show the refusal"
    );
    assert!(
        world.get::<DockedShip>(player).is_none(),
        "station docking probe: the enemy station admitted a dock"
    );
    let connections = world.query::<&DockingConnection>().iter(world).count();
    assert_eq!(
        connections, 0,
        "station docking probe: a refused dock built a connection"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!("station docking probe: the enemy station refuses on the HUD ({chip})");
    nova_probe::probe_marker(
        world,
        "outcome: an enemy station refuses docking on the HUD",
        serde_json::json!({ "t": elapsed, "line": chip, "connections": connections }),
    );
}
