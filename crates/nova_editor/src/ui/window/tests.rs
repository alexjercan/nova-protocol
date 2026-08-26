//! The picker as a LIVE WINDOW: what opening it does, what dragging one of its
//! channels does to the document, and what happens to it when the row it
//! belongs to goes away.

use bevy::{
    camera::NormalizedRenderTarget,
    ecs::system::RunSystemOnce,
    picking::pointer::{Location, PointerId},
    window::WindowRef,
};
use nova_scenario::prelude::{AsteroidConfig, BeaconConfig, ScenarioObjectKind};
use nova_ui::prelude::TextFieldSubmitted;

use super::*;
use crate::{
    config::SelectedNode,
    node::{
        EditContext, EditorNode, NextChildOrdinal, NodeId, ObjectBodyStale, ObjectNode,
        ScenarioNode,
    },
    ui::inspector::{
        apply_inspector_edits, inspector_panel, sync_inspector, InspectorField, InspectorPanel,
    },
};

/// The panel, the window layer and the two reconcilers that keep them in step.
fn window_app() -> App {
    let mut app = App::new();
    app.insert_resource(UiSkin::default());
    app.init_resource::<SelectedNode>();
    // A slider that cannot write says so on the status line.
    app.init_resource::<crate::config::EditorStatus>();
    // And the panel behind the window reads the View menu's toggles.
    app.init_resource::<crate::config::EditorOverlays>();
    app.init_resource::<Time>();
    app.add_message::<TextFieldSubmitted>();
    // The write-back announces a stale object body rather than leaning on
    // `Changed<ObjectNode>`, so the rig carries that message too.
    app.add_message::<ObjectBodyStale>();
    // A window entity, because a floating window is placed and clamped against
    // the screen it stands on.
    app.world_mut().spawn(Window::default());
    app.world_mut().spawn(inspector_panel(UiSkin::default()));
    app.world_mut().spawn(window_layer());
    app.add_systems(Update, (sync_inspector, sync_colour_windows).chain());
    app.add_observer(on_colour_slider);
    app
}

/// One scenario node with one beacon under it - the shipped object whose config
/// holds a colour.
fn document(app: &mut App) -> (Entity, Entity) {
    let scenario = app
        .world_mut()
        .spawn((
            EditorNode,
            ScenarioNode,
            NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
        ))
        .id();
    app.world_mut().insert_resource(EditContext {
        path: vec![scenario],
    });
    let beacon = app
        .world_mut()
        .spawn((
            EditorNode,
            ObjectNode {
                name: "beacon_1".to_string(),
                kind: ScenarioObjectKind::Beacon(BeaconConfig {
                    label: "BEACON".to_string(),
                    radius: 3.0,
                    color: Color::WHITE,
                    area_radius: None,
                    lock_signature: None,
                }),
            },
            NodeId("beacon_1".to_string()),
            Transform::default(),
            ChildOf(scenario),
        ))
        .id();
    (scenario, beacon)
}

fn asteroid(app: &mut App, scenario: Entity) -> Entity {
    app.world_mut()
        .spawn((
            EditorNode,
            ObjectNode {
                name: "asteroid_1".to_string(),
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    radius: 3.0,
                    texture: default(),
                    impact_sound: None,
                    destroy_sound: None,
                    mass: None,
                    invulnerable: false,
                    seed: None,
                    lock_signature: None,
                }),
            },
            NodeId("asteroid_1".to_string()),
            Transform::default(),
            ChildOf(scenario),
        ))
        .id()
}

fn select(app: &mut App, node: Entity) {
    app.world_mut().resource_mut::<SelectedNode>().0 = Some(node);
    app.update();
}

fn named(app: &mut App, name: &str) -> Option<Entity> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, named)| named.as_str() == name)
        .map(|(entity, _)| entity)
}

/// Press the beacon's colour swatch, the way a click on it does.
fn open_the_picker(app: &mut App) -> Entity {
    let swatch =
        named(app, "Inspector Swatch Color").expect("the beacon's colour row has a swatch");
    app.world_mut().trigger(Activate { entity: swatch });
    app.update();
    swatch
}

fn picker(app: &mut App) -> Option<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<ColourWindow>>()
        .iter(app.world())
        .next()
}

fn beacon_colour(app: &App, beacon: Entity) -> Srgba {
    match &app
        .world()
        .get::<ObjectNode>(beacon)
        .expect("a beacon node")
        .kind
    {
        ScenarioObjectKind::Beacon(config) => Srgba::from(config.color),
        other => panic!("not a beacon: {other:?}"),
    }
}

/// Drag one channel, the way the slider's own drag reports it.
fn drag_channel(app: &mut App, channel: &str, to: f32) {
    let slider = named(app, &format!("Colour Window Slider {channel}")).expect("a channel slider");
    app.world_mut().trigger(ValueChange::<f32> {
        source: slider,
        value: to,
        is_final: true,
    });
    app.update();
}

/// Drag the window by its bar.
fn drag_the_bar(app: &mut App, delta: Vec2) {
    let bar = named(app, "Colour Window Bar").expect("the picker has a bar");
    let screen = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .expect("one window");
    let target = NormalizedRenderTarget::Window(
        WindowRef::Entity(screen)
            .normalize(None)
            .expect("a named window normalizes"),
    );
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        Location {
            target,
            position: Vec2::ZERO,
        },
        Drag {
            button: PointerButton::Primary,
            distance: delta,
            delta,
        },
        bar,
    ));
    app.update();
}

/// Where the picker stands.
fn picker_at(app: &mut App) -> Vec2 {
    let window = picker(app).expect("the picker is open");
    let node = app.world().get::<Node>(window).expect("a node");
    match (node.left, node.top) {
        (Val::Px(left), Val::Px(top)) => Vec2::new(left, top),
        other => panic!("a floating window is placed in pixels: {other:?}"),
    }
}

#[test]
fn clicking_a_colour_swatch_opens_a_picker_on_that_field() {
    let mut app = window_app();
    let (_, beacon) = document(&mut app);
    select(&mut app, beacon);

    open_the_picker(&mut app);

    let window = picker(&mut app).expect("the swatch opened a picker");
    let field = app
        .world()
        .get::<ColourWindow>(window)
        .expect("the picker knows its field")
        .field
        .clone();
    let swatch = named(&mut app, "Inspector Swatch Color").expect("the swatch");
    let swatch_field = app
        .world()
        .get::<InspectorField>(swatch)
        .expect("the swatch carries the same field")
        .clone();
    assert!(
        field == swatch_field,
        "the picker edits the row it was opened from"
    );
}

/// The point of the whole window: a colour nobody can author by reading it is
/// authored by dragging.
#[test]
fn dragging_a_channel_writes_the_colour_into_the_document() {
    let mut app = window_app();
    let (_, beacon) = document(&mut app);
    select(&mut app, beacon);
    open_the_picker(&mut app);

    drag_channel(&mut app, "R", 0.5);

    let colour = beacon_colour(&app, beacon);
    assert!(
        (colour.red - 0.5).abs() < 2.0 / 255.0,
        "the red channel followed the slider (got {colour:?})"
    );
    assert!(
        colour.green > 0.99 && colour.blue > 0.99,
        "and the other channels were left alone (got {colour:?})"
    );
}

/// The document is the source, not the picker: a colour retyped as hex in the
/// row moves the sliders under it.
#[test]
fn the_picker_follows_a_colour_typed_into_the_row() {
    let mut app = window_app();
    let (_, beacon) = document(&mut app);
    select(&mut app, beacon);
    open_the_picker(&mut app);

    let field = named(&mut app, "Inspector Field Color").expect("the colour row's box");
    app.world_mut().write_message(TextFieldSubmitted {
        entity: field,
        value: "#0000ff".to_string(),
    });
    app.world_mut()
        .run_system_once(apply_inspector_edits)
        .expect("the write-back runs");
    app.update();

    let hex = app
        .world_mut()
        .query::<(&ColourReadout, &Text)>()
        .iter(app.world())
        .map(|(_, text)| text.0.clone())
        .next()
        .expect("the picker reads its colour back");
    assert_eq!(hex, "#0000ff");
    let blue = app
        .world_mut()
        .query::<(&ColourSlider, &SliderValue)>()
        .iter(app.world())
        .find(|(slider, _)| slider.channel == ColourChannel::Blue)
        .map(|(_, value)| value.0)
        .expect("a blue channel");
    assert!(blue > 0.99, "the blue slider went to the top (got {blue})");
}

#[test]
fn a_window_is_dragged_by_its_bar() {
    let mut app = window_app();
    let (_, beacon) = document(&mut app);
    select(&mut app, beacon);
    open_the_picker(&mut app);
    let before = picker_at(&mut app);

    drag_the_bar(&mut app, Vec2::new(-40.0, 25.0));

    let after = picker_at(&mut app);
    assert!(
        (after - before - Vec2::new(-40.0, 25.0)).length() < 0.01,
        "the window followed the pointer ({before:?} -> {after:?})"
    );
}

/// A window belongs to the row it was opened from. Inspect something else and
/// the row is gone, so the window is too - otherwise the picker would be
/// writing to a node nobody is looking at.
#[test]
fn inspecting_another_node_closes_the_picker() {
    let mut app = window_app();
    let (scenario, beacon) = document(&mut app);
    let rock = asteroid(&mut app, scenario);
    select(&mut app, beacon);
    open_the_picker(&mut app);
    assert!(picker(&mut app).is_some());

    select(&mut app, rock);

    assert!(
        picker(&mut app).is_none(),
        "a rock has no colour row for the picker to belong to"
    );
}

/// The swatch is the only control the row has, so it is both the way in and
/// the way out.
#[test]
fn a_second_press_on_the_swatch_puts_the_picker_away() {
    let mut app = window_app();
    let (_, beacon) = document(&mut app);
    select(&mut app, beacon);
    open_the_picker(&mut app);

    open_the_picker(&mut app);

    assert!(picker(&mut app).is_none(), "the second press closed it");
}

/// The panel is what the picker reads its colour off, so a panel that is not
/// on screen leaves nothing behind.
#[test]
fn a_picker_needs_the_panel_that_opened_it() {
    let mut app = window_app();
    let (_, beacon) = document(&mut app);
    select(&mut app, beacon);
    open_the_picker(&mut app);

    let panel = app
        .world_mut()
        .query_filtered::<Entity, With<InspectorPanel>>()
        .single(app.world())
        .expect("one panel");
    app.world_mut().entity_mut(panel).despawn();
    app.update();

    assert!(picker(&mut app).is_none());
}

/// A verb with no undo asks first. The document is still standing while the
/// question is up: the row does nothing at all, and the window's own button is
/// what carries the verb out.
#[test]
fn a_destructive_verb_asks_before_it_runs() {
    let mut app = window_app();
    app.add_observer(on_destructive_item);
    app.add_observer(close_confirm_window);
    // NOT a global observer: the verb rides the window's own button, which is
    // the whole point - a bare `Activate` must not reset anything.
    app.init_resource::<EditContext>();
    app.world_mut()
        .run_system_once(crate::node::ensure_document)
        .expect("the document is founded");
    let document = app.world().resource::<EditContext>().scenario();
    let row = app.world_mut().spawn(DestructiveVerb::NewScenario).id();

    app.world_mut().trigger(Activate { entity: row });
    app.update();
    assert!(
        named(&mut app, "Confirm Window").is_some(),
        "the row puts the question up"
    );
    assert_eq!(
        app.world().resource::<EditContext>().scenario(),
        document,
        "and the document it would throw away is still standing"
    );

    let keep = named(&mut app, "Confirm Keep Button").expect("the safe answer is on the window");
    app.world_mut().trigger(Activate { entity: keep });
    app.update();
    assert!(
        named(&mut app, "Confirm Window").is_none(),
        "answering takes the question down"
    );
    assert_eq!(
        app.world().resource::<EditContext>().scenario(),
        document,
        "and Keep editing kept it"
    );

    app.world_mut().trigger(Activate { entity: row });
    app.update();
    let discard =
        named(&mut app, "Confirm Discard Button").expect("the other answer is on the window");
    app.world_mut().trigger(Activate { entity: discard });
    app.update();
    assert!(
        named(&mut app, "Confirm Window").is_none(),
        "and this answer takes it down too"
    );
    let now = app.world().resource::<EditContext>().scenario();
    assert!(
        now.is_some() && now != document,
        "the verb ran: the old root is gone and a fresh one stands in its place"
    );
}
