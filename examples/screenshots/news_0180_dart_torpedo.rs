//! news_0180_dart_torpedo: the v0.18.0 news torpedo loop - a Dart leaving its
//! bay and running out to cruise, with the camera abeam of the round.
//!
//! The set is the two-type hollow of `lesson_combat_torpedo_types` with one
//! boat in it: the cleanup leader fixture with its Serpent bay swapped for the
//! shipped `dart_torpedo_section`, nose down a [`RUN_LENGTH`] run laid across
//! the rock shell, and a raider at the far end for the round to be committed
//! to. The run is not down world -Z, so the launch puff and the drive plume
//! are judged on a heading the particle system has to be told about.
//!
//! The camera rides abeam of the bay's muzzle until the round is in the air
//! and abeam of the round after it, on the side of the hull the muzzle is on
//! and off the authored run line, so the frame does not turn. The loop opens
//! on the quiet boat, takes the cold launch and the drive lighting, and holds
//! the run until [`RUN_SECS`] after the commit. The walk fails if the round
//! in the air is not a Dart, or if it has not passed [`CRUISE_FLOOR`] when
//! the loop closes: the Lance caps at 350 m/s and the Dart at 450.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the launch, the run and the
//!   asserts, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also record
//!   `news-0180-dart-torpedo.webm` (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0180-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0180_dart_torpedo --features debug
//! # look for: `dart torpedo: PASS a Dart ran at N m/s`
//! ```

#[path = "shared/hollow.rs"]
mod hollow;

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::Parser;
use hollow::{dev_fixtures, kit};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "news_0180_dart_torpedo")]
#[command(version = "1.0.0")]
#[command(
    about = "Record the v0.18.0 torpedo loop: a Dart leaving its bay and running to cruise",
    long_about = None
)]
struct Cli;

/// Scenario id of the boat that fires.
const BOAT_ID: &str = "dart_boat";

/// The bay the fixture carries, and the bay this set swaps into it.
const SERPENT_BAY_SECTION: &str = "torpedo_section";
const DART_BAY_SECTION: &str = "dart_torpedo_section";

/// The name the Dart's type carries in flight.
const DART: &str = "Dart";

/// How long the run is, from the boat to the raider: the hollow's own run
/// line, made long enough that a 450 m/s round is still short of its fuze
/// when the loop closes.
const RUN_LENGTH: Meters = Meters(3_400.0);

/// How far the camera stands off the run, square across it, and how far
/// above it.
const STANDOFF: Meters = Meters(150.0);
const RISE: Meters = Meters(8.0);

/// How far back along the run the camera aims from what it rides, so the
/// round sits forward of centre with its plume behind it.
const TRAIL_ROOM: Meters = Meters(20.0);

/// Seconds the loop shows the quiet boat before the launch.
const LEAD_SECS: f32 = 0.8;

/// Seconds the loop holds after the round is committed: the cold coast, the
/// drive lighting and the run.
const RUN_SECS: f32 = 5.5;

/// The speed the round must have passed when the loop closes, in meters per
/// second: above the Lance's 350 m/s cap.
const CRUISE_FLOOR: f32 = 360.0;

/// The loop this producer records.
const LOOP_NAME: &str = "news-0180-dart-torpedo";

/// The unit vector of the run, from the boat to the raider.
fn run() -> Vec3 {
    (hollow::TYPES_TARGET - hollow::TYPES_POSITION)
        .get()
        .normalize_or_zero()
}

/// Where the raider stands: the far end of the run.
fn run_end() -> Meters3 {
    hollow::TYPES_POSITION + Meters3(run() * RUN_LENGTH.get())
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    // A camera that follows a projectile solves where the camera authority
    // solves: the fixed step moves the round, and a pose taken in `Update`
    // is a part of a frame stale.
    app.add_systems(PostUpdate, ride_abeam.in_set(CameraAuthoritySystems::Solve));
    app.add_plugins(dart_script());
    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(dart_hollow(&game_assets, &ships)));
}

/// The cleanup leader with its one Serpent bay swapped for the Dart bay.
fn dart_loaded(mut design: ShipDesign) -> ShipDesign {
    let mut swapped = 0usize;
    for section in &mut design.sections {
        let SectionSource::Prototype { id, .. } = &section.source else {
            continue;
        };
        if id != SERPENT_BAY_SECTION {
            continue;
        }
        section.source = SectionSource::prototype(DART_BAY_SECTION);
        swapped += 1;
    }
    assert_eq!(
        swapped, 1,
        "the cleanup leader fixture carries the one bay this swap was written for"
    );
    design
}

/// The rock hollow with the Dart boat at one end of the run and a raider at
/// the other.
fn dart_hollow(game_assets: &GameAssets, ships: &GameShipDesigns) -> ScenarioConfig {
    let boat = hollow::ship(
        BOAT_ID,
        "Dart Boat",
        hollow::TYPES_POSITION,
        Transform::default().looking_to(run(), Vec3::Y).rotation,
        SpaceshipController::None,
        Some(Allegiance::Player),
        dart_loaded(dev_fixtures::cleanup_leader()),
        ShipInventoryStock::new([]),
    );
    let raider = hollow::ship(
        hollow::RAIDER_ID,
        "Raider",
        run_end(),
        Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        SpaceshipController::None,
        Some(Allegiance::Enemy),
        dev_fixtures::raider(),
        ShipInventoryStock::new([]),
    );
    // Every hollow framing is measured from a player parked at the origin.
    let player = hollow::ship(
        hollow::PLAYER_ID,
        "Player Ship",
        Meters3::ZERO,
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
        }),
        None,
        kit::catalog_ship(ships, "interceptor"),
        ShipInventoryStock::new([]),
    );
    let shell = kit::NearField {
        id_prefix: "dart_rock_",
        count: 14,
        seed: 40511,
        center: Meters3::ZERO,
        distance: (Meters(3_200.0), Meters(5_200.0)),
        radius: (Meters(40.0), Meters(100.0)),
        y_spread: Meters(2_400.0),
    };

    ScenarioConfig {
        description: "The rock hollow with one Dart boat and its target.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![shell.action(game_assets), player, raider, boat],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "rock_hollow_dart".to_string(),
            "Rock Hollow - Dart".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Ride abeam of the run, on the side `side` (1 or -1) of the run line:
/// inert until a step inserts it.
#[derive(Resource)]
struct AbeamRide {
    side: f32,
}

/// The unit vector square across the run, level.
fn across() -> Vec3 {
    run().cross(Vec3::Y).normalize_or_zero()
}

/// Where the boat's muzzle is: the torpedo spawner nearest the boat's post.
fn the_muzzle(world: &mut World) -> Option<Meters3> {
    let post = hollow::TYPES_POSITION.to_engine();
    let mut muzzles = world.query_filtered::<&GlobalTransform, With<TorpedoSectionSpawnerMarker>>();
    muzzles
        .iter(world)
        .map(GlobalTransform::translation)
        .min_by(|a, b| a.distance(post).total_cmp(&b.distance(post)))
        .map(Meters3::from_engine)
}

/// Stand the camera on the side of the hull the muzzle is on, so the hull
/// does not hide the launch.
fn stand_abeam_of_the_muzzle(world: &mut World) {
    let muzzle = the_muzzle(world).expect("dart torpedo: the boat has a muzzle");
    let offset = (muzzle - hollow::TYPES_POSITION).get().dot(across());
    world.insert_resource(AbeamRide {
        side: if offset < 0.0 { -1.0 } else { 1.0 },
    });
}

/// Where the round in the air is, and how fast it runs in meters per second.
///
/// The local `Transform`: a projectile's world pose for this frame is its own
/// transform, and its `GlobalTransform` is last frame's until propagation.
fn the_round(world: &mut World) -> Option<(Meters3, f32)> {
    let mut rounds = world.query_filtered::<(
        &Transform,
        &avian3d::prelude::LinearVelocity,
    ), With<TorpedoProjectileMarker>>();
    rounds.iter(world).next().map(|(transform, velocity)| {
        (
            Meters3::from_engine(transform.translation),
            MetersPerSecond3::from_engine(velocity.0).0.length(),
        )
    })
}

/// Pose the camera abeam of the round, or of the muzzle while no round flies.
fn ride_abeam(world: &mut World) {
    let Some(side) = world.get_resource::<AbeamRide>().map(|ride| ride.side) else {
        return;
    };
    let Some(subject) = the_round(world)
        .map(|(at, _)| at)
        .or_else(|| the_muzzle(world))
    else {
        return;
    };
    let at = subject - Meters3(run() * TRAIL_ROOM.get());
    let eye = at + Meters3(across() * (side * STANDOFF.get()) + Vec3::Y * RISE.get());
    pose_camera(world, eye, at);
}

/// Panic unless the round in the air is a Dart past [`CRUISE_FLOOR`].
fn assert_the_dart(world: &mut World) {
    let names: Vec<String> = world
        .query_filtered::<&TorpedoType, With<TorpedoProjectileMarker>>()
        .iter(world)
        .map(|kind| kind.name.clone())
        .collect();
    assert_eq!(
        names,
        [DART],
        "dart torpedo: the loop must close on one Dart in the air"
    );
    let (_, speed) = the_round(world).expect("dart torpedo: the round is in the air");
    assert!(
        speed >= CRUISE_FLOOR,
        "dart torpedo: the round runs at {speed:.0} m/s, under the {CRUISE_FLOOR} m/s floor"
    );
    info!("dart torpedo: PASS a Dart ran at {speed:.0} m/s");
}

fn dart_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the hollow")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), hollow::raider_present()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the hollow")
        .until(elapsed(1.0))
        .add()
        // The camera leaves the player's ship, so its instruments would be
        // chrome over another hull's ordnance.
        .step("clear the screen and stand abeam of the muzzle")
        .on_enter(|world: &mut World| {
            hollow::hud_cinematic(world);
            stand_abeam_of_the_muzzle(world);
        })
        .until(elapsed(0.5))
        .add()
        .step("open the loop on the quiet boat")
        .on_enter(|world: &mut World| loop_start(world, LOOP_NAME))
        .until(elapsed(LEAD_SECS))
        .add()
        .step("fire the bay")
        .on_enter(|world: &mut World| hollow::loose_torpedoes_from(world, BOAT_ID))
        .until(hollow::torpedo_salvo_in_flight(1))
        .deadline(6.0)
        .add()
        .step("commit the round to the raider")
        .on_enter(|world: &mut World| hollow::commit_torpedo_salvo(world, 1))
        .until(elapsed(RUN_SECS))
        .add()
        .step("read the round")
        .on_enter(assert_the_dart)
        .until(frames(1))
        .add()
        .step("close the loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(120.0)
        .add()
}
