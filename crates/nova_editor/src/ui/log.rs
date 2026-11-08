//! The Log tab: what the status line said, and what the log warned of.
//!
//! Two feeds fill one list. Every line a verb says through [`EditorStatus`]
//! lands here once, and so does every WARN and ERROR the app logs, through
//! [`EditorLogFeed`]. INFO and DEBUG logging stays in the terminal.

use std::collections::VecDeque;

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_ui::{prelude::*, theme::UiColor};

use super::{
    browser::{pane_text, BrowserTabType},
    shell::EditorPanels,
};
use crate::{
    config::EditorStatus,
    log_feed::{EditorLogFeed, FedLine},
};

/// How many lines the Log tab remembers.
const LOG_LINES: usize = 200;

/// How bad a log line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum LogLevelType {
    Info,
    Warning,
    Error,
}

/// Which lines the Log tab shows. Each level keeps the ones worse than it too.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LogFilterType {
    /// Every line.
    #[default]
    All,
    /// Warnings and errors.
    Warnings,
    /// Errors alone.
    Errors,
}

impl LogFilterType {
    const ALL: [Self; 3] = [Self::All, Self::Warnings, Self::Errors];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Warnings => "Warnings",
            Self::Errors => "Errors",
        }
    }

    fn keeps(self, level: LogLevelType) -> bool {
        match self {
            Self::All => true,
            Self::Warnings => level >= LogLevelType::Warning,
            Self::Errors => level == LogLevelType::Error,
        }
    }
}

struct LogLine {
    level: LogLevelType,
    text: String,
    tint: UiColor,
}

/// What the Log tab holds, oldest first, and how it is filtered.
#[derive(Resource, Default)]
pub(crate) struct StatusLog {
    lines: VecDeque<LogLine>,
    /// The status line last written down, so a line held for seconds is one
    /// line. Clearing keeps it: a line still on the bar has been read.
    last: String,
    /// Moves when the rows have to be drawn again.
    revision: u64,
    /// Moves when the list has to go back to its newest line.
    returned: u64,
    filter: LogFilterType,
    text: String,
}

impl StatusLog {
    fn push(&mut self, level: LogLevelType, text: String, tint: UiColor) {
        self.lines.push_back(LogLine { level, text, tint });
        if self.lines.len() > LOG_LINES {
            self.lines.pop_front();
        }
        self.revision += 1;
    }

    /// Write down what the status line says, once however long it holds.
    fn hear_status(&mut self, said: Option<(&str, UiColor)>) {
        let Some((line, tint)) = said else {
            self.last.clear();
            return;
        };
        if self.last == line {
            return;
        }
        line.clone_into(&mut self.last);
        let level = match tint {
            UiColor::Danger => LogLevelType::Error,
            UiColor::Accent => LogLevelType::Warning,
            _ => LogLevelType::Info,
        };
        self.push(level, line.to_string(), tint);
    }

    /// Write down a line the log warned of.
    fn hear_log(&mut self, line: FedLine) {
        let (level, tint) = if line.error {
            (LogLevelType::Error, UiColor::Danger)
        } else {
            (LogLevelType::Warning, UiColor::Accent)
        };
        self.push(level, line.text, tint);
    }

    fn clear(&mut self) {
        self.lines.clear();
        self.revision += 1;
        self.returned += 1;
    }

    fn set_filter(&mut self, filter: LogFilterType) {
        if self.filter != filter {
            self.filter = filter;
            self.revision += 1;
            self.returned += 1;
        }
    }

    fn set_text(&mut self, text: &str) {
        if self.text != text {
            text.clone_into(&mut self.text);
            self.revision += 1;
            self.returned += 1;
        }
    }

    /// The lines the filters keep, newest first.
    fn shown(&self) -> impl Iterator<Item = &LogLine> {
        let wanted = self.text.to_lowercase();
        self.lines.iter().rev().filter(move |line| {
            self.filter.keeps(line.level) && line.text.to_lowercase().contains(&wanted)
        })
    }

    /// The count beside the controls.
    fn count(&self) -> String {
        match (self.shown().count(), self.lines.len()) {
            (1, 1) => "1 line".to_string(),
            (shown, total) if shown == total => format!("{total} lines"),
            (shown, total) => format!("{shown} of {total}"),
        }
    }

    /// Every remembered line, oldest first.
    pub(crate) fn texts(&self) -> Vec<String> {
        self.lines.iter().map(|line| line.text.clone()).collect()
    }

    /// How many rows the filters leave.
    pub(crate) fn shown_count(&self) -> usize {
        self.shown().count()
    }
}

/// The list the Log pane fills.
#[derive(Component)]
pub(crate) struct LogList;

/// A control shown on the Log tab alone.
#[derive(Component)]
pub(crate) struct LogTabControl;

/// The field the Log tab filters by.
#[derive(Component)]
pub(crate) struct LogFilterField;

/// The count of lines.
#[derive(Component)]
pub(crate) struct LogCount;

/// A level button.
#[derive(Component, Clone, Copy)]
pub(crate) struct LogLevelButton(LogFilterType);

/// The tab row's controls for this tab.
pub(super) fn spawn_log_controls(row: &mut RelatedSpawnerCommands<ChildOf>) {
    row.spawn((
        Name::new("Log Count"),
        LogTabControl,
        LogCount,
        pane_text("0 lines", 11.0, UiColor::Label),
    ));
    for filter in LogFilterType::ALL {
        row.spawn((
            Name::new(format!("Log Level {}", filter.label())),
            LogTabControl,
            LogLevelButton(filter),
            button(ButtonSpec::new(filter.label()).ghost().dense().fit()),
            observe(on_log_level),
        ));
    }
    row.spawn((
        Name::new("Log Filter Slot"),
        LogTabControl,
        Node {
            width: px(170),
            ..default()
        },
    ))
    .with_child((
        Name::new("Log Filter"),
        LogFilterField,
        text_field(
            TextFieldSpec::new("")
                .placeholder("filter log")
                .max_chars(40)
                .tool(),
        ),
    ));
    row.spawn((
        Name::new("Log Clear"),
        LogTabControl,
        button(ButtonSpec::new("Clear").ghost().dense().fit()),
        observe(on_log_clear),
    ));
}

fn on_log_level(
    activate: On<Activate>,
    buttons: Query<&LogLevelButton>,
    mut log: ResMut<StatusLog>,
) {
    if let Ok(LogLevelButton(filter)) = buttons.get(activate.entity) {
        log.set_filter(*filter);
    }
}

fn on_log_clear(_activate: On<Activate>, mut log: ResMut<StatusLog>) {
    log.clear();
}

/// Show the controls on the Log tab, and say what they hold.
pub(crate) fn sync_log_controls(
    mut commands: Commands,
    open: Res<BrowserTabType>,
    panels: Res<EditorPanels>,
    log: Res<StatusLog>,
    mut controls: Query<&mut Node, With<LogTabControl>>,
    levels: Query<(Entity, &LogLevelButton, Has<Selected>)>,
    mut counts: Query<&mut Text, With<LogCount>>,
) {
    let display = if *open == BrowserTabType::Log && panels.browser {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut controls {
        if node.display != display {
            node.display = display;
        }
    }
    for (button, level, marked) in &levels {
        match (level.0 == log.filter, marked) {
            (true, false) => {
                commands.entity(button).insert(Selected);
            }
            (false, true) => {
                commands.entity(button).remove::<Selected>();
            }
            _ => {}
        }
    }
    let count = log.count();
    for mut text in &mut counts {
        if text.0 != count {
            text.0.clone_from(&count);
        }
    }
}

/// Take in both feeds, and draw the Log tab while it is open.
///
/// The newest line is the top row, so a list at rest keeps it in view. A
/// builder who scrolled down to read stays there until the list changes under
/// a control or the tab is opened again.
#[expect(
    clippy::too_many_arguments,
    reason = "two feeds, the list, its scroll and the field that filters it"
)]
pub(crate) fn sync_log(
    mut commands: Commands,
    status: Res<EditorStatus>,
    feed: Option<Res<EditorLogFeed>>,
    open: Res<BrowserTabType>,
    mut log: ResMut<StatusLog>,
    lists: Query<Entity, With<LogList>>,
    fresh: Query<(), Added<LogList>>,
    fields: Query<&TextFieldValue, With<LogFilterField>>,
    mut scrolls: Query<&mut ScrollPosition, With<LogList>>,
    mut shown: Local<Option<u64>>,
    mut returned: Local<u64>,
) {
    if !fresh.is_empty() {
        *shown = None;
    }
    log.hear_status(status.said());
    for line in feed.iter().flat_map(|feed| feed.drain()) {
        log.hear_log(line);
    }
    if let Ok(field) = fields.single() {
        log.set_text(&field.0);
    }
    if *open != BrowserTabType::Log {
        return;
    }
    if *returned != log.returned || open.is_changed() {
        *returned = log.returned;
        for mut scroll in &mut scrolls {
            scroll.y = 0.0;
        }
    }
    if *shown == Some(log.revision) {
        return;
    }
    let Ok(list) = lists.single() else {
        return;
    };
    commands.entity(list).despawn_related::<Children>();
    commands.entity(list).with_children(|list| {
        for (index, line) in log.shown().enumerate() {
            list.spawn((
                Name::new(format!("Log Row {index}")),
                pane_text(&line.text, 12.0, line.tint),
            ));
        }
    });
    *shown = Some(log.revision);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fed(error: bool, text: &str) -> FedLine {
        FedLine {
            error,
            text: text.to_string(),
        }
    }

    fn filled() -> StatusLog {
        let mut log = StatusLog::default();
        log.hear_status(Some(("saved scenario", UiColor::Nominal)));
        log.hear_log(fed(false, "bevy_ecs: stale entity"));
        log.hear_log(fed(true, "nova_editor: the file broke"));
        log
    }

    fn rows(log: &StatusLog) -> Vec<&str> {
        log.shown().map(|line| line.text.as_str()).collect()
    }

    #[test]
    fn a_level_keeps_itself_and_what_is_worse() {
        let mut log = filled();

        log.set_filter(LogFilterType::Warnings);
        assert_eq!(
            rows(&log),
            ["nova_editor: the file broke", "bevy_ecs: stale entity"]
        );

        log.set_filter(LogFilterType::Errors);
        assert_eq!(rows(&log), ["nova_editor: the file broke"]);
    }

    #[test]
    fn the_text_filter_ignores_case_and_the_count_says_how_many_it_left() {
        let mut log = filled();

        log.set_text("SAVED");

        assert_eq!(rows(&log), ["saved scenario"]);
        assert_eq!(log.count(), "1 of 3");
    }

    #[test]
    fn clearing_empties_the_list_and_later_lines_still_arrive() {
        let mut log = filled();

        log.clear();
        assert!(rows(&log).is_empty());
        log.hear_status(Some(("saved scenario", UiColor::Nominal)));
        assert!(rows(&log).is_empty(), "the bar still holds the same line");
        log.hear_log(fed(false, "bevy_ecs: stale entity"));

        assert_eq!(rows(&log), ["bevy_ecs: stale entity"]);
    }

    #[test]
    fn a_status_line_said_again_after_it_left_the_bar_is_a_new_line() {
        let mut log = StatusLog::default();

        log.hear_status(Some(("nothing to undo", UiColor::Danger)));
        log.hear_status(Some(("nothing to undo", UiColor::Danger)));
        log.hear_status(None);
        log.hear_status(Some(("nothing to undo", UiColor::Danger)));

        assert_eq!(rows(&log).len(), 2);
    }
}
