//! `flight_cancel`: an engaged maneuver drops when manual flight input takes
//! the ship back.
//!
//! The demonstration first builds enough speed for STOP to engage visibly.
//! It then opens on a hull under autopilot and records the main drive lighting
//! as the maneuver chip goes out.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` flies the whole script without recording.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1` also writes the sheet under
//!   `NOVA_CAPTURE_DIR`.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_flight_cancel --features debug
//! ```

#[path = "shared/kit.rs"]
mod kit;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_chase_plugin, lesson_profile, LessonChase, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_flight_cancel")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's CANCEL demonstration", long_about = None)]
struct Cli;

/// The sheet for "Taking the ship back".
#[cfg(feature = "debug")]
const CANCEL_LESSON: &str = "flight_cancel";

/// Scenario id of the player hull.
const PLAYER_ID: &str = "cancel_player";

/// Enough speed for the STOP order and its cancellation to read clearly.
#[cfg(feature = "debug")]
const CANCEL_RUNUP_SPEED: MetersPerSecond = MetersPerSecond(120.0);

/// Where the eye rides for the CANCEL sheet: abeam to world +X, a little aft.
///
/// The braking order has turned the hull end for end by the time this sheet
/// opens, and the beam is the one station where the drive that lights against
/// that heading is seen at full width rather than down its own axis. The SIDE
/// is the set's lighting, not a preference: the three-point rig keys from
/// +X/+Z, so the first cut of this sheet - shot from the port beam, chosen
/// only so the two sheets would not look alike - recorded a hull in its own
/// shadow with the chip going out over a silhouette.
#[cfg(feature = "debug")]
const CANCEL_EYE: Meters3 = Meters3::new(126.0, 24.0, 24.0);

/// Cells the CANCEL sheet holds the engaged maneuver before the drive lights.
///
/// The lesson is a chip GOING OUT, which needs the chip lit first. Seven cells
/// is seven tenths of a second of a hull the computer is flying - long enough
/// to read the verb on the dock, short enough to leave most of the sheet for
/// the hand-flown burn that replaces it.
#[cfg(feature = "debug")]
const CANCEL_LEAD_CELLS: u32 = 7;

/// How long the braking order is left to run before its sheet opens.
///
/// Long enough that the swing onto retrograde is over: a sheet opened mid-turn
/// spends its lead-in cells on a maneuver that looks like the thing being
/// cancelled rather than on a hull steadily under someone else's control, and
/// the reader cannot tell which motion the drive interrupted.
#[cfg(feature = "debug")]
const STOP_SETTLE_SECS: f32 = 2.5;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(flight_cancel_script());
        // Nothing is frozen: the moving hull and passing rocks make the
        // interrupted maneuver visible. The chase starts with the script.
        app.add_plugins(lesson_chase_plugin);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(cancel_range(&game_assets, &ships)));
}

/// One player gunship in a rock shell wide enough for the run-up.
fn cancel_range(game_assets: &GameAssets, ships: &GameShipDesigns) -> ScenarioConfig {
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player Ship".to_string(),
            position: Meters3::ZERO,
            // Square with the world so the burn runs down world -Z.
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                // No turret bindings: nothing here fires, and an empty map is
                // what every flight set spawns with.
                input_mapping: BTreeMap::new(),
            }),
            allegiance: None,
            design: ShipDesignSource::Inline(kit::catalog_ship(ships, "block_gunship")),
            ..default()
        }),
    };

    // Keep the rocks far enough out for the run-up and braking order.
    let shell = kit::NearField {
        id_prefix: "cancel_rock_",
        count: 40,
        seed: 40507,
        center: Meters3::ZERO,
        distance: (Meters(1_500.0), Meters(3_000.0)),
        // Small, because a scattered asteroid is DRAWN about four and a half
        // times its authored radius: the first cut authored 16-44 m and got a
        // field of seventy-to-two-hundred-metre boulders standing in front of
        // the hull in half the cells. These draw at thirty to eighty, which is
        // debris going past rather than a wall the ship is inside.
        radius: (Meters(7.0), Meters(18.0)),
        y_spread: Meters(800.0),
    };

    ScenarioConfig {
        description: "One moving hull in a rock shell, for the CANCEL lesson.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    shell.action(game_assets),
                    EventActionConfig::SpawnScenarioObject(player),
                ],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "cancel_range".to_string(),
            "Cancel Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Put the HUD on and take the fps/version status bar out of shot.
#[cfg(feature = "debug")]
fn hud_instrument(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::On;
    }
    hide_status_bar(world);
}

/// Advance once the player is moving fast enough for STOP to read clearly.
#[cfg(feature = "debug")]
fn speed_at_least(
    speed: MetersPerSecond,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let Some(mut ships) = world.try_query_filtered::<
            &avian3d::prelude::LinearVelocity,
            (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
        >() else {
            return false;
        };
        let wanted = speed.to_engine();
        ships
            .iter(world)
            .any(|velocity| velocity.0.length() >= wanted)
    })
}

/// Advance once the flight computer has a maneuver engaged on the player.
#[cfg(feature = "debug")]
fn maneuver_engaged() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&Autopilot, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
            .is_some_and(|mut ships| ships.iter(world).next().is_some())
    })
}

/// Whether the flight computer still holds the player's hull.
#[cfg(feature = "debug")]
fn under_the_computer(world: &World) -> bool {
    world
        .try_query_filtered::<&Autopilot, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>(
        )
        .is_some_and(|mut ships| ships.iter(world).next().is_some())
}

/// Build speed, engage STOP, then photograph manual input taking control.
#[cfg(feature = "debug")]
fn flight_cancel_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the cancellation range")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), scenario_camera_present()))
        .deadline(30.0)
        .add()
        .step("raise the instruments and take up the chase")
        .on_enter(|world: &mut World| {
            hud_instrument(world);
            world.insert_resource(LessonChase::new(CANCEL_EYE));
        })
        .until(elapsed(1.0))
        .add()
        .step("build speed for the braking order")
        .on_enter(press_action("main_drive"))
        .until(speed_at_least(CANCEL_RUNUP_SPEED))
        .deadline(90.0)
        .add()
        .step("let the drive up and give the braking order")
        .on_enter(|world: &mut World| {
            release_action("main_drive")(world);
            press_action("autopilot_stop")(world);
        })
        .until(frames(1))
        .add()
        .step("let the order key up")
        .on_enter(release_action("autopilot_stop"))
        .until(maneuver_engaged())
        .deadline(20.0)
        .add()
        .step("let the braking order become clear")
        .until(elapsed(STOP_SETTLE_SECS))
        .add()
        .step("open the CANCEL sheet on a hull the computer is flying")
        .on_enter(|world: &mut World| sheet_start(world, CANCEL_LESSON, LESSON_GRID))
        .until(frames(CANCEL_LEAD_CELLS))
        .add()
        .step("light the main drive inside the recording")
        .on_enter(press_action("main_drive"))
        .until(sheet_written(CANCEL_LESSON))
        .deadline(60.0)
        .add()
        .step("the burn took the ship back")
        .on_enter(|world: &mut World| {
            assert!(
                !under_the_computer(world),
                "the CANCEL sheet closed with a maneuver still engaged: the demonstration would \
                 show a drive lighting and nothing giving way. Check that a main-drive burn still \
                 disengages the autopilot."
            );
        })
        .until(frames(1))
        .add()
        .step("park the drive and the chase")
        .on_enter(|world: &mut World| {
            release_action("main_drive")(world);
            world.remove_resource::<LessonChase>();
        })
        .add()
}
