//! The file window as a LIVE WINDOW: what a typed name derives, and how the
//! window comes up and goes down.

use nova_ui::prelude::TextFieldSubmitted;

use super::*;
use crate::{project::ModProject, template::ScenarioTemplate, ui::window::window_layer};

/// The layer a window stands on, and the systems that keep the readout in step.
fn files_app() -> App {
    let mut app = App::new();
    app.init_resource::<ActiveUiTheme>();
    app.init_resource::<FileWindowRequest>();
    app.init_resource::<crate::config::EditorStatus>();
    app.init_resource::<Time>();
    app.add_message::<TextFieldSubmitted>();
    app.insert_resource(ModProject::new(
        None,
        "My Range",
        "",
        ScenarioTemplate::Empty,
    ));
    // A window entity, because a floating window is placed against the screen
    // it stands on.
    app.world_mut().spawn(Window::default());
    app.world_mut().spawn(window_layer());
    app.add_systems(Update, (open_file_window, sync_save_name).chain());
    app.add_observer(close_file_window);
    app
}

fn named(app: &mut App, name: &str) -> Option<Entity> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, named)| named.as_str() == name)
        .map(|(entity, _)| entity)
}

fn ask(app: &mut App, kind: FileWindowKind) {
    app.world_mut().resource_mut::<FileWindowRequest>().0 = Some(kind);
    app.update();
    app.update();
}

/// A typed name is a file name, so the window says what it becomes before Save
/// is pressed, under both the id it saves as and the id the portal will take.
#[test]
fn a_name_says_the_id_it_saves_and_publishes_as() {
    let (line, _, usable) = id_line("Red Run", &[]);

    assert!(usable);
    assert_eq!(line, "saves as editor_red-run, publishes as red-run");
}

/// Two names that differ only in punctuation land on one directory, and a
/// second mod there would write over the first.
#[test]
fn a_name_that_is_already_a_mod_is_refused() {
    let known = vec!["editor_red_run".to_string()];

    let (line, _, usable) = id_line("Red, Run!", &known);

    assert!(!usable);
    assert!(line.contains("editor_red-run"), "{line}");
}

/// A name with nothing an id can hold has nothing to save under.
#[test]
fn a_name_with_nothing_in_it_is_refused() {
    let (_, _, usable) = id_line("  !!  ", &[]);

    assert!(!usable);
}

/// The window opens on the mod's own name, so a save needs no retyping.
#[test]
fn naming_a_mod_offers_the_name_it_has() {
    let mut app = files_app();

    ask(&mut app, FileWindowKind::NameMod);

    let field = named(&mut app, "Save Name Field").expect("the window has a field");
    assert_eq!(
        app.world().get::<TextFieldValue>(field).expect("a field").0,
        "My Range"
    );
}

/// Cancel takes the window down and changes nothing.
#[test]
fn cancel_takes_the_window_down() {
    let mut app = files_app();
    ask(&mut app, FileWindowKind::NameMod);
    let cancel = named(&mut app, "File Cancel Button").expect("the window can be refused");

    app.world_mut().trigger(Activate { entity: cancel });
    app.update();

    assert!(named(&mut app, "File Window").is_none());
    assert_eq!(app.world().resource::<ModProject>().id, None);
}

/// A key held over a frame boundary must not stack a second window.
#[test]
fn a_second_ask_does_not_stack_a_second_window() {
    let mut app = files_app();
    ask(&mut app, FileWindowKind::NameMod);
    ask(&mut app, FileWindowKind::NameMod);

    let windows = app
        .world_mut()
        .query_filtered::<Entity, With<FileWindow>>()
        .iter(app.world())
        .count();
    assert_eq!(windows, 1);
}
