//! Copy out of the Log and Problems tabs.
//!
//! A click marks one row, Shift+click the rows from the last click to this one,
//! and Ctrl+A every row the filters show. Ctrl+C and the Copy button copy the
//! marked rows, or every shown row when none is marked, one line a row. A right
//! click on a row copies that line alone. The keys act while the pointer is
//! over the open list, so the stage's own Ctrl+A and Ctrl+C stay the stage's.
//!
//! Rows are keyed, not counted: a Log line keeps its key while newer lines push
//! it down, so a copy's own status line does not move the marks.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_ui::prelude::*;

use super::{
    browser::{BrowserPane, BrowserTabType},
    layer,
    shell::EditorPanels,
};
use crate::{clipboard::copy_lines, project::verbs::pressed, ExampleStates};

/// The lists a copy reads from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CopyListType {
    /// The Log tab.
    Log,
    /// The Problems tab.
    Problems,
}

impl CopyListType {
    /// The list the open tab shows, if it is one of these.
    pub(crate) fn of_tab(tab: BrowserTabType) -> Option<Self> {
        match tab {
            BrowserTabType::Log => Some(Self::Log),
            BrowserTabType::Problems => Some(Self::Problems),
            BrowserTabType::Assets | BrowserTabType::Ship => None,
        }
    }
}

/// One shown row as the text a copy writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ShownRow {
    /// What the row stays known by while the list moves.
    pub(crate) key: u64,
    /// The line a copy writes.
    pub(crate) text: String,
}

/// Which rows of one list are marked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowSelection {
    /// The marked keys, in list order.
    picked: Vec<u64>,
    /// Where a Shift+click range starts: the last plain click.
    anchor: Option<u64>,
}

impl RowSelection {
    /// A click on `key` over the rows `shown`, top first: that row alone, or
    /// with `extend` every row from the anchor to it.
    pub(crate) fn click(&mut self, shown: &[u64], key: u64, extend: bool) {
        let at = |key: u64| shown.iter().position(|held| *held == key);
        match (extend, self.anchor.and_then(at), at(key)) {
            (true, Some(from), Some(to)) => {
                self.picked = shown[from.min(to)..=from.max(to)].to_vec();
            }
            _ => {
                self.picked = vec![key];
                self.anchor = Some(key);
            }
        }
    }

    /// Mark every row in `shown`.
    pub(crate) fn select_all(&mut self, shown: &[u64]) {
        self.picked = shown.to_vec();
        self.anchor = shown.first().copied();
    }

    /// Mark nothing.
    pub(crate) fn clear(&mut self) {
        self.picked.clear();
        self.anchor = None;
    }

    /// Whether `key` is marked.
    pub(crate) fn has(&self, key: u64) -> bool {
        self.picked.contains(&key)
    }

    /// The lines a copy takes from `rows`: the marked ones in list order, or
    /// every row when none of the shown ones is marked.
    pub(crate) fn lines(&self, rows: &[ShownRow]) -> Vec<String> {
        let marked: Vec<String> = rows
            .iter()
            .filter(|row| self.has(row.key))
            .map(|row| row.text.clone())
            .collect();
        if marked.is_empty() {
            rows.iter().map(|row| row.text.clone()).collect()
        } else {
            marked
        }
    }
}

/// One list's shown rows and its marks.
#[derive(Debug, Default)]
pub(crate) struct ShownList {
    rows: Vec<ShownRow>,
    selection: RowSelection,
}

impl ShownList {
    /// The shown keys, top first.
    fn keys(&self) -> Vec<u64> {
        self.rows.iter().map(|row| row.key).collect()
    }

    /// What a copy of this list writes.
    pub(crate) fn lines(&self) -> Vec<String> {
        self.selection.lines(&self.rows)
    }

    /// The text of the row `key`.
    fn text_of(&self, key: u64) -> Option<String> {
        self.rows
            .iter()
            .find(|row| row.key == key)
            .map(|row| row.text.clone())
    }
}

/// The rows the Log and Problems tabs show, as text, and what is marked.
#[derive(Resource, Debug, Default)]
pub(crate) struct CopyRows {
    log: ShownList,
    problems: ShownList,
}

impl CopyRows {
    /// The list `list`.
    pub(crate) fn list(&self, list: CopyListType) -> &ShownList {
        match list {
            CopyListType::Log => &self.log,
            CopyListType::Problems => &self.problems,
        }
    }

    fn list_mut(&mut self, list: CopyListType) -> &mut ShownList {
        match list {
            CopyListType::Log => &mut self.log,
            CopyListType::Problems => &mut self.problems,
        }
    }

    /// The rows `list` now shows. `keep_marks` keeps what is marked: a key
    /// that still names the same line. Off, the marks go with the old rows.
    pub(crate) fn set_rows(&mut self, list: CopyListType, rows: Vec<ShownRow>, keep_marks: bool) {
        let shown = self.list_mut(list);
        shown.rows = rows;
        if !keep_marks {
            shown.selection.clear();
        }
    }
}

/// A row a copy can mark: its list and key.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct CopyRowKey(pub(crate) CopyListType, pub(crate) u64);

/// The Copy button, shown on the Log and Problems tabs.
#[derive(Component)]
pub(crate) struct CopyRowsButton;

/// The Copy line row of a row's menu: the line it copies.
#[derive(Component, Clone)]
struct CopyLineRow(String);

/// What a row needs to be marked and copied: its key and the click. The row
/// brings its own [`ListRow`] and [`Hovered`], which paint the mark.
pub(super) fn copy_row(list: CopyListType, key: u64) -> impl Bundle {
    (CopyRowKey(list, key), observe(on_copy_row_click))
}

/// The Copy button for the browser's tool row.
pub(super) fn spawn_copy_control(row: &mut RelatedSpawnerCommands<ChildOf>) {
    row.spawn((
        Name::new("Copy Rows Button"),
        CopyRowsButton,
        button(ButtonSpec::new("Copy").ghost().dense().fit()),
        ButtonCueType::Owned,
        observe(on_copy_button),
    ));
}

/// Whether Shift is held.
fn shift_held(keys: Option<&ButtonInput<KeyCode>>) -> bool {
    keys.is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]))
}

/// A left click marks the row, with Shift the range to it. A right click
/// offers Copy line.
fn on_copy_row_click(
    click: On<PointerClick>,
    mut commands: Commands,
    rows: Query<&CopyRowKey>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    roots: Query<Entity, With<PopupRoot>>,
    mut copy: ResMut<CopyRows>,
) {
    let Ok(CopyRowKey(list, key)) = rows.get(click.event_target()).copied() else {
        return;
    };
    match click.button {
        PointerButton::Primary => {
            let shown = copy.list(list).keys();
            let extend = shift_held(keys.as_deref());
            copy.list_mut(list).selection.click(&shown, key, extend);
        }
        PointerButton::Secondary => {
            let Some(text) = copy.list(list).text_of(key) else {
                return;
            };
            close_popups(&mut commands, &roots);
            let root = spawn_popup(
                &mut commands,
                click.pointer.position,
                layer::MENU_Z,
                |menu| {
                    menu.spawn((
                        Name::new("Copy Line Row"),
                        CopyLineRow(text),
                        popup_row("Copy line", "Ctrl+C"),
                        ButtonCueType::Owned,
                        observe(on_copy_line),
                    ));
                },
            );
            commands
                .entity(root)
                .insert(DespawnOnExit(ExampleStates::Editor));
        }
        PointerButton::Middle => {}
    }
}

/// Copy line: the one row's text.
fn on_copy_line(activate: On<Activate>, mut commands: Commands, rows: Query<&CopyLineRow>) {
    if let Ok(CopyLineRow(text)) = rows.get(activate.entity) {
        let line = vec![text.clone()];
        commands.queue(move |world: &mut World| pressed(world, |world| copy_lines(world, line)));
    }
}

/// Copy what the open list's marks say.
pub(crate) fn copy_open_list(world: &mut World) {
    let Some(list) = world
        .get_resource::<BrowserTabType>()
        .and_then(|tab| CopyListType::of_tab(*tab))
    else {
        return;
    };
    let lines = world.resource::<CopyRows>().list(list).lines();
    copy_lines(world, lines);
}

/// The Copy button.
fn on_copy_button(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| pressed(world, copy_open_list));
}

/// Whether the pointer is over the open Log or Problems list: where Ctrl+A and
/// Ctrl+C mean its rows.
pub(crate) fn copy_rows_hovered(
    open: Res<BrowserTabType>,
    panes: Query<(&BrowserPane, &Hovered)>,
) -> bool {
    CopyListType::of_tab(*open).is_some()
        && panes
            .iter()
            .any(|(pane, hovered)| pane.0 == *open && hovered.0)
}

/// Ctrl+A marks every shown row of the open list; Ctrl+C copies.
pub(crate) fn row_copy_keys(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    open: Res<BrowserTabType>,
    mut copy: ResMut<CopyRows>,
) {
    let held = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let Some(list) = CopyListType::of_tab(*open).filter(|_| held) else {
        return;
    };
    if keys.just_pressed(KeyCode::KeyA) {
        let shown = copy.list(list).keys();
        copy.list_mut(list).selection.select_all(&shown);
    } else if keys.just_pressed(KeyCode::KeyC) {
        commands.queue(copy_open_list);
    }
}

/// Mark the rows whose keys are marked.
pub(crate) fn sync_copy_marks(
    mut commands: Commands,
    copy: Res<CopyRows>,
    rows: Query<(Entity, &CopyRowKey, Has<Selected>)>,
    fresh: Query<(), Added<CopyRowKey>>,
) {
    if !copy.is_changed() && fresh.is_empty() {
        return;
    }
    for (row, CopyRowKey(list, key), marked) in &rows {
        match (copy.list(*list).selection.has(*key), marked) {
            (true, false) => {
                commands.entity(row).try_insert(Selected);
            }
            (false, true) => {
                commands.entity(row).try_remove::<Selected>();
            }
            _ => {}
        }
    }
}

/// Show the Copy button on the Log and Problems tabs.
pub(crate) fn sync_copy_controls(
    open: Res<BrowserTabType>,
    panels: Res<EditorPanels>,
    mut buttons: Query<&mut Node, With<CopyRowsButton>>,
) {
    let display = if CopyListType::of_tab(*open).is_some() && panels.browser {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut buttons {
        if node.display != display {
            node.display = display;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(texts: &[(u64, &str)]) -> Vec<ShownRow> {
        texts
            .iter()
            .map(|(key, text)| ShownRow {
                key: *key,
                text: (*text).to_string(),
            })
            .collect()
    }

    #[test]
    fn with_nothing_marked_a_copy_takes_every_shown_row() {
        let shown = rows(&[(7, "saved"), (3, "warned"), (1, "failed")]);

        assert_eq!(
            RowSelection::default().lines(&shown),
            ["saved", "warned", "failed"]
        );
    }

    #[test]
    fn a_click_marks_one_row_and_shift_click_marks_the_range_to_it() {
        let shown = [9, 8, 7, 6, 5];
        let mut marks = RowSelection::default();

        marks.click(&shown, 8, false);
        marks.click(&shown, 6, true);
        assert_eq!(marks.picked, [8, 7, 6]);

        marks.click(&shown, 9, true);
        assert_eq!(
            marks.picked,
            [9, 8],
            "the range runs from the last plain click"
        );

        marks.click(&shown, 5, false);
        assert_eq!(marks.picked, [5]);
    }

    #[test]
    fn a_shift_click_with_no_anchor_marks_the_row_alone() {
        let mut marks = RowSelection::default();

        marks.click(&[3, 2, 1], 2, true);

        assert_eq!(marks.picked, [2]);
    }

    #[test]
    fn a_copy_takes_the_marked_rows_in_list_order() {
        let shown = rows(&[(5, "e"), (4, "d"), (3, "c"), (2, "b")]);
        let mut marks = RowSelection::default();
        marks.click(&[5, 4, 3, 2], 2, false);
        marks.click(&[5, 4, 3, 2], 4, true);

        assert_eq!(marks.lines(&shown), ["d", "c", "b"]);
    }

    #[test]
    fn select_all_marks_what_the_filters_show() {
        let mut marks = RowSelection::default();

        marks.select_all(&[4, 2]);

        assert!(marks.has(4) && marks.has(2) && !marks.has(3));
    }

    /// A filter that hides every marked row leaves nothing marked in view, so
    /// a copy takes what is in view.
    #[test]
    fn marks_the_filters_hide_do_not_narrow_a_copy() {
        let mut marks = RowSelection::default();
        marks.click(&[9, 8], 9, false);

        assert_eq!(marks.lines(&rows(&[(8, "kept")])), ["kept"]);
    }

    #[test]
    fn rows_that_move_keep_their_marks_unless_the_list_was_rebuilt() {
        let mut copy = CopyRows::default();
        copy.set_rows(CopyListType::Log, rows(&[(1, "old")]), true);
        copy.log.selection.click(&[1], 1, false);

        copy.set_rows(CopyListType::Log, rows(&[(2, "new"), (1, "old")]), true);
        assert_eq!(copy.list(CopyListType::Log).lines(), ["old"]);

        copy.set_rows(CopyListType::Log, rows(&[(2, "new")]), false);
        assert_eq!(copy.list(CopyListType::Log).lines(), ["new"]);
    }
}
