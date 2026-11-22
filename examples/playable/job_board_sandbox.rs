//! job_board_sandbox: start docked at a living station with the goods its
//! board asks for in the hold, on the Jobs tab, and try the jobs by hand.
//!
//! Boots the shipped app and starts New Game on a pinned seed, as the Create
//! World modal does, but without a save folder: nothing this run does is
//! saved. When the window has streamed in, it picks the nearest generated
//! station that is living, not on the Enemy side and posts jobs, or stays
//! with the starting station when the window holds no other. The player
//! jumps beside it and is put on a berth at its collar each frame until the
//! dock takes. Then the fixture lands, once: the hold is given the goods of
//! every posted job, as far as it has room, and the Jobs tab opens on the
//! board.
//!
//! Try: click a row and read it in the inspector, Accept it, close the
//! interface with TAB, undock, open TAB > Jobs: Available lists no row and
//! Hand in is gone, and selecting the held row reads where it hands in; Show
//! on Map opens the Map's chart view on the job's mark, `JOB-1` in the cell
//! of the body it is about. A procurement hands in at once: dock again and Hand in, the
//! goods leave the hold, the payout sound plays, the balance in the pane head
//! counts up to the reward with `+<reward>` beside it, and after TAB the HUD
//! notice reads `JOB-1 PAID <reward> CR`. A located job
//! (recover, escort, distress, neutralize) greys Hand in until the world
//! marks it done: fly to the mark, do the work and come back. The
//! diagnostics panel (`F3`) lists the board with each job's task and state,
//! the jobs held and completed, and the balance.
//!
//! `NOVA_SANDBOX_ROLE=<start role id>` picks the start role, whose job mix
//! the boards lean to: the default `industrial` posts procurements and
//! supply runs, `scavenger` recoveries, `bounty_hunter` neutralizations.
//! Your jobs opens with the role goal's own line; the Map's chart view
//! (`M`, then `C`) lists it under OBJECTIVES. On the pinned seed the
//! `scavenger` and the `bounty_hunter` boards post a recovery and a
//! neutralize that name their own target and the `pirate` board a recovery
//! and two neutralizes at two legs: Accept puts the wreck or the raider in
//! the world 6 to 16 km out, Show on Map opens the chart on its mark, and
//! undocked the HUD chip `JOB-n` points at it.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk first holds the start docked at its
//! starting station and reads its starter board on the Jobs tab: every slot
//! posts a job the hull can do, so every row lists. It opens the chart view
//! and reads the role goal's own line as the next milestone. As a role other
//! than the default it then accepts the board's recovery and neutralize that
//! name their own target, waits for each body to stand wearing its marker,
//! opens the chart on each mark, shuts the interface, reads the HUD chips
//! and ends there. As the default role it moves to the chosen station, checks that every
//! board row is one line with its Accept inside the board, selects each row
//! and reads the reward terms the inspector lists for it, reads and accepts
//! the first procurement and a second job at another reward, undocks,
//! selects the second held row and reads its reward, not the first's, in
//! the inspector and, after Show on Map, in the chart view. With the chart
//! open it drops the second job by hand, where the world drops one whose
//! body is lost: the chart returns to its overview, and so does the Jobs
//! inspector. It docks again and hands the procurement in: the pane head
//! shows the reward and the open world's first-delivery bonus as one
//! `+<gain>` and counts the balance up by it to the true figure, then the
//! gain goes; with the interface shut the HUD notice reads what the job paid. `NOVA_CAPTURE=1` shoots
//! `sandbox-jobs-starter.png`, `sandbox-jobs-goal.png`, as another role
//! `sandbox-jobs-spawned-recover.png`, `sandbox-jobs-spawned-neutralize.png`
//! and `sandbox-jobs-spawned-hud.png`, and as the default `sandbox-jobs-board.png`,
//! `sandbox-jobs-terms-<row>.png` for each row the board lists,
//! `sandbox-jobs-held.png`, `sandbox-jobs-map.png`,
//! `sandbox-jobs-map-dropped.png`, `sandbox-jobs-dropped.png`,
//! `sandbox-jobs-hand-in.png`, `sandbox-jobs-count.png` during the count,
//! `sandbox-jobs-done.png` after it and `sandbox-jobs-paid.png` with the
//! notice. Unset, no walk and no deadline run.

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{
    nova_debug::diagnostics,
    nova_interface::pane::{
        BalanceCountUp, InterfacePaneType, INTERFACE_PANE_BALANCE, INTERFACE_PANE_GAIN,
    },
    prelude::*,
};
use nova_world::prelude::{desired_sectors, SectorRoot, WorldConfig};

#[derive(Parser)]
#[command(name = "job_board_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start docked at a living generated station with its board's goods in the hold, on \
             the Jobs tab, and try the jobs by hand",
    long_about = None
)]
struct Cli;

/// The world seed. The window around its start holds no second station, so
/// the run plays the starting station's own board.
const SANDBOX_SEED: u32 = 10;
/// The start role the player spawns as when `NOVA_SANDBOX_ROLE` is unset:
/// the industrial start's board on [`SANDBOX_SEED`] posts a procurement,
/// which the smoke walk needs.
const SANDBOX_ROLE: &str = "industrial";

/// The start role this run plays: `NOVA_SANDBOX_ROLE`, else [`SANDBOX_ROLE`].
fn sandbox_role() -> String {
    std::env::var("NOVA_SANDBOX_ROLE").unwrap_or_else(|_| SANDBOX_ROLE.to_string())
}
/// How far from the station the player parks before the berth.
const PARK_DISTANCE: Meters = Meters(2_000.0);
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

    let autopilot = std::env::var_os("NOVA_AUTOPILOT").is_some();
    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.insert_resource(Sandbox {
        hold_at_start: autopilot,
        ..default()
    });
    app.add_plugins((
        diagnostics::DiagnosticsHintPlugin::new(INTRO),
        jobs_overlay::JobsOverlayPlugin,
    ));
    app.add_systems(OnEnter(GameStates::MainMenu), start_the_world);
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );

    if autopilot {
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
    /// New Game is up; the window around the start is streaming in and the
    /// start is docking the player at its starting station.
    Streaming,
    /// Docked at the starting station with its starter board on the Jobs
    /// tab, held there for the smoke walk until it leaves.
    Starter,
    /// The player was asked to let go of the starting station.
    Undocking,
    /// The player jumped beside the station; its window is streaming in.
    Jumped,
    /// The player is put on the berth each frame until the dock takes.
    Docking,
    /// Docked, fixture set and the Jobs tab open. The player has the run.
    Ready,
}

/// The sandbox's setup state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
    /// Hold the start docked at its starting station on the Jobs tab
    /// ([`SandboxPhaseType::Starter`]) until [`leave_the_start`], for the
    /// smoke walk's starter board read.
    hold_at_start: bool,
    /// The chosen station's id, the cell it stands in and where it stood.
    station: Option<(GeneratedBodyId, SectorCoord, Meters3)>,
}

/// Start New Game on [`SANDBOX_SEED`] the first time the main menu comes up,
/// with no save folder. A later Back to Main Menu stays on the menu.
fn start_the_world(
    mut commands: Commands,
    mut sandbox: ResMut<Sandbox>,
    loaded: Res<LoadedSectionPacks>,
    styles: Res<GameStyles>,
    items: Res<GameItems>,
    consumables: Res<GameConsumables>,
    profiles: Res<GameTradeProfiles>,
    mut mode: ResMut<GameMode>,
    mut pick: ResMut<NewGameScenario>,
    mut state: ResMut<NextState<GameStates>>,
) {
    if sandbox.phase != SandboxPhaseType::Menu {
        return;
    }
    let session = start_session(
        SANDBOX_SEED,
        sandbox_role().as_str().into(),
        &loaded,
        &styles,
        &items,
        &consumables,
        &profiles,
    )
    .unwrap_or_else(|refusal| panic!("job sandbox: seed {SANDBOX_SEED} refused: {refusal}"));
    info!("job sandbox: the start docks at '{}'", session.start.id);
    commands.insert_resource(session);
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
        SandboxPhaseType::Menu | SandboxPhaseType::Starter | SandboxPhaseType::Ready => {}
        SandboxPhaseType::Streaming => {
            let Some(player) = the_player(world) else {
                return;
            };
            let home = player_cell(world);
            if window_is(world, home) && world.get::<DockedShip>(player).is_some() {
                if world.resource::<Sandbox>().hold_at_start {
                    open_the_jobs_tab(world);
                    world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Starter;
                } else {
                    leave_the_start(world);
                }
            }
        }
        SandboxPhaseType::Undocking => {
            let Some(player) = the_player(world) else {
                return;
            };
            if world.get::<DockedShip>(player).is_none() {
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
                open_the_jobs_tab(world);
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Ready;
            } else {
                berth_and_dock(world, player);
            }
        }
    }
}

/// Let go of the starting station: the start docked the player there and
/// the sandbox stages a station of its own.
fn leave_the_start(world: &mut World) {
    let Some(player) = the_player(world) else {
        return;
    };
    world.trigger(DockingReleaseRequest { entity: player });
    world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Undocking;
}

/// The one player ship, if exactly one stands.
fn the_player(world: &mut World) -> Option<Entity> {
    let mut players = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The player's cell, once the player stands in an armed world.
fn player_cell(world: &mut World) -> Option<SectorCoord> {
    let edge = world
        .get_resource::<WorldConfig<NovaLayeredWorld>>()?
        .sector_edge;
    let player = the_player(world)?;
    let at = world.get::<Position>(player)?.0;
    Some(SectorCoord::containing(Meters3::from_engine(at), edge))
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
/// jobs, outside the spawn's cell; the starting station itself when the
/// window holds no other.
fn choose_the_station(world: &mut World) {
    let edge = world
        .resource::<WorldConfig<NovaLayeredWorld>>()
        .sector_edge;
    let home = player_cell(world).expect("job sandbox: the player stands in the armed world");
    let start = world
        .get_resource::<OpenWorldSession>()
        .map(|session| session.start.id.clone());
    let seen: Vec<String> = world
        .query_filtered::<(
            &GeneratedBodyId,
            &Position,
            Option<&StationServices>,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
        ), With<StationMarker>>()
        .iter(world)
        .map(|(id, position, services, derelict, allegiance)| {
            format!(
                "{id} in {} jobs {} derelict {derelict} {allegiance:?}",
                SectorCoord::containing(Meters3::from_engine(position.0), edge),
                station_offers(services, derelict, StationServiceType::Jobs)
            )
        })
        .collect();
    let mut stations: Vec<(f32, GeneratedBodyId, Vec3)> = world
        .query_filtered::<(
            &GeneratedBodyId,
            &Position,
            Option<&StationServices>,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
        ), With<StationMarker>>()
        .iter(world)
        .filter(|(id, position, services, derelict, allegiance)| {
            station_offers(*services, *derelict, StationServiceType::Jobs)
                && *allegiance != Some(&Allegiance::Enemy)
                && SectorCoord::containing(Meters3::from_engine(position.0), edge) != home
                && start.as_ref() != Some(*id)
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
        // The window around the docked start holds no second station: the
        // run plays the starting station's own board instead.
        let start = start.expect("job sandbox: an open world session records its start");
        let Some(at) = world
            .query::<(&GeneratedBodyId, &Position)>()
            .iter(world)
            .find(|(id, _)| **id == start)
            .map(|(_, position)| position.0)
        else {
            panic!(
                "job sandbox: the window on seed {SANDBOX_SEED} holds neither a second station \
                 posting jobs outside the spawn's cell {home} nor the starting station '{start}'; \
                 the stations seen: {seen:#?}"
            );
        };
        let position = Meters3::from_engine(at);
        let cell = SectorCoord::containing(position, edge);
        info!(
            "job sandbox: no second station posting jobs in the window; staying with the \
             starting station '{start}' in cell {cell}"
        );
        world.resource_mut::<Sandbox>().station = Some((start, cell, position));
        return;
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
    let (own_at, own_rot) = local_pose(world, player_port, player);
    let (at, turn) = parked_hull_pose((position, rotation), (port_at, port_rot), (own_at, own_rot));
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

/// Open the TAB interface on the Jobs tab, and say how to get back to it.
fn open_the_jobs_tab(world: &mut World) {
    *world.resource_mut::<InterfacePaneType>() = InterfacePaneType::Jobs;
    world
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Interface);
    info!("job sandbox: the Jobs tab is open; TAB closes and reopens the interface");
}

/// What the sandbox is for, for the diagnostics intro.
const INTRO: &str = "Docked at a station posting jobs: Accept, undock, Show on Map, dock, hand in.";

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
                    job.reward.total_cr(),
                    job.task(&items)
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

    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
        nova_interface::jobs::prelude::*,
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
    /// The most a board row of one line stands high, in logical px: a list
    /// row, its border and layout rounding. A row that wrapped is two.
    const ROW_LINE_PX: f32 = 32.0;
    const STARTER_SHOT: &str = "sandbox-jobs-starter.png";
    const GOAL_SHOT: &str = "sandbox-jobs-goal.png";
    const RECOVER_SHOT: &str = "sandbox-jobs-spawned-recover.png";
    const NEUTRALIZE_SHOT: &str = "sandbox-jobs-spawned-neutralize.png";
    const MARKERS_SHOT: &str = "sandbox-jobs-spawned-hud.png";
    /// Seconds an accepted job's own target gets to stand in the world.
    const SPAWN_SECS: f32 = 240.0;
    const BOARD_SHOT: &str = "sandbox-jobs-board.png";
    const HELD_SHOT: &str = "sandbox-jobs-held.png";
    const MAP_SHOT: &str = "sandbox-jobs-map.png";
    const MAP_DROPPED_SHOT: &str = "sandbox-jobs-map-dropped.png";
    const DROPPED_SHOT: &str = "sandbox-jobs-dropped.png";
    const HAND_IN_SHOT: &str = "sandbox-jobs-hand-in.png";
    const COUNT_SHOT: &str = "sandbox-jobs-count.png";
    const DONE_SHOT: &str = "sandbox-jobs-done.png";
    const PAID_SHOT: &str = "sandbox-jobs-paid.png";
    /// What the open world's once `OnJobHandedIn` handler grants the player
    /// on a world's first hand-in, in the frame of the job's reward, in
    /// credits.
    const FIRST_DELIVERY_BONUS_CR: u64 = 50;

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

    fn in_phase_on_the_jobs_tab(phase: SandboxPhaseType) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<Sandbox>()
                .is_some_and(|sandbox| sandbox.phase == phase)
                && world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && world.get_resource::<InterfacePaneType>() == Some(&InterfacePaneType::Jobs)
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

    fn ui_node_absent(name: &'static str) -> Arc<Predicate> {
        when(move |world| {
            world
                .try_query::<&Name>()
                .is_none_or(|mut names| names.iter(world).all(|found| found.as_str() != name))
        })
    }

    /// How many UI nodes carry a `Name` starting with `prefix`.
    fn named_count(world: &World, prefix: &str) -> usize {
        world.try_query::<&Name>().map_or(0, |mut names| {
            names
                .iter(world)
                .filter(|found| found.as_str().starts_with(prefix))
                .count()
        })
    }

    /// What the player's hull can do, read as the Jobs pane reads it.
    fn player_fit(world: &World) -> JobHullFit {
        let Some(player) = one_player(world) else {
            return JobHullFit::default();
        };
        let mut fit = JobHullFit {
            hold_capacity_g: world
                .get::<ShipInventory>(player)
                .map_or(0, ShipInventory::capacity_g),
            has_clamp: false,
            has_weapon: false,
        };
        let Some(mut sections) = world.try_query_filtered::<(
            &ChildOf,
            Has<DockingSectionMarker>,
            Has<TurretSectionMarker>,
            Has<TorpedoSectionMarker>,
            Has<RailgunSectionMarker>,
        ), (With<SectionMarker>, Without<SectionInactiveMarker>)>(
        ) else {
            return fit;
        };
        for (child_of, clamp, turret, bay, railgun) in sections.iter(world) {
            if child_of.parent() != player {
                continue;
            }
            fit.has_clamp |= clamp;
            fit.has_weapon |= turret || bay || railgun;
        }
        fit
    }

    /// The starting station's board, as the start recorded it.
    fn starter_board(world: &World) -> Option<JobBoard> {
        let start = world.get_resource::<OpenWorldSession>()?.start.id.clone();
        let mut boards = world.try_query::<(&GeneratedBodyId, &JobBoard)>()?;
        boards
            .iter(world)
            .find(|(id, _)| **id == start)
            .map(|(_, board)| board.clone())
    }

    /// The starter board's jobs the hull can and cannot do: `(fit, unfit)`.
    fn starter_split(world: &World) -> Option<(usize, usize)> {
        let board = starter_board(world)?;
        let items = world.get_resource::<GameItems>()?;
        let fit = player_fit(world);
        let fits = board
            .0
            .iter()
            .filter(|job| plan_job_hull_fit(job, items, fit).is_ok())
            .count();
        Some((fits, board.0.len() - fits))
    }

    /// The starter board posts a job the hull can do in every slot, and the
    /// pane lists every one.
    fn starter_board_fits() -> Arc<Predicate> {
        when(|world| {
            let slots = JOBS_PER_BOARD as usize;
            starter_split(world) == Some((slots, 0)) && named_count(world, "JobRowPosted-") == slots
        })
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

    /// The Map's chart view lists the role goal's first step as the current
    /// one: the first step of [`GoalProgress`], which a new game has not met.
    fn the_chart_lists_the_own_goal_line() -> Arc<Predicate> {
        when(|world| {
            let Some(line) = world
                .get_resource::<GoalProgress>()
                .and_then(|progress| progress.steps.first().map(|step| step.line.clone()))
            else {
                return false;
            };
            let reads = |wanted: String| {
                world
                    .try_query::<&Text>()
                    .is_some_and(|mut query| query.iter(world).any(|text| text.0 == wanted))
            };
            line.have < line.need
                && named_count(world, "MapChartObjectiveMilestone") == 1
                && reads(line.label.clone())
                && reads(format!("{} / {}", line.have, line.need))
        })
    }

    /// The starter board's job that names its own target, a wreck when
    /// `recover` and a raider otherwise: the held one once it is accepted,
    /// else the first one posted.
    fn own_target_job(world: &World, recover: bool) -> Option<Job> {
        let wanted = |job: &Job| {
            job.spawns_target() && matches!(job.kind, JobType::Recover { .. }) == recover
        };
        let held = world
            .get_resource::<Jobs>()?
            .held()
            .iter()
            .find(|job| wanted(job));
        held.cloned()
            .or_else(|| starter_board(world)?.0.into_iter().find(wanted))
    }

    /// The entity wearing `id`, if one stands.
    fn body_wearing(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
        let mut bodies = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        bodies
            .iter(world)
            .find(|(_, worn)| *worn == id)
            .map(|(entity, _)| entity)
    }

    /// The own-target job of this kind is held and every body it spawned
    /// stands wearing its objective marker; `true` too when the board posts
    /// no such job, so the walk passes a role whose board has none.
    fn the_own_target_stands(recover: bool) -> Arc<Predicate> {
        when(move |world| {
            let Some(job) = own_target_job(world, recover) else {
                return true;
            };
            world.resource::<Jobs>().holds(&job.id)
                && job.spawned_bodies().iter().all(|id| {
                    body_wearing(world, id)
                        .is_some_and(|body| world.get::<ObjectiveMarkerTarget>(body).is_some())
                })
        })
    }

    /// `check` holds, or the board posts no own-target job of this kind.
    fn or_no_own_target(recover: bool, check: Arc<Predicate>) -> Arc<Predicate> {
        when(move |world| own_target_job(world, recover).is_none() || check(world))
    }

    /// A text reads the chart code of the held own-target job of this kind.
    fn the_chart_names_the_own_target(recover: bool) -> Arc<Predicate> {
        when(move |world| {
            let Some(code) = own_target_job(world, recover)
                .and_then(|job| world.resource::<Jobs>().mark_code(&job.id))
            else {
                return false;
            };
            world
                .try_query::<&Text>()
                .is_some_and(|mut query| query.iter(world).any(|text| text.0 == code))
        })
    }

    /// The HUD has grown one marker chip on every body the held own-target
    /// jobs spawned.
    fn a_chip_targets_every_own_target() -> Arc<Predicate> {
        when(|world| {
            let bodies: Vec<Entity> = [true, false]
                .into_iter()
                .filter_map(|recover| own_target_job(world, recover))
                .flat_map(|job| job.spawned_bodies())
                .filter_map(|id| body_wearing(world, &id))
                .collect();
            let Some(mut chips) = world
                .try_query_filtered::<&AnchoredChipTarget, With<ObjectiveMarkerChipHudMarker>>()
            else {
                return bodies.is_empty();
            };
            bodies
                .iter()
                .all(|body| chips.iter(world).any(|target| **target == *body))
        })
    }

    /// Accept the starter board's own-target job of one kind, open the chart
    /// on its mark and shoot it; every step passes at once when the board
    /// posts no such job.
    fn own_target_steps(
        script: AutopilotPlugin<GameStates>,
        tag: &'static str,
        recover: bool,
        shot: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!(
                "job sandbox: accept the {tag} that names its own target"
            ))
            .on_enter(
                move |world: &mut World| match own_target_job(world, recover) {
                    Some(job) => {
                        info!(
                            "job sandbox: accepting '{}', which spawns {:?}",
                            job.id,
                            job.spawned_bodies()
                        );
                        world.write_message(JobAcceptCommand { id: job.id });
                    }
                    None => {
                        info!("job sandbox: the starter board posts no {tag} with its own target")
                    }
                },
            )
            .until(and(the_own_target_stands(recover), frames(SETTLE_FRAMES)))
            .diagnose(move |world| format!("{:?}", own_target_job(world, recover)))
            .deadline(SPAWN_SECS)
            .add()
            .step(format!("job sandbox: select the held {tag}"))
            .on_enter(move |world: &mut World| {
                let Some(job) = own_target_job(world, recover) else {
                    return;
                };
                let row = world
                    .resource::<Jobs>()
                    .held()
                    .iter()
                    .position(|held| held.id == job.id)
                    .expect("job sandbox: the own-target job is held");
                activate_named(world, &format!("JobRowHeld-{row}"));
            })
            .until(or_no_own_target(
                recover,
                and(ui_node_present(JOB_SHOW_ON_MAP), frames(SETTLE_FRAMES)),
            ))
            .deadline(BEAT_SECS)
            .add()
            .step(format!(
                "job sandbox: Show on Map opens the chart on the {tag}"
            ))
            .on_enter(move |world: &mut World| {
                if own_target_job(world, recover).is_some() {
                    activate_named(world, JOB_SHOW_ON_MAP);
                }
            })
            .until(or_no_own_target(
                recover,
                and(
                    and(
                        the_interface_shows(Some(InterfacePaneType::Map)),
                        the_chart_names_the_own_target(recover),
                    ),
                    frames(SETTLE_FRAMES),
                ),
            ))
            .deadline(BEAT_SECS)
            .add()
            .step(format!("job sandbox: shoot the chart on the {tag}"))
            .on_enter(move |world: &mut World| {
                if own_target_job(world, recover).is_some() {
                    shoot(world, shot);
                }
            })
            .until(or_no_own_target(recover, shot_written(shot)))
            .deadline(SHOT_SECS)
            .add()
            .click_named(
                &format!("job sandbox: back to the Jobs tab after the {tag}"),
                "InterfaceTabJobs",
                and(
                    the_interface_shows(Some(InterfacePaneType::Jobs)),
                    frames(SETTLE_FRAMES),
                ),
                BEAT_SECS,
            )
    }

    /// The board row of the first procurement the chosen station posts.
    fn procurement_row(world: &mut World) -> usize {
        let board = station_root(world)
            .and_then(|station| world.get::<JobBoard>(station).cloned())
            .expect("job sandbox: the station posts a board");
        board
            .0
            .iter()
            .position(|job| job.kind.is_procurement())
            .expect("job sandbox: the board posts a procurement")
    }

    /// The chosen station's board as the pane lists it while no job is
    /// held: every job.
    fn listed_jobs(world: &World) -> Vec<Job> {
        let Some((id, ..)) = world
            .get_resource::<Sandbox>()
            .and_then(|sandbox| sandbox.station.clone())
        else {
            return Vec::new();
        };
        let Some(mut boards) = world.try_query::<(&GeneratedBodyId, &JobBoard)>() else {
            return Vec::new();
        };
        boards
            .iter(world)
            .find(|(worn, _)| **worn == id)
            .map(|(_, board)| board.0.clone())
            .unwrap_or_default()
    }

    /// The inspector lists a row for each term of board job `row` that pays
    /// and none for a term that pays nothing; a reward of one term lists no
    /// row at all. `Range` and `Threat` carry their measure after the word,
    /// which tells them from the board's column heads.
    fn reads_the_terms_of(row: usize) -> Arc<Predicate> {
        when(move |world| {
            let listed = listed_jobs(world);
            if listed.len() != named_count(world, "JobRowPosted-") {
                return false;
            }
            let Some(job) = listed.get(row) else {
                return false;
            };
            let goods = if matches!(job.kind, JobType::Escort { .. }) {
                ("Cargo", "Goods")
            } else {
                ("Goods", "Cargo")
            };
            let terms = [
                (job.reward.base_cr, "Base"),
                (job.reward.goods_cr, goods.0),
                (job.reward.range_cr, "Range "),
                (job.reward.threat_cr, "Threat "),
            ];
            let shown = terms.iter().filter(|(cr, _)| *cr > 0).count() > 1;
            let reads = |word: &str| {
                world.try_query::<&Text>().is_some_and(|mut query| {
                    query.iter(world).any(|text| {
                        if word.ends_with(' ') {
                            text.0.starts_with(word)
                        } else {
                            text.0 == word
                        }
                    })
                })
            };
            terms
                .iter()
                .all(|(cr, word)| reads(word) == (shown && *cr > 0))
                && !reads(goods.1)
        })
    }

    /// Every board row is one list row high, so no cell wrapped under its
    /// title, and its Accept lies inside the board panel.
    fn board_rows_fit() -> Arc<Predicate> {
        when(|world| {
            let Some(board) = ui_node_rect(world, JOBS_AVAILABLE) else {
                return false;
            };
            let rows = named_count(world, "JobRowPosted-");
            rows > 0
                && (0..rows).all(|row| {
                    let line = ui_node_rect(world, &format!("JobRowPosted-{row}"));
                    let accept = ui_node_rect(world, &format!("{JOB_ACCEPT_PREFIX}{row}"));
                    line.is_some_and(|line| line.height() <= ROW_LINE_PX)
                        && accept.is_some_and(|accept| {
                            accept.min.x >= board.min.x && accept.max.x <= board.max.x
                        })
                })
        })
    }

    /// No text node reads what `check` accepts.
    fn no_text_reads(check: impl Fn(&str) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        when(move |world| {
            world
                .try_query::<&Text>()
                .is_none_or(|mut query| query.iter(world).all(|text| !check(&text.0)))
        })
    }

    /// The first job on the chosen station's board that the player does not
    /// hold, that the hull can do and that pays another reward than the held
    /// procurement, so a reward read off the screen names one job only.
    fn second_job(world: &mut World) -> JobId {
        let board = station_root(world)
            .and_then(|station| world.get::<JobBoard>(station).cloned())
            .expect("job sandbox: the station posts a board");
        let fit = player_fit(world);
        let items = world.resource::<GameItems>();
        let jobs = world.resource::<Jobs>();
        let first = &jobs.held()[0];
        board
            .0
            .iter()
            .find(|job| {
                !jobs.holds(&job.id)
                    && job.reward.total_cr() != first.reward.total_cr()
                    && plan_job_hull_fit(job, items, fit).is_ok()
            })
            .map(|job| job.id.clone())
            .expect("job sandbox: the board posts a second job the hull can do at another reward")
    }

    /// A text reads the second held job's reward as `<prefix><reward><suffix>`
    /// and none reads the first's, with the pane's thousands commas ignored:
    /// the inspector's credit figure is digits beside an icon, the chart's
    /// note `Reward <n> cr`.
    fn reads_the_second_reward_only(prefix: &'static str, suffix: &'static str) -> Arc<Predicate> {
        when(move |world| {
            let [first, second] = world.resource::<Jobs>().held() else {
                return false;
            };
            let reads = |reward: String| {
                world.try_query::<&Text>().is_some_and(|mut query| {
                    query
                        .iter(world)
                        .any(|text| text.0.replace(',', "") == reward)
                })
            };
            reads(format!("{prefix}{}{suffix}", second.reward.total_cr()))
                && !reads(format!("{prefix}{}{suffix}", first.reward.total_cr()))
        })
    }

    /// The text of the UI node named `name`.
    fn text_named(world: &World, name: &str) -> Option<String> {
        let mut texts = world.try_query::<(&Name, &Text)>()?;
        texts
            .iter(world)
            .find(|(found, _)| found.as_str() == name)
            .map(|(_, text)| text.0.clone())
    }

    /// The player's balance and the reward of the one completed job.
    fn balance_and_reward(world: &World) -> Option<(u64, u64)> {
        let player = one_player(world)?;
        let balance = world.get::<ShipCredits>(player)?.0;
        let [job] = world.resource::<Jobs>().completed() else {
            return None;
        };
        Some((balance.into(), job.reward.total_cr().into()))
    }

    /// The pane head counts up to the hand-in: the gain reads the reward and
    /// the first-delivery bonus as one figure, all the balance rose by, and
    /// the balance figure stands at or above the balance before the hand-in
    /// and short of the true one.
    fn the_head_counts_up() -> Arc<Predicate> {
        when(|world| {
            let Some((balance, reward)) = balance_and_reward(world) else {
                return false;
            };
            let gain = reward + FIRST_DELIVERY_BONUS_CR;
            let shown = text_named(world, INTERFACE_PANE_BALANCE)
                .and_then(|text| text.replace(',', "").parse::<u64>().ok());
            world
                .get_resource::<BalanceCountUp>()
                .is_some_and(|count| count.is_counting() && count.gain_cr() == Some(gain))
                && text_named(world, INTERFACE_PANE_GAIN) == Some(format!("+{}", cr_digits(gain)))
                && shown.is_some_and(|shown| shown >= balance - gain && shown < balance)
        })
    }

    /// The count is over: the pane head reads the true balance and no gain.
    fn the_head_reads_the_balance() -> Arc<Predicate> {
        when(|world| {
            let Some((balance, _)) = balance_and_reward(world) else {
                return false;
            };
            world
                .get_resource::<BalanceCountUp>()
                .is_some_and(|count| count.gain_cr().is_none())
                && text_named(world, INTERFACE_PANE_BALANCE) == Some(cr_digits(balance))
                && text_named(world, INTERFACE_PANE_GAIN).as_deref() == Some("")
        })
    }

    /// The HUD notice line of the hand-in: the walk's procurement was held
    /// first, so it was `JOB-1`.
    fn paid_line(world: &World) -> Option<String> {
        let (_, reward) = balance_and_reward(world)?;
        Some(format!("JOB-1 PAID {}", cr_text(reward).to_uppercase()))
    }

    /// The HUD notice slot reads what the hand-in paid.
    fn the_paid_notice_shows() -> Arc<Predicate> {
        when(|world| {
            let Some(line) = paid_line(world) else {
                return false;
            };
            world
                .try_query_filtered::<&Text, With<HudNoticeText>>()
                .is_some_and(|mut texts| texts.iter(world).any(|text| text.0 == line))
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

    /// The smoke walk: read the starter board, move to the chosen station,
    /// accept the first procurement and a second job, undock, read the
    /// second held row and Show on Map, drop it with the chart open, dock
    /// again and hand the procurement in, watch the pane head count up by
    /// the reward and the bonus, shut the interface and read the paid notice, shooting
    /// each surface.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("job sandbox: docked at the starting station on the Jobs tab")
            .until(in_phase_on_the_jobs_tab(SandboxPhaseType::Starter))
            .diagnose(|world| format!("{:?}", world.get_resource::<Sandbox>()))
            .deadline(SETUP_SECS)
            .add()
            .step("job sandbox: the starter board lists a job the hull can do in every slot")
            .until(and(starter_board_fits(), frames(SETTLE_FRAMES)))
            .diagnose(|world| {
                format!(
                    "starter split (fit, unfit) {:?}, rows {}, hull {:?}",
                    starter_split(world),
                    named_count(world, "JobRowPosted-"),
                    player_fit(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("job sandbox: shoot the starter board")
            .on_enter(|world: &mut World| shoot(world, STARTER_SHOT))
            .until(shot_written(STARTER_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .click_named(
                "job sandbox: open the Map pane",
                "InterfaceTabMap",
                the_interface_shows(Some(InterfacePaneType::Map)),
                BEAT_SECS,
            );
        let script = tap(
            script,
            "the chart key",
            "map_chart",
            and(the_chart_lists_the_own_goal_line(), frames(SETTLE_FRAMES)),
        )
        .step("job sandbox: shoot the role goal in the chart view")
        .on_enter(|world: &mut World| shoot(world, GOAL_SHOT))
        .until(shot_written(GOAL_SHOT))
        .deadline(SHOT_SECS)
        .add()
        .click_named(
            "job sandbox: back to the Jobs tab",
            "InterfaceTabJobs",
            and(
                in_phase_on_the_jobs_tab(SandboxPhaseType::Starter),
                frames(SETTLE_FRAMES),
            ),
            BEAT_SECS,
        );
        if sandbox_role() != SANDBOX_ROLE {
            // Another role's board need not post a procurement: its walk
            // ends on the jobs that name their own target.
            let script = own_target_steps(script, "recovery", true, RECOVER_SHOT);
            let script = own_target_steps(script, "neutralize", false, NEUTRALIZE_SHOT);
            return tap(script, "the interface key", "interface_toggle", unpaused())
                .step("job sandbox: the HUD marks every own target")
                .until(and(
                    a_chip_targets_every_own_target(),
                    frames(SETTLE_FRAMES),
                ))
                .deadline(BEAT_SECS)
                .add()
                .step("job sandbox: shoot the HUD markers")
                .on_enter(|world: &mut World| shoot(world, MARKERS_SHOT))
                .until(shot_written(MARKERS_SHOT))
                .deadline(SHOT_SECS)
                .add();
        }
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("job sandbox: leave the start for the chosen station")
            .on_enter(leave_the_start)
            .until(in_phase_on_the_jobs_tab(SandboxPhaseType::Ready))
            .diagnose(|world| format!("{:?}", world.get_resource::<Sandbox>()))
            .deadline(SETUP_SECS)
            .add()
            .step("job sandbox: the board shows")
            .until(and(ui_node_present("JobAccept-0"), frames(SETTLE_FRAMES)))
            .deadline(BEAT_SECS)
            .add()
            // The job rows can sit below the panel's fold on a 1080p frame,
            // out of the pointer's reach: the rows and buttons are activated
            // by name.
            .step("job sandbox: select the first procurement")
            .on_enter(|world: &mut World| {
                let row = procurement_row(world);
                activate_named(world, &format!("JobRowPosted-{row}"));
            })
            .until(and(ui_node_present(JOB_SHOW_ON_MAP), frames(SETTLE_FRAMES)))
            .deadline(BEAT_SECS)
            .add()
            .step("job sandbox: shoot the board")
            .on_enter(|world: &mut World| shoot(world, BOARD_SHOT))
            .until(shot_written(BOARD_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("job sandbox: every board row is one line with its Accept inside the board")
            .until(and(board_rows_fit(), frames(SETTLE_FRAMES)))
            .diagnose(|world| {
                format!(
                    "board {:?}, row 0 {:?}, Accept 0 {:?}",
                    ui_node_rect(world, JOBS_AVAILABLE),
                    ui_node_rect(world, "JobRowPosted-0"),
                    ui_node_rect(world, "JobAccept-0")
                )
            })
            .deadline(BEAT_SECS)
            .add();
        let mut script = script;
        for row in 0..JOBS_PER_BOARD as usize {
            let shot = format!("sandbox-jobs-terms-{row}.png");
            let written = shot.clone();
            script = script
                .step(format!(
                    "job sandbox: row {row}'s inspector lists the terms that pay"
                ))
                .on_enter(move |world: &mut World| {
                    if row < listed_jobs(world).len() {
                        activate_named(world, &format!("JobRowPosted-{row}"));
                    }
                })
                .until(and(reads_the_terms_of(row), frames(SETTLE_FRAMES)))
                .diagnose(move |world| {
                    format!(
                        "{} rows, listed {:?}",
                        named_count(world, "JobRowPosted-"),
                        listed_jobs(world).get(row)
                    )
                })
                .deadline(BEAT_SECS)
                .add()
                .step(format!("job sandbox: shoot row {row}'s terms"))
                .on_enter(move |world: &mut World| {
                    if row < listed_jobs(world).len() {
                        shoot(world, &shot);
                    }
                })
                .until(when(move |world| {
                    row >= listed_jobs(world).len() || shot_written(written.clone())(world)
                }))
                .deadline(SHOT_SECS)
                .add();
        }
        let script = script
            .step("job sandbox: accept the first procurement")
            .on_enter(|world: &mut World| {
                let row = procurement_row(world);
                activate_named(world, &format!("{JOB_ACCEPT_PREFIX}{row}"));
            })
            .until(and(held_count(1), frames(SETTLE_FRAMES)))
            .deadline(BEAT_SECS)
            .add()
            .step("job sandbox: accept a second job at another reward")
            .on_enter(|world: &mut World| {
                let id = second_job(world);
                world.write_message(JobAcceptCommand { id });
            })
            .until(and(held_count(2), frames(SETTLE_FRAMES)))
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
            "job sandbox: open the Jobs pane undocked",
            "InterfaceTabJobs",
            and(
                and(
                    and(
                        ui_node_present(JOBS_AVAILABLE),
                        ui_node_absent("JobRowPosted-0"),
                    ),
                    and(
                        ui_node_present("JobRowHeld-0"),
                        // Hidden, not despawned: it holds no laid-out rect.
                        when(|world| ui_node_rect(world, "JobHandIn-0").is_none()),
                    ),
                ),
                frames(SETTLE_FRAMES),
            ),
            BEAT_SECS,
        )
        .step("job sandbox: select the second held job")
        .on_enter(|world: &mut World| activate_named(world, "JobRowHeld-1"))
        .until(and(
            and(
                ui_node_present(JOB_SHOW_ON_MAP),
                and(
                    a_text_reads(|text| text == "Progress"),
                    no_text_reads(|text| text == "HELD"),
                ),
            ),
            and(
                reads_the_second_reward_only("", " cr"),
                frames(SETTLE_FRAMES),
            ),
        ))
        .deadline(BEAT_SECS)
        .add()
        .step("job sandbox: shoot the held list")
        .on_enter(|world: &mut World| shoot(world, HELD_SHOT))
        .until(shot_written(HELD_SHOT))
        .deadline(SHOT_SECS)
        .add()
        .step("job sandbox: Show on Map opens the chart on the job")
        .on_enter(|world: &mut World| activate_named(world, JOB_SHOW_ON_MAP))
        .until(and(
            the_interface_shows(Some(InterfacePaneType::Map)),
            and(
                and(
                    and(
                        a_text_reads(|text| text == "JOB-2"),
                        a_text_reads(|text| text == "Reward"),
                    ),
                    reads_the_second_reward_only("", " cr"),
                ),
                frames(SETTLE_FRAMES),
            ),
        ))
        .deadline(BEAT_SECS)
        .add()
        .step("job sandbox: shoot the chart view")
        .on_enter(|world: &mut World| shoot(world, MAP_SHOT))
        .until(shot_written(MAP_SHOT))
        .deadline(SHOT_SECS)
        .add()
        // The world drops a held job when its body is lost
        // (`drop_on_loss`). The walk drops it by hand, with its mark
        // selected in the open chart, and leaves the bodies the job spawned:
        // the rest of the walk does not meet them.
        .step("job sandbox: drop the selected job with the chart open")
        .on_enter(|world: &mut World| {
            let id = world.resource::<Jobs>().held()[1].id.clone();
            world.resource_mut::<Jobs>().drop(&id);
        })
        .until(and(held_count(1), frames(SETTLE_FRAMES)))
        .deadline(BEAT_SECS)
        .add()
        .step("job sandbox: shoot the chart after the drop")
        .on_enter(|world: &mut World| shoot(world, MAP_DROPPED_SHOT))
        .until(shot_written(MAP_DROPPED_SHOT))
        .deadline(SHOT_SECS)
        .add()
        .step("job sandbox: the chart returns to its overview")
        .until(and(
            and(
                a_text_reads(|text| text == "OBJECTIVES"),
                no_text_reads(|text| text == "Reward"),
            ),
            frames(SETTLE_FRAMES),
        ))
        .deadline(BEAT_SECS)
        .add()
        .click_named(
            "job sandbox: the Jobs inspector returns to its overview",
            "InterfaceTabJobs",
            and(
                and(
                    the_interface_shows(Some(InterfacePaneType::Jobs)),
                    ui_node_absent(JOB_SHOW_ON_MAP),
                ),
                and(no_text_reads(|text| text == "TASK"), frames(SETTLE_FRAMES)),
            ),
            BEAT_SECS,
        )
        .step("job sandbox: shoot the Jobs tab after the drop")
        .on_enter(|world: &mut World| shoot(world, DROPPED_SHOT))
        .until(shot_written(DROPPED_SHOT))
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
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "job sandbox: open the Jobs tab",
            "InterfaceTabJobs",
            and(
                and(
                    ui_node_present(JOBS_AVAILABLE),
                    ui_node_present("JobHandIn-0"),
                ),
                and(
                    a_text_reads(|text| text.contains("HAND IN HERE")),
                    frames(SETTLE_FRAMES),
                ),
            ),
            BEAT_SECS,
        )
        .step("job sandbox: shoot the held row docked at its hand-in")
        .on_enter(|world: &mut World| shoot(world, HAND_IN_SHOT))
        .until(shot_written(HAND_IN_SHOT))
        .deadline(SHOT_SECS)
        .add()
        .step(
            "job sandbox: hand the job in and the pane head counts up by the reward and the bonus",
        )
        .on_enter(|world: &mut World| activate_named(world, &format!("{JOB_HAND_IN_PREFIX}0")))
        .until(and(completed_count(1), the_head_counts_up()))
        .diagnose(|world| {
            format!(
                "balance and reward {:?}, head '{:?}' gain '{:?}', {:?}",
                balance_and_reward(world),
                text_named(world, INTERFACE_PANE_BALANCE),
                text_named(world, INTERFACE_PANE_GAIN),
                world.get_resource::<BalanceCountUp>()
            )
        })
        .deadline(BEAT_SECS)
        .add()
        // No settle before this shot: the count runs on real time and is
        // over in under a second.
        .step("job sandbox: shoot the pane head during the count")
        .on_enter(|world: &mut World| shoot(world, COUNT_SHOT))
        .until(shot_written(COUNT_SHOT))
        .deadline(SHOT_SECS)
        .add()
        .step("job sandbox: the count lands on the true balance and the gain goes")
        .until(and(the_head_reads_the_balance(), frames(SETTLE_FRAMES)))
        .diagnose(|world| {
            format!(
                "balance and reward {:?}, head '{:?}' gain '{:?}', {:?}",
                balance_and_reward(world),
                text_named(world, INTERFACE_PANE_BALANCE),
                text_named(world, INTERFACE_PANE_GAIN),
                world.get_resource::<BalanceCountUp>()
            )
        })
        .deadline(BEAT_SECS)
        .add()
        .step("job sandbox: shoot the Jobs tab after the hand-in")
        .on_enter(|world: &mut World| shoot(world, DONE_SHOT))
        .until(shot_written(DONE_SHOT))
        .deadline(SHOT_SECS)
        .add();
        // The notice was posted under the pane, where the HUD is hidden and
        // its clock held: its dwell starts when the interface shuts.
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("job sandbox: the HUD notice reads what the job paid")
            .until(the_paid_notice_shows())
            .diagnose(|world| format!("expected {:?}", paid_line(world)))
            .deadline(BEAT_SECS)
            .add()
            .step("job sandbox: shoot the paid notice")
            .on_enter(|world: &mut World| shoot(world, PAID_SHOT))
            .until(shot_written(PAID_SHOT))
            .deadline(SHOT_SECS)
            .add()
    }
}
