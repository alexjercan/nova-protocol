//! lesson_combat_rounds: the COMBAT demonstration about what a damage type
//! changes - `combat_damage_types`, a Kinetic slug stopping at the first plate
//! it cannot destroy while a Pierce dart rakes the whole stack.
//!
//! ## A loop, because the claim is about TRAVEL
//!
//! "A damage type is not a multiplier: both hit a hull and a drive for the same
//! number. What changes is travel." A still of two lanes shows two lines of
//! different length and asks the reader to believe they are rounds. The loop
//! shows the rounds LEAVING together, and one of them going further than the
//! other, which is the whole claim.
//!
//! ## Real guns, not scripted rounds
//!
//! The game ships both halves of this comparison as sections -
//! `pdc_kinetic_turret_section` and `pdc_pierce_turret_section`, the same mount
//! and the same 1 000 m/s gatling with the ammunition swapped - so this walk
//! builds two identical rigs and fires them. Nothing here spawns a round or
//! paints one: the rounds are the guns' own, in the colours the HUD gives their
//! type (amber Kinetic, steel-blue Pierce, `nova_gameplay::damage_type_color`),
//! and they are resolved by the production sweep.
//!
//! The one thing the script writes is the two turrets' aim point and their
//! triggers (`TurretSectionTargetInput`, `TurretSectionInput`), which is how
//! every unmanaged gun in this repo's rigs is fired - see `loop_vfx_range`.
//! Neither rig has a controller, so there is no stance, no lock and no HUD in
//! the picture: two guns, ten plates and what comes back out of them.
//!
//! ## The arithmetic the set is built on
//!
//! A Pierce round spawns with `PIERCE_BASE_POWER` (300) and pays a layer's MAX
//! health for each one it crosses (`nova_gameplay::damage::pierce_remainder`).
//! A light hull section is authored at 60, so 300 buys five crossings and the
//! round dies in the fifth plate - which is why there are exactly five. A
//! Kinetic round carries 4 damage against that same 60, cannot destroy it, and
//! therefore stops in the first one. Both numbers are the shipped ones; the set
//! is sized to them rather than the other way round.
//!
//! ## Why the clock is slowed and the trigger is pulsed
//!
//! A PDC round covers 100 m in one tenth of a second, which is one whole cell
//! of a lesson sheet: at the game's own rate the entire demonstration happens
//! between two frames. The walk therefore runs the world at
//! [`LESSON_SLOWDOWN`] with the fixed step raised to match
//! ([`CAPTURE_PHYSICS_HZ`]), so exactly one physics step falls in each captured
//! frame and a round advances [`ROUND_STRIDE`] of visible travel per cell.
//! Slow motion changes no number in the model - the sweep, the speed curves and
//! the pierce budget all read the same - it only samples the flight finely
//! enough to photograph.
//!
//! The trigger is TAPPED rather than held, and the sheet is ONE ROUND of each
//! type. Held, the gatling puts 400 damage a second into the first plate,
//! destroys it a third of the way through the sheet and buries both lanes in
//! its own debris - the first cut of this frame recorded exactly that, and by
//! the last row the picture was a wall of chips with the rounds lost inside it.
//! One round each is also what the lesson's own words describe: a slug and a
//! dart, leaving together, one of them getting further. The plates all survive
//! it, so the set the last cell shows is the set the first cell showed.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_combat_rounds --features debug
//! ```

#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID};
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_combat_rounds")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's Kinetic and Pierce demonstration", long_about = None)]
struct Cli;

/// The sheet for "Kinetic and Pierce".
#[cfg(feature = "debug")]
const ROUNDS_LESSON: &str = "combat_damage_types";

/// How far above its hull's centre a PDC's muzzle sits.
///
/// A turret is half a section standing on a hull face, so its gun rides about
/// one and a half cells over the centre of the cell it is bolted to. The lane
/// is authored at the MUZZLE and the rig hangs below it, rather than the other
/// way round, because the lane is what has to be straight.
const MOUNT_RISE: Meters = Meters(8.0);
/// A turret mates half its own size above the hull face, not a whole cell:
/// its one link point is its base plate. `loop_vfx_range` writes this out.
const TURRET_MOUNT_Y: f32 = 0.75;

/// Where the two firing lines run, in world Y.
///
/// Stacked one over the other rather than side by side: the camera stands off
/// the beam so the course runs across the cell, and two lanes separated along
/// the lens's own axis would hide each other.
const KINETIC_LANE_Y: Meters = Meters(16.0);
/// The Pierce lane, the same distance below the middle.
const PIERCE_LANE_Y: Meters = Meters(-16.0);

/// Where the first plate stands, down the lane from the muzzle.
const FIRST_PLATE_Z: Meters = Meters(-30.0);
/// The gap between plate centres.
///
/// A light hull section is one cell, so 15 m of spacing leaves a 5 m slot
/// between two 10 m plates - enough for a round to be SEEN between them, which
/// is the only reason the stack is spaced at all. A solid stack would hide the
/// whole difference inside the armour.
const PLATE_SPACING: Meters = Meters(15.0);
/// How many plates stand in each lane: exactly what a Pierce round's budget
/// buys, so the fifth is where it dies. See the module docs.
const PLATES: usize = 5;

/// How slowly the world runs while the sheet is recorded.
///
/// See the module docs: the demonstration is 100 m of flight and the game
/// crosses that in one captured frame.
#[cfg(feature = "debug")]
const LESSON_SLOWDOWN: f32 = 0.05;
/// The fixed step the capture runs at, in hertz.
///
/// Chosen so that `1 / (LESSON_FPS * LESSON_SLOWDOWN)` lands exactly on one
/// step per captured frame. A round only moves when the sweep runs, so a
/// slowdown that leaves two frames between two steps photographs a stutter
/// rather than a flight.
#[cfg(feature = "debug")]
const CAPTURE_PHYSICS_HZ: f64 = 200.0;
/// How far a PDC round travels between two captured cells, at the slowdown
/// above: 1 000 m/s of authored muzzle speed times a 200 Hz step.
#[cfg(feature = "debug")]
const ROUND_STRIDE: Meters = Meters(5.0);

/// How long each trigger is held, in captured frames.
///
/// Two, because the gatling's authored 100 rounds a second is one round per
/// fiftieth of a second and a captured frame is a two-hundredth of one at the
/// slowdown: a single frame on the trigger would leave whether a round comes
/// out at all to where the gun's timer happened to be.
#[cfg(feature = "debug")]
const TRIGGER_FRAMES: u32 = 2;

/// The eye, standing off the beam.
#[cfg(feature = "debug")]
const ROUNDS_EYE: Meters3 = Meters3::new(78.0, 6.0, -48.0);
/// What it looks at: the middle of the course, a little low, because the two
/// rigs hang below their own lanes.
#[cfg(feature = "debug")]
const ROUNDS_AIM: Meters3 = Meters3::new(0.0, -4.0, -48.0);

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
        app.add_plugins(rounds_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(the_range(&game_assets, &sections)));
}

/// The set: two identical gun rigs, one loaded with each round, and a spaced
/// stack of five light hull plates down each one's lane.
fn the_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let mut objects = vec![
        shooter(
            sections,
            "kinetic_rig",
            "Kinetic PDC",
            PDC_KINETIC_TURRET_SECTION_ID,
            KINETIC_LANE_Y,
        ),
        shooter(
            sections,
            "pierce_rig",
            "Pierce PDC",
            "pdc_pierce_turret_section",
            PIERCE_LANE_Y,
        ),
    ];
    for (lane, lane_y) in [("kinetic", KINETIC_LANE_Y), ("pierce", PIERCE_LANE_Y)] {
        for index in 0..PLATES {
            objects.push(plate(sections, lane, index, lane_y));
        }
    }

    ScenarioConfig {
        description: "Two identical gun rigs, one Kinetic and one Pierce, and a spaced stack of \
                      plates down each lane."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                objects
                    .into_iter()
                    .map(EventActionConfig::SpawnScenarioObject)
                    .collect::<Vec<_>>(),
                ThreePointRig::around("rounds", Meters3::new(0.0, 0.0, -50.0), 4.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "round_travel".to_string(),
            "Round Travel".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// One gun rig: a hull cell, a controller behind it and the mount on its roof,
/// hung far enough below its lane that the gun sits ON the lane.
fn shooter(
    sections: &GameSections,
    id: &str,
    name: &str,
    turret: &str,
    lane_y: Meters,
) -> ScenarioObjectConfig {
    let specs = [
        SectionSpec::new("spine", LIGHT_HULL_SECTION_ID, Vec3::ZERO),
        SectionSpec::new(
            "controller",
            BASIC_CONTROLLER_SECTION_ID,
            Vec3::new(0.0, 0.0, 1.0),
        ),
        SectionSpec::new("mount", turret, Vec3::new(0.0, TURRET_MOUNT_Y, 0.0)),
    ];
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position: Meters3::new(0.0, lane_y.get() - MOUNT_RISE.get(), 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(fixtures::ship(
            sections,
            SpaceshipController::None,
            &specs,
        )),
    }
}

/// One plate: a single light hull section standing in a lane, and nothing else.
///
/// A plate is its own ship rather than one section of a six-section hulk
/// because the gap between two plates is the whole point - a round crossing a
/// solid stack is a round nobody can see.
fn plate(
    sections: &GameSections,
    lane: &str,
    index: usize,
    lane_y: Meters,
) -> ScenarioObjectConfig {
    let z = FIRST_PLATE_Z.get() - PLATE_SPACING.get() * index as f32;
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("{lane}_plate_{index}"),
            name: format!("{lane} plate {}", index + 1),
            position: Meters3::new(0.0, lane_y.get(), z),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(fixtures::ship(
            sections,
            SpaceshipController::None,
            &[SectionSpec::new("plate", LIGHT_HULL_SECTION_ID, Vec3::ZERO)],
        )),
    }
}

/// Where the last plate in a lane stands: what each gun is laid on, so the line
/// it fires down passes through every plate in front of that one.
#[cfg(feature = "debug")]
fn lane_aim(lane_y: Meters) -> Vec3 {
    Vec3::new(
        0.0,
        lane_y.to_engine(),
        Meters(FIRST_PLATE_Z.get() - PLATE_SPACING.get() * (PLATES - 1) as f32).to_engine(),
    )
}

/// Hold or release both triggers.
#[cfg(feature = "debug")]
fn set_triggers(world: &mut World, firing: bool) {
    let mut query = world.query::<&mut TurretSectionInput>();
    for mut trigger in query.iter_mut(world) {
        **trigger = firing;
    }
}

/// Lay each gun down its own lane.
#[cfg(feature = "debug")]
fn lay_the_guns(world: &mut World) {
    // Which rig is which is read off the mount's own damage type rather than
    // off an entity name: the aim point is per turret section, and the two
    // sections differ in exactly the thing being demonstrated.
    let mut query = world.query::<(&mut TurretSectionTargetInput, &GlobalTransform)>();
    for (mut aim, at) in query.iter_mut(world) {
        let lane = if at.translation().y >= 0.0 {
            KINETIC_LANE_Y
        } else {
            PIERCE_LANE_Y
        };
        **aim = Some(lane_aim(lane));
    }
}

/// Slow the world down and raise the step to match. See the module docs.
#[cfg(feature = "debug")]
fn slow_the_world(world: &mut World) {
    world.insert_resource(Time::<Fixed>::from_hz(CAPTURE_PHYSICS_HZ));
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(LESSON_SLOWDOWN);
}

/// How many of the guns' rounds are in the air right now.
#[cfg(feature = "debug")]
fn rounds_in_flight(world: &mut World) -> usize {
    world
        .query_filtered::<(), With<TurretBulletProjectileMarker>>()
        .iter(world)
        .count()
}

/// Whether both rigs and all ten plates arrived.
#[cfg(feature = "debug")]
fn the_range_is_standing() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<Entity, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).count() == 2 + 2 * PLATES)
    })
}

/// Fire both guns down their lanes and record what comes back out of the
/// plates.
#[cfg(feature = "debug")]
fn rounds_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(and(the_range_is_standing(), scenario_camera_present()))
        .deadline(30.0)
        .add()
        .step("stand off the beam and lay the guns")
        .on_enter(|world: &mut World| {
            // The status bar spawns with the scenario, so it is dropped HERE
            // and not at `Startup`, where there is nothing yet to despawn.
            hide_hud(world);
            hide_status_bar(world);
            pose_camera(world, ROUNDS_EYE, ROUNDS_AIM);
            lay_the_guns(world);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        // The slowdown goes on AFTER the settle: a rig that spawned into slow
        // motion would take twenty times as long to finish arriving.
        .step("slow the world down")
        .on_enter(slow_the_world)
        .until(frames(2))
        .add()
        // The sheet opens on the muzzle flash, so the reader's first cell is
        // the two rounds leaving together - which is the half of the claim a
        // picture of the aftermath cannot make.
        .step("open the sheet and tap both triggers")
        .on_enter(|world: &mut World| {
            sheet_start(world, ROUNDS_LESSON, LESSON_GRID);
            set_triggers(world, true);
        })
        .until(frames(TRIGGER_FRAMES))
        .add()
        // A sheet of two empty lanes tiles just as cleanly as a sheet of the
        // demonstration, and is the one failure this frame cannot survive.
        .step("cease fire, and follow the two rounds down")
        .on_enter(|world: &mut World| {
            set_triggers(world, false);
            assert!(
                rounds_in_flight(world) > 0,
                "no rounds in flight after {TRIGGER_FRAMES} frames on the trigger, at {} m of \
                 travel a frame: the sheet would be a picture of ten plates. Check the mounts \
                 deployed and the aim point reached them.",
                ROUND_STRIDE.get()
            );
        })
        .until(sheet_written(ROUNDS_LESSON))
        .deadline(90.0)
        .add()
        // If the front Kinetic plate died the set changed under the recording,
        // and the lane the reader is being shown is not the lane the arithmetic
        // in the module docs describes.
        .step("the plates all survived the demonstration")
        .on_enter(|world: &mut World| {
            let standing = world
                .try_query_filtered::<Entity, With<SpaceshipRootMarker>>()
                .map_or(0, |mut query| query.iter(world).count());
            assert_eq!(
                standing,
                2 + 2 * PLATES,
                "a plate came apart inside the sheet, so the set the last cell shows is not the \
                 set the first cell showed"
            );
        })
        .until(frames(1))
        .add()
        .step("stand down")
        .on_enter(|world: &mut World| set_triggers(world, false))
        .add()
}
