//! screenshot_scenario_picker: the Scenarios picker with a campaign chapter
//! selected (`news-090-scenario-campaigns.png`), driven through the shipped app
//! (`editor_app`).
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - open the picker, select the
//!   chapter, exit clean, capturing nothing.
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
use ui_walk::{Gestures, STEP_DEADLINE_SECS};

#[derive(Parser)]
#[command(name = "screenshot_scenario_picker")]
#[command(version = "1.0.0")]
#[command(about = "Capture the Scenarios picker with a campaign chapter selected. Autopilot-only: a scripted pointer walk over the real menu", long_about = None)]
struct Cli;

/// The campaign chapter the Scenarios shot selects.
///
/// The picker's own subject in that figure is the CAMPAIGN grouping - the `[-]`
/// header with its chapters indented under it - so the selection has to be a
/// chapter rather than one of the uncampaigned scenarios in the tail, and a
/// chapter partway down rather than the first, so the header above it is
/// visibly its parent. `assets/base/campaigns/nova_protocol.content.ron` lists
/// it; campaigns render expanded unless collapsed, so nothing has to open it.
#[cfg(feature = "debug")]
const CAMPAIGN_CHAPTER_ROW: &str = "Scenario Row: broadside";

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

/// The driven walk: menu -> Scenarios -> a campaign chapter selected.
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
        .step("settle the menu and its ambience backdrop")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The Scenarios picker: the campaign header and its indented chapters.
        .click("open Scenarios", "Scenarios Button")
        .step("settle the scenarios picker")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("select a campaign chapter", CAMPAIGN_CHAPTER_ROW)
        .step("settle the selected chapter's details pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the chapter is the selected row")
        .on_enter(|world: &mut World| {
            // The picker's OWN record of which click landed
            // (`select_scenario_row` inserts `Selected` on the clicked row and
            // removes it from every other), not a restatement of what the beat
            // intended. A missed click leaves the details pane on whatever was
            // selected before, and the shot would still look plausible.
            let selected = world
                .query_filtered::<&Name, With<Selected>>()
                .iter(world)
                .any(|name| name.as_str() == CAMPAIGN_CHAPTER_ROW);
            assert!(
                selected,
                "the click on `{CAMPAIGN_CHAPTER_ROW}` never landed: the picker \
                 has not marked that row Selected, so the details pane shows the \
                 PREVIOUS selection"
            );
        })
        .until(frames(1))
        .add()
        // The last step holds until the PNG is on disk, so the driver cannot
        // report done out from under a pending write.
        .step("capture the scenarios picker")
        .on_enter(shot("news-090-scenario-campaigns.png"))
        .until(shot_written("news-090-scenario-campaigns.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
