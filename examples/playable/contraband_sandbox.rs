//! contraband_sandbox: start docked at a lawful station with contraband in
//! the hold, with a station that is not lawful a short flight away, and try
//! to sell at both.
//!
//! Two shipped waystations stand in a void, Neutral and living, 400 m apart.
//! THE LAWFUL MARK IS DISCLOSED FIXTURE STATE: an authored station has no
//! civilization, so the sandbox puts `LawfulStationMarker` on WAYSTATION by
//! hand, as the open world does for a living station of a lawful
//! civilization. FREEPORT carries none. The player's hull is the tender from
//! `shared/docking_pair.rs`. THE HOLD AND THE BALANCE ARE DISCLOSED FIXTURE
//! STATE: three unmarked cores, the base contraband item, and 100 cr.
//!
//! `--at` chooses the station the run starts docked at: `lawful` (default)
//! or `freeport`. The dock is made by the docking verb itself, with the hull
//! put on the berth until the dock takes. What to do:
//!
//! 1. press Tab and open the Inventory tab;
//! 2. click Unmarked cores in your column: the row opens Sell;
//! 3. at WAYSTATION the form reads `Refused: WAYSTATION does not deal in
//!    contraband` and Confirm is disabled. The station still docks you, and
//!    its column lists no contraband;
//! 4. press Tab, then D to undock. Fly to FREEPORT, hold Ctrl on it to lock
//!    it, press D to dock;
//! 5. at FREEPORT the same form reads the 240 cr price. Confirm sells.
//!
//! The diagnostics layer (F3) is on at start: a label on each station with
//! its law and its range, a line to the station you are not docked at, and a
//! panel line with the cores and the credits aboard.
//!
//! ```text
//! nix develop --command cargo run --example contraband_sandbox --features debug
//! nix develop --command cargo run --example contraband_sandbox --features debug -- --at freeport
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk checks the staging, opens the Inventory pane, picks the cores row,
//! checks that the Sell form is refused or open as the start station
//! promises, and exits. `NOVA_CAPTURE=1` shoots the docked flight view and
//! the form.

#[expect(dead_code, reason = "this example flies the tender and moors no spar")]
#[path = "shared/docking_pair.rs"]
mod docking_pair;
#[path = "shared/open_world_stage.rs"]
mod open_world_stage;
#[path = "shared/station_void.rs"]
mod station_void;

use bevy::prelude::*;
use clap::{Parser, ValueEnum};
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};

/// The station the run starts docked at.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
enum StartStationType {
    /// WAYSTATION, marked lawful: the sale is refused.
    #[default]
    Lawful,
    /// FREEPORT, not lawful: the sale goes through.
    Freeport,
}

impl StartStationType {
    /// The scenario id of the station.
    fn id(self) -> &'static str {
        match self {
            Self::Lawful => LAWFUL_ID,
            Self::Freeport => FREE_ID,
        }
    }

    /// The station's park.
    fn position(self) -> Meters3 {
        match self {
            Self::Lawful => LAWFUL_POSITION,
            Self::Freeport => FREE_POSITION,
        }
    }

    /// The intro line.
    fn intro(self) -> &'static str {
        match self {
            Self::Lawful => {
                "Docked at a lawful station with contraband: Tab, Inventory, click Unmarked cores."
            }
            Self::Freeport => {
                "Docked at a freeport with contraband: Tab, Inventory, click Unmarked cores, \
                 Confirm."
            }
        }
    }
}

#[derive(Parser, Resource, Clone, Copy)]
#[command(name = "contraband_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start docked at a lawful station with contraband in the hold and a station that \
             is not lawful a short flight away, and try to sell at both",
    long_about = None
)]
struct Cli {
    /// The station the run starts docked at.
    #[arg(long, value_enum, default_value_t = StartStationType::Lawful)]
    at: StartStationType,
}

/// The lawful waystation's park, meters.
const LAWFUL_POSITION: Meters3 = Meters3::new(0.0, 0.0, -150.0);
/// The park of the station that is not lawful, 400 m along +X: a short
/// flight by hand.
const FREE_POSITION: Meters3 = Meters3::new(400.0, 0.0, -150.0);
/// The station object ids.
const LAWFUL_ID: &str = "waystation";
const FREE_ID: &str = "freeport";
/// The stations' names: the refusal names the lawful one.
const LAWFUL_NAME: &str = "WAYSTATION";
const FREE_NAME: &str = "FREEPORT";
/// The player's ship name.
const PLAYER_NAME: &str = "Tender";

/// The base contraband item, how many the fixture hold carries, and the bid
/// one unit sells at where no market prices it.
const CORES: &str = "UnmarkedCores";
const HOLD_CORES: u32 = 3;
const CORES_BID: u32 = 240;
/// The fixture balance.
const PLAYER_CREDITS: u32 = 100;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins(contraband_plugin)
        .build();
    app.insert_resource(cli);
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(cli.at.intro()));
    app.add_systems(
        PostUpdate,
        draw_the_stations
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Startup, force_capture_resolution);
    }
    app.add_plugins(smoke::script(cli.at));

    app.run()
}

fn contraband_plugin(app: &mut App) {
    app.init_resource::<Staging>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_the_void);
    app.add_systems(Update, stage_the_dock.run_if(in_state(GameStates::Playing)));
}

fn load_the_void(mut commands: Commands, game_assets: Res<GameAssets>, cli: Res<Cli>) {
    let player = station_void::player(
        PLAYER_NAME,
        station_void::tender_berth(cli.at.position()),
        SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            design: ShipDesignSource::Inline(docking_pair::tender()),
            inventory: station_void::stock(&[(CORES, HOLD_CORES)]),
            credits: PLAYER_CREDITS,
            ..default()
        },
    );
    commands.trigger(LoadScenario(station_void::void(
        "contraband_sandbox",
        "Contraband Sandbox",
        vec![
            station_void::waystation(LAWFUL_ID, LAWFUL_NAME, LAWFUL_POSITION),
            station_void::waystation(FREE_ID, FREE_NAME, FREE_POSITION),
            player,
        ],
        &[(LAWFUL_POSITION, 1.0), (FREE_POSITION, 1.0)],
        &game_assets,
    )));
}

/// Whether the first dock was staged. The staging stops there, so the player
/// can undock and fly to the other station.
#[derive(Resource, Default)]
struct Staging {
    docked: bool,
}

/// Mark WAYSTATION lawful, then put the player on the start station's berth
/// and ask to dock, each frame until the dock takes.
fn stage_the_dock(world: &mut World) {
    if world.resource::<Staging>().docked {
        return;
    }
    let Some(lawful) = station_void::station_root(world, LAWFUL_ID) else {
        return;
    };
    if world.get::<LawfulStationMarker>(lawful).is_none() {
        world.entity_mut(lawful).insert(LawfulStationMarker);
    }
    let at = world.resource::<Cli>().at;
    if station_void::berth_and_dock_at(world, at.id()) {
        world.resource_mut::<Staging>().docked = true;
        info!(
            "contraband_sandbox: docked at {}; TAB opens the interface",
            at.id()
        );
    }
}

/// A label on each station with its law and its range, a line to the
/// station the player is not docked at, and the panel.
fn draw_the_stations(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    pause: Option<Res<State<PauseStates>>>,
    q_player: Query<
        (
            &GlobalTransform,
            &ShipInventory,
            &ShipCredits,
            Option<&DockedShip>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_stations: Query<
        (Entity, &Name, &GlobalTransform, Has<LawfulStationMarker>),
        With<StationMarker>,
    >,
    q_connections: Query<&DockingConnection>,
) {
    let Ok((at, hold, credits, docked)) = q_player.single() else {
        return;
    };
    let connection = docked.and_then(|docked| q_connections.get(docked.connection).ok());
    let mut partner = None;
    for (station, name, station_at, lawful) in &q_stations {
        let here = connection.is_some_and(|connection| connection.joins(station));
        let range = Meters::from_engine(station_at.translation().distance(at.translation()));
        let (law, color) = if lawful {
            ("lawful: refuses contraband", palette::ALERT)
        } else {
            ("not lawful: buys contraband", palette::OK)
        };
        if here {
            partner = Some((name.to_string(), lawful));
        } else {
            gizmos.line(at.translation(), station_at.translation(), color);
        }
        panel.label_tinted(
            station_at.translation(),
            format!("{name}\n{law}\n{:.0} m", range.0),
            color,
        );
    }
    panel.line(format!(
        "Hold: {} Unmarked cores, {} cr",
        hold.count(&CORES.into()),
        credits.0
    ));
    match &partner {
        Some((name, true)) => panel.line(format!("Docked at {name}: the Sell form is refused")),
        Some((name, false)) => {
            panel.line(format!("Docked at {name}: a core sells at {CORES_BID} cr"));
        }
        None => panel.line("Not docked: fly to a station, lock it and dock"),
    }
    if partner.is_none() {
        panel.hint("Ctrl", "hold on a station to lock it");
        panel.hint("D", "dock");
    } else if station_void::interface_up(pause.as_deref()) {
        panel.hint("Inventory", "click Unmarked cores, read the Sell form");
    } else {
        panel.hint("TAB", "Inventory > Unmarked cores");
        panel.hint("D", "undock");
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS},
        nova_interface::pane::InterfacePaneType,
    };

    use super::*;

    /// Seconds the walk gives the void to load and the dock to take.
    const LOAD_SECS: f32 = 120.0;
    /// Seconds any later beat may stall before the walk fails it.
    const BEAT_SECS: f32 = 60.0;
    /// Frames the Inventory pane gets to build its rows and its form.
    const SETTLE_FRAMES: u32 = 10;

    const CORES_ROW: &str = "InventoryRowOwnUnmarkedCores";

    /// The frames the capture path writes.
    const DOCKED_SHOT: &str = "contraband-sandbox-docked.png";
    const FORM_SHOT: &str = "contraband-sandbox-form.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The staging: docked and at rest at the start station, both stations
    /// trade, only WAYSTATION is lawful, and the hold carries the cores.
    fn assert_the_staging(at: StartStationType) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let lawful = station_void::station_root(world, LAWFUL_ID)
                .expect("contraband_sandbox: WAYSTATION is up");
            let free = station_void::station_root(world, FREE_ID)
                .expect("contraband_sandbox: FREEPORT is up");
            let player = station_void::player_root(world).expect("contraband_sandbox: a player");
            assert!(
                station_void::docked_at(world, at.id()),
                "contraband_sandbox: the player starts docked at '{}'",
                at.id()
            );
            let start = if at == StartStationType::Lawful {
                lawful
            } else {
                free
            };
            station_void::assert_at_rest(world, player, start);
            for station in [lawful, free] {
                let services = world
                    .get::<StationServices>(station)
                    .expect("contraband_sandbox: a station root carries its services");
                assert!(
                    services.0.contains(&StationServiceType::Trade),
                    "contraband_sandbox: the waystation must offer Trade: {services:?}"
                );
            }
            assert!(
                world.get::<LawfulStationMarker>(lawful).is_some()
                    && world.get::<LawfulStationMarker>(free).is_none(),
                "contraband_sandbox: only WAYSTATION is lawful"
            );
            let cores = world.resource::<GameItems>().design(&CORES.into()).clone();
            assert_eq!(
                (cores.category, cores.bid_cr),
                (ItemCategoryType::Contraband, CORES_BID),
                "contraband_sandbox: the base unmarked cores are contraband at a {CORES_BID} cr bid"
            );
            let held = world
                .get::<ShipInventory>(player)
                .expect("contraband_sandbox: the player carries a hold")
                .count(&CORES.into());
            assert_eq!(held, HOLD_CORES, "contraband_sandbox: the fixture hold");
            info!(
                "contraband_sandbox: PASS staged docked at '{}' with {held} cores",
                at.id()
            );
        }
    }

    /// The Sell form reads what the start station promises: the refusal
    /// with Confirm disabled, or the price with Confirm enabled.
    fn assert_the_form(at: StartStationType) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let summary = station_void::node_text(world, station_void::DRAFT_SUMMARY)
                .expect("contraband_sandbox: the form has a summary");
            let disabled = station_void::node_disabled(world, station_void::DRAFT_CONFIRM)
                .expect("contraband_sandbox: the form has a Confirm");
            let wanted = match at {
                StartStationType::Lawful => {
                    format!("Refused: {LAWFUL_NAME} does not deal in contraband")
                }
                StartStationType::Freeport => format!(
                    "Price {CORES_BID} cr  Balance {} cr",
                    PLAYER_CREDITS + CORES_BID
                ),
            };
            assert_eq!(summary, wanted, "contraband_sandbox: the Sell form");
            assert_eq!(
                disabled,
                at == StartStationType::Lawful,
                "contraband_sandbox: Confirm is disabled only at the lawful station"
            );
            info!("contraband_sandbox: PASS the Sell form reads {summary:?}");
        }
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script(at: StartStationType) -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("contraband sandbox: load the void and dock")
            .enter(GameStates::Loading)
            .until(when(|world| {
                world
                    .get_resource::<Staging>()
                    .is_some_and(|staging| staging.docked)
            }))
            .deadline(LOAD_SECS)
            .add()
            .step("contraband sandbox: check the staging")
            .on_enter(assert_the_staging(at))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("contraband sandbox: shoot the docked view")
            .on_enter(|world: &mut World| shoot(world, DOCKED_SHOT))
            .until(shot_written(DOCKED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("contraband sandbox: open the Inventory pane")
            .on_enter(|world: &mut World| {
                station_void::open_pane(world, InterfacePaneType::Inventory);
            })
            .until(ui_node_present(CORES_ROW))
            .deadline(BEAT_SECS)
            .add()
            .step("contraband sandbox: settle the pane")
            .until(frames(SETTLE_FRAMES))
            .add()
            .click_named(
                "contraband sandbox: pick the cores row",
                CORES_ROW,
                ui_node_present(station_void::DRAFT_CONFIRM),
                BEAT_SECS,
            )
            .step("contraband sandbox: settle the form")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("contraband sandbox: check the form")
            .on_enter(assert_the_form(at))
            .until(frames(1))
            .add()
            .step("contraband sandbox: shoot the form")
            .on_enter(|world: &mut World| shoot(world, FORM_SHOT))
            .until(shot_written(FORM_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }
}
