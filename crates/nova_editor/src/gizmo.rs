//! The handles that move and turn the selected node.
//!
//! Dragging a node's BODY slides it on the ground plane
//! ([`crate::placement::on_stage_drag`]): one gesture, two axes, no way to say
//! "up" and no way to say "turn". The gizmo is the rest of that vocabulary -
//! three arrows and three rings, each one axis of the same node.
//!
//! It is a real rig of meshes rather than immediate-mode [`Gizmos`] lines
//! because the handles have to be POINTED AT, and only geometry can be picked.
//! That is also why the mesh picking backend is registered here and set to
//! `require_markers`: the stage already answers the pointer through avian's
//! colliders, and a second backend that picked every mesh in the scene would
//! double every hit on the ship. With markers required, mesh picking sees the
//! six handles and nothing else.
//!
//! Scenario-node only. Inside a ship, mating decides where a part sits - the
//! same reason [`crate::placement::on_stage_drag_start`] refuses in there.

use avian3d::prelude::{ColliderAabb, Sensor};
use bevy::{
    picking::{
        mesh_picking::{MeshPickingPlugin, MeshPickingSettings},
        PickingPlugin,
    },
    prelude::*,
};
use nova_ui::theme;

use crate::{
    config::{SectionChoice, SelectedNode},
    frame::node_bounds,
    gallery::{EditorCamera, GalleryState},
    node::{EditContext, ObjectNode, ShipNode},
    ExampleStates,
};

/// Where the arms END, in rig-local units. Everything else is a fraction of it,
/// so the whole rig scales by scaling the root.
const ARM: f32 = 1.0;
/// Where the arms BEGIN. The centre is left hollow so the node itself can still
/// be pointed at: the body under the rig is draggable and clickable, and an
/// arm through the middle of it would take every click aimed at the middle.
const ARM_START: f32 = 0.4;
/// Arm thickness. Thin enough to read as a line, thick enough to hit.
const ARM_RADIUS: f32 = 0.035;
/// The arrowhead on the end of an arm.
const TIP: f32 = 0.26;
const TIP_RADIUS: f32 = 0.1;
/// The turn ring, just inside the arrowheads so the two do not overlap.
const RING: f32 = 0.82;
const RING_THICKNESS: f32 = 0.035;

/// How much of the distance to the camera one arm spans, so the rig keeps
/// roughly the same size on screen however far the stage is being watched from.
const SCREEN_SPAN: f32 = 0.13;

/// How far past the node's own bounds the arms must reach. A rig sized only by
/// camera distance would be buried inside a corvette's hull at close range.
const CLEARANCE: f32 = 1.3;

/// The rig: one entity, moved onto the selected node and scaled every frame.
#[derive(Component)]
pub(crate) struct GizmoRig;

/// Which axis a handle is, and which gesture.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GizmoHandle {
    axis: GizmoAxis,
    kind: HandleKind,
}

/// A world axis. The rig is world-aligned - it never takes the node's own
/// rotation - so "X" means the same direction whatever is selected, and a turn
/// that went wrong is undone by dragging the same ring back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GizmoAxis {
    X,
    Y,
    Z,
}

impl GizmoAxis {
    const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    fn unit(self) -> Vec3 {
        match self {
            Self::X => Vec3::X,
            Self::Y => Vec3::Y,
            Self::Z => Vec3::Z,
        }
    }

    /// Red/green/blue for X/Y/Z, in the console's own palette rather than pure
    /// channel colours: the rig has to sit on the same screen as the rail.
    fn colour(self) -> Color {
        match self {
            Self::X => theme::RED,
            Self::Y => theme::PHOSPHOR,
            Self::Z => theme::BLUE,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }
}

/// What dragging a handle does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HandleKind {
    /// An arrow: slide along the axis.
    Move,
    /// A ring: turn about the axis.
    Turn,
}

/// The gesture in progress.
#[derive(Resource, Default, Debug)]
pub(crate) struct GizmoDrag {
    /// The node under the grab.
    node: Option<Entity>,
    /// The handle that was grabbed.
    handle: Option<GizmoHandle>,
    /// The node's pose when the grab landed.
    ///
    /// Every frame's answer is applied to THIS, not to the last frame's, so a
    /// pointer that sweeps out and comes back puts the node back where it
    /// started.
    start: Transform,
    /// The last reading of the handle's parameter: a distance along the axis
    /// for an arrow, an angle about it for a ring.
    last: f32,
    /// How far the handle has been carried since the grab.
    ///
    /// SUMMED over samples rather than taken as "now minus the grab", because
    /// an angle wraps at the half turn: a ring dragged three quarters of the
    /// way round would otherwise read as a quarter turn the other way. One
    /// frame of pointer travel is small enough to have an unambiguous
    /// direction, which is what makes the sum right.
    carried: f32,
}

/// Build the rig once per visit to the editor, hidden until something is
/// selected.
///
/// Once rather than per selection because a rig rebuilt on every click would
/// spend a frame with no mesh - and a handle you are mid-drag on would be a
/// different entity from one frame to the next.
pub(crate) fn setup_gizmo(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let arm = meshes.add(Cylinder::new(ARM_RADIUS, ARM - ARM_START));
    let tip = meshes.add(Cone {
        radius: TIP_RADIUS,
        height: TIP,
    });
    let ring = meshes.add(Torus::new(RING - RING_THICKNESS, RING + RING_THICKNESS));

    commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            Name::new("Editor Gizmo"),
            GizmoRig,
            Transform::default(),
            // Nothing is selected on the way in, and an unparented rig at the
            // origin would be six handles floating in empty space.
            Visibility::Hidden,
        ))
        .with_children(|rig| {
            for axis in GizmoAxis::ALL {
                // Bevy's cylinder, cone and torus are all built about +Y, so one
                // rotation aims a whole handle.
                let aim = Quat::from_rotation_arc(Vec3::Y, axis.unit());
                let paint = materials.add(StandardMaterial {
                    base_color: axis.colour(),
                    // Unlit: a handle is a readout, not a body in the scene, and
                    // one that went dark on its shadow side would be a handle
                    // you could not find.
                    unlit: true,
                    ..default()
                });
                rig.spawn((
                    Name::new(format!("Gizmo Move {}", axis.label())),
                    GizmoHandle {
                        axis,
                        kind: HandleKind::Move,
                    },
                    Pickable::default(),
                    Mesh3d(arm.clone()),
                    MeshMaterial3d(paint.clone()),
                    Transform::from_translation(axis.unit() * ((ARM + ARM_START) * 0.5))
                        .with_rotation(aim),
                ))
                .with_child((
                    Name::new(format!("Gizmo Tip {}", axis.label())),
                    GizmoHandle {
                        axis,
                        kind: HandleKind::Move,
                    },
                    Pickable::default(),
                    Mesh3d(tip.clone()),
                    MeshMaterial3d(paint.clone()),
                    // Local to the arm, which is already aimed: the arm's own
                    // +Y is the axis, and its own centre is the arm's middle.
                    Transform::from_xyz(0.0, (ARM - ARM_START + TIP) * 0.5, 0.0),
                ));
                rig.spawn((
                    Name::new(format!("Gizmo Turn {}", axis.label())),
                    GizmoHandle {
                        axis,
                        kind: HandleKind::Turn,
                    },
                    Pickable::default(),
                    Mesh3d(ring.clone()),
                    MeshMaterial3d(paint),
                    Transform::from_rotation(aim),
                ));
            }
        });
}

/// Put the rig on the selected node, at a size that reads from here.
///
/// Hidden unless there is a node to work on AND the pointer is free to work on
/// it: inside a ship placement owns the pointer, a part armed at the scenario
/// node is about to be put down, and the gallery covers the stage entirely.
/// How big the rig has to be for the node it is on, measured once.
///
/// Measured ONCE, and this is the whole point of the resource: a
/// [`ColliderAabb`] is world-axis-aligned, so a long hull's box grows as the
/// hull turns inside it - a corvette at 45 degrees measures half again as wide
/// as the same corvette square on. Sizing the rig from that every frame made
/// the handles swell and shrink under their own turn ring, which reads as the
/// rig fighting the gesture.
///
/// The reach of a node cannot change while its rig is up: sections are only
/// added from INSIDE a ship, and the rig is hidden in there. So the first
/// measurement stands until the selection moves.
#[derive(Resource, Default, Debug)]
pub(crate) struct GizmoReach {
    /// The node the standing measurement belongs to.
    node: Option<Entity>,
    /// Half the node's diagonal with clearance, or `None` while nothing under
    /// the node has a collider to measure yet.
    reach: Option<f32>,
}

impl GizmoReach {
    /// The reach for `node`, measuring it if this is a node we have not
    /// measured yet.
    ///
    /// A node placed THIS frame has no collider until the physics step runs,
    /// and that measures as `None` rather than as zero - so the rig takes its
    /// size from the camera for a frame and picks up the real one as soon as
    /// there is a real one.
    fn measure(
        &mut self,
        node: Entity,
        q_children: &Query<&Children>,
        q_bounds: &Query<&ColliderAabb, Without<Sensor>>,
    ) -> f32 {
        if self.node != Some(node) {
            self.node = Some(node);
            self.reach = None;
        }
        if self.reach.is_none() {
            self.reach = node_bounds(node, q_children, q_bounds)
                .map(|bounds| bounds.size().length() * 0.5 * CLEARANCE);
        }
        self.reach.unwrap_or_default()
    }
}

pub(crate) fn sync_gizmo(
    context: Res<EditContext>,
    selection: Res<SectionChoice>,
    selected: Res<SelectedNode>,
    gallery: Res<GalleryState>,
    mut reach: ResMut<GizmoReach>,
    q_children: Query<&Children>,
    q_bounds: Query<&ColliderAabb, Without<Sensor>>,
    q_staged: Query<&Transform, (Or<(With<ShipNode>, With<ObjectNode>)>, Without<GizmoRig>)>,
    // The camera's `Transform`, NOT its `GlobalTransform`: propagation runs in
    // `PostUpdate`, so the global pose this system can see is the one from
    // before framing moved the camera. Sizing against that made every frame
    // gesture flash one giant rig - a jump from an arm of 36 to an arm of 4 in
    // consecutive frames, which is what "the handles resize weirdly" looked
    // like. The editor camera is a root entity, so the two are the same value
    // one propagation apart.
    camera: Option<Single<&Transform, (With<EditorCamera>, Without<GizmoRig>)>>,
    rig: Option<Single<(&mut Transform, &mut Visibility), With<GizmoRig>>>,
) {
    let Some(rig) = rig else {
        return;
    };
    let (mut pose, mut visibility) = rig.into_inner();

    let node = shown_on(&context, &selection, &gallery, &selected, &q_staged);
    let (Some(node), Some(camera)) = (node, camera) else {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    let at = q_staged
        .get(node)
        .map(|transform| transform.translation)
        .unwrap_or_default();
    // The node's own extent, so the arms clear a corvette's hull, floored by a
    // fraction of the viewing distance so a lone beacon is not a speck.
    let reach = reach.measure(node, &q_children, &q_bounds);
    let watched = camera.translation.distance(at);
    let arm = reach.max(watched * SCREEN_SPAN);
    let wanted = Transform::from_translation(at).with_scale(Vec3::splat(arm));

    if *pose != wanted {
        *pose = wanted;
    }
    if *visibility != Visibility::Inherited {
        *visibility = Visibility::Inherited;
    }
}

/// The node the rig belongs on, if any.
fn shown_on(
    context: &EditContext,
    selection: &SectionChoice,
    gallery: &GalleryState,
    selected: &SelectedNode,
    q_staged: &Query<&Transform, (Or<(With<ShipNode>, With<ObjectNode>)>, Without<GizmoRig>)>,
) -> Option<Entity> {
    if context.ship().is_some() || *selection != SectionChoice::None || gallery.open {
        return None;
    }
    let node = selected.0?;
    q_staged.get(node).is_ok().then_some(node)
}

/// Grab a handle: remember the node's pose and where on the axis the grab fell.
///
/// The drag never starts when the axis cannot be read - an arm pointing at the
/// camera has no length on screen, and a ring seen edge-on has no angle - so a
/// hopeless gesture leaves the node alone instead of throwing it somewhere.
pub(crate) fn on_gizmo_grab(
    drag: On<Pointer<DragStart>>,
    handles: Query<&GizmoHandle>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<EditorCamera>>>,
    q_staged: Query<&Transform, Or<(With<ShipNode>, With<ObjectNode>)>>,
    selected: Res<SelectedNode>,
    mut state: ResMut<GizmoDrag>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let (Ok(handle), Some(node)) = (handles.get(drag.entity), selected.0) else {
        return;
    };
    let (Ok(pose), Some(camera)) = (q_staged.get(node), camera) else {
        return;
    };
    let (camera, camera_pose) = *camera;
    let Some(ray) = pointer_ray(camera, camera_pose, drag.pointer_location.position) else {
        return;
    };
    let Some(grab) = handle_parameter(*handle, ray, pose.translation) else {
        return;
    };
    state.node = Some(node);
    state.handle = Some(*handle);
    state.start = *pose;
    state.last = grab;
    state.carried = 0.0;
}

/// Apply the gesture: the pose it started from, plus how far the pointer has
/// carried the handle since.
pub(crate) fn on_gizmo_drag(
    drag: On<Pointer<Drag>>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<EditorCamera>>>,
    mut state: ResMut<GizmoDrag>,
    mut q_staged: Query<&mut Transform, Or<(With<ShipNode>, With<ObjectNode>)>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let (Some(node), Some(handle)) = (state.node, state.handle) else {
        return;
    };
    let (Ok(mut pose), Some(camera)) = (q_staged.get_mut(node), camera) else {
        return;
    };
    let (camera, camera_pose) = *camera;
    let Some(ray) = pointer_ray(camera, camera_pose, drag.pointer_location.position) else {
        return;
    };
    // Measured against where the node WAS: the axis line has to stay put for
    // the whole gesture, or a move would chase its own tail.
    let Some(now) = handle_parameter(handle, ray, state.start.translation) else {
        return;
    };
    state.carried += match handle.kind {
        HandleKind::Move => now - state.last,
        HandleKind::Turn => shortest_way(now - state.last),
    };
    state.last = now;
    let wanted = dragged(handle, &state.start, state.carried);
    if *pose != wanted {
        *pose = wanted;
    }
}

/// One step of a turn, brought back into the half turn either side of zero.
///
/// [`f32::atan2`] answers in that range, so two readings across the seam differ
/// by nearly a whole turn when the pointer barely moved. The step the pointer
/// actually made is the SHORT way round.
fn shortest_way(step: f32) -> f32 {
    (step + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// Let go - of the button that grabbed, for the reason
/// [`crate::placement::on_stage_drag_end`] gives.
pub(crate) fn on_gizmo_release(drag: On<Pointer<DragEnd>>, mut state: ResMut<GizmoDrag>) {
    if drag.button != PointerButton::Primary {
        return;
    }
    if state.node.is_some() {
        *state = GizmoDrag::default();
    }
}

/// `start` carried `delta` along or about the handle's axis.
fn dragged(handle: GizmoHandle, start: &Transform, delta: f32) -> Transform {
    let axis = handle.axis.unit();
    match handle.kind {
        HandleKind::Move => Transform {
            translation: start.translation + axis * delta,
            ..*start
        },
        // The turn is applied in WORLD space - on the left - so the ring the
        // pointer is on is the axis the node turns about, whichever way the
        // node is already facing.
        HandleKind::Turn => Transform {
            rotation: Quat::from_axis_angle(axis, delta) * start.rotation,
            ..*start
        },
    }
}

/// Where the pointer is on the handle's axis: a distance along it for an arrow,
/// an angle about it for a ring.
fn handle_parameter(handle: GizmoHandle, ray: Ray3d, origin: Vec3) -> Option<f32> {
    let axis = handle.axis.unit();
    match handle.kind {
        HandleKind::Move => axis_parameter(ray, origin, axis),
        HandleKind::Turn => plane_angle(ray, origin, axis),
    }
}

/// The pointer's ray into the world, or `None` when the viewport position is
/// off camera.
fn pointer_ray(camera: &Camera, camera_pose: &GlobalTransform, viewport: Vec2) -> Option<Ray3d> {
    camera.viewport_to_world(camera_pose, viewport).ok()
}

/// How far along the axis through `origin` the ray comes closest to it.
///
/// The two lines almost never meet, so this is the point on the AXIS nearest
/// the pointer's line - the standard closest-approach solution. It has no
/// answer when the two are parallel, which is exactly the case where the arm is
/// pointing at the camera and a pixel of pointer travel would be a mile of
/// node travel.
fn axis_parameter(ray: Ray3d, origin: Vec3, axis: Vec3) -> Option<f32> {
    let look = ray.direction.as_vec3();
    let between = origin - ray.origin;
    let along = axis.dot(look);
    let spread = 1.0 - along * along;
    if spread < 1e-3 {
        return None;
    }
    Some((along * look.dot(between) - axis.dot(between)) / spread)
}

/// The angle of the pointer's hit on the plane through `origin` with normal
/// `axis`, measured from that plane's own reference direction.
///
/// Only the DIFFERENCE between two of these is ever used, so which direction
/// counts as zero does not matter - only that it is the same one at the grab
/// and at every drag. [`Vec3::any_orthonormal_vector`] is a pure function of
/// the axis, and the axis does not move during a gesture.
fn plane_angle(ray: Ray3d, origin: Vec3, axis: Vec3) -> Option<f32> {
    let look = ray.direction.as_vec3();
    let facing = axis.dot(look);
    if facing.abs() < 1e-3 {
        return None;
    }
    let travel = axis.dot(origin - ray.origin) / facing;
    if travel <= 0.0 {
        // The plane is behind the camera.
        return None;
    }
    let hit = ray.origin + look * travel - origin;
    let zero = axis.any_orthonormal_vector();
    Some(hit.dot(axis.cross(zero)).atan2(hit.dot(zero)))
}

/// Register the rig, its gestures, and the picking backend that reaches them.
pub(crate) fn register(app: &mut App) {
    // A backend is no use without the core that feeds it rays, and a headless
    // rig has neither. Same shape as the editor's other render-side gates.
    if app.is_plugin_added::<PickingPlugin>() {
        // `require_markers` goes in BEFORE the plugin, which only
        // `init_resource`s the settings: inserted after, the plugin's default
        // would win and every mesh on the stage would answer the pointer twice.
        app.insert_resource(MeshPickingSettings {
            require_markers: true,
            ..default()
        });
        app.add_plugins(MeshPickingPlugin);
    }
    app.init_resource::<GizmoDrag>();
    app.init_resource::<GizmoReach>();
    app.add_observer(on_gizmo_grab);
    app.add_observer(on_gizmo_drag);
    app.add_observer(on_gizmo_release);
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        (
            setup_gizmo
                .run_if(resource_exists::<Assets<Mesh>>)
                .run_if(resource_exists::<Assets<StandardMaterial>>),
            // A gesture cannot survive its rig being rebuilt.
            |mut drag: ResMut<GizmoDrag>| *drag = GizmoDrag::default(),
            |mut reach: ResMut<GizmoReach>| *reach = GizmoReach::default(),
        ),
    );
}

#[cfg(test)]
mod tests;
