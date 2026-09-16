//! screenshot_training: the Training handbook, the `Lessons` row that opens it,
//! the first-launch prompt in the corner, and a lesson's media frame, driven
//! through the shipped app (`editor_app`).
//!
//! THREE frames per run, and the run is repeated at the supported narrow
//! window with `--narrow`: the handbook is a layout claim, and a layout claim
//! that has only been looked at on a desk monitor has not been checked. The
//! narrow size is nova_core's `MIN_WINDOW_WIDTH` x `MIN_WINDOW_HEIGHT` - the
//! floor below which no layout is asked to cope.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - open the handbook, select a
//!   lesson, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNGs (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU), both widths:
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_training --features debug
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_training --features debug -- --narrow
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, window::PrimaryWindow};
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::nova_debug::harness::CAPTURE_RESOLUTION;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::widget::Selected;

// The pointer gestures, shared with the other menu walks.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::Gestures;

#[derive(Parser)]
#[command(name = "screenshot_training")]
#[command(version = "1.0.0")]
#[command(about = "Capture the Training handbook, the first-pilot card and a lesson visual. Autopilot-only: a scripted pointer walk over the real menu", long_about = None)]
struct Cli {
    /// Shoot at the supported narrow window instead of the capture 16:9.
    #[arg(long)]
    narrow: bool,
}

/// The lesson the visual frame shot selects: a loop tip that also carries a
/// binding chip and a Practice button, so the shot holds every part of the tip
/// layout at once.
#[cfg(feature = "debug")]
const VISUAL_LESSON_ROW: &str = "Lesson Row: flight_aim";

/// The window a `--narrow` run is pinned to: nova_core's native floor.
#[cfg(feature = "debug")]
const NARROW_RESOLUTION: (u32, u32) = (MIN_WINDOW_WIDTH as u32, MIN_WINDOW_HEIGHT as u32);

/// The window every shot in this run is taken at.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct ShotWindow((u32, u32));

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();

    // The same app the game/binary runs (main menu over the ambience backdrop).
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
        app.add_plugins(training_script(cli.narrow));
    }

    let _ = cli;
    app.run()
}

/// Pin the primary window to this run's shot size. The local twin of
/// [`force_capture_resolution`], which pins the fleet's 16:9 and has no way to
/// be told about a second width.
#[cfg(feature = "debug")]
fn force_shot_window(mut windows: Query<&mut Window, With<PrimaryWindow>>, shot: Res<ShotWindow>) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(shot.0 .0 as f32, shot.0 .1 as f32);
        window.resizable = false;
    }
}

/// The driven walk: menu (with the corner prompt up) -> the handbook -> a
/// lesson whose tip is a visual.
///
/// The settings store is inert under the autopilot, so every run is a FRESH
/// install: the prompt is up, and the walk opens the handbook the way a player
/// who already answered it would - through the menu card's `Lessons` row.
#[cfg(feature = "debug")]
fn training_script(
    narrow: bool,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let suffix = if narrow { "-narrow" } else { "" };
    let name = move |stem: &str| format!("{stem}{suffix}.png");
    // The HUD chrome is dropped right before each shot rather than once at
    // Startup, because other states re-raise it. `shoot` is the capture gate:
    // unarmed, this whole walk runs and writes nothing.
    let shot = |path: String| {
        move |world: &mut World| {
            hide_hud(world);
            shoot(world, &path);
        }
    };

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the menu and its ambience backdrop")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The front door: the prompt bottom-left, the menu card opposite.
        .step("the corner prompt and the Lessons row are up")
        .on_enter(|world: &mut World| {
            // `assert_named_visible` resolves a LAID-OUT, inherited-visible
            // node, so a card spawned but hidden behind a modal fails here
            // rather than producing a plausible-looking shot of the menu.
            assert_named_visible("Training Prompt")(world);
            assert_named_visible("Training Prompt Start")(world);
            assert_named_visible("Lessons Button")(world);
            // The corner's second notice, drawn from the SHIPPED catalog: a
            // build whose lessons carry no field note would fail here.
            assert_named_visible("Menu Field Note")(world);
        })
        .add()
        .step("capture the front door")
        .on_enter(shot(name("training-menu")))
        .until(shot_written(name("training-menu")))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("open the handbook", "Lessons Button")
        .step("settle the handbook")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The corner must be OFF the screen behind the modal: the modal is 85
        // percent of the window, so a card at the edge would otherwise be
        // visible and clickable beside a screen that owns the input.
        .step("the modal owns the screen")
        .on_enter(|world: &mut World| {
            assert!(
                ui_node_rect(world, "Menu Aside").is_none(),
                "the bottom-left corner is still laid out and clickable beside \
                 a modal that owns the screen"
            );
        })
        .add()
        .step("capture the lesson list")
        .on_enter(shot(name("training-lessons")))
        .until(shot_written(name("training-lessons")))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("select a lesson with a visual", VISUAL_LESSON_ROW)
        .step("settle the lesson's details pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the visual lesson is the selected row")
        .on_enter(|world: &mut World| {
            // The screen's OWN record of which click landed, not a restatement
            // of what the beat intended: a missed click leaves the pane on the
            // previous lesson and the shot would still look plausible.
            let selected = world
                .query_filtered::<&Name, With<Selected>>()
                .iter(world)
                .any(|name| name.as_str() == VISUAL_LESSON_ROW);
            assert!(
                selected,
                "the click on `{VISUAL_LESSON_ROW}` never landed: the list has \
                 not marked that row Selected, so the details pane shows the \
                 PREVIOUS lesson"
            );
            assert_named_visible("Lesson Media Frame")(world);
        })
        .add()
        .step("capture the lesson visual")
        .on_enter(shot(name("training-lesson-visual")))
        .until(shot_written(name("training-lesson-visual")))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
