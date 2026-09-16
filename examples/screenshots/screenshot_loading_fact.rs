//! screenshot_loading_fact: the switch that owns the corner prompt, and the
//! scenario loading screen the prompt's own button hands off to.
//!
//! TWO frames, and the prompt is what joins them. Settings > Interface is
//! where a player turns the first-launch prompt back on, so frame one is that
//! row; frame two is where pressing `Start Basic Training` actually goes. The
//! second frame asks whether a two-line fact sits under the sweep track
//! without moving the panel that was there before it, and whether the fact is
//! one the DESTINATION teaches rather than any note in the book. `--narrow`
//! repeats both at the supported narrow window.
//!
//! The settings store is INERT under the autopilot, so this walk reads and
//! writes nothing of the developer's own profile even though it presses a
//! button that changes a setting.
//!
//! The SCENARIO loading screen rather than the boot one, because this walk can
//! ask for it: `Start Training` hands off to the base bundle's declared start,
//! and the panel is up for as long as that scenario is settling. The boot panel
//! wears the same slot from the same builder, so what is checked here is what
//! boot draws.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach the loading screen, exit
//!   clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_loading_fact --features debug
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_loading_fact --features debug -- --narrow
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, window::PrimaryWindow};
use clap::Parser;
#[cfg(feature = "debug")]
use nova_autopilot::predicate;
#[cfg(feature = "debug")]
use nova_protocol::nova_debug::harness::CAPTURE_RESOLUTION;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_training::prelude::{notes_for_scenario, TrainingCatalog};

#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::Gestures;

#[derive(Parser)]
#[command(name = "screenshot_loading_fact")]
#[command(version = "1.0.0")]
#[command(about = "Capture the training-prompt setting row and the loading screen's field-note slot. Autopilot-only: a scripted pointer walk over the real menu", long_about = None)]
struct Cli {
    /// Shoot at the supported narrow window instead of the capture 16:9.
    #[arg(long)]
    narrow: bool,
}

/// The note slot spawned into both loading panels.
#[cfg(feature = "debug")]
const NOTE_SLOT: &str = "Loading Field Note";

#[cfg(feature = "debug")]
const NARROW_RESOLUTION: (u32, u32) = (MIN_WINDOW_WIDTH as u32, MIN_WINDOW_HEIGHT as u32);

#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct ShotWindow((u32, u32));

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();

    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.insert_resource(ShotWindow(if cli.narrow {
            NARROW_RESOLUTION
        } else {
            CAPTURE_RESOLUTION
        }));
        app.add_systems(Startup, (force_shot_window, hide_dev_overlays));
        app.add_plugins(loading_script(cli.narrow));
    }

    let _ = cli;
    app.run()
}

#[cfg(feature = "debug")]
fn force_shot_window(mut windows: Query<&mut Window, With<PrimaryWindow>>, shot: Res<ShotWindow>) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(shot.0 .0 as f32, shot.0 .1 as f32);
        window.resizable = false;
    }
}

/// The note on screen must be one the DESTINATION teaches.
///
/// The destination is read from the declared start rather than named here: this
/// walk pressed `Start Basic Training`, which is the same hand-off New Game
/// takes, and an example that spelled a content id would keep passing after the
/// bundle renamed its start.
#[cfg(feature = "debug")]
fn assert_the_note_belongs_to_the_destination(world: &mut World) {
    let destination = world
        .resource::<NewGameStart>()
        .0
        .clone()
        .expect("the base bundle declares a start");
    let wanted = notes_for_scenario(world.resource::<TrainingCatalog>(), &destination);
    assert!(
        !wanted.is_empty(),
        "no lesson teaches '{destination}', so this walk cannot check destination context"
    );

    let drawn = drawn_note_lines(world);
    assert!(
        wanted.iter().any(|note| note.lines == drawn),
        "the note on screen is not one of the lessons '{destination}' teaches: {drawn:?}"
    );
}

/// The note's own lines, without the slot's `FIELD NOTE` header.
#[cfg(feature = "debug")]
fn drawn_note_lines(world: &mut World) -> Vec<String> {
    let slot = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, name)| name.as_str() == NOTE_SLOT)
        .map(|(entity, _)| entity)
        .expect("the note slot is up");
    let children: Vec<Entity> = world
        .get::<Children>(slot)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .filter_map(|child| world.get::<Text>(child).map(|text| text.0.clone()))
        .filter(|line| line != "FIELD NOTE")
        .collect()
}

#[cfg(feature = "debug")]
fn loading_script(narrow: bool) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let suffix = if narrow { "-narrow" } else { "" };
    let row = format!("training-prompt-setting{suffix}.png");
    let path = format!("training-loading-fact{suffix}.png");
    let row_shot = row.clone();

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the menu")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the offer is up")
        .on_enter(|world: &mut World| {
            // A LAID-OUT, inherited-visible node, so a prompt that spawned
            // hidden fails here rather than producing a plausible-looking shot.
            assert_named_visible("Training Prompt Start")(world);
            assert_named_visible("Training Prompt Dismiss")(world);
        })
        .add()
        .click("open Settings", "Settings Button")
        .click("open the Interface tab", "Settings Tab: Interface")
        .step("settle the Interface tab")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the training-prompt row is on the tab")
        .on_enter(|world: &mut World| {
            // The switch the prompt is remembered by. Without this row,
            // answering the prompt is a one-way door.
            assert_named_visible("Training Prompt Row")(world);
            assert_named_visible("Training Prompt On")(world);
            assert_named_visible("Training Prompt Off")(world);
        })
        .add()
        .step("capture the training-prompt switch")
        .on_enter(move |world: &mut World| {
            hide_hud(world);
            shoot(world, &row_shot);
        })
        .until(shot_written(row))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("close Settings", "Settings Back Button")
        .step("settle the menu")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The prompt's own action, so this also proves the route a new player
        // takes reaches a scenario at all.
        .click("start basic training", "Training Prompt Start")
        .step("the loading screen raises its note slot")
        .until(ui_node_present(NOTE_SLOT))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("the note is about where the load is going")
        .on_enter(assert_the_note_belongs_to_the_destination)
        .add()
        // The shot goes in the same breath as the check: the panel comes down
        // on the scenario's own settle test, and nothing here is allowed to
        // hold it up for the picture.
        .step("capture the loading fact")
        .on_enter({
            let path = path.clone();
            move |world: &mut World| {
                assert!(
                    *world.resource::<State<GameStates>>().get() == GameStates::Playing,
                    "the loading screen is a gameplay surface; this is not Playing"
                );
                shoot(world, &path);
            }
        })
        .until(shot_written(path))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("the scenario arrives and the panel comes down")
        // `predicate::not`, spelled out: bevy's prelude carries a run-condition
        // combinator of the same name and this file globs both.
        .until(predicate::not(ui_node_present(NOTE_SLOT)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}
