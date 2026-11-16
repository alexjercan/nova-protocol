//! The Ship pane's Repairs block: one row per running repair job of the
//! player ship, with its progress bar, its time left and Cancel.
//!
//! The jobs themselves live in `nova_ship` ([`ShipRepairJobs`]) and run on
//! the game clock, so the TAB interface that shows this block holds them.
//! The block says so in one line. A Cancel raises a
//! [`RepairJobCancelRequest`]; plates a plate job already fitted stay fitted.
//!
//! Touch this module when changing how the pane shows or cancels a job.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    theme::UiColorType,
    widget::{ButtonSpec, ThemedFill, ThemedRadius},
};

use super::ShipRuntime;
use crate::pane::{compact_button, control_row, divider, themed_label};

/// The Repairs block, hidden while the player ship runs no job.
#[derive(Component)]
pub(crate) struct ShipRepairJobsBlock;

/// The rows inside [`ShipRepairJobsBlock`], and the job keys they were built
/// for, so the rows respawn only when a job starts or ends.
#[derive(Component, Default)]
pub(crate) struct ShipRepairJobRows(Vec<RepairJobKey>);

/// A row's live part, by the index of its job in [`ShipRepairJobs`].
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShipRepairJobField {
    /// The section code and what the job does (`HULL-3  plate 2/5`).
    Label(usize),
    /// The game seconds left.
    Time(usize),
}

/// A row's progress bar fill, by the index of its job.
#[derive(Component, Clone, Copy)]
pub(crate) struct ShipRepairJobFill(usize);

/// The line under the block's title. The jobs run on the game clock, which
/// this pane stops. Kept short enough for one line at the panel's width.
pub(crate) const SHIP_REPAIR_JOBS_NOTE: &str = "Jobs run only while you fly.";

/// Build the Repairs block: a title, the clock note, and the rows
/// [`update_ship_repair_jobs`] fills.
pub(crate) fn spawn_ship_repair_jobs(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            ShipRepairJobsBlock,
            Name::new("ShipRepairJobs"),
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|block| {
            block.spawn(divider());
            block.spawn(themed_label("Repairs", 13.0, UiColorType::Accent));
            block.spawn(themed_label(
                SHIP_REPAIR_JOBS_NOTE,
                12.0,
                UiColorType::TextBody,
            ));
            block.spawn((
                ShipRepairJobRows::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ));
        });
}

/// The row label of `job`: its code and what it does, such as
/// `HULL-3  plate 2/5` or `DOCK-1  field rebuild`.
pub(crate) fn repair_job_label(job: &RepairJob) -> String {
    match &job.work {
        RepairWorkType::Plates { queued, spent, .. } => {
            format!("{}  plate {}/{queued}", job.code, (spent + 1).min(*queued))
        }
        RepairWorkType::FieldRebuild { .. } => format!("{}  field rebuild", job.code),
    }
}

/// Show the Repairs block while the player ship runs a job, respawn its rows
/// when a job starts or ends, and fill each row's label, bar and time.
#[expect(
    clippy::type_complexity,
    reason = "the block, its rows, their texts and their fills"
)]
pub(crate) fn update_ship_repair_jobs(
    mut commands: Commands,
    q_player: Query<
        (Entity, Option<&ShipRepairJobs>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    mut q_block: Query<&mut Node, (With<ShipRepairJobsBlock>, Without<ShipRepairJobFill>)>,
    mut q_rows: Query<(Entity, &mut ShipRepairJobRows)>,
    mut q_text: Query<(&ShipRepairJobField, &mut Text)>,
    mut q_fill: Query<(&ShipRepairJobFill, &mut Node), Without<ShipRepairJobsBlock>>,
) {
    let Ok(mut block) = q_block.single_mut() else {
        return;
    };
    let Ok((rows_entity, mut shown)) = q_rows.single_mut() else {
        return;
    };
    let (player, jobs) = match q_player.single() {
        Ok((player, jobs)) => (Some(player), jobs.map_or(&[][..], |jobs| jobs.0.as_slice())),
        Err(_) => (None, &[][..]),
    };
    let display = if jobs.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    if block.display != display {
        block.display = display;
    }
    let keys: Vec<RepairJobKey> = jobs.iter().map(RepairJob::key).collect();
    if shown.0 != keys {
        commands.entity(rows_entity).despawn_children();
        if let Some(player) = player {
            commands.entity(rows_entity).with_children(|parent| {
                for (index, job) in jobs.iter().enumerate() {
                    spawn_job_row(parent, player, index, job);
                }
            });
        }
        shown.0 = keys;
        // The new rows are filled on the next frame, once they exist.
        return;
    }
    for (field, mut text) in &mut q_text {
        let value = match *field {
            ShipRepairJobField::Label(index) => jobs.get(index).map(repair_job_label),
            ShipRepairJobField::Time(index) => jobs
                .get(index)
                .map(|job| format!("{:.0} s", job.remaining_secs().ceil())),
        };
        if let Some(value) = value {
            if text.0 != value {
                text.0 = value;
            }
        }
    }
    for (fill, mut node) in &mut q_fill {
        if let Some(job) = jobs.get(fill.0) {
            let width = Val::Percent(job.progress() * 100.0);
            if node.width != width {
                node.width = width;
            }
        }
    }
}

/// One job row: the label, the time left and Cancel over the progress bar.
fn spawn_job_row(parent: &mut ChildSpawnerCommands, player: Entity, index: usize, job: &RepairJob) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn(control_row(JustifyContent::FlexStart))
                .with_children(|line| {
                    line.spawn((
                        ShipRepairJobField::Label(index),
                        themed_label(&repair_job_label(job), 12.0, UiColorType::Text),
                        TextLayout::new(Justify::Left, LineBreak::NoWrap),
                        Node {
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            min_width: Val::Px(0.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                    ));
                    line.spawn((
                        ShipRepairJobField::Time(index),
                        themed_label("", 12.0, UiColorType::TextBody),
                        TextLayout::new(Justify::Right, LineBreak::NoWrap),
                        Node {
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                    line.spawn((
                        Name::new(format!("ShipRepairCancel-{}", job.code)),
                        compact_button(ButtonSpec::new("Cancel").fit().ghost()),
                    ))
                    .observe(on_cancel_button(
                        player,
                        job.key(),
                        job.code.clone(),
                    ));
                });
            row.spawn((
                Node {
                    height: Val::Px(6.0),
                    flex_shrink: 0.0,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ThemedFill::alpha(UiColorType::TextSecondary, 0.25),
                ThemedRadius::control(),
            ))
            .with_children(|track| {
                track.spawn((
                    ShipRepairJobFill(index),
                    Node {
                        width: Val::Percent(job.progress() * 100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ThemedFill::new(UiColorType::Accent),
                ));
            });
        });
}

/// Cancel the job `code` on `key`'s section of `player` when its Cancel
/// button is clicked, and say so on the note line. The open pane holds the
/// clock, so no job ends on its own while the note shows.
fn on_cancel_button(
    player: Entity,
    key: RepairJobKey,
    code: String,
) -> impl FnMut(On<Activate>, Commands, ResMut<ShipRuntime>) {
    move |_activate, mut commands, mut runtime| {
        commands.trigger(RepairJobCancelRequest {
            entity: player,
            key: key.clone(),
        });
        runtime.note = Some((format!("cancelled the {code} job"), 2.5));
    }
}
