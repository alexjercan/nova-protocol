//! refine_sandbox: start docked at a station that refines, with ore and
//! credits aboard, and refine alloy plates from the Inventory pane.
//!
//! The shipped waystation (`block_waystation`) stands in a void; Refine is
//! one of its services. The player's hull is the tender from
//! `shared/docking_pair.rs`: three hull sections, so a 300 kg hold. THE HOLD
//! AND THE BALANCE ARE DISCLOSED FIXTURE STATE, and `--short` chooses them:
//!
//! | `--short` | iron ore | carbon ore | credits | the Refine form |
//! | - | - | - | - | - |
//! | `none` (default) | 12 | 6 | 300 | open: six batches, 30 cr each |
//! | `ore` | 4 | 0 | 300 | refused: names the carbon ore |
//! | `credits` | 12 | 6 | 20 | refused: names the fee |
//!
//! The dock is made by the docking verb itself, with the hull put on the
//! berth until the dock takes. What to do:
//!
//! 1. press Tab and open the Inventory tab;
//! 2. click Iron ore (or Carbon ore) in your column;
//! 3. click Refine: the form states one batch and the balance after it;
//! 4. click Confirm. Two iron ore and one carbon ore leave the hold, one
//!    alloy plate enters it, and the fee goes to the station.
//!
//! The diagnostics layer (F3) is on at start: a panel line with the ore, the
//! plates and the credits aboard, and how many batches the hold covers.
//! Press D to undock; the station stays in reach to dock again.
//!
//! ```text
//! nix develop --command cargo run --example refine_sandbox --features debug
//! nix develop --command cargo run --example refine_sandbox --features debug -- --short ore
//! nix develop --command cargo run --example refine_sandbox --features debug -- --short credits
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk checks the staging, opens the Inventory pane, picks the iron ore row
//! and the Refine chip, checks that the form is open or refused as the flag
//! says, and exits. `NOVA_CAPTURE=1` shoots the docked flight view and the
//! form.

#[expect(dead_code, reason = "this example flies the tender and moors no spar")]
#[path = "shared/docking_pair.rs"]
mod docking_pair;
#[path = "shared/open_world_stage.rs"]
mod open_world_stage;
#[path = "shared/station_void.rs"]
mod station_void;

use bevy::prelude::*;
use clap::{Parser, ValueEnum};
use diagnostics::{DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};

/// Which input of a refine batch the fixture leaves short.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RefineShortType {
    /// Ore and credits for six batches.
    #[default]
    None,
    /// No carbon ore: the form refuses by the ore.
    Ore,
    /// 20 cr against the 30 cr fee: the form refuses by the fee.
    Credits,
}

#[derive(Parser, Resource, Clone, Copy)]
#[command(name = "refine_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start docked at a station that refines, with ore and credits aboard, and refine \
             alloy plates from the Inventory pane",
    long_about = None
)]
struct Cli {
    /// The input the fixture leaves short, so the Refine form refuses.
    #[arg(long, value_enum, default_value_t = RefineShortType::None)]
    short: RefineShortType,
}

/// The waystation's park, meters.
const STATION_POSITION: Meters3 = Meters3::new(0.0, 0.0, -150.0);
/// The station's scenario id and name.
const STATION_ID: &str = "waystation";
const STATION_NAME: &str = "WAYSTATION";
/// The player's ship name, which a refusal names.
const PLAYER_NAME: &str = "Tender";

/// The refined item and the two ores of its base recipe.
const ALLOY: &str = "AlloyPlate";
const IRON: &str = "IronOre";
const CARBON: &str = "CarbonOre";

impl RefineShortType {
    /// The fixture's iron ore, carbon ore and credits.
    fn fixture(self) -> (u32, u32, u32) {
        match self {
            Self::None => (12, 6, 300),
            Self::Ore => (4, 0, 300),
            Self::Credits => (12, 6, 20),
        }
    }

    /// The intro line.
    fn intro(self) -> &'static str {
        match self {
            Self::None => "Docked with ore: Tab, Inventory, click Iron ore, Refine, Confirm.",
            Self::Ore => {
                "Docked with no carbon ore: Tab, Inventory, click Iron ore, Refine is refused."
            }
            Self::Credits => {
                "Docked with 20 cr: Tab, Inventory, click Iron ore, Refine is refused by the fee."
            }
        }
    }
}

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(refine_plugin).build();
    app.insert_resource(cli);
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(cli.short.intro()));
    app.add_systems(
        PostUpdate,
        draw_the_refine
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
    app.add_plugins(smoke::script(cli.short));

    app.run()
}

fn refine_plugin(app: &mut App) {
    app.init_resource::<Staging>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_the_void);
    app.add_systems(Update, stage_the_dock.run_if(in_state(GameStates::Playing)));
}

fn load_the_void(mut commands: Commands, game_assets: Res<GameAssets>, cli: Res<Cli>) {
    let (iron, carbon, credits) = cli.short.fixture();
    let player = station_void::player(
        PLAYER_NAME,
        station_void::tender_berth(STATION_POSITION),
        SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            design: ShipDesignSource::Inline(docking_pair::tender()),
            inventory: station_void::stock(&[(IRON, iron), (CARBON, carbon)]),
            credits,
            ..default()
        },
    );
    commands.trigger(LoadScenario(station_void::void(
        "refine_sandbox",
        "Refine Sandbox",
        vec![
            station_void::waystation(STATION_ID, STATION_NAME, STATION_POSITION),
            player,
        ],
        &[(STATION_POSITION, 1.0)],
        &game_assets,
    )));
}

/// Whether the first dock was staged. The staging stops there, so the player
/// can undock and fly.
#[derive(Resource, Default)]
struct Staging {
    docked: bool,
}

/// Put the player on the waystation's berth and ask to dock, each frame
/// until the dock takes.
fn stage_the_dock(world: &mut World) {
    if world.resource::<Staging>().docked {
        return;
    }
    if station_void::berth_and_dock_at(world, STATION_ID) {
        world.resource_mut::<Staging>().docked = true;
        info!("refine_sandbox: docked at {STATION_NAME}; TAB opens the interface");
    }
}

/// The panel: what the hold carries of the recipe, and the batches it covers.
fn draw_the_refine(
    mut panel: ResMut<DiagnosticsPanel>,
    items: Res<GameItems>,
    pause: Option<Res<State<PauseStates>>>,
    q_player: Query<(&ShipInventory, &ShipCredits, Has<DockedShip>), With<PlayerSpaceshipMarker>>,
) {
    let Ok((hold, credits, docked)) = q_player.single() else {
        return;
    };
    let alloy = items.design(&ALLOY.into());
    let Some(recipe) = alloy.refine.as_ref() else {
        panel.line(format!("{} has no refine recipe", alloy.name));
        return;
    };
    let held: Vec<String> = recipe
        .inputs
        .iter()
        .map(|(item, _)| format!("{} {}", hold.count(item), items.design(item).name))
        .collect();
    panel.line(format!(
        "Hold: {}, {} {}, {} cr",
        held.join(", "),
        hold.count(&ALLOY.into()),
        alloy.name,
        credits.0
    ));
    let by_ore = recipe.batches_from(hold);
    let by_fee = credits.0.checked_div(recipe.fee_cr).unwrap_or(u32::MAX);
    panel.line(format!(
        "Batches covered: {} (ore covers {by_ore}, credits cover {by_fee} at {} cr each)",
        by_ore.min(by_fee),
        recipe.fee_cr
    ));
    panel.detail(format!(
        "Hold load: {} / {}",
        kg_text(u64::from(hold.used_g(&items))),
        kg_text(u64::from(hold.capacity_g()))
    ));
    if !docked {
        panel.hint("Ctrl", "hold on the station to lock it");
        panel.hint("D", "dock");
    } else if station_void::interface_up(pause.as_deref()) {
        panel.hint("Inventory", "click Iron ore, then Refine, then Confirm");
    } else {
        panel.hint("TAB", "Inventory > Iron ore > Refine");
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

    const IRON_ROW: &str = "InventoryRowOwnIronOre";
    const REFINE_CHIP: &str = "InventoryDraftRefine";

    /// The frames the capture path writes.
    const DOCKED_SHOT: &str = "refine-sandbox-docked.png";
    const FORM_SHOT: &str = "refine-sandbox-form.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The staging: docked at the waystation, which refines, with the
    /// fixture hold and balance.
    fn assert_the_staging(short: RefineShortType) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let station = station_void::station_root(world, STATION_ID)
                .expect("refine_sandbox: the waystation is up");
            assert!(
                station_void::docked_at(world, STATION_ID),
                "refine_sandbox: the player starts docked at the waystation"
            );
            let services = world
                .get::<StationServices>(station)
                .expect("refine_sandbox: a station root carries its services");
            assert!(
                services.0.contains(&StationServiceType::Refine),
                "refine_sandbox: the waystation must offer Refine: {services:?}"
            );
            let player = station_void::player_root(world).expect("refine_sandbox: a player");
            station_void::assert_at_rest(world, player, station);
            let hold = world
                .get::<ShipInventory>(player)
                .expect("refine_sandbox: the player carries a hold");
            let got = (
                hold.count(&IRON.into()),
                hold.count(&CARBON.into()),
                world.get::<ShipCredits>(player).map(|credits| credits.0),
            );
            let (iron, carbon, credits) = short.fixture();
            assert_eq!(
                got,
                (iron, carbon, Some(credits)),
                "refine_sandbox: the fixture hold and balance"
            );
            info!("refine_sandbox: PASS staged docked with {got:?}");
        }
    }

    /// The form reads what the flag promises: an open batch, or a refusal
    /// by the short input with Confirm disabled.
    fn assert_the_form(short: RefineShortType) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let summary = station_void::node_text(world, station_void::DRAFT_SUMMARY)
                .expect("refine_sandbox: the form has a summary");
            let disabled = station_void::node_disabled(world, station_void::DRAFT_CONFIRM)
                .expect("refine_sandbox: the form has a Confirm");
            let (_, _, credits) = short.fixture();
            let batch = "2 Iron ore + 1 Carbon ore + 30 cr -> 1 Alloy plate";
            let wanted = match short {
                RefineShortType::None => format!("{batch}\nBalance {} cr", credits - 30),
                RefineShortType::Ore => {
                    format!("{batch}\nRefused: only 0 Carbon ore in {PLAYER_NAME}, needs 1")
                }
                RefineShortType::Credits => format!(
                    "{batch}\nRefused: {PLAYER_NAME} has only {credits} cr, the fee is 30 cr"
                ),
            };
            assert_eq!(summary, wanted, "refine_sandbox: the Refine form");
            assert_eq!(
                disabled,
                short != RefineShortType::None,
                "refine_sandbox: Confirm is disabled only on a refusal"
            );
            info!("refine_sandbox: PASS the Refine form reads {summary:?}");
        }
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script(short: RefineShortType) -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("refine sandbox: load the void and dock")
            .enter(GameStates::Loading)
            .until(when(|world| {
                world
                    .get_resource::<Staging>()
                    .is_some_and(|staging| staging.docked)
            }))
            .deadline(LOAD_SECS)
            .add()
            .step("refine sandbox: check the staging")
            .on_enter(assert_the_staging(short))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("refine sandbox: shoot the docked view")
            .on_enter(|world: &mut World| shoot(world, DOCKED_SHOT))
            .until(shot_written(DOCKED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("refine sandbox: open the Inventory pane")
            .on_enter(|world: &mut World| {
                station_void::open_pane(world, InterfacePaneType::Inventory);
            })
            .until(ui_node_present(IRON_ROW))
            .deadline(BEAT_SECS)
            .add()
            .step("refine sandbox: settle the pane")
            .until(frames(SETTLE_FRAMES))
            .add()
            .click_named(
                "refine sandbox: pick the iron ore row",
                IRON_ROW,
                ui_node_present(REFINE_CHIP),
                BEAT_SECS,
            )
            .click_named(
                "refine sandbox: switch the draft to Refine",
                REFINE_CHIP,
                frames(SETTLE_FRAMES),
                BEAT_SECS,
            )
            .step("refine sandbox: check the form")
            .on_enter(assert_the_form(short))
            .until(frames(1))
            .add()
            .step("refine sandbox: shoot the form")
            .on_enter(|world: &mut World| shoot(world, FORM_SHOT))
            .until(shot_written(FORM_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }
}
