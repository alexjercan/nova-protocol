//! The TAB interface's Jobs pane: the jobs the player holds, always, and
//! the docked station's board.
//!
//! The pane is one row: a column of panels and an inspector beside it.
//! Docked at a living station that offers Jobs, the column starts with the
//! station's board ("Jobs posted here"): one line per offer the player
//! neither holds nor completed and whose window has not passed, as kind
//! icon | title | reward | target sector and distance | time-left meter |
//! Accept. Every offer asks something of the hull ([`plan_job_hull_fit`]):
//! the starting station's board ([`StarterBoardMarker`]) leaves out what the
//! hull cannot do, every other board lists it with a greyed Accept and the
//! reason in a tooltip, as it does for an offer whose target is gone. Under
//! it, docked or not, "Your jobs" lists every held job as kind icon | title
//! | `have / need` meter | hand-in | Hand in: HAND IN HERE at the job's
//! target, else `hand in at <station>` docked and the station with its
//! distance undocked; a refused Hand in is greyed with the reason in a
//! tooltip. Over the held rows, one row reads the start role's goal: its
//! title, its next unmet line ([`open_objectives`]) and a `have / need`
//! meter, or the play time the goal was met; a role with no goal has no row.
//! Clicking a job row selects it and the inspector reads its task, its
//! poster, its target, its hand-in station, its reward, its time left or
//! progress and a Show on Map button, which opens the Map's chart view on
//! the job's mark ([`MapRuntime::chart_show`]).
//!
//! A click on Accept or Hand in raises a [`JobAcceptCommand`] or a
//! [`JobHandInCommand`]; [`apply_job_commands`] plans it with
//! [`plan_job_accept`] or [`plan_job_hand_in`], moves the goods and the
//! credits, moves the job in [`Jobs`] and raises [`JobAccepted`] so the open
//! world puts the job's bodies in place. The result shows on the note line
//! under the panels.
//!
//! Touch this module when changing what the lists show, what gates an
//! offer, or what a hand-in moves.

use bevy::{
    ecs::system::SystemParam, picking::hover::Hovered, prelude::*, ui::InteractionDisabled,
    ui_widgets::Activate,
};
use nova_events::prelude::{CreditChangeCauseType, GeneratedBodyId, Meters, Meters3, SectorCoord};
use nova_gameplay::prelude::*;
use nova_ship::prelude::DockingSectionMarker;
use nova_ui::{prelude::*, theme::UiColor, widget::ThemedBorder};
use nova_world_base::prelude::record_merit;

use crate::{
    icons::{icon_node, ChartIconType, InterfaceIcons, JobIconType},
    inventory::cr_text,
    map::{job_cell, MapChartMarkType, MapRuntime},
    pane::{
        compact_button, control_row, divider, panel_paint, play_menu_select, themed_label,
        view_split, InterfacePaneType,
    },
    ship::StationSite,
};

/// Glob-import surface: `use nova_interface::jobs::prelude::*`.
pub mod prelude {
    pub use super::{
        JobAcceptCommand, JobHandInCommand, JobsRuntime, JOBS_BOARD, JOBS_HELD, JOBS_MILESTONE,
        JOBS_PANE, JOB_ACCEPT_PREFIX, JOB_HAND_IN_PREFIX, JOB_SHOW_ON_MAP,
    };
}

/// The `Name` of the pane's row of panels.
pub const JOBS_PANE: &str = "JobsPane";
/// The `Name` of the board panel, present only while docked at a station
/// that posts jobs.
pub const JOBS_BOARD: &str = "JobsBoard";
/// The `Name` of the held jobs panel.
pub const JOBS_HELD: &str = "JobsHeld";
/// The `Name` of the held panel's milestone row, present while the start
/// role has a goal.
pub const JOBS_MILESTONE: &str = "JobsMilestone";
/// The `Name` prefix of a board row's Accept button: `JobAccept-<row>`.
pub const JOB_ACCEPT_PREFIX: &str = "JobAccept-";
/// The `Name` prefix of a held row's Hand in button: `JobHandIn-<row>`.
pub const JOB_HAND_IN_PREFIX: &str = "JobHandIn-";
/// The `Name` of the inspector's Show on Map button.
pub const JOB_SHOW_ON_MAP: &str = "JobShowOnMap";

/// Seconds an action result stays on the note line.
const NOTE_SECS: f32 = 2.5;

/// A request to accept the posted job `id` off the docked station's board,
/// raised by a board row's Accept button.
#[derive(Message, Clone, Debug)]
pub struct JobAcceptCommand {
    /// The posted job.
    pub id: JobId,
}

/// A request to hand the held job `id` in at the docked station, raised by
/// a held row's Hand in button.
#[derive(Message, Clone, Debug)]
pub struct JobHandInCommand {
    /// The held job.
    pub id: JobId,
}

/// What the Jobs pane shows beyond the records: the selected row and the
/// note line. Kept while the interface is closed, like the pane itself. A
/// selection no row shows any more is cleared by [`update_jobs_pane`].
#[derive(Resource, Clone, PartialEq, Debug, Default)]
pub struct JobsRuntime {
    /// The row the inspector reads.
    pub(crate) selected: Option<JobId>,
    /// The last action result and its remaining seconds on screen.
    pub(crate) note: Option<(String, f32)>,
}

impl JobsRuntime {
    /// The last action result on the note line, while it shows.
    pub fn note(&self) -> Option<&str> {
        self.note.as_ref().map(|(note, _)| note.as_str())
    }

    /// Show `note` on the note line for [`NOTE_SECS`].
    pub(crate) fn say(&mut self, note: String) {
        self.note = Some((note, NOTE_SECS));
    }
}

/// The pane's row of panels, rebuilt by [`update_jobs_pane`].
#[derive(Component)]
pub(crate) struct JobsPaneMarker;

/// The note line under the panels.
#[derive(Component)]
pub(crate) struct JobsNoteMarker;

/// A row of either list: a click selects its job.
#[derive(Component, Clone, Debug)]
struct JobRow(JobId);

/// Which list a button acts for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum JobListType {
    /// The docked station's board: Accept.
    Posted,
    /// The jobs the player holds: Hand in.
    Held,
}

/// A row's action button: Accept on the board, Hand in on a held job.
#[derive(Component, Clone, Debug)]
struct JobActionButton {
    list: JobListType,
    id: JobId,
}

/// The inspector's Show on Map button: the mark it opens the chart on.
#[derive(Component, Clone, Debug)]
struct JobShowOnMap(MapChartMarkType);

/// Build the Jobs pane body: the row of panels and the note line.
pub(crate) fn jobs_body(body: &mut ChildSpawnerCommands) {
    body.spawn((JobsPaneMarker, Name::new(JOBS_PANE), view_split()));
    body.spawn((
        JobsNoteMarker,
        themed_label("", 12.0, UiColor::Accent),
        Node {
            flex_shrink: 0.0,
            ..default()
        },
    ));
}

/// The bodies a located job can name, read where they stand.
type JobTargets<'w, 's> = Query<
    'w,
    's,
    (
        &'static GeneratedBodyId,
        Option<&'static ShipInventory>,
        Has<NeutralizedMarker>,
    ),
>;

/// Whether the body a recovery or a neutralize names still qualifies: the
/// wreck carries the whole stack, or the hull stands un-neutralized. Read
/// for those two kinds only; every other kind reads `true`.
fn target_standing(job: &Job, q_targets: &JobTargets) -> bool {
    let standing = |body: &GeneratedBodyId| q_targets.iter().find(|(id, _, _)| *id == body);
    match &job.kind {
        JobType::Recover {
            wreck, item, count, ..
        } => standing(wreck)
            .and_then(|(_, hold, _)| hold)
            .is_some_and(|hold| hold.count(item) >= *count),
        JobType::Neutralize { hull, .. } => {
            standing(hull).is_some_and(|(_, _, neutralized)| !neutralized)
        }
        JobType::Procure { .. } | JobType::Escort { .. } | JobType::Distress { .. } => true,
    }
}

/// The player's hull as the board's gate reads it: its live sections, its
/// hold and where it stands.
#[derive(SystemParam)]
pub(crate) struct PlayerHull<'w, 's> {
    player: Query<
        'w,
        's,
        (
            Entity,
            &'static GlobalTransform,
            Option<&'static ShipInventory>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    sections: Query<
        'w,
        's,
        (
            &'static ChildOf,
            Has<DockingSectionMarker>,
            Has<TurretSectionMarker>,
            Has<TorpedoSectionMarker>,
            Has<RailgunSectionMarker>,
        ),
        (With<SectionMarker>, Without<SectionInactiveMarker>),
    >,
}

impl PlayerHull<'_, '_> {
    /// What the hull can do: its hold's capacity and whether a live docking
    /// section and a live weapon section stand on it. A scene with no player
    /// ship fits nothing.
    pub(crate) fn fit(&self) -> JobHullFit {
        let Ok((player, _, hold)) = self.player.single() else {
            return JobHullFit::default();
        };
        let mut fit = JobHullFit {
            hold_capacity_g: hold.map_or(0, ShipInventory::capacity_g),
            has_clamp: false,
            has_weapon: false,
        };
        for (&ChildOf(root), clamp, turret, bay, railgun) in &self.sections {
            if root != player {
                continue;
            }
            fit.has_clamp |= clamp;
            fit.has_weapon |= turret || bay || railgun;
        }
        fit
    }

    /// The cell the player ship stands in, on a streamed world of cell
    /// `edge`.
    pub(crate) fn cell(&self, edge: Option<Meters>) -> Option<SectorCoord> {
        let (_, transform, _) = self.player.single().ok()?;
        Some(SectorCoord::containing(
            Meters3::from_engine(transform.translation()),
            edge?,
        ))
    }
}

/// The docked station the lists and the handler plan against: the player,
/// the station entity and its id, while it is a living station offering
/// Jobs.
fn jobs_partner(
    site: &StationSite,
    q_ids: &Query<&GeneratedBodyId>,
) -> Option<(Entity, Entity, GeneratedBodyId)> {
    let resolved = site.resolve(StationServiceType::Jobs)?;
    let (station, true, _) = resolved.partner? else {
        return None;
    };
    let id = q_ids.get(station).ok()?.clone();
    Some((resolved.player, station, id))
}

/// A found station's name, or its id while the chart has no name for it.
fn station_name(chart: &Chart, id: &GeneratedBodyId) -> String {
    chart
        .station(id)
        .and_then(|station| station.summary.as_ref())
        .map_or_else(|| id.to_string(), |summary| summary.name.clone())
}

/// The distance from the player's cell to `to`, cell centre to cell centre:
/// `this sector`, or `48 km`. `None` while the player's cell is unknown.
fn cell_distance_text(from: Option<SectorCoord>, to: SectorCoord, edge: Meters) -> Option<String> {
    let from = from?;
    if from == to {
        return Some("this sector".to_string());
    }
    let dx = f64::from(to.x - from.x);
    let dy = f64::from(to.y - from.y);
    let dz = f64::from(to.z - from.z);
    let km = (dx * dx + dy * dy + dz * dz).sqrt() * f64::from(edge.get()) / 1_000.0;
    Some(format!("{km:.0} km"))
}

/// A cell and the distance to it: `(1, 0, -2), 48 km`, or the cell alone
/// while the distance is unknown.
fn place_text(cell: SectorCoord, distance: Option<String>) -> String {
    match distance {
        Some(distance) => format!("{cell}, {distance}"),
        None => cell.to_string(),
    }
}

/// What a job is about, for the inspector: the poster for a procurement,
/// else the wreck, the hauler, the caller or the hull.
fn target_name(job: &Job, chart: &Chart) -> String {
    match &job.kind {
        JobType::Procure { .. } => station_name(chart, &job.target),
        JobType::Recover { wreck_name, .. } => wreck_name.clone(),
        JobType::Escort { .. } => "the hauler".to_string(),
        JobType::Distress { .. } => "the caller".to_string(),
        JobType::Neutralize { hull_name, .. } => hull_name.clone(),
    }
}

/// Why the hull cannot do a job, for a greyed Accept.
fn hull_refusal_text(refusal: &JobHullRefusalType) -> String {
    match refusal {
        JobHullRefusalType::HoldTooSmall { need_g, have_g } => format!(
            "accept: {} does not fit the {} hold",
            kg_text(*need_g),
            kg_text(u64::from(*have_g))
        ),
        JobHullRefusalType::NoClamp => "accept: the hull has no docking section".to_string(),
        JobHullRefusalType::NoWeapon => "accept: the hull has no weapon".to_string(),
    }
}

/// Why an accept is refused.
fn accept_refusal_text(refusal: &JobAcceptRefusalType) -> String {
    match refusal {
        JobAcceptRefusalType::NotDocked => "accept: dock at a station that posts jobs".to_string(),
        JobAcceptRefusalType::NotPostedHere => "accept: this station did not post it".to_string(),
        JobAcceptRefusalType::AlreadyHeld => "accept: you hold it already".to_string(),
        JobAcceptRefusalType::AlreadyDone => "accept: you completed it already".to_string(),
        JobAcceptRefusalType::Expired => "accept: this offer expired".to_string(),
        JobAcceptRefusalType::TargetGone => "accept: its target is gone".to_string(),
    }
}

/// Why a hand-in of the job titled `title` is refused; the list and the
/// handler share it.
fn hand_in_refusal_text(
    job: &Job,
    title: &str,
    refusal: &JobHandInRefusalType,
    chart: &Chart,
) -> String {
    match refusal {
        JobHandInRefusalType::NotDocked => {
            format!("dock at {} to hand in", station_name(chart, &job.target))
        }
        JobHandInRefusalType::WrongStation { needed } => {
            format!("hand in at {}", station_name(chart, needed))
        }
        JobHandInRefusalType::ShortItems { item, need, have } => {
            format!("the hold carries {have} of {need} {item}")
        }
        JobHandInRefusalType::NotDone => format!("{title}: not done yet"),
        JobHandInRefusalType::StationCannotPay { need, have } => format!(
            "hand in: {title} pays {}, the station holds {}",
            cr_text(*need),
            cr_text(*have)
        ),
        JobHandInRefusalType::CreditOverflow => {
            format!("hand in: the ship's balance after {title} does not fit")
        }
    }
}

/// What a board does with an offer the hull gate judged `fit` and the
/// accept rule judged `accept`: `None` leaves it off a starter board,
/// `Some(None)` lists it with Accept, `Some(Some(why))` lists it with a
/// greyed Accept and `why` in its tooltip. The hull's reason comes first:
/// it is the one the player can act on.
fn board_listing(
    starter: bool,
    fit: Result<(), JobHullRefusalType>,
    accept: Result<(), JobAcceptRefusalType>,
) -> Option<Option<String>> {
    match (starter, fit, accept) {
        (true, Err(_), _) => None,
        (false, Err(refusal), _) => Some(Some(hull_refusal_text(&refusal))),
        (_, Ok(()), Err(refusal)) => Some(Some(accept_refusal_text(&refusal))),
        (_, Ok(()), Ok(())) => Some(None),
    }
}

/// One board row as the pane shows it.
#[derive(Clone, Debug, PartialEq)]
struct PostedRow {
    id: JobId,
    icon: JobIconType,
    title: String,
    reward: String,
    target: String,
    /// The offer's window left, as a share of the whole.
    time_left: f32,
    /// Why Accept is greyed, if it is.
    refusal: Option<String>,
}

/// Where a held job hands in, as its row says it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum HandInType {
    /// The docked station is the target.
    Here,
    /// Docked elsewhere: the target's name.
    At(String),
    /// Undocked: the target's name and its distance.
    Away(String),
}

/// One held row as the pane shows it.
#[derive(Clone, Debug, PartialEq)]
struct HeldRow {
    id: JobId,
    icon: JobIconType,
    title: String,
    have: u32,
    need: u32,
    hand_in: HandInType,
    /// Why Hand in is greyed, if it is.
    refusal: Option<String>,
}

/// The docked station's board as the pane shows it.
#[derive(Clone, Debug, PartialEq)]
struct BoardView {
    station: String,
    rows: Vec<PostedRow>,
}

/// The selected job as the inspector reads it.
#[derive(Clone, Debug, PartialEq)]
struct InspectorView {
    icon: JobIconType,
    title: String,
    kind: &'static str,
    held: bool,
    done: bool,
    task: String,
    poster: String,
    target: String,
    hand_in: String,
    reward: String,
    /// A posted offer's window left: the share and the minutes.
    time_left: Option<(f32, String)>,
    /// A held job's `have / need`.
    progress: Option<(u32, u32)>,
    /// A held job's hand-in refusal.
    refusal: Option<String>,
    /// The mark Show on Map opens the chart on.
    show: MapChartMarkType,
}

/// Everything the pane draws, so it is rebuilt only on a difference.
#[derive(Clone, Debug, PartialEq, Default)]
struct JobsView {
    board: Option<BoardView>,
    held: Vec<HeldRow>,
    /// The role goal's heading.
    goal_title: String,
    /// The role goal's next unmet line, as [`open_objectives`] lists it.
    milestone: Option<GoalLine>,
    /// The play time the role goal was met at.
    goal_met_at: Option<f64>,
    docked: bool,
    inspector: Option<InspectorView>,
}

/// Rebuild the pane whenever what it shows changes: the board, the jobs,
/// the hold, the hull, the dock, the clock, the role goal or the selection.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the catalog, the table, the chart, the jobs, the postings, the clock, the edge, the goal, the site, the hull, the boards, the targets, both balances and the pane"
)]
#[expect(
    clippy::type_complexity,
    reason = "the records and the station queries each grouped in one tuple"
)]
fn update_jobs_pane(
    mut commands: Commands,
    mut runtime: ResMut<JobsRuntime>,
    icons: Res<InterfaceIcons>,
    items: Option<Res<GameItems>>,
    table: Option<Res<GamePriceTable>>,
    (chart, jobs, postings, clock, edge, progress, record): (
        Res<Chart>,
        Res<Jobs>,
        Res<JobPostings>,
        Res<WorldClock>,
        Option<Res<ChartSectorEdge>>,
        Res<GoalProgress>,
        Res<GoalRecord>,
    ),
    site: StationSite,
    hull: PlayerHull,
    (q_ids, q_targets, q_boards, q_credits, q_holds): (
        Query<&GeneratedBodyId>,
        JobTargets,
        Query<(&JobBoard, Has<StarterBoardMarker>)>,
        Query<&ShipCredits>,
        Query<&ShipInventory>,
    ),
    q_pane: Query<(Entity, Ref<JobsPaneMarker>)>,
    mut built: Local<Option<JobsView>>,
) {
    let Ok((pane, marker)) = q_pane.single() else {
        *built = None;
        return;
    };
    let (Some(items), Some(table)) = (items, table) else {
        return;
    };
    let edge = edge.map(|edge| edge.0);
    let player_cell = hull.cell(edge);
    let fit = hull.fit();
    let partner = jobs_partner(&site, &q_ids);
    let place = |cell: SectorCoord| {
        place_text(
            cell,
            edge.and_then(|edge| cell_distance_text(player_cell, cell, edge)),
        )
    };
    let now = clock.play_seconds;
    let rule = table.0.jobs;

    let board = partner.as_ref().and_then(|(_, station, station_id)| {
        let (board, starter) = q_boards.get(*station).ok()?;
        let rows = board
            .0
            .iter()
            .filter(|job| !jobs.holds(&job.id) && !jobs.is_completed(&job.id))
            .filter_map(|job| {
                let remaining = postings.slot(&job.id)?.remaining(rule, now);
                if remaining <= 0.0 {
                    return None;
                }
                let refusal = board_listing(
                    starter,
                    plan_job_hull_fit(job, &items, fit),
                    plan_job_accept(
                        job,
                        Some(station_id),
                        &jobs,
                        false,
                        target_standing(job, &q_targets),
                    ),
                )?;
                Some(PostedRow {
                    id: job.id.clone(),
                    icon: JobIconType::of(&job.kind),
                    title: job.title(&items),
                    reward: cr_text(job.reward_cr),
                    target: place(job_cell(job)),
                    time_left: (remaining / f64::from(rule.offer_seconds)) as f32,
                    refusal,
                })
            })
            .collect();
        Some(BoardView {
            station: station_name(&chart, station_id),
            rows,
        })
    });

    let player = partner
        .as_ref()
        .map(|(player, _, _)| *player)
        .or_else(|| hull.player.single().ok().map(|(player, _, _)| player));
    let hold = player.and_then(|player| q_holds.get(player).ok());
    let own_cr = player.and_then(|player| q_credits.get(player).ok().map(|credits| credits.0));
    let hand_in_partner = partner.as_ref().and_then(|(_, station, station_id)| {
        Some(JobHandInPartner {
            id: station_id.clone(),
            credits_cr: q_credits.get(*station).ok()?.0,
        })
    });
    let hand_in_refusal = |job: &Job, title: &str| {
        let (Some(hold), Some(own_cr)) = (hold, own_cr) else {
            return Some(hand_in_refusal_text(
                job,
                title,
                &JobHandInRefusalType::NotDocked,
                &chart,
            ));
        };
        plan_job_hand_in(
            job,
            hand_in_partner.as_ref(),
            hold,
            own_cr,
            jobs.is_done(&job.id),
        )
        .err()
        .map(|refusal| hand_in_refusal_text(job, title, &refusal, &chart))
    };
    let held: Vec<HeldRow> = jobs
        .held()
        .iter()
        .map(|job| {
            let title = job.title(&items);
            let target = station_name(&chart, &job.target);
            let hand_in = match &partner {
                Some((_, _, station_id)) if *station_id == job.target => HandInType::Here,
                Some(_) => HandInType::At(target),
                None => HandInType::Away(
                    match edge
                        .and_then(|edge| cell_distance_text(player_cell, job.target.origin, edge))
                    {
                        Some(distance) => format!("{target}, {distance}"),
                        None => target,
                    },
                ),
            };
            let (have, need) = job.progress(hold, jobs.is_done(&job.id));
            HeldRow {
                id: job.id.clone(),
                icon: JobIconType::of(&job.kind),
                title: title.clone(),
                have,
                need,
                hand_in,
                refusal: hand_in_refusal(job, &title),
            }
        })
        .collect();

    let selected = runtime.selected.as_ref().and_then(|id| {
        jobs.job(id).map(|job| (job, true)).or_else(|| {
            let (_, station, _) = partner.as_ref()?;
            let (board, _) = q_boards.get(*station).ok()?;
            board
                .job(id)
                .filter(|job| !jobs.is_completed(&job.id))
                .map(|job| (job, false))
        })
    });
    if selected.is_none() && runtime.selected.is_some() {
        runtime.selected = None;
    }
    let inspector = selected.map(|(job, held)| {
        let title = job.title(&items);
        let done = jobs.is_done(&job.id);
        let time_left = postings
            .slot(&job.id)
            .filter(|_| !held)
            .map(|slot| slot.remaining(rule, now))
            .map(|remaining| {
                (
                    (remaining / f64::from(rule.offer_seconds)) as f32,
                    format!("{} min", (remaining.max(0.0) / 60.0).ceil()),
                )
            });
        let show = if held {
            MapChartMarkType::Job(job.id.clone())
        } else if chart.station(job.posted_by()).is_some() {
            MapChartMarkType::Station(job.posted_by().clone())
        } else {
            MapChartMarkType::Cell(job.posted_by().origin)
        };
        InspectorView {
            icon: JobIconType::of(&job.kind),
            title: title.clone(),
            kind: job.kind.label(),
            held,
            done,
            task: job.task(),
            poster: station_name(&chart, job.posted_by()),
            target: format!("{}, {}", target_name(job, &chart), place(job_cell(job))),
            hand_in: station_name(&chart, &job.target),
            reward: cr_text(job.reward_cr),
            time_left,
            progress: held.then(|| job.progress(hold, done)),
            refusal: held.then(|| hand_in_refusal(job, &title)).flatten(),
            show,
        }
    });

    let milestone = open_objectives(&chart, &jobs, Some(&items), hold, &progress, &record)
        .into_iter()
        .find_map(|objective| match objective {
            OpenObjectiveType::Milestone(line) => Some(line),
            OpenObjectiveType::Job { .. } => None,
        });

    let view = JobsView {
        board,
        held,
        goal_title: progress.title.clone(),
        milestone,
        goal_met_at: record.met_at,
        docked: partner.is_some(),
        inspector,
    };
    if !marker.is_added() && built.as_ref() == Some(&view) {
        return;
    }
    debug!(
        "nova_interface: the Jobs pane rebuilds: {} posted, {} held, docked {}",
        view.board.as_ref().map_or(0, |board| board.rows.len()),
        view.held.len(),
        view.docked
    );
    commands.entity(pane).despawn_children();
    commands.entity(pane).with_children(|split| {
        split.spawn(column()).with_children(|col| {
            if let Some(board) = &view.board {
                board_panel(col, &icons, board, runtime.selected.as_ref());
            }
            held_panel(col, &icons, &view, runtime.selected.as_ref());
        });
        split
            .spawn((
                Node {
                    width: percent(26),
                    min_width: px(SIDE_PANEL_MIN_PX),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    padding: UiRect::all(px(12)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(4)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                panel_paint(),
            ))
            .with_children(|side| inspector_panel(side, &icons, view.inspector.as_ref()));
    });
    *built = Some(view);
}

/// Least width of the inspector, in logical px, as the viewer panes' side
/// panel.
const SIDE_PANEL_MIN_PX: f32 = 300.0;
/// Width of the reward column.
const REWARD_PX: f32 = 90.0;
/// Width of the target and the hand-in columns.
const PLACE_PX: f32 = 190.0;
/// Width of the time-left column.
const TIME_PX: f32 = 120.0;
/// Width of the progress column.
const PROGRESS_PX: f32 = 200.0;
/// Width of a row's action button column.
const ACTION_PX: f32 = 78.0;
/// Width of a row's icon column.
const ICON_PX: f32 = 28.0;
/// Width of a meter inside its column.
const METER_PX: f32 = 110.0;
/// Width of an inspector's label column.
const LABEL_PX: f32 = 84.0;

/// The column of panels: it takes what the inspector leaves of the row.
fn column() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        min_height: px(0),
        flex_direction: FlexDirection::Column,
        row_gap: px(10),
        ..default()
    }
}

/// A panel of the column: an equal share of its height.
fn column_panel() -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            padding: UiRect::all(px(8)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Secondary, 0.35),
    )
}

/// A themed text that never wraps and takes no clicks, so a row's click
/// lands on the row.
fn text_nowrap(value: &str, size: f32, color: UiColor) -> impl Bundle {
    (
        themed_label(value, size, color),
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
        Node {
            overflow: Overflow::clip(),
            min_width: px(0),
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// A fixed-width column cell, so the columns line up down a table.
fn cell(width: f32, value: &str, size: f32, color: UiColor) -> impl Bundle {
    (
        Node {
            width: px(width),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
        children![text_nowrap(value, size, color)],
    )
}

/// A cell that takes the rest of the row.
fn grow_cell(value: &str, size: f32, color: UiColor) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            overflow: Overflow::clip(),
            ..default()
        },
        Pickable::IGNORE,
        children![text_nowrap(value, size, color)],
    )
}

/// A fixed-width column holding `inner`.
fn fixed(width: f32, inner: impl Bundle) -> impl Bundle {
    (
        Node {
            width: px(width),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        },
        Pickable::IGNORE,
        children![inner],
    )
}

/// A display meter at a fixed width.
fn bare_meter(width: f32, fraction: f32) -> impl Bundle {
    (
        Node {
            width: px(width),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
        children![slider_track(fraction.clamp(0.0, 1.0))],
    )
}

/// A `have / need` meter: a display meter with the counts beside it.
fn meter(have: u64, need: u64) -> impl Bundle {
    let fraction = if need == 0 {
        0.0
    } else {
        have as f32 / need as f32
    };
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
        children![
            bare_meter(METER_PX, fraction),
            text_nowrap(&format!("{have} / {need}"), 12.0, UiColor::Body),
        ],
    )
}

/// A table's header line: `(width, label)` per column, width 0 for the
/// column that grows.
fn table_head(panel: &mut ChildSpawnerCommands, columns: &[(f32, &str)]) {
    panel
        .spawn(Node {
            padding: UiRect::axes(px(13), px(2)),
            column_gap: px(12),
            flex_shrink: 0.0,
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|row| {
            for (width, label) in columns {
                if *width == 0.0 {
                    row.spawn(grow_cell(label, 11.0, UiColor::Label));
                } else {
                    row.spawn(cell(*width, label, 11.0, UiColor::Label));
                }
            }
        });
}

/// A clickable row of a list, tighter than a plain `list_row` so a board
/// fits beside its inspector, lit while it is the selection.
fn spawn_row<'a>(
    panel: &'a mut ChildSpawnerCommands,
    id: &JobId,
    name: String,
    selected: bool,
) -> EntityCommands<'a> {
    let mut row = panel.spawn((
        Name::new(name),
        JobRow(id.clone()),
        list_row(),
        Button,
        Hovered::default(),
    ));
    row.insert(Node {
        width: percent(100),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(12),
        flex_shrink: 0.0,
        padding: UiRect::axes(px(13), px(3)),
        border: UiRect::all(px(1)),
        ..default()
    });
    row.observe(on_job_row);
    if selected {
        row.insert(Selected);
    }
    row
}

/// A row's action button, greyed with `refusal` in its tooltip when given.
fn action_button(
    row: &mut ChildSpawnerCommands,
    name: String,
    label: &str,
    button: JobActionButton,
    refusal: Option<&String>,
) {
    // The button sits in a column as wide as the header's blank last column,
    // so the row's growing title cell matches the header's.
    row.spawn((
        Node {
            width: px(ACTION_PX),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::Center,
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|column| {
        let mut action = column.spawn((
            Name::new(name),
            button,
            compact_button(ButtonSpec::new(label).fit().primary()),
        ));
        action.observe(on_job_button);
        if let Some(why) = refusal {
            action.insert((InteractionDisabled, Tooltip(why.clone())));
        }
    });
}

/// A line that says a list is empty.
fn empty_line(panel: &mut ChildSpawnerCommands, text: &str) {
    panel.spawn((
        Node {
            padding: UiRect::axes(px(13), px(10)),
            ..default()
        },
        children![themed_label(text, 12.0, UiColor::Label)],
    ));
}

/// The docked station's board.
fn board_panel(
    col: &mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    board: &BoardView,
    selected: Option<&JobId>,
) {
    col.spawn((Name::new(JOBS_BOARD), column_panel()))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    column_gap: px(10),
                    ..control_row(JustifyContent::FlexStart)
                })
                .with_children(|head| {
                    head.spawn(themed_label(
                        &format!("Jobs posted here: {}", board.station),
                        15.0,
                        UiColor::Primary,
                    ));
                });
            table_head(
                panel,
                &[
                    (ICON_PX, ""),
                    (0.0, "Job"),
                    (REWARD_PX, "Reward"),
                    (PLACE_PX, "Target"),
                    (TIME_PX, "Time left"),
                    (ACTION_PX, ""),
                ],
            );
            for (index, row) in board.rows.iter().enumerate() {
                spawn_row(
                    panel,
                    &row.id,
                    format!("JobRowPosted-{index}"),
                    selected == Some(&row.id),
                )
                .with_children(|r| {
                    r.spawn(fixed(
                        ICON_PX,
                        icon_node(icons.job(row.icon), row.icon.color(), 18.0),
                    ));
                    r.spawn(grow_cell(&row.title, 13.0, UiColor::Primary));
                    r.spawn(cell(REWARD_PX, &row.reward, 13.0, UiColor::Accent));
                    r.spawn(cell(PLACE_PX, &row.target, 12.0, UiColor::Body));
                    r.spawn(bare_meter(TIME_PX, row.time_left));
                    action_button(
                        r,
                        format!("{JOB_ACCEPT_PREFIX}{index}"),
                        "Accept",
                        JobActionButton {
                            list: JobListType::Posted,
                            id: row.id.clone(),
                        },
                        row.refusal.as_ref(),
                    );
                });
            }
            if board.rows.is_empty() {
                empty_line(panel, "No open job on this board.");
            }
        });
}

/// The jobs the player holds.
fn held_panel(
    col: &mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    view: &JobsView,
    selected: Option<&JobId>,
) {
    col.spawn((Name::new(JOBS_HELD), column_panel()))
        .with_children(|panel| {
            panel.spawn(themed_label("Your jobs", 15.0, UiColor::Primary));
            let goal = match (&view.milestone, view.goal_met_at) {
                (Some(line), _) => Some((
                    format!("{}: {}", view.goal_title, line.label),
                    UiColor::Primary,
                    Some((line.have, line.need)),
                )),
                (None, Some(at)) => Some((
                    format!(
                        "{}: Goal reached at {}",
                        view.goal_title,
                        play_time_text(at)
                    ),
                    UiColor::Accent,
                    None,
                )),
                (None, None) => None,
            };
            if let Some((text, color, progress)) = goal {
                panel
                    .spawn((
                        Name::new(JOBS_MILESTONE),
                        Node {
                            padding: UiRect::axes(px(13), px(3)),
                            column_gap: px(12),
                            flex_shrink: 0.0,
                            ..control_row(JustifyContent::FlexStart)
                        },
                    ))
                    .with_children(|row| {
                        row.spawn(fixed(
                            ICON_PX,
                            icon_node(
                                icons.chart(ChartIconType::Milestone),
                                ChartIconType::Milestone.color(),
                                18.0,
                            ),
                        ));
                        row.spawn(grow_cell(&text, 13.0, color));
                        match progress {
                            Some((have, need)) => row.spawn(fixed(PROGRESS_PX, meter(have, need))),
                            None => row.spawn(fixed(PROGRESS_PX, ())),
                        };
                        row.spawn(fixed(PLACE_PX, ()));
                        row.spawn(fixed(ACTION_PX, ()));
                    });
            }
            table_head(
                panel,
                &[
                    (ICON_PX, ""),
                    (0.0, "Job"),
                    (PROGRESS_PX, "Progress"),
                    (PLACE_PX, if view.docked { "Hand-in" } else { "Hand-in at" }),
                    (ACTION_PX, ""),
                ],
            );
            for (index, row) in view.held.iter().enumerate() {
                spawn_row(
                    panel,
                    &row.id,
                    format!("JobRowHeld-{index}"),
                    selected == Some(&row.id),
                )
                .with_children(|r| {
                    r.spawn(fixed(
                        ICON_PX,
                        icon_node(icons.job(row.icon), row.icon.color(), 18.0),
                    ));
                    r.spawn(grow_cell(&row.title, 13.0, UiColor::Primary));
                    r.spawn(fixed(PROGRESS_PX, meter(row.have.into(), row.need.into())));
                    match &row.hand_in {
                        HandInType::Here => {
                            r.spawn(fixed(PLACE_PX, badge(BadgeKind::Green, "HAND IN HERE")));
                        }
                        HandInType::At(station) => {
                            r.spawn(cell(
                                PLACE_PX,
                                &format!("hand in at {station}"),
                                12.0,
                                UiColor::Body,
                            ));
                        }
                        HandInType::Away(place) => {
                            r.spawn(cell(PLACE_PX, place, 12.0, UiColor::Body));
                        }
                    }
                    action_button(
                        r,
                        format!("{JOB_HAND_IN_PREFIX}{index}"),
                        "Hand in",
                        JobActionButton {
                            list: JobListType::Held,
                            id: row.id.clone(),
                        },
                        row.refusal.as_ref(),
                    );
                });
            }
            if view.held.is_empty() {
                empty_line(panel, "No job held.");
            }
        });
}

/// A `label  value` line of the inspector.
fn kv(side: &mut ChildSpawnerCommands, label: &str, value: &str) {
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|row| {
        row.spawn(cell(LABEL_PX, label, 12.0, UiColor::Label));
        row.spawn(grow_cell(value, 13.0, UiColor::Primary));
    });
}

/// The inspector: the selected job, or only its heading.
fn inspector_panel(
    side: &mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    inspector: Option<&InspectorView>,
) {
    let Some(job) = inspector else {
        side.spawn(themed_label("JOB", 11.0, UiColor::Label));
        return;
    };
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|head| {
        head.spawn(icon_node(icons.job(job.icon), job.icon.color(), 24.0));
        head.spawn(themed_label(&job.title, 15.0, UiColor::Primary));
    });
    side.spawn(Node {
        column_gap: px(6),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|tags| {
        tags.spawn(badge(BadgeKind::Mute, job.kind));
        if job.held {
            tags.spawn(badge(BadgeKind::Blue, "HELD"));
        }
        if job.done {
            tags.spawn(badge(BadgeKind::Green, "DONE"));
        }
    });
    side.spawn(divider());
    side.spawn(themed_label("TASK", 11.0, UiColor::Label));
    side.spawn(themed_label(&job.task, 12.0, UiColor::Body));
    side.spawn(divider());
    kv(side, "Poster", &job.poster);
    kv(side, "Target", &job.target);
    kv(side, "Hand in", &job.hand_in);
    kv(side, "Reward", &job.reward);
    if let Some((fraction, minutes)) = &job.time_left {
        side.spawn(Node {
            column_gap: px(8),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|row| {
            row.spawn(cell(LABEL_PX, "Time left", 12.0, UiColor::Label));
            row.spawn(bare_meter(METER_PX, *fraction));
            row.spawn(text_nowrap(minutes, 12.0, UiColor::Body));
        });
    }
    if let Some((have, need)) = job.progress {
        side.spawn(Node {
            column_gap: px(8),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|row| {
            row.spawn(cell(LABEL_PX, "Progress", 12.0, UiColor::Label));
            row.spawn(meter(have.into(), need.into()));
        });
    }
    if let Some(refusal) = &job.refusal {
        side.spawn(themed_label(refusal, 12.0, UiColor::Accent));
    }
    side.spawn(divider());
    side.spawn(control_row(JustifyContent::FlexStart))
        .with_children(|row| {
            row.spawn((
                Name::new(JOB_SHOW_ON_MAP),
                JobShowOnMap(job.show.clone()),
                compact_button(ButtonSpec::new("Show on Map").fit()),
            ))
            .observe(on_show_on_map);
        });
}

/// Select a row's job when it is activated, with one click if the
/// selection changes.
fn on_job_row(
    activate: On<Activate>,
    q_row: Query<&JobRow>,
    mut runtime: ResMut<JobsRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok(JobRow(id)) = q_row.get(activate.entity) else {
        return;
    };
    if runtime.selected.as_ref() != Some(id) {
        runtime.selected = Some(id.clone());
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Raise the list's command for the button's job when it is activated.
fn on_job_button(
    activate: On<Activate>,
    q_button: Query<&JobActionButton>,
    mut accepts: MessageWriter<JobAcceptCommand>,
    mut hand_ins: MessageWriter<JobHandInCommand>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok(button) = q_button.get(activate.entity) else {
        return;
    };
    match button.list {
        JobListType::Posted => {
            accepts.write(JobAcceptCommand {
                id: button.id.clone(),
            });
        }
        JobListType::Held => {
            hand_ins.write(JobHandInCommand {
                id: button.id.clone(),
            });
        }
    }
    play_menu_select(&mut commands, bank.as_deref());
}

/// Open the Map pane's chart view on the button's mark when it is
/// activated: the map takes the mark as it activates
/// ([`MapRuntime::chart_show`]).
fn on_show_on_map(
    activate: On<Activate>,
    q_button: Query<&JobShowOnMap>,
    mut map: ResMut<MapRuntime>,
    mut pane: ResMut<InterfacePaneType>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok(JobShowOnMap(mark)) = q_button.get(activate.entity) else {
        return;
    };
    map.chart_show = Some(mark.clone());
    map.chart_requested = true;
    *pane = InterfacePaneType::Map;
    play_menu_select(&mut commands, bank.as_deref());
}

/// Count the note down on the real clock and write it to the note line.
fn update_jobs_note(
    time: Res<Time<Real>>,
    mut runtime: ResMut<JobsRuntime>,
    mut q_note: Query<&mut Text, With<JobsNoteMarker>>,
) {
    if let Some((_, remaining)) = runtime.note.as_mut() {
        *remaining -= time.delta_secs().min(0.25);
        if *remaining <= 0.0 {
            runtime.note = None;
        }
    }
    let note = runtime.note().unwrap_or_default();
    for mut text in &mut q_note {
        if text.0 != note {
            text.0 = note.to_string();
        }
    }
}

/// Apply [`JobAcceptCommand`]s by the [`plan_job_accept`] rule and
/// [`JobHandInCommand`]s by the [`plan_job_hand_in`] rule: an accept holds
/// the job and raises [`JobAccepted`]; a hand-in takes a procurement's goods
/// out of the hold, pays the reward with `JobReward` credit changes on both
/// sides and completes the job. A refusal changes nothing and shows its
/// reason on the note line. An accept of a job the docked station's board
/// no longer posts is refused as expired: the slot renewed between the click
/// and this frame.
///
/// # Panics
///
/// On a hand-in of a job the player does not hold: the list only shows a
/// button for a job it lists.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading both commands, the board, the jobs, the postings, the clock, the table, the site, the targets, the hold, both balances and the stances"
)]
#[expect(
    clippy::type_complexity,
    reason = "both balances and report queues in one query"
)]
fn apply_job_commands(
    mut commands: Commands,
    mut accepts: MessageReader<JobAcceptCommand>,
    mut hand_ins: MessageReader<JobHandInCommand>,
    mut runtime: ResMut<JobsRuntime>,
    items: Option<Res<GameItems>>,
    table: Option<Res<GamePriceTable>>,
    chart: Res<Chart>,
    mut jobs: ResMut<Jobs>,
    postings: Res<JobPostings>,
    clock: Res<WorldClock>,
    site: StationSite,
    q_ids: Query<&GeneratedBodyId>,
    q_boards: Query<&JobBoard>,
    mut q_holds: ParamSet<(JobTargets, Query<&mut ShipInventory>)>,
    mut q_credits: ParamSet<(
        Query<&ShipCredits>,
        Query<(&mut ShipCredits, &mut ShipInventoryReports)>,
    )>,
    // The civilization a hand-in merits: the station's, read with the
    // player's table of stances.
    (q_markets, mut stances): (Query<&TraderMarket>, ResMut<CivilizationStances>),
) {
    if accepts.is_empty() && hand_ins.is_empty() {
        return;
    }
    let (Some(items), Some(table)) = (items, table) else {
        accepts.clear();
        hand_ins.clear();
        return;
    };
    let partner = jobs_partner(&site, &q_ids);
    for command in accepts.read() {
        let Some((_, station, station_id)) = &partner else {
            runtime.say(accept_refusal_text(&JobAcceptRefusalType::NotDocked));
            continue;
        };
        let Some(job) = q_boards
            .get(*station)
            .ok()
            .and_then(|board| board.job(&command.id))
        else {
            info!(
                "nova_interface: the accept of '{}' finds it off the board: expired",
                command.id
            );
            runtime.say(accept_refusal_text(&JobAcceptRefusalType::Expired));
            continue;
        };
        let expired = postings
            .slot(&job.id)
            .is_none_or(|slot| slot.is_expired(table.0.jobs, clock.play_seconds));
        match plan_job_accept(
            job,
            Some(station_id),
            &jobs,
            expired,
            target_standing(job, &q_holds.p0()),
        ) {
            Ok(()) => {
                let title = job.title(&items);
                jobs.accept(job.clone());
                commands.trigger(JobAccepted { job: job.clone() });
                info!("nova_interface: accepted the job '{}' ({title})", job.id);
                runtime.say(format!("accepted: {title}"));
            }
            Err(refusal) => runtime.say(accept_refusal_text(&refusal)),
        }
    }
    for command in hand_ins.read() {
        let job = jobs
            .job(&command.id)
            .unwrap_or_else(|| panic!("hand in: the job '{}' is not held", command.id))
            .clone();
        let title = job.title(&items);
        let hand_in_partner = partner.as_ref().and_then(|(_, station, station_id)| {
            Some(JobHandInPartner {
                id: station_id.clone(),
                credits_cr: q_credits.p0().get(*station).ok()?.0,
            })
        });
        let Some((player, station, _)) = &partner else {
            runtime.say(hand_in_refusal_text(
                &job,
                &title,
                &JobHandInRefusalType::NotDocked,
                &chart,
            ));
            continue;
        };
        let own_cr = q_credits
            .p0()
            .get(*player)
            .expect("the player ship carries ShipCredits")
            .0;
        let mut holds = q_holds.p1();
        let Ok(hold) = holds.get(*player) else {
            panic!("hand in: the player ship {player:?} has no hold");
        };
        let done = jobs.is_done(&job.id);
        let plan = match plan_job_hand_in(&job, hand_in_partner.as_ref(), hold, own_cr, done) {
            Ok(plan) => plan,
            Err(refusal) => {
                runtime.say(hand_in_refusal_text(&job, &title, &refusal, &chart));
                continue;
            }
        };
        if let Some((item, count)) = &plan.goods {
            let mut hold = holds
                .get_mut(*player)
                .expect("the player ship's hold was read above");
            hold.remove(item, *count);
        }
        let mut balances = q_credits.p1();
        let Ok([(mut own_cr, mut own_reports), (mut station_cr, mut station_reports)]) =
            balances.get_many_mut([*player, *station])
        else {
            panic!("hand in: the player ship and station {station:?} each carry credits");
        };
        own_reports.change_credits(&mut own_cr, plan.own_cr, CreditChangeCauseType::JobReward);
        station_reports.change_credits(
            &mut station_cr,
            plan.station_cr,
            CreditChangeCauseType::JobReward,
        );
        jobs.complete(&job.id);
        info!(
            "nova_interface: handed in the job '{}' ({title}) for {} cr",
            job.id, plan.reward_cr
        );
        runtime.say(format!("handed in {title} for {}", cr_text(plan.reward_cr)));
        if let Ok(market) = q_markets.get(*station) {
            if let Some(step) = record_merit(&mut stances, market.civilization, clock.play_seconds)
            {
                info!(
                    "stance: {} moved from {:?} to {:?} on the hand-in at {:.1} s of play",
                    step.civilization.name(),
                    step.from,
                    step.to,
                    clock.play_seconds
                );
                runtime.say(format!(
                    "{} now holds you {}",
                    step.civilization.name(),
                    format!("{:?}", step.to).to_lowercase()
                ));
            }
        }
    }
}

/// Drives the Jobs pane's lists and actions.
pub(crate) struct JobsPanePlugin;

impl Plugin for JobsPanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JobsRuntime>();
        app.add_message::<JobAcceptCommand>();
        app.add_message::<JobHandInCommand>();
        app.add_systems(
            Update,
            (apply_job_commands, update_jobs_pane, update_jobs_note)
                .chain()
                .in_set(JobsPaneSystems),
        );
    }
}

/// System set for the Jobs pane's per-frame work. It runs after
/// `InterfaceSystems`, which builds the pane body it fills.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct JobsPaneSystems;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_starter_board_hides_a_job_the_hull_cannot_do_and_a_core_board_greys_its_accept() {
        let unfit = || Err(JobHullRefusalType::NoWeapon);
        assert_eq!(
            board_listing(true, unfit(), Ok(())),
            None,
            "the starter board leaves the job off"
        );
        assert_eq!(
            board_listing(false, unfit(), Ok(())),
            Some(Some("accept: the hull has no weapon".to_string())),
            "a core board lists it with the hull's reason"
        );
        assert_eq!(
            board_listing(false, unfit(), Err(JobAcceptRefusalType::TargetGone)),
            Some(Some("accept: the hull has no weapon".to_string())),
            "the hull's reason comes before the accept rule's"
        );
        assert_eq!(
            board_listing(true, Ok(()), Err(JobAcceptRefusalType::TargetGone)),
            Some(Some("accept: its target is gone".to_string())),
            "a fit job with a gone target is listed greyed on either board"
        );
        assert_eq!(
            board_listing(true, Ok(()), Ok(())),
            Some(None),
            "a fit, open job lists with Accept"
        );
    }

    #[test]
    fn a_cell_distance_reads_this_sector_or_whole_kilometres_from_the_player_s_cell() {
        let edge = Meters(40_000.0);
        let here = SectorCoord::new(1, 0, -2);
        assert_eq!(
            cell_distance_text(Some(here), here, edge).as_deref(),
            Some("this sector")
        );
        assert_eq!(
            cell_distance_text(Some(here), SectorCoord::new(1, 0, 0), edge).as_deref(),
            Some("80 km")
        );
        assert_eq!(
            cell_distance_text(Some(here), SectorCoord::new(4, 4, -2), edge).as_deref(),
            Some("200 km"),
            "a diagonal reads the straight line"
        );
        assert_eq!(
            cell_distance_text(None, here, edge),
            None,
            "no player cell, no distance"
        );
        assert_eq!(place_text(here, None), "(1, 0, -2)");
        assert_eq!(
            place_text(here, Some("80 km".to_string())),
            "(1, 0, -2), 80 km"
        );
    }
}
