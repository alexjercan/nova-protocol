//! screenshot_scenario_picker: the Scenarios picker with a scenario selected,
//! driven through the shipped app (`editor_app`).
//!
//! Ships three manifest images: `wiki-scenarios-picker` (the base game's list,
//! nothing enabled but base), `wiki-first-scenario-picker` (the example mod's
//! arena, for the create docs) and the frozen `news-090-scenario-campaigns`.
//!
//! The walk REPLACES the enabled mod set rather than adding to it. A capture
//! host carries whatever mods its owner installed, and a wiki figure listing
//! them shows the reader a game that does not exist.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - open the picker, select the
//!   rows, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_scenario_picker --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_scenario_picker --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::widget::Selected;

// The pointer gestures, shared with the other menu walks. Script-only, so the
// whole module sits behind one gate here.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::Gestures;

#[derive(Parser)]
#[command(name = "screenshot_scenario_picker")]
#[command(version = "1.0.0")]
#[command(about = "Capture the Scenarios picker with a scenario selected. Autopilot-only: a scripted pointer walk over the real menu", long_about = None)]
struct Cli;

/// The row the Scenarios shot selects: the base game's training range, the one
/// scenario every install lists whatever its enabled mods are.
#[cfg(feature = "debug")]
const BASE_SCENARIO_ROW: &str = "Scenario Row: tutorial";
/// The example mod's arena, the second row the walk selects and the one it
/// plays into.
#[cfg(feature = "debug")]
const EXAMPLE_SCENARIO_ROW: &str = "Scenario Row: example_arena";

/// The wiki's living picker figure. Shot with NO mod enabled: the wiki
/// documents the base game, and a picker row belonging to a mod on a base page
/// tells a reader the game ships something it does not.
#[cfg(feature = "debug")]
const WIKI_PICKER_SHOT: &str = "wiki-scenarios-picker.png";

/// The base game's own enabled set - the merge's baseline, with nothing added.
#[cfg(feature = "debug")]
fn base_only() -> EnabledMods {
    EnabledMods(["base".to_string()].into_iter().collect())
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game/binary runs (main menu over the ambience backdrop).
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets on the menu
            // teardown) into panics so the run fails loudly on them.
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // Clean frames at a known 16:9 size: force the window resolution and drop
        // the dev overlays.
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(picker_script());
    }

    app.run()
}

/// The driven walk: menu -> Scenarios -> a scenario selected.
#[cfg(feature = "debug")]
fn picker_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // The HUD chrome is dropped right before the shot rather than once at
    // `Startup`, because other states re-raise it. `shoot` itself is the capture
    // gate: unarmed, this whole walk runs and writes nothing.
    let shot = |path: &'static str| {
        move |world: &mut World| {
            hide_hud(world);
            shoot(world, path);
        }
    };

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The enabled set is REPLACED, not added to: the capture host has its
        // own installed mods (a developer box has the web mods in its data
        // dir), and a picker listing whichever of those happen to be switched
        // on is neither reproducible nor a picture of the base game. The merge
        // re-runs on an `EnabledMods` change and the picker rebuilds when
        // `GameScenarios` does, so this applies live.
        .step("cut the enabled set back to base")
        .on_enter(|world: &mut World| {
            *world.resource_mut::<EnabledMods>() = base_only();
        })
        .until(frames(SETTLE_FRAMES * 2))
        .add()
        .step("settle the menu and its ambience backdrop")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The Scenarios picker: the scenario list and its details pane.
        .click("open Scenarios", "Scenarios Button")
        .step("settle the scenarios picker")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("select a scenario", BASE_SCENARIO_ROW)
        .step("settle the selected scenario's details pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the training range is the selected row")
        .on_enter(|world: &mut World| {
            // The picker's OWN record of which click landed
            // (`select_scenario_row` inserts `Selected` on the clicked row and
            // removes it from every other), not a restatement of what the beat
            // intended. A missed click leaves the details pane on whatever was
            // selected before, and the shot would still look plausible.
            let selected = world
                .query_filtered::<&Name, With<Selected>>()
                .iter(world)
                .any(|name| name.as_str() == BASE_SCENARIO_ROW);
            assert!(
                selected,
                "the click on `{BASE_SCENARIO_ROW}` never landed: the picker \
                 has not marked that row Selected, so the details pane shows the \
                 PREVIOUS selection"
            );
        })
        .add()
        // The last step holds until the PNG is on disk, so the driver cannot
        // report done out from under a pending write. Two names off this one
        // frame, one shot at a time: the wiki's living figure and the frozen
        // v0.9.0 news evidence. Bevy services one primary-window capture per
        // frame, so they cannot share a step.
        .step("capture the scenarios picker")
        .on_enter(shot(WIKI_PICKER_SHOT))
        .until(shot_written(WIKI_PICKER_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("capture the campaigns news frame")
        .on_enter(shot("news-090-scenario-campaigns.png"))
        .until(shot_written("news-090-scenario-campaigns.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // Only now does a mod enter the picture, and only for the CREATE
        // docs' figure: the example mod is the one a reader authoring their
        // first scenario is following along with.
        .step("enable the example mod")
        .on_enter(|world: &mut World| {
            let mut enabled = base_only();
            enabled.0.insert("example".to_string());
            *world.resource_mut::<EnabledMods>() = enabled;
        })
        .until(frames(SETTLE_FRAMES * 2))
        .add()
        .click("select the example scenario", EXAMPLE_SCENARIO_ROW)
        .step("settle the example scenario details")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the example scenario")
        .on_enter(shot("wiki-first-scenario-picker.png"))
        .until(shot_written("wiki-first-scenario-picker.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("play the example scenario", "Scenario Play Button")
        .step("reach the example arena")
        .until(state_is(GameStates::Playing))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}
