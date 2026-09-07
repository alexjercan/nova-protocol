//! loop_cockpit: a quiet flight view gains live GOTO instruments, then opens
//! NOVA OS and its map on the same ship and target.
//!
//! Every transition uses the production input path. The script only presses
//! the controls, records the result, and exits.

#[path = "shared/computer.rs"]
mod computer;
#[path = "shared/ring.rs"]
mod ring;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use computer::{press_enter, press_tab, type_word};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_cockpit")]
#[command(version = "1.0.0")]
#[command(about = "Capture the contextual cockpit opening NOVA OS on its live GOTO target")]
struct Cli;

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "landing-cockpit";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(cockpit_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
    #[cfg(feature = "debug")]
    app.add_systems(Update, ring::drive_leg_camera);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(ring::the_ring(&game_assets, &ships)));
}

/// Put the ice world behind the hull, and keep it there for the whole flight.
///
/// The side offset is NEGATIVE, which is the whole point of this framing.
/// `ring::lit_side` is the direction the key light comes from, and on this set
/// that direction is INBOARD - the same side of the ship the planetoid is on -
/// so every positive-side framing in the ring producers looks outboard at 3 km
/// of empty sky, which is what made this loop eleven seconds of a small hull
/// on black. Standing outboard instead turns the lens back down the orbit
/// radius: the body fills two thirds of the frame height at this range, the
/// ship sits in front of it, and the 16000 lux rim - whose horizontal bearing
/// IS the start radial - is now behind the camera lighting the hull rather
/// than behind the hull silhouetting it.
///
/// The stand-off is the house number for this set: the lens spans 1.47 times
/// its distance at 16:9, so 150 m puts a 110 m hull across half the frame.
#[cfg(feature = "debug")]
fn frame_against_the_world(world: &mut World) {
    ring::leg(
        world,
        Meters(-150.0),
        Meters(35.0),
        Meters(85.0),
        Meters(25.0),
    );
}

/// Re-frame for the climb to the beacon, where the resting bearing collapses.
///
/// A leg camera's `up` is WORLD Y while its `along` is the ship's TRACK, and the
/// beacon is 7.6 km straight up: once the travel computer has the ship pointed
/// at it the track is five sixths of the way onto Y, the 85 m of `up` stops
/// being height and becomes more lead, and the lens ends up looking down the
/// hull's own length from above - a 110 m ship reads as its 30 m beam. Putting
/// nearly the whole stand-off on `side`, which [`ring::lit_side`] guarantees is
/// perpendicular to the track whatever the track is, keeps the departure
/// broadside and the drive plume across the frame.
#[cfg(feature = "debug")]
fn frame_the_climb(world: &mut World) {
    ring::leg(
        world,
        Meters(-155.0),
        Meters(-35.0),
        Meters(20.0),
        Meters(30.0),
    );
}

#[cfg(feature = "debug")]
fn cockpit_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the quiet cockpit")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle without a maneuver")
        .on_enter(|world: &mut World| {
            ring::hud_instrument(world);
            frame_against_the_world(world);
        })
        .until(elapsed(0.8))
        .add()
        .step("open the cockpit loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .add()
        .step("hold the quiet view")
        .until(elapsed(0.7))
        .add()
        .step("engage GOTO")
        .on_enter(|world: &mut World| {
            ring::engage_goto(world);
            frame_the_climb(world);
        })
        .until(ring::player_burning())
        .deadline(30.0)
        .add()
        .step("read the live maneuver")
        .until(elapsed(1.0))
        .add()
        .step("open NOVA OS")
        .on_enter(press_tab)
        .until(elapsed(0.8))
        .add()
        .step("type the map command")
        .on_enter(|world| type_word(world, "map"))
        .until(frames(6))
        .add()
        .step("launch the map")
        .on_enter(press_enter)
        .until(elapsed(1.5))
        .add()
        .step("close the cockpit loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}
