//! A reusable "world anchor -> UI node" indicator widget for the HUD.
//!
//! [`screen_indicator`] spawns an absolutely-positioned UI node that
//! [`ScreenIndicatorPlugin`] moves to its anchor's viewport projection every
//! frame: give it an entity to follow or a bare world point, and the widget
//! owns projection, visibility lifecycle, sizing, and off-screen policy, so
//! each HUD overlay is a thin consumer instead of a fresh copy of that loop.
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
use nova_gameplay::prelude::subtree_bounding_sphere;
use nova_ship::prelude::{BodyRadius, CameraAuthoritySystems};
use nova_ui::widget::metrics;

use crate::{
    hud_keep_out::HudKeepOuts,
    keybind_dock::{column_top_px, LaidOutDock},
    target_inset::TargetInsetHudMarker,
};

/// The screen-indicator spawners, its anchor, offset, size and offscreen components, and
/// `ScreenIndicatorPlugin` with `ScreenIndicatorSystems`.
pub mod prelude {
    pub use super::{
        screen_indicator, screen_indicator_layer, screen_indicator_node, ScreenIndicatorAnchor,
        ScreenIndicatorAnchorKind, ScreenIndicatorArrowMarker, ScreenIndicatorCamera,
        ScreenIndicatorClearance, ScreenIndicatorConfig, ScreenIndicatorMarker,
        ScreenIndicatorOffscreen, ScreenIndicatorOffset, ScreenIndicatorPlugin,
        ScreenIndicatorSize, ScreenIndicatorSystems, ScreenLabelType,
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
    /// for computed anchors (e.g. an autopilot GOTO point).
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
    /// Hug the node's own laid-out content: the indicator writes only its
    /// POSITION and leaves `width`/`height` to UI layout, centring on the size
    /// bevy_ui computed for it. This is what a text chip needs - a fixed box
    /// would either clip a long readout or pad a short one into a slab.
    ///
    /// Centring uses LAST frame's computed size (UI layout runs after this
    /// system), so a chip whose text just changed length is centred one frame
    /// late. That is invisible at readout cadence and is the price of hugging.
    Content,
    /// Track the anchor entity's on-screen extent (from the union of the
    /// NON-SENSOR collider AABBs of its subtree - trigger volumes are
    /// invisible and do not count as apparent size), never shrinking
    /// below `min_px`. `Point` anchors and entities without qualifying
    /// collider AABBs (e.g. a sensor-only beacon) fall back to `min_px`.
    ApparentSize {
        /// Minimum indicator size (px), also the fallback size.
        min_px: f32,
        /// Multiplier on the tracked extent: two
        /// indicators of the same anchor at scale 1.0 converge to the SAME
        /// pixel size on big/close targets and shimmer over each other; a
        /// distinct scale keeps them concentric rings. Applied to the
        /// apparent extent only - `min_px` stays the absolute floor.
        scale: f32,
    },
}

/// Offset (px) applied to the projected anchor point before the off-screen
/// test, so content can sit beside its anchor. Ignored while the anchor is
/// behind the camera (a clamped indicator hugs the edge regardless).
#[derive(Component, Debug, Clone, Copy, PartialEq, Deref, DerefMut, Reflect)]
pub struct ScreenIndicatorOffset(pub Vec2);

/// Stand the indicator clear of its anchor's own on-screen extent, instead of
/// at a fixed pixel offset from its centre.
///
/// A fixed offset is a promise about how big the thing underneath looks: the
/// 40 px that floats a marker above a cutter puts it amidships on a carrier and
/// on the mesh of a planetoid. This pushes the widget to the anchor's projected
/// EDGE plus a visual gap plus the widget's own reach along the push, so the
/// composition follows whatever the anchor currently looks like.
///
/// The extent is what the player SEES of the anchor: the union of its
/// subtree's non-sensor collider AABBs, or its [`BodyRadius`] when it has no
/// visible collider at all - a nav beacon's only collider is its trigger
/// sphere, and clearing THAT would park the chip kilometres off the orb.
///
/// Composes with [`ScreenIndicatorOffset`], which is still added on top: the
/// clearance owns the push away from the anchor, the offset owns any fixed
/// nudge the widget wants on top of it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
pub struct ScreenIndicatorClearance {
    /// Screen-space direction to push along (y down); normalized on use. A
    /// zero direction disables the clearance.
    pub direction: Vec2,
    /// Gap (px) between the anchor's projected edge and the widget's near
    /// edge - the visual breathing room, calibrated so a small target at
    /// normal range keeps the composition the fixed offset gave it.
    pub gap_px: f32,
    /// Floor (px) on the total push. A distant or unmeasured anchor projects
    /// to almost nothing, and a widget sitting on its dot reads as a lock
    /// marker rather than as a label; this is the composition it keeps.
    pub min_px: f32,
}

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
    ///
    /// A [`ScreenLabelType`] indicator then slides along the edge past any
    /// fixed HUD panel it would cover, and stands over the bottom-centre
    /// column (the key row and the notice slot) across the whole width; the
    /// arrow keeps pointing at the anchor.
    ClampToEdge {
        /// Inset (px) from the viewport edges the indicator is clamped to.
        margin_px: f32,
    },
}

/// A floating label: an indicator that never draws over a fixed HUD panel
/// ([`HudKeepOutPanel`](crate::hud_keep_out::HudKeepOutPanel)) nor over a label
/// of an earlier type.
///
/// Labels are fitted in declaration order, ties by entity: each label moves the
/// shortest way clear of the panels and of every label fitted before it, so a
/// later label gives way and an earlier one holds. A label moved on screen
/// shows no arrow: the arrow marks the edge clamp alone. Graphics that
/// frame their anchor (reticles, rings, ship markers) carry no label type and
/// never move.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Reflect)]
pub enum ScreenLabelType {
    /// The speed, mode, RCS and repair chips beside the player's ship. Their
    /// owner fits the stack as one box, so they hold and others give way.
    ShipReadout,
    /// A keycap cue (GOTO, ORBIT): the next input the player can press.
    InputCue,
    /// An objective marker chip.
    Objective,
    /// The chart waypoint chip.
    Waypoint,
    /// A maneuver readout: the destination ETA, the flip mark, the orbit radius.
    Maneuver,
    /// A beacon chip.
    Beacon,
    /// An edge pointer with its distance. Only takes part while pinned: on
    /// screen it draws nothing.
    EdgePointer,
    /// A cargo canister chip.
    Canister,
}

/// The least clearance (UI px) between a floating label and a fixed panel or
/// another label.
pub(crate) const LABEL_GAP_PX: f32 = metrics::GAP;

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
    trace!("screen_indicator: config {:?}", config);

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

/// Like [`screen_indicator`], but for an indicator that is ALSO a styled box
/// (a chip with a border, padding and children). Bevy refuses a bundle with two
/// `Node`s, so a caller that needs its own layout cannot simply add one next to
/// [`screen_indicator`]'s - it hands its node here instead and the projection
/// fields are stamped onto it.
pub fn screen_indicator_node(config: ScreenIndicatorConfig, mut node: Node) -> impl Bundle {
    trace!("screen_indicator_node: config {:?}", config);

    node.position_type = PositionType::Absolute;
    (
        ScreenIndicatorMarker,
        ScreenIndicatorAnchor(config.anchor),
        config.size,
        ScreenIndicatorOffset(config.offset),
        config.offscreen,
        node,
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
        trace!("ScreenIndicatorPlugin: build");

        // Projection must sample the SAME camera pose the frame renders
        // with. In Update the chase camera has not moved yet (the rig moves it
        // in PostUpdate), so indicators lagged the world by one frame of
        // camera motion - a visible HUD twitch. The slot is: after the LAST
        // camera writer, before UI layout consumes the node positions (bevy_ui
        // runs layout BEFORE transform propagation, so fresh poses are computed
        // via TransformHelper inside the system rather than read from
        // GlobalTransform).
        //
        // The last writer is `Override`, not the chase rig: a scripted pose - a
        // cinematic, photo mode, a capture script - overwrites the solved pose
        // afterwards, and indicators ordered against the rig alone projected
        // every one of those frames through a camera that was not the one
        // rendering them.
        app.configure_sets(
            PostUpdate,
            ScreenIndicatorSystems
                .after(CameraAuthoritySystems::Override)
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

/// Clamp `pos` to the viewport rect inset by `margin_px` (a negative margin is
/// zero). The indicator never leaves the viewport, and an inset wider than half
/// the viewport degenerates to the viewport center instead of panicking.
fn clamp_to_rect(pos: Vec2, viewport: Vec2, margin_px: f32) -> Vec2 {
    let margin = Vec2::splat(margin_px.max(0.0));
    let min = margin.min(viewport / 2.0);
    let max = (viewport - margin).max(viewport / 2.0);
    pos.clamp(min, max)
}

/// Clamp the center of a box of `size` so the WHOLE box stays inside the
/// viewport inset by `margin_px`. [`clamp_to_rect`] keeps only the center
/// inside, which is right for a point and wrong for anything as wide as a
/// labelled chip. Degenerate cases (a box or inset wider than the viewport)
/// collapse to the viewport center rather than panicking.
fn clamp_box_to_rect(center: Vec2, size: Vec2, viewport: Vec2, margin_px: f32) -> Vec2 {
    let margin = Vec2::splat(margin_px.max(0.0));
    let half = size.max(Vec2::ZERO) / 2.0;
    let min = (margin + half).min(viewport / 2.0);
    let max = (viewport - margin - half).max(viewport / 2.0);
    center.clamp(min, max)
}

/// The shift (px) that holds `min..max` inside `room_min..room_max`. An extent
/// wider than the room centres on it, like [`clamp_box_to_rect`].
fn hold_extent(min: f32, max: f32, room_min: f32, room_max: f32) -> f32 {
    if max - min > room_max - room_min {
        (room_min + room_max) / 2.0 - (min + max) / 2.0
    } else if min < room_min {
        room_min - min
    } else if max > room_max {
        room_max - max
    } else {
        0.0
    }
}

/// Where a `desired` box must move (UI px) to stay whole inside a `view` inset
/// by `margin_px` on both axes AND clear of every `keep_outs` rect grown by
/// `gap_px`. Returns the displacement to add to `desired`; zero when it
/// already fits.
///
/// The view wins: a box that cannot clear the keep-outs without leaving the
/// margin stays where the view holds it, overlap and all. Otherwise the
/// shortest path of at most [`FIT_STEPS`] axis moves is taken, each move just
/// clearing one rect the box still hits, and a tie takes the vertical move.
/// Stateless on purpose - the same inputs give the same answer, so a box on
/// the boundary cannot flicker between two states it remembers.
pub(crate) fn fit_box(
    desired: Rect,
    view: Vec2,
    margin_px: f32,
    keep_outs: &[Rect],
    gap_px: f32,
) -> Vec2 {
    let margin_px = margin_px.max(0.0);
    let (room_min, room_max) = (Vec2::splat(margin_px), view - margin_px);
    let held = Vec2::new(
        hold_extent(desired.min.x, desired.max.x, room_min.x, room_max.x),
        hold_extent(desired.min.y, desired.max.y, room_min.y, room_max.y),
    );
    let box_ = Rect {
        min: desired.min + held,
        max: desired.max + held,
    };
    let grown: Vec<Rect> = keep_outs
        .iter()
        .map(|rect| rect.inflate(gap_px.max(0.0)))
        .collect();
    let at = |shift: Vec2| Rect {
        min: box_.min + shift,
        max: box_.max + shift,
    };
    let overlaps = |a: Rect, b: &Rect| {
        a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
    };
    // Float slack so a move that lands exactly on the margin stays legal.
    const EDGE_SLACK_PX: f32 = 1e-3;
    let in_room = |rect: Rect| {
        rect.min.x >= room_min.x - EDGE_SLACK_PX
            && rect.min.y >= room_min.y - EDGE_SLACK_PX
            && rect.max.x <= room_max.x + EDGE_SLACK_PX
            && rect.max.y <= room_max.y + EDGE_SLACK_PX
    };

    let mut best: Option<Vec2> = None;
    let mut frontier = vec![Vec2::ZERO];
    for _ in 0..=FIT_STEPS {
        let mut next = Vec::new();
        for shift in frontier {
            if best.is_some_and(|best| shift.length() >= best.length()) {
                continue;
            }
            let moved = at(shift);
            let mut hit = grown.iter().filter(|rect| overlaps(moved, rect)).peekable();
            if hit.peek().is_none() {
                best = Some(shift);
                continue;
            }
            for rect in hit {
                // Vertical moves first, so the strict `<` above leaves a tie
                // to them.
                for step in [
                    Vec2::new(0.0, rect.min.y - moved.max.y),
                    Vec2::new(0.0, rect.max.y - moved.min.y),
                    Vec2::new(rect.min.x - moved.max.x, 0.0),
                    Vec2::new(rect.max.x - moved.min.x, 0.0),
                ] {
                    if in_room(at(shift + step)) {
                        next.push(shift + step);
                    }
                }
            }
        }
        frontier = next;
    }
    held + best.unwrap_or(Vec2::ZERO)
}

/// The most axis moves [`fit_box`] chains to clear its keep-outs. Three clears
/// a box wedged in a corner between two panels and a label; the HUD has a few
/// panels and labels, so the search stays small.
const FIT_STEPS: usize = 3;

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

/// A camera's projection in UI px: the px a [`Node`]'s `Val::Px` counts.
///
/// [`Camera::world_to_viewport`] and [`Camera::logical_viewport_size`] answer
/// in window logical px. The UI multiplies every `Val::Px` by [`UiScale`], so
/// a window px is `1 / UiScale` UI px. Every indicator converts here, once;
/// without it a 150% interface draws each indicator half as far again from
/// the view's corner as its anchor.
#[derive(Clone, Copy)]
pub(crate) struct UiProjection<'a> {
    camera: &'a Camera,
    pose: &'a GlobalTransform,
    ui_scale: f32,
}

impl<'a> UiProjection<'a> {
    /// The projection of `camera` at `pose` for a UI drawn at `ui_scale`.
    pub(crate) fn new(camera: &'a Camera, pose: &'a GlobalTransform, ui_scale: &UiScale) -> Self {
        Self {
            camera,
            pose,
            ui_scale: ui_scale.0,
        }
    }

    /// The size of the view, or `None` while the camera has no target.
    pub(crate) fn view_size(&self) -> Option<Vec2> {
        self.camera
            .logical_viewport_size()
            .map(|size| size / self.ui_scale)
    }

    /// Where `world` lands on the view, or `None` when it does not project
    /// (behind the camera).
    pub(crate) fn point(&self, world: Vec3) -> Option<Vec2> {
        self.camera
            .world_to_viewport(self.pose, world)
            .ok()
            .map(|point| point / self.ui_scale)
    }

    /// The camera's right vector in the world.
    pub(crate) fn right(&self) -> Dir3 {
        self.pose.right()
    }
}

/// The on-screen RADIUS (px) of a world-space bounding radius about
/// `anchor_pos`: project a point one radius toward the camera's right and read
/// the pixel distance from the anchor's own projected point. Only the anchor
/// centre has to project, which keeps this robust when a close target's far
/// corners fall behind the camera. `None` when the offset point does not
/// project.
fn projected_radius_px(
    world_radius: f32,
    anchor_pos: Vec3,
    center: Vec2,
    projection: UiProjection,
) -> Option<f32> {
    let edge = projection.point(anchor_pos + projection.right() * world_radius)?;
    Some(center.distance(edge))
}

/// The world-space bounding radius of what the player SEES of `entity`: the
/// radius of its subtree's solid-collider bounding sphere
/// ([`subtree_bounding_sphere`]), and failing that its authored
/// [`BodyRadius`].
///
/// The fallback is what makes a nav beacon work: its only collider is the
/// trigger sphere the walk deliberately skips, and its published body radius
/// is the orb the chip has to clear. `None` when neither is available - an
/// anchor that has not spawned its colliders yet keeps whatever floor the
/// consumer authored.
fn anchor_world_radius(
    entity: Entity,
    q_children: &Query<&Children>,
    q_aabb: &Query<&ColliderAabb, Without<Sensor>>,
    q_body_radius: &Query<&BodyRadius>,
) -> Option<f32> {
    subtree_bounding_sphere(entity, q_children, q_aabb)
        .map(|(_, radius)| radius)
        .or_else(|| q_body_radius.get(entity).ok().map(|radius| **radius))
}

/// The push (px) that stands a `size` widget clear of its anchor: out to the
/// anchor's projected edge, plus the authored visual gap, plus the widget's own
/// reach along the push direction - never less than the authored floor.
///
/// The widget's reach is the support of its box along the direction, so a wide
/// chip pushed up clears by its half-height and the same chip pushed sideways
/// clears by its half-width.
fn clearance_offset(
    clearance: &ScreenIndicatorClearance,
    size: Vec2,
    world_radius: Option<f32>,
    anchor_pos: Vec3,
    projected: Option<Vec2>,
    projection: UiProjection,
) -> Vec2 {
    let Some(direction) = clearance.direction.try_normalize() else {
        return Vec2::ZERO;
    };
    let edge = match (world_radius, projected) {
        (Some(radius), Some(center)) => {
            projected_radius_px(radius, anchor_pos, center, projection).unwrap_or(0.0)
        }
        _ => 0.0,
    };
    let reach = (direction.x.abs() * size.x + direction.y.abs() * size.y) * 0.5;
    direction * (edge + clearance.gap_px + reach).max(clearance.min_px)
}

/// Indicator size (px) for the frame. `ApparentSize` measures the anchor
/// entity's on-screen extent: take the world bounding-sphere radius from the
/// subtree's collider AABBs, project a point one radius to the camera's
/// right, and read the pixel distance from the projected center - so the
/// size grows with apparent size and shrinks with distance. Only the center
/// must project, which keeps this robust when a close target's far corners
/// fall behind the camera. Falls back to `min_px` for `Point` anchors, for
/// entities without NON-SENSOR collider AABBs (the frame they spawned, or
/// a sensor-only body like a locked beacon), and when the anchor itself
/// did not project.
#[expect(
    clippy::too_many_arguments,
    reason = "one projection reading camera, anchor and node"
)]
fn indicator_size(
    computed: &ComputedNode,
    size_mode: ScreenIndicatorSize,
    anchor_entity: Option<Entity>,
    anchor_pos: Vec3,
    projected: Option<Vec2>,
    projection: UiProjection,
    q_children: &Query<&Children>,
    q_aabb: &Query<&ColliderAabb, Without<Sensor>>,
) -> Vec2 {
    match size_mode {
        ScreenIndicatorSize::Fixed(size) => size,
        // Logical px: `ComputedNode::size` is PHYSICAL, and left/top are Val::Px.
        ScreenIndicatorSize::Content => computed.size() * computed.inverse_scale_factor(),
        ScreenIndicatorSize::ApparentSize { min_px, scale } => {
            let fallback = Vec2::splat(min_px);
            let (Some(entity), Some(center)) = (anchor_entity, projected) else {
                return fallback;
            };
            let Some((_, radius)) = subtree_bounding_sphere(entity, q_children, q_aabb) else {
                return fallback;
            };
            projected_radius_px(radius * scale, anchor_pos, center, projection)
                .map(|px| Vec2::splat((2.0 * px).max(min_px)))
                .unwrap_or(fallback)
        }
    }
}

/// Project every indicator to its anchor: resolve the anchor (a missing one
/// hides the node), project through the [`ScreenIndicatorCamera`], size the
/// node, apply the pixel offset, and apply the off-screen policy including
/// the direction arrow. Then fit every [`ScreenLabelType`] label clear of the
/// fixed panels and of the labels fitted before it.
#[expect(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "one query term per anchor placement input"
)]
fn update_screen_indicators(
    mut q_indicator: Query<
        (
            Entity,
            &ScreenIndicatorAnchor,
            &ScreenIndicatorSize,
            &ScreenIndicatorOffset,
            Option<&ScreenIndicatorClearance>,
            &ScreenIndicatorOffscreen,
            Option<&ScreenLabelType>,
            &ComputedNode,
            &UiGlobalTransform,
            &mut Node,
            &mut Visibility,
        ),
        (With<ScreenIndicatorMarker>, Without<TargetInsetHudMarker>),
    >,
    // This runs BEFORE this frame's transform propagation (UI layout comes
    // first in PostUpdate), so `GlobalTransform` still holds last frame's
    // poses. TransformHelper composes fresh `Transform`s instead: the
    // camera pose the chase camera just wrote and the eased anchor poses
    // the frame will render with. HUD-scale anchor counts make the
    // per-entity hierarchy walk negligible.
    transform_helper: TransformHelper,
    q_children: Query<&Children>,
    q_aabb: Query<&ColliderAabb, Without<Sensor>>,
    q_body_radius: Query<&BodyRadius>,
    q_nested: Query<(), With<ScreenIndicatorMarker>>,
    q_drawn: Query<
        (&ComputedNode, &UiGlobalTransform, &InheritedVisibility),
        Without<ScreenIndicatorMarker>,
    >,
    mut q_arrow: Query<
        (&mut UiTransform, &mut Visibility),
        (
            With<ScreenIndicatorArrowMarker>,
            Without<ScreenIndicatorMarker>,
            Without<TargetInsetHudMarker>,
        ),
    >,
    q_camera: Query<(Entity, &Camera), With<ScreenIndicatorCamera>>,
    ui_scale: Res<UiScale>,
    keep_outs: HudKeepOuts,
    q_dock: LaidOutDock,
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
    let view = camera.and_then(|(camera_entity, camera)| {
        let camera_transform = transform_helper
            .compute_global_transform(camera_entity)
            .ok()?;
        let viewport = UiProjection::new(camera, &camera_transform, &ui_scale).view_size()?;
        Some((camera, camera_transform, viewport))
    });
    let Some((camera, camera_transform, viewport)) = view else {
        for (.., mut visibility) in &mut q_indicator {
            visibility.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let camera_transform = &camera_transform;
    let projection = UiProjection::new(camera, camera_transform, &ui_scale);

    // A label is written after every label of an earlier type is placed.
    let mut labels: Vec<PlacedLabel> = Vec::new();
    for (
        entity,
        anchor,
        size_mode,
        offset,
        clearance,
        offscreen,
        label,
        computed,
        drawn,
        mut node,
        mut visibility,
    ) in &mut q_indicator
    {
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
        let projected = projection.point(anchor_pos);
        let view_pos = camera_transform
            .affine()
            .inverse()
            .transform_point3(anchor_pos);

        // Sized BEFORE placement: a clearance offset stands the widget's own
        // near edge off the anchor, so the push cannot be known until the box
        // is.
        let size = indicator_size(
            computed,
            *size_mode,
            anchor_entity,
            anchor_pos,
            projected,
            projection,
            &q_children,
            &q_aabb,
        );
        let offset = **offset
            + clearance.map_or(Vec2::ZERO, |clearance| {
                clearance_offset(
                    clearance,
                    size,
                    anchor_entity.and_then(|entity| {
                        anchor_world_radius(entity, &q_children, &q_aabb, &q_body_radius)
                    }),
                    anchor_pos,
                    projected,
                    projection,
                )
            });

        let placement = place(
            viewport,
            projected.map(|pos| pos + offset),
            view_pos,
            *offscreen,
        );
        let Placement::Visible { center, arrow } = placement else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };

        // `Content` leaves the box to UI layout - writing width/height would
        // freeze the chip at last frame's size and it could never grow back.
        if !matches!(size_mode, ScreenIndicatorSize::Content) {
            node.width = Val::Px(size.x);
            node.height = Val::Px(size.y);
        }
        // Re-clamp a CLAMPED indicator now that its box is known. `place`
        // clamps the centre before anything has measured the node, and the
        // corner below then subtracts half the width - so a 180px chip clamped
        // to x = 20 draws from x = -70 and loses its first characters (the
        // beacon chip read `VEY 7.87 km` in any frame whose beacon was off past
        // a corner). A `Content` node's size is last frame's measurement, so a
        // chip that changes width is off by that change for exactly one frame.
        // A `Content` chip is held whole in view on screen too: its anchor can
        // stand nearer the edge than half the chip's width. A sized box is
        // not, since it must stay on the anchor it frames.
        let margin_px = match *offscreen {
            ScreenIndicatorOffscreen::ClampToEdge { margin_px } => margin_px,
            ScreenIndicatorOffscreen::Hide => 0.0,
        };
        let center = match *offscreen {
            ScreenIndicatorOffscreen::ClampToEdge { margin_px }
                if arrow.is_some() || matches!(size_mode, ScreenIndicatorSize::Content) =>
            {
                clamp_box_to_rect(center, size, viewport, margin_px)
            }
            _ => center,
        };
        visibility.set_if_neq(Visibility::Visible);
        match label {
            // An edge pointer on screen draws nothing, so it neither moves nor
            // claims room.
            Some(&kind) if kind != ScreenLabelType::EdgePointer || arrow.is_some() => {
                labels.push(PlacedLabel {
                    kind,
                    entity,
                    center,
                    size,
                    drawn: drawn_extent(entity, size, computed, drawn, &q_children, &q_drawn),
                    margin_px,
                    arrow,
                });
            }
            _ => {
                node.left = Val::Px(center.x - size.x / 2.0);
                node.top = Val::Px(center.y - size.y / 2.0);
                update_arrows(entity, arrow, &q_children, &q_nested, &mut q_arrow);
            }
        }
    }

    labels.sort_by_key(|label| (label.kind, label.entity));
    let mut taken = keep_outs.rects(viewport);
    // A pinned label stands over the whole bottom-centre column, shown notice
    // or not, so it does not hop when a notice comes.
    let column =
        column_top_px(&q_dock).map(|top| Rect::new(0.0, viewport.y - top, viewport.x, viewport.y));
    for label in labels {
        let drawn = Rect {
            min: label.center + label.drawn.min,
            max: label.center + label.drawn.max,
        };
        let shift = match (label.arrow, column) {
            (Some(_), Some(column)) => {
                let keep: Vec<Rect> = taken.iter().copied().chain([column]).collect();
                fit_box(drawn, viewport, label.margin_px, &keep, LABEL_GAP_PX)
            }
            _ => fit_box(drawn, viewport, label.margin_px, &taken, LABEL_GAP_PX),
        };
        let center = label.center + shift;
        taken.push(Rect {
            min: drawn.min + shift,
            max: drawn.max + shift,
        });
        if let Ok((.., mut node, _)) = q_indicator.get_mut(label.entity) {
            node.left = Val::Px(center.x - label.size.x / 2.0);
            node.top = Val::Px(center.y - label.size.y / 2.0);
        }
        // The arrow stays the edge clamp's alone: a label moved on screen
        // shows none.
        update_arrows(
            label.entity,
            label.arrow,
            &q_children,
            &q_nested,
            &mut q_arrow,
        );
    }
}

/// A label placed from its anchor and waiting for its fit.
struct PlacedLabel {
    kind: ScreenLabelType,
    entity: Entity,
    /// Box centre (UI px) before the fit.
    center: Vec2,
    /// Box size (UI px).
    size: Vec2,
    /// What the label draws, relative to its box centre: the box and every
    /// shown descendant that hangs outside it (an arrow, a distance line).
    drawn: Rect,
    margin_px: f32,
    arrow: Option<Vec2>,
}

/// The extent a label draws, relative to its box centre: its own box of `size`
/// grown by every shown descendant's box from the last layout. Children hang
/// at fixed offsets from the box, so last frame's offsets hold for this frame.
/// Nested indicators place themselves and are left out.
fn drawn_extent(
    entity: Entity,
    size: Vec2,
    computed: &ComputedNode,
    transform: &UiGlobalTransform,
    q_children: &Query<&Children>,
    q_drawn: &Query<
        (&ComputedNode, &UiGlobalTransform, &InheritedVisibility),
        Without<ScreenIndicatorMarker>,
    >,
) -> Rect {
    let scale = computed.inverse_scale_factor();
    let origin = transform.translation * scale;
    let half = size / 2.0;
    let mut extent = Rect {
        min: -half,
        max: half,
    };
    let mut stack: Vec<Entity> = q_children
        .get(entity)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    while let Some(current) = stack.pop() {
        let Ok((child, child_transform, visible)) = q_drawn.get(current) else {
            continue;
        };
        if !visible.get() {
            continue;
        }
        let size = child.size() * child.inverse_scale_factor();
        if size.min_element() > 0.0 {
            let centre = child_transform.translation * child.inverse_scale_factor() - origin;
            extent = extent.union(Rect::from_center_size(centre, size));
        }
        if let Ok(children) = q_children.get(current) {
            stack.extend(children.iter());
        }
    }
    extent
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
            Without<TargetInsetHudMarker>,
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
        ecs::system::RunSystemOnce,
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
    fn clamp_box_to_rect_keeps_the_whole_box_inside() {
        let viewport = Vec2::new(800.0, 600.0);
        let chip = Vec2::new(180.0, 30.0);
        // The clamped centre leaves room for the half-width: at the plain
        // point clamp (x = 780) the chip would draw out to x = 870.
        assert_eq!(
            clamp_box_to_rect(Vec2::new(900.0, 300.0), chip, viewport, 20.0),
            Vec2::new(690.0, 300.0)
        );
        assert_eq!(
            clamp_box_to_rect(Vec2::new(-50.0, -50.0), chip, viewport, 20.0),
            Vec2::new(110.0, 35.0)
        );
        // Inside with room to spare: untouched.
        assert_eq!(
            clamp_box_to_rect(Vec2::new(400.0, 300.0), chip, viewport, 20.0),
            Vec2::new(400.0, 300.0)
        );
    }

    #[test]
    fn a_box_is_held_inside_the_view_on_both_axes() {
        let view = Vec2::new(800.0, 600.0);
        let rect = |x: f32, y: f32, w: f32, h: f32| Rect::new(x, y, x + w, y + h);

        // Already inside the margin: no move.
        assert_eq!(
            fit_box(rect(100.0, 100.0, 80.0, 30.0), view, 12.0, &[], 12.0),
            Vec2::ZERO
        );
        // Past the right and bottom edges: pulled back to the margin on both.
        assert_eq!(
            fit_box(rect(760.0, 580.0, 80.0, 30.0), view, 12.0, &[], 12.0),
            Vec2::new(-52.0, -22.0)
        );
        // Past the left and top edges: pushed in to the margin on both.
        assert_eq!(
            fit_box(rect(-30.0, -10.0, 80.0, 30.0), view, 12.0, &[], 12.0),
            Vec2::new(42.0, 22.0)
        );
        // Wider than the room: centred on that axis, not clamped to one side.
        assert_eq!(
            fit_box(rect(0.0, 100.0, 900.0, 30.0), view, 12.0, &[], 12.0),
            Vec2::new(-50.0, 0.0)
        );
    }

    #[test]
    fn a_box_over_a_keep_out_takes_the_shortest_move_that_stays_in_view() {
        let view = Vec2::new(800.0, 600.0);
        let rect = |x: f32, y: f32, w: f32, h: f32| Rect::new(x, y, x + w, y + h);
        let inset = rect(544.0, 12.0, 244.0, 244.0);

        // The grown keep-out is (532, 0)-(800, 268). A box hanging 20 px into
        // it clears downward (to y = 268) cheaper than leftward.
        let desired = rect(600.0, 248.0, 100.0, 60.0);
        let shift = fit_box(desired, view, 12.0, &[inset], 12.0);
        assert_eq!(shift, Vec2::new(0.0, 20.0));

        // Deep in the keep-out's corner the left move is the shorter one.
        let desired = rect(500.0, 100.0, 100.0, 60.0);
        let shift = fit_box(desired, view, 12.0, &[inset], 12.0);
        assert_eq!(shift, Vec2::new(-68.0, 0.0));

        // Down would leave the view: the box moves up instead of off-screen,
        // and with no legal move at all the view keeps its hold.
        let near_floor = rect(560.0, 300.0, 100.0, 60.0);
        let low_view = Vec2::new(800.0, 372.0);
        assert_eq!(
            fit_box(
                near_floor,
                low_view,
                12.0,
                &[rect(500.0, 280.0, 200.0, 60.0)],
                0.0
            ),
            Vec2::new(0.0, -80.0),
            "the up move clears the keep-out and stays in view"
        );
        let squeezed = rect(560.0, 130.0, 100.0, 60.0);
        let tight_view = Vec2::new(800.0, 184.0);
        assert_eq!(
            fit_box(
                squeezed,
                tight_view,
                12.0,
                &[rect(0.0, 0.0, 800.0, 184.0)],
                0.0
            ),
            Vec2::new(0.0, -18.0),
            "nothing clears the keep-out inside the view, so only the hold applies"
        );

        // A tie between a vertical and a horizontal move takes the vertical.
        let tie = fit_box(
            rect(300.0, 300.0, 100.0, 100.0),
            view,
            12.0,
            &[rect(250.0, 250.0, 200.0, 200.0)],
            0.0,
        );
        assert_eq!(tie, Vec2::new(0.0, -150.0));
    }

    #[test]
    fn a_box_between_two_keep_outs_clears_both_by_the_shortest_path() {
        let view = Vec2::new(800.0, 600.0);
        let rect = |x: f32, y: f32, w: f32, h: f32| Rect::new(x, y, x + w, y + h);
        // A panel under the box and a placed label just above it: the single
        // up move off the panel lands on the label, so the fit chains a
        // second up move past the label, shorter than any sideways path.
        let panel = rect(0.0, 400.0, 400.0, 200.0);
        let label = rect(100.0, 340.0, 120.0, 40.0);
        let desired = rect(120.0, 390.0, 100.0, 30.0);
        let shift = fit_box(desired, view, 12.0, &[panel, label], 8.0);
        let moved = Rect {
            min: desired.min + shift,
            max: desired.max + shift,
        };
        for keep_out in [panel, label] {
            assert!(
                moved.intersect(keep_out.inflate(8.0)).is_empty(),
                "{moved:?} still meets {keep_out:?}"
            );
        }
        assert_eq!(shift, Vec2::new(0.0, -88.0));
    }

    #[test]
    fn clamp_box_to_rect_degenerate_box_centers() {
        // A box wider than the viewport must not panic (f32::clamp panics on
        // min > max); it degenerates to the center.
        let viewport = Vec2::new(100.0, 100.0);
        assert_eq!(
            clamp_box_to_rect(Vec2::ZERO, Vec2::splat(400.0), viewport, 10.0),
            Vec2::new(50.0, 50.0)
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

    // -- whole-system behavior against a fabricated camera --

    /// A camera whose computed values are filled in by hand, since no render
    /// backend runs in tests: 90 degree vertical FOV, 800x600 viewport,
    /// identity transform (at the origin looking down -Z).
    fn test_camera() -> (Transform, Camera) {
        let camera = Camera {
            computed: ComputedCameraValues {
                clip_from_view: bevy::math::proj::perspective_infinite_reverse(
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

    /// The rig's view: the test camera and a UI at 100%.
    fn spawn_camera(world: &mut World) {
        let (transform, camera) = test_camera();
        world.spawn((transform, camera, ScreenIndicatorCamera));
        world.init_resource::<UiScale>();
    }

    fn node_rect(world: &World, entity: Entity) -> (f32, f32, f32, f32) {
        let node = world.entity(entity).get::<Node>().expect("node exists");
        let px = |val: Val| match val {
            Val::Px(px) => px,
            other => panic!("expected Val::Px, got {other:?}"),
        };
        (px(node.left), px(node.top), px(node.width), px(node.height))
    }

    /// The camera answers in window px and the UI multiplies `Val::Px` by the
    /// interface scale. At 150% an indicator is written at two thirds of its
    /// window position, or it draws half as far again from the view's corner
    /// as its anchor.
    #[test]
    fn an_indicator_stays_on_its_anchor_at_a_larger_interface_scale() {
        let mut world = World::new();
        spawn_camera(&mut world);
        world.resource_mut::<UiScale>().0 = 1.5;
        let indicator = world
            .spawn(screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Point(Vec3::new(0.0, 0.0, -10.0))),
                size: ScreenIndicatorSize::Fixed(Vec2::splat(30.0)),
                ..default()
            }))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();

        // The anchor is at the window centre, (400, 300) window px.
        let (left, top, width, height) = node_rect(&world, indicator);
        let centre = Vec2::new(left + width / 2.0, top + height / 2.0);
        assert!(
            centre.distance(Vec2::new(400.0, 300.0) / 1.5) < 0.01,
            "the indicator centre is {centre} UI px"
        );
    }

    /// A shown fixed panel standing at `centre` with `size` (UI px), as the
    /// last layout left it.
    fn spawn_panel(world: &mut World, centre: Vec2, size: Vec2) -> Rect {
        world.spawn((
            crate::hud_keep_out::HudKeepOutPanel,
            ComputedNode {
                size,
                ..ComputedNode::DEFAULT
            },
            UiGlobalTransform::from_translation(centre),
            InheritedVisibility::VISIBLE,
        ));
        Rect::from_center_size(centre, size)
    }

    /// A 60x20 label of `kind` on a point anchor, with a hidden arrow child.
    fn spawn_label(world: &mut World, kind: ScreenLabelType, anchor: Vec3) -> (Entity, Entity) {
        let arrow = world
            .spawn((
                ScreenIndicatorArrowMarker,
                UiTransform::default(),
                Visibility::Hidden,
            ))
            .id();
        let label = world
            .spawn((
                kind,
                screen_indicator(ScreenIndicatorConfig {
                    anchor: Some(ScreenIndicatorAnchorKind::Point(anchor)),
                    size: ScreenIndicatorSize::Fixed(Vec2::new(60.0, 20.0)),
                    ..default()
                }),
            ))
            .add_child(arrow)
            .id();
        (label, arrow)
    }

    fn label_rect(world: &World, label: Entity) -> Rect {
        let (left, top, width, height) = node_rect(world, label);
        Rect::new(left, top, left + width, top + height)
    }

    /// A label whose anchor stands under a fixed panel moves clear of it by
    /// the label gap, and shows no edge arrow, since it is not clamped. The
    /// test camera puts `(x, y, -10)` at `(400 + 30x, 300 - 30y)` px.
    #[test]
    fn a_label_over_a_panel_moves_clear_without_an_edge_arrow() {
        let mut world = World::new();
        spawn_camera(&mut world);
        let panel = spawn_panel(&mut world, Vec2::new(200.0, 450.0), Vec2::new(300.0, 100.0));
        let (label, arrow) = spawn_label(
            &mut world,
            ScreenLabelType::Objective,
            Vec3::new(-20.0 / 3.0, -5.0, -10.0),
        );

        world.run_system_once(update_screen_indicators).unwrap();

        let rect = label_rect(&world, label);
        assert!(
            rect.intersect(panel.inflate(LABEL_GAP_PX - 0.01))
                .is_empty(),
            "the label {rect:?} draws over the panel {panel:?}"
        );
        assert_eq!(
            *world.entity(arrow).get::<Visibility>().unwrap(),
            Visibility::Hidden,
            "a label moved on screen shows no edge arrow"
        );
    }

    /// Two labels on one anchor: the earlier type holds its anchor and the
    /// later one gives way, clear of it by the label gap. Neither is clamped,
    /// so neither shows its edge arrow.
    #[test]
    fn a_later_label_gives_way_to_an_earlier_one() {
        let mut world = World::new();
        spawn_camera(&mut world);
        let anchor = Vec3::new(0.0, 0.0, -10.0);
        let (beacon, beacon_arrow) = spawn_label(&mut world, ScreenLabelType::Beacon, anchor);
        let (objective, objective_arrow) =
            spawn_label(&mut world, ScreenLabelType::Objective, anchor);

        world.run_system_once(update_screen_indicators).unwrap();

        let (held, moved) = (label_rect(&world, objective), label_rect(&world, beacon));
        assert_eq!(
            held.center(),
            Vec2::new(400.0, 300.0),
            "the objective holds"
        );
        assert!(
            moved
                .intersect(held.inflate(LABEL_GAP_PX - 0.01))
                .is_empty(),
            "the beacon {moved:?} draws over the objective {held:?}"
        );
        for arrow in [beacon_arrow, objective_arrow] {
            assert_eq!(
                *world.entity(arrow).get::<Visibility>().unwrap(),
                Visibility::Hidden,
                "an on-screen label shows no edge arrow"
            );
        }
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

    /// `Content` sizing leaves the BOX to UI layout and only positions it:
    /// `width`/`height` stay untouched (a written `Val::Px` would freeze the
    /// chip at one frame's size and it could never grow back), and the node is
    /// centred on the size layout computed for it. Every restyled HUD chip
    /// rides this - a regression here squashes all of them at once, and only a
    /// screenshot would show it.
    #[test]
    fn content_sizing_positions_without_writing_the_box() {
        let mut world = World::new();
        spawn_camera(&mut world);
        let indicator = world
            .spawn((
                screen_indicator_node(
                    ScreenIndicatorConfig {
                        anchor: Some(ScreenIndicatorAnchorKind::Point(Vec3::new(0.0, 0.0, -10.0))),
                        size: ScreenIndicatorSize::Content,
                        ..default()
                    },
                    Node::default(),
                ),
                // Stand in for what UI layout would have measured last frame.
                ComputedNode {
                    size: Vec2::new(60.0, 20.0),
                    ..ComputedNode::DEFAULT
                },
            ))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();

        let node = world.entity(indicator).get::<Node>().unwrap();
        assert_eq!(node.width, Val::Auto, "the box is UI layout's to decide");
        assert_eq!(node.height, Val::Auto);
        // Centred on the computed size: (400,300) - (60,20)/2.
        let px = |val: Val| match val {
            Val::Px(px) => px,
            other => panic!("expected Val::Px, got {other:?}"),
        };
        assert!((px(node.left) - 370.0).abs() < 0.5, "left {:?}", node.left);
        assert!((px(node.top) - 290.0).abs() < 0.5, "top {:?}", node.top);
        assert_eq!(
            *world.entity(indicator).get::<Visibility>().unwrap(),
            Visibility::Visible
        );
    }

    #[test]
    fn a_content_chip_on_an_anchor_near_the_edge_stays_whole_in_view() {
        let mut world = World::new();
        spawn_camera(&mut world);
        // The anchor projects on screen, about 28 px from the left edge: a
        // 180 px chip centred on it would start at x = -62.
        let indicator = world
            .spawn((
                screen_indicator_node(
                    ScreenIndicatorConfig {
                        anchor: Some(ScreenIndicatorAnchorKind::Point(Vec3::new(
                            -12.4, 0.0, -10.0,
                        ))),
                        size: ScreenIndicatorSize::Content,
                        offscreen: ScreenIndicatorOffscreen::ClampToEdge { margin_px: 20.0 },
                        ..default()
                    },
                    Node::default(),
                ),
                ComputedNode {
                    size: Vec2::new(180.0, 30.0),
                    ..ComputedNode::DEFAULT
                },
            ))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();

        let node = world.entity(indicator).get::<Node>().unwrap();
        assert_eq!(
            node.left,
            Val::Px(20.0),
            "the chip starts at the margin, not off the edge"
        );
        assert_eq!(
            node.top,
            Val::Px(285.0),
            "the chip keeps the anchor's height"
        );
        assert_eq!(
            *world.entity(indicator).get::<Visibility>().unwrap(),
            Visibility::Visible
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
        // (rotated by pi from the up-pointing art). The 32px box sits fully
        // inside the 20px margin - its BOTTOM edge lands on 580, not its
        // centre, which is what `clamp_box_to_rect` buys.
        let (left, top, _, _) = node_rect(&world, clamper);
        assert!((left - 384.0).abs() < 0.5, "left {left}");
        assert!((top - (580.0 - 32.0)).abs() < 0.5, "top {top}");
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
                size: ScreenIndicatorSize::ApparentSize {
                    min_px: 32.0,
                    scale: 1.0,
                },
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
                size: ScreenIndicatorSize::ApparentSize {
                    min_px: 32.0,
                    scale: 1.0,
                },
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

    /// A clearance pushes the widget past the anchor's own projected edge, so
    /// a chip authored against a small contact does not end up amidships on a
    /// big one. The floor is what keeps the small-contact composition.
    #[test]
    fn clearance_stands_a_widget_off_the_anchors_projected_edge() {
        const SIZE: Vec2 = Vec2::new(20.0, 10.0);
        const CLEARANCE: ScreenIndicatorClearance = ScreenIndicatorClearance {
            direction: Vec2::NEG_Y,
            gap_px: 12.0,
            min_px: 40.0,
        };

        let push_for = |half_extent: f32| -> f32 {
            let mut world = World::new();
            spawn_camera(&mut world);
            let target = world
                .spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.0, -10.0)),
                    ColliderAabb::from_min_max(
                        Vec3::new(-half_extent, -half_extent, -10.0 - half_extent),
                        Vec3::new(half_extent, half_extent, -10.0 + half_extent),
                    ),
                ))
                .id();
            let indicator = world
                .spawn((
                    screen_indicator(ScreenIndicatorConfig {
                        anchor: Some(ScreenIndicatorAnchorKind::Entity(target)),
                        size: ScreenIndicatorSize::Fixed(SIZE),
                        ..default()
                    }),
                    CLEARANCE,
                ))
                .id();

            world.run_system_once(update_screen_indicators).unwrap();

            // The viewport centre is where the anchor itself projects, so the
            // push is how far the widget's centre sits above it.
            let (_, top, _, height) = node_rect(&world, indicator);
            300.0 - (top + height / 2.0)
        };

        // A small contact keeps the authored composition exactly.
        assert!(
            (push_for(0.05) - CLEARANCE.min_px).abs() < 0.5,
            "a small contact must keep the authored 40 px, got {}",
            push_for(0.05)
        );

        // A big one is cleared: past its projected edge, plus the gap, plus the
        // widget's own half-height.
        let radius = (3.0_f32).sqrt() * 2.0;
        let edge = 400.0 * radius / (10.0 * (800.0 / 600.0));
        let expected = edge + CLEARANCE.gap_px + SIZE.y / 2.0;
        let big = push_for(2.0);
        assert!(
            (big - expected).abs() < 1.0,
            "a big contact must be cleared: push {big}, expected {expected}"
        );
        assert!(big > CLEARANCE.min_px * 2.0, "the push must actually grow");
    }

    /// A nav beacon's only collider is its trigger sphere, which the apparent
    /// extent deliberately skips. The clearance falls back to the authored
    /// `BodyRadius`, so the chip clears the ORB - a 50 m marker used to wear
    /// its own label.
    #[test]
    fn clearance_falls_back_to_the_authored_body_radius() {
        const CLEARANCE: ScreenIndicatorClearance = ScreenIndicatorClearance {
            direction: Vec2::NEG_Y,
            gap_px: 12.0,
            min_px: 28.0,
        };

        let mut world = World::new();
        spawn_camera(&mut world);
        let beacon = world
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, -10.0)),
                // Sensor-only: the trigger volume is not apparent size.
                Sensor,
                ColliderAabb::from_min_max(Vec3::new(-8.0, -8.0, -18.0), Vec3::new(8.0, 8.0, -2.0)),
                BodyRadius(5.0),
            ))
            .id();
        let indicator = world
            .spawn((
                screen_indicator(ScreenIndicatorConfig {
                    anchor: Some(ScreenIndicatorAnchorKind::Entity(beacon)),
                    size: ScreenIndicatorSize::Fixed(Vec2::new(40.0, 16.0)),
                    ..default()
                }),
                CLEARANCE,
            ))
            .id();

        world.run_system_once(update_screen_indicators).unwrap();

        let (_, top, _, height) = node_rect(&world, indicator);
        let push = 300.0 - (top + height / 2.0);
        let edge = 400.0 * 5.0 / (10.0 * (800.0 / 600.0));
        let expected = edge + CLEARANCE.gap_px + 8.0;
        assert!(
            (push - expected).abs() < 1.0,
            "push {push}, expected {expected} (the orb, not the trigger volume)"
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
        world.init_resource::<UiScale>();
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
    /// frame renders with. Before the PostUpdate
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

        use bevy::time::TimeUpdateStrategy;
        use nova_ship::camera::chase::{ChaseCamera, ChaseCameraInput, ChaseCameraPlugin};

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
        // `UiPlugin` owns the scale in the game; this rig has no UI stack.
        app.init_resource::<UiScale>();
        // The production camera-write order: the chase move lands before
        // propagation, so the rendered camera pose is this frame's.
        app.add_plugins(nova_ship::camera::CameraAuthorityPlugin);
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
