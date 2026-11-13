//! medkit_rescue_sandbox: start 500 m off a distress caller with the call
//! held, with or without a Medkit aboard, and dock with the caller.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the bounty
//! hunter, whose boards lean to distress calls, as the Create World modal
//! does but without a save folder. When the start's window has streamed in,
//! it picks the nearest living station whose board posts a distress call,
//! berths the player there until the dock takes, and accepts the call with
//! [`JobAcceptCommand`], the message the board row's Accept button writes.
//! The open world then puts the caller in the world, 10 km off the poster.
//! The player undocks and jumps 500 m off the caller, at rest, bow on it.
//! THE HOLD IS DISCLOSED FIXTURE STATE: `--medkit aboard` (default) leaves
//! one Medkit in the hold, `--medkit none` takes every Medkit out.
//!
//! What to do:
//!
//! 1. fly to the caller, hold Ctrl on it to lock it, press D to dock;
//! 2. with a Medkit aboard the rescue takes it: the hold loses one Medkit
//!    and the call is done. TAB > Jobs lists it done;
//! 3. with no Medkit aboard the HUD reads `NO MEDKIT ABOARD` and the call
//!    stays open. The poster sells Medkits when the panel says so: fly back,
//!    buy one, come back and dock again;
//! 4. the hand-in is at the poster, 10 km back: dock there, TAB > Jobs,
//!    Hand in.
//!
//! The diagnostics layer (F3) is on at start: a label and a line on the
//! caller and on the poster with the range, and panel lines with the state
//! of the call, the Medkits aboard and the Medkits the poster stocks.
//!
//! ```text
//! nix develop --command cargo run --example medkit_rescue_sandbox --features debug
//! nix develop --command cargo run --example medkit_rescue_sandbox --features debug -- --medkit none
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk waits for the staging, checks it and exits. `NOVA_CAPTURE=1` shoots
//! `medkit-rescue-sandbox.png`.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use avian3d::prelude::{LinearVelocity, Position};
use bevy::prelude::*;
use clap::{Parser, ValueEnum};
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, nova_interface::jobs::JobAcceptCommand, prelude::*};
use open_world_stage::{ChosenStation, OpenWorldStartPlugin};

/// What the fixture hold carries.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
enum MedkitFixtureType {
    /// One Medkit: the dock with the caller answers the call.
    #[default]
    Aboard,
    /// No Medkit: the dock is refused with `NO MEDKIT ABOARD`.
    None,
}

impl MedkitFixtureType {
    /// How many Medkits the fixture hold carries.
    fn count(self) -> u32 {
        match self {
            Self::Aboard => 1,
            Self::None => 0,
        }
    }

    /// The intro line.
    fn intro(self) -> &'static str {
        match self {
            Self::Aboard => {
                "A distress caller drifts 500 m ahead and a Medkit is aboard: lock it, dock."
            }
            Self::None => {
                "A distress caller drifts 500 m ahead and no Medkit is aboard: lock it, dock."
            }
        }
    }
}

#[derive(Parser, Resource, Clone, Copy)]
#[command(name = "medkit_rescue_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start 500 m off a distress caller with the call held, with or without a Medkit \
             aboard, and dock with the caller",
    long_about = None
)]
struct Cli {
    /// What the fixture hold carries.
    #[arg(long, value_enum, default_value_t = MedkitFixtureType::Aboard)]
    medkit: MedkitFixtureType,
}

/// The world seed and the start role: `system_job_distress`'s, whose window
/// around the start holds a living station that posts a distress call.
const SANDBOX_SEED: u32 = 16;
const SANDBOX_ROLE: &str = "bounty_hunter";
/// How far from the poster the player parks while its window streams in.
const PARK_DISTANCE: Meters = Meters(2_000.0);
/// How far off the caller the player starts, on the caller's poster side.
const CALLER_STANDOFF: Meters = Meters(500.0);

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.add_plugins(OpenWorldStartPlugin {
        seed: SANDBOX_SEED,
        role: SANDBOX_ROLE.to_string(),
    });
    app.insert_resource(cli);
    app.init_resource::<Sandbox>();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(cli.medkit.intro()));
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        draw_the_rescue
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Update, open_world_stage::hold_the_walk_clock);
        app.add_systems(Startup, force_capture_resolution);
    }
    app.add_plugins(smoke::script(cli.medkit));

    app.run()
}

/// Where the staging stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// The start's window is streaming in and the start is docking the
    /// player at its starting station.
    #[default]
    Streaming,
    /// The player let go of the starting station; a board that posts a
    /// distress call is looked for.
    Choosing,
    /// The player jumped beside the poster; its window is streaming in.
    Jumped,
    /// The player is put on the poster's berth until the dock takes.
    Docking,
    /// The accept was asked for.
    Accepting,
    /// The call is held; the player lets go of the poster and the caller
    /// comes into the world.
    Leaving,
    /// The player stands off the caller. The player has the run.
    Ready,
}

/// The staging state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
    /// The poster.
    station: Option<ChosenStation>,
    /// The distress call.
    job: Option<Job>,
}

/// The caller's id of a distress call.
fn caller_of(job: &Job) -> Option<&GeneratedBodyId> {
    match &job.kind {
        JobType::Distress { caller } => Some(caller),
        _ => None,
    }
}

/// The nearest living station, not on the Enemy side, whose board posts a
/// distress call, with ties broken by id.
fn choose_the_poster(world: &mut World, player: Entity) -> Option<ChosenStation> {
    let edge = open_world_stage::sector_edge(world)?;
    let at = world.get::<Position>(player)?.0;
    let mut posters: Vec<(f32, GeneratedBodyId, Vec3)> = world
        .query_filtered::<(
            &GeneratedBodyId,
            &Position,
            &JobBoard,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
        ), With<StationMarker>>()
        .iter(world)
        .filter(|(_, _, board, derelict, allegiance)| {
            !derelict
                && *allegiance != Some(&Allegiance::Enemy)
                && board.0.iter().any(|job| caller_of(job).is_some())
        })
        .map(|(id, position, ..)| (position.0.distance(at), id.clone(), position.0))
        .collect();
    posters.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let (_, id, at) = posters.into_iter().next()?;
    let position = Meters3::from_engine(at);
    let cell = SectorCoord::containing(position, edge);
    info!("medkit_rescue_sandbox: '{id}' in cell {cell} posts a distress call");
    Some(ChosenStation { id, cell, position })
}

/// Put the fixture hold in place: [`MedkitFixtureType::count`] Medkits.
fn set_the_hold(world: &mut World, player: Entity, fixture: MedkitFixtureType) {
    let items = world.resource::<GameItems>().clone();
    let medkit = items.role(ItemRoleType::Medkit).id.clone();
    let mut hold = world
        .get_mut::<ShipInventory>(player)
        .expect("medkit_rescue_sandbox: the player carries a hold");
    let aboard = hold.count(&medkit);
    // The hold refuses a move of zero units.
    if aboard > fixture.count() {
        hold.remove(&medkit, aboard - fixture.count());
    } else if aboard < fixture.count() {
        hold.add(&items, &medkit, fixture.count() - aboard);
    }
    info!(
        "medkit_rescue_sandbox: the hold carried {aboard} {medkit}, the fixture leaves {}",
        fixture.count()
    );
}

/// Jump the player [`CALLER_STANDOFF`] off the caller toward the poster, at
/// rest, bow on the caller.
fn stand_off_the_caller(world: &mut World, player: Entity, caller: Vec3, poster: Vec3) {
    let back = (poster - caller).normalize_or(Vec3::Z);
    let at = Meters3::from_engine(caller + back * CALLER_STANDOFF.to_engine());
    open_world_stage::jump_facing(world, player, at, -back);
}

/// Advance the staging one phase at a time.
///
/// # Panics
///
/// When the poster's board no longer posts a distress call at the dock.
fn drive_the_sandbox(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    let Some(player) = open_world_stage::the_player(world) else {
        return;
    };
    let docked = world.get::<DockedShip>(player).is_some();
    let next = match phase {
        SandboxPhaseType::Ready => return,
        SandboxPhaseType::Streaming => {
            if !(open_world_stage::window_settled(world) && docked) {
                return;
            }
            world.trigger(DockingReleaseRequest { entity: player });
            SandboxPhaseType::Choosing
        }
        SandboxPhaseType::Choosing => {
            if docked {
                return;
            }
            let Some(chosen) = choose_the_poster(world, player) else {
                return;
            };
            open_world_stage::park_beside(world, player, chosen.position, PARK_DISTANCE);
            world.resource_mut::<Sandbox>().station = Some(chosen);
            SandboxPhaseType::Jumped
        }
        SandboxPhaseType::Jumped => {
            let chosen = world
                .resource::<Sandbox>()
                .station
                .clone()
                .expect("medkit_rescue_sandbox: the poster is chosen");
            // A hull the station parks takes its port first, so the berth
            // takes a port left free.
            let stands = open_world_stage::body_by_id(world, &chosen.id).is_some_and(|station| {
                world.get::<Position>(station).is_some()
                    && world.get::<JobBoard>(station).is_some()
                    && world.get::<PendingParkedHulls>(station).is_none()
                    && world.get::<ParkedHulls>(station).is_none_or(|parked| {
                        parked.iter().all(|hull| {
                            matches!(
                                world.get::<ParkedHullType>(hull),
                                Some(ParkedHullType::Hire { .. })
                            ) || world.get::<DockedShip>(hull).is_some()
                        })
                    })
            });
            if !(open_world_stage::window_is(world, chosen.cell) && stands) {
                return;
            }
            SandboxPhaseType::Docking
        }
        SandboxPhaseType::Docking => {
            let chosen = world
                .resource::<Sandbox>()
                .station
                .clone()
                .expect("medkit_rescue_sandbox: the poster is chosen");
            let Some(station) = open_world_stage::body_by_id(world, &chosen.id) else {
                return;
            };
            if !docked {
                if open_world_stage::berth(world, player, station) {
                    world.trigger(DockingConnectionRequest {
                        entity: player,
                        target: station,
                    });
                }
                return;
            }
            let job = world
                .get::<JobBoard>(station)
                .and_then(|board| board.0.iter().find(|job| caller_of(job).is_some()))
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "medkit_rescue_sandbox: the board of '{}' no longer posts a distress call",
                        chosen.id
                    )
                });
            world.write_message(JobAcceptCommand { id: job.id.clone() });
            world.resource_mut::<Sandbox>().job = Some(job);
            SandboxPhaseType::Accepting
        }
        SandboxPhaseType::Accepting => {
            let job = world.resource::<Sandbox>().job.clone();
            let held = job.is_some_and(|job| world.resource::<Jobs>().job(&job.id).is_some());
            if !held {
                return;
            }
            let fixture = world.resource::<Cli>().medkit;
            set_the_hold(world, player, fixture);
            world.trigger(DockingReleaseRequest { entity: player });
            SandboxPhaseType::Leaving
        }
        SandboxPhaseType::Leaving => {
            let sandbox = world.resource::<Sandbox>();
            let (Some(chosen), Some(job)) = (sandbox.station.clone(), sandbox.job.clone()) else {
                return;
            };
            let caller = caller_of(&job)
                .cloned()
                .expect("medkit_rescue_sandbox: the held job is a distress call");
            let place = |world: &mut World, id: &GeneratedBodyId| {
                open_world_stage::body_by_id(world, id)
                    .and_then(|body| world.get::<Position>(body).map(|position| position.0))
            };
            let (Some(caller_at), Some(poster_at)) =
                (place(world, &caller), place(world, &chosen.id))
            else {
                return;
            };
            if docked {
                return;
            }
            stand_off_the_caller(world, player, caller_at, poster_at);
            info!(
                "medkit_rescue_sandbox: the call is held and the caller '{caller}' is {} m ahead",
                CALLER_STANDOFF.0
            );
            SandboxPhaseType::Ready
        }
    };
    world.resource_mut::<Sandbox>().phase = next;
}

/// A label and a line on the caller and on the poster, and the panel.
fn draw_the_rescue(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    sandbox: Res<Sandbox>,
    jobs: Res<Jobs>,
    items: Res<GameItems>,
    q_player: Query<
        (&GlobalTransform, &ShipInventory, Option<&DockedShip>),
        With<PlayerSpaceshipMarker>,
    >,
    q_bodies: Query<(
        Entity,
        &GeneratedBodyId,
        &GlobalTransform,
        Option<&ShipInventory>,
    )>,
    q_connections: Query<&DockingConnection>,
) {
    if sandbox.phase != SandboxPhaseType::Ready {
        panel.line(format!("Staging the rescue: {:?}", sandbox.phase));
        return;
    }
    let (Ok((at, hold, docked)), Some(chosen), Some(job)) = (
        q_player.single(),
        sandbox.station.as_ref(),
        sandbox.job.as_ref(),
    ) else {
        return;
    };
    let medkit = &items.role(ItemRoleType::Medkit).id;
    let connection = docked.and_then(|docked| q_connections.get(docked.connection).ok());
    let body = |id: &GeneratedBodyId| q_bodies.iter().find(|(_, worn, ..)| *worn == id);
    let range =
        |to: &GlobalTransform| Meters::from_engine(to.translation().distance(at.translation()));

    let (state, color) = if jobs.is_completed(&job.id) {
        ("handed in", palette::OK)
    } else if jobs.is_done(&job.id) {
        ("done: hand it in at the poster", palette::OK)
    } else if jobs.job(&job.id).is_some() {
        ("held, not done", palette::ALERT)
    } else {
        ("dropped", palette::ALERT)
    };
    panel.line(format!("{}: {state}", job.title(&items)));
    panel.line(format!("Medkits aboard: {}", hold.count(medkit)));

    let mut with_caller = false;
    if let Some((caller, _, caller_at, _)) = caller_of(job).and_then(body) {
        with_caller = connection.is_some_and(|connection| connection.joins(caller));
        if !with_caller {
            gizmos.line(at.translation(), caller_at.translation(), color);
        }
        panel.label_tinted(
            caller_at.translation(),
            format!("CALLER\n{:.0} m", range(caller_at).0),
            color,
        );
    }
    if let Some((_, _, poster_at, stock)) = body(&chosen.id) {
        let sells = stock.map_or(0, |stock| stock.count(medkit));
        panel.line(format!(
            "Poster: {:.0} m, stocks {sells} Medkits",
            range(poster_at).0
        ));
        panel.label(
            poster_at.translation(),
            format!("POSTER\n{:.0} m", range(poster_at).0),
        );
    }
    if with_caller {
        panel.hint("D", "undock");
        panel.hint("TAB", "Jobs: read the call");
    } else if docked.is_some() {
        panel.hint("TAB", "Jobs > Hand in, Inventory > buy a Medkit");
        panel.hint("D", "undock");
    } else {
        panel.hint("Ctrl", "hold on the caller to lock it");
        panel.hint("D", "dock");
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS};

    use super::*;

    /// Seconds the world gets to start, stream, dock and accept on a
    /// software GPU.
    const SETUP_SECS: f32 = 800.0;
    /// Frames the jump gets to settle.
    const SETTLE_FRAMES: u32 = 10;

    /// The frame the capture path writes.
    const SHOT: &str = "medkit-rescue-sandbox.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The staging: the call is held and not done, the caller stands
    /// [`CALLER_STANDOFF`] off the free player at rest, and the hold carries
    /// the fixture's Medkits.
    fn assert_the_staging(
        fixture: MedkitFixtureType,
    ) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let job = world
                .resource::<Sandbox>()
                .job
                .clone()
                .expect("medkit_rescue_sandbox: a call is chosen");
            let jobs = world.resource::<Jobs>();
            assert!(
                jobs.job(&job.id).is_some() && !jobs.is_done(&job.id),
                "medkit_rescue_sandbox: the call '{}' is held and not done",
                job.id
            );
            let player = open_world_stage::the_player(world)
                .expect("medkit_rescue_sandbox: one player ship");
            assert!(
                world.get::<DockedShip>(player).is_none(),
                "medkit_rescue_sandbox: the player starts free"
            );
            let caller_id = caller_of(&job)
                .cloned()
                .expect("medkit_rescue_sandbox: the held job is a distress call");
            let caller = open_world_stage::body_by_id(world, &caller_id)
                .expect("medkit_rescue_sandbox: the caller stands");
            let place = |entity: Entity| {
                world
                    .get::<Position>(entity)
                    .expect("medkit_rescue_sandbox: a hull has a physics position")
                    .0
            };
            let gap = Meters::from_engine(place(player).distance(place(caller)));
            let speed = world
                .get::<LinearVelocity>(player)
                .map_or(0.0, |velocity| Meters::from_engine(velocity.0.length()).0);
            assert!(
                (gap.0 - CALLER_STANDOFF.0).abs() < 50.0 && speed < 1.0,
                "medkit_rescue_sandbox: the player stands {} m off the caller at rest: {:.0} m \
                 at {speed:.2} m/s",
                CALLER_STANDOFF.0,
                gap.0
            );
            let medkit = world
                .resource::<GameItems>()
                .role(ItemRoleType::Medkit)
                .id
                .clone();
            let aboard = world
                .get::<ShipInventory>(player)
                .expect("medkit_rescue_sandbox: the player carries a hold")
                .count(&medkit);
            assert_eq!(
                aboard,
                fixture.count(),
                "medkit_rescue_sandbox: the fixture hold"
            );
            info!(
                "medkit_rescue_sandbox: PASS staged {:.0} m off '{caller_id}' with the call \
                 held and {aboard} Medkit(s) aboard",
                gap.0
            );
        }
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script(fixture: MedkitFixtureType) -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("medkit rescue sandbox: start, dock, accept and stand off the caller")
            .until(when(|world| {
                world
                    .get_resource::<Sandbox>()
                    .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
            }))
            .deadline(SETUP_SECS)
            .add()
            .step("medkit rescue sandbox: settle the jump")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("medkit rescue sandbox: check the staging")
            .on_enter(assert_the_staging(fixture))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("medkit rescue sandbox: shoot the view")
            .on_enter(|world: &mut World| shoot(world, SHOT))
            .until(shot_written(SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }
}
