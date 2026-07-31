use bevy::prelude::*;
use bevy_common_systems::prelude::{GameObjectives, Objective};
use nova_os::prelude::*;
use nova_ui::theme;

use super::{components::*, content::*, style::*};
use crate::{prelude::*, PauseStates};

/// Update the NOVA OS's combined left-panel flight log from the story feed and
/// active objective list.
pub(crate) fn sync_nova_os_logs(
    story: Res<StoryFeed>,
    objectives: Res<GameObjectives>,
    mut log: ResMut<NovaOsFlightLog>,
) {
    if story.0.len() < log.seen_story {
        log.clear();
    }

    for line in story.0.iter().skip(log.seen_story) {
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::Comms,
            objective_id: None,
            speaker: Some(line.speaker.clone()),
            message: line.text.clone(),
            icon: line.icon.clone(),
        });
    }
    log.seen_story = story.0.len();

    let completed: Vec<Objective> = log
        .previous_active
        .iter()
        .filter(|old| {
            !objectives
                .objectives
                .iter()
                .any(|current| current.id == old.id)
        })
        .cloned()
        .collect();
    for objective in completed {
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::ObjectiveCompleted,
            objective_id: Some(objective.id.clone()),
            speaker: None,
            message: objective.message.clone(),
            icon: None,
        });
        log.active_objective_entries
            .retain(|entry| entry.id != objective.id);
    }

    for objective in &objectives.objectives {
        if let Some(active) = log
            .active_objective_entries
            .iter()
            .find(|entry| entry.id == objective.id)
            .cloned()
        {
            if let Some(entry) = log.entries.get_mut(active.entry_index) {
                entry.message = objective.message.clone();
            }
            continue;
        }

        let entry_index = log.entries.len();
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::ObjectivePosted,
            objective_id: Some(objective.id.clone()),
            speaker: None,
            message: objective.message.clone(),
            icon: None,
        });
        log.active_objective_entries
            .push(NovaOsFlightLogActiveObjective {
                id: objective.id.clone(),
                entry_index,
            });
    }

    log.previous_active = objectives.objectives.clone();
}

/// Announce objective flips into the LIVE terminal scrollback while the computer
/// is open at the prompt (PoC `checkObjectives` pushes an `OBJ x ...` line the
/// moment an objective completes, so the player sees it without typing `log`).
/// Only completions that happen while open are announced; ones that flipped while
/// the computer was closed stay in the flight log (counted by the boot banner's
/// unread-events line instead of dumping on open).
pub(crate) fn announce_objectives_in_terminal(
    log: Res<NovaOsFlightLog>,
    pause: Res<State<PauseStates>>,
    mut terminal: ResMut<NovaOsTerminal>,
    mut announced: Local<Option<usize>>,
) {
    let total = log.entries.len();
    // `None` on the first run (and `min` if the log was cleared) means we start
    // from "everything already seen" - nothing is announced retroactively.
    let from = announced.unwrap_or(total).min(total);
    let open =
        *pause.get() == PauseStates::NovaOs && terminal.active_mode() == TerminalMode::Prompt;
    if open {
        let fresh: Vec<TerminalRow> = log.entries[from..]
            .iter()
            .filter(|entry| entry.kind == NovaOsFlightLogEntryKind::ObjectiveCompleted)
            .map(|entry| TerminalRow {
                kind: TerminalRowKind::Info,
                text: nova_os_flight_log_text(entry),
            })
            .collect();
        // Only touch the scrollback (and so mark the terminal changed, forcing a
        // rebuild that snaps the view to the bottom) when there is actually
        // something to announce - most objective-change frames have no completion.
        if !fresh.is_empty() {
            terminal.extend_scrollback(fresh);
        }
    }
    *announced = Some(total);
}

/// Rebuild the right objectives-section rows from the active objectives list.
pub(crate) fn rebuild_nova_os_objectives(
    mut commands: Commands,
    objectives: Res<GameObjectives>,
    q_list: Query<(Entity, Option<&Children>), With<NovaOsObjectivesListMarker>>,
) {
    let Ok((list, children)) = q_list.single() else {
        return;
    };
    if let Some(children) = children {
        for &child in children {
            commands.entity(child).despawn();
        }
    }
    commands.entity(list).with_children(|parent| {
        if objectives.objectives.is_empty() {
            spawn_nova_os_empty_objective_row(parent);
            return;
        }
        for objective in &objectives.objectives {
            spawn_nova_os_objective_row(parent, objective);
        }
    });
}

pub(crate) fn spawn_nova_os_empty_objective_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Name::new("NovaOsObjectiveEmpty"),
            NovaOsObjectiveEmptyMarker,
            Node {
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                ..default()
            },
            BorderColor::all(theme::PHOSPHOR_MUTED),
            BackgroundColor(theme::SCREEN_1.with_alpha(0.45)),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new("No active objectives."),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::PHOSPHOR_DIM),
            ));
        });
}

pub(crate) fn spawn_nova_os_objective_row(
    parent: &mut ChildSpawnerCommands,
    objective: &Objective,
) {
    parent
        .spawn((
            Name::new(format!("NovaOsObjective {}", objective.id)),
            NovaOsObjectiveRowMarker,
            NovaOsObjectiveId(objective.id.clone()),
            NovaOsObjectiveRowStatus::Active,
            Node {
                min_height: Val::Px(34.0),
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(DRAWER_ROW_GAP_PX),
                ..default()
            },
            BorderColor::all(theme::PHOSPHOR_DIM),
            BackgroundColor(theme::SCREEN_1),
        ))
        .with_children(|row| {
            row.spawn((
                NovaOsObjectiveGlyphMarker,
                Text::new(">"),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::semantic::OBJECTIVE),
                Node {
                    width: Val::Px(DRAWER_OBJECTIVE_GLYPH_WIDTH_PX),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            row.spawn(Node {
                position_type: PositionType::Relative,
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|text_wrap| {
                text_wrap.spawn((
                    NovaOsObjectiveTextMarker,
                    Text::new(objective.message.clone()),
                    TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                    TextLayout {
                        justify: Justify::Left,
                        linebreak: LineBreak::WordBoundary,
                    },
                    TextColor(theme::SCREEN_TEXT),
                ));
            });
        });
}

/// Rebuild the left combined flight-log stream.
pub(crate) fn rebuild_nova_os_flight_log(
    mut commands: Commands,
    log: Res<NovaOsFlightLog>,
    asset_server: Option<Res<AssetServer>>,
    q_list: Query<(Entity, Option<&Children>), With<NovaOsFlightLogListMarker>>,
) {
    let Ok((list, children)) = q_list.single() else {
        return;
    };
    if let Some(children) = children {
        for &child in children {
            commands.entity(child).despawn();
        }
    }
    commands.entity(list).with_children(|parent| {
        if log.entries.is_empty() {
            spawn_nova_os_empty_flight_log_row(parent);
            return;
        }
        for entry in &log.entries {
            spawn_nova_os_flight_log_row(parent, entry, asset_server.as_deref());
        }
    });
}

pub(crate) fn spawn_nova_os_empty_flight_log_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Name::new("NovaOsFlightLogEmpty"),
            NovaOsFlightLogEmptyMarker,
            Node {
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                ..default()
            },
            BorderColor::all(theme::PHOSPHOR_MUTED),
            BackgroundColor(theme::SCREEN_1.with_alpha(0.45)),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new("No log entries."),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::PHOSPHOR_DIM),
            ));
        });
}

pub(crate) fn spawn_nova_os_flight_log_row(
    parent: &mut ChildSpawnerCommands,
    entry: &NovaOsFlightLogEntry,
    asset_server: Option<&AssetServer>,
) {
    let icon_kind = match entry.kind {
        NovaOsFlightLogEntryKind::Comms if entry.icon.is_some() => {
            NovaOsFlightLogIconKind::CommsAuthored
        }
        NovaOsFlightLogEntryKind::Comms => NovaOsFlightLogIconKind::Fallback,
        NovaOsFlightLogEntryKind::ObjectivePosted
        | NovaOsFlightLogEntryKind::ObjectiveCompleted => NovaOsFlightLogIconKind::Objective,
    };
    let accent = match entry.kind {
        NovaOsFlightLogEntryKind::Comms => theme::BLUE,
        NovaOsFlightLogEntryKind::ObjectivePosted => theme::semantic::OBJECTIVE,
        NovaOsFlightLogEntryKind::ObjectiveCompleted => theme::semantic::ALLY,
    };

    parent
        .spawn((
            Name::new("NovaOsFlightLogRow"),
            NovaOsFlightLogRowMarker,
            NovaOsFlightLogIconMarker { kind: icon_kind },
            Node {
                min_height: Val::Px(30.0),
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(DRAWER_ROW_GAP_PX),
                ..default()
            },
            BorderColor::all(theme::PHOSPHOR_MUTED),
            BackgroundColor(theme::SCREEN_1.with_alpha(0.58)),
        ))
        .with_children(|row| {
            spawn_nova_os_flight_log_icon(row, entry, icon_kind, accent, asset_server);
            row.spawn((
                NovaOsFlightLogTextMarker,
                Text::new(nova_os_flight_log_text(entry)),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::SCREEN_TEXT),
                TextLayout {
                    justify: Justify::Left,
                    linebreak: LineBreak::WordBoundary,
                },
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ));
        });
}

pub(crate) fn spawn_nova_os_flight_log_icon(
    row: &mut ChildSpawnerCommands,
    entry: &NovaOsFlightLogEntry,
    icon_kind: NovaOsFlightLogIconKind,
    accent: Color,
    asset_server: Option<&AssetServer>,
) {
    let node = Node {
        width: Val::Px(DRAWER_LOG_ICON_SIZE_PX),
        height: Val::Px(DRAWER_LOG_ICON_SIZE_PX),
        min_width: Val::Px(DRAWER_LOG_ICON_SIZE_PX),
        border: UiRect::all(Val::Px(theme::BORDER_W)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        flex_shrink: 0.0,
        ..default()
    };
    match (&entry.icon, icon_kind) {
        (Some(icon), NovaOsFlightLogIconKind::CommsAuthored) => {
            row.spawn((
                node,
                ImageNode::new(
                    asset_server
                        .map(|server| icon.resolve(server))
                        .unwrap_or_default(),
                ),
                BorderColor::all(accent),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ));
        }
        _ => {
            row.spawn((
                node,
                BorderColor::all(accent),
                BackgroundColor(accent.with_alpha(0.16)),
            ))
            .with_children(|icon| {
                icon.spawn((
                    Text::new(match icon_kind {
                        NovaOsFlightLogIconKind::Objective => ">",
                        NovaOsFlightLogIconKind::CommsAuthored
                        | NovaOsFlightLogIconKind::Fallback => "#",
                    }),
                    TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                    TextColor(accent),
                ));
            });
        }
    }
}
