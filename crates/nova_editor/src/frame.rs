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
use nova_ui::prelude::InputMode;

use crate::{
    config::{SectionChoice, SelectedNode},
    gallery::EditorCamera,
    node::{frame_stage, EditContext},
};

/// Frames the selection. Free in select mode: the placement keys only answer
/// while a part is armed (`cycle_placement_pose` reads the same F to cycle a
/// socket), so the two never both fire.
pub(crate) const FRAME_KEY: KeyCode = KeyCode::KeyF;

/// The node the camera has been asked to look at, and from where, until it has.
///
/// Held rather than acted on where it is raised: an observer on a tree row
/// cannot see the camera, and a node placed this frame has no collider AABB
/// until the physics step runs. The request survives until it is served.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub(crate) struct FrameRequest {
    /// What to look at.
    pub(crate) node: Option<Entity>,
    /// Which way to come at it from.
    pub(crate) angle: ViewAngle,
}

/// Where the camera stands to look at what it is framing.
///
/// Sockets are axis-aligned, so an axis-TRUE view is how a builder checks that
/// a part mated where they meant it to - a mate that is one socket out reads as
/// correct from every other angle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ViewAngle {
    /// The stock editor view: above and behind, the pose the stage spawns in.
    #[default]
    Stock,
    /// Down the nose, from ahead of the ship.
    Front,
    /// From the ship's starboard side.
    Side,
    /// Straight down.
    Top,
    /// Off one shoulder, all three axes in view.
    Iso,
}

impl ViewAngle {
    /// The presets the View menu offers, in the order it lists them.
    pub(crate) const PRESETS: [Self; 4] = [Self::Front, Self::Side, Self::Top, Self::Iso];

    /// The menu row's label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Stock => "Stage",
            Self::Front => "Front",
            Self::Side => "Side",
            Self::Top => "Top",
            Self::Iso => "Iso",
        }
    }

    /// Which way the eye sits from the target, as a unit vector. `None` for
    /// [`ViewAngle::Stock`], which is not a direction but the stage's own pose.
    ///
    /// A ship's nose points along -Z, so FRONT is out at +Z looking back down
    /// the hull.
    fn eye(self) -> Option<Vec3> {
        match self {
            Self::Stock => None,
            Self::Front => Some(Vec3::Z),
            Self::Side => Some(Vec3::X),
            Self::Top => Some(Vec3::Y),
            Self::Iso => Some(Vec3::new(1.0, 1.0, 1.0).normalize()),
        }
    }

    /// Which way is up on screen. Straight down needs its own answer: with the
    /// eye on +Y the world's up is the way the camera is looking, and a camera
    /// cannot be levelled against the axis it points along.
    fn up(self) -> Vec3 {
        match self {
            Self::Top => Vec3::NEG_Z,
            _ => Vec3::Y,
        }
    }
}

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

/// View > Front/Side/Top/Iso: the same target, from a named direction.
pub(crate) fn on_view_preset(
    activate: On<Activate>,
    presets: Query<&ViewPresetItem>,
    selected: Res<SelectedNode>,
    context: Res<EditContext>,
    mut request: ResMut<FrameRequest>,
) {
    let Ok(preset) = presets.get(activate.entity) else {
        return;
    };
    look_from(
        &mut request,
        selected.0.or_else(|| context.current()),
        preset.0,
    );
}

/// Raise a request for `node`, if there is one to frame, from the stage's own
/// pose.
pub(crate) fn ask_for(request: &mut FrameRequest, node: Option<Entity>) {
    look_from(request, node, ViewAngle::Stock);
}

/// The same, from a named direction.
pub(crate) fn look_from(request: &mut FrameRequest, node: Option<Entity>, angle: ViewAngle) {
    // The ANGLE is half of the request: a second Front on the same node is a
    // builder asking to be put back on the axis after flying off it.
    if node.is_some() && (request.node != node || request.angle != angle) {
        request.node = node;
        request.angle = angle;
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
    let Some(node) = request.node else {
        return;
    };
    let Ok(pose) = q_poses.get(node) else {
        // The node is gone (deleted while the request stood). Nothing to look
        // at, and a request nobody can serve would stand for ever.
        request.node = None;
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
    let angle = request.angle;
    request.node = None;
    let (entity, mut transform) = camera.into_inner();
    *transform = view_stage(centre, spread, angle);
    commands
        .entity(entity)
        .remove::<WASDCameraController>()
        .insert(WASDCameraController);
}

/// Marks the camera whose free-fly rig the mode hold took away, so putting it
/// back is not a guess. The same shape the gallery parks with.
#[derive(Component)]
pub(crate) struct ModeHold;

/// Take the free-fly rig off the camera whenever the keyboard is not in
/// [`InputMode::Normal`], and give it back when it is.
///
/// The rig is `bevy_enhanced_input`'s and answers WASD wherever the keystrokes
/// came from, so a run condition on the editor's own systems does not reach it.
/// Typing "wasp" into a beacon label flew the camera four ways; binding `W` to
/// a thruster - the most natural thruster binding there is - flew it forward
/// for as long as the key was held, so the part the chip points at slid off
/// screen mid-gesture.
///
/// Every mode above Normal, not a list of them: a mode holds the keyboard by
/// definition, and a rig that answers keys is a keyboard consumer like any
/// other. The marker says the hold is OURS, so the gallery's park - which stows
/// the pose and the skybox for its own reasons - cannot be undone by this.
pub(crate) fn hold_camera_above_normal(
    mut commands: Commands,
    mode: Res<InputMode>,
    camera: Option<Single<(Entity, Has<WASDCameraController>, Has<ModeHold>), With<EditorCamera>>>,
) {
    let Some(camera) = camera else {
        return;
    };
    let (entity, driving, held) = *camera;
    let taken = *mode != InputMode::Normal;
    if taken && driving {
        commands
            .entity(entity)
            .insert(ModeHold)
            .remove::<WASDCameraController>();
    } else if !taken && held {
        commands
            .entity(entity)
            .remove::<ModeHold>()
            .insert(WASDCameraController);
    }
}

/// The view over `target` from `angle`, at the reach the stage view uses - so
/// an axis view is the stock view turned, and not a jump to a new distance.
pub(crate) fn view_stage(target: Vec3, spread: f32, angle: ViewAngle) -> Transform {
    let stock = frame_stage(target, spread);
    match angle.eye() {
        None => stock,
        Some(eye) => {
            let reach = stock.translation.distance(target);
            Transform::from_translation(target + eye * reach).looking_at(target, angle.up())
        }
    }
}

/// The View menu's rows that need something to look at: Frame Selection and the
/// four presets, greyed together by [`sync_frame_item`].
#[derive(Component)]
pub(crate) struct FrameSelectionItem;

/// One preset row, carrying the direction it looks from.
#[derive(Component)]
pub(crate) struct ViewPresetItem(pub(crate) ViewAngle);

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

#[cfg(test)]
mod tests;
