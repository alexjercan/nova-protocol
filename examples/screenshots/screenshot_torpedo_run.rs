//! screenshot_torpedo_run: a guided salvo diving onto a corvette, and what it
//! left.
//!
//! Ships `wiki-combat-torpedo.png` (the salvo inbound, a beat before the fuze)
//! and `wiki-combat-aftermath.png` (the burst, the debris and a hull short some
//! sections).
//!
//! The set is the Rock hollow with only three ships in it - the player's camera
//! rig, the raider and the torpedo boat - so nothing but the salvo is moving.
//! The script pulls the boat's trigger and commits the salvo; the bay, the
//! projectile, the guidance, the fuze and the blast are all the production path.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, drive the whole
//!   script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shots (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_torpedo_run --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_torpedo_run --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/hollow.rs"]
mod hollow;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_torpedo_run")]
#[command(version = "1.0.0")]
#[command(about = "Capture a guided torpedo salvo and its aftermath. Autopilot-only: the launch, the commit and the framing are scripted", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(torpedo_run_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(hollow::ordnance_hollow(&game_assets, &ships)));
}

/// Frame the run, loose the salvo, shoot it inbound, then shoot what the blast
/// left.
///
/// Every capture is its OWN step held until the PNG is on disk: Bevy services
/// one primary-window capture per frame, so the rule is structural here rather
/// than a guard inside a shared step.
#[cfg(feature = "debug")]
fn torpedo_run_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the ordnance hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the ordnance hollow")
        .until(frames(12))
        .add()
        // One camera for both ordnance frames, so they are a before/after of the
        // same shot. It is framed on the midpoint between the raider and where
        // the fuze will go, NOT on the raider: a proximity fuze detonates 150 m
        // short of its target, which at a close camera throws the blast a
        // third of the way across the frame from the ship it is hitting.
        .step("frame the torpedo run")
        .on_enter(|world| {
            let subject = hollow::ordnance_subject(world);
            // From BELOW, looking up the run. The rock field is a horizontal
            // annulus 460 m thick, so any level camera in the hollow frames
            // its subject against the far wall and the shot is rock soup;
            // tipping the lens up puts open sky behind the target and the
            // torpedo dives into frame.
            hollow::pose(world, subject + Meters3::new(160.0, -140.0, 120.0), subject)
        })
        .until(elapsed(0.4))
        .add()
        .step("loose the torpedoes")
        .on_enter(hollow::loose_torpedoes)
        .until(hollow::torpedo_salvo_in_flight(
            hollow::EXPECTED_TORPEDO_COUNT,
        ))
        .deadline(6.0)
        .add()
        // The salvo is committed to the raider the frame after launch, the way
        // both production commit systems do it, and the trigger drops so the
        // boat fires once.
        .step("commit the salvo")
        .on_enter(hollow::commit_torpedoes)
        .until(elapsed(0.1))
        .add()
        // Inbound: a beat before the fuze, with the drive still lit and the
        // target intact.
        .step("track the torpedoes in")
        .each(hollow::assert_salvo_still_live)
        .until(hollow::torpedo_within(
            hollow::TORPEDO_FUZE_RANGE + Meters(80.0),
        ))
        .deadline(12.0)
        .add()
        .step("capture the torpedo run")
        .on_enter(move |world| shoot(world, "wiki-combat-torpedo.png"))
        .until(shot_written("wiki-combat-torpedo.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("wait for the detonation")
        .until(hollow::no_torpedo_in_flight())
        .deadline(8.0)
        .add()
        // The AFTERMATH, not the flash. The vacuum burst is intentionally too
        // brief to make a reliable still target. Half a second later its hot
        // ejecta is gone and what it did is not: tumbling debris and a hull
        // short some sections.
        .step("let the blast clear")
        .on_enter(|world| {
            hollow::blow_raider_section(world, hollow::RAIDER_BLAST_SECTION);
            // Re-framed off the LIVE raider: it has been drifting since the run
            // was framed, and a rock in the wall behind it is one drift away
            // from being in front of it.
            let raider = hollow::raider_position(world);
            hollow::pose(world, raider + Meters3::new(160.0, -140.0, 120.0), raider)
        })
        .until(elapsed(0.5))
        .add()
        .step("capture the aftermath")
        .on_enter(move |world| shoot(world, "wiki-combat-aftermath.png"))
        .until(shot_written("wiki-combat-aftermath.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
