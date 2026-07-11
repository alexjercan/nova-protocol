//! A reusable "world anchor -> UI node" indicator widget for the HUD.
//!
//! [`screen_indicator`] spawns an absolutely-positioned UI node that
//! [`ScreenIndicatorPlugin`] moves to its anchor's viewport projection every
//! frame: give it an entity to follow or a bare world point, and the widget
//! owns projection, visibility lifecycle, sizing, and off-screen policy, so
//! each HUD overlay is a thin consumer instead of a fresh copy of that loop
//! (task 20260708-165700; architecture in
//! docs/spikes/20260709-164502-screen-indicator-architecture.md).
//!
//! The indicator IS the projected node: parent it under a full-screen
//! [`screen_indicator_layer`] and attach arbitrary content to it - an
//! [`ImageNode`] sprite on the node itself, [`Text`] or bar children. The
//! widget only writes the node's position, size and visibility. Neither
//! bundle carries a [`Name`], so consumers name their own nodes.
//!
//! Deliberately UI-pass: not a second Camera2d (a second window-targeting
//! camera on Bevy 0.19 blacks out the 3D scene camera) and not gizmos
//! (debug-grade, no image/text styling). Projection uses the camera tagged
//! [`ScreenIndicatorCamera`].

use avian3d::prelude::*;
use bevy::{prelude::*, ui::UiSystems};
use bevy_common_systems::prelude::ChaseCameraSystems;

pub mod prelude {
    pub use super::{
        screen_indicator, screen_indicator_layer, ScreenIndicatorAnchor, ScreenIndicatorAnchorKind,
        ScreenIndicatorArrowMarker, ScreenIndicatorCamera, ScreenIndicatorConfig,
        ScreenIndicatorMarker, ScreenIndicatorOffscreen, ScreenIndicatorOffset,
        ScreenIndicatorPlugin, ScreenIndicatorSize, ScreenIndicatorSystems,
    };
}

/// Marker for a screen-projected indicator node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ScreenIndicatorMarker;

/// What an indicator tracks in the world.
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub enum ScreenIndicatorAnchorKind {
    /// Follow this entity's `GlobalTransform`. The indicator hides when the
    /// entity no longer resolves (despawned, or not yet spawned).
    Entity(Entity),
    /// Project this fixed world point. Driver systems overwrite it each frame
    /// for computed anchors (e.g. a turret's lead intercept point).
    Point(Vec3),
}

/// The indicator's current anchor; `None` hides the indicator. Retargeting is
/// just writing this component.
#[derive(Component, Debug, Clone, Copy, PartialEq, Deref, DerefMut, Reflect)]
pub struct ScreenIndicatorAnchor(pub Option<ScreenIndicatorAnchorKind>);

/// How the indicator node is sized each frame.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
pub enum ScreenIndicatorSize {
    /// A fixed on-screen size in logical pixels.
    Fixed(Vec2),
    /// Track the anchor entity's on-screen extent (from the union of the
    /// collider AABBs of its subtree), never shrinking below `min_px`.
    /// `Point` anchors and entities without collider AABBs fall back to
    /// `min_px`.
    ApparentSize {
        /// Minimum indicator size (px), also the fallback size.
        min_px: f32,
    },
}

/// Offset (px) applied to the projected anchor point before the off-screen
/// test, so content can sit beside its anchor. Ignored while the anchor is
/// behind the camera (a clamped indicator hugs the edge regardless).
#[derive(Component, Debug, Clone, Copy, PartialEq, Deref, DerefMut, Reflect)]
pub struct ScreenIndicatorOffset(pub Vec2);

/// What happens when the anchor projects outside the viewport (or is behind
/// the camera).
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
pub enum ScreenIndicatorOffscreen {
    /// Hide the indicator until the anchor is back on-screen.
    Hide,
    /// Keep the indicator visible, clamped to the viewport edge inset by
    /// `margin_px`. A descendant tagged [`ScreenIndicatorArrowMarker`] is
    /// shown and rotated to point toward the anchor while clamped, and hidden
    /// while the anchor is on-screen.
    ClampToEdge {
        /// Inset (px) from the viewport edges the indicator is clamped to.
        margin_px: f32,
    },
}

/// Marker for the optional direction-arrow node under a clamping indicator.
/// The arrow art is expected to point up; the widget rotates it via
/// [`UiTransform`].
#[derive(Component, Debug, Clone, Reflect)]
pub struct ScreenIndicatorArrowMarker;

/// Marker for the camera indicators project through. Exactly one camera
/// should carry it: without one every indicator hides, and with more than
/// one the first is used and a warning is logged once.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ScreenIndicatorCamera;

/// Configuration for [`screen_indicator`].
#[derive(Clone, Debug)]
pub struct ScreenIndicatorConfig {
    /// Initial anchor; usually `None` until a driver system sets one.
    pub anchor: Option<ScreenIndicatorAnchorKind>,
    /// Sizing mode.
    pub size: ScreenIndicatorSize,
    /// Pixel offset from the projected anchor point.
    pub offset: Vec2,
    /// Off-screen policy.
    pub offscreen: ScreenIndicatorOffscreen,
}

impl Default for ScreenIndicatorConfig {
    fn default() -> Self {
        Self {
            anchor: None,
            size: ScreenIndicatorSize::Fixed(Vec2::splat(24.0)),
            offset: Vec2::ZERO,
            offscreen: ScreenIndicatorOffscreen::Hide,
        }
    }
}

/// Bundle for one screen-projected indicator: the absolutely-positioned node
/// [`ScreenIndicatorPlugin`] drives. Starts hidden; parent it under a
/// [`screen_indicator_layer`] and attach content (sprite, text, arrow).
pub fn screen_indicator(config: ScreenIndicatorConfig) -> impl Bundle {
    debug!("screen_indicator: config {:?}", config);

    (
        ScreenIndicatorMarker,
        ScreenIndicatorAnchor(config.anchor),
        config.size,
        ScreenIndicatorOffset(config.offset),
        config.offscreen,
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        Pickable::IGNORE,
        Visibility::Hidden,
    )
}

/// Bundle for a full-screen click-through container to parent indicators
/// under, so consumers stop copy-pasting the layer node.
pub fn screen_indicator_layer() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// System set for the indicator update, so consumers can order driver systems
/// before it. Lives in PostUpdate, between the chase camera's final move and
/// UI layout; Update-schedule drivers precede it by schedule order.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScreenIndicatorSystems;

/// Plugin that projects every [`ScreenIndicatorMarker`] node to its anchor
/// each frame.
#[derive(Default)]
pub struct ScreenIndicatorPlugin;

impl Plugin for ScreenIndicatorPlugin {
    fn build(&self, app: &mut App) {
        debug!("ScreenIndicatorPlugin: build");

        // Projection must sample the SAME camera pose the frame renders
        // with. In Update the chase camera has not moved yet (bcs moves it
        // in PostUpdate), so indicators lagged the world by one frame of
        // camera motion - the HUD twitch of task 20260710-231928. The slot
        // is: after the chase camera writes the camera Transform, before UI
        // layout consumes the node positions (bevy_ui runs layout BEFORE
        // transform propagation, so fresh poses are computed via
        // TransformHelper inside the system rather than read from
        // GlobalTransform).
        app.configure_sets(
            PostUpdate,
            ScreenIndicatorSystems
                .after(ChaseCameraSystems::Sync)
                .before(UiSystems::Layout),
        );
        app.add_systems(
            PostUpdate,
            update_screen_indicators.in_set(ScreenIndicatorSystems),
        );
    }
}

/// Where an indicator ends up this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Placement {
    /// Not drawn this frame.
    Hidden,
    /// Drawn centered at `center` (viewport px). `arrow` is the direction
    /// toward the anchor when the indicator is clamped to the edge, `None`
    /// while the anchor is on-screen.
    Visible { center: Vec2, arrow: Option<Vec2> },
}

/// Decide visibility and position from the projected anchor.
///
/// `projected` is the viewport projection of the (offset) anchor point, or
/// `None` when the anchor does not project (behind the camera); `view_pos` is
/// the anchor in camera space, used to keep a direction in that case.
fn place(
    viewport: Vec2,
    projected: Option<Vec2>,
    view_pos: Vec3,
    policy: ScreenIndicatorOffscreen,
) -> Placement {
    let center = viewport / 2.0;
    match projected {
        Some(pos) if pos.x >= 0.0 && pos.y >= 0.0 && pos.x <= viewport.x && pos.y <= viewport.y => {
            Placement::Visible {
                center: pos,
                arrow: None,
            }
        }
        Some(pos) => match policy {
            ScreenIndicatorOffscreen::Hide => Placement::Hidden,
            ScreenIndicatorOffscreen::ClampToEdge { margin_px } => {
                let dir = direction_from_center(pos, center);
                Placement::Visible {
                    center: clamp_to_rect(pos, viewport, margin_px),
                    arrow: Some(dir),
                }
            }
        },
        None => match policy {
            ScreenIndicatorOffscreen::Hide => Placement::Hidden,
            ScreenIndicatorOffscreen::ClampToEdge { margin_px } => {
                let dir = behind_camera_direction(view_pos);
                // A virtual point far along the direction, clamped back to
                // the edge rect, lands the indicator on the correct edge.
                let pos = center + dir * viewport.max_element();
                Placement::Visible {
                    center: clamp_to_rect(pos, viewport, margin_px),
                    arrow: Some(dir),
                }
            }
        },
    }
}

/// Clamp `pos` to the viewport rect inset by `margin_px` on every side. A
/// negative margin is treated as zero (the indicator never leaves the
/// viewport), and a margin wider than half the viewport degenerates to the
/// viewport center instead of panicking.
fn clamp_to_rect(pos: Vec2, viewport: Vec2, margin_px: f32) -> Vec2 {
    let margin_px = margin_px.max(0.0);
    let min = Vec2::splat(margin_px).min(viewport / 2.0);
    let max = (viewport - margin_px).max(viewport / 2.0);
    pos.clamp(min, max)
}

/// Direction from the viewport center toward an off-screen projected point.
/// Falls back to straight down when the point sits on the center.
fn direction_from_center(pos: Vec2, center: Vec2) -> Vec2 {
    (pos - center).try_normalize().unwrap_or(Vec2::Y)
}

/// Screen direction toward an anchor that does not project (behind the
/// camera): its camera-space offset flattened onto the view plane, with y
/// flipped because viewport y grows downward. Falls back to straight down
/// for a point exactly on the camera axis.
fn behind_camera_direction(view_pos: Vec3) -> Vec2 {
    Vec2::new(view_pos.x, -view_pos.y)
        .try_normalize()
        .unwrap_or(Vec2::Y)
}

/// Rotation (radians, for [`Rot2`]) that turns up-pointing arrow art toward
/// `dir` in viewport coordinates (x right, y down).
fn arrow_angle(dir: Vec2) -> f32 {
    dir.x.atan2(-dir.y)
}

/// Union the world-space [`ColliderAabb`]s of `entity` and all of its
/// descendants into a single bounding box, or `None` if none of them has a
/// collider AABB.
///
/// A tracked body keeps its colliders on child entities (an asteroid's
/// collider node, or a ship's sections), so the whole subtree is walked
/// rather than just the root.
fn target_world_aabb(
    entity: Entity,
    q_children: &Query<&Children>,
    q_aabb: &Query<&ColliderAabb>,
) -> Option<ColliderAabb> {
    let mut acc: Option<ColliderAabb> = None;
    let mut stack = vec![entity];
    while let Some(current) = stack.pop() {
        if let Ok(aabb) = q_aabb.get(current) {
            acc = Some(match acc {
                Some(existing) => existing.merged(*aabb),
                None => *aabb,
            });
        }
        if let Ok(children) = q_children.get(current) {
            stack.extend(children.iter());
        }
    }
    acc
}

/// Indicator size (px) for the frame. `ApparentSize` measures the anchor
/// entity's on-screen extent: take the world bounding-sphere radius from the
/// subtree's collider AABBs, project a point one radius to the camera's
/// right, and read the pixel distance from the projected center - so the
/// size grows with apparent size and shrinks with distance. Only the center
/// must project, which keeps this robust when a close target's far corners
/// fall behind the camera. Falls back to `min_px` for `Point` anchors, for
/// entities without collider AABBs (e.g. the frame they spawned), and when
/// the anchor itself did not project.
#[allow(clippy::too_many_arguments)]
fn indicator_size(
    size_mode: ScreenIndicatorSize,
    anchor_entity: Option<Entity>,
    anchor_pos: Vec3,
    projected: Option<Vec2>,
    camera_transform: &GlobalTransform,
    camera: &Camera,
    q_children: &Query<&Children>,
    q_aabb: &Query<&ColliderAabb>,
) -> Vec2 {
    match size_mode {
        ScreenIndicatorSize::Fixed(size) => size,
        ScreenIndicatorSize::ApparentSize { min_px } => {
            let fallback = Vec2::splat(min_px);
            let (Some(entity), Some(center)) = (anchor_entity, projected) else {
                return fallback;
            };
            let Some(aabb) = target_world_aabb(entity, q_children, q_aabb) else {
                return fallback;
            };
            let world_radius = aabb.size().length() * 0.5;
            let edge_world = anchor_pos + camera_transform.right() * world_radius;
            let Ok(edge) = camera.world_to_viewport(camera_transform, edge_world) else {
                return fallback;
            };
            Vec2::splat((2.0 * center.distance(edge)).max(min_px))
        }
    }
}

/// Project every indicator to its anchor: resolve the anchor (a missing one
/// hides the node), project through the [`ScreenIndicatorCamera`], size the
/// node, apply the pixel offset, and apply the off-screen policy including
/// the direction arrow.
#[allow(clippy::type_complexity)]
fn update_screen_indicators(
    mut q_indicator: Query<
        (
            Entity,
            &ScreenIndicatorAnchor,
            &ScreenIndicatorSize,
            &ScreenIndicatorOffset,
            &ScreenIndicatorOffscreen,
            &mut Node,
            &mut Visibility,
        ),
        With<ScreenIndicatorMarker>,
    >,
    // This runs BEFORE this frame's transform propagation (UI layout comes
    // first in PostUpdate), so `GlobalTransform` still holds last frame's
    // poses. TransformHelper composes fresh `Transform`s instead: the
    // camera pose the chase camera just wrote and the eased anchor poses
    // the frame will render with. HUD-scale anchor counts make the
    // per-entity hierarchy walk negligible.
    transform_helper: TransformHelper,
    q_children: Query<&Children>,
    q_aabb: Query<&ColliderAabb>,
    q_nested: Query<(), With<ScreenIndicatorMarker>>,
    mut q_arrow: Query<
        (&mut UiTransform, &mut Visibility),
        (
            With<ScreenIndicatorArrowMarker>,
            Without<ScreenIndicatorMarker>,
        ),
    >,
    q_camera: Query<(Entity, &Camera), With<ScreenIndicatorCamera>>,
) {
    // Without a projection camera every indicator hides, rather than freezing
    // at a stale position. More than one tagged camera is a consumer bug:
    // warn and project through the first, so indicators stay live and the
    // problem stays diagnosable (an Option<Single> would silently skip the
    // whole system and freeze every indicator instead).
    let mut cameras = q_camera.iter();
    let camera = cameras.next();
    if cameras.next().is_some() {
        warn_once!(
            "update_screen_indicators: multiple ScreenIndicatorCamera cameras, using the first"
        );
    }

    for (entity, anchor, size_mode, offset, offscreen, mut node, mut visibility) in &mut q_indicator
    {
        let Some((camera_entity, camera)) = camera else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let Ok(camera_transform) = transform_helper.compute_global_transform(camera_entity) else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let camera_transform = &camera_transform;
        let Some(viewport) = camera.logical_viewport_size() else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };

        // Resolve the anchor to a world position; entity anchors follow their
        // freshly composed pose and hide when the entity no longer resolves.
        let Some(kind) = **anchor else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let (anchor_pos, anchor_entity) = match kind {
            ScreenIndicatorAnchorKind::Entity(anchor_entity) => {
                let Ok(transform) = transform_helper.compute_global_transform(anchor_entity) else {
                    visibility.set_if_neq(Visibility::Hidden);
                    continue;
                };
                (transform.translation(), Some(anchor_entity))
            }
            ScreenIndicatorAnchorKind::Point(point) => (point, None),
        };
        if !anchor_pos.is_finite() {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        }

        // Err here means the anchor does not project (behind the camera); the
        // viewport size was checked above.
        let projected = camera.world_to_viewport(camera_transform, anchor_pos).ok();
        let view_pos = camera_transform
            .affine()
            .inverse()
            .transform_point3(anchor_pos);

        let placement = place(
            viewport,
            projected.map(|pos| pos + **offset),
            view_pos,
            *offscreen,
        );
        let Placement::Visible { center, arrow } = placement else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };

        let size = indicator_size(
            *size_mode,
            anchor_entity,
            anchor_pos,
            projected,
            camera_transform,
            camera,
            &q_children,
            &q_aabb,
        );

        node.width = Val::Px(size.x);
        node.height = Val::Px(size.y);
        node.left = Val::Px(center.x - size.x / 2.0);
        node.top = Val::Px(center.y - size.y / 2.0);
        visibility.set_if_neq(Visibility::Visible);

        update_arrows(entity, arrow, &q_children, &q_nested, &mut q_arrow);
    }
}

/// Show and rotate (or hide) every [`ScreenIndicatorArrowMarker`] descendant
/// of `indicator`. Shown arrows use `Visibility::Inherited` so they still
/// vanish with the indicator. Nested indicators own their own arrows, so the
/// walk does not descend into them.
fn update_arrows(
    indicator: Entity,
    arrow_dir: Option<Vec2>,
    q_children: &Query<&Children>,
    q_nested: &Query<(), With<ScreenIndicatorMarker>>,
    q_arrow: &mut Query<
        (&mut UiTransform, &mut Visibility),
        (
            With<ScreenIndicatorArrowMarker>,
            Without<ScreenIndicatorMarker>,
        ),
    >,
) {
    let mut stack: Vec<Entity> = q_children
        .get(indicator)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    while let Some(current) = stack.pop() {
        if q_nested.contains(current) {
            continue;
        }
        if let Ok((mut ui_transform, mut visibility)) = q_arrow.get_mut(current) {
            match arrow_dir {
                Some(dir) => {
                    let rotation = Rot2::radians(arrow_angle(dir));
                    if ui_transform.rotation != rotation {
                        ui_transform.rotation = rotation;
                    }
                    visibility.set_if_neq(Visibility::Inherited);
                }
                None => {
                    visibility.set_if_neq(Visibility::Hidden);
                }
            }
        }
        if let Ok(children) = q_children.get(current) {
            stack.extend(children.iter());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use bevy::{
        camera::{ComputedCameraValues, RenderTargetInfo},
        ecs::system::{RunSystemOnce, SystemState},
    };

    use super::*;

    // -- pure helpers --

    #[test]
    fn arrow_angle_matches_cardinal_directions() {
        // Up-pointing art: up keeps it, right/down/left rotate it.
        assert!(arrow_angle(Vec2::new(0.0, -1.0)).abs() < 1e-6);
        assert!((arrow_angle(Vec2::new(1.0, 0.0)) - FRAC_PI_2).abs() < 1e-6);
        assert!((arrow_angle(Vec2::new(0.0, 1.0)).abs() - PI).abs() < 1e-6);
        assert!((arrow_angle(Vec2::new(-1.0, 0.0)) + FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn clamp_to_rect_respects_margin() {
        let viewport = Vec2::new(800.0, 600.0);
        assert_eq!(
            clamp_to_rect(Vec2::new(900.0, 300.0), viewport, 20.0),
            Vec2::new(780.0, 300.0)
        );
        assert_eq!(
            clamp_to_rect(Vec2::new(-50.0, -50.0), viewport, 20.0),
            Vec2::new(20.0, 20.0)
        );
        // Inside the rect: untouched.
        assert_eq!(
            clamp_to_rect(Vec2::new(400.0, 300.0), viewport, 20.0),
            Vec2::new(400.0, 300.0)
        );
    }

    #[test]
    fn clamp_to_rect_degenerate_margin_centers() {
        // A margin wider than half the viewport must not panic (f32::clamp
        // panics on min > max); it degenerates to the center.
        let viewport = Vec2::new(100.0, 100.0);
        assert_eq!(
            clamp_to_rect(Vec2::new(0.0, 0.0), viewport, 500.0),
            Vec2::new(50.0, 50.0)
        );
    }

    #[test]
    fn behind_camera_direction_flattens_view_offset() {
        // Camera space: x right, y up, looking down -z. Behind and to the
        // right -> arrow right; behind and above -> arrow up (screen -y).
        assert_eq!(behind_camera_direction(Vec3::new(1.0, 0.0, 5.0)), Vec2::X);
        assert_eq!(
            behind_camera_direction(Vec3::new(0.0, 2.0, 5.0)),
            Vec2::new(0.0, -1.0)
        );
        // Exactly on the camera axis: fall back to straight down.
        assert_eq!(behind_camera_direction(Vec3::new(0.0, 0.0, 5.0)), Vec2::Y);
    }

    #[test]
    fn place_keeps_on_screen_points() {
        let placement = place(
            Vec2::new(800.0, 600.0),
            Some(Vec2::new(100.0, 100.0)),
            Vec3::new(0.0, 0.0, -10.0),
            ScreenIndicatorOffscreen::Hide,
        );
        assert_eq!(
            placement,
            Placement::Visible {
                center: Vec2::new(100.0, 100.0),
                arrow: None
            }
        );
    }

    #[test]
    fn place_hides_offscreen_points_under_hide_policy() {
        let placement = place(
            Vec2::new(800.0, 600.0),
            Some(Vec2::new(900.0, 300.0)),
            Vec3::new(5.0, 0.0, -10.0),
            ScreenIndicatorOffscreen::Hide,
        );
        assert_eq!(placement, Placement::Hidden);
    }

    #[test]
    fn place_clamps_offscreen_points_with_direction() {
        let placement = place(
            Vec2::new(800.0, 600.0),
            Some(Vec2::new(900.0, 300.0)),
            Vec3::new(5.0, 0.0, -10.0),
            ScreenIndicatorOffscreen::ClampToEdge { margin_px: 20.0 },
        );
        assert_eq!(
            placement,
            Placement::Visible {
                center: Vec2::new(780.0, 300.0),
                arrow: Some(Vec2::X)
            }
        );
    }

    #[test]
    fn place_clamps_behind_camera_points() {
        // Directly behind: the fallback direction is straight down, so the
        // indicator lands on the bottom edge.
        let placement = place(
            Vec2::new(800.0, 600.0),
            None,
            Vec3::new(0.0, 0.0, 5.0),
            ScreenIndicatorOffscreen::ClampToEdge { margin_px: 20.0 },
        );
        assert_eq!(
            placement,
            Placement::Visible {
                center: Vec2::new(400.0, 580.0),
                arrow: Some(Vec2::Y)
            }
        );
    }

    #[test]
    fn place_hides_behind_camera_points_under_hide_policy() {
        let placement = place(
            Vec2::new(800.0, 600.0),
            None,
            Vec3::new(0.0, 0.0, 5.0),
            ScreenIndicatorOffscreen::Hide,
        );
        assert_eq!(placement, Placement::Hidden);
    }

    // -- collider AABB union (moved with the ApparentSize mode from
    //    hud/torpedo_target.rs) --

    #[test]
    fn target_world_aabb_unions_child_collider_aabbs() {
        // ApparentSize sizes to the whole target, whose colliders live on
        // child nodes (asteroid collider node, ship sections). The parent
        // itself has no collider, so the union must come from walking the
        // children.
        let mut world = World::new();
        let child_a = world
            .spawn(ColliderAabb::from_min_max(
                Vec3::new(-1.0, -1.0, -1.0),
                Vec3::ZERO,
            ))
            .id();
        let child_b = world
            .spawn(ColliderAabb::from_min_max(
                Vec3::ZERO,
                Vec3::new(2.0, 3.0, 4.0),
            ))
            .id();
        let parent = world.spawn_empty().add_children(&[child_a, child_b]).id();

        let mut state: SystemState<(Query<&Children>, Query<&ColliderAabb>)> =
            SystemState::new(&mut world);
        let (q_children, q_aabb) = state.get(&world).unwrap();

        let aabb =
            target_world_aabb(parent, &q_children, &q_aabb).expect("subtree has collider AABBs");
        assert_eq!(aabb.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vec3::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn target_world_aabb_is_none_without_colliders() {
        // A target whose colliders are not ready yet (e.g. spawn frame)
        // yields no AABB, so ApparentSize falls back to the minimum size.
        let mut world = World::new();
        let entity = world.spawn_empty().id();

        let mut state: SystemState<(Query<&Children>, Query<&ColliderAabb>)> =
            SystemState::new(&mut world);
        let (q_children, q_aabb) = state.get(&world).unwrap();

        assert!(target_world_aabb(entity, &q_children, &q_aabb).is_none());
    }

    // -- whole-system behavior against a fabricated camera --

    /// A camera whose computed values are filled in by hand, since no render
    /// backend runs in tests: 90 degree vertical FOV, 800x600 viewport,
    /// identity transform (at the origin looking down -Z).
    fn test_camera() -> (Transform, Camera) {
        let camera = Camera {
            computed: ComputedCameraValues {
                clip_from_view: Mat4::perspective_infinite_reverse_rh(
                    FRAC_PI_2,
                    800.0 / 600.0,
                    0.1,
                ),
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(800, 600),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        };
        (Transform::IDENTITY, camera)
    }

    fn spawn_camera(world: &mut World) {
        let (transform, camera) = test_camera();
        world.spawn((transform, camera, ScreenIndicatorCamera));
    }

    fn node_rect(world: &World, entity: Entity) -> (f32, f32, f32, f32) {
        let node = world.entity(entity).get::<Node>().expect("node exists");
        let px = |val: Val| match val {
            Val::Px(px) => px,
            other => panic!("expected Val::Px, got {other:?}"),
        };
        (px(node.left), px(node.top), px(node.width), px(node.height))
    }

    #[test]
    fn anchor_none_stays_hidden() {
        let mut world = World::new();
        spawn_camera(&mut world);
        let indicator = world
            .spawn(screen_indicator(ScreenIndicatorConfig::default()))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();

        assert_eq!(
            *world.entity(indicator).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );
    }

    #[test]
    fn point_anchor_centers_the_node() {
        let mut world = World::new();
        spawn_camera(&mut world);
        // A point on the camera axis 10 units ahead projects to the viewport
        // center (400, 300).
        let indicator = world
            .spawn(screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Point(Vec3::new(0.0, 0.0, -10.0))),
                size: ScreenIndicatorSize::Fixed(Vec2::splat(32.0)),
                ..default()
            }))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();

        assert_eq!(
            *world.entity(indicator).get::<Visibility>().unwrap(),
            Visibility::Visible
        );
        let (left, top, width, height) = node_rect(&world, indicator);
        assert!((left - 384.0).abs() < 0.5, "left {left}");
        assert!((top - 284.0).abs() < 0.5, "top {top}");
        assert_eq!(width, 32.0);
        assert_eq!(height, 32.0);
    }

    #[test]
    fn entity_anchor_follows_and_dead_entity_hides() {
        let mut world = World::new();
        spawn_camera(&mut world);
        let target = world
            .spawn(Transform::from_translation(Vec3::new(0.0, 0.0, -10.0)))
            .id();
        let indicator = world
            .spawn(screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Entity(target)),
                size: ScreenIndicatorSize::Fixed(Vec2::splat(32.0)),
                ..default()
            }))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();
        assert_eq!(
            *world.entity(indicator).get::<Visibility>().unwrap(),
            Visibility::Visible
        );

        world.despawn(target);
        world.run_system_once(update_screen_indicators).unwrap();
        assert_eq!(
            *world.entity(indicator).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );
    }

    #[test]
    fn offset_shifts_the_node() {
        let mut world = World::new();
        spawn_camera(&mut world);
        let indicator = world
            .spawn(screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Point(Vec3::new(0.0, 0.0, -10.0))),
                size: ScreenIndicatorSize::Fixed(Vec2::splat(32.0)),
                offset: Vec2::new(10.0, -5.0),
                ..default()
            }))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();

        let (left, top, _, _) = node_rect(&world, indicator);
        assert!((left - 394.0).abs() < 0.5, "left {left}");
        assert!((top - 279.0).abs() < 0.5, "top {top}");
    }

    #[test]
    fn behind_camera_hides_or_clamps_by_policy() {
        let mut world = World::new();
        spawn_camera(&mut world);
        let behind = Vec3::new(0.0, 0.0, 10.0);
        let hider = world
            .spawn(screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Point(behind)),
                ..default()
            }))
            .id();
        let clamper = world
            .spawn((
                screen_indicator(ScreenIndicatorConfig {
                    anchor: Some(ScreenIndicatorAnchorKind::Point(behind)),
                    size: ScreenIndicatorSize::Fixed(Vec2::splat(32.0)),
                    offscreen: ScreenIndicatorOffscreen::ClampToEdge { margin_px: 20.0 },
                    ..default()
                }),
                children![(
                    ScreenIndicatorArrowMarker,
                    Node::default(),
                    UiTransform::default(),
                    Visibility::Hidden,
                )],
            ))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();

        assert_eq!(
            *world.entity(hider).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *world.entity(clamper).get::<Visibility>().unwrap(),
            Visibility::Visible
        );
        // Directly behind falls back to the bottom edge, arrow pointing down
        // (rotated by pi from the up-pointing art).
        let (left, top, _, _) = node_rect(&world, clamper);
        assert!((left - 384.0).abs() < 0.5, "left {left}");
        assert!((top - (580.0 - 16.0)).abs() < 0.5, "top {top}");
        let arrow = world
            .entity(clamper)
            .get::<Children>()
            .expect("clamper has the arrow child")[0];
        assert_eq!(
            *world.entity(arrow).get::<Visibility>().unwrap(),
            Visibility::Inherited
        );
        let rotation = world
            .entity(arrow)
            .get::<UiTransform>()
            .unwrap()
            .rotation
            .as_radians();
        assert!((rotation.abs() - PI).abs() < 1e-3, "rotation {rotation}");
    }

    #[test]
    fn arrow_hides_again_when_anchor_returns_on_screen() {
        let mut world = World::new();
        spawn_camera(&mut world);
        let indicator = world
            .spawn((
                screen_indicator(ScreenIndicatorConfig {
                    anchor: Some(ScreenIndicatorAnchorKind::Point(Vec3::new(0.0, 0.0, 10.0))),
                    offscreen: ScreenIndicatorOffscreen::ClampToEdge { margin_px: 20.0 },
                    ..default()
                }),
                children![(
                    ScreenIndicatorArrowMarker,
                    Node::default(),
                    UiTransform::default(),
                    Visibility::Hidden,
                )],
            ))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();
        let arrow = world.entity(indicator).get::<Children>().unwrap()[0];
        assert_eq!(
            *world.entity(arrow).get::<Visibility>().unwrap(),
            Visibility::Inherited
        );

        // Move the anchor in front of the camera: the arrow hides, the
        // indicator stays visible.
        **world
            .entity_mut(indicator)
            .get_mut::<ScreenIndicatorAnchor>()
            .unwrap() = Some(ScreenIndicatorAnchorKind::Point(Vec3::new(0.0, 0.0, -10.0)));
        world.run_system_once(update_screen_indicators).unwrap();
        assert_eq!(
            *world.entity(indicator).get::<Visibility>().unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            *world.entity(arrow).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );
    }

    #[test]
    fn apparent_size_point_anchor_falls_back_to_min() {
        let mut world = World::new();
        spawn_camera(&mut world);
        let indicator = world
            .spawn(screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Point(Vec3::new(0.0, 0.0, -10.0))),
                size: ScreenIndicatorSize::ApparentSize { min_px: 32.0 },
                ..default()
            }))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();

        let (_, _, width, height) = node_rect(&world, indicator);
        assert_eq!(width, 32.0);
        assert_eq!(height, 32.0);
    }

    #[test]
    fn apparent_size_tracks_entity_extent() {
        let mut world = World::new();
        spawn_camera(&mut world);
        // A 2x2x2 AABB centered 10 ahead: bounding-sphere radius sqrt(12)/2.
        // With a 90 degree vertical FOV the edge point lands at
        // x_ndc = r / (z * aspect), i.e. 400 * r / (10 * 4/3) px right of
        // center on the 800 px viewport, so the node is 2x that wide.
        let target = world
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, -10.0)),
                ColliderAabb::from_min_max(Vec3::new(-1.0, -1.0, -11.0), Vec3::new(1.0, 1.0, -9.0)),
            ))
            .id();
        let indicator = world
            .spawn(screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Entity(target)),
                size: ScreenIndicatorSize::ApparentSize { min_px: 32.0 },
                ..default()
            }))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();

        let radius = (12.0_f32).sqrt() / 2.0;
        let expected = 2.0 * (400.0 * radius / (10.0 * (800.0 / 600.0)));
        let (_, _, width, _) = node_rect(&world, indicator);
        assert!(
            (width - expected).abs() < 1.0,
            "width {width}, expected {expected}"
        );
    }

    #[test]
    fn multiple_cameras_use_the_first_and_stay_live() {
        // Two tagged cameras is a consumer bug, but indicators must keep
        // projecting (through the first) instead of silently freezing.
        let mut world = World::new();
        spawn_camera(&mut world);
        spawn_camera(&mut world);
        let indicator = world
            .spawn(screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Point(Vec3::new(0.0, 0.0, -10.0))),
                size: ScreenIndicatorSize::Fixed(Vec2::splat(32.0)),
                ..default()
            }))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();

        assert_eq!(
            *world.entity(indicator).get::<Visibility>().unwrap(),
            Visibility::Visible
        );
    }

    #[test]
    fn missing_camera_hides_indicators() {
        let mut world = World::new();
        let indicator = world
            .spawn(screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Point(Vec3::new(0.0, 0.0, -10.0))),
                ..default()
            }))
            .id();
        *world.entity_mut(indicator).get_mut::<Visibility>().unwrap() = Visibility::Visible;

        world.run_system_once(update_screen_indicators).unwrap();

        assert_eq!(
            *world.entity(indicator).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );
    }

    /// The projection must place indicators with the SAME camera pose the
    /// frame renders with (task 20260710-231928). Before the PostUpdate
    /// move the system ran in Update, one chase-camera step early: every
    /// frame the HUD was placed with LAST frame's camera while the world
    /// rendered with this frame's, so anchored text jittered by one frame
    /// of camera motion. A smoothed chase camera trailing a cruising ship
    /// keeps the camera moving every frame; the node position must match a
    /// projection recomputed from the END-of-frame (rendered) poses to
    /// sub-pixel precision on every frame.
    #[test]
    fn indicator_projects_with_the_frames_final_camera_pose() {
        use core::time::Duration;

        use bevy::{time::TimeUpdateStrategy, transform::TransformSystems};
        use bevy_common_systems::prelude::{ChaseCamera, ChaseCameraInput, ChaseCameraPlugin};

        #[derive(Component)]
        struct CruisingShip;

        fn move_ship(time: Res<Time>, mut q_ship: Query<&mut Transform, With<CruisingShip>>) {
            for mut transform in &mut q_ship {
                transform.translation.x += 120.0 * time.delta_secs();
            }
        }

        // Mirrors nova's update_chase_camera_input: the camera anchor is the
        // ship pose read in Update.
        fn drive_camera_input(
            q_ship: Query<&Transform, With<CruisingShip>>,
            mut q_input: Query<&mut ChaseCameraInput>,
        ) {
            let Ok(ship) = q_ship.single() else {
                return;
            };
            for mut input in &mut q_input {
                input.anchor_pos = ship.translation;
            }
        }

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, ChaseCameraPlugin));
        app.add_plugins(ScreenIndicatorPlugin);
        // Mirror the production pin from nova's camera controller: the
        // chase move lands before propagation, so the rendered camera pose
        // is this frame's.
        app.configure_sets(
            PostUpdate,
            bevy_common_systems::prelude::ChaseCameraSystems::Sync
                .before(TransformSystems::Propagate),
        );
        app.add_systems(Update, (move_ship, drive_camera_input).chain());
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));

        let ship = app
            .world_mut()
            .spawn((CruisingShip, Transform::default()))
            .id();
        let (_, camera) = test_camera();
        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                camera,
                ScreenIndicatorCamera,
                ChaseCamera {
                    offset: Vec3::new(0.0, 0.0, 15.0),
                    focus_offset: Vec3::ZERO,
                    // Heavy smoothing keeps the camera trailing (and thus
                    // MOVING) every frame - the regime where a stale camera
                    // pose separates HUD from world.
                    smoothing: 0.5,
                },
            ))
            .id();
        let indicator = app
            .world_mut()
            .spawn(screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Entity(ship)),
                size: ScreenIndicatorSize::Fixed(Vec2::splat(32.0)),
                ..default()
            }))
            .id();

        let camera_start = app
            .world()
            .entity(camera)
            .get::<Transform>()
            .unwrap()
            .translation;

        // Warmup: the camera starts at the origin on top of the ship; let
        // the chase state converge to a sane trailing pose first.
        for _ in 0..3 {
            app.update();
        }

        let mut max_error = 0.0f32;
        for _ in 0..30 {
            app.update();
            // What this frame RENDERS with: the post-propagation poses.
            let world = app.world_mut();
            let camera_pose = *world.entity(camera).get::<GlobalTransform>().unwrap();
            let ship_pose = *world.entity(ship).get::<GlobalTransform>().unwrap();
            let camera_component = world.entity(camera).get::<Camera>().unwrap();
            let expected = camera_component
                .world_to_viewport(&camera_pose, ship_pose.translation())
                .expect("ship projects on screen");
            let (left, top, width, height) = node_rect(world, indicator);
            let placed = Vec2::new(left + width / 2.0, top + height / 2.0);
            max_error = max_error.max(placed.distance(expected));
        }
        // Delivery guards: the camera must genuinely trail (move) and the
        // rig must have kept the indicator visible, or the sub-pixel bound
        // is vacuous.
        let camera_moved = app
            .world()
            .entity(camera)
            .get::<Transform>()
            .unwrap()
            .translation
            .distance(camera_start);
        assert!(
            camera_moved > 10.0,
            "the chase camera must trail the cruising ship, moved {camera_moved}"
        );
        assert_eq!(
            *app.world().entity(indicator).get::<Visibility>().unwrap(),
            Visibility::Visible
        );
        assert!(
            max_error < 0.5,
            "indicator must sit on the rendered projection every frame, \
             worst mismatch {max_error} px"
        );
    }
}
