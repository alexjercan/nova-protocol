//! Picking nodes that are not under a click: a press on nothing, a box, Ctrl+A.
//!
//! The clicks on a body are `crate::placement`'s and the clicks on a row are the
//! tree's. What lives here is the rest of what a selection answers to.

use bevy::{prelude::*, ui::UiScale, ui_widgets::Activate};
use nova_ship::prelude::HeldPart;

use crate::{
    camera::ViewportPointer,
    config::{EditorCamera, EditorViewType, SelectedNode},
    event::EventNode,
    keybind::EditorRebind,
    node::{EditContext, ObjectNode, SectionNode, ShipNode},
    palette,
    placement::alt_held,
    tool::{snapping, StageTool, StageToolType},
    ExampleStates,
};

/// How far a press may wander, in pixels, and still be a click. Past it, an
/// empty-space press is a box.
const CLICK_SLOP: f32 = 4.0;

/// Whether a click takes a node into the selection or puts it down, rather than
/// replacing the selection.
pub(crate) fn toggling(keys: Option<&ButtonInput<KeyCode>>) -> bool {
    snapping(keys)
        || keys.is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]))
}

/// A press that landed on nothing, while it is held.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EmptyPress {
    /// Where it landed, or `None` with no such press held.
    from: Option<Vec2>,
    /// Where the pointer is now.
    to: Vec2,
    /// Ctrl or Shift was down, so the box adds and a click keeps.
    adding: bool,
}

impl EmptyPress {
    /// Whether this press has wandered past a click.
    fn wandered(&self) -> bool {
        self.from
            .is_some_and(|from| from.distance(self.to) > CLICK_SLOP)
    }
}

/// The box on screen, shown while an empty press draws one.
#[derive(Component)]
pub(crate) struct SelectionFrame;

/// Whether `point` is inside the box the two corners draw, either way round.
fn inside(a: Vec2, b: Vec2, point: Vec2) -> bool {
    let (low, high) = (a.min(b), a.max(b));
    point.cmpge(low).all() && point.cmple(high).all()
}

/// The nodes a box or Ctrl+A can take in this context.
fn listed(
    context: &EditContext,
    q_children: &Query<&Children>,
    q_selectable: &Query<(), Or<(With<ShipNode>, With<ObjectNode>, With<SectionNode>)>>,
) -> Vec<Entity> {
    context
        .current()
        .and_then(|node| q_children.get(node).ok())
        .into_iter()
        .flatten()
        .copied()
        .filter(|child| q_selectable.contains(*child))
        .collect()
}

/// A press on nothing deselects, and a drag from one draws a box.
///
/// A system, not an observer: nothing is there to observe. A body and a handle
/// both take the press first, and [`ViewportPointer::over_nothing`] says neither
/// did. Alt is the camera's and a part in hand owns the left button.
#[expect(
    clippy::too_many_arguments,
    reason = "a press reads the pointer, the tool, and every node a box can reach"
)]
pub(crate) fn empty_space_press(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    pointer: Res<ViewportPointer>,
    held: Res<HeldPart>,
    rebind: Res<EditorRebind>,
    tool: Res<StageTool>,
    context: Res<EditContext>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<EditorCamera>>>,
    q_children: Query<&Children>,
    q_selectable: Query<(), Or<(With<ShipNode>, With<ObjectNode>, With<SectionNode>)>>,
    q_poses: Query<&GlobalTransform>,
    mut press: ResMut<EmptyPress>,
    mut selected: ResMut<SelectedNode>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        let free = pointer.over_nothing
            && !alt_held(keys.as_deref())
            && *held == HeldPart(None)
            && rebind.target.is_none();
        *press = EmptyPress {
            from: pointer.at.filter(|_| free),
            to: pointer.at.unwrap_or_default(),
            adding: toggling(keys.as_deref()),
        };
        return;
    }
    let Some(from) = press.from else {
        return;
    };
    if mouse.pressed(MouseButton::Left) {
        if let Some(at) = pointer.at.filter(|at| *at != press.to) {
            press.to = at;
        }
        return;
    }

    let done = std::mem::take(&mut *press);
    if !done.wandered() {
        if !done.adding && !selected.is_empty() {
            selected.set(None);
        }
        return;
    }
    if tool.tool != StageToolType::Select {
        return;
    }
    let Some(camera) = camera else {
        return;
    };
    let (camera, camera_pose) = *camera;
    let boxed: Vec<Entity> = listed(&context, &q_children, &q_selectable)
        .into_iter()
        .filter(|node| {
            q_poses
                .get(*node)
                .ok()
                .and_then(|pose| {
                    camera
                        .world_to_viewport(camera_pose, pose.translation())
                        .ok()
                })
                .is_some_and(|point| inside(from, done.to, point))
        })
        .collect();
    if done.adding {
        for node in boxed {
            selected.add(node);
        }
    } else {
        selected.set_all(boxed);
    }
}

/// Ctrl+A: everything at the current level, or every handler on the script tab.
pub(crate) fn select_all(
    keys: Res<ButtonInput<KeyCode>>,
    context: Res<EditContext>,
    tab: Res<EditorViewType>,
    held: Res<HeldPart>,
    q_children: Query<&Children>,
    q_selectable: Query<(), Or<(With<ShipNode>, With<ObjectNode>, With<SectionNode>)>>,
    q_events: Query<Entity, With<EventNode>>,
    mut selected: ResMut<SelectedNode>,
) {
    if !(snapping(Some(&keys)) && keys.just_pressed(KeyCode::KeyA)) || *held != HeldPart(None) {
        return;
    }
    mark_all(
        &tab,
        &context,
        &q_children,
        &q_selectable,
        &q_events,
        &mut selected,
    );
}

/// Edit > Select All: the same verb as Ctrl+A.
pub(crate) fn select_all_item(
    _activate: On<Activate>,
    context: Res<EditContext>,
    tab: Res<EditorViewType>,
    q_children: Query<&Children>,
    q_selectable: Query<(), Or<(With<ShipNode>, With<ObjectNode>, With<SectionNode>)>>,
    q_events: Query<Entity, With<EventNode>>,
    mut selected: ResMut<SelectedNode>,
) {
    mark_all(
        &tab,
        &context,
        &q_children,
        &q_selectable,
        &q_events,
        &mut selected,
    );
}

/// Mark every handler on the script tab, or everything at the current level.
fn mark_all(
    tab: &EditorViewType,
    context: &EditContext,
    q_children: &Query<&Children>,
    q_selectable: &Query<(), Or<(With<ShipNode>, With<ObjectNode>, With<SectionNode>)>>,
    q_events: &Query<Entity, With<EventNode>>,
    selected: &mut SelectedNode,
) {
    if tab.is_events() {
        selected.set_all(q_events.iter());
    } else {
        selected.set_all(listed(context, q_children, q_selectable));
    }
}

/// Put the box frame on the screen, once per visit.
pub(crate) fn setup_selection_frame(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(ExampleStates::Editor),
        Name::new("Selection Box"),
        SelectionFrame,
        Pickable::IGNORE,
        GlobalZIndex(i32::MAX - 1),
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            border: UiRect::all(px(1.0)),
            ..default()
        },
        BorderColor::all(palette::GO),
        BackgroundColor(palette::GO.with_alpha(0.08)),
    ));
}

/// Keep the frame on the press. A click that has not wandered draws nothing.
pub(crate) fn sync_selection_frame(
    press: Res<EmptyPress>,
    tool: Res<StageTool>,
    scale: Option<Res<UiScale>>,
    mut frames: Query<&mut Node, With<SelectionFrame>>,
) {
    let shown = press
        .from
        .filter(|_| press.wandered() && tool.tool == StageToolType::Select);
    let scale = scale.map_or(1.0, |scale| scale.0);
    for mut frame in &mut frames {
        let Some(from) = shown else {
            if frame.display != Display::None {
                frame.display = Display::None;
            }
            continue;
        };
        let low = from.min(press.to) / scale;
        let size = (from - press.to).abs() / scale;
        frame.display = Display::Flex;
        frame.left = px(low.x);
        frame.top = px(low.y);
        frame.width = px(size.x);
        frame.height = px(size.y);
    }
}

#[cfg(test)]
mod tests;
