//! What the stage draws AROUND the document: the ground plane the range is laid
//! out on, and the parts of an object that have no body to be seen by.
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
//! picking ray can hit. That is also what keeps these off the object itself -
//! a trigger sphere made of collider would be tens of units of pickable
//! nothing sitting over the beacon it belongs to.

use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::{ColliderAabb, Sensor};
use bevy::prelude::*;
use nova_scenario::prelude::{LightConfig, ScenarioObjectKind};
use nova_ui::theme;

use crate::{
    config::{EditorGizmos, EditorOverlays, HoveredNode, SelectedNode},
    frame::node_bounds,
    gallery::EditorCamera,
    gizmo::GizmoAxis,
    node::{objects_of, EditContext, ObjectNodes},
    ui::inspector::PANEL_W as INSPECTOR_W,
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
///
/// The fine pass is `PHOSPHOR_MUTED` taken down by alpha rather than a new
/// palette entry: the theme mirrors the PoC `:root` tokens one for one and is
/// drift-tested against the site. Sixty cells of it at full strength read as
/// the subject; at this weight the decade lines carry the scale and the grid
/// is the floor again.
const GRID: Color = Color::srgba_u8(0x0d, 0x6e, 0x35, 0x9c);
const GRID_DECADE: Color = theme::PHOSPHOR_DIM;

/// The drop line from the selected node to the plane, and the footprint ring
/// where it lands.
const PLUMB: Color = theme::AMBER_NOVA;

/// A trigger volume - the sphere a beacon or a crate fires `OnEnter` from. One
/// colour for all of them, because what they have in common is the thing worth
/// seeing: fly in here and the scenario hears about it.
const TRIGGER: Color = theme::BLUE;

/// How much of the distance to the camera a sun's arrow spans, and the disc its
/// parallel rays leave from as a fraction of that arrow.
///
/// Screen-sized rather than world-sized, the same rule
/// [`crate::gizmo`]'s handles are sized by: a directional light has no size and
/// no reach, so there is nothing about it for the drawing to be a picture OF -
/// only which way it shines. A fixed world length is a few pixels across a
/// range and a wall across a crate.
const SUN_SPAN: f32 = 0.10;
const SUN_DISC: f32 = 0.18;
/// The shortest that arrow gets, so a sun the camera is sitting on still points
/// somewhere.
const SUN_MIN: f32 = 3.0;

/// How far in front of the eye the axis rose floats, in world units, and how
/// long an arm is as a fraction of that.
///
/// Pinned to the camera rather than to the world: a rose the size of the scene
/// would be a wall inside a ship and a speck over a range. At this depth it is
/// in front of everything the editor draws, which is what an overlay has to be.
const ROSE_DEPTH: f32 = 2.0;
const ROSE_ARM: f32 = 0.035;

/// How far the rose sits from the bottom-right of the VIEWPORT, in logical
/// pixels - the inspector's width taken off the right, because the corner of
/// the window is behind that panel and a rose nobody can see says nothing. It
/// clears the key legend along the bottom by the same margin.
const ROSE_INSET: Vec2 = Vec2::new(56.0, 80.0);

/// The dot on the far end of an arm, as a fraction of the arm. It is what says
/// which end is POSITIVE, on a rose with no room for letters.
const ROSE_TIP: f32 = 0.16;

/// Draw the axis rose in the viewport's bottom-right corner.
///
/// Always on, and in both contexts. Inside a ship the grid is off by design -
/// a part's place is decided by mating, not by a plane - which leaves a grey
/// hull on black with nothing at all to say which way is up. Only the POSITIVE
/// half of each axis is drawn, with a dot on its end: an arm that exists in one
/// direction says which way as well as which axis, and drawing the negative
/// half faint read as a cross at this size.
///
/// The colours are [`crate::gizmo`]'s: the arm that says where X is and the
/// handle that drags along X are the same red.
pub(crate) fn draw_axis_rose(
    camera: Option<Single<(&Camera, &GlobalTransform), With<EditorCamera>>>,
    mut gizmos: Gizmos<EditorGizmos>,
) {
    let Some(camera) = camera else {
        return;
    };
    let (camera, pose) = camera.into_inner();
    let Some(viewport) = camera.logical_viewport_size() else {
        return;
    };
    let corner = Vec2::new(
        viewport.x - INSPECTOR_W - ROSE_INSET.x,
        viewport.y - ROSE_INSET.y,
    );
    let Ok(ray) = camera.viewport_to_world(pose, corner) else {
        return;
    };
    let origin = ray.get_point(ROSE_DEPTH);
    let arm = ROSE_DEPTH * ROSE_ARM;
    // The dots face the eye, so an axis pointing at the camera reads as a ring
    // rather than as a missing arm.
    let facing = Quat::from_rotation_arc(Vec3::Z, -Vec3::from(ray.direction));
    for axis in GizmoAxis::ALL {
        let tip = origin + axis.unit() * arm;
        gizmos.line(origin, tip, axis.colour());
        gizmos.circle(Isometry3d::new(tip, facing), arm * ROSE_TIP, axis.colour());
    }
}

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
    mut gizmos: Gizmos<EditorGizmos>,
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

/// The colour the mark is drawn in. The brightest phosphor on the stage: the
/// selection is the one thing on screen every other panel is about.
const MARK: Color = theme::PHOSPHOR;

/// The colour a HOVER is drawn in. A step down from the selection, because the
/// pointer answers "which one is this" and the selection answers "which one am
/// I working on" - and the second must still be findable while the first moves.
const HOVER_MARK: Color = theme::PHOSPHOR_DIM;

/// How far off the node's own skin the box stands, as a fraction of its
/// longest side. Off the body rather than on it - an outline flush with a hull
/// face reads as part of the hull.
const MARK_MARGIN: f32 = 0.06;

/// The box drawn around a node of these bounds.
fn mark_box(bounds: ColliderAabb) -> Transform {
    let size = Vec3::from(bounds.size());
    let pad = size.max_element() * MARK_MARGIN;
    Transform::from_translation(bounds.center()).with_scale(size + Vec3::splat(pad))
}

/// Draw a box around whatever is marked, and a dimmer one around whatever the
/// pointer is resting on.
///
/// EVERY context, ship included. Out in the world the handle rig doubles as
/// the mark, but inside a ship the rig is deliberately suppressed - a part's
/// pose belongs to its socket - so a marked part was reported only by the
/// tree row and the Inspector, both of them off at the edges of the screen.
/// Rebinding a key, deleting a part or reading its stats all act on a thing
/// the stage would not point at.
///
/// The hover is skipped when it IS the selection: two boxes a hair apart on
/// the same hull read as a drawing fault, not as two facts.
pub(crate) fn draw_node_marks(
    selected: Res<SelectedNode>,
    hovered: Res<HoveredNode>,
    q_children: Query<&Children>,
    q_bounds: Query<&ColliderAabb, Without<Sensor>>,
    mut gizmos: Gizmos<EditorGizmos>,
) {
    let marks = [
        (
            hovered.0.filter(|node| Some(*node) != selected.0),
            HOVER_MARK,
        ),
        (selected.0, MARK),
    ];
    for (node, colour) in marks {
        let Some(bounds) = node.and_then(|node| node_bounds(node, &q_children, &q_bounds)) else {
            continue;
        };
        gizmos.cube(mark_box(bounds), colour);
    }
}

/// Draw what an object HAS but does not show.
///
/// Every kind on the stage gets a schematic body from
/// [`crate::preview::insert_preview_object`], so a rock is a rock and a crate
/// is a crate. What none of them get is the part with no body at all: the
/// sphere a crate is collected inside, the sphere a beacon fires from, the
/// distance a lamp reaches, the direction a sun shines. Those are authored
/// numbers with real consequences for the scenario, and until now the only way
/// to see one was to fly the range and find out.
///
/// A light draws in its OWN colour, so a warm key and a cold rim are told apart
/// without selecting either; a trigger draws in one colour for all kinds,
/// because what a trigger is matters more than which object owns it.
///
/// Scenario-node only, like the grid: inside a ship the whole world is off the
/// stage ([`crate::node::sync_ship_focus`]), and a volume drawn around a hidden
/// object would be a sphere around nothing.
pub(crate) fn draw_object_volumes(
    overlays: Res<EditorOverlays>,
    context: Res<EditContext>,
    q_objects: ObjectNodes,
    camera: Option<Single<&GlobalTransform, With<EditorCamera>>>,
    mut gizmos: Gizmos<EditorGizmos>,
) {
    if !overlays.object_volumes || context.ship().is_some() {
        return;
    }
    let (Some(scenario), Some(camera)) = (context.scenario(), camera) else {
        return;
    };
    let eye = camera.translation();
    for (_, _, object, pose) in objects_of(scenario, &q_objects) {
        if let ScenarioObjectKind::Light(light) = &object.kind {
            draw_light(&mut gizmos, pose, light, eye);
        }
        if let Some(radius) = trigger_radius(&object.kind) {
            gizmos.sphere(
                Isometry3d::from_translation(pose.translation),
                radius,
                TRIGGER,
            );
        }
    }
}

/// The radius of the sphere this kind fires `OnEnter` from, if it fires at all.
///
/// AUTHORED-OR-ABSENT for a beacon: it is its own trigger area only when a
/// radius says so, and a beacon that fires nothing must not draw a sphere
/// claiming it does. Not optional for a crate - the pickup volume IS how a
/// crate is collected, and it is always wider than the box you can see.
///
/// The rest are their own picture: a rock, a hull and an anchor's published
/// radius are all drawn as bodies by [`crate::preview::insert_preview_object`].
fn trigger_radius(kind: &ScenarioObjectKind) -> Option<f32> {
    match kind {
        ScenarioObjectKind::Beacon(beacon) => beacon.area_radius,
        ScenarioObjectKind::SalvageCrate(salvage) => Some(salvage.area_radius),
        ScenarioObjectKind::Anchor(_)
        | ScenarioObjectKind::Asteroid(_)
        | ScenarioObjectKind::Spaceship(_)
        | ScenarioObjectKind::Light(_) => None,
    }
}

/// One light's own gizmo: which way a sun shines, or how far a lamp reaches.
fn draw_light(gizmos: &mut Gizmos<EditorGizmos>, pose: &Transform, light: &LightConfig, eye: Vec3) {
    match light {
        LightConfig::Directional { color, aim, .. } => {
            let toward = sun_direction(pose, *aim);
            let reach = (pose.translation.distance(eye) * SUN_SPAN).max(SUN_MIN);
            gizmos.arrow(pose.translation, pose.translation + toward * reach, *color);
            // The disc the parallel rays leave from, square to the beam: an
            // arrow alone reads as a point light with an opinion.
            gizmos.circle(
                Isometry3d::new(pose.translation, Quat::from_rotation_arc(Vec3::Z, toward)),
                reach * SUN_DISC,
                *color,
            );
        }
        // The RANGE, not the source radius: range is where the light stops
        // reaching, which is the number that decides what is lit.
        LightConfig::Point { color, range, .. } => {
            gizmos.sphere(
                Isometry3d::from_translation(pose.translation),
                *range,
                *color,
            );
        }
    }
}

/// Which way a sun shines.
///
/// The node's rotation aims it unless the config names a point, which is the
/// rule the SPAWN follows: an authored `aim` is re-aimed there with the same
/// `looking_at`. A light drawn shining the other way would be a light that
/// turned as the range was flown. It is also why a placed light is aimed with
/// the ordinary turn handles.
fn sun_direction(pose: &Transform, aim: Option<Vec3>) -> Vec3 {
    match aim {
        Some(at) => (at - pose.translation).normalize_or(Vec3::NEG_Z),
        None => pose.rotation * Vec3::NEG_Z,
    }
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
    use nova_gameplay::prelude::AssetRef;
    use nova_scenario::prelude::{
        aimed_light_base, AnchorConfig, AsteroidConfig, BeaconConfig, SalvageCrateConfig,
    };

    use super::*;

    fn beacon(area_radius: Option<f32>) -> ScenarioObjectKind {
        ScenarioObjectKind::Beacon(BeaconConfig {
            label: "BEACON".to_string(),
            radius: 3.0,
            color: Color::WHITE,
            area_radius,
            lock_signature: None,
        })
    }

    /// A trigger sphere is drawn for exactly the kinds that fire `OnEnter`, and
    /// a beacon that was never given an area is not one of them: it would be a
    /// sphere promising a scenario event that never comes.
    #[test]
    fn only_the_kinds_that_fire_on_enter_get_a_trigger_sphere() {
        assert_eq!(trigger_radius(&beacon(Some(40.0))), Some(40.0));
        assert_eq!(trigger_radius(&beacon(None)), None);
        assert_eq!(
            trigger_radius(&ScenarioObjectKind::SalvageCrate(SalvageCrateConfig {
                size: 2.0,
                area_radius: 12.0,
                pickup_sound: None,
            })),
            Some(12.0),
            "a crate's pickup volume is not optional"
        );
        assert_eq!(
            trigger_radius(&ScenarioObjectKind::Anchor(AnchorConfig {
                body_radius: 5.0,
                mass: None,
            })),
            None,
            "an anchor publishes a radius, but nothing enters it"
        );
        assert_eq!(
            trigger_radius(&ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius: 3.0,
                texture: AssetRef::from("self://textures/rock.png"),
                impact_sound: None,
                destroy_sound: None,
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            })),
            None,
            "a rock is its own picture"
        );
    }

    /// A sun with no aim point shines the way the node faces - and "the way the
    /// node faces" has to mean what the SPAWN means by it, or the arrow points
    /// somewhere the flown range does not light.
    #[test]
    fn a_sun_shines_the_way_its_node_faces() {
        let from = Vec3::new(30.0, 40.0, 0.0);
        let target = Vec3::ZERO;
        let base = aimed_light_base("key", "Key", from, target);
        let pose = Transform::from_translation(base.position).with_rotation(base.rotation);

        let toward = sun_direction(&pose, None);
        let wanted = (target - from).normalize();
        assert!(
            toward.distance(wanted) < 1.0e-4,
            "an aimed light base shines at its target: {toward:?} vs {wanted:?}"
        );
    }

    /// An authored aim point beats the node's rotation, because the spawn
    /// re-aims the light at that point and ignores the rotation too.
    #[test]
    fn an_aim_point_beats_the_nodes_rotation() {
        let pose = Transform::from_translation(Vec3::new(0.0, 10.0, 0.0));
        let toward = sun_direction(&pose, Some(Vec3::new(0.0, 10.0, -5.0)));
        assert!(toward.distance(Vec3::NEG_Z) < 1.0e-4, "{toward:?}");

        // A degenerate aim - the light's own position - has no direction in it.
        // The fallback is the node's own forward, not a NaN pointed nowhere.
        let toward = sun_direction(&pose, Some(pose.translation));
        assert!(toward.is_finite(), "{toward:?}");
    }

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

    /// The mark stands OFF the thing it marks. A box the same size as the hull
    /// it outlines draws inside the hull's own faces and reads as plating.
    #[test]
    fn the_selection_mark_stands_clear_of_the_node_it_marks() {
        let bounds = ColliderAabb::new(Vec3::new(2.0, 0.0, 0.0), Vec3::splat(0.5));
        let box_of = mark_box(bounds);

        assert_eq!(
            box_of.translation,
            Vec3::new(2.0, 0.0, 0.0),
            "the box is centred on the node, wherever the node stands"
        );
        assert!(
            box_of.scale.cmpgt(Vec3::ONE).all(),
            "and every side clears the unit cube it is drawn around, got {:?}",
            box_of.scale
        );

        // A long thin part pads by its LONGEST side, so the mark is the same
        // weight on a hull as on a drive nozzle.
        let long = ColliderAabb::new(Vec3::ZERO, Vec3::new(4.0, 0.25, 0.25));
        let padded = mark_box(long).scale - Vec3::new(8.0, 0.5, 0.5);
        assert!(
            (padded.x - padded.y).abs() < 1e-5 && (padded.y - padded.z).abs() < 1e-5,
            "the clearance is one number, not one per axis; got {padded:?}"
        );
    }
}
