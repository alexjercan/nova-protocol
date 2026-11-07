//! The two gestures as arithmetic, and the rule that decides whether the rig
//! is on screen at all. Picking is Bevy's; what is tested here is what a hit
//! on a handle DOES.

use avian3d::prelude::{Collider, SimpleCollider};
use bevy::ecs::system::RunSystemOnce;

use super::*;
use crate::{
    history::tests::{assert_one_step, depth, end_drag, first_with, history_app},
    node::{EditorNode, NodeView, ObjectChoice},
};

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

    let moved = applied(Travel::Slide(Vec3::Y * 4.0), &start, start.translation);

    assert_eq!(moved.translation, Vec3::new(1.0, 6.0, 3.0));
    assert_eq!(moved.rotation, start.rotation, "a move does not turn");
}

#[test]
fn a_turn_is_applied_about_the_world_axis_the_ring_names() {
    // Already yawed: a turn about world X must still be about world X, not
    // about the node's own side.
    let start = Transform::from_xyz(5.0, 0.0, 0.0).with_rotation(Quat::from_rotation_y(1.0));

    let turned = applied(
        Travel::Turn(Vec3::X, std::f32::consts::FRAC_PI_2),
        &start,
        start.translation,
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
    // The pointer sweeps out and comes back to where it started.
    let out = applied(Travel::Slide(Vec3::X * 12.0), &start, Vec3::ZERO);
    let back = applied(Travel::Slide(Vec3::ZERO), &start, Vec3::ZERO);

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
    app.init_resource::<StageTool>();
    app.add_message::<ObjectBodyStale>();
    app.world_mut().spawn((
        GizmoRig,
        Transform::default(),
        Visibility::Hidden,
        GlobalTransform::default(),
    ));
    // The GLOBAL pose is deliberately stale here - a frame behind the local
    // one, the way it is in a real app between `Update` and propagation. A rig
    // that sizes itself off it would read 300 units of distance the moment the
    // camera is framed somewhere, and flash one giant arm.
    app.world_mut().spawn((
        EditorCamera,
        Transform::from_xyz(0.0, 5.0, 30.0),
        GlobalTransform::from_xyz(0.0, 5.0, 900.0),
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
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));

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
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));

    place(&mut app);

    let (pose, _) = rig(&app);
    assert!(
        pose.scale.x > 4.0,
        "the arms have to reach past the hull to be pointed at: {:?}",
        pose.scale
    );
}

/// The rig sizes itself against where the camera IS, not where propagation
/// last saw it. Framing a node writes the camera's `Transform` and the global
/// pose catches up a stage later, so a rig reading the global one flashes an
/// arm sized for the distance it just left.
#[test]
fn the_rig_is_sized_from_where_the_camera_is_now() {
    let mut app = gizmo_app();
    // Small enough that the camera-distance floor decides the size, which is
    // the term the stale pose would poison.
    let node = app
        .world_mut()
        .spawn((EditorNode, ShipNode::default(), Transform::default()))
        .id();
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));

    place(&mut app);

    let arm = rig(&app).0.scale.x;
    assert!(
        arm < 10.0,
        "the camera is 30 units out, not 900: {arm} is the stale pose talking"
    );
}

/// The other half of the measure-once rule. A turn must not resize the rig; a
/// RESIZE must - and what resizes a node is its CONFIG. Scrubbing a rock's
/// Radius rebuilds the mesh and leaves `Transform.scale` at one, so a
/// measurement keyed on the scale never came back and the arms stayed sized
/// for the radius the rock used to have.
#[test]
fn a_redrawn_body_resizes_its_rig() {
    let mut app = gizmo_app();
    // Close in, so the node's own extent decides the arm rather than the
    // camera-distance floor.
    let at = Vec3::new(0.0, 5.0, 28.0);
    let node = rock(&mut app, at, Collider::sphere(3.0));
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));
    place(&mut app);
    let before = rig(&app).0.scale.x;

    regrow(&mut app, at, Collider::sphere(9.0));
    app.world_mut().write_message(ObjectBodyStale(node));
    place(&mut app);

    let after = rig(&app).0.scale.x;
    assert!(
        after > before * 1.5,
        "a rock scrubbed from radius 3 to radius 9 wears a bigger rig; it went \
         {before} -> {after}"
    );
}

/// And the announcement is what does it: the same new body with nobody saying
/// so leaves the standing measurement alone, which is the rule that keeps a
/// turn from making the arms swell.
#[test]
fn an_unannounced_body_leaves_the_rig_the_size_it_was() {
    let mut app = gizmo_app();
    let at = Vec3::new(0.0, 5.0, 28.0);
    let node = rock(&mut app, at, Collider::sphere(3.0));
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));
    place(&mut app);
    let before = rig(&app).0.scale.x;

    regrow(&mut app, at, Collider::sphere(9.0));
    place(&mut app);

    assert_eq!(rig(&app).0.scale.x, before);
}

/// One rock on the stage, with a body under it to measure.
fn rock(app: &mut App, at: Vec3, body: Collider) -> Entity {
    let node = app
        .world_mut()
        .spawn((
            EditorNode,
            ObjectNode {
                name: "Rock".to_string(),
                ..ObjectChoice::Asteroid.stock()
            },
            Transform::from_translation(at),
        ))
        .id();
    app.world_mut()
        .spawn((NodeView, ChildOf(node), body.aabb(at, Quat::IDENTITY)));
    node
}

/// Swap the body under the selected node, the way a rebuilt mesh does.
fn regrow(app: &mut App, at: Vec3, body: Collider) {
    let view = {
        let mut query = app
            .world()
            .try_query_filtered::<Entity, With<NodeView>>()
            .expect("the view is queryable");
        query.single(app.world()).expect("one view")
    };
    app.world_mut()
        .entity_mut(view)
        .insert(body.aabb(at, Quat::IDENTITY));
}

#[test]
fn turning_a_node_does_not_resize_its_rig() {
    let mut app = gizmo_app();
    let node = ship(&mut app, Vec3::ZERO);
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));
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
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));
    place(&mut app);
    assert_eq!(rig(&app).1, Visibility::Inherited);

    app.world_mut().resource_mut::<SelectedNode>().set(None);
    place(&mut app);

    assert_eq!(rig(&app).1, Visibility::Hidden);
}

#[test]
fn the_rig_stays_off_inside_a_ship_and_under_an_armed_part() {
    let mut app = gizmo_app();
    let node = ship(&mut app, Vec3::ZERO);
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));

    // Mating decides where a part sits, so there is nothing here to drag.
    let scenario = app.world_mut().spawn(EditorNode).id();
    app.world_mut().insert_resource(EditContext {
        path: vec![scenario, node],
    });
    place(&mut app);
    assert_eq!(rig(&app).1, Visibility::Hidden, "inside a ship");

    app.world_mut().insert_resource(EditContext::default());
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
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(section));

    place(&mut app);

    assert_eq!(rig(&app).1, Visibility::Hidden);
}

#[test]
fn a_ring_the_camera_looks_along_leaves_the_stage() {
    let mut app = gizmo_app();
    app.world_mut().resource_mut::<StageTool>().tool = StageToolType::Turn;
    let node = ship(&mut app, Vec3::ZERO);
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));
    let rings = [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z].map(|axis| {
        app.world_mut()
            .spawn((turn_handle(axis), Visibility::Inherited))
            .id()
    });

    place(&mut app);

    // The fixture's camera stands in the X ring's plane and near enough the Y
    // ring's - which is the stock framing angle, not a corner of one. A ring
    // left there swallows the press that was meant for the body under it.
    let shown = |entity| {
        *app.world()
            .get::<Visibility>(entity)
            .expect("a handle keeps its visibility")
    };
    assert_eq!(shown(rings[0]), Visibility::Hidden, "X, seen edge-on");
    assert_eq!(
        shown(rings[1]),
        Visibility::Hidden,
        "Y, seen nearly edge-on"
    );
    assert_eq!(shown(rings[2]), Visibility::Inherited, "Z, seen square-on");
}

/// A turn is one step named for what it did, however many frames it wrote.
#[test]
fn a_gizmo_turn_is_one_step_on_release() {
    let mut app = history_app();
    app.init_resource::<GizmoDrag>();
    app.add_observer(on_gizmo_release);
    let rock = first_with::<ObjectNode>(&mut app);

    assert_one_step(&mut app, "Turn", |app| {
        let mut state = app.world_mut().resource_mut::<GizmoDrag>();
        state.members = vec![(rock, Transform::default())];
        state.handle = Some(turn_handle(GizmoAxis::Y));
        for _ in 0..3 {
            app.world_mut()
                .get_mut::<Transform>(rock)
                .expect("the rock stands")
                .rotate_y(0.1);
            app.update();
            assert_eq!(depth(app), 0, "a held handle is no step yet");
        }
        end_drag(app, rock);
    });
}

/// Which handles are up, by kind, with `tool` in hand.
fn kinds_up(tool: StageToolType) -> Vec<HandleKind> {
    let mut app = gizmo_app();
    app.world_mut().resource_mut::<StageTool>().tool = tool;
    let node = app
        .world_mut()
        .spawn((
            EditorNode,
            ObjectNode {
                name: "Rock".to_string(),
                ..ObjectChoice::Asteroid.stock()
            },
            Transform::default(),
        ))
        .id();
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));
    let kinds = [
        HandleKind::Move,
        HandleKind::Plane,
        HandleKind::Turn,
        HandleKind::Stretch,
        HandleKind::Uniform,
    ];
    let handles = kinds.map(|kind| {
        app.world_mut()
            .spawn((
                GizmoHandle {
                    axis: GizmoAxis::Z,
                    kind,
                },
                Visibility::Inherited,
            ))
            .id()
    });
    place(&mut app);
    kinds
        .into_iter()
        .zip(handles)
        .filter(|(_, handle)| {
            app.world().get::<Visibility>(*handle) == Some(&Visibility::Inherited)
        })
        .map(|(kind, _)| kind)
        .collect()
}

#[test]
fn each_tool_shows_only_its_own_handles() {
    assert_eq!(
        kinds_up(StageToolType::Move),
        [HandleKind::Move, HandleKind::Plane]
    );
    assert_eq!(kinds_up(StageToolType::Turn), [HandleKind::Turn]);
    assert_eq!(
        kinds_up(StageToolType::Scale),
        [HandleKind::Stretch, HandleKind::Uniform]
    );
}

#[test]
fn the_select_tool_shows_no_rig() {
    let mut app = gizmo_app();
    app.world_mut().resource_mut::<StageTool>().tool = StageToolType::Select;
    let node = ship(&mut app, Vec3::ZERO);
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));

    place(&mut app);

    assert_eq!(rig(&app).1, Visibility::Hidden);
}

#[test]
fn a_ship_shows_no_scale_handles() {
    let mut app = gizmo_app();
    app.world_mut().resource_mut::<StageTool>().tool = StageToolType::Scale;
    let node = ship(&mut app, Vec3::ZERO);
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));

    place(&mut app);

    assert_eq!(rig(&app).1, Visibility::Hidden);
}

#[test]
fn local_space_turns_the_rig_to_the_primary_node() {
    let mut app = gizmo_app();
    let node = ship(&mut app, Vec3::ZERO);
    let facing = Quat::from_rotation_y(0.7);
    app.world_mut()
        .get_mut::<Transform>(node)
        .expect("a pose")
        .rotation = facing;
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));

    place(&mut app);
    assert_eq!(rig(&app).0.rotation, Quat::IDENTITY, "world axes first");

    app.world_mut().resource_mut::<StageTool>().space = GizmoSpaceType::Local;
    place(&mut app);
    assert_eq!(rig(&app).0.rotation, facing);
}

#[test]
fn a_group_rig_stands_at_the_centre_of_the_selection() {
    let mut app = gizmo_app();
    let left = ship(&mut app, Vec3::new(-10.0, 0.0, 0.0));
    let right = ship(&mut app, Vec3::new(30.0, 4.0, 0.0));
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set_all([left, right]);

    place(&mut app);

    assert_eq!(rig(&app).0.translation, Vec3::new(10.0, 2.0, 0.0));
}

#[test]
fn a_group_turn_swings_each_node_about_the_centre() {
    let start = Transform::from_xyz(10.0, 0.0, 0.0);

    let turned = applied(
        Travel::Turn(Vec3::Y, std::f32::consts::PI),
        &start,
        Vec3::ZERO,
    );

    assert!((turned.translation - Vec3::new(-10.0, 0.0, 0.0)).length() < 1e-4);
    assert!((turned.rotation * Vec3::X + Vec3::X).length() < 1e-4);
}

#[test]
fn a_plane_square_slides_on_its_plane_and_leaves_the_normal_alone() {
    let eye = Vec3::new(0.0, 20.0, 0.0);
    let grab = plane_hit(ray(eye, Vec3::new(1.0, 0.0, 1.0)), Vec3::ZERO, Vec3::Y)
        .expect("the plane is across the view");
    let now = plane_hit(ray(eye, Vec3::new(4.0, 0.0, -2.0)), Vec3::ZERO, Vec3::Y)
        .expect("the plane is across the view");

    let across = now - grab;

    assert!((across - Vec3::new(3.0, 0.0, -3.0)).length() < 1e-4);
}

#[test]
fn ctrl_snaps_a_plane_slide_to_the_grid_on_both_axes() {
    let cell = MOVE_SNAP.to_engine();
    let centre = Vec3::new(0.3, 0.0, 0.2);

    let by = snapped_across(
        centre,
        Vec3::new(cell * 2.1, 0.0, cell * 3.1),
        Quat::IDENTITY,
        cell,
    );

    let landed = centre + by;
    assert!((landed.x - snapped(centre.x + cell * 2.1, cell)).abs() < 1e-4);
    assert!((landed.z - snapped(centre.z + cell * 3.1, cell)).abs() < 1e-4);
    assert_eq!(by.y, 0.0, "the normal stays put");
}

#[test]
fn a_size_handle_doubles_at_one_arm_and_stops_at_the_floor() {
    assert_eq!(stretched(Meters(40.0), 5.0, 5.0, false), Meters(80.0));
    assert_eq!(stretched(Meters(40.0), -5.0, 5.0, false), MIN_SIZE);
}

#[test]
fn ctrl_snaps_a_size_to_its_step() {
    assert_eq!(stretched(Meters(40.0), 0.61, 5.0, true), Meters(45.0));
}

/// A group drag is one step on release, however many nodes and frames it wrote.
#[test]
fn a_group_move_is_one_undo_step() {
    let mut app = history_app();
    app.init_resource::<GizmoDrag>();
    app.add_observer(on_gizmo_release);
    let rock = first_with::<ObjectNode>(&mut app);
    let ship = first_with::<ShipNode>(&mut app);

    assert_one_step(&mut app, "Move", |app| {
        let mut state = app.world_mut().resource_mut::<GizmoDrag>();
        state.members = vec![(rock, Transform::default()), (ship, Transform::default())];
        state.handle = Some(move_handle(GizmoAxis::Y));
        for _ in 0..3 {
            for node in [rock, ship] {
                app.world_mut()
                    .get_mut::<Transform>(node)
                    .expect("the node stands")
                    .translation
                    .y += 1.0;
            }
            app.update();
            assert_eq!(depth(app), 0, "a held handle is no step yet");
        }
        end_drag(app, rock);
    });
}
