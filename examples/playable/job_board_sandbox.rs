//! job_board_sandbox: start docked at a living station with the goods its
//! board asks for in the hold, on the Station tab, and try the jobs by hand.
//!
//! Boots the shipped app and starts New Game on a pinned seed, as the Create
//! World modal does, but without a save folder: nothing this run does is
//! saved. When the window has streamed in, it picks the nearest generated
//! station that is living, not on the Enemy side and posts jobs. The player
//! jumps beside it and is put on a berth at its collar each frame until the
//! dock takes. Then the fixture lands, once: the hold is given the goods of
//! every posted job, as far as it has room, and the Station tab opens on the
//! board.
//!
//! Try: Accept a job, close the interface with TAB, undock, open TAB > Map
//! and press the chart key: the held job is marked `JOB-1` in the cell of
//! the body it is about. A procurement hands in at once: dock again and Hand
//! in, the goods leave the hold and the reward lands on the balance. A
//! located job (recover, escort, distress, neutralize) refuses the hand-in
//! until the world marks it done: fly to the mark, do the work and come
//! back. The diagnostics panel (`F3`) lists the board with each job's task
//! and state, the jobs held and completed, and the balance.
//!
//! `NOVA_SANDBOX_ROLE=<start role id>` picks the start role, whose job mix
//! the boards lean to: the default `transport` posts procurements and
//! escorts, `scavenger` recoveries, `bounty_hunter` distress calls and
//! neutralizations.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk accepts the first procurement,
//! undocks, reads it in the chart view, docks again and hands it in.
//! `NOVA_CAPTURE=1` shoots `sandbox-jobs-board.png`, `sandbox-jobs-map.png`
//! and `sandbox-jobs-done.png`. Unset, no walk and no deadline run.

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, nova_interface::pane::InterfacePaneType, prelude::*};
use nova_world::prelude::{desired_sectors, SectorRoot, WorldConfig};

#[derive(Parser)]
#[command(name = "job_board_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start docked at a living generated station with its board's goods in the hold, on \
             the Station tab, and try the jobs by hand",
    long_about = None
)]
struct Cli;

/// The world seed: `system_station_jobs`'s, whose window around the spawn
/// holds a living Neutral station.
const SANDBOX_SEED: u32 = 10;
/// The start role the player spawns as when `NOVA_SANDBOX_ROLE` is unset:
/// the transport's boards lean to procurements, which the smoke walk needs.
const SANDBOX_ROLE: &str = "transport";

/// The start role this run plays: `NOVA_SANDBOX_ROLE`, else [`SANDBOX_ROLE`].
fn sandbox_role() -> String {
    std::env::var("NOVA_SANDBOX_ROLE").unwrap_or_else(|_| SANDBOX_ROLE.to_string())
}
/// How far from the station the player parks before the berth.
const PARK_DISTANCE: Meters = Meters(2_000.0);
/// The gap the berth leaves between the two port faces, engine units: half a
/// cell, inside the 10 m capture distance.
const BERTH_GAP: f32 = 0.5;
/// Where a port's face sits in its own frame, as the docking verb reads it.
const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

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
    app.init_resource::<Sandbox>();
    app.add_plugins((
        diagnostics::DiagnosticsHintPlugin::new(INTRO),
        jobs_overlay::JobsOverlayPlugin,
    ));
    app.add_systems(OnEnter(GameStates::MainMenu), start_the_world);
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        // The software rasterizer's frames are seconds long: the clamp
        // keeps the berth from integrating a whole frame of drift.
        app.add_systems(Update, smoke::hold_the_walk_clock);
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
    /// The world has not started yet.
    #[default]
    Menu,
    /// New Game is up; the window around the spawn is streaming in.
    Streaming,
    /// The player jumped beside the station; its window is streaming in.
    Jumped,
    /// The player is put on the berth each frame until the dock takes.
    Docking,
    /// Docked, fixture set and the Station tab open. The player has the run.
    Ready,
}

/// The sandbox's setup state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
    /// The chosen station's id, the cell it stands in and where it stood.
    station: Option<(GeneratedBodyId, SectorCoord, Meters3)>,
}

/// Start New Game on [`SANDBOX_SEED`] the first time the main menu comes up,
/// with no save folder. A later Back to Main Menu stays on the menu.
fn start_the_world(
    mut commands: Commands,
    mut sandbox: ResMut<Sandbox>,
    mut mode: ResMut<GameMode>,
    mut pick: ResMut<NewGameScenario>,
    mut state: ResMut<NextState<GameStates>>,
) {
    if sandbox.phase != SandboxPhaseType::Menu {
        return;
    }
    commands.insert_resource(OpenWorldSession {
        seed: SANDBOX_SEED,
        role: sandbox_role().as_str().into(),
    });
    pick.0 = None;
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
    sandbox.phase = SandboxPhaseType::Streaming;
    info!("job sandbox: New Game on seed {SANDBOX_SEED}, no save folder");
}

/// Advance the setup one phase at a time, each on a world value.
fn drive_the_sandbox(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    match phase {
        SandboxPhaseType::Menu | SandboxPhaseType::Ready => {}
        SandboxPhaseType::Streaming => {
            if window_is(world, Some(SectorCoord::ORIGIN)) {
                choose_the_station(world);
                park_beside(world);
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Jumped;
            }
        }
        SandboxPhaseType::Jumped => {
            let cell = world
                .resource::<Sandbox>()
                .station
                .as_ref()
                .map(|(_, cell, _)| *cell);
            let stands = station_root(world).is_some_and(|station| {
                world.get::<Position>(station).is_some()
                    && world.get::<PendingParkedHulls>(station).is_none()
                    && world.get::<JobBoard>(station).is_some()
            });
            if window_is(world, cell) && stands {
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Docking;
            }
        }
        SandboxPhaseType::Docking => {
            let Some(player) = the_player(world) else {
                return;
            };
            if world.get::<DockedShip>(player).is_some() {
                set_the_fixture(world, player);
                open_the_station_tab(world);
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Ready;
            } else {
                berth_and_dock(world, player);
            }
        }
    }
}

/// The one player ship, if exactly one stands.
fn the_player(world: &mut World) -> Option<Entity> {
    let mut players = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The entity wearing the chosen station's id, if one does.
fn station_root(world: &mut World) -> Option<Entity> {
    let (id, ..) = world.resource::<Sandbox>().station.clone()?;
    world
        .query::<(Entity, &GeneratedBodyId)>()
        .iter(world)
        .find(|(_, worn)| **worn == id)
        .map(|(entity, _)| entity)
}

/// Whether the world is armed around one player ship and the live set is
/// exactly the window around `centre`.
fn window_is(world: &mut World, centre: Option<SectorCoord>) -> bool {
    let Some(centre) = centre else {
        return false;
    };
    let Some(radius) = world
        .get_resource::<WorldConfig<NovaLayeredWorld>>()
        .map(|config| config.active_radius)
    else {
        return false;
    };
    if the_player(world).is_none() {
        return false;
    }
    let live: std::collections::BTreeSet<SectorCoord> = world
        .query::<&SectorRoot>()
        .iter(world)
        .map(|root| root.0)
        .collect();
    live == desired_sectors(centre, radius)
}

/// Pick the nearest station that is living, not on the Enemy side and posts
/// jobs, outside the spawn's cell.
fn choose_the_station(world: &mut World) {
    let edge = world
        .resource::<WorldConfig<NovaLayeredWorld>>()
        .sector_edge;
    let mut stations: Vec<(f32, GeneratedBodyId, Vec3)> = world
        .query_filtered::<(
            &GeneratedBodyId,
            &Position,
            Option<&StationServices>,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
        ), With<StationMarker>>()
        .iter(world)
        .filter(|(_, position, services, derelict, allegiance)| {
            station_offers(*services, *derelict, StationServiceType::Jobs)
                && *allegiance != Some(&Allegiance::Enemy)
                && SectorCoord::containing(Meters3::from_engine(position.0), edge)
                    != SectorCoord::ORIGIN
        })
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
            "job sandbox: the window on seed {SANDBOX_SEED} holds no living, non-Enemy station \
             posting jobs outside the spawn's cell"
        );
    };
    let position = Meters3::from_engine(at);
    let cell = SectorCoord::containing(position, edge);
    info!("job sandbox: chose '{id}' in cell {cell}, {distance:.0} m out");
    world.resource_mut::<Sandbox>().station = Some((id, cell, position));
}

/// Jump the player [`PARK_DISTANCE`] from the station toward the spawn, at
/// rest, so the station's window streams in around it.
fn park_beside(world: &mut World) {
    let Some((_, _, position)) = world.resource::<Sandbox>().station.clone() else {
        return;
    };
    let Some(player) = the_player(world) else {
        return;
    };
    let toward_spawn = (-position.get()).normalize_or_zero();
    let beside = Meters3(position.get() + toward_spawn * PARK_DISTANCE.get());
    world.entity_mut(player).insert((
        Position(beside.to_engine()),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    ));
}

/// `section`'s position and rotation in `root`'s frame, by its mount chain.
fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
    let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
    while at != root {
        let transform = world
            .get::<Transform>(at)
            .expect("job sandbox: a mounted section has a transform");
        position = transform.translation + transform.rotation * position;
        rotation = transform.rotation * rotation;
        at = world
            .get::<ChildOf>(at)
            .expect("job sandbox: a section's chain reaches its root")
            .parent();
    }
    (position, rotation)
}

/// The free, live docking ports mounted on `root`.
fn free_ports(world: &mut World, root: Entity) -> Vec<Entity> {
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
        .collect()
}

/// The free port of `target` that faces most squarely away from its root,
/// so the berth sets the player outside the hull.
fn outward_port(world: &mut World, target: Entity) -> Option<Entity> {
    free_ports(world, target)
        .into_iter()
        .map(|port| {
            let (at, rotation) = local_pose(world, port, target);
            let facing = (rotation * Vec3::NEG_Z).dot(at.normalize_or_zero());
            (port, facing)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(port, _)| port)
}

/// Put the player on the berth at the station's collar, moving and turning
/// with the station, then ask to dock.
fn berth_and_dock(world: &mut World, player: Entity) {
    let Some(station) = station_root(world) else {
        return;
    };
    let (Some(station_port), Some(player_port)) = (
        outward_port(world, station),
        free_ports(world, player).into_iter().min(),
    ) else {
        panic!("job sandbox: the station or the player has no free docking port");
    };
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

/// Give the hold the goods of every posted job, in board order, as far as
/// it has room.
fn set_the_fixture(world: &mut World, player: Entity) {
    let items = world.resource::<GameItems>().clone();
    let board = station_root(world)
        .and_then(|station| world.get::<JobBoard>(station).cloned())
        .expect("job sandbox: the station posts a board");
    let mut given: Vec<(ItemDesignId, u32)> = Vec::new();
    if let Some(mut hold) = world.get_mut::<ShipInventory>(player) {
        for job in &board.0 {
            let JobType::Procure { item, count } = &job.kind else {
                continue;
            };
            let short = count.saturating_sub(hold.count(item));
            if short == 0 {
                continue;
            }
            let unit_g = items.design(item).stack_mass_g(1).max(1);
            let room = u64::from(hold.free_g(&items)) / unit_g;
            let count = short.min(u32::try_from(room).unwrap_or(u32::MAX));
            if count > 0 {
                hold.add(&items, item, count);
                given.push((item.clone(), count));
            }
        }
    }
    info!("job sandbox: docked; goods in the hold: {given:?}");
}

/// Open the TAB interface on the Station tab, and say how to get back to it.
fn open_the_station_tab(world: &mut World) {
    *world.resource_mut::<InterfacePaneType>() = InterfacePaneType::Station;
    world
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Interface);
    info!("job sandbox: the Station tab is open; TAB closes and reopens the interface");
}

/// What the sandbox is for, for the diagnostics intro.
const INTRO: &str =
    "Docked at a station posting jobs: Accept, undock, read the Map chart, hand in.";

/// The diagnostics panel: the docked station's board, the jobs held and
/// completed, and the balance.
mod jobs_overlay {
    use super::*;

    pub(super) struct JobsOverlayPlugin;

    impl Plugin for JobsOverlayPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(
                PostUpdate,
                read_the_jobs.in_set(diagnostics::DiagnosticsSystems),
            );
        }
    }

    fn read_the_jobs(world: &mut World) {
        let Some(items) = world.get_resource::<GameItems>().cloned() else {
            return;
        };
        let jobs = world.resource::<Jobs>().clone();
        let board = station_root(world).and_then(|station| world.get::<JobBoard>(station).cloned());
        let credits =
            the_player(world).and_then(|player| world.get::<ShipCredits>(player).map(|c| c.0));
        let mut panel = world.resource_mut::<diagnostics::DiagnosticsPanel>();
        panel.line(format!(
            "Jobs: {} held, {} completed",
            jobs.held().len(),
            jobs.completed().len()
        ));
        if let Some(credits) = credits {
            panel.line(format!("Balance: {credits} cr"));
        }
        if let Some(board) = board {
            panel.line(format!("Board: {} posted", board.0.len()));
            for job in &board.0 {
                let state = if jobs.is_done(&job.id) {
                    "held, done"
                } else if jobs.holds(&job.id) {
                    "held"
                } else if jobs.is_completed(&job.id) {
                    "completed"
                } else {
                    "posted"
                };
                panel.detail(format!(
                    "{} - {} cr - {state} - {}",
                    job.title(&items),
                    job.reward_cr,
                    job.task()
                ));
            }
        }
        for job in jobs.held() {
            panel.detail(format!("held: {} -> {}", job.title(&items), job.target));
        }
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{
        press_action, release_action, shoot, AutopilotPlugin, Predicate,
    };

    use super::*;

    /// Seconds the world gets to start, stream and dock on a software GPU.
    const SETUP_SECS: f32 = 600.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Frames a pane gets to draw.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;
    const BOARD_SHOT: &str = "sandbox-jobs-board.png";
    const MAP_SHOT: &str = "sandbox-jobs-map.png";
    const DONE_SHOT: &str = "sandbox-jobs-done.png";

    /// Hold the virtual clock's clamp every frame: a scenario load hands
    /// `Time<Virtual>` back at its default.
    pub(super) fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
        let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
        if time.max_delta() != clamp {
            time.set_max_delta(clamp);
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn the_interface_shows(pane: Option<InterfacePaneType>) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && pane.is_none_or(|pane| world.get_resource::<InterfacePaneType>() == Some(&pane))
        })
    }

    fn unpaused() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
        })
    }

    fn ready_on_the_station_tab() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<Sandbox>()
                .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
                && world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && world.get_resource::<InterfacePaneType>() == Some(&InterfacePaneType::Station)
        })
    }

    fn one_player(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        players.iter(world).next()
    }

    fn player_docked(docked: bool) -> Arc<Predicate> {
        when(move |world| {
            one_player(world)
                .is_some_and(|player| world.get::<DockedShip>(player).is_some() == docked)
        })
    }

    fn held_count(count: usize) -> Arc<Predicate> {
        when(move |world| world.resource::<Jobs>().held().len() == count)
    }

    fn completed_count(count: usize) -> Arc<Predicate> {
        when(move |world| world.resource::<Jobs>().completed().len() == count)
    }

    /// Activate the UI node named `name`, as a click on it would.
    fn activate_named(world: &mut World, name: &str) {
        let target = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("job sandbox: no node named '{name}'"));
        world.trigger(bevy::ui_widgets::Activate { entity: target });
    }

    fn a_text_reads(check: impl Fn(&str) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        when(move |world| {
            world
                .try_query::<&Text>()
                .is_some_and(|mut query| query.iter(world).any(|text| check(&text.0)))
        })
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("job sandbox: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("job sandbox: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SETUP_SECS)
            .add()
    }

    /// The smoke walk: accept the first job, undock, read it in the chart
    /// view, dock again and hand it in, shooting each surface.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("job sandbox: docked on the Station tab")
            .until(ready_on_the_station_tab())
            .diagnose(|world| format!("{:?}", world.get_resource::<Sandbox>()))
            .deadline(SETUP_SECS)
            .add()
            .step("job sandbox: the board shows")
            .until(and(ui_node_present("JobAccept-0"), frames(SETTLE_FRAMES)))
            .deadline(BEAT_SECS)
            .add()
            .step("job sandbox: shoot the board")
            .on_enter(|world: &mut World| shoot(world, BOARD_SHOT))
            .until(shot_written(BOARD_SHOT))
            .deadline(SHOT_SECS)
            .add()
            // The job rows can sit below the panel's fold on a 1080p frame,
            // out of the pointer's reach: the buttons are activated by name.
            .step("job sandbox: accept the first procurement")
            .on_enter(|world: &mut World| {
                let board = station_root(world)
                    .and_then(|station| world.get::<JobBoard>(station).cloned())
                    .expect("job sandbox: the station posts a board");
                let row = board
                    .0
                    .iter()
                    .position(|job| job.kind.is_procurement())
                    .expect("job sandbox: the board posts a procurement");
                activate_named(world, &format!("JobAccept-{row}"));
            })
            .until(and(held_count(1), frames(SETTLE_FRAMES)))
            .deadline(BEAT_SECS)
            .add();
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("job sandbox: undock")
            .on_enter(|world: &mut World| {
                let player = one_player(world).expect("job sandbox: one player ship");
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_docked(false), frames(SETTLE_FRAMES)))
            .deadline(SETUP_SECS)
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "job sandbox: open the Map pane",
            "InterfaceTabMap",
            the_interface_shows(Some(InterfacePaneType::Map)),
            BEAT_SECS,
        )
        .step("job sandbox: settle the map")
        .until(frames(SETTLE_FRAMES))
        .add();
        let script = tap(
            script,
            "the chart key",
            "map_chart",
            a_text_reads(|text| text == "JOB-1"),
        )
        .click_named(
            "job sandbox: select the job on the chart",
            "MapChartMarkJOB-1",
            and(
                a_text_reads(|text| text == "HELD JOB"),
                frames(SETTLE_FRAMES),
            ),
            BEAT_SECS,
        )
        .step("job sandbox: shoot the chart view")
        .on_enter(|world: &mut World| shoot(world, MAP_SHOT))
        .until(shot_written(MAP_SHOT))
        .deadline(SHOT_SECS)
        .add();
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("job sandbox: berth and dock again")
            .each(|world, _, _| {
                let player = one_player(world).expect("job sandbox: one player ship");
                if world.get::<DockedShip>(player).is_none() {
                    berth_and_dock(world, player);
                }
            })
            .until(player_docked(true))
            .deadline(SETUP_SECS)
            .add();
        tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "job sandbox: open the Station tab",
            "InterfaceTabStation",
            and(ui_node_present("JobHandIn-0"), frames(SETTLE_FRAMES)),
            BEAT_SECS,
        )
        .step("job sandbox: hand the job in")
        .on_enter(|world: &mut World| activate_named(world, "JobHandIn-0"))
        .until(and(completed_count(1), frames(SETTLE_FRAMES)))
        .deadline(BEAT_SECS)
        .add()
        .step("job sandbox: shoot the Station tab after the hand-in")
        .on_enter(|world: &mut World| shoot(world, DONE_SHOT))
        .until(shot_written(DONE_SHOT))
        .deadline(SHOT_SECS)
        .add()
    }
}
