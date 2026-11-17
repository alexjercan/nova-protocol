//! system_contraband_trade: a lawful station docks a hull that carries
//! contraband and its market refuses the sale; a station that is not lawful
//! buys the same item at its bid.
//!
//! Two base waystations stand in a void, Neutral and living, 1 km apart.
//! THE LAWFUL MARK IS DISCLOSED FIXTURE STATE: an authored station has no
//! civilization, so the walk puts `LawfulStationMarker` on WAYSTATION by
//! hand, as the open world does for a living station of a lawful
//! civilization. FREEPORT carries none. The player's hull is a controller
//! over a hull block with a port on its nose and a drive on its tail. THE
//! HOLD AND THE BALANCE ARE DISCLOSED FIXTURE STATE: one unmarked core, the
//! base contraband item, and 100 cr. EACH BERTH IS A FIXTURE STEP: the walk
//! parks the hull on it with the travel lock on the station, where a player
//! flies there.
//!
//! The player docks at WAYSTATION with the core aboard: the lock posts no
//! refusal. In the Inventory pane the core's row opens Sell, and the form
//! refuses it with Confirm disabled. The player undocks, docks at FREEPORT
//! and sells the core there from the same form.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 0 | `outcome: a lawful station docks a hull that carries contraband` | at the lock the hints carry no refusal, the HUD notice is empty and admission answers none; the request docks the hull at the station marked lawful with the core still in its hold |
//! | 1 | `outcome: a lawful station's market refuses a contraband sale` | the Sell form shows no price (`--`, danger tone) with Confirm disabled; a click on Confirm moves nothing: the hold and both balances are unchanged |
//! | 2 | `outcome: a station that is not lawful buys contraband at its bid` | at FREEPORT the same form reads the 240 cr price; Confirm moves the core to the station and 240 cr to the player |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_contraband_trade --features debug
//! # look for: `contraband trade probe: PASS the lawful station docks the carrier`,
//! #           `contraband trade probe: PASS the lawful market refuses the sale`,
//! #           `contraband trade probe: PASS the station that is not lawful buys the core`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the refused Sell
//! form as `contraband-trade-refusal.png`.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_contraband_trade")]
#[command(version = "1.0.0")]
#[command(about = "A living lawful station docks a hull with one unmarked core in its hold and its market refuses the sale on the Inventory form; a station that is not lawful buys the core at its bid. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The lawful waystation's park, down -Z from the player.
const LAWFUL_POSITION: Meters3 = Meters3::new(0.0, 0.0, -150.0);
/// The park of the station that is not lawful, 1 km along +X.
const FREE_POSITION: Meters3 = Meters3::new(1_000.0, 0.0, -150.0);
/// The station object ids.
const LAWFUL_ID: &str = "waystation";
const FREE_ID: &str = "freeport";
/// The stations' names: the refusal names the lawful one.
const LAWFUL_NAME: &str = "WAYSTATION";
const FREE_NAME: &str = "FREEPORT";
/// The scenario id the void loads under.
const SCENARIO_ID: &str = "contraband_trade_rig";
/// The player object id.
const PLAYER_ID: &str = "player_spaceship";

/// Each station is turned a quarter turn about +Y, so its collar, which
/// stands off its -X flank, faces +Z: back toward the player.
const STATION_TURN: f32 = std::f32::consts::FRAC_PI_2;

/// Where the player's hull parks relative to a station, engine units: the
/// turned collar's face stands 2.5 cells along +Z from the station origin and
/// the player's port face 1.5 ahead of its hull origin, [`BERTH_GAP`] apart.
const BERTH_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 4.0 + BERTH_GAP);

/// The base contraband item, and the bid one unit sells at where no market
/// prices it: an authored station deals at reference.
const CORES: &str = "UnmarkedCores";
const CORES_BID: u32 = 240;

/// The disclosed fixture balance.
const PLAYER_CREDITS: u32 = 100;

/// The PNG the capture path shoots of the refused Sell form.
#[cfg(feature = "debug")]
const REFUSAL_SHOT: &str = "contraband-trade-refusal.png";

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

/// The void: the two living waystations and the player off the lawful one's
/// berth, each station lit by a three-point rig because the engine spawns no
/// light.
fn station_void(game_assets: &GameAssets) -> ScenarioConfig {
    let station = |id: &str, name: &str, position: Meters3| ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
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
            position: Meters3::from_engine(LAWFUL_POSITION.to_engine() + BERTH_OFFSET),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(player_ship()),
    };
    ScenarioConfig {
        description: "A lawful waystation, one that is not, and a player with one unmarked core."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    EventActionConfig::SpawnScenarioObject(station(
                        LAWFUL_ID,
                        LAWFUL_NAME,
                        LAWFUL_POSITION,
                    )),
                    EventActionConfig::SpawnScenarioObject(station(
                        FREE_ID,
                        FREE_NAME,
                        FREE_POSITION,
                    )),
                    EventActionConfig::SpawnScenarioObject(player),
                ],
                ThreePointRig::around("rig_lawful", LAWFUL_POSITION, 1.0).actions(),
                ThreePointRig::around("rig_free", FREE_POSITION, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Contraband Trade Rig".to_string(),
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
        credits: PLAYER_CREDITS,
        ..default()
    }
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use bevy::{ecs::system::RunSystemOnce, ui::InteractionDisabled};
    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS,
        },
        nova_interface::pane::InterfacePaneType,
    };

    use super::*;

    /// Seconds a load, a pane or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 120.0;
    /// How close a teleported hull must settle to its berth, and how slow.
    const BERTH_TOLERANCE: f32 = 0.05;
    const BERTH_REST: f32 = 0.01;
    /// Frames the hints, the Inventory pane and its form get to settle.
    const SETTLE_FRAMES: u32 = 10;

    const CORES_ROW: &str = "InventoryRowOwnUnmarkedCores";
    const DRAFT_PRICE: &str = "InventoryDraftPrice";
    const CONFIRM: &str = "InventoryDraftConfirm";

    /// The station balances the walk reads before the trades.
    #[derive(Resource, Clone, Copy, Debug)]
    struct StationBalances {
        lawful: u32,
        free: u32,
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn station_root(world: &World, id: &str) -> Option<Entity> {
        let mut stations =
            world.try_query_filtered::<(Entity, &EntityId), With<StationMarker>>()?;
        stations
            .iter(world)
            .find(|(_, found)| found.0 == id)
            .map(|(entity, _)| entity)
    }

    fn player_root(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        players.iter(world).next()
    }

    fn player(world: &World) -> Entity {
        player_root(world).expect("contraband trade probe: the player is up")
    }

    fn station(world: &World, id: &str) -> Entity {
        station_root(world, id)
            .unwrap_or_else(|| panic!("contraband trade probe: the station '{id}' is up"))
    }

    fn cores_held(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipInventory>(root)
            .expect("contraband trade probe: every ship carries a hold")
            .count(&CORES.into())
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .expect("contraband trade probe: every ship carries a balance")
            .0
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

    fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && world
                    .get_resource::<InterfacePaneType>()
                    .is_some_and(|shown| *shown == pane)
        })
    }

    fn unpaused() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
        })
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("contraband trade: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("contraband trade: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Park the player's hull on the berth of the station `id`, at rest, with
    /// its travel lock on that station.
    fn berth_at(id: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let station = station(world, id);
            let berth = world
                .get::<Position>(station)
                .expect("contraband trade probe: the station has a physics position")
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
    }

    fn berthed_at(id: &'static str) -> Arc<Predicate> {
        when(move |world| {
            let (Some(player), Some(station)) = (player_root(world), station_root(world, id))
            else {
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

    fn request_dock(world: &mut World, id: &str) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world, id);
        world.trigger(DockingConnectionRequest {
            entity: player,
            target: station,
        });
    }

    /// Whether the player is docked at the station `id`.
    fn docked_at(world: &World, id: &str) -> bool {
        let (Some(player), Some(station)) = (player_root(world), station_root(world, id)) else {
            return false;
        };
        world.get::<DockedShip>(player).is_some_and(|docked| {
            world
                .get::<DockingConnection>(docked.connection)
                .is_some_and(|connection| connection.joins(station))
        })
    }

    /// Mark the lawful station, check the fixture and record both balances.
    fn stage(world: &mut World) {
        let player = player(world);
        assert_eq!(
            cores_held(world, player),
            1,
            "contraband trade probe: the fixture hold carries one unmarked core"
        );
        assert_eq!(credits(world, player), PLAYER_CREDITS);
        let cores = world.resource::<GameItems>().design(&CORES.into()).clone();
        assert_eq!(
            (cores.category, cores.bid_cr),
            (ItemCategoryType::Contraband, CORES_BID),
            "contraband trade probe: the base unmarked cores are contraband at a {CORES_BID} cr bid"
        );
        let (lawful, free) = (station(world, LAWFUL_ID), station(world, FREE_ID));
        for station in [lawful, free] {
            let services = world
                .get::<StationServices>(station)
                .expect("contraband trade probe: a station root carries its services");
            assert!(
                services.0.contains(&StationServiceType::Trade),
                "contraband trade probe: the base waystation must offer Trade: {services:?}"
            );
            assert_eq!(
                cores_held(world, station),
                0,
                "contraband trade probe: no station stocks the cores"
            );
        }
        world.entity_mut(lawful).insert(LawfulStationMarker);
        let balances = StationBalances {
            lawful: credits(world, lawful),
            free: credits(world, free),
        };
        world.insert_resource(balances);
    }

    /// The lock on the lawful station posts no refusal: the hints carry
    /// none, the HUD notice is empty and admission answers none.
    fn assert_no_refusal(world: &mut World) {
        let (player, lawful) = (player(world), station(world, LAWFUL_ID));
        assert!(
            world.get::<LawfulStationMarker>(lawful).is_some(),
            "contraband trade probe: the station is lawful"
        );
        let refusal = world
            .run_system_once(move |admission: DockAdmission| admission.refusal(player, lawful))
            .expect("contraband trade probe: DockAdmission runs as an ordinary system");
        assert_eq!(
            refusal, None,
            "contraband trade probe: a lawful station admits a contraband carrier"
        );
        let hints = world.resource::<FlightVerbHints>().clone();
        assert_eq!(
            hints.dock_refusal, None,
            "contraband trade probe: the hints carry no refusal"
        );
        assert_eq!(
            notice_text(world),
            "",
            "contraband trade probe: the HUD posts no refusal notice"
        );
    }

    /// Claim 0: the hull is docked at the lawful station with the core
    /// aboard.
    fn assert_the_dock(world: &mut World) {
        let player = player(world);
        assert!(
            docked_at(world, LAWFUL_ID),
            "contraband trade probe: the player docked at the lawful station"
        );
        assert_eq!(
            cores_held(world, player),
            1,
            "contraband trade probe: the core is still aboard"
        );
        let notice = notice_text(world);
        assert!(
            !notice.contains("REFUSES"),
            "contraband trade probe: the dock posts no refusal: '{notice}'"
        );
        info!(
            "contraband trade probe: PASS the lawful station docks the carrier: notice '{notice}'"
        );
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: a lawful station docks a hull that carries contraband",
            serde_json::json!({ "t": elapsed, "notice": notice }),
        );
    }

    fn node_text(world: &mut World, name: &str) -> String {
        let mut texts = world.query::<(&Name, &Text)>();
        texts
            .iter(world)
            .find(|(found, _)| found.as_str() == name)
            .map(|(_, text)| text.0.clone())
            .unwrap_or_else(|| panic!("contraband trade probe: no text named '{name}'"))
    }

    fn confirm_disabled(world: &mut World) -> bool {
        let mut buttons = world.query::<(&Name, Has<InteractionDisabled>)>();
        buttons
            .iter(world)
            .find(|(found, _)| found.as_str() == CONFIRM)
            .map(|(_, disabled)| disabled)
            .expect("contraband trade probe: the form has a Confirm")
    }

    /// The Sell form at the lawful station shows no price and disables
    /// Confirm.
    fn assert_the_refused_form(world: &mut World) {
        assert_eq!(
            node_text(world, DRAFT_PRICE),
            "--",
            "contraband trade probe: the market that refuses names no price"
        );
        assert!(
            confirm_disabled(world),
            "contraband trade probe: Confirm is disabled on the refusal"
        );
    }

    /// Claim 1: the refused form stands and the click on Confirm moved
    /// nothing.
    fn assert_the_refusal(world: &mut World) {
        assert_the_refused_form(world);
        let (player, lawful) = (player(world), station(world, LAWFUL_ID));
        let balances = *world.resource::<StationBalances>();
        assert_eq!(
            (cores_held(world, player), cores_held(world, lawful)),
            (1, 0),
            "contraband trade probe: a refused sale moves no core"
        );
        assert_eq!(
            (credits(world, player), credits(world, lawful)),
            (PLAYER_CREDITS, balances.lawful),
            "contraband trade probe: a refused sale moves no credit"
        );
        let price = node_text(world, DRAFT_PRICE);
        info!("contraband trade probe: PASS the lawful market refuses the sale: price '{price}'");
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: a lawful station's market refuses a contraband sale",
            serde_json::json!({ "t": elapsed, "price": price }),
        );
    }

    /// The Sell form at the station that is not lawful states the price,
    /// with Confirm enabled.
    fn assert_the_open_form(world: &mut World) {
        assert_eq!(
            node_text(world, DRAFT_PRICE),
            format!("{CORES_BID} cr"),
            "contraband trade probe: the form states the sale"
        );
        assert!(
            !confirm_disabled(world),
            "contraband trade probe: Confirm is enabled where the market deals"
        );
    }

    /// Claim 2: the core is the station's and its bid is the player's.
    fn assert_the_sale(world: &mut World) {
        let (player, free) = (player(world), station(world, FREE_ID));
        let balances = *world.resource::<StationBalances>();
        assert!(
            world.get::<LawfulStationMarker>(free).is_none(),
            "contraband trade probe: the buying station is not lawful"
        );
        assert_eq!(
            (cores_held(world, player), cores_held(world, free)),
            (0, 1),
            "contraband trade probe: the core moved to the station"
        );
        assert_eq!(
            (credits(world, player), credits(world, free)),
            (PLAYER_CREDITS + CORES_BID, balances.free - CORES_BID),
            "contraband trade probe: the bid moved to the player"
        );
        info!(
            "contraband trade probe: PASS the station that is not lawful buys the core: +{CORES_BID} cr, player {} cr",
            credits(world, player)
        );
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: a station that is not lawful buys contraband at its bid",
            serde_json::json!({ "t": elapsed, "bid_cr": CORES_BID }),
        );
    }

    /// The walk. Every beat waits on a world value, so a slow load delays it
    /// instead of truncating it.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("contraband trade: load the void")
            .enter(GameStates::Loading)
            .until(when(|world| {
                [LAWFUL_ID, FREE_ID].into_iter().all(|id| {
                    station_root(world, id)
                        .is_some_and(|station| world.get::<Position>(station).is_some())
                }) && player_root(world)
                    .is_some_and(|player| world.get::<Position>(player).is_some())
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("contraband trade: mark the lawful station and settle on its berth")
            .on_enter(|world: &mut World| {
                stage(world);
                berth_at(LAWFUL_ID)(world);
            })
            .until(berthed_at(LAWFUL_ID))
            .deadline(SESSION_SECS)
            .add()
            .step("contraband trade: let the lock settle")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("contraband trade: assert no refusal at the lock")
            .on_enter(assert_no_refusal)
            .until(frames(1))
            .add()
            .step("contraband trade: dock with the core aboard")
            .each(|world, _, _| request_dock(world, LAWFUL_ID))
            .until(when(|world| docked_at(world, LAWFUL_ID)))
            .deadline(SESSION_SECS)
            .add()
            .step("contraband trade: assert the dock")
            .on_enter(assert_the_dock)
            .until(frames(1))
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Map),
        )
        .click_named(
            "contraband trade: open the Inventory pane",
            "InterfaceTabInventory",
            the_interface_shows(InterfacePaneType::Inventory),
            SESSION_SECS,
        )
        .step("contraband trade: settle the pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        // A row on a trading station opens Sell.
        .click_named(
            "contraband trade: pick the cores row",
            CORES_ROW,
            ui_node_present(CONFIRM),
            SESSION_SECS,
        )
        .step("contraband trade: settle the form")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("contraband trade: assert the refused form")
        .on_enter(assert_the_refused_form)
        .until(frames(1))
        .add()
        .step("contraband trade: shoot the refused form")
        .on_enter(|world: &mut World| shoot(world, REFUSAL_SHOT))
        .until(shot_written(REFUSAL_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click_named(
            "contraband trade: click the disabled Confirm",
            CONFIRM,
            frames(SETTLE_FRAMES),
            SESSION_SECS,
        )
        .step("contraband trade: assert the refusal")
        .on_enter(assert_the_refusal)
        .until(frames(1))
        .add();
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("contraband trade: undock")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(when(|world| {
                player_root(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("contraband trade: settle on the berth of the station that is not lawful")
            .on_enter(berth_at(FREE_ID))
            .until(berthed_at(FREE_ID))
            .deadline(SESSION_SECS)
            .add()
            .step("contraband trade: dock at the station that is not lawful")
            .each(|world, _, _| request_dock(world, FREE_ID))
            .until(when(|world| docked_at(world, FREE_ID)))
            .deadline(SESSION_SECS)
            .add();
        // The interface opens on the pane it closed on.
        tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Inventory),
        )
        .step("contraband trade: settle the pane again")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "contraband trade: pick the cores row again",
            CORES_ROW,
            ui_node_present(CONFIRM),
            SESSION_SECS,
        )
        .step("contraband trade: settle the form again")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("contraband trade: assert the open form")
        .on_enter(assert_the_open_form)
        .until(frames(1))
        .add()
        .click_named(
            "contraband trade: confirm the sale",
            CONFIRM,
            when(|world| cores_held(world, player(world)) == 0),
            SESSION_SECS,
        )
        .step("contraband trade: assert the sale")
        .on_enter(assert_the_sale)
        .add()
    }
}
