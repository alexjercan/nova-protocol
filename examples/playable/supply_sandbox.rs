//! supply_sandbox: start 500 m off the caller of a held supply job, with
//! the goods it asks for aboard or one short, and dock with the caller.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the bounty
//! hunter, whose boards lean to supply jobs, as the Create World modal does
//! but without a save folder. When the start's window has streamed in, it
//! picks the nearest living station whose board posts a supply job, berths
//! the player there until the dock takes, and accepts the job with
//! [`JobAcceptCommand`], the message the board row's Accept button writes.
//! The open world then puts the caller in the world, 10 km off the poster.
//! The player undocks and jumps 500 m off the caller, at rest, bow on it.
//!
//! THE GOODS AND THE HOLD ARE DISCLOSED FIXTURE STATE. Before the accept the
//! sandbox rewrites the posted job's goods to `--item` (`medkit` 3, the
//! default, framed as a rescue; `rations` 12, a resupply; `ammo` 500 PDC
//! rounds, a rearm; `plates` 4 hull plates, a repair); the reward stays the
//! board's. After the accept the hold carries exactly that stack, or one
//! fewer with `--short`.
//!
//! What to do:
//!
//! 1. fly to the caller, hold Ctrl on it to lock it, press D to dock;
//! 2. with the whole stack aboard the dock takes it: the hold loses the
//!    goods and the job is done. TAB > Jobs lists it done;
//! 3. with `--short` the HUD reads `JOB-1: SHORT 1 <ITEM>`, the hold keeps
//!    every unit and the job stays open. The poster sells the goods when
//!    the panel says so: fly back, buy one, come back and dock again;
//! 4. the hand-in is at the poster, 10 km back: dock there, TAB > Jobs,
//!    Hand in.
//!
//! The diagnostics layer (F3) is on at start: a label and a line on the
//! caller and on the poster with the range, and panel lines with the state
//! of the job, the goods aboard and the goods the poster stocks.
//!
//! ```text
//! nix develop --command cargo run --example supply_sandbox --features debug
//! nix develop --command cargo run --example supply_sandbox --features debug -- --item rations --short
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk waits for the staging, checks it and exits. `NOVA_CAPTURE=1` shoots
//! `supply-sandbox.png`.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use avian3d::prelude::{LinearVelocity, Position};
use bevy::prelude::*;
use clap::{Parser, ValueEnum};
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, nova_interface::jobs::JobAcceptCommand, prelude::*};
use open_world_stage::{ChosenStation, OpenWorldStartPlugin};

/// The goods the fixture job asks for.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SupplyItemType {
    /// Medkits: a rescue.
    #[default]
    Medkit,
    /// Rations: a resupply.
    Rations,
    /// PDC rounds: a rearm.
    Ammo,
    /// Hull plates: a repair.
    Plates,
}

impl SupplyItemType {
    /// The base item the goods are.
    fn role(self) -> ItemRoleType {
        match self {
            Self::Medkit => ItemRoleType::Medkit,
            Self::Rations => ItemRoleType::Rations,
            Self::Ammo => ItemRoleType::PdcRound,
            Self::Plates => ItemRoleType::HullPlate,
        }
    }

    /// How many the fixture job asks for.
    fn count(self) -> u32 {
        match self {
            Self::Medkit => 3,
            Self::Rations => 12,
            Self::Ammo => 500,
            Self::Plates => 4,
        }
    }
}

#[derive(Parser, Resource, Clone, Copy)]
#[command(name = "supply_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start 500 m off the caller of a held supply job, with the goods it asks for \
             aboard or one short, and dock with the caller",
    long_about = None
)]
struct Cli {
    /// The goods the fixture job asks for.
    #[arg(long, value_enum, default_value_t = SupplyItemType::Medkit)]
    item: SupplyItemType,
    /// Carry one fewer than the job asks for: the dock is refused.
    #[arg(long)]
    short: bool,
}

impl Cli {
    /// How many of the goods the fixture hold carries.
    fn aboard(self) -> u32 {
        self.item.count() - u32::from(self.short)
    }

    /// The intro line.
    fn intro(self) -> &'static str {
        if self.short {
            "A supply caller drifts 500 m ahead and the hold is one short: lock it, dock."
        } else {
            "A supply caller drifts 500 m ahead with its goods aboard: lock it, dock."
        }
    }
}

/// The world seed and the start role: `system_job_supply`'s, whose window
/// around the start holds a living station that posts a supply job.
const SANDBOX_SEED: u32 = 5;
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
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(cli.intro()));
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        draw_the_supply
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
    app.add_plugins(smoke::script(cli));

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
    /// supply job is looked for.
    Choosing,
    /// The player jumped beside the poster; its window is streaming in.
    Jumped,
    /// The player is put on the poster's berth until the dock takes.
    Docking,
    /// The accept was asked for.
    Accepting,
    /// The job is held; the player lets go of the poster and the caller
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
    /// The supply job, with the fixture's goods.
    job: Option<Job>,
}

/// The caller's id of a supply job.
fn caller_of(job: &Job) -> Option<&GeneratedBodyId> {
    match &job.kind {
        JobType::Supply { caller, .. } => Some(caller),
        JobType::Procure { .. }
        | JobType::Recover { .. }
        | JobType::Escort { .. }
        | JobType::Distress { .. }
        | JobType::Neutralize { .. } => None,
    }
}

/// The nearest living station, not on the Enemy side, whose board posts a
/// supply job, with ties broken by id.
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
    info!("supply_sandbox: '{id}' in cell {cell} posts a supply job");
    Some(ChosenStation { id, cell, position })
}

/// Rewrite the supply job on `station`'s board to ask for the fixture's
/// goods, and return it.
///
/// # Panics
///
/// When the board no longer posts a supply job.
fn rewrite_the_job(world: &mut World, station: Entity, fixture: Cli) -> Job {
    let item = ItemDesignId::from(fixture.item.role());
    let mut board = world
        .get_mut::<JobBoard>(station)
        .expect("supply_sandbox: the poster carries a board");
    let job = board
        .0
        .iter_mut()
        .find(|job| caller_of(job).is_some())
        .unwrap_or_else(|| panic!("supply_sandbox: the board no longer posts a supply job"));
    let caller = caller_of(job)
        .cloned()
        .expect("a supply job names a caller");
    job.kind = JobType::Supply {
        caller,
        item,
        count: fixture.item.count(),
    };
    job.clone()
}

/// Put the fixture hold in place: [`Cli::aboard`] of the goods.
fn set_the_hold(world: &mut World, player: Entity, fixture: Cli) {
    let items = world.resource::<GameItems>().clone();
    let item = ItemDesignId::from(fixture.item.role());
    let mut hold = world
        .get_mut::<ShipInventory>(player)
        .expect("supply_sandbox: the player carries a hold");
    let aboard = hold.count(&item);
    // The hold refuses a move of zero units.
    if aboard > fixture.aboard() {
        hold.remove(&item, aboard - fixture.aboard());
    } else if aboard < fixture.aboard() {
        hold.add(&items, &item, fixture.aboard() - aboard);
    }
    info!(
        "supply_sandbox: the hold carried {aboard} {item}, the fixture leaves {}",
        fixture.aboard()
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
/// When the poster's board no longer posts a supply job at the dock.
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
                .expect("supply_sandbox: the poster is chosen");
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
                .expect("supply_sandbox: the poster is chosen");
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
            let fixture = *world.resource::<Cli>();
            let job = rewrite_the_job(world, station, fixture);
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
            let fixture = *world.resource::<Cli>();
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
                .expect("supply_sandbox: the held job is a supply job");
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
                "supply_sandbox: the job is held and the caller '{caller}' is {} m ahead",
                CALLER_STANDOFF.0
            );
            SandboxPhaseType::Ready
        }
    };
    world.resource_mut::<Sandbox>().phase = next;
}

/// A label and a line on the caller and on the poster, and the panel.
fn draw_the_supply(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    sandbox: Res<Sandbox>,
    cli: Res<Cli>,
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
        panel.line(format!("Staging the supply job: {:?}", sandbox.phase));
        return;
    }
    let (Ok((at, hold, docked)), Some(chosen), Some(job)) = (
        q_player.single(),
        sandbox.station.as_ref(),
        sandbox.job.as_ref(),
    ) else {
        return;
    };
    let goods = items.role(cli.item.role());
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
    panel.line(format!(
        "{} aboard: {} of {}",
        goods.name,
        hold.count(&goods.id),
        cli.item.count()
    ));

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
        let sells = stock.map_or(0, |stock| stock.count(&goods.id));
        panel.line(format!(
            "Poster: {:.0} m, stocks {sells} {}",
            range(poster_at).0,
            goods.name
        ));
        panel.label(
            poster_at.translation(),
            format!("POSTER\n{:.0} m", range(poster_at).0),
        );
    }
    if with_caller {
        panel.hint("D", "undock");
        panel.hint("TAB", "Jobs: read the job");
    } else if docked.is_some() {
        panel.hint("TAB", "Jobs > Hand in, Inventory > buy the goods");
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
    const SHOT: &str = "supply-sandbox.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The staging: the job asks for the fixture's goods and is held and not
    /// done, the caller stands [`CALLER_STANDOFF`] off the free player at
    /// rest, and the hold carries [`Cli::aboard`] of the goods.
    fn assert_the_staging(fixture: Cli) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let job = world
                .resource::<Sandbox>()
                .job
                .clone()
                .expect("supply_sandbox: a supply job is chosen");
            let item = ItemDesignId::from(fixture.item.role());
            let jobs = world.resource::<Jobs>();
            let held = jobs
                .job(&job.id)
                .unwrap_or_else(|| panic!("supply_sandbox: the job '{}' is held", job.id));
            assert!(
                matches!(&held.kind, JobType::Supply { item: asked, count, .. }
                    if *asked == item && *count == fixture.item.count()),
                "supply_sandbox: the held job asks for the fixture's goods: {:?}",
                held.kind
            );
            assert!(
                !jobs.is_done(&job.id),
                "supply_sandbox: the job '{}' is not done",
                job.id
            );
            let player =
                open_world_stage::the_player(world).expect("supply_sandbox: one player ship");
            assert!(
                world.get::<DockedShip>(player).is_none(),
                "supply_sandbox: the player starts free"
            );
            let caller_id = caller_of(&job)
                .cloned()
                .expect("supply_sandbox: the held job is a supply job");
            let caller = open_world_stage::body_by_id(world, &caller_id)
                .expect("supply_sandbox: the caller stands");
            let place = |entity: Entity| {
                world
                    .get::<Position>(entity)
                    .expect("supply_sandbox: a hull has a physics position")
                    .0
            };
            let gap = Meters::from_engine(place(player).distance(place(caller)));
            let speed = world
                .get::<LinearVelocity>(player)
                .map_or(0.0, |velocity| Meters::from_engine(velocity.0.length()).0);
            assert!(
                (gap.0 - CALLER_STANDOFF.0).abs() < 50.0 && speed < 1.0,
                "supply_sandbox: the player stands {} m off the caller at rest: {:.0} m \
                 at {speed:.2} m/s",
                CALLER_STANDOFF.0,
                gap.0
            );
            let aboard = world
                .get::<ShipInventory>(player)
                .expect("supply_sandbox: the player carries a hold")
                .count(&item);
            assert_eq!(aboard, fixture.aboard(), "supply_sandbox: the fixture hold");
            let caller_stock = world
                .get::<ShipInventory>(caller)
                .map_or(0, |hold| hold.count(&item));
            assert_eq!(
                caller_stock, 0,
                "supply_sandbox: the caller carries none of the goods it asks for"
            );
            info!(
                "supply_sandbox: PASS staged {:.0} m off '{caller_id}' with the job held, \
                 {aboard} of {} {item} aboard",
                gap.0,
                fixture.item.count()
            );
        }
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script(fixture: Cli) -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("supply sandbox: start, dock, accept and stand off the caller")
            .until(when(|world| {
                world
                    .get_resource::<Sandbox>()
                    .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
            }))
            .deadline(SETUP_SECS)
            .add()
            .step("supply sandbox: settle the jump")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("supply sandbox: check the staging")
            .on_enter(assert_the_staging(fixture))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("supply sandbox: shoot the view")
            .on_enter(|world: &mut World| shoot(world, SHOT))
            .until(shot_written(SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }
}
