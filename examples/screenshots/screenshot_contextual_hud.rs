//! screenshot_contextual_hud: the HUD with nothing to say.
//!
//! Ships `news-090-contextual-hud.png`: the player parked in the Rock hollow
//! with weapons lowered and no lock, so the contextual rules keep the idle
//! chrome off the frame. That quiet stance IS the shot - the fight around it is
//! there to prove the HUD is holding back rather than switched off.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, drive the whole
//!   script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shot (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_contextual_hud --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_contextual_hud --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/hollow.rs"]
mod hollow;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_contextual_hud")]
#[command(version = "1.0.0")]
#[command(about = "Capture the contextual HUD with weapons lowered and no lock. Autopilot-only: the quiet stance is scripted", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PERF_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(contextual_hud_script());
        // Only under the script (`NOVA_AUTOPILOT`, named literally - the
        // harness re-exports `CAPTURE_ENV` but not the autopilot's own): a
        // plain run is the owner flying this set, and a pinned ship cannot be
        // flown.
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.add_systems(
                Update,
                hollow::pin_player.run_if(resource_exists::<hollow::HoldStation>),
            );
        }
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    ships: Res<GameShips>,
) {
    commands.trigger(LoadScenario(hollow::ambush_hollow(
        &game_assets,
        &sections,
        &ships,
    )));
}

/// Settle the hollow, then shoot the idle HUD.
#[cfg(feature = "debug")]
fn contextual_hud_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        // Hold the player on the station the set is measured from, drift the
        // lock subject, and give the AI flights their engage grace plus a beat:
        // a fight that opens on parked hulls reads as a diorama.
        .step("settle the hollow")
        .on_enter(|world| {
            hollow::hold_station(world);
            hollow::nudge_raider(world);
        })
        .until(elapsed(hollow::ENGAGE_DELAY + 1.5))
        .add()
        .step("capture the contextual HUD")
        .on_enter(move |world| {
            hollow::hud_instrument(world);
            shoot(world, "news-090-contextual-hud.png");
        })
        .until(shot_written("news-090-contextual-hud.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
