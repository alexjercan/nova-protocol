//! The handles that move, turn and size the selection.
//!
//! Dragging a node's BODY slides it on the ground plane
//! ([`crate::placement::on_stage_drag`]): one gesture, two axes, no way to say
//! "up" and no way to say "turn". The gizmo is the rest of that vocabulary, and
//! [`crate::tool::StageTool`] picks which part of it is up: arrows and plane
//! squares, rings, or size handles.
//!
//! A multi-selection shares one rig at its centre. A move carries every node,
//! and a turn swings them about that centre.
//!
//! It is a real rig of meshes rather than immediate-mode [`Gizmos`] lines
//! because the handles have to be POINTED AT, and only geometry can be picked.
//! That is also why the mesh picking backend is registered here and set to
//! `require_markers`: the stage already answers the pointer through avian's
//! colliders, and a second backend that picked every mesh in the scene would
//! double every hit on the ship. With markers required, mesh picking sees the
//! handles and nothing else.
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
use nova_events::units::prelude::*;
use nova_gameplay::prelude::subtree_bounding_sphere;
use nova_ship::prelude::GalleryState;

use crate::{
    asset::SocketNode,
    config::{EditorCamera, SelectedNode},
    history::commit_edit,
    node::{EditContext, NodeView, ObjectBodyStale, ObjectNode, ShipNode},
    palette,
    placement::alt_held,
    tool::{
        set_size, size_of, snap_active, snapped, GizmoSpaceType, StageTool, StageToolType,
        MIN_SIZE, MOVE_SNAP, SIZE_SNAP, TURN_SNAP_DEGREES,
    },
    ExampleStates,
};

/// What the rig can move.
type GizmoStaged = Or<(With<ShipNode>, With<ObjectNode>, With<SocketNode>)>;

/// The grid a socket snaps to: half a build cell, where a cell's faces and
/// centre all fall.
const SOCKET_SNAP: Meters = Meters(5.0);

/// Where the arms END, in rig-local units. Everything else is a fraction of it,
/// so the whole rig scales by scaling the root.
const ARM: f32 = 1.0;
/// Where the arms BEGIN. The centre is left hollow so the node itself can still
/// be pointed at: the body under the rig is draggable and clickable, and an
/// arm through the middle of it would take every click aimed at the middle.
const ARM_START: f32 = 0.4;
/// Arm thickness. A line, not a pipe: the arms are a readout of which way the
/// axis goes, and at the old thickness they were the loudest thing on a stage
/// that also has a grid to read.
const ARM_RADIUS: f32 = 0.012;
/// The arrowhead on the end of an arm.
const TIP: f32 = 0.26;
const TIP_RADIUS: f32 = 0.1;
/// The turn ring, just inside the arrowheads so the two do not overlap.
const RING: f32 = 0.82;
const RING_THICKNESS: f32 = 0.012;
/// The size handle's cube, on the end of an arm.
const STRETCH_TIP: f32 = 0.11;
/// How far along each of its two axes a plane square sits, and half its side.
/// Far enough out that a square clears the body the arms were sized to clear.
const PLANE_AT: f32 = 0.65;
const PLANE_HALF: f32 = 0.1;
/// How far along each axis the all-axes size handle sits, for the same reason.
const UNIFORM_AT: f32 = 0.62;

/// What the pointer actually hits.
///
/// Handles are picked off the DRAWN mesh, so thinning the arm would thin the
/// grab target with it and leave a rig that has to be aimed at. The grab
/// bodies keep the old thickness and paint nothing: one shape to see, a
/// fatter one to hit, in the same place.
const ARM_GRAB_RADIUS: f32 = 0.045;
const RING_GRAB_THICKNESS: f32 = 0.045;

/// How much of the distance to the camera one arm spans, so the rig keeps
/// roughly the same size on screen however far the stage is being watched from.
const SCREEN_SPAN: f32 = 0.13;

/// How far past the node's own bounds the arms must reach. A rig sized only by
/// camera distance would be buried inside a corvette's hull at close range.
const CLEARANCE: f32 = 1.3;

/// How square-on a ring has to be before it is worth drawing, as the cosine
/// between its axis and the line of sight.
///
/// A ring the camera looks ALONG is a ring nobody can turn: the angle a drag
/// reads is measured in the ring's own plane, and from there that plane is a
/// line - a pixel of pointer travel is most of a revolution. Worse, its rim
/// then lies across the middle of the node, which is where the body drag
/// presses. The stock framing angle stands the camera in the X ring's plane,
/// so this is the DEFAULT view, not a corner of one.
const SQUARE_ON: f32 = 0.2;

/// The rig: one entity, moved onto the selected node and scaled every frame.
#[derive(Component)]
pub(crate) struct GizmoRig;

/// Which axis a handle is, and which gesture.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GizmoHandle {
    axis: GizmoAxis,
    kind: HandleKind,
}

/// One of the rig's three axes: the world's, or the primary node's own in
/// [`GizmoSpaceType::Local`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GizmoAxis {
    X,
    Y,
    Z,
}

impl GizmoAxis {
    pub(crate) const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    pub(crate) fn unit(self) -> Vec3 {
        match self {
            Self::X => Vec3::X,
            Self::Y => Vec3::Y,
            Self::Z => Vec3::Z,
        }
    }

    /// Red/green/blue for X/Y/Z, in the console's own palette rather than pure
    /// channel colours: the rig has to sit on the same screen as the rail.
    ///
    /// Read by the Inspector too: a number and the handle that drags it are the
    /// same axis, so they are the same colour.
    pub(crate) fn colour(self) -> Color {
        match self {
            Self::X => palette::NO,
            Self::Y => palette::GO,
            Self::Z => palette::TRIGGER,
        }
    }

    pub(crate) fn label(self) -> &'static str {
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
    /// A square: slide across the plane the axis is the normal of.
    Plane,
    /// A ring: turn about the axis.
    Turn,
    /// A stick with a cube on it: grow along the axis.
    Stretch,
    /// The cube off the corner: grow whatever the axis.
    Uniform,
}

impl HandleKind {
    /// The tool this handle belongs to.
    fn tool(self) -> StageToolType {
        match self {
            Self::Move | Self::Plane => StageToolType::Move,
            Self::Turn => StageToolType::Turn,
            Self::Stretch | Self::Uniform => StageToolType::Scale,
        }
    }

    /// The undo step a gesture on this handle makes.
    fn step(self) -> &'static str {
        match self.tool() {
            StageToolType::Turn => "Turn",
            StageToolType::Scale => "Scale",
            StageToolType::Move | StageToolType::Select => "Move",
        }
    }
}

impl GizmoHandle {
    /// The handle's axis in the world, for a rig turned to `basis`.
    fn axis_in(self, basis: Quat) -> Vec3 {
        let local = match self.kind {
            HandleKind::Uniform => Vec3::ONE.normalize(),
            _ => self.axis.unit(),
        };
        basis * local
    }
}

/// The gesture in progress.
#[derive(Resource, Default, Debug)]
pub(crate) struct GizmoDrag {
    /// The nodes under the grab, each with its pose when the grab landed.
    ///
    /// Every frame's answer is applied to THESE, not to the last frame's, so a
    /// pointer that sweeps out and comes back puts the nodes back where they
    /// started.
    members: Vec<(Entity, Transform)>,
    /// The handle that was grabbed. `None` when nothing is held.
    handle: Option<GizmoHandle>,
    /// The rig's centre at the grab. A turn swings about it.
    centre: Vec3,
    /// The rig's axes at the grab, so a local turn does not move its own axis.
    basis: Quat,
    /// The rig's arm at the grab: a size handle carried this far doubles.
    span: f32,
    /// Where a plane square was grabbed, on its plane.
    grab_point: Vec3,
    /// The size field's value at the grab, on a size handle.
    size: Meters,
    /// The last reading of the handle's parameter: a distance along the axis
    /// for an arrow, an angle about it for a ring.
    last: f32,
    /// The grid a snapped move steps along, in engine units.
    step: f32,
    /// Whether the gesture snaps with or without the snap key: a socket off the
    /// half-cell grid mates nothing a builder can line up.
    on_grid: bool,
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
    let square = meshes.add(Cuboid::new(PLANE_HALF * 2.0, PLANE_HALF * 2.0, 0.01));
    let cube = meshes.add(Cuboid::from_length(STRETCH_TIP));
    let arm_grab = meshes.add(Cylinder::new(ARM_GRAB_RADIUS, ARM - ARM_START));
    let ring_grab = meshes.add(Torus::new(
        RING - RING_GRAB_THICKNESS,
        RING + RING_GRAB_THICKNESS,
    ));
    // Transparent rather than `Visibility::Hidden`: mesh picking only casts at
    // what a camera can see, so a hidden body is a body nothing can grab.
    let grab = materials.add(StandardMaterial {
        base_color: Color::NONE,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            Name::new("Editor Gizmo"),
            GizmoRig,
            Transform::default(),
            // Nothing is selected on the way in, and an unparented rig at the
            // origin would be handles floating in empty space.
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
                ))
                .with_child((
                    Name::new(format!("Gizmo Move Grab {}", axis.label())),
                    GizmoHandle {
                        axis,
                        kind: HandleKind::Move,
                    },
                    Pickable::default(),
                    Mesh3d(arm_grab.clone()),
                    MeshMaterial3d(grab.clone()),
                    Transform::default(),
                ));
                rig.spawn((
                    Name::new(format!("Gizmo Turn {}", axis.label())),
                    GizmoHandle {
                        axis,
                        kind: HandleKind::Turn,
                    },
                    Pickable::default(),
                    Mesh3d(ring.clone()),
                    MeshMaterial3d(paint.clone()),
                    Transform::from_rotation(aim),
                ))
                .with_child((
                    Name::new(format!("Gizmo Turn Grab {}", axis.label())),
                    GizmoHandle {
                        axis,
                        kind: HandleKind::Turn,
                    },
                    Pickable::default(),
                    Mesh3d(ring_grab.clone()),
                    MeshMaterial3d(grab.clone()),
                    Transform::default(),
                ));
                rig.spawn((
                    Name::new(format!("Gizmo Scale {}", axis.label())),
                    GizmoHandle {
                        axis,
                        kind: HandleKind::Stretch,
                    },
                    Pickable::default(),
                    Mesh3d(arm.clone()),
                    MeshMaterial3d(paint.clone()),
                    Transform::from_translation(axis.unit() * ((ARM + ARM_START) * 0.5))
                        .with_rotation(aim),
                ))
                .with_child((
                    Name::new(format!("Gizmo Scale Tip {}", axis.label())),
                    GizmoHandle {
                        axis,
                        kind: HandleKind::Stretch,
                    },
                    Pickable::default(),
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(paint.clone()),
                    Transform::from_xyz(0.0, (ARM - ARM_START) * 0.5, 0.0),
                ))
                .with_child((
                    Name::new(format!("Gizmo Scale Grab {}", axis.label())),
                    GizmoHandle {
                        axis,
                        kind: HandleKind::Stretch,
                    },
                    Pickable::default(),
                    Mesh3d(arm_grab.clone()),
                    MeshMaterial3d(grab.clone()),
                    Transform::default(),
                ));
                // The square lies across the two OTHER axes, and a cuboid built
                // flat on Z takes its normal from the same aim the arm does.
                let flat = Quat::from_rotation_arc(Vec3::Z, axis.unit());
                rig.spawn((
                    Name::new(format!("Gizmo Plane {}", axis.label())),
                    GizmoHandle {
                        axis,
                        kind: HandleKind::Plane,
                    },
                    Pickable::default(),
                    Mesh3d(square.clone()),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: axis.colour().with_alpha(0.55),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_translation((Vec3::ONE - axis.unit()) * PLANE_AT)
                        .with_rotation(flat),
                ));
            }
            rig.spawn((
                Name::new("Gizmo Scale All"),
                GizmoHandle {
                    axis: GizmoAxis::X,
                    kind: HandleKind::Uniform,
                },
                Pickable::default(),
                Mesh3d(cube.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: palette::GO,
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(Vec3::splat(UNIFORM_AT)),
            ));
        });
}

/// How big the rig has to be for the node it is on, measured once.
///
/// Measured ONCE, and this is the whole point of the resource: a
/// [`ColliderAabb`] is world-axis-aligned, so a long hull's box grows as the
/// hull turns inside it - a corvette at 45 degrees measures half again as wide
/// as the same corvette square on. Sizing the rig from that every frame made
/// the handles swell and shrink under their own turn ring, which reads as the
/// rig fighting the gesture.
///
/// Sections are only added from INSIDE a ship and the rig is hidden in there,
/// so the measurement stands until the selection moves - or until the node's
/// body is REDRAWN, which is the one thing that resizes it with the rig still
/// up. A rock's `radius`, a crate's `size` and a beacon's `radius` all rebuild
/// the mesh and leave `Transform.scale` at `Vec3::ONE`, so a key on the scale
/// answered nothing: there is no Scale field, and `inspect.rs` says there will
/// not be one.
#[derive(Resource, Default, Debug)]
pub(crate) struct GizmoReach {
    /// The node the standing measurement belongs to.
    node: Option<Entity>,
    /// Half the node's diagonal with clearance, or `None` while nothing under
    /// the node has a collider to measure yet.
    reach: Option<f32>,
    /// Frames of re-measuring still owed to an announced resize.
    settling: u8,
}

impl GizmoReach {
    /// How long a re-measure keeps going after a resize is announced.
    ///
    /// The old body is despawned and the new one is built over the frames that
    /// FOLLOW the announcement, and its collider arrives with the physics step
    /// after that - so one measurement taken on the frame of the message can
    /// still find the size that is on its way out.
    const SETTLE_FRAMES: u8 = 3;

    /// `node`'s body is being redrawn, so whatever was measured is about to
    /// stop being true.
    fn resized(&mut self, node: Entity) {
        if self.node == Some(node) {
            self.settling = Self::SETTLE_FRAMES;
        }
    }

    /// The reach for `node`, measuring it if it is not a node we have a
    /// standing measurement for.
    ///
    /// A node placed THIS frame has no collider until the physics step runs,
    /// and that measures as `None` rather than as zero - so the rig takes its
    /// size from the camera for a frame and picks up the real one as soon as
    /// there is a real one. A re-measure that finds nothing keeps the last
    /// real answer rather than collapsing the rig mid-rebuild.
    fn measure(
        &mut self,
        node: Entity,
        q_children: &Query<&Children>,
        q_bounds: &Query<&ColliderAabb, Without<Sensor>>,
    ) -> f32 {
        if self.node != Some(node) {
            self.node = Some(node);
            self.reach = None;
            self.settling = 0;
        }
        if self.reach.is_none() || self.settling > 0 {
            self.settling = self.settling.saturating_sub(1);
            self.reach = subtree_bounding_sphere(node, q_children, q_bounds)
                .map(|(_, radius)| radius * CLEARANCE)
                .or(self.reach);
        }
        self.reach.unwrap_or_default()
    }
}

/// What the rig stands on: the nodes it moves, their centre, and its axes.
#[derive(Debug, PartialEq)]
struct GizmoTarget {
    nodes: Vec<(Entity, Transform)>,
    centre: Vec3,
    basis: Quat,
}

/// Put the rig on the selection, at a size that reads from here.
///
/// Hidden unless there is a node to work on and a tool to work on it with.
#[expect(
    clippy::too_many_arguments,
    reason = "the rig reads the selection, the tool, the camera and every handle"
)]
pub(crate) fn sync_gizmo(
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
    gallery: Res<GalleryState>,
    tool: Res<StageTool>,
    mut reach: ResMut<GizmoReach>,
    q_children: Query<&Children>,
    q_bounds: Query<&ColliderAabb, Without<Sensor>>,
    q_staged: Query<&Transform, (GizmoStaged, Without<GizmoRig>)>,
    q_objects: Query<&ObjectNode>,
    // The camera's `Transform`, NOT its `GlobalTransform`: propagation runs in
    // `PostUpdate`, so the global pose this system can see is the one from
    // before framing moved the camera. Sizing against that made every frame
    // gesture flash one giant rig - a jump from an arm of 36 to an arm of 4 in
    // consecutive frames, which is what "the handles resize weirdly" looked
    // like. The editor camera is a root entity, so the two are the same value
    // one propagation apart.
    camera: Option<Single<&Transform, (With<EditorCamera>, Without<GizmoRig>)>>,
    rig: Option<Single<(&mut Transform, &mut Visibility), With<GizmoRig>>>,
    mut q_handles: Query<(&GizmoHandle, &mut Visibility), Without<GizmoRig>>,
    // The one thing that resizes a node with the rig up: the same announcement
    // the body redraw listens to (see `crate::node::drop_edited_views`).
    mut resized: MessageReader<ObjectBodyStale>,
) {
    for ObjectBodyStale(node) in resized.read() {
        reach.resized(*node);
    }
    let Some(rig) = rig else {
        return;
    };
    let (mut pose, mut visibility) = rig.into_inner();

    let target = shown_on(&context, &gallery, &selected, &tool, |node| {
        let pose = q_staged.get(node).ok()?;
        let object = q_objects.get(node).ok();
        Some((*pose, object.and_then(|object| size_of(&object.kind))))
    });
    let (Some(target), Some(camera)) = (target, camera) else {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    // The node's own extent, so the arms clear a corvette's hull, floored by a
    // fraction of the viewing distance so a lone beacon is not a speck. A group
    // has no one extent: its centre is usually open space.
    let watched = camera.translation.distance(target.centre);
    let arm = match target.nodes.as_slice() {
        [(node, _)] => reach
            .measure(*node, &q_children, &q_bounds)
            .max(watched * SCREEN_SPAN),
        _ => watched * SCREEN_SPAN,
    };
    let wanted = Transform::from_translation(target.centre)
        .with_rotation(target.basis)
        .with_scale(Vec3::splat(arm));

    // Up: the tool's handles, less the rings the camera is looking along.
    let look = (camera.translation - target.centre).normalize_or_zero();
    for (handle, mut visibility) in &mut q_handles {
        let square_on = handle.kind != HandleKind::Turn
            || handle.axis_in(target.basis).dot(look).abs() >= SQUARE_ON;
        let shown = if square_on && handle.kind.tool() == tool.tool {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != shown {
            *visibility = shown;
        }
    }

    if *pose != wanted {
        *pose = wanted;
    }
    if *visibility != Visibility::Inherited {
        *visibility = Visibility::Inherited;
    }
}

/// What the rig belongs on, if anything. `read` answers a node's pose and its
/// size field, and `None` for a node the rig cannot stand on.
///
/// Three suppressions, all deliberate. Inside a ship a part's pose is decided by
/// the socket it mates to, so an axis drag would break the mate the moment it
/// moved - `on_stage_drag_start` says so to anyone who tries. With the gallery
/// up the rig is behind a full-screen overlay anyway. And Scale needs exactly
/// one node with a size field: `crate::tool` says why to anyone who tries.
fn shown_on(
    context: &EditContext,
    gallery: &GalleryState,
    selected: &SelectedNode,
    tool: &StageTool,
    read: impl Fn(Entity) -> Option<(Transform, Option<Meters>)>,
) -> Option<GizmoTarget> {
    if context.ship().is_some() || gallery.open || tool.tool == StageToolType::Select {
        return None;
    }
    let read: Vec<_> = selected
        .iter()
        .filter_map(|node| read(node).map(|(pose, size)| (node, pose, size)))
        .collect();
    let (_, primary, size) = read.last()?;
    if tool.tool == StageToolType::Scale && (read.len() != 1 || size.is_none()) {
        return None;
    }
    let basis = match tool.space {
        GizmoSpaceType::World => Quat::IDENTITY,
        GizmoSpaceType::Local => primary.rotation,
    };
    let nodes: Vec<_> = read.iter().map(|(node, pose, _)| (*node, *pose)).collect();
    let sum: Vec3 = nodes.iter().map(|(_, pose)| pose.translation).sum();
    Some(GizmoTarget {
        centre: sum / nodes.len() as f32,
        nodes,
        basis,
    })
}

/// Grab a handle: remember the poses and where on the axis the grab fell.
///
/// The drag never starts when the axis cannot be read - an arm pointing at the
/// camera has no length on screen, and a ring seen edge-on has no angle - so a
/// hopeless gesture leaves the nodes alone instead of throwing them somewhere.
#[expect(
    clippy::too_many_arguments,
    reason = "a grab reads the rig, the camera and the whole selection once"
)]
pub(crate) fn on_gizmo_grab(
    drag: On<PointerDragStart>,
    handles: Query<&GizmoHandle>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<EditorCamera>>>,
    rig: Option<
        Single<
            &Transform,
            (
                With<GizmoRig>,
                Without<ShipNode>,
                Without<ObjectNode>,
                Without<SocketNode>,
            ),
        >,
    >,
    q_staged: Query<&Transform, GizmoStaged>,
    (q_objects, q_sockets): (Query<&ObjectNode>, Query<(), With<SocketNode>>),
    (context, gallery, tool): (Res<EditContext>, Res<GalleryState>, Res<StageTool>),
    selected: Res<SelectedNode>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut state: ResMut<GizmoDrag>,
) {
    if drag.button != PointerButton::Primary || alt_held(keys.as_deref()) {
        return;
    }
    let Ok(handle) = handles.get(drag.entity) else {
        return;
    };
    let target = shown_on(&context, &gallery, &selected, &tool, |node| {
        let pose = q_staged.get(node).ok()?;
        let object = q_objects.get(node).ok();
        Some((*pose, object.and_then(|object| size_of(&object.kind))))
    });
    let (Some(target), Some(camera)) = (target, camera) else {
        return;
    };
    let (camera, camera_pose) = *camera;
    let Some(ray) = pointer_ray(camera, camera_pose, drag.pointer.position) else {
        return;
    };
    let axis = handle.axis_in(target.basis);
    if handle.kind == HandleKind::Plane {
        let Some(hit) = plane_hit(ray, target.centre, axis) else {
            return;
        };
        state.grab_point = hit;
    } else {
        let Some(grab) = handle_parameter(*handle, ray, target.centre, axis) else {
            return;
        };
        state.last = grab;
    }
    state.handle = Some(*handle);
    state.centre = target.centre;
    state.basis = target.basis;
    state.span = rig.map_or(ARM, |rig| rig.scale.x);
    state.size = target
        .nodes
        .last()
        .and_then(|(node, _)| q_objects.get(*node).ok())
        .and_then(|object| size_of(&object.kind))
        .unwrap_or_default();
    // A section is a cell or so across: the stage's grid would step a socket
    // off the part.
    state.on_grid = target
        .nodes
        .iter()
        .any(|(node, _)| q_sockets.contains(*node));
    state.step = if state.on_grid {
        SOCKET_SNAP.to_engine()
    } else {
        MOVE_SNAP.to_engine()
    };
    state.members = target.nodes;
    state.carried = 0.0;
}

/// Apply the gesture: the poses it started from, plus how far the pointer has
/// carried the handle since.
pub(crate) fn on_gizmo_drag(
    drag: On<PointerDrag>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<EditorCamera>>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    tool: Res<StageTool>,
    mut state: ResMut<GizmoDrag>,
    mut q_staged: Query<&mut Transform, GizmoStaged>,
    mut q_objects: Query<&mut ObjectNode>,
    q_children: Query<&Children>,
    mut q_views: Query<
        &mut Transform,
        (
            With<NodeView>,
            Without<ShipNode>,
            Without<ObjectNode>,
            Without<SocketNode>,
        ),
    >,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let (Some(handle), Some(camera)) = (state.handle, camera) else {
        return;
    };
    let (camera, camera_pose) = *camera;
    let Some(ray) = pointer_ray(camera, camera_pose, drag.pointer.position) else {
        return;
    };
    let snap = state.on_grid || snap_active(&tool, keys.as_deref());
    // Measured against where the rig WAS: the axis line has to stay put for the
    // whole gesture, or a move would chase its own tail.
    let (centre, axis) = (state.centre, handle.axis_in(state.basis));
    let travel = if handle.kind == HandleKind::Plane {
        let Some(hit) = plane_hit(ray, centre, axis) else {
            return;
        };
        let across = hit - state.grab_point;
        Travel::Slide(if snap {
            snapped_across(centre, across, state.basis, state.step)
        } else {
            across
        })
    } else {
        let Some(now) = handle_parameter(handle, ray, centre, axis) else {
            return;
        };
        state.carried += match handle.kind {
            HandleKind::Turn => shortest_way(now - state.last),
            _ => now - state.last,
        };
        state.last = now;
        match handle.kind {
            HandleKind::Turn if snap => {
                Travel::Turn(axis, snapped(state.carried, TURN_SNAP_DEGREES.to_radians()))
            }
            HandleKind::Turn => Travel::Turn(axis, state.carried),
            HandleKind::Move if snap => {
                let step = state.step;
                let along = centre.dot(axis);
                Travel::Slide(axis * (snapped(along + state.carried, step) - along))
            }
            HandleKind::Move => Travel::Slide(axis * state.carried),
            HandleKind::Plane | HandleKind::Stretch | HandleKind::Uniform => {
                Travel::Size(stretched(state.size, state.carried, state.span, snap))
            }
        }
    };
    match travel {
        Travel::Size(size) => {
            let Some((node, _)) = state.members.last() else {
                return;
            };
            let Ok(mut object) = q_objects.get_mut(*node) else {
                return;
            };
            if size_of(&object.kind) != Some(size) {
                set_size(&mut object.kind, size);
            }
            // The body rebuilds once, on release: a planet is too dear to mesh
            // every frame of a drag. Until then its view stands in, stretched.
            let stand_in = Vec3::splat(size.get() / state.size.get().max(f32::EPSILON));
            for child in q_children.get(*node).into_iter().flatten() {
                if let Ok(mut view) = q_views.get_mut(*child) {
                    view.scale = stand_in;
                }
            }
        }
        _ => {
            for (node, start) in &state.members {
                let Ok(mut pose) = q_staged.get_mut(*node) else {
                    continue;
                };
                let wanted = applied(travel, start, centre);
                if *pose != wanted {
                    *pose = wanted;
                }
            }
        }
    }
}

/// `across`, with each of the plane's axes snapped to the grid. An axis of the
/// rig that the square does not slide along reads zero and stays there.
fn snapped_across(centre: Vec3, across: Vec3, basis: Quat, step: f32) -> Vec3 {
    GizmoAxis::ALL
        .into_iter()
        .map(|axis| basis * axis.unit())
        .map(|axis| {
            let carried = across.dot(axis);
            if carried.abs() < 1e-4 {
                return Vec3::ZERO;
            }
            let along = centre.dot(axis);
            axis * (snapped(along + carried, step) - along)
        })
        .sum()
}

/// A size carried `distance` along a size handle of the given `span`: one span
/// out is double, and one span in is the floor.
fn stretched(size: Meters, distance: f32, span: f32, snap: bool) -> Meters {
    let wanted = size.get() * (1.0 + distance / span.max(f32::EPSILON));
    let wanted = if snap {
        snapped(wanted, SIZE_SNAP.get())
    } else {
        wanted
    };
    Meters(wanted.max(MIN_SIZE.get()))
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
pub(crate) fn on_gizmo_release(
    drag: On<PointerDragEnd>,
    mut commands: Commands,
    mut state: ResMut<GizmoDrag>,
    mut stale: MessageWriter<ObjectBodyStale>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let Some(handle) = state.handle else {
        return;
    };
    if handle.kind.tool() == StageToolType::Scale {
        for (node, _) in &state.members {
            stale.write(ObjectBodyStale(*node));
        }
    }
    commit_edit(&mut commands, handle.kind.step());
    *state = GizmoDrag::default();
}

/// What a gesture has done so far.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Travel {
    /// Carried this far, in the world.
    Slide(Vec3),
    /// Turned about this axis through the rig's centre, by this angle.
    Turn(Vec3, f32),
    /// The size field now reads this.
    Size(Meters),
}

/// `start` carried by `travel`, for a rig standing at `centre`.
fn applied(travel: Travel, start: &Transform, centre: Vec3) -> Transform {
    match travel {
        Travel::Slide(by) => Transform {
            translation: start.translation + by,
            ..*start
        },
        // The turn is applied in WORLD space - on the left - so the ring the
        // pointer is on is the axis the node turns about, whichever way the
        // node is already facing.
        Travel::Turn(axis, angle) => {
            let turn = Quat::from_axis_angle(axis, angle);
            Transform {
                translation: centre + turn * (start.translation - centre),
                rotation: turn * start.rotation,
                ..*start
            }
        }
        Travel::Size(_) => *start,
    }
}

/// Where the pointer is on the handle's `axis`: a distance along it for an
/// arrow or a size handle, an angle about it for a ring.
fn handle_parameter(handle: GizmoHandle, ray: Ray3d, origin: Vec3, axis: Vec3) -> Option<f32> {
    match handle.kind {
        HandleKind::Turn => plane_angle(ray, origin, axis),
        _ => axis_parameter(ray, origin, axis),
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
    let hit = plane_hit(ray, origin, axis)? - origin;
    let zero = axis.any_orthonormal_vector();
    Some(hit.dot(axis.cross(zero)).atan2(hit.dot(zero)))
}

/// Where the ray crosses the plane through `origin` with normal `axis`, or
/// `None` seen edge-on or with the plane behind the camera.
fn plane_hit(ray: Ray3d, origin: Vec3, axis: Vec3) -> Option<Vec3> {
    let look = ray.direction.as_vec3();
    let facing = axis.dot(look);
    if facing.abs() < 1e-3 {
        return None;
    }
    let travel = axis.dot(origin - ray.origin) / facing;
    (travel > 0.0).then(|| ray.origin + look * travel)
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
    app.init_resource::<StageTool>();
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
