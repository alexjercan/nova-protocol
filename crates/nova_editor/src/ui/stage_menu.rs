//! The stage's right-click menu: the Add menu's tree, Paste, and Frame and
//! Delete on the node under the pointer.
//!
//! The right button also turns the camera, so a press is a click only while the
//! pointer and the mouse stay within [`CLICK_SLOP`] of where it went down (see
//! [`RightPress`]). [`verb_state`] decides every row as it decides the bar
//! menus' rows. A thing added from here lands at the clicked point, at the
//! framing distance along the pointer's ray.

use bevy::{
    input::mouse::AccumulatedMouseMotion,
    picking::pointer::{PointerId, PointerInteraction},
    prelude::*,
    ui_widgets::observe,
};
use nova_ui::prelude::*;

use super::{
    add_menu::{add_items, open_add_menu, point_under, AddFacts},
    menu_context::{verb_state, MenuContext, MenuVerbType, VerbState},
};
use crate::{
    camera::ViewportPointer,
    clipboard::paste_item,
    config::{EditorCamera, EditorViewType, SelectedNode},
    frame::{on_frame_selection, CameraFraming},
    node::{node_of_view, EditContext, NodeView, SectionNode},
    placement::delete_selected_node,
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
    facts: AddFacts,
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
    let point = point_under(camera.as_deref(), &framing, at);
    let mut items = add_items(&menu, &facts.designs(), point);
    let verbs = node_items(&menu, node.is_some());
    if !verbs.is_empty() {
        items.push(CascadeItem::separator());
        items.extend(verbs);
    }
    open_add_menu(&mut commands, &roots, at, items);
}

/// The stage menu's rows after the Add tree: Paste, and Frame and Delete on
/// the node under the pointer. Only the rows `menu` does not hide.
fn node_items(menu: &MenuContext, on_node: bool) -> Vec<CascadeItem> {
    let shown = |verb| verb_state(verb, menu) != VerbState::Hidden;
    let mut items = Vec::new();
    if shown(MenuVerbType::Paste) {
        items.push(
            CascadeItem::leaf("Stage Paste", "Paste")
                .hint("Ctrl+V")
                .on_row(|row| {
                    row.insert(observe(paste_item));
                }),
        );
    }
    if !on_node {
        return items;
    }
    if shown(MenuVerbType::Frame) {
        items.push(
            CascadeItem::leaf("Stage Frame", "Frame")
                .hint("F")
                .on_row(|row| {
                    row.insert(observe(on_frame_selection));
                }),
        );
    }
    if shown(MenuVerbType::Delete) {
        items.push(
            CascadeItem::leaf("Stage Delete", "Delete")
                .hint("Del")
                .on_row(|row| {
                    row.insert(observe(delete_selected_node));
                }),
        );
    }
    items
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
