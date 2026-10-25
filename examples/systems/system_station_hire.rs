//! system_station_hire: an escort hired at a living generated station joins
//! the player's fleet, crosses sectors beside the player, and comes back with
//! it through a save and a Load; a killed escort is gone from the fleet for
//! good.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the window stream in
//! around the session's starting station, where the start docks the player.
//! The walk undocks and jumps to the origin, the spawn point before the
//! docked start: stations are a civilization apart, so the station it wants
//! never streams in round the start. Once the window stands round the origin
//! it picks the nearest generated station that is living, not on the Enemy
//! side, offers Refit, and hires out two escorts or more, and stands outside
//! the origin's cell. The walk then warms the station's civilization to Ally
//! by merits, the way a player who hands in its jobs would, since the hire
//! gate needs an Ally. The player jumps beside it and is put on
//! a berth at its collar each frame until the dock takes; the walk never
//! flies the approach (`system_station_docking` does). It hires two escorts,
//! one per fleet slot, through the same button.
//!
//! DISCLOSED FIXTURE STATE: once docked, the player holds 1,000,000 cr, so
//! the escort is affordable. The hire goes through the Station pane's Hire
//! button, the code path a player clicks. After the undock the player JUMPS
//! two kilometres straight out from the station, at rest, as a pilot who
//! flies off would stand; the berth points the mouse rig where it points the
//! hull, so the undock does not turn the hull back through the station. The
//! crossing JUMPS the player and the escort together three cells out, so the
//! station's cell retires and freezes behind them; the follow is then proved
//! by jumping the player a kilometre on and waiting for the escort to fly
//! back into formation. The player then starts from rest on its main drive,
//! the key a pilot holds, and a trace of every physics step checks that each
//! escort's velocity changed only by what its own drive and RCS delivered and
//! logs how fast each escort was when the player had opened 30 m; no escort
//! may fly faster than the player by more than the follow order's closing
//! speed. The player
//! stops, and its own flight computer burns it to
//! 50 m/s along its nose and turns its travel a right angle to starboard,
//! and the escort flies formation through both: a trace of every physics
//! step checks that only its own velocity moved it. The kill is exact damage
//! on every flight computer of the escort, which neutralizes an armed hull
//! through the production damage entry point.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 0 | `outcome: escorts for hire coast beside the station` | before the hire, every escort for hire the station holds is undocked, Neutral and driven by nobody, and its hull keeps daylight to the station and to every other escort for hire |
//! | 1 | `outcome: an escort is hired from a station's catalogue` | the first escort stands with the offer's name and the id its roster slot gives, Player-sided, in the fleet and following the player on the starboard slot, owned by no cell, and the second on the port slot; both hired hulls left the station's escorts for hire; the player paid both prices and the station took them, all as `EscortHire`; `OnEscortHired` fired once per escort with the escort, the hull and the price |
//! | 1b | `outcome: hired escorts spawn where their offers coasted, clear of every hull` | each escort spawned within 10 m of where its offer coasted, in the station's frame, and at that pose its hull keeps daylight to the other escort, the player and the station, every hull measured by its `HullRadius` about its centre of mass |
//! | 2 | `outcome: an escort crosses sectors beside the player` | after the jump the station's cell is frozen, no frozen record holds the escort, the station's record holds the escorts for hire it kept, and the escort is live, owned by no cell, and back in formation after the player jumps on |
//! | 2b | `outcome: escorts follow a start from rest on their own drive` | from the player at rest to the fleet back in formation on the coasting player, no physics step changed an escort's velocity by more than its own drive impulse over its mass plus its RCS push and its well's pull, its RCS delivered under 0.5 m/s while the player's drive was on and under a tenth of what its drive delivered in all, no escort moved before its drive fired, and no escort flew faster than the player by more than `AI_FOLLOW_CLOSING_SPEED` plus 5 m/s |
//! | 3 | `outcome: an escort flies formation through a turn` | while the player burns to 50 m/s and turns its travel a right angle, the escort ends in formation behind and beside the new travel, matching the player's velocity; its drive took it from rest to the cruise; no physics step moved it more than 0.5 m past its own velocity; it never came within 50 m of the player's hull |
//! | 4 | `outcome: the leave save holds the escort in the player's fleet` | the saved player's fleet is the two escorts, on the starboard and port slots, and no saved sector holds either |
//! | 5 | `outcome: a loaded escort is beside the player and follows` | after Load the escort stands in the fleet beside the player, following the loaded player, and is back in formation after the player jumps on |
//! | 6 | `outcome: a killed escort is gone from the fleet after a save and a Load` | both escorts are killed; each neutralized escort leaves the fleet and a cell takes its wreck; the leave save's fleet is empty and a sector holds the first wreck; after Load no ship is in the fleet and the wreck stands in its cell, neutralized |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_station_hire --features debug
//! # look for: `station hire: PASS the killed escort is gone from the fleet`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the hire list as
//! `station-hire-list.png`, the list with the fleet full as
//! `station-hire-full.png`, the two escorts as they spawned from above as
//! `station-hire-pair.png`, the start from rest from above as
//! `station-hire-start-0.png` to `station-hire-start-3.png`, the escort in
//! formation as
//! `station-hire-escort.png`, after the crossing as
//! `station-hire-crossed.png`, and the turn from above as
//! `station-hire-turn-0.png` to `station-hire-turn-5.png`.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_hire")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed: dock at a living generated station that hires out \
             escorts, warm its civilization to Ally by merits, hire one from the Station pane, \
             cross sectors with it, check it through a save and a Load, then kill it and check it \
             is gone from the fleet. Autopilot-only correctness range",
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
        // The fleet's 1920x1080, the size the hire list is shot at.
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        // No frame-time claim: the range holds counts, credits and ids.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::HireWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{
        AngularVelocity, ComputedCenterOfMass, ComputedMass, LinearVelocity, Position, Rotation,
    };
    use bevy::ui::InteractionDisabled;
    use nova_protocol::{
        nova_debug::harness::{
            pose_camera, press_action, release_action, shoot, AutopilotPlugin, Predicate,
            SHOT_DEADLINE_SECS,
        },
        nova_events::engine::GameEvent,
        nova_interface::pane::InterfacePaneType,
        prelude::*,
    };
    use nova_ui::widget::{TextFieldValue, Tooltip};
    use nova_world::prelude::{
        desired_sectors, FrozenBodyType, FrozenSectors, SectorRoot, WorldClock, WorldConfig,
    };

    use super::*;

    /// The seed the range plays: `system_station_buy`'s seed, whose window
    /// around the origin holds a living Neutral station outside the origin's
    /// cell that builds fighting roles.
    const STATION_SEED: u32 = 10;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Station Hire World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    /// The start role the walk picks: the pirate's short raider hull. The
    /// scavenger's hauler strikes the seed-10 derelict-field station at this
    /// berth and wrecks both hulls.
    const START_ROLE_BUTTON: &str = "Start Role Button: pirate";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";

    /// The escort for hire the walk hires, twice: the first the station
    /// holds, which the second takes the place of once hired.
    const HIRE_OFFER: usize = 0;
    const HIRE_BUTTON: &str = "EscortHire-0";
    /// Where a hired escort may spawn from where its offer coasted, in the
    /// station's frame, engine units (10 m).
    const SPAWN_SLIP: f32 = 1.0;
    /// The Station pane's panel, which scrolls under the wheel.
    const STATION_PANEL: &str = "StationPanel";

    /// The disclosed fixture balance of the player.
    const FIXTURE_CREDITS: u32 = 1_000_000;

    /// The PNGs the capture path shoots.
    const LIST_SHOT: &str = "station-hire-list.png";
    const FULL_SHOT: &str = "station-hire-full.png";
    const ESCORT_SHOT: &str = "station-hire-escort.png";
    const CROSSED_SHOT: &str = "station-hire-crossed.png";
    const PAIR_SHOT: &str = "station-hire-pair.png";
    /// The shots of the start from rest, in order, [`TURN_SHOT_GAP`] frames
    /// apart.
    const START_SHOTS: [&str; 4] = [
        "station-hire-start-0.png",
        "station-hire-start-1.png",
        "station-hire-start-2.png",
        "station-hire-start-3.png",
    ];
    /// How far (engine units, 30 m) the player opens from rest before the
    /// start trace reads each escort's speed.
    const START_OPENED: f32 = 3.0;

    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// How many cells the crossing jumps: past the active radius, so the
    /// station's cell retires.
    const CROSSING_CELLS: i32 = 3;
    /// How far the player jumps on to prove the follow.
    const FOLLOW_JUMP: Meters = Meters(1_000.0);
    /// The escort is in formation within this distance of its goal (engine
    /// units, 100 m) and this speed against the player (engine units per
    /// second, 20 m/s).
    const FORMATION_REACH: f32 = 10.0;
    const FORMATION_SPEED: f32 = 2.0;

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Frames the Station pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;

    /// What the walk recorded right before the hire.
    #[derive(Clone, Debug)]
    struct BeforeHire {
        player_cr: u32,
        station_cr: u32,
        price_cr: u32,
        name: String,
        escort: GeneratedBodyId,
        /// The names of the escorts for hire the station held.
        hires: Vec<String>,
        /// Where the offer coasted, in the station's frame.
        offer: Vec3,
    }

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct HireWatch {
        /// Every hire and balance event, by name, with its payload.
        events: Vec<(String, serde_json::Value)>,
        /// The chosen station's typed id.
        id: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// Where it stood when chosen, for the park point.
        position: Option<Meters3>,
        /// The facts right before the hire.
        before: Option<BeforeHire>,
        /// The cell the crossing landed in.
        crossed: Option<SectorCoord>,
        /// The second escort's id, price and where its offer coasted, in
        /// the station's frame.
        second: Option<(GeneratedBodyId, u32, Vec3)>,
        /// Each hired escort's spawn, in hire order.
        spawns: Vec<EscortSpawn>,
    }

    /// The pose a hired escort spawned in, and the player's and the
    /// station's at that moment, as root position and rotation.
    #[derive(Clone, Copy, Debug)]
    struct EscortSpawn {
        escort: Entity,
        pose: (Vec3, Quat),
        player: (Vec3, Quat),
        station: (Vec3, Quat),
    }

    impl HireWatch {
        fn named(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events
                .iter()
                .filter(|(event, _)| event == name)
                .map(|(_, data)| data)
                .collect()
        }

        fn before(&self) -> &BeforeHire {
            self.before
                .as_ref()
                .expect("station hire: the facts before the hire are recorded")
        }
    }

    pub(super) struct HireWatchPlugin;

    impl Plugin for HireWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<HireWatch>();
            app.init_resource::<FollowTrace>();
            app.init_resource::<StartTrace>();
            app.add_systems(FixedUpdate, trace_the_follow);
            app.add_systems(FixedLast, trace_the_start);
            app.add_observer(record_the_spawn);
            app.add_systems(Update, hold_the_walk_clock);
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<HireWatch>| {
                let name = event.name();
                if !matches!(name, "onescorthired" | "oncreditschanged") {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station hire: {name} {data}");
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
        the_player(world).expect("station hire: exactly one player ship")
    }

    /// The entity wearing `id`, if one does.
    fn wearing(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| *worn == id)
            .map(|(entity, _)| entity)
    }

    fn station_root(world: &World) -> Option<Entity> {
        wearing(world, world.resource::<HireWatch>().id.as_ref()?)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station hire: the station stands")
    }

    fn station_id(world: &World) -> GeneratedBodyId {
        world
            .resource::<HireWatch>()
            .id
            .clone()
            .expect("station hire: the station is chosen")
    }

    fn escort_root(world: &World) -> Option<Entity> {
        wearing(
            world,
            &world.resource::<HireWatch>().before.as_ref()?.escort,
        )
    }

    fn escort(world: &World) -> Entity {
        escort_root(world).expect("station hire: the escort stands")
    }

    fn second_root(world: &World) -> Option<Entity> {
        wearing(world, &world.resource::<HireWatch>().second.as_ref()?.0)
    }

    fn second(world: &World) -> Entity {
        second_root(world).expect("station hire: the second escort stands")
    }

    /// Both escorts, starboard then port.
    fn escorts(world: &World) -> [Entity; 2] {
        [escort(world), second(world)]
    }

    /// Record the pose a hired escort spawns in, and the player's and the
    /// station's: the spawn reads its root `Transform`, which the scenario
    /// object carries from the first command.
    fn record_the_spawn(
        add: On<Add, PlayerFleetMarker>,
        mut watch: ResMut<HireWatch>,
        q_root: Query<&Transform>,
        q_player: Query<&Transform, With<PlayerSpaceshipMarker>>,
        q_station: Query<(&Transform, &GeneratedBodyId), With<StationMarker>>,
    ) {
        if watch.spawns.len() >= 2 {
            return;
        }
        let pose = |transform: &Transform| (transform.translation, transform.rotation);
        let (Ok(escort), Ok(player)) = (q_root.get(add.entity), q_player.single()) else {
            return;
        };
        let Some(station) = q_station
            .iter()
            .find(|(_, id)| Some(*id) == watch.id.as_ref())
            .map(|(transform, _)| pose(transform))
        else {
            return;
        };
        let spawn = EscortSpawn {
            escort: add.entity,
            pose: pose(escort),
            player: pose(player),
            station,
        };
        info!("station hire: an escort spawns {spawn:?}");
        watch.spawns.push(spawn);
    }

    fn escort_name(world: &World) -> String {
        world.resource::<HireWatch>().before().escort.to_string()
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .expect("station hire: every ship carries a balance")
            .0
    }

    /// The escorts for hire `station` holds, in the order the list shows
    /// them.
    fn hire_hulls(world: &World, station: Entity) -> Vec<Entity> {
        world
            .get::<ParkedHulls>(station)
            .map_or_else(Vec::new, |parked| {
                parked
                    .iter()
                    .filter(|hull| {
                        matches!(
                            world.get::<ParkedHullType>(*hull),
                            Some(ParkedHullType::Hire { .. })
                        )
                    })
                    .collect()
            })
    }

    /// The names of the escorts for hire `station` holds.
    fn hire_names(world: &World, station: Entity) -> Vec<String> {
        hire_hulls(world, station)
            .into_iter()
            .filter_map(|hull| world.get::<Name>(hull).map(ToString::to_string))
            .collect()
    }

    /// How many escorts for hire `station` holds or is about to.
    fn hires_offered(world: &World, station: Entity) -> usize {
        let pending = world
            .get::<PendingParkedHulls>(station)
            .map_or(0, |pending| {
                pending
                    .0
                    .iter()
                    .filter(|hull| match hull {
                        PendingParkedHull::Offer(offer) => offer.terms != HullOfferType::Sale,
                        PendingParkedHull::Frozen(frozen) => {
                            matches!(frozen.terms(), ParkedHullType::Hire { .. })
                        }
                    })
                    .count()
            });
        pending + hire_hulls(world, station).len()
    }

    /// Where `body` stands in the station's frame.
    fn in_station_frame(world: &World, station: Entity, body: Entity) -> Vec3 {
        let pose = |entity: Entity| {
            (
                world.get::<Position>(entity).expect("a body stands").0,
                world.get::<Rotation>(entity).expect("a body stands").0,
            )
        };
        let (at, turn) = pose(station);
        turn.inverse() * (pose(body).0 - at)
    }

    /// Every ship in the player's fleet.
    fn fleet(world: &World) -> Vec<Entity> {
        world
            .try_query_filtered::<Entity, With<PlayerFleetMarker>>()
            .map(|mut query| query.iter(world).collect())
            .unwrap_or_default()
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

    /// The cell the player stands in.
    fn player_cell(world: &World) -> Option<SectorCoord> {
        let player = the_player(world)?;
        let (edge, _) = world_shape(world);
        Some(SectorCoord::containing(
            Meters3::from_engine(world.get::<Position>(player)?.0),
            edge,
        ))
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
                    && world.get::<PendingParkedHulls>(station).is_none()
                    && !hire_hulls(world, station).is_empty()
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

    /// Both escorts stand with the sections they spawn with.
    fn fleet_stands() -> Arc<Predicate> {
        when(|world| {
            [escort_root(world), second_root(world)]
                .into_iter()
                .all(|escort| {
                    escort.is_some_and(|escort| {
                        world.get::<Position>(escort).is_some()
                            && world
                                .get::<HullRadius>(escort)
                                .is_some_and(|arm| **arm > 0.0)
                    })
                })
        })
    }

    /// The escort stands with the sections it spawns with.
    fn escort_stands() -> Arc<Predicate> {
        when(|world| {
            escort_root(world).is_some_and(|escort| {
                world.get::<Position>(escort).is_some()
                    && world
                        .get::<HullRadius>(escort)
                        .is_some_and(|arm| **arm > 0.0)
            })
        })
    }

    /// The hire reported both balances and the escort.
    fn hire_reported() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<HireWatch>();
            !watch.named("onescorthired").is_empty()
                && watch
                    .named("oncreditschanged")
                    .iter()
                    .filter(|data| data["cause"] == "EscortHire")
                    .count()
                    >= 4
        })
    }

    /// How far the escort stands from its formation goal and how fast it
    /// moves against the player, engine units.
    fn formation_error(world: &World) -> Option<(f32, f32)> {
        formation_error_of(world, escort_root(world)?)
    }

    /// [`formation_error`] for `escort`.
    fn formation_error_of(world: &World, escort: Entity) -> Option<(f32, f32)> {
        let player = the_player(world)?;
        let follow = world.get::<AIFollow>(escort)?;
        let anchor = |root: Entity| {
            let transform = world.get::<Transform>(root)?;
            Some(live_structure_anchor(
                transform,
                world.get::<ComputedCenterOfMass>(root),
            ))
        };
        let arm = |root: Entity| world.get::<HullRadius>(root).map_or(0.0, |arm| **arm);
        let velocity = |root: Entity| {
            world
                .get::<LinearVelocity>(root)
                .map_or(Vec3::ZERO, |v| v.0)
        };
        let at = anchor(escort)?;
        let goal = formation_goal(
            at,
            anchor(player)?,
            velocity(player),
            world.get::<Transform>(player)?.rotation * Vec3::Y,
            follow.side,
            arm(player) + arm(escort),
        );
        Some((
            at.distance(goal),
            (velocity(escort) - velocity(player)).length(),
        ))
    }

    fn escort_in_formation() -> Arc<Predicate> {
        when(|world| {
            formation_error(world).is_some_and(|(distance, speed)| {
                distance < FORMATION_REACH && speed < FORMATION_SPEED
            })
        })
    }

    /// Both escorts are in formation.
    fn fleet_in_formation() -> Arc<Predicate> {
        when(|world| {
            [escort_root(world), second_root(world)]
                .into_iter()
                .all(|escort| {
                    escort
                        .and_then(|escort| formation_error_of(world, escort))
                        .is_some_and(|(distance, speed)| {
                            distance < FORMATION_REACH && speed < FORMATION_SPEED
                        })
                })
        })
    }

    /// Every escort has stopped: an escort still braking from its last
    /// formation move would spend RCS that the start trace counts against
    /// the start.
    fn fleet_at_rest() -> Arc<Predicate> {
        when(|world| {
            [escort_root(world), second_root(world)]
                .into_iter()
                .all(|escort| {
                    escort
                        .and_then(|escort| world.get::<LinearVelocity>(escort))
                        .is_some_and(|velocity| velocity.length() < 0.05)
                })
        })
    }

    /// The player is at rest, with no flight computer engaged.
    fn player_at_rest() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| {
                world.get::<Autopilot>(player).is_none()
                    && world
                        .get::<LinearVelocity>(player)
                        .is_some_and(|velocity| velocity.length() < 0.05)
            })
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

    /// Whether the Hire button lies inside the Station panel's box, where a
    /// click lands on it.
    fn hire_button_in_view(world: &World) -> bool {
        match (
            ui_node_rect(world, HIRE_BUTTON),
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
        info!("station hire: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// origin, in the station's own cell.
    fn park_beside(world: &mut World) {
        let watch = world.resource::<HireWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_origin = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_origin * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            watch.cell.expect("the station is chosen"),
            "station hire: the park point stands in the station's cell"
        );
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("station hire: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("station hire: a section's chain reaches its root")
                .parent();
        }
        (position, rotation)
    }

    /// The free, live docking ports mounted on `root`, lowest first.
    fn free_ports(world: &mut World, root: Entity) -> Vec<Entity> {
        let ports: Vec<Entity> = world
            .query_filtered::<Entity, (
                With<DockingSectionMarker>,
                Without<DockedPort>,
                Without<SectionInactiveMarker>,
            )>()
            .iter(world)
            .collect();
        let mut ports: Vec<Entity> = ports
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
            .collect();
        ports.sort();
        ports
    }

    /// The lowest free, live docking port mounted on `root`.
    fn free_port(world: &mut World, root: Entity) -> Entity {
        free_ports(world, root)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("station hire: {root:?} has no free docking port"))
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
            .unwrap_or_else(|| panic!("station hire: {target:?} has no free docking port"))
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
        // Point the mouse rig where the berth points the hull, as a pilot who
        // flew in would. Left on the spawn heading, it turns the hull back
        // the moment it undocks and sweeps it through the station.
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
            .unwrap_or_else(|| panic!("station hire: no field named '{name}'"));
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
            .step(format!("station hire: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station hire: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Pick the nearest station that is living, not on the Enemy side,
    /// offers Refit, hires out two escorts or more, and stands outside the
    /// origin's cell, where the walk stands after its jump.
    fn choose_the_station(world: &mut World) {
        let (edge, _) = world_shape(world);
        let mut q_stations = world.query_filtered::<(
            Entity,
            &GeneratedBodyId,
            &Position,
            Option<&StationServices>,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
        ), With<StationMarker>>();
        let view: &World = world;
        let mut stations: Vec<(f32, GeneratedBodyId, Entity, Vec3)> = q_stations
            .iter(view)
            .filter(|(station, _, position, services, derelict, allegiance)| {
                station_offers(*services, *derelict, StationServiceType::Refit)
                    && hires_offered(view, *station) >= 2
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
                "station hire: the window on seed {STATION_SEED} holds no living, non-Enemy \
                 station with Refit and two escorts for hire outside the origin's cell"
            );
        };
        let position = Meters3::from_engine(at);
        let cell = SectorCoord::containing(position, edge);
        info!(
            "station hire: chose '{id}' in cell {cell}, {distance:.0} m out, hiring out {} \
             escorts",
            hires_offered(world, entity)
        );
        let mut watch = world.resource_mut::<HireWatch>();
        watch.id = Some(id);
        watch.cell = Some(cell);
        watch.position = Some(position);
    }

    /// Warm the station's civilization to Ally the way a player does: one
    /// merit per handed-in job, `MERIT_STEP` merits per step. Nothing when
    /// it is Ally.
    fn warm_to_ally(world: &mut World, station: Entity) {
        let civ = world
            .get::<TraderMarket>(station)
            .expect("station hire: the chosen station prices goods")
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
                "station hire: '{}' never warmed to Ally",
                civ.name()
            );
        }
        info!(
            "station hire: '{}' warmed to Ally after {merits} merit(s)",
            civ.name()
        );
    }

    /// Disclosed fixture state: the player's balance set to
    /// [`FIXTURE_CREDITS`]. Then record both balances, the offer, its price,
    /// where it coasts and the id the escort will wear.
    fn record_before_hire(world: &mut World) {
        let player = player(world);
        let station = station(world);
        world.get_mut::<ShipCredits>(player).expect("a balance").0 = FIXTURE_CREDITS;
        info!("station hire: fixture sets the player's balance to {FIXTURE_CREDITS} cr");
        let (hull, slot, price_cr) = hire_offer(world, station);
        let before = BeforeHire {
            player_cr: credits(world, player),
            station_cr: credits(world, station),
            price_cr: u32::try_from(price_cr).expect("the price fits"),
            name: world
                .get::<Name>(hull)
                .expect("an escort for hire is named")
                .to_string(),
            escort: escort_id(&station_id(world), slot),
            hires: hire_names(world, station),
            offer: in_station_frame(world, station, hull),
        };
        info!(
            "station hire: '{}' hires for {} cr as '{}'",
            before.name, before.price_cr, before.escort
        );
        world.resource_mut::<HireWatch>().before = Some(before);
    }

    /// The escort for hire at [`HIRE_OFFER`] of `station`: the hull, its
    /// roster slot and its price.
    fn hire_offer(world: &World, station: Entity) -> (Entity, u32, u64) {
        let hull = *hire_hulls(world, station)
            .get(HIRE_OFFER)
            .expect("station hire: the station holds an escort for hire");
        let Some(&ParkedHullType::Hire { slot }) = world.get::<ParkedHullType>(hull) else {
            panic!("station hire: {hull:?} is not for hire");
        };
        let design = world
            .get::<ShipDesignSections>(hull)
            .expect("an escort for hire has its design");
        let price = escort_price_cr(world.resource::<GameItems>(), &design.0);
        (hull, slot, price)
    }

    /// Claim 0: every escort for hire coasts beside the station, undocked,
    /// Neutral and driven by nobody, its hull clear of the station's and of
    /// every other's.
    fn assert_the_offers(world: &mut World) {
        let station = station(world);
        let hulls = hire_hulls(world, station);
        // The centre of mass and arm of `root`, where it stands.
        let hull = |root: Entity| {
            let at = world.get::<Position>(root).expect("a body stands").0;
            let turn = world.get::<Rotation>(root).expect("a body stands").0;
            let com = world
                .get::<ComputedCenterOfMass>(root)
                .map_or(Vec3::ZERO, |com| com.0);
            let arm = world.get::<HullRadius>(root).map_or(0.0, |arm| **arm);
            (at + turn * com, arm)
        };
        let daylight =
            |(a, arm_a): (Vec3, f32), (b, arm_b): (Vec3, f32)| a.distance(b) - arm_a - arm_b;
        let mut gaps = Vec::new();
        for (index, &offer) in hulls.iter().enumerate() {
            let name = world.get::<Name>(offer).map_or("", Name::as_str);
            assert!(
                world.get::<DockedShip>(offer).is_none(),
                "station hire: '{name}' for hire is not docked"
            );
            assert_eq!(
                world.get::<Allegiance>(offer),
                Some(&Allegiance::Neutral),
                "station hire: '{name}' for hire is Neutral"
            );
            assert!(
                world.get::<AISpaceshipMarker>(offer).is_none()
                    && world.get::<PlayerSpaceshipMarker>(offer).is_none(),
                "station hire: nobody drives '{name}' for hire"
            );
            gaps.push(("offer to station", daylight(hull(offer), hull(station))));
            for &other in &hulls[index + 1..] {
                gaps.push(("offer to offer", daylight(hull(offer), hull(other))));
            }
        }
        let worst = gaps
            .iter()
            .copied()
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .expect("station hire: the station holds escorts for hire");
        info!(
            "station hire: {} escorts for hire coast, least daylight {} {:.0} m",
            hulls.len(),
            worst.0,
            Meters::from_engine(worst.1).get()
        );
        assert!(
            worst.1 > 0.0,
            "station hire: an escort for hire overlaps a hull ({}: {:.0} m)",
            worst.0,
            Meters::from_engine(worst.1).get()
        );
        nova_probe::probe_marker(
            world,
            "outcome: escorts for hire coast beside the station",
            serde_json::json!({
                "offers": hulls.len(),
                "least_m": Meters::from_engine(worst.1).get(),
            }),
        );
    }

    /// The next escort stands at [`HIRE_OFFER`] once the first left the
    /// station, and its Hire button is in view.
    fn next_escort_offered() -> Arc<Predicate> {
        when(|world| {
            let Some(station) = station_root(world) else {
                return false;
            };
            let first = &world.resource::<HireWatch>().before().escort;
            hire_hulls(world, station)
                .get(HIRE_OFFER)
                .and_then(|hull| world.get::<ParkedHullType>(*hull))
                .is_some_and(|terms| match terms {
                    ParkedHullType::Hire { slot } => escort_id(&station_id(world), *slot) != *first,
                    _ => false,
                })
                && hire_button_in_view(world)
        })
    }

    /// Record the second escort's id, price and where its offer coasts.
    fn record_second_hire(world: &mut World) {
        let station = station(world);
        let (hull, slot, price) = hire_offer(world, station);
        let id = escort_id(&station_id(world), slot);
        let price = u32::try_from(price).expect("the price fits");
        info!(
            "station hire: second escort '{}' hires for {price} cr as '{id}'",
            world.get::<Name>(hull).map_or("", Name::as_str)
        );
        let offer = in_station_frame(world, station, hull);
        world.resource_mut::<HireWatch>().second = Some((id, price, offer));
    }

    /// With the fleet full, the list's header names each escort in its slot
    /// box, and every Hire left is greyed with the full-fleet refusal as its
    /// tooltip.
    fn assert_the_full_list(world: &mut World) {
        let player = player(world);
        let held: Vec<(&str, String)> = world
            .query_filtered::<(&AIFollow, &Name), With<PlayerFleetMarker>>()
            .iter(world)
            .filter(|(follow, _)| follow.leader == player)
            .map(|(follow, name)| (fleet_side_label(follow.side), name.to_string()))
            .collect();
        assert_eq!(
            held.len(),
            FLEET_SIDES.len(),
            "station hire: both fleet slots are held"
        );
        for (label, name) in &held {
            let slot = world
                .query::<(&Text, &ChildOf)>()
                .iter(world)
                .find(|(text, _)| text.0 == *label)
                .map(|(_, child_of)| child_of.parent())
                .unwrap_or_else(|| panic!("station hire: no {label} slot box"));
            let shown: Vec<String> = world
                .get::<Children>(slot)
                .expect("station hire: a slot box holds its texts")
                .iter()
                .filter_map(|child| world.get::<Text>(child).map(|text| text.0.clone()))
                .collect();
            assert_eq!(
                shown,
                [label.to_string(), name.clone()],
                "station hire: the {label} slot box names its escort"
            );
        }
        let refusal = format!("hire: the fleet holds its {} escorts", FLEET_SIDES.len());
        let hires: Vec<(String, bool, Option<String>)> = world
            .query::<(&Name, Has<InteractionDisabled>, Option<&Tooltip>)>()
            .iter(world)
            .filter(|(name, ..)| name.as_str().starts_with("EscortHire-"))
            .map(|(name, disabled, tooltip)| {
                (name.to_string(), disabled, tooltip.map(|tip| tip.0.clone()))
            })
            .collect();
        assert!(
            !hires.is_empty(),
            "station hire: the station still lists an escort for hire after both hires"
        );
        for (name, disabled, tooltip) in &hires {
            assert!(
                *disabled,
                "station hire: {name} is greyed with the fleet full"
            );
            assert_eq!(
                tooltip.as_deref(),
                Some(refusal.as_str()),
                "station hire: {name} says why in its tooltip"
            );
        }
        info!(
            "station hire: PASS the fleet row names {held:?}, and {} Hire button(s) are greyed \
             with '{refusal}'",
            hires.len()
        );
    }

    /// Claim 1b: each escort spawned where its offer coasted, clear of the
    /// other escort, the player and the station.
    fn assert_the_spawns(world: &mut World) {
        let watch = world.resource::<HireWatch>();
        assert_eq!(
            watch.spawns.len(),
            2,
            "station hire: two escort spawns recorded"
        );
        let spawns = [watch.spawns[0], watch.spawns[1]];
        let offers = [
            watch.before().offer,
            watch
                .second
                .as_ref()
                .expect("station hire: the second hire is recorded")
                .2,
        ];
        let mut slips = Vec::new();
        for (spawn, offer) in spawns.iter().zip(offers) {
            let (at, turn) = spawn.station;
            let slip = (turn.inverse() * (spawn.pose.0 - at)).distance(offer);
            info!(
                "station hire: an escort spawned {:.1} m from where its offer coasted",
                Meters::from_engine(slip).get()
            );
            assert!(
                slip < SPAWN_SLIP,
                "station hire: an escort spawned {:.1} m from where its offer coasted",
                Meters::from_engine(slip).get()
            );
            slips.push(Meters::from_engine(slip).get());
        }
        let player = player(world);
        let station = station(world);
        // The centre of mass and arm of `root`, at a recorded root pose.
        let hull = |root: Entity, (at, rotation): (Vec3, Quat)| {
            let com = world
                .get::<ComputedCenterOfMass>(root)
                .map_or(Vec3::ZERO, |com| com.0);
            let arm = world.get::<HullRadius>(root).map_or(0.0, |arm| **arm);
            (at + rotation * com, arm)
        };
        let daylight =
            |(a, arm_a): (Vec3, f32), (b, arm_b): (Vec3, f32)| a.distance(b) - arm_a - arm_b;
        let first = hull(spawns[0].escort, spawns[0].pose);
        let second = hull(spawns[1].escort, spawns[1].pose);
        let mut gaps = vec![("escort to escort", daylight(first, second))];
        for spawn in spawns {
            let own = hull(spawn.escort, spawn.pose);
            gaps.push((
                "escort to player",
                daylight(own, hull(player, spawn.player)),
            ));
            gaps.push((
                "escort to station",
                daylight(own, hull(station, spawn.station)),
            ));
        }
        for (pair, gap) in &gaps {
            info!(
                "station hire: spawn daylight {pair} {:.0} m",
                Meters::from_engine(*gap).get()
            );
        }
        let worst = gaps
            .iter()
            .copied()
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .expect("pairs measured");
        assert!(
            worst.1 > 0.0,
            "station hire: a hired escort spawned overlapping a hull ({}: {:.0} m)",
            worst.0,
            Meters::from_engine(worst.1).get()
        );
        nova_probe::probe_marker(
            world,
            "outcome: hired escorts spawn where their offers coasted, clear of every hull",
            serde_json::json!({
                "slip_m": slips,
                "escort_to_escort_m": Meters::from_engine(gaps[0].1).get(),
                "least_m": Meters::from_engine(worst.1).get(),
            }),
        );
    }

    /// Claim 1: the escort, the catalogue, the credits and the report; then
    /// undock.
    fn assert_the_hire(world: &mut World) {
        let player = player(world);
        let station = station(world);
        let escort = escort(world);
        let watch = world.resource::<HireWatch>();
        let before = watch.before().clone();
        assert_eq!(
            world.get::<Name>(escort).map(Name::as_str),
            Some(before.name.as_str()),
            "station hire: the escort carries the offer's name"
        );
        assert_eq!(
            world.get::<EntityId>(escort).map(|id| id.0.clone()),
            Some(before.escort.to_string()),
            "station hire: the escort wears its roster slot's id"
        );
        assert_eq!(
            world.get::<Allegiance>(escort),
            Some(&Allegiance::Player),
            "station hire: the escort is Player-sided"
        );
        assert!(
            world.get::<PlayerFleetMarker>(escort).is_some(),
            "station hire: the escort is in the fleet"
        );
        assert_eq!(
            world
                .get::<AIFollow>(escort)
                .map(|follow| (follow.leader, follow.side)),
            Some((player, FLEET_SIDES[0])),
            "station hire: the escort follows the player on the starboard slot"
        );
        assert!(
            world.get::<ChildOf>(escort).is_none(),
            "station hire: no cell owns the escort"
        );
        let second = second(world);
        assert_eq!(
            world
                .get::<AIFollow>(second)
                .map(|follow| (follow.leader, follow.side)),
            Some((player, FLEET_SIDES[1])),
            "station hire: the second escort follows the player on the port slot"
        );
        let second_price = watch
            .second
            .as_ref()
            .expect("station hire: the second hire is recorded")
            .1;
        let mut hires = before.hires.clone();
        hires.remove(HIRE_OFFER);
        hires.remove(HIRE_OFFER);
        assert_eq!(
            hire_names(world, station),
            hires,
            "station hire: both hired hulls left the station's escorts for hire"
        );
        assert_eq!(
            credits(world, player),
            before.player_cr - before.price_cr - second_price,
            "station hire: the player paid both prices"
        );
        assert_eq!(
            credits(world, station),
            before.station_cr + before.price_cr + second_price,
            "station hire: the station took both prices"
        );
        let causes = watch
            .named("oncreditschanged")
            .into_iter()
            .filter(|data| data["cause"] == "EscortHire")
            .count();
        assert_eq!(
            causes, 4,
            "station hire: both balances moved as EscortHire, per hire"
        );
        let hired = watch.named("onescorthired");
        assert_eq!(hired.len(), 2, "station hire: one OnEscortHired per escort");
        let event = hired[0].clone();
        assert_eq!(event["escort"], before.escort.to_string());
        assert_eq!(event["hull"], before.name.as_str());
        assert_eq!(event["price_cr"], before.price_cr);
        nova_probe::probe_marker(
            world,
            "outcome: an escort is hired from a station's catalogue",
            serde_json::json!({
                "station": station_id(world).to_string(),
                "escort": before.escort.to_string(),
                "hull": before.name,
                "price_cr": before.price_cr,
            }),
        );
        info!(
            "station hire: hired '{}' as '{}' for {} cr",
            before.name, before.escort, before.price_cr
        );
        world.trigger(DockingReleaseRequest { entity: player });
    }

    /// Jump the undocked player [`PARK_DISTANCE`] straight out from the
    /// station, at rest, as a pilot who flies off would stand: the escort's
    /// slot then lies in open space and not across the station.
    fn fly_clear(world: &mut World) {
        let player = player(world);
        let station = world
            .get::<Position>(station(world))
            .expect("a station stands")
            .0;
        let at = world.get::<Position>(player).expect("a body stands").0;
        let out = (at - station).normalize_or(Vec3::Y);
        world.entity_mut(player).insert((
            Position(at + out * PARK_DISTANCE.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("station hire: the player flies {PARK_DISTANCE:?} clear of the station");
    }

    /// Jump the player and the escort together [`CROSSING_CELLS`] cells out
    /// along +X, each keeping its velocity.
    fn jump_the_pair(world: &mut World) {
        let (edge, _) = world_shape(world);
        let offset = Vec3::X * Meters(edge.get() * CROSSING_CELLS as f32).to_engine();
        let mut roots = vec![player(world)];
        roots.extend(fleet(world));
        for root in roots {
            let at = world.get::<Position>(root).expect("a body stands").0 + offset;
            world.entity_mut(root).insert(Position(at));
        }
        let cell = SectorCoord::containing(
            Meters3::from_engine(world.get::<Position>(player(world)).expect("stands").0),
            edge,
        );
        info!("station hire: the player and the escort jump to cell {cell}");
        world.resource_mut::<HireWatch>().crossed = Some(cell);
    }

    /// Jump the player [`FOLLOW_JUMP`] on along its nose, at rest relative
    /// to where it was.
    fn jump_the_player_on(world: &mut World) {
        let player = player(world);
        let rotation = world.get::<Rotation>(player).expect("a body stands").0;
        let at = world.get::<Position>(player).expect("a body stands").0
            + rotation * Vec3::NEG_Z * FOLLOW_JUMP.to_engine();
        world.entity_mut(player).insert(Position(at));
        info!("station hire: the player jumps {FOLLOW_JUMP:?} on");
    }

    /// Frame the player and the escort side by side from behind and above
    /// the pair, so both hulls stand in the shot. Re-posed every frame: the
    /// pair is flying.
    fn frame_the_pair(world: &mut World) {
        let player = player(world);
        let escort = escort(world);
        let at = |root: Entity| {
            world
                .get::<Transform>(root)
                .expect("a body stands")
                .translation
        };
        let (leader, follower) = (at(player), at(escort));
        let rotation = world
            .get::<Transform>(player)
            .expect("a body stands")
            .rotation;
        let middle = (leader + follower) * 0.5;
        let span = leader.distance(follower).max(1.0);
        let eye = middle + rotation * Vec3::new(0.0, 0.45 * span, 1.3 * span);
        pose_camera(
            world,
            Meters3::from_engine(eye),
            Meters3::from_engine(middle),
        );
    }

    /// Hand the camera back to its own rig.
    fn release_the_camera(world: &mut World) {
        let cameras: Vec<Entity> = world
            .query_filtered::<Entity, With<ScriptedCameraPose>>()
            .iter(world)
            .collect();
        for camera in cameras {
            world.entity_mut(camera).remove::<ScriptedCameraPose>();
        }
    }

    /// Frame the pair, settle, shoot `name` and hand the camera back.
    fn shoot_the_pair(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        name: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("station hire: frame the pair {label}"))
            .each(|world, _, _| frame_the_pair(world))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("station hire: shoot the pair {label}"))
            .on_enter(move |world: &mut World| shoot(world, name))
            .each(|world, _, _| frame_the_pair(world))
            .until(shot_written(name))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step(format!("station hire: release the camera {label}"))
            .on_enter(release_the_camera)
            .add()
    }

    /// Whether any record of `sectors` holds the body `id`.
    fn frozen_holds(sectors: &FrozenSectors, id: &str) -> bool {
        sectors
            .iter()
            .any(|(_, record)| record.bodies().iter().any(|body| body.id() == Some(id)))
    }

    /// The station's cell is frozen behind the jump.
    fn station_cell_frozen() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<HireWatch>()
                .cell
                .is_some_and(|cell| world.resource::<FrozenSectors>().get(cell).is_some())
        })
    }

    /// The names of the escorts for hire the frozen station `id` holds.
    fn frozen_hires(sectors: &FrozenSectors, id: &str) -> Option<Vec<String>> {
        sectors.iter().find_map(|(_, record)| {
            record.bodies().iter().find_map(|body| match body.body() {
                FrozenBodyType::Station(station) if body.id() == Some(id) => Some(
                    station
                        .parked()
                        .iter()
                        .filter(|hull| matches!(hull.terms(), ParkedHullType::Hire { .. }))
                        .map(|hull| hull.name().to_string())
                        .collect(),
                ),
                _ => None,
            })
        })
    }

    /// Claim 2, first half: the station's cell froze without the escort and
    /// with the escorts for hire it kept, and the escort is live and owned
    /// by no cell.
    fn assert_the_crossing(world: &mut World) {
        let escort = escort(world);
        let id = escort_name(world);
        assert!(
            !frozen_holds(world.resource::<FrozenSectors>(), &id),
            "station hire: no frozen record holds the escort"
        );
        let mut kept = world.resource::<HireWatch>().before().hires.clone();
        kept.remove(HIRE_OFFER);
        kept.remove(HIRE_OFFER);
        let station_id = station_id(world).to_string();
        assert_eq!(
            frozen_hires(world.resource::<FrozenSectors>(), &station_id),
            Some(kept),
            "station hire: the station's record holds the escorts for hire it kept"
        );
        assert!(
            world.get::<PlayerFleetMarker>(escort).is_some()
                && world.get::<ChildOf>(escort).is_none(),
            "station hire: the escort crossed in the fleet, owned by no cell"
        );
        assert_eq!(
            world.get::<AIFollow>(escort).map(|follow| follow.leader),
            Some(player(world)),
            "station hire: the escort still follows the player"
        );
    }

    /// Claim 2: the escort is back in formation after the player jumped on.
    fn mark_the_crossing(world: &mut World) {
        let (distance, speed) = formation_error(world).expect("station hire: the escort follows");
        let watch = world.resource::<HireWatch>();
        let (cell, crossed) = (watch.cell, watch.crossed);
        nova_probe::probe_marker(
            world,
            "outcome: an escort crosses sectors beside the player",
            serde_json::json!({
                "escort": escort_name(world),
                "from": cell.map(|cell| cell.to_string()),
                "to": crossed.map(|cell| cell.to_string()),
                "formation_m": Meters::from_engine(distance).get(),
                "relative_mps": Meters::from_engine(speed).get(),
            }),
        );
        info!(
            "station hire: the escort crossed to {crossed:?} and flies formation {:.0} m off",
            Meters::from_engine(distance).get()
        );
    }

    /// The player's cruise for the flight beats, engine units per second
    /// (50 m/s).
    const CRUISE: f32 = 5.0;
    /// The shots of the turn, in order.
    const TURN_SHOTS: [&str; 6] = [
        "station-hire-turn-0.png",
        "station-hire-turn-1.png",
        "station-hire-turn-2.png",
        "station-hire-turn-3.png",
        "station-hire-turn-4.png",
        "station-hire-turn-5.png",
    ];
    /// Frames between two shots of the turn.
    const TURN_SHOT_GAP: u32 = 6;
    /// The most a physics step may move the escort past what its own
    /// velocity explains, engine units (0.5 m): a written pose shows as more.
    const MAX_UNEXPLAINED_STEP: f32 = 0.05;

    /// What the follow trace saw over the flight beats, one physics step at
    /// a time.
    #[derive(Resource, Default, Debug)]
    struct FollowTrace {
        /// Whether the flight beats run.
        on: bool,
        /// The escort's position and velocity at the last step.
        last: Option<(Vec3, Vec3)>,
        /// The largest step the escort moved that its own velocity does
        /// not explain, engine units.
        worst_jump: f32,
        /// The escort's lowest and highest speed seen, engine units per
        /// second.
        speed: (f32, f32),
        /// The strongest thruster input seen on the escort.
        thrust: f32,
        /// The narrowest gap seen between the two hulls, engine units.
        closest: f32,
        /// Steps traced.
        steps: u32,
    }

    /// One physics step of the follow trace: the escort's step against its
    /// velocity, its speed, its thrusters, and every tenth step a log line
    /// of the pair's poses.
    fn trace_the_follow(world: &mut World) {
        if !world
            .get_resource::<FollowTrace>()
            .is_some_and(|trace| trace.on)
        {
            return;
        }
        let (Some(player), Some(escort)) = (the_player(world), escort_root(world)) else {
            return;
        };
        let dt = world.resource::<Time>().delta_secs();
        let read = |root: Entity| {
            (
                world.get::<Position>(root).map_or(Vec3::ZERO, |p| p.0),
                world
                    .get::<LinearVelocity>(root)
                    .map_or(Vec3::ZERO, |v| v.0),
            )
        };
        let ((leader_at, leader_v), (at, velocity)) = (read(player), read(escort));
        // Anchor to anchor less both arms: the daylight between the hulls.
        let anchor = |root: Entity| {
            let (position, rotation) = (
                world.get::<Position>(root).map_or(Vec3::ZERO, |p| p.0),
                world.get::<Rotation>(root).map_or(Quat::IDENTITY, |r| r.0),
            );
            position
                + rotation
                    * world
                        .get::<ComputedCenterOfMass>(root)
                        .map_or(Vec3::ZERO, |com| com.0)
        };
        let arm = |root: Entity| world.get::<HullRadius>(root).map_or(0.0, |arm| **arm);
        let gap = anchor(escort).distance(anchor(player)) - arm(escort) - arm(player);
        let thrust = world
            .try_query::<(&ChildOf, &ThrusterSectionInput)>()
            .map_or(0.0, |mut query| {
                query
                    .iter(world)
                    .filter(|(parent, _)| parent.parent() == escort)
                    .map(|(_, input)| input.0.abs())
                    .fold(0.0, f32::max)
            });
        let nose = world
            .get::<Rotation>(player)
            .map_or(Vec3::NEG_Z, |rotation| rotation.0 * Vec3::NEG_Z);
        let mut trace = world.resource_mut::<FollowTrace>();
        if let Some((last_at, last_v)) = trace.last {
            // Semi-implicit Euler moves a body by its NEW velocity; what a
            // step may leave over is the velocity change it made.
            let unexplained =
                ((at - last_at) - velocity * dt).length() - (velocity - last_v).length() * dt;
            trace.worst_jump = trace.worst_jump.max(unexplained);
        }
        trace.last = Some((at, velocity));
        let speed = velocity.length();
        trace.speed = if trace.steps == 0 {
            (speed, speed)
        } else {
            (trace.speed.0.min(speed), trace.speed.1.max(speed))
        };
        trace.thrust = trace.thrust.max(thrust);
        trace.closest = if trace.steps == 0 {
            gap
        } else {
            trace.closest.min(gap)
        };
        if trace.steps.is_multiple_of(10) {
            let offset = at - leader_at;
            info!(
                "station hire: trace step={} leader=({:.1},{:.1},{:.1}) leader_v=({:.2},{:.2},{:.2}) \
                 nose=({:.2},{:.2},{:.2}) escort=({:.1},{:.1},{:.1}) escort_v=({:.2},{:.2},{:.2}) \
                 offset=({:.1},{:.1},{:.1}) range={:.1} rel_speed={:.2} thrust={:.2}",
                trace.steps,
                leader_at.x,
                leader_at.y,
                leader_at.z,
                leader_v.x,
                leader_v.y,
                leader_v.z,
                nose.x,
                nose.y,
                nose.z,
                at.x,
                at.y,
                at.z,
                velocity.x,
                velocity.y,
                velocity.z,
                offset.x,
                offset.y,
                offset.z,
                offset.length(),
                (velocity - leader_v).length(),
                thrust,
            );
        }
        trace.steps += 1;
    }

    /// One escort as the start trace saw it.
    #[derive(Clone, Debug, Default)]
    struct EscortStart {
        /// Its velocity at the last step.
        last: Option<Vec3>,
        /// The largest velocity change of one step past its own drive, RCS
        /// and well, engine units per second.
        worst_excess: f32,
        /// The first step its velocity changed, and its drive fired.
        first_move: Option<u32>,
        first_thrust: Option<u32>,
        /// The velocity its drive and its RCS delivered, engine units per
        /// second, and its RCS while the player's drive was still on.
        drive: f32,
        rcs: f32,
        rcs_starting: f32,
        /// Its speed and the player's when the player had opened
        /// [`START_OPENED`] from rest.
        at_opened: Option<(f32, f32)>,
        /// The most it flew faster than the player, engine units per second.
        overtake: f32,
        /// Its top speed, engine units per second.
        top: f32,
    }

    /// What the start trace saw over the start from rest, one physics step at
    /// a time.
    #[derive(Resource, Default, Debug)]
    struct StartTrace {
        /// Whether the start runs.
        on: bool,
        /// Where the player stood at rest.
        origin: Option<Vec3>,
        /// Steps traced.
        steps: u32,
        /// The first step the player moved.
        player_moved: Option<u32>,
        /// Whether the main drive was let go at the cruise.
        released: bool,
        /// Starboard, then port.
        escorts: [EscortStart; 2],
    }

    /// One physics step of the start trace, after the solver: each escort's
    /// velocity change against what its own drives (per-tick impulse times
    /// input over the mass), its RCS push and the dominant well delivered
    /// this step, logged per step.
    fn trace_the_start(world: &mut World) {
        if !world
            .get_resource::<StartTrace>()
            .is_some_and(|trace| trace.on)
        {
            return;
        }
        let (Some(player), Some(first), Some(second)) =
            (the_player(world), escort_root(world), second_root(world))
        else {
            return;
        };
        let dt = world.resource::<Time>().delta_secs();
        let rcs_accel = world.resource::<FlightSettings>().rcs_accel;
        let gravity = world.resource::<GravitySettings>().clone();
        let wells: Vec<(Entity, Vec3, GravityWell)> = world
            .query::<(Entity, &Position, &GravityWell)>()
            .iter(world)
            .map(|(entity, position, well)| (entity, position.0, well.clone()))
            .collect();
        let drives: Vec<(Entity, f32)> = world
            .query_filtered::<(&ChildOf, &ThrusterSectionMagnitude, &ThrusterSectionInput), Without<SectionInactiveMarker>>()
            .iter(world)
            .map(|(parent, magnitude, input)| (parent.parent(), **magnitude * input.clamp(0.0, 1.0)))
            .collect();
        let read = |root: Entity| {
            (
                world.get::<Position>(root).map_or(Vec3::ZERO, |p| p.0),
                world
                    .get::<LinearVelocity>(root)
                    .map_or(Vec3::ZERO, |v| v.0),
            )
        };
        let (player_at, player_v) = read(player);
        let samples: Vec<(Vec3, Vec3, f32, f32, f32)> = [first, second]
            .into_iter()
            .map(|escort| {
                let (at, velocity) = read(escort);
                let impulse: f32 = drives
                    .iter()
                    .filter(|(parent, _)| *parent == escort)
                    .map(|(_, impulse)| impulse)
                    .sum();
                let mass = world
                    .get::<ComputedMass>(escort)
                    .map_or(f32::INFINITY, |mass| mass.value());
                let rcs = world
                    .get::<RcsBudget>(escort)
                    .map_or(0.0, |rcs| rcs.applied);
                let well = world
                    .get::<GravityAffected>(escort)
                    .and_then(|_| {
                        dominant_well_acceleration(
                            at,
                            world.get::<DominantWell>(escort).map(|well| well.0),
                            &wells,
                            &gravity,
                            &mut Vec::new(),
                            &mut Vec::new(),
                        )
                    })
                    .map_or(0.0, |(_, accel)| accel.length());
                (
                    at,
                    velocity,
                    impulse / mass,
                    rcs * rcs_accel * dt,
                    well * dt,
                )
            })
            .collect();
        // Let the drive go at the cruise, whatever frame the shots are on.
        if player_v.length() >= CRUISE
            && !std::mem::replace(&mut world.resource_mut::<StartTrace>().released, true)
        {
            release_action("main_drive")(world);
            info!("station hire: the player lets the drive go at the cruise");
        }
        let mut trace = world.resource_mut::<StartTrace>();
        let step = trace.steps;
        let origin = *trace.origin.get_or_insert(player_at);
        let opened = player_at.distance(origin);
        let released = trace.released;
        if trace.player_moved.is_none() && player_v.length() > 1e-5 {
            trace.player_moved = Some(step);
        }
        for (index, (at, velocity, drive, rcs, well)) in samples.into_iter().enumerate() {
            let own = drive + rcs + well;
            let escort = &mut trace.escorts[index];
            let change = escort.last.map_or(0.0, |last| (velocity - last).length());
            if escort.last.is_some() {
                escort.worst_excess = escort.worst_excess.max(change - own);
                if change > 1e-5 && escort.first_move.is_none() {
                    escort.first_move = Some(step);
                }
            }
            escort.drive += drive;
            escort.rcs += rcs;
            if !released {
                escort.rcs_starting += rcs;
            }
            if drive > 0.0 && escort.first_thrust.is_none() {
                escort.first_thrust = Some(step);
            }
            escort.overtake = escort.overtake.max(velocity.length() - player_v.length());
            escort.top = escort.top.max(velocity.length());
            if opened >= START_OPENED && escort.at_opened.is_none() {
                escort.at_opened = Some((velocity.length(), player_v.length()));
            }
            escort.last = Some(velocity);
            let offset = at - player_at;
            info!(
                "station hire: start step={step} escort={index} opened={opened:.2} \
                 player_v=({:.3},{:.3},{:.3}) v=({:.3},{:.3},{:.3}) dv={change:.4} own={own:.4} \
                 offset=({:.1},{:.1},{:.1})",
                player_v.x,
                player_v.y,
                player_v.z,
                velocity.x,
                velocity.y,
                velocity.z,
                offset.x,
                offset.y,
                offset.z,
            );
        }
        trace.steps += 1;
    }

    /// Start the start trace and hold the main drive down.
    fn start_on_the_drive(world: &mut World) {
        world.insert_resource(StartTrace {
            on: true,
            ..default()
        });
        press_action("main_drive")(world);
        info!("station hire: the player starts from rest on its main drive");
    }

    /// Claim 2b: each escort's velocity changed only by its own drive and
    /// thrusters, its drive did the following and its RCS only trimmed, and
    /// no escort moved before its drive fired; then the player stops.
    fn mark_the_start(world: &mut World) {
        let trace = std::mem::take(&mut *world.resource_mut::<StartTrace>());
        info!(
            "station hire: start trace steps={} player_moved={:?} escorts={:?}",
            trace.steps, trace.player_moved, trace.escorts
        );
        assert!(trace.steps > 0, "station hire: the start was traced");
        for (index, escort) in trace.escorts.iter().enumerate() {
            assert!(
                escort.worst_excess < 1e-3,
                "station hire: a step changed escort {index}'s velocity {} u/s past its own \
                 thrust",
                escort.worst_excess
            );
            assert!(
                escort.overtake < AI_FOLLOW_CLOSING_SPEED + 0.5,
                "station hire: escort {index} flew {} u/s faster than the player",
                escort.overtake
            );
            assert!(
                escort.rcs_starting < 0.05 && escort.rcs < 0.1 * escort.drive,
                "station hire: escort {index} followed on its drive, its RCS only trimming: \
                 drive {} u/s, RCS {} u/s, {} u/s of it while the player sped up",
                escort.drive,
                escort.rcs,
                escort.rcs_starting
            );
            assert!(
                escort.first_thrust.is_some() && escort.first_thrust <= escort.first_move,
                "station hire: escort {index} moved no sooner than its drive fired: drive at \
                 step {:?}, moved at step {:?}",
                escort.first_thrust,
                escort.first_move
            );
        }
        let at_opened: Vec<_> = trace
            .escorts
            .iter()
            .map(|escort| {
                escort.at_opened.map(|(escort, player)| {
                    (
                        Meters::from_engine(escort).get(),
                        Meters::from_engine(player).get(),
                    )
                })
            })
            .collect();
        nova_probe::probe_marker(
            world,
            "outcome: escorts follow a start from rest on their own drive",
            serde_json::json!({
                "steps": trace.steps,
                "worst_excess_mps": trace
                    .escorts
                    .iter()
                    .map(|escort| Meters::from_engine(escort.worst_excess).get())
                    .collect::<Vec<_>>(),
                "speed_at_30m_opened_mps": at_opened,
                "drive_mps": trace
                    .escorts
                    .iter()
                    .map(|escort| Meters::from_engine(escort.drive).get())
                    .collect::<Vec<_>>(),
                "rcs_mps": trace
                    .escorts
                    .iter()
                    .map(|escort| Meters::from_engine(escort.rcs).get())
                    .collect::<Vec<_>>(),
                "rcs_starting_mps": trace
                    .escorts
                    .iter()
                    .map(|escort| Meters::from_engine(escort.rcs_starting).get())
                    .collect::<Vec<_>>(),
                "top_mps": trace
                    .escorts
                    .iter()
                    .map(|escort| Meters::from_engine(escort.top).get())
                    .collect::<Vec<_>>(),
                "overtake_mps": trace
                    .escorts
                    .iter()
                    .map(|escort| Meters::from_engine(escort.overtake).get())
                    .collect::<Vec<_>>(),
            }),
        );
        info!(
            "station hire: escort and player speeds (m/s) when the player had opened 30 m: \
             {at_opened:?}"
        );
        let player = player(world);
        world
            .entity_mut(player)
            .insert(Autopilot::engage(AutopilotAction::Stop));
    }

    /// Frame the player and the fleet from above in the world's frame.
    fn frame_the_fleet(world: &mut World) {
        let mut roots = vec![player(world)];
        roots.extend(fleet(world));
        let points: Vec<Vec3> = roots
            .iter()
            .map(|root| {
                world
                    .get::<Transform>(*root)
                    .expect("a body stands")
                    .translation
            })
            .collect();
        let middle = points.iter().sum::<Vec3>() / points.len() as f32;
        let span = points
            .iter()
            .map(|point| point.distance(middle))
            .fold(0.0, f32::max)
            .max(20.0)
            * 2.0;
        let eye = middle + Vec3::new(0.0, 2.2 * span, 0.4 * span);
        pose_camera(
            world,
            Meters3::from_engine(eye),
            Meters3::from_engine(middle),
        );
    }

    /// Frame the fleet, settle, shoot `name` and hand the camera back.
    fn shoot_the_fleet(
        script: AutopilotPlugin<GameStates>,
        name: &'static str,
        frames_before: u32,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("station hire: frame the fleet for {name}"))
            .each(|world, _, _| frame_the_fleet(world))
            .until(frames(frames_before))
            .add()
            .step(format!("station hire: shoot {name}"))
            .on_enter(move |world: &mut World| shoot(world, name))
            .each(|world, _, _| frame_the_fleet(world))
            .until(shot_written(name))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }

    /// Hand the player's helm a held velocity of [`CRUISE`] along
    /// `direction`, nose on it: the player flies under its own computer.
    fn cruise_along(world: &mut World, direction: Vec3) {
        let player = player(world);
        let direction = direction.normalize();
        world
            .entity_mut(player)
            .insert(Autopilot::engage(AutopilotAction::MatchVelocity {
                velocity: direction * CRUISE,
                facing: Dir3::new(direction).ok(),
                rcs: true,
            }));
        info!("station hire: the player cruises along {direction}");
    }

    /// Start the trace and burn the player ahead along its nose.
    fn burn_ahead(world: &mut World) {
        world.insert_resource(FollowTrace {
            on: true,
            ..default()
        });
        let player = player(world);
        let nose = world.get::<Rotation>(player).expect("a body stands").0 * Vec3::NEG_Z;
        cruise_along(world, nose);
    }

    /// Turn the player's cruise a right angle to starboard.
    fn turn_starboard(world: &mut World) {
        let player = player(world);
        let rotation = world.get::<Rotation>(player).expect("a body stands").0;
        cruise_along(world, rotation * Vec3::X);
    }

    /// The player cruises at [`CRUISE`].
    fn player_cruising() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| {
                world
                    .get::<LinearVelocity>(player)
                    .is_some_and(|velocity| velocity.length() > CRUISE * 0.9)
            })
        })
    }

    /// Frame the pair from above in the world's frame, so a turn reads as
    /// the pair's tracks and not as a camera swing.
    fn frame_from_above(world: &mut World) {
        let at = |root: Entity| {
            world
                .get::<Transform>(root)
                .expect("a body stands")
                .translation
        };
        let (leader, follower) = (at(player(world)), at(escort(world)));
        let middle = (leader + follower) * 0.5;
        let span = leader.distance(follower).max(40.0);
        let eye = middle + Vec3::new(0.0, 2.2 * span, 0.4 * span);
        pose_camera(
            world,
            Meters3::from_engine(eye),
            Meters3::from_engine(middle),
        );
    }

    /// Shoot the turn as it flies, [`TURN_SHOT_GAP`] frames apart.
    fn shoot_the_turn(mut script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        for name in TURN_SHOTS {
            script = script
                .step(format!("station hire: frame the turn for {name}"))
                .each(|world, _, _| frame_from_above(world))
                .until(frames(TURN_SHOT_GAP))
                .add()
                .step(format!("station hire: shoot {name}"))
                .on_enter(move |world: &mut World| shoot(world, name))
                .each(|world, _, _| frame_from_above(world))
                .until(shot_written(name))
                .deadline(SHOT_DEADLINE_SECS)
                .add();
        }
        script
            .step("station hire: release the camera after the turn")
            .on_enter(release_the_camera)
            .add()
    }

    /// Claim 3: the escort flew formation on the player through the burn
    /// and the turn under its own drive; then the player stops.
    fn mark_the_flight(world: &mut World) {
        let (distance, speed) = formation_error(world).expect("station hire: the escort follows");
        let trace = std::mem::take(&mut *world.resource_mut::<FollowTrace>());
        info!(
            "station hire: flight trace steps={} worst_jump={:.3} speed=({:.2},{:.2}) thrust={:.2} \
             closest={:.1}",
            trace.steps,
            trace.worst_jump,
            trace.speed.0,
            trace.speed.1,
            trace.thrust,
            trace.closest
        );
        assert!(trace.steps > 0, "station hire: the flight was traced");
        assert!(
            trace.worst_jump < MAX_UNEXPLAINED_STEP,
            "station hire: every physics step moved the escort by its own velocity, the worst \
             left {} u over",
            trace.worst_jump
        );
        assert!(
            trace.speed.0 < 1.0 && trace.speed.1 > CRUISE * 0.9 && trace.thrust > 0.0,
            "station hire: the escort's drive took it from rest to the player's cruise: speed \
             {:?}, thrust {}",
            trace.speed,
            trace.thrust
        );
        assert!(
            trace.closest > AI_FOLLOW_GAP * 0.25,
            "station hire: the escort kept clear of the player's hull, closest {} u",
            trace.closest
        );
        nova_probe::probe_marker(
            world,
            "outcome: an escort flies formation through a turn",
            serde_json::json!({
                "escort": escort_name(world),
                "cruise_mps": Meters::from_engine(CRUISE).get(),
                "top_mps": Meters::from_engine(trace.speed.1).get(),
                "closest_m": Meters::from_engine(trace.closest).get(),
                "worst_jump_m": Meters::from_engine(trace.worst_jump).get(),
                "formation_m": Meters::from_engine(distance).get(),
                "relative_mps": Meters::from_engine(speed).get(),
            }),
        );
        info!(
            "station hire: the escort flew formation through the turn, {:.0} m off its goal",
            Meters::from_engine(distance).get()
        );
        let player = player(world);
        world
            .entity_mut(player)
            .insert(Autopilot::engage(AutopilotAction::Stop));
    }

    /// The leave save, read back off disk.
    fn open_the_leave_save(world: &World) -> (WorldSaveHeader, WorldSaveState) {
        let root = nova_assets::storage::worlds_root()
            .expect("station hire: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("the world name has a slug");
        let (_folder, _lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| panic!("station hire: cannot open the leave save: {refusal}"));
        (header, state)
    }

    /// Claim 4: the leave save holds the escort in the player's fleet.
    fn assert_the_save(world: &mut World) {
        let id = escort_name(world);
        let second = world
            .resource::<HireWatch>()
            .second
            .as_ref()
            .expect("station hire: the second hire is recorded")
            .0
            .to_string();
        let (header, state) = open_the_leave_save(world);
        let fleet: Vec<(String, Vec3)> = state
            .player
            .fleet
            .iter()
            .map(|escort| (escort.id.0.clone(), escort.side))
            .collect();
        let mut expected = vec![
            (id.clone(), FLEET_SIDES[0]),
            (second.clone(), FLEET_SIDES[1]),
        ];
        expected.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            fleet, expected,
            "station hire: the saved fleet is the two escorts on the starboard and port slots"
        );
        assert!(
            !frozen_holds(&state.sectors, &id) && !frozen_holds(&state.sectors, &second),
            "station hire: no saved sector holds an escort"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save holds the escort in the player's fleet",
            serde_json::json!({
                "generation": header.generation,
                "format": header.format,
                "fleet": [id, second],
            }),
        );
        info!(
            "station hire: save generation {} (format {}) holds the fleet {fleet:?}",
            header.generation, header.format
        );
    }

    /// Claim 5, first half: the loaded escort stands in the fleet beside the
    /// player, following it.
    fn assert_the_loaded_escort(world: &mut World) {
        let player = player(world);
        let escort = escort(world);
        assert!(
            world.get::<PlayerFleetMarker>(escort).is_some()
                && world.get::<ChildOf>(escort).is_none(),
            "station hire: the loaded escort is in the fleet, owned by no cell"
        );
        assert_eq!(
            world
                .get::<AIFollow>(escort)
                .map(|follow| (follow.leader, follow.side)),
            Some((player, FLEET_SIDES[0])),
            "station hire: the loaded escort follows the loaded player"
        );
        assert_eq!(
            fleet(world).len(),
            2,
            "station hire: both escorts loaded in the fleet"
        );
        let (distance, _) = formation_error(world).expect("station hire: the escort follows");
        assert!(
            distance < FORMATION_REACH,
            "station hire: the loaded escort stands {distance} u off its formation goal"
        );
        info!(
            "station hire: the loaded escort stands {:.0} m off its formation goal",
            Meters::from_engine(distance).get()
        );
    }

    fn mark_the_loaded_follow(world: &mut World) {
        let (distance, speed) = formation_error(world).expect("station hire: the escort follows");
        nova_probe::probe_marker(
            world,
            "outcome: a loaded escort is beside the player and follows",
            serde_json::json!({
                "escort": escort_name(world),
                "formation_m": Meters::from_engine(distance).get(),
                "relative_mps": Meters::from_engine(speed).get(),
            }),
        );
        info!("station hire: the loaded escort flew back into formation");
    }

    /// Exact damage on every live flight computer of the escort: an armed
    /// hull with no computer left is neutralized.
    fn kill_the_escorts(world: &mut World) {
        for escort in escorts(world) {
            kill_the_escort(world, escort);
        }
    }

    fn kill_the_escort(world: &mut World, escort: Entity) {
        let computers: Vec<(Entity, f32)> = world
            .query_filtered::<(Entity, &ChildOf, &Health), (
                With<ControllerSectionMarker>,
                Without<SectionInactiveMarker>,
            )>()
            .iter(world)
            .filter(|(_, parent, _)| parent.parent() == escort)
            .map(|(section, _, health)| (section, health.current))
            .collect();
        assert!(
            !computers.is_empty(),
            "station hire: the escort carries a flight computer to lose"
        );
        info!(
            "station hire: {} flight computers of the escort take exact damage",
            computers.len()
        );
        for (section, amount) in computers {
            world.trigger(HealthApplyDamage {
                entity: section,
                source: None,
                amount,
            });
        }
    }

    /// Both escorts are neutralized, out of the fleet, and a cell owns each.
    fn escort_lost() -> Arc<Predicate> {
        when(|world| {
            [escort_root(world), second_root(world)]
                .into_iter()
                .all(|escort| {
                    escort.is_some_and(|escort| {
                        world.get::<NeutralizedMarker>(escort).is_some()
                            && world.get::<PlayerFleetMarker>(escort).is_none()
                            && world.get::<ChildOf>(escort).is_some_and(|parent| {
                                world.get::<SectorRoot>(parent.parent()).is_some()
                            })
                    })
                })
        })
    }

    fn assert_the_loss(world: &mut World) {
        let escort = escort(world);
        assert!(
            world.get::<AIFollow>(escort).is_none(),
            "station hire: the lost escort follows no one"
        );
        assert!(fleet(world).is_empty(), "station hire: the fleet is empty");
        info!("station hire: the escort is lost and its cell took the wreck");
    }

    /// The leave save after the loss: an empty fleet, and the wreck in a
    /// sector's record.
    fn assert_the_lost_save(world: &mut World) {
        let id = escort_name(world);
        let (header, state) = open_the_leave_save(world);
        assert!(
            state.player.fleet.is_empty(),
            "station hire: the saved fleet is empty"
        );
        let wreck = state
            .sectors
            .iter()
            .find_map(|(coord, record)| {
                record
                    .bodies()
                    .iter()
                    .find(|body| body.id() == Some(id.as_str()))
                    .map(|body| (coord, body.clone()))
            })
            .unwrap_or_else(|| panic!("station hire: no saved sector holds the wreck '{id}'"));
        assert!(
            matches!(wreck.1.body(), FrozenBodyType::Ship(_)),
            "station hire: the wreck froze as a ship"
        );
        info!(
            "station hire: save generation {} keeps no fleet and the wreck in {}",
            header.generation, wreck.0
        );
    }

    /// Claim 6: after Load no ship is in the fleet and the wreck stands in
    /// its cell, neutralized.
    fn assert_the_lost_load(world: &mut World) {
        let escort = escort(world);
        assert!(
            fleet(world).is_empty(),
            "station hire: the loaded fleet is empty"
        );
        assert!(
            world.get::<NeutralizedMarker>(escort).is_some()
                && world.get::<AIFollow>(escort).is_none(),
            "station hire: the loaded wreck is neutralized and follows no one"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a killed escort is gone from the fleet after a save and a Load",
            serde_json::json!({ "escort": escort_name(world) }),
        );
        info!("station hire: PASS the killed escort is gone from the fleet");
    }

    /// Leave to the main menu, run `saved` on the leave save, Load the world
    /// again and wait for `loaded`.
    fn leave_and_load(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        saved: fn(&mut World),
        loaded: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        let row = format!(
            "Load World Row: {}",
            world_slug(WORLD_NAME).expect("the world name has a slug")
        );
        script
            .step(format!("station hire: {label}: open the pause menu"))
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step(format!("station hire: {label}: let escape go"))
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                &format!("station hire: {label}: click Back to Main Menu"),
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step(format!("station hire: {label}: assert the leave save"))
            .on_enter(saved)
            .add()
            .click_named(
                &format!("station hire: {label}: click Load"),
                LOAD_BUTTON,
                ui_node_present(row.clone()),
                BEAT_SECS,
            )
            .click_named(
                &format!("station hire: {label}: select the world"),
                &row,
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                &format!("station hire: {label}: click Load World"),
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step(format!(
                "station hire: {label}: the loaded window streams in"
            ))
            .until(and(window_is(player_cell), loaded))
            .deadline(SESSION_SECS)
            .add()
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station hire: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "station hire: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("station hire: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "station hire: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "station hire: pick the start role",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "station hire: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station hire: the window streams in around the start")
            .until(window_is(start_cell))
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: the start docks the player at its starting station")
            .until(and(player_docked(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: jump to the origin, the spawn before the docked start")
            .on_enter(|world: &mut World| park_player(world, Meters3::ZERO))
            .until(window_is(|_| Some(SectorCoord::ORIGIN)))
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: choose the station")
            .on_enter(choose_the_station)
            .add()
            .step("station hire: jump to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<HireWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: warm the station's civilization to Ally")
            .on_enter(|world: &mut World| warm_to_ally(world, station(world)))
            .until(when(|world: &World| {
                world.get::<Allegiance>(station(world)) == Some(&Allegiance::Player)
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: assert the escorts for hire")
            .on_enter(assert_the_offers)
            .add()
            .step("station hire: set the fixture and record the balances")
            .on_enter(record_before_hire)
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
            "station hire: open the Station pane",
            "InterfaceTabStation",
            the_interface_shows(InterfacePaneType::Station),
            SESSION_SECS,
        )
        .step("station hire: settle the lists")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The escort list stands under the hull list and can pass the bottom
        // of the panel: the wheel over the panel scrolls it into view.
        .step("station hire: scroll the escort list into view")
        .each(|world, _, frame| {
            keep_hovering_named(STATION_PANEL)(world);
            if frame % 2 == 1 && !hire_button_in_view(world) {
                scroll_lines(-1.0)(world);
            }
        })
        .until(when(hire_button_in_view))
        .deadline(SESSION_SECS)
        .add()
        .step("station hire: settle the scroll")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station hire: shoot the escort list")
        .on_enter(|world: &mut World| shoot(world, LIST_SHOT))
        .until(shot_written(LIST_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click_named(
            "station hire: hire the escort",
            HIRE_BUTTON,
            // The escort spawns under the interface pause; its arms are
            // measured once the clock runs again.
            when(|world| escort_root(world).is_some()),
            SESSION_SECS,
        )
        // The next escort takes the hired one's place in the list.
        .step("station hire: the next escort is offered")
        .until(next_escort_offered())
        .deadline(SESSION_SECS)
        .add()
        .step("station hire: record the second hire")
        .on_enter(record_second_hire)
        .add()
        .click_named(
            "station hire: hire the second escort",
            HIRE_BUTTON,
            when(|world| second_root(world).is_some()),
            SESSION_SECS,
        )
        .step("station hire: settle the full list")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station hire: assert the full fleet's list")
        .on_enter(assert_the_full_list)
        .add()
        .step("station hire: shoot the full list")
        .on_enter(|world: &mut World| shoot(world, FULL_SHOT))
        .until(shot_written(FULL_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add();
        // The pane pushed its reports under the interface pause; the tracker
        // drains them once the clock runs again.
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station hire: the hire reports")
            .until(and(hire_reported(), fleet_stands()))
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: assert the spawns")
            .on_enter(assert_the_spawns)
            .add();
        let script = shoot_the_fleet(script, PAIR_SHOT, SETTLE_FRAMES)
            .step("station hire: release the camera after the spawn")
            .on_enter(release_the_camera)
            .add()
            .step("station hire: assert the hire, and undock")
            .on_enter(assert_the_hire)
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: fly clear of the station")
            .on_enter(fly_clear)
            .until(frames(2))
            .add()
            .step("station hire: the escort flies into formation")
            .until(escort_in_formation())
            .deadline(SESSION_SECS)
            .add();
        let script = shoot_the_pair(script, "in formation", ESCORT_SHOT)
            .step("station hire: jump the pair three cells out")
            .on_enter(jump_the_pair)
            .until(and(
                window_is(|world| world.resource::<HireWatch>().crossed),
                and(station_cell_frozen(), escort_stands()),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: assert the crossing")
            .on_enter(assert_the_crossing)
            .add()
            .step("station hire: jump the player on")
            .on_enter(jump_the_player_on)
            .until(frames(2))
            .add()
            .step("station hire: the escort flies back into formation")
            .until(escort_in_formation())
            .deadline(SESSION_SECS)
            .add();
        let mut script = shoot_the_pair(script, "after the crossing", CROSSED_SHOT)
            .step("station hire: mark the crossing")
            .on_enter(mark_the_crossing)
            .add()
            .step("station hire: the fleet holds round the player at rest")
            .until(and(
                fleet_in_formation(),
                and(fleet_at_rest(), player_at_rest()),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: the player starts on its main drive")
            .on_enter(start_on_the_drive)
            .add();
        for name in START_SHOTS {
            script = shoot_the_fleet(script, name, TURN_SHOT_GAP);
        }
        let script = script
            .step("station hire: the player reaches the cruise on its drive")
            .until(player_cruising())
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: release the camera after the start")
            .on_enter(release_the_camera)
            .add()
            .step("station hire: the fleet flies into formation on the coasting player")
            .until(fleet_in_formation())
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: mark the start and stop")
            .on_enter(mark_the_start)
            .until(player_at_rest())
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: the fleet holds round the stopped player")
            .until(fleet_in_formation())
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: the player burns ahead")
            .on_enter(burn_ahead)
            .until(and(player_cruising(), escort_in_formation()))
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: the player turns to starboard")
            .on_enter(turn_starboard)
            .add();
        let script = shoot_the_turn(script)
            .step("station hire: the escort follows through the turn")
            .until(and(player_cruising(), escort_in_formation()))
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: mark the flight and stop")
            .on_enter(mark_the_flight)
            .until(when(|world| {
                the_player(world).is_some_and(|player| world.get::<Autopilot>(player).is_none())
            }))
            .deadline(SESSION_SECS)
            .add();
        let script = leave_and_load(script, "first leave", assert_the_save, escort_stands())
            .step("station hire: assert the loaded escort")
            .on_enter(assert_the_loaded_escort)
            .add()
            .step("station hire: jump the loaded player on")
            .on_enter(jump_the_player_on)
            .until(frames(2))
            .add()
            .step("station hire: the loaded escort flies back into formation")
            .until(escort_in_formation())
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: mark the loaded follow")
            .on_enter(mark_the_loaded_follow)
            .add()
            .step("station hire: kill both escorts")
            .on_enter(kill_the_escorts)
            .until(escort_lost())
            .deadline(SESSION_SECS)
            .add()
            .step("station hire: assert the loss")
            .on_enter(assert_the_loss)
            .add();
        leave_and_load(
            script,
            "second leave",
            assert_the_lost_save,
            when(|world| escort_root(world).is_some()),
        )
        .step("station hire: assert the lost escort after Load")
        .on_enter(assert_the_lost_load)
        .add()
    }
}
