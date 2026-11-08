//! The list pop-ups a field opens: a choice's variants and a reference's ids.
//!
//! One list for the Inspector's rows and the canvas chips, anchored under the
//! control that opened it. It lives on [`nova_ui`]'s popup, so a press outside
//! it and Esc close it and change nothing; a pick is one undo step.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    picking::hover::Hovered,
    prelude::*,
    ui::{UiGlobalTransform, UiScale},
    ui_widgets::{observe, Activate},
};
use nova_scenario::names::prelude::Names;
use nova_ui::{
    prelude::{
        close_popups, popup_row, scroll_bar, scroll_column, scroll_row, scroll_viewport,
        spawn_popup, text_field, InputMode, PopupRoot, TextFieldFocused, TextFieldSpec,
        TextFieldSubmitted, TextFieldValue, UiText,
    },
    theme::{density, UiColor},
    widget::{ButtonCueType, ThemedText},
};

use crate::{
    asset_index::prelude::AssetIndex,
    config::EditorSays,
    event::{action_config, ActionNode},
    history::commit_edit,
    inspect::{clipped, section_scope, write_field, DocumentIds},
    project::ModProject,
    ui::{
        inspector::{
            apply_choice, EditTargets, InspectorField, InspectorPick, InspectorRef, Offers,
        },
        layer,
        list_field::content_ids,
    },
    ExampleStates,
};

/// How wide a list stands.
const PICK_W: f32 = 260.0;

/// How tall the rows get before they scroll.
const PICK_LIST_H: f32 = 240.0;

/// The gap between the control and the list under it.
const ANCHOR_GAP: f32 = 2.0;

/// The shortest choice that gets a filter box. Below this the list reads at a
/// glance, and a box would be one more thing to look past.
const FILTER_FROM: usize = 10;

/// How much of an option's meaning the hint line holds.
const HINT_CHARS: usize = 120;

/// What a pick writes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickSortType {
    /// A variant of a choice, which may retype the node.
    Choice,
    /// An id, written as text. A typed id may name what does not exist yet.
    Reference,
}

/// An open list, on its popup's root: the field it writes to.
#[derive(Component, Clone)]
pub(crate) struct PickPopup {
    pub(crate) field: InspectorField,
    sort: PickSortType,
}

/// The box that narrows a list.
#[derive(Component)]
pub(crate) struct PickFilter;

/// The line that says what the hovered option means.
#[derive(Component)]
pub(crate) struct PickHint;

/// One row of a list.
#[derive(Component, Clone)]
pub(crate) struct PickOption {
    pub(crate) field: InspectorField,
    pub(crate) value: String,
    pub(crate) hint: String,
    sort: PickSortType,
}

/// One row to build: what it writes, what it prints, the muted word at its end,
/// and what it means.
struct PickRow {
    value: String,
    label: String,
    tail: &'static str,
    hint: String,
}

/// Where a list opens under `node`, in logical window pixels.
fn anchor(node: &ComputedNode, transform: &UiGlobalTransform, scale: Option<&UiScale>) -> Vec2 {
    let to_px = node.inverse_scale_factor;
    let (stretch, _, centre) = transform.to_scale_angle_translation();
    let rect = Rect::from_center_size(centre * to_px, node.size() * to_px * stretch.abs());
    Vec2::new(rect.min.x, rect.max.y + ANCHOR_GAP) * scale.map_or(1.0, |scale| scale.0)
}

/// Open the list of ids a reference can name.
///
/// It lists what the DOCUMENT holds, because that is the set the field is
/// judged against. An id can still be typed: a builder naming an object they
/// are about to add is not making a mistake yet.
pub(crate) fn on_open_ref_list(
    activate: On<Activate>,
    mut commands: Commands,
    chips: Query<(
        &InspectorField,
        &InspectorRef,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
    ids: DocumentIds,
    actions: Query<&ActionNode>,
    files: AssetIndex,
    project: Option<Res<ModProject>>,
    roots: Query<Entity, With<PopupRoot>>,
    scale: Option<Res<UiScale>>,
) {
    let Ok((field, chip, node, transform)) = chips.get(activate.entity) else {
        return;
    };
    close_popups(&mut commands, &roots);
    // What the list is, and what its being empty MEANS: a document with no
    // rocks on it yet is a different answer from a game shipping no sounds.
    let (offered, empty) = match chip.offers {
        // A section id means something only inside the ship the same action
        // names, so the list is that ship's sections.
        Offers::Named(Names::Section) => {
            let ship = actions
                .get(field.node)
                .ok()
                .and_then(|action| action_config(&action.kind))
                .and_then(|config| section_scope(config, field.path()));
            let empty = match &ship {
                Some(ship) => format!("the editor holds no sections of {ship}"),
                None => "name the ship first".to_string(),
            };
            (
                ids.names().offers_in(Names::Section, ship.as_deref()),
                empty,
            )
        }
        Offers::Named(named) => (ids.names().offers(named), "nothing to name yet".to_string()),
        Offers::File(sort) => (
            files.offers(sort),
            format!("no installed mod ships {}", sort.holds()),
        ),
        Offers::Content(catalog) => (
            project
                .as_deref()
                .map(|project| content_ids(catalog.kind(), project, &files))
                .unwrap_or_default(),
            format!("the mod sees no {}: add one first", catalog.kind()),
        ),
    };
    let clear = field.is_optional().then(|| PickRow {
        value: String::new(),
        label: "None".to_string(),
        tail: "",
        hint: "Leave this field empty.".to_string(),
    });
    let rows = clear
        .into_iter()
        .chain(offered.iter().map(|id| PickRow {
            value: id.clone(),
            label: id.clone(),
            // Said on the row: picking one adds the dependency.
            tail: if files.is_undeclared(id) {
                "undeclared"
            } else {
                ""
            },
            hint: String::new(),
        }))
        .collect();
    spawn_pick_popup(
        &mut commands,
        field.clone(),
        PickSortType::Reference,
        anchor(node, transform, scale.as_deref()),
        rows,
        Some("filter, or type an id"),
        &empty,
    );
}

/// Open the list of a choice.
pub(crate) fn on_open_choice_list(
    activate: On<Activate>,
    mut commands: Commands,
    buttons: Query<(
        &InspectorField,
        &InspectorPick,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
    roots: Query<Entity, With<PopupRoot>>,
    scale: Option<Res<UiScale>>,
) {
    let Ok((field, pick, node, transform)) = buttons.get(activate.entity) else {
        return;
    };
    close_popups(&mut commands, &roots);
    let rows: Vec<PickRow> = pick
        .options
        .iter()
        .enumerate()
        .map(|(index, value)| PickRow {
            value: value.clone(),
            label: value.clone(),
            tail: "",
            hint: pick.hints.get(index).cloned().unwrap_or_default(),
        })
        .collect();
    let filter = (rows.len() >= FILTER_FROM).then_some("filter");
    spawn_pick_popup(
        &mut commands,
        field.clone(),
        PickSortType::Choice,
        anchor(node, transform, scale.as_deref()),
        rows,
        filter,
        "",
    );
}

/// The popup: the box, the rows, and the line that explains them.
fn spawn_pick_popup(
    commands: &mut Commands,
    field: InspectorField,
    sort: PickSortType,
    at: Vec2,
    rows: Vec<PickRow>,
    filter: Option<&str>,
    empty: &str,
) {
    let hinted = rows.iter().any(|row| !row.hint.is_empty());
    let popup = PickPopup {
        field: field.clone(),
        sort,
    };
    let root = spawn_popup(commands, at, layer::WINDOW_Z, |panel| {
        panel
            .spawn((
                Name::new("Pick Body"),
                Node {
                    width: px(PICK_W),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(density::GAP),
                    ..default()
                },
            ))
            .with_children(|body| {
                if let Some(placeholder) = filter {
                    body.spawn((
                        Name::new("Pick Filter"),
                        PickFilter,
                        TextFieldFocused::at_end(""),
                        text_field(
                            TextFieldSpec::new("")
                                .placeholder(placeholder)
                                .max_chars(80)
                                .tool(),
                        ),
                    ));
                }
                if rows.iter().all(|row| row.value.is_empty()) {
                    // The empty list still OPENS, and says why it is empty. A
                    // chip that did nothing would read as a broken button on a
                    // document that simply has not spawned anything yet.
                    body.spawn((Name::new("Pick Empty"), small_text(empty)));
                }
                if rows.is_empty() {
                    return;
                }
                spawn_rows(body, &field, sort, &rows);
                if hinted {
                    body.spawn((
                        Name::new("Pick Hint"),
                        PickHint,
                        small_text(""),
                        Node {
                            min_height: px(density::ROW_H * 2.0),
                            ..default()
                        },
                    ));
                }
            });
    });
    commands
        .entity(root)
        .insert((popup, DespawnOnExit(ExampleStates::Editor)));
}

/// Muted small text. Over the panel, so the pointer goes through it.
fn small_text(text: &str) -> impl Bundle {
    (
        UiText,
        Text::new(text.to_string()),
        TextFont {
            font_size: FontSize::Px(density::TEXT_SMALL),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(UiColor::Label),
        Pickable::IGNORE,
    )
}

/// The scrolling rows.
fn spawn_rows(
    body: &mut RelatedSpawnerCommands<ChildOf>,
    field: &InspectorField,
    sort: PickSortType,
    rows: &[PickRow],
) {
    let word = match sort {
        PickSortType::Choice => "Choice",
        PickSortType::Reference => "Ref",
    };
    body.spawn((
        Name::new("Pick List Scroll"),
        Node {
            max_height: px(PICK_LIST_H),
            ..scroll_row()
        },
    ))
    .with_children(|scroll| {
        scroll
            .spawn((
                Name::new("Pick List"),
                Node {
                    align_items: AlignItems::Stretch,
                    ..scroll_column()
                },
                scroll_viewport(),
            ))
            .with_children(|list| {
                for row in rows {
                    list.spawn((
                        Name::new(format!("{word} Option {}", row.label)),
                        PickOption {
                            field: field.clone(),
                            value: row.value.clone(),
                            hint: clipped(&row.hint, HINT_CHARS),
                            sort,
                        },
                        popup_row(&row.label, row.tail),
                        ButtonCueType::Owned,
                        observe(on_pick_option),
                    ));
                }
            });
        scroll.spawn((Name::new("Pick List Bar"), scroll_bar()));
    });
}

/// Write `value` to `field` as one undo step.
fn write_pick(
    commands: &mut Commands,
    field: &InspectorField,
    sort: PickSortType,
    value: &str,
    targets: &mut EditTargets,
    says: &mut EditorSays,
) {
    match sort {
        PickSortType::Choice => apply_choice(commands, field, value, targets, says),
        PickSortType::Reference => {
            let written = targets.edit(field, |root, path, optional| {
                write_field(root, path, optional, value)
            });
            match written {
                Ok(()) => {
                    commit_edit(commands, field.edit_label());
                    says.accept();
                }
                Err(reason) => says.refuse(reason),
            }
        }
    }
}

/// A pressed row writes its value. Its popup closes with the press.
pub(crate) fn on_pick_option(
    activate: On<Activate>,
    mut commands: Commands,
    options: Query<&PickOption>,
    mut targets: EditTargets,
    mut says: EditorSays,
) {
    let Ok(option) = options.get(activate.entity) else {
        return;
    };
    write_pick(
        &mut commands,
        &option.field,
        option.sort,
        &option.value,
        &mut targets,
        &mut says,
    );
}

/// Whether `value` passes what the box holds.
fn passes(value: &str, typed: &str) -> bool {
    value.to_lowercase().contains(&typed.trim().to_lowercase())
}

/// What Enter takes with `typed` in the box. Only the whole text of a row picks
/// it: a near miss in a reference is a new id, and in a choice it is nothing.
fn entered_pick(rows: &[&str], typed: &str, sort: PickSortType) -> Option<String> {
    let typed = typed.trim();
    let listed = rows.contains(&typed);
    let picks = !typed.is_empty() && (listed || sort == PickSortType::Reference);
    picks.then(|| typed.to_string())
}

/// Enter in the box takes the row it names, or the typed id.
///
/// A press outside the box submits it too. That is a pick or a click away, and
/// neither one means what is typed.
pub(crate) fn submit_pick_filter(
    mut commands: Commands,
    mut submitted: MessageReader<TextFieldSubmitted>,
    mouse: Res<ButtonInput<MouseButton>>,
    filters: Query<(), With<PickFilter>>,
    popups: Query<&PickPopup>,
    roots: Query<Entity, With<PopupRoot>>,
    options: Query<&PickOption>,
    mut targets: EditTargets,
    mut says: EditorSays,
) {
    for TextFieldSubmitted { entity, value } in submitted.read() {
        if !filters.contains(*entity) || mouse.get_just_pressed().next().is_some() {
            continue;
        }
        let Some(popup) = popups.iter().next() else {
            continue;
        };
        let rows: Vec<&str> = options.iter().map(|row| row.value.as_str()).collect();
        let Some(picked) = entered_pick(&rows, value, popup.sort) else {
            continue;
        };
        write_pick(
            &mut commands,
            &popup.field,
            popup.sort,
            &picked,
            &mut targets,
            &mut says,
        );
        close_popups(&mut commands, &roots);
    }
}

/// Esc with the caret in the box closes the list. The field only hears it, and
/// the editor's own Esc does not run while a field has the keyboard.
pub(crate) fn pick_filter_escape(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mode: Option<Res<InputMode>>,
    popups: Query<(), With<PickPopup>>,
    roots: Query<Entity, With<PopupRoot>>,
) {
    let typing = mode.is_some_and(|mode| *mode == InputMode::Insert);
    if typing && keys.just_pressed(KeyCode::Escape) && !popups.is_empty() {
        close_popups(&mut commands, &roots);
    }
}

/// Close a list whose row has gone, narrow it to the box, and say what the
/// hovered row means.
pub(crate) fn sync_pick_popups(
    mut commands: Commands,
    popups: Query<(Entity, &PickPopup)>,
    rows: Query<&InspectorField, Or<(With<InspectorRef>, With<InspectorPick>)>>,
    filters: Query<&TextFieldValue, (With<PickFilter>, Changed<TextFieldValue>)>,
    mut options: Query<(&PickOption, &mut Node, &Hovered)>,
    mut hints: Query<&mut Text, With<PickHint>>,
) {
    for (entity, popup) in &popups {
        if !rows.iter().any(|field| *field == popup.field) {
            commands.entity(entity).despawn();
        }
    }
    if let Some(typed) = filters.iter().next() {
        for (option, mut node, _) in &mut options {
            let display = if passes(&option.value, &typed.0) {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
    let said = options
        .iter()
        .find(|(.., hovered)| hovered.get())
        .map_or("", |(option, ..)| option.hint.as_str());
    for mut text in &mut hints {
        if text.0 != said {
            text.0 = said.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A near miss is not a pick: `rock` must not write `rock_2`.
    #[test]
    fn typed_text_with_no_exact_row_stays_what_was_typed() {
        let picked = entered_pick(&["rock_2"], "rock", PickSortType::Reference);
        assert_eq!(picked.as_deref(), Some("rock"));
        assert_eq!(
            entered_pick(&["Sequence"], "seq", PickSortType::Choice),
            None
        );
    }

    /// The whole text of a row picks it, in either sort.
    #[test]
    fn an_exact_match_picks_its_row() {
        let rows = ["rock_2", "rock"];
        assert_eq!(
            entered_pick(&rows, " rock ", PickSortType::Reference).as_deref(),
            Some("rock")
        );
        assert_eq!(
            entered_pick(&["Sequence"], "Sequence", PickSortType::Choice).as_deref(),
            Some("Sequence")
        );
    }

    /// Enter on an empty box says nothing, however short the list.
    #[test]
    fn an_empty_box_picks_nothing() {
        assert_eq!(
            entered_pick(&["rock_2"], "  ", PickSortType::Reference),
            None
        );
        assert_eq!(entered_pick(&["Sequence"], "", PickSortType::Choice), None);
    }
}
