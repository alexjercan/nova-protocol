//! A list editor: one row per entry, each with the caller's cells and a remove
//! `x`, and a last row whose search dropdown adds an entry.
//!
//! The widget holds no entries. The caller spawns the rows from its document
//! ([`list_entry_row`], [`list_picker`], [`list_count`], [`list_remove`],
//! [`list_add`]) and answers the events this triggers on the list entity:
//! [`ListEntryAdded`], [`ListEntryRemoved`], [`ListEntryPicked`] and
//! [`ListEntryCounted`]. The caller makes the edit, or refuses it and says why.
//!
//! The add dropdown and a row's picker are a [`spawn_cascade`] of the list's
//! [`ListEditor::choices`], so typing filters them at once.

use bevy::{
    ecs::system::EntityCommands,
    prelude::*,
    ui::{UiGlobalTransform, UiScale},
    ui_widgets::{observe, Activate},
};

use super::{
    button, close_popups, spawn_cascade, text_field, ButtonSpec, CascadeItem, PopupRoot,
    TextFieldSpec, TextFieldSubmitted,
};
use crate::theme::density;

/// One choice of a list's add dropdown and of its row pickers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListChoice {
    /// What the list stores when this is picked.
    pub id: String,
    /// What the row says.
    pub label: String,
    /// A muted note at the right of the row.
    pub hint: String,
}

impl ListChoice {
    /// A choice stored as `id` and shown as `label`.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            hint: String::new(),
        }
    }

    /// The muted note at the right of the row.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }
}

/// The list: what its dropdowns offer, and on which rung they open.
#[derive(Component, Clone, Debug, Default)]
pub struct ListEditor {
    /// What the add dropdown and the row pickers offer.
    pub choices: Vec<ListChoice>,
    /// Why the add dropdown offers nothing, shown on its one greyed row.
    pub empty_reason: String,
    /// The rung the dropdowns open on.
    pub z: i32,
}

/// The add dropdown took `id`.
#[derive(EntityEvent, Clone, Debug)]
pub struct ListEntryAdded {
    /// The list.
    pub entity: Entity,
    /// The choice it took.
    pub id: String,
}

/// The `x` of row `index` was pressed.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct ListEntryRemoved {
    /// The list.
    pub entity: Entity,
    /// The row.
    pub index: usize,
}

/// The picker of row `index` took `id`.
#[derive(EntityEvent, Clone, Debug)]
pub struct ListEntryPicked {
    /// The list.
    pub entity: Entity,
    /// The row.
    pub index: usize,
    /// The choice it took.
    pub id: String,
}

/// The count of row `index` was submitted as `text`. Unparsed: the caller
/// owns the range and the refusal.
#[derive(EntityEvent, Clone, Debug)]
pub struct ListEntryCounted {
    /// The list.
    pub entity: Entity,
    /// The row.
    pub index: usize,
    /// What was typed.
    pub text: String,
}

/// A row's `x`.
#[derive(Component, Clone, Copy, Debug)]
pub struct ListRemove {
    list: Entity,
    index: usize,
}

/// The last row's add button.
#[derive(Component, Clone, Copy, Debug)]
pub struct ListAdd {
    list: Entity,
}

/// A row's picker button.
#[derive(Component, Clone, Copy, Debug)]
pub struct ListPicker {
    list: Entity,
    index: usize,
}

/// A row's count field.
#[derive(Component, Clone, Copy, Debug)]
pub struct ListCount {
    list: Entity,
    index: usize,
}

/// A row of an open dropdown: the list it writes, and the row it replaces, or
/// `None` to add.
#[derive(Component, Clone, Debug)]
pub struct ListPick {
    list: Entity,
    index: Option<usize>,
    id: String,
}

/// The width of a count field: four digits.
const COUNT_W: f32 = 52.0;

/// The characters a count takes.
const COUNT_CHARS: usize = 4;

/// The list's column. The caller adds a `Name` and the observers of the
/// events it answers.
pub fn list_editor(editor: ListEditor) -> impl Bundle {
    (
        editor,
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(2),
            ..default()
        },
    )
}

/// One entry's row. The caller spawns its cells into it, then [`list_remove`].
pub fn list_entry_row() -> impl Bundle {
    Node {
        width: percent(100),
        min_height: px(density::CONTROL_H),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(density::GAP),
        ..default()
    }
}

/// The `x` that removes row `index` of `list`.
pub fn list_remove(list: Entity, index: usize) -> impl Bundle {
    (
        ListRemove { list, index },
        button(ButtonSpec::new("x").dense().ghost().fit()),
        observe(on_list_remove),
    )
}

/// The last row: a button that drops the search of what may be added.
pub fn list_add(list: Entity, word: &str) -> impl Bundle {
    (
        ListAdd { list },
        button(ButtonSpec::new(format!("+ {word}")).dense().ghost().block()),
        observe(on_list_add),
    )
}

/// A button in row `index` that shows `label` and drops the search of what
/// the row may hold instead.
pub fn list_picker(list: Entity, index: usize, label: &str) -> impl Bundle {
    (
        ListPicker { list, index },
        button(
            ButtonSpec::new(format!("{label} v"))
                .dense()
                .ghost()
                .block(),
        ),
        observe(on_list_picker),
    )
}

/// A count field in row `index`, holding `count`.
pub fn list_count(list: Entity, index: usize, count: u32) -> impl Bundle {
    (
        ListCount { list, index },
        Node {
            width: px(COUNT_W),
            flex_shrink: 0.0,
            ..default()
        },
        children![text_field(
            TextFieldSpec::new(count.to_string())
                .max_chars(COUNT_CHARS)
                .tool()
        )],
    )
}

/// The rows of a dropdown over `list`'s choices. `index` is the row a pick
/// replaces, or `None` to add.
fn pick_items(editor: &ListEditor, list: Entity, index: Option<usize>) -> Vec<CascadeItem> {
    if editor.choices.is_empty() {
        let reason = editor.empty_reason.clone();
        return vec![CascadeItem::leaf("List Pick None", "Nothing to add").greyed(reason)];
    }
    editor
        .choices
        .iter()
        .map(|choice| {
            let pick = ListPick {
                list,
                index,
                id: choice.id.clone(),
            };
            CascadeItem::leaf(format!("List Pick {}", choice.id), choice.label.clone())
                .hint(choice.hint.clone())
                .on_row(move |row: &mut EntityCommands| {
                    row.insert((pick.clone(), observe(on_list_pick)));
                })
        })
        .collect()
}

/// Open the dropdown of `list` under `button`.
fn open_under(
    commands: &mut Commands,
    button: (&ComputedNode, &UiGlobalTransform),
    scale: Option<&UiScale>,
    roots: &Query<Entity, With<PopupRoot>>,
    editor: &ListEditor,
    list: Entity,
    index: Option<usize>,
) {
    let (computed, at) = button;
    let scale = scale.map_or(1.0, |scale| scale.0);
    let bottom_left = at.translation + computed.size() * Vec2::new(-0.5, 0.5);
    let items = pick_items(editor, list, index);
    close_popups(commands, roots);
    spawn_cascade(
        commands,
        bottom_left * computed.inverse_scale_factor() * scale,
        editor.z,
        items,
    );
}

/// The add button: drop the search of every choice.
fn on_list_add(
    activate: On<Activate>,
    mut commands: Commands,
    scale: Option<Res<UiScale>>,
    adds: Query<(&ListAdd, &ComputedNode, &UiGlobalTransform)>,
    editors: Query<&ListEditor>,
    roots: Query<Entity, With<PopupRoot>>,
) {
    let Ok((add, computed, at)) = adds.get(activate.entity) else {
        return;
    };
    let Ok(editor) = editors.get(add.list) else {
        return;
    };
    open_under(
        &mut commands,
        (computed, at),
        scale.as_deref(),
        &roots,
        editor,
        add.list,
        None,
    );
}

/// A row's picker: drop the search of what the row may hold.
fn on_list_picker(
    activate: On<Activate>,
    mut commands: Commands,
    scale: Option<Res<UiScale>>,
    pickers: Query<(&ListPicker, &ComputedNode, &UiGlobalTransform)>,
    editors: Query<&ListEditor>,
    roots: Query<Entity, With<PopupRoot>>,
) {
    let Ok((picker, computed, at)) = pickers.get(activate.entity) else {
        return;
    };
    let Ok(editor) = editors.get(picker.list) else {
        return;
    };
    open_under(
        &mut commands,
        (computed, at),
        scale.as_deref(),
        &roots,
        editor,
        picker.list,
        Some(picker.index),
    );
}

/// A dropdown row: say what was picked, on the list.
fn on_list_pick(activate: On<Activate>, mut commands: Commands, picks: Query<&ListPick>) {
    let Ok(pick) = picks.get(activate.entity).cloned() else {
        return;
    };
    match pick.index {
        None => commands.trigger(ListEntryAdded {
            entity: pick.list,
            id: pick.id,
        }),
        Some(index) => commands.trigger(ListEntryPicked {
            entity: pick.list,
            index,
            id: pick.id,
        }),
    }
}

/// A row's `x`: say which row, on the list.
fn on_list_remove(activate: On<Activate>, mut commands: Commands, removes: Query<&ListRemove>) {
    if let Ok(remove) = removes.get(activate.entity).copied() {
        commands.trigger(ListEntryRemoved {
            entity: remove.list,
            index: remove.index,
        });
    }
}

/// A submitted count: say what was typed, on the list.
pub(super) fn submit_list_counts(
    mut commands: Commands,
    mut submitted: MessageReader<TextFieldSubmitted>,
    parents: Query<&ChildOf>,
    counts: Query<&ListCount>,
) {
    for message in submitted.read() {
        let Some(count) = parents
            .get(message.entity)
            .ok()
            .and_then(|parent| counts.get(parent.parent()).ok())
        else {
            continue;
        };
        commands.trigger(ListEntryCounted {
            entity: count.list,
            index: count.index,
            text: message.value.trim().to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use bevy::{prelude::*, ui_widgets::Activate};

    use super::{
        list_count, list_editor, list_remove, pick_items, ListChoice, ListEditor, ListEntryAdded,
        ListEntryCounted, ListEntryPicked, ListEntryRemoved, ListPick,
    };
    use crate::{
        prelude::{TextFieldSubmitted, TextFieldValue},
        widget::fixtures::themed_app,
    };

    /// What the list was told, in order.
    #[derive(Resource, Default)]
    struct Heard(Vec<String>);

    fn heard_app() -> (App, Entity) {
        let mut app = themed_app(crate::theme::config::PHOSPHOR_THEME_ID);
        app.init_resource::<Heard>();
        let list = app
            .world_mut()
            .spawn(list_editor(ListEditor {
                choices: vec![
                    ListChoice::new("alloy", "Alloy"),
                    ListChoice::new("wire", "Wire"),
                ],
                empty_reason: "every item is listed".to_string(),
                z: 1,
            }))
            .observe(|on: On<ListEntryAdded>, mut heard: ResMut<Heard>| {
                heard.0.push(format!("add {}", on.id));
            })
            .observe(|on: On<ListEntryRemoved>, mut heard: ResMut<Heard>| {
                heard.0.push(format!("remove {}", on.index));
            })
            .observe(|on: On<ListEntryPicked>, mut heard: ResMut<Heard>| {
                heard.0.push(format!("pick {} {}", on.index, on.id));
            })
            .observe(|on: On<ListEntryCounted>, mut heard: ResMut<Heard>| {
                heard.0.push(format!("count {} {}", on.index, on.text));
            })
            .id();
        app.update();
        (app, list)
    }

    fn heard(app: &App) -> Vec<String> {
        app.world().resource::<Heard>().0.clone()
    }

    /// A dropdown row adds, or replaces the row it was opened from, and the
    /// list hears which.
    #[test]
    fn a_dropdown_pick_adds_or_replaces_on_the_list() {
        let (mut app, list) = heard_app();
        let editor = app.world().get::<ListEditor>(list).unwrap().clone();
        assert_eq!(pick_items(&editor, list, None).len(), 2, "one row a choice");
        let add = app
            .world_mut()
            .spawn(ListPick {
                list,
                index: None,
                id: "wire".to_string(),
            })
            .observe(super::on_list_pick)
            .id();
        let replace = app
            .world_mut()
            .spawn(ListPick {
                list,
                index: Some(0),
                id: "alloy".to_string(),
            })
            .observe(super::on_list_pick)
            .id();

        app.world_mut().trigger(Activate { entity: add });
        app.world_mut().trigger(Activate { entity: replace });
        app.world_mut().flush();

        assert_eq!(heard(&app), ["add wire", "pick 0 alloy"]);
    }

    /// The `x` names its row.
    #[test]
    fn the_remove_x_names_its_row() {
        let (mut app, list) = heard_app();
        let x = app.world_mut().spawn(list_remove(list, 2)).id();

        app.world_mut().trigger(Activate { entity: x });
        app.world_mut().flush();

        assert_eq!(heard(&app), ["remove 2"]);
    }

    /// A submitted count reaches the list as typed, for the caller to judge.
    #[test]
    fn a_submitted_count_reaches_the_list_unjudged() {
        let (mut app, list) = heard_app();
        let count = app.world_mut().spawn(list_count(list, 1, 3)).id();
        app.update();
        let field = app.world().get::<Children>(count).unwrap()[0];
        assert_eq!(app.world().get::<TextFieldValue>(field).unwrap().0, "3");

        app.world_mut().write_message(TextFieldSubmitted {
            entity: field,
            value: " lots ".to_string(),
        });
        app.update();

        assert_eq!(heard(&app), ["count 1 lots"]);
    }

    /// With nothing to offer, the dropdown holds one greyed row that says why.
    #[test]
    fn an_empty_dropdown_says_why() {
        let editor = ListEditor {
            choices: Vec::new(),
            empty_reason: "every item is listed".to_string(),
            z: 1,
        };

        let items = pick_items(&editor, Entity::PLACEHOLDER, None);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].greyed.as_deref(), Some("every item is listed"));
    }
}
