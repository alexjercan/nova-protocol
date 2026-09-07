//! loop_turret_stow: the retractable mount, close - `loop-section-turret-stow`.
//!
//! One PDC on a short spine, alone in the dark with nothing to shoot at, and
//! the lens close enough that the housing fills the frame. The walk raises the
//! combat stance and holds it long enough for the mount to come up, then drops
//! it and waits out the quiet the mount folds after: lids part, gun rises, gun
//! sinks, lids shut.
//!
//! Nothing here animates the mount. Raising the stance is a
//! [`press_action`] on the same `combat_stance` binding the player holds, and
//! everything after it is the production state machine
//! (`nova_ship::sections::turret_section::stow`) reading the safety it feeds:
//! the script only presses, watches [`TurretStowPhase`] and records.
//!
//! The set is deliberately empty. A contact would combat-lock the ship, and a
//! locked ship keeps its weapons hot whatever the stance says, so the mount
//! would never fold and the loop would be half a cycle.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full cycle, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: record and encode the loop into
//!   `NOVA_CAPTURE_DIR/loop-section-turret-stow.webm`.
//!
//! Capture:
//! ```text
//! NOVA_CAPTURE_DIR=target/loop-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_turret_stow --features debug
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_turret_stow")]
#[command(version = "1.0.0")]
#[command(about = "The retractable point-defence mount deploying and stowing, close. Autopilot-only: the stance is scripted and the mount runs itself", long_about = None)]
struct Cli;

/// The loop this example records - the webm's file stem.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "loop-section-turret-stow";

/// The scenario id the set loads under.
const SCENARIO_ID: &str = "turret_stow_bench";

/// Scenario id of the hull the mount stands on.
const BENCH_ID: &str = "stow_bench";

/// Where the mount's centre sits above the cell it stands on: half a cell out
/// to that cell's top face, then a quarter for the PDC's own base plate. The
/// same seat the point-defence range measures from.
const MOUNT_SEAT: f32 = 0.75;

/// Where the mount stands, in meters, given a bench at the origin.
#[cfg(feature = "debug")]
const MOUNT_AT: Meters3 = Meters3::new(0.0, MOUNT_SEAT * 10.0, 0.0);

/// Where the lens stands off the mount, meters.
///
/// A three-quarter view from above the housing's lip: the two lids part across
/// the frame rather than edge-on, and the gun rises toward the camera instead
/// of away from it. The length is what has to fit - the housing is a 10 m cell
/// and the deployed gun stands most of another one over it, which is 17 m of
/// subject, and the lens spans 0.83 times its distance VERTICALLY at 16:9. So
/// 26 m, which holds the whole travel with headroom at both ends of it.
#[cfg(feature = "debug")]
const MOUNT_EYE: Meters3 = Meters3::new(16.0, 9.0, 18.0);

/// Where in that travel the lens is centred: above the housing's own centre,
/// because the mount only ever moves UP from it. Aiming at the housing puts
/// the deployed gun against the top edge and half the frame under a lid that
/// never moves.
#[cfg(feature = "debug")]
const MOUNT_LOOK_LIFT: Meters3 = Meters3::new(0.0, 2.0, 0.0);

/// How fast gameplay time runs through the quiet a stow waits out.
///
/// A deployed mount folds after four seconds cold and untracked
/// (`STOW_SETTLE_SECONDS`), which is a design cost - a lull in a fight must
/// not fold the battery - and four seconds of a gun standing still is not a
/// picture. The recording runs that wait at four times and drops back to real
/// speed the moment the fold starts, so both animations play at the speed they
/// were authored at and the dead middle is a beat rather than a third of the
/// loop.
#[cfg(feature = "debug")]
const QUIET_TIME_SCALE: f32 = 4.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(stow_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(stow_bench(&game_assets, &sections)));
}

/// The bench: a flight computer, a hull cell either side of it, and one mount
/// standing on the middle cell.
///
/// Piloted, and that is the only reason there is a cycle to film: the stow
/// machine arms on a LIVE turret, and a turret is live when a controller hands
/// it a `TurretSectionInput`. An uncrewed emplacement keeps the deployed gun
/// the art rests in and never moves it. No bindings on the mount itself - the
/// walk never pulls a trigger, and a gun with nothing to shoot at would only
/// slew off the framing.
fn bench(sections: &GameSections) -> SpaceshipConfig {
    let specs = vec![
        SectionSpec::new("controller", BASIC_CONTROLLER_SECTION_ID, Vec3::ZERO),
        SectionSpec::new(
            "spine_port",
            REINFORCED_HULL_SECTION_ID,
            Vec3::new(-1.0, 0.0, 0.0),
        ),
        SectionSpec::new(
            "spine_starboard",
            REINFORCED_HULL_SECTION_ID,
            Vec3::new(1.0, 0.0, 0.0),
        ),
        SectionSpec::new(
            "mount",
            PDC_KINETIC_TURRET_SECTION_ID,
            Vec3::new(0.0, MOUNT_SEAT, 0.0),
        ),
    ];

    SpaceshipConfig {
        allegiance: Some(Allegiance::Player),
        ..fixtures::ship(
            sections,
            SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: default(),
                speed_cap: None,
            }),
            &specs,
        )
    }
}

/// The set: the bench under the photo rig, and nothing else in the sky.
fn stow_bench(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let bench = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BENCH_ID.to_string(),
            name: "Stow Bench".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(bench(sections)),
    };

    ScenarioConfig {
        description: "A point-defence mount deploying and stowing.".to_string(),
        hidden: true,
        events: fixtures::spawn_on_start(
            [
                vec![bench],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).objects(),
            ]
            .concat(),
        ),
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Turret Stow Loop".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Put the lens on the mount and take the HUD down.
#[cfg(feature = "debug")]
fn frame_the_mount(world: &mut World) {
    hide_hud(world);
    pose_camera(world, MOUNT_AT + MOUNT_EYE, MOUNT_AT + MOUNT_LOOK_LIFT);
}

/// Run gameplay time at `scale`.
#[cfg(feature = "debug")]
fn set_time_scale(world: &mut World, scale: f32) {
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(scale);
}

/// Advance once the bench's mount has armed its stow machine.
#[cfg(feature = "debug")]
fn mount_armed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query::<&TurretStow>()
            .is_some_and(|mut query| query.iter(world).next().is_some())
    })
}

/// Advance once the mount reaches `phase`.
#[cfg(feature = "debug")]
fn mount_phase(
    phase: TurretStowPhase,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query::<&TurretStow>()
            .is_some_and(|mut query| query.iter(world).any(|stow| stow.phase() == phase))
    })
}

/// The driven walk: frame the shut housing, raise, hold, drop, and record the
/// mount folding itself away.
#[cfg(feature = "debug")]
fn stow_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the stow bench")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), mount_armed()))
        .deadline(60.0)
        .add()
        .step("frame the mount")
        .on_enter(frame_the_mount)
        .until(elapsed(0.8))
        .add()
        .step("open the stow loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .add()
        // A beat of the shut housing, so a reader who arrives mid-loop sees
        // what the mount looks like at rest before anything opens.
        .step("hold the shut housing")
        .until(elapsed(0.7))
        .add()
        // The production hold, not a toggle: `combat_stance` is held down for
        // as long as the weapons are up, and releasing it is what starts the
        // clock the stow waits out.
        .step("raise the weapons")
        .on_enter(press_action("combat_stance"))
        .until(mount_phase(TurretStowPhase::Deployed))
        .deadline(20.0)
        .add()
        .step("hold the deployed gun")
        .until(elapsed(1.0))
        .add()
        // Cold from here. The mount does not fold on the release - it folds
        // after four seconds of quiet - so the clock runs fast until the fold
        // actually starts.
        .step("drop the stance")
        .on_enter(|world: &mut World| {
            release_action("combat_stance")(world);
            set_time_scale(world, QUIET_TIME_SCALE);
        })
        .until(mount_phase(TurretStowPhase::Stowing))
        .deadline(30.0)
        .add()
        .step("watch it fold away")
        .on_enter(|world: &mut World| set_time_scale(world, 1.0))
        .until(mount_phase(TurretStowPhase::Stowed))
        .deadline(30.0)
        .add()
        .step("hold the shut housing again")
        .until(elapsed(0.7))
        .add()
        .step("close the stow loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(120.0)
        .add()
}
