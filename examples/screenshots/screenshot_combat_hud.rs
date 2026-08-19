//! screenshot_combat_hud: the HUD showcase, shot over the ship's shoulder mid
//! fight.
//!
//! Ships `feature-hud.png` (the near shoulder, hull in frame) and
//! `wiki-hud.png` (the wider reference from the other side). Both are the same
//! beat - weapons up, combat lock latched, the player's turret firing - from two
//! pinned cameras, with every situational readout up.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, drive the whole
//!   script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shots (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_combat_hud --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_combat_hud --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/hollow.rs"]
mod hollow;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_combat_hud")]
#[command(version = "1.0.0")]
#[command(about = "Capture the HUD showcase over the ship's shoulder in a fight. Autopilot-only: both framings are scripted camera poses", long_about = None)]
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
        app.add_plugins(combat_hud_script());
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

/// Raise, latch, fire, then pose and shoot each showcase framing.
///
/// Every capture is its OWN step held until the PNG is on disk: Bevy services
/// one primary-window capture per frame, so the rule is structural here rather
/// than a guard inside a shared step.
#[cfg(feature = "debug")]
fn combat_hud_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
        // Raise weapons (RMB), then hold radar (CTRL) a beat later - the natural
        // order. At the hold threshold the radar latches the combat slot on the
        // raider and the reticle + inset come up.
        .step("raise the weapons")
        .on_enter(hollow::raise_stance)
        .until(elapsed(0.3))
        .add()
        .step("latch the combat lock")
        .on_enter(hollow::hold_radar)
        .until(elapsed(1.8))
        .add()
        // Guns live: the player's own turret streams tracers at the locked
        // raider, so the frames have the player's fire in them and not just the
        // AI's.
        .step("open fire")
        .on_enter(hollow::open_fire)
        .until(elapsed(0.8))
        .add()
        // The fight from the ship's shoulder: every situational readout up with
        // the hull in frame.
        .step("frame the HUD showcase")
        .on_enter(|world| hollow::pose(world, Vec3::new(5.0, 1.6, 7.0), Vec3::new(0.0, 0.4, -14.0)))
        .until(elapsed(0.4))
        .add()
        .step("capture the HUD in combat")
        .on_enter(move |world| {
            hollow::hud_instrument(world);
            shoot(world, "feature-hud.png");
        })
        .until(shot_written("feature-hud.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("frame the HUD reference")
        .on_enter(|world| {
            hollow::pose(world, Vec3::new(-6.0, 2.8, 5.0), Vec3::new(0.0, 0.2, -16.0))
        })
        .until(elapsed(0.4))
        .add()
        .step("capture the HUD reference")
        .on_enter(move |world| {
            hollow::hud_instrument(world);
            shoot(world, "wiki-hud.png");
        })
        .until(shot_written("wiki-hud.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
