//! system_station_visits: real seeded generated traffic visits a station in
//! the full game: a profiled ship binds a visit, flies a Dock order, trades
//! and posts jobs once on the capture, dwells, is released and flies clear.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, then lets the player go from
//! the session's starting station (its one port, taken, refuses every
//! visitor) and holds it at rest while play time runs. Nothing is staged:
//! the ships, the stations, the trade profiles and the visit writer
//! (`nova_world_base::station_visits`) are the shipped ones. The example
//! only READS. It never writes the world clock or the visit ledger, and the
//! player moves only for the shot and, once the visit is over, to dock and
//! take a visit job off the board. A watcher system samples
//! the visitor's and the station's balances and holds each frame, so the
//! receipt can be checked against what moved on the capture frame.
//!
//! The seed holds a profiled generated ship within the writer's reach of a
//! living station that offers Trade or Jobs (`NOVA_VISIT_SCAN=1` prints that
//! census for a seed and ends the run, to scan seeds; `NOVA_VISIT_SEED`
//! overrides the pinned seed).
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a profiled ship binds a visit to a living station` | within the bounded play time a visit is pending whose ship carries a trade profile and whose station is a live, living one that offers Trade or Jobs |
//! | 2 | `outcome: a docked visit's receipt conserves credits and goods` | the visit is Docked with a receipt, its ship stands docked at its station, and across the capture frame the ship's and the station's balances and holds moved by exactly the receipt's lines, nothing created or lost |
//! | 3 | `outcome: a docked visit posts its jobs under visit ids` | every receipt job is a valid [`JobSourceType::Visit`] job of this visit and index, posted at this station, and stands in [`JobPostings`] stamped at the capture |
//! | 4 | `outcome: a visit ends with its ship released` | after the dwell the visit departs, then is no longer pending; it was let go between its capture and its end, the ship is undocked and holds no order of the visit, its next visit is later than the end, and the posted jobs still stand |
//! | 5 | `outcome: pending visits stay within the living stations` | on every sampled frame the pending visits number at most the live living stations, and at least one visit was pending |
//! | 6 | `outcome: a docked player accepts a visit job from the board` | the player docks at the station under a Dock order, the Jobs pane lists each open visit posting the hull can take with no refresh `X` (a slot row keeps its `X`), and an Accept click makes [`Jobs`] hold the job while [`JobPostings`] keeps its visit posting |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=7200 \
//!   RUST_LOG=info,nova_world_base=info \
//!   cargo run --example system_station_visits --features debug
//! # look for: `station visits: PASS the visit bound`,
//! #           `station visits: PASS the visit docked`,
//! #           `station visits: PASS the visit ended`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! The frame-time window opens when the tracked visit binds and is refused if
//! the visit leaves its approach first: it measures the streamed window,
//! station traffic included, while one Dock order flies. The walk holds
//! `Time<Virtual>`'s clamp at 1 s, not the default 0.25 s;
//! `NOVA_PROBE_MAX_DELTA` pins it for a comparison. `probe run
//! system_station_visits --release --repeat 5 --timeout 900` gives a repeat
//! set on a hardware GPU; on lavapipe the approach can end inside the window.
//! The clean pass's deadline is the timeout less 30 s; at seed 13 the visit
//! ends about 370 s into play, before the player docks and accepts, so the
//! default timeout ends the walk on its backstop. Let the profiled pass finish: `checks.json` is written
//! only at the end. A matched reference is the same seed on the same build
//! with only the compared change.
//!
//! `NOVA_CAPTURE=1` also writes `station-visits-docked.png` of the docked
//! visitor, `station-visits-released.png` of the same station after the
//! visitor left and `station-visits-jobs-pane.png` of the Jobs pane with the
//! player docked.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_visits")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed, then play time runs until a generated ship with a trade \
             profile visits a living station: it binds, docks, trades and posts jobs once, \
             dwells, is released and flies clear. Reads the world only. Its frame-time window runs \
             while the visit approaches",
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
        // The frame-time claim is the cost of the window while the tracked
        // visit approaches: every streamed hull, the station traffic among
        // them, flies, and the visitor's Dock order runs. It opens on the bind
        // and is refused once the visit docks or ends.
        app.add_plugins(
            nova_probe::NovaProbePlugin::default()
                .ready_frametime(walk::the_visit_approaches)
                .live_frametime(walk::the_visit_approaches),
        );
        app.add_plugins(walk::VisitWatchPlugin);
        app.add_plugins(walk::script());
        app.add_systems(
            Startup,
            (
                nova_protocol::nova_debug::harness::force_capture_resolution,
                nova_protocol::nova_debug::harness::hide_dev_overlays,
            ),
        );
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::{collections::BTreeMap, sync::Arc};

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use bevy::{ecs::system::RunSystemOnce, ui::InteractionDisabled};
    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
        nova_interface::{
            jobs::prelude::{JOBS_PANE, JOB_ACCEPT_PREFIX},
            pane::InterfacePaneType,
        },
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{desired_sectors, SectorRoot, WorldConfig};

    use super::*;

    /// The seed the range plays unless `NOVA_VISIT_SEED` names another. Its
    /// window around the docked start holds one living station that offers
    /// Trade and Jobs, and two `base/ore_hauler` ships within 2 km of it: the
    /// first to come due binds, the second finds the station taken. (Scanned
    /// with `NOVA_VISIT_SCAN=1`: seeds 3, 6, 15, 16, 17 and 10 hold no
    /// profiled ship in reach, seed 9 only a `base/idle_traffic` ship whose
    /// profile trades and posts nothing.)
    const VISIT_SEED: u32 = 13;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Visits World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    /// The start role the walk picks: the transport's light freighter.
    const START_ROLE_BUTTON: &str = "Start Role Button: transport";

    /// The frames the capture path writes.
    const DOCKED_SHOT: &str = "station-visits-docked.png";
    const RELEASED_SHOT: &str = "station-visits-released.png";
    const JOBS_SHOT: &str = "station-visits-jobs-pane.png";

    /// The interface tab that opens the Jobs pane.
    const JOBS_TAB: &str = "InterfaceTabJobs";
    /// A board row's `X` prefix. `nova_interface` keeps it crate-private; a
    /// rename there fails the control assert below, not silently.
    const JOB_REFRESH_PREFIX: &str = "JobRefresh-";
    /// A board row's name prefix: `JobRowPosted-<row>`.
    const JOB_ROW_PREFIX: &str = "JobRowPosted-";

    /// The writer's reach, centre to centre, as a census filter. The census
    /// only reports; the writer owns the real rule.
    const CENSUS_REACH: Meters = Meters(20_000.0);

    /// The play second by which a first visit must be pending: the writer's
    /// longest first wait (300 s) and slack for a ship that is out of reach
    /// at its due time.
    const BIND_BY_S: f64 = 420.0;
    /// Play seconds a bound visit gets to dock, from its bind.
    const DOCK_WITHIN_S: f64 = 900.0;
    /// Play seconds a docked visit gets to end, from its capture: the
    /// writer's 60 s dwell, its 300 s departure limit and slack for the
    /// undock.
    const END_WITHIN_S: f64 = 420.0;

    /// How far aside of the station the player rests while the visit runs.
    const REST_ASIDE: Meters = Meters(3_000.0);

    /// How far to the side of the visitor-and-station midpoint the shot
    /// stands.
    const SHOT_STANDOFF: Meters = Meters(700.0);
    /// How far from its station the visitor must stand for the released shot.
    const CLEARED_BY: Meters = Meters(400.0);

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Real seconds the play-time waits get: a lavapipe frame costs seconds
    /// and the virtual clock advances at most [`WALK_CLOCK_SECS`] per frame.
    const PLAY_WAIT_SECS: f32 = 6_000.0;
    /// Frames a pane gets to draw.
    const SETTLE_FRAMES: u32 = 6;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;

    /// One watched balance-and-hold reading of the visitor and its station.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Ledger {
        ship_cr: u32,
        station_cr: u32,
        /// For each item the profile trades: (the ship's count, the
        /// station's count).
        goods: BTreeMap<ItemDesignId, (u32, u32)>,
    }

    /// What the watcher recorded and the beats read.
    #[derive(Resource, Default, Debug)]
    struct VisitWatch {
        /// The first visit seen pending: the one the walk asserts.
        tracked: Option<StationVisitId>,
        /// The tracked visit as first seen pending: the bind, latched, so a
        /// Dock order that fails a frame later cannot erase it.
        bound: Option<StationVisit>,
        /// The tracked visit's last reading while it was approaching.
        before: Option<Ledger>,
        /// The first reading with the tracked visit docked.
        after: Option<Ledger>,
        /// The jobs the tracked visit's receipt lists.
        posted: Vec<JobId>,
        /// The Accept button the walk clicks and the job behind it.
        accept: Option<(String, JobId)>,
        /// The play seconds the tracked visit docked, was let go and ended
        /// at.
        docked_at_s: Option<f64>,
        released_at_s: Option<f64>,
        ended_at_s: Option<f64>,
        /// The most pending visits on one frame, and the live living
        /// stations on that frame.
        max_pending: usize,
        live_at_max: usize,
        /// Frames on which more visits were pending than living stations
        /// stood, and how many frames were sampled.
        over_bound_frames: u32,
        samples: u32,
    }

    pub(super) struct VisitWatchPlugin;

    impl Plugin for VisitWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<VisitWatch>();
            app.add_systems(Update, (hold_the_walk_clock, watch_visits));
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

    /// Read the visitor's and the station's balances and the holds of the
    /// items the visit's profile trades.
    fn reading(
        visit: &StationVisit,
        profiles: &GameTradeProfiles,
        bodies: &Query<(&GeneratedBodyId, &ShipCredits, &ShipInventory)>,
    ) -> Option<Ledger> {
        let mut ship = None;
        let mut station = None;
        for (id, credits, hold) in bodies {
            if *id == visit.id.ship {
                ship = Some((credits.0, hold));
            } else if *id == visit.station {
                station = Some((credits.0, hold));
            }
        }
        let (ship_cr, ship_hold) = ship?;
        let (station_cr, station_hold) = station?;
        let profile = profiles.get(&visit.profile)?;
        let goods = profile
            .trades
            .iter()
            .map(|intent| {
                (
                    intent.item.clone(),
                    (
                        ship_hold.count(&intent.item),
                        station_hold.count(&intent.item),
                    ),
                )
            })
            .collect();
        Some(Ledger {
            ship_cr,
            station_cr,
            goods,
        })
    }

    /// Sample the ledger every frame. Reads only.
    #[expect(
        clippy::type_complexity,
        reason = "the living-station filter repeats the writer's markers"
    )]
    fn watch_visits(
        mut watch: ResMut<VisitWatch>,
        visits: Option<Res<StationVisits>>,
        clock: Option<Res<WorldClock>>,
        profiles: Option<Res<GameTradeProfiles>>,
        bodies: Query<(&GeneratedBodyId, &ShipCredits, &ShipInventory)>,
        hulls: Query<
            (
                Entity,
                Option<&GeneratedBodyId>,
                &GlobalTransform,
                Has<PlayerSpaceshipMarker>,
            ),
            With<SpaceshipRootMarker>,
        >,
        living: Query<
            (),
            (
                With<StationMarker>,
                With<ShipInventory>,
                Without<DerelictShipMarker>,
                Without<NeutralizedMarker>,
                Without<OwnedStationMarker>,
            ),
        >,
    ) {
        let (Some(visits), Some(clock), Some(profiles)) = (visits, clock, profiles) else {
            return;
        };
        let now = clock.play_seconds;
        let pending = visits.pending_visits().count();
        let stations = living.iter().count();
        watch.samples += 1;
        if pending > stations {
            watch.over_bound_frames += 1;
        }
        if pending > watch.max_pending {
            watch.max_pending = pending;
            watch.live_at_max = stations;
        }
        if watch.bound.is_none() {
            if let Some(visit) = visits.pending_visits().next() {
                watch.tracked = Some(visit.id.clone());
                watch.bound = Some(visit.clone());
                // The hull entities near the station, so a Dock failure that
                // names a blocking body by entity can be matched to a hull.
                let hub = hulls
                    .iter()
                    .find(|(_, id, ..)| id.is_some_and(|id| *id == visit.station))
                    .map(|(.., at, _)| at.translation());
                if let Some(hub) = hub {
                    for (entity, id, at, is_player) in &hulls {
                        let metres = (at.translation() - hub).length() * 10.0;
                        if metres <= 6_000.0 {
                            info!(
                                "station visits: hull {entity:?} {} stands {metres:.0} m from \
                                 the station{}",
                                id.map_or_else(|| "(no id)".to_string(), ToString::to_string),
                                if is_player { " (the player)" } else { "" }
                            );
                        }
                    }
                }
            }
        }
        let Some(tracked) = watch.tracked.clone() else {
            return;
        };
        match visits.pending(&tracked.ship) {
            Some(visit) if visit.id == tracked => match &visit.phase {
                StationVisitPhaseType::Approaching => {
                    if let Some(ledger) = reading(visit, &profiles, &bodies) {
                        watch.before = Some(ledger);
                    }
                }
                StationVisitPhaseType::Docked { docked_at_s, .. } => {
                    if watch.after.is_none() {
                        watch.after = reading(visit, &profiles, &bodies);
                        watch.docked_at_s = Some(*docked_at_s);
                    }
                }
                StationVisitPhaseType::Departing { released_at_s, .. } => {
                    watch.released_at_s = Some(*released_at_s);
                }
                StationVisitPhaseType::Withdrawing { failed_at_s } => {
                    panic!(
                        "station visits: FAIL the visit '{}' withdrew at {failed_at_s:.1} s of \
                         play without docking",
                        visit.id
                    );
                }
            },
            _ => {
                if watch.ended_at_s.is_none() {
                    watch.ended_at_s = Some(now);
                }
            }
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
        the_player(world).expect("station visits: exactly one player ship")
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

    /// The entity wearing `id`, if one does.
    fn body(world: &mut World, id: &GeneratedBodyId) -> Option<Entity> {
        world
            .query::<(Entity, &GeneratedBodyId)>()
            .iter(world)
            .find(|(_, worn)| *worn == id)
            .map(|(entity, _)| entity)
    }

    fn play_seconds(world: &World) -> f64 {
        world.resource::<WorldClock>().play_seconds
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

    fn live_cells(world: &World) -> std::collections::BTreeSet<SectorCoord> {
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

    fn set_field_text(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("station visits: no field named '{name}'"));
        value.0 = text.to_string();
    }

    fn seed() -> u32 {
        match std::env::var("NOVA_VISIT_SEED") {
            Ok(seed) => seed
                .parse()
                .unwrap_or_else(|error| panic!("NOVA_VISIT_SEED '{seed}' is not a u32: {error}")),
            Err(_) => VISIT_SEED,
        }
    }

    fn scanning() -> bool {
        std::env::var_os("NOVA_VISIT_SCAN").is_some()
    }

    /// A live station that offers Trade or Jobs, with where it stands.
    #[expect(
        clippy::type_complexity,
        reason = "the living-station filter repeats the writer's markers"
    )]
    fn offering_stations(world: &mut World) -> Vec<(GeneratedBodyId, Meters3, bool, bool)> {
        world
            .query_filtered::<(&GeneratedBodyId, &GlobalTransform, Option<&StationServices>), (
                With<StationMarker>,
                With<ShipInventory>,
                Without<DerelictShipMarker>,
                Without<NeutralizedMarker>,
                Without<OwnedStationMarker>,
            )>()
            .iter(world)
            .filter_map(|(id, at, services)| {
                let trades = station_offers(services, false, StationServiceType::Trade);
                let posts = station_offers(services, false, StationServiceType::Jobs);
                (trades || posts).then(|| {
                    (
                        id.clone(),
                        Meters3::from_engine(at.translation()),
                        trades,
                        posts,
                    )
                })
            })
            .collect()
    }

    /// Log which profiled ships stand near which offering station on this
    /// seed. Fails loudly when no profiled ship has an offering station in
    /// reach, so a seed that cannot prove the claim says so at once.
    #[expect(
        clippy::type_complexity,
        reason = "the ship filter repeats the writer's markers"
    )]
    fn census(world: &mut World) {
        let seed = seed();
        let stations = offering_stations(world);
        let ships: Vec<(GeneratedBodyId, TradeProfileDesignId, Meters3)> = world
            .query_filtered::<(&GeneratedBodyId, &GlobalTransform, &TraderMarket), (
                With<SpaceshipRootMarker>,
                With<AISpaceshipMarker>,
                With<ShipInventory>,
                Without<StationMarker>,
                Without<NeutralizedMarker>,
                Without<DerelictShipMarker>,
                Without<PlayerSpaceshipMarker>,
                Without<PlayerFleetMarker>,
                Without<OwnedShipMarker>,
            )>()
            .iter(world)
            .filter_map(|(id, at, market)| {
                Some((
                    id.clone(),
                    market.profile.clone()?,
                    Meters3::from_engine(at.translation()),
                ))
            })
            .collect();
        info!(
            "station visits: census seed {seed}: {} offering station(s), {} profiled ship(s)",
            stations.len(),
            ships.len()
        );
        for (id, trades, posts) in stations.iter().map(|(id, _, t, p)| (id, t, p)) {
            info!("station visits: census station '{id}' trades={trades} posts={posts}");
        }
        // Why the dock rule would refuse each ship at each station right now.
        let refusals: Vec<(GeneratedBodyId, GeneratedBodyId, Option<DockRefusalType>)> = world
            .run_system_once(
                |admission: DockAdmission,
                 ships: Query<(Entity, &GeneratedBodyId), With<TraderMarket>>,
                 stations: Query<(Entity, &GeneratedBodyId), With<StationMarker>>| {
                    let mut out = Vec::new();
                    for (ship, ship_id) in &ships {
                        for (station, station_id) in &stations {
                            out.push((
                                ship_id.clone(),
                                station_id.clone(),
                                admission.refusal(ship, station),
                            ));
                        }
                    }
                    out
                },
            )
            .expect("station visits: the admission census runs");
        for (ship, station, refusal) in &refusals {
            if refusal.is_some() {
                info!("station visits: census refusal '{ship}' -> '{station}': {refusal:?}");
            }
        }
        let catalog = world.resource::<GameTradeProfiles>();
        let mut reachable = 0;
        for (id, profile, at) in &ships {
            // A profile with no trade and no job (base/idle_traffic) visits
            // as an empty receipt, which proves nothing about the transfers.
            let works = catalog
                .get(profile)
                .is_some_and(|design| !design.trades.is_empty() || !design.jobs.is_empty());
            let nearest = stations
                .iter()
                .map(|(station, position, ..)| (station, at.distance(*position).get()))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            let in_reach = nearest.is_some_and(|(_, distance)| distance <= CENSUS_REACH.get());
            reachable += usize::from(in_reach && works);
            info!(
                "station visits: census ship '{id}' profile '{profile}' nearest {:?} in_reach={in_reach} works={works}",
                nearest.map(|(station, distance)| format!("{station} {distance:.0} m"))
            );
        }
        info!("station visits: census seed {seed}: {reachable} profiled ship(s) in reach that trade or post");
        assert!(
            reachable > 0,
            "station visits: seed {seed} holds no ship in reach whose profile trades or posts within {} m of an offering \
             station; pick another seed (NOVA_VISIT_SCAN=1 NOVA_VISIT_SEED=<n>)",
            CENSUS_REACH.get()
        );
    }

    /// Put the player at rest [`REST_ASIDE`] from the offering station, in the
    /// station's own frame, so its hull stands in no Dock corridor (a first
    /// run parked at the collar and the writer's Dock order failed on `no safe
    /// DOCK corridor`).
    ///
    /// A station that orbits the well nearest to it: the player rides the
    /// station's own orbit, a [`REST_ASIDE`] chord behind it, with the
    /// station's velocity turned through the same angle about the well. It
    /// keeps that chord and the orbit's radius, so the planet the station
    /// circles never reaches it. Left at world rest it fell into the planet
    /// (seed 26); at the station's velocity off the orbit it climbed away on
    /// an eccentric orbit and the observer retired the station's sector
    /// (seed 29). Otherwise, or on an orbit too small for the chord, the
    /// player stands beside the line from the nearest profiled ship that
    /// works, at the station's velocity.
    fn rest_aside_of_the_station(world: &mut World) {
        // `NOVA_VISIT_REST=collar` leaves the player where the release set it
        // (at the station's collar): a repro of the first run's Dock failure.
        if std::env::var("NOVA_VISIT_REST").is_ok_and(|rest| rest == "collar") {
            let player = player(world);
            info!("station visits: the player {player:?} stays at the collar");
            return;
        }
        let stations = offering_stations(world);
        let (station_id, hub, ..) = stations
            .first()
            .cloned()
            .expect("station visits: an offering station stands");
        let ship = world
            .query_filtered::<(&GlobalTransform, &TraderMarket), With<AISpaceshipMarker>>()
            .iter(world)
            .filter(|(_, market)| market.profile.is_some())
            .map(|(at, _)| Meters3::from_engine(at.translation()))
            .min_by(|a, b| a.distance(hub).get().total_cmp(&b.distance(hub).get()))
            .expect("station visits: a profiled ship stands");
        let velocity = world
            .query_filtered::<(&GeneratedBodyId, Option<&LinearVelocity>), With<StationMarker>>()
            .iter(world)
            .find(|(id, _)| **id == station_id)
            .and_then(|(_, velocity)| velocity.map(|velocity| velocity.0))
            .unwrap_or(Vec3::ZERO);
        let well = world
            .query_filtered::<&GlobalTransform, With<GravityWell>>()
            .iter(world)
            .map(|at| Meters3::from_engine(at.translation()))
            .min_by(|a, b| a.distance(hub).get().total_cmp(&b.distance(hub).get()));
        let behind = well.and_then(|well| {
            let radius = hub.get() - well.get();
            let normal = radius.cross(velocity).try_normalize()?;
            let half_chord = REST_ASIDE.get() / (2.0 * radius.length());
            (half_chord < 1.0).then(|| {
                let turn = Quat::from_axis_angle(normal, -2.0 * half_chord.asin());
                (Meters3(well.get() + turn * radius), turn * velocity)
            })
        });
        let (rule, stand, velocity) = match behind {
            Some((stand, velocity)) => ("behind on the station's orbit", stand, velocity),
            None => {
                let along = (ship.get() - hub.get()).normalize_or_zero();
                let mut side = along.cross(Vec3::Y);
                if side.length_squared() < 1e-6 {
                    side = along.cross(Vec3::X);
                }
                (
                    "beside the line",
                    Meters3(hub.get() + side.normalize() * REST_ASIDE.get()),
                    velocity,
                )
            }
        };
        let player = player(world);
        world.entity_mut(player).insert((
            Position(stand.to_engine()),
            LinearVelocity(velocity),
            AngularVelocity::ZERO,
        ));
        info!(
            "station visits: the player {player:?} rests at {stand:?}, {} m aside of \
             '{station_id}' ({rule}), moving at {:?} m/s",
            REST_ASIDE.get(),
            Meters3::from_engine(velocity)
        );
    }

    /// Advance when a visit is pending or the bind window has passed.
    fn a_visit_is_pending_or_late() -> Arc<Predicate> {
        when(|world| {
            world.resource::<VisitWatch>().bound.is_some() || play_seconds(world) > BIND_BY_S
        })
    }

    /// The tracked visit, while it stands.
    fn tracked_visit(world: &World) -> Option<StationVisit> {
        let tracked = world.resource::<VisitWatch>().tracked.clone()?;
        world
            .resource::<StationVisits>()
            .pending(&tracked.ship)
            .filter(|visit| visit.id == tracked)
            .cloned()
    }

    /// Whether the tracked visit is approaching its station while play time
    /// runs: the frame-time window.
    pub(super) fn the_visit_approaches(world: &World) -> bool {
        let running = world
            .get_resource::<Time<Virtual>>()
            .is_some_and(|time| !time.is_paused() && time.relative_speed() > 0.0);
        let playing = world
            .get_resource::<State<GameStates>>()
            .is_some_and(|state| *state.get() == GameStates::Playing);
        running
            && playing
            && world.get_resource::<VisitWatch>().is_some()
            && world.get_resource::<StationVisits>().is_some()
            && tracked_visit(world)
                .is_some_and(|visit| matches!(visit.phase, StationVisitPhaseType::Approaching))
    }

    /// Advance when the tracked visit is docked, ended, or past its window.
    fn the_visit_docked_or_gone() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<VisitWatch>();
            if watch.tracked.is_none() {
                return true;
            }
            let docked = watch.after.is_some() && watch.docked_at_s.is_some();
            let gone = watch.ended_at_s.is_some();
            let late = tracked_visit(world)
                .is_some_and(|visit| play_seconds(world) > visit.bound_at_s + DOCK_WITHIN_S);
            docked || gone || late
        })
    }

    /// Advance when the tracked visit has ended, or its window passed.
    fn the_visit_ended_or_late() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<VisitWatch>();
            if watch.ended_at_s.is_some() {
                // The ship also lets go of the station.
                let Some(tracked) = watch.tracked.clone() else {
                    return true;
                };
                return find_ship(world, &tracked.ship)
                    .is_none_or(|ship| world.get::<DockedShip>(ship).is_none());
            }
            watch
                .docked_at_s
                .is_some_and(|docked| play_seconds(world) > docked + END_WITHIN_S)
        })
    }

    /// Advance when the visitor stands [`CLEARED_BY`] from the station it
    /// left, or [`END_WITHIN_S`] of play have passed since the end.
    fn the_visitor_cleared() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<VisitWatch>();
            let (Some(visit), Some(ended)) = (watch.bound.clone(), watch.ended_at_s) else {
                return false;
            };
            if play_seconds(world) > ended + END_WITHIN_S {
                return true;
            }
            let at = |id: &GeneratedBodyId| {
                find_ship(world, id)
                    .and_then(|entity| world.get::<GlobalTransform>(entity))
                    .map(|at| Meters3::from_engine(at.translation()))
            };
            match (at(&visit.id.ship), at(&visit.station)) {
                (Some(ship), Some(station)) => ship.distance(station).get() >= CLEARED_BY.get(),
                _ => true,
            }
        })
    }

    fn find_ship(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| *worn == id)
            .map(|(entity, _)| entity)
    }

    /// Claim 1: a profiled ship bound a visit to a living station.
    fn assert_the_bind(world: &mut World) {
        let now = play_seconds(world);
        let visit = world
            .resource::<VisitWatch>()
            .bound
            .clone()
            .unwrap_or_else(|| {
                panic!(
                    "station visits: no visit bound by {now:.0} s of play (bound by \
                     {BIND_BY_S:.0} s)"
                )
            });
        assert!(
            visit.bound_at_s <= BIND_BY_S,
            "station visits: the visit '{}' bound at {:.1} s, past the bounded {BIND_BY_S:.0} s",
            visit.id,
            visit.bound_at_s
        );
        let ship = body(world, &visit.id.ship)
            .unwrap_or_else(|| panic!("station visits: the visitor '{}' stands", visit.id.ship));
        let market = world
            .get::<TraderMarket>(ship)
            .unwrap_or_else(|| panic!("station visits: '{}' carries a market", visit.id.ship));
        assert_eq!(
            market.profile.as_ref(),
            Some(&visit.profile),
            "station visits: the visitor carries the profile its visit runs"
        );
        let offering = offering_stations(world);
        let (_, position, trades, posts) = offering
            .iter()
            .find(|(id, ..)| *id == visit.station)
            .unwrap_or_else(|| {
                panic!(
                    "station visits: '{}' is no live station that offers Trade or Jobs: {offering:?}",
                    visit.station
                )
            });
        let at = Meters3::from_engine(
            world
                .get::<GlobalTransform>(ship)
                .expect("station visits: the visitor has a transform")
                .translation(),
        );
        let distance = at.distance(*position).get();
        nova_probe::probe_marker(
            world,
            "outcome: a profiled ship binds a visit to a living station",
            serde_json::json!({
                "visit": visit.id.to_string(),
                "ship": visit.id.ship.to_string(),
                "station": visit.station.to_string(),
                "profile": visit.profile.to_string(),
                "bound_at_s": visit.bound_at_s,
                "station_trades": trades,
                "station_posts": posts,
                "distance_m_now": distance,
            }),
        );
        info!(
            "station visits: PASS the visit bound: '{}' of '{}' to '{}' under '{}' at {:.1} s of \
             play, {distance:.0} m off now",
            visit.id, visit.id.ship, visit.station, visit.profile, visit.bound_at_s
        );
    }

    /// Claims 2 and 3: the dock, its receipt, its balances and its postings.
    fn assert_the_dock(world: &mut World) {
        let watch = world.resource::<VisitWatch>();
        let tracked = watch
            .tracked
            .clone()
            .expect("station visits: a visit bound");
        let (before, after, docked_at_s) =
            (watch.before.clone(), watch.after.clone(), watch.docked_at_s);
        let visit = tracked_visit(world).unwrap_or_else(|| {
            panic!(
                "station visits: the visit '{tracked}' ended or never docked by {:.0} s of play \
                 (ended at {:?}, docked at {docked_at_s:?})",
                play_seconds(world),
                world.resource::<VisitWatch>().ended_at_s
            )
        });
        let StationVisitPhaseType::Docked {
            docked_at_s: stamp,
            receipt,
        } = visit.phase.clone()
        else {
            panic!(
                "station visits: the visit '{}' is still approaching",
                visit.id
            );
        };
        assert_eq!(
            Some(stamp),
            docked_at_s,
            "station visits: the watcher read the capture stamp"
        );
        let ship = body(world, &visit.id.ship).expect("station visits: the visitor stands");
        let station =
            body(world, &visit.station).expect("station visits: the visited station stands");
        let connection = world
            .get::<DockedShip>(ship)
            .unwrap_or_else(|| panic!("station visits: '{}' stands docked", visit.id.ship))
            .connection;
        assert!(
            world
                .get::<DockingConnection>(connection)
                .expect("station visits: a docked ship's connection stands")
                .joins(station),
            "station visits: '{}' is docked at '{}'",
            visit.id.ship,
            visit.station
        );

        // Claim 2: what moved is what the receipt lists.
        let before = before.expect("station visits: an approaching reading was taken");
        let after = after.expect("station visits: a docked reading was taken");
        let mut ship_cr = i64::from(before.ship_cr);
        let mut station_cr = i64::from(before.station_cr);
        let mut goods: BTreeMap<ItemDesignId, (i64, i64)> = before
            .goods
            .iter()
            .map(|(item, (ship, station))| (item.clone(), (i64::from(*ship), i64::from(*station))))
            .collect();
        let mut moved = 0;
        for line in &receipt.trades {
            let (count, price) = (i64::from(line.count), i64::from(line.price_cr));
            let held = goods.entry(line.item.clone()).or_default();
            match line.side {
                ItemTradeType::Buy => {
                    ship_cr -= price;
                    station_cr += price;
                    held.0 += count;
                    held.1 -= count;
                }
                ItemTradeType::Sell => {
                    ship_cr += price;
                    station_cr -= price;
                    held.0 -= count;
                    held.1 += count;
                }
            }
            moved += usize::from(line.count > 0);
        }
        assert_eq!(
            (ship_cr, station_cr),
            (i64::from(after.ship_cr), i64::from(after.station_cr)),
            "station visits: the balances moved by the receipt lines {:?}: before {before:?}, \
             after {after:?}",
            receipt.trades
        );
        assert_eq!(
            before.ship_cr + before.station_cr,
            after.ship_cr + after.station_cr,
            "station visits: the visit created or lost credits"
        );
        for (item, (ship_count, station_count)) in &goods {
            let (now_ship, now_station) = after.goods[item];
            assert_eq!(
                (*ship_count, *station_count),
                (i64::from(now_ship), i64::from(now_station)),
                "station visits: the holds of {item} moved by the receipt lines"
            );
        }
        assert!(
            !receipt.trades.is_empty() || !receipt.jobs.is_empty(),
            "station visits: the visit '{}' neither traded nor posted: {receipt:?}",
            visit.id
        );
        nova_probe::probe_marker(
            world,
            "outcome: a docked visit's receipt conserves credits and goods",
            serde_json::json!({
                "visit": visit.id.to_string(),
                "station": visit.station.to_string(),
                "docked_at_s": stamp,
                "trade_lines": receipt.trades.iter().map(|line| format!(
                    "{:?} {} x{} for {} cr", line.side, line.item, line.count, line.price_cr
                )).collect::<Vec<_>>(),
                "lines_that_moved": moved,
                "ship_cr": [before.ship_cr, after.ship_cr],
                "station_cr": [before.station_cr, after.station_cr],
            }),
        );
        info!(
            "station visits: PASS the visit docked: '{}' at '{}' at {stamp:.1} s of play, {} \
             trade line(s) ({moved} moved), ship {} -> {} cr, station {} -> {} cr",
            visit.id,
            visit.station,
            receipt.trades.len(),
            before.ship_cr,
            after.ship_cr,
            before.station_cr,
            after.station_cr
        );

        // Claim 3: the posted jobs are visit jobs of this visit.
        let postings = world.resource::<JobPostings>().clone();
        for (position, id) in receipt.jobs.iter().enumerate() {
            assert_eq!(
                id.station, visit.station,
                "station visits: '{id}' is posted at the visited station"
            );
            let JobSourceType::Visit {
                visit: posted_by,
                index,
            } = &id.source
            else {
                panic!("station visits: '{id}' is no visit job");
            };
            assert_eq!(
                posted_by, &visit.id,
                "station visits: '{id}' was posted by this visit"
            );
            assert_eq!(
                *index as usize, position,
                "station visits: '{id}' holds its profile list place"
            );
            let posting = postings
                .visit_posting(id)
                .unwrap_or_else(|| panic!("station visits: JobPostings holds no visit job '{id}'"));
            posting
                .offer
                .validate()
                .unwrap_or_else(|error| panic!("station visits: '{id}' is invalid: {error}"));
            assert_eq!(
                posting.posted_at, stamp,
                "station visits: '{id}' is stamped at the capture"
            );
        }
        world.resource_mut::<VisitWatch>().posted = receipt.jobs.clone();
        nova_probe::probe_marker(
            world,
            "outcome: a docked visit posts its jobs under visit ids",
            serde_json::json!({
                "visit": visit.id.to_string(),
                "jobs": receipt.jobs.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "station_visit_postings": postings.visit_postings(&visit.station).len(),
            }),
        );
        info!(
            "station visits: PASS the visit posted {} job(s) {:?}",
            receipt.jobs.len(),
            receipt
                .jobs
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        );
    }

    /// Release the player from the starting station and stand it off the
    /// visitor, looking at it, at rest. Only the frame needs it: the claims
    /// are asserted before this runs. If the visit has ended there is no
    /// visitor to frame, and the step does nothing.
    fn stand_off_the_visitor(world: &mut World) {
        let Some(visit) = tracked_visit(world) else {
            return;
        };
        let (Some(ship), Some(station)) =
            (body(world, &visit.id.ship), body(world, &visit.station))
        else {
            return;
        };
        let at = |world: &World, entity: Entity| {
            Meters3::from_engine(
                world
                    .get::<GlobalTransform>(entity)
                    .expect("station visits: a hull has a transform")
                    .translation(),
            )
        };
        let (visitor, hub) = (at(world, ship), at(world, station));
        // Beside the visitor, off the line from its port, so the player's
        // hull stands in no Dock corridor, and framing the visitor and its
        // station together.
        let away = (visitor.get() - hub.get()).normalize_or_zero();
        let mut side = away.cross(Vec3::Y);
        if side.length_squared() < 1e-6 {
            side = away.cross(Vec3::X);
        }
        let middle = (visitor.get() + hub.get()) * 0.5;
        let stand = Meters3(middle + side.normalize() * SHOT_STANDOFF.get());
        let looking = Transform::from_translation(stand.get()).looking_at(middle, Vec3::Y);
        let player = player(world);
        world.entity_mut(player).insert((
            Position(stand.to_engine()),
            Rotation(looking.rotation),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("station visits: the player stands off the visitor at {stand:?}");
    }

    /// Pin the scenario camera on the tracked visit's station, from the side
    /// away from the player's stand-off, far enough out for the whole station
    /// and the visitor while it is docked. The chase rig frames the player's
    /// own hull, which hid the station in every shot at an orbiting station.
    fn frame_the_station(world: &mut World) {
        let Some(visit) = world.resource::<VisitWatch>().bound.clone() else {
            return;
        };
        let Some(station) = body(world, &visit.station) else {
            return;
        };
        let at = |world: &World, entity: Entity| {
            world
                .get::<GlobalTransform>(entity)
                .map(|transform| transform.translation())
        };
        let Some(hub) = at(world, station) else {
            return;
        };
        let visitor = body(world, &visit.id.ship).and_then(|ship| at(world, ship));
        let envelope = world
            .get::<HullEnvelopeRadius>(station)
            .map_or(0.0, |radius| **radius);
        let player = at(world, player(world)).unwrap_or(hub + Vec3::X);
        // Engine units: the station's reach, out to a visitor that still
        // stands near it.
        let reach = visitor
            .map_or(0.0, |visitor| visitor.distance(hub))
            .min(4.0 * envelope)
            .max(envelope);
        let middle = visitor.filter(|visitor| visitor.distance(hub) <= 4.0 * envelope);
        let middle = middle.map_or(hub, |visitor| (visitor + hub) * 0.5);
        let away = (middle - player).normalize_or(Vec3::Z);
        let eye = middle + (away + Vec3::Y * 0.3).normalize() * (3.0 * reach).max(40.0);
        let camera = world
            .query_filtered::<Entity, With<ScenarioCameraMarker>>()
            .iter(world)
            .next();
        let Some(camera) = camera else {
            warn!("station visits: no scenario camera to frame the station");
            return;
        };
        world.entity_mut(camera).insert(ScriptedCameraPose {
            position: Meters3::from_engine(eye),
            look_at: Meters3::from_engine(middle),
        });
        info!(
            "station visits: the camera frames '{}' from {} m, visitor {}",
            visit.station,
            Meters::from_engine(eye.distance(middle)).get(),
            if visitor.is_some() {
                "in frame"
            } else {
                "gone"
            }
        );
    }

    /// Hand the camera back to the chase rig, which never stopped solving.
    fn release_the_frame(world: &mut World) {
        let cameras: Vec<Entity> = world
            .query_filtered::<Entity, With<ScriptedCameraPose>>()
            .iter(world)
            .collect();
        for camera in cameras {
            world.entity_mut(camera).remove::<ScriptedCameraPose>();
        }
    }

    /// Claims 4 and 5: the visit ended, the ship is free, the bound held.
    fn assert_the_end(world: &mut World) {
        let watch = world.resource::<VisitWatch>();
        let tracked = watch
            .tracked
            .clone()
            .expect("station visits: a visit bound");
        let (docked_at_s, ended_at_s) = (
            watch.docked_at_s.expect("station visits: the visit docked"),
            watch.ended_at_s,
        );
        let ended_at_s = ended_at_s.unwrap_or_else(|| {
            panic!(
                "station visits: the visit '{tracked}' docked at {docked_at_s:.1} s and was still \
                 pending at {:.1} s of play",
                play_seconds(world)
            )
        });
        let released_at_s = watch.released_at_s.unwrap_or_else(|| {
            panic!("station visits: the visit '{tracked}' ended with no departure seen")
        });
        assert!(
            docked_at_s <= released_at_s && released_at_s <= ended_at_s,
            "station visits: '{tracked}' docked at {docked_at_s:.1} s, was let go at \
             {released_at_s:.1} s and ended by {ended_at_s:.1} s"
        );
        let visits = world.resource::<StationVisits>().clone();
        assert!(
            visits.pending(&tracked.ship).is_none(),
            "station visits: '{tracked}' is no longer pending"
        );
        let next = visits
            .next_visit_at(&tracked.ship)
            .expect("station visits: the ship keeps its next visit stamp");
        assert!(
            next > ended_at_s,
            "station visits: the next visit at {next:.1} s is later than the end near \
             {ended_at_s:.1} s"
        );
        if let Some(ship) = find_ship(world, &tracked.ship) {
            assert!(
                world.get::<DockedShip>(ship).is_none(),
                "station visits: '{}' is released from the station",
                tracked.ship
            );
            let keys = [tracked.to_string(), format!("{tracked}_depart")];
            assert!(
                world
                    .get::<ShipHelmOrder>(ship)
                    .is_none_or(|order| !keys.contains(&order.key)),
                "station visits: '{}' holds no order of the ended visit",
                tracked.ship
            );
        }
        let postings = world.resource::<JobPostings>();
        let still = postings
            .each_visit_posting()
            .filter(|posting| {
                matches!(
                    &posting.offer.id.source,
                    JobSourceType::Visit { visit, .. } if *visit == tracked
                )
            })
            .count();
        nova_probe::probe_marker(
            world,
            "outcome: a visit ends with its ship released",
            serde_json::json!({
                "visit": tracked.to_string(),
                "docked_at_s": docked_at_s,
                "released_at_s": released_at_s,
                "ended_seen_at_s": ended_at_s,
                "next_visit_at_s": next,
                "jobs_still_posted": still,
            }),
        );
        info!(
            "station visits: PASS the visit ended: '{tracked}' docked at {docked_at_s:.1} s, let go \
             at {released_at_s:.1} s, ended by {ended_at_s:.1} s, next visit not before {next:.1} \
             s, {still} posted job(s) stand"
        );

        let (max_pending, live_at_max, samples, over_bound_frames) = {
            let watch = world.resource::<VisitWatch>();
            (
                watch.max_pending,
                watch.live_at_max,
                watch.samples,
                watch.over_bound_frames,
            )
        };
        assert_eq!(
            over_bound_frames, 0,
            "station visits: {over_bound_frames} of {samples} frames held more pending visits \
             than living stations"
        );
        assert!(
            max_pending >= 1,
            "station visits: no visit was ever pending"
        );
        nova_probe::probe_marker(
            world,
            "outcome: pending visits stay within the living stations",
            serde_json::json!({
                "max_pending": max_pending,
                "live_stations_then": live_at_max,
                "frames_sampled": samples,
            }),
        );
        info!(
            "station visits: PASS pending visits stayed within the living stations: max pending \
             {max_pending} with {live_at_max} live station(s) then, {samples} frames sampled, \
             {over_bound_frames} over the bound"
        );
    }

    /// Whether the player is docked at the tracked visit's station.
    fn player_docked_at_the_visit_station() -> Arc<Predicate> {
        when(|world| {
            let Some(visit) = world.resource::<VisitWatch>().bound.clone() else {
                return false;
            };
            let (Some(player), Some(station)) =
                (the_player(world), find_ship(world, &visit.station))
            else {
                return false;
            };
            world.get::<DockedShip>(player).is_some_and(|docked| {
                world
                    .get::<DockingConnection>(docked.connection)
                    .is_some_and(|connection| connection.joins(station))
            })
        })
    }

    /// Give the player the Dock order the visit writer gives a visitor: the
    /// helm flies the corridor and asks for the capture the way the DOCK key
    /// does. No pose is written. The autopilot harness sends no flight input,
    /// so the order holds the helm alone.
    fn order_the_player_to_dock(world: &mut World) {
        let visit = world
            .resource::<VisitWatch>()
            .bound
            .clone()
            .expect("station visits: a visit bound");
        let station =
            body(world, &visit.station).expect("station visits: the visited station stands");
        let player = player(world);
        let mut entity = world.entity_mut(player);
        if !entity.contains::<ShipOrderReports>() {
            entity.insert(ShipOrderReports::default());
        }
        entity.insert((
            ShipHelmOrder::new(
                "station visits: the player docks".to_string(),
                ShipOrderDirective::Dock { station },
            ),
            ShipOrderHelmAuthority,
        ));
        info!(
            "station visits: the player is ordered to dock with '{}'",
            visit.station
        );
    }

    /// Every board row the Jobs pane draws, with the texts under it.
    fn posted_rows(world: &mut World) -> Vec<(usize, Vec<String>)> {
        let rows: Vec<(usize, Entity)> = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .filter_map(|(entity, name)| {
                let index = name.as_str().strip_prefix(JOB_ROW_PREFIX)?.parse().ok()?;
                Some((index, entity))
            })
            .collect();
        rows.into_iter()
            .map(|(index, row)| {
                let mut texts = Vec::new();
                let mut stack = vec![row];
                while let Some(entity) = stack.pop() {
                    let entity = world.entity(entity);
                    if let Some(text) = entity.get::<Text>() {
                        texts.push(text.0.clone());
                    }
                    if let Some(children) = entity.get::<Children>() {
                        stack.extend(children.iter());
                    }
                }
                (index, texts)
            })
            .collect()
    }

    fn node_named(world: &mut World, name: &str) -> Option<Entity> {
        world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)
    }

    /// Whether the interface shows `pane` over the paused world.
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

    /// Claim 6, first half: the board lists the open visit postings with no
    /// refresh `X`, and a slot row keeps its own. Picks the Accept to click.
    fn assert_the_board(world: &mut World) {
        let station = world
            .resource::<VisitWatch>()
            .bound
            .clone()
            .expect("station visits: a visit bound")
            .station;
        let posted = world.resource::<VisitWatch>().posted.clone();
        let items = world.resource::<GameItems>().clone();
        assert!(
            world.get_resource::<InterfacePaneType>() == Some(&InterfacePaneType::Jobs),
            "station visits: the Jobs pane is open"
        );
        assert!(
            node_named(world, JOBS_PANE).is_some(),
            "station visits: the Jobs pane '{JOBS_PANE}' is drawn"
        );
        let rows = posted_rows(world);
        let mut listed = Vec::new();
        let mut unlisted = Vec::new();
        for id in &posted {
            let open = world.resource::<JobPostings>().visit_posting(id).cloned();
            let Some(posting) = open else {
                continue;
            };
            if world.resource::<Jobs>().holds(id) {
                continue;
            }
            let title = posting.offer.title(&items);
            match rows.iter().find(|(_, texts)| texts.contains(&title)) {
                Some((row, _)) => listed.push((id.clone(), title, *row)),
                None => unlisted.push((id.clone(), title, posting.offer.kind.clone())),
            }
        }
        info!(
            "station visits: the Jobs pane draws {} board row(s); open visit postings listed \
             {listed:?}, unlisted {unlisted:?}",
            rows.len()
        );
        assert!(
            !listed.is_empty(),
            "station visits: the Jobs pane lists no open visit posting of {posted:?} at \
             '{station}': rows {rows:?}, unlisted {unlisted:?}"
        );
        let mut accept = None;
        for (id, title, row) in &listed {
            let refresh = format!("{JOB_REFRESH_PREFIX}{row}");
            assert!(
                node_named(world, &refresh).is_none(),
                "station visits: the visit row {row} '{title}' ({id}) shows a refresh button \
                 '{refresh}'"
            );
            let button = format!("{JOB_ACCEPT_PREFIX}{row}");
            let button_entity = node_named(world, &button)
                .unwrap_or_else(|| panic!("station visits: the visit row {row} shows {button}"));
            if accept.is_none()
                && !world
                    .entity(button_entity)
                    .contains::<InteractionDisabled>()
            {
                accept = Some((button, id.clone()));
            }
        }
        // Control: a slot row of the same board keeps its `X`, so the absence
        // above is the row kind, not a renamed prefix.
        let slot_refresh = rows
            .iter()
            .filter(|(row, _)| !listed.iter().any(|(.., listed_row)| listed_row == row))
            .filter(|(row, _)| node_named(world, &format!("{JOB_REFRESH_PREFIX}{row}")).is_some())
            .count();
        assert!(
            slot_refresh > 0,
            "station visits: no slot row of the board shows a refresh button, so the absence on \
             the visit rows proves nothing: rows {rows:?}"
        );
        let accept = accept.unwrap_or_else(|| {
            panic!("station visits: every listed visit row greys its Accept: {listed:?}")
        });
        info!(
            "station visits: PASS the board lists {} open visit posting(s) with no refresh \
             button, {slot_refresh} slot row(s) keep theirs; Accept '{}' for '{}'",
            listed.len(),
            accept.0,
            accept.1
        );
        world.resource_mut::<VisitWatch>().accept = Some(accept);
    }

    /// The Accept button the walk aims at.
    fn accept_button(world: &World) -> String {
        world
            .resource::<VisitWatch>()
            .accept
            .as_ref()
            .map(|(button, _)| button.clone())
            .expect("station visits: an Accept was chosen")
    }

    /// Advance when the pointer stands over the chosen Accept.
    fn pointer_over_the_accept() -> Arc<Predicate> {
        when(|world| pointer_over_node(accept_button(world))(world))
    }

    fn the_accept_is_held() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<VisitWatch>()
                .accept
                .as_ref()
                .is_some_and(|(_, id)| world.resource::<Jobs>().holds(id))
        })
    }

    /// Claim 6, second half: the click made the job held, and the visit
    /// posting stays.
    fn assert_the_accept(world: &mut World) {
        let (button, id) = world
            .resource::<VisitWatch>()
            .accept
            .clone()
            .expect("station visits: an Accept was chosen");
        assert!(
            world.resource::<Jobs>().holds(&id),
            "station visits: Jobs holds '{id}' after the click on '{button}'"
        );
        let postings = world.resource::<JobPostings>();
        let posting = postings.visit_posting(&id).unwrap_or_else(|| {
            panic!("station visits: JobPostings dropped the held visit posting '{id}'")
        });
        let remaining = posting.remaining(play_seconds(world));
        let held = world.resource::<Jobs>().held().len();
        nova_probe::probe_marker(
            world,
            "outcome: a docked player accepts a visit job from the board",
            serde_json::json!({
                "job": id.to_string(),
                "button": button,
                "held_jobs": held,
                "posting_remaining_s": remaining,
            }),
        );
        info!(
            "station visits: PASS the player accepted the visit job '{id}' through '{button}': \
             Jobs holds it ({held} held), its visit posting stands with {remaining:.0} s left"
        );
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station visits: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "station visits: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("station visits: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &seed().to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "station visits: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "station visits: pick the start role",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "station visits: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station visits: the window streams in around the start")
            .until(window_is(start_cell))
            .deadline(SESSION_SECS)
            .add()
            .step("station visits: the start docks the player at its starting station")
            .until(and(player_docked(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station visits: census the profiled ships and the offering stations")
            .on_enter(census)
            .add();
        if scanning() {
            return script;
        }
        // The starting station has one port and the player's dock fills it:
        // the dock rule refuses every ship there with `StationOccupied`
        // (the census prints it). The player lets go and holds still.
        script
            // Moved aside on the release frame: a visit that binds while the
            // player still stands at its berth finds that port's corridor
            // blocked by the player and withdraws.
            .step("station visits: release the player and hold it at rest beside the station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
                rest_aside_of_the_station(world);
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station visits: play time runs until a visit binds")
            .until(a_visit_is_pending_or_late())
            .deadline(PLAY_WAIT_SECS)
            .add()
            .step("station visits: assert the bind")
            .on_enter(assert_the_bind)
            .add()
            .step("station visits: play time runs until the visit docks")
            .until(the_visit_docked_or_gone())
            .deadline(PLAY_WAIT_SECS)
            .add()
            .step("station visits: assert the dock and its postings")
            .on_enter(assert_the_dock)
            .add()
            .step("station visits: stand off the docked visitor")
            .on_enter(stand_off_the_visitor)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station visits: shoot the docked visitor")
            .on_enter(|world: &mut World| {
                frame_the_station(world);
                shoot(world, DOCKED_SHOT);
            })
            .until(shot_written(DOCKED_SHOT))
            .deadline(BEAT_SECS)
            .add()
            .step("station visits: hand the camera back after the docked shot")
            .on_enter(release_the_frame)
            .until(frames(1))
            .add()
            .step("station visits: play time runs until the visit ends")
            .until(the_visit_ended_or_late())
            .deadline(PLAY_WAIT_SECS)
            .add()
            .step("station visits: play time runs until the visitor clears the station")
            .until(the_visitor_cleared())
            .deadline(PLAY_WAIT_SECS)
            .add()
            .step("station visits: settle the released station")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station visits: shoot the station after the visitor left")
            .on_enter(|world: &mut World| {
                frame_the_station(world);
                shoot(world, RELEASED_SHOT);
            })
            .until(shot_written(RELEASED_SHOT))
            .deadline(BEAT_SECS)
            .add()
            .step("station visits: hand the camera back after the released shot")
            .on_enter(release_the_frame)
            .until(frames(1))
            .add()
            .step("station visits: assert the end and the bound")
            .on_enter(assert_the_end)
            .add()
            .step("station visits: order the player to dock at the station")
            .on_enter(order_the_player_to_dock)
            .until(and(
                player_docked_at_the_visit_station(),
                frames(SETTLE_FRAMES),
            ))
            .deadline(PLAY_WAIT_SECS)
            .add()
            .step("station visits: press the interface key")
            .on_enter(press_action("interface_toggle"))
            .until(frames(1))
            .add()
            .step("station visits: let the interface key up")
            .on_enter(release_action("interface_toggle"))
            .until(when(|world| {
                world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            }))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "station visits: open the Jobs tab",
                JOBS_TAB,
                the_interface_shows(InterfacePaneType::Jobs),
                BEAT_SECS,
            )
            .step("station visits: the board shows")
            .until(ui_node_present(format!("{JOB_ACCEPT_PREFIX}0")))
            .deadline(BEAT_SECS)
            .add()
            .step("station visits: settle the Jobs pane")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station visits: assert the board rows")
            .on_enter(assert_the_board)
            .add()
            .step("station visits: shoot the Jobs pane")
            .on_enter(|world: &mut World| shoot(world, JOBS_SHOT))
            .until(shot_written(JOBS_SHOT))
            .deadline(BEAT_SECS)
            .add()
            .step("station visits: aim at the visit job's Accept")
            .each(|world: &mut World, _, _| {
                let button = accept_button(world);
                keep_hovering_named(button)(world);
            })
            .until(pointer_over_the_accept())
            .deadline(BEAT_SECS)
            .add()
            .step("station visits: press Accept")
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_SECS)
            .add()
            .step("station visits: release Accept")
            .on_enter(release_mouse(MouseButton::Left))
            .until(and(the_accept_is_held(), frames(SETTLE_FRAMES)))
            .deadline(BEAT_SECS)
            .add()
            .step("station visits: assert the accept")
            .on_enter(assert_the_accept)
            .add()
    }
}
