//! Putting the camera on a node, on demand.
//!
//! [`crate::node::sync_camera_focus`] frames the CONTEXT, and only when the
//! context changes - it is what makes entering a ship show that ship. This is
//! the other half: the gestures that say "look at THIS", whatever the context
//! is. A click on a tree row, the F key, and View > Frame Selection all raise
//! the same request, and one system serves it.
//!
//! A request rather than three systems that each move the camera, because the
//! move is not a bare pose write - the free-fly rig has to be taken off and put
//! back around it, and a second place that got that wrong would be a camera
//! that snaps back a frame later.

use avian3d::prelude::{ColliderAabb, Sensor};
use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use nova_ship::prelude::WASDCameraController;

use crate::{
    config::{SectionChoice, SelectedNode},
    gallery::EditorCamera,
    node::{frame_stage, EditContext},
};

/// Frames the selection. Free in select mode: the placement keys only answer
/// while a part is armed (`cycle_placement_pose` reads the same F to cycle a
/// socket), so the two never both fire.
pub(crate) const FRAME_KEY: KeyCode = KeyCode::KeyF;

/// The node the camera has been asked to look at, until it has.
///
/// Held rather than acted on where it is raised: an observer on a tree row
/// cannot see the camera, and a node placed this frame has no collider AABB
/// until the physics step runs. The request survives until it is served.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub(crate) struct FrameRequest(pub(crate) Option<Entity>);

/// F frames the selection, or the node the editor is standing in when nothing
/// is selected - which is the answer to "I have flown away, put me back".
pub(crate) fn frame_key(
    keys: Res<ButtonInput<KeyCode>>,
    selection: Res<SectionChoice>,
    selected: Res<SelectedNode>,
    context: Res<EditContext>,
    mut request: ResMut<FrameRequest>,
) {
    if *selection != SectionChoice::None || !keys.just_pressed(FRAME_KEY) {
        return;
    }
    ask_for(&mut request, selected.0.or_else(|| context.current()));
}

/// View > Frame Selection: the same request the key raises.
pub(crate) fn on_frame_selection(
    _activate: On<Activate>,
    selected: Res<SelectedNode>,
    context: Res<EditContext>,
    mut request: ResMut<FrameRequest>,
) {
    ask_for(&mut request, selected.0.or_else(|| context.current()));
}

/// Raise a request for `node`, if there is one to frame.
pub(crate) fn ask_for(request: &mut FrameRequest, node: Option<Entity>) {
    if node.is_some() && request.0 != node {
        request.0 = node;
    }
}

/// Serve the standing request: put the camera where the node fills the frame.
///
/// The pose comes from the node's own collider AABBs rather than from its
/// `Transform`, so framing a ship frames the SHIP and not the point its first
/// section was founded on. A node with no colliders yet - one placed this
/// frame - falls back to its origin rather than waiting, because a camera that
/// moved a frame late is better than one that did not move at all.
///
/// The free-fly rig rewrites the camera transform every frame from private
/// state, so it comes off and goes back on around the write; its setup re-reads
/// the transform this leaves. The same move [`crate::node::sync_camera_focus`]
/// and the gallery's parking both make.
pub(crate) fn apply_frame_request(
    mut commands: Commands,
    mut request: ResMut<FrameRequest>,
    q_children: Query<&Children>,
    q_bounds: Query<&ColliderAabb, Without<Sensor>>,
    q_poses: Query<&Transform, Without<EditorCamera>>,
    camera: Option<Single<(Entity, &mut Transform), With<EditorCamera>>>,
) {
    let Some(node) = request.0 else {
        return;
    };
    let Ok(pose) = q_poses.get(node) else {
        // The node is gone (deleted while the request stood). Nothing to look
        // at, and a request nobody can serve would stand for ever.
        request.0 = None;
        return;
    };
    // Held, not dropped: the camera is spawned through commands on the way into
    // the editor, so a request raised in the same frame arrives before it.
    let Some(camera) = camera else {
        return;
    };
    let (centre, spread) = match node_bounds(node, &q_children, &q_bounds) {
        Some(bounds) => (bounds.center(), bounds.size().length() * 0.5),
        None => (pose.translation, 0.0),
    };
    request.0 = None;
    let (entity, mut transform) = camera.into_inner();
    *transform = frame_stage(centre, spread);
    commands
        .entity(entity)
        .remove::<WASDCameraController>()
        .insert(WASDCameraController);
}

/// The world-space box `node` and everything under it occupies, or `None` when
/// nothing in the subtree has a collider.
///
/// The whole subtree, because a document node carries no geometry itself: a
/// ship's size is its sections' views, an object's is its own view. SENSORS are
/// excluded - a beacon's trigger sphere is tens of units of trigger volume, not
/// tens of units of beacon, and framing the trigger would put the beacon in the
/// middle of an empty screen.
pub(crate) fn node_bounds(
    node: Entity,
    q_children: &Query<&Children>,
    q_bounds: &Query<&ColliderAabb, Without<Sensor>>,
) -> Option<ColliderAabb> {
    let mut found: Option<ColliderAabb> = None;
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if let Ok(bounds) = q_bounds.get(current) {
            found = Some(match found {
                Some(existing) => existing.merged(*bounds),
                None => *bounds,
            });
        }
        if let Ok(children) = q_children.get(current) {
            stack.extend(children.iter());
        }
    }
    found
}

/// Grey View > Frame Selection when there is nothing to frame.
///
/// There almost always is - the context node is the fallback - so this reads
/// as enabled for the whole session and greys only before the document exists.
/// Painted rather than assumed, because the item shipped greyed and a row that
/// silently became live would say nothing about when it did.
pub(crate) fn sync_frame_item(
    mut commands: Commands,
    selected: Res<SelectedNode>,
    context: Res<EditContext>,
    items: Query<(Entity, Has<InteractionDisabled>), With<FrameSelectionItem>>,
) {
    let disabled = selected.0.or_else(|| context.current()).is_none();
    for (entity, marked) in &items {
        match (disabled, marked) {
            (true, false) => {
                commands.entity(entity).insert(InteractionDisabled);
            }
            (false, true) => {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
            _ => {}
        }
    }
}

/// The View menu's Frame Selection row, so [`sync_frame_item`] can find it.
#[derive(Component)]
pub(crate) struct FrameSelectionItem;

#[cfg(test)]
mod tests;
