//! market_sandbox: dock at a generated station with iron ore in the hold, on
//! the Inventory tab, and read its computed prices by hand.
//!
//! Boots the shipped app and starts New Game as the Create World modal does,
//! with no save folder. When the window has streamed in, it picks the
//! nearest living station that trades and prices by a market of the wanted
//! cluster type, jumps the player beside it and berths it until the dock
//! takes. Then the fixture lands, once: [`ORE_COUNT`] iron ore goes into the
//! hold and the Inventory tab opens.
//!
//! Try: click Iron ore on your side. Its row and the station's read the
//! price of one with its percent off the reference, and the inspector reads
//! the reference, what the station pays and asks against it, and the
//! scarcity, cluster and civilization factors that made them. Under the
//! columns the MARKET strip names what the station pays most for and sells
//! cheapest, its restock gap and a refusal. Sell it, or Sell all, and the
//! Sell pays that bid.
//!
//! [`MARKETS`] pins one world per cluster type: seed 1 stands an
//! asteroid-rich station by the spawn and seed 7 a planet-heavy one. No
//! seed found stands both in one window. `NOVA_SANDBOX_MARKET=planet-heavy`
//! starts on the planet-heavy world; unset, the asteroid-rich one.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk sells the ore at the asteroid-rich
//! station, goes back to the main menu, sells it at the planet-heavy one, and
//! asserts each Sell paid the bid `price` computes and the asteroid-rich bid
//! is the lower, then picks the station's iron ore to open a Buy draft.
//! `NOVA_CAPTURE=1` shoots `sandbox-market-asteroid-rich.png`,
//! `sandbox-market-planet-heavy.png` and `sandbox-market-buy-all.png`. Unset,
//! no walk and no deadline run.

use avian3d::prelude::Position;
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, nova_interface::pane::InterfacePaneType, prelude::*};

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use open_world_stage::{
    berth_and_dock, body_by_id, cell_of, park_beside, the_player, window_is, window_settled,
    ChosenStation, OpenWorldStart, OpenWorldStartPlugin,
};

#[derive(Parser)]
#[command(name = "market_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Dock at a generated station with iron ore in the hold, on the Inventory tab, and \
             read its computed prices",
    long_about = None
)]
struct Cli;

/// One world per cluster type: the seed, and the cluster type of the
/// station by its spawn. Each seed's window stands one living station that
/// trades, in a cluster of that type.
const MARKETS: [(u32, ClusterType); 2] = [
    (1, ClusterType::AsteroidRich),
    (7, ClusterType::PlanetHeavy),
];
/// The start role the player spawns as: the industrial mining tug.
const SANDBOX_ROLE: &str = "industrial";
/// How much iron ore the fixture puts in the hold, as far as it has room.
const ORE_COUNT: u32 = 20;
/// How far from the station the player parks before the berth.
const PARK_DISTANCE: Meters = Meters(2_000.0);

/// The [`MARKETS`] row this run starts on: `NOVA_SANDBOX_MARKET`, else the
/// first.
///
/// # Panics
///
/// On a value that names no row's cluster type.
fn first_market() -> usize {
    let Ok(wanted) = std::env::var("NOVA_SANDBOX_MARKET") else {
        return 0;
    };
    MARKETS
        .iter()
        .position(|(_, cluster)| cluster.label() == wanted)
        .unwrap_or_else(|| {
            let known: Vec<&str> = MARKETS.iter().map(|(_, cluster)| cluster.label()).collect();
            panic!("market sandbox: NOVA_SANDBOX_MARKET={wanted} names none of {known:?}")
        })
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    // The walk sells at every market in order; a hand run starts on one.
    let first = if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        0
    } else {
        first_market()
    };
    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.add_plugins(OpenWorldStartPlugin {
        seed: MARKETS[first].0,
        role: SANDBOX_ROLE.to_string(),
    });
    app.insert_resource(Sandbox {
        market: first,
        ..default()
    });
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        read_the_market
            .in_set(diagnostics::DiagnosticsSystems)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Update, open_world_stage::hold_the_walk_clock);
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
    }
    app.add_plugins(smoke::script());

    app.run()
}

/// Where the sandbox's setup stands in the current world.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// The window around the spawn is streaming in.
    #[default]
    Streaming,
    /// The player jumped beside the station; its window is streaming in.
    Jumped,
    /// The player is put on the berth each frame until the dock takes.
    Docking,
    /// Docked, the ore in the hold and the Inventory tab open.
    Ready,
}

/// One Sell of the hold's iron ore, as the walk checked it.
#[derive(Clone, Copy, Debug)]
struct OreSale {
    /// The cluster type of the station that bought it.
    cluster: ClusterType,
    /// The price `price` computed before the Sell.
    quote: PriceQuote,
    /// How many were sold.
    sold: u32,
}

/// The sandbox's setup state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    /// The [`MARKETS`] row the current world plays.
    market: usize,
    phase: SandboxPhaseType,
    /// The chosen station.
    station: Option<ChosenStation>,
    /// The Sells the walk checked, in [`MARKETS`] order.
    sales: Vec<OreSale>,
}

impl Sandbox {
    /// The cluster type the current world's station stands in.
    fn cluster(&self) -> ClusterType {
        MARKETS[self.market].1
    }
}

/// Advance the setup one phase at a time, each on a world value.
fn drive_the_sandbox(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    match phase {
        SandboxPhaseType::Ready => {}
        SandboxPhaseType::Streaming => {
            if window_settled(world) {
                let player = the_player(world).expect("market sandbox: one player ship");
                if world.get::<DockedShip>(player).is_some() {
                    // The start docked the player at its starting station;
                    // the sandbox stages a station of its own.
                    world.trigger(DockingReleaseRequest { entity: player });
                    return;
                }
                let chosen = choose_the_station(world);
                park_beside(world, player, chosen.position, PARK_DISTANCE);
                let mut sandbox = world.resource_mut::<Sandbox>();
                sandbox.station = Some(chosen);
                sandbox.phase = SandboxPhaseType::Jumped;
            }
        }
        SandboxPhaseType::Jumped => {
            let Some(chosen) = world.resource::<Sandbox>().station.clone() else {
                return;
            };
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
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Docking;
            }
        }
        SandboxPhaseType::Docking => {
            let Some(chosen) = world.resource::<Sandbox>().station.clone() else {
                return;
            };
            let (Some(player), Some(station)) = (the_player(world), body_by_id(world, &chosen.id))
            else {
                return;
            };
            if world.get::<DockedShip>(player).is_some() {
                set_the_fixture(world, player);
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Ready;
            } else {
                berth_and_dock(world, player, station);
            }
        }
    }
}

/// The nearest living station to the origin that trades, is not on the
/// Enemy side and prices by a market of the current world's cluster type,
/// ties broken by id. The spawn's own cell counts: both pinned stations
/// stand in it.
///
/// # Panics
///
/// When the spawn's window holds none.
fn choose_the_station(world: &mut World) -> ChosenStation {
    let (seed, cluster) = MARKETS[world.resource::<Sandbox>().market];
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
                && market.cluster == cluster
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
            "market sandbox: the window on seed {seed} holds no living {} station that trades",
            cluster.label()
        );
    };
    let cell = cell_of(world, at).expect("market sandbox: the world is armed");
    info!(
        "market sandbox: seed {seed}, chose {} station '{id}' in cell {cell}, {distance:.0} m out",
        cluster.label()
    );
    ChosenStation {
        id,
        cell,
        position: Meters3::from_engine(at),
    }
}

/// Put [`ORE_COUNT`] iron ore in the hold, as far as it has room, and open
/// the Inventory tab.
///
/// # Panics
///
/// When the hold has no room for one.
fn set_the_fixture(world: &mut World, player: Entity) {
    let items = world.resource::<GameItems>().clone();
    let ore = ItemDesignId::from(ItemRoleType::IronOre);
    let mut hold = world
        .get_mut::<ShipInventory>(player)
        .expect("market sandbox: the player ship has a hold");
    let room = u64::from(hold.free_g(&items)) / items.design(&ore).stack_mass_g(1).max(1);
    let count = ORE_COUNT.min(u32::try_from(room).unwrap_or(u32::MAX));
    assert!(
        count > 0,
        "market sandbox: the hold has no room for iron ore"
    );
    hold.add(&items, &ore, count);
    *world.resource_mut::<InterfacePaneType>() = InterfacePaneType::Inventory;
    world
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Interface);
    info!("market sandbox: docked; {count} iron ore in the hold, the Inventory tab is open");
}

/// The docked station and its market, once chosen and standing.
fn the_station(world: &mut World) -> Option<(Entity, TraderMarket)> {
    let chosen = world.resource::<Sandbox>().station.clone()?;
    let station = body_by_id(world, &chosen.id)?;
    Some((station, world.get::<TraderMarket>(station)?.clone()))
}

/// The iron ore reference price, which the pane's percentages are against.
fn ore_reference(world: &World) -> ItemPrice {
    world
        .resource::<GameItems>()
        .design(&ItemDesignId::from(ItemRoleType::IronOre))
        .reference_price()
}

/// The price the station deals iron ore at now: `price` over its market and
/// its live stock.
fn ore_quote(world: &mut World) -> Option<PriceQuote> {
    let (station, market) = the_station(world)?;
    let ore = ItemDesignId::from(ItemRoleType::IronOre);
    let stock = world.get::<ShipInventory>(station)?.count(&ore);
    let items = world.get_resource::<GameItems>()?;
    let table = world.get_resource::<GamePriceTable>()?;
    Some(price(&table.0, items.design(&ore), &market, stock))
}

/// What the sandbox is for, for the diagnostics intro.
const INTRO: &str = "Docked with iron ore: click it on your side to read the station's prices.";

/// The diagnostics panel: the docked station's market, its iron ore price
/// and the Sells checked so far.
fn read_the_market(world: &mut World) {
    let cluster = world.resource::<Sandbox>().cluster();
    let market = the_station(world).map(|(_, market)| market);
    let quote = ore_quote(world);
    let sales = world.resource::<Sandbox>().sales.clone();
    let mut panel = world.resource_mut::<diagnostics::DiagnosticsPanel>();
    panel.line(format!("Wanted cluster: {}", cluster.label()));
    if let Some(market) = market {
        panel.line(format!(
            "Market: {}, led by {}",
            market.cluster.label(),
            market.leading_role().label()
        ));
    }
    if let Some(quote) = quote {
        panel.detail(format!(
            "iron ore: ask {} bid {} (scarcity {:?}, cluster {:?}, civilization {:?})",
            quote.price.ask_cr,
            quote.price.bid_cr,
            quote.scarcity,
            quote.cluster,
            quote.civilization
        ));
    }
    for sale in sales {
        panel.detail(format!(
            "sold {} at {}: {} cr each",
            sale.sold,
            sale.cluster.label(),
            sale.quote.price.bid_cr
        ));
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin, Predicate};

    use super::*;

    /// Seconds the world gets to start, stream and dock on a software GPU.
    const SETUP_SECS: f32 = 600.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Frames a pane gets to draw.
    const SETTLE_FRAMES: u32 = 10;
    /// The own-side row of the hold's iron ore.
    const ORE_ROW: &str = "InventoryRowOwnIronOre";
    /// The station's row of the iron ore it bought.
    const STATION_ORE_ROW: &str = "InventoryRowPartnerIronOre";
    /// The frame of the Buy draft with Buy all beside Confirm.
    const BUY_SHOT: &str = "sandbox-market-buy-all.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn ready_on_the_inventory_tab() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<Sandbox>()
                .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
                && world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && world.get_resource::<InterfacePaneType>() == Some(&InterfacePaneType::Inventory)
        })
    }

    fn the_player_ship(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        players.iter(world).next()
    }

    fn ore_held(world: &World) -> u32 {
        the_player_ship(world)
            .and_then(|player| world.get::<ShipInventory>(player))
            .map_or(0, |hold| {
                hold.count(&ItemDesignId::from(ItemRoleType::IronOre))
            })
    }

    fn credits(world: &World) -> u32 {
        the_player_ship(world)
            .and_then(|player| world.get::<ShipCredits>(player))
            .map_or(0, |credits| credits.0)
    }

    /// Advance once every text in `wanted` is drawn by some laid-out UI text.
    fn texts_read(wanted: Vec<String>) -> Arc<Predicate> {
        when(move |world| {
            world
                .try_query::<(&Text, &ComputedNode, &InheritedVisibility)>()
                .is_some_and(|mut texts| {
                    let shown: Vec<&str> = texts
                        .iter(world)
                        .filter(|(_, node, visible)| visible.get() && node.size().x > 0.0)
                        .map(|(text, ..)| text.0.as_str())
                        .collect();
                    wanted.iter().all(|text| shown.contains(&text.as_str()))
                })
        })
    }

    /// The walk's state for the Sell in progress: the quote and the
    /// balance and hold before it.
    #[derive(Resource, Clone, Copy, Debug)]
    struct PendingSale {
        quote: PriceQuote,
        reference: ItemPrice,
        credits: u32,
        held: u32,
    }

    /// The inspector's Pays, Asks and Cluster rows for `quote` at a station
    /// of `cluster`, as the pane draws them against the iron ore reference.
    fn price_rows(cluster: ClusterType, quote: &PriceQuote, reference: ItemPrice) -> Vec<String> {
        let factor = |factor: PriceFactor| {
            if factor.ask == factor.bid {
                format!("x{:.2}", factor.ask)
            } else {
                format!("x{:.2} / x{:.2}", factor.ask, factor.bid)
            }
        };
        vec![
            format!(
                "{} cr ({:+}%)",
                quote.price.bid_cr,
                delta_percent(quote.price.bid_cr, reference.bid_cr)
            ),
            format!(
                "{} cr ({:+}%)",
                quote.price.ask_cr,
                delta_percent(quote.price.ask_cr, reference.ask_cr)
            ),
            format!("{} {}", cluster.label(), factor(quote.cluster)),
        ]
    }

    /// Sell the hold's iron ore at the docked station through the pane and
    /// check it paid the computed bid.
    fn sell_at(
        script: AutopilotPlugin<GameStates>,
        cluster: ClusterType,
    ) -> AutopilotPlugin<GameStates> {
        let label = cluster.label();
        let shot = format!("sandbox-market-{label}.png");
        script
            .step(format!("market sandbox: docked at the {label} station"))
            .until(and(ready_on_the_inventory_tab(), ui_node_present(ORE_ROW)))
            .diagnose(|world| format!("{:?}", world.get_resource::<Sandbox>()))
            .deadline(SETUP_SECS)
            .add()
            .step(format!("market sandbox: settle the {label} pane"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .click_named(
                &format!("market sandbox: pick iron ore at the {label} station"),
                ORE_ROW,
                ui_node_present("InventoryDraftSell"),
                BEAT_SECS,
            )
            .click_named(
                &format!("market sandbox: switch to Sell at the {label} station"),
                "InventoryDraftSell",
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                &format!("market sandbox: sell all at the {label} station"),
                "InventoryDraftAll",
                pointer_released(),
                BEAT_SECS,
            )
            .step(format!(
                "market sandbox: the {label} price rows read the quote"
            ))
            .on_enter(move |world: &mut World| {
                let quote = ore_quote(world).expect("market sandbox: the docked station prices");
                let market = the_station(world)
                    .map(|(_, market)| market)
                    .expect("market sandbox: the docked station has a market");
                assert_eq!(market.cluster, cluster, "the station's market");
                let pending = PendingSale {
                    quote,
                    reference: ore_reference(world),
                    credits: credits(world),
                    held: ore_held(world),
                };
                info!("market sandbox: {label} iron ore quote {quote:?}");
                world.insert_resource(pending);
            })
            .until(and(
                when(move |world| {
                    world.get_resource::<PendingSale>().is_some_and(|pending| {
                        texts_read(price_rows(cluster, &pending.quote, pending.reference))(world)
                    })
                }),
                frames(SETTLE_FRAMES),
            ))
            .deadline(BEAT_SECS)
            .add()
            .step(format!("market sandbox: shoot the {label} prices"))
            .on_enter({
                let shot = shot.clone();
                move |world: &mut World| shoot(world, &shot)
            })
            .until(shot_written(shot))
            .deadline(SHOT_SECS)
            .add()
            .click_named(
                &format!("market sandbox: confirm the Sell at the {label} station"),
                "InventoryDraftConfirm",
                when(|world| ore_held(world) == 0),
                BEAT_SECS,
            )
            .step(format!("market sandbox: check the {label} Sell"))
            .on_enter(move |world: &mut World| {
                let pending = world
                    .remove_resource::<PendingSale>()
                    .expect("market sandbox: the quote was read");
                let earned = credits(world) - pending.credits;
                assert_eq!(
                    earned,
                    pending.held * pending.quote.price.bid_cr,
                    "the {label} Sell pays the computed bid for each ore"
                );
                let sale = OreSale {
                    cluster,
                    quote: pending.quote,
                    sold: pending.held,
                };
                info!(
                    "market sandbox: sold {} iron ore at the {label} station for {earned} cr, \
                     {} cr each",
                    sale.sold, sale.quote.price.bid_cr
                );
                world.resource_mut::<Sandbox>().sales.push(sale);
            })
            .until(frames(1))
            .add()
    }

    /// Go back to the main menu with the next market's seed queued, as the
    /// pause menu's Main Menu does in a world with no save folder.
    fn next_world(
        script: AutopilotPlugin<GameStates>,
        market: usize,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step("market sandbox: back to the main menu")
            .on_enter(move |world: &mut World| {
                {
                    let mut start = world.resource_mut::<OpenWorldStart>();
                    start.seed = MARKETS[market].0;
                    start.started = false;
                }
                {
                    let mut sandbox = world.resource_mut::<Sandbox>();
                    sandbox.market = market;
                    sandbox.phase = SandboxPhaseType::Streaming;
                    sandbox.station = None;
                }
                world
                    .resource_mut::<NextState<GameStates>>()
                    .set(GameStates::MainMenu);
                world
                    .resource_mut::<NextState<PauseStates>>()
                    .set(PauseStates::Unpaused);
            })
            .until(state_is(GameStates::MainMenu))
            .deadline(BEAT_SECS)
            .add()
            .step("market sandbox: the next world starts")
            .until(state_is(GameStates::Playing))
            .deadline(SETUP_SECS)
            .add()
    }

    /// The smoke walk: sell the ore at each market in turn, compare the bids,
    /// then open a Buy draft on the station's iron ore and shoot Buy all.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let mut script = AutopilotPlugin::<GameStates>::new();
        for (market, (_, cluster)) in MARKETS.into_iter().enumerate() {
            if market > 0 {
                script = next_world(script, market);
            }
            script = sell_at(script, cluster);
        }
        script
            .step("market sandbox: ore sells cheap where rocks are many")
            .on_enter(|world: &mut World| {
                let sales = world.resource::<Sandbox>().sales.clone();
                let bid = |cluster: ClusterType| {
                    sales
                        .iter()
                        .find(|sale| sale.cluster == cluster)
                        .map(|sale| sale.quote.price.bid_cr)
                        .unwrap_or_else(|| panic!("market sandbox: no {} sale", cluster.label()))
                };
                let (rich, heavy) = (
                    bid(ClusterType::AsteroidRich),
                    bid(ClusterType::PlanetHeavy),
                );
                assert!(
                    rich < heavy,
                    "iron ore bids {rich} cr asteroid-rich, {heavy} cr planet-heavy"
                );
                info!(
                    "market sandbox: iron ore bid {rich} cr asteroid-rich < {heavy} cr \
                     planet-heavy"
                );
            })
            .until(frames(1))
            .add()
            .click_named(
                "market sandbox: pick the station's iron ore to buy",
                STATION_ORE_ROW,
                and(
                    texts_read(vec!["Buy all".to_string()]),
                    frames(SETTLE_FRAMES),
                ),
                BEAT_SECS,
            )
            .step("market sandbox: shoot the Buy draft")
            .on_enter(|world: &mut World| shoot(world, BUY_SHOT))
            .until(shot_written(BUY_SHOT.to_string()))
            .deadline(SHOT_SECS)
            .add()
    }
}
