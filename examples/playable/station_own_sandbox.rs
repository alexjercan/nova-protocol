//! station_own_sandbox: start docked at a living station with the credits to
//! buy it, buy it, and check it is the player's.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the
//! scavenger, as the Create World modal does after the role page, but
//! without a save folder: nothing this run does is saved. When the origin's
//! window has streamed in, it picks the nearest generated station that is
//! living and on the Neutral side, outside the origin's cell. The player
//! jumps beside it and is put on a berth at its collar each frame until the
//! dock takes.
//! Then the fixture lands, once: the station's civilization is warmed to
//! Ally by merits, the way a player who hands in its jobs would, since the
//! buy gate needs an Ally; the player's balance is set to the station's
//! price plus [`SPARE_CREDITS`], to buy the station and then trade at it.
//!
//! What to try:
//!
//! 1. TAB opens the interface; the Station tab's station block names the
//!    price, and Buy station buys it from its civilization;
//! 2. the station's label then reads the owned name, the Player side and
//!    OWNED, and the ring turns the player's colour;
//! 3. the Inventory tab: a Buy from the bought station pays bid, not ask;
//! 4. `M` steps to the Map, and `C` there draws the chart: the station's
//!    mark reads OWNED STATION once the chart records it;
//! 5. undock (the docking key), fly off and dock again: the station admits
//!    the player on any side.
//!
//! ```text
//! nix develop --command cargo run --example station_own_sandbox --features debug
//! ```
//!
//! The diagnostics (`F3`, on at start) label the station with its name, its
//! side and its price or OWNED, ring it in its side's colour and draw a line
//! to each ship within [`NEARBY_RANGE`], coloured by how the station's guns
//! read it: red hostile, green own, grey neutral. The panel keeps the
//! player's balance, and the next key is hinted. Detail (`Shift+F3`) lists
//! the station's civilization, marker, chart entry, its admission answer for
//! the player and for an Enemy-flagged ship, and what a Buy of one unit of
//! its first stock item pays.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk waits for the dock,
//! buys the station from the Station tab, checks it is owned, on the Player
//! side and named as the player's, closes the interface and shoots
//! `station-own-sandbox.png` with `NOVA_CAPTURE=1`. Unset, no walk and no
//! deadline run.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use bevy::prelude::*;
use clap::Parser;
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};
use nova_world::prelude::WorldClock;
use open_world_stage::{advance_station_dock, OpenWorldStartPlugin, StationDock};

#[derive(Parser)]
#[command(name = "station_own_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start docked at a living Neutral station, warm its civilization to Ally by merits, \
             with the credits to buy it, buy it, and check it is the player's",
    long_about = None
)]
struct Cli;

/// The world seed: `system_station_own`'s, whose window around the origin
/// holds a living Neutral station outside the origin's cell.
const SANDBOX_SEED: u32 = 10;
/// The start role the player spawns as: the scavenger's salvage hauler.
const SANDBOX_ROLE: &str = "scavenger";
/// What the player holds above the station's price, to trade at it after.
const SPARE_CREDITS: u32 = 50_000;
/// How far from the station a ship gets a relation line, engine units:
/// 10 km.
const NEARBY_RANGE: f32 = 1_000.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.add_plugins(OpenWorldStartPlugin {
        seed: SANDBOX_SEED,
        role: SANDBOX_ROLE.to_string(),
    });
    app.init_resource::<Sandbox>();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "buy the docked station, then trade with it and chart it",
    ));
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        draw_the_station
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
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

/// The sandbox's setup: the dock, then the station once docked.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    dock: StationDock,
    /// The docked station, once the fixture landed.
    station: Option<Entity>,
}

/// Dock at the station, then set the player's balance to its price plus
/// [`SPARE_CREDITS`], once.
fn drive_the_sandbox(world: &mut World) {
    if world.resource::<Sandbox>().station.is_some() {
        return;
    }
    let mut dock = world.resource::<Sandbox>().dock.clone();
    let docked = advance_station_dock(world, &mut dock, "living Neutral station", |station| {
        !station.derelict && station.allegiance == Some(&Allegiance::Neutral)
    });
    world.resource_mut::<Sandbox>().dock = dock;
    let Some((player, station)) = docked else {
        return;
    };
    warm_to_ally(world, station);
    let price = station_price(world, station);
    let credits = u32::try_from(price)
        .ok()
        .and_then(|price| price.checked_add(SPARE_CREDITS))
        .unwrap_or_else(|| panic!("station own sandbox: the {price} cr price fits a balance"));
    if let Some(mut balance) = world.get_mut::<ShipCredits>(player) {
        balance.0 = credits;
    }
    world.resource_mut::<Sandbox>().station = Some(station);
    info!(
        "station own sandbox: docked; fixture sets the balance to {credits} cr ({price} cr price)"
    );
}

/// Warm the station's civilization to Ally the way a player does: one merit
/// per handed-in job, `MERIT_STEP` merits per step. Nothing when it is Ally.
fn warm_to_ally(world: &mut World, station: Entity) {
    let civ = world
        .get::<TraderMarket>(station)
        .expect("station own sandbox: the chosen station prices goods")
        .civilization;
    let now = world.resource::<WorldClock>().play_seconds;
    let seeded = seeded_stance(&civ);
    let mut stances = world.resource_mut::<CivilizationStances>();
    let mut merits = 0;
    while stances.effective(&civ, seeded, now) != StanceType::Ally {
        record_merit(&mut stances, civ, now);
        merits += 1;
        assert!(
            merits <= 2 * MERIT_STEP,
            "station own sandbox: '{}' never warmed to Ally",
            civ.name()
        );
    }
    info!(
        "station own sandbox: '{}' warmed to Ally after {merits} merit(s)",
        civ.name()
    );
}

/// The station's price: see [`station_price_cr`].
fn station_price(world: &World, station: Entity) -> u64 {
    let recipes: Vec<BuildRecipe> = world
        .get::<Children>(station)
        .map(|children| {
            children
                .iter()
                .filter_map(|child| world.get::<SectionBuildConfig>(child))
                .map(|config| config.0.base.build.clone())
                .collect()
        })
        .unwrap_or_default();
    let empty = ShipInventory::default();
    station_price_cr(
        world.resource::<GameItems>(),
        &recipes,
        world.get::<ShipInventory>(station).unwrap_or(&empty),
        world
            .get::<ShipCredits>(station)
            .map_or(0, |credits| credits.0),
    )
}

/// The colour of a side: the player's green, the Enemy's red, the Neutral
/// side's yellow and no side grey.
fn side_color(side: Option<&Allegiance>) -> Color {
    match side {
        Some(Allegiance::Player) => palette::OK,
        Some(Allegiance::Enemy) => palette::ALERT,
        Some(Allegiance::Neutral) => palette::TARGET,
        None => palette::MUTED,
    }
}

/// A side as the panel reads it: its name, or `none`.
fn side_text(side: Option<&Allegiance>) -> String {
    side.map_or("none".to_string(), |side| format!("{side:?}"))
}

/// The colour of a relation: hostile red, own green, neutral grey.
fn relation_color(relation: Relation) -> Color {
    match relation {
        Relation::Hostile => palette::ALERT,
        Relation::Own => palette::OK,
        Relation::Neutral => palette::MUTED,
    }
}

/// The station ringed by side, a line to each nearby ship by how the
/// station's guns read it, and the panel reading the ownership facts.
#[expect(
    clippy::too_many_arguments,
    reason = "one panel reading the station, the player, the chart and the admission"
)]
fn draw_the_station(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    sandbox: Res<Sandbox>,
    chart: Option<Res<Chart>>,
    items: Option<Res<GameItems>>,
    price_table: Option<Res<GamePriceTable>>,
    admission: DockAdmission,
    q_station: Query<(
        &GlobalTransform,
        &HullRadius,
        &GeneratedBodyId,
        &Name,
        Option<&Allegiance>,
        Option<&RetaliationTarget>,
        Has<OwnedStationMarker>,
        Has<DerelictShipMarker>,
        &ShipCredits,
        &ShipInventory,
        Option<&TraderMarket>,
        &Children,
    )>,
    q_sections: Query<&SectionBuildConfig>,
    q_player: Query<(Entity, &ShipCredits), With<PlayerSpaceshipMarker>>,
    q_ships: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&Allegiance>,
            Option<&RetaliationTarget>,
        ),
        With<SpaceshipRootMarker>,
    >,
) {
    let Some(station) = sandbox.station else {
        panel.line(format!("setup: {:?}", sandbox.dock.phase));
        return;
    };
    let Ok((
        transform,
        radius,
        id,
        name,
        side,
        retaliation,
        owned,
        derelict,
        station_cr,
        stock,
        market,
        children,
    )) = q_station.get(station)
    else {
        panel.line("the station is gone from the window");
        return;
    };
    let centre = transform.translation();
    let station_party = RelationParty {
        entity: station,
        allegiance: side,
        retaliation,
    };

    // The ring faces the player, so it reads as a circle from the chase
    // camera.
    let player = q_player.single().ok();
    let facing = player
        .and_then(|(player, _)| q_ships.get(player).ok())
        .map_or(Vec3::Y, |(_, at, ..)| at.translation() - centre);
    diagnostics::ring(
        &mut gizmos,
        centre,
        facing,
        radius.0 * 1.1,
        side_color(side),
    );
    for (ship, at, ship_side, ship_retaliation) in &q_ships {
        let at = at.translation();
        if ship == station || at.distance(centre) > NEARBY_RANGE {
            continue;
        }
        let relation = ship_relation(
            station_party,
            RelationParty {
                entity: ship,
                allegiance: ship_side,
                retaliation: ship_retaliation,
            },
        );
        gizmos.line(centre, at, relation_color(relation));
    }

    let civilization = if owned {
        "none (the player's)".to_string()
    } else {
        name.as_str()
            .strip_suffix(" station")
            .unwrap_or("-")
            .to_string()
    };
    panel.detail(format!("station: {} ({id})", name.as_str()));
    panel.detail(format!(
        "side: {}   civilization: {civilization}   OwnedStationMarker: {}",
        side_text(side),
        if owned { "yes" } else { "no" }
    ));
    panel.detail("civilization warmed to Ally by merits at the dock, before the buy".to_string());
    let charted = chart
        .as_deref()
        .and_then(|chart| chart.station(id))
        .map_or("not charted".to_string(), |entry| {
            format!("owned {}, derelict {}", entry.owned, entry.derelict)
        });
    panel.detail(format!("chart: {charted}"));

    let enemy_hostile = ship_relation(
        station_party,
        RelationParty {
            entity: Entity::PLACEHOLDER,
            allegiance: Some(&Allegiance::Enemy),
            retaliation: None,
        },
    ) == Relation::Hostile;
    let enemy_flagged =
        station_admission(side.copied(), derelict, false, owned, enemy_hostile).err();
    let for_player = player.and_then(|(player, _)| admission.refusal(player, station));
    panel.detail(format!(
        "admission: the player {}   an Enemy-flagged ship at a free port {}",
        admits(for_player),
        admits(enemy_flagged)
    ));

    let player_cr = player.map_or(0, |(_, credits)| credits.0);
    panel.line(format!("you: {player_cr} cr"));
    if owned {
        panel.hint("M", "the Map, then C for the chart");
    } else {
        panel.hint("TAB", "Station tab > Buy station");
    }
    let Some(items) = items.as_deref() else {
        panel.label_tinted(
            centre,
            format!("{}\n{}", name.as_str(), side_text(side)),
            side_color(side),
        );
        return;
    };
    let recipes: Vec<BuildRecipe> = children
        .iter()
        .filter_map(|child| q_sections.get(child).ok())
        .map(|config| config.0.base.build.clone())
        .collect();
    let price = station_price_cr(items, &recipes, stock, station_cr.0);
    panel.label_tinted(
        centre,
        format!(
            "{}\n{}   {}",
            name.as_str(),
            side_text(side),
            if owned {
                "OWNED".to_string()
            } else {
                format!("{price} cr")
            }
        ),
        side_color(side),
    );
    panel.detail(format!(
        "price: {price} cr ({})   player: {player_cr} cr   station: {} cr",
        if owned { "owned" } else { "for sale" },
        station_cr.0
    ));
    let sample = stock
        .stacks()
        .next()
        .map_or("the hold is empty".to_string(), |(item, _)| {
            let design = items.design(item);
            let quote = match (market, price_table.as_deref()) {
                // `price` names the station's sale price here.
                (Some(market), Some(table)) => {
                    nova_protocol::prelude::price(&table.0, design, market, stock.count(item))
                }
                (Some(_), None) => return "the price table is not loaded".to_string(),
                (None, _) => PriceQuote::reference(design),
            };
            let prices = if owned {
                TradePriceType::Owned(quote.price)
            } else {
                TradePriceType::Market(quote.price)
            };
            let pays = match prices {
                TradePriceType::Owned(price) => price.bid_cr,
                TradePriceType::Market(price) => price.ask_cr,
            };
            format!(
                "{prices:?}: a Buy of 1 {item} pays {pays} cr (ask {}, bid {})",
                quote.price.ask_cr, quote.price.bid_cr
            )
        });
    panel.detail(format!("trade prices: {sample}"));
}

/// An admission answer as the panel reads it.
fn admits(refusal: Option<DockRefusalType>) -> String {
    refusal.map_or("admitted".to_string(), |refusal| {
        format!("refused ({refusal:?})")
    })
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS},
        nova_interface::{pane::InterfacePaneType, station::STATION_BUY_BUTTON},
    };

    use super::*;

    /// Seconds the world gets to start, stream and dock on a software GPU.
    const SETUP_SECS: f32 = 600.0;
    /// Seconds one UI beat gets.
    const BEAT_SECS: f32 = 240.0;
    /// Frames the scene gets to settle before a scroll or the shot.
    const SETTLE_FRAMES: u32 = 10;
    /// The Station pane's scroll panel.
    const STATION_PANEL: &str = "StationPanel";
    /// The frame the walk shoots.
    const SHOT: &str = "station-own-sandbox.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn docked(world: &World) -> Option<Entity> {
        world.get_resource::<Sandbox>()?.station
    }

    fn pause_is(state: PauseStates) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == state)
        })
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

    /// Whether the named node lies inside the Station panel's box, where a
    /// click lands on it.
    fn in_panel(world: &World, name: &str) -> bool {
        match (
            ui_node_rect(world, name),
            ui_node_rect(world, STATION_PANEL),
        ) {
            (Some(node), Some(panel)) => node.min.y >= panel.min.y && node.max.y <= panel.max.y,
            _ => false,
        }
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("station own sandbox: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station own sandbox: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(BEAT_SECS)
            .add()
    }

    /// The station is the player's: owned, on the Player side and named as
    /// the player's.
    fn assert_the_players(world: &mut World) {
        let station = docked(world).expect("station own sandbox: docked");
        assert!(
            world.get::<OwnedStationMarker>(station).is_some(),
            "station own sandbox: owned"
        );
        assert_eq!(
            world.get::<Allegiance>(station),
            Some(&Allegiance::Player),
            "station own sandbox: on the player's side"
        );
        assert_eq!(
            world.get::<Name>(station).map(Name::as_str),
            Some(OWNED_STATION_NAME),
            "station own sandbox: named as the player's"
        );
        info!("station own sandbox: PASS the bought station is the player's");
    }

    /// The smoke walk: dock, buy the station from the Station tab, check it
    /// is the player's, close the interface and shoot the overlay.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station own sandbox: dock at the station")
            .until(when(|world| docked(world).is_some()))
            .diagnose(|world| format!("{:?}", world.get_resource::<Sandbox>()))
            .deadline(SETUP_SECS)
            .add()
            .step("station own sandbox: settle the dock")
            .until(frames(SETTLE_FRAMES))
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            pause_is(PauseStates::Interface),
        )
        .click_named(
            "station own sandbox: open the Station tab",
            "InterfaceTabStation",
            the_interface_shows(InterfacePaneType::Station),
            BEAT_SECS,
        )
        .step("station own sandbox: settle the lists")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station own sandbox: scroll the buy into view")
        .each(|world, _, frame| {
            keep_hovering_named(STATION_PANEL)(world);
            if frame % 2 == 1 && !in_panel(world, STATION_BUY_BUTTON) {
                scroll_lines(-1.0)(world);
            }
        })
        .until(when(|world| in_panel(world, STATION_BUY_BUTTON)))
        .deadline(BEAT_SECS)
        .add()
        .click_named(
            "station own sandbox: buy the station",
            STATION_BUY_BUTTON,
            when(|world| {
                docked(world)
                    .is_some_and(|station| world.get::<OwnedStationMarker>(station).is_some())
            }),
            BEAT_SECS,
        )
        .step("station own sandbox: check the station is the player's")
        .on_enter(assert_the_players)
        .add();
        tap(
            script,
            "the interface key again",
            "interface_toggle",
            pause_is(PauseStates::Unpaused),
        )
        .step("station own sandbox: settle")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station own sandbox: shoot the overlay")
        .on_enter(|world: &mut World| shoot(world, SHOT))
        .until(shot_written(SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
    }
}
