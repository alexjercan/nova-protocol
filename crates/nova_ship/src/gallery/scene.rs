//! The gallery's 3D half: one preview per tile on the host's stage, kept
//! centred in the UI cell that owns it, and the focus view's turntable.
//!
//! The previews follow the UI rather than the other way round: layout owns the
//! grid, and a tile is placed by unprojecting its cell's centre through the
//! host's camera. Change this module when the tile fitting or the focus view
//! changes.

use bevy::{
    app::Propagate,
    camera::primitives::Aabb,
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    ui::{ComputedNode, UiGlobalTransform},
};
use nova_ui::theme::{ActiveUiTheme, UiColor};

use super::{catalog, GalleryHost, GalleryState};
use crate::prelude::*;

/// Ray distance from the host camera to every tile. Equal distances mean
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
pub struct FocusView {
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

/// The UI cell a tile centres itself on. A relationship, so despawning the
/// overlay (or the host it sits in) takes its tiles with it.
#[derive(Component, Debug)]
#[relationship(relationship_target = GalleryTiles)]
pub struct GalleryTileOf(pub Entity);

/// The tiles centred on a cell: one, or none for a cell still waiting for
/// its preview.
#[derive(Component, Debug, Default)]
#[relationship_target(relationship = GalleryTileOf, linked_spawn)]
pub struct GalleryTiles(Vec<Entity>);

/// One 3D preview of a listed part.
#[derive(Component)]
pub struct GalleryItem {
    /// The part's sockets, in its own unscaled local space. Carried only by the
    /// focused preview, which is the one that draws them.
    sockets: Vec<LinkPoint>,
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
    host: &GalleryHost,
) {
    let extent = catalog::extent(section).max_element().max(f32::EPSILON);
    let mut entity = commands.spawn((
        Name::new(format!("Gallery Preview {}", section.base.name)),
        GalleryTileOf(cell),
        Propagate(host.layers.clone()),
        GalleryItem {
            sockets: if focused {
                section.base.link_points.clone()
            } else {
                Vec::new()
            },
            extent,
            radius: None,
            focused,
        },
        Transform::from_translation(host.stage).with_rotation(Quat::from_rotation_y(PRESENT_YAW)),
        // A tile is scenery: the build observers must never see it as a
        // section of the ship under the pointer.
        Pickable {
            should_block_lower: false,
            is_hoverable: false,
        },
    ));
    insert_preview_section(&mut entity, section, PreviewRole::Display);
    // Hidden AFTER the preview bundle, which carries a `Visibility` of its own
    // and used to overwrite this one. An unhidden tile draws one frame at the
    // stage origin - which projects to the middle of the screen - so every
    // category switch flashed its parts across the view before the grid caught
    // them.
    entity.insert(Visibility::Hidden);
}

/// Measure what each tile actually DRAWS, so the fit is to the part on screen
/// rather than to the box physics gives it.
///
/// A section's authored collider is not its silhouette: a turret's collider is
/// the small box it mounts through, while the thing that renders is a joint
/// tree with a barrel over a meter long. Fitting to the collider drew that
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
    host: Single<(&GalleryHost, &ComputedNode, &UiGlobalTransform)>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    view: Res<FocusView>,
    cells: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut items: Query<(
        &GalleryItem,
        &GalleryTileOf,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    let (host, host_node, host_transform) = *host;
    let Ok((camera, camera_transform)) = cameras.get(host.camera) else {
        return;
    };
    let Some(viewport) = camera.logical_viewport_size() else {
        return;
    };
    // The camera's viewport shows the host node, so a point in the node is a
    // point in the viewport scaled by the two sizes: the window for a
    // full-screen host, an image for a node that shows one.
    let host_size = host_node.size();
    if host_size.cmple(Vec2::ZERO).any() {
        return;
    }
    let host_corner = host_transform.translation - host_size * 0.5;
    let to_viewport = viewport / host_size;

    for (item, cell, mut transform, mut visibility) in &mut items {
        let Ok((node, cell_transform)) = cells.get(cell.0) else {
            continue;
        };
        let size = node.size() * to_viewport;
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let centre = (cell_transform.translation - host_corner) * to_viewport;
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
    time: Res<Time<Real>>,
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

/// The group the focus view's socket marks draw in, on the host's layers.
#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
pub struct GalleryGizmos;

/// Put the socket marks on the host's layers at the editor's line weight, so
/// the host's camera draws them and no other.
pub(crate) fn sync_gallery_gizmo_layers(
    host: Single<&GalleryHost>,
    mut store: ResMut<GizmoConfigStore>,
) {
    let (config, _) = store.config_mut::<GalleryGizmos>();
    if config.render_layers != host.layers {
        config.render_layers = host.layers.clone();
    }
    if config.line.width != SOCKET_LINE_WIDTH {
        config.line.width = SOCKET_LINE_WIDTH;
    }
}

/// The socket marks' line width (px), the editor's: under Bevy's 2 px
/// default, so the marks do not outweigh the part they sit on.
const SOCKET_LINE_WIDTH: f32 = 1.0;

/// Draw the focused part's sockets on the stage.
///
/// Where a part can mate is the question the focus view exists to answer, and
/// the stat block could only count them. The same [`draw_socket`] marks the
/// editor's build view draws, so a socket looks the same wherever it is met.
pub(crate) fn draw_focus_sockets(
    state: Res<GalleryState>,
    host: Single<&GalleryHost>,
    theme: Res<ActiveUiTheme>,
    cameras: Query<&GlobalTransform>,
    items: Query<(&GalleryItem, &GlobalTransform)>,
    mut gizmos: Gizmos<GalleryGizmos>,
) {
    if !state.open || !state.focused {
        return;
    }
    let Ok(camera) = cameras.get(host.camera) else {
        return;
    };
    let eye = camera.translation();
    let colour = theme.color(UiColor::Nominal);

    for (item, pose) in &items {
        if !item.focused {
            continue;
        }
        for socket in &item.sockets {
            draw_socket(
                &mut gizmos,
                eye,
                pose.transform_point(socket.position),
                (pose.rotation() * socket.normal).normalize_or(Vec3::Z),
                colour,
                false,
            );
        }
    }
}

/// Pose the focused preview: the builder's orbit, and the turntable once they
/// have left it alone for [`FOCUS_IDLE_RESUME`].
pub(crate) fn pose_focused_item(
    time: Res<Time<Real>>,
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

/// A socket ring's radius as a fraction of its distance from the eye.
///
/// An ANGLE rather than a length: a world-space ring shrank to nothing the
/// moment the camera pulled back far enough to see the whole ship, which is
/// exactly when a builder is looking for somewhere to put a part.
const SOCKET_SCREEN_SIZE: f32 = 0.018;
/// The smallest and largest a ring is allowed to get in world units, so a
/// socket under the nose does not swallow the ship and one across the range is
/// still a mark rather than a pixel.
const SOCKET_RADIUS_RANGE: (f32, f32) = (0.05, 0.5);
/// The stub that shows which way a socket faces, as a multiple of the ring.
const SOCKET_NORMAL_LENGTH: f32 = 4.0;
/// How much wider the halo around the aimed socket is drawn.
const SOCKET_HALO: f32 = 1.8;
/// How far up the view ray a ring floats, as a multiple of its radius, to clear
/// the hull it sits on.
const SOCKET_LIFT: f32 = 2.0;

/// One socket marker: a ring facing the eye plus a stub along its normal, so
/// both WHERE it is and which way it faces read at a glance.
///
/// Sized from the EYE, not from the ship: see [`SOCKET_SCREEN_SIZE`]. `halo`
/// draws a second ring around the first - shape, not only colour, for the one
/// socket the ghost is about to take.
pub fn draw_socket<G: GizmoConfigGroup>(
    gizmos: &mut Gizmos<G>,
    eye: Vec3,
    position: Vec3,
    normal: Vec3,
    colour: Color,
    halo: bool,
) {
    let (near, far) = SOCKET_RADIUS_RANGE;
    let radius = (eye.distance(position) * SOCKET_SCREEN_SIZE).clamp(near, far);
    // The ring faces the EYE, not the socket's plane: a ring lying on the plane
    // goes edge-on the moment you look along the hull, which is a socket you
    // cannot see. The stub carries the facing instead.
    //
    // It also floats a little way up the view ray, because a ring drawn ON the
    // hull is half buried in it. Sliding towards the eye leaves it over the
    // same pixel and clear of the surface.
    let towards_eye = (eye - position).normalize_or(Vec3::Z);
    let centre = position + towards_eye * radius * SOCKET_LIFT;
    let facing = Isometry3d::new(centre, Quat::from_rotation_arc(Vec3::Z, towards_eye));
    gizmos.circle(facing, radius, colour);
    if halo {
        gizmos.circle(facing, radius * SOCKET_HALO, colour);
    }
    gizmos.line(
        position,
        position + normal * radius * SOCKET_NORMAL_LENGTH,
        colour,
    );
}
