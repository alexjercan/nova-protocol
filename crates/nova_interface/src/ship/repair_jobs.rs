//! The Ship pane's Repairs block: one row per running repair job of the
//! player ship, with its progress bar, its time left and Cancel.
//!
//! The jobs themselves live in `nova_ship` ([`ShipRepairJobs`]) and run on
//! the game clock, so the TAB interface that shows this block holds them. A
//! Cancel raises a [`RepairJobCancelRequest`] and the row goes; plates a
//! plate job already fitted stay fitted.
//!
//! Touch this module when changing how the pane shows or cancels a job.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    prelude::{bar, button, control_row, metrics, spawn_block, text, Bar, TextRole},
    theme::UiColorType,
    widget::ButtonSpec,
};

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

/// A row's progress bar, by the index of its job.
#[derive(Component, Clone, Copy)]
pub(crate) struct ShipRepairJobBar(usize);

/// Build the Repairs block: the rows [`update_ship_repair_jobs`] fills.
pub(crate) fn spawn_ship_repair_jobs(panel: &mut ChildSpawnerCommands) {
    let mut block = spawn_block(
        panel,
        "Repairs",
        (ShipRepairJobsBlock, Name::new("ShipRepairJobs")),
    );
    block
        .entry::<Node>()
        .and_modify(|mut node| node.display = Display::None);
    block.with_children(|block| {
        block.spawn((
            ShipRepairJobRows::default(),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(metrics::GAP),
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
    mut q_block: Query<&mut Node, With<ShipRepairJobsBlock>>,
    mut q_rows: Query<(Entity, &mut ShipRepairJobRows)>,
    mut q_text: Query<(&ShipRepairJobField, &mut Text)>,
    mut q_bar: Query<(&ShipRepairJobBar, &mut Bar)>,
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
    for (index, mut bar) in &mut q_bar {
        if let Some(job) = jobs.get(index.0) {
            let progress = job.progress();
            if bar.0 != progress {
                bar.0 = progress;
            }
        }
    }
}

/// One job row: the label, the time left and Cancel over the progress bar.
fn spawn_job_row(parent: &mut ChildSpawnerCommands, player: Entity, index: usize, job: &RepairJob) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(metrics::GAP_S),
            ..default()
        })
        .with_children(|row| {
            row.spawn(control_row(JustifyContent::FlexStart))
                .with_children(|line| {
                    line.spawn((
                        ShipRepairJobField::Label(index),
                        text(&repair_job_label(job), TextRole::Value),
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
                        text("", TextRole::Body),
                        TextLayout::new(Justify::Right, LineBreak::NoWrap),
                        Node {
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                    line.spawn((
                        Name::new(format!("ShipRepairCancel-{}", job.code)),
                        button(ButtonSpec::control("Cancel").ghost()),
                    ))
                    .observe(on_cancel_button(player, job.key()));
                });
            row.spawn((
                ShipRepairJobBar(index),
                bar(job.progress(), UiColorType::Accent),
            ));
        });
}

/// Cancel the job on `key`'s section of `player` when its Cancel button is
/// clicked.
fn on_cancel_button(player: Entity, key: RepairJobKey) -> impl FnMut(On<Activate>, Commands) {
    move |_activate, mut commands| {
        commands.trigger(RepairJobCancelRequest {
            entity: player,
            key: key.clone(),
        });
    }
}
