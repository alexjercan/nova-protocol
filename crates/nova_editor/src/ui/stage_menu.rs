//! The stage's right-click menu: the Add verbs of the view, Paste, and Frame
//! and Delete on the node under the pointer.
//!
//! The right button also turns the camera, so a press is a click only while the
//! pointer and the mouse stay within [`CLICK_SLOP`] of where it went down (see
//! [`RightPress`]). Every row carries
//! its [`MenuVerbType`], so [`verb_state`] decides it as it decides the bar
//! menus' rows. An object added from here lands at the clicked point, at the
//! framing distance along the pointer's ray.

use bevy::{
    input::mouse::AccumulatedMouseMotion,
    picking::pointer::{PointerId, PointerInteraction},
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_ship::prelude::GalleryState;
use nova_ui::prelude::*;

use super::{
    layer,
    menu_context::{
        on_add_collider, on_add_mesh, on_add_socket, verb_label, verb_state, MenuFacts,
        MenuVerbType, VerbState, COLLIDER_SHAPES,
    },
};
use crate::{
    camera::ViewportPointer,
    clipboard::paste_item,
    config::{EditorCamera, EditorViewType, SelectedNode},
    frame::{on_frame_selection, CameraFraming},
    node::{node_of_view, EditContext, NodeView, ObjectChoice, SectionNode},
    placement::{create_blank_ship, create_scenario_object, delete_selected_node},
    ExampleStates,
};

/// How far a right press may wander, in pixels, and still open the menu.
const CLICK_SLOP: f32 = 4.0;

/// Where an Add row of the stage menu puts what it adds, in world space.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct PlaceAt(pub(crate) Vec3);

/// A right press that may end as a click: where it went down, and how far the
/// mouse has moved since. Look locks the cursor while the right button turns
/// the view, so the pointer stays put through a look and only the mouse's own
/// travel tells a look from a click.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RightPress {
    from: Option<Vec2>,
    travel: f32,
}

impl RightPress {
    /// The button went down at `at`, or off the target with `None`.
    pub(crate) fn press(&mut self, at: Option<Vec2>) {
        self.from = at;
        self.travel = 0.0;
    }

    /// The mouse moved by `delta` while the button is held.
    pub(crate) fn moved(&mut self, delta: Vec2) {
        if self.from.is_some() {
            self.travel += delta.length();
        }
    }

    /// The button came up at `at`: the point to open the menu at, when neither
    /// the pointer nor the mouse went past [`CLICK_SLOP`].
    pub(crate) fn release(&mut self, at: Option<Vec2>) -> Option<Vec2> {
        let from = self.from.take()?;
        let at = at?;
        (from.distance(at) <= CLICK_SLOP && self.travel <= CLICK_SLOP).then_some(at)
    }
}

/// The right press over the stage, while it is held.
#[derive(Resource, Default)]
pub(crate) struct StageRightPress(RightPress);

/// The node the pointer is over now, if a node's view is the nearest hit.
fn node_under(
    pointers: &Query<(&PointerId, &PointerInteraction)>,
    views: &Query<&ChildOf, With<NodeView>>,
) -> Option<Entity> {
    pointers
        .iter()
        .filter(|(id, _)| id.is_mouse())
        .find_map(|(_, interaction)| interaction.get_nearest_hit())
        .and_then(|(hit, _)| node_of_view(*hit, views))
}

/// A right click on the stage opens its menu at the pointer. A right drag is
/// the camera's.
#[expect(
    clippy::too_many_arguments,
    reason = "a click reads the pointer, the hit, the camera and the menu rule"
)]
pub(crate) fn stage_context_menu(
    mut commands: Commands,
    (mouse, motion): (Res<ButtonInput<MouseButton>>, Res<AccumulatedMouseMotion>),
    pointer: Res<ViewportPointer>,
    view: Res<EditorViewType>,
    context: Res<EditContext>,
    framing: Res<CameraFraming>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<EditorCamera>>>,
    (pointers, views, sections): (
        Query<(&PointerId, &PointerInteraction)>,
        Query<&ChildOf, With<NodeView>>,
        Query<&ChildOf, With<SectionNode>>,
    ),
    facts: MenuFacts,
    mut selected: ResMut<SelectedNode>,
    mut press: ResMut<StageRightPress>,
    roots: Query<Entity, With<PopupRoot>>,
) {
    if !matches!(
        *view,
        EditorViewType::Scene | EditorViewType::Ship | EditorViewType::Section
    ) {
        press.0.press(None);
        return;
    }
    if mouse.just_pressed(MouseButton::Right) {
        press.0.press(pointer.at.filter(|_| !pointer.over_ui));
        return;
    }
    press.0.moved(motion.delta);
    if !mouse.just_released(MouseButton::Right) {
        return;
    }
    let Some(at) = press.0.release(pointer.at) else {
        return;
    };
    // A part of a ship the editor is not in stands for its ship, as a left
    // click on it does.
    let node = node_under(&pointers, &views).map(|node| match sections.get(node) {
        Ok(owner) if context.ship() != Some(owner.parent()) => owner.parent(),
        _ => node,
    });
    if let Some(node) = node.filter(|node| !selected.contains(*node)) {
        selected.set(Some(node));
    }
    let menu = facts.context(&selected);
    let point = camera
        .and_then(|camera| {
            let (camera, pose) = *camera;
            camera.viewport_to_world(pose, at).ok()
        })
        .map(|ray| ray.origin + *ray.direction * framing.distance.to_engine());
    close_popups(&mut commands, &roots);
    let popup = spawn_popup(&mut commands, at, layer::MENU_Z, |rows| {
        spawn_stage_rows(rows, &menu, point, node.is_some());
    });
    commands
        .entity(popup)
        .insert(DespawnOnExit(ExampleStates::Editor));
}

/// The rows of the stage menu that `menu` does not hide.
fn spawn_stage_rows(
    rows: &mut bevy::ecs::relationship::RelatedSpawnerCommands<ChildOf>,
    menu: &super::menu_context::MenuContext,
    point: Option<Vec3>,
    on_node: bool,
) {
    let shown = |verb| verb_state(verb, menu) != VerbState::Hidden;
    let label = |verb, fallback: String| verb_label(verb, menu).unwrap_or(fallback);
    if shown(MenuVerbType::AddWorld) {
        rows.spawn((
            Name::new("Stage Add Ship"),
            MenuVerbType::AddWorld,
            popup_row("Add Ship", ""),
            observe(create_blank_ship),
        ));
        for choice in ObjectChoice::ALL {
            let mut row = rows.spawn((
                Name::new(format!("Stage Add {}", choice.label())),
                MenuVerbType::AddWorld,
                popup_row(&format!("Add {}", choice.label()), ""),
                choice,
                observe(create_scenario_object),
            ));
            if let Some(point) = point {
                row.insert(PlaceAt(point));
            }
        }
    }
    if shown(MenuVerbType::AddPart) {
        rows.spawn((
            Name::new("Stage Add Part"),
            MenuVerbType::AddPart,
            popup_row("Add Part...", ""),
            observe(open_parts),
        ));
    }
    if shown(MenuVerbType::AddMesh) {
        rows.spawn((
            Name::new("Stage Add Mesh"),
            MenuVerbType::AddMesh,
            popup_row(&label(MenuVerbType::AddMesh, "Mesh...".into()), ""),
            observe(on_add_mesh),
        ));
    }
    for shape in COLLIDER_SHAPES {
        let verb = MenuVerbType::AddCollider(shape);
        if shown(verb) {
            rows.spawn((
                Name::new(format!("Stage Add {shape} Collider")),
                verb,
                popup_row(&label(verb, format!("{shape} Collider")), ""),
                observe(on_add_collider),
            ));
        }
    }
    if shown(MenuVerbType::AddSocket) {
        rows.spawn((
            Name::new("Stage Add Socket"),
            MenuVerbType::AddSocket,
            popup_row("Add Socket", ""),
            observe(on_add_socket),
        ));
    }
    if shown(MenuVerbType::Paste) {
        rows.spawn((
            Name::new("Stage Paste"),
            MenuVerbType::Paste,
            popup_row("Paste", "Ctrl+V"),
            observe(paste_item),
        ));
    }
    if !on_node {
        return;
    }
    if shown(MenuVerbType::Frame) {
        rows.spawn((
            Name::new("Stage Frame"),
            MenuVerbType::Frame,
            popup_row("Frame", "F"),
            observe(on_frame_selection),
        ));
    }
    if shown(MenuVerbType::Delete) {
        rows.spawn((
            Name::new("Stage Delete"),
            MenuVerbType::Delete,
            popup_row("Delete", "Del"),
            observe(delete_selected_node),
        ));
    }
}

/// Add Part: the parts gallery, for the ship the editor is in.
fn open_parts(_activate: On<Activate>, mut gallery: ResMut<GalleryState>) {
    *gallery = gallery.opened();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_right_press_that_stays_put_is_a_click() {
        let mut press = RightPress::default();
        press.press(Some(Vec2::new(100.0, 80.0)));
        press.moved(Vec2::new(1.0, 1.0));
        assert_eq!(
            press.release(Some(Vec2::new(102.0, 80.0))),
            Some(Vec2::new(102.0, 80.0))
        );
        assert_eq!(press.release(Some(Vec2::new(102.0, 80.0))), None);
    }

    /// Look locks the cursor, so the pointer comes up where it went down; the
    /// mouse's travel alone says the press turned the view.
    #[test]
    fn a_look_with_the_cursor_locked_is_not_a_click() {
        let mut press = RightPress::default();
        let at = Vec2::new(100.0, 80.0);
        press.press(Some(at));
        for _ in 0..5 {
            press.moved(Vec2::new(12.0, -3.0));
        }
        assert_eq!(press.release(Some(at)), None);
    }

    #[test]
    fn a_right_drag_or_a_press_off_the_target_is_not_a_click() {
        let mut press = RightPress::default();
        press.press(Some(Vec2::new(100.0, 80.0)));
        assert_eq!(press.release(Some(Vec2::new(140.0, 80.0))), None);
        press.press(None);
        press.moved(Vec2::ZERO);
        assert_eq!(press.release(Some(Vec2::new(100.0, 80.0))), None);
    }
}
