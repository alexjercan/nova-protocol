//! The TAB interface's Jobs pane: the jobs the player holds, always, and
//! the docked station's board.
//!
//! The pane is one row: a column of panels and an inspector beside it.
//! Docked at a living station that offers Jobs, the column starts with the
//! station's board ("Jobs posted here"): one line per offer the player
//! neither holds nor completed and whose window has not passed, as kind
//! icon | title | reward | time-left meter with its minutes | Accept. Every
//! offer asks something of the hull ([`plan_job_hull_fit`]): the starting
//! station's board ([`StarterBoardMarker`]) leaves out what the
//! hull cannot do, every other board lists it with a greyed Accept and the
//! reason in a tooltip, as it does for an offer whose target is gone. Under
//! it, docked or not, "Your jobs" lists every held job as kind icon | title
//! | `have / need` meter | hand-in | Hand in | `X`. The hand-in column reads
//! HAND IN HERE docked at the job's target, else blank; a refused Hand in is
//! greyed with the reason in a tooltip. Over the held rows, one row reads the start role's goal: its
//! title, its next unmet line ([`open_objectives`]) and a `have / need`
//! meter, or the play time the goal was met; a role with no goal has no row.
//! Clicking a job row selects it and the inspector reads its task, its
//! target, its hand-in station, its reward, its time left or progress and a
//! Show on Map button, which opens the Map's chart view on the job's mark
//! ([`MapRuntime::chart_show`]).
//!
//! A click on Accept or Hand in raises a [`JobAcceptCommand`] or a
//! [`JobHandInCommand`]; [`apply_job_commands`] plans it with
//! [`plan_job_accept`] or [`plan_job_hand_in`], moves the goods and the
//! credits, moves the job in [`Jobs`] and raises [`JobAccepted`] so the open
//! world puts the job's bodies in place. The result shows on the notice line
//! under the panels for 4 s, green for a result, red for a refusal, with one
//! sound; the buttons are silent on press. A hand-in's line names the
//! payout, the new balance and the stance step it made.
//!
//! A held row's `X` drops nothing: it asks. The row then reads `Drop
//! <title>?` and swaps Hand in and `X` for Drop and Back. Back leaves the
//! job held either way; Drop raises a [`JobDropCommand`], and
//! [`apply_job_commands`] drops the job ([`Jobs::drop`]) with no payout,
//! whatever station it is docked at, and takes out the bodies its accept
//! put in the world, as a loss does. A drop of a job no longer held is
//! refused on the notice line.
//!
//! A board row's offer is not held: docked at its issuing station, it
//! carries a compact `X` of its own. Pressing it asks a paid delete instead
//! of a drop: the row reads `<title>` with the fee beside the credit glyph
//! and swaps Accept and `X` for Delete and Back. Delete raises a
//! [`JobOfferRefreshCommand`]; [`apply_job_commands`] revalidates the slot
//! is still current, unheld, unexpired and docked before it moves the fee
//! ([`JOB_OFFER_REFRESH_FEE_CR`], [`plan_job_offer_refresh`]) and marks the
//! slot for a fresh offer ([`JobPostings::request_paid_refresh`]), same as
//! a hand-in skips the wait. A slot already marked counts as stale, so a
//! second [`JobOfferRefreshCommand`] for the same id, the same frame or the
//! next, refuses instead of paying the fee twice. A refusal, including one
//! an accept the same frame raises by holding the job first, changes
//! nothing and shows on the notice line.
//!
//! Touch this module when changing what the lists show, what gates an
//! offer, or what a hand-in moves.

use bevy::{
    ecs::system::SystemParam,
    picking::hover::Hovered,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, Button},
};
use nova_events::prelude::{CreditChangeCauseType, GeneratedBodyId, Meters, Meters3, SectorCoord};
use nova_gameplay::prelude::*;
use nova_ship::prelude::DockingSectionMarker;
use nova_ui::{
    prelude::*,
    theme::UiColor,
    widget::{ThemedBorder, ThemedText},
};
use nova_world_base::prelude::record_merit;

use crate::{
    icons::{icon_node, ChartIconType, InterfaceIcons, JobIconType},
    map::{job_cell, MapChartMarkType, MapRuntime},
    pane::{
        compact_button, control_row, divider, notice_line, play_menu_select, refusable_button,
        side_panel, spawn_credit_figure, themed_label, view_split, ActionNotice, ActionOutcomeType,
        InterfacePaneType,
    },
    ship::StationSite,
};

/// Glob-import surface: `use nova_interface::jobs::prelude::*`.
pub mod prelude {
    pub use super::{
        JobAcceptCommand, JobHandInCommand, JobsRuntime, JOBS_BOARD, JOBS_HELD, JOBS_MILESTONE,
        JOBS_NOTICE, JOBS_PANE, JOB_ACCEPT_PREFIX, JOB_HAND_IN_PREFIX, JOB_SHOW_ON_MAP,
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
/// The `Name` prefix of a held row's `X` button: `JobDrop-<row>`.
pub(crate) const JOB_DROP_PREFIX: &str = "JobDrop-";
/// The `Name` prefix of a confirming held row's Drop button:
/// `JobDropConfirm-<row>`.
pub(crate) const JOB_DROP_CONFIRM_PREFIX: &str = "JobDropConfirm-";
/// The `Name` prefix of a confirming held row's Back button:
/// `JobDropBack-<row>`.
pub(crate) const JOB_DROP_BACK_PREFIX: &str = "JobDropBack-";
/// The `Name` prefix of a board row's `X` button: `JobRefresh-<row>`.
pub(crate) const JOB_REFRESH_PREFIX: &str = "JobRefresh-";
/// The `Name` prefix of a confirming board row's Delete button:
/// `JobRefreshConfirm-<row>`.
pub(crate) const JOB_REFRESH_CONFIRM_PREFIX: &str = "JobRefreshConfirm-";
/// The `Name` prefix of a confirming board row's Back button:
/// `JobRefreshBack-<row>`.
pub(crate) const JOB_REFRESH_BACK_PREFIX: &str = "JobRefreshBack-";
/// The `Name` of the inspector's Show on Map button.
pub const JOB_SHOW_ON_MAP: &str = "JobShowOnMap";
/// The `Name` of the pane's notice line, under the panels.
pub const JOBS_NOTICE: &str = "JobsNotice";

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

/// A confirmed request to drop the held job `id`, raised only by a
/// confirming held row's Drop button.
#[derive(Message, Clone, Debug)]
pub(crate) struct JobDropCommand {
    /// The held job.
    pub(crate) id: JobId,
}

/// A confirmed request to pay the fee ([`JOB_OFFER_REFRESH_FEE_CR`]) that
/// deletes the current, unheld board offer `id` ahead of its window, for a
/// fresh one in its place, raised only by a confirming board row's Delete
/// button. [`apply_job_commands`] revalidates the slot's live state before
/// it moves the fee, rather than trusting the ask.
#[derive(Message, Clone, Debug)]
pub(crate) struct JobOfferRefreshCommand {
    /// The posted offer.
    pub(crate) id: JobId,
}

/// What the Jobs pane shows beyond the records: the selected row, the drop
/// that waits for its confirm and the notice line. Kept while the interface
/// is closed, like the pane itself. A selection no row shows any more, or a
/// confirm for a job no longer held, is cleared by [`update_jobs_pane`].
#[derive(Resource, Default, Debug)]
pub struct JobsRuntime {
    /// The row the inspector reads.
    pub(crate) selected: Option<JobId>,
    /// The held job whose `X` was pressed: its row asks Drop or Back.
    pub(crate) confirm_drop: Option<JobId>,
    /// The board offer whose `X` was pressed: its row asks Refresh or Back.
    pub(crate) confirm_refresh: Option<JobId>,
    /// The last Accept or Hand in result, shown on the notice line.
    pub notice: ActionNotice,
}

/// The pane's row of panels, rebuilt by [`update_jobs_pane`].
#[derive(Component)]
pub(crate) struct JobsPaneMarker;

/// The notice line under the panels.
#[derive(Component)]
pub(crate) struct JobsNoticeMarker;

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

/// Which step of a row's two-click ask a button takes: `X` asks for a
/// confirm, the primary button acts on it and Back cancels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum JobAskStepType {
    /// `X`: ask for a confirm.
    Ask,
    /// Drop or Refresh: raise the command.
    Confirm,
    /// Back: keep the job or the offer as it was.
    Back,
}

/// A held row's `X`, Drop or Back button.
#[derive(Component, Clone, Debug)]
struct JobDropButton {
    step: JobAskStepType,
    id: JobId,
}

/// A board row's `X`, Refresh or Back button, asking a paid offer refresh.
#[derive(Component, Clone, Debug)]
struct JobRefreshButton {
    step: JobAskStepType,
    id: JobId,
}

/// The inspector's Show on Map button: the mark it opens the chart on.
#[derive(Component, Clone, Debug)]
struct JobShowOnMap(MapChartMarkType);

/// Build the Jobs pane body: the row of panels and the notice line.
pub(crate) fn jobs_body(body: &mut ChildSpawnerCommands) {
    body.spawn((JobsPaneMarker, Name::new(JOBS_PANE), view_split()));
    body.spawn((JobsNoticeMarker, Name::new(JOBS_NOTICE), notice_line()));
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
        JobAcceptRefusalType::AlreadyHeld => "accept: you took it already".to_string(),
        JobAcceptRefusalType::AlreadyDone => "accept: you completed it already".to_string(),
        JobAcceptRefusalType::Expired => "accept: this offer expired".to_string(),
        JobAcceptRefusalType::TargetGone => "accept: its target is gone".to_string(),
    }
}

/// Why a hand-in of `job` is refused; the list and the handler share it.
/// The text never names the job: every place that shows it sits beside the
/// job's title.
fn hand_in_refusal_text(
    job: &Job,
    refusal: &JobHandInRefusalType,
    chart: &Chart,
    items: &GameItems,
) -> String {
    match refusal {
        JobHandInRefusalType::NotDocked => {
            format!("dock at {} to hand in", station_name(chart, &job.target))
        }
        JobHandInRefusalType::WrongStation { needed } => {
            format!("hand in at {}", station_name(chart, needed))
        }
        JobHandInRefusalType::ShortItems { item, need, have } => {
            format!(
                "the hold carries {have} of {need} {}",
                items.design(item).name
            )
        }
        JobHandInRefusalType::NotDone => "not done yet".to_string(),
        JobHandInRefusalType::StationCannotPay { need, have } => format!(
            "hand in: it pays {}, the station holds {}",
            cr_text(*need),
            cr_text(*have)
        ),
        JobHandInRefusalType::CreditOverflow => {
            "hand in: the ship's balance after the reward does not fit".to_string()
        }
    }
}

/// Why an offer refresh would be refused, for its confirming row and the
/// notice line. The text never names the job, like [`hand_in_refusal_text`].
fn offer_refresh_refusal_text(refusal: &JobOfferRefreshRefusalType) -> String {
    match refusal {
        JobOfferRefreshRefusalType::Stale => "delete: this posting renewed already".to_string(),
        JobOfferRefreshRefusalType::Held => "delete: you already hold this job".to_string(),
        JobOfferRefreshRefusalType::Expired => "delete: this offer expired".to_string(),
        JobOfferRefreshRefusalType::NotDocked => format!(
            "delete: dock at the issuing station to pay {}",
            cr_text(JOB_OFFER_REFRESH_FEE_CR)
        ),
        JobOfferRefreshRefusalType::CreditsShort { need, have } => format!(
            "delete: it costs {}, the balance holds {}",
            cr_text(*need),
            cr_text(*have)
        ),
        JobOfferRefreshRefusalType::CreditOverflow => {
            "delete: the station's balance after the fee does not fit".to_string()
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
    reward_cr: u64,
    /// The offer's window left: the share and the minutes.
    time_left: (f32, String),
    /// Why Accept is greyed, if it is.
    refusal: Option<String>,
    /// Its `X` was pressed: the row asks Refresh or Back.
    confirming: bool,
    /// Why a Refresh confirm would be refused right now, if `confirming`
    /// and it would be.
    refresh_refusal: Option<String>,
}

/// One held row as the pane shows it.
#[derive(Clone, Debug, PartialEq)]
struct HeldRow {
    id: JobId,
    icon: JobIconType,
    title: String,
    have: u32,
    need: u32,
    /// Docked at the station this job hands in at: the row shows HAND IN HERE.
    hand_in_here: bool,
    /// Why Hand in is greyed, if it is.
    refusal: Option<String>,
    /// Its `X` was pressed: the row asks Drop or Back.
    confirming: bool,
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
    done: bool,
    task: String,
    target: String,
    hand_in: String,
    reward_cr: u64,
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
    // The meter's share of the window `SlotPosting::remaining` counts down.
    let time_left = |job: &Job, remaining: f64| {
        let window = if matches!(job.kind, JobType::Escort { .. }) {
            rule.escort_offer_seconds
        } else {
            rule.offer_seconds
        };
        (
            (remaining / f64::from(window)) as f32,
            format!("{} min", (remaining.max(0.0) / 60.0).ceil()),
        )
    };

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
    let hand_in_refusal = |job: &Job| {
        let (Some(hold), Some(own_cr)) = (hold, own_cr) else {
            return Some(hand_in_refusal_text(
                job,
                &JobHandInRefusalType::NotDocked,
                &chart,
                &items,
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
        .map(|refusal| hand_in_refusal_text(job, &refusal, &chart, &items))
    };
    if runtime
        .confirm_drop
        .as_ref()
        .is_some_and(|id| !jobs.holds(id))
    {
        runtime.confirm_drop = None;
    }
    if runtime.confirm_refresh.as_ref().is_some_and(|id| {
        jobs.holds(id)
            || postings
                .slot(id)
                .is_none_or(|slot| slot.is_expired(rule, now))
    }) {
        runtime.confirm_refresh = None;
    }
    // Reused by the confirming row and the notice line, so both read the fee
    // and the shortfall off the same check order as `apply_job_commands`.
    let offer_refresh_refusal = |job: &Job| -> Option<String> {
        let docked = partner.as_ref().filter(|(_, _, id)| *id == job.id.station);
        let station_cr = docked
            .and_then(|(_, station, _)| q_credits.get(*station).ok())
            .map_or(0, |credits| credits.0);
        plan_job_offer_refresh(
            &job.id,
            postings
                .slot(&job.id)
                .is_some_and(|slot| !slot.paid_refresh_requested),
            jobs.holds(&job.id),
            postings
                .slot(&job.id)
                .is_none_or(|slot| slot.is_expired(rule, now)),
            partner.as_ref().map(|(_, _, id)| id),
            own_cr.unwrap_or(0),
            station_cr,
        )
        .err()
        .map(|refusal| offer_refresh_refusal_text(&refusal))
    };

    let board = partner.as_ref().and_then(|(_, station, station_id)| {
        let (board, starter) = q_boards.get(*station).ok()?;
        let rows = board
            .0
            .iter()
            .filter(|job| {
                !jobs.holds(&job.id) && !jobs.is_completed(&job.id) && !jobs.is_dropped(&job.id)
            })
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
                let confirming = runtime.confirm_refresh.as_ref() == Some(&job.id);
                let refresh_refusal = confirming.then(|| offer_refresh_refusal(job)).flatten();
                Some(PostedRow {
                    id: job.id.clone(),
                    icon: JobIconType::of(&job.kind),
                    title: job.title(&items),
                    reward_cr: job.reward_cr.into(),
                    time_left: time_left(job, remaining),
                    refusal,
                    confirming,
                    refresh_refusal,
                })
            })
            .collect();
        Some(BoardView {
            station: station_name(&chart, station_id),
            rows,
        })
    });

    let held: Vec<HeldRow> = jobs
        .held()
        .iter()
        .map(|job| {
            let hand_in_here =
                matches!(&partner, Some((_, _, station_id)) if *station_id == job.target);
            let (have, need) = job.progress(hold, jobs.is_done(&job.id));
            let confirming = runtime.confirm_drop.as_ref() == Some(&job.id);
            HeldRow {
                id: job.id.clone(),
                icon: JobIconType::of(&job.kind),
                title: job.title(&items),
                have,
                need,
                hand_in_here,
                refusal: hand_in_refusal(job),
                confirming,
            }
        })
        .collect();

    let selected = runtime.selected.as_ref().and_then(|id| {
        jobs.job(id).map(|job| (job, true)).or_else(|| {
            let (_, station, _) = partner.as_ref()?;
            let (board, _) = q_boards.get(*station).ok()?;
            board
                .job(id)
                .filter(|job| !jobs.is_completed(&job.id) && !jobs.is_dropped(&job.id))
                .map(|job| (job, false))
        })
    });
    if selected.is_none() && runtime.selected.is_some() {
        runtime.selected = None;
    }
    let inspector = selected.map(|(job, held)| {
        let done = jobs.is_done(&job.id);
        let show = if held {
            MapChartMarkType::Job(job.id.clone())
        } else if chart.station(job.posted_by()).is_some() {
            MapChartMarkType::Station(job.posted_by().clone())
        } else {
            MapChartMarkType::Cell(job.posted_by().origin)
        };
        InspectorView {
            icon: JobIconType::of(&job.kind),
            title: job.title(&items),
            done,
            task: job.task(&items),
            target: format!("{}\n{}", target_name(job, &chart), place(job_cell(job))),
            hand_in: station_name(&chart, &job.target),
            reward_cr: job.reward_cr.into(),
            time_left: postings
                .slot(&job.id)
                .filter(|_| !held)
                .map(|slot| time_left(job, slot.remaining(rule, now))),
            progress: held.then(|| job.progress(hold, done)),
            refusal: held.then(|| hand_in_refusal(job)).flatten(),
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
            .spawn(side_panel(Overflow::clip()))
            .with_children(|side| inspector_panel(side, &icons, view.inspector.as_ref()));
    });
    *built = Some(view);
}

/// Width of the reward column.
const REWARD_PX: f32 = 90.0;
/// Width of a held row's hand-in column: the HAND IN HERE badge.
const HAND_IN_PX: f32 = 90.0;
/// Width of the time-left column.
const TIME_PX: f32 = 120.0;
/// Width of a meter inside a row column, so its text fits beside it.
const ROW_METER_PX: f32 = 72.0;
/// Width of the progress column: a row meter and the widest `nn / nn`.
const PROGRESS_PX: f32 = 130.0;
/// Width of a row's action button column.
const ACTION_PX: f32 = 78.0;
/// Width of a held row's `X` column.
const DROP_PX: f32 = 28.0;
/// Width of a row's icon column.
const ICON_PX: f32 = 28.0;
/// Width of a meter in the inspector.
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

/// A `have / need` meter: a display meter `width` wide with the counts beside
/// it.
fn meter(width: f32, have: u64, need: u64) -> impl Bundle {
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
            bare_meter(width, fraction),
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
        refusable_button(column, name, label, refusal.map(String::as_str))
            .insert(button)
            .observe(on_job_button);
    });
}

/// A held row's drop step button, right-aligned in a column `width` wide,
/// greyed with `refusal` as its tooltip when given, as [`refusable_button`]
/// greys an action button.
fn drop_step_button(
    row: &mut ChildSpawnerCommands,
    width: f32,
    name: String,
    spec: ButtonSpec,
    button: JobDropButton,
    refusal: Option<&str>,
) {
    row.spawn((
        Node {
            width: px(width),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::Center,
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|column| {
        let mut button = column.spawn((Name::new(name), compact_button(spec), button));
        button.observe(on_job_drop_button);
        if let Some(why) = refusal {
            button.insert((InteractionDisabled, Tooltip(why.to_string())));
        }
    });
}

/// A board row's refresh step button, right-aligned in a column `width`
/// wide, greyed with `refusal` as its tooltip when given, as
/// [`drop_step_button`] greys a held row's.
fn refresh_step_button(
    row: &mut ChildSpawnerCommands,
    width: f32,
    name: String,
    spec: ButtonSpec,
    button: JobRefreshButton,
    refusal: Option<&str>,
) {
    row.spawn((
        Node {
            width: px(width),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::Center,
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|column| {
        let mut button = column.spawn((Name::new(name), compact_button(spec), button));
        button.observe(on_job_refresh_button);
        if let Some(why) = refusal {
            button.insert((InteractionDisabled, Tooltip(why.to_string())));
        }
    });
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
                    (TIME_PX, "Time left"),
                    (ACTION_PX, ""),
                    (DROP_PX, ""),
                ],
            );
            if board.rows.is_empty() {
                panel.spawn(badge(BadgeKind::Mute, "EMPTY"));
            }
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
                    if row.confirming {
                        r.spawn(grow_cell(&row.title, 13.0, UiColor::Accent));
                    } else {
                        r.spawn(grow_cell(&row.title, 13.0, UiColor::Primary));
                    }
                    if row.confirming {
                        let tooltip = row.refresh_refusal.clone().unwrap_or_else(|| {
                            "pays the station to post a fresh offer in its place".to_string()
                        });
                        r.spawn((
                            Node {
                                width: px(REWARD_PX),
                                flex_shrink: 0.0,
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            Tooltip(tooltip),
                        ))
                        .with_children(|fee| {
                            spawn_credit_figure(
                                fee,
                                icons,
                                u64::from(JOB_OFFER_REFRESH_FEE_CR),
                                13.0,
                                UiColor::Accent,
                                (),
                            );
                        });
                    } else {
                        r.spawn((
                            Node {
                                width: px(REWARD_PX),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|reward| {
                            spawn_credit_figure(
                                reward,
                                icons,
                                row.reward_cr,
                                13.0,
                                UiColor::Accent,
                                (),
                            );
                        });
                    }
                    if row.confirming {
                        let refresh_button = |step| JobRefreshButton {
                            step,
                            id: row.id.clone(),
                        };
                        refresh_step_button(
                            r,
                            TIME_PX,
                            format!("{JOB_REFRESH_CONFIRM_PREFIX}{index}"),
                            ButtonSpec::new("Delete").fit().danger(),
                            refresh_button(JobAskStepType::Confirm),
                            row.refresh_refusal.as_deref(),
                        );
                        refresh_step_button(
                            r,
                            ACTION_PX,
                            format!("{JOB_REFRESH_BACK_PREFIX}{index}"),
                            ButtonSpec::new("Back").fit(),
                            refresh_button(JobAskStepType::Back),
                            None,
                        );
                        r.spawn(fixed(DROP_PX, ()));
                        return;
                    }
                    r.spawn(fixed(
                        TIME_PX,
                        (
                            Node {
                                align_items: AlignItems::Center,
                                column_gap: px(8),
                                ..default()
                            },
                            Pickable::IGNORE,
                            children![
                                bare_meter(ROW_METER_PX, row.time_left.0),
                                text_nowrap(&row.time_left.1, 12.0, UiColor::Body),
                            ],
                        ),
                    ));
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
                    // The ask step swaps the `X` label for the job-drop-cancel
                    // mark, the same asset the held row's `X` uses.
                    r.spawn((
                        Node {
                            width: px(DROP_PX),
                            flex_shrink: 0.0,
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::FlexEnd,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|column| {
                        let mut ask = column.spawn((
                            Name::new(format!("{JOB_REFRESH_PREFIX}{index}")),
                            compact_button(ButtonSpec::new("").fit().ghost()),
                            JobRefreshButton {
                                step: JobAskStepType::Ask,
                                id: row.id.clone(),
                            },
                            Tooltip("Delete offer".to_string()),
                        ));
                        ask.observe(on_job_refresh_button);
                        ask.with_children(|b| {
                            b.spawn(icon_node(icons.job_drop_cancel(), UiColor::Primary, 14.0));
                        });
                    });
                });
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
                        row.spawn((
                            Node {
                                flex_grow: 1.0,
                                flex_basis: px(0),
                                min_width: px(0),
                                ..default()
                            },
                            Pickable::IGNORE,
                            children![themed_label(&text, 13.0, color)],
                        ));
                        match progress {
                            Some((have, need)) => {
                                row.spawn(fixed(PROGRESS_PX, meter(ROW_METER_PX, have, need)))
                            }
                            None => row.spawn(fixed(PROGRESS_PX, ())),
                        };
                        row.spawn(fixed(HAND_IN_PX, ()));
                        row.spawn(fixed(ACTION_PX, ()));
                        row.spawn(fixed(DROP_PX, ()));
                    });
            }
            table_head(
                panel,
                &[
                    (ICON_PX, ""),
                    (0.0, "Job"),
                    (PROGRESS_PX, "Progress"),
                    (HAND_IN_PX, "Hand-in"),
                    (ACTION_PX, ""),
                    (DROP_PX, ""),
                ],
            );
            if view.held.is_empty() {
                panel.spawn(badge(BadgeKind::Mute, "EMPTY"));
            }
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
                    if row.confirming {
                        r.spawn(grow_cell(
                            &format!("Drop {}?", row.title),
                            13.0,
                            UiColor::Accent,
                        ));
                    } else {
                        r.spawn(grow_cell(&row.title, 13.0, UiColor::Primary));
                    }
                    r.spawn(fixed(
                        PROGRESS_PX,
                        meter(ROW_METER_PX, row.have.into(), row.need.into()),
                    ));
                    let drop_button = |step| JobDropButton {
                        step,
                        id: row.id.clone(),
                    };
                    if row.confirming {
                        drop_step_button(
                            r,
                            HAND_IN_PX,
                            format!("{JOB_DROP_CONFIRM_PREFIX}{index}"),
                            ButtonSpec::new("Drop").fit().danger(),
                            drop_button(JobAskStepType::Confirm),
                            None,
                        );
                        drop_step_button(
                            r,
                            ACTION_PX,
                            format!("{JOB_DROP_BACK_PREFIX}{index}"),
                            ButtonSpec::new("Back").fit(),
                            drop_button(JobAskStepType::Back),
                            None,
                        );
                        r.spawn(fixed(DROP_PX, ()));
                        return;
                    }
                    if row.hand_in_here {
                        r.spawn(fixed(HAND_IN_PX, badge(BadgeKind::Green, "HAND IN HERE")));
                    } else {
                        r.spawn(fixed(HAND_IN_PX, ()));
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
                    // The ask step swaps the `X` label for the job-drop-cancel
                    // mark: Bevy UI cannot draw the authored SVG directly, so
                    // the icon is an AssetServer-loaded raster child instead
                    // of the button's own label span.
                    r.spawn((
                        Node {
                            width: px(DROP_PX),
                            flex_shrink: 0.0,
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::FlexEnd,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|column| {
                        let mut ask = column.spawn((
                            Name::new(format!("{JOB_DROP_PREFIX}{index}")),
                            compact_button(ButtonSpec::new("").fit().ghost()),
                            drop_button(JobAskStepType::Ask),
                            Tooltip("Drop job".to_string()),
                        ));
                        ask.observe(on_job_drop_button);
                        ask.with_children(|b| {
                            b.spawn(icon_node(icons.job_drop_cancel(), UiColor::Primary, 14.0));
                        });
                    });
                });
            }
        });
}

/// A `label  value` line of the inspector. The value wraps under itself, and
/// the label stays level with its first line.
fn kv(side: &mut ChildSpawnerCommands, label: &str, value: &str) {
    side.spawn(Node {
        column_gap: px(8),
        align_items: AlignItems::FlexStart,
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|row| {
        row.spawn(cell(LABEL_PX, label, 12.0, UiColor::Label));
        row.spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
            children![themed_label(value, 13.0, UiColor::Primary)],
        ));
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
        if job.done {
            head.spawn(badge(BadgeKind::Green, "DONE"));
        }
    });
    side.spawn(divider());
    side.spawn(themed_label("TASK", 11.0, UiColor::Label));
    side.spawn(themed_label(&job.task, 12.0, UiColor::Body));
    side.spawn(divider());
    kv(side, "Target", &job.target);
    kv(side, "Hand in", &job.hand_in);
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|row| {
        row.spawn(cell(LABEL_PX, "Reward", 12.0, UiColor::Label));
        spawn_credit_figure(row, icons, job.reward_cr, 13.0, UiColor::Accent, ());
    });
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
            row.spawn(meter(METER_PX, have.into(), need.into()));
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

/// Raise the list's command for the button's job when it is activated. The
/// press is silent: the result's notice plays the one cue.
fn on_job_button(
    activate: On<Activate>,
    q_button: Query<&JobActionButton>,
    mut accepts: MessageWriter<JobAcceptCommand>,
    mut hand_ins: MessageWriter<JobHandInCommand>,
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
}

/// Take the button's drop step when it is activated: `X` asks for a
/// confirm and Back keeps the job, each with one click; Drop raises the
/// [`JobDropCommand`] silently, as the result's notice plays the one cue.
fn on_job_drop_button(
    activate: On<Activate>,
    q_button: Query<&JobDropButton>,
    mut runtime: ResMut<JobsRuntime>,
    mut drops: MessageWriter<JobDropCommand>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok(button) = q_button.get(activate.entity) else {
        return;
    };
    match button.step {
        JobAskStepType::Ask => {
            runtime.confirm_drop = Some(button.id.clone());
            play_menu_select(&mut commands, bank.as_deref());
        }
        JobAskStepType::Back => {
            runtime.confirm_drop = None;
            play_menu_select(&mut commands, bank.as_deref());
        }
        JobAskStepType::Confirm => {
            runtime.confirm_drop = None;
            drops.write(JobDropCommand {
                id: button.id.clone(),
            });
        }
    }
}

/// Take the button's refresh step when it is activated: `X` asks for a
/// confirm and Back keeps the offer, each with one click; Refresh raises
/// the [`JobOfferRefreshCommand`] silently, as the result's notice plays
/// the one cue.
fn on_job_refresh_button(
    activate: On<Activate>,
    q_button: Query<&JobRefreshButton>,
    mut runtime: ResMut<JobsRuntime>,
    mut refreshes: MessageWriter<JobOfferRefreshCommand>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok(button) = q_button.get(activate.entity) else {
        return;
    };
    match button.step {
        JobAskStepType::Ask => {
            runtime.confirm_refresh = Some(button.id.clone());
            play_menu_select(&mut commands, bank.as_deref());
        }
        JobAskStepType::Back => {
            runtime.confirm_refresh = None;
            play_menu_select(&mut commands, bank.as_deref());
        }
        JobAskStepType::Confirm => {
            runtime.confirm_refresh = None;
            refreshes.write(JobOfferRefreshCommand {
                id: button.id.clone(),
            });
        }
    }
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

/// Tick the Jobs notice: count its dwell down on the real clock, play the
/// cue a result owes once the sound bank is present, and paint the line in
/// its outcome's tone. It runs whatever pane shows, like the Station pane's,
/// so a result raised with the Jobs pane closed still plays its one cue.
fn update_jobs_notice(
    time: Res<Time<Real>>,
    mut runtime: ResMut<JobsRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut q_notice: Query<(&mut Text, &mut ThemedText), With<JobsNoticeMarker>>,
) {
    runtime
        .notice
        .tick(time.delta_secs(), bank.as_deref(), &mut commands);
    let (note, tone) = runtime.notice.line();
    for (mut text, mut themed) in &mut q_notice {
        if text.0 != note {
            text.0 = note.to_string();
        }
        if themed.color != tone {
            themed.color = tone;
        }
    }
}

/// Apply [`JobAcceptCommand`]s by the [`plan_job_accept`] rule and
/// [`JobHandInCommand`]s by the [`plan_job_hand_in`] rule: an accept holds
/// the job and raises [`JobAccepted`]; a hand-in takes a procurement's goods
/// out of the hold, pays the reward with `JobReward` credit changes on both
/// sides and completes the job, and reports the payout, the new balance and
/// the stance step it made on one Done notice. A refusal changes nothing and
/// shows its reason on the notice line. An accept of a job the docked
/// station's board no longer posts is refused as expired: the slot renewed
/// between the click and this frame. A [`JobDropCommand`] drops the held job
/// ([`Jobs::drop`]) with no payout and takes out the bodies its accept put
/// in the world ([`Job::spawned_bodies`]), as `drop_on_loss` does in the
/// open world crate, whatever station it is docked at; a drop of a job no
/// longer held, lost or handed in since the confirm, is refused. A
/// [`JobOfferRefreshCommand`] instead pays to delete a current, unheld
/// board offer ahead of its window, for a fresh one in its place: planned
/// by [`plan_job_offer_refresh`] against the slot's live state, read fresh
/// rather than trusted from the ask (never held, since an accept the same
/// frame already moved the job into [`Jobs`]), it moves
/// [`JOB_OFFER_REFRESH_FEE_CR`] from the player to the station with
/// `JobRefresh` credit changes on both sides and marks the slot for a
/// fresh offer ([`JobPostings::request_paid_refresh`]), both only once the
/// plan holds; a refusal changes nothing.
///
/// # Panics
///
/// On a hand-in of a job the player does not hold: the list only shows a
/// button for a job it lists. On an allowed offer refresh whose docked
/// station carries no `ShipCredits`: a station the board lists always does.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the four commands, the board, the jobs, the postings, the clock, the table, the site, the bodies, the targets, the hold, both balances and the stances"
)]
#[expect(
    clippy::type_complexity,
    reason = "both balances and report queues in one query"
)]
fn apply_job_commands(
    mut commands: Commands,
    (mut accepts, mut hand_ins, mut drops, mut refreshes): (
        MessageReader<JobAcceptCommand>,
        MessageReader<JobHandInCommand>,
        MessageReader<JobDropCommand>,
        MessageReader<JobOfferRefreshCommand>,
    ),
    mut runtime: ResMut<JobsRuntime>,
    items: Option<Res<GameItems>>,
    table: Option<Res<GamePriceTable>>,
    chart: Res<Chart>,
    mut jobs: ResMut<Jobs>,
    mut postings: ResMut<JobPostings>,
    clock: Res<WorldClock>,
    site: StationSite,
    (q_ids, q_bodies): (Query<&GeneratedBodyId>, Query<(Entity, &GeneratedBodyId)>),
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
    if accepts.is_empty() && hand_ins.is_empty() && drops.is_empty() && refreshes.is_empty() {
        return;
    }
    let (Some(items), Some(table)) = (items, table) else {
        accepts.clear();
        hand_ins.clear();
        drops.clear();
        refreshes.clear();
        return;
    };
    let partner = jobs_partner(&site, &q_ids);
    for command in accepts.read() {
        let Some((_, station, station_id)) = &partner else {
            runtime.notice.say(
                ActionOutcomeType::Refused,
                accept_refusal_text(&JobAcceptRefusalType::NotDocked),
            );
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
            runtime.notice.say(
                ActionOutcomeType::Refused,
                accept_refusal_text(&JobAcceptRefusalType::Expired),
            );
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
                runtime
                    .notice
                    .say(ActionOutcomeType::Done, format!("accepted: {title}"));
            }
            Err(refusal) => runtime
                .notice
                .say(ActionOutcomeType::Refused, accept_refusal_text(&refusal)),
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
            runtime.notice.say(
                ActionOutcomeType::Refused,
                hand_in_refusal_text(&job, &JobHandInRefusalType::NotDocked, &chart, &items),
            );
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
                runtime.notice.say(
                    ActionOutcomeType::Refused,
                    hand_in_refusal_text(&job, &refusal, &chart, &items),
                );
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
        let mut line = format!(
            "handed in {title} for {}; balance {}",
            cr_text(plan.reward_cr),
            cr_text(own_cr.0)
        );
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
                line.push_str(&format!(
                    "; {} now holds you {}",
                    step.civilization.name(),
                    format!("{:?}", step.to).to_lowercase()
                ));
            }
        }
        runtime.notice.say(ActionOutcomeType::Done, line);
    }
    for command in drops.read() {
        if !jobs.holds(&command.id) {
            warn!(
                "nova_interface: the drop of '{}' finds it not held",
                command.id
            );
            runtime.notice.say(
                ActionOutcomeType::Refused,
                "drop: the job is no longer held".to_string(),
            );
            continue;
        }
        let job = jobs.drop(&command.id);
        let gone = job.spawned_bodies();
        for (entity, id) in &q_bodies {
            if gone.contains(id) {
                commands.entity(entity).try_despawn();
            }
        }
        let title = job.title(&items);
        info!("nova_interface: dropped the job '{}' ({title})", job.id);
        runtime
            .notice
            .say(ActionOutcomeType::Done, format!("dropped: {title}"));
    }
    for command in refreshes.read() {
        let current = postings
            .slot(&command.id)
            .is_some_and(|slot| !slot.paid_refresh_requested);
        let held = jobs.holds(&command.id);
        let expired = postings
            .slot(&command.id)
            .is_none_or(|slot| slot.is_expired(table.0.jobs, clock.play_seconds));
        let docked = partner
            .as_ref()
            .filter(|(_, _, id)| *id == command.id.station);
        let (own_cr, station_cr) = match docked {
            Some((player, station, _)) => (
                q_credits.p0().get(*player).map_or(0, |c| c.0),
                q_credits.p0().get(*station).map_or(0, |c| c.0),
            ),
            None => (0, 0),
        };
        let plan = plan_job_offer_refresh(
            &command.id,
            current,
            held,
            expired,
            docked.map(|(_, _, id)| id),
            own_cr,
            station_cr,
        );
        let plan = match plan {
            Ok(plan) => plan,
            Err(refusal) => {
                runtime.notice.say(
                    ActionOutcomeType::Refused,
                    offer_refresh_refusal_text(&refusal),
                );
                continue;
            }
        };
        let title = postings
            .slot(&command.id)
            .expect("an allowed refresh plan reads a current slot")
            .offer
            .title(&items);
        let (player, station, _) =
            docked.expect("an allowed refresh plan is docked at the issuing station");
        let mut balances = q_credits.p1();
        let Ok([(mut own_credits, mut own_reports), (mut station_credits, mut station_reports)]) =
            balances.get_many_mut([*player, *station])
        else {
            panic!("offer refresh: the player ship and station {station:?} each carry credits");
        };
        own_reports.change_credits(
            &mut own_credits,
            plan.own_cr,
            CreditChangeCauseType::JobRefresh,
        );
        station_reports.change_credits(
            &mut station_credits,
            plan.station_cr,
            CreditChangeCauseType::JobRefresh,
        );
        postings.request_paid_refresh(&command.id);
        info!(
            "nova_interface: paid {} cr to refresh the offer '{}'",
            plan.fee_cr, command.id
        );
        runtime.notice.say(
            ActionOutcomeType::Done,
            format!(
                "deleted {title} for {}; balance {}",
                cr_text(plan.fee_cr),
                cr_text(own_credits.0)
            ),
        );
    }
}

/// Drives the Jobs pane's lists and actions.
pub(crate) struct JobsPanePlugin;

impl Plugin for JobsPanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JobsRuntime>();
        app.add_message::<JobAcceptCommand>();
        app.add_message::<JobHandInCommand>();
        app.add_message::<JobDropCommand>();
        app.add_message::<JobOfferRefreshCommand>();
        app.add_systems(
            Update,
            (apply_job_commands, update_jobs_pane, update_jobs_notice)
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
    use core::time::Duration;
    use std::collections::BTreeMap;

    use bevy::{asset::AssetPlugin, time::TimeUpdateStrategy};
    use nova_gameplay::test_support::test_items;
    use nova_ship::prelude::*;
    use nova_world_base::prelude::{CivilizationField, CivilizationStatusType};

    use super::*;
    use crate::pointer_rig::{hear_ui_cues, take_cues};

    /// A price table with every factor at one: the hand-in reads only its
    /// offer window.
    fn price_table() -> PriceTable {
        PriceTable {
            clamp: PriceClamp {
                min: 0.25,
                max: 3.0,
            },
            scarcity: ScarcityRule {
                starved: PriceFactor::ONE,
                glut: PriceFactor::ONE,
                glut_share: 3.0,
                unkept: PriceFactor::ONE,
            },
            restock: RestockRule {
                recovery_seconds: 1_200.0,
            },
            jobs: JobOfferRule {
                offer_seconds: 1_200.0,
                escort_offer_seconds: 3_600.0,
            },
            clusters: Vec::new(),
            roles: Vec::new(),
        }
    }

    /// The job handler and the notice painter on a manual real clock, with
    /// the UI sound bank captured and no UI tree: the cue must not need the
    /// line.
    fn jobs_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.5,
        )));
        app.insert_resource(test_items());
        app.insert_resource(GamePriceTable(price_table()));
        app.init_resource::<JobsRuntime>();
        app.init_resource::<Chart>();
        app.init_resource::<Jobs>();
        app.init_resource::<JobPostings>();
        app.init_resource::<WorldClock>();
        app.init_resource::<CivilizationStances>();
        app.add_message::<JobAcceptCommand>();
        app.add_message::<JobHandInCommand>();
        app.add_message::<JobDropCommand>();
        app.add_message::<JobOfferRefreshCommand>();
        app.add_systems(Update, (apply_job_commands, update_jobs_notice).chain());
        hear_ui_cues(&mut app);
        // The first frame advances the clock by nothing.
        app.update();
        app
    }

    /// A living civilization whose seeded stance is Neutral: one step up is
    /// Ally.
    fn neutral_civilization() -> CivilizationId {
        for seed in 0..64u32 {
            let field = CivilizationField::new(seed);
            for x in -2..=2i32 {
                for y in -2..=2i32 {
                    for z in -2..=2i32 {
                        let civilization = field.civilization([x, y, z]);
                        if civilization.status == CivilizationStatusType::Living
                            && civilization.stance == StanceType::Neutral
                        {
                            return civilization.id;
                        }
                    }
                }
            }
        }
        panic!("no seed in [0, 64) holds a living Neutral civilization");
    }

    /// A player ship with 2,000 cr and 4 Iron ore docked at a living station
    /// of `civilization` that posts jobs, holding that station's procurement
    /// of 4 Iron ore for 146 cr. The civilization stands one hand-in short of
    /// a step.
    fn hand_in_fixture(app: &mut App, civilization: CivilizationId) -> (Entity, Job) {
        let items = test_items();
        let iron = ItemDesignId::from(ItemRoleType::IronOre.id());
        let station_id = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let job = Job {
            id: JobId {
                station: station_id.clone(),
                slot: 0,
                posting: 0,
            },
            kind: JobType::Procure {
                item: iron.clone(),
                count: 4,
            },
            target: station_id.clone(),
            reward_cr: 146,
        };
        let world = app.world_mut();
        world.resource_mut::<Jobs>().accept(job.clone());
        let mut stances = world.resource_mut::<CivilizationStances>();
        for _ in 1..MERIT_STEP {
            assert_eq!(
                record_merit(&mut stances, civilization, 0.0),
                None,
                "the fixture stops one merit short of a step"
            );
        }
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                ShipDesignSections::default(),
                ShipCredits(2_000),
                ShipInventoryReports::default(),
                ShipInventory::new(&items, 100_000, [(iron, 4)]),
            ))
            .id();
        let station = world
            .spawn((
                StationMarker,
                station_id,
                StationServices(vec![StationServiceType::Jobs]),
                StationBuilds(Vec::new()),
                JobBoard(Vec::new()),
                TraderMarket {
                    cluster: ClusterType::RockOnly,
                    civilization,
                    role_preference: [1.0; 4],
                    target: BTreeMap::new(),
                },
                ShipCredits(10_000),
                ShipInventoryReports::default(),
            ))
            .id();
        let connection = world
            .spawn(DockingConnection {
                first_ship: player,
                first_section: Entity::PLACEHOLDER,
                second_ship: station,
                second_section: Entity::PLACEHOLDER,
                helm: DockedHelmType::Neutral,
                measurement_fault: false,
            })
            .id();
        world.entity_mut(player).insert(DockedShip {
            connection,
            helm: Quat::IDENTITY,
            drives: false,
        });
        (player, job)
    }

    fn notice(app: &App) -> Option<(ActionOutcomeType, String)> {
        app.world()
            .resource::<JobsRuntime>()
            .notice
            .shown()
            .map(|(outcome, note)| (outcome, note.to_string()))
    }

    /// A hand-in that steps the station's civilization posts one Done notice
    /// with the payout, the balance after and the step, and clicks once; the
    /// frame after is silent.
    #[test]
    fn a_hand_in_that_steps_a_stance_shows_payout_balance_and_stance_on_one_done_notice_and_clicks_once(
    ) {
        let civilization = neutral_civilization();
        let mut app = jobs_app();
        let (player, job) = hand_in_fixture(&mut app, civilization);

        app.world_mut()
            .write_message(JobHandInCommand { id: job.id.clone() });
        app.update();
        assert_eq!(
            notice(&app),
            Some((
                ActionOutcomeType::Done,
                format!(
                    "handed in Bring 4 Iron ore for 146 cr; balance 2,146 cr; {} now holds you ally",
                    civilization.name()
                )
            ))
        );
        assert_eq!(
            app.world().get::<ShipCredits>(player),
            Some(&ShipCredits(2_146))
        );
        assert!(app.world().resource::<Jobs>().is_completed(&job.id));
        assert_eq!(take_cues(&mut app), [UiSfx::MenuSelect]);
        app.update();
        assert!(take_cues(&mut app).is_empty(), "the cue plays once");
    }

    /// An accept while undocked posts one Refused notice with the reason and
    /// buzzes once; the frame after is silent.
    #[test]
    fn a_refused_accept_posts_a_refusal_notice_and_buzzes_once() {
        let mut app = jobs_app();
        let id = JobId {
            station: GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0),
            slot: 0,
            posting: 0,
        };

        app.world_mut().write_message(JobAcceptCommand { id });
        app.update();
        assert_eq!(
            notice(&app),
            Some((
                ActionOutcomeType::Refused,
                accept_refusal_text(&JobAcceptRefusalType::NotDocked)
            ))
        );
        assert_eq!(take_cues(&mut app), [UiSfx::EditorDeny]);
        app.update();
        assert!(take_cues(&mut app).is_empty(), "the cue plays once");
    }

    /// A drop of a job the player no longer holds posts one Refused notice,
    /// buzzes once and changes no record.
    #[test]
    fn a_drop_of_a_job_no_longer_held_is_refused_and_changes_nothing() {
        let mut app = jobs_app();
        let id = JobId {
            station: GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0),
            slot: 0,
            posting: 0,
        };

        app.world_mut()
            .write_message(JobDropCommand { id: id.clone() });
        app.update();
        assert_eq!(
            notice(&app),
            Some((
                ActionOutcomeType::Refused,
                "drop: the job is no longer held".to_string()
            ))
        );
        assert!(!app.world().resource::<Jobs>().is_dropped(&id));
        assert_eq!(take_cues(&mut app), [UiSfx::EditorDeny]);
        app.update();
        assert!(take_cues(&mut app).is_empty(), "the cue plays once");
    }

    /// A held row's `X` only asks and Back only cancels the ask: the job
    /// stays held. Drop after `X` drops it with no payout and takes its
    /// caller out of the world.
    #[test]
    fn only_drop_after_x_drops_a_held_job_and_back_keeps_it() {
        use crate::pointer_rig::{click_at, pane_pointer_rig, settle};

        let mut rig = pane_pointer_rig();
        let app = &mut rig.app;
        app.insert_resource(test_items());
        app.insert_resource(GamePriceTable(price_table()));
        app.init_resource::<JobsRuntime>();
        app.init_resource::<Chart>();
        app.init_resource::<Jobs>();
        app.init_resource::<JobPostings>();
        app.init_resource::<WorldClock>();
        app.init_resource::<CivilizationStances>();
        app.add_message::<JobAcceptCommand>();
        app.add_message::<JobHandInCommand>();
        app.add_message::<JobDropCommand>();
        app.add_message::<JobOfferRefreshCommand>();
        app.add_systems(Update, apply_job_commands);
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let caller = GeneratedBodyId::new(SectorCoord::ORIGIN, "job_caller", 0);
        let job = Job {
            id: JobId {
                station: station.clone(),
                slot: 0,
                posting: 0,
            },
            kind: JobType::Distress {
                caller: caller.clone(),
            },
            target: station,
            reward_cr: 146,
        };
        app.world_mut().resource_mut::<Jobs>().accept(job.clone());
        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                ShipCredits(2_000),
            ))
            .id();
        let standing = app.world_mut().spawn(caller).id();
        // The three steps a held row shows across its confirm, side by side.
        let steps = [
            ("JobDrop-0", "X", JobAskStepType::Ask),
            ("JobDropBack-0", "Back", JobAskStepType::Back),
            ("JobDropConfirm-0", "Drop", JobAskStepType::Confirm),
        ];
        let content = rig.content_root;
        app.world_mut()
            .commands()
            .entity(content)
            .with_children(|content| {
                content.spawn(column_panel()).with_children(|panel| {
                    for (name, label, step) in steps {
                        drop_step_button(
                            panel,
                            ACTION_PX,
                            name.to_string(),
                            ButtonSpec::new(label).fit(),
                            JobDropButton {
                                step,
                                id: job.id.clone(),
                            },
                            None,
                        );
                    }
                });
            });
        settle(&mut rig.app);
        let click = |rig: &mut crate::pointer_rig::PanePointerRig, name: &str| {
            let centre = rig
                .app
                .world_mut()
                .query::<(&Name, &ComputedNode, &UiGlobalTransform)>()
                .iter(rig.app.world())
                .find(|(each, ..)| each.as_str() == name)
                .map(|(_, node, xf)| {
                    assert!(node.size().min_element() > 0.0, "{name} has no area");
                    xf.translation
                })
                .expect("the button is laid out");
            click_at(rig, centre);
        };
        let kept = |rig: &crate::pointer_rig::PanePointerRig| {
            let world = rig.app.world();
            world.resource::<Jobs>().holds(&job.id)
                && world.get::<ShipCredits>(player) == Some(&ShipCredits(2_000))
                && world.get_entity(standing).is_ok()
                && notice(&rig.app).is_none()
        };
        let asking = |rig: &crate::pointer_rig::PanePointerRig| {
            rig.app
                .world()
                .resource::<JobsRuntime>()
                .confirm_drop
                .as_ref()
                == Some(&job.id)
        };

        click(&mut rig, "JobDrop-0");
        assert!(asking(&rig), "X asks for a confirm");
        assert!(kept(&rig), "X keeps the job, its caller and the balance");
        click(&mut rig, "JobDropBack-0");
        assert!(!asking(&rig), "Back cancels the ask");
        assert!(kept(&rig), "Back keeps the job, its caller and the balance");

        click(&mut rig, "JobDrop-0");
        click(&mut rig, "JobDropConfirm-0");
        let world = rig.app.world();
        let jobs = world.resource::<Jobs>();
        assert!(!asking(&rig), "Drop ends the ask");
        assert!(jobs.is_dropped(&job.id) && !jobs.holds(&job.id));
        assert!(!jobs.is_completed(&job.id), "a drop is no hand-in");
        assert_eq!(
            world.get::<ShipCredits>(player),
            Some(&ShipCredits(2_000)),
            "a drop pays nothing"
        );
        assert!(
            world.get_entity(standing).is_err(),
            "the caller the accept put in the world is gone"
        );
        assert_eq!(
            notice(&rig.app),
            Some((
                ActionOutcomeType::Done,
                format!("dropped: {}", job.title(&test_items()))
            ))
        );
    }

    /// A player ship holding a Distress job at a fresh station's board slot
    /// 0, with `own_cr` credits and the reports queue a credit change needs;
    /// docked there with `station_cr` credits when `docked`, else free in
    /// space. Distress asks nothing of the hold, so a drop fixture needs no
    /// goods.
    fn drop_fixture(
        app: &mut App,
        own_cr: u32,
        station_cr: u32,
        docked: bool,
    ) -> (Entity, Option<Entity>, Job) {
        let station_id = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let caller = GeneratedBodyId::new(SectorCoord::ORIGIN, "job_caller", 0);
        let job = Job {
            id: JobId {
                station: station_id.clone(),
                slot: 0,
                posting: 0,
            },
            kind: JobType::Distress { caller },
            target: station_id.clone(),
            reward_cr: 146,
        };
        let world = app.world_mut();
        world.resource_mut::<Jobs>().accept(job.clone());
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                ShipDesignSections::default(),
                ShipCredits(own_cr),
                ShipInventoryReports::default(),
            ))
            .id();
        if !docked {
            return (player, None, job);
        }
        let station = world
            .spawn((
                StationMarker,
                station_id,
                StationServices(vec![StationServiceType::Jobs]),
                StationBuilds(Vec::new()),
                JobBoard(Vec::new()),
                ShipCredits(station_cr),
                ShipInventoryReports::default(),
            ))
            .id();
        let connection = world
            .spawn(DockingConnection {
                first_ship: player,
                first_section: Entity::PLACEHOLDER,
                second_ship: station,
                second_section: Entity::PLACEHOLDER,
                helm: DockedHelmType::Neutral,
                measurement_fault: false,
            })
            .id();
        world.entity_mut(player).insert(DockedShip {
            connection,
            helm: Quat::IDENTITY,
            drives: false,
        });
        (player, Some(station), job)
    }

    /// A player ship with `own_cr` credits docked at a fresh station with
    /// `station_cr` credits that posts jobs, holding no job: the slot the
    /// refresh tests act on names an unheld Distress offer there.
    fn offer_fixture(app: &mut App, own_cr: u32, station_cr: u32) -> (Entity, Entity, Job) {
        let station_id = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let caller = GeneratedBodyId::new(SectorCoord::ORIGIN, "job_caller", 0);
        let job = Job {
            id: JobId {
                station: station_id.clone(),
                slot: 0,
                posting: 0,
            },
            kind: JobType::Distress { caller },
            target: station_id.clone(),
            reward_cr: 146,
        };
        let world = app.world_mut();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                ShipDesignSections::default(),
                ShipCredits(own_cr),
                ShipInventoryReports::default(),
            ))
            .id();
        let station = world
            .spawn((
                StationMarker,
                station_id,
                StationServices(vec![StationServiceType::Jobs]),
                StationBuilds(Vec::new()),
                JobBoard(Vec::new()),
                ShipCredits(station_cr),
                ShipInventoryReports::default(),
            ))
            .id();
        let connection = world
            .spawn(DockingConnection {
                first_ship: player,
                first_section: Entity::PLACEHOLDER,
                second_ship: station,
                second_section: Entity::PLACEHOLDER,
                helm: DockedHelmType::Neutral,
                measurement_fault: false,
            })
            .id();
        world.entity_mut(player).insert(DockedShip {
            connection,
            helm: Quat::IDENTITY,
            drives: false,
        });
        (player, station, job)
    }

    /// Post `job` at its station's slot 0 with two filler slots, so
    /// [`JobPostings::slot`] reads its posting current.
    fn post_current(app: &mut App, job: &Job) {
        let iron = ItemDesignId::from(ItemRoleType::IronOre.id());
        let filler = |slot: u32| Job {
            id: JobId {
                station: job.id.station.clone(),
                slot,
                posting: 0,
            },
            kind: JobType::Procure {
                item: iron.clone(),
                count: 1,
            },
            target: job.id.station.clone(),
            reward_cr: 10,
        };
        app.world_mut()
            .resource_mut::<JobPostings>()
            .post(vec![job.clone(), filler(1), filler(2)], 0.0);
    }

    /// An offer refresh of a current, unheld board offer docked at the
    /// issuing station moves the fee from the player to the station in one
    /// frame, queues a `JobRefresh` report on each side and marks the slot
    /// for a refresh, naming its title (not held, since it was never
    /// accepted) on the Done notice.
    #[test]
    fn an_offer_refresh_moves_the_fee_atomically_with_both_reports_and_marks_the_slot() {
        let mut app = jobs_app();
        let (player, station, job) = offer_fixture(&mut app, 2_000, 10_000);
        post_current(&mut app, &job);

        app.world_mut()
            .write_message(JobOfferRefreshCommand { id: job.id.clone() });
        app.update();

        let world = app.world();
        assert_eq!(world.get::<ShipCredits>(player), Some(&ShipCredits(1_900)));
        assert_eq!(
            world.get::<ShipCredits>(station),
            Some(&ShipCredits(10_100))
        );
        assert!(matches!(
            world.get::<ShipInventoryReports>(player).unwrap().0.last(),
            Some(ShipInventoryReport::CreditsChanged {
                current_cr: 1_900,
                cause: CreditChangeCauseType::JobRefresh,
                ..
            })
        ));
        assert!(matches!(
            world.get::<ShipInventoryReports>(station).unwrap().0.last(),
            Some(ShipInventoryReport::CreditsChanged {
                current_cr: 10_100,
                cause: CreditChangeCauseType::JobRefresh,
                ..
            })
        ));
        assert!(
            world
                .resource::<JobPostings>()
                .slot(&job.id)
                .unwrap()
                .paid_refresh_requested,
            "an offer refresh marks the slot for a refresh"
        );
        assert!(
            !world.resource::<Jobs>().holds(&job.id),
            "the offer was never accepted"
        );
        assert_eq!(
            notice(&app),
            Some((
                ActionOutcomeType::Done,
                format!(
                    "deleted {} for 100 cr; balance 1,900 cr",
                    job.title(&test_items())
                )
            ))
        );
    }

    /// Two refresh commands for the same offer the same frame move the fee
    /// once: the first marks the slot, so the second reads it as stale and
    /// is refused rather than paying again. Input order is accepted either
    /// way, since both read the slot fresh and only the first to run finds
    /// it unmarked.
    #[test]
    fn a_second_offer_refresh_the_same_frame_refuses_and_never_double_charges() {
        let mut app = jobs_app();
        let (player, station, job) = offer_fixture(&mut app, 2_000, 10_000);
        post_current(&mut app, &job);

        app.world_mut()
            .write_message(JobOfferRefreshCommand { id: job.id.clone() });
        app.world_mut()
            .write_message(JobOfferRefreshCommand { id: job.id.clone() });
        app.update();

        let world = app.world();
        assert_eq!(world.get::<ShipCredits>(player), Some(&ShipCredits(1_900)));
        assert_eq!(
            world.get::<ShipCredits>(station),
            Some(&ShipCredits(10_100))
        );
        assert_eq!(
            world
                .get::<ShipInventoryReports>(player)
                .unwrap()
                .0
                .iter()
                .filter(|report| matches!(
                    report,
                    ShipInventoryReport::CreditsChanged {
                        cause: CreditChangeCauseType::JobRefresh,
                        ..
                    }
                ))
                .count(),
            1,
            "only the first command's fee is reported"
        );
        assert_eq!(
            world
                .get::<ShipInventoryReports>(station)
                .unwrap()
                .0
                .iter()
                .filter(|report| matches!(
                    report,
                    ShipInventoryReport::CreditsChanged {
                        cause: CreditChangeCauseType::JobRefresh,
                        ..
                    }
                ))
                .count(),
            1,
            "only the first command's fee is reported"
        );
        assert_eq!(
            notice(&app),
            Some((
                ActionOutcomeType::Refused,
                "delete: this posting renewed already".to_string()
            )),
            "the second command reads the slot the first already marked"
        );
    }

    /// An offer refresh refuses, moving no credit, both when its slot never
    /// posted that id (stale, since `JobPostings::slot` reads nothing
    /// current) and when the balance is short of the fee.
    #[test]
    fn an_offer_refresh_refuses_when_stale_or_short_of_the_fee() {
        let mut app = jobs_app();
        let (player, station, job) = offer_fixture(&mut app, 2_000, 10_000);

        app.world_mut()
            .write_message(JobOfferRefreshCommand { id: job.id.clone() });
        app.update();
        {
            let world = app.world();
            assert_eq!(world.get::<ShipCredits>(player), Some(&ShipCredits(2_000)));
            assert_eq!(
                world.get::<ShipCredits>(station),
                Some(&ShipCredits(10_000))
            );
            assert_eq!(
                notice(&app),
                Some((
                    ActionOutcomeType::Refused,
                    "delete: this posting renewed already".to_string()
                ))
            );
        }

        let mut app = jobs_app();
        let (player, station, job) = offer_fixture(&mut app, 99, 10_000);
        post_current(&mut app, &job);

        app.world_mut()
            .write_message(JobOfferRefreshCommand { id: job.id.clone() });
        app.update();
        let world = app.world();
        assert_eq!(world.get::<ShipCredits>(player), Some(&ShipCredits(99)));
        assert_eq!(
            world.get::<ShipCredits>(station),
            Some(&ShipCredits(10_000))
        );
        assert_eq!(
            notice(&app),
            Some((
                ActionOutcomeType::Refused,
                "delete: it costs 100 cr, the balance holds 99 cr".to_string()
            ))
        );
    }

    /// A held drop is free wherever it is docked, including at the job's
    /// own issuing station: only a board offer's refresh carries a fee.
    #[test]
    fn a_held_drop_is_free_even_docked_at_the_issuing_station() {
        let mut app = jobs_app();
        let (player, station, job) = drop_fixture(&mut app, 2_000, 10_000, true);
        let station = station.expect("drop_fixture docks when asked");

        app.world_mut()
            .write_message(JobDropCommand { id: job.id.clone() });
        app.update();

        let world = app.world();
        assert_eq!(world.get::<ShipCredits>(player), Some(&ShipCredits(2_000)));
        assert_eq!(
            world.get::<ShipCredits>(station),
            Some(&ShipCredits(10_000))
        );
        let jobs = world.resource::<Jobs>();
        assert!(jobs.is_dropped(&job.id) && !jobs.holds(&job.id));
        assert_eq!(
            notice(&app),
            Some((
                ActionOutcomeType::Done,
                format!("dropped: {}", job.title(&test_items()))
            ))
        );
    }

    /// A board row's `X` only asks and Back only cancels the ask: the offer
    /// stays untouched. Refresh after `X` pays the fee and marks the slot.
    #[test]
    fn only_refresh_after_x_pays_the_fee_for_a_board_offer_and_back_keeps_it_untouched() {
        use crate::pointer_rig::{click_at, pane_pointer_rig, settle};

        let mut rig = pane_pointer_rig();
        let app = &mut rig.app;
        app.insert_resource(test_items());
        app.insert_resource(GamePriceTable(price_table()));
        app.init_resource::<JobsRuntime>();
        app.init_resource::<Chart>();
        app.init_resource::<Jobs>();
        app.init_resource::<JobPostings>();
        app.init_resource::<WorldClock>();
        app.init_resource::<CivilizationStances>();
        app.add_message::<JobAcceptCommand>();
        app.add_message::<JobHandInCommand>();
        app.add_message::<JobDropCommand>();
        app.add_message::<JobOfferRefreshCommand>();
        app.add_systems(Update, apply_job_commands);
        let (player, station, job) = offer_fixture(app, 2_000, 10_000);
        post_current(app, &job);

        // The three steps a board row shows across its confirm, side by side.
        let steps = [
            ("JobRefresh-0", "X", JobAskStepType::Ask),
            ("JobRefreshBack-0", "Back", JobAskStepType::Back),
            ("JobRefreshConfirm-0", "Delete", JobAskStepType::Confirm),
        ];
        let content = rig.content_root;
        app.world_mut()
            .commands()
            .entity(content)
            .with_children(|content| {
                content.spawn(column_panel()).with_children(|panel| {
                    for (name, label, step) in steps {
                        refresh_step_button(
                            panel,
                            ACTION_PX,
                            name.to_string(),
                            ButtonSpec::new(label).fit(),
                            JobRefreshButton {
                                step,
                                id: job.id.clone(),
                            },
                            None,
                        );
                    }
                });
            });
        settle(&mut rig.app);
        let click = |rig: &mut crate::pointer_rig::PanePointerRig, name: &str| {
            let centre = rig
                .app
                .world_mut()
                .query::<(&Name, &ComputedNode, &UiGlobalTransform)>()
                .iter(rig.app.world())
                .find(|(each, ..)| each.as_str() == name)
                .map(|(_, node, xf)| {
                    assert!(node.size().min_element() > 0.0, "{name} has no area");
                    xf.translation
                })
                .expect("the button is laid out");
            click_at(rig, centre);
        };
        let untouched = |rig: &crate::pointer_rig::PanePointerRig| {
            let world = rig.app.world();
            world.get::<ShipCredits>(player) == Some(&ShipCredits(2_000))
                && world.get::<ShipCredits>(station) == Some(&ShipCredits(10_000))
                && !world
                    .resource::<JobPostings>()
                    .slot(&job.id)
                    .unwrap()
                    .paid_refresh_requested
                && notice(&rig.app).is_none()
        };
        let asking = |rig: &crate::pointer_rig::PanePointerRig| {
            rig.app
                .world()
                .resource::<JobsRuntime>()
                .confirm_refresh
                .as_ref()
                == Some(&job.id)
        };

        click(&mut rig, "JobRefresh-0");
        assert!(asking(&rig), "X asks for a confirm");
        assert!(untouched(&rig), "X moves nothing");
        click(&mut rig, "JobRefreshBack-0");
        assert!(!asking(&rig), "Back cancels the ask");
        assert!(untouched(&rig), "Back moves nothing");

        click(&mut rig, "JobRefresh-0");
        click(&mut rig, "JobRefreshConfirm-0");
        let world = rig.app.world();
        assert!(
            world.resource::<JobsRuntime>().confirm_refresh.is_none(),
            "Confirm ends the ask"
        );
        assert_eq!(world.get::<ShipCredits>(player), Some(&ShipCredits(1_900)));
        assert_eq!(
            world.get::<ShipCredits>(station),
            Some(&ShipCredits(10_100))
        );
        assert!(
            world
                .resource::<JobPostings>()
                .slot(&job.id)
                .unwrap()
                .paid_refresh_requested
        );
        assert_eq!(
            notice(&rig.app),
            Some((
                ActionOutcomeType::Done,
                format!(
                    "deleted {} for 100 cr; balance 1,900 cr",
                    job.title(&test_items())
                )
            ))
        );
    }

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
    fn a_short_hold_hand_in_refusal_names_the_item_not_its_id() {
        let item = ItemDesignId::from("CarbonOre");
        let items = GameItems::new([ItemDesign {
            id: item.clone(),
            name: "Carbon ore".to_string(),
            about: "Ore.".to_string(),
            category: ItemCategoryType::Raw,
            mass_g: 1_000,
            price_lot: 1,
            ask_cr: 10,
            bid_cr: 8,
        }]);
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let job = Job {
            id: JobId {
                station: station.clone(),
                slot: 0,
                posting: 0,
            },
            kind: JobType::Procure {
                item: item.clone(),
                count: 4,
            },
            target: station,
            reward_cr: 100,
        };
        let refusal = JobHandInRefusalType::ShortItems {
            item,
            need: 4,
            have: 2,
        };
        assert_eq!(
            hand_in_refusal_text(&job, &refusal, &Chart::default(), &items),
            "the hold carries 2 of 4 Carbon ore"
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

    #[test]
    fn a_click_on_a_job_row_selects_its_job() {
        use crate::pointer_rig::{click_at, pane_pointer_rig, settle};

        let mut rig = pane_pointer_rig();
        rig.app.init_resource::<JobsRuntime>();
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let job = |slot| JobId {
            station: station.clone(),
            slot,
            posting: 0,
        };
        // One row as each list names it: both lists spawn rows through
        // `spawn_row`, so the posted and the held row take the same path.
        let rows = [("JobRowPosted-0", job(0)), ("JobRowHeld-0", job(1))];
        let content = rig.content_root;
        rig.app
            .world_mut()
            .commands()
            .entity(content)
            .with_children(|content| {
                content.spawn(column_panel()).with_children(|panel| {
                    for (name, id) in &rows {
                        spawn_row(panel, id, name.to_string(), false);
                    }
                });
            });
        settle(&mut rig.app);

        for (name, id) in &rows {
            let centre = rig
                .app
                .world_mut()
                .query::<(&Name, &ComputedNode, &UiGlobalTransform)>()
                .iter(rig.app.world())
                .find(|(each, ..)| each.as_str() == *name)
                .map(|(_, node, xf)| {
                    assert!(node.size().min_element() > 0.0, "{name} has no area");
                    xf.translation
                })
                .expect("the row is laid out");
            click_at(&mut rig, centre);
            assert_eq!(
                rig.app.world().resource::<JobsRuntime>().selected.as_ref(),
                Some(id),
                "a click on {name} selects its job"
            );
        }
    }
}
