//! What the stage draws UNDER the document: the ground plane the range is laid
//! out on, and where the selected node stands on it.
//!
//! The stage is one open space with nothing in it but the objects themselves,
//! so until now there was no way to answer "how big is that gap" or "which way
//! am I facing" except by dragging something and watching it move. The grid is
//! the ruler that was missing: it carries the world origin, a decade scale that
//! grows with the camera, and the altitude of whatever is selected.
//!
//! Scenario-node only. Inside a ship there IS no ground - a part's place is
//! decided by mating, not by a plane - which is why
//! [`crate::placement::on_stage_drag_start`] refuses in there too.
//!
//! Immediate-mode [`Gizmos`] rather than [`crate::gizmo`]'s rig of meshes:
//! nothing drawn here is ever pointed at, so none of it has to be geometry the
//! picking ray can hit.

use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::{ColliderAabb, Sensor};
use bevy::prelude::*;
use nova_ui::theme;

use crate::{
    config::{EditorOverlays, SelectedNode},
    frame::node_bounds,
    gallery::EditorCamera,
    node::EditContext,
};

/// Cells across the fine grid, and so how far it reaches: half of them each way
/// from the centre. Even, so the decade pass divides into it exactly.
const CELLS: u32 = 60;

/// Every tenth line is drawn again in a brighter pass, so the eye can count
/// tens without counting lines - and so the step the grid is currently on can
/// be read off the screen instead of guessed.
const DECADE: u32 = 10;

/// Roughly how many cells should fill the view. The spacing is the power of ten
/// that comes nearest to this, which is what keeps a cell about the same size on
/// screen whether the camera is on top of a crate or looking at the whole range.
const WANTED_CELLS: f32 = 20.0;

/// The nearest and furthest the sizing looks. A camera sat on the plane would
/// otherwise ask for a grid of hairlines, and one pointed just above the horizon
/// for one whose cells are bigger than the range.
const MIN_REACH: f32 = 8.0;
const MAX_REACH: f32 = 20_000.0;

/// Spacing bounds, in world units. The floor is one section cell - the smallest
/// distance the editor lets anything move by - and the ceiling is a step already
/// wider than the authored range.
const MIN_STEP: f32 = 1.0;
const MAX_STEP: f32 = 1000.0;

/// How level a view has to be before it is treated as having no ground point at
/// all. Below this the intersection runs away to the horizon and back.
const LEVEL: f32 = 1.0e-3;

/// The grid itself, and the brighter line every [`DECADE`] cells.
const GRID: Color = theme::PHOSPHOR_MUTED;
const GRID_DECADE: Color = theme::PHOSPHOR_DIM;

/// The drop line from the selected node to the plane, and the footprint ring
/// where it lands.
const PLUMB: Color = theme::AMBER_NOVA;

/// Draw the ground plane the range is laid out on.
///
/// Three things at once, because they answer three questions a builder asks of
/// the same picture: the grid says how far apart, the origin's two axis lines
/// say which way, and the plumb line under the selection says how high - the
/// one number a drag cannot change, because [`crate::placement::on_stage_drag`]
/// holds a node's altitude while it slides.
pub(crate) fn draw_world_grid(
    overlays: Res<EditorOverlays>,
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
    camera: Option<Single<&GlobalTransform, With<EditorCamera>>>,
    q_children: Query<&Children>,
    q_bounds: Query<&ColliderAabb, Without<Sensor>>,
    q_poses: Query<&GlobalTransform, Without<EditorCamera>>,
    mut gizmos: Gizmos,
) {
    if !overlays.world_grid || context.ship().is_some() {
        return;
    }
    let Some(camera) = camera else {
        return;
    };
    let eye = camera.translation();
    let forward = Vec3::from(camera.forward());
    // What the camera is LOOKING at sizes the grid, not where it is: a camera
    // low over the plane and pointed at something far away is watching a wide
    // scene, and a grid sized by its height would draw that scene in hairlines.
    let (centre, reach) = match ground_focus(eye, forward) {
        Some(point) => (point, eye.distance(point)),
        None => (Vec3::new(eye.x, 0.0, eye.z), eye.y.abs()),
    };
    let step = grid_step(reach);
    let centre = grid_centre(centre, step);

    // A gizmo grid is drawn in its own XY plane, so it is laid flat by a quarter
    // turn about X.
    let flat = Quat::from_rotation_x(-FRAC_PI_2);
    gizmos.grid(
        Isometry3d::new(centre, flat),
        UVec2::splat(CELLS),
        Vec2::splat(step),
        GRID,
    );
    gizmos.grid(
        Isometry3d::new(centre, flat),
        UVec2::splat(CELLS / DECADE),
        Vec2::splat(step * DECADE as f32),
        GRID_DECADE,
    );

    // The world origin's own two lines, in the gizmo rig's axis colours, so
    // "where is 0,0,0" and "which way is +X" have the same answer on the plane
    // as they do on the handles. Drawn only while the origin is inside the
    // grid: past its edge they would be two coloured lines meaning nothing.
    let half = CELLS as f32 * step * 0.5;
    if centre.z.abs() <= half {
        gizmos.line(
            Vec3::new(centre.x - half, 0.0, 0.0),
            Vec3::new(centre.x + half, 0.0, 0.0),
            theme::RED,
        );
    }
    if centre.x.abs() <= half {
        gizmos.line(
            Vec3::new(0.0, 0.0, centre.z - half),
            Vec3::new(0.0, 0.0, centre.z + half),
            theme::BLUE,
        );
    }

    // Where the selection actually stands. Without it the plane is scenery: a
    // node hangs in front of it at an altitude nothing on screen reports.
    let Some(node) = selected.0 else {
        return;
    };
    let Ok(pose) = q_poses.get(node) else {
        return;
    };
    let bounds = node_bounds(node, &q_children, &q_bounds);
    let stand = bounds.map_or_else(|| pose.translation(), |bounds| bounds.center());
    let foot = Vec3::new(stand.x, 0.0, stand.z);
    gizmos.line(stand, foot, PLUMB);
    // The node's own footprint, so the ring reads as that thing's shadow rather
    // than as a fixed marker. Never smaller than a quarter cell, or a beacon
    // three units across leaves no mark on a grid stepping in hundreds.
    let ring = bounds
        .map_or(0.0, |bounds| bounds.size().xz().max_element() * 0.5)
        .max(step * 0.25);
    gizmos.circle(Isometry3d::new(foot, flat), ring, PLUMB);
}

/// The point on the ground plane the camera is looking at.
///
/// `None` when the view is level, turned away from the plane, or grazing it so
/// finely that the answer is past [`MAX_REACH`]: there is no useful point in
/// front of the camera then, and what matters instead is the ground under it.
fn ground_focus(eye: Vec3, forward: Vec3) -> Option<Vec3> {
    if forward.y.abs() < LEVEL {
        return None;
    }
    let toward = -eye.y / forward.y;
    if !(LEVEL..=MAX_REACH).contains(&toward) {
        return None;
    }
    Some(eye + forward * toward)
}

/// The cell size for a camera `reach` units from what it is looking at: the
/// power of ten that puts about [`WANTED_CELLS`] cells across the view.
///
/// Rounded UP to the next decade rather than to the nearest, because the two
/// failures are not equal: a grid one step too coarse is a grid you can still
/// count, and one step too fine is a grey wash.
fn grid_step(reach: f32) -> f32 {
    let wanted = reach.clamp(MIN_REACH, MAX_REACH) / WANTED_CELLS;
    let decade = wanted.log10().ceil();
    10f32.powf(decade).clamp(MIN_STEP, MAX_STEP)
}

/// `point` snapped onto the plane at a whole decade of `step`.
///
/// Snapped so the lines STAND STILL while the camera moves over them: a grid
/// centred on the exact focus point would slide under the scene, which reads as
/// the range moving rather than the camera. The decade rather than the step, so
/// the bright lines stay put too.
fn grid_centre(point: Vec3, step: f32) -> Vec3 {
    let coarse = step * DECADE as f32;
    Vec3::new(
        (point.x / coarse).round() * coarse,
        0.0,
        (point.z / coarse).round() * coarse,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Looking down at the plane finds the point ahead of the camera, not the
    /// one under it: that point is what the grid is sized and centred on.
    #[test]
    fn the_grid_is_sized_by_what_the_camera_looks_at() {
        let eye = Vec3::new(0.0, 100.0, 0.0);
        let focus = ground_focus(eye, Vec3::new(0.0, -1.0, -1.0).normalize())
            .expect("a view onto the plane has a ground point");
        assert!(focus.y.abs() < 1.0e-3, "the point is ON the plane");
        assert!(
            (focus.z + 100.0).abs() < 1.0e-3,
            "a 45 degree view from 100 up lands 100 ahead: {focus:?}"
        );
    }

    /// A level or upward view has no ground point in front of it. The caller
    /// falls back to the ground under the camera, which is why this must answer
    /// `None` rather than a number that ran away to the horizon.
    #[test]
    fn a_view_off_the_plane_has_no_ground_point() {
        let eye = Vec3::new(0.0, 50.0, 0.0);
        assert_eq!(ground_focus(eye, Vec3::NEG_Z), None, "level");
        assert_eq!(ground_focus(eye, Vec3::Y), None, "turned away");
        assert_eq!(
            ground_focus(eye, Vec3::new(0.0, -0.001, -1.0).normalize()),
            None,
            "grazing, so the point is past the far clamp"
        );
    }

    /// The step is a power of ten, it grows with the distance being watched,
    /// and it stops at both ends. A builder reads the scale off the bright
    /// lines, so a step of 30 or of 0.4 would be a scale nobody can count in.
    #[test]
    fn the_step_climbs_by_decades_between_its_bounds() {
        for reach in [1.0f32, 12.0, 90.0, 400.0, 3_000.0, 100_000.0] {
            let step = grid_step(reach);
            let decade = step.log10();
            assert!(
                (decade - decade.round()).abs() < 1.0e-4,
                "step {step} for reach {reach} is not a power of ten"
            );
            assert!((MIN_STEP..=MAX_STEP).contains(&step), "step {step} is out");
        }
        assert!(
            grid_step(50.0) < grid_step(5_000.0),
            "a wider view gets a wider cell"
        );
        assert_eq!(grid_step(0.5), MIN_STEP, "clamped at the near end");
        assert_eq!(grid_step(1.0e9), MAX_STEP, "clamped at the far end");
    }

    /// The centre lands on a whole decade of the step, so the bright lines hold
    /// still while the camera slides over them.
    #[test]
    fn the_centre_snaps_to_a_whole_decade() {
        let centre = grid_centre(Vec3::new(137.0, 42.0, -61.0), 10.0);
        assert_eq!(centre, Vec3::new(100.0, 0.0, -100.0));
        let centre = grid_centre(Vec3::new(4.0, 0.0, -4.0), 1.0);
        assert_eq!(centre, Vec3::ZERO, "and it is flattened onto the plane");
    }
}
