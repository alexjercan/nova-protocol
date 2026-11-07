//! The picker as a LIVE WINDOW: what opening it does, what dragging one of its
//! channels does to the document, and what happens to it when the row it
//! belongs to goes away.

use bevy::{
    camera::NormalizedRenderTarget,
    ecs::system::RunSystemOnce,
    picking::{
        backend::HitData,
        pointer::{Location, PointerId},
    },
    state::app::StatesPlugin,
    window::WindowRef,
};
use nova_events::units::prelude::*;
use nova_scenario::prelude::{
    AsteroidConfig, BeaconConfig, EntityFilterConfig, ScenarioObjectKind, KIND_ROCK,
};
use nova_ui::{prelude::TextFieldSubmitted, theme::ActiveUiTheme};

use super::*;
use crate::{
    config::SelectedNode,
    event::{ActionChoice, ActionChoiceExt, ActionNode, FilterKind, FilterNode},
    history::tests::{assert_one_step, assert_one_step_of, depth},
    node::{
        EditContext, EditorNode, NextChildOrdinal, NodeId, ObjectBodyStale, ObjectNode,
        ScenarioNode,
    },
    ui::{
        inspector::{
            apply_inspector_edits, inspector_panel, sync_inspector, InspectorField, InspectorPanel,
        },
        pick::{sync_pick_popups, PickOption, PickPopup},
    },
};

/// The panel, the window layer and the two reconcilers that keep them in step.
fn window_app() -> App {
    let mut app = App::new();
    app.init_resource::<crate::ui::inspector::InspectorFolds>();
    app.init_resource::<ActiveUiTheme>();
    app.init_resource::<SelectedNode>();
    // A slider that cannot write says so on the status line.
    app.init_resource::<crate::config::EditorStatus>();
    // And the panel behind the window reads the View menu's toggles.
    app.init_resource::<crate::config::EditorOverlays>();
    app.init_resource::<EditHistory>();
    app.init_resource::<UnsavedWork>();
    app.add_plugins(StatesPlugin);
    app.init_state::<GameStates>();
    app.init_state::<PauseStates>();
    app.init_state::<ExampleStates>();
    app.init_resource::<Time>();
    app.add_message::<TextFieldSubmitted>();
    // The write-back announces a stale object body rather than leaning on
    // `Changed<ObjectNode>`, so the rig carries that message too.
    app.add_message::<ObjectBodyStale>();
    // A window entity, because a floating window is placed and clamped against
    // the screen it stands on.
    app.world_mut().spawn(Window::default());
    app.world_mut().spawn(inspector_panel());
    app.world_mut().spawn(window_layer());
    app.add_systems(
        Update,
        (sync_inspector, sync_colour_windows, sync_pick_popups).chain(),
    );
    app.add_observer(on_colour_slider);
    app.add_observer(on_colour_release);
    app
}

/// One scenario node with one beacon under it - the shipped object whose config
/// holds a colour.
fn document(app: &mut App) -> (Entity, Entity) {
    let scenario = app
        .world_mut()
        .spawn((
            EditorNode,
            ScenarioNode::default(),
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
                    radius: Meters(30.0),
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
                    radius: Meters(30.0),
                    initial_velocity: MetersPerSecond3::ZERO,
                    texture: default(),
                    kind: KIND_ROCK.into(),
                    destroy_sound: None,
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
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));
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

/// A verb that clears the history asks first. The document is still standing while the
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
        "and the open scenario is still standing"
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
    // New Scenario has no single answer: the TEMPLATES are the answers, one
    // button each, and pressing one both picks the world and goes through.
    let template =
        named(&mut app, "Template Duelling Arena").expect("the templates are on the window");
    app.world_mut().trigger(Activate { entity: template });
    app.update();
    assert!(
        named(&mut app, "Confirm Window").is_none(),
        "and this answer takes it down too"
    );
    let now = app.world().resource::<EditContext>().scenario();
    assert!(
        now.is_some() && now != document,
        "the verb ran: the new scenario is the open one"
    );
    assert_eq!(
        app.world()
            .resource::<crate::project::ModProject>()
            .data
            .scenarios()
            .count(),
        2,
        "and the first is still in the mod"
    );
    assert_eq!(
        app.world()
            .get::<crate::node::ScenarioNode>(now.expect("a document stands"))
            .expect("the new root is a scenario node")
            .name,
        crate::template::ScenarioTemplate::Arena.settings().name,
        "and it founded the world the row named"
    );
}

/// The picker is right-anchored off the Inspector, and at a narrow window that
/// put it on the rail.
#[test]
fn a_fresh_window_stands_clear_of_the_rail() {
    assert_eq!(
        fresh_window_left(1024.0),
        412.0,
        "at the stock width it hangs off the Inspector's left edge"
    );
    assert!(
        fresh_window_left(760.0) >= crate::ui::RAIL_W,
        "at 760 there is no place that clears both panels, and the rail is the \
         one it must not cover: {}",
        fresh_window_left(760.0)
    );
}

/// A filter that matches one object by id - the row the picker exists for.
fn entity_filter(app: &mut App, scenario: Entity, id: &str) -> Entity {
    app.world_mut()
        .spawn((
            EditorNode,
            FilterNode {
                kind: FilterKind::Entity(EntityFilterConfig {
                    id: Some(id.to_string()),
                    ..default()
                }),
            },
            NodeId("entity_1".to_string()),
            ChildOf(scenario),
        ))
        .id()
}

/// Press the chip beside the Id row.
fn open_the_ref_picker(app: &mut App) {
    let chip = named(app, "Inspector Ref Id").expect("the id row carries a picker chip");
    app.world_mut().trigger(Activate { entity: chip });
    app.update();
}

fn ref_picker(app: &mut App) -> Option<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<PickPopup>>()
        .iter(app.world())
        .next()
}

/// What the open picker offers, in the order it lists them.
fn offered(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&PickOption>()
        .iter(app.world())
        .map(|option| option.value.clone())
        .collect()
}

fn filter_id(app: &App, filter: Entity) -> Option<String> {
    match &app
        .world()
        .get::<FilterNode>(filter)
        .expect("a filter")
        .kind
    {
        FilterKind::Entity(config) => config.id.clone(),
        other => panic!("not an entity filter: {other:?}"),
    }
}

/// The chip lists what the DOCUMENT holds, not what the type allows: a string
/// field cannot say which ids exist, and this is the answer only the document
/// can give.
#[test]
fn a_reference_chip_lists_the_ids_the_document_holds() {
    let mut app = window_app();
    let (scenario, _) = document(&mut app);
    asteroid(&mut app, scenario);
    let filter = entity_filter(&mut app, scenario, "");
    select(&mut app, filter);

    open_the_ref_picker(&mut app);

    assert!(ref_picker(&mut app).is_some(), "the picker opened");
    // The filter's id is optional, so the first row clears it.
    assert_eq!(offered(&mut app), vec!["", "asteroid_1", "beacon_1"]);
}

/// Picking writes the id the row holds.
#[test]
fn picking_an_id_writes_it_into_the_row() {
    let mut app = window_app();
    let (scenario, _) = document(&mut app);
    let filter = entity_filter(&mut app, scenario, "");
    select(&mut app, filter);
    open_the_ref_picker(&mut app);

    let option = named(&mut app, "Ref Option beacon_1").expect("the beacon is offered");
    app.world_mut().trigger(Activate { entity: option });
    app.update();

    assert_eq!(filter_id(&app, filter), Some("beacon_1".to_string()));
}

/// The picker belongs to the ROW, like every other window here: inspect
/// something else and it goes.
#[test]
fn inspecting_another_node_closes_the_reference_picker() {
    let mut app = window_app();
    let (scenario, beacon) = document(&mut app);
    let filter = entity_filter(&mut app, scenario, "");
    select(&mut app, filter);
    open_the_ref_picker(&mut app);
    assert!(ref_picker(&mut app).is_some());

    select(&mut app, beacon);

    assert!(ref_picker(&mut app).is_none());
}

/// The picker is where a builder MEETS the vocabulary: twenty-six action names
/// with nothing beside them is a list to be looked up somewhere else, so each
/// one carries the sentence its own config was documented with.
#[test]
fn a_long_choice_lists_what_each_option_does() {
    let mut app = window_app();
    let (scenario, _) = document(&mut app);
    let action = app
        .world_mut()
        .spawn((
            EditorNode,
            ActionNode {
                kind: ActionChoice::NarrativeCue.stock(),
            },
            NodeId("action_1".to_string()),
            ChildOf(scenario),
        ))
        .id();
    select(&mut app, action);

    let chip = named(&mut app, "Inspector Choice Action").expect("the kind row opens a picker");
    app.world_mut().trigger(Activate { entity: chip });
    app.update();

    let option = named(&mut app, "Choice Option Outcome").expect("the outcome is offered");
    let hint = &app
        .world()
        .get::<PickOption>(option)
        .expect("the row is an option")
        .hint;
    assert!(hint.contains("won or lost"), "the choice's own doc comment");
}

/// Let go of the pointer on `target`, the way the picking layer reports it.
fn release_on(app: &mut App, target: Entity) {
    let screen = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .expect("one window");
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(
                WindowRef::Entity(screen)
                    .normalize(None)
                    .expect("a named window normalizes"),
            ),
            position: Vec2::ZERO,
        },
        Release {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
        target,
    ));
    app.update();
}

/// A slider held down writes every frame, and only its end is an edit.
fn hold_channel(app: &mut App, channel: &str, to: f32) -> Entity {
    let slider = named(app, &format!("Colour Window Slider {channel}")).expect("a channel slider");
    app.world_mut().trigger(ValueChange::<f32> {
        source: slider,
        value: to,
        is_final: false,
    });
    app.update();
    slider
}

#[test]
fn a_colour_drag_is_one_step_at_its_end() {
    let mut app = window_app();
    let (_, beacon) = document(&mut app);
    select(&mut app, beacon);
    open_the_picker(&mut app);

    assert_one_step(&mut app, "Edit Color", |app| {
        hold_channel(app, "R", 0.2);
        hold_channel(app, "R", 0.4);
        assert_eq!(depth(app), 0, "a held slider is no step yet");
        drag_channel(app, "R", 0.5);
    });
}

/// A press on the track moves the channel with no drag, so the slider never
/// says `is_final`. The release ends that gesture, and the drag's own release
/// must not make it a second step.
#[test]
fn a_click_on_a_colour_track_is_one_step_on_release() {
    let mut app = window_app();
    let (_, beacon) = document(&mut app);
    select(&mut app, beacon);
    open_the_picker(&mut app);

    assert_one_step(&mut app, "Edit Color", |app| {
        let slider = hold_channel(app, "R", 0.3);
        release_on(app, slider);
        release_on(app, slider);
    });
}

#[test]
fn picking_a_reference_is_one_step() {
    let mut app = window_app();
    let (scenario, _) = document(&mut app);
    let filter = entity_filter(&mut app, scenario, "");
    select(&mut app, filter);
    open_the_ref_picker(&mut app);

    let read = move |app: &mut App| format!("{:?}", filter_id(app, filter));
    assert_one_step_of(&mut app, "Edit Id", read, |app| {
        let option = named(app, "Ref Option beacon_1").expect("the beacon is offered");
        app.world_mut().trigger(Activate { entity: option });
    });
}

/// A choice's list writes through the same step as every other edit.
#[test]
fn an_enum_popup_pick_is_one_undo_step() {
    let mut app = window_app();
    let (scenario, _) = document(&mut app);
    let action = app
        .world_mut()
        .spawn((
            EditorNode,
            ActionNode {
                kind: ActionChoice::NarrativeCue.stock(),
            },
            NodeId("action_1".to_string()),
            ChildOf(scenario),
        ))
        .id();
    select(&mut app, action);
    let chip = named(&mut app, "Inspector Choice Action").expect("the kind row opens a list");
    app.world_mut().trigger(Activate { entity: chip });
    app.update();

    let read = move |app: &mut App| {
        format!(
            "{:?}",
            app.world()
                .get::<ActionNode>(action)
                .expect("an action")
                .kind
        )
    };
    assert_one_step_of(&mut app, "Edit Kind", read, |app| {
        let option = named(app, "Choice Option Outcome").expect("the outcome is offered");
        app.world_mut().trigger(Activate { entity: option });
    });
}

/// A document with nothing unsaved has nothing to lose, so the row just leaves.
#[test]
fn leaving_a_saved_document_asks_nothing() {
    let mut app = window_app();
    app.add_observer(on_destructive_item);
    document(&mut app);
    let row = app.world_mut().spawn(DestructiveVerb::MainMenu).id();

    app.world_mut().trigger(Activate { entity: row });
    app.update();
    app.update();

    assert!(named(&mut app, "Confirm Window").is_none());
    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::MainMenu
    );
}

#[test]
fn leaving_with_unsaved_work_asks_first() {
    let mut app = window_app();
    app.add_observer(on_destructive_item);
    let (_, beacon) = document(&mut app);
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(beacon));
    crate::history::tests::baseline(&mut app);
    app.world_mut()
        .get_mut::<Transform>(beacon)
        .expect("the beacon stands")
        .translation
        .x = 40.0;
    crate::history::commit_edit_now(app.world_mut(), "Move");
    let before = app.world().resource::<State<GameStates>>().get().clone();
    let row = app.world_mut().spawn(DestructiveVerb::MainMenu).id();

    app.world_mut().trigger(Activate { entity: row });
    app.update();

    assert!(named(&mut app, "Confirm Window").is_some());
    assert_eq!(*app.world().resource::<State<GameStates>>().get(), before);
}

/// The pause menu and the outcome panel live outside the editor, and they stop
/// a leave through [`UnsavedWork`]. The question still comes up here.
#[test]
fn a_leave_stopped_outside_the_editor_is_asked_in_the_editor() {
    let mut app = window_app();
    app.add_systems(Update, ask_about_stopped_leave);
    document(&mut app);
    let mut unsaved = app.world_mut().resource_mut::<UnsavedWork>();
    unsaved.hold(true);
    assert!(
        unsaved.stops(GuardedLeave::Exit),
        "held work stops the leave"
    );

    app.update();

    assert!(named(&mut app, "Confirm Window").is_some());
    assert!(named(&mut app, "Confirm Discard Button").is_some());
}
