//! news_0160_market_inspector: the v0.16.0 news still of the Inventory pane
//! docked at a planet-heavy station with Iron ore selected
//! (`news-0160-market-inspector.png`).
//!
//! The world starts the way the Create World modal starts it, on
//! [`WORLD_SEED`] as `industrial`, with no save folder. Once the start's
//! window is in, the tug lets go of its starting station, jumps beside the
//! nearest living station that trades, is not on the Enemy side and prices
//! by a planet-heavy market, and is put on a berth until the dock takes.
//! Seed 7 stands one by the spawn, as `market_sandbox` found.
//!
//! DISCLOSED FIXTURE STATE: once docked, [`ORE_COUNT`] iron ore goes into the
//! hold, as far as it has room, as `market_sandbox` does. The tug's kit holds
//! none, and mining it is not this figure's subject.
//!
//! The walk opens the Inventory tab and clicks the hold's Iron ore row
//! through the real pointer path. Before the shot it asserts what the frame
//! must read: the station's market is planet-heavy; the row's Price is the
//! bid `price` computes, over the reference, so its Deal badge favours the
//! player; the inspector reads Reference, Pays, Asks, Scarcity, Cluster,
//! Civilization and Total from the same quote; and the MARKET strip names
//! what the station Buys and Sells.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG, staged under
//!   `NOVA_CAPTURE_DIR`.
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_market_inspector --features debug
//! ```

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

use std::sync::Arc;

use avian3d::prelude::Position;
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{
    nova_debug::harness::{AutopilotPlugin, Predicate},
    nova_interface::{inventory::prelude::INVENTORY_MARKET_STRIP, pane::InterfacePaneType},
    prelude::*,
};
use open_world_stage::{
    berth_and_dock, body_by_id, cell_of, park_beside, the_player, window_is, window_settled,
    ChosenStation, OpenWorldStartPlugin,
};

#[derive(Parser)]
#[command(name = "news_0160_market_inspector")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 market inspector: Iron ore selected on the Inventory pane at a planet-heavy station",
    long_about = None
)]
struct Cli;

/// The seed whose spawn window stands a living planet-heavy station that
/// trades.
const WORLD_SEED: u32 = 7;

/// The start role the walk flies.
const ROLE: &str = "industrial";

/// The cluster type the docked station's market must price by.
const CLUSTER: ClusterType = ClusterType::PlanetHeavy;

/// How much iron ore the fixture puts in the hold, as far as it has room.
const ORE_COUNT: u32 = 20;

/// How far from the station the player parks before the berth.
const PARK_DISTANCE: Meters = Meters(2_000.0);

/// The still this producer writes.
const SHOT_FILE: &str = "news-0160-market-inspector.png";

/// The hold's Iron ore row.
const ORE_ROW: &str = "InventoryRowOwnIronOre";

/// Real seconds the world gets to start, stream and dock on a software
/// rasterizer.
const SETUP_SECS: f32 = 600.0;

/// Real seconds a click gets.
const CLICK_SECS: f32 = 60.0;

/// Frames the pane gets to draw before the shot.
const PANE_FRAMES: u32 = 10;

/// Real seconds a frame capture gets on a software rasterizer.
const SHOT_SECS: f32 = 120.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(OpenWorldStartPlugin {
        seed: WORLD_SEED,
        role: ROLE.to_string(),
    });
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.init_resource::<Stage>();
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_systems(
        Update,
        (open_world_stage::hold_the_walk_clock, drive_the_stage)
            .run_if(in_state(GameStates::Playing)),
    );
    app.add_plugins(script());
    app.run()
}

/// Where the staging stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum StagePhaseType {
    /// The window around the start is streaming in.
    #[default]
    Streaming,
    /// The player jumped beside the station; its window is streaming in.
    Jumped,
    /// The player is put on the berth each frame until the dock takes.
    Docking,
    /// Docked, the ore in the hold and the Inventory tab open.
    Ready,
}

/// The staging state.
#[derive(Resource, Default, Debug)]
struct Stage {
    phase: StagePhaseType,
    station: Option<ChosenStation>,
}

/// Advance the staging one phase at a time.
fn drive_the_stage(world: &mut World) {
    match world.resource::<Stage>().phase {
        StagePhaseType::Ready => {}
        StagePhaseType::Streaming => {
            if !window_settled(world) {
                return;
            }
            let player = the_player(world).expect("market inspector: one player ship");
            if world.get::<DockedShip>(player).is_some() {
                // The start docked the tug at its starting station; the
                // figure docks at a station of its own.
                world.trigger(DockingReleaseRequest { entity: player });
                return;
            }
            let chosen = choose_the_station(world);
            park_beside(world, player, chosen.position, PARK_DISTANCE);
            let mut stage = world.resource_mut::<Stage>();
            stage.station = Some(chosen);
            stage.phase = StagePhaseType::Jumped;
        }
        StagePhaseType::Jumped => {
            let chosen = world
                .resource::<Stage>()
                .station
                .clone()
                .expect("market inspector: a station was chosen");
            let stands = body_by_id(world, &chosen.id).is_some_and(|station| {
                world.get::<Position>(station).is_some()
                    && world.get::<PendingParkedHulls>(station).is_none()
                    && world.get::<ParkedHulls>(station).is_none_or(|parked| {
                        parked.iter().all(|hull| {
                            matches!(
                                world.get::<ParkedHullType>(hull),
                                Some(ParkedHullType::Hire { .. })
                            ) || world.get::<DockedShip>(hull).is_some()
                        })
                    })
            });
            if window_is(world, chosen.cell) && stands {
                world.resource_mut::<Stage>().phase = StagePhaseType::Docking;
            }
        }
        StagePhaseType::Docking => {
            let chosen = world
                .resource::<Stage>()
                .station
                .clone()
                .expect("market inspector: a station was chosen");
            let (Some(player), Some(station)) = (the_player(world), body_by_id(world, &chosen.id))
            else {
                return;
            };
            if world.get::<DockedShip>(player).is_some() {
                set_the_fixture(world, player);
                world.resource_mut::<Stage>().phase = StagePhaseType::Ready;
            } else {
                berth_and_dock(world, player, station);
            }
        }
    }
}

/// The nearest living station to the origin that trades, is not on the
/// Enemy side and prices by a [`CLUSTER`] market, ties broken by id.
///
/// # Panics
///
/// When the window holds none: the seed no longer gives the figure its
/// station.
fn choose_the_station(world: &mut World) -> ChosenStation {
    let mut q_stations = world.query_filtered::<(
        &GeneratedBodyId,
        &Position,
        Option<&StationServices>,
        Has<DerelictShipMarker>,
        Option<&Allegiance>,
        &TraderMarket,
    ), With<StationMarker>>();
    let mut stations: Vec<(f32, GeneratedBodyId, Vec3)> = q_stations
        .iter(world)
        .filter(|(_, _, services, derelict, allegiance, market)| {
            station_offers(*services, *derelict, StationServiceType::Trade)
                && *allegiance != Some(&Allegiance::Enemy)
                && market.cluster == CLUSTER
        })
        .map(|(id, position, ..)| {
            (
                Meters3::from_engine(position.0)
                    .distance(Meters3::ZERO)
                    .get(),
                id.clone(),
                position.0,
            )
        })
        .collect();
    stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let Some((distance, id, at)) = stations.into_iter().next() else {
        panic!(
            "market inspector: the window on seed {WORLD_SEED} holds no living {} station that \
             trades",
            CLUSTER.label()
        );
    };
    let cell = cell_of(world, at).expect("market inspector: the world is armed");
    info!(
        "market inspector: chose {} station '{id}' in cell {cell}, {distance:.0} m out",
        CLUSTER.label()
    );
    ChosenStation {
        id,
        cell,
        position: Meters3::from_engine(at),
    }
}

/// Disclosed fixture state: [`ORE_COUNT`] iron ore in the hold, as far as it
/// has room. Then open the Inventory tab.
///
/// # Panics
///
/// When the hold has no room for one.
fn set_the_fixture(world: &mut World, player: Entity) {
    let items = world.resource::<GameItems>().clone();
    let ore = ItemDesignId::from(ItemRoleType::IronOre);
    let mut hold = world
        .get_mut::<ShipInventory>(player)
        .expect("market inspector: the tug has a hold");
    let room = u64::from(hold.free_g(&items)) / items.design(&ore).stack_mass_g(1).max(1);
    let count = ORE_COUNT.min(u32::try_from(room).unwrap_or(u32::MAX));
    assert!(
        count > 0,
        "market inspector: the hold has no room for iron ore"
    );
    hold.add(&items, &ore, count);
    info!("market inspector: fixture puts {count} iron ore in the hold");
    *world.resource_mut::<InterfacePaneType>() = InterfacePaneType::Inventory;
    world
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Interface);
}

/// The docked station and its market.
fn the_station(world: &mut World) -> (Entity, TraderMarket) {
    let chosen = world
        .resource::<Stage>()
        .station
        .clone()
        .expect("market inspector: a station was chosen");
    let station = body_by_id(world, &chosen.id).expect("market inspector: the station stands");
    let market = world
        .get::<TraderMarket>(station)
        .expect("market inspector: the station prices by a market")
        .clone();
    (station, market)
}

/// What the frame must read: the quote `price` computes for iron ore at the
/// docked station's live stock, against the reference.
#[derive(Resource, Clone, Debug)]
struct Expected {
    quote: PriceQuote,
    reference: ItemPrice,
    market: TraderMarket,
}

impl Expected {
    /// The texts the row, the inspector and the strip draw for this quote,
    /// in the pane's own formats.
    fn texts(&self) -> Vec<String> {
        let factor = |factor: PriceFactor| {
            if factor.ask == factor.bid {
                format!("x{:.2}", factor.ask)
            } else {
                format!("x{:.2} / x{:.2}", factor.ask, factor.bid)
            }
        };
        let priced = |price_cr: u32, reference_cr: u32| {
            format!(
                "{} ({:+}%)",
                cr_text(price_cr),
                delta_percent(price_cr, reference_cr)
            )
        };
        let total = if self.quote.clamped {
            format!("{} clamped", factor(self.quote.total))
        } else {
            factor(self.quote.total)
        };
        vec![
            // The row's Deal badge, bracketed as the terminal look draws it.
            format!(
                "[{:+}%]",
                delta_percent(self.quote.price.bid_cr, self.reference.bid_cr)
            ),
            // Reference, Pays and Asks.
            format!(
                "{} / {}",
                cr_text(self.reference.ask_cr),
                cr_text(self.reference.bid_cr)
            ),
            priced(self.quote.price.bid_cr, self.reference.bid_cr),
            priced(self.quote.price.ask_cr, self.reference.ask_cr),
            // Scarcity, Cluster, Civilization and Total.
            factor(self.quote.scarcity),
            format!(
                "{} {}",
                self.market.cluster.label(),
                factor(self.quote.cluster)
            ),
            format!(
                "{} {}",
                self.market.leading_role().label(),
                factor(self.quote.civilization)
            ),
            total,
            // The MARKET strip.
            "MARKET".to_string(),
            "Buys".to_string(),
            "Sells".to_string(),
        ]
    }
}

/// Compute the quote the frame must read and assert the figure's subject:
/// a planet-heavy market whose bid for iron ore is over the reference.
fn expect_the_quote(world: &mut World) {
    let (station, market) = the_station(world);
    assert_eq!(
        market.cluster, CLUSTER,
        "market inspector: the docked station's market"
    );
    let ore = ItemDesignId::from(ItemRoleType::IronOre);
    let stock = world
        .get::<ShipInventory>(station)
        .expect("market inspector: the station has a hold")
        .count(&ore);
    let items = world.resource::<GameItems>();
    let design = items.design(&ore);
    let reference = design.reference_price();
    let quote = price(
        &world.resource::<GamePriceTable>().0,
        design,
        &market,
        stock,
    );
    assert!(
        quote.price.bid_cr > reference.bid_cr,
        "market inspector: the {} bid for iron ore, {} cr, is not over the reference {} cr, so \
         the Deal badge would not favour the player",
        CLUSTER.label(),
        quote.price.bid_cr,
        reference.bid_cr
    );
    info!(
        "market inspector: iron ore at stock {stock}: {quote:?} against reference {reference:?}, \
         market led by {}",
        market.leading_role().label()
    );
    world.insert_resource(Expected {
        quote,
        reference,
        market,
    });
}

fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(check)
}

/// Every text in `wanted` drawn by some visible UI text laid out with a
/// width and a height.
fn texts_drawn(world: &World, wanted: &[String]) -> bool {
    world
        .try_query::<(&Text, &ComputedNode, &InheritedVisibility)>()
        .is_some_and(|mut texts| {
            let shown: Vec<&str> = texts
                .iter(world)
                .filter(|(_, node, visible)| visible.get() && node.size().min_element() > 0.0)
                .map(|(text, ..)| text.0.as_str())
                .collect();
            wanted.iter().all(|text| shown.contains(&text.as_str()))
        })
}

/// The pane draws everything [`Expected`] names.
fn the_pane_reads_the_quote() -> Arc<Predicate> {
    when(|world| {
        world
            .get_resource::<Expected>()
            .is_some_and(|expected| texts_drawn(world, &expected.texts()))
    })
}

/// The Inventory tab is open over the docked station with the ore in hand.
fn ready_on_the_inventory_tab() -> Arc<Predicate> {
    when(|world| {
        world
            .get_resource::<Stage>()
            .is_some_and(|stage| stage.phase == StagePhaseType::Ready)
            && world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world.get_resource::<InterfacePaneType>() == Some(&InterfacePaneType::Inventory)
    })
}

/// The walk: dock, select the hold's Iron ore, check the drawn quote, shoot.
fn script() -> AutopilotPlugin<GameStates> {
    AutopilotPlugin::<GameStates>::new()
        .step("market inspector: docked on the Inventory tab")
        .until(and(ready_on_the_inventory_tab(), ui_node_present(ORE_ROW)))
        .diagnose(|world| format!("{:?}", world.get_resource::<Stage>()))
        .deadline(SETUP_SECS)
        .add()
        .step("market inspector: settle the pane")
        .on_enter(hide_status_bar)
        .until(frames(PANE_FRAMES))
        .add()
        .click_named(
            "market inspector: select the hold's Iron ore",
            ORE_ROW,
            ui_node_present("InventoryDraftSell"),
            CLICK_SECS,
        )
        .step("market inspector: the row, the inspector and the strip read the quote")
        .on_enter(expect_the_quote)
        .until(and(
            the_pane_reads_the_quote(),
            ui_node_present(INVENTORY_MARKET_STRIP),
        ))
        .diagnose(|world| format!("{:?}", world.get_resource::<Expected>()))
        .deadline(CLICK_SECS)
        .add()
        .step("market inspector: settle the inspector")
        .until(frames(PANE_FRAMES))
        .add()
        .step("market inspector: shoot the pane")
        .on_enter(|world: &mut World| shoot(world, SHOT_FILE))
        .until(shot_written(SHOT_FILE))
        .deadline(SHOT_SECS)
        .add()
}
