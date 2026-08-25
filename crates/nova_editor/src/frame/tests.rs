//! What a framing request does to the camera, and what it does when there is
//! nothing to frame. The pose itself is `crate::node::frame_stage`'s; these
//! tests are about which node is chosen and when the request clears.

use avian3d::prelude::{Collider, SimpleCollider};
use bevy::ecs::system::RunSystemOnce;
use nova_scenario::prelude::{AnchorConfig, ScenarioObjectKind};

use super::*;
use crate::node::{EditorNode, NodeView, ObjectNode, ShipNode};

/// The stage's resources, with nothing running on them.
fn frame_app() -> App {
    let mut app = App::new();
    app.init_resource::<FrameRequest>();
    app.init_resource::<SelectedNode>();
    app.init_resource::<EditContext>();
    app.insert_resource(SectionChoice::None);
    app
}

fn camera(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            EditorCamera,
            WASDCameraController,
            Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id()
}

/// An object node with one view under it, so it has bounds to be framed by.
fn rock(app: &mut App, at: Vec3, radius: f32) -> Entity {
    let node = app
        .world_mut()
        .spawn((
            EditorNode,
            ObjectNode {
                name: "rock".to_string(),
                kind: ScenarioObjectKind::Anchor(AnchorConfig {
                    body_radius: radius,
                    mass: None,
                }),
            },
            Transform::from_translation(at),
        ))
        .id();
    app.world_mut().spawn((
        NodeView,
        ChildOf(node),
        Collider::sphere(radius).aabb(at, Quat::IDENTITY),
        Transform::from_translation(at),
    ));
    node
}

fn camera_pose(app: &App, camera: Entity) -> Transform {
    *app.world()
        .entity(camera)
        .get::<Transform>()
        .expect("a pose")
}

fn serve(app: &mut App) {
    app.world_mut()
        .run_system_once(apply_frame_request)
        .expect("the system runs");
}

fn bounds_of(app: &mut App, node: Entity) -> Option<ColliderAabb> {
    app.world_mut()
        .run_system_once(
            move |q_children: Query<&Children>, q_bounds: Query<&ColliderAabb, Without<Sensor>>| {
                node_bounds(node, &q_children, &q_bounds)
            },
        )
        .expect("the system runs")
}

#[test]
fn a_served_request_puts_the_camera_on_the_node() {
    let mut app = frame_app();
    let eye = camera(&mut app);
    let at = Vec3::new(40.0, 0.0, -12.0);
    let node = rock(&mut app, at, 3.0);

    app.world_mut().resource_mut::<FrameRequest>().0 = Some(node);
    serve(&mut app);

    let pose = camera_pose(&app, eye);
    let wanted = (at - pose.translation).normalize();
    assert!(
        pose.forward().as_vec3().dot(wanted) > 0.999,
        "the camera aims at the framed node, not {:?}",
        pose.forward()
    );
    assert_eq!(
        app.world().resource::<FrameRequest>().0,
        None,
        "a served request must not be served again next frame"
    );
}

#[test]
fn a_node_with_no_bounds_is_framed_at_its_origin() {
    let mut app = frame_app();
    let eye = camera(&mut app);
    // No view under it: a node placed this frame has no collider yet.
    let node = app
        .world_mut()
        .spawn((
            EditorNode,
            ShipNode::default(),
            Transform::from_xyz(0.0, 0.0, -60.0),
        ))
        .id();

    app.world_mut().resource_mut::<FrameRequest>().0 = Some(node);
    serve(&mut app);

    assert_eq!(
        camera_pose(&app, eye).translation,
        frame_stage(Vec3::new(0.0, 0.0, -60.0), 0.0).translation,
        "the origin is the fallback, rather than waiting for a collider"
    );
}

#[test]
fn a_request_for_a_deleted_node_clears_itself() {
    let mut app = frame_app();
    let eye = camera(&mut app);
    let before = camera_pose(&app, eye).translation;
    let node = rock(&mut app, Vec3::new(9.0, 0.0, 0.0), 1.0);
    app.world_mut().entity_mut(node).despawn();

    app.world_mut().resource_mut::<FrameRequest>().0 = Some(node);
    serve(&mut app);

    assert_eq!(app.world().resource::<FrameRequest>().0, None);
    assert_eq!(
        camera_pose(&app, eye).translation,
        before,
        "nothing to look at, so nothing moves"
    );
}

#[test]
fn a_request_raised_before_the_camera_exists_is_held() {
    let mut app = frame_app();
    let node = rock(&mut app, Vec3::new(0.0, 0.0, -30.0), 2.0);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(node);

    // The frame the editor is entered on: the camera is still a command away.
    serve(&mut app);
    assert_eq!(
        app.world().resource::<FrameRequest>().0,
        Some(node),
        "a request nobody could serve is held, not dropped"
    );

    let eye = camera(&mut app);
    serve(&mut app);
    assert_eq!(app.world().resource::<FrameRequest>().0, None);
    assert!(
        camera_pose(&app, eye).translation.z < 0.0,
        "and then it lands"
    );
}

#[test]
fn framing_hands_the_free_fly_rig_back() {
    let mut app = frame_app();
    let eye = camera(&mut app);
    let node = rock(&mut app, Vec3::new(5.0, 0.0, 0.0), 1.0);

    app.world_mut().resource_mut::<FrameRequest>().0 = Some(node);
    serve(&mut app);

    assert!(
        app.world().entity(eye).contains::<WASDCameraController>(),
        "the rig re-reads the pose on setup; without the round trip it would \
         snap the camera back next frame"
    );
}

#[test]
fn the_key_frames_the_selection_and_falls_back_to_the_context() {
    let mut app = frame_app();
    let scenario = app
        .world_mut()
        .spawn((EditorNode, Transform::default()))
        .id();
    let node = rock(&mut app, Vec3::ZERO, 1.0);
    app.world_mut().insert_resource(EditContext {
        path: vec![scenario],
    });

    /// One fresh press of F, and what it asked for. Fresh because `press` only
    /// counts as JUST pressed on a key that was up.
    fn tap(app: &mut App) -> Option<Entity> {
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(FRAME_KEY);
        app.world_mut().insert_resource(keys);
        app.world_mut()
            .run_system_once(frame_key)
            .expect("the system runs");
        let asked = app.world().resource::<FrameRequest>().0;
        app.world_mut().resource_mut::<FrameRequest>().0 = None;
        asked
    }

    assert_eq!(
        tap(&mut app),
        Some(scenario),
        "nothing selected frames what you are standing in"
    );
    app.world_mut().resource_mut::<SelectedNode>().0 = Some(node);
    assert_eq!(tap(&mut app), Some(node), "a selection wins");
}

#[test]
fn the_key_stays_out_of_the_way_of_an_armed_part() {
    let mut app = frame_app();
    let node = rock(&mut app, Vec3::ZERO, 1.0);
    app.world_mut().resource_mut::<SelectedNode>().0 = Some(node);
    app.world_mut()
        .insert_resource(SectionChoice::Section("thruster".to_string()));
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(FRAME_KEY);
    app.world_mut().insert_resource(keys);

    app.world_mut()
        .run_system_once(frame_key)
        .expect("the system runs");

    assert_eq!(
        app.world().resource::<FrameRequest>().0,
        None,
        "F cycles the armed part's socket; it must not also move the camera"
    );
}

#[test]
fn the_menu_row_greys_only_when_there_is_nothing_to_frame() {
    let mut app = frame_app();
    let row = app.world_mut().spawn(FrameSelectionItem).id();

    // An empty context and no selection: the document does not exist yet.
    app.world_mut()
        .run_system_once(sync_frame_item)
        .expect("the system runs");
    assert!(app.world().entity(row).contains::<InteractionDisabled>());

    let scenario = app
        .world_mut()
        .spawn((EditorNode, Transform::default()))
        .id();
    app.world_mut().insert_resource(EditContext {
        path: vec![scenario],
    });
    app.world_mut()
        .run_system_once(sync_frame_item)
        .expect("the system runs");
    assert!(
        !app.world().entity(row).contains::<InteractionDisabled>(),
        "standing somewhere is always something to frame"
    );
}

#[test]
fn a_ships_bounds_are_its_sections_and_not_its_origin() {
    let mut app = frame_app();
    let ship = app
        .world_mut()
        .spawn((EditorNode, ShipNode::default(), Transform::default()))
        .id();
    let hull = Collider::cuboid(2.0, 2.0, 2.0);
    for offset in [Vec3::new(20.0, 0.0, 0.0), Vec3::new(24.0, 0.0, 0.0)] {
        app.world_mut().spawn((
            NodeView,
            ChildOf(ship),
            hull.aabb(offset, Quat::IDENTITY),
            Transform::from_translation(offset),
        ));
    }

    let bounds = bounds_of(&mut app, ship).expect("two sections have bounds");

    assert!(
        (bounds.center().x - 22.0).abs() < 0.01,
        "the union of the sections, not the node's origin: {:?}",
        bounds.center()
    );
}

#[test]
fn a_sensor_volume_is_not_part_of_a_nodes_size() {
    let mut app = frame_app();
    let node = rock(&mut app, Vec3::ZERO, 1.0);
    // A beacon's trigger sphere: tens of units of trigger, not of beacon.
    app.world_mut().spawn((
        NodeView,
        ChildOf(node),
        Sensor,
        Collider::sphere(60.0).aabb(Vec3::ZERO, Quat::IDENTITY),
    ));

    let bounds = bounds_of(&mut app, node).expect("the body has bounds");

    assert!(
        bounds.size().length() < 10.0,
        "the trigger volume would put the beacon in an empty screen: {:?}",
        bounds.size()
    );
}
