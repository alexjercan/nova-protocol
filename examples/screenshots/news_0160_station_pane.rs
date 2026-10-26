//! news_0160_station_pane: the v0.16.0 news still of the Station pane
//! docked at the starting station of a seeded Ally civilization, just after
//! a hire (`news-0160-station-pane.png`).
//!
//! The world starts the way the Create World modal starts it, on
//! [`WORLD_SEED`] as `industrial`, with no save folder, and the start docks
//! the tug at its starting station. The walk stays there: on this seed it
//! is a Refit station that sells [`SALES`] hull, hires out [`HIRES`]
//! escorts or more, and belongs to a civilization the seed drew Ally with no
//! record of the player's yet, so its stance is the seed's alone. No other
//! seeded Ally station that hires two escorts stood round the origin on
//! seeds 1 to 40, and no starting station on those seeds parks more than one
//! hull for sale: the tug holds the port it keeps free. The seed is pinned
//! for that station; a seed that gives it up stops the run, naming what it
//! saw.
//!
//! DISCLOSED FIXTURE STATE: once docked, the player's balance is set to
//! [`FIXTURE_CREDITS`], as `system_station_hire` sets it, so the hire and
//! every price on the pane are affordable.
//!
//! The walk opens the interface with the real key, selects the Station tab
//! and clicks the first escort's Hire button. The shot follows while the
//! notice line still reads the hire as done: This station with its Buy,
//! Ships for sale with its card, and Escorts for hire with STBD filled and
//! PORT free. Before the shot it asserts each of those on the world and on
//! the drawn pane, and that all of them stand inside the panel.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG, staged under
//!   `NOVA_CAPTURE_DIR`.
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_station_pane --features debug
//! ```

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

use std::sync::Arc;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{
    nova_debug::harness::{AutopilotPlugin, Predicate},
    nova_interface::{
        pane::{ActionOutcomeType, InterfacePaneType},
        station::prelude::{StationRuntime, STATION_BUY_BUTTON, STATION_PANEL},
    },
    prelude::*,
};
use open_world_stage::{body_by_id, the_player, window_settled};

#[derive(Parser)]
#[command(name = "news_0160_station_pane")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 Station pane at a seeded Ally Refit station, just after a hire",
    long_about = None
)]
struct Cli {
    /// The world seed. Another seed must still start the tug at a seeded
    /// Ally Refit station that sells one hull and hires out two escorts.
    #[arg(long, default_value_t = WORLD_SEED)]
    seed: u32,
}

/// The seed the figure plays.
const WORLD_SEED: u32 = 23;

/// The start role the walk flies.
const ROLE: &str = "industrial";

/// Hulls for sale the station must park: the figure shows one card.
const SALES: usize = 1;

/// Escorts for hire the station must offer: the hire takes the first, and
/// the list only shows while one is left.
const HIRES: usize = 2;

/// Disclosed fixture state: the player's balance once docked.
const FIXTURE_CREDITS: u32 = 1_000_000;

/// The Hire button the walk clicks.
const HIRE_BUTTON: &str = "EscortHire-0";

/// The still this producer writes.
const SHOT_FILE: &str = "news-0160-station-pane.png";

/// Real seconds the world gets to start, stream and dock the tug on a
/// software rasterizer.
const SETUP_SECS: f32 = 600.0;

/// Real seconds a beat gets.
const BEAT_SECS: f32 = 60.0;

/// Frames the pane gets to draw its lists and hull views before the hire.
const PANE_FRAMES: u32 = 10;

/// Real seconds a frame capture gets on a software rasterizer.
const SHOT_SECS: f32 = 120.0;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(open_world_stage::OpenWorldStartPlugin {
        seed: cli.seed,
        role: ROLE.to_string(),
    });
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_systems(
        Update,
        open_world_stage::hold_the_walk_clock.run_if(in_state(GameStates::Playing)),
    );
    app.add_plugins(script());
    app.run()
}

/// The starting station, once the window is in, the start docked the tug
/// there and its hulls are parked.
#[derive(Resource)]
struct Docked(Entity);

/// The escort the walk hires: its hull's name.
#[derive(Resource, Clone, Debug)]
struct Hired {
    name: String,
}

fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(check)
}

/// The one player ship, if exactly one stands.
fn player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The docked starting station.
fn station(world: &World) -> Entity {
    world
        .get_resource::<Docked>()
        .expect("station pane: docked at the starting station")
        .0
}

/// The starting station, once it stands.
fn start_station(world: &mut World) -> Option<Entity> {
    let id = world.get_resource::<OpenWorldSession>()?.start.id.clone();
    body_by_id(world, &id)
}

/// Record the starting station once the window is in, the tug is docked
/// and the station's hulls are parked.
fn record_the_dock(world: &mut World) {
    if world.contains_resource::<Docked>() {
        return;
    }
    let docked = the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some());
    let Some(station) = start_station(world) else {
        return;
    };
    if docked && world.get::<PendingParkedHulls>(station).is_none() && window_settled(world) {
        world.insert_resource(Docked(station));
    }
}

/// Disclosed fixture state: [`FIXTURE_CREDITS`] in the player's balance.
fn set_the_balance(world: &mut World) {
    let player = player(world).expect("station pane: one player ship");
    world
        .get_mut::<ShipCredits>(player)
        .expect("station pane: the player has a balance")
        .0 = FIXTURE_CREDITS;
    info!("station pane: fixture sets the balance to {FIXTURE_CREDITS} cr");
}

/// The parked hulls of `station` on `terms`, in the order the station
/// parks them, which is the order the Station pane lists them in.
fn parked_of(world: &World, station: Entity, sale: bool) -> Vec<Entity> {
    world
        .get::<ParkedHulls>(station)
        .map_or_else(Vec::new, |parked| {
            parked
                .iter()
                .filter(|hull| match world.get::<ParkedHullType>(*hull) {
                    Some(ParkedHullType::Sale) => sale,
                    Some(ParkedHullType::Hire { .. }) => !sale,
                    Some(ParkedHullType::Owned) | None => false,
                })
                .collect()
        })
}

/// Assert the docked station is the figure's subject before the hire, and
/// name the escort the Hire button hires.
fn assert_the_station(world: &mut World) {
    let station = station(world);
    let civ = world
        .get::<TraderMarket>(station)
        .expect("station pane: the station prices goods")
        .civilization;
    assert_eq!(
        seeded_stance(&civ),
        StanceType::Ally,
        "station pane: '{}' is not seeded Ally",
        civ.name()
    );
    assert!(
        world
            .resource::<CivilizationStances>()
            .record(&civ)
            .is_none(),
        "station pane: the player moved '{}' before the shot",
        civ.name()
    );
    let sale = parked_of(world, station, true);
    let hires = parked_of(world, station, false);
    assert_eq!(
        sale.len(),
        SALES,
        "station pane: {} hull(s) for sale, not {SALES}",
        sale.len()
    );
    assert!(
        hires.len() >= HIRES,
        "station pane: {} escort(s) for hire, not {HIRES}",
        hires.len()
    );
    let name = world
        .get::<Name>(hires[0])
        .expect("station pane: an escort for hire is named")
        .to_string();
    info!(
        "station pane: '{}' of seeded Ally '{}', {} hull(s) for sale, {} escort(s) for hire; \
         hiring '{name}'",
        world
            .get::<Name>(station)
            .map_or_else(String::new, ToString::to_string),
        civ.name(),
        sale.len(),
        hires.len()
    );
    world.insert_resource(Hired { name });
}

/// The fleet's escorts, side and name.
fn fleet(world: &World) -> Vec<(&'static str, String)> {
    let Some(player) = player(world) else {
        return Vec::new();
    };
    world
        .try_query_filtered::<(&AIFollow, &Name), With<PlayerFleetMarker>>()
        .map(|mut fleet| {
            fleet
                .iter(world)
                .filter(|(follow, _)| follow.leader == player)
                .map(|(follow, name)| (fleet_side_label(follow.side), name.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// The texts of the fleet slot box whose first text is `label`.
fn slot_texts(world: &World, label: &str) -> Option<Vec<String>> {
    let mut q_texts = world.try_query::<(&Text, &ChildOf)>()?;
    let slot = q_texts
        .iter(world)
        .find(|(text, _)| text.0 == label)
        .map(|(_, child_of)| child_of.parent())?;
    Some(
        world
            .get::<Children>(slot)?
            .iter()
            .filter_map(|child| world.get::<Text>(child).map(|text| text.0.clone()))
            .collect(),
    )
}

/// Every visible UI text laid out with a width and a height: a zero-height
/// line draws nothing.
fn drawn_texts(world: &World) -> Vec<String> {
    world
        .try_query::<(&Text, &ComputedNode, &InheritedVisibility)>()
        .map(|mut texts| {
            texts
                .iter(world)
                .filter(|(_, node, visible)| visible.get() && node.size().min_element() > 0.0)
                .map(|(text, ..)| text.0.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Whether the node `name` lays out inside the Station panel.
fn in_the_panel(world: &World, name: &str) -> bool {
    match (
        ui_node_rect(world, name),
        ui_node_rect(world, STATION_PANEL),
    ) {
        (Some(node), Some(panel)) => node.min.y >= panel.min.y && node.max.y <= panel.max.y,
        _ => false,
    }
}

/// What the frame must show after the hire: the notice reads it as done,
/// STBD names the escort, PORT is free, and This station's Buy, the sale
/// card's Buy and the next escort's Hire stand inside the panel.
fn the_pane_reads(world: &World) -> Result<(), String> {
    let hired = world.get_resource::<Hired>().ok_or("no hire recorded")?;
    let fleet = fleet(world);
    if fleet != [("STBD", hired.name.clone())] {
        return Err(format!("the fleet is {fleet:?}"));
    }
    let notice = format!("hired {} for ", hired.name);
    let shown = world.resource::<StationRuntime>().notice.shown();
    if !shown.is_some_and(|(outcome, text)| {
        outcome == ActionOutcomeType::Done && text.starts_with(&notice)
    }) {
        return Err(format!("the notice holds {shown:?}"));
    }
    if !drawn_texts(world)
        .iter()
        .any(|text| text.starts_with(&notice))
    {
        return Err("the notice line does not read the hire".to_string());
    }
    let stbd = slot_texts(world, "STBD");
    if stbd != Some(vec!["STBD".to_string(), hired.name.clone()]) {
        return Err(format!("the STBD slot reads {stbd:?}"));
    }
    let port = slot_texts(world, "PORT");
    if port != Some(vec!["PORT".to_string(), "--".to_string()]) {
        return Err(format!("the PORT slot reads {port:?}"));
    }
    for name in [STATION_BUY_BUTTON, "HullBuy-0", HIRE_BUTTON] {
        if !in_the_panel(world, name) {
            return Err(format!("'{name}' is not inside the Station panel"));
        }
    }
    Ok(())
}

/// The interface is open on `pane`.
fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
    when(move |world| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world.get_resource::<InterfacePaneType>() == Some(&pane)
    })
}

/// The walk: wait at the starting station, open the Station tab, hire and
/// shoot the pane.
fn script() -> AutopilotPlugin<GameStates> {
    AutopilotPlugin::<GameStates>::new()
        .step("station pane: the window streams in and the start docks")
        // `window_settled` reads through a query state and needs the world
        // mutably, which a predicate never has, so each frame records it.
        .each(|world: &mut World, _, _| record_the_dock(world))
        .until(when(|world| world.contains_resource::<Docked>()))
        .deadline(SETUP_SECS)
        .add()
        .step("station pane: check the station")
        .on_enter(|world: &mut World| {
            hide_status_bar(world);
            set_the_balance(world);
            assert_the_station(world);
        })
        .add()
        .step("station pane: press the interface key")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("station pane: let the interface key up")
        .on_enter(release_action("interface_toggle"))
        .until(when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
        }))
        .deadline(BEAT_SECS)
        .add()
        .click_named(
            "station pane: open the Station tab",
            "InterfaceTabStation",
            and(
                the_interface_shows(InterfacePaneType::Station),
                ui_node_present(HIRE_BUTTON),
            ),
            BEAT_SECS,
        )
        .step("station pane: settle the lists and the hull views")
        .until(frames(PANE_FRAMES))
        .add()
        .click_named(
            "station pane: hire the first escort",
            HIRE_BUTTON,
            when(|world| the_pane_reads(world).is_ok()),
            BEAT_SECS,
        )
        // No settle beat: the notice line holds the hire for four real
        // seconds, and a software frame takes most of one.
        .step("station pane: shoot the pane")
        .on_enter(|world: &mut World| {
            if let Err(why) = the_pane_reads(world) {
                panic!("station pane: {why}");
            }
            shoot(world, SHOT_FILE);
        })
        .until(shot_written(SHOT_FILE))
        .deadline(SHOT_SECS)
        .add()
}
