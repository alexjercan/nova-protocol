//! lesson_flight_limits: the two FLIGHT demonstrations about who is holding
//! the ship - `flight_speedcap` (a held burn tapering off against the range's
//! soft cap) and `flight_cancel` (an engaged maneuver dropping the moment the
//! main drive lights).
//!
//! One producer, two sheets, one flight, and one set built for it: a lone
//! player hull under a 150 m/s governor - the figure Basic Training's trainer
//! flies under (`base_content::scenarios::tutorial::range::TRAINER_SPEED_CAP`)
//! and the number the lesson names. A cap is scenario content, not a keybind,
//! so neither lesson can be photographed on any set the fleet already owns:
//! every hollow spawns its player with `speed_cap: None`, and an uncapped burn
//! accelerates forever with nothing for the readout to level off against.
//!
//! The two are one flight because the second needs what the first leaves
//! behind. `flight_cancel`'s whole sentence is that the computer hands the
//! ship back ALREADY MOVING, so the beat has to open on a hull that is
//! genuinely travelling - and the beat before it has just spent four seconds
//! building exactly that.
//!
//! ## Two ACTION loops, shot from a chase
//!
//! Both are the ACTION kind (see `shared/lesson.rs`): the camera holds a fixed
//! WORLD offset from the hull ([`lesson::LessonChase`]) and the motion in the
//! cell is the ship and its instruments. Neither claim is a pose - one is a
//! number that stops climbing, the other is a chip that goes out - and both
//! only mean anything against the state they replace, so each sheet opens on
//! the before and spends its last cells on the after.
//!
//! Both wrap as a tutorial clip wraps: back to the start to do it again. The
//! sheets close on a state their first cell does not carry - a readout pinned
//! at the cap, a hull back under manual power - and that cut IS the
//! demonstration repeating, the same wrap `lesson_combat_moves` records the
//! weapons coming up on.
//!
//! ## The shell is pushed out, and that is the flight's doing
//!
//! `hollow::flight_hollow` puts its rocks 480 m out because the lessons shot
//! on it are burns that cover less than that. This one is not: the cap beat
//! alone builds to 140 m/s and then holds it for the whole two seconds of a
//! sheet, so the hull covers the best part of a kilometre before the braking
//! order is even given. The field here starts at 1500 m for that reason - far
//! enough that the flight stays inside the pocket, close enough that a rock
//! abeam still swings right across the cell in the two seconds a sheet lasts,
//! which is what makes 150 m/s look like 150 m/s.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - fly the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the two sheets (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_flight_limits --features debug
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
#[command(name = "lesson_flight_limits")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's speed-cap and CANCEL demonstrations", long_about = None)]
struct Cli;

/// The sheet for "The speed cap".
#[cfg(feature = "debug")]
const SPEEDCAP_LESSON: &str = "flight_speedcap";
/// The sheet for "Taking the ship back".
#[cfg(feature = "debug")]
const CANCEL_LESSON: &str = "flight_cancel";

/// Scenario id of the capped hull.
const PLAYER_ID: &str = "capped_player";

/// The governor the range flies under.
///
/// Basic Training's own figure, copied rather than imported: the lesson names
/// "150 m/s" in its text, and a producer that followed a tutorial constant
/// would re-shoot a demonstration whose caption no longer matched the moment
/// somebody retuned the trainer. The number is the lesson's.
const RANGE_SPEED_CAP: MetersPerSecond = MetersPerSecond(150.0);

/// Where the sheet opens on the climb, as a fraction of the cap.
///
/// `SPEED_CAP_TAPER_FRACTION` is a fifth (`nova_ship/src/flight/manual.rs`),
/// so the budget starts biting at 120 m/s and the last thirty are a
/// first-order relaxation onto the ceiling. Opening at four fifths is opening
/// exactly at the top of that band: the readout climbs through the first cells
/// at a rate the eye can see and through the last ones at a rate it cannot,
/// which is the taper drawn. Opening lower spends most of the sheet on a
/// climb that is not yet capped; opening higher opens on a number that is
/// already still.
#[cfg(feature = "debug")]
const SHEET_OPENS_AT: f32 = 0.80;

/// Where the eye rides for the speed-cap sheet: off the starboard quarter and
/// a little above, about 130 m out.
///
/// Behind the beam, like the momentum lesson's, and for half the same reason:
/// the drive is HELD for every cell of this sheet, so the plume is the proof
/// the throttle never came off and it has to be in shot. The other half is the
/// rock field - from the quarter the rocks stream diagonally across the cell
/// rather than expanding out of its centre, which is the difference between a
/// frame that reads as travel and one that reads as a zoom.
#[cfg(feature = "debug")]
const SPEEDCAP_EYE: Meters3 = Meters3::new(96.0, 26.0, 86.0);

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
        app.add_plugins(flight_limits_script());
        // NOTHING is frozen: both subjects are the hull actually travelling,
        // and the rocks going past are how the reader knows it is. The chase
        // is inert until a step inserts its offset.
        app.add_plugins(lesson_chase_plugin);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(capped_range(&game_assets, &ships)));
}

/// The set: one capped hull in a rock shell wide enough to fly a capped burn
/// inside.
///
/// The hull is the fleet's gunship, the same one every other flight lesson is
/// shot on, so a reader moving down the Flight category sees one ship rather
/// than a different craft per screen.
fn capped_range(game_assets: &GameAssets, ships: &GameShipDesigns) -> ScenarioConfig {
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player Ship".to_string(),
            position: Meters3::ZERO,
            // Square with the world, so the burn runs down world -Z and both
            // chase offsets below are measured in the axes they are written in.
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                // No turret bindings: nothing here fires, and an empty map is
                // what every flight set spawns with.
                input_mapping: BTreeMap::new(),
                speed_cap: Some(RANGE_SPEED_CAP),
            }),
            allegiance: None,
            design: ShipDesignSource::Inline(kit::catalog_ship(ships, "block_gunship")),
            ..default()
        }),
    };

    // See the module docs: a shell that starts where the fighting sets' does
    // is a shell this flight ends up inside.
    let shell = kit::NearField {
        id_prefix: "capped_rock_",
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
        description: "One capped hull in a rock shell, for the speed-cap and CANCEL lessons."
            .to_string(),
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
            "capped_range".to_string(),
            "Capped Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Put the HUD on and take the fps/version status bar out of shot.
///
/// The instruments ARE the subject of both sheets - a speed readout and a verb
/// chip - so the HUD stays up and only the bar goes, for the reason
/// `hide_status_bar` gives: its version item would bake this build's commit
/// into the handbook.
#[cfg(feature = "debug")]
fn hud_instrument(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::On;
    }
    hide_status_bar(world);
}

/// Advance once the player is travelling at least `fraction` of the range's
/// cap.
///
/// Read off the hull's own velocity rather than counting seconds of burn: what
/// a gunship's drives do to a gunship's mass is the ship catalog's business,
/// and a timed burn would re-shoot at a different point on the taper the day
/// a thruster is retuned.
#[cfg(feature = "debug")]
fn speed_at_least(fraction: f32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let Some(mut ships) = world.try_query_filtered::<
            &avian3d::prelude::LinearVelocity,
            (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
        >() else {
            return false;
        };
        // Engine boundary: avian publishes world units per second, and the cap
        // is authored in meters per second.
        let wanted = MetersPerSecond(RANGE_SPEED_CAP.0 * fraction).to_engine();
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

/// Fly one capped burn and one interrupted braking order, and photograph a
/// lesson out of each.
#[cfg(feature = "debug")]
fn flight_limits_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the capped range")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), scenario_camera_present()))
        .deadline(30.0)
        .add()
        .step("raise the instruments and take up the chase")
        .on_enter(|world: &mut World| {
            hud_instrument(world);
            world.insert_resource(LessonChase::new(SPEEDCAP_EYE));
        })
        .until(elapsed(1.0))
        .add()
        // THE SPEED CAP. The throttle goes down here and does not come up
        // again until the sheet is tiled: the claim is what a HELD burn does,
        // so a run that eased off inside the recording would be showing a
        // pilot obeying the cap rather than the cap holding a pilot.
        .step("hold the drive up to the top of the taper band")
        .on_enter(press_action("main_drive"))
        .until(speed_at_least(SHEET_OPENS_AT))
        .deadline(90.0)
        .add()
        .step("record the burn levelling off")
        .on_enter(|world: &mut World| sheet_start(world, SPEEDCAP_LESSON, LESSON_GRID))
        .until(sheet_written(SPEEDCAP_LESSON))
        .deadline(60.0)
        .add()
        // A sheet that closed below the taper band is a sheet of a ship
        // accelerating, which is the opposite of the lesson. Fail the run
        // rather than ship it.
        .step("the sheet closed against the cap")
        .on_enter(|world: &mut World| {
            assert!(
                speed_at_least(SHEET_OPENS_AT)(world),
                "the speed-cap sheet closed below {}% of the cap: the demonstration would show a \
                 burn still building rather than one tapering off. Check that the range still \
                 spawns its player with a speed cap.",
                (SHEET_OPENS_AT * 100.0) as u32
            );
        })
        .until(frames(1))
        .add()
        // TAKING THE SHIP BACK. The drive comes off and the computer takes the
        // hull, so the sheet below opens on a ship somebody else is flying.
        .step("let the drive up and give the braking order")
        .on_enter(|world: &mut World| {
            release_action("main_drive")(world);
            world.insert_resource(LessonChase::new(CANCEL_EYE));
            press_action("autopilot_stop")(world);
        })
        .until(frames(1))
        .add()
        .step("let the order key up")
        .on_enter(release_action("autopilot_stop"))
        .until(maneuver_engaged())
        .deadline(20.0)
        .add()
        .step("let the hull come round onto retrograde")
        .until(elapsed(STOP_SETTLE_SECS))
        .add()
        .step("open the CANCEL sheet on a hull the computer is flying")
        .on_enter(|world: &mut World| sheet_start(world, CANCEL_LESSON, LESSON_GRID))
        .until(frames(CANCEL_LEAD_CELLS))
        .add()
        // The main drive is the CANCEL here, not the CANCEL key: the lesson's
        // sentence is that a burn takes the ship back on its own, and a key
        // press would demonstrate the one path a reader would have guessed.
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
