//! thruster_section: the thruster section - burn in, thrust and plume out.
//!
//! One minimal ship (controller + hull + main drive, no player input) burns
//! at whatever throttle is written into [`ThrusterSectionInput`] - the seam
//! the key bindings, the manual-burn allocator and the autopilot write. The
//! section under test converts that 0..1 throttle into impulse on the hull
//! and into the exhaust plume's shader (the same
//! `thruster_shader_update_system` the game runs).
//!
//! The scripted run walks FIVE named invariants across three throttle rounds -
//! full, partial, released - so "the drive works" is not one binary sample:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: burn accelerates` | a full burn grows the nose speed |
//! | 2 | `outcome: plume material exists` | the drive spawned its exhaust material |
//! | 3 | `outcome: plume follows throttle` | the shader uniform sits at the held 1.0 |
//! | 4 | `outcome: partial throttle is proportional` | half throttle accelerates, strictly slower than full |
//! | 5 | `outcome: plume returns to idle` | releasing the throttle zeroes the shader |
//!
//! Invariant 4 compares two MEASURED accelerations rather than a magic
//! constant: each round divides its own speed gain by its own duration, so the
//! rate is `u/s^2` and the two rounds' windows need not match.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example thruster_section --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `burn probe: full burn accelerates the hull ...`,
//! #           `burn probe: partial throttle is proportional ...`,
//! #           `burn probe: the plume returned to idle`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use std::sync::Arc;

#[cfg(feature = "debug")]
use avian3d::prelude::{LinearVelocity, Rotation};
#[cfg(feature = "debug")]
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::nova_ship::sections::thruster_section::ThrusterExhaustMaterial;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "thruster_section")]
#[command(version = "1.0.0")]
#[command(about = "Thruster section: throttle drives thrust and the plume shader, proportionally", long_about = None)]
struct Cli;

/// The throttle the script holds. A resource rather than a hard-coded 1.0 so
/// the beats can change it and the same production seam still carries it.
#[derive(Resource)]
struct HeldThrottle(f32);

/// The partial setting round 2 burns at. Well clear of both ends so the
/// proportionality claim is about the middle of the range, not a boundary.
#[cfg(feature = "debug")]
const PARTIAL_THROTTLE: f32 = 0.4;

/// How long each burn round holds the throttle open once the drive has taken
/// it, in seconds.
///
/// A settle beat, and deliberately NOT "the hull has gained N u/s" - that is
/// the same comparison [`assert_full_burn`] makes, which would leave "a burn
/// accelerates" unfailable and surface a dead drive as a deadline stall on the
/// beat's name. The beat delivers the stimulus (the throttle reached the
/// section); whether the hull ACCELERATED is the assertions' question.
///
/// Sized off the measured full-burn acceleration (~22 u/s^2, so ~8.8 at
/// [`PARTIAL_THROTTLE`]): this window gains ~55 u/s full and ~22 partial, both
/// orders of magnitude above the speed noise the assertions compare against,
/// and both rounds still finish far inside their deadlines. [`measure_round`]
/// divides by the round's own duration, so the two rates stay comparable.
#[cfg(feature = "debug")]
const BURN_WINDOW_SECS: f32 = 2.5;

/// How many driven frames the release beat gives the shader sync. Frames, not
/// seconds: the claim is "the sync system has RUN", and under a software
/// renderer a fixed wall-clock wait buys a wildly different number of runs.
#[cfg(feature = "debug")]
const RELEASE_SYNC_FRAMES: u32 = 2;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<BurnProbe>();
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PERF_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(burn_script());
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

/// The scripted run: burn full for a [`BURN_WINDOW_SECS`] window, repeat at
/// [`PARTIAL_THROTTLE`], then release.
///
/// Not the stock `nova_autopilot()` preset: every beat waits on the drive
/// having taken the throttle or on the plumes existing, so a slow load delays
/// the walk instead of truncating it. The two settles - the burn window and
/// the release frame count - each state their reason on their constant, and
/// neither reads a quantity an assertion decides. The per-step deadlines NAME
/// the beat that stalled, and their sum (65s) stays well under
/// `DEFAULT_DEADLINE_SECS` (120s) so a named stall wins the race against the
/// generic collector deadline.
#[cfg(feature = "debug")]
fn burn_script() -> Script {
    Script::new()
        .step("load the rig")
        .enter(GameStates::Loading)
        .until(any_entity::<With<ThrusterSectionMarker>>())
        .deadline(20.0)
        .add()
        .step("hold the full burn")
        .on_enter(begin_burn(1.0))
        .until(burn_window_held(1.0))
        .deadline(20.0)
        .add()
        .step("assert the full burn")
        .on_enter(assert_full_burn)
        .add()
        .step("hold the partial throttle")
        .on_enter(begin_burn(PARTIAL_THROTTLE))
        .until(burn_window_held(PARTIAL_THROTTLE))
        .deadline(20.0)
        .add()
        .step("assert the partial throttle")
        .on_enter(assert_partial_throttle)
        .add()
        .step("release the throttle")
        .on_enter(begin_burn(0.0))
        .until(plume_idle())
        .deadline(5.0)
        .add()
        // Last beat: the driver reports done after it, so the run ends on the
        // assertion rather than idling out a runway.
        .step("assert the plume idles")
        .on_enter(assert_plume_idle)
        .add()
}

fn custom_plugin(app: &mut App) {
    app.insert_resource(HeldThrottle(1.0));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_rig);
    app.add_systems(Update, hold_throttle);
}

fn setup_rig(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(burn_rig(&game_assets, &sections)));
}

/// The rig scenario: one sectioned ship, no player and no AI - throttle
/// authority belongs to this example's burn writer alone.
fn burn_rig(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    let ship = SpaceshipConfig {
        collapse_threshold: None,
        skin: false,
        style: None,
        allegiance: None,
        controller: SpaceshipController::None,
        sections: vec![
            SpaceshipSectionConfig {
                id: "controller".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("basic_controller_section")),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "hull".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("reinforced_hull_section")),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "main_drive".to_string(),
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("basic_thruster_section")),
                modifications: vec![],
            },
        ],
    };

    ScenarioConfig {
        description: "A minimal ship under a scripted burn.".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            // The rig lights itself: the engine spawns no light, so a
            // scenario that authors none renders black.
            actions: [
                vec![EventActionConfig::SpawnScenarioObject(
                    ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "rig_ship".to_string(),
                            name: "Rig Ship".to_string(),
                            position: Vec3::new(0.0, 0.0, -12.0),
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Spaceship(ship),
                    },
                )],
                ThreePointRig::around("rig", Vec3::new(0.0, 0.0, -12.0), 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "thruster_rig".to_string(),
            "Thruster Section Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Hold the drive at the script's throttle - the seam the key bindings and
/// the manual-burn allocator write.
fn hold_throttle(
    throttle: Res<HeldThrottle>,
    mut q_input: Query<
        &mut ThrusterSectionInput,
        (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>),
    >,
) {
    for mut input in &mut q_input {
        if input.0 != throttle.0 {
            input.0 = throttle.0;
        }
    }
}

/// The burn round in flight: what the hull was doing when the throttle was
/// opened, and the rate the last finished round measured.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct BurnProbe {
    /// Nose speed and wall-clock time at the start of the current round.
    round: Option<(f32, f32)>,
    /// Acceleration the full-burn round measured, u/s^2.
    full_rate: Option<f32>,
}

/// Hull speed along its own nose (-Z), the axis the drive pushes on.
#[cfg(feature = "debug")]
fn nose_speed(world: &World) -> Option<f32> {
    let mut query =
        world.try_query_filtered::<(&Rotation, &LinearVelocity), With<SpaceshipRootMarker>>()?;
    let (rotation, velocity) = query.iter(world).next()?;
    Some(velocity.0.dot(rotation.0 * Vec3::NEG_Z))
}

/// Open the throttle to `throttle` and mark the round's starting speed and
/// time, so the beat after it can divide the gain by the time it took.
#[cfg(feature = "debug")]
fn begin_burn(throttle: f32) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let speed = nose_speed(world).expect("burn probe: the rig ship must exist");
        let now = world.resource::<Time>().elapsed_secs();
        world.resource_mut::<BurnProbe>().round = Some((speed, now));
        world.resource_mut::<HeldThrottle>().0 = throttle;
        info!("burn probe: throttle {throttle:.2} from {speed:.3} u/s");
    }
}

/// The beat a burn round waits on: the drive has TAKEN the round's throttle,
/// and has then held it for [`BURN_WINDOW_SECS`].
///
/// The throttle clause reads the production seam `hold_throttle` writes, so a
/// binding that never reaches the section stalls this beat by name instead of
/// silently handing the assertions a hull nothing ever pushed. Nothing here
/// reads the hull's SPEED - that is what the assertions decide.
#[cfg(feature = "debug")]
fn burn_window_held(throttle: f32) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    and(
        elapsed(BURN_WINDOW_SECS),
        Arc::new(move |world: &World| {
            world
                .try_query_filtered::<&ThrusterSectionInput, (
                    With<ThrusterSectionMarker>,
                    Without<SectionInactiveMarker>,
                )>()
                .is_some_and(|mut query| {
                    let mut inputs = query.iter(world).peekable();
                    inputs.peek().is_some() && inputs.all(|input| (input.0 - throttle).abs() < 1e-6)
                })
        }),
    )
}

/// The finished round's acceleration in u/s^2, plus the raw gain, read at
/// assert time. Dividing by the round's own duration is what makes the two
/// rounds comparable without pinning them to equal-length windows.
#[cfg(feature = "debug")]
fn measure_round(world: &mut World) -> (f32, f32) {
    let (baseline, started) = world
        .resource::<BurnProbe>()
        .round
        .expect("burn probe: a round must be in flight at assert time");
    let speed = nose_speed(world).expect("burn probe: the rig ship must exist");
    let seconds = world.resource::<Time>().elapsed_secs() - started;
    assert!(
        seconds > 0.0,
        "burn probe: a burn round cannot take zero seconds"
    );
    let gain = speed - baseline;
    (gain, gain / seconds)
}

/// Every exhaust plume material handle the drive spawned.
#[cfg(feature = "debug")]
fn plume_handles(
    world: &World,
) -> Vec<Handle<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>> {
    let Some(mut query) = world
        .try_query::<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>>(
        )
    else {
        return Vec::new();
    };
    query
        .iter(world)
        .map(|material| material.0.clone())
        .collect()
}

/// The shader uniform on every plume, as the production sync
/// (`thruster_shader_update_system`) left it. Empty when the drive has not
/// spawned its plume yet.
#[cfg(feature = "debug")]
fn plume_inputs(world: &World) -> Vec<f32> {
    let handles = plume_handles(world);
    let Some(materials) =
        world.get_resource::<Assets<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>>()
    else {
        return Vec::new();
    };
    handles
        .iter()
        .filter_map(|handle| materials.get(handle))
        .map(|material| material.extension.thruster_input)
        .collect()
}

/// The release beat's condition: the plumes are still there and the shader sync
/// has HAD ITS TURN since the throttle was zeroed.
///
/// A settle beat, and the reason is exact rather than guessed: the sync
/// (`thruster_shader_update_system`) ASSIGNS the input rather than lerping it,
/// so whatever it is going to write it has written by the frame after the
/// release - [`RELEASE_SYNC_FRAMES`] gives it two.
///
/// Deliberately NOT "every plume input is back to 0", which is what invariant 5
/// then asserts. Gating on the assert's own condition made the assert
/// unfailable: a plume latched on by the first burn could only ever surface as
/// a deadline stall on this beat's name, never as the invariant message that
/// explains it.
#[cfg(feature = "debug")]
fn plume_idle() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    and(
        frames(RELEASE_SYNC_FRAMES),
        Arc::new(|world: &World| !plume_inputs(world).is_empty()),
    )
}

/// Round 1: invariants 1 (the full burn accelerated the hull), 2 (the drive
/// spawned its plume material) and 3 (the shader followed the held throttle).
#[cfg(feature = "debug")]
fn assert_full_burn(world: &mut World) {
    let (gain, rate) = measure_round(world);
    assert!(
        gain > 0.0,
        "burn probe: nose speed must grow under a full burn, gained {gain:.3} u/s"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!("burn probe: full burn accelerates the hull ({rate:.3} u/s^2)");
    nova_probe::probe_marker(
        world,
        "outcome: burn accelerates",
        serde_json::json!({ "t": elapsed, "gain": gain, "rate": rate }),
    );

    // The plume follows the throttle through the production sync
    // (thruster_shader_update_system), not through anything this example
    // wired by hand.
    let inputs = plume_inputs(world);
    assert!(
        !inputs.is_empty(),
        "burn probe: the drive must have spawned its exhaust plume material"
    );
    nova_probe::probe_marker(
        world,
        "outcome: plume material exists",
        serde_json::json!({ "t": elapsed, "plumes": inputs.len() }),
    );

    for input in &inputs {
        assert!(
            (input - 1.0).abs() < 1e-6,
            "burn probe: plume shader input {input} did not follow the held throttle (1.0)"
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: plume follows throttle",
        serde_json::json!({ "t": elapsed, "inputs": inputs }),
    );

    world.resource_mut::<BurnProbe>().full_rate = Some(rate);
}

/// Round 2: invariant 4 - the partial setting still accelerates, and does it
/// strictly slower than the full burn MEASURED in round 1. Two measurements
/// compared against each other, so no magic constant encodes the drive's
/// tuning.
#[cfg(feature = "debug")]
fn assert_partial_throttle(world: &mut World) {
    let (gain, rate) = measure_round(world);
    let full_rate = world
        .resource::<BurnProbe>()
        .full_rate
        .expect("burn probe: the full-burn round must have measured its rate first");
    assert!(
        rate > 0.0,
        "burn probe: {PARTIAL_THROTTLE} throttle must still accelerate the hull, \
         measured {rate:.3} u/s^2"
    );
    assert!(
        rate < full_rate,
        "burn probe: {PARTIAL_THROTTLE} throttle accelerated at {rate:.3} u/s^2, \
         not below the full burn's {full_rate:.3} u/s^2 - thrust does not follow \
         the throttle"
    );
    info!("burn probe: partial throttle is proportional ({rate:.3} < {full_rate:.3} u/s^2)");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: partial throttle is proportional",
        serde_json::json!({ "t": elapsed, "gain": gain, "rate": rate, "full_rate": full_rate }),
    );
}

/// Round 3: invariant 5 - releasing the throttle takes the plume back to
/// idle, so the shader is driven by the input rather than latched on by the
/// first burn.
#[cfg(feature = "debug")]
fn assert_plume_idle(world: &mut World) {
    let inputs = plume_inputs(world);
    assert!(
        !inputs.is_empty(),
        "burn probe: the plume material must still exist after the release"
    );
    for input in &inputs {
        assert!(
            input.abs() < 1e-6,
            "burn probe: plume shader input {input} did not return to idle after \
             the throttle was released"
        );
    }
    info!("burn probe: the plume returned to idle");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: plume returns to idle",
        serde_json::json!({ "t": elapsed, "inputs": inputs }),
    );
}
