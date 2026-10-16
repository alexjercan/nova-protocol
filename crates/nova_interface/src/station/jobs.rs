//! The Station pane's "Jobs posted here" and "Your jobs" lists, and the
//! Accept and Hand in actions.
//!
//! While the ship is docked at a living station that offers Jobs, the pane
//! lists the station's [`JobBoard`] less the jobs the player holds or
//! completed, each with its title, its reward and an Accept button, and
//! under it every job the player holds, from any station, with a Hand in
//! button while this station is its target and the job is done, by the
//! goods in the hold for a procurement and by the world for a located job,
//! else the reason. A click raises a [`JobAcceptCommand`] or a
//! [`JobHandInCommand`]; [`apply_station_job_commands`] plans it with
//! [`plan_job_accept`] or [`plan_job_hand_in`], moves the goods and the
//! credits, moves the job in [`Jobs`] and raises [`JobAccepted`] so the open
//! world puts the job's bodies in place.
//!
//! Touch this module when changing what the lists show or what a hand-in
//! moves.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, GeneratedBodyId};
use nova_gameplay::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::StationRuntime;
use crate::{
    inventory::cr_text,
    pane::{compact_button, play_menu_select, themed_label},
    ship::StationSite,
};

/// A request to accept the posted job `id` off the docked station's board,
/// raised by a "Jobs posted here" Accept button.
#[derive(Message, Clone, Debug)]
pub struct JobAcceptCommand {
    /// The posted job.
    pub id: JobId,
}

/// A request to hand the held job `id` in at the docked station, raised by
/// a "Your jobs" Hand in button.
#[derive(Message, Clone, Debug)]
pub struct JobHandInCommand {
    /// The held job.
    pub id: JobId,
}

/// The job lists container, hidden while the jobs gate does not hold and
/// the player holds no job.
#[derive(Component)]
pub(crate) struct StationJobList;

/// Which of the two lists a rows container is.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StationJobListType {
    /// The docked station's board, less the jobs held or completed.
    Posted,
    /// The jobs the player holds.
    Held,
}

/// The rows inside one list, and the row texts they were built from, so
/// the rows respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct StationJobRows(Vec<JobRow>);

/// One job as a list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct JobRow {
    id: JobId,
    title: String,
    /// The line after the title: the reward, and where the job hands in.
    detail: String,
    /// The refusal line, or `None` when the row's action is allowed.
    refusal: Option<String>,
}

/// Why a hand-in of the job titled `title` is refused; the list and the
/// handler share it.
fn hand_in_refusal_text(
    title: &str,
    refusal: &JobHandInRefusalType,
    station_name: &dyn Fn(&GeneratedBodyId) -> String,
) -> String {
    match refusal {
        JobHandInRefusalType::NotDocked => "hand in: dock at a station that posts jobs".to_string(),
        JobHandInRefusalType::WrongStation { needed } => {
            format!("hand in at {}", station_name(needed))
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

/// Why an accept is refused.
fn accept_refusal_text(refusal: &JobAcceptRefusalType) -> String {
    match refusal {
        JobAcceptRefusalType::NotDocked => "accept: dock at a station that posts jobs".to_string(),
        JobAcceptRefusalType::NotPostedHere => "accept: this station did not post it".to_string(),
        JobAcceptRefusalType::AlreadyHeld => "accept: you hold it already".to_string(),
        JobAcceptRefusalType::AlreadyDone => "accept: you completed it already".to_string(),
        JobAcceptRefusalType::TargetGone => "accept: its target is gone".to_string(),
    }
}

/// Whether the body a recovery or a neutralize names still qualifies: the
/// wreck carries the whole stack, or the hull stands un-neutralized. Read
/// for those two kinds only; every other kind reads `true`.
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

/// The docked station the lists and the handler plan against: its entity
/// and its id, while it is a living station offering Jobs.
fn jobs_partner(
    site: &StationSite,
    q_ids: &Query<&GeneratedBodyId>,
) -> Option<(Entity, Entity, GeneratedBodyId)> {
    let resolved = site.resolve(StationServiceType::Jobs)?;
    let (station, true) = resolved.partner? else {
        return None;
    };
    let id = q_ids.get(station).ok()?.clone();
    Some((resolved.player, station, id))
}

/// Build the two job lists under the escort list: a header and the rows
/// [`update_station_job_lists`] fills, for the board and for the held jobs.
pub(crate) fn spawn_station_job_lists(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            StationJobList,
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|list| {
            for (kind, header) in [
                (StationJobListType::Posted, "Jobs posted here"),
                (StationJobListType::Held, "Your jobs"),
            ] {
                list.spawn(crate::pane::divider());
                list.spawn(themed_label(header, 13.0, UiColor::Accent));
                list.spawn((kind, StationJobRows::default(), job_rows()));
            }
        });
}

/// The column of job rows inside a list.
fn job_rows() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(10),
        ..default()
    }
}

/// One job row: title, detail, a refusal and the action button.
fn job_row() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::FlexStart,
        row_gap: px(4),
        ..default()
    }
}

/// Show the job lists while the jobs gate holds or the player holds a job,
/// and respawn their rows when a job, a reward or a refusal changes.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the board, the jobs, the site, the targets, the hold, both balances and the lists"
)]
pub(crate) fn update_station_job_lists(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    chart: Res<Chart>,
    jobs: Res<Jobs>,
    site: StationSite,
    q_ids: Query<&GeneratedBodyId>,
    q_targets: JobTargets,
    q_boards: Query<&JobBoard>,
    q_credits: Query<&ShipCredits>,
    q_holds: Query<&ShipInventory>,
    mut q_list: Query<&mut Node, With<StationJobList>>,
    mut q_rows: Query<(Entity, &StationJobListType, &mut StationJobRows)>,
) {
    let Ok(mut list) = q_list.single_mut() else {
        return;
    };
    let Some(items) = items else {
        return;
    };
    let partner = jobs_partner(&site, &q_ids);
    let station_name = |id: &GeneratedBodyId| {
        chart
            .station(id)
            .and_then(|station| station.summary.as_ref())
            .map_or_else(|| id.to_string(), |summary| summary.name.clone())
    };
    let mut posted = Vec::new();
    let mut held = Vec::new();
    if let Some((player, station, station_id)) = &partner {
        if let Ok(board) = q_boards.get(*station) {
            posted = board
                .0
                .iter()
                .filter(|job| !jobs.holds(&job.id) && !jobs.is_completed(&job.id))
                .map(|job| JobRow {
                    id: job.id.clone(),
                    title: job.title(&items),
                    detail: format!("Reward {}", cr_text(job.reward_cr)),
                    refusal: plan_job_accept(
                        job,
                        Some(station_id),
                        &jobs,
                        target_standing(job, &q_targets),
                    )
                    .err()
                    .map(|refusal| accept_refusal_text(&refusal)),
                })
                .collect();
        }
        let hand_in_partner = q_credits
            .get(*station)
            .ok()
            .map(|credits| JobHandInPartner {
                id: station_id.clone(),
                credits_cr: credits.0,
            });
        let own_cr = q_credits.get(*player).map_or(0, |credits| credits.0);
        held = jobs
            .held()
            .iter()
            .map(|job| {
                let title = job.title(&items);
                let refusal = q_holds.get(*player).ok().and_then(|hold| {
                    plan_job_hand_in(
                        job,
                        hand_in_partner.as_ref(),
                        hold,
                        own_cr,
                        jobs.is_done(&job.id),
                    )
                    .err()
                    .map(|refusal| hand_in_refusal_text(&title, &refusal, &station_name))
                });
                JobRow {
                    id: job.id.clone(),
                    detail: format!(
                        "Reward {}, hand in at {}",
                        cr_text(job.reward_cr),
                        station_name(&job.target)
                    ),
                    title,
                    refusal,
                }
            })
            .collect();
    }
    let display = if posted.is_empty() && held.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    if list.display != display {
        list.display = display;
    }
    for (rows_entity, kind, mut shown) in &mut q_rows {
        let (rows, verb, prefix) = match kind {
            StationJobListType::Posted => (&posted, "Accept", "JobAccept"),
            StationJobListType::Held => (&held, "Hand in", "JobHandIn"),
        };
        if shown.0 == *rows {
            continue;
        }
        commands.entity(rows_entity).despawn_children();
        commands.entity(rows_entity).with_children(|parent| {
            for (index, row) in rows.iter().enumerate() {
                parent.spawn(job_row()).with_children(|cell| {
                    cell.spawn(themed_label(&row.title, 13.0, UiColor::Primary));
                    cell.spawn(themed_label(&row.detail, 12.0, UiColor::Body));
                    if let Some(refusal) = &row.refusal {
                        cell.spawn(themed_label(refusal, 12.0, UiColor::Danger));
                    } else {
                        cell.spawn((
                            Name::new(format!("{prefix}-{index}")),
                            compact_button(ButtonSpec::new(verb).fit().primary()),
                        ))
                        .observe(on_job_button(*kind, row.id.clone()));
                    }
                });
            }
        });
        shown.0 = rows.clone();
    }
}

/// Raise the list's command for the job `id` when its button is clicked.
fn on_job_button(
    kind: StationJobListType,
    id: JobId,
) -> impl FnMut(
    On<Activate>,
    MessageWriter<JobAcceptCommand>,
    MessageWriter<JobHandInCommand>,
    Option<Res<SoundBank<UiSfx>>>,
    Commands,
) {
    move |_activate, mut accepts, mut hand_ins, bank, mut commands| {
        match kind {
            StationJobListType::Posted => {
                accepts.write(JobAcceptCommand { id: id.clone() });
            }
            StationJobListType::Held => {
                hand_ins.write(JobHandInCommand { id: id.clone() });
            }
        }
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Apply [`JobAcceptCommand`]s by the [`plan_job_accept`] rule and
/// [`JobHandInCommand`]s by the [`plan_job_hand_in`] rule: an accept holds
/// the job and raises [`JobAccepted`]; a hand-in takes a procurement's goods
/// out of the hold, pays the reward with `JobReward` credit changes on both
/// sides and completes the job. A refusal changes nothing and shows its
/// reason on the note line.
///
/// # Panics
///
/// On an accept of a job the docked station's board does not post, or a
/// hand-in of a job the player does not hold: the lists only show a button
/// for a job they list.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading both commands, the board, the jobs, the site, the targets, the hold and both balances"
)]
#[expect(
    clippy::type_complexity,
    reason = "both balances and report queues in one query"
)]
pub(crate) fn apply_station_job_commands(
    mut commands: Commands,
    mut accepts: MessageReader<JobAcceptCommand>,
    mut hand_ins: MessageReader<JobHandInCommand>,
    mut runtime: ResMut<StationRuntime>,
    items: Option<Res<GameItems>>,
    chart: Res<Chart>,
    mut jobs: ResMut<Jobs>,
    site: StationSite,
    q_ids: Query<&GeneratedBodyId>,
    q_boards: Query<&JobBoard>,
    mut q_holds: ParamSet<(JobTargets, Query<&mut ShipInventory>)>,
    mut q_credits: ParamSet<(
        Query<&ShipCredits>,
        Query<(&mut ShipCredits, &mut ShipInventoryReports)>,
    )>,
) {
    if accepts.is_empty() && hand_ins.is_empty() {
        return;
    }
    let Some(items) = items else {
        accepts.clear();
        hand_ins.clear();
        return;
    };
    let partner = jobs_partner(&site, &q_ids);
    let station_name = |id: &GeneratedBodyId| {
        chart
            .station(id)
            .and_then(|station| station.summary.as_ref())
            .map_or_else(|| id.to_string(), |summary| summary.name.clone())
    };
    for command in accepts.read() {
        let Some((_, station, station_id)) = &partner else {
            runtime.say(accept_refusal_text(&JobAcceptRefusalType::NotDocked));
            continue;
        };
        let board = q_boards
            .get(*station)
            .unwrap_or_else(|_| panic!("accept: the docked station {station:?} posts no board"));
        let job = board
            .job(&command.id)
            .unwrap_or_else(|| panic!("accept: the docked station posts no job '{}'", command.id));
        match plan_job_accept(
            job,
            Some(station_id),
            &jobs,
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
                &title,
                &JobHandInRefusalType::NotDocked,
                &station_name,
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
                runtime.say(hand_in_refusal_text(&title, &refusal, &station_name));
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
    }
}
