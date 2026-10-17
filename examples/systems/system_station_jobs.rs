//! system_station_jobs: a docked station posts a seeded job board, an accept
//! holds the job, the Map's chart view marks it, a save and a Load keep it,
//! and a hand-in pays the reward.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the window stream in
//! around the spawn's cell. It picks the nearest living, non-Enemy station
//! outside that cell, jumps beside it, and is put on a berth at its collar
//! each frame until the dock takes; the walk never flies the approach
//! (`system_station_docking` does). On the Station tab it reads the board,
//! accepts the lightest procurement, undocks, reads the held job in the
//! Map's chart view, leaves to the main menu, reads the leave save off disk
//! through [`open_world`], loads the world again through the Load screen,
//! docks again, bumps the play-time clock past the offer window and reads
//! the two unheld slots renew while the held slot and job stand pinned, then
//! undocks, retires the station's cell, bumps the clock again and jumps back
//! to read the thaw renew the same two slots and despawn their old postings'
//! bodies, with the held job still pinned. It docks once more, fills the
//! hold with the goods, hands the job in and reads the hand-in's slot renew
//! under a new posting. The pinned job is the procurement: it has no body in
//! the world to lose while the player is away (an escort's hauler falls to
//! its raiders unescorted, which drops the job by design).
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a docked station posts a seeded board` | the station root carries [`JOBS_PER_BOARD`] valid jobs of its own, one of them a procurement, and the Station tab lists every title |
//! | 2 | `outcome: an accept holds the job` | after the Accept click the job is held, gone from the posted list and listed under Your jobs |
//! | 3 | `outcome: the chart view marks the held job` | undocked, the chart view labels the job `JOB-1`; a click shows HELD JOB, its title and its reward |
//! | 4 | `outcome: the leave save holds the job` | the leave save, read with [`open_world`], holds the one job and no completed id |
//! | 5 | `outcome: a loaded world restores the job and the board` | after the Load the job is held again and the station posts the same board |
//! | 6 | `outcome: a passed window renews the unheld offers and pins the held job` | docked again, once the clock passes the offer window the two unheld slots renew, stamped at the clock, and the tab lists them with their deadline, while the held slot and job stand unchanged |
//! | 7 | `outcome: a thaw renews only the unheld offers` | after the station's cell retires and thaws with the clock again past the offer window, the two unheld slots renew and their old postings' bodies are gone, while the held slot still carries the pinned job |
//! | 8 | `outcome: a hand-in pays the reward and completes the job` | docked a third time with the goods, the Hand in click takes the goods, pays the reward from the station and lists the job completed |
//! | 9 | `outcome: a hand-in renews the slot under a new posting` | the hand-in's slot posts a new job one posting above the old, the other two slots hold, and the tab lists the new offer with its deadline |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example system_station_jobs --features debug
//! # look for: `station jobs: PASS the thaw renewed`,
//! #           `station jobs: PASS the hand-in paid`,
//! #           `station jobs: PASS the hand-in renewed`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_CAPTURE=1` also writes `station-jobs-board.png` of the Station tab,
//! `station-jobs-map.png` of the chart view and `station-jobs-renewed.png`
//! of the Station tab after the passed-window renewal.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_jobs")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed, dock at a living generated station, accept a job off \
             its board, read it in the Map's chart view, check it through a save and a Load, \
             and hand it in. Autopilot-only correctness range",
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
        // No frame-time claim: the range holds the board and the job's path.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::JobWatchPlugin);
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
    use nova_world::prelude::{desired_sectors, SectorClusterTypes, SectorRoot, WorldConfig};

    use super::*;

    /// The seed the range plays. Its window around the spawn holds one
    /// station, a living Neutral waystation in cell (1, 0, 0), 48 km out
    /// (`system_world_chart` docks at the same one).
    const STATION_SEED: u32 = 10;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Jobs World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    /// The start role the walk picks: the transport's light freighter, a trade
    /// start whose boards lean to procurements, with a hold for the goods.
    const START_ROLE_BUTTON: &str = "Start Role Button: transport";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";
    const STATION_TAB: &str = "InterfaceTabStation";
    const MAP_TAB: &str = "InterfaceTabMap";

    /// The frames the capture path writes.
    const BOARD_SHOT: &str = "station-jobs-board.png";
    const MAP_SHOT: &str = "station-jobs-map.png";
    const RENEWED_SHOT: &str = "station-jobs-renewed.png";

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
    struct JobWatch {
        /// The chosen station's typed id.
        id: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// The cell the retire jump lands in, past the window around `cell`.
        away: Option<SectorCoord>,
        /// Where it stood when chosen.
        position: Option<Meters3>,
        /// The board as the station posted it, updated each time a renewal
        /// is asserted so the next claim reads the board it renewed from.
        board: Vec<Job>,
        /// The job the walk accepts, and its row on the posted list.
        job: Option<(usize, Job)>,
        /// The player's and the station's balances before the hand-in.
        before_cr: Option<(u32, u32)>,
        /// The play seconds the last clock bump landed on: a renewal it
        /// causes stamps no earlier.
        bumped_to: Option<f64>,
    }

    pub(super) struct JobWatchPlugin;

    impl Plugin for JobWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<JobWatch>();
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
        the_player(world).expect("station jobs: exactly one player ship")
    }

    /// The entity wearing the chosen station's id, if one does.
    fn station_root(world: &World) -> Option<Entity> {
        let id = world.resource::<JobWatch>().id.clone()?;
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| **worn == id)
            .map(|(entity, _)| entity)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station jobs: the station stands")
    }

    fn station_id(world: &World) -> GeneratedBodyId {
        world
            .resource::<JobWatch>()
            .id
            .clone()
            .expect("station jobs: the station is chosen")
    }

    fn the_job(world: &World) -> (usize, Job) {
        world
            .resource::<JobWatch>()
            .job
            .clone()
            .expect("station jobs: the job is chosen")
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

    fn the_station_posts_a_board() -> Arc<Predicate> {
        when(|world| {
            station_root(world).is_some_and(|station| world.get::<JobBoard>(station).is_some())
        })
    }

    fn the_job_is_held() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<JobWatch>()
                .job
                .as_ref()
                .is_some_and(|(_, job)| world.resource::<Jobs>().holds(&job.id))
        })
    }

    fn the_job_is_completed() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<JobWatch>()
                .job
                .as_ref()
                .is_some_and(|(_, job)| world.resource::<Jobs>().is_completed(&job.id))
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

    /// Whether a shown text is the posted row's detail `Reward <n> cr,
    /// <deadline>` for `reward_cr`. The pane groups thousands with commas,
    /// so the digits are compared with the commas stripped.
    fn reward_detail_shown(shown: &[String], reward_cr: u32, deadline: &str) -> bool {
        let tail = format!(" cr, {deadline}");
        shown.iter().any(|text| {
            text.strip_prefix("Reward ")
                .and_then(|rest| rest.strip_suffix(&tail))
                .is_some_and(|amount| amount.replace(',', "") == reward_cr.to_string())
        })
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
        info!("station jobs: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// spawn, in the station's own cell.
    fn park_beside(world: &mut World) {
        let watch = world.resource::<JobWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_spawn = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_spawn * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            watch.cell.expect("the station is chosen"),
            "station jobs: the park point stands in the station's cell"
        );
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("station jobs: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("station jobs: a section's chain reaches its root")
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
            .unwrap_or_else(|| panic!("station jobs: {root:?} has no free docking port"))
    }

    /// The free port of `target` that faces most squarely away from its
    /// root, so the berth sets the player outside the hull.
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
            .unwrap_or_else(|| panic!("station jobs: {target:?} has no free docking port"))
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

    /// Activate the UI node named `name`, as a click on it would.
    fn activate_named(world: &mut World, name: &str) {
        let target = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("station jobs: no node named '{name}'"));
        world.trigger(bevy::ui_widgets::Activate { entity: target });
    }

    fn set_field_text(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("station jobs: no field named '{name}'"));
        value.0 = text.to_string();
    }

    /// Pick the nearest living, non-Enemy station outside the spawn's cell.
    fn choose_the_station(world: &mut World) {
        let (edge, radius) = world_shape(world);
        let mut stations: Vec<(f32, GeneratedBodyId, Meters3)> = world
            .query_filtered::<(
                &GeneratedBodyId,
                &Position,
                Has<DerelictShipMarker>,
                Option<&Allegiance>,
            ), With<StationMarker>>()
            .iter(world)
            .filter(|(_, position, derelict, allegiance)| {
                !derelict
                    && *allegiance != Some(&Allegiance::Enemy)
                    && SectorCoord::containing(Meters3::from_engine(position.0), edge)
                        != SectorCoord::ORIGIN
            })
            .map(|(id, position, ..)| {
                let position = Meters3::from_engine(position.0);
                (position.distance(Meters3::ZERO).get(), id.clone(), position)
            })
            .collect();
        stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let Some((distance, id, position)) = stations.into_iter().next() else {
            panic!(
                "station jobs: the window on seed {STATION_SEED} holds no living, non-Enemy \
                 station outside the spawn's cell"
            );
        };
        let cell = SectorCoord::containing(position, edge);
        // The retire cell: past the far face along the station's farthest
        // axis, as `system_station_restock` chooses it.
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
            "station jobs: the window around {away} leaves {cell} out"
        );
        info!("station jobs: chose '{id}' in cell {cell}, {distance:.0} m out");
        let mut watch = world.resource_mut::<JobWatch>();
        watch.id = Some(id);
        watch.cell = Some(cell);
        watch.away = Some(away);
        watch.position = Some(position);
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("station jobs: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station jobs: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Open the interface and its Station tab.
    fn open_the_station_tab(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "station jobs: open the Station tab",
            STATION_TAB,
            the_interface_shows(Some(InterfacePaneType::Station)),
            BEAT_SECS,
        )
        .step("station jobs: settle the Station tab")
        .until(frames(SETTLE_FRAMES))
        .add()
    }

    /// Claim 1: the station posts its board, and the tab lists it.
    fn assert_the_board(world: &mut World) {
        let id = station_id(world);
        let station = station(world);
        let seed = world.resource::<WorldConfig<NovaLayeredWorld>>().seed;
        let root = world
            .get::<ChildOf>(station)
            .expect("station jobs: a generated station stands under its cell root")
            .parent();
        let clusters = world
            .get::<SectorClusterTypes>(root)
            .expect("station jobs: a cell root carries its cluster types")
            .0
            .clone();
        let items = world.resource::<GameItems>().clone();
        let board = world
            .get::<JobBoard>(station)
            .expect("station jobs: the station posts a board")
            .0
            .clone();
        assert_eq!(
            board.len(),
            JOBS_PER_BOARD as usize,
            "station jobs: the board posts {JOBS_PER_BOARD} jobs"
        );
        for job in &board {
            job.validate().expect("a posted job is valid");
            assert_eq!(
                job.posted_by(),
                &id,
                "station jobs: the station posts its own jobs"
            );
        }
        assert!(
            board.iter().any(|job| job.kind.is_procurement()),
            "station jobs: the trade start's board on seed {seed} posts a procurement: {board:?}"
        );
        let shown = texts(world);
        for job in &board {
            let title = job.title(&items);
            assert!(
                shown.contains(&title),
                "station jobs: the Station tab lists '{title}': {shown:?}"
            );
        }
        // The lightest procurement, so the hold takes its goods at the end.
        let (index, job) = board
            .iter()
            .enumerate()
            .filter_map(|(index, job)| match &job.kind {
                JobType::Procure { item, count } => {
                    Some((index, job, items.design(item).stack_mass_g(*count)))
                }
                _ => None,
            })
            .min_by_key(|(_, _, mass_g)| *mass_g)
            .map(|(index, job, _)| (index, job))
            .map(|(index, job)| (index, job.clone()))
            .expect("the board has a job");
        nova_probe::probe_marker(
            world,
            "outcome: a docked station posts a seeded board",
            serde_json::json!({
                "station": id.to_string(),
                "seed": seed,
                "clusters": format!("{clusters:?}"),
                "jobs": board.iter().map(|job| job.title(&items)).collect::<Vec<_>>(),
                "rewards": board.iter().map(|job| job.reward_cr).collect::<Vec<_>>(),
                "chosen": index,
            }),
        );
        info!(
            "station jobs: '{id}' posts {} job(s); the walk takes row {index}: {} for {} cr",
            board.len(),
            job.title(&items),
            job.reward_cr
        );
        let mut watch = world.resource_mut::<JobWatch>();
        watch.board = board;
        watch.job = Some((index, job));
    }

    /// Claim 2: the accept holds the job, and the lists moved it.
    fn assert_the_accept(world: &mut World) {
        let (_, job) = the_job(world);
        let items = world.resource::<GameItems>().clone();
        let jobs = world.resource::<Jobs>().clone();
        assert_eq!(jobs.held(), &[job.clone()], "station jobs: the job is held");
        assert!(
            jobs.completed().is_empty(),
            "station jobs: nothing is completed"
        );
        let title = job.title(&items);
        let shown = texts(world);
        assert_eq!(
            shown.iter().filter(|text| **text == title).count(),
            1,
            "station jobs: the title is listed once, under Your jobs: {shown:?}"
        );
        assert!(
            shown.contains(&"Your jobs".to_string()),
            "station jobs: the Your jobs header shows: {shown:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: an accept holds the job",
            serde_json::json!({ "job": job.id.to_string(), "title": title }),
        );
        info!("station jobs: holds '{}' ({title})", job.id);
    }

    /// Claim 3: the chart view labels the job and its panel reads it.
    fn assert_the_chart_view(world: &mut World) {
        let (_, job) = the_job(world);
        let items = world.resource::<GameItems>().clone();
        let title = job.title(&items);
        let shown = texts(world);
        assert!(
            shown.contains(&"JOB-1".to_string()),
            "station jobs: the chart view labels the job JOB-1: {shown:?}"
        );
        assert!(
            shown.contains(&"HELD JOB".to_string()),
            "station jobs: the panel says HELD JOB: {shown:?}"
        );
        assert!(
            shown.contains(&title),
            "station jobs: the panel names the job: {shown:?}"
        );
        let reward = shown
            .iter()
            .find(|text| text.starts_with("Reward ") && text.ends_with(" cr"))
            .cloned()
            .unwrap_or_else(|| panic!("station jobs: the panel shows the reward: {shown:?}"));
        assert!(
            reward.contains(&(job.reward_cr % 1000).to_string()),
            "station jobs: the reward line reads {} cr: {reward:?}",
            job.reward_cr
        );
        nova_probe::probe_marker(
            world,
            "outcome: the chart view marks the held job",
            serde_json::json!({ "code": "JOB-1", "title": title, "reward": reward }),
        );
        info!("station jobs: the chart view shows JOB-1: {title}, {reward}");
    }

    /// Claim 4: the leave save holds the job and its slot's first posting.
    fn assert_the_saved_jobs(world: &mut World) {
        let (_, job) = the_job(world);
        let station = station_id(world);
        let root = nova_assets::storage::worlds_root()
            .expect("station jobs: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("the world name has a slug");
        let (_folder, _lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| panic!("station jobs: cannot open the leave save: {refusal}"));
        assert_eq!(
            state.jobs.held(),
            &[job.clone()],
            "station jobs: the save holds the job"
        );
        assert!(
            state.jobs.completed().is_empty(),
            "station jobs: the save lists nothing completed"
        );
        let board = state
            .postings
            .board(&station)
            .expect("station jobs: the save holds the station's postings");
        assert_eq!(
            board.len(),
            JOBS_PER_BOARD as usize,
            "station jobs: the save holds {JOBS_PER_BOARD} slot postings"
        );
        assert!(
            board.iter().all(|slot| slot.posting == 0),
            "station jobs: every slot still stands on its first posting: {board:?}"
        );
        assert!(
            state.postings.slot(&job.id).is_some(),
            "station jobs: the save holds the accepted job's slot posting"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save holds the job",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "held": state.jobs.held().len(),
                "postings": JOBS_PER_BOARD,
            }),
        );
        info!(
            "station jobs: save generation {} (format {}) holds '{}'",
            header.generation, header.format, job.id
        );
    }

    /// Claim 5: the Load restores the job, and the station posts the same
    /// board.
    fn assert_the_loaded_jobs(world: &mut World) {
        let (_, job) = the_job(world);
        let board = world.resource::<JobWatch>().board.clone();
        let jobs = world.resource::<Jobs>().clone();
        assert_eq!(
            jobs.held(),
            &[job.clone()],
            "station jobs: the Load restored the job"
        );
        let station = station(world);
        let posted = world
            .get::<JobBoard>(station)
            .expect("station jobs: the loaded station posts a board")
            .0
            .clone();
        assert_eq!(
            posted, board,
            "station jobs: the loaded station posts the same board"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a loaded world restores the job and the board",
            serde_json::json!({ "job": job.id.to_string(), "board": posted.len() }),
        );
        info!("station jobs: the Load restored '{}' and the board", job.id);
    }

    /// Fill the hold with the job's goods and note both balances.
    fn give_the_goods(world: &mut World) {
        let (_, job) = the_job(world);
        let JobType::Procure { item, count } = job.kind.clone() else {
            panic!("station jobs: the walked job is a procurement: {job:?}")
        };
        let items = world.resource::<GameItems>().clone();
        let player = player(world);
        let station = station(world);
        let mut hold = world
            .get_mut::<ShipInventory>(player)
            .expect("station jobs: the player has a hold");
        let short = count.saturating_sub(hold.count(&item));
        if short > 0 {
            assert!(
                items.design(&item).stack_mass_g(short) <= u64::from(hold.free_g(&items)),
                "station jobs: the hold takes {short} of {item}"
            );
            hold.add(&items, &item, short);
        }
        let own_cr = world.get::<ShipCredits>(player).expect("credits").0;
        let station_cr = world.get::<ShipCredits>(station).expect("credits").0;
        assert!(
            station_cr >= job.reward_cr,
            "station jobs: the station holds {station_cr} cr, the job pays {}",
            job.reward_cr
        );
        world.resource_mut::<JobWatch>().before_cr = Some((own_cr, station_cr));
        info!(
            "station jobs: the hold carries {count} {item}; player {own_cr} cr, station \
             {station_cr} cr"
        );
    }

    /// Claim 8: the hand-in took the goods, paid the reward and completed
    /// the job.
    fn assert_the_hand_in(world: &mut World) {
        let (_, job) = the_job(world);
        let JobType::Procure { item, count } = job.kind.clone() else {
            panic!("station jobs: the walked job is a procurement: {job:?}")
        };
        let (own_before, station_before) =
            world.resource::<JobWatch>().before_cr.expect("recorded");
        let player = player(world);
        let station = station(world);
        let jobs = world.resource::<Jobs>().clone();
        assert!(jobs.held().is_empty(), "station jobs: nothing is held");
        assert_eq!(
            jobs.completed(),
            &[job.id.clone()],
            "station jobs: the job is completed"
        );
        let own_cr = world.get::<ShipCredits>(player).expect("credits").0;
        let station_cr = world.get::<ShipCredits>(station).expect("credits").0;
        assert_eq!(
            own_cr,
            own_before + job.reward_cr,
            "station jobs: the player was paid"
        );
        assert_eq!(
            station_cr,
            station_before - job.reward_cr,
            "station jobs: the station paid"
        );
        let held = world
            .get::<ShipInventory>(player)
            .expect("hold")
            .count(&item);
        assert_eq!(held, 0, "station jobs: the goods left the hold");
        let shown = texts(world);
        let items = world.resource::<GameItems>().clone();
        let title = job.title(&items);
        assert!(
            !shown.contains(&title),
            "station jobs: the completed job is off both lists: {shown:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a hand-in pays the reward and completes the job",
            serde_json::json!({
                "job": job.id.to_string(),
                "item": item.to_string(),
                "count": count,
                "reward_cr": job.reward_cr,
                "player_cr": own_cr,
                "station_cr": station_cr,
            }),
        );
        info!(
            "station jobs: PASS the hand-in paid {} cr for {count} {item}",
            job.reward_cr
        );
    }

    /// Advance once the hand-in's slot carries a job one posting above the
    /// one the walk accepted.
    fn the_slot_renews() -> Arc<Predicate> {
        when(|world| {
            let (_, job) = the_job(world);
            station_root(world).is_some_and(|station| {
                world.get::<JobBoard>(station).is_some_and(|board| {
                    board
                        .0
                        .get(job.id.slot as usize)
                        .is_some_and(|posted| posted.id.posting == job.id.posting + 1)
                })
            })
        })
    }

    /// Advance once every board slot but the held one carries a posting one
    /// above `JobWatch.board`'s: the renewal a passed offer window or a
    /// thaw gives the slots the player does not hold.
    fn unheld_slots_renewed() -> Arc<Predicate> {
        when(|world| {
            let before = world.resource::<JobWatch>().board.clone();
            let held_slot = world
                .resource::<Jobs>()
                .held()
                .first()
                .map(|job| job.id.slot);
            station_root(world).is_some_and(|station| {
                world.get::<JobBoard>(station).is_some_and(|board| {
                    board.0.len() == before.len()
                        && board.0.iter().enumerate().all(|(slot, posted)| {
                            held_slot == Some(slot as u32)
                                || posted.id.posting == before[slot].id.posting + 1
                        })
                })
            })
        })
    }

    /// Claim 9: the hand-in's slot renews under a new posting; the other
    /// two stand unchanged.
    fn assert_the_renewed_slot(world: &mut World) {
        let (_, job) = the_job(world);
        let before = world.resource::<JobWatch>().board.clone();
        let items = world.resource::<GameItems>().clone();
        let rule = world.resource::<GamePriceTable>().0.jobs;
        let clock = world.resource::<WorldClock>().play_seconds;
        let station = station(world);
        let board = world
            .get::<JobBoard>(station)
            .expect("station jobs: the station posts a board")
            .0
            .clone();
        assert_eq!(
            board.len(),
            JOBS_PER_BOARD as usize,
            "station jobs: the renewed board still posts {JOBS_PER_BOARD} jobs"
        );
        for (slot, old) in before.iter().enumerate() {
            if slot as u32 == job.id.slot {
                continue;
            }
            assert_eq!(
                &board[slot], old,
                "station jobs: slot {slot} is unchanged by the hand-in"
            );
        }
        let new = board[job.id.slot as usize].clone();
        let expected = JobId {
            station: job.id.station.clone(),
            slot: job.id.slot,
            posting: job.id.posting + 1,
        };
        assert_eq!(
            new.id, expected,
            "station jobs: the renewed slot posts under the next posting"
        );
        let posting = world
            .resource::<JobPostings>()
            .slot(&new.id)
            .copied()
            .unwrap_or_else(|| panic!("station jobs: JobPostings records '{}'", new.id));
        assert!(
            posting.posted_at <= clock,
            "station jobs: the renewal stamps no later than the clock: {} > {clock}",
            posting.posted_at
        );
        let remaining = posting.remaining(rule, clock);
        assert!(
            remaining > 0.0,
            "station jobs: the renewed offer has time left: {remaining}"
        );
        let title = new.title(&items);
        let deadline = offer_deadline_text(remaining);
        let shown = texts(world);
        assert!(
            shown.contains(&title),
            "station jobs: the Station tab lists '{title}': {shown:?}"
        );
        assert!(
            reward_detail_shown(&shown, new.reward_cr, &deadline),
            "station jobs: the Station tab reads 'Reward {} cr, {deadline}': {shown:?}",
            new.reward_cr
        );
        nova_probe::probe_marker(
            world,
            "outcome: a hand-in renews the slot under a new posting",
            serde_json::json!({
                "old": job.id.to_string(),
                "new": new.id.to_string(),
                "title": title,
                "deadline": deadline,
            }),
        );
        info!(
            "station jobs: PASS the hand-in renewed slot {}: '{}' -> '{}' ({title}), {deadline}",
            job.id.slot, job.id, new.id
        );
    }

    /// DISCLOSED FIXTURE STATE: the Station pane holds the game paused, so
    /// nothing but this step moves play time across the offer window.
    fn bump_the_clock_past_offer(world: &mut World) {
        let offer_seconds = world.resource::<GamePriceTable>().0.jobs.offer_seconds;
        let mut clock = world.resource_mut::<WorldClock>();
        clock.play_seconds += f64::from(offer_seconds) + 1.0;
        let bumped_to = clock.play_seconds;
        info!(
            "station jobs: the clock advances past the {offer_seconds:.0} s offer window, to \
             {bumped_to:.0} s of play"
        );
        world.resource_mut::<JobWatch>().bumped_to = Some(bumped_to);
    }

    /// Claim 6: a passed offer window renews the two unheld slots and pins
    /// the held job's slot.
    fn assert_the_passed_window(world: &mut World) {
        let (_, held) = the_job(world);
        let before = world.resource::<JobWatch>().board.clone();
        let bumped_to = world.resource::<JobWatch>().bumped_to.expect("bumped");
        let items = world.resource::<GameItems>().clone();
        let rule = world.resource::<GamePriceTable>().0.jobs;
        let clock = world.resource::<WorldClock>().play_seconds;
        let jobs = world.resource::<Jobs>().clone();
        assert_eq!(
            jobs.held(),
            &[held.clone()],
            "station jobs: the walked job is the one held"
        );
        let station = station(world);
        let board = world
            .get::<JobBoard>(station)
            .expect("station jobs: the station posts a board")
            .0
            .clone();
        assert_eq!(
            board[held.id.slot as usize], held,
            "station jobs: the held slot keeps its job"
        );
        let deadline = offer_deadline_text(f64::from(rule.offer_seconds));
        let mut renewed_ids = Vec::new();
        for (slot, old) in before.iter().enumerate() {
            if slot as u32 == held.id.slot {
                continue;
            }
            let new = board[slot].clone();
            assert_eq!(
                new.id.posting,
                old.id.posting + 1,
                "station jobs: slot {slot} renews once past the offer window"
            );
            let posting = world
                .resource::<JobPostings>()
                .slot(&new.id)
                .copied()
                .unwrap_or_else(|| panic!("station jobs: JobPostings records '{}'", new.id));
            assert!(
                (bumped_to..=clock).contains(&posting.posted_at),
                "station jobs: slot {slot} stamps between the bump ({bumped_to}) and the clock \
                 ({clock}): {}",
                posting.posted_at
            );
            let title = new.title(&items);
            let shown = texts(world);
            assert!(
                shown.contains(&title),
                "station jobs: the Station tab lists '{title}': {shown:?}"
            );
            assert!(
                reward_detail_shown(&shown, new.reward_cr, &deadline),
                "station jobs: the Station tab reads 'Reward {} cr, {deadline}': {shown:?}",
                new.reward_cr
            );
            renewed_ids.push(new.id);
        }
        let held_title = held.title(&items);
        let shown = texts(world);
        assert_eq!(
            shown.iter().filter(|text| **text == held_title).count(),
            1,
            "station jobs: the held title is listed once, under Your jobs: {shown:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a passed window renews the unheld offers and pins the held job",
            serde_json::json!({
                "held": held.id.to_string(),
                "renewed": renewed_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "deadline": deadline,
            }),
        );
        info!(
            "station jobs: the passed window renews {renewed_ids:?}, pins '{}'",
            held.id
        );
        world.resource_mut::<JobWatch>().board = board;
    }

    /// Claim 7: a thaw renews the same two unheld slots again, taking out
    /// every body the old postings could have put in the world, while the
    /// held slot and job stand pinned.
    fn assert_the_thawed_board(world: &mut World) {
        let (_, held) = the_job(world);
        let before = world.resource::<JobWatch>().board.clone();
        let bumped_to = world.resource::<JobWatch>().bumped_to.expect("bumped");
        let clock = world.resource::<WorldClock>().play_seconds;
        let jobs = world.resource::<Jobs>().clone();
        assert_eq!(
            jobs.held(),
            &[held.clone()],
            "station jobs: the walked job is still held after the thaw"
        );
        let station = station(world);
        let board = world
            .get::<JobBoard>(station)
            .expect("station jobs: the thawed station posts a board")
            .0
            .clone();
        assert_eq!(
            board[held.id.slot as usize], held,
            "station jobs: the thaw keeps the held slot's job"
        );
        let mut renewed_ids = Vec::new();
        let mut replaced_ids = Vec::new();
        for (slot, old) in before.iter().enumerate() {
            if slot as u32 == held.id.slot {
                continue;
            }
            let new = board[slot].clone();
            assert_eq!(
                new.id.posting,
                old.id.posting + 1,
                "station jobs: slot {slot} renews once on the thaw"
            );
            let posting = world
                .resource::<JobPostings>()
                .slot(&new.id)
                .copied()
                .unwrap_or_else(|| panic!("station jobs: JobPostings records '{}'", new.id));
            assert!(
                (bumped_to..=clock).contains(&posting.posted_at),
                "station jobs: slot {slot} stamps between the bump ({bumped_to}) and the thaw \
                 ({clock}): {}",
                posting.posted_at
            );
            renewed_ids.push(new.id);
            replaced_ids.push(old.id.clone());
        }
        let live_bodies: Vec<GeneratedBodyId> = world
            .query::<&GeneratedBodyId>()
            .iter(world)
            .cloned()
            .collect();
        for replaced in &replaced_ids {
            for possible in replaced.possible_bodies() {
                assert!(
                    !live_bodies.contains(&possible),
                    "station jobs: the retired posting '{replaced}' leaves no live body \
                     '{possible}'"
                );
            }
        }
        nova_probe::probe_marker(
            world,
            "outcome: a thaw renews only the unheld offers",
            serde_json::json!({
                "held": held.id.to_string(),
                "renewed": renewed_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
            }),
        );
        info!(
            "station jobs: PASS the thaw renewed {renewed_ids:?}, pinned '{}'",
            held.id
        );
        world.resource_mut::<JobWatch>().board = board;
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station jobs: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "station jobs: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("station jobs: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "station jobs: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "station jobs: pick the start role",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "station jobs: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station jobs: the window streams in around the spawn")
            .until(window_is(|_| Some(SectorCoord::ORIGIN)))
            .deadline(SESSION_SECS)
            .add()
            .step("station jobs: choose the station")
            .on_enter(choose_the_station)
            .add()
            .step("station jobs: jump to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<JobWatch>().cell),
                the_station_posts_a_board(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station jobs: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add();
        let script = open_the_station_tab(script)
            .step("station jobs: the board shows")
            .until(ui_node_present("JobAccept-0"))
            .deadline(BEAT_SECS)
            .add()
            .step("station jobs: assert the board")
            .on_enter(assert_the_board)
            .add()
            .step("station jobs: shoot the board")
            .on_enter(|world: &mut World| shoot(world, BOARD_SHOT))
            .until(shot_written(BOARD_SHOT))
            .deadline(BEAT_SECS)
            .add()
            .step("station jobs: click Accept")
            .on_enter(|world: &mut World| {
                let (index, _) = the_job(world);
                activate_named(world, &format!("JobAccept-{index}"));
            })
            .until(and(the_job_is_held(), frames(SETTLE_FRAMES)))
            .deadline(BEAT_SECS)
            .add()
            .step("station jobs: assert the accept")
            .on_enter(assert_the_accept)
            .add();
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station jobs: undock")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "station jobs: open the Map pane",
            MAP_TAB,
            the_interface_shows(Some(InterfacePaneType::Map)),
            BEAT_SECS,
        )
        .step("station jobs: settle the map")
        .until(frames(SETTLE_FRAMES))
        .add();
        let script = tap(
            script,
            "the chart key",
            "map_chart",
            a_text_reads(|text| text == "JOB-1"),
        )
        .step("station jobs: settle the chart view")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station jobs: select the job on the chart")
        .on_enter(|world: &mut World| activate_named(world, "MapChartMarkJOB-1"))
        .until(a_text_reads(|text| text == "HELD JOB"))
        .deadline(BEAT_SECS)
        .add()
        .step("station jobs: settle the panel")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station jobs: assert the chart view")
        .on_enter(assert_the_chart_view)
        .add()
        .step("station jobs: shoot the chart view")
        .on_enter(|world: &mut World| shoot(world, MAP_SHOT))
        .until(shot_written(MAP_SHOT))
        .deadline(BEAT_SECS)
        .add();
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station jobs: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("station jobs: let escape go")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "station jobs: click Back to Main Menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("station jobs: assert the leave save")
            .on_enter(assert_the_saved_jobs)
            .add()
            .click_named(
                "station jobs: click Load",
                LOAD_BUTTON,
                ui_node_present(format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                )),
                BEAT_SECS,
            )
            .click_named(
                "station jobs: select the world",
                &format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                ),
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "station jobs: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station jobs: the loaded window streams in")
            .until(and(
                window_is(|world| world.resource::<JobWatch>().cell),
                the_station_posts_a_board(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station jobs: assert the loaded jobs")
            .on_enter(assert_the_loaded_jobs)
            .add()
            .step("station jobs: berth and dock again")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add();
        let script = open_the_station_tab(script)
            .step("station jobs: bump the clock past the offer window")
            .on_enter(bump_the_clock_past_offer)
            .until(unheld_slots_renewed())
            .deadline(BEAT_SECS)
            .add()
            .step("station jobs: settle the passed window")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station jobs: assert the passed window")
            .on_enter(assert_the_passed_window)
            .add()
            .step("station jobs: shoot the renewed board")
            .on_enter(|world: &mut World| shoot(world, RENEWED_SHOT))
            .until(shot_written(RENEWED_SHOT))
            .deadline(BEAT_SECS)
            .add();
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station jobs: undock for the thaw")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station jobs: jump away until the station's cell retires")
            .on_enter(|world: &mut World| {
                let (edge, _) = world_shape(world);
                let away = world.resource::<JobWatch>().away.expect("chosen");
                park_player(world, away.centre(edge));
            })
            .until(window_is(|world| world.resource::<JobWatch>().away))
            .deadline(SESSION_SECS)
            .add()
            .step("station jobs: assert the station retired")
            .on_enter(|world: &mut World| {
                let cell = world.resource::<JobWatch>().cell.expect("chosen");
                assert!(
                    !live_cells(world).contains(&cell),
                    "station jobs: the station's cell left the live window"
                );
                assert!(
                    station_root(world).is_none(),
                    "station jobs: the retired station stands nowhere"
                );
            })
            .add()
            .step("station jobs: bump the clock past the offer window again")
            .on_enter(bump_the_clock_past_offer)
            .add()
            .step("station jobs: jump back to thaw the station")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<JobWatch>().cell),
                and(the_station_posts_a_board(), unheld_slots_renewed()),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station jobs: settle the thawed board")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station jobs: assert the thawed board")
            .on_enter(assert_the_thawed_board)
            .add()
            .step("station jobs: berth and dock a third time")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add();
        open_the_station_tab(script)
            .step("station jobs: give the goods")
            .on_enter(give_the_goods)
            .until(ui_node_present("JobHandIn-0"))
            .deadline(BEAT_SECS)
            .add()
            // The row sits below the panel's fold on a 1080p frame, out of
            // the pointer's reach: the button is activated by name.
            .step("station jobs: click Hand in")
            .on_enter(|world: &mut World| activate_named(world, "JobHandIn-0"))
            .until(and(the_job_is_completed(), frames(SETTLE_FRAMES)))
            .deadline(BEAT_SECS)
            .add()
            .step("station jobs: assert the hand-in")
            .on_enter(assert_the_hand_in)
            .add()
            .step("station jobs: wait for the hand-in to renew the slot")
            .until(the_slot_renews())
            .deadline(BEAT_SECS)
            .add()
            .step("station jobs: settle the renewed board")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station jobs: assert the renewed slot")
            .on_enter(assert_the_renewed_slot)
            .add()
    }
}
