//! The two gestures as arithmetic, and the rule that decides whether the rig
//! is on screen at all. Picking is Bevy's; what is tested here is what a hit
//! on a handle DOES.

use avian3d::prelude::{Collider, SimpleCollider};
use bevy::ecs::system::RunSystemOnce;

use super::*;
use crate::node::{EditorNode, NodeView};

/// A ray from `from` through `towards`, the way a pointer's would arrive.
fn ray(from: Vec3, towards: Vec3) -> Ray3d {
    Ray3d::new(from, Dir3::new(towards - from).expect("a direction"))
}

fn move_handle(axis: GizmoAxis) -> GizmoHandle {
    GizmoHandle {
        axis,
        kind: HandleKind::Move,
    }
}

fn turn_handle(axis: GizmoAxis) -> GizmoHandle {
    GizmoHandle {
        axis,
        kind: HandleKind::Turn,
    }
}

#[test]
fn an_arrow_reads_how_far_along_its_axis_the_pointer_is() {
    // Straight down the -Z view at a point three units along +X.
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let along = axis_parameter(ray(eye, Vec3::new(3.0, 0.0, 0.0)), Vec3::ZERO, Vec3::X)
        .expect("the axis is across the view");

    assert!(
        (along - 3.0).abs() < 1e-3,
        "the closest point on the axis is where the pointer aims: {along}"
    );
}

#[test]
fn an_arrow_pointing_at_the_camera_refuses_the_grab() {
    // The Z arm, seen from straight down the Z axis: a pixel of pointer travel
    // would be an unbounded slide.
    let refused = axis_parameter(
        ray(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO),
        Vec3::ZERO,
        Vec3::Z,
    );

    assert_eq!(refused, None);
}

#[test]
fn a_ring_seen_edge_on_refuses_the_grab() {
    // The Y ring lies in the XZ plane; a ray travelling inside that plane
    // never crosses it.
    let refused = plane_angle(
        ray(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO),
        Vec3::ZERO,
        Vec3::Y,
    );

    assert_eq!(refused, None);
}

#[test]
fn a_ring_measures_a_quarter_turn_as_a_quarter_turn() {
    // Looking down at the XZ plane from above: two hits ninety degrees apart
    // about the Y axis. Which direction the plane calls zero does not matter -
    // only the step between two readings is ever used.
    let eye = Vec3::new(0.0, 10.0, 0.0);
    let first = plane_angle(ray(eye, Vec3::new(4.0, 0.0, 0.0)), Vec3::ZERO, Vec3::Y)
        .expect("the plane is across the view");
    let second = plane_angle(ray(eye, Vec3::new(0.0, 0.0, 4.0)), Vec3::ZERO, Vec3::Y)
        .expect("the plane is across the view");

    assert!(
        (shortest_way(second - first).abs() - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
        "a quarter of the way round reads as a quarter turn: {first} -> {second}"
    );
}

#[test]
fn a_step_across_the_seam_is_the_short_way_round() {
    // atan2 answers in the half turn either side of zero, so a pointer that
    // creeps past the seam reads as nearly a whole turn backwards.
    let seam = shortest_way(-std::f32::consts::TAU + 0.2);

    assert!(
        (seam - 0.2).abs() < 1e-5,
        "a fifth of a radian forwards, not a turn back: {seam}"
    );
    assert!(
        (shortest_way(0.4) - 0.4).abs() < 1e-5,
        "an ordinary step is left alone"
    );
}

#[test]
fn a_move_slides_along_one_axis_and_leaves_the_others_alone() {
    let start = Transform::from_xyz(1.0, 2.0, 3.0).with_rotation(Quat::from_rotation_x(0.5));

    let moved = dragged(move_handle(GizmoAxis::Y), &start, 4.0);

    assert_eq!(moved.translation, Vec3::new(1.0, 6.0, 3.0));
    assert_eq!(moved.rotation, start.rotation, "a move does not turn");
}

#[test]
fn a_turn_is_applied_about_the_world_axis_the_ring_names() {
    // Already yawed: a turn about world X must still be about world X, not
    // about the node's own side.
    let start = Transform::from_xyz(5.0, 0.0, 0.0).with_rotation(Quat::from_rotation_y(1.0));

    let turned = dragged(
        turn_handle(GizmoAxis::X),
        &start,
        std::f32::consts::FRAC_PI_2,
    );

    assert_eq!(
        turned.translation, start.translation,
        "a turn does not move"
    );
    let up = turned.rotation * Vec3::Y;
    assert!(
        (up - (Quat::from_rotation_x(std::f32::consts::FRAC_PI_2) * (start.rotation * Vec3::Y)))
            .length()
            < 1e-4,
        "the world X turn is applied on the left: {up:?}"
    );
}

#[test]
fn a_gesture_is_measured_from_the_grab_and_not_frame_by_frame() {
    let start = Transform::from_xyz(0.0, 0.0, 0.0);
    let handle = move_handle(GizmoAxis::X);

    // The pointer sweeps out and comes back to where it started.
    let out = dragged(handle, &start, 12.0);
    let back = dragged(handle, &start, 0.0);

    assert_eq!(out.translation.x, 12.0);
    assert_eq!(
        back.translation, start.translation,
        "a sweep that returns puts the node back; an accumulated one would drift"
    );
}

/// The stage's resources plus a rig, with nothing else running.
fn gizmo_app() -> App {
    let mut app = App::new();
    app.init_resource::<EditContext>();
    app.init_resource::<SelectedNode>();
    app.init_resource::<GalleryState>();
    app.init_resource::<GizmoReach>();
    app.insert_resource(SectionChoice::None);
    app.world_mut().spawn((
        GizmoRig,
        Transform::default(),
        Visibility::Hidden,
        GlobalTransform::default(),
    ));
    app.world_mut().spawn((
        EditorCamera,
        Transform::from_xyz(0.0, 5.0, 30.0),
        GlobalTransform::from_xyz(0.0, 5.0, 30.0),
    ));
    app
}

fn ship(app: &mut App, at: Vec3) -> Entity {
    let node = app
        .world_mut()
        .spawn((
            EditorNode,
            ShipNode::default(),
            Transform::from_translation(at),
        ))
        .id();
    app.world_mut().spawn((
        NodeView,
        ChildOf(node),
        Collider::cuboid(4.0, 2.0, 8.0).aabb(at, Quat::IDENTITY),
    ));
    node
}

fn place(app: &mut App) {
    app.world_mut()
        .run_system_once(sync_gizmo)
        .expect("the system runs");
}

fn rig(app: &App) -> (Transform, Visibility) {
    let mut query = app
        .world()
        .try_query_filtered::<(&Transform, &Visibility), With<GizmoRig>>()
        .expect("the rig is queryable");
    let (pose, visibility) = query.single(app.world()).expect("one rig");
    (*pose, *visibility)
}

#[test]
fn the_rig_rides_the_selected_node() {
    let mut app = gizmo_app();
    let node = ship(&mut app, Vec3::new(12.0, 0.0, -4.0));
    app.world_mut().resource_mut::<SelectedNode>().0 = Some(node);

    place(&mut app);

    let (pose, visibility) = rig(&app);
    assert_eq!(visibility, Visibility::Inherited);
    assert_eq!(pose.translation, Vec3::new(12.0, 0.0, -4.0));
}

#[test]
fn the_rig_clears_a_hull_it_would_otherwise_sit_inside() {
    let mut app = gizmo_app();
    // Close enough that camera distance alone would give a rig smaller than
    // the ship it is on.
    let node = ship(&mut app, Vec3::new(0.0, 5.0, 28.0));
    app.world_mut().resource_mut::<SelectedNode>().0 = Some(node);

    place(&mut app);

    let (pose, _) = rig(&app);
    assert!(
        pose.scale.x > 4.0,
        "the arms have to reach past the hull to be pointed at: {:?}",
        pose.scale
    );
}

#[test]
fn turning_a_node_does_not_resize_its_rig() {
    let mut app = gizmo_app();
    let node = ship(&mut app, Vec3::ZERO);
    app.world_mut().resource_mut::<SelectedNode>().0 = Some(node);
    place(&mut app);
    let before = rig(&app).0.scale;

    // A world-axis box drawn around a TURNED hull is a bigger box - here half
    // again as wide. Sizing the rig off that every frame is what made the
    // handles swell under their own turn ring.
    let turn = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
    let square_on = Collider::cuboid(4.0, 2.0, 8.0).aabb(Vec3::ZERO, Quat::IDENTITY);
    let turned = Collider::cuboid(4.0, 2.0, 8.0).aabb(Vec3::ZERO, turn);
    assert!(
        turned.size().length() > square_on.size().length() * 1.1,
        "the fixture has to actually grow, or this proves nothing"
    );
    heel_over(&mut app, node, turn, turned);

    place(&mut app);

    assert_eq!(
        rig(&app).0.scale,
        before,
        "the rig is sized by the node it is on, not by the box the world draws round it"
    );
}

/// Turn `node` and grow its view's world-axis box to match, the way a real
/// turn does once the physics step has run.
fn heel_over(app: &mut App, node: Entity, turn: Quat, bounds: ColliderAabb) {
    app.world_mut()
        .entity_mut(node)
        .get_mut::<Transform>()
        .expect("the node has a pose")
        .rotation = turn;
    let view = app
        .world_mut()
        .try_query_filtered::<Entity, With<NodeView>>()
        .expect("the view is queryable")
        .single(app.world())
        .expect("one view");
    app.world_mut().entity_mut(view).insert(bounds);
}

#[test]
fn nothing_selected_hides_the_rig() {
    let mut app = gizmo_app();
    let node = ship(&mut app, Vec3::ZERO);
    app.world_mut().resource_mut::<SelectedNode>().0 = Some(node);
    place(&mut app);
    assert_eq!(rig(&app).1, Visibility::Inherited);

    app.world_mut().resource_mut::<SelectedNode>().0 = None;
    place(&mut app);

    assert_eq!(rig(&app).1, Visibility::Hidden);
}

#[test]
fn the_rig_stays_off_inside_a_ship_and_under_an_armed_part() {
    let mut app = gizmo_app();
    let node = ship(&mut app, Vec3::ZERO);
    app.world_mut().resource_mut::<SelectedNode>().0 = Some(node);

    // Mating decides where a part sits, so there is nothing here to drag.
    let scenario = app.world_mut().spawn(EditorNode).id();
    app.world_mut().insert_resource(EditContext {
        path: vec![scenario, node],
    });
    place(&mut app);
    assert_eq!(rig(&app).1, Visibility::Hidden, "inside a ship");

    app.world_mut().insert_resource(EditContext::default());
    app.world_mut()
        .insert_resource(SectionChoice::Section("thruster".to_string()));
    place(&mut app);
    assert_eq!(rig(&app).1, Visibility::Hidden, "with a part armed");

    app.world_mut().insert_resource(SectionChoice::None);
    app.world_mut().resource_mut::<GalleryState>().open = true;
    place(&mut app);
    assert_eq!(rig(&app).1, Visibility::Hidden, "under the gallery");
}

#[test]
fn a_section_selected_inside_a_ship_gets_no_handles() {
    let mut app = gizmo_app();
    // A section node is neither a ship nor an object, so it never answers the
    // query the rig rides on.
    let section = app
        .world_mut()
        .spawn((EditorNode, Transform::default()))
        .id();
    app.world_mut().resource_mut::<SelectedNode>().0 = Some(section);

    place(&mut app);

    assert_eq!(rig(&app).1, Visibility::Hidden);
}
