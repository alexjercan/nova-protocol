//! The gallery's 3D half: parking the editor camera on an empty stage far from
//! the build area, spawning one preview per tile, and keeping each preview
//! centred in the UI cell that owns it.
//!
//! The previews follow the UI rather than the other way round: layout owns the
//! grid, and a tile is placed by unprojecting its cell's centre. Change this
//! module when the stage framing or the tile fitting changes.

use bevy::{
    camera::primitives::Aabb,
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    ui::{ComputedNode, UiGlobalTransform},
};
use nova_ship::prelude::*;

use crate::{
    gallery::{catalog, GalleryState},
    preview::{insert_preview_section, PreviewRole},
    ExampleStates,
};

/// Where the gallery stage sits. Far above the build area so a tile's collider
/// can never be picked as part of the ship, and so nothing the player built
/// wanders into frame.
const STAGE_ORIGIN: Vec3 = Vec3::new(0.0, 2_000.0, 0.0);

/// Ray distance from the parked camera to every tile. Equal distances mean
/// equal apparent size, so a part in a corner cell reads as large as the same
/// part in the middle.
const STAGE_DISTANCE: f32 = 8.0;

/// Fraction of a grid cell's smaller side the largest part axis fills.
const TILE_FILL: f32 = 0.7;

/// The same fraction for the focus stage, before the builder zooms. Lower than
/// a tile's: the stage is most of the screen, and a part that fills it reads as
/// a perspective smear rather than as a shape.
const FOCUS_FILL: f32 = 0.55;

/// Zoom limits on that fraction. The far end still leaves a small part legible;
/// the near end stops before the part outgrows the stage it is framed in.
const FOCUS_FILL_MIN: f32 = 0.2;
/// See [`FOCUS_FILL_MIN`].
const FOCUS_FILL_MAX: f32 = 1.6;

/// Zoom factor per wheel notch.
const FOCUS_ZOOM_STEP: f32 = 1.12;

/// Radians of turn per pixel of drag.
const FOCUS_DRAG_RATE: f32 = 0.006;

/// How far the orbit tips before it stops, so a part is never viewed from
/// exactly its own pole (where yaw stops meaning anything).
const FOCUS_PITCH_LIMIT: f32 = 1.2;

/// Seconds of no drag or zoom before the turntable takes back over. Long enough
/// not to fight a builder who paused mid-inspection, short enough that the view
/// does not sit frozen once they have moved on.
const FOCUS_IDLE_RESUME: f32 = 2.0;

/// Three-quarter presentation, lifted from the parts viewer: yaw the nose
/// (-Z) mostly toward the camera.
const PRESENT_YAW: f32 = 2.5;

/// Turntable rate of the focused preview, in radians per second.
const SPIN_RATE: f32 = 0.5;

/// How the focus view is framed right now: how much of the stage the part
/// fills, where the builder turned it to, and how long since they last touched
/// it.
///
/// The turntable and the drag share one yaw on purpose - the spin resumes from
/// wherever the drag left the part rather than snapping back to a canned pose.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct FocusView {
    /// Fraction of the stage's smaller side the part fills.
    fill: f32,
    /// Turntable angle, driven by the drag and then by the spin.
    yaw: f32,
    /// Orbit elevation, clamped to [`FOCUS_PITCH_LIMIT`].
    pitch: f32,
    /// Seconds since the last drag or zoom.
    idle: f32,
}

impl Default for FocusView {
    fn default() -> Self {
        Self {
            fill: FOCUS_FILL,
            yaw: PRESENT_YAW,
            pitch: 0.0,
            // Already idle, so a freshly focused part turntables at once rather
            // than sitting still for the resume delay.
            idle: FOCUS_IDLE_RESUME,
        }
    }
}

/// The editor's free-fly camera. The gallery parks it, so it needs a handle
/// that does not depend on there being exactly one [`Camera3d`] in the scene.
#[derive(Component)]
pub(crate) struct EditorCamera;

/// The pose the camera had when the gallery parked it, restored on close.
///
/// Stored on the camera rather than in a `Local` so it dies with the scene: a
/// second visit to the editor spawns a fresh camera and must not inherit the
/// previous visit's parked pose.
#[derive(Component)]
pub(crate) struct ParkedPose(Transform);

/// One 3D preview, bound to the UI cell it sits in.
#[derive(Component)]
pub(crate) struct GalleryItem {
    /// The UI node this preview centres itself on.
    cell: Entity,
    /// Largest authored axis of the part: the fit-to-cell size until the real
    /// meshes have loaded and [`measure_gallery_items`] has measured them.
    extent: f32,
    /// Measured bounding-sphere radius of everything the tile actually DRAWS,
    /// in the tile's own unscaled local space. `None` until the meshes are up.
    radius: Option<f32>,
    /// Whether this is the focus view's single part: it turntables, and it
    /// fits to the stage more loosely than a tile does.
    focused: bool,
}

/// Spawn one preview for `section`, centred on `cell` from the next placement
/// pass. Starts hidden: layout has not measured the cell yet, and a tile that
/// appeared at the stage origin for one frame would flash across the view.
pub(crate) fn spawn_tile(
    commands: &mut Commands,
    section: &SectionConfig,
    cell: Entity,
    focused: bool,
) {
    let extent = catalog::extent(section).max_element().max(f32::EPSILON);
    let mut entity = commands.spawn((
        DespawnOnExit(ExampleStates::Editor),
        Name::new(format!("Gallery Preview {}", section.base.name)),
        GalleryItem {
            cell,
            extent,
            radius: None,
            focused,
        },
        Transform::from_translation(STAGE_ORIGIN).with_rotation(Quat::from_rotation_y(PRESENT_YAW)),
        // A tile is scenery: the build observers must never see it as a
        // section of the ship under the pointer.
        Pickable {
            should_block_lower: false,
            is_hoverable: false,
        },
    ));
    insert_preview_section(&mut entity, section, PreviewRole::Display, vec![]);
    // Hidden AFTER the preview bundle, which carries a `Visibility` of its own
    // and used to overwrite this one. An unhidden tile draws one frame at the
    // stage origin - which projects to the middle of the screen - so every
    // category switch flashed its parts across the view before the grid caught
    // them.
    entity.insert(Visibility::Hidden);
}

/// Park the camera on the gallery stage while the gallery is open, and put it
/// back where the player left it on close.
///
/// The free-fly rig is REMOVED rather than ignored: it writes the camera
/// transform from its own input context, and gallery typing must not fly the
/// camera. The pose is re-applied every frame for the same reason the
/// scenario's cinematic camera does it - one stale input frame would drift the
/// stage.
pub(crate) fn park_camera_for_gallery(
    mut commands: Commands,
    state: Res<GalleryState>,
    camera: Option<Single<(Entity, &mut Transform, Option<&ParkedPose>), With<EditorCamera>>>,
) {
    let Some(camera) = camera else {
        return;
    };
    let (entity, mut transform, parked) = camera.into_inner();

    if state.open {
        if parked.is_none() {
            commands
                .entity(entity)
                .insert(ParkedPose(*transform))
                .remove::<WASDCameraController>();
        }
        *transform = Transform::from_translation(STAGE_ORIGIN + Vec3::Z * STAGE_DISTANCE)
            .looking_at(STAGE_ORIGIN, Vec3::Y);
    } else if let Some(ParkedPose(pose)) = parked {
        *transform = *pose;
        commands
            .entity(entity)
            .remove::<ParkedPose>()
            .insert(WASDCameraController);
    }
}

/// Measure what each tile actually DRAWS, so the fit is to the part on screen
/// rather than to the box physics gives it.
///
/// A section's authored collider is not its silhouette: a turret's collider is
/// the small box it mounts through, while the thing that renders is a joint
/// tree with a barrel over a metre long. Fitting to the collider drew that
/// barrel four cells wide. Measured as a bounding-sphere RADIUS about the
/// tile's origin, which no rotation changes - so the focus turntable cannot
/// pulse the part's size as it turns.
///
/// Re-measured every frame rather than cached: a scene streams its meshes in
/// over several frames, and a radius cached from the first of them would fit
/// the part that had arrived so far.
pub(crate) fn measure_gallery_items(
    mut items: Query<(Entity, &mut GalleryItem, &GlobalTransform)>,
    q_children: Query<&Children>,
    q_bounds: Query<(&Aabb, &GlobalTransform)>,
) {
    for (entity, mut item, tile) in &mut items {
        let to_local = tile.affine().inverse();
        let mut radius = 0.0f32;
        let mut stack = vec![entity];
        while let Some(current) = stack.pop() {
            if let Ok((aabb, transform)) = q_bounds.get(current) {
                let centre = Vec3::from(aabb.center);
                let half = Vec3::from(aabb.half_extents);
                for corner in AABB_CORNERS {
                    let world = transform.transform_point(centre + half * corner);
                    radius = radius.max(to_local.transform_point3(world).length());
                }
            }
            if let Ok(children) = q_children.get(current) {
                stack.extend(children.iter());
            }
        }

        if radius <= f32::EPSILON {
            continue;
        }
        // Compared with a tolerance: a bare inequality on floats would flag the
        // component changed every frame for no change anyone can see.
        if item
            .radius
            .is_none_or(|previous| (previous - radius).abs() > previous.max(radius) * 1e-3)
        {
            item.radius = Some(radius);
        }
    }
}

/// The eight sign combinations of an AABB's half-extents.
const AABB_CORNERS: [Vec3; 8] = [
    Vec3::new(-1.0, -1.0, -1.0),
    Vec3::new(-1.0, -1.0, 1.0),
    Vec3::new(-1.0, 1.0, -1.0),
    Vec3::new(-1.0, 1.0, 1.0),
    Vec3::new(1.0, -1.0, -1.0),
    Vec3::new(1.0, -1.0, 1.0),
    Vec3::new(1.0, 1.0, -1.0),
    Vec3::new(1.0, 1.0, 1.0),
];

/// Centre every preview on its cell and fit it to that cell's height.
///
/// Reads the PREVIOUS frame's layout (UI layout runs in `PostUpdate`), which
/// is invisible on a static grid and keeps this out of the layout schedule.
pub(crate) fn place_gallery_items(
    camera: Option<Single<(&Camera, &GlobalTransform), With<EditorCamera>>>,
    view: Res<FocusView>,
    cells: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut items: Query<(&GalleryItem, &mut Transform, &mut Visibility)>,
) {
    let Some(camera) = camera else {
        return;
    };
    let (camera, camera_transform) = *camera;

    for (item, mut transform, mut visibility) in &mut items {
        let Ok((node, cell_transform)) = cells.get(item.cell) else {
            continue;
        };
        // `ComputedNode` is physical pixels and the camera answers in logical
        // ones; the conversion belongs here, not at the call site.
        let scale = node.inverse_scale_factor();
        let size = node.size() * scale;
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let centre = cell_transform.translation * scale;
        let (Ok(middle), Ok(top), Ok(bottom), Ok(left), Ok(right)) = (
            camera.viewport_to_world(camera_transform, centre),
            camera.viewport_to_world(camera_transform, centre - Vec2::new(0.0, size.y * 0.5)),
            camera.viewport_to_world(camera_transform, centre + Vec2::new(0.0, size.y * 0.5)),
            camera.viewport_to_world(camera_transform, centre - Vec2::new(size.x * 0.5, 0.0)),
            camera.viewport_to_world(camera_transform, centre + Vec2::new(size.x * 0.5, 0.0)),
        ) else {
            continue;
        };

        // Fit the SMALLER side: a part fitted to a wide stage's height would
        // still overflow it sideways once the turntable brings its long axis
        // round.
        let at = |ray: Ray3d| ray.get_point(STAGE_DISTANCE);
        let side = at(top)
            .distance(at(bottom))
            .min(at(left).distance(at(right)));
        let fill = if item.focused { view.fill } else { TILE_FILL };
        // The measured silhouette once the meshes are up, the authored collider
        // until then - so a tile is never fitted to nothing.
        let size = item.radius.map_or(item.extent, |radius| radius * 2.0);
        transform.translation = at(middle);
        transform.scale = Vec3::splat(side * fill / size.max(f32::EPSILON));
        *visibility = Visibility::Inherited;
    }
}

/// Zoom and orbit the focused part: the wheel scales it, a left-drag turns it.
///
/// Resets whenever the focus card is not up, so every part is met at the same
/// framing rather than at whatever the last one was left in.
pub(crate) fn drive_focus_view(
    state: Res<GalleryState>,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut view: ResMut<FocusView>,
) {
    if !state.open || !state.focused {
        // Drained, so a drag or scroll made in the grid is not replayed into
        // the focus view on the frame it opens.
        motion.clear();
        wheel.clear();
        if *view != FocusView::default() {
            *view = FocusView::default();
        }
        return;
    }

    let mut touched = false;
    let drag: Vec2 = motion.read().map(|event| event.delta).sum();
    if mouse.pressed(MouseButton::Left) && drag != Vec2::ZERO {
        view.yaw -= drag.x * FOCUS_DRAG_RATE;
        view.pitch =
            (view.pitch - drag.y * FOCUS_DRAG_RATE).clamp(-FOCUS_PITCH_LIMIT, FOCUS_PITCH_LIMIT);
        touched = true;
    }
    for event in wheel.read() {
        // Line and pixel units both arrive here; only the SIGN is a gesture.
        let factor = match event.y.partial_cmp(&0.0) {
            Some(std::cmp::Ordering::Greater) => FOCUS_ZOOM_STEP,
            Some(std::cmp::Ordering::Less) => 1.0 / FOCUS_ZOOM_STEP,
            _ => continue,
        };
        view.fill = (view.fill * factor).clamp(FOCUS_FILL_MIN, FOCUS_FILL_MAX);
        touched = true;
    }

    view.idle = if touched {
        0.0
    } else {
        view.idle + time.delta_secs()
    };
}

/// Pose the focused preview: the builder's orbit, and the turntable once they
/// have left it alone for [`FOCUS_IDLE_RESUME`].
pub(crate) fn pose_focused_item(
    time: Res<Time>,
    mut view: ResMut<FocusView>,
    mut items: Query<(&GalleryItem, &mut Transform)>,
) {
    if view.idle >= FOCUS_IDLE_RESUME {
        view.yaw += SPIN_RATE * time.delta_secs();
    }
    let rotation = Quat::from_euler(EulerRot::YXZ, view.yaw, view.pitch, 0.0);
    for (item, mut transform) in &mut items {
        if item.focused {
            transform.rotation = rotation;
        }
    }
}
