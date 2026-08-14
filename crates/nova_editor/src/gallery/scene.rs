//! The gallery's 3D half: parking the editor camera on an empty stage far from
//! the build area, spawning one preview per tile, and keeping each preview
//! centred in the UI cell that owns it.
//!
//! The previews follow the UI rather than the other way round: layout owns the
//! grid, and a tile is placed by unprojecting its cell's centre. Change this
//! module when the stage framing or the tile fitting changes.

use bevy::{
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

/// The same fraction for the focus stage. Lower than a tile's: the stage is
/// most of the screen, and a part that fills it reads as a perspective smear
/// rather than as a shape.
const FOCUS_FILL: f32 = 0.55;

/// Three-quarter presentation, lifted from the parts viewer: yaw the nose
/// (-Z) mostly toward the camera.
const PRESENT_YAW: f32 = 2.5;

/// Turntable rate of the focused preview, in radians per second.
const SPIN_RATE: f32 = 0.5;

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
    /// Largest authored axis of the part, for the fit-to-cell scale.
    extent: f32,
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
            focused,
        },
        Transform::from_translation(STAGE_ORIGIN).with_rotation(Quat::from_rotation_y(PRESENT_YAW)),
        Visibility::Hidden,
        // A tile is scenery: the build observers must never see it as a
        // section of the ship under the pointer.
        Pickable {
            should_block_lower: false,
            is_hoverable: false,
        },
    ));
    insert_preview_section(&mut entity, section, PreviewRole::Display, vec![]);
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

/// Centre every preview on its cell and fit it to that cell's height.
///
/// Reads the PREVIOUS frame's layout (UI layout runs in `PostUpdate`), which
/// is invisible on a static grid and keeps this out of the layout schedule.
pub(crate) fn place_gallery_items(
    camera: Option<Single<(&Camera, &GlobalTransform), With<EditorCamera>>>,
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
        let fill = if item.focused { FOCUS_FILL } else { TILE_FILL };
        transform.translation = at(middle);
        transform.scale = Vec3::splat(side * fill / item.extent);
        *visibility = Visibility::Inherited;
    }
}

/// Turntable the focused preview.
pub(crate) fn spin_focused_item(time: Res<Time>, mut items: Query<(&GalleryItem, &mut Transform)>) {
    for (item, mut transform) in &mut items {
        if item.focused {
            transform.rotation =
                Quat::from_rotation_y(PRESENT_YAW + time.elapsed_secs() * SPIN_RATE);
        }
    }
}
