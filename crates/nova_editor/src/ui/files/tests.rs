//! The file window as a LIVE WINDOW: what a typed name derives, what the rows
//! do in each of the two windows, and what a press writes.

use bevy::ecs::system::RunSystemOnce;
use nova_ui::prelude::TextFieldSubmitted;

use super::*;
use crate::{node::NextChildOrdinal, ui::window::window_layer};

/// The layer a window stands on, the resources the file verbs write, and the
/// two systems that keep the readout in step.
fn files_app() -> App {
    let mut app = App::new();
    app.insert_resource(UiSkin::default());
    app.init_resource::<FileRequest>();
    app.init_resource::<FileWindowRequest>();
    app.init_resource::<DocumentSlot>();
    app.init_resource::<EditContext>();
    app.init_resource::<crate::config::EditorStatus>();
    app.init_resource::<Time>();
    app.add_message::<TextFieldSubmitted>();
    // A window entity, because a floating window is placed against the screen
    // it stands on.
    app.world_mut().spawn(Window::default());
    app.world_mut().spawn(window_layer());
    app.add_systems(Update, (open_file_window, sync_save_name).chain());
    app.add_observer(close_file_window);
    app
}

/// One document root, standing on itself, named `name`.
fn document(app: &mut App, name: &str) -> Entity {
    let scenario = app
        .world_mut()
        .spawn((
            crate::node::EditorNode,
            ScenarioNode {
                name: name.to_string(),
                ..default()
            },
            crate::node::NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
        ))
        .id();
    app.world_mut().insert_resource(EditContext {
        path: vec![scenario],
    });
    scenario
}

fn named(app: &mut App, name: &str) -> Option<Entity> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, named)| named.as_str() == name)
        .map(|(entity, _)| entity)
}

/// Put a window up over a list the test chose, so what the rows offer is not
/// whatever the machine running the test happens to have saved.
fn put_up(app: &mut App, kind: FileWindowKind, name: &str, bundles: Vec<SaveSlot>) {
    let layer = named(app, "Editor Window Layer").expect("the rig stands a layer up");
    let offered = name.to_string();
    app.world_mut()
        .commands()
        .entity(layer)
        .with_children(|layer| {
            spawn_file_window(
                layer,
                kind,
                &offered,
                bundles,
                Vec2::ZERO,
                UiSkin::default(),
            );
        });
    app.world_mut().flush();
    app.update();
}

/// Type into the window's name field, the way the widget does.
fn type_name(app: &mut App, typed: &str) {
    let field = named(app, "Save Name Field").expect("a Save As window has a name field");
    app.world_mut()
        .entity_mut(field)
        .insert(TextFieldValue(typed.to_string()));
    app.update();
}

fn readout(app: &mut App) -> String {
    let line = named(app, "Save Id Readout").expect("the window says where the name lands");
    app.world().get::<Text>(line).expect("it is text").0.clone()
}

fn slot(id: &str, name: &str) -> SaveSlot {
    SaveSlot {
        id: id.to_string(),
        name: name.to_string(),
    }
}

/// The id is DERIVED, so a builder types a name and never a directory.
#[test]
fn a_name_makes_the_slot_it_saves_into() {
    assert_eq!(
        bundle_id("Asteroid Gauntlet").as_deref(),
        Some("editor_asteroid_gauntlet")
    );
    assert_eq!(
        bundle_id("  Spaced   Out  ").as_deref(),
        Some("editor_spaced_out"),
        "the runs between words collapse, and the edges do not become an id"
    );
    assert_eq!(bundle_id("Range 2").as_deref(), Some("editor_range_2"));
    assert_eq!(
        bundle_id("   ").as_deref(),
        None,
        "a name with nothing in it names nothing"
    );
    assert_eq!(bundle_id("!!!").as_deref(), None);
}

/// Two names one id: the collision the window has to say out loud, because the
/// second one written replaces the first.
#[test]
fn two_names_that_differ_only_in_punctuation_land_in_one_slot() {
    assert_eq!(bundle_id("My Range"), bundle_id("My-Range!"));
}

/// Save As opens on the name the range already has, so a first save is one
/// press rather than a blank field to fill in.
#[test]
fn save_as_offers_the_name_the_range_already_has() {
    let mut app = files_app();
    document(&mut app, "Asteroid Gauntlet");

    app.world_mut().resource_mut::<FileWindowRequest>().0 = Some(FileWindowKind::SaveAs);
    app.update();

    let field = named(&mut app, "Save Name Field").expect("the window has a name field");
    assert_eq!(
        app.world()
            .get::<TextFieldValue>(field)
            .expect("it holds text")
            .0,
        "Asteroid Gauntlet"
    );
}

/// The press writes the slot AND the range's own name: a document has one
/// name, and the file it is in is what the Scenarios picker will list.
#[test]
fn saving_writes_the_typed_name_into_the_request_and_onto_the_range() {
    let mut app = files_app();
    let scenario = document(&mut app, "Saved Range");
    app.add_observer(on_save);
    put_up(&mut app, FileWindowKind::SaveAs, "Saved Range", vec![]);

    type_name(&mut app, "Asteroid Gauntlet");
    let save = named(&mut app, "File Save Button").expect("Save As has a Save button");
    app.world_mut().trigger(Activate { entity: save });
    app.update();

    assert_eq!(
        *app.world().resource::<FileRequest>(),
        FileRequest::SaveAs(slot("editor_asteroid_gauntlet", "Asteroid Gauntlet"))
    );
    assert_eq!(
        app.world()
            .get::<ScenarioNode>(scenario)
            .expect("the root")
            .name,
        "Asteroid Gauntlet",
        "and the range is called what the file is called"
    );
    assert!(
        named(&mut app, "File Window").is_none(),
        "the answer takes the window down"
    );
}

/// A name that derives no id cannot be saved, and the button says so by being
/// unpressable rather than by refusing after the press.
#[test]
fn a_name_with_nothing_in_it_greys_save() {
    let mut app = files_app();
    document(&mut app, "Saved Range");
    put_up(&mut app, FileWindowKind::SaveAs, "Saved Range", vec![]);
    let save = named(&mut app, "File Save Button").expect("Save As has a Save button");
    assert!(
        !app.world().entity(save).contains::<InteractionDisabled>(),
        "the offered name is a name"
    );

    type_name(&mut app, "???");

    assert!(app.world().entity(save).contains::<InteractionDisabled>());
    assert_eq!(readout(&mut app), "A name needs a letter or a digit.");

    type_name(&mut app, "Back Again");

    assert!(
        !app.world().entity(save).contains::<InteractionDisabled>(),
        "and it comes back the moment the name does"
    );
    assert_eq!(readout(&mut app), "saves as editor_back_again");
}

/// The overwrite is said BEFORE the press. A save that silently replaced
/// another range is the one mistake this window exists to prevent.
#[test]
fn a_name_that_is_already_a_slot_says_it_overwrites() {
    let mut app = files_app();
    document(&mut app, "Saved Range");
    put_up(
        &mut app,
        FileWindowKind::SaveAs,
        "Saved Range",
        vec![slot("editor_my_range", "My Range")],
    );

    type_name(&mut app, "My-Range!");

    assert_eq!(
        readout(&mut app),
        "overwrites \"My Range\" (editor_my_range)"
    );
}

/// In an Open window a row IS the answer: pressing one asks for that slot and
/// takes the window down.
#[test]
fn a_row_in_an_open_window_is_the_answer() {
    let mut app = files_app();
    document(&mut app, "Saved Range");
    app.add_observer(on_bundle_row);
    put_up(
        &mut app,
        FileWindowKind::Open,
        "Saved Range",
        vec![slot("editor_my_range", "My Range")],
    );

    let row = named(&mut app, "Bundle Row editor_my_range").expect("the slot is offered");
    app.world_mut().trigger(Activate { entity: row });
    app.update();

    assert_eq!(
        *app.world().resource::<FileRequest>(),
        FileRequest::Open("editor_my_range".to_string())
    );
    assert!(named(&mut app, "File Window").is_none());
}

/// In a Save As window the same row is a name to write OVER, so it fills the
/// field and leaves the writing to the Save button.
#[test]
fn a_row_in_a_save_as_window_fills_the_name_rather_than_writing() {
    let mut app = files_app();
    document(&mut app, "Saved Range");
    app.add_observer(on_bundle_row);
    put_up(
        &mut app,
        FileWindowKind::SaveAs,
        "Saved Range",
        vec![slot("editor_my_range", "My Range")],
    );

    let row = named(&mut app, "Bundle Row editor_my_range").expect("the slot is offered");
    app.world_mut().trigger(Activate { entity: row });
    app.update();

    let field = named(&mut app, "Save Name Field").expect("the window has a name field");
    assert_eq!(
        app.world()
            .get::<TextFieldValue>(field)
            .expect("it holds text")
            .0,
        "My Range"
    );
    assert_eq!(
        *app.world().resource::<FileRequest>(),
        FileRequest::None,
        "nothing is written until Save is pressed"
    );
    assert!(
        named(&mut app, "File Window").is_some(),
        "and the window is still up to press it on"
    );
    assert_eq!(
        readout(&mut app),
        "overwrites \"My Range\" (editor_my_range)"
    );
}

/// The safe answer is on every file window, and it writes nothing.
#[test]
fn cancel_takes_the_window_down_and_asks_for_nothing() {
    let mut app = files_app();
    document(&mut app, "Saved Range");
    put_up(&mut app, FileWindowKind::Open, "Saved Range", vec![]);

    let cancel = named(&mut app, "File Cancel Button").expect("every file window can be left");
    app.world_mut().trigger(Activate { entity: cancel });
    app.update();

    assert!(named(&mut app, "File Window").is_none());
    assert_eq!(*app.world().resource::<FileRequest>(), FileRequest::None);
}

/// One window at a time: a second ask over an open one changes nothing.
#[test]
fn a_second_ask_does_not_stack_a_second_window() {
    let mut app = files_app();
    document(&mut app, "Saved Range");
    put_up(&mut app, FileWindowKind::SaveAs, "Saved Range", vec![]);

    app.world_mut().resource_mut::<FileWindowRequest>().0 = Some(FileWindowKind::Open);
    app.update();

    let windows = app
        .world_mut()
        .query_filtered::<Entity, With<FileWindow>>()
        .iter(app.world())
        .count();
    assert_eq!(windows, 1);
}

/// A document that has never been saved is what `run_system_once` sees on the
/// way in, so the offered name falls back to the range's own.
#[test]
fn a_document_with_no_slot_is_offered_its_range_name() {
    let mut app = files_app();
    document(&mut app, "First Shift Practice");

    let offered = app
        .world_mut()
        .run_system_once(
            |document: Res<DocumentSlot>,
             context: Res<EditContext>,
             q_settings: Query<&ScenarioNode>| {
                offered_name(&document, &context, &q_settings)
            },
        )
        .expect("the lookup runs");

    assert_eq!(offered, "First Shift Practice");
}

/// Once it has one, the slot's name wins: that is the file it will save back
/// into, whatever the range has since been renamed to.
#[test]
fn a_saved_document_is_offered_the_name_of_its_slot() {
    let mut app = files_app();
    document(&mut app, "Renamed On The Stage");
    app.world_mut().resource_mut::<DocumentSlot>().0 = Some(slot("editor_my_range", "My Range"));

    let offered = app
        .world_mut()
        .run_system_once(
            |document: Res<DocumentSlot>,
             context: Res<EditContext>,
             q_settings: Query<&ScenarioNode>| {
                offered_name(&document, &context, &q_settings)
            },
        )
        .expect("the lookup runs");

    assert_eq!(offered, "My Range");
}
