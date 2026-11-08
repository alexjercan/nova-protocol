//! Putting the camera on a node, on demand.
//!
//! [`crate::node::sync_camera_focus`] frames the CONTEXT, and only when the
//! context changes - it is what makes entering a ship show that ship. This is
//! the other half: the gestures that say "look at THIS", whatever the context
//! is. A click on a tree row, the F key, and View > Frame Selection all raise
//! the same request, and one system serves it.
//!
//! A request rather than three systems that each move the camera, because a
//! node placed this frame has no bounds yet, and the request waits for them.

use avian3d::prelude::{ColliderAabb, Sensor};
use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::units::prelude::*;
use nova_gameplay::prelude::subtree_bounding_sphere;
use nova_ship::prelude::HeldPart;

use crate::{
    config::{EditorCamera, EditorViewType, SelectedNode},
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

/// What the camera was last put on, and how far its eye stood from it.
///
/// The editor's two distance-dependent gestures - dropping a new object out in
/// front of the camera, and dragging a position field - are unusable at a fixed
/// scale. A fixed 30 u lands a beacon on the lens while a 7 km range is in
/// frame and behind everything while a cockpit is; a fixed half-metre per pixel
/// is five metres a pixel inside a hull and 16,000 px across the tutorial's
/// range. Both read this instead, so the gesture follows what is on screen.
///
/// Written wherever the camera is PUT on something - context focus, F, Frame
/// Selection, a view preset, the end of an orbit, a dolly or a pan - and never
/// while the builder looks about. A turn in place has no target to measure
/// against.
///
/// [`crate::camera`] reads it as the pivot's distance when nothing is selected.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct CameraFraming {
    /// The point the camera was framed on.
    pub(crate) point: Vec3,
    /// How far the eye stood from that point.
    pub(crate) distance: Meters,
}

impl Default for CameraFraming {
    fn default() -> Self {
        Self::on(Vec3::ZERO, frame_stage(Vec3::ZERO, 0.0).translation)
    }
}

impl CameraFraming {
    /// The framing an eye at `eye` looking at `point` makes.
    pub(crate) fn on(point: Vec3, eye: Vec3) -> Self {
        Self {
            point,
            distance: Meters::from_engine(eye.distance(point)),
        }
    }
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
    selection: Res<HeldPart>,
    selected: Res<SelectedNode>,
    context: Res<EditContext>,
    rail: Res<EditorViewType>,
    mut request: ResMut<FrameRequest>,
) {
    // In Events the key is the canvas's: there is no stage to frame.
    if rail.is_events() || *selection != HeldPart(None) || !keys.just_pressed(FRAME_KEY) {
        return;
    }
    ask_for(
        &mut request,
        selected.primary().or_else(|| context.current()),
    );
}

/// View > Frame Selection: the same request the key raises.
pub(crate) fn on_frame_selection(
    _activate: On<Activate>,
    selected: Res<SelectedNode>,
    context: Res<EditContext>,
    mut request: ResMut<FrameRequest>,
) {
    ask_for(
        &mut request,
        selected.primary().or_else(|| context.current()),
    );
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
        selected.primary().or_else(|| context.current()),
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
    // builder asking to be put back on the axis after moving off it.
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
pub(crate) fn apply_frame_request(
    mut request: ResMut<FrameRequest>,
    mut framing: ResMut<CameraFraming>,
    q_children: Query<&Children>,
    q_bounds: Query<&ColliderAabb, Without<Sensor>>,
    q_poses: Query<&Transform, Without<EditorCamera>>,
    camera: Option<Single<&mut Transform, With<EditorCamera>>>,
    context: Res<EditContext>,
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
    // The entered ship stands on the origin, whatever its pose says.
    let origin = if context.ship() == Some(node) {
        Vec3::ZERO
    } else {
        pose.translation
    };
    let (centre, spread) =
        subtree_bounding_sphere(node, &q_children, &q_bounds).unwrap_or((origin, 0.0));
    let angle = request.angle;
    request.node = None;
    let mut transform = camera.into_inner();
    *transform = view_stage(centre, spread, angle);
    *framing = CameraFraming::on(centre, transform.translation);
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

/// One preset row, carrying the direction it looks from.
#[derive(Component)]
pub(crate) struct ViewPresetItem(pub(crate) ViewAngle);

#[cfg(test)]
mod tests;
