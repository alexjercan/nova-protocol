//! loop_goto_arrival: one complete GOTO arrival, from the departure burn
//! through the retrograde flip to the settled braking plume.
//!
//! The production flight computer flies the ship. The script only engages it,
//! cuts between fixed world-space cameras during arrival, records, and exits.

#[path = "shared/ring.rs"]
mod ring;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_goto_arrival")]
#[command(version = "1.0.0")]
#[command(about = "Capture a complete GOTO arrival flown by the production autopilot")]
struct Cli;

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "goto-arrival";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(arrival_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, (ring::drive_leg_camera, drive_cut_camera).chain());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(ring::the_ring_with_hull(
        &game_assets,
        &ships,
        "block_gunship",
    )));
}

#[cfg(feature = "debug")]
#[derive(Resource)]
struct CutCamera {
    elapsed: f32,
    next: usize,
}

#[cfg(feature = "debug")]
const CUT_INTERVAL: f32 = 0.65;

/// The arrival montage: ten bearings, each `(side, along, up, look_ahead)` in
/// meters, relative to the ship and its track. Each is installed as a
/// [`ring::LegCamera`], so the rig FLIES with the ship for its 0.65 s and the
/// change of bearing is a cut.
///
/// A montage of PINNED poses is what this used to be, and it recorded eleven
/// seconds of empty starfield: a ship braking from a transfer crosses 200 m in
/// the time one cut holds, so it left every frame it was placed in almost as
/// soon as the cut landed. `LegCamera` says so on itself; this is the same
/// lesson learned twice.
///
/// Two numbers decide how the ship SITS in that frame. The STAND-OFF (the
/// length of the first three) sets how much of the width the hull fills: the
/// lens is 45 degrees vertical, so at 16:9 it spans 1.47 times its distance,
/// and a 110 m gunship is half the frame width at about 150 m and a quarter of
/// it at 300. The LOOK-AHEAD decides where in the frame it sits, because the
/// camera aims at a point down the track rather than at the ship: half the
/// horizontal field is 36 degrees, so a lead of 200 m at a 300 m stand-off
/// pushes the subject 34 degrees off axis, onto the frame edge.
///
/// So: stand-offs between 120 and 170 m, and leads short enough to keep the
/// hull inside the middle two thirds with the space it is flying into ahead of
/// it.
///
/// The third number is WHERE the lens stands, and on this leg it is the only
/// one that decides whether the shot has a world in it. The beacon is 7.6 km
/// straight up from the planetoid, so at the top of the leg the body is a 13
/// degree disc directly BELOW the ship and everything else is empty sky: an
/// abeam bearing records a grey hull on black, which is what these ten cuts
/// used to be. Standing DOWN THE TRACK instead - the whole stand-off on
/// `along`, the side offset kept under a third of it - aims the lens back past
/// the ship at the world it climbed away from, and puts the planetoid inside
/// the 22.5 degree half-field rather than just outside it. It is also the
/// bearing the beat deserves: the ship is braking, so the drive fires up the
/// track, and a camera up the track is looking into the plume.
#[cfg(feature = "debug")]
const CUTS: [(Meters, Meters, Meters, Meters); 10] = [
    (Meters(45.0), Meters(140.0), Meters(10.0), Meters(-10.0)),
    (Meters(-55.0), Meters(150.0), Meters(-5.0), Meters(-15.0)),
    (Meters(35.0), Meters(120.0), Meters(25.0), Meters(-20.0)),
    (Meters(-40.0), Meters(155.0), Meters(0.0), Meters(0.0)),
    (Meters(60.0), Meters(130.0), Meters(20.0), Meters(-25.0)),
    (Meters(-30.0), Meters(145.0), Meters(15.0), Meters(-10.0)),
    (Meters(50.0), Meters(125.0), Meters(30.0), Meters(-30.0)),
    (Meters(-45.0), Meters(160.0), Meters(-10.0), Meters(0.0)),
    (Meters(25.0), Meters(125.0), Meters(30.0), Meters(-20.0)),
    (Meters(-55.0), Meters(130.0), Meters(20.0), Meters(-30.0)),
];

#[cfg(feature = "debug")]
fn start_cut_camera(world: &mut World) {
    let (side, along, up, look_ahead) = CUTS[0];
    ring::leg(world, side, along, up, look_ahead);
    world.insert_resource(CutCamera {
        elapsed: 0.0,
        next: 1,
    });
}

#[cfg(feature = "debug")]
fn drive_cut_camera(world: &mut World) {
    let Some(mut cuts) = world.remove_resource::<CutCamera>() else {
        return;
    };
    cuts.elapsed += world.resource::<Time>().delta_secs();
    if cuts.elapsed >= CUT_INTERVAL {
        cuts.elapsed -= CUT_INTERVAL;
        let (side, along, up, look_ahead) = CUTS[cuts.next % CUTS.len()];
        ring::leg(world, side, along, up, look_ahead);
        cuts.next += 1;
    }
    world.insert_resource(cuts);
}

/// The last shot: the cutting stops and the ship is held on one bearing while
/// the braking plume dies.
///
/// Still a leg camera, not a pin - "parked" is the autopilot's word for a ship
/// that has stopped closing, not for one that has stopped moving.
#[cfg(feature = "debug")]
fn settle_camera(world: &mut World) {
    world.remove_resource::<CutCamera>();
    ring::leg(
        world,
        Meters(40.0),
        Meters(140.0),
        Meters(15.0),
        Meters(-20.0),
    );
}

#[cfg(feature = "debug")]
fn arrival_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the ring")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("engage GOTO")
        .on_enter(|world| {
            hide_hud(world);
            ring::engage_goto(world);
        })
        .until(ring::player_burning())
        .deadline(30.0)
        .add()
        .step("frame the departure burn")
        .on_enter(|world| {
            ring::chase(
                world,
                Meters(180.0),
                Meters(140.0),
                Meters(60.0),
                Meters(120.0),
            )
        })
        .until(elapsed(0.3))
        .add()
        .step("ride the outbound burn and coast")
        .until(ring::player_braking())
        .deadline(150.0)
        .add()
        .step("start the fixed-camera arrival sequence")
        .on_enter(start_cut_camera)
        .until(elapsed(0.2))
        .add()
        .step("open the arrival loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .add()
        .step("watch the retrograde flip")
        .until(ring::player_retro_burning())
        .deadline(25.0)
        .add()
        .step("watch the braking run")
        .until(ring::player_arrived())
        .deadline(60.0)
        .add()
        .step("cut to the settled arrival")
        .on_enter(settle_camera)
        .until(elapsed(1.5))
        .add()
        .step("close the arrival loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}
