//! trade_loop: dock with a trader and buy, sell and get refused through the
//! Inventory pane.
//!
//! The trade rule is proved by `plan_item_trade`'s unit tests and the pane's
//! ECS examples. Neither proves the loop a player runs: lock, dock, open the
//! pane, click a row and Confirm, and read both holds and both balances move
//! together. This is where that is answered.
//!
//! The tender berths with its bow port 5 m off the trader's port, square and
//! at rest, inside the one-cell capture distance. The trader is a live spar
//! that is not lootable, so the pane trades with it: Buy on its rows and Sell
//! beside Give on yours. Its credits pay for what you sell. It exists only in
//! this example: no shipped scenario carries it.
//!
//! What to do:
//!
//! 1. HOLD the radar key on the trader until the lock takes.
//! 2. press the dock key.
//! 3. press TAB and pick Inventory.
//! 4. click Rations on the trader's side and Confirm: you pay 8 cr.
//! 5. click Hull plate on your side, pick Sell, All and Confirm: you earn
//!    120 cr.
//! 6. click Salvaged parts on the trader's side and All: 240 cr is more than
//!    you hold, so the form shows the refusal, Confirm is disabled, and a
//!    click on it moves nothing.
//!
//! ```text
//! cargo run --example trade_loop --features debug
//! ```
//!
//! Under `NOVA_AUTOPILOT=1` the script runs all six steps through the pane's
//! own named widgets. After each Confirm it waits for the pane's note line,
//! then checks both ships' stacks and credits and that no item or credit was
//! made or lost. On the refusal it waits for the summary and a disabled
//! Confirm, clicks it, and checks that no note showed and nothing moved.
//! `NOVA_CAPTURE=1` also writes one frame per result.

#[path = "shared/docking_pair.rs"]
mod docking_pair;

use bevy::prelude::*;
#[cfg(feature = "debug")]
use bevy::ui::InteractionDisabled;
use clap::Parser;
use docking_pair::{spar, tender};
#[cfg(feature = "debug")]
use nova_protocol::nova_interface::pane::InterfacePaneType;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "trade_loop")]
#[command(version = "1.0.0")]
#[command(about = "Dock with a trader and buy, sell and get refused", long_about = None)]
struct Cli;

/// Scenario id of the hull the player flies.
const TENDER_ID: &str = "trade_tender";
/// Scenario id of the trader it docks with.
const TRADER_ID: &str = "trade_trader";

/// The player hull's name, which the pane's note line reads.
const TENDER_NAME: &str = "Tender";
/// The trader's name, which the pane's note line reads.
const TRADER_NAME: &str = "Trader";

/// Where the trader stands, in meters: straight ahead, turned to face the
/// tender. Both port faces stand 25 m off their hull's origin, so this leaves
/// a 5 m face gap, half the capture distance.
const TRADER_POSITION: Meters3 = Meters3::new(0.0, 0.0, -55.0);

/// What the tender carries at spawn: plates to sell.
const TENDER_STOCK: &[(ItemType, u32)] = &[(ItemType::HullPlate, 4)];
/// The tender's credits at spawn: enough for Rations, short of two Salvaged
/// parts after the plates sell.
const TENDER_CREDITS: u32 = 100;
/// What the trader carries at spawn.
const TRADER_STOCK: &[(ItemType, u32)] = &[(ItemType::Rations, 10), (ItemType::SalvagedParts, 2)];
/// The trader's credits at spawn: enough to buy the plates.
const TRADER_CREDITS: u32 = 1_000;

/// Seconds the harnessed walk gives the scenario to load.
#[cfg(feature = "debug")]
const LOAD_DEADLINE: f32 = 30.0;

/// Seconds any later beat may stall before the walk fails it.
#[cfg(feature = "debug")]
const STEP_DEADLINE: f32 = 20.0;

/// Seconds the script holds the radar key: past the gesture threshold and the
/// acquisition dwell, as in `docking_approach`.
#[cfg(feature = "debug")]
const LOCK_HOLD: f32 = 3.0;

/// The note line after the Buy: 1 Rations at 8 cr.
#[cfg(feature = "debug")]
const BUY_NOTE: &str = "Bought 1 Rations from Trader for 8 cr";
/// The note line after the Sell: 4 Hull plates at 30 cr.
#[cfg(feature = "debug")]
const SELL_NOTE: &str = "Sold 4 Hull plate to Trader for 120 cr";
/// The form's summary for the refused Buy: 2 Salvaged parts ask 240 cr.
#[cfg(feature = "debug")]
const REFUSED_SUMMARY: &str = "Refused: Tender has only 212 cr";

/// The frames the capture path writes, one per result.
#[cfg(feature = "debug")]
const BUY_SHOT: &str = "trade-buy.png";
#[cfg(feature = "debug")]
const SELL_SHOT: &str = "trade-sell.png";
#[cfg(feature = "debug")]
const REFUSED_SHOT: &str = "trade-refused.png";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(trade_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Run timeline and engine invariants only: a pane walk holds no
        // steady-state load worth a frame-time claim.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(trade_script());
        app.add_systems(Startup, hide_dev_overlays);
    }

    app.run()
}

fn trade_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_trade);
}

fn load_trade(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(trade(&game_assets)));
}

fn ship_object(
    id: &str,
    name: &str,
    position: Meters3,
    rotation: Quat,
    ship: SpaceshipConfig,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    })
}

fn trade(game_assets: &GameAssets) -> ScenarioConfig {
    let tender = ship_object(
        TENDER_ID,
        TENDER_NAME,
        Meters3::ZERO,
        Quat::IDENTITY,
        SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            design: ShipDesignSource::Inline(tender()),
            inventory: ShipInventoryStock::new(TENDER_STOCK.iter().copied()),
            credits: TENDER_CREDITS,
            ..default()
        },
    );
    // Live and not lootable: a docked ship the pane trades with rather than
    // takes from. Its credits pay for the Sell.
    let trader = ship_object(
        TRADER_ID,
        TRADER_NAME,
        TRADER_POSITION,
        Quat::from_rotation_y(std::f32::consts::PI),
        SpaceshipConfig {
            controller: SpaceshipController::None,
            design: ShipDesignSource::Inline(spar()),
            allegiance: Some(Allegiance::Neutral),
            inventory: ShipInventoryStock::new(TRADER_STOCK.iter().copied()),
            lootable: false,
            credits: TRADER_CREDITS,
            ..default()
        },
    );

    ScenarioConfig {
        description: "A ported tender berthed at a trader".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![tender, trader],
                ThreePointRig::around("trade", TRADER_POSITION, 8.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "trade_loop".to_string(),
            "Trade Loop".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// One ship's hold and balance as the walk expects them.
#[cfg(feature = "debug")]
struct Ledger {
    stacks: &'static [(ItemType, u32)],
    credits: u32,
}

/// The tender's and the trader's ledgers after the Buy.
#[cfg(feature = "debug")]
const AFTER_BUY: (Ledger, Ledger) = (
    Ledger {
        stacks: &[(ItemType::HullPlate, 4), (ItemType::Rations, 1)],
        credits: 92,
    },
    Ledger {
        stacks: &[(ItemType::Rations, 9), (ItemType::SalvagedParts, 2)],
        credits: 1_008,
    },
);

/// The tender's and the trader's ledgers after the Sell, and after the refused
/// Buy, which moves nothing.
#[cfg(feature = "debug")]
const AFTER_SELL: (Ledger, Ledger) = (
    Ledger {
        stacks: &[(ItemType::Rations, 1)],
        credits: 212,
    },
    Ledger {
        stacks: &[
            (ItemType::HullPlate, 4),
            (ItemType::Rations, 9),
            (ItemType::SalvagedParts, 2),
        ],
        credits: 888,
    },
);

/// The ship root with scenario id `id`.
#[cfg(feature = "debug")]
fn ship(world: &mut World, id: &str) -> Entity {
    let mut roots = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    roots
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("trade_loop: no ship {id}"))
}

/// A ship's stacks in item order and its credits, read from the ECS.
#[cfg(feature = "debug")]
fn read_ledger(world: &mut World, id: &str) -> (Vec<(ItemType, u32)>, u32) {
    let root = ship(world, id);
    let stacks = world
        .get::<ShipInventory>(root)
        .unwrap_or_else(|| panic!("trade_loop: {id} has no ShipInventory"))
        .stacks()
        .collect();
    let credits = world
        .get::<ShipCredits>(root)
        .unwrap_or_else(|| panic!("trade_loop: {id} has no ShipCredits"))
        .0;
    (stacks, credits)
}

/// Assert both ships hold exactly `expected`, and that the pair holds every
/// item and credit it spawned with: a trade moves stock and credits, it never
/// makes or loses them.
#[cfg(feature = "debug")]
fn check_ledgers(world: &mut World, stage: &str, expected: &(Ledger, Ledger)) {
    let tender = read_ledger(world, TENDER_ID);
    let trader = read_ledger(world, TRADER_ID);
    for (name, (stacks, credits), want) in [
        (TENDER_NAME, &tender, &expected.0),
        (TRADER_NAME, &trader, &expected.1),
    ] {
        assert_eq!(
            stacks.as_slice(),
            want.stacks,
            "trade_loop {stage}: {name} stacks"
        );
        assert_eq!(*credits, want.credits, "trade_loop {stage}: {name} credits");
    }
    assert_eq!(
        tender.1 + trader.1,
        TENDER_CREDITS + TRADER_CREDITS,
        "trade_loop {stage}: credits made or lost"
    );
    for item in TENDER_STOCK
        .iter()
        .chain(TRADER_STOCK)
        .map(|(item, _)| *item)
    {
        let held = |stacks: &[(ItemType, u32)]| {
            stacks
                .iter()
                .filter(|(held, _)| *held == item)
                .map(|(_, count)| count)
                .sum::<u32>()
        };
        let spawned = held(TENDER_STOCK) + held(TRADER_STOCK);
        assert_eq!(
            held(&tender.0) + held(&trader.0),
            spawned,
            "trade_loop {stage}: {item:?} made or lost"
        );
    }
}

/// The ledgers the scenario spawned, checked once the pair is docked.
#[cfg(feature = "debug")]
fn check_spawned(world: &mut World) {
    let spawned = (
        Ledger {
            stacks: TENDER_STOCK,
            credits: TENDER_CREDITS,
        },
        Ledger {
            stacks: TRADER_STOCK,
            credits: TRADER_CREDITS,
        },
    );
    check_ledgers(world, "before the trades", &spawned);
}

/// Advance once exactly `count` laid-out, visible UI texts read `wanted`, as a
/// player sees them.
///
/// A count, not a presence: the refusal holds at one, the draft's summary,
/// only while the note line does not repeat it.
#[cfg(feature = "debug")]
fn text_shown(
    wanted: &'static str,
    count: usize,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query::<(&Text, &ComputedNode, &InheritedVisibility)>()
            .is_some_and(|mut texts| {
                texts
                    .iter(world)
                    .filter(|(text, node, visible)| {
                        text.0 == wanted && visible.get() && node.size().x > 0.0
                    })
                    .count()
                    == count
            })
    })
}

/// Advance once the interface shows `pane`.
#[cfg(feature = "debug")]
fn the_interface_shows(
    pane: InterfacePaneType,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world
                .get_resource::<InterfacePaneType>()
                .is_some_and(|shown| *shown == pane)
    })
}

/// Click the draft form's Confirm and wait for `note` to show.
#[cfg(feature = "debug")]
fn confirm(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    label: &str,
    note: &'static str,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script.click_named(
        &format!("{label}: confirm"),
        "InventoryDraftConfirm",
        text_shown(note, 1),
        STEP_DEADLINE,
    )
}

/// Whether the draft form's Confirm carries [`InteractionDisabled`].
#[cfg(feature = "debug")]
fn confirm_disabled(world: &World) -> bool {
    world
        .try_query::<(&Name, Has<InteractionDisabled>)>()
        .and_then(|mut buttons| {
            buttons
                .iter(world)
                .find(|(name, _)| name.as_str() == "InventoryDraftConfirm")
                .map(|(_, disabled)| disabled)
        })
        .expect("trade_loop: the draft form has a Confirm")
}

/// Write the capture of one result and wait for it.
#[cfg(feature = "debug")]
fn shoot_result(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    label: &str,
    path: &'static str,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script
        .step(format!("shoot the {label}"))
        .on_enter(move |world: &mut World| shoot(world, path))
        .until(shot_written(path))
        .deadline(STEP_DEADLINE)
        .add()
}

/// The harnessed walk: lock, dock, open the Inventory pane and run the three
/// trades through its named widgets, checking the ECS after each.
#[cfg(feature = "debug")]
fn trade_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the tender and the trader")
        .enter(GameStates::Loading)
        .until(and(state_is(GameStates::Playing), player_ship_present()))
        .deadline(LOAD_DEADLINE)
        .add()
        .step("settle the chase camera")
        .on_enter(hide_status_bar)
        .until(elapsed(1.0))
        .add()
        .step("hold the radar onto the trader")
        .on_enter(press_action("radar_hold"))
        .until(elapsed(LOCK_HOLD))
        .add()
        .step("release the radar")
        .on_enter(release_action("radar_hold"))
        .until(elapsed(0.5))
        .add()
        .step("press the dock key")
        .on_enter(press_action("dock"))
        .until(frames(1))
        .add()
        .step("let the key up and wait for the dock")
        .on_enter(release_action("dock"))
        .until(any_entity::<(With<PlayerSpaceshipMarker>, With<DockedShip>)>())
        .deadline(STEP_DEADLINE)
        .add()
        .step("check the spawned holds and balances")
        .on_enter(check_spawned)
        .until(frames(1))
        .add()
        .step("press the interface key")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("let the key up and wait for the interface")
        .on_enter(release_action("interface_toggle"))
        .until(the_interface_shows(InterfacePaneType::Map))
        .deadline(STEP_DEADLINE)
        .add()
        .click_named(
            "open the Inventory pane",
            "InterfaceTabInventory",
            ui_node_present("InventoryRowPartnerRations"),
            STEP_DEADLINE,
        );

    // BUY: the trader's row opens Buy at one unit.
    let script = script.click_named(
        "buy: pick Rations",
        "InventoryRowPartnerRations",
        ui_node_present("InventoryDraftConfirm"),
        STEP_DEADLINE,
    );
    let script = confirm(script, "buy", BUY_NOTE)
        .step("check the Buy")
        .on_enter(|world: &mut World| check_ledgers(world, "after the Buy", &AFTER_BUY))
        .until(frames(1))
        .add();
    let script = shoot_result(script, "Buy", BUY_SHOT);

    // SELL: the tender's row opens Give; the chip switches it to Sell and All
    // takes the whole stock of four.
    let script = script
        .click_named(
            "sell: pick Hull plate",
            "InventoryRowOwnHullPlate",
            ui_node_present("InventoryDraftSell"),
            STEP_DEADLINE,
        )
        .click_named(
            "sell: switch to Sell",
            "InventoryDraftSell",
            pointer_released(),
            STEP_DEADLINE,
        )
        .click_named(
            "sell: take all",
            "InventoryDraftAll",
            pointer_released(),
            STEP_DEADLINE,
        );
    let script = confirm(script, "sell", SELL_NOTE)
        .step("check the Sell")
        .on_enter(|world: &mut World| check_ledgers(world, "after the Sell", &AFTER_SELL))
        .until(frames(1))
        .add();
    let script = shoot_result(script, "Sell", SELL_SHOT);

    // REFUSED: All asks for both Salvaged parts, 240 cr against 212. One
    // part, 120 cr, is affordable, so the summary's refusal is the proof All
    // set the quantity to two. The refusal disables Confirm; a real click on
    // it must send nothing, so no note repeats the summary.
    let script = script
        .click_named(
            "refused: pick Salvaged parts",
            "InventoryRowPartnerSalvagedParts",
            ui_node_present("InventoryDraftConfirm"),
            STEP_DEADLINE,
        )
        .click_named(
            "refused: take all",
            "InventoryDraftAll",
            and(
                text_shown(REFUSED_SUMMARY, 1),
                std::sync::Arc::new(confirm_disabled),
            ),
            STEP_DEADLINE,
        )
        .click_named(
            "refused: click the disabled confirm",
            "InventoryDraftConfirm",
            pointer_released(),
            STEP_DEADLINE,
        )
        .step("let a sent command reach the note line")
        .until(frames(10))
        .add()
        .step("check the refusal moved nothing")
        .on_enter(|world: &mut World| {
            assert!(
                confirm_disabled(world),
                "trade_loop after the refused Buy: Confirm is enabled"
            );
            assert!(
                text_shown(REFUSED_SUMMARY, 1)(world),
                "trade_loop after the refused Buy: the refusal is not shown once, \
                 on the summary alone"
            );
            check_ledgers(world, "after the refused Buy", &AFTER_SELL);
        })
        .until(frames(1))
        .add();
    shoot_result(script, "refusal", REFUSED_SHOT)
}
