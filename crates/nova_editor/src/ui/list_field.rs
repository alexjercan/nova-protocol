//! The Inspector's list rows: a section's build parts, a start role's stock and
//! a campaign's scenarios, each drawn as a [`ListEditor`].
//!
//! A row per entry holds a picker of the id, a count where the list counts,
//! and a remove `x`. The last row drops a search of the ids the mod sees.
//! Every edit is one undo step through [`edited_entries`]; an edit it refuses
//! changes nothing and says why on the status line.

use bevy::{ecs::relationship::RelatedSpawnerCommands, prelude::*, ui_widgets::observe};
use nova_modding::prelude::Content;
use nova_ui::prelude::{
    list_add, list_count, list_editor, list_entry_row, list_picker, list_remove, ListChoice,
    ListEditor, ListEntryAdded, ListEntryCounted, ListEntryPicked, ListEntryRemoved,
};

use crate::{
    asset::AssetNode,
    asset_index::AssetIndex,
    history::commit_edit_now,
    inspect::{
        edited_entries, list_entries, set_list_entries, AssetListType, InspectorRow, ListEditType,
        ListEntry,
    },
    project::{
        verbs::{pressed, refuse},
        ModProject,
    },
    ui::{
        inspector::{row_hint, row_label},
        layer,
    },
};

/// A list row's editor: which list of which bench item, and every id a row
/// may name.
#[derive(Component, Clone, Debug)]
pub(crate) struct InspectorList {
    /// The document root holding the [`AssetNode`].
    node: Entity,
    /// Which list it is.
    kind: AssetListType,
    /// The ids the mod sees, listed or not. Filled by [`fill_list_choices`].
    known: Vec<String>,
}

/// Spawn the row: the label over the list editor.
pub(super) fn spawn_list_row(
    list: &mut RelatedSpawnerCommands<ChildOf>,
    row: &InspectorRow,
    node: Entity,
    kind: AssetListType,
    entries: &[ListEntry],
    step: f32,
) {
    list.spawn((
        Name::new(format!("Inspector Row {}", row.label)),
        row_hint(row),
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            padding: UiRect::left(px(step)),
            margin: UiRect::bottom(px(6)),
            row_gap: px(3),
            ..default()
        },
    ))
    .with_children(|block| {
        block.spawn(row_label(row, step));
        block
            .spawn((
                Name::new(format!("Inspector List {}", row.label)),
                InspectorList {
                    node,
                    kind,
                    known: Vec::new(),
                },
                list_editor(ListEditor {
                    z: layer::MENU_Z,
                    ..default()
                }),
                observe(on_list_added),
                observe(on_list_removed),
                observe(on_list_picked),
                observe(on_list_counted),
            ))
            .with_children(|rows| {
                let editor = rows.target_entity();
                for (index, entry) in entries.iter().enumerate() {
                    let id = &entry.id;
                    rows.spawn((Name::new(format!("List Row {id}")), list_entry_row()))
                        .with_children(|cells| {
                            cells.spawn((
                                Name::new(format!("List Picker {id}")),
                                list_picker(editor, index, id),
                            ));
                            if let Some(count) = entry.count {
                                cells.spawn((
                                    Name::new(format!("List Count Cell {id}")),
                                    list_count(editor, index, count, format!("List Count {id}")),
                                ));
                            }
                            cells.spawn((
                                Name::new(format!("List Remove {id}")),
                                list_remove(editor, index),
                            ));
                        });
                }
                rows.spawn((
                    Name::new(format!("List Add {}", row.label)),
                    list_add(editor, kind.add_word()),
                ));
            });
    });
}

/// Every id `kind` takes that the mod sees, with its name and where it comes
/// from: the mod's own content, then each visible pack's.
fn list_choices(kind: AssetListType, project: &ModProject, index: &AssetIndex) -> Vec<ListChoice> {
    let named = |item: &Content, from: &str| {
        let name = match item {
            Content::Item(design) => design.name.clone(),
            Content::Scenario(scenario) => scenario.name.clone(),
            other => other.id().to_string(),
        };
        ListChoice::new(item.id(), name).hint(format!("{} - {from}", item.id()))
    };
    let mut choices: Vec<ListChoice> = project
        .data
        .items()
        .filter(|item| kind.takes(item))
        .map(|item| named(item, "this mod"))
        .collect();
    for pack in index.visible_packs(&project.data.meta.dependencies) {
        for item in pack.items.iter().filter(|item| kind.takes(item)) {
            if !choices.iter().any(|held| held.id == item.id()) {
                choices.push(named(item, pack.id));
            }
        }
    }
    choices.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
    choices
}

/// The ids of content `kind` the open mod sees: its own, then every pack it can
/// see, sorted. The set a content id field is offered from and judged against.
pub(crate) fn content_ids(kind: &str, project: &ModProject, index: &AssetIndex) -> Vec<String> {
    let mut ids: Vec<String> = project
        .data
        .items()
        .filter(|item| item.kind() == kind)
        .map(|item| item.id().to_string())
        .collect();
    for pack in index.visible_packs(&project.data.meta.dependencies) {
        for item in pack.items.iter().filter(|item| item.kind() == kind) {
            if !ids.iter().any(|held| held == item.id()) {
                ids.push(item.id().to_string());
            }
        }
    }
    ids.sort();
    ids
}

/// Fill each list row's dropdown when it appears or the project moves: the
/// ids the mod sees that the list does not hold yet.
pub(crate) fn fill_list_choices(
    project: Option<Res<ModProject>>,
    index: AssetIndex,
    mut lists: Query<(&mut InspectorList, &mut ListEditor)>,
    assets: Query<&AssetNode>,
) {
    let Some(project) = project else {
        return;
    };
    let fresh = lists.iter_mut().any(|(list, _)| list.is_added());
    if !fresh && !project.is_changed() {
        return;
    }
    for (mut list, mut editor) in &mut lists {
        let Ok(asset) = assets.get(list.node) else {
            continue;
        };
        let listed = list_entries(&asset.0, list.kind);
        let choices = list_choices(list.kind, &project, &index);
        list.known = choices.iter().map(|choice| choice.id.clone()).collect();
        editor.choices = choices
            .into_iter()
            .filter(|choice| !listed.iter().any(|entry| entry.id == choice.id))
            .collect();
        editor.empty_reason = if list.known.is_empty() {
            "the mod sees none: add one, or a dependency that has one".to_string()
        } else {
            "everything the mod sees is listed".to_string()
        };
    }
}

/// Make `edit` on `list`'s field as one step, or refuse it and say why.
fn apply_list_edit(world: &mut World, list: Entity, edit: ListEditType) {
    let Some(held) = world.get::<InspectorList>(list).cloned() else {
        return;
    };
    pressed(world, |world| {
        let Some(mut asset) = world.get_mut::<AssetNode>(held.node) else {
            return;
        };
        let entries = list_entries(&asset.0, held.kind);
        let made = edited_entries(&entries, held.kind, &edit, &held.known)
            .and_then(|next| set_list_entries(&mut asset.0, held.kind, &next));
        match made {
            Ok(()) => commit_edit_now(world, held.kind.edit_label()),
            Err(reason) => refuse(world, reason),
        }
    });
}

/// The add dropdown took an id.
fn on_list_added(added: On<ListEntryAdded>, mut commands: Commands) {
    let (list, edit) = (added.entity, ListEditType::Add(added.id.clone()));
    commands.queue(move |world: &mut World| apply_list_edit(world, list, edit));
}

/// A row's `x`.
fn on_list_removed(removed: On<ListEntryRemoved>, mut commands: Commands) {
    let (list, edit) = (removed.entity, ListEditType::Remove(removed.index));
    commands.queue(move |world: &mut World| apply_list_edit(world, list, edit));
}

/// A row's picker took another id.
fn on_list_picked(picked: On<ListEntryPicked>, mut commands: Commands) {
    let (list, edit) = (
        picked.entity,
        ListEditType::Pick(picked.index, picked.id.clone()),
    );
    commands.queue(move |world: &mut World| apply_list_edit(world, list, edit));
}

/// A row's count was submitted.
fn on_list_counted(counted: On<ListEntryCounted>, mut commands: Commands) {
    let (list, edit) = (
        counted.entity,
        ListEditType::Count(counted.index, counted.text.clone()),
    );
    commands.queue(move |world: &mut World| apply_list_edit(world, list, edit));
}
