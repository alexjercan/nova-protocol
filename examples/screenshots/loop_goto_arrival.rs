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
fn fixed_leg_pose(world: &mut World, side: Meters, along: Meters, up: Meters, look_ahead: Meters) {
    let ship = ring::ship_position(world);
    let track = ring::ship_heading(world);
    ring::pin(
        world,
        ship + Meters3(
            ring::lit_side(track) * side.get() + track * along.get() + Vec3::Y * up.get(),
        ),
        ship + Meters3(track * look_ahead.get()),
    );
}

#[cfg(feature = "debug")]
#[derive(Resource)]
struct CutCamera {
    elapsed: f32,
    next: usize,
}

#[cfg(feature = "debug")]
const CUT_INTERVAL: f32 = 0.65;

#[cfg(feature = "debug")]
const CUTS: [(Meters, Meters, Meters, Meters); 10] = [
    (Meters(300.0), Meters(-100.0), Meters(120.0), Meters(200.0)),
    (Meters(320.0), Meters(80.0), Meters(180.0), Meters(180.0)),
    (Meters(340.0), Meters(-160.0), Meters(50.0), Meters(180.0)),
    (Meters(360.0), Meters(120.0), Meters(100.0), Meters(190.0)),
    (Meters(310.0), Meters(160.0), Meters(-50.0), Meters(160.0)),
    (Meters(340.0), Meters(-120.0), Meters(140.0), Meters(150.0)),
    (Meters(290.0), Meters(50.0), Meters(200.0), Meters(130.0)),
    (Meters(320.0), Meters(-150.0), Meters(70.0), Meters(120.0)),
    (Meters(280.0), Meters(120.0), Meters(110.0), Meters(100.0)),
    (Meters(300.0), Meters(-60.0), Meters(-40.0), Meters(90.0)),
];

#[cfg(feature = "debug")]
fn start_cut_camera(world: &mut World) {
    let (side, along, up, look_ahead) = CUTS[0];
    fixed_leg_pose(world, side, along, up, look_ahead);
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
        fixed_leg_pose(world, side, along, up, look_ahead);
        cuts.next += 1;
    }
    world.insert_resource(cuts);
}

#[cfg(feature = "debug")]
fn settle_camera(world: &mut World) {
    world.remove_resource::<CutCamera>();
    fixed_leg_pose(
        world,
        Meters(250.0),
        Meters(-70.0),
        Meters(70.0),
        Meters::ZERO,
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
