//! system_station_restock: a thawed trader's stock drifts back to its target
//! by the play time it stood frozen, and a Load reads that play time off the
//! saved clock.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on the pinned
//! seed [`system_generated_stations`] already proved holds a living
//! generated station outside the origin's own cell, 53 km out, and lets the
//! window stream in around the session's starting station, where the start
//! docks the player. The walk undocks and jumps to the origin, the spawn
//! point before the docked start: stations are a civilization apart, so the
//! station it wants never streams in round the start. Once the window stands
//! round the origin the walk picks
//! the item the station's [`TraderMarket`] targets the most of (`item_x`,
//! a target of at least 2, so a half recovery lands strictly between empty
//! and full) and, as DISCLOSED FIXTURE STATE, sets its hold to zero and
//! parks a second item the market does not target (`item_y`) at a known
//! count. The player jumps away until the station's cell retires, the walk
//! mutates [`WorldClock`] forward by half the table's recovery (DISCLOSED
//! FIXTURE STATE: a real retiring cell reads whatever the clock says, and
//! nothing else advances it while a cell stands frozen), then jumps back to
//! thaw it. The claim reads the thawed body's own [`ThawedAfter`] stamp
//! rather than predicting it, so the assert holds however many frames the
//! jump actually took, and it requires the stock to land strictly below the
//! target, so a thaw that snapped to the target would fail. A second cycle
//! sets `item_x` to zero again on the live station and freezes and thaws it
//! directly with [`freeze_body`] and [`thaw_record`] (`system_station_add`'s
//! technique, but handed the station's own live `SectorRoot` rather than a
//! throwaway entity, so the station stays a normal cell-adopted body and a
//! later jump can retire it again) with no frame between the two clock
//! reads: a zero-elapsed thaw leaves it empty, which a thaw that ignored the
//! elapsed time could not.
//!
//! The walk then resets `item_x` to zero again, retires the station a second
//! time, advances the clock a whole recovery while it stands frozen, and leaves to
//! the main menu: the real leave save freezes the live window, so the
//! station's record keeps the lowered stock under the `frozen_at` the first
//! retire stamped it with, and the save's own clock is read back off disk
//! through [`open_world`] to confirm it before the Load. The world is then
//! loaded again in this process through the Load screen, the way
//! `system_station_buy` does it, and the jump back to the station thaws it
//! under the save's own `frozen_at` and the restored clock: the claim is that
//! the stock the Load leaves it with is `restocked_count` of the saved gap,
//! not the gap from the earlier, in-session cycle.
//!
//! ONE SUBJECT: `nova_world_base::economy::restock_thawed_traders` driven by
//! play time, in session and across a Load. What a trader's stock PRICES at
//! is `nova_gameplay::market`'s own unit tests; what a civilization's station
//! generates with is `nova_world_base::sector_stations`'s.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a thawed station restocks by the play time it stood frozen` | the thawed stock of the most-targeted item equals `restocked_count` of the play time `ThawedAfter` reports, above what it froze with and below its target; an untargeted item is unchanged; a second, zero-elapsed thaw of the same station, emptied again first, leaves it empty |
//! | 2 | `outcome: a loaded world restocks the station from the saved clock` | after a Load, the thawed stock of the targeted item equals `restocked_count` of the play time between the saved `frozen_at` and the saved clock, not the earlier in-session gap |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example system_station_restock --features debug
//! # look for: `station restock probe: thawed after ... s of play`,
//! #           `station restock probe: a zero-elapsed thaw left ... unchanged`,
//! #           `station restock probe: the loaded world restocked ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_restock")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed, then a generated station's trade stock is driven to \
             zero, retired, advanced forward in play time and thawed, proving the restock math \
             and that a zero-elapsed thaw changes nothing, then retired, saved, Loaded and \
             thawed again to prove the Load reads the saved clock. Autopilot-only correctness \
             range",
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
        // No frame-time claim: the range holds counts and a play-time stamp.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::RestockWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position};
    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, Predicate},
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{
        desired_sectors, freeze_body, thaw_record, DockPolicy, FrozenBody, FrozenBodyType,
        FrozenSector, FrozenSectors, PreparedFrozenBody, SectorRoot, ThawedAfter, WorldClock,
        WorldConfig,
    };

    use super::*;

    /// `system_generated_stations`'s seed: its window around the origin holds
    /// one living generated station outside the origin's own cell, 53 km out.
    const STATION_SEED: u32 = 6;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Restock World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";

    /// How far from the station the player parks: outside the observer's
    /// hold radius, as `system_generated_stations` parks.
    const PARK_DISTANCE: Meters = Meters(2_000.0);

    /// The disclosed fixture stock a round sets `item_x` to before a retire.
    const BEFORE_STOCK: u32 = 0;
    /// The disclosed fixture stock of `item_y`, the item the market does not
    /// target: untouched by the restock system's own loop over its target
    /// map, whatever this is.
    const UNTARGETED_ITEM_COUNT: u32 = 2;
    /// The least target `item_x` may carry: a half recovery of 2 or more
    /// lands strictly between empty and full.
    const LEAST_TARGET: u32 = 2;
    /// The play time the walk advances the clock by while the station
    /// stands frozen, as a share of the table's recovery: half in round 1,
    /// so the thaw lands strictly below the target, and a whole recovery in
    /// round 2, so the Load's number is not round 1's.
    const CLOCK_BUMP_1_RECOVERIES: f64 = 0.5;
    const CLOCK_BUMP_2_RECOVERIES: f64 = 1.0;

    /// Seconds a load, a stream or a Load screen gets on a software-rendered
    /// GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds a menu click gets.
    const BEAT_SECS: f32 = 60.0;
    /// The virtual clock's clamp on this software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;

    /// What the run chose and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct RestockWatch {
        /// The chosen station's typed id.
        id: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// The cell the retire jump lands in.
        away: Option<SectorCoord>,
        /// Where it stood when chosen, for the park point.
        position: Option<Meters3>,
        /// The targeted item the walk drives to zero and back.
        item_x: Option<ItemDesignId>,
        /// An item the market does not target.
        item_y: Option<ItemDesignId>,
        /// `item_x`'s target stock.
        target_x: u32,
        /// The content's restock clock.
        rule: Option<RestockRule>,
        /// `item_x`'s count right before the zero-elapsed direct cycle.
        before_2: u32,
    }

    pub(super) struct RestockWatchPlugin;

    impl Plugin for RestockWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<RestockWatch>();
            app.add_systems(Update, hold_the_walk_clock);
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
        the_player(world).expect("restock: exactly one player ship")
    }

    /// The open world's cell edge and live radius, as New Game armed them.
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

    fn live_cells(world: &World) -> std::collections::BTreeSet<SectorCoord> {
        world
            .try_query::<&SectorRoot>()
            .map(|mut query| query.iter(world).map(|root| root.0).collect())
            .unwrap_or_default()
    }

    /// Advance once the world is armed around one player ship and the live
    /// set is exactly the window `centre` gives.
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

    /// The entity wearing `id`, if one does.
    fn body_wearing(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| *worn == id)
            .map(|(entity, _)| entity)
    }

    /// The entity wearing the chosen station's id.
    fn station(world: &World) -> Entity {
        let id = world
            .resource::<RestockWatch>()
            .id
            .clone()
            .expect("restock: the station is chosen before it is read");
        body_wearing(world, &id).expect("restock: the station stands")
    }

    fn the_station_stands() -> Arc<Predicate> {
        when(|world| {
            let Some(id) = world.resource::<RestockWatch>().id.clone() else {
                return false;
            };
            body_wearing(world, &id).is_some()
        })
    }

    /// Advance once the station stands again, newly thawed, with its hold
    /// rebuilt.
    fn the_station_rebuilt() -> Arc<Predicate> {
        when(|world| {
            let Some(id) = world.resource::<RestockWatch>().id.clone() else {
                return false;
            };
            body_wearing(world, &id)
                .is_some_and(|entity| world.get::<ShipInventory>(entity).is_some())
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

    /// Teleport the player to `position`, at rest: the walk crosses cell
    /// faces without flying them.
    fn park_player(world: &mut World, position: Meters3) {
        let player = player(world);
        world.entity_mut(player).insert((
            Position(position.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("station restock probe: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// origin, in the station's own cell.
    fn park_beside(world: &World) -> Meters3 {
        let watch = world.resource::<RestockWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_origin = (-position.get()).normalize_or_zero();
        Meters3(position.get() + toward_origin * PARK_DISTANCE.get())
    }

    /// The live `SectorRoot` entity of `cell`.
    fn sector_root_entity(world: &mut World, cell: SectorCoord) -> Entity {
        world
            .query::<(Entity, &SectorRoot)>()
            .iter(world)
            .find(|(_, root)| root.0 == cell)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("restock: no live SectorRoot for {cell}"))
    }

    fn set_field_text(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("restock: no field named '{name}'"));
        value.0 = text.to_string();
    }

    /// Force `item`'s count in `hold` to exactly `count`.
    fn set_item_count(
        hold: &mut ShipInventory,
        items: &GameItems,
        item: &ItemDesignId,
        count: u32,
    ) {
        let current = hold.count(item);
        if current < count {
            hold.add(items, item, count - current);
        } else if current > count {
            hold.remove(item, current - count);
        }
    }

    /// Pick the generated station nearest the origin that carries a
    /// `TraderMarket`, choose the retire jump the way
    /// `system_generated_stations` does, then pick `item_x` (a targeted item,
    /// above zero target) and `item_y` (an item the market names nowhere).
    ///
    /// Refuses a seed whose window holds no such station, whose nearest one
    /// stands in the origin's own cell, where the walk stands after its jump,
    /// whose market targets none of its
    /// items, or whose market targets every catalog item: each would make
    /// this claim about something else.
    fn find_the_station_and_its_market(world: &mut World) {
        let (edge, radius) = world_shape(world);
        let mut stations: Vec<(f32, GeneratedBodyId, Entity)> = world
            .query_filtered::<(Entity, &GeneratedBodyId, &Position), (
                With<StationMarker>,
                With<TraderMarket>,
            )>()
            .iter(world)
            .map(|(entity, id, position)| {
                (
                    Meters3::from_engine(position.0)
                        .distance(Meters3::ZERO)
                        .get(),
                    id.clone(),
                    entity,
                )
            })
            .collect();
        stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let Some((distance, id, entity)) = stations.into_iter().next() else {
            panic!(
                "restock: the window around the origin holds no generated station with a \
                 TraderMarket on seed {STATION_SEED}, so this claim cannot be made here"
            );
        };
        let position = Meters3::from_engine(
            world
                .get::<Position>(entity)
                .expect("restock: a station has a position")
                .0,
        );
        let cell = SectorCoord::containing(position, edge);
        assert_ne!(
            cell,
            SectorCoord::ORIGIN,
            "restock: the nearest station stands in the origin's own cell, where the walk stands \
             after its jump, on seed {STATION_SEED}, which one jump cannot retire"
        );

        // The retire cell: past the far face along the station's farthest
        // axis, as `system_generated_stations` chooses it.
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
            "the window around {away} leaves {cell} out"
        );

        let market = world
            .get::<TraderMarket>(entity)
            .expect("restock: a found station carries a TraderMarket")
            .clone();
        let item_x = market
            .target
            .iter()
            .max_by_key(|(item, target)| (**target, std::cmp::Reverse((*item).clone())))
            .map(|(item, _)| item.clone())
            .unwrap_or_else(|| {
                panic!("restock: '{id}' targets none of its items; cannot prove a restock")
            });
        let target_x = market.target[&item_x];
        assert!(
            target_x >= LEAST_TARGET,
            "restock: '{id}' targets at most {target_x} of any item; a half recovery needs a \
             target of {LEAST_TARGET} or more to land strictly between empty and full"
        );
        let item_y = world
            .resource::<GameItems>()
            .iter()
            .map(|design| design.id.clone())
            .find(|candidate| !market.target.contains_key(candidate))
            .unwrap_or_else(|| {
                panic!("restock: '{id}' targets every catalog item; no untouched item to prove")
            });
        let rule = world.resource::<GamePriceTable>().0.restock;

        info!(
            "station restock probe: nearest station '{id}' in cell {cell}, {distance:.0} m from \
             the origin; {item_x} targets {target_x}, recovers in {:.0} s; {item_y} is untargeted",
            rule.recovery_seconds
        );
        let mut watch = world.resource_mut::<RestockWatch>();
        watch.id = Some(id);
        watch.cell = Some(cell);
        watch.away = Some(away);
        watch.position = Some(position);
        watch.item_x = Some(item_x);
        watch.item_y = Some(item_y);
        watch.target_x = target_x;
        watch.rule = Some(rule);
    }

    /// The disclosed round-1 fixture: `item_x` to zero, `item_y` to
    /// [`UNTARGETED_ITEM_COUNT`].
    fn set_round1_fixture(world: &mut World) {
        let entity = station(world);
        let item_x = world
            .resource::<RestockWatch>()
            .item_x
            .clone()
            .expect("chosen");
        let item_y = world
            .resource::<RestockWatch>()
            .item_y
            .clone()
            .expect("chosen");
        world.resource_scope(|world, items: Mut<GameItems>| {
            let mut hold = world
                .get_mut::<ShipInventory>(entity)
                .expect("restock: a station carries a hold");
            set_item_count(&mut hold, &items, &item_x, BEFORE_STOCK);
            set_item_count(&mut hold, &items, &item_y, UNTARGETED_ITEM_COUNT);
        });
        info!(
            "station restock probe: round 1 fixture: {item_x} set to {BEFORE_STOCK}, {item_y} \
             set to {UNTARGETED_ITEM_COUNT}"
        );
    }

    /// The disclosed round-2 fixture: `item_x` reset to zero on the live
    /// station, decoupling the Load claim from round 1's numbers.
    fn set_round2_fixture(world: &mut World) {
        let entity = station(world);
        let item_x = world
            .resource::<RestockWatch>()
            .item_x
            .clone()
            .expect("chosen");
        world.resource_scope(|world, items: Mut<GameItems>| {
            let mut hold = world
                .get_mut::<ShipInventory>(entity)
                .expect("restock: a station carries a hold");
            set_item_count(&mut hold, &items, &item_x, BEFORE_STOCK);
        });
        info!("station restock probe: round 2 fixture: {item_x} reset to {BEFORE_STOCK}");
    }

    /// The record a cell's frozen ledger holds for `id`.
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
                .unwrap_or_else(|| panic!("restock: {cell}'s record has no '{id}'")),
            other => panic!("restock: {cell} froze as {other:?}, not a visit"),
        }
    }

    fn station_stock(body: &FrozenBody, item: &ItemDesignId) -> u32 {
        let FrozenBodyType::Station(station) = body.body() else {
            panic!("restock: the station froze as {:?}", body.body());
        };
        station.ship().inventory().count(item)
    }

    /// The jump away retired the station's cell holding the lowered stock.
    fn assert_retired_stock(world: &World, label: &str) {
        let watch = world.resource::<RestockWatch>();
        let id = watch.id.clone().expect("chosen").to_string();
        let cell = watch.cell.expect("chosen");
        let item_x = watch.item_x.clone().expect("chosen");
        assert!(
            !live_cells(world).contains(&cell),
            "restock: {label}: the station's cell {cell} is retired"
        );
        let body = the_station_record(world.resource::<FrozenSectors>().get(cell), &id, cell);
        assert_eq!(
            station_stock(&body, &item_x),
            BEFORE_STOCK,
            "restock: {label}: the retired record holds the lowered stock"
        );
        info!("station restock probe: {label}: sector {cell} retired holding {item_x} at {BEFORE_STOCK}");
    }

    /// Advance the play-time clock by `bump_secs` while the station's cell
    /// stands frozen. DISCLOSED FIXTURE STATE: a real retiring cell reads
    /// whatever the clock says when it thaws, and this is the only thing
    /// that moves it while nothing else is live to drive `Time<Virtual>`
    /// forward on its own.
    fn bump_the_clock(recoveries: f64) -> impl Fn(&mut World) {
        move |world: &mut World| {
            let rule = world.resource::<RestockWatch>().rule.expect("chosen");
            let bump_secs = f64::from(rule.recovery_seconds) * recoveries;
            let mut clock = world.resource_mut::<WorldClock>();
            clock.play_seconds += bump_secs;
            info!(
                "station restock probe: the clock advances {bump_secs:.0} s while the station \
                 stands frozen, to {:.0} s",
                clock.play_seconds
            );
        }
    }

    /// Claim 1, first half: the thawed stock matches `restocked_count` of the
    /// play time [`ThawedAfter`] reports, above what it froze with and below
    /// its target; the untargeted item is unchanged.
    fn assert_the_main_restock(world: &mut World) {
        let (id, item_x, item_y, target_x, rule) = {
            let watch = world.resource::<RestockWatch>();
            (
                watch.id.clone().expect("chosen"),
                watch.item_x.clone().expect("chosen"),
                watch.item_y.clone().expect("chosen"),
                watch.target_x,
                watch.rule.expect("chosen"),
            )
        };
        let entity = body_wearing(world, &id).expect("restock: the thawed station stands");
        let elapsed = world
            .get::<ThawedAfter>(entity)
            .expect("restock: a thawed body carries ThawedAfter")
            .play_seconds;
        assert!(
            elapsed > 0.0,
            "restock: the station must stand frozen a positive play time, not {elapsed}"
        );
        let hold = world
            .get::<ShipInventory>(entity)
            .expect("restock: a station carries a hold");
        let count_x = hold.count(&item_x);
        let count_y = hold.count(&item_y);
        let expected = restocked_count(BEFORE_STOCK, target_x, elapsed, rule);
        assert_eq!(
            count_x, expected,
            "restock: the thawed stock matches restocked_count of the play time it stood frozen"
        );
        assert!(
            count_x > BEFORE_STOCK,
            "restock: the thaw must move the stock above {BEFORE_STOCK}, not leave it at {count_x}"
        );
        assert!(
            count_x < target_x,
            "restock: a half recovery must land below the target {target_x}, not at {count_x}"
        );
        assert_eq!(
            count_y, UNTARGETED_ITEM_COUNT,
            "restock: an item outside the target map must stay untouched"
        );
        info!(
            "station restock probe: thawed after {elapsed:.1} s of play, {item_x} {BEFORE_STOCK} \
             -> {count_x} of {target_x}, {item_y} stayed at {count_y}"
        );
    }

    /// Empty `item_x` again (DISCLOSED FIXTURE STATE, so the stock stands
    /// below its target and a thaw that ignored the elapsed time would show),
    /// then freeze and thaw the live station directly, with no frame between
    /// the two reads of [`WorldClock`], so the play time it stands frozen is
    /// exactly zero. Handed the station's own live `SectorRoot` (not a
    /// throwaway entity, as `system_station_add` hands the player's thaw):
    /// the station is a cell-adopted body, and a later jump must still be
    /// able to retire it.
    fn zero_elapsed_cycle(world: &mut World) {
        let (id, item_x, cell) = {
            let watch = world.resource::<RestockWatch>();
            (
                watch.id.clone().expect("chosen"),
                watch.item_x.clone().expect("chosen"),
                watch.cell.expect("chosen"),
            )
        };
        let entity = body_wearing(world, &id).expect("restock: the station stands");
        world.resource_scope(|world, items: Mut<GameItems>| {
            let mut hold = world
                .get_mut::<ShipInventory>(entity)
                .expect("restock: a station carries a hold");
            set_item_count(&mut hold, &items, &item_x, BEFORE_STOCK);
        });
        let before_2 = world
            .get::<ShipInventory>(entity)
            .expect("restock: a station carries a hold")
            .count(&item_x);
        assert_eq!(
            before_2, BEFORE_STOCK,
            "restock: the fixture emptied {item_x}"
        );
        world
            .entity_mut(entity)
            .insert((LinearVelocity(Vec3::ZERO), AngularVelocity(Vec3::ZERO)));
        let frozen = freeze_body(
            world,
            entity,
            DockPolicy::Refuse,
            world.resource::<WorldClock>(),
        )
        .expect("restock: an undocked station at rest is settled");
        world.entity_mut(entity).despawn();
        let root = sector_root_entity(world, cell);
        let prepared = PreparedFrozenBody::prepare(cell, frozen, &[], &[]);
        let clock = *world.resource::<WorldClock>();
        let mut commands = world.commands();
        thaw_record(&mut commands, root, vec![prepared], &clock);
        world.flush();
        world.resource_mut::<RestockWatch>().before_2 = before_2;
    }

    /// Claim 1, second half: the zero-elapsed thaw left the emptied stock
    /// empty. Fires claim 1's marker, bundling both halves.
    fn assert_the_zero_elapsed_thaw(world: &mut World) {
        let (id, item_x, target_x, before_2) = {
            let watch = world.resource::<RestockWatch>();
            (
                watch.id.clone().expect("chosen"),
                watch.item_x.clone().expect("chosen"),
                watch.target_x,
                watch.before_2,
            )
        };
        let entity = body_wearing(world, &id).expect("restock: the directly-thawed station stands");
        let elapsed = world
            .get::<ThawedAfter>(entity)
            .expect("restock: a thawed body carries ThawedAfter")
            .play_seconds;
        assert_eq!(
            elapsed, 0.0,
            "restock: a direct freeze and thaw with no clock advance between the two reads must \
             show zero elapsed play time, not {elapsed}"
        );
        let count_x = world
            .get::<ShipInventory>(entity)
            .expect("restock: a station carries a hold")
            .count(&item_x);
        assert_eq!(
            count_x, before_2,
            "restock: a zero-elapsed thaw must change nothing, not {before_2} -> {count_x}"
        );
        info!("station restock probe: a zero-elapsed thaw left {item_x} at {count_x} unchanged");
        nova_probe::probe_marker(
            world,
            "outcome: a thawed station restocks by the play time it stood frozen",
            serde_json::json!({
                "item": item_x.to_string(),
                "target": target_x,
                "zero_elapsed_count": count_x,
            }),
        );
    }

    /// The leave save, read back off disk through [`open_world`], holds the
    /// lowered stock under the retired cell.
    fn assert_the_leave_save(world: &mut World) {
        let (id, cell, item_x) = {
            let watch = world.resource::<RestockWatch>();
            (
                watch.id.clone().expect("chosen").to_string(),
                watch.cell.expect("chosen"),
                watch.item_x.clone().expect("chosen"),
            )
        };
        let root = nova_assets::storage::worlds_root()
            .expect("restock: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("the world name has a slug");
        let (_folder, _lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| panic!("restock: cannot open the leave save: {refusal}"));
        let body = the_station_record(state.sectors.get(cell), &id, cell);
        assert_eq!(
            station_stock(&body, &item_x),
            BEFORE_STOCK,
            "restock: the leave save holds the lowered stock"
        );
        info!(
            "station restock probe: save generation {} (format {}) holds {item_x} at \
             {BEFORE_STOCK}",
            header.generation, header.format
        );
    }

    /// Claim 2: after the Load, the thawed stock matches `restocked_count` of
    /// the play time between the saved `frozen_at` and the saved clock.
    fn assert_the_loaded_restock(world: &mut World) {
        let (id, item_x, target_x, rule) = {
            let watch = world.resource::<RestockWatch>();
            (
                watch.id.clone().expect("chosen"),
                watch.item_x.clone().expect("chosen"),
                watch.target_x,
                watch.rule.expect("chosen"),
            )
        };
        let entity = body_wearing(world, &id).expect("restock: the loaded station thaws");
        let elapsed = world
            .get::<ThawedAfter>(entity)
            .expect("restock: a thawed body carries ThawedAfter")
            .play_seconds;
        assert!(
            elapsed > 0.0,
            "restock: the saved gap must be a positive play time, not {elapsed}"
        );
        let count_x = world
            .get::<ShipInventory>(entity)
            .expect("restock: a station carries a hold")
            .count(&item_x);
        let expected = restocked_count(BEFORE_STOCK, target_x, elapsed, rule);
        assert_eq!(
            count_x, expected,
            "restock: the loaded stock matches restocked_count of the saved clock's gap"
        );
        assert!(
            count_x > BEFORE_STOCK,
            "restock: the loaded thaw must move the stock above {BEFORE_STOCK}, not leave it at \
             {count_x}"
        );
        info!(
            "station restock probe: the loaded world restocked {item_x} to {count_x} of \
             {target_x} over the saved {elapsed:.1} s"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a loaded world restocks the station from the saved clock",
            serde_json::json!({
                "item": item_x.to_string(),
                "target": target_x,
                "elapsed_s": elapsed,
                "count": count_x,
            }),
        );
    }

    /// The walk. Every beat waits on a world value, so a slow load delays it
    /// instead of truncating it.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("restock: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "restock: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("restock: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "restock: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "restock: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("restock: the window streams in around the start")
            .until(window_is(start_cell))
            .deadline(SESSION_SECS)
            .add()
            .step("restock: the start docks the player at its starting station")
            .until(and(player_docked(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("restock: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("restock: jump to the origin, the spawn before the docked start")
            .on_enter(|world: &mut World| park_player(world, Meters3::ZERO))
            .until(window_is(|_| Some(SectorCoord::ORIGIN)))
            .deadline(SESSION_SECS)
            .add()
            .step("restock: find the station and its market")
            .on_enter(find_the_station_and_its_market)
            .add()
            .step("restock: jump to the station's side")
            .on_enter(|world: &mut World| {
                let beside = park_beside(world);
                park_player(world, beside);
            })
            .until(and(
                window_is(|world| world.resource::<RestockWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("restock: set the round 1 fixture stock")
            .on_enter(set_round1_fixture)
            .until(frames(1))
            .add()
            .step("restock: jump away until the station's cell retires")
            .on_enter(|world: &mut World| {
                let (edge, _) = world_shape(world);
                let away = world.resource::<RestockWatch>().away.expect("chosen");
                park_player(world, away.centre(edge));
            })
            .until(window_is(|world| world.resource::<RestockWatch>().away))
            .deadline(SESSION_SECS)
            .add()
            .step("restock: assert the round 1 retire")
            .on_enter(|world: &mut World| assert_retired_stock(world, "round 1"))
            .add()
            .step("restock: let real play time pass while the station stands frozen")
            .on_enter(bump_the_clock(CLOCK_BUMP_1_RECOVERIES))
            .until(frames(1))
            .add()
            .step("restock: jump back to thaw the station")
            .on_enter(|world: &mut World| {
                let beside = park_beside(world);
                park_player(world, beside);
            })
            .until(and(
                window_is(|world| world.resource::<RestockWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("restock: assert the main restock")
            .on_enter(assert_the_main_restock)
            .add()
            .step("restock: freeze and thaw the station directly with no clock advance")
            .on_enter(zero_elapsed_cycle)
            .until(the_station_rebuilt())
            .deadline(SESSION_SECS)
            .add()
            .step("restock: assert the zero-elapsed thaw")
            .on_enter(assert_the_zero_elapsed_thaw)
            .add()
            .step("restock: reset the fixture stock for the save round")
            .on_enter(set_round2_fixture)
            .until(frames(1))
            .add()
            .step("restock: jump away until the station's cell retires again")
            .on_enter(|world: &mut World| {
                let (edge, _) = world_shape(world);
                let away = world.resource::<RestockWatch>().away.expect("chosen");
                park_player(world, away.centre(edge));
            })
            .until(window_is(|world| world.resource::<RestockWatch>().away))
            .deadline(SESSION_SECS)
            .add()
            .step("restock: assert the save round's retire")
            .on_enter(|world: &mut World| assert_retired_stock(world, "save round"))
            .add()
            .step("restock: let real play time pass before the save")
            .on_enter(bump_the_clock(CLOCK_BUMP_2_RECOVERIES))
            .until(frames(1))
            .add()
            .step("restock: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("restock: let escape go")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "restock: click Back to Main Menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("restock: assert the leave save")
            .on_enter(assert_the_leave_save)
            .add()
            .click_named(
                "restock: click Load",
                LOAD_BUTTON,
                ui_node_present(format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                )),
                BEAT_SECS,
            )
            .click_named(
                "restock: select the world",
                &format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                ),
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "restock: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("restock: the resumed window streams in")
            .until(window_is(|world| world.resource::<RestockWatch>().away))
            .deadline(SESSION_SECS)
            .add()
            .step("restock: jump back to thaw the station after the Load")
            .on_enter(|world: &mut World| {
                let beside = park_beside(world);
                park_player(world, beside);
            })
            .until(and(
                window_is(|world| world.resource::<RestockWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("restock: assert the loaded restock")
            .on_enter(assert_the_loaded_restock)
            .add()
    }
}
