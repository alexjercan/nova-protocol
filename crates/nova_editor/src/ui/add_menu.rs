//! The Add menu: one tree of what the open document takes.
//!
//! The top bar's Add drops it under its button, Shift+A opens it at the
//! pointer over the stage, and the stage's and the Hierarchy's right-click
//! menus hold the same rows. It is a `nova_ui` cascading menu: a submenu per
//! group, and a search over every row.
//!
//! [`verb_state`] decides what shows, as it decides every other menu: a
//! scenario adds ships, objects, lights and markers; a ship opens its parts; a
//! section adds its mesh, a collider and sockets; and every document can start
//! a new asset. A row added at a point carries [`PlaceAt`], so the thing lands
//! where the menu opened.

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_ship::prelude::{GalleryAction, GalleryCategory};
use nova_ui::prelude::*;

use super::{
    assets::{new_asset_items, short_name},
    layer,
    menu::OpenMenu,
    menu_context::{
        on_add_collider, on_add_mesh, on_add_socket, verb_label, verb_state, ColliderShape,
        MenuContext, MenuFacts, MenuVerbType, VerbState, COLLIDER_SHAPES,
    },
    stage_menu::PlaceAt,
};
use crate::{
    camera::ViewportPointer,
    config::{EditorCamera, EditorViewType, SelectedNode},
    frame::CameraFraming,
    glyph::{asset_mark, category_mark, choice_mark, SHIP_AI},
    node::ObjectChoice,
    placement::{create_blank_ship, create_scenario_object, place_design},
    project::{ship::owner_scenario, verbs::pressed, ModProject},
    ExampleStates,
};

/// A ship design of the mod: its id, and the name its row prints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DesignRow {
    /// The design's id.
    pub(crate) id: String,
    /// What the row says.
    pub(crate) label: String,
}

/// The ship designs a scenario can place: every ship of the mod that no
/// scenario owns. A scenario's own ship is written for that one spawn.
pub(crate) fn placeable_designs(project: Option<&ModProject>) -> Vec<DesignRow> {
    let Some(project) = project else {
        return Vec::new();
    };
    project
        .data
        .items()
        .filter(|item| item.kind() == "ship" && owner_scenario(&project.data, item.id()).is_none())
        .map(|item| DesignRow {
            id: item.id().to_string(),
            label: short_name(item, &project.namespace),
        })
        .collect()
}

/// What the Add tree reads beside the selection.
#[derive(SystemParam)]
pub(crate) struct AddFacts<'w, 's> {
    facts: MenuFacts<'w, 's>,
    project: Option<Res<'w, ModProject>>,
}

impl AddFacts<'_, '_> {
    /// The menu context with `selected` the selection.
    pub(crate) fn context(&self, selected: &SelectedNode) -> MenuContext {
        self.facts.context(selected)
    }

    /// The ship designs a scenario can place.
    pub(crate) fn designs(&self) -> Vec<DesignRow> {
        placeable_designs(self.project.as_deref())
    }
}

/// `item` as `verb` shows in `context`: none where it is hidden, greyed with
/// the reason where it cannot run.
fn by_verb(item: CascadeItem, verb: MenuVerbType, context: &MenuContext) -> Option<CascadeItem> {
    match verb_state(verb, context) {
        VerbState::Hidden => None,
        VerbState::Greyed(reason) => Some(item.greyed(reason)),
        VerbState::Live => Some(item),
    }
}

/// The group a scenario object is listed under.
fn group_of(choice: ObjectChoice) -> &'static str {
    match choice {
        ObjectChoice::Asteroid | ObjectChoice::Planet => "Objects",
        ObjectChoice::Light => "Lights",
        ObjectChoice::Anchor | ObjectChoice::Beacon => "Markers",
    }
}

/// The scenario's groups: Ship (New Ship, then every design to place),
/// Objects, Lights and Markers. Rows that add at a point carry `at`.
pub(crate) fn world_items(designs: &[DesignRow], at: Option<Vec3>) -> Vec<CascadeItem> {
    let mut ships = vec![CascadeItem::leaf("Add New Ship", "New Ship")
        // The OUTLINE ship: the row adds a design beside the one already
        // there. The first ship of an empty document becomes the player's.
        .lead(SHIP_AI)
        .on_row(|row| {
            row.insert((ButtonCueType::Owned, observe(create_blank_ship)));
        })];
    if !designs.is_empty() {
        ships.push(CascadeItem::separator());
    }
    for design in designs {
        let id = design.id.clone();
        ships.push(
            CascadeItem::leaf(format!("Add Design {}", design.id), design.label.clone())
                .lead(asset_mark("ship"))
                .on_row(move |row| {
                    row.insert((PlacedDesign(id.clone()), observe(on_add_design)));
                    if let Some(at) = at {
                        row.insert(PlaceAt(at));
                    }
                }),
        );
    }
    let mut items = vec![CascadeItem::submenu("Add Menu Ship", "Ship", ships)];
    for group in ["Objects", "Lights", "Markers"] {
        let rows = ObjectChoice::ALL
            .into_iter()
            .filter(|choice| group_of(*choice) == group)
            .map(|choice| {
                CascadeItem::leaf(format!("Add {}", choice.label()), choice.label())
                    .lead(choice_mark(choice))
                    .on_row(move |row| {
                        row.insert((
                            choice,
                            ButtonCueType::Owned,
                            observe(create_scenario_object),
                        ));
                        if let Some(at) = at {
                            row.insert(PlaceAt(at));
                        }
                    })
            })
            .collect();
        items.push(CascadeItem::submenu(
            format!("Add Menu {group}"),
            group,
            rows,
        ));
    }
    items
}

/// The whole Add tree in `context`. Rows that add at a point carry `at`.
pub(crate) fn add_items(
    context: &MenuContext,
    designs: &[DesignRow],
    at: Option<Vec3>,
) -> Vec<CascadeItem> {
    let shown = |verb| verb_state(verb, context) != VerbState::Hidden;
    let mut items = Vec::new();
    if shown(MenuVerbType::AddWorld) {
        items.extend(world_items(designs, at));
    }
    // Blocks are added by dragging them out of the palette; the greyed row
    // says where a builder used to the menu should go.
    items.extend(by_verb(
        CascadeItem::leaf("Add Script Hint", "Blocks"),
        MenuVerbType::AddScript,
        context,
    ));
    if shown(MenuVerbType::AddPart) {
        let parts = GalleryCategory::ROW
            .into_iter()
            .filter(|category| *category != GalleryCategory::All)
            .map(|category| {
                CascadeItem::leaf(
                    format!("Add {} Item", category.label()),
                    format!("{}...", category.label()),
                )
                .lead(category_mark(category))
                .on_row(move |row| {
                    row.insert(GalleryAction::Browse(category));
                })
            })
            .collect();
        items.push(CascadeItem::submenu("Add Menu Parts", "Parts", parts));
    }
    if shown(MenuVerbType::AddSectionPart) {
        let label = |verb, fallback: &str| verb_label(verb, context).unwrap_or(fallback.into());
        items.extend(by_verb(
            CascadeItem::leaf("Add Mesh Item", label(MenuVerbType::AddMesh, "Mesh...")).on_row(
                |row| {
                    row.insert(observe(on_add_mesh));
                },
            ),
            MenuVerbType::AddMesh,
            context,
        ));
        let shapes = COLLIDER_SHAPES
            .into_iter()
            .filter_map(|shape| {
                let verb = MenuVerbType::AddCollider(shape);
                by_verb(
                    CascadeItem::leaf(
                        format!("Add {shape} Collider Item"),
                        label(verb, &format!("{shape} Collider")),
                    )
                    .on_row(move |row| {
                        row.insert((ColliderShape(shape), observe(on_add_collider)));
                    }),
                    verb,
                    context,
                )
            })
            .collect();
        items.push(CascadeItem::submenu(
            "Add Menu Collider",
            "Collider",
            shapes,
        ));
        items.extend(by_verb(
            CascadeItem::leaf("Add Socket Item", "Socket").on_row(|row| {
                row.insert(observe(on_add_socket));
            }),
            MenuVerbType::AddSocket,
            context,
        ));
    }
    if !items.is_empty() {
        items.push(CascadeItem::separator());
    }
    items.push(CascadeItem::submenu(
        "Add Menu Asset",
        "Asset",
        new_asset_items(),
    ));
    items
}

/// Open the Add tree at `at`, in logical window pixels, over any open popup.
pub(crate) fn open_add_menu(
    commands: &mut Commands,
    roots: &Query<Entity, With<PopupRoot>>,
    at: Vec2,
    items: Vec<CascadeItem>,
) {
    close_popups(commands, roots);
    let menu = spawn_cascade(commands, at, layer::MENU_Z, items);
    commands
        .entity(menu)
        .insert(DespawnOnExit(ExampleStates::Editor));
}

/// The design a row places.
#[derive(Component, Clone, Debug)]
pub(crate) struct PlacedDesign(pub(crate) String);

/// Add > Ship > a design: place it in the open scenario, at the menu's point
/// or in front of the camera. One step.
fn on_add_design(
    activate: On<Activate>,
    mut commands: Commands,
    rows: Query<(&PlacedDesign, Option<&PlaceAt>)>,
) {
    let Ok((PlacedDesign(id), at)) = rows.get(activate.entity) else {
        return;
    };
    let id = id.clone();
    let at = at.map(|place| place.0);
    commands.queue(move |world: &mut World| {
        pressed(world, |world| place_design(world, &id, at));
    });
}

/// The bottom-left corner of a laid-out node, in logical window pixels: where
/// a popup anchored to it opens.
pub(crate) fn below(computed: &ComputedNode, at: &UiGlobalTransform, scale: f32) -> Vec2 {
    let corner = at.translation + computed.size() * Vec2::new(-0.5, 0.5);
    corner * computed.inverse_scale_factor() * scale
}

/// The top bar's Add: the tree, under the button.
pub(crate) fn on_add_menu_button(
    activate: On<Activate>,
    mut commands: Commands,
    scale: Option<Res<UiScale>>,
    buttons: Query<(&ComputedNode, &UiGlobalTransform)>,
    roots: Query<Entity, With<PopupRoot>>,
    facts: AddFacts,
    selected: Res<SelectedNode>,
    mut open: ResMut<OpenMenu>,
) {
    let Ok((computed, at)) = buttons.get(activate.entity) else {
        return;
    };
    open.0 = None;
    let at = below(computed, at, scale.map_or(1.0, |scale| scale.0));
    let items = add_items(&facts.context(&selected), &facts.designs(), None);
    open_add_menu(&mut commands, &roots, at, items);
}

/// The world point under the pointer at `at`, at the framing distance along
/// its ray: where a thing added at the pointer lands.
pub(crate) fn point_under(
    camera: Option<&(&Camera, &GlobalTransform)>,
    framing: &CameraFraming,
    at: Vec2,
) -> Option<Vec3> {
    let (camera, pose) = camera?;
    let ray = camera.viewport_to_world(pose, at).ok()?;
    Some(ray.origin + *ray.direction * framing.distance.to_engine())
}

/// Shift+A over the stage: the Add tree at the pointer, and what it adds lands
/// under the pointer.
#[expect(
    clippy::too_many_arguments,
    reason = "a key reads the pointer, the camera and the menu rule"
)]
pub(crate) fn add_menu_key(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    view: Res<EditorViewType>,
    pointer: Res<ViewportPointer>,
    framing: Res<CameraFraming>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<EditorCamera>>>,
    roots: Query<Entity, With<PopupRoot>>,
    facts: AddFacts,
    selected: Res<SelectedNode>,
) {
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let chord = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    if !shift || chord || !keys.just_pressed(KeyCode::KeyA) {
        return;
    }
    if !matches!(
        *view,
        EditorViewType::Scene | EditorViewType::Ship | EditorViewType::Section
    ) {
        return;
    }
    let Some(at) = pointer.at.filter(|_| !pointer.over_ui) else {
        return;
    };
    let point = point_under(camera.as_deref(), &framing, at);
    let items = add_items(&facts.context(&selected), &facts.designs(), point);
    open_add_menu(&mut commands, &roots, at, items);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{project::DocumentType, ui::menu_context::tests::context};

    /// The names of `items`, separators left out.
    fn names(items: &[CascadeItem]) -> Vec<&str> {
        items
            .iter()
            .filter(|item| !item.separator)
            .map(|item| item.name.as_str())
            .collect()
    }

    /// Each document's Add tree holds its own groups, then New Asset; a
    /// scenario's Ship group lists New Ship, then every design to place.
    #[test]
    fn each_document_adds_its_own_groups_then_a_new_asset() {
        use DocumentType as Doc;
        use EditorViewType as View;

        let designs = [DesignRow {
            id: "mod:corvette".into(),
            label: "corvette".into(),
        }];
        for (document, view, groups) in [
            (
                Doc::Scenario,
                View::Scene,
                vec![
                    "Add Menu Ship",
                    "Add Menu Objects",
                    "Add Menu Lights",
                    "Add Menu Markers",
                    "Add Menu Asset",
                ],
            ),
            (
                Doc::Scenario,
                View::Events,
                vec!["Add Script Hint", "Add Menu Asset"],
            ),
            (
                Doc::Ship,
                View::Ship,
                vec!["Add Menu Parts", "Add Menu Asset"],
            ),
            (
                Doc::Section,
                View::Section,
                vec![
                    "Add Mesh Item",
                    "Add Menu Collider",
                    "Add Socket Item",
                    "Add Menu Asset",
                ],
            ),
            (Doc::Form, View::Form, vec!["Add Menu Asset"]),
        ] {
            let items = add_items(&context(document, view), &designs, None);
            assert_eq!(names(&items), groups, "{document:?} in {view:?}");
        }
        let items = add_items(&context(Doc::Scenario, View::Scene), &designs, None);
        assert_eq!(
            names(&items[0].children),
            ["Add New Ship", "Add Design mod:corvette"]
        );
    }
}
