//! shipyard_sandbox: dock at a station that parks its hulls for sale at its
//! ports, with the ship you traded in parked beside them for rebuy.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the
//! scavenger, as the Create World modal does, but without a save folder. When
//! the window has streamed in, it picks the nearest generated station that
//! is living, not on the Enemy side, offers Refit, parks hulls for sale and
//! has escorts for hire,
//! waits for every parked hull to dock, and berths the player at the port
//! left free until the dock takes. Then the fixture lands, once:
//!
//! - the player holds [`PLAYER_CREDITS`] and [`HELD_COUNT`] more
//!   [`HELD_ITEM`] in its hold;
//! - the first hull for sale is bought through [`HullBuyCommand`], the
//!   message the Station tab's Buy button writes;
//! - the bought hull redocks with the station, and the old ship parks at the
//!   port the player held, listed for rebuy with the hold it had.
//!
//! From there it is yours. Nothing ends the run.
//!
//! What to try:
//!
//! 1. open the interface (`TAB`), Station tab: **Ships for sale** shows a
//!    card per parked hull and the old ship as `(rebuy)`, each with the
//!    hull's own model turning, one line (name, length, price), what the
//!    buy costs in plain words ("Your current ship is taken in for ...; you
//!    pay ...") and Buy;
//! 2. Inventory tab: the old ship's hold lists as storage while you are
//!    docked;
//! 3. buy another hull, or rebuy the old ship: you take its helm and the
//!    hull you flew parks at its port;
//! 4. Station tab, **Escorts for hire**: a card per escort, the same way;
//!    each escort coasts beside the station, undocked and Neutral; Hire one
//!    and it joins your fleet and follows you;
//! 5. Info on any card: the hull in the Ship view, read-only; right-drag
//!    turns it, the wheel zooms, a click selects a section; Close goes back
//!    to the Station tab;
//! 6. undock (`D`) and fly clear: the parked hulls stay docked and inert.
//!
//! ```text
//! nix develop --command cargo run --example shipyard_sandbox --features debug
//! ```
//!
//! The diagnostics (`F3`) mark every docking port of the station, coloured
//! and labelled by what holds it: free (green), the player (cyan), a hull for
//! sale (yellow), a hull parked for rebuy (violet), anything else (red). A
//! line runs from each held port to the hull on it, and a ring with its price
//! marks each parked hull. Each escort for hire gets a green ring (red if it
//! docked) and a line from the station, labelled with its price and
//! distance. The panel counts the ports, the hulls for sale and rebuy, the
//! free ports, and the escorts for hire and hired. `Shift+F3` lists each
//! port's holder, each hull's price, default keys and hold, each escort's
//! price, distance, coasting or DOCKED and side, and the hired escorts.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk waits for the fixture,
//! checks the buy note says the trade-in in plain words, the station parks
//! one hull per port but one with the old ship among them and escorts for
//! hire coast undocked, and shoots
//! `shipyard-sandbox.png` and the Station tab's cards as
//! `shipyard-sandbox-cards.png`, then again as
//! `shipyard-sandbox-cards-turned.png` once the hulls have turned. It then
//! opens Info on the first hull for sale, checks no refit control shows,
//! selects another section and closes, shooting `shipyard-sandbox-info.png`,
//! `shipyard-sandbox-info-section.png` and
//! `shipyard-sandbox-info-closed.png`, and checks the Ship tab shows the
//! player ship with its Catalogue again (`shipyard-sandbox-ship-after-info.png`),
//! all with `NOVA_CAPTURE=1`.
//! Unset, no walk and no deadline run.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use std::collections::HashMap;

use avian3d::prelude::ComputedCenterOfMass;
use bevy::prelude::*;
use clap::Parser;
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_input::prelude::InputSource;
use nova_protocol::{
    nova_debug::diagnostics,
    nova_interface::{
        pane::InterfacePaneType,
        ship::{ShipRuntime, ShipViewSubject},
        station::{HullBuyCommand, StationRuntime, HULL_INFO_CLOSE},
    },
    prelude::*,
};
use open_world_stage::{advance_station_dock, the_player, OpenWorldStartPlugin, StationDock};

#[derive(Parser)]
#[command(name = "shipyard_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Dock at a generated station that parks its hulls for sale at its ports, with your \
             traded-in ship parked for rebuy",
    long_about = None
)]
struct Cli;

/// The world seed: `system_station_buy`'s, whose window holds a living
/// Neutral station that parks hulls for sale.
const SANDBOX_SEED: u32 = 10;
/// The start role the player flies.
const SANDBOX_ROLE: &str = "scavenger";
/// The player's balance once docked: past any hull's price.
const PLAYER_CREDITS: u32 = 1_000_000;
/// The cargo the fixture adds to the hold, so the old ship parks with some.
const HELD_ITEM: &str = "SectionPanel";
const HELD_COUNT: u32 = 2;
/// The radius of a port's marker, world units.
const PORT_MARK: f32 = 0.6;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.add_plugins(OpenWorldStartPlugin {
        seed: SANDBOX_SEED,
        role: SANDBOX_ROLE.into(),
    });
    app.init_resource::<Sandbox>();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "A shipyard station: hulls for sale dock at its ports, escorts for hire coast beside it.",
    ));
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        draw_the_shipyard
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

/// Where the sandbox's setup stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// Streaming in and docking at the station.
    #[default]
    Docking,
    /// Docked; the buy is asked for.
    Buying,
    /// The bought hull redocks and the old ship parks.
    Redocking,
    /// The player is docked in the bought hull. The run is the player's.
    Ready,
}

/// The sandbox's setup.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
    dock: StationDock,
    /// The connection the player held before the buy.
    released: Option<Entity>,
    /// The Station pane's note after the buy: what the buy cost in plain
    /// words.
    buy_note: Option<String>,
}

/// The chosen station, once it stands.
fn chosen_station(world: &mut World) -> Option<Entity> {
    let chosen = world.resource::<Sandbox>().dock.station.clone()?;
    open_world_stage::body_by_id(world, &chosen.id)
}

/// The hulls `station` lists of terms `hire` or not, in park order: the
/// escorts for hire coasting beside it, or the hulls docked for sale or
/// rebuy.
fn listed_hulls(world: &World, station: Entity, hire: bool) -> Vec<Entity> {
    world
        .get::<ParkedHulls>(station)
        .map_or_else(Vec::new, |parked| parked.iter().collect())
        .into_iter()
        .filter(|hull| {
            matches!(
                world.get::<ParkedHullType>(*hull),
                Some(ParkedHullType::Hire { .. })
            ) == hire
        })
        .collect()
}

/// The hulls docked at `station` for sale or rebuy, in park order.
fn parked_hulls(world: &World, station: Entity) -> Vec<Entity> {
    listed_hulls(world, station, false)
}

/// Every hull parked at `station` is docked.
fn all_parked_docked(world: &World, station: Entity) -> bool {
    parked_hulls(world, station)
        .iter()
        .all(|hull| world.get::<DockedShip>(*hull).is_some())
}

/// Advance the setup one phase at a time.
fn drive_the_sandbox(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    match phase {
        SandboxPhaseType::Docking => {
            let mut dock = std::mem::take(&mut world.resource_mut::<Sandbox>().dock);
            let docked = advance_station_dock(
                world,
                &mut dock,
                "living, non-Enemy station with Refit that sells hulls and hires escorts",
                |station| {
                    station.offers_service(StationServiceType::Refit)
                        && station.not_enemy()
                        && station.sells_hulls
                        && station.hires_escorts
                },
            );
            world.resource_mut::<Sandbox>().dock = dock;
            if let Some((player, _)) = docked {
                let items = world.resource::<GameItems>().clone();
                if let Some(mut hold) = world.get_mut::<ShipInventory>(player) {
                    hold.add(&items, &HELD_ITEM.into(), HELD_COUNT);
                }
                if let Some(mut credits) = world.get_mut::<ShipCredits>(player) {
                    credits.0 = PLAYER_CREDITS;
                }
                info!(
                    "shipyard sandbox: docked; the player holds {PLAYER_CREDITS} cr and \
                     {HELD_COUNT} more {HELD_ITEM}"
                );
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Buying;
            }
        }
        SandboxPhaseType::Buying => {
            let (Some(player), Some(station)) = (the_player(world), chosen_station(world)) else {
                return;
            };
            let Some(hull) = parked_hulls(world, station)
                .into_iter()
                .find(|hull| world.get::<ParkedHullType>(*hull) == Some(&ParkedHullType::Sale))
            else {
                panic!("shipyard sandbox: the station parks no hull for sale");
            };
            let released = world
                .get::<DockedShip>(player)
                .expect("shipyard sandbox: the player is docked")
                .connection;
            info!(
                "shipyard sandbox: buying '{}'",
                world.get::<Name>(hull).map_or("?", Name::as_str)
            );
            world.write_message(HullBuyCommand { hull });
            let mut sandbox = world.resource_mut::<Sandbox>();
            sandbox.released = Some(released);
            sandbox.phase = SandboxPhaseType::Redocking;
        }
        SandboxPhaseType::Redocking => {
            let (Some(player), Some(station)) = (the_player(world), chosen_station(world)) else {
                return;
            };
            if world.resource::<Sandbox>().buy_note.is_none() {
                let note = world
                    .resource::<StationRuntime>()
                    .note()
                    .map(str::to_string);
                if let Some(note) = note {
                    info!("shipyard sandbox: the note line reads '{note}'");
                    world.resource_mut::<Sandbox>().buy_note = Some(note);
                }
            }
            let released = world.resource::<Sandbox>().released;
            let redocked = world
                .get::<DockedShip>(player)
                .is_some_and(|docked| Some(docked.connection) != released);
            let parked_old = parked_hulls(world, station)
                .iter()
                .any(|hull| world.get::<OwnedShipMarker>(*hull).is_some());
            if redocked && parked_old && all_parked_docked(world, station) {
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Ready;
                info!("shipyard sandbox: the bought hull is docked and the old ship parked; the run is yours");
            }
        }
        SandboxPhaseType::Ready => {}
    }
}

/// A ship's live anchor: its transform about its centre of mass.
fn anchor(transform: &GlobalTransform, com: Option<&ComputedCenterOfMass>) -> Vec3 {
    let transform = transform.compute_transform();
    live_structure_anchor(&transform, com)
}

/// The default keys of a hull's design, as the buy binds them: each weapon
/// and emitter by section id.
fn default_keys(
    design: &SpaceshipDesign,
    designs: &GameShipDesigns,
    sections: &GameSections,
) -> String {
    let (resolved, errors) = resolve_ship_design(&design.0, designs, sections);
    if !errors.is_empty() {
        return format!("unresolved: {errors:?}");
    }
    let keys: Vec<String> = start_role_bindings(&resolved)
        .into_iter()
        .map(|(section, sources)| {
            let first = sources
                .first()
                .map_or_else(|| "none".to_string(), InputSource::label);
            format!("{section} {first}")
        })
        .collect();
    if keys.is_empty() {
        "no weapons".to_string()
    } else {
        keys.join(", ")
    }
}

/// What holds a station port, for its colour and label.
enum PortHolderType {
    Free,
    Player,
    Sale(String),
    Rebuy(String),
    Other(String),
}

/// Every port of the chosen station by what holds it, the line from each
/// held port to its hull, a ring where each parked hull sits, and the panel
/// rows that count them.
#[expect(
    clippy::too_many_arguments,
    reason = "one overlay reading the station, its ports, its connections, its parked hulls and \
              the catalogs that price and bind them"
)]
#[expect(
    clippy::type_complexity,
    reason = "the parked hull's pose, design and hold"
)]
fn draw_the_shipyard(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    mut keys: Local<HashMap<Entity, String>>,
    sandbox: Res<Sandbox>,
    items: Res<GameItems>,
    designs: Res<GameShipDesigns>,
    sections: Res<GameSections>,
    q_station: Query<(Entity, &GeneratedBodyId, &Children, Option<&ParkedHulls>)>,
    q_ports: Query<(&GlobalTransform, &EntityId, Option<&DockedPort>), With<DockingSectionMarker>>,
    q_connections: Query<&DockingConnection>,
    q_ships: Query<(
        &Name,
        &GlobalTransform,
        Option<&ComputedCenterOfMass>,
        Option<&HullRadius>,
        Option<&ParkedHullType>,
        Option<&ShipDesignSections>,
        Option<&ShipInventory>,
        Option<&SpaceshipDesign>,
        Has<PlayerSpaceshipMarker>,
        Option<&DockedShip>,
    )>,
    q_sides: Query<Option<&Allegiance>>,
    q_fleet: Query<&Name, With<PlayerFleetMarker>>,
    q_player: Query<Has<DockedShip>, With<PlayerSpaceshipMarker>>,
) {
    if sandbox.phase != SandboxPhaseType::Ready {
        panel.line(format!("setup: {:?}", sandbox.phase));
    }
    let Some(chosen) = sandbox.dock.station.as_ref() else {
        return;
    };
    let Some((station, _, children, parked)) =
        q_station.iter().find(|(_, id, ..)| **id == chosen.id)
    else {
        return;
    };
    let listed: Vec<Entity> = parked.map_or_else(Vec::new, |parked| parked.iter().collect());
    keys.retain(|hull, _| listed.contains(hull));
    let (hires, parked): (Vec<Entity>, Vec<Entity>) = listed.into_iter().partition(|hull| {
        q_ships.get(*hull).is_ok_and(|(.., terms, _, _, _, _, _)| {
            matches!(terms, Some(ParkedHullType::Hire { .. }))
        })
    });

    let mut ports = 0usize;
    let mut held = 0;
    for port in children.iter() {
        let Ok((at, id, docked)) = q_ports.get(port) else {
            continue;
        };
        ports += 1;
        let at = at.translation();
        let holder = docked
            .and_then(|docked| q_connections.get(docked.0).ok())
            .map(|connection| {
                let ship = if connection.first_ship == station {
                    connection.second_ship
                } else {
                    connection.first_ship
                };
                (ship, connection)
            });
        let state = match holder {
            None => PortHolderType::Free,
            Some((ship, _)) => {
                held += 1;
                match q_ships.get(ship) {
                    Ok((name, ship_at, com, .., terms, _, _, _, player, _)) => {
                        let name = name.to_string();
                        gizmos.line(at, anchor(ship_at, com), palette::MUTED);
                        match (player, terms) {
                            (true, _) => PortHolderType::Player,
                            (false, Some(ParkedHullType::Sale)) => PortHolderType::Sale(name),
                            (false, Some(ParkedHullType::Rebuy { .. })) => {
                                PortHolderType::Rebuy(name)
                            }
                            (false, Some(ParkedHullType::Hire { .. }) | None) => {
                                PortHolderType::Other(name)
                            }
                        }
                    }
                    Err(_) => PortHolderType::Other(format!("{ship:?}")),
                }
            }
        };
        let (color, label) = match &state {
            PortHolderType::Free => (palette::OK, "free".to_string()),
            PortHolderType::Player => (palette::VELOCITY, "you".to_string()),
            PortHolderType::Sale(_) => (palette::TARGET, "sale".to_string()),
            PortHolderType::Rebuy(_) => (palette::ZONE, "rebuy".to_string()),
            PortHolderType::Other(name) => (palette::ALERT, name.clone()),
        };
        diagnostics::sphere(&mut gizmos, at, PORT_MARK, color);
        panel.label_tinted(at, label, color);
        let holder = match &state {
            PortHolderType::Free => "free".to_string(),
            PortHolderType::Player => "you".to_string(),
            PortHolderType::Sale(name) => format!("for sale, {name}"),
            PortHolderType::Rebuy(name) => format!("rebuy, {name}"),
            PortHolderType::Other(name) => name.clone(),
        };
        panel.detail(format!("port {}: {holder}", id.0));
    }

    let sales = parked
        .iter()
        .filter(|hull| {
            q_ships
                .get(**hull)
                .is_ok_and(|(.., terms, _, _, _, _, _)| terms == Some(&ParkedHullType::Sale))
        })
        .count();
    panel.line(format!(
        "ports {ports}: {sales} for sale, {} rebuy, {} free",
        parked.len() - sales,
        ports.saturating_sub(held),
    ));
    panel.detail(format!(
        "parked {} of ports less one ({}), connections held {held}",
        parked.len(),
        ports.saturating_sub(1),
    ));

    for hull in &parked {
        let Ok((name, at, com, radius, terms, design, hold, source, _, docked)) =
            q_ships.get(*hull)
        else {
            continue;
        };
        let centre = anchor(at, com);
        let color = match terms {
            Some(ParkedHullType::Rebuy { .. }) => palette::ZONE,
            _ => palette::TARGET,
        };
        let color = if docked.is_some() {
            color
        } else {
            palette::ALERT
        };
        let up = at.rotation() * Vec3::Y;
        diagnostics::ring(&mut gizmos, centre, up, radius.map_or(2.0, |r| r.0), color);
        let price = match (terms, design) {
            (Some(ParkedHullType::Rebuy { price_cr }), _) => u64::from(*price_cr),
            (_, Some(design)) => hull_price_cr(
                &items,
                design.0.iter().map(|section| &section.config.base.build),
            ),
            (_, None) => 0,
        };
        let bindings = match source {
            Some(source) => keys
                .entry(*hull)
                .or_insert_with(|| default_keys(source, &designs, &sections))
                .clone(),
            None => "no design".to_string(),
        };
        let docked = if docked.is_some() { "" } else { ", NOT DOCKED" };
        match terms {
            Some(ParkedHullType::Rebuy { price_cr }) => {
                let cargo: Vec<String> = hold
                    .map(|hold| {
                        hold.stacks()
                            .map(|(item, count)| format!("{count} {item}"))
                            .collect()
                    })
                    .unwrap_or_default();
                panel.detail(format!(
                    "rebuy {name}: {price_cr} cr, hold [{}]{docked}",
                    cargo.join(", ")
                ));
                panel.detail(format!("    keys on rebuy: {bindings}"));
            }
            _ => {
                panel.detail(format!("for sale {name}: {price} cr{docked}"));
                panel.detail(format!("    keys: {bindings}"));
            }
        }
        panel.label_tinted(centre, format!("{price} cr"), color);
    }

    let Ok((_, station_at, station_com, ..)) = q_ships.get(station) else {
        return;
    };
    let station_centre = anchor(station_at, station_com);
    for hull in &hires {
        let Ok((name, at, com, radius, _, design, _, _, _, docked)) = q_ships.get(*hull) else {
            continue;
        };
        let centre = anchor(at, com);
        let state = if docked.is_some() {
            "DOCKED"
        } else {
            "coasting"
        };
        let side = match q_sides.get(*hull) {
            Ok(Some(side)) => format!("{side:?}"),
            _ => "no side".to_string(),
        };
        let price = design.map_or(0, |design| escort_price_cr(&items, &design.0));
        let away = Meters3::from_engine(centre)
            .distance(Meters3::from_engine(station_centre))
            .get();
        gizmos.line(station_centre, centre, palette::MUTED);
        let up = at.rotation() * Vec3::Y;
        let color = if docked.is_some() {
            palette::ALERT
        } else {
            palette::OK
        };
        diagnostics::ring(&mut gizmos, centre, up, radius.map_or(2.0, |r| r.0), color);
        panel.detail(format!(
            "hire {name}: {price} cr, {away:.0} m, {state}, {side}"
        ));
        panel.label_tinted(centre, format!("hire {price} cr, {away:.0} m"), color);
    }
    let fleet: Vec<&str> = q_fleet.iter().map(Name::as_str).collect();
    panel.line(format!(
        "escorts: {} for hire, {} hired",
        hires.len(),
        fleet.len()
    ));
    panel.detail(format!("hired: {fleet:?}"));
    if q_player.iter().any(|docked| docked) {
        panel.hint("TAB", "buy, rebuy or hire");
        panel.hint("D", "undock");
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin};

    use super::*;

    /// Seconds the world gets to start, stream, dock and buy on a software
    /// GPU.
    const SETUP_SECS: f32 = 900.0;
    /// Seconds a shot gets to land.
    const SHOT_SECS: f32 = 120.0;
    /// Frames the view gets before the shot.
    const SETTLE_FRAMES: u32 = 20;
    /// The PNG of the station with its parked hulls and the overlay.
    const SHOT: &str = "shipyard-sandbox.png";
    /// The PNGs of the Station tab's hull and escort cards, the second
    /// [`TURN_FRAMES`] after the first, so the hulls have turned.
    const CARDS_SHOT: &str = "shipyard-sandbox-cards.png";
    const CARDS_TURNED_SHOT: &str = "shipyard-sandbox-cards-turned.png";
    /// Frames between the two card shots.
    const TURN_FRAMES: u32 = 30;
    /// The PNGs of Info on the first hull for sale: as it opens, with another
    /// section selected, and back on the Station tab after Close.
    const INFO_SHOT: &str = "shipyard-sandbox-info.png";
    const INFO_SECTION_SHOT: &str = "shipyard-sandbox-info-section.png";
    const INFO_CLOSED_SHOT: &str = "shipyard-sandbox-info-closed.png";
    /// The PNG of the Ship tab after Info: the player ship, editable again.
    const SHIP_AFTER_INFO_SHOT: &str = "shipyard-sandbox-ship-after-info.png";
    /// The only buttons Info may show: the pane tabs, the view's Fit and
    /// Reset, and Close. Anything else is an edit.
    const INFO_BUTTONS: [&str; 7] = [
        "Map",
        "Ship",
        "Inventory",
        "Station",
        "Fit",
        "Reset",
        "Close",
    ];

    /// Click the button named `name`, as a pointer click would.
    fn click(world: &mut World, name: &str) {
        let button = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("shipyard sandbox: no button named '{name}'"));
        world.trigger(bevy::ui_widgets::Activate { entity: button });
    }

    /// The hull the first sale card's Info shows.
    fn info_hull(world: &mut World) -> Entity {
        let station = chosen_station(world).expect("shipyard sandbox: the station stands");
        parked_hulls(world, station)
            .into_iter()
            .find(|hull| world.get::<ParkedHullType>(*hull) == Some(&ParkedHullType::Sale))
            .expect("shipyard sandbox: a hull is for sale")
    }

    /// The labels of the buttons on screen.
    fn shown_buttons(world: &mut World) -> Vec<String> {
        let buttons: Vec<Entity> = world
            .query_filtered::<(Entity, &ComputedNode, &InheritedVisibility), With<bevy::ui_widgets::Button>>()
            .iter(world)
            .filter(|(_, node, visible)| node.size().min_element() > 0.0 && visible.get())
            .map(|(entity, ..)| entity)
            .collect();
        let mut labels: Vec<String> = world
            .query::<(&Text, &ChildOf)>()
            .iter(world)
            .filter(|(_, parent)| buttons.contains(&parent.parent()))
            .map(|(text, _)| text.0.clone())
            .collect();
        labels.sort();
        labels
    }

    /// Info shows the first hull for sale read-only: the Ship pane draws it,
    /// a section of it is selected, and no button but the view's and Close
    /// is on screen.
    fn assert_info_is_read_only(world: &mut World) {
        let hull = info_hull(world);
        assert_eq!(
            *world.resource::<InterfacePaneType>(),
            InterfacePaneType::Ship,
            "shipyard sandbox: Info opens the Ship pane"
        );
        assert_eq!(
            world.resource::<ShipViewSubject>().0,
            Some(hull),
            "shipyard sandbox: Info shows the hull of its card"
        );
        let selected = world
            .resource::<ShipRuntime>()
            .selected()
            .expect("shipyard sandbox: Info selects a section");
        assert_eq!(
            world.get::<ChildOf>(selected).map(ChildOf::parent),
            Some(hull),
            "shipyard sandbox: the selection is a section of the shown hull"
        );
        assert!(
            world.resource::<ShipRuntime>().delete_verdict().is_none(),
            "shipyard sandbox: Info offers no Delete"
        );
        assert!(
            world.resource::<HeldPart>().0.is_none(),
            "shipyard sandbox: Info holds no part"
        );
        let buttons = shown_buttons(world);
        info!("shipyard sandbox: Info shows the buttons {buttons:?}");
        assert!(
            buttons.contains(&"Close".to_string()),
            "shipyard sandbox: Info shows Close, got {buttons:?}"
        );
        let edits: Vec<&String> = buttons
            .iter()
            .filter(|label| !INFO_BUTTONS.contains(&label.as_str()))
            .collect();
        assert!(
            edits.is_empty(),
            "shipyard sandbox: Info shows no refit control, got {edits:?}"
        );
    }

    /// Click the last section badge of the shown hull: the selection moves to
    /// it.
    fn select_another_section(world: &mut World) {
        let before = world.resource::<ShipRuntime>().selected();
        let viewport = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, name)| name.as_str() == "ShipViewport")
            .map(|(entity, _)| entity)
            .expect("shipyard sandbox: the Ship view is on screen");
        let badge = world
            .get::<Children>(viewport)
            .expect("shipyard sandbox: the Ship view has section badges")
            .iter()
            .filter(|child| world.get::<bevy::ui_widgets::Button>(*child).is_some())
            .last()
            .expect("shipyard sandbox: a section badge");
        world.trigger(bevy::ui_widgets::Activate { entity: badge });
        let after = world.resource::<ShipRuntime>().selected();
        assert_ne!(
            after, before,
            "shipyard sandbox: a badge click selects its section"
        );
        info!("shipyard sandbox: Info selected {after:?}, was {before:?}");
    }

    /// The Ship tab after Info shows the player ship with its refit
    /// controls.
    fn assert_ship_tab_is_the_players(world: &mut World) {
        let player = the_player(world).expect("shipyard sandbox: one player ship");
        let selected = world
            .resource::<ShipRuntime>()
            .selected()
            .expect("shipyard sandbox: the Ship tab selects a section");
        assert_eq!(
            world.get::<ChildOf>(selected).map(ChildOf::parent),
            Some(player),
            "shipyard sandbox: the Ship tab shows the player ship after Info"
        );
        let buttons = shown_buttons(world);
        info!("shipyard sandbox: the Ship tab shows the buttons {buttons:?}");
        assert!(
            buttons.contains(&"Catalogue".to_string()),
            "shipyard sandbox: the Ship tab offers the Catalogue again, got {buttons:?}"
        );
    }

    /// Close went back to the Station tab and the Ship view shows the player
    /// ship again.
    fn assert_info_closed(world: &mut World) {
        assert_eq!(
            *world.resource::<InterfacePaneType>(),
            InterfacePaneType::Station,
            "shipyard sandbox: Close goes back to the Station tab"
        );
        assert_eq!(
            world.resource::<ShipViewSubject>().0,
            None,
            "shipyard sandbox: Close ends Info"
        );
    }

    fn ready(world: &World) -> bool {
        world
            .get_resource::<Sandbox>()
            .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
    }

    /// The station parks one hull per port but one, every one docked, the
    /// old ship among them for rebuy with its hold, and escorts for hire
    /// coast beside it, none docked.
    fn assert_the_shipyard(world: &mut World) {
        let player = the_player(world).expect("shipyard sandbox: one player ship");
        let station = chosen_station(world).expect("shipyard sandbox: the station stands");
        let note = world.resource::<Sandbox>().buy_note.clone();
        assert!(
            note.as_deref()
                .is_some_and(|note| note.starts_with("bought ")
                    && note.contains("your old ship was taken in for")),
            "shipyard sandbox: the buy note says the trade-in in plain words, got {note:?}"
        );
        assert!(
            world.get::<DockedShip>(player).is_some(),
            "shipyard sandbox: the player is docked"
        );
        let ports = world
            .get::<Children>(station)
            .expect("shipyard sandbox: the station has sections")
            .iter()
            .filter(|section| world.get::<DockingSectionMarker>(*section).is_some())
            .count();
        let parked = parked_hulls(world, station);
        assert_eq!(
            parked.len(),
            ports - 1,
            "shipyard sandbox: one parked hull per port but one"
        );
        assert!(
            all_parked_docked(world, station),
            "shipyard sandbox: every parked hull is docked"
        );
        let old: Vec<Entity> = parked
            .iter()
            .copied()
            .filter(|hull| world.get::<OwnedShipMarker>(*hull).is_some())
            .collect();
        assert_eq!(old.len(), 1, "shipyard sandbox: one old ship parked");
        let hold = world
            .get::<ShipInventory>(old[0])
            .expect("shipyard sandbox: the old ship has a hold");
        assert!(
            hold.stacks()
                .any(|(item, count)| item.as_str() == HELD_ITEM && count >= HELD_COUNT),
            "shipyard sandbox: the old ship keeps the hold"
        );
        let hires = listed_hulls(world, station, true);
        assert!(
            !hires.is_empty(),
            "shipyard sandbox: escorts for hire coast"
        );
        assert!(
            hires
                .iter()
                .all(|hull| world.get::<DockedShip>(*hull).is_none()),
            "shipyard sandbox: no escort for hire is docked"
        );
        info!(
            "shipyard sandbox: PASS {ports} ports, {} parked, the old ship among them, {} \
             escorts for hire coasting",
            parked.len(),
            hires.len()
        );
    }

    /// The smoke walk: the fixture lands, and a frame of the station with the
    /// diagnostics up is shot.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("shipyard sandbox: dock, buy a parked hull and redock")
            .until(Arc::new(ready))
            .diagnose(|world| format!("{:?}", world.get_resource::<Sandbox>()))
            .deadline(SETUP_SECS)
            .add()
            .step("shipyard sandbox: check the shipyard")
            .on_enter(assert_the_shipyard)
            .add()
            .step("shipyard sandbox: settle the view")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("shipyard sandbox: shoot the shipyard")
            .on_enter(|world: &mut World| shoot(world, SHOT))
            .until(shot_written(SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("shipyard sandbox: open the Station tab")
            .on_enter(|world: &mut World| {
                *world.resource_mut::<InterfacePaneType>() = InterfacePaneType::Station;
                world
                    .resource_mut::<NextState<PauseStates>>()
                    .set(PauseStates::Interface);
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("shipyard sandbox: shoot the cards")
            .on_enter(|world: &mut World| shoot(world, CARDS_SHOT))
            .until(shot_written(CARDS_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("shipyard sandbox: let the hulls turn")
            .until(frames(TURN_FRAMES))
            .add()
            .step("shipyard sandbox: shoot the turned cards")
            .on_enter(|world: &mut World| shoot(world, CARDS_TURNED_SHOT))
            .until(shot_written(CARDS_TURNED_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("shipyard sandbox: open Info on the first hull for sale")
            .on_enter(|world: &mut World| click(world, "HullInfo-0"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("shipyard sandbox: check Info is read-only")
            .on_enter(assert_info_is_read_only)
            .add()
            .step("shipyard sandbox: shoot Info")
            .on_enter(|world: &mut World| shoot(world, INFO_SHOT))
            .until(shot_written(INFO_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("shipyard sandbox: select another section in Info")
            .on_enter(select_another_section)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("shipyard sandbox: check Info is still read-only")
            .on_enter(assert_info_is_read_only)
            .add()
            .step("shipyard sandbox: shoot the selected section")
            .on_enter(|world: &mut World| shoot(world, INFO_SECTION_SHOT))
            .until(shot_written(INFO_SECTION_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("shipyard sandbox: close Info")
            .on_enter(|world: &mut World| click(world, HULL_INFO_CLOSE))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("shipyard sandbox: check Info closed")
            .on_enter(assert_info_closed)
            .add()
            .step("shipyard sandbox: shoot the Station tab after Info")
            .on_enter(|world: &mut World| shoot(world, INFO_CLOSED_SHOT))
            .until(shot_written(INFO_CLOSED_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("shipyard sandbox: open the Ship tab after Info")
            .on_enter(|world: &mut World| {
                *world.resource_mut::<InterfacePaneType>() = InterfacePaneType::Ship;
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("shipyard sandbox: check the Ship tab is the player's")
            .on_enter(assert_ship_tab_is_the_players)
            .add()
            .step("shipyard sandbox: shoot the Ship tab after Info")
            .on_enter(|world: &mut World| shoot(world, SHIP_AFTER_INFO_SHOT))
            .until(shot_written(SHIP_AFTER_INFO_SHOT))
            .deadline(SHOT_SECS)
            .add()
    }
}
