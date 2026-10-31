//! system_station_own: a living generated station bought from its
//! civilization becomes the player's: it leaves its civilization for the
//! Player side and the owned name, the chart and the Map show it as the
//! player's, and it stays so across a retire, a thaw, a save and a Load.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the window stream in
//! around the session's starting station, where the start docks the player.
//! The walk undocks and jumps to the origin, the spawn point before the
//! docked start: stations are a civilization apart, so the station it wants
//! never streams in round the start. Once the window stands round the origin
//! it picks the nearest generated station that is living, on the Neutral
//! side, and stands outside the origin's cell: a side the buy visibly
//! changes. Before the buy the walk warms the station's civilization to
//! Ally by merits, the way a player who hands in its jobs would, since the
//! buy gate needs an Ally. The player jumps beside it and is put on a berth
//! at its collar each frame until the dock takes; the walk never flies the
//! approach (`system_station_docking` does).
//!
//! DISCLOSED FIXTURE STATE: once docked, the player holds 100,000,000 cr, so
//! the station is affordable. The player's side is set to Enemy twice, as
//! the stance graph will one day set it, each time for one admission check
//! with no frame run, and set back to Player at once: the bought station is
//! on the Player side, and its guns would answer an Enemy-flagged ship.
//! The buy goes through the Station pane's Buy station button, the code path a
//! player clicks. After the undock the player JUMPS two kilometres straight
//! out from the station, at rest; the berth points the mouse rig where it
//! points the hull, so the undock does not turn the hull back through the
//! station. The crossing JUMPS the player three cells out, so the station's
//! cell retires and freezes, then back beside the station, so it thaws.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: an unowned station is its civilization's` | the chosen station is not owned, on the Neutral side, named `<civilization> station`, and its guns read an Enemy ship as Neutral |
//! | 2 | `outcome: a station is bought from its civilization` | the station carries `OwnedStationMarker`, is on the Player side and named `Owned station`, no longer for its civilization; its guns read an Enemy ship as Hostile and the player as Own; the player paid the price the rule gives (twice its sections, stock and credits at ask) as one `StationBuy` change; the station's balance did not move; `OnStationBought` fired once with the station and the price |
//! | 3 | `outcome: the chart holds the bought station as the player's` | after the undock the chart's entry for the station is owned, living and last seen as `Owned station` |
//! | 4 | `outcome: a bought station stays the player's across a retire and a thaw` | after the jump the station's cell is frozen and its record is owned; after the jump back the thawed station is owned, on the Player side and named `Owned station`, and admits an Enemy-flagged ship its side would refuse |
//! | 5 | `outcome: the leave save holds the station owned` | the saved sector's station record is owned, and the saved chart holds it owned |
//! | 6 | `outcome: a loaded station is still the player's` | after Load the station is owned, on the Player side and named `Owned station`; Enemy-flagged, the admission names no refusal; the chart holds it owned; the player docks |
//! | 7 | `outcome: the Map's chart view shows the station as the player's` | the chart view labels the station `STN-n`; selected, the panel reads `OWNED STATION` and `Owned station` |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_station_own --features debug
//! # look for: `station own: PASS the Map's chart view shows the station as the player's`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the Buy station row
//! as `station-own-buy.png`, the docked pair at the loaded owned station as
//! `station-own-docked.png`, the block there as `station-own-owned.png` and
//! the Map's chart view with the station selected as `station-own-chart.png`.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_own")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed: dock at a living Neutral station, warm its civilization to \
             Ally by merits, and buy it from the Station pane; it takes the Player side and the \
             owned name, the chart holds it as the player's, and it stays so across a retire, a \
             thaw, a save and a Load. Autopilot-only correctness range",
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
        // The fleet's 1920x1080, the size the block is shot at.
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        // No frame-time claim: the range holds markers, credits and ids.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::OwnWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use bevy::ecs::system::RunSystemOnce;
    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS,
        },
        nova_events::engine::GameEvent,
        nova_interface::{pane::InterfacePaneType, station::STATION_BUY_BUTTON},
        nova_world_base::prelude::WorldSaveState,
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{
        desired_sectors, FrozenBodyType, FrozenSectors, SectorRoot, WorldClock, WorldConfig,
    };

    use super::*;

    /// The seed the range plays: its window around the origin holds a living
    /// Neutral station outside the origin's cell.
    const STATION_SEED: u32 = 10;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Station Own World";
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

    /// The Station pane's panel, which scrolls under the wheel, and the
    /// station block in it.
    const STATION_PANEL: &str = "StationPanel";
    const STATION_BLOCK: &str = "StationSaleBlock";

    /// The disclosed fixture balance of the player.
    const FIXTURE_CREDITS: u32 = 100_000_000;

    /// The PNGs the capture path shoots.
    const BUY_SHOT: &str = "station-own-buy.png";
    const OWNED_SHOT: &str = "station-own-owned.png";
    const DOCKED_SHOT: &str = "station-own-docked.png";
    const CHART_SHOT: &str = "station-own-chart.png";

    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// How many cells the crossing jumps: past the active radius, so the
    /// station's cell retires.
    const CROSSING_CELLS: i32 = 3;

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
        price_cr: u32,
    }

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct OwnWatch {
        /// Every buy and balance event, by name, with its payload.
        events: Vec<(String, serde_json::Value)>,
        /// The chosen station's typed id.
        id: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// Where it stood when chosen, for the park point.
        position: Option<Meters3>,
        /// Its civilization's name, from its name before the buy.
        civilization: Option<String>,
        /// The facts right before the buy.
        before: Option<BeforeBuy>,
        /// The cell the crossing landed in.
        crossed: Option<SectorCoord>,
    }

    impl OwnWatch {
        fn named(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events
                .iter()
                .filter(|(event, _)| event == name)
                .map(|(_, data)| data)
                .collect()
        }
    }

    pub(super) struct OwnWatchPlugin;

    impl Plugin for OwnWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<OwnWatch>();
            app.add_systems(Update, hold_the_walk_clock);
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<OwnWatch>| {
                let name = event.name();
                if !matches!(name, "onstationbought" | "oncreditschanged") {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station own: {name} {data}");
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
        the_player(world).expect("station own: exactly one player ship")
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
        wearing(world, world.resource::<OwnWatch>().id.as_ref()?)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station own: the station stands")
    }

    fn station_id(world: &World) -> GeneratedBodyId {
        world
            .resource::<OwnWatch>()
            .id
            .clone()
            .expect("station own: the station is chosen")
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .expect("station own: every ship carries a balance")
            .0
    }

    fn owned(world: &World, station: Entity) -> bool {
        world.get::<OwnedStationMarker>(station).is_some()
    }

    fn side(world: &World, root: Entity) -> Option<Allegiance> {
        world.get::<Allegiance>(root).copied()
    }

    fn name(world: &World, root: Entity) -> String {
        world
            .get::<Name>(root)
            .expect("station own: a station is named")
            .as_str()
            .to_string()
    }

    /// How the station's guns read `other`: the relation its AI targets by.
    fn station_reads(world: &World, station: Entity, other: RelationParty<'_>) -> Relation {
        ship_relation(
            RelationParty {
                entity: station,
                allegiance: world.get::<Allegiance>(station),
                retaliation: world.get::<RetaliationTarget>(station),
            },
            other,
        )
    }

    /// An Enemy ship that is not in the world: the station's guns read it by
    /// its side alone.
    fn an_enemy() -> RelationParty<'static> {
        RelationParty {
            entity: Entity::PLACEHOLDER,
            allegiance: Some(&Allegiance::Enemy),
            retaliation: None,
        }
    }

    /// The player's party, as the station's guns read it.
    fn the_player_party(world: &World) -> RelationParty<'_> {
        let player = player(world);
        RelationParty {
            entity: player,
            allegiance: world.get::<Allegiance>(player),
            retaliation: world.get::<RetaliationTarget>(player),
        }
    }

    /// The station is the player's: owned, on the Player side, named
    /// [`OWNED_STATION_NAME`] and no longer for its civilization. `when` names
    /// the beat for the failure.
    fn assert_the_players(world: &World, when: &str) {
        let station = station(world);
        let civilization = world
            .resource::<OwnWatch>()
            .civilization
            .clone()
            .expect("station own: the civilization is recorded");
        assert!(owned(world, station), "station own: {when}: owned");
        assert_eq!(
            side(world, station),
            Some(Allegiance::Player),
            "station own: {when}: on the player's side"
        );
        let name = name(world, station);
        assert_eq!(
            name, OWNED_STATION_NAME,
            "station own: {when}: named as the player's"
        );
        assert!(
            !name.contains(&civilization),
            "station own: {when}: '{name}' no longer names {civilization}"
        );
    }

    /// The chart's entry for the station: owned, living, last seen with the
    /// owned name.
    fn assert_the_chart_owns(chart: &Chart, id: &GeneratedBodyId, when: &str) {
        let entry = chart
            .station(id)
            .unwrap_or_else(|| panic!("station own: {when}: the chart holds the station"));
        assert!(entry.owned, "station own: {when}: the chart holds it owned");
        assert!(
            !entry.derelict,
            "station own: {when}: the chart holds it living"
        );
        assert_eq!(
            entry.summary.as_ref().map(|summary| summary.name.as_str()),
            Some(OWNED_STATION_NAME),
            "station own: {when}: the chart last saw it as the player's"
        );
    }

    /// Enemy-flagged, the player is admitted by the station its side would
    /// refuse; no frame runs while the side is Enemy.
    fn admitted_enemy_flagged(world: &mut World) -> Option<DockRefusalType> {
        set_side(world, Allegiance::Enemy);
        let station = station(world);
        let hostile = station_reads(world, station, the_player_party(world));
        let refused = refusal(world);
        set_side(world, Allegiance::Player);
        assert_eq!(
            hostile,
            Relation::Hostile,
            "station own: the Player-side station reads an Enemy-flagged ship as hostile"
        );
        refused
    }
    /// Why the station refuses the player now, through the one admission
    /// point the dock request reads.
    fn refusal(world: &mut World) -> Option<DockRefusalType> {
        let (player, station) = (player(world), station(world));
        world
            .run_system_once(move |admission: DockAdmission| admission.refusal(player, station))
            .expect("station own: the admission runs")
    }

    fn set_side(world: &mut World, side: Allegiance) {
        let player = player(world);
        world.entity_mut(player).insert(side);
        info!("station own: the player's side is {side:?}");
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
                    && world.get::<StationServices>(station).is_some()
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

    /// The buy reported the player's balance and the station.
    fn buy_reported() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<OwnWatch>();
            !watch.named("onstationbought").is_empty()
                && watch
                    .named("oncreditschanged")
                    .iter()
                    .any(|data| data["cause"] == "StationBuy")
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

    /// Whether the named node lies inside the Station panel's box, where a
    /// click lands on it and a shot shows it.
    fn in_panel(world: &World, name: &str) -> bool {
        match (
            ui_node_rect(world, name),
            ui_node_rect(world, STATION_PANEL),
        ) {
            (Some(node), Some(panel)) => node.min.y >= panel.min.y && node.max.y <= panel.max.y,
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
        info!("station own: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// origin, in the station's own cell.
    fn park_beside(world: &mut World) {
        let watch = world.resource::<OwnWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_origin = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_origin * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            watch.cell.expect("the station is chosen"),
            "station own: the park point stands in the station's cell"
        );
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("station own: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("station own: a section's chain reaches its root")
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
            .unwrap_or_else(|| panic!("station own: {root:?} has no free docking port"))
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
            .unwrap_or_else(|| panic!("station own: {target:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's collar, moving and
    /// turning with the station, then ask to dock. Every frame until the dock
    /// takes, as `system_station_hire` does.
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
            .unwrap_or_else(|| panic!("station own: no field named '{name}'"));
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
            .step(format!("station own: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station own: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Open the Station pane, scroll the station block into view, settle and
    /// shoot `shot`, waiting for `shown` before the shot.
    fn shoot_the_block(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        shown: &'static str,
        shot: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            &format!("the interface key {label}"),
            "interface_toggle",
            when(|world| {
                world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            }),
        )
        .click_named(
            &format!("station own: open the Station pane {label}"),
            "InterfaceTabStation",
            the_interface_shows(InterfacePaneType::Station),
            SESSION_SECS,
        )
        .step(format!("station own: settle the lists {label}"))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step(format!(
            "station own: scroll the station block into view {label}"
        ))
        .each(move |world, _, frame| {
            keep_hovering_named(STATION_PANEL)(world);
            if frame % 2 == 1 && !in_panel(world, shown) {
                scroll_lines(-1.0)(world);
            }
        })
        .until(when(move |world| in_panel(world, shown)))
        .deadline(SESSION_SECS)
        .add()
        .step(format!("station own: settle the scroll {label}"))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step(format!("station own: shoot the block {label}"))
        .on_enter(move |world: &mut World| shoot(world, shot))
        .until(shot_written(shot))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
    }

    /// Pick the nearest station that is living, generated, on the Neutral
    /// side, and stands outside the origin's cell, where the walk stands
    /// after its jump.
    fn choose_the_station(world: &mut World) {
        let (edge, _) = world_shape(world);
        let seen: Vec<(GeneratedBodyId, Option<Allegiance>, bool, Vec3)> = world
            .query_filtered::<(
                &GeneratedBodyId,
                &Position,
                Has<DerelictShipMarker>,
                Option<&Allegiance>,
            ), With<StationMarker>>()
            .iter(world)
            .map(|(id, position, derelict, side)| (id.clone(), side.copied(), derelict, position.0))
            .collect();
        for (id, side, derelict, at) in &seen {
            info!(
                "station own: the window holds '{id}', side {side:?}, derelict {derelict}, {:.0} \
                 m out",
                Meters3::from_engine(*at).distance(Meters3::ZERO).get()
            );
        }
        let mut stations: Vec<(f32, GeneratedBodyId, Vec3)> = seen
            .into_iter()
            .filter(|(_, side, derelict, at)| {
                !derelict
                    && *side == Some(Allegiance::Neutral)
                    && SectorCoord::containing(Meters3::from_engine(*at), edge)
                        != SectorCoord::ORIGIN
            })
            .map(|(id, _, _, at)| {
                (
                    Meters3::from_engine(at).distance(Meters3::ZERO).get(),
                    id,
                    at,
                )
            })
            .collect();
        stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let Some((distance, id, at)) = stations.into_iter().next() else {
            panic!(
                "station own: the window on seed {STATION_SEED} holds no living Neutral station \
                 outside the origin's cell"
            );
        };
        let position = Meters3::from_engine(at);
        let cell = SectorCoord::containing(position, edge);
        info!("station own: chose '{id}' in cell {cell}, {distance:.0} m out");
        let mut watch = world.resource_mut::<OwnWatch>();
        watch.id = Some(id);
        watch.cell = Some(cell);
        watch.position = Some(position);
    }

    /// Claim 1: before the buy the station is its civilization's: no one
    /// owns it, it is on its civilization's Neutral side and named for it,
    /// and its guns read an Enemy ship as Neutral.
    fn assert_the_civilization(world: &mut World) {
        let station = station(world);
        assert!(
            !owned(world, station),
            "station own: the station is no one's before the buy"
        );
        assert_eq!(
            side(world, station),
            Some(Allegiance::Neutral),
            "station own: the station is on its civilization's side"
        );
        let name = name(world, station);
        let civilization = name
            .strip_suffix(" station")
            .unwrap_or_else(|| panic!("station own: '{name}' is a generated station name"))
            .to_string();
        assert_eq!(
            station_reads(world, station, an_enemy()),
            Relation::Neutral,
            "station own: the Neutral station's guns leave an Enemy ship alone"
        );
        nova_probe::probe_marker(
            world,
            "outcome: an unowned station is its civilization's",
            serde_json::json!({
                "station": station_id(world).to_string(),
                "name": name,
                "civilization": civilization,
            }),
        );
        info!("station own: '{name}' is {civilization}'s, on the Neutral side");
        world.resource_mut::<OwnWatch>().civilization = Some(civilization);
    }

    /// Warm the station's civilization to Ally the way a player does: one
    /// merit per handed-in job, `MERIT_STEP` merits per step. Nothing when
    /// it is Ally.
    fn warm_to_ally(world: &mut World, station: Entity) {
        let civ = world
            .get::<TraderMarket>(station)
            .expect("station own: the chosen station prices goods")
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
                "station own: '{}' never warmed to Ally",
                civ.name()
            );
        }
        info!(
            "station own: '{}' warmed to Ally after {merits} merit(s)",
            civ.name()
        );
    }

    /// Disclosed fixture state: the player's balance set to
    /// [`FIXTURE_CREDITS`]. Then record both balances and the price.
    fn record_before_buy(world: &mut World) {
        let player = player(world);
        let station = station(world);
        world.get_mut::<ShipCredits>(player).expect("a balance").0 = FIXTURE_CREDITS;
        info!("station own: fixture sets the player's balance to {FIXTURE_CREDITS} cr");
        let items = world.resource::<GameItems>().clone();
        let recipes: Vec<BuildRecipe> = world
            .get::<Children>(station)
            .expect("station own: a station has sections")
            .iter()
            .filter_map(|child| world.get::<SectionBuildConfig>(child))
            .map(|config| config.0.base.build.clone())
            .collect();
        let stock = world
            .get::<ShipInventory>(station)
            .expect("station own: a station has a hold")
            .clone();
        let station_cr = credits(world, station);
        let price_cr = station_price_cr(&items, &recipes, &stock, station_cr);
        assert!(
            price_cr <= u64::from(FIXTURE_CREDITS),
            "station own: the fixture balance covers the {price_cr} cr price"
        );
        let before = BeforeBuy {
            player_cr: credits(world, player),
            station_cr,
            price_cr: u32::try_from(price_cr).expect("the price fits"),
        };
        info!(
            "station own: '{}' sells for {} cr ({} sections, station balance {} cr)",
            station_id(world),
            before.price_cr,
            recipes.len(),
            station_cr
        );
        world.resource_mut::<OwnWatch>().before = Some(before);
    }

    /// Claim 2: ownership, the credits and the report; then undock.
    fn assert_the_buy(world: &mut World) {
        let player = player(world);
        let station = station(world);
        let before = world
            .resource::<OwnWatch>()
            .before
            .clone()
            .expect("station own: the buy is recorded");
        assert_the_players(world, "after the buy");
        assert_eq!(
            station_reads(world, station, an_enemy()),
            Relation::Hostile,
            "station own: the bought station's guns read an Enemy ship as hostile"
        );
        assert_eq!(
            station_reads(world, station, the_player_party(world)),
            Relation::Own,
            "station own: the bought station's guns read the player as its own"
        );
        let watch = world.resource::<OwnWatch>();
        assert_eq!(
            credits(world, player),
            before.player_cr - before.price_cr,
            "station own: the player paid the price"
        );
        assert_eq!(
            credits(world, station),
            before.station_cr,
            "station own: the price went to the civilization, not the station"
        );
        let changes: Vec<&serde_json::Value> = watch
            .named("oncreditschanged")
            .into_iter()
            .filter(|data| data["cause"] == "StationBuy")
            .collect();
        assert_eq!(changes.len(), 1, "station own: one StationBuy change");
        assert_eq!(changes[0]["delta_cr"], -i64::from(before.price_cr));
        let bought = watch.named("onstationbought");
        assert_eq!(bought.len(), 1, "station own: one OnStationBought");
        assert_eq!(bought[0]["station"], station_id(world).to_string());
        assert_eq!(bought[0]["price_cr"], before.price_cr);
        nova_probe::probe_marker(
            world,
            "outcome: a station is bought from its civilization",
            serde_json::json!({
                "station": station_id(world).to_string(),
                "price_cr": before.price_cr,
            }),
        );
        info!(
            "station own: bought '{}' for {} cr",
            station_id(world),
            before.price_cr
        );
        world.trigger(DockingReleaseRequest { entity: player });
    }

    /// The chart holds the station owned: the undock's record landed.
    fn the_chart_owns_it() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<OwnWatch>()
                .id
                .as_ref()
                .and_then(|id| world.resource::<Chart>().station(id))
                .is_some_and(|entry| entry.owned)
        })
    }

    /// Claim 3: the chart holds the bought station as the player's.
    fn assert_the_chart(world: &mut World) {
        let id = station_id(world);
        assert_the_chart_owns(world.resource::<Chart>(), &id, "after the undock");
        nova_probe::probe_marker(
            world,
            "outcome: the chart holds the bought station as the player's",
            serde_json::json!({ "station": id.to_string(), "name": OWNED_STATION_NAME }),
        );
        info!("station own: the chart holds '{id}' as the player's");
    }

    /// Jump the undocked player [`PARK_DISTANCE`] straight out from the
    /// station, at rest, as a pilot who flies off would stand.
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
        info!("station own: the player flies {PARK_DISTANCE:?} clear of the station");
    }

    /// Jump the player [`CROSSING_CELLS`] cells out along +X.
    fn jump_out(world: &mut World) {
        let (edge, _) = world_shape(world);
        let player = player(world);
        let offset = Vec3::X * Meters(edge.get() * CROSSING_CELLS as f32).to_engine();
        let at = world.get::<Position>(player).expect("a body stands").0 + offset;
        world.entity_mut(player).insert(Position(at));
        let cell = SectorCoord::containing(Meters3::from_engine(at), edge);
        info!("station own: the player jumps to cell {cell}");
        world.resource_mut::<OwnWatch>().crossed = Some(cell);
    }

    /// Whether the station's record in `sectors` is owned; `None` when no
    /// record holds it as a station.
    fn frozen_owned(sectors: &FrozenSectors, id: &str) -> Option<bool> {
        sectors.iter().find_map(|(_, record)| {
            record
                .bodies()
                .iter()
                .find(|body| body.id() == Some(id))
                .and_then(|body| match body.body() {
                    FrozenBodyType::Station(station) => Some(station.owned()),
                    _ => None,
                })
        })
    }

    /// The station's cell is frozen behind the jump.
    fn station_cell_frozen() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<OwnWatch>()
                .cell
                .is_some_and(|cell| world.resource::<FrozenSectors>().get(cell).is_some())
        })
    }

    /// Claim 4, first half: the retired station's record is owned.
    fn assert_the_retire(world: &mut World) {
        let id = station_id(world).to_string();
        assert!(
            station_root(world).is_none(),
            "station own: the station retired with its cell"
        );
        assert_eq!(
            frozen_owned(world.resource::<FrozenSectors>(), &id),
            Some(true),
            "station own: the frozen station record is owned"
        );
        info!(
            "station own: the station froze owned in {:?}",
            world.resource::<OwnWatch>().cell
        );
    }

    /// Claim 4: the thawed station is the player's, and admits an
    /// Enemy-flagged ship its Player side would refuse.
    fn assert_the_thaw(world: &mut World) {
        assert_the_players(world, "after the thaw");
        let refused = admitted_enemy_flagged(world);
        assert_eq!(
            refused, None,
            "station own: the thawed owned station admits an Enemy-flagged ship"
        );
        let watch = world.resource::<OwnWatch>();
        let (cell, crossed) = (watch.cell, watch.crossed);
        nova_probe::probe_marker(
            world,
            "outcome: a bought station stays the player's across a retire and a thaw",
            serde_json::json!({
                "station": station_id(world).to_string(),
                "cell": cell.map(|cell| cell.to_string()),
                "crossed": crossed.map(|cell| cell.to_string()),
            }),
        );
        info!("station own: the station thawed the player's");
    }

    /// The leave save, read back off disk.
    fn open_the_leave_save(world: &World) -> (WorldSaveHeader, WorldSaveState) {
        let root = nova_assets::storage::worlds_root()
            .expect("station own: the sandboxed CONFIG_ROOT gives a worlds root");
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
        .unwrap_or_else(|refusal| panic!("station own: cannot open the leave save: {refusal}"));
        (header, state)
    }

    /// Claim 5: the leave save holds the station owned, on the record and
    /// on the chart.
    fn assert_the_save(world: &mut World) {
        let id = station_id(world).to_string();
        let (header, state) = open_the_leave_save(world);
        assert_eq!(
            frozen_owned(&state.sectors, &id),
            Some(true),
            "station own: the saved station record is owned"
        );
        assert_the_chart_owns(&state.chart, &station_id(world), "in the save");
        nova_probe::probe_marker(
            world,
            "outcome: the leave save holds the station owned",
            serde_json::json!({
                "generation": header.generation,
                "format": header.format,
                "station": id,
            }),
        );
        info!(
            "station own: save generation {} (format {}) holds '{id}' owned",
            header.generation, header.format
        );
    }

    /// Claim 6, first half: the loaded station is the player's, admits the
    /// player Enemy-flagged, and the loaded chart holds it owned.
    fn assert_the_loaded_station(world: &mut World) {
        assert_the_players(world, "after the Load");
        let refused = admitted_enemy_flagged(world);
        assert_eq!(
            refused, None,
            "station own: the loaded owned station admits an Enemy-flagged ship"
        );
        assert_the_chart_owns(
            world.resource::<Chart>(),
            &station_id(world),
            "after the Load",
        );
    }

    /// Claim 6: the player docked at the loaded station.
    fn mark_the_loaded_dock(world: &mut World) {
        nova_probe::probe_marker(
            world,
            "outcome: a loaded station is still the player's",
            serde_json::json!({ "station": station_id(world).to_string() }),
        );
        info!("station own: the loaded station is still the player's, and the player docked");
    }

    /// Every UI text on screen.
    fn texts(world: &mut World) -> Vec<String> {
        world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.clone())
            .collect()
    }

    fn a_text_reads(check: impl Fn(&str) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        when(move |world| {
            world
                .try_query::<&Text>()
                .is_some_and(|mut query| query.iter(world).any(|text| check(&text.0)))
        })
    }

    /// The station's chart code, from the chart's found order.
    fn station_code(world: &World) -> String {
        let id = station_id(world);
        let index = world
            .resource::<Chart>()
            .stations()
            .iter()
            .position(|station| station.id == id)
            .expect("station own: the chart holds the station");
        format!("STN-{}", index + 1)
    }

    /// Select the station's mark on the chart view by its code.
    fn select_the_mark(world: &mut World) {
        let target = format!("MapChartMark{}", station_code(world));
        let label = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, name)| name.as_str() == target)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("station own: no label named '{target}'"));
        world.trigger(bevy::ui_widgets::Activate { entity: label });
    }

    /// Claim 7: the chart view labels the station by code, and its panel
    /// says it is the player's.
    fn assert_the_chart_view(world: &mut World) {
        let code = station_code(world);
        let shown = texts(world);
        assert!(
            shown.contains(&code),
            "station own: the chart view labels the station {code}: {shown:?}"
        );
        assert!(
            shown.iter().any(|text| text == "OWNED STATION"),
            "station own: the panel says the station is owned: {shown:?}"
        );
        assert!(
            shown.iter().any(|text| text == OWNED_STATION_NAME),
            "station own: the panel names the station as the player's: {shown:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the Map's chart view shows the station as the player's",
            serde_json::json!({ "code": code, "station": station_id(world).to_string() }),
        );
        info!("station own: PASS the Map's chart view shows the station as the player's");
    }

    /// Leave to the main menu, run `saved` on the leave save, Load the world
    /// again and wait for `loaded`.
    fn leave_and_load(
        script: AutopilotPlugin<GameStates>,
        saved: fn(&mut World),
        loaded: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        let row = format!(
            "Load World Row: {}",
            world_slug(WORLD_NAME).expect("the world name has a slug")
        );
        script
            .step("station own: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("station own: let escape go")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "station own: click Back to Main Menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("station own: assert the leave save")
            .on_enter(saved)
            .add()
            .click_named(
                "station own: click Load",
                LOAD_BUTTON,
                ui_node_present(row.clone()),
                BEAT_SECS,
            )
            .click_named("station own: select the world", &row, frames(2), BEAT_SECS)
            .click_named(
                "station own: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station own: the loaded window streams in")
            .until(and(window_is(player_cell), loaded))
            .deadline(SESSION_SECS)
            .add()
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station own: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "station own: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("station own: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "station own: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "station own: pick the start role",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "station own: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station own: the window streams in around the start")
            .until(window_is(start_cell))
            .deadline(SESSION_SECS)
            .add()
            .step("station own: the start docks the player at its starting station")
            .until(and(player_docked(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station own: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station own: jump to the origin, the spawn before the docked start")
            .on_enter(|world: &mut World| park_player(world, Meters3::ZERO))
            .until(window_is(|_| Some(SectorCoord::ORIGIN)))
            .deadline(SESSION_SECS)
            .add()
            .step("station own: choose the station")
            .on_enter(choose_the_station)
            .add()
            .step("station own: jump to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<OwnWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station own: the unowned station is its civilization's")
            .on_enter(assert_the_civilization)
            .add()
            .step("station own: warm the station's civilization to Ally")
            .on_enter(|world: &mut World| warm_to_ally(world, station(world)))
            .until(when(|world: &World| {
                world.get::<Allegiance>(station(world)) == Some(&Allegiance::Player)
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("station own: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station own: set the fixture and record the price")
            .on_enter(record_before_buy)
            .until(frames(1))
            .add();
        let script = shoot_the_block(script, "to buy", STATION_BUY_BUTTON, BUY_SHOT).click_named(
            "station own: buy the station",
            STATION_BUY_BUTTON,
            when(|world| station_root(world).is_some_and(|station| owned(world, station))),
            SESSION_SECS,
        );
        // The pane pushed its report under the interface pause; the tracker
        // drains it once the clock runs again.
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station own: the buy reports")
            .until(buy_reported())
            .deadline(SESSION_SECS)
            .add()
            .step("station own: assert the buy, and undock")
            .on_enter(assert_the_buy)
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("station own: the chart records the undock")
            .until(the_chart_owns_it())
            .deadline(BEAT_SECS)
            .add()
            .step("station own: assert the chart")
            .on_enter(assert_the_chart)
            .add()
            .step("station own: fly clear of the station")
            .on_enter(fly_clear)
            .until(frames(2))
            .add()
            .step("station own: jump three cells out")
            .on_enter(jump_out)
            .until(and(
                window_is(|world| world.resource::<OwnWatch>().crossed),
                station_cell_frozen(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station own: assert the retire")
            .on_enter(assert_the_retire)
            .add()
            .step("station own: jump back beside the station")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<OwnWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station own: assert the thaw")
            .on_enter(assert_the_thaw)
            .add();
        let script = leave_and_load(script, assert_the_save, the_station_stands())
            .step("station own: the loaded station is still the player's")
            .on_enter(assert_the_loaded_station)
            .add()
            .step("station own: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station own: mark the loaded dock")
            .on_enter(mark_the_loaded_dock)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station own: shoot the docked pair")
            .on_enter(|world: &mut World| shoot(world, DOCKED_SHOT))
            .until(shot_written(DOCKED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
        let script = shoot_the_block(script, "owned", STATION_BLOCK, OWNED_SHOT).click_named(
            "station own: open the Map pane",
            "InterfaceTabMap",
            the_interface_shows(InterfacePaneType::Map),
            BEAT_SECS,
        );
        tap(
            script,
            "the chart key",
            "map_chart",
            a_text_reads(|text| text.starts_with("STN-")),
        )
        .step("station own: settle the chart view")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station own: select the station on the chart")
        .on_enter(select_the_mark)
        .until(a_text_reads(|text| text == "OWNED STATION"))
        .deadline(BEAT_SECS)
        .add()
        .step("station own: settle the panel")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station own: assert the chart view")
        .on_enter(assert_the_chart_view)
        .add()
        .step("station own: shoot the chart view")
        .on_enter(|world: &mut World| shoot(world, CHART_SHOT))
        .until(shot_written(CHART_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
    }
}
