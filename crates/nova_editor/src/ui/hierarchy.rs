//! The Hierarchy panel's verbs: the filter, the expand caret, and the context
//! menu of a row. The rows themselves are `ui::sync_scene_list`'s.

use bevy::{
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_ship::prelude::GalleryState;
use nova_ui::prelude::*;

use super::{
    add_menu::{world_items, AddFacts, DesignRow},
    inspector::InspectorField,
    layer,
    menu_context::{verb_state, MenuVerbType},
    rail::SceneRowHint,
    SceneRow, WantedRow,
};
use crate::{
    clipboard::duplicate_item,
    config::SelectedNode,
    event::Expanded,
    frame::on_frame_selection,
    node::{EditContext, ObjectNode, ScenarioNode, SectionNode, ShipNode},
    placement::{deletable, delete_selected_node, DeletableNode},
    project::ship::open_ship_node,
    ExampleStates,
};

/// The text field over the tree.
#[derive(Component)]
pub(crate) struct HierarchyFilter;

/// The node a Rename row asked to name. The Inspector rebuilds for the new
/// selection first, so the focus lands a frame or two later.
#[derive(Resource, Default)]
pub(crate) struct PendingRename(pub(crate) Option<Entity>);

/// The rows a filter text keeps, each with its ancestors.
pub(crate) fn filtered_rows(rows: Vec<WantedRow>, filter: &str) -> Vec<WantedRow> {
    let shape: Vec<(usize, bool)> = rows
        .iter()
        .map(|row| {
            let found = [&row.full, &row.id, &row.kind]
                .into_iter()
                .any(|text| text.to_lowercase().contains(filter));
            (row.depth, found)
        })
        .collect();
    rows.into_iter()
        .zip(tree_keep(&shape))
        .filter_map(|(row, keep)| keep.then_some(row))
        .collect()
}

/// The rows between two rows of the shown list, both ends included.
pub(crate) fn row_range(order: &[Entity], from: Entity, to: Entity) -> Vec<Entity> {
    let at = |node| order.iter().position(|row| *row == node);
    match (at(from), at(to)) {
        (Some(a), Some(b)) => order[a.min(b)..=a.max(b)].to_vec(),
        _ => vec![to],
    }
}

/// A caret press opens or shuts its row. A leaf row has no caret to draw, so a
/// press in that column is a press on the row.
pub(crate) fn on_tree_caret(
    activate: On<Activate>,
    mut commands: Commands,
    carets: Query<&ChildOf, With<TreeCaret>>,
    rows: Query<(&SceneRow, &SceneRowHint)>,
    expanded: Query<(), With<Expanded>>,
) {
    let Ok(parent) = carets.get(activate.entity) else {
        return;
    };
    let row = parent.parent();
    let Ok((SceneRow(node), hint)) = rows.get(row) else {
        return;
    };
    if hint.open.is_none() {
        commands.trigger(Activate { entity: row });
    } else if expanded.contains(*node) {
        commands.entity(*node).remove::<Expanded>();
    } else {
        commands.entity(*node).insert(Expanded);
    }
}

/// What the context menu of one node offers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowVerbs {
    /// The Inspector has a name for this node to type into.
    pub(crate) rename: bool,
    /// The clipboard can copy this node.
    pub(crate) duplicate: bool,
    /// The delete verb accepts this node.
    pub(crate) delete: bool,
    /// The node stands somewhere the camera can go.
    pub(crate) frame: bool,
    /// The Add menu's scenario groups: the node is the scenario root.
    pub(crate) add_world: bool,
    /// Add Part: the node is a ship.
    pub(crate) add_part: bool,
}

/// Right-click on a row: mark it, unless it is already one of the marks, and
/// open its menu at the pointer.
///
/// `Button` also turns this click into `Activate`, so `on_scene_row` drops an
/// activation on the frame the right button came up.
#[expect(
    clippy::too_many_arguments,
    reason = "a Bevy observer; each query is one legality rule of the menu"
)]
pub(crate) fn on_scene_row_menu(
    click: On<PointerClick>,
    mut commands: Commands,
    rows: Query<&SceneRow>,
    context: Res<EditContext>,
    mut selected: ResMut<SelectedNode>,
    facts: AddFacts,
    deletes: Query<(), DeletableNode>,
    kinds: Query<(
        Has<ScenarioNode>,
        Has<ShipNode>,
        Has<ObjectNode>,
        Has<SectionNode>,
    )>,
    roots: Query<Entity, With<PopupRoot>>,
) {
    if click.button != PointerButton::Secondary {
        return;
    }
    let Ok(SceneRow(node)) = rows.get(click.event_target()) else {
        return;
    };
    let node = *node;
    if !selected.contains(node) {
        selected.set(Some(node));
    }
    let Ok((root, ship, object, section)) = kinds.get(node) else {
        return;
    };
    // The row's kind says which verbs fit the node; the menu context says
    // which of those the open document and view offer, the same rule the bar
    // menus show.
    let menu = facts.context(&selected);
    let offered = |verb| verb_state(verb, &menu).is_live();
    let verbs = RowVerbs {
        rename: ship || object,
        duplicate: (ship || object) && offered(MenuVerbType::Duplicate),
        delete: deletable(node, &context, &deletes),
        frame: (root || ship || object || section) && offered(MenuVerbType::Frame),
        add_world: root && offered(MenuVerbType::AddWorld),
        add_part: ship && offered(MenuVerbType::EnterShipParts),
    };
    close_popups(&mut commands, &roots);
    let popup = spawn_cascade(
        &mut commands,
        click.pointer.position,
        layer::MENU_Z,
        row_items(&verbs, &facts.designs()),
    );
    commands
        .entity(popup)
        .insert(DespawnOnExit(ExampleStates::Editor));
}

/// The rows of a context menu, each one an existing verb that acts on the
/// selection. Only legal rows are listed. The scenario root also holds the
/// Add menu's scenario groups, with `designs` under Ship.
fn row_items(verbs: &RowVerbs, designs: &[DesignRow]) -> Vec<CascadeItem> {
    let mut items = Vec::new();
    if verbs.rename {
        items.push(CascadeItem::leaf("Row Rename", "Rename").on_row(|row| {
            row.insert(observe(rename_selection));
        }));
    }
    if verbs.duplicate {
        items.push(
            CascadeItem::leaf("Row Duplicate", "Duplicate")
                .hint("Ctrl+D")
                .on_row(|row| {
                    row.insert(observe(duplicate_item));
                }),
        );
    }
    if verbs.delete {
        items.push(
            CascadeItem::leaf("Row Delete", "Delete")
                .hint("Del")
                .on_row(|row| {
                    row.insert(observe(delete_selected_node));
                }),
        );
    }
    if verbs.frame {
        items.push(
            CascadeItem::leaf("Row Frame", "Frame")
                .hint("F")
                .on_row(|row| {
                    row.insert(observe(on_frame_selection));
                }),
        );
    }
    if verbs.add_part {
        items.push(CascadeItem::leaf("Row Add Part", "Add Part").on_row(|row| {
            row.insert(observe(add_part_to_row));
        }));
    }
    if verbs.add_world {
        if !items.is_empty() {
            items.push(CascadeItem::separator());
        }
        items.extend(world_items(designs, None));
    }
    items
}

/// Rename: put the caret in the Inspector's name field.
fn rename_selection(
    _activate: On<Activate>,
    selected: Res<SelectedNode>,
    mut pending: ResMut<PendingRename>,
) {
    pending.0 = selected.primary();
}

/// Add Part: open the ship on its own and open the parts gallery there.
fn add_part_to_row(
    _activate: On<Activate>,
    mut commands: Commands,
    selected: Res<SelectedNode>,
    ships: Query<(), With<ShipNode>>,
) {
    let Some(ship) = selected.primary().filter(|node| ships.contains(*node)) else {
        return;
    };
    commands.queue(move |world: &mut World| {
        open_ship_node(world, ship, None);
        if world.resource::<EditContext>().ship().is_some() {
            let mut gallery = world.resource_mut::<GalleryState>();
            *gallery = gallery.opened();
        }
    });
}

/// Focus the name field of the node a Rename row asked for, once the Inspector
/// shows it. The field keeps its own validation and its own undo step at Enter.
pub(crate) fn focus_pending_rename(
    mut commands: Commands,
    mut pending: ResMut<PendingRename>,
    fields: Query<(Entity, &Name, &InspectorField, &TextFieldValue)>,
) {
    let Some(node) = pending.0 else {
        return;
    };
    let name_field = fields
        .iter()
        .find(|(_, name, field, _)| field.node == node && name.as_str() == RENAME_FIELD);
    if let Some((entity, _, _, value)) = name_field {
        commands
            .entity(entity)
            .insert(TextFieldFocused::at_end(&value.0));
        pending.0 = None;
    }
}

/// The Inspector field a Rename row focuses.
const RENAME_FIELD: &str = "Inspector Field Name";

#[cfg(test)]
mod tests {
    use bevy::{prelude::*, ui_widgets::Activate};

    use super::{filtered_rows, row_items, row_range, RowVerbs};
    use crate::{
        clipboard::EditorClipboard,
        config::SelectedNode,
        history::tests::{depth, first_with, history_app, undo},
        node::ObjectNode,
        ui::WantedRow,
    };

    fn row(depth: usize, full: &str) -> WantedRow {
        WantedRow {
            node: Entity::PLACEHOLDER,
            depth,
            open: None,
            count: 0,
            lead: String::new(),
            id: full.to_string(),
            full: full.to_string(),
            label: full.to_string(),
            trail: String::new(),
            kind: "PART".to_string(),
        }
    }

    #[test]
    fn the_tree_filter_keeps_matches_under_their_parents() {
        let rows = vec![
            row(0, "range"),
            row(1, "corvette"),
            row(2, "thruster"),
            row(2, "hull"),
            row(1, "rock"),
        ];
        let kept: Vec<String> = filtered_rows(rows, "thrus")
            .into_iter()
            .map(|row| row.full)
            .collect();
        assert_eq!(kept, ["range", "corvette", "thruster"]);
    }

    /// The Row Delete and Row Duplicate rows are the menu's whole promise: the
    /// same verb as the key, so one step in the history.
    #[test]
    fn a_context_menu_verb_is_one_undo_step() {
        let mut app = history_app();
        app.init_resource::<EditorClipboard>();
        let rock = first_with::<ObjectNode>(&mut app);
        app.world_mut()
            .resource_mut::<SelectedNode>()
            .set(Some(rock));
        let before = depth(&app);

        let verbs = RowVerbs {
            duplicate: true,
            ..default()
        };
        let items = row_items(&verbs, &[]);
        let duplicate = items
            .iter()
            .find(|item| item.name == "Row Duplicate")
            .and_then(|item| item.row.clone())
            .expect("the row is listed");
        let entity = {
            let mut commands = app.world_mut().commands();
            let mut row = commands.spawn_empty();
            duplicate(&mut row);
            row.id()
        };
        app.world_mut().flush();
        app.world_mut().trigger(Activate { entity });
        app.world_mut().flush();
        app.update();

        assert_eq!(depth(&app), before + 1, "one press, one step");
        undo(&mut app);
        assert_eq!(depth(&app), before, "and one undo takes it back");
    }

    #[test]
    fn a_shift_range_runs_either_way_over_the_shown_order() {
        let mut world = World::new();
        let rows: Vec<Entity> = (0..4).map(|_| world.spawn_empty().id()).collect();
        assert_eq!(row_range(&rows, rows[1], rows[3]), rows[1..=3]);
        assert_eq!(row_range(&rows, rows[3], rows[1]), rows[1..=3]);
    }
}
