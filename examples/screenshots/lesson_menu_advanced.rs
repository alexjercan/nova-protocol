//! lesson_menu_advanced: the three ADVANCED demonstrations, which are pictures
//! of the game's own screens - `advanced_scenarios`, `advanced_mods` and
//! `advanced_bindings`.
//!
//! One producer, three stills, because they are one walk: a pointer crossing
//! the main menu, opening each screen the lesson is about and shooting it.
//! Splitting them would be three builds and three boots of the same app to
//! photograph three panels of it. Three is also the producer cap (see the
//! example catalog in `Cargo.toml`), which is why the handbook's own picture is
//! `lesson_start_welcome` rather than a fourth frame here.
//!
//! The walk is a POINTER walk over the real menu (`shared/ui_walk.rs`), not a
//! set of posed panels: a screen that would not open for a player does not open
//! here either, and each shot is asserted against the screen's own record of
//! what is selected before it is taken.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - walk every screen, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNGs (staged under
//!   `NOVA_CAPTURE_DIR`). `scripts/capture-lesson-media.sh` runs this and
//!   encodes each one as the WebP the handbook ships.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_menu_advanced --features debug
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other menu walks.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use nova_ui::widget::Selected;
#[cfg(feature = "debug")]
use ui_walk::{hide_menu_version, Gestures};

#[derive(Parser)]
#[command(name = "lesson_menu_advanced")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's three advanced screen demonstrations", long_about = None)]
struct Cli;

/// `advanced_scenarios`: the scenario picker with a scenario chosen.
#[cfg(feature = "debug")]
const SCENARIOS_SHOT: &str = "advanced_scenarios.png";
/// `advanced_mods`: the mods screen with an enabled mod chosen.
#[cfg(feature = "debug")]
const MODS_SHOT: &str = "advanced_mods.png";
/// `advanced_bindings`: Settings on the tab that rebinds a control.
#[cfg(feature = "debug")]
const BINDINGS_SHOT: &str = "advanced_bindings.png";

/// The scenario row the picker shot selects: the training range, which every
/// install has and no mod is needed to see.
#[cfg(feature = "debug")]
const SCENARIO_ROW: &str = "Scenario Row: tutorial";

/// The mod row the mods shot selects: THE BASE GAME, which is the lesson's own
/// claim - the game is itself a mod, and it is the row a fresh install has.
#[cfg(feature = "debug")]
const MOD_ROW: &str = "Mod Row: base";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game binary runs: the main menu over its live backdrop.
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(menu_advanced_script());
    }

    app.run()
}

/// Assert the named row is the one the screen has marked `Selected`.
///
/// The screen's OWN record, not a restatement of what the beat intended: a
/// missed click leaves the details pane on the previous row, and the shot would
/// still look plausible.
#[cfg(feature = "debug")]
fn the_selected_row_is(name: &'static str) -> impl Fn(&mut World) {
    move |world: &mut World| {
        let landed = world
            .query_filtered::<&Name, With<Selected>>()
            .iter(world)
            .any(|selected| selected.as_str() == name);
        assert!(
            landed,
            "the click on `{name}` never landed: the screen has not marked that \
             row Selected, so the details pane still shows the PREVIOUS one"
        );
    }
}

/// Menu -> scenarios -> mods -> settings, shooting each screen the lesson that
/// names it is about.
#[cfg(feature = "debug")]
fn menu_advanced_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // The chrome is dropped right before each shot rather than once at Startup,
    // because other states re-raise it. `shoot` is the capture gate: unarmed,
    // this whole walk runs and writes nothing.
    let shot = |path: &'static str| {
        move |world: &mut World| {
            hide_hud(world);
            hide_menu_version(world);
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
        // ADVANCED: "Scenarios and campaigns" - the picker, with the row a
        // fresh install can select.
        .click("open the scenarios picker", "Scenarios Button")
        .step("settle the picker")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("select a scenario", SCENARIO_ROW)
        .step("settle the scenario's details pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the picker is open on that scenario")
        .on_enter(|world: &mut World| {
            the_selected_row_is(SCENARIO_ROW)(world);
            assert_named_visible("Scenario Details Name")(world);
        })
        .add()
        .step("capture the scenarios picker")
        .on_enter(shot(SCENARIOS_SHOT))
        .until(shot_written(SCENARIOS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("leave the picker", "Scenarios Back Button")
        // ADVANCED: "Mods" - the installed list, on the row that IS the base
        // game.
        .click("open the mods screen", "Mods Button")
        .step("settle the mods screen")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("select the base game's own mod", MOD_ROW)
        .step("settle the mod's details pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the mods screen is open on that mod")
        .on_enter(the_selected_row_is(MOD_ROW))
        .add()
        .step("capture the mods screen")
        .on_enter(shot(MODS_SHOT))
        .until(shot_written(MODS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("leave the mods screen", "Mods Back Button")
        // ADVANCED: "Rebinding controls" - Settings on the Controls tab, which
        // is the tab the lesson tells the reader to open.
        .click("open Settings", "Settings Button")
        .click("open the Controls tab", "Settings Tab: Controls")
        .step("settle the controls tab")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the bindings are on the tab")
        .on_enter(|world: &mut World| {
            assert_named_visible("Settings Controls Header")(world);
        })
        .add()
        .step("capture the bindings tab")
        .on_enter(shot(BINDINGS_SHOT))
        .until(shot_written(BINDINGS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // Out through the menu's own front door. A harnessed example owes the
        // smoke contract either way: a run that ends in a settings modal cannot
        // be told from an app that died while still loading, which is what
        // `reached_playing` is there to catch.
        .click("close Settings", "Settings Back Button")
        .click("start a new game", "New Game Button")
        .click("create the world", "Create World Button")
        .step("reach the first flight")
        .until(state_is(GameStates::Playing))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}
