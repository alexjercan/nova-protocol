//! system_station_buy: a whole hull is bought from a living generated
//! station's catalogue, the old hull traded in, and the bought hull is the
//! player's ship from then on.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the window stream in.
//! It picks the nearest generated station that is living, not on the Enemy
//! side, offers Refit, sells hulls, and stands outside the spawn's cell. The
//! player jumps beside it and is put on a berth at its collar each frame
//! until the dock takes; the walk never flies the approach
//! (`system_station_docking` does).
//!
//! THE HOLD AND BOTH BALANCES ARE DISCLOSED FIXTURE STATE: once docked, the
//! hold is set to two section panels, and the player and the station each
//! hold 1,000,000 cr, so the first hull is affordable and the station can pay
//! any trade-in. The buy goes through the Station pane's Buy button on the first
//! hull, the code path a player clicks. The player undocks, leaves to the
//! main menu, the leave save is read back off disk through [`open_world`],
//! and the world is loaded again in this process through the Load screen.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a hull is bought from a station's catalogue` | the player's design is the offer, inline, and every offered section is live; the hold moved whole into the station's storage with one `OnStored` per stack; the player paid the price less the trade-in and the station took it, both as `ShipBuy`; `OnShipBought` fired once with the name, the price and the trade-in; the new hull docked with the station through a new connection |
//! | 2 | `outcome: the leave save holds the bought hull and the station's catalogue` | the saved player carries every offered section and none the offer lacks; the saved station keeps its catalogue, name for name |
//! | 3 | `outcome: a loaded world gives back the bought hull` | after Load the player's live sections are the offer's, and the station sells the same catalogue |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=2400 \
//!   cargo run --example system_station_buy --features debug
//! # look for: `station buy: PASS the loaded world gives back the bought hull`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the hull list as
//! `station-buy-offer.png` and the bought hull docked as
//! `station-buy-docked.png`.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_buy")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed: dock at a living generated station that sells hulls, buy \
             the first from the Station pane, then check the bought hull through a save and a Load. \
             Autopilot-only correctness range",
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
        desired_sectors, FrozenBody, FrozenBodyType, FrozenSector, SectorRoot, WorldConfig,
    };

    use super::*;

    /// The seed the range plays: `system_station_round_trip`'s seed, whose
    /// window around the spawn holds a living Neutral station outside the
    /// spawn's cell.
    const STATION_SEED: u32 = 10;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Station Buy World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";

    /// The offer the walk buys, and its Buy button.
    const OFFER: usize = 0;
    const BUY_BUTTON: &str = "HullBuy-0";
    /// The Station pane's panel, which scrolls under the wheel.
    const STATION_PANEL: &str = "StationPanel";

    /// The disclosed fixture hold, which the buy moves into storage.
    const HELD_ITEM: &str = "SectionPanel";
    const HELD_COUNT: u32 = 2;
    /// The disclosed fixture balance of the player and of the station.
    const FIXTURE_CREDITS: u32 = 1_000_000;

    /// The PNGs the capture path shoots.
    const OFFER_SHOT: &str = "station-buy-offer.png";
    const DOCKED_SHOT: &str = "station-buy-docked.png";

    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// The gap the berth leaves between the two port faces, engine units:
    /// half a cell, inside the 10 m capture distance.
    const BERTH_GAP: f32 = 0.5;
    /// Where a port's face sits in its own frame, as the docking verb reads it.
    const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Frames the Station pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;

    /// What the walk recorded right before the buy.
    #[derive(Clone, Debug)]
    struct BeforeBuy {
        player_cr: u32,
        station_cr: u32,
        hold: BTreeMap<String, u32>,
        storage: BTreeMap<String, u32>,
        price_cr: u32,
        trade_in_cr: u32,
        name: String,
        design: String,
        offered: BTreeSet<String>,
        old: BTreeSet<String>,
        catalogue: Vec<String>,
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
        /// Where it stood when chosen, for the park point.
        position: Option<Meters3>,
        /// The facts right before the buy.
        before: Option<BeforeBuy>,
    }

    impl BuyWatch {
        fn named(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events
                .iter()
                .filter(|(event, _)| event == name)
                .map(|(_, data)| data)
                .collect()
        }

        fn before(&self) -> &BeforeBuy {
            self.before
                .as_ref()
                .expect("station buy: the facts before the buy are recorded")
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

    /// The names of the hulls `station` sells.
    fn catalogue_names(world: &World, station: Entity) -> Vec<String> {
        world
            .get::<StationHullCatalogue>(station)
            .map_or_else(Vec::new, |catalogue| {
                catalogue.0.iter().map(|offer| offer.name.clone()).collect()
            })
    }

    fn offered_ids(world: &World) -> BTreeSet<String> {
        world.resource::<BuyWatch>().before().offered.clone()
    }

    fn world_shape(world: &World) -> (Meters, i32) {
        let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
        (config.sector_edge, config.active_radius)
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

    fn the_station_stands() -> Arc<Predicate> {
        when(|world| {
            station_root(world).is_some_and(|station| {
                world.get::<Position>(station).is_some()
                    && world.get::<HullOffers>(station).is_some()
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

    /// The player's design is the offer's.
    fn hull_bought() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| design_ids(world, player) == offered_ids(world))
        })
    }

    /// Every offered section is live on the player ship.
    fn bought_hull_live() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| live_ids(world, player) == offered_ids(world))
        })
    }

    /// The buy reported and the new hull docked through a new connection.
    fn bought_and_redocked() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<BuyWatch>();
            let Some(before) = watch.before.as_ref() else {
                return false;
            };
            let Some(player) = the_player(world) else {
                return false;
            };
            !watch.named("onshipbought").is_empty()
                && watch.named("oncreditschanged").len() >= 2
                && live_ids(world, player) == before.offered
                && world
                    .get::<DockedShip>(player)
                    .is_some_and(|docked| docked.connection != before.connection)
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

    /// Whether the Buy button lies inside the Station panel's box, where a
    /// click lands on it.
    fn buy_button_in_view(world: &World) -> bool {
        match (
            ui_node_rect(world, BUY_BUTTON),
            ui_node_rect(world, STATION_PANEL),
        ) {
            (Some(button), Some(panel)) => {
                button.min.y >= panel.min.y && button.max.y <= panel.max.y
            }
            _ => false,
        }
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
    /// spawn, in the station's own cell.
    fn park_beside(world: &mut World) {
        let watch = world.resource::<BuyWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_spawn = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_spawn * PARK_DISTANCE.get());
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

    /// The lowest free, live docking port mounted on `root`.
    fn free_port(world: &mut World, root: Entity) -> Entity {
        let ports: Vec<Entity> = world
            .query_filtered::<Entity, (
                With<DockingSectionMarker>,
                Without<DockedPort>,
                Without<SectionInactiveMarker>,
            )>()
            .iter(world)
            .collect();
        ports
            .into_iter()
            .filter(|port| {
                let mut at = *port;
                while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
                    if parent == root {
                        return true;
                    }
                    at = parent;
                }
                false
            })
            .min()
            .unwrap_or_else(|| panic!("station buy: {root:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's collar, moving and
    /// turning with the station, then ask to dock. Every frame until the dock
    /// takes, as `system_station_round_trip` does.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
        let station_port = free_port(world, station);
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
        let station_face_rot = rotation * port_rot;
        let station_face = position + rotation * port_at + station_face_rot * PORT_FACE;
        let station_axis = station_face_rot * Vec3::NEG_Z;
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let turn = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -station_axis);
        let face_offset = turn * own_at + turn * own_rot * PORT_FACE;
        let at = station_face + station_axis * BERTH_GAP - face_offset;
        world.entity_mut(player).insert((
            Position(at),
            Rotation(turn),
            LinearVelocity(velocity),
            AngularVelocity(spin),
        ));
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
    /// offers Refit, sells hulls, and stands outside the spawn's cell.
    fn choose_the_station(world: &mut World) {
        let (edge, _) = world_shape(world);
        let mut stations: Vec<(f32, GeneratedBodyId, Entity, Vec3)> = world
            .query_filtered::<(
                Entity,
                &GeneratedBodyId,
                &Position,
                Option<&StationServices>,
                Has<DerelictShipMarker>,
                Option<&Allegiance>,
                Option<&HullOffers>,
            ), With<StationMarker>>()
            .iter(world)
            .filter(|(_, _, position, services, derelict, allegiance, offers)| {
                station_offers(*services, *derelict, StationServiceType::Refit)
                    && offers.is_some_and(|offers| !offers.0.is_empty())
                    && *allegiance != Some(&Allegiance::Enemy)
                    && SectorCoord::containing(Meters3::from_engine(position.0), edge)
                        != SectorCoord::ORIGIN
            })
            .map(|(entity, id, position, ..)| {
                (
                    Meters3::from_engine(position.0)
                        .distance(Meters3::ZERO)
                        .get(),
                    id.clone(),
                    entity,
                    position.0,
                )
            })
            .collect();
        stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let Some((distance, id, entity, at)) = stations.into_iter().next() else {
            panic!(
                "station buy: the window on seed {STATION_SEED} holds no living, non-Enemy \
                 station with Refit and hulls outside the spawn's cell"
            );
        };
        let position = Meters3::from_engine(at);
        let cell = SectorCoord::containing(position, edge);
        info!(
            "station buy: chose '{id}' in cell {cell}, {distance:.0} m out, selling {:?}",
            catalogue_names(world, entity)
        );
        let mut watch = world.resource_mut::<BuyWatch>();
        watch.id = Some(id);
        watch.cell = Some(cell);
        watch.position = Some(position);
    }

    /// Disclosed fixture state: the hold set to [`HELD_COUNT`] of
    /// [`HELD_ITEM`] and both balances set to [`FIXTURE_CREDITS`]. Then record
    /// the balances, the hold, the storage, the offer and what the buy must
    /// cost and pay.
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

        let offers = world
            .get::<HullOffers>(station)
            .expect("station buy: the chosen station sells hulls");
        let offer = offers.0[OFFER].clone();
        let price_cr = hull_price_cr(
            &items,
            offer
                .sections
                .iter()
                .map(|section| &section.config.base.build),
        );
        let design = world
            .get::<ShipDesignSections>(player)
            .expect("station buy: the player ship has a design");
        let children: Vec<Entity> = world
            .get::<Children>(player)
            .map_or_else(Vec::new, |children| children.iter().collect());
        let live: Vec<(BuildRecipe, f32)> = children
            .into_iter()
            .filter(|child| world.get::<SectionMarker>(*child).is_some())
            .map(|child| {
                let id = &world.get::<EntityId>(child).expect("a section has an id").0;
                let health = world.get::<Health>(child).expect("a section has health");
                (
                    design
                        .get(id)
                        .expect("a live section is in the design")
                        .config
                        .base
                        .build
                        .clone(),
                    health.current / health.max,
                )
            })
            .collect();
        let trade_in_cr =
            hull_trade_in_cr(&items, live.iter().map(|(recipe, share)| (recipe, *share)));
        let catalogue = world
            .get::<StationHullCatalogue>(station)
            .expect("station buy: the chosen station carries its catalogue");
        let before = BeforeBuy {
            player_cr: credits(world, player),
            station_cr: credits(world, station),
            hold: hold_map(world.get::<ShipInventory>(player).expect("a hold")),
            storage: storage_map(world, station),
            price_cr: u32::try_from(price_cr).expect("the price fits"),
            trade_in_cr: u32::try_from(trade_in_cr).expect("the trade-in fits"),
            name: offer.name.clone(),
            design: format!("{:?}", catalogue.0[OFFER].design),
            offered: offer
                .sections
                .iter()
                .map(|section| section.id.clone())
                .collect(),
            old: design_ids(world, player),
            catalogue: catalogue_names(world, station),
            connection: world
                .get::<DockedShip>(player)
                .expect("station buy: the player is docked")
                .connection,
        };
        info!(
            "station buy: offer {OFFER} '{}' costs {} cr, the old hull trades in for {} cr",
            before.name, before.price_cr, before.trade_in_cr
        );
        world.resource_mut::<BuyWatch>().before = Some(before);
    }

    /// Claim 1: the bought hull, the hold, the credits, the reports and the
    /// redock; then undock.
    fn assert_the_buy(world: &mut World) {
        let player = player(world);
        let station = station(world);
        let watch = world.resource::<BuyWatch>();
        let before = watch.before().clone();
        assert_eq!(
            design_ids(world, player),
            before.offered,
            "station buy: the player's design is the offer"
        );
        assert_eq!(
            live_ids(world, player),
            before.offered,
            "station buy: every offered section is live"
        );
        match world.get::<SpaceshipDesign>(player) {
            Some(SpaceshipDesign(ShipDesignSource::Inline(design))) => assert_eq!(
                format!("{design:?}"),
                before.design,
                "station buy: the root's design is the offer, inline"
            ),
            other => panic!("station buy: the root's design is not inline: {other:?}"),
        }
        let hold = world.get::<ShipInventory>(player).expect("a hold");
        assert!(hold.is_empty(), "station buy: the new hold opens empty");
        let mut storage = before.storage.clone();
        for (item, count) in &before.hold {
            *storage.entry(item.clone()).or_default() += count;
        }
        assert_eq!(
            storage_map(world, station),
            storage,
            "station buy: the old hold moved whole into storage"
        );
        let stored = watch.named("onstored");
        assert_eq!(
            stored.len(),
            before.hold.len(),
            "station buy: one OnStored per stack"
        );
        assert_eq!(
            credits(world, player),
            before.player_cr - before.price_cr + before.trade_in_cr,
            "station buy: the player paid the price less the trade-in"
        );
        assert_eq!(
            credits(world, station),
            before.station_cr + before.price_cr - before.trade_in_cr,
            "station buy: the station took the price less the trade-in"
        );
        let causes: Vec<&serde_json::Value> = watch
            .named("oncreditschanged")
            .into_iter()
            .filter(|data| data["cause"] == "ShipBuy")
            .collect();
        assert_eq!(
            causes.len(),
            2,
            "station buy: both balances moved as ShipBuy"
        );
        let bought = watch.named("onshipbought");
        assert_eq!(bought.len(), 1, "station buy: one OnShipBought");
        let event = bought[0].clone();
        assert_eq!(event["hull"], before.name.as_str());
        assert_eq!(event["price_cr"], before.price_cr);
        assert_eq!(event["trade_in_cr"], before.trade_in_cr);
        let docked = world
            .get::<DockedShip>(player)
            .expect("station buy: the new hull is docked");
        assert_ne!(
            docked.connection, before.connection,
            "station buy: the new hull docked through a new connection"
        );
        let connection = world
            .get::<DockingConnection>(docked.connection)
            .expect("station buy: the dock has a connection");
        assert!(
            [connection.first_ship, connection.second_ship].contains(&station),
            "station buy: the new hull docked with the station"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a hull is bought from a station's catalogue",
            serde_json::json!({
                "station": station_id(world).to_string(),
                "hull": before.name,
                "sections": before.offered.len(),
                "price_cr": before.price_cr,
                "trade_in_cr": before.trade_in_cr,
                "stored": before.hold,
            }),
        );
        info!(
            "station buy: bought '{}' ({} sections) for {} cr less {} cr trade-in, redocked",
            before.name,
            before.offered.len(),
            before.price_cr,
            before.trade_in_cr
        );
        world.trigger(DockingReleaseRequest { entity: player });
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

    /// Claim 2: the leave save holds the bought hull and the catalogue.
    fn assert_the_save(world: &mut World) {
        let id = station_id(world).to_string();
        let watch = world.resource::<BuyWatch>();
        let cell = watch.cell.expect("chosen");
        let before = watch.before().clone();
        let root = nova_assets::storage::worlds_root()
            .expect("station buy: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("the world name has a slug");
        let (_folder, _lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| panic!("station buy: cannot open the leave save: {refusal}"));
        for section in &before.offered {
            assert!(
                state.player.ship.has_section(section),
                "station buy: the saved player carries the offered '{section}'"
            );
        }
        for section in before.old.difference(&before.offered) {
            assert!(
                !state.player.ship.has_section(section),
                "station buy: the saved player keeps no old '{section}'"
            );
        }
        let body = the_station_record(state.sectors.get(cell), &id, cell);
        let saved: Vec<String> = match body.body() {
            FrozenBodyType::Station(station) => station
                .hulls()
                .iter()
                .map(|offer| offer.name.clone())
                .collect(),
            other => panic!("station buy: the station froze as {other:?}"),
        };
        assert_eq!(
            saved, before.catalogue,
            "station buy: the save keeps the station's catalogue"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save holds the bought hull and the station's catalogue",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "sections": before.offered.len(),
                "catalogue": saved,
            }),
        );
        info!(
            "station buy: save generation {} (format {}) holds '{}' and the catalogue {saved:?}",
            header.generation, header.format, before.name
        );
    }

    /// Claim 3: the loaded world gives back the bought hull and the
    /// catalogue.
    fn assert_the_load(world: &mut World) {
        let before = world.resource::<BuyWatch>().before().clone();
        let player = player(world);
        let station = station(world);
        assert_eq!(
            live_ids(world, player),
            before.offered,
            "station buy: the loaded player's live sections are the offer's"
        );
        let catalogue = catalogue_names(world, station);
        assert_eq!(
            catalogue, before.catalogue,
            "station buy: the loaded station sells the same catalogue"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a loaded world gives back the bought hull",
            serde_json::json!({
                "id": station_id(world).to_string(),
                "hull": before.name,
                "sections": before.offered.len(),
                "catalogue": catalogue,
            }),
        );
        info!(
            "station buy: PASS the loaded world gives back the bought hull '{}' ({} sections)",
            before.name,
            before.offered.len()
        );
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
                ui_node_present(CREATE_WORLD_BUTTON),
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
                "station buy: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station buy: the window streams in around the spawn")
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
            .step("station buy: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: set the fixture and record the balances")
            .on_enter(record_before_buy)
            .until(frames(1))
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            when(|world| {
                world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            }),
        )
        .click_named(
            "station buy: open the Station pane",
            "InterfaceTabStation",
            the_interface_shows(InterfacePaneType::Station),
            SESSION_SECS,
        )
        .step("station buy: settle the lists")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The hull list can stand past the bottom of a short panel: the
        // wheel over the panel scrolls it into view.
        .step("station buy: scroll the hull list into view")
        .each(|world, _, frame| {
            keep_hovering_named(STATION_PANEL)(world);
            if frame % 2 == 1 && !buy_button_in_view(world) {
                scroll_lines(-1.0)(world);
            }
        })
        .until(when(buy_button_in_view))
        .deadline(SESSION_SECS)
        .add()
        .step("station buy: settle the scroll")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station buy: shoot the hull list")
        .on_enter(|world: &mut World| shoot(world, OFFER_SHOT))
        .until(shot_written(OFFER_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click_named(
            "station buy: buy the first hull",
            BUY_BUTTON,
            and(hull_bought(), bought_hull_live()),
            SESSION_SECS,
        );
        // The pane pushed its reports under the interface pause; the tracker
        // drains them once the clock runs again.
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station buy: the buy reports and the new hull docks")
            .until(bought_and_redocked())
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
                and(the_station_stands(), bought_hull_live()),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station buy: assert the loaded world")
            .on_enter(assert_the_load)
            .add()
    }
}
