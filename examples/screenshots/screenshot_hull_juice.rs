//! screenshot_hull_juice: a section blown off a hull while the rounds are still
//! arriving.
//!
//! Ships `feature-juice.png`. Three-quarter on the raider from inside the fire
//! line, so the frame carries the player's incoming tracers, the impact flashes
//! and the burnt-out section together. The section dies through the production
//! damage path - the same `HealthApplyDamage` a bullet delivers - so the shot is
//! of the real destruction, not of a prop.
//!
//! The destruction is a BURNT HULL, not a fireball: a dead section is graded to
//! `DEAD_COLOR` in place (`sections/damage_cracks.rs`) and the burst is over in a
//! frame or two, so the shot is the damage, shown while the rounds are still
//! arriving.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, drive the whole
//!   script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shot (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_hull_juice --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_hull_juice --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/kit.rs"]
mod kit;

#[path = "shared/hollow.rs"]
mod hollow;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_hull_juice")]
#[command(version = "1.0.0")]
#[command(about = "Capture a section blown off a hull under live fire. Autopilot-only: the framing and the section death are scripted", long_about = None)]
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
        app.add_plugins(hull_juice_script());
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

/// Raise, latch, fire, frame the raider, blow a section off it and shoot.
#[cfg(feature = "debug")]
fn hull_juice_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
        // raider, so the frame has the player's fire in it and not just the AI's.
        .step("open fire")
        .on_enter(hollow::open_fire)
        .until(elapsed(0.8))
        .add()
        .step("frame the raider")
        .on_enter(|world| {
            // Off the LIVE raider, not its spawn point: it drifts, and a stream
            // of turret rounds pushes it further, which throws a close camera
            // clean off the subject.
            let raider = hollow::raider_position(world);
            hollow::pose(world, raider + Vec3::new(6.5, 2.2, 9.0), raider)
        })
        .until(elapsed(0.5))
        .add()
        .step("blow a section off the raider")
        .on_enter(|world| hollow::blow_raider_section(world, hollow::RAIDER_BLOWN_SECTION))
        .until(frames(12))
        .add()
        .step("capture the juice")
        .on_enter(move |world| shoot(world, "feature-juice.png"))
        .until(shot_written("feature-juice.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
