//! system_station_buy: a station parks its hulls for sale at its docking
//! ports, a buy hands the player one of them at its full price and parks the
//! old ship as the player's own, and a free board hands the old ship back.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the window stream in
//! around the session's starting station, where the start docks the player.
//! The walk undocks and jumps to the origin, the spawn point before the
//! docked start: stations are a civilization apart, so the station it wants
//! never streams in round the start. Once the window stands round the origin
//! it picks the nearest generated station that is living, not on the Enemy
//! side, offers Refit, parks hulls for sale, and stands outside the origin's
//! cell. The player jumps beside it and is put on a berth at its one free
//! port each frame until the dock takes; the walk never flies the approach
//! (`system_station_docking` does).
//!
//! THE HOLD AND BOTH BALANCES ARE DISCLOSED FIXTURE STATE: once docked, the
//! hold is set to two section panels, and the player and the station each
//! hold 1,000,000 cr, so every hull is affordable. The buy and the board go
//! through the Station pane's Buy and Board buttons, the code path a player
//! clicks. Between them the player undocks, jumps away
//! until the station's cell retires, and jumps back until it thaws. Then the
//! player leaves to the main menu, the leave save is read back off disk
//! through [`open_world`], and the world is loaded again in this process
//! through the Load screen.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a station parks its hulls for sale at its ports` | the station parks one hull for sale per docking port but one, each docked with it and inert: no driver, Neutral, not lootable; one port stays free |
//! | 2 | `outcome: a buy hands the player the parked hull and parks the old one` | the player's design is the parked hull's, every section live, its hold empty and docked with the station through a new connection; the old ship is parked as the player's own (`Owned`, `OwnedShipMarker`), docked, with its design and the hold it had; the player paid the full price and the station took it, both as `ShipBuy`; `OnShipBought` fired once; storage did not change and nothing was stored; the bought hull then undocks |
//! | 3 | `outcome: the retired record holds the parked hulls` | the cell is not live and the station's record lists every parked hull and escort for hire by name and listing, the old ship with its sections and its hold |
//! | 4 | `outcome: the thawed station parks the same hulls` | the thawed station lists the same hulls and escorts for hire in the same order, each hull for sale or owned docked with it, only the owned ones the player's, the old ship with its hold |
//! | 5 | `outcome: a board hands the player the old ship back for free` | the Station pane lists the old ship as "your ship" with Board; after the board the player's design and hold are the old ship's, the hull it flew is parked as the player's own, neither balance moved and no buy was reported |
//! | 6 | `outcome: the leave save holds the player's hull and the parked hulls` | the saved player carries the old ship's sections and none the bought hull adds; the saved station lists the same parked hulls and escorts for hire |
//! | 7 | `outcome: a loaded world gives back the player's hull and the parked hulls` | after Load the player's live sections and hold are the old ship's, and the station lists the same parked hulls and escorts for hire |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_station_buy --features debug
//! # look for: `station buy: PASS the loaded world gives back the player's hull and the parked hulls`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the parked hulls as
//! `station-buy-parked.png`, the hull list as `station-buy-offer.png`, the
//! bought hull docked as `station-buy-docked.png` and the old ship's Board
//! card as `station-buy-board.png`.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_buy")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed: dock at a living generated station that parks hulls for \
             sale, buy one, retire and thaw the station, board the old ship, then check both \
             through a save and a Load. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: no player's saved world may be in the way.
    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{}-{}-{}",
            env!("CARGO_CRATE_NAME"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );

    let mut app = nova_protocol::prelude::editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // The fleet's 1920x1080, the size the hull list is shot at.
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        // No frame-time claim: the range holds counts, credits and ids.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::BuyWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::Arc,
    };

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS,
        },
        nova_events::engine::GameEvent,
        nova_interface::pane::InterfacePaneType,
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{
        desired_sectors, FrozenBody, FrozenBodyType, FrozenSector, FrozenSectors, SectorRoot,
        WorldConfig,
    };

    use super::*;

    /// The seed the range plays: `system_station_round_trip`'s seed, whose
    /// window around the origin holds a living Neutral station outside the
    /// origin's cell.
    const STATION_SEED: u32 = 10;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Station Buy World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    /// The start role the walk picks: the scavenger's salvage hauler, a compact
    /// box. The first role's mining tug has a bow boom that can strike a berthed
    /// partner.
    const START_ROLE_BUTTON: &str = "Start Role Button: scavenger";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";

    /// The Station pane's panel, which scrolls under the wheel.
    const STATION_PANEL: &str = "StationPanel";
    /// The end of the card line of a ship the player owns, after its length.
    const OWNED_DETAIL: &str = ", your ship";

    /// The disclosed fixture hold, which stays in the old ship on the buy.
    const HELD_ITEM: &str = "SectionPanel";
    const HELD_COUNT: u32 = 2;
    /// The disclosed fixture balance of the player and of the station.
    const FIXTURE_CREDITS: u32 = 1_000_000;

    /// The PNGs the capture path shoots.
    const PARKED_SHOT: &str = "station-buy-parked.png";
    const OFFER_SHOT: &str = "station-buy-offer.png";
    const DOCKED_SHOT: &str = "station-buy-docked.png";
    const BOARD_SHOT: &str = "station-buy-board.png";

    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Frames the Station pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;

    /// A parked hull as the walk compares it: its name, its listing, its
    /// design's section ids and its hold.
    #[derive(Clone, Debug, PartialEq)]
    struct ParkedView {
        name: String,
        terms: ParkedHullType,
        sections: BTreeSet<String>,
        hold: BTreeMap<String, u32>,
    }

    /// What the walk recorded right before a buy or a board.
    #[derive(Clone, Debug)]
    struct BeforeBuy {
        player_cr: u32,
        station_cr: u32,
        hold: BTreeMap<String, u32>,
        storage: BTreeMap<String, u32>,
        /// The hull bought or boarded, and its price: none for a board.
        hull: Entity,
        price_cr: u32,
        name: String,
        offered: BTreeSet<String>,
        /// The player's design and name before the buy.
        old: BTreeSet<String>,
        flown: String,
        parked: Vec<ParkedView>,
        connection: Entity,
    }

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct BuyWatch {
        /// Every buy, store and balance event, by name, with its payload.
        events: Vec<(String, serde_json::Value)>,
        /// The chosen station's typed id.
        id: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// The cell the retire jump lands in.
        away: Option<SectorCoord>,
        /// Where it stood when chosen, for the park point.
        position: Option<Meters3>,
        /// The station's docking ports.
        ports: usize,
        /// The facts right before the buy.
        buy: Option<BeforeBuy>,
        /// The station's parked hulls as the player left it.
        left: Option<Vec<ParkedView>>,
        /// The entity that wore the station's id before the retire.
        retired: Option<Entity>,
        /// The facts right before the board.
        board: Option<BeforeBuy>,
        /// The station's parked hulls after the board.
        after: Option<Vec<ParkedView>>,
        /// The Buy or Board button the next click aims at.
        button: Option<String>,
        /// The hull list's card lines of the ships the player owns.
        board_rows: Vec<String>,
    }

    impl BuyWatch {
        fn named(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events
                .iter()
                .filter(|(event, _)| event == name)
                .map(|(_, data)| data)
                .collect()
        }

        fn buy(&self) -> &BeforeBuy {
            self.buy
                .as_ref()
                .expect("station buy: the facts before the buy are recorded")
        }

        fn board(&self) -> &BeforeBuy {
            self.board
                .as_ref()
                .expect("station buy: the facts before the board are recorded")
        }
    }

    pub(super) struct BuyWatchPlugin;

    impl Plugin for BuyWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<BuyWatch>();
            app.add_systems(Update, hold_the_walk_clock);
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<BuyWatch>| {
                let name = event.name();
                if !matches!(name, "onshipbought" | "oncreditschanged" | "onstored") {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station buy: {name} {data}");
                watch.events.push((name.to_string(), data));
            });
        }
    }

    /// Hold the virtual clock's clamp at [`WALK_CLOCK_SECS`] every frame: a
    /// scenario load hands `Time<Virtual>` back at its default.
    fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
        let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
        if time.max_delta() != clamp {
            time.set_max_delta(clamp);
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The one player ship, if exactly one stands.
    fn the_player(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        let mut players = players.iter(world);
        let player = players.next()?;
        players.next().is_none().then_some(player)
    }

    fn player(world: &World) -> Entity {
        the_player(world).expect("station buy: exactly one player ship")
    }

    /// The entity wearing the chosen station's id, if one does.
    fn station_root(world: &World) -> Option<Entity> {
        let id = world.resource::<BuyWatch>().id.clone()?;
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| **worn == id)
            .map(|(entity, _)| entity)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station buy: the station stands")
    }

    fn station_id(world: &World) -> GeneratedBodyId {
        world
            .resource::<BuyWatch>()
            .id
            .clone()
            .expect("station buy: the station is chosen")
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .expect("station buy: every ship carries a balance")
            .0
    }

    fn hold_map(inventory: &ShipInventory) -> BTreeMap<String, u32> {
        inventory
            .stacks()
            .map(|(item, count)| (item.to_string(), count))
            .collect()
    }

    fn hold_of(world: &World, root: Entity) -> BTreeMap<String, u32> {
        hold_map(
            world
                .get::<ShipInventory>(root)
                .expect("station buy: every ship carries a hold"),
        )
    }

    fn storage_map(world: &World, station: Entity) -> BTreeMap<String, u32> {
        world
            .get::<StationStorage>(station)
            .expect("station buy: a station root carries StationStorage")
            .stacks()
            .map(|(item, count)| (item.to_string(), count))
            .collect()
    }

    /// The ids of every live section of `root`.
    fn live_ids(world: &World, root: Entity) -> BTreeSet<String> {
        world
            .get::<Children>(root)
            .map_or_else(BTreeSet::new, |children| {
                children
                    .iter()
                    .filter(|child| world.get::<SectionMarker>(*child).is_some())
                    .filter_map(|child| world.get::<EntityId>(child).map(|id| id.0.clone()))
                    .collect()
            })
    }

    /// The ids of every section of `root`'s design.
    fn design_ids(world: &World, root: Entity) -> BTreeSet<String> {
        world
            .get::<ShipDesignSections>(root)
            .map_or_else(BTreeSet::new, |design| {
                design.0.iter().map(|section| section.id.clone()).collect()
            })
    }

    /// Every hull `station` lists, in park order: the docked hulls and the
    /// escorts for hire coasting beside it.
    fn listed_hulls(world: &World, station: Entity) -> Vec<Entity> {
        world
            .get::<ParkedHulls>(station)
            .map_or_else(Vec::new, |parked| parked.iter().collect())
    }

    /// The hulls docked at `station` for sale or owned, in park order: the
    /// Station pane's hull rows.
    fn parked_hulls(world: &World, station: Entity) -> Vec<Entity> {
        listed_hulls(world, station)
            .into_iter()
            .filter(|hull| {
                !matches!(
                    world.get::<ParkedHullType>(*hull),
                    Some(ParkedHullType::Hire { .. })
                )
            })
            .collect()
    }

    /// The names of the escorts for hire coasting beside `station`.
    fn hire_names(world: &World, station: Entity) -> Vec<String> {
        listed_hulls(world, station)
            .into_iter()
            .map(|hull| parked_view(world, hull))
            .filter(|view| matches!(view.terms, ParkedHullType::Hire { .. }))
            .map(|view| view.name)
            .collect()
    }

    fn parked_view(world: &World, hull: Entity) -> ParkedView {
        ParkedView {
            name: world
                .get::<Name>(hull)
                .expect("station buy: a parked hull is named")
                .to_string(),
            terms: *world
                .get::<ParkedHullType>(hull)
                .expect("station buy: a parked hull is listed"),
            sections: design_ids(world, hull),
            hold: hold_of(world, hull),
        }
    }

    /// Every hull `station` lists, escorts for hire included, as the walk
    /// compares them.
    fn parked_views(world: &World, station: Entity) -> Vec<ParkedView> {
        listed_hulls(world, station)
            .into_iter()
            .map(|hull| parked_view(world, hull))
            .collect()
    }

    /// The live docking ports mounted on `root`.
    fn ports_of(world: &mut World, root: Entity) -> Vec<Entity> {
        let ports: Vec<Entity> = world
            .query_filtered::<Entity, (With<DockingSectionMarker>, Without<SectionInactiveMarker>)>(
            )
            .iter(world)
            .collect();
        ports
            .into_iter()
            .filter(|port| mounted_on(world, *port, root))
            .collect()
    }

    fn mounted_on(world: &World, section: Entity, root: Entity) -> bool {
        let mut at = section;
        while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
            if parent == root {
                return true;
            }
            at = parent;
        }
        false
    }

    /// Whether `ship` is docked with `station`.
    fn docked_with(world: &World, ship: Entity, station: Entity) -> bool {
        world.get::<DockedShip>(ship).is_some_and(|docked| {
            world
                .get::<DockingConnection>(docked.connection)
                .is_some_and(|connection| connection.joins(station))
        })
    }

    /// The parked hull whose row the hull list shows first among those with
    /// `terms`, and its row index, which names its Buy button.
    fn row_of(
        world: &World,
        station: Entity,
        terms: fn(ParkedHullType) -> bool,
    ) -> (usize, Entity) {
        parked_hulls(world, station)
            .into_iter()
            .enumerate()
            .find(|(_, hull)| {
                world
                    .get::<ParkedHullType>(*hull)
                    .is_some_and(|listed| terms(*listed))
            })
            .unwrap_or_else(|| panic!("station buy: no parked hull of the wanted listing"))
    }

    fn world_shape(world: &World) -> (Meters, i32) {
        let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
        (config.sector_edge, config.active_radius)
    }

    /// The cell of the session's starting station, where the start docks the
    /// player, once the session and the world config stand.
    fn start_cell(world: &World) -> Option<SectorCoord> {
        let edge = world
            .get_resource::<WorldConfig<NovaLayeredWorld>>()?
            .sector_edge;
        let start = world.get_resource::<OpenWorldSession>()?.start.position;
        Some(SectorCoord::containing(start, edge))
    }

    fn live_cells(world: &World) -> BTreeSet<SectorCoord> {
        world
            .try_query::<&SectorRoot>()
            .map(|mut query| query.iter(world).map(|root| root.0).collect())
            .unwrap_or_default()
    }

    /// Advance once the world is armed around one player ship and the live
    /// set is exactly the window around `centre`.
    fn window_is(
        centre: impl Fn(&World) -> Option<SectorCoord> + Send + Sync + 'static,
    ) -> Arc<Predicate> {
        when(move |world| {
            if !world.contains_resource::<WorldConfig<NovaLayeredWorld>>()
                || the_player(world).is_none()
            {
                return false;
            }
            let Some(centre) = centre(world) else {
                return false;
            };
            let (_, radius) = world_shape(world);
            live_cells(world) == desired_sectors(centre, radius)
        })
    }

    /// The station stands with its hulls parked and every one docked with it.
    fn the_station_stands() -> Arc<Predicate> {
        when(|world| {
            station_root(world).is_some_and(|station| {
                let parked = parked_hulls(world, station);
                world.get::<Position>(station).is_some()
                    && world.get::<PendingParkedHulls>(station).is_none()
                    && !parked.is_empty()
                    && parked.iter().all(|hull| docked_with(world, *hull, station))
            })
        })
    }

    fn player_docked() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
        })
    }

    fn player_free() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
        })
    }

    /// The player flies the design `pick` reads off the record, every section
    /// live.
    fn flies(pick: fn(&BuyWatch) -> &BTreeSet<String>) -> Arc<Predicate> {
        when(move |world| {
            let Some(player) = the_player(world) else {
                return false;
            };
            let watch = world.resource::<BuyWatch>();
            let want = pick(watch);
            design_ids(world, player) == *want && live_ids(world, player) == *want
        })
    }

    /// The swap reported and the player docked through a new connection, the
    /// old hull parked and docked with the station.
    fn swapped_and_redocked(pick: fn(&BuyWatch) -> Option<&BeforeBuy>) -> Arc<Predicate> {
        when(move |world| {
            let watch = world.resource::<BuyWatch>();
            let Some(before) = pick(watch) else {
                return false;
            };
            let (Some(player), Some(station)) = (the_player(world), station_root(world)) else {
                return false;
            };
            // A board reports no buy, so the buy's report stands for both.
            !watch.named("onshipbought").is_empty()
                && live_ids(world, player) == before.offered
                && world
                    .get::<DockedShip>(player)
                    .is_some_and(|docked| docked.connection != before.connection)
                && parked_hulls(world, station)
                    .iter()
                    .all(|hull| docked_with(world, *hull, station))
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

    fn unpaused() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
        })
    }

    fn the_button(world: &World) -> String {
        world
            .resource::<BuyWatch>()
            .button
            .clone()
            .expect("station buy: a Buy or Board button is aimed at")
    }

    /// Whether the aimed Buy or Board button lies inside the Station panel's box,
    /// where a click lands on it.
    fn buy_button_in_view(world: &World) -> bool {
        match (
            ui_node_rect(world, &the_button(world)),
            ui_node_rect(world, STATION_PANEL),
        ) {
            (Some(button), Some(panel)) => {
                button.min.y >= panel.min.y && button.max.y <= panel.max.y
            }
            _ => false,
        }
    }

    /// The hull list's card lines that mark a ship the player owns.
    fn board_rows(world: &World) -> Vec<String> {
        world
            .try_query::<&Text>()
            .map(|mut texts| {
                texts
                    .iter(world)
                    .filter(|text| text.0.ends_with(OWNED_DETAIL))
                    .map(|text| text.0.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Teleport the player to `position`, at rest.
    fn park_player(world: &mut World, position: Meters3) {
        let player = player(world);
        world.entity_mut(player).insert((
            Position(position.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("station buy: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// origin, in the station's own cell.
    fn park_beside(world: &mut World) {
        let watch = world.resource::<BuyWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_origin = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_origin * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            watch.cell.expect("the station is chosen"),
            "station buy: the park point stands in the station's cell"
        );
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("station buy: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("station buy: a section's chain reaches its root")
                .parent();
        }
        (position, rotation)
    }

    /// The free, live docking ports mounted on `root`, lowest first.
    fn free_ports(world: &mut World, root: Entity) -> Vec<Entity> {
        let mut ports: Vec<Entity> = ports_of(world, root)
            .into_iter()
            .filter(|port| world.get::<DockedPort>(*port).is_none())
            .collect();
        ports.sort();
        ports
    }

    /// The lowest free, live docking port mounted on `root`.
    fn free_port(world: &mut World, root: Entity) -> Entity {
        free_ports(world, root)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("station buy: {root:?} has no free docking port"))
    }

    /// The free port of `target` that faces most squarely away from its
    /// root, so the berth sets the player outside the hull. Which port is
    /// lowest changes with entity ids, and a station's lowest port can face
    /// into its own structure, where a berth crushes both hulls.
    fn outward_port(world: &mut World, target: Entity) -> Entity {
        free_ports(world, target)
            .into_iter()
            .map(|port| {
                let (at, rotation) = local_pose(world, port, target);
                let facing = (rotation * Vec3::NEG_Z).dot(at.normalize_or_zero());
                (port, facing)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .map(|(port, _)| port)
            .unwrap_or_else(|| panic!("station buy: {target:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's free port, moving and
    /// turning with the station, then ask to dock. Every frame until the dock
    /// takes, as `system_station_round_trip` does.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
        let station_port = outward_port(world, station);
        let player_port = free_port(world, player);
        let position = world.get::<Position>(station).expect("a station stands").0;
        let rotation = world.get::<Rotation>(station).expect("a station stands").0;
        let velocity = world
            .get::<LinearVelocity>(station)
            .map_or(Vec3::ZERO, |v| v.0);
        let spin = world
            .get::<AngularVelocity>(station)
            .map_or(Vec3::ZERO, |w| w.0);
        let (port_at, port_rot) = local_pose(world, station_port, station);
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let (at, turn) =
            parked_hull_pose((position, rotation), (port_at, port_rot), (own_at, own_rot));
        world.entity_mut(player).insert((
            Position(at),
            Rotation(turn),
            LinearVelocity(velocity),
            AngularVelocity(spin),
        ));
        // Point the mouse rig where the berth points the hull. Left on the
        // spawn heading, it turns the hull off the berth once the buy lets
        // the old dock go, and the new hull finds no port pair to redock.
        let rigs: Vec<Entity> = world
            .query_filtered::<Entity, With<PointRotationOutput>>()
            .iter(world)
            .collect();
        for rig in rigs {
            world.entity_mut(rig).insert((
                PointRotation {
                    initial_rotation: turn,
                },
                PointRotationOutput(turn),
            ));
        }
        world.trigger(DockingConnectionRequest {
            entity: player,
            target: station,
        });
    }

    fn set_field_text(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("station buy: no field named '{name}'"));
        value.0 = text.to_string();
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("station buy: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station buy: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Pick the nearest station that is living, not on the Enemy side,
    /// offers Refit, parks hulls for sale, and stands outside the origin's
    /// cell, where the walk stands after its jump; choose the retire jump.
    fn choose_the_station(world: &mut World) {
        let (edge, radius) = world_shape(world);
        let mut stations: Vec<(f32, GeneratedBodyId, Vec3)> = world
            .query_filtered::<(
                &GeneratedBodyId,
                &Position,
                Option<&StationServices>,
                Has<DerelictShipMarker>,
                Option<&Allegiance>,
                Has<PendingParkedHulls>,
                Has<ParkedHulls>,
            ), With<StationMarker>>()
            .iter(world)
            .filter(
                |(_, position, services, derelict, allegiance, pending, parked)| {
                    station_offers(*services, *derelict, StationServiceType::Refit)
                        && (*pending || *parked)
                        && *allegiance != Some(&Allegiance::Enemy)
                        && SectorCoord::containing(Meters3::from_engine(position.0), edge)
                            != SectorCoord::ORIGIN
                },
            )
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
                "station buy: the window on seed {STATION_SEED} holds no living, non-Enemy \
                 station with Refit and parked hulls outside the origin's cell"
            );
        };
        let position = Meters3::from_engine(at);
        let cell = SectorCoord::containing(position, edge);
        let axes = [cell.x, cell.y, cell.z];
        let (axis, _) = axes
            .iter()
            .enumerate()
            .max_by_key(|(_, index)| index.abs())
            .expect("three axes");
        let mut away = [0; 3];
        away[axis] = axes[axis] - axes[axis].signum() * (radius + 1);
        let away = SectorCoord::new(away[0], away[1], away[2]);
        assert!(
            !desired_sectors(away, radius).contains(&cell),
            "station buy: the window around {away} leaves {cell} out"
        );
        info!("station buy: chose '{id}' in cell {cell}, {distance:.0} m out");
        let mut watch = world.resource_mut::<BuyWatch>();
        watch.id = Some(id);
        watch.cell = Some(cell);
        watch.away = Some(away);
        watch.position = Some(position);
    }

    /// Claim 1: the station parks one hull for sale per port but one, each
    /// docked and inert, and one port stays free.
    fn assert_the_parked(world: &mut World) {
        let station = station(world);
        let ports = ports_of(world, station).len();
        let free = free_ports(world, station).len();
        let parked = parked_hulls(world, station);
        assert!(ports >= 2, "station buy: the station has {ports} ports");
        assert_eq!(
            parked.len(),
            ports - 1,
            "station buy: one hull for sale per port but one"
        );
        assert_eq!(free, 1, "station buy: one port stays free");
        for hull in &parked {
            let view = parked_view(world, *hull);
            assert_eq!(
                view.terms,
                ParkedHullType::Sale,
                "station buy: '{}' is for sale",
                view.name
            );
            assert!(
                docked_with(world, *hull, station),
                "station buy: '{}' is docked with the station",
                view.name
            );
            assert!(
                matches!(
                    world.get::<SpaceshipController>(*hull),
                    Some(SpaceshipController::None)
                ),
                "station buy: '{}' has no driver",
                view.name
            );
            assert_eq!(
                world.get::<Allegiance>(*hull),
                Some(&Allegiance::Neutral),
                "station buy: '{}' is Neutral",
                view.name
            );
            assert!(
                world.get::<LootableShipMarker>(*hull).is_none(),
                "station buy: '{}' is not lootable",
                view.name
            );
        }
        let names: Vec<String> = parked
            .iter()
            .map(|hull| parked_view(world, *hull).name)
            .collect();
        let hires = hire_names(world, station);
        world.resource_mut::<BuyWatch>().ports = ports;
        nova_probe::probe_marker(
            world,
            "outcome: a station parks its hulls for sale at its ports",
            serde_json::json!({
                "station": station_id(world).to_string(),
                "ports": ports,
                "parked": names,
                "hires": hires,
            }),
        );
        info!(
            "station buy: {ports} ports, parked for sale {names:?}, one port free, escorts for \
             hire {hires:?}"
        );
    }

    /// The facts before buying or boarding the parked hull `hull`, and aim
    /// the next click at its Buy or Board button, row `row`.
    fn record_before(world: &mut World, row: usize, hull: Entity) -> BeforeBuy {
        let player = player(world);
        let station = station(world);
        let view = parked_view(world, hull);
        let price_cr = match view.terms {
            ParkedHullType::Sale => {
                let design = world
                    .get::<ShipDesignSections>(hull)
                    .expect("station buy: a parked hull has a design");
                u32::try_from(hull_price_cr(
                    world.resource::<GameItems>(),
                    design.0.iter().map(|section| &section.config.base.build),
                ))
                .expect("the price fits")
            }
            ParkedHullType::Owned => 0,
            ParkedHullType::Hire { .. } => {
                panic!("station buy: '{}' is for hire, not for sale", view.name)
            }
        };
        let action = if view.terms == ParkedHullType::Owned {
            "Board"
        } else {
            "Buy"
        };
        world.resource_mut::<BuyWatch>().button = Some(format!("Hull{action}-{row}"));
        BeforeBuy {
            player_cr: credits(world, player),
            station_cr: credits(world, station),
            hold: hold_of(world, player),
            storage: storage_map(world, station),
            hull,
            price_cr,
            name: view.name,
            offered: view.sections,
            old: design_ids(world, player),
            flown: world
                .get::<Name>(player)
                .expect("station buy: the player's ship has a name")
                .to_string(),
            parked: parked_views(world, station),
            connection: world
                .get::<DockedShip>(player)
                .expect("station buy: the player is docked")
                .connection,
        }
    }

    /// Disclosed fixture state: the hold set to [`HELD_COUNT`] of
    /// [`HELD_ITEM`] and both balances set to [`FIXTURE_CREDITS`]. Then record
    /// the facts the buy of the first hull for sale must change.
    fn record_before_buy(world: &mut World) {
        let player = player(world);
        let station = station(world);
        let items = world.resource::<GameItems>().clone();
        {
            let mut hold = world
                .get_mut::<ShipInventory>(player)
                .expect("station buy: the player carries a hold");
            let stacks: Vec<(ItemDesignId, u32)> = hold
                .stacks()
                .map(|(item, count)| (item.clone(), count))
                .collect();
            for (item, count) in stacks {
                hold.remove(&item, count);
            }
            hold.add(&items, &HELD_ITEM.into(), HELD_COUNT);
        }
        world.get_mut::<ShipCredits>(player).expect("a balance").0 = FIXTURE_CREDITS;
        world.get_mut::<ShipCredits>(station).expect("a balance").0 = FIXTURE_CREDITS;
        info!(
            "station buy: fixture sets the hold to {HELD_COUNT} {HELD_ITEM} and both balances to \
             {FIXTURE_CREDITS} cr"
        );
        let (row, hull) = row_of(world, station, |terms| terms == ParkedHullType::Sale);
        let before = record_before(world, row, hull);
        info!(
            "station buy: row {row} '{}' costs {} cr",
            before.name, before.price_cr
        );
        world.resource_mut::<BuyWatch>().buy = Some(before);
    }

    /// Check the swap `before` led to: the player flies the hull bought or
    /// boarded, the hull it flew is parked as the player's own with the hold
    /// it had, the balances moved by the price (none for a board), and one
    /// `OnShipBought` report stands, the buy's. Returns the station's parked
    /// hulls.
    fn assert_the_swap(world: &World, before: &BeforeBuy, buy: &BeforeBuy) -> Vec<ParkedView> {
        let player = player(world);
        let station = station(world);
        let watch = world.resource::<BuyWatch>();
        assert_eq!(
            design_ids(world, player),
            before.offered,
            "station buy: the player's design is the bought hull's"
        );
        assert_eq!(
            live_ids(world, player),
            before.offered,
            "station buy: every bought section is live"
        );
        assert!(
            world.get_entity(before.hull).is_err(),
            "station buy: the bought hull is no longer parked"
        );
        let parked = parked_views(world, station);
        assert_eq!(
            parked.len(),
            before.parked.len(),
            "station buy: the station parks as many hulls as before"
        );
        let mut want = before.parked.clone();
        want.retain(|view| view.name != before.name || view.sections != before.offered);
        let old_ship = parked_hulls(world, station)
            .into_iter()
            .find(|hull| !want.iter().any(|view| *view == parked_view(world, *hull)))
            .expect("station buy: the old ship is parked");
        let old = parked_view(world, old_ship);
        assert_eq!(
            world.get::<Name>(player).map(Name::as_str),
            Some(before.name.as_str()),
            "station buy: the player's ship takes the bought hull's name"
        );
        assert_eq!(
            old.name, before.flown,
            "station buy: the old ship parks under the name it was flown as"
        );
        assert_eq!(
            old.terms,
            ParkedHullType::Owned,
            "station buy: the old ship is parked as the player's own"
        );
        assert_eq!(
            old.sections, before.old,
            "station buy: the old ship keeps its design"
        );
        assert_eq!(
            old.hold, before.hold,
            "station buy: the old ship keeps its hold"
        );
        assert!(
            world.get::<OwnedShipMarker>(old_ship).is_some(),
            "station buy: the old ship is the player's own"
        );
        assert!(
            docked_with(world, old_ship, station),
            "station buy: the old ship is docked with the station"
        );
        assert!(
            matches!(
                world.get::<SpaceshipController>(old_ship),
                Some(SpaceshipController::None)
            ),
            "station buy: the old ship has no driver"
        );
        assert_eq!(
            storage_map(world, station),
            before.storage,
            "station buy: storage did not change"
        );
        assert!(
            watch.named("onstored").is_empty(),
            "station buy: nothing was stored"
        );
        assert_eq!(
            credits(world, player),
            before.player_cr - before.price_cr,
            "station buy: the player paid the full price, nothing for the old ship"
        );
        assert_eq!(
            credits(world, station),
            before.station_cr + before.price_cr,
            "station buy: the station took the full price"
        );
        let causes = watch
            .named("oncreditschanged")
            .into_iter()
            .filter(|data| data["cause"] == "ShipBuy")
            .count();
        assert_eq!(
            causes, 2,
            "station buy: both balances moved as ShipBuy on the buy, and not on the board"
        );
        let reports = watch.named("onshipbought");
        assert_eq!(
            reports.len(),
            1,
            "station buy: one OnShipBought, the buy's; a board reports none"
        );
        let event = reports[0].clone();
        assert_eq!(event["hull"], buy.name.as_str());
        assert_eq!(event["price_cr"], buy.price_cr);
        assert!(
            event.get("trade_in_cr").is_none(),
            "station buy: OnShipBought carries no trade-in"
        );
        assert!(
            docked_with(world, player, station),
            "station buy: the player docked with the station"
        );
        assert_ne!(
            world
                .get::<DockedShip>(player)
                .expect("station buy: the player is docked")
                .connection,
            before.connection,
            "station buy: the player docked through a new connection"
        );
        parked
    }

    /// Claim 2: the buy; then undock.
    fn assert_the_buy(world: &mut World) {
        let before = world.resource::<BuyWatch>().buy().clone();
        let player = player(world);
        assert!(
            world
                .get::<ShipInventory>(player)
                .is_some_and(ShipInventory::is_empty),
            "station buy: the bought hull's hold opens empty"
        );
        let parked = assert_the_swap(world, &before, &before);
        let old = parked
            .iter()
            .find(|view| view.terms == ParkedHullType::Owned)
            .expect("station buy: the old ship is parked")
            .clone();
        nova_probe::probe_marker(
            world,
            "outcome: a buy hands the player the parked hull and parks the old one",
            serde_json::json!({
                "station": station_id(world).to_string(),
                "hull": before.name,
                "sections": before.offered.len(),
                "price_cr": before.price_cr,
                "old": old.name,
                "old_hold": old.hold,
            }),
        );
        info!(
            "station buy: bought '{}' ({} sections) for {} cr; '{}' parked as the player's own \
             holding {:?}",
            before.name,
            before.offered.len(),
            before.price_cr,
            old.name,
            old.hold
        );
        world.trigger(DockingReleaseRequest { entity: player });
    }

    /// The station's parked hulls as the player leaves it for the retire.
    fn record_the_station_as_left(world: &mut World) {
        let station = station(world);
        let left = parked_views(world, station);
        let mut watch = world.resource_mut::<BuyWatch>();
        watch.left = Some(left);
        watch.retired = Some(station);
    }

    fn the_station_record(
        sector: Option<&FrozenSector>,
        id: &str,
        cell: SectorCoord,
    ) -> FrozenBody {
        match sector {
            Some(FrozenSector::Visited(bodies)) => bodies
                .iter()
                .find(|body| body.id() == Some(id))
                .cloned()
                .unwrap_or_else(|| panic!("station buy: {cell}'s record has no '{id}'")),
            other => panic!("station buy: {cell} froze as {other:?}, not a visit"),
        }
    }

    /// The parked hulls a station record lists, as the walk compares them.
    /// A record keeps a design, not its section list, so `sections` holds
    /// the ids of `want` the record carries.
    fn record_views(body: &FrozenBody, want: &[ParkedView]) -> Vec<ParkedView> {
        let FrozenBodyType::Station(station) = body.body() else {
            panic!("station buy: the station froze as {:?}", body.body());
        };
        station
            .parked()
            .iter()
            .zip(want.iter().map(Some).chain(std::iter::repeat(None)))
            .map(|(parked, want)| ParkedView {
                name: parked.name().to_string(),
                terms: parked.terms(),
                sections: want.map_or_else(BTreeSet::new, |want| {
                    want.sections
                        .iter()
                        .filter(|id| parked.ship().has_section(id))
                        .cloned()
                        .collect()
                }),
                hold: hold_map(parked.ship().inventory()),
            })
            .collect()
    }

    /// Claim 3: the retired record lists the parked hulls as left.
    fn assert_the_retire(world: &mut World) {
        let id = station_id(world).to_string();
        let watch = world.resource::<BuyWatch>();
        let cell = watch.cell.expect("chosen");
        let left = watch.left.clone().expect("left");
        let retired = watch.retired.expect("left");
        assert!(
            !live_cells(world).contains(&cell),
            "station buy: the station's cell {cell} is retired"
        );
        assert!(
            world.get_entity(retired).is_err(),
            "station buy: the retire despawned the station"
        );
        let body = the_station_record(world.resource::<FrozenSectors>().get(cell), &id, cell);
        let recorded = record_views(&body, &left);
        assert_eq!(
            recorded, left,
            "station buy: the record lists the parked hulls as left"
        );
        let names: Vec<&str> = recorded.iter().map(|view| view.name.as_str()).collect();
        nova_probe::probe_marker(
            world,
            "outcome: the retired record holds the parked hulls",
            serde_json::json!({ "id": id, "cell": cell.to_string(), "parked": names }),
        );
        info!("station buy: sector {cell} retired with '{id}' parking {names:?}");
    }

    /// Claim 4: the thawed station parks the same hulls, each docked.
    fn assert_the_thaw(world: &mut World) {
        let watch = world.resource::<BuyWatch>();
        let left = watch.left.clone().expect("left");
        let retired = watch.retired.expect("left");
        let station = station(world);
        assert_ne!(station, retired, "station buy: the thaw is a new entity");
        assert_eq!(
            parked_views(world, station),
            left,
            "station buy: the thawed station parks the same hulls"
        );
        for hull in parked_hulls(world, station) {
            let owned = world.get::<OwnedShipMarker>(hull).is_some();
            let listed_owned = world.get::<ParkedHullType>(hull) == Some(&ParkedHullType::Owned);
            assert_eq!(
                owned, listed_owned,
                "station buy: only a hull listed as owned is the player's own"
            );
        }
        let names: Vec<&str> = left.iter().map(|view| view.name.as_str()).collect();
        nova_probe::probe_marker(
            world,
            "outcome: the thawed station parks the same hulls",
            serde_json::json!({ "id": station_id(world).to_string(), "parked": names }),
        );
        info!("station buy: the thawed station parks {names:?}, each docked");
    }

    /// Record the facts the board must change.
    fn record_before_board(world: &mut World) {
        let station = station(world);
        let (row, hull) = row_of(world, station, |terms| terms == ParkedHullType::Owned);
        let before = record_before(world, row, hull);
        info!(
            "station buy: row {row} '{}' boards for {} cr",
            before.name, before.price_cr
        );
        world.resource_mut::<BuyWatch>().board = Some(before);
    }

    /// Claim 5: the board hands the old ship back for free; then undock.
    fn assert_the_board(world: &mut World) {
        let watch = world.resource::<BuyWatch>();
        let buy = watch.buy().clone();
        let before = watch.board().clone();
        let rows = watch.board_rows.clone();
        assert_eq!(
            rows.len(),
            1,
            "station buy: the hull list shows one ship the player owns: {rows:?}"
        );
        assert_eq!(before.price_cr, 0, "station buy: a board is free");
        let player = player(world);
        assert_eq!(
            before.offered, buy.old,
            "station buy: the hull boarded is the old ship"
        );
        assert_eq!(
            hold_of(world, player),
            buy.hold,
            "station buy: the player has the old ship's hold back"
        );
        let parked = assert_the_swap(world, &before, &buy);
        world.resource_mut::<BuyWatch>().after = Some(parked.clone());
        let names: Vec<&str> = parked.iter().map(|view| view.name.as_str()).collect();
        nova_probe::probe_marker(
            world,
            "outcome: a board hands the player the old ship back for free",
            serde_json::json!({
                "station": station_id(world).to_string(),
                "hull": before.name,
                "player_cr": before.player_cr,
                "row": rows,
                "parked": names,
            }),
        );
        info!(
            "station buy: boarded '{}' for 0 cr, {} cr before and after; the station parks \
             {names:?}",
            before.name, before.player_cr
        );
        world.trigger(DockingReleaseRequest { entity: player });
    }

    /// Claim 6: the leave save holds the old ship and the parked hulls.
    fn assert_the_save(world: &mut World) {
        let id = station_id(world).to_string();
        let watch = world.resource::<BuyWatch>();
        let cell = watch.cell.expect("chosen");
        let buy = watch.buy().clone();
        let after = watch.after.clone().expect("boarded");
        let root = nova_assets::storage::worlds_root()
            .expect("station buy: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("the world name has a slug");
        let (_folder, _lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
            world.resource::<GameShipDesigns>(),
        )
        .unwrap_or_else(|refusal| panic!("station buy: cannot open the leave save: {refusal}"));
        for section in &buy.old {
            assert!(
                state.player.ship.has_section(section),
                "station buy: the saved player carries the old ship's '{section}'"
            );
        }
        for section in buy.offered.difference(&buy.old) {
            assert!(
                !state.player.ship.has_section(section),
                "station buy: the saved player keeps no bought '{section}'"
            );
        }
        let body = the_station_record(state.sectors.get(cell), &id, cell);
        let saved = record_views(&body, &after);
        assert_eq!(saved, after, "station buy: the save lists the parked hulls");
        let names: Vec<&str> = saved.iter().map(|view| view.name.as_str()).collect();
        nova_probe::probe_marker(
            world,
            "outcome: the leave save holds the player's hull and the parked hulls",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "sections": buy.old.len(),
                "parked": names,
            }),
        );
        info!(
            "station buy: save generation {} (format {}) holds the old ship and parks {names:?}",
            header.generation, header.format
        );
    }

    /// Claim 7: the loaded world gives back the old ship and the parked
    /// hulls.
    fn assert_the_load(world: &mut World) {
        let watch = world.resource::<BuyWatch>();
        let buy = watch.buy().clone();
        let after = watch.after.clone().expect("boarded");
        let player = player(world);
        let station = station(world);
        assert_eq!(
            live_ids(world, player),
            buy.old,
            "station buy: the loaded player's live sections are the old ship's"
        );
        assert_eq!(
            hold_of(world, player),
            buy.hold,
            "station buy: the loaded player holds the old ship's hold"
        );
        assert_eq!(
            parked_views(world, station),
            after,
            "station buy: the loaded station parks the same hulls"
        );
        let names: Vec<&str> = after.iter().map(|view| view.name.as_str()).collect();
        nova_probe::probe_marker(
            world,
            "outcome: a loaded world gives back the player's hull and the parked hulls",
            serde_json::json!({
                "id": station_id(world).to_string(),
                "sections": buy.old.len(),
                "parked": names,
            }),
        );
        info!(
            "station buy: PASS the loaded world gives back the player's hull and the parked \
             hulls {names:?}"
        );
    }

    /// Click the Buy or Board button [`BuyWatch::button`] names, the beats
    /// `click_named` appends with the name read at run time: which row a
    /// hull takes is known only once it is parked.
    fn click_the_button(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("{label}: the button is up"))
            .until(when(|world| {
                ui_node_rect(world, &the_button(world)).is_some()
            }))
            .deadline(BEAT_SECS)
            .add()
            .step(format!("{label}: aim"))
            .on_enter(|world: &mut World| hover_named(the_button(world))(world))
            .each(|world, _, _| keep_hovering_named(the_button(world))(world))
            .until(when(|world| pointer_over_node(the_button(world))(world)))
            .deadline(BEAT_SECS)
            .add()
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_SECS)
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Open the interface on the Station pane and scroll the aimed Buy or
    /// Board button into view.
    fn open_the_hull_list(
        script: AutopilotPlugin<GameStates>,
        label: &str,
    ) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            &format!("the interface key ({label})"),
            "interface_toggle",
            when(|world| {
                world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            }),
        )
        .click_named(
            &format!("station buy: open the Station pane ({label})"),
            "InterfaceTabStation",
            the_interface_shows(InterfacePaneType::Station),
            SESSION_SECS,
        )
        .step(format!("station buy: settle the lists ({label})"))
        .until(frames(SETTLE_FRAMES))
        .add()
        // The hull list can stand past the bottom of a short panel: the
        // wheel over the panel scrolls it into view.
        .step(format!(
            "station buy: scroll the hull list into view ({label})"
        ))
        .each(|world, _, frame| {
            keep_hovering_named(STATION_PANEL)(world);
            if frame % 2 == 1 && !buy_button_in_view(world) {
                scroll_lines(-1.0)(world);
            }
        })
        .until(when(buy_button_in_view))
        .deadline(SESSION_SECS)
        .add()
        .step(format!("station buy: settle the scroll ({label})"))
        .until(frames(SETTLE_FRAMES))
        .add()
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station buy: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "station buy: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("station buy: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "station buy: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "station buy: pick the start role",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "station buy: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station buy: the window streams in around the start")
            .until(window_is(start_cell))
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: the start docks the player at its starting station")
            .until(and(player_docked(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: jump to the origin, the spawn before the docked start")
            .on_enter(|world: &mut World| park_player(world, Meters3::ZERO))
            .until(window_is(|_| Some(SectorCoord::ORIGIN)))
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: choose the station")
            .on_enter(choose_the_station)
            .add()
            .step("station buy: jump to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<BuyWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: assert the parked hulls")
            .on_enter(assert_the_parked)
            .add()
            .step("station buy: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: settle the dock")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station buy: shoot the parked hulls")
            .on_enter(|world: &mut World| shoot(world, PARKED_SHOT))
            .until(shot_written(PARKED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("station buy: set the fixture and record the balances")
            .on_enter(record_before_buy)
            .until(frames(1))
            .add();
        let script = open_the_hull_list(script, "buy")
            .step("station buy: shoot the hull list")
            .on_enter(|world: &mut World| shoot(world, OFFER_SHOT))
            .until(shot_written(OFFER_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
        let script = click_the_button(
            script,
            "station buy: buy the first hull for sale",
            flies(|watch| &watch.buy().offered),
        );
        // The pane pushed its reports under the interface pause; the tracker
        // drains them once the clock runs again.
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station buy: the buy reports and the bought hull docks")
            .until(swapped_and_redocked(|watch| watch.buy.as_ref()))
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: settle the view")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station buy: shoot the docked hull")
            .on_enter(|world: &mut World| shoot(world, DOCKED_SHOT))
            .until(shot_written(DOCKED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("station buy: assert the buy, and undock")
            .on_enter(assert_the_buy)
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: record the station as left")
            .on_enter(record_the_station_as_left)
            .add()
            .step("station buy: jump away until the station's cell retires")
            .on_enter(|world: &mut World| {
                let (edge, _) = world_shape(world);
                let away = world.resource::<BuyWatch>().away.expect("chosen");
                park_player(world, away.centre(edge));
            })
            .until(window_is(|world| world.resource::<BuyWatch>().away))
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: assert the retired record")
            .on_enter(assert_the_retire)
            .add()
            .step("station buy: jump back to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<BuyWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: assert the thaw")
            .on_enter(assert_the_thaw)
            .add()
            .step("station buy: berth and dock again")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: record the board")
            .on_enter(record_before_board)
            .until(frames(1))
            .add();
        let script = open_the_hull_list(script, "board")
            .step("station buy: the hull list shows the old ship to board")
            .until(when(|world| !board_rows(world).is_empty()))
            .deadline(BEAT_SECS)
            .add()
            .step("station buy: record the board row")
            .on_enter(|world: &mut World| {
                let rows = board_rows(world);
                info!("station buy: the hull list shows {rows:?}");
                world.resource_mut::<BuyWatch>().board_rows = rows;
            })
            .add()
            .step("station buy: shoot the board row")
            .on_enter(|world: &mut World| shoot(world, BOARD_SHOT))
            .until(shot_written(BOARD_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
        let script = click_the_button(
            script,
            "station buy: board the old ship",
            flies(|watch| &watch.board().offered),
        );
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station buy: the old ship docks")
            .until(swapped_and_redocked(|watch| watch.board.as_ref()))
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: assert the board, and undock")
            .on_enter(assert_the_board)
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("station buy: let escape go")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "station buy: click Back to Main Menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("station buy: assert the leave save")
            .on_enter(assert_the_save)
            .add()
            .click_named(
                "station buy: click Load",
                LOAD_BUTTON,
                ui_node_present(format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                )),
                BEAT_SECS,
            )
            .click_named(
                "station buy: select the world",
                &format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                ),
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "station buy: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station buy: the loaded window streams the station back")
            .until(and(
                window_is(|world| world.resource::<BuyWatch>().cell),
                and(the_station_stands(), flies(|watch| &watch.board().offered)),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: assert the loaded world")
            .on_enter(assert_the_load)
            .add()
    }
}
