//! system_contraband_dock: a lawful station refuses a hull that carries
//! contraband, and admits it once the contraband left the hold.
//!
//! A base waystation stands in a void, Neutral and living. THE LAWFUL MARK IS
//! DISCLOSED FIXTURE STATE: an authored station has no civilization, so the
//! walk puts `LawfulStationMarker` on it by hand, as the open world does for
//! a living station of a lawful civilization. The player's hull is a
//! controller over a hull block with a port on its nose and a drive on its
//! tail. THE HOLD IS DISCLOSED FIXTURE STATE: one unmarked core, the base
//! contraband item. The player parks on the berth with the travel lock on
//! the station and asks for a dock. The station refuses: the hints and the
//! HUD notice read `WAYSTATION REFUSES CONTRABAND` and no connection is
//! built. THE CORE LEAVING THE HOLD IS A FIXTURE STEP: the walk removes it,
//! where a player sells or jettisons it. The next request docks.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 0 | `outcome: a lawful station refuses a hull that carries contraband` | admission answers `Contraband`; the hints carry `WAYSTATION REFUSES CONTRABAND`, the HUD notice showed it at the lock and DOCK is not offered; the dock requests built no connection; the core is still in the hold |
//! | 1 | `outcome: the lawful station admits the hull once the contraband left` | with the hold empty admission answers no refusal, the hints carry none, and the request docks the hull at the same station, still marked lawful |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_contraband_dock --features debug
//! # look for: `contraband dock probe: PASS the lawful station refuses the carrier`,
//! #           `contraband dock probe: PASS the lawful station admits the hull`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_contraband_dock")]
#[command(version = "1.0.0")]
#[command(about = "A living lawful station refuses a dock to a hull with one unmarked core in its hold, on the HUD, and admits the same hull once the core left. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The waystation's park, down -Z from the player.
const STATION_POSITION: Meters3 = Meters3::new(0.0, 0.0, -150.0);
/// The station object id.
const STATION_ID: &str = "waystation";
/// The station's name, which the refusal line leads with.
const STATION_NAME: &str = "WAYSTATION";
/// The scenario id the void loads under.
const SCENARIO_ID: &str = "contraband_dock_rig";
/// The player object id.
const PLAYER_ID: &str = "player_spaceship";

/// The station is turned a quarter turn about +Y, so its collar, which stands
/// off its -X flank, faces +Z: back toward the player.
const STATION_TURN: f32 = std::f32::consts::FRAC_PI_2;

/// Where the player's hull parks relative to the station, engine units: the
/// turned collar's face stands 2.5 cells along +Z from the station origin and
/// the player's port face 1.5 ahead of its hull origin, [`BERTH_GAP`] apart.
const BERTH_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 4.0 + BERTH_GAP);

/// The base contraband item.
const CORES: &str = "UnmarkedCores";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_void);
}

fn setup_void(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(station_void(&game_assets)));
}

/// The void: the living waystation and the player off its berth, lit by a
/// three-point rig because the engine spawns no light.
fn station_void(game_assets: &GameAssets) -> ScenarioConfig {
    let station = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: STATION_ID.to_string(),
            name: STATION_NAME.to_string(),
            position: STATION_POSITION,
            rotation: Quat::from_rotation_y(STATION_TURN),
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
            id: PLAYER_ID.to_string(),
            name: "Player".to_string(),
            position: Meters3::from_engine(STATION_POSITION.to_engine() + BERTH_OFFSET),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(player_ship()),
    };
    ScenarioConfig {
        description: "A lawful waystation, and a player with one unmarked core.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
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
            "Contraband Dock Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player's hull: a controller over a hull block (the hold), a port on
/// its nose and a drive on its tail, with one unmarked core in the hold.
fn player_ship() -> SpaceshipConfig {
    let section =
        |id: &str, prototype: &str, position: Vec3, rotation: Quat| SpaceshipSectionConfig {
            id: id.to_string(),
            position,
            rotation,
            source: SectionSource::prototype(prototype),
        };
    SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig::default()),
        design: ShipDesignSource::Inline(ShipDesign {
            sections: vec![
                section(
                    "controller",
                    BASIC_CONTROLLER_SECTION_ID,
                    Vec3::ZERO,
                    Quat::IDENTITY,
                ),
                section(
                    "hold",
                    REINFORCED_HULL_SECTION_ID,
                    Vec3::NEG_Y,
                    Quat::IDENTITY,
                ),
                section("port", DOCKING_PORT_SECTION_ID, Vec3::NEG_Z, Quat::IDENTITY),
                section("drive", BASIC_THRUSTER_SECTION_ID, Vec3::Z, Quat::IDENTITY),
            ],
            ..default()
        }),
        inventory: ShipInventoryStock::new([(CORES.into(), 1)]),
        ..default()
    }
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use bevy::ecs::system::RunSystemOnce;
    use nova_protocol::nova_debug::harness::{AutopilotPlugin, Predicate};

    use super::*;

    /// Seconds a load or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 120.0;
    /// How close a teleported hull must settle to its berth, and how slow.
    const BERTH_TOLERANCE: f32 = 0.05;
    const BERTH_REST: f32 = 0.01;
    /// Frames the refused requests run: each must build nothing.
    const REFUSED_FRAMES: u32 = 30;

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn station_root(world: &World) -> Option<Entity> {
        let mut stations =
            world.try_query_filtered::<(Entity, &EntityId), With<StationMarker>>()?;
        stations
            .iter(world)
            .find(|(_, id)| id.0 == STATION_ID)
            .map(|(entity, _)| entity)
    }

    fn player_root(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        players.iter(world).next()
    }

    fn player(world: &World) -> Entity {
        player_root(world).expect("contraband dock probe: the player is up")
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("contraband dock probe: the station is up")
    }

    fn cores_held(world: &World) -> u32 {
        world
            .get::<ShipInventory>(player(world))
            .expect("contraband dock probe: every ship carries a hold")
            .count(&CORES.into())
    }

    /// The line the lawful station's refusal carries.
    fn refusal_line() -> String {
        format!("{STATION_NAME} REFUSES CONTRABAND")
    }

    /// What the HUD notice reads, empty while the slot is empty.
    fn notice_text(world: &mut World) -> String {
        world
            .query_filtered::<&Text, With<HudNoticeText>>()
            .iter(world)
            .map(|text| text.0.clone())
            .next()
            .unwrap_or_default()
    }

    /// What admission answers the player at the station.
    fn refusal(world: &mut World) -> Option<DockRefusalType> {
        let (player, station) = (player(world), station(world));
        world
            .run_system_once(move |admission: DockAdmission| admission.refusal(player, station))
            .expect("contraband dock probe: DockAdmission runs as an ordinary system")
    }

    /// Mark the station lawful, park the player's hull on the berth at rest
    /// and put its travel lock on the station.
    fn stage(world: &mut World) {
        let station = station(world);
        assert_eq!(
            cores_held(world),
            1,
            "contraband dock probe: the fixture hold carries one unmarked core"
        );
        let category = world.resource::<GameItems>().design(&CORES.into()).category;
        assert_eq!(
            category,
            ItemCategoryType::Contraband,
            "contraband dock probe: the base unmarked cores are contraband"
        );
        world.entity_mut(station).insert(LawfulStationMarker);
        let berth = world
            .get::<Position>(station)
            .expect("contraband dock probe: the station has a physics position")
            .0
            + BERTH_OFFSET;
        let player = player(world);
        world.entity_mut(player).insert((
            Position(berth),
            Rotation(Quat::IDENTITY),
            LinearVelocity(Vec3::ZERO),
            AngularVelocity(Vec3::ZERO),
            TravelLock(Some(station)),
        ));
    }

    fn player_berthed() -> Arc<Predicate> {
        when(|world| {
            let (Some(player), Some(station)) = (player_root(world), station_root(world)) else {
                return false;
            };
            let (Some(position), Some(velocity), Some(station_at)) = (
                world.get::<Position>(player),
                world.get::<LinearVelocity>(player),
                world.get::<Position>(station),
            ) else {
                return false;
            };
            position.0.distance(station_at.0 + BERTH_OFFSET) < BERTH_TOLERANCE
                && velocity.0.length() < BERTH_REST
        })
    }

    fn request_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
        world.trigger(DockingConnectionRequest {
            entity: player,
            target: station,
        });
    }

    fn player_docked() -> Arc<Predicate> {
        when(|world| {
            player_root(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
        })
    }

    /// The hints carry the refusal line and the HUD notice shows it.
    fn refusal_line_shown() -> Arc<Predicate> {
        when(|world| {
            let Some(hints) = world.get_resource::<FlightVerbHints>() else {
                return false;
            };
            hints.dock_refusal.as_deref() == Some(refusal_line().as_str())
                && world
                    .try_query_filtered::<&Text, With<HudNoticeText>>()
                    .is_some_and(|mut lines| lines.iter(world).any(|text| text.0 == refusal_line()))
        })
    }

    /// The HUD notice reads the refusal while it is posted: the notice fades
    /// after four seconds of play, so it is read at the lock, not after the
    /// refused requests.
    fn assert_the_notice(world: &mut World) {
        assert_eq!(
            notice_text(world),
            refusal_line(),
            "contraband dock probe: the HUD notice shows the refusal"
        );
    }

    /// Claim 0: the station refuses for contraband, the hints say so, and
    /// the requests built nothing.
    fn assert_the_refusal(world: &mut World) {
        assert_eq!(
            refusal(world),
            Some(DockRefusalType::Contraband),
            "contraband dock probe: a lawful station refuses a contraband carrier"
        );
        let hints = world.resource::<FlightVerbHints>().clone();
        assert_eq!(
            hints.dock_refusal.as_deref(),
            Some(refusal_line().as_str()),
            "contraband dock probe: the hints carry the station's refusal"
        );
        assert!(
            !hints.dock.available,
            "contraband dock probe: DOCK is not offered on a refusing station"
        );
        let line = hints.dock_refusal.unwrap_or_default();
        assert!(
            world.get::<DockedShip>(player(world)).is_none(),
            "contraband dock probe: the lawful station admitted the carrier"
        );
        let connections = world.query::<&DockingConnection>().iter(world).count();
        assert_eq!(
            connections, 0,
            "contraband dock probe: a refused dock built a connection"
        );
        assert_eq!(
            cores_held(world),
            1,
            "contraband dock probe: a refusal takes nothing from the hold"
        );
        info!("contraband dock probe: PASS the lawful station refuses the carrier: '{line}'");
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: a lawful station refuses a hull that carries contraband",
            serde_json::json!({ "t": elapsed, "line": line, "connections": connections }),
        );
    }

    /// The fixture step: the core leaves the hold.
    fn drop_the_core(world: &mut World) {
        let player = player(world);
        world
            .get_mut::<ShipInventory>(player)
            .expect("contraband dock probe: every ship carries a hold")
            .remove(&CORES.into(), 1);
    }

    /// Claim 1: with the hold empty the same lawful station docks the hull.
    fn assert_the_dock(world: &mut World) {
        let (player, station) = (player(world), station(world));
        assert_eq!(cores_held(world), 0);
        assert!(
            world.get::<LawfulStationMarker>(station).is_some(),
            "contraband dock probe: the station is still lawful"
        );
        let docked = world
            .get::<DockedShip>(player)
            .expect("contraband dock probe: the player is docked");
        let joined = world
            .get::<DockingConnection>(docked.connection)
            .expect("contraband dock probe: the dock has a connection")
            .joins(station);
        assert!(
            joined,
            "contraband dock probe: the player docked at the lawful station"
        );
        let hints = world.resource::<FlightVerbHints>().clone();
        assert_eq!(
            hints.dock_refusal, None,
            "contraband dock probe: a docked hull reads no refusal"
        );
        info!("contraband dock probe: PASS the lawful station admits the hull with no contraband");
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: the lawful station admits the hull once the contraband left",
            serde_json::json!({ "t": elapsed }),
        );
    }

    /// The walk. Every beat waits on a world value, so a slow load delays it
    /// instead of truncating it.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("contraband dock: load the void")
            .enter(GameStates::Loading)
            .until(when(|world| {
                station_root(world).is_some_and(|station| world.get::<Position>(station).is_some())
                    && player_root(world)
                        .is_some_and(|player| world.get::<Position>(player).is_some())
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("contraband dock: mark the station lawful and settle on the berth")
            .on_enter(stage)
            .until(player_berthed())
            .deadline(SESSION_SECS)
            .add()
            .step("contraband dock: the lock posts the refusal")
            .until(refusal_line_shown())
            .deadline(SESSION_SECS)
            .add()
            .step("contraband dock: assert the HUD notice")
            .on_enter(assert_the_notice)
            .until(frames(1))
            .add()
            .step("contraband dock: ask for a dock with the core aboard")
            .each(|world, _, _| request_dock(world))
            .until(frames(REFUSED_FRAMES))
            .add()
            .step("contraband dock: assert the refusal")
            .on_enter(assert_the_refusal)
            .until(frames(1))
            .add()
            .step("contraband dock: the core leaves the hold")
            .on_enter(drop_the_core)
            .until(when(|world| {
                world
                    .get_resource::<FlightVerbHints>()
                    .is_some_and(|hints| hints.dock_refusal.is_none())
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("contraband dock: assert admission with the hold empty")
            .on_enter(|world: &mut World| {
                assert_eq!(
                    refusal(world),
                    None,
                    "contraband dock probe: the lawful station admits an empty hold"
                );
            })
            .until(frames(1))
            .add()
            .step("contraband dock: dock")
            .each(|world, _, _| request_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("contraband dock: assert the dock")
            .on_enter(assert_the_dock)
            .add()
    }
}
