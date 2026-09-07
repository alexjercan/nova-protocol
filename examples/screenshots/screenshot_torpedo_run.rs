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
/// How far the lens stands off the ordnance pair, meters.
///
/// The frame has to hold the torpedo AND the hull it is diving on, which are
/// [`RUN_IN_CAPTURE_RANGE`] apart. The lens spans 1.47 times its distance at
/// 16:9, so this is that gap, plus the raider's own length, laid across the
/// frame's diagonal with margin at both ends.
#[cfg(feature = "debug")]
const ORDNANCE_STANDOFF: Meters = Meters(185.0);

/// Where between the two the run-in frame is centred, from the raider.
///
/// Short of the midpoint on purpose. The pair lies across the frame's
/// diagonal, so a lens centred between them puts both in the corners and the
/// empty middle carries the picture; biasing back toward the hull brings the
/// ship being shot at to the centre and leaves the torpedo diving in from the
/// top.
#[cfg(feature = "debug")]
const RUN_IN_BIAS: f32 = 0.45;

/// How far short of the raider the run-in frame is shot.
///
/// A proximity fuze goes off [`hollow::TORPEDO_FUZE_RANGE`] out, so this is
/// the last moment there is still a torpedo to photograph, plus enough margin
/// that the write lands before the fuze does.
#[cfg(feature = "debug")]
const RUN_IN_CAPTURE_RANGE: Meters = Meters(hollow::TORPEDO_FUZE_RANGE.get() + 30.0);

/// Where the lens stands, relative to what it is looking at.
///
/// BROADSIDE to the run, and from BELOW. Both halves are the picture:
///
/// - Broadside because the boat, the torpedo and the raider are on one line,
///   and a lens anywhere along it puts the round behind the hull it is aimed
///   at. That is what the old fixed offset did - the run-in still was a
///   corvette filling the frame with its attacker eclipsed dead centre behind
///   it. Standing off the line puts the gap between them ACROSS the frame,
///   which is the only framing in which a run-in reads as a run-in.
/// - From below because the rock field is a horizontal annulus 460 m thick, so
///   any level camera in the hollow frames its subject against the far wall
///   and the shot is rock soup. Tipping the lens up puts open sky behind.
///
/// Derived from the set rather than authored, so it stays broadside if the
/// boat or the raider is ever moved. Both ordnance frames use it, so they are
/// a before/after of the same shot.
#[cfg(feature = "debug")]
fn ordnance_offset() -> Meters3 {
    let run_in = (hollow::LANCE_POSITION - hollow::RAIDER_POSITION)
        .get()
        .normalize();
    let broadside = run_in.cross(Vec3::Y).normalize();
    Meters3((broadside * 0.85 - Vec3::Y * 0.45).normalize() * ORDNANCE_STANDOFF.get())
}

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
        // Framed on the midpoint between the raider and where the fuze will go,
        // NOT on the raider: a proximity fuze detonates 150 m short of its
        // target, which at a close camera throws the blast a third of the way
        // across the frame from the ship it is hitting. The HUD comes down for
        // both ordnance frames - the camera has left the player's ship, so its
        // fps bar and its two contact chevrons are chrome over a picture of
        // someone else's fight.
        .step("frame the torpedo run")
        .on_enter(|world: &mut World| {
            hollow::hud_cinematic(world);
            let subject = hollow::ordnance_subject(world);
            hollow::pose(world, subject + ordnance_offset(), subject)
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
        .until(hollow::torpedo_within(RUN_IN_CAPTURE_RANGE))
        .deadline(12.0)
        .add()
        // Re-framed on the pair as it actually stands: the raider has been
        // drifting since the run was framed and the torpedo is most of a
        // kilometre from where it started, so the midpoint the first framing
        // guessed at is not the midpoint the shot needs.
        .step("re-frame on the pair")
        .on_enter(|world: &mut World| {
            let raider = hollow::raider_position(world);
            let subject = hollow::lead_torpedo_position(world)
                .map_or(raider, |torpedo| raider + (torpedo - raider) * RUN_IN_BIAS);
            hollow::pose(world, subject + ordnance_offset(), subject);
        })
        .until(frames(2))
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
            hollow::pose(world, raider + ordnance_offset(), raider)
        })
        .until(elapsed(0.5))
        .add()
        .step("capture the aftermath")
        .on_enter(move |world| shoot(world, "wiki-combat-aftermath.png"))
        .until(shot_written("wiki-combat-aftermath.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
