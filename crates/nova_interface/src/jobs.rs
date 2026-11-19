//! The TAB interface's Jobs pane: the jobs the player holds, always, and
//! the docked station's board.
//!
//! The pane is one row: a column of panels and an inspector beside it.
//! Docked at a living station that offers Jobs, the column starts with the
//! station's Board: one row per offer the player neither holds nor completed
//! and whose window has not passed, as kind icon | title | reward | time-left
//! meter with its minutes | Accept `X`. Away from such a station the board is
//! not drawn; the pane head's mode tags say where the player is. Every offer
//! asks something of the hull ([`plan_job_hull_fit`]): the starting station's
//! board ([`StarterBoardMarker`]) leaves out what the hull cannot do; every
//! other board lists it with Accept disabled and one tag word for the part
//! the hull lacks, as it tags an offer whose target is gone. Under it, docked
//! or not, "Your jobs" lists every held job as kind icon | title | `have /
//! need` meter | hand-in | Hand in `X`. The hand-in column reads HAND IN HERE
//! docked at the job's target, else blank; Hand in is hidden elsewhere, and
//! disabled with the short count painted Danger while the work or the hold
//! is short. Under the held table head, one row reads the start role's goal:
//! its title, its next unmet line ([`open_objectives`]) and a `have / need`
//! meter, or the play time the goal was met; a role with no goal has no row.
//! An empty list draws nothing. Clicking a job row selects it and the
//! inspector reads its task, its target, its hand-in station, its reward, its
//! time left or progress and a Show on Map button, which opens the Map's
//! chart view on the job's mark ([`MapRuntime::chart_show`]).
//!
//! A click on Accept or Hand in raises a [`JobAcceptCommand`] or a
//! [`JobHandInCommand`]; [`apply_job_commands`] plans it with
//! [`plan_job_accept`] or [`plan_job_hand_in`], moves the goods and the
//! credits, moves the job in [`Jobs`] and raises [`JobAccepted`] so the open
//! world puts the job's bodies in place. The result plays one sound and the
//! changed balance and lists show it; the buttons are silent on press. A
//! paid hand-in plays the payout cue in place of the click, and the pane
//! head counts the balance up with the gain beside it ([`BalanceCountUp`]). A verb
//! that cannot work is hidden or disabled by its [`VerbGate`].
//!
//! A held row's `X` drops nothing: it asks. The row then reads `Drop?
//! <title>` and swaps Hand in and `X` for Back and Drop. Back leaves the
//! job held either way; Drop raises a [`JobDropCommand`], and
//! [`apply_job_commands`] drops the job ([`Jobs::drop`]) with no payout,
//! whatever station it is docked at, and takes out the bodies its accept
//! put in the world, as a loss does. A drop of a job no longer held is
//! refused with the deny sound.
//!
//! A board row's offer is not held: docked at its issuing station, it
//! carries a compact `X` of its own. Pressing it asks a paid delete instead
//! of a drop: the row reads `Delete? <title>` with the fee beside the
//! credit glyph, Danger while the balance is short, and swaps Accept and `X`
//! for Back and Delete. Delete raises a
//! [`JobOfferRefreshCommand`]; [`apply_job_commands`] revalidates the slot
//! is still current, unheld, unexpired and docked before it moves the fee
//! ([`JOB_OFFER_REFRESH_FEE_CR`], [`plan_job_offer_refresh`]) and marks the
//! slot for a fresh offer ([`JobPostings::request_paid_refresh`]), same as
//! a hand-in skips the wait. A slot already marked counts as stale, so a
//! second [`JobOfferRefreshCommand`] for the same id, the same frame or the
//! next, refuses instead of paying the fee twice. A refusal, including one
//! an accept the same frame raises by holding the job first, changes
//! nothing and plays the deny sound.
//!
//! In space, away from a station that posts jobs, a standing hail offer
//! ([`Encounters::offer`]) is the Board's one row, with the same cells: its
//! window counts down from [`HAIL_OFFER_SECONDS`], Accept is gated by
//! [`plan_hail_accept`] with the board's tags, and its `X` declines at one
//! click, with no fee and no confirm. Accept and `X` raise a
//! [`HailAnswerCommand`], which the open world's encounter director
//! answers; its [`HailAnswered`] plays the one cue. A hail job, offered or
//! held, names its caller as target and hand-in: it is done and paid on the
//! dock with that hull, so its held row never shows Hand in.
//!
//! Touch this module when changing what the lists show, what gates an
//! offer, or what a hand-in moves.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Activate, Button},
};
use nova_events::prelude::{
    CreditChangeCauseType, GeneratedBodyId, JobDropCauseType, Meters, SectorCoord,
};
use nova_gameplay::prelude::*;
use nova_sound::prelude::*;
use nova_ui::{
    prelude::*,
    theme::{UiColorType, BORDER_W},
    widget::ThemedBorder,
};
use nova_world_base::prelude::{record_merit, PlayerHull};

use crate::{
    icons::{icon_node, ChartIconType, InterfaceIcons, JobIconType},
    map::{job_cell, MapChartMarkType, MapRuntime},
    pane::{
        gated_verb, spawn_credit_figure, view_split, ActionCue, ActionOutcomeType, BalanceCountUp,
        InterfacePaneType,
    },
    ship::StationSite,
};

/// Glob-import surface: `use nova_interface::jobs::prelude::*`.
pub mod prelude {
    pub use super::{
        JobAcceptCommand, JobHandInCommand, JobsRuntime, JOBS_BOARD, JOBS_HELD, JOBS_MILESTONE,
        JOBS_PANE, JOB_ACCEPT_PREFIX, JOB_HAIL_DECLINE, JOB_HAND_IN_PREFIX, JOB_SHOW_ON_MAP,
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
/// The `Name` of a hail row's `X` button, which declines the offer.
pub const JOB_HAIL_DECLINE: &str = "JobHailDecline";
/// The `Name` of the inspector's Show on Map button.
pub const JOB_SHOW_ON_MAP: &str = "JobShowOnMap";

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
/// that waits for its confirm and the result cue. Kept while the interface
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
    /// The last command result's sound.
    pub cue: ActionCue,
}

impl JobsRuntime {
    /// Cancel a drop or refresh confirm, as its Back button does. Returns
    /// whether one was up.
    pub(crate) fn clear_ask(&mut self) -> bool {
        let asking = self.confirm_drop.is_some() || self.confirm_refresh.is_some();
        self.confirm_drop = None;
        self.confirm_refresh = None;
        asking
    }
}

/// The pane's row of panels, rebuilt by [`update_jobs_pane`].
#[derive(Component)]
pub(crate) struct JobsPaneMarker;

/// Which of the pane's three scrolling viewports a node is, so a rebuild
/// carries each one's offset to its successor.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum JobsScrollType {
    /// The board panel's rows.
    Board,
    /// The held panel's rows.
    Held,
    /// The inspector.
    Inspector,
}

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
    /// The standing hail offer: Accept, in space.
    Hail,
}

/// A hail row's `X`: it declines the standing offer.
#[derive(Component, Clone, Copy, Debug)]
struct HailDeclineButton;

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

/// Build the Jobs pane body: the row of panels.
pub(crate) fn jobs_body(body: &mut ChildSpawnerCommands) {
    body.spawn((JobsPaneMarker, Name::new(JOBS_PANE), view_split()));
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
        JobType::Procure { .. }
        | JobType::Escort { .. }
        | JobType::Distress { .. }
        | JobType::Supply { .. } => true,
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
/// else the wreck, the hauler, the caller or the hull. A hail job's caller
/// reads by its name, `hailer`.
fn target_name(job: &Job, chart: &Chart, hailer: Option<&str>) -> String {
    match &job.kind {
        JobType::Procure { .. } => station_name(chart, &job.target),
        JobType::Recover { wreck_name, .. } => wreck_name.clone(),
        JobType::Escort { .. } => "the hauler".to_string(),
        JobType::Distress { .. } | JobType::Supply { .. } => {
            hailer.unwrap_or("the caller").to_string()
        }
        JobType::Neutralize { hull_name, .. } => hull_name.clone(),
    }
}

/// A job's leg as a list writes it: to the tenth of a kilometre under
/// 10 km (`4.2 km`), to the kilometre from there (`33 km`), so the widest
/// is six characters, and nothing for a job with no leg.
fn range_text(leg_m: u32) -> String {
    let tenths = leg_m.saturating_add(50) / 100;
    match (tenths / 10, tenths % 10) {
        (0, 0) => String::new(),
        (10.., _) => format!("{} km", leg_m.saturating_add(500) / 1_000),
        (km, 0) => format!("{km} km"),
        (km, tenth) => format!("{km}.{tenth} km"),
    }
}

/// A job's threat as a list writes it: the weapon count, and nothing for a
/// job that puts none against the player.
fn threat_text(threat: u32) -> String {
    if threat == 0 {
        String::new()
    } else {
        threat.to_string()
    }
}

/// One term of a reward as the inspector lists it under the total.
#[derive(Clone, Debug, PartialEq)]
struct RewardTermRow {
    /// The term's name, with its measure after it: `Range 12 km`,
    /// `Threat 3`.
    label: String,
    cr: u32,
}

/// The rows the inspector lists under a job's reward: one per term that
/// pays, in the order base, goods, range, threat. A reward of one term
/// lists none: the total already says it.
fn reward_term_rows(job: &Job) -> Vec<RewardTermRow> {
    let goods = if matches!(job.kind, JobType::Escort { .. }) {
        "Cargo"
    } else {
        "Goods"
    };
    let rows: Vec<RewardTermRow> = [
        ("Base".to_string(), job.reward.base_cr),
        (goods.to_string(), job.reward.goods_cr),
        (
            format!("Range {}", range_text(job.leg_m)),
            job.reward.range_cr,
        ),
        (format!("Threat {}", job.threat), job.reward.threat_cr),
    ]
    .into_iter()
    .filter(|(_, cr)| *cr > 0)
    .map(|(label, cr)| RewardTermRow { label, cr })
    .collect();
    if rows.len() < 2 {
        Vec::new()
    } else {
        rows
    }
}

/// How Accept answers a hull that cannot do a job: one word for the part
/// the hull lacks.
fn hull_gate(refusal: &JobHullRefusalType) -> VerbGate {
    match refusal {
        JobHullRefusalType::HoldTooSmall { .. } => VerbGate::Tagged("HOLD"),
        JobHullRefusalType::NoClamp => VerbGate::Tagged("NO DOCK"),
        JobHullRefusalType::NoWeapon => VerbGate::Tagged("NO WEAPON"),
    }
}

/// How Accept answers a refused accept: hidden where the offer cannot be
/// taken, tagged when its target is gone.
fn accept_gate(refusal: &JobAcceptRefusalType) -> VerbGate {
    match refusal {
        JobAcceptRefusalType::NotDocked
        | JobAcceptRefusalType::NotPostedHere
        | JobAcceptRefusalType::AlreadyHeld
        | JobAcceptRefusalType::AlreadyDone
        | JobAcceptRefusalType::Expired => VerbGate::Hidden,
        JobAcceptRefusalType::TargetGone => VerbGate::Tagged("GONE"),
    }
}

/// How Accept answers a refused hail accept: the board's own answers.
fn hail_gate(refusal: &HailAcceptRefusalType) -> VerbGate {
    match refusal {
        HailAcceptRefusalType::Accept(refusal) => accept_gate(refusal),
        HailAcceptRefusalType::Hull(refusal) => hull_gate(refusal),
    }
}

/// How Hand in answers a refused hand-in: hidden away from the hand-in
/// station (HAND IN HERE marks it), disabled while the work or the hold is
/// short (the Progress count shows it), tagged when the station cannot pay.
fn hand_in_gate(refusal: &JobHandInRefusalType) -> VerbGate {
    match refusal {
        JobHandInRefusalType::NotDocked | JobHandInRefusalType::WrongStation { .. } => {
            VerbGate::Hidden
        }
        JobHandInRefusalType::ShortItems { .. }
        | JobHandInRefusalType::NotDone
        | JobHandInRefusalType::CreditOverflow => VerbGate::Disabled,
        JobHandInRefusalType::StationCannotPay { .. } => VerbGate::Tagged("NO FUNDS"),
    }
}

/// How a refresh's Delete answers a refused refresh: hidden where no refresh
/// can happen, disabled while the fee is short (the fee shows red).
fn offer_refresh_gate(refusal: &JobOfferRefreshRefusalType) -> VerbGate {
    match refusal {
        JobOfferRefreshRefusalType::Stale
        | JobOfferRefreshRefusalType::Held
        | JobOfferRefreshRefusalType::Expired
        | JobOfferRefreshRefusalType::NotDocked => VerbGate::Hidden,
        JobOfferRefreshRefusalType::CreditsShort { .. }
        | JobOfferRefreshRefusalType::CreditOverflow => VerbGate::Disabled,
    }
}

/// What a board does with an offer the hull gate judged `fit` and the
/// accept rule judged `accept`: `None` leaves it off a starter board, else it
/// is listed with Accept under the gate. The hull's cause comes first: it is
/// the one the player can act on.
fn board_listing(
    starter: bool,
    fit: Result<(), JobHullRefusalType>,
    accept: Result<(), JobAcceptRefusalType>,
) -> Option<VerbGate> {
    match (starter, fit, accept) {
        (true, Err(_), _) => None,
        (false, Err(refusal), _) => Some(hull_gate(&refusal)),
        (_, Ok(()), Err(refusal)) => Some(accept_gate(&refusal)),
        (_, Ok(()), Ok(())) => Some(VerbGate::Ready),
    }
}

/// What a board row reads where the minutes left go, for an offer with no
/// window: a scenario's posting.
const NO_WINDOW_TEXT: &str = "open";

/// One board row as the pane shows it.
#[derive(Clone, Debug, PartialEq)]
struct PostedRow {
    id: JobId,
    icon: JobIconType,
    title: String,
    /// The job's leg ([`range_text`]), empty for a job with none.
    range: String,
    /// The weapons against the player ([`threat_text`]), empty for none.
    threat: String,
    reward_cr: u64,
    /// The offer's window left: the share and the minutes. A scenario
    /// posting has no window: a full meter and [`NO_WINDOW_TEXT`].
    time_left: (f32, String),
    /// How Accept answers the row.
    gate: VerbGate,
    /// A slot posting: the row shows the `X` that asks a paid refresh. A
    /// visit posting has no slot to renew.
    refreshable: bool,
    /// The standing hail offer: Accept answers it in space and the row
    /// shows the `X` that declines it.
    hail: bool,
    /// Its `X` was pressed: the row asks Refresh or Back.
    confirming: bool,
    /// How the refresh's Delete answers the row while `confirming`.
    refresh_gate: VerbGate,
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
    /// How Hand in answers the row.
    gate: VerbGate,
    /// Its `X` was pressed: the row asks Drop or Back.
    confirming: bool,
}

/// The docked station's board as the pane shows it.
#[derive(Clone, Debug, PartialEq)]
struct BoardView {
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
    /// The reward's terms ([`reward_term_rows`]).
    terms: Vec<RewardTermRow>,
    /// A posted offer's window left: the share and the minutes.
    time_left: Option<(f32, String)>,
    /// A held job's `have / need`.
    progress: Option<(u32, u32)>,
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
    (chart, jobs, postings, clock, edge, progress, record, encounters): (
        Res<Chart>,
        Res<Jobs>,
        Res<JobPostings>,
        Res<WorldClock>,
        Option<Res<ChartSectorEdge>>,
        Res<GoalProgress>,
        Res<GoalRecord>,
        Res<Encounters>,
    ),
    site: StationSite,
    hull: PlayerHull,
    (q_ids, q_targets, q_boards, q_credits, q_holds, q_scroll): (
        Query<&GeneratedBodyId>,
        JobTargets,
        Query<(&JobBoard, Has<StarterBoardMarker>)>,
        Query<&ShipCredits>,
        Query<&ShipInventory>,
        Query<(&JobsScrollType, &ScrollPosition)>,
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
        .or_else(|| hull.entity());
    let hold = player.and_then(|player| q_holds.get(player).ok());
    let own_cr = player.and_then(|player| q_credits.get(player).ok().map(|credits| credits.0));
    let hand_in_partner = partner.as_ref().and_then(|(_, station, station_id)| {
        Some(JobHandInPartner {
            id: station_id.clone(),
            credits_cr: q_credits.get(*station).ok()?.0,
        })
    });
    let hand_in_check = |job: &Job| {
        let (Some(hold), Some(own_cr)) = (hold, own_cr) else {
            return Some(JobHandInRefusalType::NotDocked);
        };
        plan_job_hand_in(
            job,
            hand_in_partner.as_ref(),
            hold,
            own_cr,
            jobs.is_done(&job.id),
        )
        .err()
    };
    let hand_in_gate =
        |job: &Job| hand_in_check(job).map_or(VerbGate::Ready, |refusal| hand_in_gate(&refusal));
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
    // The confirming row reads the fee and the shortfall off the same check
    // order as `apply_job_commands`.
    let offer_refresh_gate = |job: &Job| -> VerbGate {
        let docked = partner.as_ref().filter(|(_, _, id)| *id == job.id.poster);
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
        .map_or(VerbGate::Ready, |refusal| offer_refresh_gate(&refusal))
    };

    let board = partner.as_ref().and_then(|(_, station, station_id)| {
        let (board, starter) = q_boards.get(*station).ok()?;
        let rows: Vec<PostedRow> = board
            .0
            .iter()
            .filter(|job| {
                !jobs.holds(&job.id) && !jobs.is_completed(&job.id) && !jobs.is_dropped(&job.id)
            })
            .filter_map(|job| {
                if !postings.offers(&job.id, rule, now) {
                    return None;
                }
                // A scenario posting has no window to count down.
                let time_left = postings.remaining(&job.id, rule, now).map_or_else(
                    || (1.0, NO_WINDOW_TEXT.to_string()),
                    |remaining| time_left(job, remaining),
                );
                let Some(gate) = board_listing(
                    starter,
                    plan_job_hull_fit(job, &items, fit),
                    plan_job_accept(
                        job,
                        Some(station_id),
                        &jobs,
                        false,
                        target_standing(job, &q_targets),
                    ),
                ) else {
                    return None;
                };
                let confirming = runtime.confirm_refresh.as_ref() == Some(&job.id);
                let refresh_gate = if confirming {
                    offer_refresh_gate(job)
                } else {
                    VerbGate::Ready
                };
                Some(PostedRow {
                    id: job.id.clone(),
                    icon: JobIconType::of(&job.kind),
                    title: job.title(&items),
                    range: range_text(job.leg_m),
                    threat: threat_text(job.threat),
                    reward_cr: job.reward.total_cr().into(),
                    time_left,
                    gate,
                    refreshable: postings.slot(&job.id).is_some(),
                    hail: false,
                    confirming,
                    refresh_gate,
                })
            })
            .collect();
        Some(BoardView { rows })
    });

    // The share and the minutes left of a hail offer's window.
    let hail_time_left = |offer: &EncounterOffer| {
        let remaining = offer.expires_at - now;
        (
            (remaining / HAIL_OFFER_SECONDS) as f32,
            format!("{} min", (remaining.max(0.0) / 60.0).ceil()),
        )
    };
    // Away from a station that posts jobs, the standing hail is the board.
    let hail = encounters.offer.as_ref().filter(|_| partner.is_none());
    let board = board.or_else(|| {
        let offer = hail?;
        let job = &offer.job;
        let gate = plan_hail_accept(job, &jobs, encounters.offer_expired(now), &items, fit)
            .map_or_else(|refusal| hail_gate(&refusal), |()| VerbGate::Ready);
        Some(BoardView {
            rows: vec![PostedRow {
                id: job.id.clone(),
                icon: JobIconType::of(&job.kind),
                title: job.title(&items),
                range: range_text(job.leg_m),
                threat: threat_text(job.threat),
                reward_cr: job.reward.total_cr().into(),
                time_left: hail_time_left(offer),
                gate,
                refreshable: false,
                hail: true,
                confirming: false,
                refresh_gate: VerbGate::Ready,
            }],
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
                gate: hand_in_gate(job),
                confirming,
            }
        })
        .collect();

    // The selected job and whether the player holds it.
    let selected = runtime.selected.as_ref().and_then(|id| {
        jobs.job(id)
            .map(|job| (job, true))
            .or_else(|| {
                let (_, station, _) = partner.as_ref()?;
                let (board, _) = q_boards.get(*station).ok()?;
                board
                    .job(id)
                    .filter(|job| !jobs.is_completed(&job.id) && !jobs.is_dropped(&job.id))
                    .map(|job| (job, false))
            })
            .or_else(|| {
                hail.filter(|offer| offer.job.id == *id)
                    .map(|offer| (&offer.job, false))
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
            target: format!(
                "{}\n{}",
                target_name(job, &chart, encounters.caller_name(&job.id)),
                place(job_cell(job))
            ),
            // A hail job is paid on the dock with its caller.
            hand_in: encounters
                .caller_name(&job.id)
                .map_or_else(|| station_name(&chart, &job.target), str::to_string),
            reward_cr: job.reward.total_cr().into(),
            terms: reward_term_rows(job),
            time_left: postings
                .remaining(&job.id, rule, now)
                .filter(|_| !held)
                .map(|remaining| time_left(job, remaining))
                .or_else(|| {
                    hail.filter(|offer| !held && offer.job.id == job.id)
                        .map(hail_time_left)
                }),
            progress: held.then(|| job.progress(hold, done)),
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
    let offset = |kind: JobsScrollType| {
        q_scroll
            .iter()
            .find_map(|(each, position)| (*each == kind).then(|| position.clone()))
            .unwrap_or_default()
    };
    let (board_offset, held_offset, inspector_offset) = (
        offset(JobsScrollType::Board),
        offset(JobsScrollType::Held),
        offset(JobsScrollType::Inspector),
    );
    commands.entity(pane).despawn_children();
    commands.entity(pane).with_children(|split| {
        split.spawn(column()).with_children(|col| {
            // Away from a station that posts jobs the board is hidden: the
            // pane head's mode tags say where the player is.
            if let Some(board) = &view.board {
                board_panel(col, &icons, board, runtime.selected.as_ref(), board_offset);
            }
            held_panel(col, &icons, &view, runtime.selected.as_ref(), held_offset);
        });
        split
            .spawn((
                nova_ui::prelude::inspector(Overflow::scroll_y()),
                jobs_viewport(JobsScrollType::Inspector, inspector_offset),
            ))
            .with_children(|side| inspector_panel(side, &icons, view.inspector.as_ref()));
    });
    *built = Some(view);
}

/// Width of the range column: the widest leg ([`range_text`]), `9.9 km`.
const RANGE_PX: f32 = 44.0;
/// Width of the threat column: its head, over a count of two digits.
const THREAT_PX: f32 = 40.0;
/// Width of the reward column: the credit mask and six digits, `123,456`.
const REWARD_PX: f32 = 68.0;
/// Width of a held row's hand-in column: the HAND IN HERE badge.
const HAND_IN_PX: f32 = 90.0;
/// Width of the time-left column: its meter and the widest `nn min`.
const TIME_PX: f32 = 92.0;
/// Width of the time-left meter, so its minutes fit beside it.
const TIME_METER_PX: f32 = 40.0;
/// Width of a meter inside a row column, so its text fits beside it.
const ROW_METER_PX: f32 = 72.0;
/// Width of the progress column: a row meter and the widest `nn / nn`.
const PROGRESS_PX: f32 = 130.0;
/// Width of a row's verb column: the widest tag (`NO WEAPON`), Accept and
/// the `X`, [`metrics::GAP`] apart, or Back and Delete.
const VERBS_PX: f32 = 160.0;
/// Width of a row's icon column.
const ICON_PX: f32 = 28.0;
/// Least width of a row's title cell. A row whose fixed cells leave the title
/// less than this wraps them onto a second line. It is what a board row's
/// fixed cells leave of the pane in a 1024 px window, so from that width
/// every row is one line and the title is the cell that gives way.
const TITLE_MIN_PX: f32 = 100.0;
/// The scrollbar's track and its margin, which narrow a list's rows. A table
/// head pads its right side by this much to stay over the same columns.
const SCROLL_GUTTER_PX: f32 = 12.0;
/// A list row's side padding plus its border: where a row's cells start.
const ROW_INSET_PX: f32 = metrics::PAD_X + BORDER_W;

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
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColorType::TextSecondary, 0.35),
        ThemedRadius::control(),
    )
}

/// The scroll offset and kind a list's viewport carries.
fn jobs_viewport(kind: JobsScrollType, offset: ScrollPosition) -> impl Bundle {
    (ScrollViewport, Hovered::default(), offset, kind)
}

/// A panel's rows: a scrolling column of them with its bar beside it. The
/// panel's heading stays outside, fixed.
fn scroll_rows(
    panel: &mut ChildSpawnerCommands,
    kind: JobsScrollType,
    offset: ScrollPosition,
    rows: impl FnOnce(&mut ChildSpawnerCommands),
) {
    panel.spawn(scroll_row()).with_children(|row| {
        row.spawn((
            Node {
                row_gap: px(4),
                ..scroll_column()
            },
            jobs_viewport(kind, offset),
        ))
        .with_children(rows);
        row.spawn(scroll_bar());
    });
}

/// A themed text that never wraps and takes no clicks, so a row's click
/// lands on the row.
fn text_nowrap(value: &str, role: TextRole, color: UiColorType) -> impl Bundle {
    (
        text_toned(value.to_string(), role, color),
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
fn text_cell(width: f32, value: &str, role: TextRole, color: UiColorType) -> impl Bundle {
    (
        Node {
            width: px(width),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
        children![text_nowrap(value, role, color)],
    )
}

/// A one-line label that ends in `...` when its cell is too narrow, and takes
/// no clicks, so a row's click lands on the row.
fn ellipsis_label(value: &str, role: TextRole, color: UiColorType) -> impl Bundle {
    (
        text_toned(value.to_string(), role, color),
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
        EllipsisText(value.to_string()),
        Pickable::IGNORE,
    )
}

/// A row's title cell: it grows, and it never gets narrower than
/// [`TITLE_MIN_PX`], so the row wraps its other cells before the title is
/// lost.
fn title_cell(value: &str, role: TextRole, color: UiColorType) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            flex_basis: px(TITLE_MIN_PX),
            min_width: px(TITLE_MIN_PX),
            overflow: Overflow::clip(),
            ..default()
        },
        Pickable::IGNORE,
        children![ellipsis_label(value, role, color)],
    )
}

/// A confirming row's title cell: the question whole, then the job's title
/// shortened to what is left.
fn ask_title_cell(question: &str, title: &str, role: TextRole) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            flex_basis: px(TITLE_MIN_PX),
            min_width: px(TITLE_MIN_PX),
            column_gap: px(8),
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
            ..default()
        },
        Pickable::IGNORE,
        children![
            (
                text_toned(question.to_string(), role, UiColorType::Accent),
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ),
            (
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                Pickable::IGNORE,
                children![ellipsis_label(title, role, UiColorType::Text)],
            ),
        ],
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
/// it, painted Danger while `short` blocks a verb.
fn meter(width: f32, have: u64, need: u64, short: bool) -> impl Bundle {
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
            (
                text_nowrap(
                    &format!("{have} / {need}"),
                    TextRole::Body,
                    UiColorType::TextBody
                ),
                Short(short),
            ),
        ],
    )
}

/// A table's header line: `(width, label)` per column, width 0 for the
/// column that grows. It wraps the way [`spawn_row`] does, with the same
/// inner width, so each label stays over its cell when a narrow panel moves
/// the fixed cells to a second line.
fn table_head(panel: &mut ChildSpawnerCommands, columns: &[(f32, &str)]) {
    panel
        .spawn(Node {
            padding: UiRect {
                left: px(ROW_INSET_PX),
                right: px(ROW_INSET_PX + SCROLL_GUTTER_PX),
                top: px(2),
                bottom: px(2),
            },
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(metrics::GAP_L),
            row_gap: px(metrics::GAP_S),
            overflow: Overflow::clip(),
            ..control_row(JustifyContent::FlexEnd)
        })
        .with_children(|row| {
            for (width, label) in columns {
                if *width == 0.0 {
                    row.spawn(title_cell(
                        label,
                        TextRole::Label,
                        UiColorType::TextSecondary,
                    ));
                } else {
                    row.spawn(text_cell(
                        *width,
                        label,
                        TextRole::Label,
                        UiColorType::TextSecondary,
                    ));
                }
            }
        });
}

/// A clickable row of a list, lit while it is the selection.
fn spawn_row<'a>(
    panel: &'a mut ChildSpawnerCommands,
    id: &JobId,
    name: String,
    selected: bool,
) -> EntityCommands<'a> {
    let mut row = panel.spawn((
        Name::new(name),
        JobRow(id.clone()),
        tool_list_row(),
        Button,
        Hovered::default(),
        // The selected row again changes nothing, so the handler cues.
        ButtonCueType::Owned,
    ));
    row.observe(on_job_row);
    if selected {
        row.insert(Selected);
    }
    row
}

/// A row's verbs: [`VERBS_PX`] wide, gathered at its end, so every row's
/// columns line up under the table head.
fn verbs_cell() -> impl Bundle {
    (
        Node {
            width: px(VERBS_PX),
            ..verb_row(RowAlign::End)
        },
        Pickable::IGNORE,
    )
}

/// A row's action verb under `gate`: Accept on the board, Hand in on a held
/// job. Plain, since every row has one.
fn action_button(
    verbs: &mut ChildSpawnerCommands,
    name: String,
    label: &str,
    button: JobActionButton,
    gate: VerbGate,
) {
    gated_verb(verbs, name, ButtonSpec::control(label), gate)
        .insert(button)
        .observe(on_job_button);
}

/// A row's `X`: a square ghost verb with the job-drop-cancel mark. Bevy UI
/// cannot draw the authored SVG directly, so the mark is an
/// AssetServer-loaded raster child tinted in the text ink.
fn ask_button<'a>(
    verbs: &'a mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    name: String,
    marker: impl Bundle,
) -> EntityCommands<'a> {
    let mut ask = verbs.spawn((
        Name::new(name),
        button(ButtonSpec::control("").ghost()),
        marker,
        ButtonCueType::Owned,
    ));
    ask.with_children(|b| {
        b.spawn(icon_node(icons.job_drop_cancel(), UiColorType::Text, 14.0));
    });
    ask
}

/// A held row's Drop or Back step.
fn drop_step_button(
    verbs: &mut ChildSpawnerCommands,
    name: String,
    spec: ButtonSpec,
    marker: JobDropButton,
) {
    verbs
        .spawn((Name::new(name), button(spec), ButtonCueType::Owned, marker))
        .observe(on_job_drop_button);
}

/// A board row's Delete or Back step under `gate`.
fn refresh_step_button(
    verbs: &mut ChildSpawnerCommands,
    name: String,
    spec: ButtonSpec,
    marker: JobRefreshButton,
    gate: VerbGate,
) {
    verbs
        .spawn((
            Name::new(name),
            button(spec),
            ButtonCueType::Owned,
            marker,
            gate,
        ))
        .observe(on_job_refresh_button);
}

/// A panel's title row.
fn panel_title(panel: &mut ChildSpawnerCommands, title: &str) {
    panel
        .spawn(control_row(JustifyContent::FlexStart))
        .with_children(|head| {
            head.spawn(text(title, TextRole::Title));
        });
}

/// The docked station's board. The pane head's mode tag names the station.
fn board_panel(
    col: &mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    board: &BoardView,
    selected: Option<&JobId>,
    offset: ScrollPosition,
) {
    col.spawn((Name::new(JOBS_BOARD), column_panel()))
        .with_children(|panel| {
            panel_title(panel, "Board");
            if board.rows.is_empty() {
                return;
            }
            table_head(
                panel,
                &[
                    (ICON_PX, ""),
                    (0.0, "Job"),
                    (RANGE_PX, "Range"),
                    (THREAT_PX, "Threat"),
                    (REWARD_PX, "Reward"),
                    (TIME_PX, "Time left"),
                    (VERBS_PX, ""),
                ],
            );
            scroll_rows(panel, JobsScrollType::Board, offset, |panel| {
                for (index, row) in board.rows.iter().enumerate() {
                    spawn_row(
                        panel,
                        &row.id,
                        format!("JobRowPosted-{index}"),
                        selected == Some(&row.id),
                    )
                    .with_children(|r| posted_row(r, icons, index, row));
                }
            });
        });
}

/// One board row's cells: icon | title | range | threat | reward | time left
/// | Accept `X`, or, asking, icon | `Delete?` title | blank | fee | blank |
/// Back Delete.
fn posted_row(r: &mut ChildSpawnerCommands, icons: &InterfaceIcons, index: usize, row: &PostedRow) {
    r.spawn(fixed(
        ICON_PX,
        icon_node(icons.job(row.icon), row.icon.color(), 18.0),
    ));
    if row.confirming {
        r.spawn(ask_title_cell("Delete?", &row.title, TextRole::Value));
        // One blank for the range and threat columns, so the fee stays
        // under Reward.
        r.spawn(fixed(RANGE_PX + THREAT_PX + metrics::GAP_L, ()));
        // A short balance paints the fee it cannot pay.
        let tone = if row.refresh_gate == VerbGate::Disabled {
            UiColorType::Danger
        } else {
            UiColorType::Accent
        };
        r.spawn(fixed(REWARD_PX, ())).with_children(|fee| {
            spawn_credit_figure(
                fee,
                icons,
                u64::from(JOB_OFFER_REFRESH_FEE_CR),
                TextRole::Value,
                tone,
                (),
            );
        });
        r.spawn(fixed(TIME_PX, ()));
        let refresh_button = |step| JobRefreshButton {
            step,
            id: row.id.clone(),
        };
        r.spawn(verbs_cell()).with_children(|verbs| {
            refresh_step_button(
                verbs,
                format!("{JOB_REFRESH_BACK_PREFIX}{index}"),
                ButtonSpec::control("Back"),
                refresh_button(JobAskStepType::Back),
                VerbGate::Ready,
            );
            refresh_step_button(
                verbs,
                format!("{JOB_REFRESH_CONFIRM_PREFIX}{index}"),
                ButtonSpec::control("Delete").danger(),
                refresh_button(JobAskStepType::Confirm),
                row.refresh_gate.clone(),
            );
        });
        return;
    }
    r.spawn(title_cell(&row.title, TextRole::Value, UiColorType::Text));
    r.spawn(text_cell(
        RANGE_PX,
        &row.range,
        TextRole::Body,
        UiColorType::TextBody,
    ));
    r.spawn(text_cell(
        THREAT_PX,
        &row.threat,
        TextRole::Body,
        UiColorType::TextBody,
    ));
    r.spawn(fixed(REWARD_PX, ())).with_children(|reward| {
        spawn_credit_figure(
            reward,
            icons,
            row.reward_cr,
            TextRole::Value,
            UiColorType::Accent,
            (),
        );
    });
    r.spawn(fixed(
        TIME_PX,
        (
            Node {
                align_items: AlignItems::Center,
                column_gap: px(metrics::GAP),
                ..default()
            },
            Pickable::IGNORE,
            children![
                bare_meter(TIME_METER_PX, row.time_left.0),
                text_nowrap(&row.time_left.1, TextRole::Body, UiColorType::TextBody),
            ],
        ),
    ));
    r.spawn(verbs_cell()).with_children(|verbs| {
        action_button(
            verbs,
            format!("{JOB_ACCEPT_PREFIX}{index}"),
            "Accept",
            JobActionButton {
                list: if row.hail {
                    JobListType::Hail
                } else {
                    JobListType::Posted
                },
                id: row.id.clone(),
            },
            row.gate.clone(),
        );
        if row.hail {
            ask_button(
                verbs,
                icons,
                JOB_HAIL_DECLINE.to_string(),
                HailDeclineButton,
            )
            .observe(on_hail_decline);
        }
        if row.refreshable {
            ask_button(
                verbs,
                icons,
                format!("{JOB_REFRESH_PREFIX}{index}"),
                JobRefreshButton {
                    step: JobAskStepType::Ask,
                    id: row.id.clone(),
                },
            )
            .observe(on_job_refresh_button);
        }
    });
}

/// The start role's goal line under the held table head: its title and next
/// unmet line with a `have / need` meter, or the play time it was met.
fn goal_row(panel: &mut ChildSpawnerCommands, icons: &InterfaceIcons, view: &JobsView) {
    let (line, color, progress) = match (&view.milestone, view.goal_met_at) {
        (Some(line), _) => (
            format!("{}: {}", view.goal_title, line.label),
            UiColorType::Text,
            Some((line.have, line.need)),
        ),
        (None, Some(at)) => (
            format!("{}: {}", view.goal_title, play_time_text(at)),
            UiColorType::Accent,
            None,
        ),
        (None, None) => return,
    };
    panel
        .spawn((
            Name::new(JOBS_MILESTONE),
            Node {
                // The cells line up with the held rows under it.
                width: percent(100),
                min_height: px(metrics::LIST_ROW_H),
                padding: UiRect {
                    left: px(ROW_INSET_PX),
                    right: px(ROW_INSET_PX + SCROLL_GUTTER_PX),
                    ..default()
                },
                column_gap: px(metrics::GAP_L),
                row_gap: px(metrics::GAP_S),
                flex_wrap: FlexWrap::Wrap,
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
            row.spawn(title_cell(&line, TextRole::Value, color));
            match progress {
                Some((have, need)) => {
                    row.spawn(fixed(PROGRESS_PX, meter(ROW_METER_PX, have, need, false)))
                }
                None => row.spawn(fixed(PROGRESS_PX, ())),
            };
            // One blank for the hand-in and verb columns, so a wrapped row
            // spends one empty line, not two cells.
            row.spawn(fixed(HAND_IN_PX + VERBS_PX + metrics::GAP_L, ()));
        });
}

/// The jobs the player holds: the title, the table head, the role goal's
/// line and the held rows.
fn held_panel(
    col: &mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    view: &JobsView,
    selected: Option<&JobId>,
    offset: ScrollPosition,
) {
    col.spawn((Name::new(JOBS_HELD), column_panel()))
        .with_children(|panel| {
            panel_title(panel, "Your jobs");
            let goal = view.milestone.is_some() || view.goal_met_at.is_some();
            if view.held.is_empty() && !goal {
                return;
            }
            table_head(
                panel,
                &[
                    (ICON_PX, ""),
                    (0.0, "Job"),
                    (PROGRESS_PX, "Progress"),
                    (HAND_IN_PX, "Hand-in"),
                    (VERBS_PX, ""),
                ],
            );
            goal_row(panel, icons, view);
            scroll_rows(panel, JobsScrollType::Held, offset, |panel| {
                for (index, row) in view.held.iter().enumerate() {
                    spawn_row(
                        panel,
                        &row.id,
                        format!("JobRowHeld-{index}"),
                        selected == Some(&row.id),
                    )
                    .with_children(|r| held_row(r, icons, index, row));
                }
            });
        });
}

/// One held row's cells: icon | title | progress | HAND IN HERE | Hand in
/// `X`, or, asking, icon | `Drop?` title | progress | blank | Back Drop. A
/// short count that blocks Hand in is painted Danger.
fn held_row(r: &mut ChildSpawnerCommands, icons: &InterfaceIcons, index: usize, row: &HeldRow) {
    r.spawn(fixed(
        ICON_PX,
        icon_node(icons.job(row.icon), row.icon.color(), 18.0),
    ));
    if row.confirming {
        r.spawn(ask_title_cell("Drop?", &row.title, TextRole::Value));
    } else {
        r.spawn(title_cell(&row.title, TextRole::Value, UiColorType::Text));
    }
    let short = row.gate == VerbGate::Disabled && row.have < row.need;
    r.spawn(fixed(
        PROGRESS_PX,
        meter(ROW_METER_PX, row.have.into(), row.need.into(), short),
    ));
    let drop_button = |step| JobDropButton {
        step,
        id: row.id.clone(),
    };
    if row.confirming {
        r.spawn(fixed(HAND_IN_PX, ()));
        r.spawn(verbs_cell()).with_children(|verbs| {
            drop_step_button(
                verbs,
                format!("{JOB_DROP_BACK_PREFIX}{index}"),
                ButtonSpec::control("Back"),
                drop_button(JobAskStepType::Back),
            );
            drop_step_button(
                verbs,
                format!("{JOB_DROP_CONFIRM_PREFIX}{index}"),
                ButtonSpec::control("Drop").danger(),
                drop_button(JobAskStepType::Confirm),
            );
        });
        return;
    }
    if row.hand_in_here {
        r.spawn(fixed(HAND_IN_PX, tag("HAND IN HERE", UiColorType::Nominal)));
    } else {
        r.spawn(fixed(HAND_IN_PX, ()));
    }
    r.spawn(verbs_cell()).with_children(|verbs| {
        action_button(
            verbs,
            format!("{JOB_HAND_IN_PREFIX}{index}"),
            "Hand in",
            JobActionButton {
                list: JobListType::Held,
                id: row.id.clone(),
            },
            row.gate.clone(),
        );
        ask_button(
            verbs,
            icons,
            format!("{JOB_DROP_PREFIX}{index}"),
            drop_button(JobAskStepType::Ask),
        )
        .observe(on_job_drop_button);
    });
}

/// The inspector: the selected job, or nothing.
fn inspector_panel(
    side: &mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    inspector: Option<&InspectorView>,
) {
    let Some(job) = inspector else {
        return;
    };
    side.spawn(control_row(JustifyContent::FlexStart))
        .with_children(|head| {
            head.spawn(icon_node(icons.job(job.icon), job.icon.color(), 24.0));
            head.spawn((
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    ..default()
                },
                children![text(&job.title, TextRole::Title)],
            ));
            if job.done {
                head.spawn(tag("DONE", UiColorType::Nominal));
            }
        });
    spawn_block(side, "Task", ()).with_children(|block| {
        block.spawn(text(&job.task, TextRole::Body));
        spawn_kv(block, "Target", text(&job.target, TextRole::Value));
        spawn_kv(block, "Hand in", text(&job.hand_in, TextRole::Value));
        spawn_kv(
            block,
            "Reward",
            text_toned(cr_text(job.reward_cr), TextRole::Value, UiColorType::Accent),
        );
        for term in &job.terms {
            spawn_kv(block, &term.label, text(cr_text(term.cr), TextRole::Value));
        }
        if let Some((_, minutes)) = &job.time_left {
            spawn_kv(block, "Time left", text(minutes, TextRole::Value));
        }
        if let Some((have, need)) = job.progress {
            spawn_kv(
                block,
                "Progress",
                text(format!("{have} / {need}"), TextRole::Value),
            );
        }
        block.spawn(verb_row(RowAlign::End)).with_children(|row| {
            row.spawn((
                Name::new(JOB_SHOW_ON_MAP),
                JobShowOnMap(job.show.clone()),
                button(ButtonSpec::control("Show on Map")),
            ))
            .observe(on_show_on_map);
        });
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
/// press is silent: the result plays the one cue.
fn on_job_button(
    activate: On<Activate>,
    q_button: Query<&JobActionButton>,
    mut accepts: MessageWriter<JobAcceptCommand>,
    mut hand_ins: MessageWriter<JobHandInCommand>,
    mut answers: MessageWriter<HailAnswerCommand>,
) {
    let Ok(button) = q_button.get(activate.entity) else {
        return;
    };
    match button.list {
        JobListType::Hail => {
            answers.write(HailAnswerCommand::Accept);
        }
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

/// Decline the standing hail offer when its row's `X` is activated. The
/// press is silent: the result plays the one cue.
fn on_hail_decline(
    activate: On<Activate>,
    q_button: Query<(), With<HailDeclineButton>>,
    mut answers: MessageWriter<HailAnswerCommand>,
) {
    if q_button.contains(activate.entity) {
        answers.write(HailAnswerCommand::Decline);
    }
}

/// Raise the cue of each hail answer the encounter director gave: the click
/// for a taken accept or decline, the deny sound for one it refused.
fn cue_hail_answers(mut answered: MessageReader<HailAnswered>, mut runtime: ResMut<JobsRuntime>) {
    for answer in answered.read() {
        runtime.cue.raise(if answer.taken {
            ActionOutcomeType::Done
        } else {
            ActionOutcomeType::Refused
        });
    }
}

/// Take the button's drop step when it is activated: `X` asks for a
/// confirm and Back keeps the job, each with one click; Drop raises the
/// [`JobDropCommand`] silently, as the result plays the one cue.
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
/// the [`JobOfferRefreshCommand`] silently, as the result plays the one
/// cue.
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
) {
    let Ok(JobShowOnMap(mark)) = q_button.get(activate.entity) else {
        return;
    };
    map.chart_show = Some(mark.clone());
    map.chart_requested = true;
    *pane = InterfacePaneType::Map;
}

/// Play the cue a Jobs result owes once the sound bank is present. It runs
/// whatever pane shows, like the Station pane's, so a result raised with the
/// Jobs pane closed still plays its one cue.
fn play_jobs_cue(
    mut runtime: ResMut<JobsRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    runtime.cue.tick(bank.as_deref(), &mut commands);
}

/// Apply [`JobAcceptCommand`]s by the [`plan_job_accept`] rule and
/// [`JobHandInCommand`]s by the [`plan_job_hand_in`] rule: an accept holds
/// the job and raises [`JobAccepted`]; a hand-in takes a procurement's goods
/// out of the hold, pays the reward with `JobReward` credit changes on both
/// sides, completes the job, raises [`JobHandedIn`] under the code the job
/// was held by and starts the pane head's count-up ([`BalanceCountUp`]). Each
/// result raises one cue, a paid hand-in the payout cue; a refusal changes
/// nothing. An accept of a job the docked
/// station's board no longer posts is refused as expired: the slot renewed
/// between the click and this frame. A [`JobDropCommand`] drops the held job
/// ([`Jobs::drop`]) with no payout, raises [`JobDropped`] with the cause
/// `Abandoned` and takes out the bodies its accept put in the world
/// ([`Job::spawned_bodies`]), as `drop_on_loss` does in the open world
/// crate, whatever station it is docked at; a drop of a job no
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
    reason = "one system reading the four commands, the board, the jobs, the postings, the clock, the table, the site, the bodies, the targets, the hold, both balances, the count-up and the stances"
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
    mut count_up: ResMut<BalanceCountUp>,
    items: Option<Res<GameItems>>,
    table: Option<Res<GamePriceTable>>,
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
            runtime.cue.raise(ActionOutcomeType::Refused);
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
            runtime.cue.raise(ActionOutcomeType::Refused);
            continue;
        };
        let expired = !postings.offers(&job.id, table.0.jobs, clock.play_seconds);
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
                runtime.cue.raise(ActionOutcomeType::Done);
            }
            Err(refusal) => {
                debug!("nova_interface: refused an accept: {refusal:?}");
                runtime.cue.raise(ActionOutcomeType::Refused);
            }
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
            runtime.cue.raise(ActionOutcomeType::Refused);
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
                debug!("nova_interface: refused: {refusal:?}");
                runtime.cue.raise(ActionOutcomeType::Refused);
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
        count_up.raise();
        let code = jobs
            .mark_code(&job.id)
            .expect("the hand-in was planned for a held job");
        let handed_in = jobs.complete(&job.id);
        commands.trigger(JobHandedIn {
            job: handed_in,
            code,
        });
        info!(
            "nova_interface: handed in the job '{}' ({title}) for {} cr",
            job.id, plan.reward_cr
        );
        if let Ok(market) = q_markets.get(*station) {
            if let Some(step) = record_merit(&mut stances, market.civilization, clock.play_seconds)
            {
                commands.trigger(StanceChanged(step));
                info!(
                    "stance: {} moved from {:?} to {:?} on the hand-in at {:.1} s of play",
                    step.civilization.name(),
                    step.from,
                    step.to,
                    clock.play_seconds
                );
            }
        }
        runtime.cue.raise(ActionOutcomeType::Paid);
    }
    for command in drops.read() {
        if !jobs.holds(&command.id) {
            warn!(
                "nova_interface: the drop of '{}' finds it not held",
                command.id
            );
            runtime.cue.raise(ActionOutcomeType::Refused);
            continue;
        }
        let job = jobs.drop(&command.id);
        commands.trigger(JobDropped {
            job: job.clone(),
            cause: JobDropCauseType::Abandoned,
        });
        let gone = job.spawned_bodies();
        for (entity, id) in &q_bodies {
            if gone.contains(id) {
                commands.entity(entity).try_despawn();
            }
        }
        let title = job.title(&items);
        info!("nova_interface: dropped the job '{}' ({title})", job.id);
        runtime.cue.raise(ActionOutcomeType::Done);
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
            .filter(|(_, _, id)| *id == command.id.poster);
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
                debug!("nova_interface: refused: {refusal:?}");
                runtime.cue.raise(ActionOutcomeType::Refused);
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
            "nova_interface: paid {} cr to refresh the offer '{}' ({title})",
            plan.fee_cr, command.id
        );
        runtime.cue.raise(ActionOutcomeType::Done);
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
        // Owned by the open world's encounter director; registered here as
        // well so the pane stands alone. A second registration is a no-op.
        app.add_message::<HailAnswerCommand>();
        app.add_message::<HailAnswered>();
        app.init_resource::<Encounters>();
        app.add_systems(
            Update,
            (
                apply_job_commands,
                cue_hail_answers,
                update_jobs_pane,
                play_jobs_cue,
            )
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

    /// The job handler and the cue player on a manual real clock, with the
    /// UI sound bank captured and no UI tree: the cue must not need the pane.
    fn jobs_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.5,
        )));
        app.insert_resource(test_items());
        app.insert_resource(GamePriceTable(price_table()));
        app.init_resource::<JobsRuntime>();
        app.init_resource::<BalanceCountUp>();
        app.init_resource::<Chart>();
        app.init_resource::<Jobs>();
        app.init_resource::<JobPostings>();
        app.init_resource::<WorldClock>();
        app.init_resource::<CivilizationStances>();
        app.add_message::<JobAcceptCommand>();
        app.add_message::<JobHandInCommand>();
        app.add_message::<JobDropCommand>();
        app.add_message::<JobOfferRefreshCommand>();
        app.add_systems(Update, (apply_job_commands, play_jobs_cue).chain());
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
            id: JobId::slot(station_id.clone(), 0, 0),
            kind: JobType::Procure {
                item: iron.clone(),
                count: 4,
            },
            target: station_id.clone(),
            leg_m: 0,
            threat: 0,
            reward: JobReward::flat(146),
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
                    profile: None,
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

    /// A hand-in pays the reward, completes the job and plays the payout cue
    /// once, with no click; the frame after is silent.
    #[test]
    fn a_hand_in_pays_completes_the_job_and_plays_the_payout_cue_once_with_no_click() {
        let civilization = neutral_civilization();
        let mut app = jobs_app();
        let (player, job) = hand_in_fixture(&mut app, civilization);

        app.world_mut()
            .write_message(JobHandInCommand { id: job.id.clone() });
        app.update();
        assert_eq!(
            app.world().get::<ShipCredits>(player),
            Some(&ShipCredits(2_146))
        );
        assert!(app.world().resource::<Jobs>().is_completed(&job.id));
        assert_eq!(take_cues(&mut app), [UiSfx::Payout]);
        app.update();
        assert!(take_cues(&mut app).is_empty(), "the cue plays once");
    }

    /// An accept while undocked buzzes once; the frame after is silent.
    #[test]
    fn a_refused_accept_buzzes_once() {
        let mut app = jobs_app();
        let id = JobId::slot(
            GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0),
            0,
            0,
        );

        app.world_mut().write_message(JobAcceptCommand { id });
        app.update();
        assert_eq!(take_cues(&mut app), [UiSfx::EditorDeny]);
        app.update();
        assert!(take_cues(&mut app).is_empty(), "the cue plays once");
    }

    /// A drop of a job the player no longer holds buzzes once and changes no
    /// record.
    #[test]
    fn a_drop_of_a_job_no_longer_held_is_refused_and_changes_nothing() {
        let mut app = jobs_app();
        let id = JobId::slot(
            GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0),
            0,
            0,
        );

        app.world_mut()
            .write_message(JobDropCommand { id: id.clone() });
        app.update();
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
        app.init_resource::<BalanceCountUp>();
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
            id: JobId::slot(station.clone(), 0, 0),
            kind: JobType::Distress {
                caller: caller.clone(),
            },
            target: station,
            leg_m: 0,
            threat: 0,
            reward: JobReward::flat(146),
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
                            name.to_string(),
                            ButtonSpec::control(label),
                            JobDropButton {
                                step,
                                id: job.id.clone(),
                            },
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
            id: JobId::slot(station_id.clone(), 0, 0),
            kind: JobType::Distress { caller },
            target: station_id.clone(),
            leg_m: 0,
            threat: 0,
            reward: JobReward::flat(146),
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
            id: JobId::slot(station_id.clone(), 0, 0),
            kind: JobType::Distress { caller },
            target: station_id.clone(),
            leg_m: 0,
            threat: 0,
            reward: JobReward::flat(146),
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
            id: JobId::slot(job.id.poster.clone(), slot, 0),
            kind: JobType::Procure {
                item: iron.clone(),
                count: 1,
            },
            target: job.id.poster.clone(),
            leg_m: 0,
            threat: 0,
            reward: JobReward::flat(10),
        };
        app.world_mut()
            .resource_mut::<JobPostings>()
            .post(vec![job.clone(), filler(1), filler(2)], 0.0);
    }

    /// An offer refresh of a current, unheld board offer docked at the
    /// issuing station moves the fee from the player to the station in one
    /// frame, queues a `JobRefresh` report on each side and marks the slot
    /// for a refresh; the offer is not held, since it was never accepted.
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
        }
        assert_eq!(
            take_cues(&mut app),
            [UiSfx::EditorDeny],
            "a stale refresh buzzes"
        );

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
            take_cues(&mut app),
            [UiSfx::EditorDeny],
            "a short refresh buzzes"
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
    }

    /// The pane raises one `JobHandedIn` for a paid hand-in and one
    /// `JobDropped` with the cause `Abandoned` for a confirmed drop, each
    /// with the whole job; a drop of the job again, no longer held, raises
    /// none.
    #[test]
    fn a_hand_in_and_a_drop_each_raise_their_job_event_once() {
        #[derive(Resource, Default)]
        struct Raised {
            handed_in: Vec<Job>,
            dropped: Vec<(Job, JobDropCauseType)>,
        }
        fn hear(app: &mut App) {
            app.init_resource::<Raised>();
            app.add_observer(|handed_in: On<JobHandedIn>, mut raised: ResMut<Raised>| {
                raised.handed_in.push(handed_in.job.clone());
            });
            app.add_observer(|dropped: On<JobDropped>, mut raised: ResMut<Raised>| {
                raised.dropped.push((dropped.job.clone(), dropped.cause));
            });
        }

        let mut app = jobs_app();
        hear(&mut app);
        let (_, procure) = hand_in_fixture(&mut app, neutral_civilization());
        app.world_mut().write_message(JobHandInCommand {
            id: procure.id.clone(),
        });
        app.update();
        let raised = app.world().resource::<Raised>();
        assert_eq!(raised.handed_in, [procure]);
        assert!(raised.dropped.is_empty());

        let mut app = jobs_app();
        hear(&mut app);
        let (_, _, distress) = drop_fixture(&mut app, 2_000, 10_000, false);
        for _ in 0..2 {
            app.world_mut().write_message(JobDropCommand {
                id: distress.id.clone(),
            });
            app.update();
        }
        let raised = app.world().resource::<Raised>();
        assert_eq!(raised.dropped, [(distress, JobDropCauseType::Abandoned)]);
        assert!(raised.handed_in.is_empty());
    }

    /// A scenario's job on the docked station's board has no window: the
    /// accept goes through long after any offer would have expired. The
    /// accept and the hand-in each raise their job event with the job, whose
    /// id carries the key.
    #[test]
    fn a_scenario_s_job_is_accepted_past_any_window_and_handed_in_under_its_key() {
        #[derive(Resource, Default)]
        struct Raised(Vec<(&'static str, Job)>);

        let mut app = jobs_app();
        app.init_resource::<Raised>();
        app.add_observer(|accepted: On<JobAccepted>, mut raised: ResMut<Raised>| {
            raised.0.push(("accepted", accepted.job.clone()));
        });
        app.add_observer(|handed_in: On<JobHandedIn>, mut raised: ResMut<Raised>| {
            raised.0.push(("handed in", handed_in.job.clone()));
        });
        let (_, slot_job) = hand_in_fixture(&mut app, neutral_civilization());
        let station_id = slot_job.id.poster.clone();
        let job = Job {
            id: JobId::scenario(station_id.clone(), "walk"),
            kind: JobType::Procure {
                item: ItemDesignId::from(ItemRoleType::IronOre.id()),
                count: 4,
            },
            target: station_id,
            leg_m: 0,
            threat: 0,
            reward: JobReward::flat(90),
        };
        let world = app.world_mut();
        world
            .resource_mut::<JobPostings>()
            .post_scenario(job.clone(), 0.0);
        let mut boards = world.query::<&mut JobBoard>();
        boards
            .single_mut(world)
            .expect("the fixture spawns one station")
            .0
            .push(job.clone());
        world.resource_mut::<WorldClock>().play_seconds = 1.0e6;

        app.world_mut()
            .write_message(JobAcceptCommand { id: job.id.clone() });
        app.update();
        assert!(
            app.world().resource::<Jobs>().holds(&job.id),
            "the accept holds the job"
        );
        app.world_mut()
            .write_message(JobHandInCommand { id: job.id.clone() });
        app.update();
        assert!(app.world().resource::<Jobs>().is_completed(&job.id));
        assert_eq!(
            app.world().resource::<Raised>().0,
            [("accepted", job.clone()), ("handed in", job)]
        );
    }

    /// The hand-in that takes a civilization's third merit raises one
    /// `StanceChanged` with its step up.
    #[test]
    fn a_hand_in_that_steps_a_stance_raises_one_stance_changed() {
        #[derive(Resource, Default)]
        struct StanceSteps(Vec<StanceStep>);

        let civilization = neutral_civilization();
        let mut app = jobs_app();
        app.init_resource::<StanceSteps>();
        app.add_observer(
            |changed: On<StanceChanged>, mut steps: ResMut<StanceSteps>| {
                steps.0.push(changed.0);
            },
        );
        let (_, job) = hand_in_fixture(&mut app, civilization);

        app.world_mut()
            .write_message(JobHandInCommand { id: job.id });
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<StanceSteps>().0,
            [StanceStep {
                civilization,
                from: StanceType::Neutral,
                to: StanceType::Ally,
            }]
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
        app.init_resource::<BalanceCountUp>();
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
                            name.to_string(),
                            ButtonSpec::control(label),
                            JobRefreshButton {
                                step,
                                id: job.id.clone(),
                            },
                            VerbGate::Ready,
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
    }

    #[test]
    fn the_starter_board_hides_a_job_the_hull_cannot_do_and_a_core_board_tags_its_accept() {
        let unfit = || Err(JobHullRefusalType::NoWeapon);
        assert_eq!(
            board_listing(true, unfit(), Ok(())),
            None,
            "the starter board leaves the job off"
        );
        assert_eq!(
            board_listing(false, unfit(), Ok(())),
            Some(VerbGate::Tagged("NO WEAPON")),
            "a core board lists it tagged with the part the hull lacks"
        );
        assert_eq!(
            board_listing(false, unfit(), Err(JobAcceptRefusalType::TargetGone)),
            Some(VerbGate::Tagged("NO WEAPON")),
            "the hull's cause comes before the accept rule's"
        );
        assert_eq!(
            board_listing(true, Ok(()), Err(JobAcceptRefusalType::TargetGone)),
            Some(VerbGate::Tagged("GONE")),
            "a fit job with a gone target is listed tagged on either board"
        );
        assert_eq!(
            board_listing(true, Ok(()), Ok(())),
            Some(VerbGate::Ready),
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

    /// At 853 logical px (1280 at 150%) a held row with a long title wraps its
    /// fixed cells under the title: every button stays in the panel, the
    /// title keeps room to read, and Hand in and the `X` are one control
    /// high, side by side with a gap.
    #[test]
    fn jobs_rows_keep_their_buttons_inside_the_panel_at_a_narrow_width() {
        use crate::pointer_rig::{pane_pointer_rig, settle};

        let mut rig = pane_pointer_rig();
        rig.app.add_plugins(nova_ui::NovaUiPlugin);
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let title = "Bring 4 Iron ore to the far outpost beyond the Marianne Lament belt";
        let view = JobsView {
            held: vec![HeldRow {
                id: JobId::slot(station, 0, 0),
                icon: JobIconType::Bring,
                title: title.to_string(),
                have: 1,
                need: 4,
                hand_in_here: true,
                gate: VerbGate::Ready,
                confirming: false,
            }],
            ..default()
        };
        let icons = InterfaceIcons::blank();
        // The card's inner width at 853: the window less the root padding
        // (16 each side) and the body padding (12 each side), then the
        // inspector and the gap beside the column.
        let content = rig.content_root;
        rig.app
            .world_mut()
            .commands()
            .entity(content)
            .with_children(|content| {
                content
                    .spawn(Node {
                        width: px(853.0 - 32.0 - 24.0),
                        height: px(400),
                        flex_grow: 0.0,
                        ..view_split()
                    })
                    .with_children(|split| {
                        split.spawn(column()).with_children(|col| {
                            held_panel(col, &icons, &view, None, ScrollPosition::default());
                        });
                        split.spawn(inspector(Overflow::scroll_y()));
                    });
            });
        settle(&mut rig.app);
        settle(&mut rig.app);

        let world = rig.app.world_mut();
        let rect = |world: &World, entity: Entity| {
            let node = world.get::<ComputedNode>(entity).expect("a laid-out node");
            let centre = world
                .get::<UiGlobalTransform>(entity)
                .expect("a laid-out node")
                .translation;
            Rect::from_center_size(centre, node.size())
        };
        let by_name = |world: &mut World, name: &str| {
            world
                .query::<(Entity, &Name)>()
                .iter(world)
                .find(|(_, each)| each.as_str() == name)
                .map(|(entity, _)| entity)
                .unwrap_or_else(|| panic!("a node named {name}"))
        };
        let held = by_name(world, JOBS_HELD);
        let panel = rect(world, held);
        let row = by_name(world, "JobRowHeld-0");
        let buttons: Vec<Rect> = world
            .query_filtered::<(Entity, &ChildOf), With<ThemedButton>>()
            .iter(world)
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>()
            .into_iter()
            .filter(|button| {
                let mut at = *button;
                while let Some(parent) = world.get::<ChildOf>(at) {
                    at = parent.parent();
                    if at == row {
                        return true;
                    }
                }
                false
            })
            .map(|button| rect(world, button))
            .collect();
        assert_eq!(buttons.len(), 2, "Hand in and the X ask: {buttons:?}");
        let (hand_in, ask) = if buttons[0].min.x < buttons[1].min.x {
            (buttons[0], buttons[1])
        } else {
            (buttons[1], buttons[0])
        };
        for button in &buttons {
            assert!(
                (button.height() - metrics::CONTROL_H).abs() < 0.5,
                "a row verb is one control high: {button:?}"
            );
        }
        assert!(
            (hand_in.center().y - ask.center().y).abs() < 0.5
                && ask.min.x - hand_in.max.x >= metrics::GAP - 0.5,
            "Hand in and the X share a line with a gap: {hand_in:?} {ask:?}"
        );
        for button in &buttons {
            assert!(
                button.min.x >= panel.min.x - 0.5
                    && button.max.x <= panel.max.x + 0.5
                    && button.min.y >= panel.min.y - 0.5
                    && button.max.y <= panel.max.y + 0.5,
                "button {button:?} inside the panel {panel:?}"
            );
        }
        // The table head holds an ellipsis label too: take the row's own.
        let label = world
            .query_filtered::<Entity, With<EllipsisText>>()
            .iter(world)
            .collect::<Vec<_>>()
            .into_iter()
            .find(|label| {
                let mut at = *label;
                while let Some(parent) = world.get::<ChildOf>(at) {
                    at = parent.parent();
                    if at == row {
                        return true;
                    }
                }
                false
            })
            .expect("the row title is an ellipsis label");
        let label = rect(world, label);
        assert!(
            label.width() >= 100.0
                && label.min.x >= panel.min.x - 0.5
                && label.max.x <= panel.max.x + 0.5,
            "the title keeps room inside the panel: {label:?} in {panel:?}"
        );
    }

    #[test]
    fn a_leg_reads_in_kilometres_to_one_decimal_and_a_job_without_one_reads_nothing() {
        assert_eq!(range_text(0), "");
        assert_eq!(range_text(12_000), "12 km");
        assert_eq!(range_text(4_200), "4.2 km");
        assert_eq!(range_text(9_940), "9.9 km");
        assert_eq!(range_text(9_960), "10 km", "rounds to the tenth");
        assert_eq!(range_text(33_440), "33 km", "whole kilometres from 10 km");
        assert_eq!(range_text(u32::MAX), "4294967 km");
        assert_eq!(threat_text(0), "");
        assert_eq!(threat_text(3), "3");
    }

    #[test]
    fn the_inspector_lists_each_term_that_pays_and_none_for_a_reward_of_one_term() {
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let id = JobId::slot(station.clone(), 0, 0);
        let escort = Job {
            id: id.clone(),
            kind: JobType::Escort {
                hauler: GeneratedBodyId::new(SectorCoord::ORIGIN, "hauler", 0),
                cargo_cr: 1_200,
                raiders: 2,
            },
            target: station.clone(),
            leg_m: 12_000,
            threat: 4,
            reward: JobReward {
                base_cr: 250,
                goods_cr: 300,
                range_cr: 360,
                threat_cr: 1_200,
            },
        };
        let rows: Vec<(String, u32)> = reward_term_rows(&escort)
            .into_iter()
            .map(|row| (row.label, row.cr))
            .collect();
        assert_eq!(
            rows,
            [
                ("Base".to_string(), 250),
                ("Cargo".to_string(), 300),
                ("Range 12 km".to_string(), 360),
                ("Threat 4".to_string(), 1_200),
            ]
        );

        let procure = Job {
            id: id.clone(),
            kind: JobType::Procure {
                item: ItemDesignId::from(ItemRoleType::IronOre),
                count: 4,
            },
            target: station.clone(),
            leg_m: 0,
            threat: 0,
            reward: JobReward {
                base_cr: 100,
                goods_cr: 46,
                range_cr: 0,
                threat_cr: 0,
            },
        };
        let labels: Vec<String> = reward_term_rows(&procure)
            .into_iter()
            .map(|row| row.label)
            .collect();
        assert_eq!(
            labels,
            ["Base", "Goods"],
            "a term that pays nothing has no row"
        );

        let flat = Job {
            reward: JobReward::flat(90),
            ..procure
        };
        assert_eq!(reward_term_rows(&flat), [], "the total already says it");
    }

    /// At 1024 logical px a board row with a long title, a leg, a threat and
    /// a gate tag keeps every cell on one line: its verbs stay whole inside
    /// the panel and the title is the cell that gives way.
    #[test]
    fn a_board_row_stays_on_one_line_with_its_verbs_whole_at_1024_px() {
        use crate::pointer_rig::{pane_pointer_rig, settle};

        let mut rig = pane_pointer_rig();
        rig.app.add_plugins(nova_ui::NovaUiPlugin);
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let board = BoardView {
            rows: vec![PostedRow {
                id: JobId::slot(station, 0, 0),
                icon: JobIconType::Neutralize,
                title: "Neutralize the raider flagship Marianne Lament beyond the belt".to_string(),
                range: "9.9 km".to_string(),
                threat: "12".to_string(),
                reward_cr: 12_345,
                time_left: (0.5, "60 min".to_string()),
                gate: VerbGate::Tagged("NO WEAPON"),
                refreshable: true,
                hail: false,
                confirming: false,
                refresh_gate: VerbGate::Ready,
            }],
        };
        let icons = InterfaceIcons::blank();
        // The pane's inner width at 1024: the window less, on each side, the
        // root padding (16), the card border (1) and the body padding (12).
        let content = rig.content_root;
        rig.app
            .world_mut()
            .commands()
            .entity(content)
            .with_children(|content| {
                content
                    .spawn(Node {
                        width: px(1024.0 - 2.0 * (16.0 + 1.0 + 12.0)),
                        height: px(400),
                        flex_grow: 0.0,
                        ..view_split()
                    })
                    .with_children(|split| {
                        split.spawn(column()).with_children(|col| {
                            board_panel(col, &icons, &board, None, ScrollPosition::default());
                        });
                        split.spawn(inspector(Overflow::scroll_y()));
                    });
            });
        settle(&mut rig.app);
        settle(&mut rig.app);

        let world = rig.app.world_mut();
        let rect = |world: &World, entity: Entity| {
            let node = world.get::<ComputedNode>(entity).expect("a laid-out node");
            let centre = world
                .get::<UiGlobalTransform>(entity)
                .expect("a laid-out node")
                .translation;
            Rect::from_center_size(centre, node.size())
        };
        let by_name = |world: &mut World, name: &str| {
            world
                .query::<(Entity, &Name)>()
                .iter(world)
                .find(|(_, each)| each.as_str() == name)
                .map(|(entity, _)| entity)
                .unwrap_or_else(|| panic!("a node named {name}"))
        };
        let panel = by_name(world, JOBS_BOARD);
        let panel = rect(world, panel);
        let row = by_name(world, "JobRowPosted-0");
        let row_rect = rect(world, row);
        assert!(
            row_rect.height() <= metrics::LIST_ROW_H + 2.0 * BORDER_W + 0.5,
            "the row is one line high: {row_rect:?}"
        );
        let cells: Vec<Rect> = world
            .get::<Children>(row)
            .expect("the row has cells")
            .iter()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|cell| rect(world, cell))
            .collect();
        for pair in cells.windows(2) {
            assert!(
                pair[1].min.x >= pair[0].max.x + metrics::GAP_L - 0.5,
                "cells stand side by side, a gap apart: {cells:?}"
            );
        }
        let verbs = cells.last().expect("the verbs cell");
        assert!(
            verbs.max.x <= panel.max.x + 0.5 && verbs.min.x >= panel.min.x - 0.5,
            "the verbs cell is inside the panel: {verbs:?} in {panel:?}"
        );
        let buttons: Vec<Rect> = world
            .query_filtered::<Entity, With<ThemedButton>>()
            .iter(world)
            .collect::<Vec<_>>()
            .into_iter()
            .map(|button| rect(world, button))
            .collect();
        assert_eq!(buttons.len(), 2, "Accept and the X ask: {buttons:?}");
        for button in &buttons {
            assert!(
                button.min.x >= verbs.min.x - 0.5 && button.max.x <= verbs.max.x + 0.5,
                "the verb {button:?} is whole inside its cell {verbs:?}"
            );
        }
        // The tag's own width is not judged here: this rig lays text out in
        // the engine's fallback font, a fifth wider than the game's.
        // Icon, then the title: the one cell that gives way.
        let title = cells[1];
        assert!(
            title.width() >= TITLE_MIN_PX - 0.5,
            "the title keeps its least width: {title:?}"
        );
    }

    /// A hail row's Accept and `X` each raise the director's command at one
    /// click, with no confirm, and never a board accept.
    #[test]
    fn a_hail_row_s_accept_and_x_raise_the_hail_answers() {
        use crate::pointer_rig::{click_at, pane_pointer_rig, settle};

        let mut rig = pane_pointer_rig();
        rig.app.add_message::<JobAcceptCommand>();
        rig.app.add_message::<JobHandInCommand>();
        rig.app.add_message::<HailAnswerCommand>();
        rig.app.init_resource::<JobsRuntime>();
        let caller = GeneratedBodyId::new(SectorCoord::ORIGIN, "encounter_0_caller", 0);
        let board = BoardView {
            rows: vec![PostedRow {
                id: JobId::hail(caller, 0),
                icon: JobIconType::Answer,
                title: "Answer a distress call".to_string(),
                range: "6 km".to_string(),
                threat: String::new(),
                reward_cr: 320,
                time_left: (0.8, "4 min".to_string()),
                gate: VerbGate::Ready,
                refreshable: false,
                hail: true,
                confirming: false,
                refresh_gate: VerbGate::Ready,
            }],
        };
        let icons = InterfaceIcons::blank();
        let content = rig.content_root;
        rig.app
            .world_mut()
            .commands()
            .entity(content)
            .with_children(|content| {
                content.spawn(column()).with_children(|col| {
                    board_panel(col, &icons, &board, None, ScrollPosition::default());
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
                .unwrap_or_else(|| panic!("{name} is laid out"));
            click_at(rig, centre);
        };
        let answers = |rig: &mut crate::pointer_rig::PanePointerRig| -> Vec<HailAnswerCommand> {
            rig.app
                .world_mut()
                .resource_mut::<Messages<HailAnswerCommand>>()
                .drain()
                .collect()
        };

        click(&mut rig, "JobAccept-0");
        assert_eq!(answers(&mut rig), [HailAnswerCommand::Accept]);
        click(&mut rig, JOB_HAIL_DECLINE);
        assert_eq!(answers(&mut rig), [HailAnswerCommand::Decline]);
        assert!(
            rig.app
                .world()
                .resource::<Messages<JobAcceptCommand>>()
                .is_empty(),
            "a hail is not accepted off a station's board"
        );
    }

    /// A hail answer the director took clicks once; one it refused buzzes
    /// once.
    #[test]
    fn a_taken_hail_answer_clicks_and_a_refused_one_buzzes() {
        let mut app = jobs_app();
        app.add_message::<HailAnswered>();
        app.add_systems(Update, cue_hail_answers.before(play_jobs_cue));
        app.world_mut().write_message(HailAnswered {
            answer: HailAnswerCommand::Decline,
            taken: true,
        });
        app.update();
        assert_eq!(take_cues(&mut app), [UiSfx::MenuSelect]);
        app.world_mut().write_message(HailAnswered {
            answer: HailAnswerCommand::Accept,
            taken: false,
        });
        app.update();
        assert_eq!(take_cues(&mut app), [UiSfx::EditorDeny]);
        app.update();
        assert!(take_cues(&mut app).is_empty(), "each cue plays once");
    }

    #[test]
    fn a_click_on_a_job_row_selects_its_job() {
        use crate::pointer_rig::{click_at, pane_pointer_rig, settle};

        let mut rig = pane_pointer_rig();
        rig.app.init_resource::<JobsRuntime>();
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let job = |slot| JobId::slot(station.clone(), slot, 0);
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
