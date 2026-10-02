//! lesson_mining: the FLIGHT lesson `flight_mine` - one press of MINE on a
//! locked rock, from the shut emitter to the beam cutting ore.
//!
//! The scene is `screenshot_mining_beam`'s: the shipped `block_line_warship`
//! parked at the origin with one ore rock dead ahead of its bow emitter. The
//! script travel-locks the rock and holds the real `mine` action (`V`), so the
//! doors, the tip, the pulse, the sparks and the carve are the production path.
//!
//! ## An ACTION loop
//!
//! The twenty cells hold the shut emitter, the press, the doors parting, the
//! tip running out, and one paying pulse with its sparks and the canister it
//! frees. Press to first pulse takes most of the sheet, so the release is not
//! in it: the sheet wraps from the cutting beam back to the shut emitter, a
//! repeat of the press. The camera holds still, close on the emitter with the
//! rock's near face in the cell.
//!
//! Nothing is frozen: the canister the pulse frees has to drift off the cut.
//!
//! ## Why the rock is primed before the sheet
//!
//! The first pulse into an untouched rock only seeds its field and cuts
//! nothing (`crates/nova_scenario/src/mining.rs`). A sheet that recorded that
//! pulse would show a beam that takes no ore, which is the opposite of the
//! lesson. So the walk fires one unrecorded pulse first, lets the emitter shut,
//! and the recorded pulse is the first one that cuts.
//!
//! The run fails loudly if any pulse is refused, if the priming pulse is not
//! the seed, or if the recorded press pays no ore.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - prime, press, check, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_mining --features debug
//! ```

#[path = "shared/kit.rs"]
mod kit;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_mining")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's MINE demonstration", long_about = None)]
struct Cli;

/// The sheet for "MINE".
#[cfg(feature = "debug")]
const MINE_LESSON: &str = "flight_mine";

const PLAYER_ID: &str = "player";
const ROCK_ID: &str = "ore_rock";

/// The rock of `screenshot_mining_beam`: dead ahead of the emitter, its
/// meshed near face about 58 m out, clear of the bow.
const ROCK_CENTRE: Vec3 = Vec3::new(1.0, 1.0, -23.5);
const ROCK_RADIUS: Meters = Meters(25.0);
const ROCK_SEED: u32 = 7;

/// Where the eye stands, in engine units from the warship's root: above,
/// to starboard and ahead of the emitter face, so the face with its doors and
/// tip, the beam and its hit on the rock's near face share the cell.
#[cfg(feature = "debug")]
const EYE: Vec3 = Vec3::new(7.0, 6.0, -12.0);

/// What the eye looks at: on the beam line, between the emitter face and the
/// rock's near face.
#[cfg(feature = "debug")]
const AIM: Vec3 = Vec3::new(1.0, 1.0, -10.3);

/// Cells the sheet opens on before the key goes down: the shut emitter.
#[cfg(feature = "debug")]
const LEAD_CELLS: u32 = 2;

/// Real-seconds backstop for a state wait, long enough for lavapipe frames.
#[cfg(feature = "debug")]
const WAIT_DEADLINE_SECS: f32 = 90.0;

/// Every [`MiningPulse`] outcome in order.
#[derive(Resource, Default)]
struct PulseLog(Vec<Result<u32, MiningRefusalType>>);

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
        app.add_plugins(mining_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.init_resource::<PulseLog>();
    app.add_observer(|pulse: On<MiningPulse>, mut log: ResMut<PulseLog>| {
        log.0.push(pulse.outcome);
    });
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    let player = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player Ship".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            design: ShipDesignSource::Inline(kit::catalog_ship(&ships, "block_line_warship")),
            inventory: ShipInventoryStock::new([]),
            ..default()
        }),
    });
    let rock = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ROCK_ID.to_string(),
            name: "Ore Rock".to_string(),
            position: Meters3::from_engine(ROCK_CENTRE),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: ROCK_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
            kind: KIND_ROCK.into(),
            destroy_sound: None,
            mass: None,
            lock_signature: None,
            seed: Some(ROCK_SEED),
        }),
    });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "The line warship's mining beam on one locked rock.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![player, rock],
                ThreePointRig::around("mine", Meters3::from_engine(ROCK_CENTRE), 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "lesson_mining".to_string(),
            "MINE".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// The emitter's door and tip progress (1 is stowed) and whether it has a hit.
#[cfg(feature = "debug")]
fn emitter_state(world: &World) -> Option<(f32, f32, bool)> {
    let mut emitters = world
        .try_query_filtered::<(&SectionAnimations, Has<MiningBeamHit>), With<MiningEmitter>>()?;
    let (animations, hit) = emitters.iter(world).next()?;
    Some((
        animations.cue_progress(SectionAnimationCue::StowDoors)?,
        animations.cue_progress(SectionAnimationCue::StowLift)?,
        hit,
    ))
}

#[cfg(feature = "debug")]
fn when(
    check: impl Fn(&World) -> bool + Send + Sync + 'static,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(check)
}

/// Advance once the emitter is fully shut with no hit.
#[cfg(feature = "debug")]
fn emitter_shut() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    when(|world| emitter_state(world) == Some((1.0, 1.0, false)))
}

/// Advance once more than `pulses` pulses are logged.
#[cfg(feature = "debug")]
fn pulses_past(pulses: usize) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    when(move |world| world.resource::<PulseLog>().0.len() > pulses)
}

/// Lock the rock and hold the camera beside the beam.
#[cfg(feature = "debug")]
fn lock_the_rock(world: &mut World) {
    let player = kit::ship_root(world, PLAYER_ID).expect("the warship spawned");
    let rock = {
        let mut rocks = world.query_filtered::<(Entity, &EntityId), With<AsteroidMarker>>();
        rocks
            .iter(world)
            .find(|(_, id)| id.0 == ROCK_ID)
            .map(|(entity, _)| entity)
            .expect("the rock spawned")
    };
    world.entity_mut(player).insert(TravelLock(Some(rock)));
    pose_camera(world, Meters3::from_engine(EYE), Meters3::from_engine(AIM));
}

/// The seed pulse is the only pulse so far, and it passed.
#[cfg(feature = "debug")]
fn check_the_seed(world: &mut World) {
    let log = world.resource::<PulseLog>().0.clone();
    info!("lesson_mining: priming pulse log {log:?}");
    assert_eq!(
        log,
        vec![Ok(0)],
        "the priming press fires one pulse that seeds the rock and cuts nothing"
    );
}

/// Every recorded pulse passed and at least one paid ore.
#[cfg(feature = "debug")]
fn check_the_cut(world: &mut World) {
    let log = world.resource::<PulseLog>().0.clone();
    info!("lesson_mining: pulse log {log:?}");
    assert!(
        log.iter().all(Result::is_ok),
        "a pulse was refused: {log:?}"
    );
    assert!(
        log[1..]
            .iter()
            .any(|outcome| outcome.is_ok_and(|corners| corners > 0)),
        "the recorded press cut no ore: {log:?}"
    );
}

/// Lock the rock, prime it with one unrecorded pulse, then record one press of
/// MINE from the shut emitter to a paying pulse.
#[cfg(feature = "debug")]
fn mining_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the rock and the warship")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), scenario_camera_present()))
        .deadline(WAIT_DEADLINE_SECS)
        .add()
        .step("lock the rock")
        .on_enter(lock_the_rock)
        .until(and(scenario_is_built(), frames(10)))
        .deadline(WAIT_DEADLINE_SECS)
        .add()
        // The status bar carries the build's commit and the capture rig's
        // frame rate. Dropped before any recording.
        .step("drop the status bar")
        .on_enter(hide_status_bar)
        .until(frames(1))
        .add()
        .step("prime the rock: hold the mine key until the seed pulse")
        .on_enter(press_action("mine"))
        .until(pulses_past(0))
        .deadline(WAIT_DEADLINE_SECS)
        .add()
        .step("release and wait for the emitter to shut")
        .on_enter(|world: &mut World| {
            release_action("mine")(world);
            check_the_seed(world);
        })
        .until(emitter_shut())
        .deadline(WAIT_DEADLINE_SECS)
        .add()
        .step("open the sheet on the shut emitter")
        .on_enter(|world: &mut World| sheet_start(world, MINE_LESSON, LESSON_GRID))
        .until(frames(LEAD_CELLS))
        .add()
        .step("hold the mine key until the beam cuts")
        .on_enter(press_action("mine"))
        .until(and(
            pulses_past(1),
            when(|world| emitter_state(world).is_some_and(|(_, _, hit)| hit)),
        ))
        .deadline(WAIT_DEADLINE_SECS)
        .add()
        .step("hold the beam until the sheet finishes recording")
        .until(sheet_written(MINE_LESSON))
        .deadline(60.0)
        .add()
        .step("release and wait for the emitter to shut")
        .on_enter(release_action("mine"))
        .until(emitter_shut())
        .deadline(WAIT_DEADLINE_SECS)
        .add()
        .step("check the recorded press cut ore")
        .on_enter(check_the_cut)
        .until(frames(1))
        .add()
}
