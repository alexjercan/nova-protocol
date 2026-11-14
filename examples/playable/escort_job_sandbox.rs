//! escort_job_sandbox: start docked at the poster of a held escort job whose
//! raiders come from one advancement band, and fly the fight.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the armed
//! military start, as the Create World modal does but without a save folder.
//! When the start's window has streamed in, it picks the nearest living
//! station, not on the Enemy side, whose board posts an escort, berths the
//! player there until the dock takes, and accepts the escort with
//! [`JobAcceptCommand`], the message the board row's Accept button writes.
//! The open world then puts the hauler 1 km off the poster and the raiders
//! in wait down its leg.
//!
//! THE POSTER'S CIVILIZATION IS DISCLOSED FIXTURE STATE. A station posts and
//! spawns its escort by the civilization its market names. `--band` picks a
//! living, non-Enemy civilization of the world seed whose advancement falls
//! in that band's row of the escort raider table, nearest the band's
//! advancement, and the sandbox writes it into the market of every station
//! as the station spawns, before any board posts. The board's own draw then
//! sets the raider count, and the accept's own spawn lays each raider out
//! to the band's size budget. Nothing else is set.
//!
//! | `--band` | advancement | raiders | each raider |
//! | - | - | - | - |
//! | `start` (default) | about 0.10 | 1 | up to 40 sections, 1 turret |
//! | `frontier` | about 0.35 | 1-2 | up to 68 sections, 2 turrets |
//! | `developed` | about 0.60 | 2-3 | up to 108 sections, 3 turrets |
//! | `core` | about 0.85 | 3 | the civilization's own scavenger |
//!
//! On start the log prints the band, the advancement, the raider count, each
//! raider's sections and turrets, and the hauler's hp.
//!
//! What to do:
//!
//! 1. `D` to undock, fly within 1 km of the hauler: it leaves for its mark;
//! 2. hold formation on the hauler; the raiders spring about 2 km down the
//!    leg;
//! 3. lock a raider (hold `Ctrl` on it), fire (LMB): the job is done when
//!    the hauler reaches its mark alive. Dock at the poster and TAB > Jobs >
//!    Hand in.
//!
//! The diagnostics layer (F3) is on at start: a label on each raider with
//! its sections, turrets, hp and range, a label on the hauler with its hp,
//! and panel lines with the band, the advancement and the raider count.
//!
//! ```text
//! nix develop --command cargo run --example escort_job_sandbox --features debug
//! nix develop --command cargo run --example escort_job_sandbox --features debug -- --band core
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk waits for the staging, checks the raider count against the band's
//! row and each raider against the band's size budget, undocks, faces the
//! raiders and exits. `NOVA_CAPTURE=1` shoots
//! `escort-job-sandbox-<band>.png`.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use avian3d::prelude::Position;
use bevy::prelude::*;
use clap::{Parser, ValueEnum};
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, nova_interface::jobs::JobAcceptCommand, prelude::*};
use open_world_stage::{ChosenStation, OpenWorldStartPlugin};

/// An advancement band of the escort raider table.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
enum BandType {
    /// Below 0.25: one raider about the start ship's size.
    #[default]
    Start,
    /// 0.25 to 0.5: one or two raiders, up to two turrets each.
    Frontier,
    /// 0.5 to 0.75: two or three raiders, up to three turrets each.
    Developed,
    /// 0.75 and over: three of the civilization's own scavengers.
    Core,
}

impl BandType {
    const ALL: [Self; 4] = [Self::Start, Self::Frontier, Self::Developed, Self::Core];

    /// The advancement the band's civilization is chosen nearest to.
    fn advancement(self) -> f32 {
        match self {
            Self::Start => 0.1,
            Self::Frontier => 0.35,
            Self::Developed => 0.6,
            Self::Core => 0.85,
        }
    }

    /// The name `--band` takes.
    fn label(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Frontier => "frontier",
            Self::Developed => "developed",
            Self::Core => "core",
        }
    }
}

/// The raider table row an advancement falls in: the least and the most
/// raiders, and each raider's size.
fn raider_row(advancement: f32) -> ((u32, u32), ShipSizeType) {
    (
        escort_raider_counts(advancement),
        escort_raider_size(advancement),
    )
}

#[derive(Parser, Resource, Clone, Copy)]
#[command(name = "escort_job_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start docked at the poster of a held escort job whose raiders come from one \
             advancement band",
    long_about = None
)]
struct Cli {
    /// The advancement band the escort's raiders come from.
    #[arg(long, value_enum, default_value_t = BandType::Start)]
    band: BandType,
}

/// The world seed and the start role: `system_job_escort`'s, whose window
/// around the start holds a living station that posts an escort for the
/// armed military start.
const SANDBOX_SEED: u32 = 37;
const SANDBOX_ROLE: &str = "military";
/// How far from the poster the player parks while its window streams in.
const PARK_DISTANCE: Meters = Meters(2_000.0);
/// How many lattice nodes out from the origin, along x, the band's
/// civilization is looked for: past the advancement curve's saturation.
const NODE_REACH_X: i32 = 20;
/// How many lattice nodes off the x axis, along y and z.
const NODE_REACH_YZ: i32 = 2;

/// The civilization the band writes into every station's market.
#[derive(Resource, Clone, Copy, Debug)]
struct BandCivilization(Civilization);

/// The living, non-Enemy civilization of `seed` whose advancement falls in
/// `band`'s raider table row, nearest the band's advancement, ties broken by
/// node.
///
/// # Panics
///
/// When no civilization in reach of the search falls in the row.
fn band_civilization(seed: u32, band: BandType) -> Civilization {
    let target = band.advancement();
    let row = raider_row(target);
    let field = CivilizationField::new(seed);
    let mut best: Option<Civilization> = None;
    for x in -NODE_REACH_X..=NODE_REACH_X {
        for y in -NODE_REACH_YZ..=NODE_REACH_YZ {
            for z in -NODE_REACH_YZ..=NODE_REACH_YZ {
                let civilization = field.civilization([x, y, z]);
                if civilization.status != CivilizationStatusType::Living
                    || civilization.stance == StanceType::Enemy
                    || raider_row(civilization.advancement) != row
                {
                    continue;
                }
                let off = (civilization.advancement - target).abs();
                if best.is_none_or(|best| off < (best.advancement - target).abs()) {
                    best = Some(civilization);
                }
            }
        }
    }
    best.unwrap_or_else(|| {
        panic!(
            "escort_job_sandbox: no living, non-Enemy civilization of seed {seed} falls in the \
             '{}' row {row:?}",
            band.label()
        )
    })
}

/// Write the band's civilization into a station's market.
fn repoint(
    entity: Entity,
    band: &BandCivilization,
    markets: &mut Query<&mut TraderMarket, With<StationMarker>>,
) {
    if let Ok(mut market) = markets.get_mut(entity) {
        if market.civilization != band.0.id {
            market.civilization = band.0.id;
        }
    }
}

/// A station's market as it is written names the band's civilization.
fn repoint_inserted_market(
    insert: On<Insert<TraderMarket>>,
    band: Res<BandCivilization>,
    mut markets: Query<&mut TraderMarket, With<StationMarker>>,
) {
    repoint(insert.entity, &band, &mut markets);
}

/// A market on a hull that becomes a station names the band's civilization.
fn repoint_new_station(
    add: On<Add<StationMarker>>,
    band: Res<BandCivilization>,
    mut markets: Query<&mut TraderMarket, With<StationMarker>>,
) {
    repoint(add.entity, &band, &mut markets);
}

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    // Each band names its own row of the raider table: a table that merges
    // two rows fails here, not in a fight.
    let rows: Vec<_> = BandType::ALL
        .iter()
        .map(|band| raider_row(band.advancement()))
        .collect();
    for (index, row) in rows.iter().enumerate() {
        assert!(
            !rows[..index].contains(row),
            "escort_job_sandbox: the band '{}' shares its raider row {row:?} with another",
            BandType::ALL[index].label()
        );
    }
    let civilization = band_civilization(SANDBOX_SEED, cli.band);

    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.add_plugins(OpenWorldStartPlugin {
        seed: SANDBOX_SEED,
        role: SANDBOX_ROLE.to_string(),
    });
    app.insert_resource(cli);
    app.insert_resource(BandCivilization(civilization));
    app.init_resource::<Sandbox>();
    app.add_observer(repoint_inserted_market);
    app.add_observer(repoint_new_station);
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "An escort is held: D to undock, fly within 1 km of the hauler, fight its raiders.",
    ));
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        draw_the_escort
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
    /// The player let go of the starting station; a board that posts an
    /// escort is looked for.
    Choosing,
    /// The player jumped beside the poster; its window is streaming in.
    Jumped,
    /// The player is put on the poster's berth until the dock takes.
    Docking,
    /// The accept was asked for; the hauler and the raiders are spawning.
    Accepting,
    /// The hauler and every raider stand with their sections; the counts
    /// settle for [`SETTLE_FRAMES`].
    Settling(u32),
    /// The staging is printed. The player has the run.
    Ready,
}

/// Frames the spawned hulls get, with sections and hp, before the print.
const SETTLE_FRAMES: u32 = 10;

/// The staging state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
    /// The poster.
    station: Option<ChosenStation>,
    /// The escort job.
    job: Option<Job>,
}

/// One spawned hull as the sandbox reads it.
#[derive(Clone, Debug)]
struct HullRead {
    id: GeneratedBodyId,
    entity: Entity,
    sections: u32,
    turrets: u32,
    hp: (f32, f32),
}

/// The raider count a held escort names.
fn raiders_of(job: &Job) -> Option<u32> {
    match &job.kind {
        JobType::Escort { raiders, .. } => Some(*raiders),
        JobType::Procure { .. }
        | JobType::Recover { .. }
        | JobType::Supply { .. }
        | JobType::Distress { .. }
        | JobType::Neutralize { .. } => None,
    }
}

/// The hauler a held escort names.
fn hauler_of(job: &Job) -> Option<&GeneratedBodyId> {
    match &job.kind {
        JobType::Escort { hauler, .. } => Some(hauler),
        JobType::Procure { .. }
        | JobType::Recover { .. }
        | JobType::Supply { .. }
        | JobType::Distress { .. }
        | JobType::Neutralize { .. } => None,
    }
}

/// The hull wearing `id`, once it stands with sections and hp.
fn read_hull(world: &mut World, id: &GeneratedBodyId) -> Option<HullRead> {
    let entity = open_world_stage::body_by_id(world, id)?;
    let hp = world
        .get::<Health>(entity)
        .map(|health| (health.current, health.max))?;
    let (mut sections, mut turrets) = (0, 0);
    for (child_of, turret) in world
        .query_filtered::<(&ChildOf, Has<TurretSectionMarker>), With<SectionMarker>>()
        .iter(world)
    {
        if child_of.parent() == entity {
            sections += 1;
            turrets += u32::from(turret);
        }
    }
    (sections > 0).then(|| HullRead {
        id: id.clone(),
        entity,
        sections,
        turrets,
        hp,
    })
}

/// The hauler and every raider of `job`, once each stands with sections
/// and hp.
fn read_the_escort(world: &mut World, job: &Job) -> Option<(HullRead, Vec<HullRead>)> {
    let hauler = read_hull(world, hauler_of(job)?)?;
    let raiders: Option<Vec<HullRead>> = job
        .raider_ids()
        .iter()
        .map(|id| read_hull(world, id))
        .collect();
    Some((hauler, raiders?))
}

/// The nearest living station, not on the Enemy side, whose board posts an
/// escort, with ties broken by id.
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
                && board.0.iter().any(|job| raiders_of(job).is_some())
        })
        .map(|(id, position, ..)| (position.0.distance(at), id.clone(), position.0))
        .collect();
    posters.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let (_, id, at) = posters.into_iter().next()?;
    let position = Meters3::from_engine(at);
    let cell = SectorCoord::containing(position, edge);
    info!("escort_job_sandbox: '{id}' in cell {cell} posts an escort");
    Some(ChosenStation { id, cell, position })
}

/// Print the staging: the band, the advancement, the raider count against
/// its row, each raider's sections and turrets, and the hauler's hp.
fn print_the_staging(world: &mut World, job: &Job) {
    let band = world.resource::<Cli>().band;
    let civilization = world.resource::<BandCivilization>().0;
    let (hauler, raiders) =
        read_the_escort(world, job).expect("escort_job_sandbox: the escort stands");
    let ((least, most), size) = raider_row(civilization.advancement);
    info!(
        "escort_job_sandbox: STAGED band={} advancement={:.3} civilization='{}' raiders={} \
         row={least}..={most} size={size:?} hauler='{}' hauler_hp={:.0}/{:.0}",
        band.label(),
        civilization.advancement,
        civilization.id.name(),
        raiders.len(),
        hauler.id,
        hauler.hp.0,
        hauler.hp.1
    );
    for (index, raider) in raiders.iter().enumerate() {
        info!(
            "escort_job_sandbox: raider {index} '{}': {} sections, {} turret(s), hp {:.0}/{:.0}",
            raider.id, raider.sections, raider.turrets, raider.hp.0, raider.hp.1
        );
    }
}

/// Advance the staging one phase at a time.
///
/// # Panics
///
/// When the poster's board no longer posts an escort at the dock.
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
                .expect("escort_job_sandbox: the poster is chosen");
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
                .expect("escort_job_sandbox: the poster is chosen");
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
                .and_then(|board| board.0.iter().find(|job| raiders_of(job).is_some()))
                .cloned()
                .unwrap_or_else(|| {
                    panic!("escort_job_sandbox: the board no longer posts an escort")
                });
            world.write_message(JobAcceptCommand { id: job.id.clone() });
            world.resource_mut::<Sandbox>().job = Some(job);
            SandboxPhaseType::Accepting
        }
        SandboxPhaseType::Accepting => {
            let Some(job) = world.resource::<Sandbox>().job.clone() else {
                return;
            };
            if world.resource::<Jobs>().job(&job.id).is_none()
                || read_the_escort(world, &job).is_none()
            {
                return;
            }
            SandboxPhaseType::Settling(0)
        }
        SandboxPhaseType::Settling(frames) => {
            if frames < SETTLE_FRAMES {
                SandboxPhaseType::Settling(frames + 1)
            } else {
                let job = world
                    .resource::<Sandbox>()
                    .job
                    .clone()
                    .expect("escort_job_sandbox: the escort is held");
                print_the_staging(world, &job);
                SandboxPhaseType::Ready
            }
        }
    };
    world.resource_mut::<Sandbox>().phase = next;
}

/// A label on each raider and on the hauler, and the panel.
#[expect(
    clippy::type_complexity,
    reason = "the section query reads two markers"
)]
fn draw_the_escort(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    sandbox: Res<Sandbox>,
    cli: Res<Cli>,
    band: Res<BandCivilization>,
    jobs: Res<Jobs>,
    q_player: Query<(&GlobalTransform, Option<&DockedShip>), With<PlayerSpaceshipMarker>>,
    q_bodies: Query<(Entity, &GeneratedBodyId, &GlobalTransform, Option<&Health>)>,
    q_sections: Query<
        (&ChildOf, Has<TurretSectionMarker>),
        (With<SectionMarker>, Without<SectionInactiveMarker>),
    >,
) {
    let civilization = band.0;
    panel.line(format!(
        "Band {}: '{}' at advancement {:.3}",
        cli.band.label(),
        civilization.id.name(),
        civilization.advancement
    ));
    if sandbox.phase != SandboxPhaseType::Ready {
        panel.line(format!("Staging the escort: {:?}", sandbox.phase));
        return;
    }
    let (Ok((at, docked)), Some(job)) = (q_player.single(), sandbox.job.as_ref()) else {
        return;
    };
    let ((least, most), _) = raider_row(civilization.advancement);
    let state = if jobs.is_completed(&job.id) {
        "handed in"
    } else if jobs.is_done(&job.id) {
        "done: hand it in at the poster"
    } else if jobs.job(&job.id).is_some() {
        "held, not done"
    } else {
        "dropped"
    };
    panel.line(format!(
        "Escort: {state}; {} raider(s), row {least}-{most}",
        job.raider_ids().len()
    ));
    let range =
        |to: &GlobalTransform| Meters::from_engine(to.translation().distance(at.translation()));
    let parts = |root: Entity| {
        q_sections
            .iter()
            .filter(|(child_of, _)| child_of.parent() == root)
            .fold((0, 0), |(sections, turrets), (_, turret)| {
                (sections + 1, turrets + u32::from(turret))
            })
    };
    let hp = |health: Option<&Health>| {
        health.map_or("none".to_string(), |health| {
            format!("{:.0}/{:.0}", health.current, health.max)
        })
    };
    for (index, id) in job.raider_ids().iter().enumerate() {
        let Some((raider, _, raider_at, health)) =
            q_bodies.iter().find(|(_, worn, ..)| *worn == id)
        else {
            continue;
        };
        let (sections, turrets) = parts(raider);
        panel.label_tinted(
            raider_at.translation(),
            format!(
                "RAIDER {index}\n{sections} sections, {turrets} turret(s)\nhp {}\n{:.0} m",
                hp(health),
                range(raider_at).0
            ),
            palette::ALERT,
        );
    }
    if let Some((_, _, hauler_at, health)) =
        hauler_of(job).and_then(|hauler| q_bodies.iter().find(|(_, worn, ..)| *worn == hauler))
    {
        gizmos.line(at.translation(), hauler_at.translation(), palette::OK);
        panel.label_tinted(
            hauler_at.translation(),
            format!("HAULER\nhp {}\n{:.0} m", hp(health), range(hauler_at).0),
            palette::OK,
        );
    }
    if docked.is_some() {
        panel.hint("D", "undock");
        panel.hint("TAB", "Jobs: read the escort, Hand in when done");
    } else {
        panel.hint("Ctrl", "hold on a raider to lock it");
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS};

    use super::*;

    /// Seconds the world gets to start, stream, dock, accept and spawn.
    const SETUP_SECS: f32 = 800.0;
    /// Frames the undock and the jump get to settle.
    const SETTLE_FRAMES: u32 = 10;
    /// How far past the raiders' spread the shot stands.
    const SHOT_MARGIN: Meters = Meters(700.0);
    /// How far below the raiders' centre the bow aims, as a share of the
    /// standoff.
    const SHOT_DIP: f32 = 0.15;

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The frame the capture path writes for `band`.
    fn shot(band: BandType) -> String {
        format!("escort-job-sandbox-{}.png", band.label())
    }

    /// The staging: the poster's market names the band's civilization, the
    /// escort is held, its raider count lies in the band's row, and each
    /// raider fits the band's size budget.
    fn assert_the_staging(world: &mut World) {
        let band = world.resource::<Cli>().band;
        let civilization = world.resource::<BandCivilization>().0;
        let sandbox = world.resource::<Sandbox>();
        let (Some(chosen), Some(job)) = (sandbox.station.clone(), sandbox.job.clone()) else {
            panic!("escort_job_sandbox: an escort is chosen");
        };
        let poster = open_world_stage::body_by_id(world, &chosen.id)
            .expect("escort_job_sandbox: the poster stands");
        let named = world
            .get::<TraderMarket>(poster)
            .expect("escort_job_sandbox: the poster carries a market")
            .civilization;
        assert_eq!(
            named, civilization.id,
            "escort_job_sandbox: the poster's market names the band's civilization"
        );
        assert!(
            world.resource::<Jobs>().job(&job.id).is_some(),
            "escort_job_sandbox: the escort '{}' is held",
            job.id
        );
        let ((least, most), size) = raider_row(civilization.advancement);
        assert_eq!(
            ((least, most), size),
            raider_row(band.advancement()),
            "escort_job_sandbox: the civilization's advancement {:.3} lies in the '{}' row",
            civilization.advancement,
            band.label()
        );
        let posted = raiders_of(&job).expect("escort_job_sandbox: the held job is an escort");
        assert!(
            (least..=most).contains(&posted),
            "escort_job_sandbox: the board posted {posted} raider(s), outside the '{}' row \
             {least}..={most}",
            band.label()
        );
        let (_, raiders) =
            read_the_escort(world, &job).expect("escort_job_sandbox: the escort stands");
        assert_eq!(
            raiders.len() as u32,
            posted,
            "escort_job_sandbox: one raider stands per posted raider"
        );
        for raider in &raiders {
            assert!(
                raider.turrets >= 1,
                "escort_job_sandbox: the raider '{}' mounts a turret",
                raider.id
            );
            if let ShipSizeType::Budget { sections, turrets } = size {
                assert!(
                    raider.sections <= sections && raider.turrets <= turrets,
                    "escort_job_sandbox: the raider '{}' lays out {} sections and {} turret(s), \
                     over the '{}' budget of {sections} and {turrets}",
                    raider.id,
                    raider.sections,
                    raider.turrets,
                    band.label()
                );
            }
        }
        info!(
            "escort_job_sandbox: PASS band={} advancement={:.3} raiders={posted} in \
             {least}..={most}, sections={:?} turrets={:?} within {size:?}",
            band.label(),
            civilization.advancement,
            raiders
                .iter()
                .map(|raider| raider.sections)
                .collect::<Vec<_>>(),
            raiders
                .iter()
                .map(|raider| raider.turrets)
                .collect::<Vec<_>>()
        );
    }

    /// Jump the free player off the raiders, on the poster's side, bow on
    /// their centre, far enough to hold all of them in view.
    fn face_the_raiders(world: &mut World) {
        let job = world
            .resource::<Sandbox>()
            .job
            .clone()
            .expect("escort_job_sandbox: the escort is held");
        let (hauler, raiders) =
            read_the_escort(world, &job).expect("escort_job_sandbox: the escort stands");
        let place = |entity: Entity| {
            world
                .get::<Position>(entity)
                .expect("escort_job_sandbox: a hull has a physics position")
                .0
        };
        let spots: Vec<Vec3> = raiders.iter().map(|raider| place(raider.entity)).collect();
        let centre = spots.iter().copied().sum::<Vec3>() / spots.len() as f32;
        let spread = spots
            .iter()
            .map(|spot| spot.distance(centre))
            .fold(0.0, f32::max);
        let back = (place(hauler.entity) - centre).normalize_or(Vec3::Z);
        let standoff = spread * 1.5 + SHOT_MARGIN.to_engine();
        let stand = centre + back * standoff;
        // The bow aims below the raiders' centre, so the chase camera sees
        // them above the player's own hull instead of behind it.
        let up = Vec3::Y.reject_from(back).normalize_or(Vec3::Z);
        let aim = centre - up * standoff * SHOT_DIP;
        let player = open_world_stage::the_player(world).expect("escort_job_sandbox: one player");
        open_world_stage::jump_facing(
            world,
            player,
            Meters3::from_engine(stand),
            (aim - stand).normalize(),
        );
        info!(
            "escort_job_sandbox: the shot stands {:.0} m off the raiders' centre",
            Meters::from_engine(standoff).0
        );
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script(fixture: Cli) -> AutopilotPlugin<GameStates> {
        let shot = shot(fixture.band);
        let written = shot.clone();
        AutopilotPlugin::<GameStates>::new()
            .step("escort job sandbox: start, dock, accept and spawn the escort")
            .until(when(|world| {
                world
                    .get_resource::<Sandbox>()
                    .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
            }))
            .deadline(SETUP_SECS)
            .add()
            .step("escort job sandbox: check the staging")
            .on_enter(assert_the_staging)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("escort job sandbox: undock")
            .on_enter(|world: &mut World| {
                let player =
                    open_world_stage::the_player(world).expect("escort_job_sandbox: one player");
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(when(|world| {
                let mut players =
                    world.try_query_filtered::<Has<DockedShip>, With<PlayerSpaceshipMarker>>();
                players
                    .as_mut()
                    .is_some_and(|players| players.iter(world).any(|docked| !docked))
            }))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("escort job sandbox: face the raiders")
            .on_enter(face_the_raiders)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("escort job sandbox: shoot the raiders")
            .on_enter(move |world: &mut World| shoot(world, &shot))
            .until(shot_written(&written))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }
}
