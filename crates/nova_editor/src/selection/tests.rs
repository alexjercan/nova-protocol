use super::*;
use crate::node::EditorNode;

fn press_app() -> App {
    let mut app = App::new();
    app.init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ViewportPointer>()
        .init_resource::<EditorRebind>()
        .init_resource::<StageTool>()
        .init_resource::<EditContext>()
        .init_resource::<EmptyPress>()
        .init_resource::<SelectedNode>()
        .insert_resource(HeldPart(None))
        .add_systems(Update, empty_space_press);
    app
}

fn aim(app: &mut App, at: Vec2, over_nothing: bool) {
    *app.world_mut().resource_mut::<ViewportPointer>() = ViewportPointer {
        over_ui: false,
        over_nothing,
        at: Some(at),
    };
}

fn click(app: &mut App, button_down_at: Vec2, released_at: Vec2) {
    aim(app, button_down_at, true);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    aim(app, released_at, true);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    app.update();
}

fn selected_rock(app: &mut App) -> Entity {
    let rock = app.world_mut().spawn(EditorNode).id();
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(rock));
    rock
}

#[test]
fn a_click_on_nothing_deselects() {
    let mut app = press_app();
    selected_rock(&mut app);

    click(&mut app, Vec2::splat(100.0), Vec2::splat(100.0));

    assert!(app.world().resource::<SelectedNode>().is_empty());
}

#[test]
fn a_shift_click_on_nothing_keeps_the_selection() {
    let mut app = press_app();
    let rock = selected_rock(&mut app);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);

    click(&mut app, Vec2::splat(100.0), Vec2::splat(100.0));

    assert_eq!(app.world().resource::<SelectedNode>().primary(), Some(rock));
}

#[test]
fn a_press_on_a_body_is_not_a_press_on_nothing() {
    let mut app = press_app();
    let rock = selected_rock(&mut app);
    aim(&mut app, Vec2::splat(100.0), false);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    app.update();

    assert_eq!(app.world().resource::<SelectedNode>().primary(), Some(rock));
}

#[test]
fn alt_on_nothing_is_the_cameras_and_keeps_the_selection() {
    let mut app = press_app();
    let rock = selected_rock(&mut app);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::AltLeft);

    click(&mut app, Vec2::splat(100.0), Vec2::splat(100.0));

    assert_eq!(app.world().resource::<SelectedNode>().primary(), Some(rock));
}

#[test]
fn a_drag_on_nothing_with_a_handle_tool_keeps_the_selection() {
    let mut app = press_app();
    let rock = selected_rock(&mut app);

    click(&mut app, Vec2::splat(100.0), Vec2::splat(300.0));

    assert_eq!(app.world().resource::<SelectedNode>().primary(), Some(rock));
}

#[test]
fn a_box_holds_a_point_either_way_it_was_drawn() {
    let (a, b) = (Vec2::new(10.0, 10.0), Vec2::new(50.0, 40.0));

    assert!(inside(a, b, Vec2::new(20.0, 20.0)));
    assert!(inside(b, a, Vec2::new(20.0, 20.0)));
    assert!(!inside(a, b, Vec2::new(60.0, 20.0)));
}

#[test]
fn the_selection_set_keeps_the_last_taken_as_primary() {
    let mut world = World::new();
    let (a, b) = (world.spawn_empty().id(), world.spawn_empty().id());
    let mut selected = SelectedNode::default();

    selected.toggle(a);
    selected.toggle(b);
    assert_eq!(selected.primary(), Some(b));
    assert_eq!(selected.len(), 2);

    selected.toggle(b);
    assert_eq!(selected.primary(), Some(a));

    selected.add(b);
    selected.add(a);
    assert_eq!(
        selected.primary(),
        Some(a),
        "taking one again moves it last"
    );
    assert_eq!(selected.len(), 2);
}
