//! system_world_chart: a fresh start docks the player at the allied starting
//! station, the player's chart records the cells entered and a docked
//! station's summary, the Map draws the chart and its discovery log, and a
//! save and a Load keep the chart and the starting station.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the window stream in
//! around the start's cell: the start must dock the player at the session's
//! starting station, an Ally one. The Station pane is opened on the dock and
//! shot. The walk accepts the starting station's first procurement on the
//! Jobs pane and opens the Map's chart view: its OBJECTIVES block must list
//! the held job with its `have / need` and the starting station's code, and
//! only the role's next unmet milestone; it is shot and the chart view
//! closed. The walk undocks. Stations are a civilization apart, so no other
//! one streams in round the start; the walk teleports to the pinned
//! waystation's cell, picks the nearest living, non-Enemy station outside
//! the start's cell other than the starting station, jumps beside it, and is
//! put on a berth at its collar each frame until the dock takes; the walk
//! never flies the approach (`system_station_docking` does). It undocks and
//! teleports past the cell's +X edge into a third cell, so the player has
//! entered exactly three cells.
//!
//! The chart must hold those three cells and the station with the summary
//! seen on the undock: its name, services and the stock in its hold. The Map
//! pane's chart view is opened with the `map_chart` key: the discovery log
//! must list the station and the start's cell with how long ago. Then the
//! station's label, which shows only its chart code, is clicked: the panel
//! must show the last-seen name and its SECTOR block the stock seen on the
//! dock. The player leaves to the main menu; the leave save is read back off
//! disk through [`open_world`], and the world is loaded again in this
//! process through the Load screen.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the player starts docked at the allied starting station` | once the window streams in the player is docked with the station whose id the session's start names, a living station on the Player side |
//! | 2 | `outcome: a docked station's summary reaches the chart` | after the undock the chart holds the station, living, with its name, services and hold stock |
//! | 3 | `outcome: the chart records the three entered cells` | the chart's visited cells are exactly the start's cell, the station's cell and the third cell |
//! | 4 | `outcome: the map's chart view reads the station by its code` | the chart view labels the station `STN-n`; a click shows its last-seen name and its SECTOR block lists the stock |
//! | 5 | `outcome: the discovery log lists the station` | with nothing selected, the panel's log has an icon row reading `STN-n`, its cell and how long ago, and a row for the start's cell |
//! | 6 | `outcome: the leave save holds the chart and the start` | the leave save, read with [`open_world`], holds the three cells, the station's summary and the starting station's id |
//! | 7 | `outcome: a loaded world restores the chart` | after Load the live chart holds the same three cells and the same station summary, and the session names the same starting station |
//! | 8 | `outcome: the chart's objectives list what the player has to do` | at the start, with a procurement held, the OBJECTIVES block lists a `JOB-1` row with the job's title, `have / need` and the starting station's `STN-n`, and one milestone row: the role goal's first unmet line with its `have / need`; no other goal line and no `Met at` row shows |
//!
//! ONE SUBJECT: the chart a streamed world keeps. The save's refusal of an
//! unknown item named by a chart is unit-tested in `nova_world_base::save`.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example system_world_chart --features debug
//! # look for: `world chart: PASS the loaded chart holds 3 cells`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_CAPTURE=1` also writes `start-docked-station.png` of the Station
//! pane at the starting station, `map-objectives.png` of the chart view's
//! OBJECTIVES block at the start and `map-chart-view.png` of the chart view.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_world_chart")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed, dock at a living generated station, enter a third cell, \
             read the chart in the Map's chart view, and check the chart through a save and a \
             Load. Autopilot-only correctness range",
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
        // No frame-time claim: the range holds the chart and its records.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::ChartWatchPlugin);
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
    use std::{collections::BTreeSet, sync::Arc};

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
        nova_interface::pane::InterfacePaneType,
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{desired_sectors, CurrentSector, SectorRoot, WorldConfig};

    use super::*;

    /// The seed the range plays. Its start docks the player at the living
    /// Ally station in cell (0, 6, 2); a living Neutral waystation stands in
    /// [`waystation_cell`] (`system_station_storage` pins it) for the walk to
    /// dock at.
    const STATION_SEED: u32 = 10;

    /// The cell of seed 10's living Neutral waystation, 48 km from the
    /// origin, which the walk jumps to for its second dock.
    fn waystation_cell() -> SectorCoord {
        SectorCoord::new(1, 0, 0)
    }

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Chart World";
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

    /// The frame the capture path writes of the chart view.
    const CHART_SHOT: &str = "map-chart-view.png";
    /// The shot of the Station pane at the starting station.
    const START_SHOT: &str = "start-docked-station.png";
    /// The shot of the chart view's OBJECTIVES block at the start.
    const OBJECTIVES_SHOT: &str = "map-objectives.png";
    /// The `Name` prefix of the OBJECTIVES block's rows.
    const OBJECTIVE_ROW_PREFIX: &str = "MapChartObjective";

    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// The gap the berth leaves between the two port faces, engine units.
    const BERTH_GAP: f32 = 0.5;
    /// Where a port's face sits in its own frame, as the docking verb reads it.
    const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Frames a pane gets to draw.
    const SETTLE_FRAMES: u32 = 6;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;

    /// What the run chose and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct ChartWatch {
        /// The session's starting station, as the start docked the player.
        start: Option<GeneratedBodyId>,
        /// The cell the starting station stands in: the first cell entered.
        start_cell: Option<SectorCoord>,
        /// The chosen station's typed id.
        id: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// Where it stood when chosen.
        position: Option<Meters3>,
        /// The third cell the walk enters.
        third: Option<SectorCoord>,
        /// The station's summary as the chart held it after the undock.
        summary: Option<StationSummary>,
    }

    pub(super) struct ChartWatchPlugin;

    impl Plugin for ChartWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<ChartWatch>();
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
        the_player(world).expect("world chart: exactly one player ship")
    }

    /// The entity wearing the chosen station's id, if one does.
    fn station_root(world: &World) -> Option<Entity> {
        let id = world.resource::<ChartWatch>().id.clone()?;
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| **worn == id)
            .map(|(entity, _)| entity)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("world chart: the station stands")
    }

    fn station_id(world: &World) -> GeneratedBodyId {
        world
            .resource::<ChartWatch>()
            .id
            .clone()
            .expect("world chart: the station is chosen")
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

    fn visited(chart: &Chart) -> BTreeSet<SectorCoord> {
        chart.visited().map(|(cell, _)| cell).collect()
    }

    /// The player's cell, once the player stands in an armed world.
    fn player_cell(world: &World) -> Option<SectorCoord> {
        let (edge, _) = world_shape(world);
        let at = world.get::<Position>(the_player(world)?)?.0;
        Some(SectorCoord::containing(Meters3::from_engine(at), edge))
    }

    /// The three cells the walk enters, in order.
    fn entered(world: &World) -> BTreeSet<SectorCoord> {
        let watch = world.resource::<ChartWatch>();
        [watch.start_cell, watch.cell, watch.third]
            .into_iter()
            .map(|cell| cell.expect("world chart: every entered cell is chosen"))
            .collect()
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

    fn the_chart_has_visited(
        cell: impl Fn(&World) -> Option<SectorCoord> + Send + Sync + 'static,
    ) -> Arc<Predicate> {
        when(move |world| {
            cell(world).is_some_and(|cell| world.resource::<Chart>().has_visited(cell))
        })
    }

    fn the_chart_found_the_station() -> Arc<Predicate> {
        when(|world| {
            let Some(id) = world.resource::<ChartWatch>().id.clone() else {
                return false;
            };
            world.resource::<Chart>().station(&id).is_some()
        })
    }

    fn the_chart_holds_a_summary() -> Arc<Predicate> {
        when(|world| {
            let Some(id) = world.resource::<ChartWatch>().id.clone() else {
                return false;
            };
            world
                .resource::<Chart>()
                .station(&id)
                .is_some_and(|station| station.summary.is_some())
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

    fn the_interface_shows(pane: Option<InterfacePaneType>) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && pane.is_none_or(|pane| {
                    world
                        .get_resource::<InterfacePaneType>()
                        .is_some_and(|shown| *shown == pane)
                })
        })
    }

    fn unpaused() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
        })
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

    /// Teleport the player to `position`, at rest.
    fn park_player(world: &mut World, position: Meters3) {
        let player = player(world);
        world.entity_mut(player).insert((
            Position(position.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("world chart: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// start, in the station's own cell.
    fn park_beside(world: &mut World) {
        let start = world.resource::<OpenWorldSession>().start.position;
        let watch = world.resource::<ChartWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_spawn = (start.get() - position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_spawn * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            watch.cell.expect("the station is chosen"),
            "world chart: the park point stands in the station's cell"
        );
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("world chart: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("world chart: a section's chain reaches its root")
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
            .unwrap_or_else(|| panic!("world chart: {root:?} has no free docking port"))
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
            .unwrap_or_else(|| panic!("world chart: {target:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's collar and ask to dock,
    /// every frame until the dock takes.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
        let station_port = outward_port(world, station);
        let player_port = free_port(world, player);
        let body = |entity: Entity| {
            (
                world
                    .get::<Position>(entity)
                    .expect("a hull has a position")
                    .0,
                world
                    .get::<Rotation>(entity)
                    .expect("a hull has a rotation")
                    .0,
                world
                    .get::<LinearVelocity>(entity)
                    .map_or(Vec3::ZERO, |v| v.0),
                world
                    .get::<AngularVelocity>(entity)
                    .map_or(Vec3::ZERO, |w| w.0),
            )
        };
        let (station_at, station_rot, station_velocity, station_spin) = body(station);
        let (port_at, port_rot) = local_pose(world, station_port, station);
        let station_face_rot = station_rot * port_rot;
        let station_face = station_at + station_rot * port_at + station_face_rot * PORT_FACE;
        let station_axis = station_face_rot * Vec3::NEG_Z;
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let rotation = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -station_axis);
        let face_offset = rotation * own_at + rotation * own_rot * PORT_FACE;
        let position = station_face + station_axis * BERTH_GAP - face_offset;
        world.entity_mut(player).insert((
            Position(position),
            Rotation(rotation),
            LinearVelocity(station_velocity),
            AngularVelocity(station_spin),
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
            .unwrap_or_else(|| panic!("world chart: no field named '{name}'"));
        value.0 = text.to_string();
    }

    /// The living, non-Enemy stations outside the start's cell other than
    /// the starting station, nearest the player first, ties by id.
    fn other_stations(world: &World) -> Vec<(f32, GeneratedBodyId, Meters3)> {
        let (edge, _) = world_shape(world);
        let Some(player) = the_player(world) else {
            return Vec::new();
        };
        let Some(from) = world.get::<Position>(player) else {
            return Vec::new();
        };
        let from = Meters3::from_engine(from.0);
        let watch = world.resource::<ChartWatch>();
        let (Some(start), Some(start_cell)) = (watch.start.clone(), watch.start_cell) else {
            return Vec::new();
        };
        let Some(mut query) = world.try_query_filtered::<(
            &GeneratedBodyId,
            &Position,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
        ), With<StationMarker>>() else {
            return Vec::new();
        };
        let mut stations: Vec<(f32, GeneratedBodyId, Meters3)> = query
            .iter(world)
            .filter(|(id, position, derelict, allegiance)| {
                !derelict
                    && *allegiance != Some(&Allegiance::Enemy)
                    && **id != start
                    && SectorCoord::containing(Meters3::from_engine(position.0), edge) != start_cell
            })
            .map(|(id, position, ..)| {
                let position = Meters3::from_engine(position.0);
                (position.distance(from).get(), id.clone(), position)
            })
            .collect();
        stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        stations
    }

    /// A station the walk can dock at has streamed in.
    fn other_station_stands() -> Arc<Predicate> {
        when(|world| !other_stations(world).is_empty())
    }

    /// Teleport the player to the pinned waystation's cell centre.
    fn jump_to_the_waystation_cell(world: &mut World) {
        let (edge, _) = world_shape(world);
        park_player(world, waystation_cell().centre(edge));
    }

    /// Pick the nearest living, non-Enemy station outside the start's cell,
    /// other than the starting station.
    fn choose_the_station(world: &mut World) {
        let (edge, _) = world_shape(world);
        let Some((distance, id, position)) = other_stations(world).into_iter().next() else {
            panic!(
                "world chart: the window on seed {STATION_SEED} holds no living, non-Enemy \
                 station outside the start's cell other than the starting station"
            );
        };
        let cell = SectorCoord::containing(position, edge);
        let third = cell.offset(1, 0, 0);
        info!("world chart: chose '{id}' in cell {cell}, {distance:.0} m out; third cell {third}");
        let mut watch = world.resource_mut::<ChartWatch>();
        watch.id = Some(id);
        watch.cell = Some(cell);
        watch.position = Some(position);
        watch.third = Some(third);
    }

    /// Teleport the player past its cell's +X edge into the third cell.
    fn cross_sector_edge(world: &mut World) {
        let (edge, _) = world_shape(world);
        let current = world.resource::<CurrentSector>().0;
        let third = world.resource::<ChartWatch>().third.expect("chosen");
        assert_eq!(
            current.offset(1, 0, 0),
            third,
            "world chart: +X is the third cell"
        );
        park_player(world, third.centre(edge));
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("world chart: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("world chart: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Claim 2: the undock left the station's summary on the chart.
    fn assert_the_summary(world: &mut World) {
        let id = station_id(world);
        let station = station(world);
        let name = world
            .get::<Name>(station)
            .expect("world chart: a station root has a name")
            .as_str()
            .to_string();
        let services = world
            .get::<StationServices>(station)
            .expect("world chart: a station root carries its services")
            .0
            .clone();
        let stock: Vec<(ItemDesignId, u32)> = world
            .get::<ShipInventory>(station)
            .expect("world chart: a station carries a hold")
            .stacks()
            .map(|(item, count)| (item.clone(), count))
            .collect();
        let charted = world
            .resource::<Chart>()
            .station(&id)
            .cloned()
            .expect("world chart: the chart holds the station");
        assert!(
            !charted.derelict,
            "world chart: the station is charted living"
        );
        let summary = charted
            .summary
            .expect("world chart: the undock left a summary");
        assert_eq!(
            summary.name, name,
            "world chart: the summary names the station"
        );
        assert_eq!(
            summary.services, services,
            "world chart: the summary lists the services"
        );
        assert_eq!(
            summary.stock, stock,
            "world chart: the summary holds the stock"
        );
        assert!(
            !summary.stock.is_empty(),
            "world chart: a living waystation shows stock"
        );
        assert!(
            summary.stored.is_empty(),
            "world chart: nothing was stored here"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a docked station's summary reaches the chart",
            serde_json::json!({
                "station": id.to_string(),
                "name": summary.name,
                "services": format!("{:?}", summary.services),
                "stock": summary.stock.len(),
            }),
        );
        info!(
            "world chart: the chart holds '{id}' as '{}', {} stock stack(s), services {:?}",
            summary.name,
            summary.stock.len(),
            summary.services
        );
        world.resource_mut::<ChartWatch>().summary = Some(summary);
    }

    /// Claim 1: the start docked the player at the session's starting
    /// station, an allied one, with the undock step listed and nothing
    /// marked.
    fn assert_the_start(world: &mut World) {
        let start = world.resource::<OpenWorldSession>().start.clone();
        let (edge, _) = world_shape(world);
        let player = player(world);
        let docked = world
            .get::<DockedShip>(player)
            .expect("world chart: the start docks the player");
        let partner = world
            .get::<DockingConnection>(docked.connection)
            .map(|connection| {
                if connection.first_ship == player {
                    connection.second_ship
                } else {
                    connection.first_ship
                }
            })
            .expect("world chart: a docked ship's connection stands");
        let worn = world
            .get::<GeneratedBodyId>(partner)
            .expect("world chart: the dock partner is a generated body")
            .clone();
        assert_eq!(
            worn, start.id,
            "world chart: the player is docked at the session's starting station"
        );
        assert!(
            world.get::<StationMarker>(partner).is_some()
                && world.get::<DerelictShipMarker>(partner).is_none(),
            "world chart: the starting station is a living station"
        );
        assert_eq!(
            world.get::<Allegiance>(partner),
            Some(&Allegiance::Player),
            "world chart: the starting station is on the player's side"
        );
        let cell = SectorCoord::containing(start.position, edge);
        assert_eq!(
            player_cell(world),
            Some(cell),
            "world chart: the player stands in the starting station's cell"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the player starts docked at the allied starting station",
            serde_json::json!({
                "station": start.id.to_string(),
                "cell": cell.to_string(),
                "position": format!("{:?}", start.position.get()),
            }),
        );
        info!(
            "world chart: the player starts docked at '{}' in cell {cell}",
            start.id
        );
        let mut watch = world.resource_mut::<ChartWatch>();
        watch.start = Some(start.id);
        watch.start_cell = Some(cell);
    }

    /// The starting station's board row of its first procurement: the
    /// posted row whose title is that job's, as the Jobs pane lists it.
    fn starter_procurement_row(world: &mut World) -> usize {
        let start = world
            .resource::<ChartWatch>()
            .start
            .clone()
            .expect("world chart: the start is recorded");
        let job = world
            .query::<(&GeneratedBodyId, &JobBoard)>()
            .iter(world)
            .find(|(id, _)| **id == start)
            .and_then(|(_, board)| board.0.iter().find(|job| job.kind.is_procurement()))
            .cloned()
            .expect("world chart: the starting station posts a procurement");
        let title = job.title(world.resource::<GameItems>());
        (0..)
            .map(|index| (index, named_row(world, &format!("JobRowPosted-{index}"))))
            .take_while(|(_, row)| row.is_some())
            .find(|(_, row)| row.as_ref().is_some_and(|row| row.contains(&title)))
            .map(|(index, _)| index)
            .unwrap_or_else(|| panic!("world chart: the board lists '{title}'"))
    }

    /// Activate the UI node named `name`.
    fn activate_named(world: &mut World, name: &str) {
        let target = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("world chart: no node named '{name}'"));
        world.trigger(bevy::ui_widgets::Activate { entity: target });
    }

    fn held_count(count: usize) -> Arc<Predicate> {
        when(move |world| world.resource::<Jobs>().held().len() == count)
    }

    /// Whether the chart view's OBJECTIVES block shows.
    fn objectives_show(shown: bool) -> Arc<Predicate> {
        when(move |world| {
            world
                .try_query::<(&Name, &Node)>()
                .and_then(|mut query| {
                    query
                        .iter(world)
                        .find(|(name, _)| name.as_str() == "MapChartObjectives")
                        .map(|(_, node)| (node.display != Display::None) == shown)
                })
                .unwrap_or(false)
        })
    }

    /// Whether a text reads `have / need`.
    fn have_need(text: &str) -> bool {
        text.split_once(" / ")
            .is_some_and(|(have, need)| have.parse::<u64>().is_ok() && need.parse::<u64>().is_ok())
    }

    /// Claim 8: at the start, with the procurement held, the OBJECTIVES
    /// block lists the job with `have / need` and its hand-in station's
    /// code, and only the next unmet milestone.
    fn assert_the_objectives(world: &mut World) {
        let rows = named_row(world, "MapChartObjectives")
            .expect("world chart: the panel holds the OBJECTIVES block");
        let job = world.resource::<Jobs>().held()[0].clone();
        let title = job.title(world.resource::<GameItems>());
        let start = world
            .resource::<ChartWatch>()
            .start
            .clone()
            .expect("world chart: the start is recorded");
        let index = world
            .resource::<Chart>()
            .stations()
            .iter()
            .position(|station| station.id == start)
            .expect("world chart: the chart holds the starting station");
        let hand_in = format!("STN-{}", index + 1);
        let job_row = named_row(world, &format!("{OBJECTIVE_ROW_PREFIX}JOB-1"))
            .unwrap_or_else(|| panic!("world chart: the block lists JOB-1: {rows:?}"));
        assert!(
            job_row.contains(&title)
                && job_row.iter().any(|text| have_need(text))
                && job_row.contains(&hand_in),
            "world chart: the job row reads '{title}', have / need and {hand_in}: {job_row:?}"
        );
        let goal = world.resource::<GoalProgress>().clone();
        let next = goal
            .lines
            .iter()
            .find(|line| line.have < line.need)
            .expect("world chart: a fresh role has an unmet milestone");
        // `texts_under` reads depth first, so a row's texts come in no
        // fixed order: compare them sorted.
        let mut milestones = named_rows(world, &format!("{OBJECTIVE_ROW_PREFIX}Milestone"));
        milestones.iter_mut().for_each(|row| row.sort());
        let mut expected = vec![next.label.clone(), format!("{} / {}", next.have, next.need)];
        expected.sort();
        assert_eq!(
            milestones,
            vec![expected],
            "world chart: the block lists only the next unmet milestone: {rows:?}"
        );
        for line in goal.lines.iter().filter(|line| line.label != next.label) {
            assert!(
                !rows.contains(&line.label),
                "world chart: '{}' is not the next milestone and is not listed: {rows:?}",
                line.label
            );
        }
        assert!(
            named_rows(world, &format!("{OBJECTIVE_ROW_PREFIX}Met")).is_empty(),
            "world chart: a standing goal shows no Met row: {rows:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the chart's objectives list what the player has to do",
            serde_json::json!({ "rows": rows }),
        );
        info!("world chart: PASS the objectives read {rows:?}");
    }

    /// Claim 3: the chart's visited cells are exactly the three entered.
    fn assert_the_visited_cells(world: &mut World) {
        let expected = entered(world);
        let chart = world.resource::<Chart>().clone();
        let cells = visited(&chart);
        assert_eq!(
            cells, expected,
            "world chart: the chart holds exactly the entered cells"
        );
        let types: Vec<String> = chart
            .visited()
            .map(|(coord, cell)| format!("{coord} {:?}", cell.clusters))
            .collect();
        let others: Vec<String> = chart
            .stations()
            .iter()
            .map(|station| station.id.to_string())
            .chain(chart.wrecks().iter().map(|wreck| wreck.id.to_string()))
            .collect();
        nova_probe::probe_marker(
            world,
            "outcome: the chart records the three entered cells",
            serde_json::json!({ "cells": types, "finds": others }),
        );
        info!("world chart: the chart holds {types:?}; finds {others:?}");
    }

    /// The station's chart code, from the chart's found order.
    fn station_code(world: &World) -> String {
        let id = station_id(world);
        let index = world
            .resource::<Chart>()
            .stations()
            .iter()
            .position(|station| station.id == id)
            .expect("world chart: the chart holds the station");
        format!("STN-{}", index + 1)
    }

    /// Claim 4: the chart view labels the station by code, and its panel
    /// shows the last-seen name and stock.
    fn assert_the_chart_view(world: &mut World) {
        let code = station_code(world);
        let summary = world
            .resource::<ChartWatch>()
            .summary
            .clone()
            .expect("recorded");
        let shown = texts(world);
        assert!(
            shown.contains(&code),
            "world chart: the chart view labels the station {code}: {shown:?}"
        );
        assert!(
            shown.contains(&summary.name),
            "world chart: the panel names the station: {shown:?}"
        );
        let note = shown
            .iter()
            .find(|text| text.contains("Last seen docked."))
            .unwrap_or_else(|| panic!("world chart: the panel shows the last dock: {shown:?}"))
            .clone();
        let (item, count) = summary.stock.first().expect("checked non-empty");
        let item_name = world.resource::<GameItems>().design(item).name.clone();
        let stock_row = sector_rows(world)
            .into_iter()
            .find(|row| row.contains(&item_name) && row.contains(&format!("x{count}")))
            .unwrap_or_else(|| panic!("world chart: the SECTOR block lists the stock: {shown:?}"));
        assert!(
            shown.iter().any(|text| text == "LIVING STATION"),
            "world chart: the panel says the station lives: {shown:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the map's chart view reads the station by its code",
            serde_json::json!({ "code": code, "name": summary.name, "note": note, "stock_row": stock_row }),
        );
        info!("world chart: the chart view shows {code}: {note:?}, {stock_row:?}");
    }

    /// Every text under `root`, depth first.
    fn texts_under(world: &mut World, root: Entity) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(entity) = stack.pop() {
            if let Some(text) = world.get::<Text>(entity) {
                out.push(text.0.clone());
            }
            if let Some(children) = world.get::<Children>(entity) {
                stack.extend(children.iter());
            }
        }
        out
    }

    /// The texts of every panel row whose name starts with `prefix`, one row
    /// at a time.
    fn named_rows(world: &mut World, prefix: &str) -> Vec<Vec<String>> {
        let rows: Vec<Entity> = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .filter(|(_, name)| name.as_str().starts_with(prefix))
            .map(|(entity, _)| entity)
            .collect();
        rows.into_iter()
            .map(|row| texts_under(world, row))
            .collect()
    }

    /// The texts of the one node named `name`, if it stands.
    fn named_row(world: &mut World, name: &str) -> Option<Vec<String>> {
        let row = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)?;
        Some(texts_under(world, row))
    }

    /// The icon rows of the SECTOR block.
    fn sector_rows(world: &mut World) -> Vec<Vec<String>> {
        let block = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, name)| name.as_str() == "MapChartSector")
            .map(|(entity, _)| entity)
            .expect("world chart: the panel holds the SECTOR block");
        let Some(children) = world.get::<Children>(block) else {
            return Vec::new();
        };
        let rows: Vec<Entity> = children.iter().collect();
        rows.into_iter()
            .map(|row| texts_under(world, row))
            .collect()
    }

    /// Whether a text reads how long ago a find was.
    fn relative(text: &str) -> bool {
        text == "just now" || text.ends_with(" ago")
    }

    /// Claim 5: the discovery log in the panel lists the station's find and
    /// the spawn's cell, each an icon row with its cell and how long ago.
    fn assert_the_discovery_log(world: &mut World) {
        let code = station_code(world);
        let cell = world
            .resource::<ChartWatch>()
            .cell
            .expect("world chart: the station's cell is chosen");
        let rows = named_rows(world, "MapChartLog");
        let station_row = rows
            .iter()
            .find(|row| {
                row.iter().any(|text| text.starts_with(&code))
                    && row.contains(&cell.to_string())
                    && row.iter().any(|text| relative(text))
            })
            .unwrap_or_else(|| panic!("world chart: the log lists {code} in {cell}: {rows:?}"))
            .clone();
        let start_cell = world
            .resource::<ChartWatch>()
            .start_cell
            .expect("world chart: the start's cell is recorded");
        let origin_row = rows
            .iter()
            .find(|row| {
                row.contains(&start_cell.to_string()) && row.iter().any(|text| relative(text))
            })
            .unwrap_or_else(|| panic!("world chart: the log lists the start's cell: {rows:?}"))
            .clone();
        nova_probe::probe_marker(
            world,
            "outcome: the discovery log lists the station",
            serde_json::json!({ "station_row": station_row, "origin_row": origin_row }),
        );
        info!("world chart: the log reads {station_row:?} and {origin_row:?}");
    }

    /// Claim 6: the leave save holds the three cells, the summary and the
    /// starting station.
    fn assert_the_saved_chart(world: &mut World) {
        let id = station_id(world);
        let expected = entered(world);
        let start = world
            .resource::<ChartWatch>()
            .start
            .clone()
            .expect("world chart: the start is recorded");
        let summary = world
            .resource::<ChartWatch>()
            .summary
            .clone()
            .expect("recorded");
        let root = nova_assets::storage::worlds_root()
            .expect("world chart: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("the world name has a slug");
        let (_folder, _lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| panic!("world chart: cannot open the leave save: {refusal}"));
        assert_eq!(
            visited(&state.chart),
            expected,
            "world chart: the save holds exactly the entered cells"
        );
        assert_eq!(
            state
                .chart
                .station(&id)
                .and_then(|station| station.summary.clone()),
            Some(summary),
            "world chart: the save holds the station's summary"
        );
        assert_eq!(
            state.start.id, start,
            "world chart: the save names the starting station"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save holds the chart and the start",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "cells": state.chart.visited().count(),
                "stations": state.chart.stations().len(),
                "wrecks": state.chart.wrecks().len(),
                "start": state.start.id.to_string(),
            }),
        );
        info!(
            "world chart: save generation {} (format {}) holds {} cells, {} station(s), {} \
             wreck(s)",
            header.generation,
            header.format,
            state.chart.visited().count(),
            state.chart.stations().len(),
            state.chart.wrecks().len()
        );
    }

    /// Claim 7: the loaded chart holds the same cells and summary, and the
    /// session the same starting station.
    fn assert_the_loaded_chart(world: &mut World) {
        let id = station_id(world);
        let expected = entered(world);
        let start = world
            .resource::<ChartWatch>()
            .start
            .clone()
            .expect("world chart: the start is recorded");
        assert_eq!(
            world.resource::<OpenWorldSession>().start.id,
            start,
            "world chart: the Load restored the starting station"
        );
        let summary = world
            .resource::<ChartWatch>()
            .summary
            .clone()
            .expect("recorded");
        let chart = world.resource::<Chart>().clone();
        assert_eq!(
            visited(&chart),
            expected,
            "world chart: the Load restored exactly the entered cells"
        );
        assert_eq!(
            chart
                .station(&id)
                .and_then(|station| station.summary.clone()),
            Some(summary),
            "world chart: the Load restored the station's summary"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a loaded world restores the chart",
            serde_json::json!({
                "cells": chart.visited().count(),
                "stations": chart.stations().len(),
                "wrecks": chart.wrecks().len(),
            }),
        );
        info!(
            "world chart: PASS the loaded chart holds {} cells and '{id}'",
            chart.visited().count()
        );
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("world chart: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "world chart: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("world chart: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "world chart: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "world chart: pick the start role",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "world chart: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("world chart: the window streams in around the start")
            .until(and(
                window_is(player_cell),
                the_chart_has_visited(player_cell),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("world chart: the start docks the player at its starting station")
            .until(and(player_docked(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("world chart: assert the start")
            .on_enter(assert_the_start)
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "world chart: open the Station pane",
            "InterfaceTabStation",
            the_interface_shows(Some(InterfacePaneType::Station)),
            BEAT_SECS,
        )
        .step("world chart: settle the Station pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("world chart: shoot the Station pane at the start")
        .on_enter(|world: &mut World| shoot(world, START_SHOT))
        .until(shot_written(START_SHOT))
        .deadline(BEAT_SECS)
        .add()
        .click_named(
            "world chart: open the Jobs pane",
            "InterfaceTabJobs",
            and(ui_node_present("JobAccept-0"), frames(SETTLE_FRAMES)),
            BEAT_SECS,
        )
        // The rows can sit below the panel's fold: the Accept is activated
        // by name.
        .step("world chart: accept the starting station's procurement")
        .on_enter(|world: &mut World| {
            let row = starter_procurement_row(world);
            activate_named(world, &format!("JobAccept-{row}"));
        })
        .until(and(held_count(1), frames(SETTLE_FRAMES)))
        .deadline(BEAT_SECS)
        .add()
        .click_named(
            "world chart: open the Map pane at the start",
            "InterfaceTabMap",
            the_interface_shows(Some(InterfacePaneType::Map)),
            BEAT_SECS,
        )
        .step("world chart: settle the map at the start")
        .until(frames(SETTLE_FRAMES))
        .add();
        let script = tap(
            script,
            "the chart key at the start",
            "map_chart",
            ui_node_present(format!("{OBJECTIVE_ROW_PREFIX}JOB-1")),
        )
        .step("world chart: settle the objectives")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("world chart: assert the objectives")
        .on_enter(assert_the_objectives)
        .add()
        .step("world chart: shoot the objectives")
        .on_enter(|world: &mut World| shoot(world, OBJECTIVES_SHOT))
        .until(shot_written(OBJECTIVES_SHOT))
        .deadline(BEAT_SECS)
        .add();
        let script = tap(
            script,
            "the chart key to close the chart",
            "map_chart",
            objectives_show(false),
        );
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("world chart: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("world chart: jump to the waystation's cell")
            .on_enter(jump_to_the_waystation_cell)
            .until(and(
                window_is(|_| Some(waystation_cell())),
                other_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("world chart: choose the station")
            .on_enter(choose_the_station)
            .add()
            .step("world chart: jump to the station's side")
            .on_enter(park_beside)
            .until(and(
                and(
                    window_is(|world| world.resource::<ChartWatch>().cell),
                    the_chart_has_visited(|world| world.resource::<ChartWatch>().cell),
                ),
                the_chart_found_the_station(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("world chart: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(and(player_docked(), the_chart_holds_a_summary()))
            .deadline(SESSION_SECS)
            .add()
            .step("world chart: undock")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("world chart: assert the station's summary")
            .on_enter(assert_the_summary)
            .add()
            .step("world chart: cross into the third cell")
            .on_enter(cross_sector_edge)
            .until(and(
                window_is(|world| world.resource::<ChartWatch>().third),
                the_chart_has_visited(|world| world.resource::<ChartWatch>().third),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("world chart: assert the visited cells")
            .on_enter(assert_the_visited_cells)
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "world chart: open the Map pane",
            "InterfaceTabMap",
            the_interface_shows(Some(InterfacePaneType::Map)),
            BEAT_SECS,
        )
        .step("world chart: settle the map")
        .until(frames(SETTLE_FRAMES))
        .add();
        let script = tap(
            script,
            "the chart key",
            "map_chart",
            a_text_reads(|text| text.starts_with("STN-")),
        )
        .step("world chart: settle the chart view")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("world chart: assert the discovery log")
        .on_enter(assert_the_discovery_log)
        .add()
        // The label's name holds the code minted from the found order; the
        // walk's station is not the first found if the spawn's window held
        // another station.
        .step("world chart: select the station on the chart")
        .on_enter(|world: &mut World| {
            let code = station_code(world);
            let target = format!("MapChartMark{code}");
            let label = world
                .query::<(Entity, &Name)>()
                .iter(world)
                .find(|(_, name)| name.as_str() == target)
                .map(|(entity, _)| entity)
                .unwrap_or_else(|| panic!("world chart: no label named '{target}'"));
            world.trigger(bevy::ui_widgets::Activate { entity: label });
        })
        .until(a_text_reads(|text| text.contains("Last seen docked.")))
        .deadline(BEAT_SECS)
        .add()
        .step("world chart: settle the panel")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("world chart: assert the chart view")
        .on_enter(assert_the_chart_view)
        .add()
        .step("world chart: shoot the chart view")
        .on_enter(|world: &mut World| shoot(world, CHART_SHOT))
        .until(shot_written(CHART_SHOT))
        .deadline(BEAT_SECS)
        .add();
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("world chart: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("world chart: let escape go")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "world chart: click Back to Main Menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("world chart: assert the leave save")
            .on_enter(assert_the_saved_chart)
            .add()
            .click_named(
                "world chart: click Load",
                LOAD_BUTTON,
                ui_node_present(format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                )),
                BEAT_SECS,
            )
            .click_named(
                "world chart: select the world",
                &format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                ),
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "world chart: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("world chart: the loaded window streams in")
            .until(and(
                window_is(|world| world.resource::<ChartWatch>().third),
                the_chart_found_the_station(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("world chart: assert the loaded chart")
            .on_enter(assert_the_loaded_chart)
            .add()
    }
}
