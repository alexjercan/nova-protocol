//! editor_undo: the editor with its undo history on the panel, to edit
//! and take back by hand.
//!
//! Boots the shipped app and opens a scratch mod past the Mod Hub, on a
//! profile folder of its own.
//!
//! Try: Add > Asteroid, drag it, type a new radius, press Del. Each gesture is
//! one step. `Ctrl+Z` takes one back, `Ctrl+Shift+Z` or `Ctrl+Y` puts it back,
//! and the Edit menu names the step each would take. Save with `Ctrl+S`: the
//! UNSAVED mark in the top bar goes, and undoing past the save brings it back.
//! With the mark up, File > Back to Main Menu, the pause menu's way out and
//! closing the window ask first.
//!
//! The panel reads the step counts, the next step each way, and the mark.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk does these gestures through the real
//! keys, menu rows and window close, and asserts the document after each.
//! `NOVA_CAPTURE=1` shoots `editor-undo-menu.png` and `editor-undo-confirm.png`.
//! Unset, no walk and no deadline run.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

#[derive(Parser)]
#[command(name = "editor_undo")]
#[command(version = "1.0.0")]
#[command(
    about = "The editor with its undo history on the panel, to edit and take back by hand",
    long_about = None
)]
struct Cli;

const INTRO: &str = "Edit the stage. Ctrl+Z undoes, Ctrl+Y redoes, Ctrl+S saves.";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(EditorOpensScratch);
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(OnEnter(GameStates::MainMenu), open_the_sandbox);
    app.add_systems(
        PostUpdate,
        read_the_history
            .in_set(diagnostics::DiagnosticsSystems)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
    }
    app.add_plugins(smoke::script());

    app.run()
}

/// What the Sandbox button does, once: a later Back to Main Menu stays on the
/// menu.
fn open_the_sandbox(
    mut opened: Local<bool>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    if !*opened {
        *opened = true;
        *mode = GameMode::Sandbox;
        state.set(GameStates::Playing);
    }
}

/// The diagnostics panel: where the history stands.
fn read_the_history(world: &mut World) {
    let Some(probe) = world.get_resource::<EditorProbe>().cloned() else {
        return;
    };
    let mut panel = world.resource_mut::<diagnostics::DiagnosticsPanel>();
    panel.line(format!(
        "Undo steps: {}{}",
        probe.history_depth,
        if probe.dirty { ", unsaved" } else { ", saved" }
    ));
    panel.line(format!(
        "Next undo: {}",
        probe.next_undo.as_deref().unwrap_or("none")
    ));
    panel.line(format!(
        "Next redo: {}",
        probe.next_redo.as_deref().unwrap_or("none")
    ));
}

mod smoke {
    use std::sync::Arc;

    use bevy::window::{PrimaryWindow, WindowCloseRequested};
    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin, Predicate};

    use super::*;

    /// Seconds a cold boot gets on a software GPU.
    const BOOT_SECS: f32 = 300.0;
    /// Seconds a beat gets.
    const BEAT_SECS: f32 = 60.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    /// The top bar's Add menu, and its group that holds the rock.
    const ADD_MENU: &str = "Add Menu Button";
    const ADD_OBJECTS: &str = "Add Menu Objects";
    const ADD_ASTEROID: &str = "Add Asteroid";
    const EDIT_MENU: &str = "Edit Menu Button";
    const FILE_MENU: &str = "File Menu Button";
    const CONFIRM: &str = "Confirm Window";
    const KEEP: &str = "Confirm Keep Button";
    const MENU_SHOT: &str = "editor-undo-menu.png";
    const CONFIRM_SHOT: &str = "editor-undo-confirm.png";
    const UNDO: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyZ];
    const REDO_SHIFT: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::ShiftLeft, KeyCode::KeyZ];
    const REDO_Y: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyY];
    const SAVE: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyS];

    /// How many nodes the scenario held when the document opened.
    #[derive(Resource)]
    struct Opened(usize);

    /// Advance once the probe, and the node count against the opening one, pass.
    fn editor(
        added: usize,
        check: impl Fn(&EditorProbe) -> bool + Send + Sync + 'static,
    ) -> Arc<Predicate> {
        Arc::new(move |world: &World| {
            let (Some(probe), Some(opened)) = (
                world.get_resource::<EditorProbe>(),
                world.get_resource::<Opened>(),
            ) else {
                return false;
            };
            probe.context_nodes.len() == opened.0 + added && check(probe)
        })
    }

    /// The probe, for a stalled beat to print.
    fn readout(world: &World) -> String {
        world.get_resource::<EditorProbe>().map_or_else(
            || "no probe".to_string(),
            |probe| {
                format!(
                    "nodes {} depth {} redo {} dirty {} next {:?} status {:?}",
                    probe.context_nodes.len(),
                    probe.history_depth,
                    probe.can_redo,
                    probe.dirty,
                    probe.next_undo,
                    probe.status
                )
            },
        )
    }

    /// Hold `keys`, wait for `landed`, let go, and log what the editor reads.
    fn chord(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        keys: &'static [KeyCode],
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("editor undo: {label}"))
            .on_enter(move |world: &mut World| {
                for key in keys {
                    press_key(*key)(world);
                }
            })
            .until(landed)
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step(format!("editor undo: let go after {label}"))
            .on_enter(move |world: &mut World| {
                for key in keys {
                    release_key(*key)(world);
                }
                info!("editor undo: after {}", readout(world));
            })
            .until(frames(2))
            .add()
    }

    /// Drop `menu`, then press `row`, and wait for `landed`.
    fn menu_row(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        menu: &str,
        row: &str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .click_named(
                &format!("editor undo: {label}: open the menu"),
                menu,
                ui_node_present(row),
                BEAT_SECS,
            )
            .click_named(&format!("editor undo: {label}"), row, landed, BEAT_SECS)
    }

    /// Add > Objects > Asteroid, through the menu's submenu.
    fn add_rock(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        menu_row(
            script,
            label,
            ADD_MENU,
            ADD_OBJECTS,
            ui_node_present(ADD_ASTEROID),
        )
        .click_named(
            &format!("editor undo: {label}: the rock"),
            ADD_ASTEROID,
            landed,
            BEAT_SECS,
        )
    }

    /// Advance once the confirm window stands with this title.
    fn question_titled(title: &'static str) -> Arc<Predicate> {
        Arc::new(move |world: &World| {
            ui_node_present(CONFIRM)(world)
                && world
                    .try_query::<&Text>()
                    .is_some_and(|mut texts| texts.iter(world).any(|text| text.0 == title))
        })
    }

    /// Advance once no confirm window stands.
    fn no_question() -> Arc<Predicate> {
        Arc::new(|world: &World| !ui_node_present(CONFIRM)(world))
    }

    /// Ask to close the window, as the title bar does.
    fn close_the_window(world: &mut World) {
        let window = world
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(world)
            .expect("the primary window stands");
        world.write_message(WindowCloseRequested { window });
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("editor undo: the editor is up")
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<EditorProbe>()
                    .is_some_and(|probe| !probe.context_nodes.is_empty())
                    && ui_node_present(ADD_MENU)(world)
            }))
            .deadline(BOOT_SECS)
            .add()
            .step("editor undo: a fresh document has no history and nothing unsaved")
            .on_enter(|world: &mut World| {
                let probe = world.resource::<EditorProbe>().clone();
                assert!(
                    !probe.can_undo && !probe.can_redo && !probe.dirty,
                    "a fresh document: {}",
                    readout(world)
                );
                world.insert_resource(Opened(probe.context_nodes.len()));
            })
            .add();

        let script = add_rock(
            script,
            "add a rock",
            editor(1, |probe| probe.history_depth == 1 && probe.dirty),
        );
        let script = chord(
            script,
            "Ctrl+Z takes the rock back",
            UNDO,
            editor(0, |probe| !probe.can_undo && probe.can_redo && !probe.dirty),
        );
        let script = chord(
            script,
            "Ctrl+Shift+Z puts the rock back",
            REDO_SHIFT,
            editor(1, |probe| probe.history_depth == 1 && !probe.can_redo),
        );
        let script = script
            .click_named(
                "editor undo: drop the Edit menu",
                EDIT_MENU,
                ui_node_present("Undo Item"),
                BEAT_SECS,
            )
            .step("editor undo: the Undo row names the step")
            .until(Arc::new(|world: &World| {
                world.try_query::<&Text>().is_some_and(|mut texts| {
                    texts.iter(world).any(|text| text.0 == "Undo Add Asteroid")
                })
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("editor undo: shoot the Edit menu")
            .on_enter(|world: &mut World| shoot(world, MENU_SHOT))
            .until(shot_written(MENU_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .click_named(
                "editor undo: press the Undo row",
                "Undo Item",
                editor(0, |probe| !probe.can_undo && !probe.dirty),
                BEAT_SECS,
            );
        let script = chord(
            script,
            "Ctrl+Y puts the rock back",
            REDO_Y,
            editor(1, |probe| probe.history_depth == 1),
        );
        // The selection the Add left is the second rock, so Del takes that one.
        let script = add_rock(
            script,
            "add a second rock",
            editor(2, |probe| probe.history_depth == 2),
        );
        let script = chord(
            script,
            "Del takes the rock off",
            &[KeyCode::Delete],
            editor(1, |probe| probe.history_depth == 3),
        );
        let script = chord(
            script,
            "Ctrl+Z brings the deleted rock back",
            UNDO,
            editor(2, |probe| probe.history_depth == 2 && probe.can_redo),
        );
        let script = chord(
            script,
            "Ctrl+S asks for a name",
            SAVE,
            ui_node_present("File Save Button"),
        );
        let script = script
            .click_named(
                "editor undo: accept the offered name",
                "File Save Button",
                frames(2),
                BEAT_SECS,
            )
            .step("editor undo: the save reads saved")
            .until(editor(2, |probe| {
                !probe.dirty && probe.status.starts_with("saved")
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor undo: the save kept the history")
            .on_enter(|world: &mut World| {
                let probe = world.resource::<EditorProbe>();
                assert_eq!(probe.history_depth, 2);
                assert_eq!(probe.next_undo.as_deref(), Some("Add Asteroid"));
            })
            .add();
        let script = chord(
            script,
            "undoing past the save reads unsaved",
            UNDO,
            editor(1, |probe| probe.dirty),
        );
        let script = chord(
            script,
            "redoing to the save reads saved",
            REDO_Y,
            editor(2, |probe| !probe.dirty),
        );
        let script = chord(
            script,
            "Ctrl+Z leaves unsaved work for the guards",
            UNDO,
            editor(1, |probe| probe.dirty),
        );
        let script = menu_row(
            script,
            "File > Back to Main Menu with unsaved work",
            FILE_MENU,
            "Back To Main Menu Item",
            ui_node_present(CONFIRM),
        );
        let script = script
            .step("editor undo: shoot the question")
            .on_enter(|world: &mut World| shoot(world, CONFIRM_SHOT))
            .until(shot_written(CONFIRM_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .click_named("editor undo: keep the work", KEEP, no_question(), BEAT_SECS)
            .step("editor undo: closing the window asks too")
            .on_enter(close_the_window)
            .until(question_titled("EXIT"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "editor undo: keep the work again",
                KEEP,
                no_question(),
                BEAT_SECS,
            );
        let script = chord(
            script,
            "Esc opens the pause menu",
            &[KeyCode::Escape],
            ui_node_present("Back To Menu Button"),
        );
        script
            .click_named(
                "editor undo: the pause menu's way out asks too",
                "Back To Menu Button",
                question_titled("BACK TO MAIN MENU"),
                BEAT_SECS,
            )
            .click_named(
                "editor undo: keep the work a third time",
                KEEP,
                no_question(),
                BEAT_SECS,
            )
            .step("editor undo: the walk stands in the unpaused editor, work intact")
            .until(editor(1, |probe| probe.dirty))
            .deadline(BEAT_SECS)
            .add()
            .step("editor undo: judge the end state")
            .on_enter(|world: &mut World| {
                assert_eq!(
                    *world.resource::<State<GameStates>>().get(),
                    GameStates::Playing
                );
                assert_eq!(
                    *world.resource::<State<PauseStates>>().get(),
                    PauseStates::Unpaused
                );
                info!("editor undo: PASS {}", readout(world));
            })
            .add()
    }
}
