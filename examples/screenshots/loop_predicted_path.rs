//! loop_predicted_path: a GOTO near a planetoid, shot to show the flight
//! computer's forecast bending with the well and the ship flying the curve it
//! drew.
//!
//! The production prediction (`nova_ship::flight::prediction`) and the holo
//! ribbon it feeds (`nova_hud::holo_instruments`) draw the curve; the script
//! only engages the real `G` key over a travel lock, frames the chase, and
//! records.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - fly the leg, check the bend
//!   and the followed curve, exit clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the `news-0150-predicted-path`
//!   webm loop (staged under `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0150-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_predicted_path --features debug
//! ```

#[path = "shared/ring.rs"]
mod ring;

#[cfg(feature = "debug")]
use std::{sync::Arc, time::Duration};

#[cfg(feature = "debug")]
use avian3d::prelude::*;
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::nova_debug::harness::Predicate;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_predicted_path")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture a GOTO forecast bending with a well and the ship flying the drawn curve",
    long_about = None
)]
struct Cli;

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "news-0150-predicted-path";

const PLAYER_ID: &str = "path_player";
const BEACON_ID: &str = "path_beacon";

/// Start, on the ring's own well (mu 60000, mean radius 900 m, so a 940.5 m
/// surface - [`ring::planetoid`]'s own sizing).
const START: Meters3 = Meters3::new(-4_000.0, 0.0, 2_500.0);
/// The leg's target: with [`START`], an 8 km leg whose chord passes 2.5 km
/// from the well's centre - inside the pull, clear of the 940.5 m surface, and
/// long enough to outrun the 30 s prediction horizon, so the dim guide draws
/// past the end of the curve.
const BEACON: Meters3 = Meters3::new(4_000.0, 0.0, 2_500.0);
/// Radar signature; the same ratio [`ring::BEACON_SIGNATURE`] uses, so the
/// lock bracket reads at scale.
const BEACON_SIGNATURE: Meters = Meters(400.0);

/// How long the scene settles before the walk engages anything.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = 0.5;
/// Real-seconds backstop for the predicted curve and guide to appear: the
/// predictor runs its 30 s horizon in passes of 120 ticks, so a cold seed
/// takes several frames even on a software-rendered GPU.
#[cfg(feature = "debug")]
const PREDICTION_DEADLINE_SECS: f32 = 30.0;
/// Recorded frames: the loop profile's 30 fps makes this 15 s, comfortably
/// under the 600-frame cap and long enough to show the curve bending and the
/// ship following it.
#[cfg(feature = "debug")]
const RECORD_FRAMES: u32 = 450;
/// Real-seconds backstop for the loop file to land once its last frame is in.
#[cfg(feature = "debug")]
const LOOP_CLOSE_DEADLINE_SECS: f32 = 60.0;

/// The drawn curve's least bend off the straight chord to the beacon: well
/// above ribbon decimation and physics step noise. Unproven against a run:
/// a failure here means the well does not visibly bend this leg.
#[cfg(feature = "debug")]
const BEND_MIN: Meters = Meters(150.0);
/// The flown track's least bend off its own chord. Smaller than [`BEND_MIN`]
/// because the recording covers only the first 15 s of the leg.
#[cfg(feature = "debug")]
const TRACK_BEND_MIN: Meters = Meters(30.0);
/// How far the ship's centre of mass may stray from its own short-horizon
/// forecast before the walk calls the follow broken.
#[cfg(feature = "debug")]
const FOLLOW_TOL: Meters = Meters(50.0);
/// How old a stored forecast may be before `trace_track` stops comparing
/// against it: the prediction re-solves as the leg proceeds, so a stale
/// forecast is not ground truth for where the ship is now.
#[cfg(feature = "debug")]
const FOLLOW_HORIZON_SECS: f32 = 4.0;
/// Sim-seconds between two stored forecast snapshots.
#[cfg(feature = "debug")]
const SNAPSHOT_INTERVAL_SECS: f32 = 2.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(path_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, ring::drive_leg_camera);
        // `Time<Fixed>` is what `FlightPrediction::seed_time` and the holo
        // ribbon's own `now` are measured in, so the trace reads the same
        // clock rather than a rendered-frame approximation of it.
        app.add_systems(
            FixedPostUpdate,
            trace_track.run_if(any_with_component::<TrackTrace>),
        );
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

/// The scene: [`ring::planetoid`]'s well, the player parked at [`START`]
/// facing the beacon, and a local survey beacon at [`BEACON`].
fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    // `ring`'s own catalog lookup (`shared/kit.rs::catalog_ship`) is behind a
    // private `mod kit` inside `ring.rs`, unreachable from here - this is the
    // same lookup it performs, read straight off `GameShipDesigns`.
    let hull = ships
        .get_design(&"block_line_warship".into())
        .unwrap_or_else(|| panic!("loop_predicted_path: unknown ship 'block_line_warship'"))
        .design
        .clone();
    let facing = Quat::from_rotation_arc(
        Vec3::NEG_Z,
        (BEACON.get() - START.get())
            .try_normalize()
            .expect("loop_predicted_path: START and BEACON must differ"),
    );
    let player = ring::ship(
        PLAYER_ID,
        "Path Player",
        START,
        facing,
        SpaceshipController::Player(PlayerControllerConfig {
            // The walk never fires the beam; the scenario lint refuses a player
            // ship whose mining section has no binding.
            input_mapping: std::collections::BTreeMap::from([(
                "mining_beam".to_string(),
                vec![nova_input::prelude::InputSource::Keyboard(KeyCode::KeyV)],
            )]),
        }),
        None,
        hull,
    );
    let beacon = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BEACON_ID.to_string(),
            name: "Survey Beacon".to_string(),
            position: BEACON,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "SURVEY".to_string(),
            radius: Meters(60.0),
            color: Color::srgb(0.4, 0.75, 1.0),
            area_radius: None,
            lock_signature: Some(BEACON_SIGNATURE),
        }),
    });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "A planetoid well and a ship flying its GOTO forecast past it.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![ring::planetoid(), player, beacon],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "loop_predicted_path".to_string(),
            "Predicted Path".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// One forecast taken off the live [`FlightPrediction`], held until it ages
/// past [`FOLLOW_HORIZON_SECS`].
#[cfg(feature = "debug")]
#[derive(Clone)]
struct PathSnapshot {
    points: Vec<Vec3>,
    seed_time: Duration,
    sample_interval: f32,
}

/// The follow proof `trace_track` builds while the recorded leg flies: the
/// forecasts taken so far, the flown centre-of-mass track, and how many
/// short-horizon comparisons held (every one that did not is a panic, not a
/// recorded failure).
#[cfg(feature = "debug")]
#[derive(Component, Default)]
struct TrackTrace {
    snapshots: Vec<PathSnapshot>,
    flown: Vec<Vec3>,
    follow_checks: u32,
    worst_follow: f32,
}

/// Take a forecast snapshot every [`SNAPSHOT_INTERVAL_SECS`], record the
/// ship's flown centre of mass, and check every forecast younger than
/// [`FOLLOW_HORIZON_SECS`] against where the ship actually is: linearly
/// interpolate the stored points at the forecast's own age and compare to the
/// live centre of mass. A miss past [`FOLLOW_TOL`] panics at once - the
/// ship's own CoM from avian `Position`/`Rotation`/`ComputedCenterOfMass`
/// (`crates/nova_ship/src/flight/tests/prediction.rs`'s own comparison), not
/// the eased render `Transform` the ribbon draws from.
#[cfg(feature = "debug")]
fn trace_track(
    time: Res<Time<Fixed>>,
    mut q_player: Query<
        (
            &Position,
            &Rotation,
            &ComputedCenterOfMass,
            Option<&FlightPrediction>,
            &mut TrackTrace,
        ),
        With<PlayerSpaceshipMarker>,
    >,
) {
    let Ok((position, rotation, com, prediction, mut trace)) = q_player.single_mut() else {
        return;
    };
    let now = time.elapsed();
    let ship_com = rotation.mul_vec3(com.0) + position.0;
    trace.flown.push(ship_com);

    if let Some(prediction) = prediction {
        // Measured seed to seed, so a prediction that has not reseeded is
        // never stored twice.
        let due = trace.snapshots.last().is_none_or(|snapshot| {
            prediction
                .seed_time
                .saturating_sub(snapshot.seed_time)
                .as_secs_f32()
                >= SNAPSHOT_INTERVAL_SECS
        });
        if due {
            trace.snapshots.push(PathSnapshot {
                points: prediction.points.clone(),
                seed_time: prediction.seed_time,
                sample_interval: prediction.sample_interval,
            });
        }
    }

    trace.snapshots.retain(|snapshot| {
        now.saturating_sub(snapshot.seed_time).as_secs_f32() <= FOLLOW_HORIZON_SECS
    });

    // Cloned so the comparison loop can freely update `trace.follow_checks` /
    // `trace.worst_follow` without holding the snapshots borrow open.
    let snapshots = trace.snapshots.clone();
    for snapshot in &snapshots {
        let age = now.saturating_sub(snapshot.seed_time).as_secs_f32();
        let last = snapshot.points.len().saturating_sub(1);
        let grid = (age / snapshot.sample_interval).min(last as f32);
        let low = (grid.floor() as usize).min(last);
        let high = (low + 1).min(last);
        if low == high {
            continue;
        }
        let predicted = snapshot.points[low].lerp(snapshot.points[high], grid - low as f32);
        let deviation = predicted.distance(ship_com);
        assert!(
            deviation <= FOLLOW_TOL.to_engine(),
            "loop_predicted_path: the ship strayed {deviation:.1} units from its own \
             {age:.1} s forecast, past the {:.1} unit tolerance",
            FOLLOW_TOL.to_engine()
        );
        trace.follow_checks += 1;
        trace.worst_follow = trace.worst_follow.max(deviation);
    }
}

/// Advance once the player's [`TrackTrace`] holds a first forecast snapshot.
#[cfg(feature = "debug")]
fn snapshot_ready() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        ring::player_root_ref(world)
            .and_then(|player| world.get::<TrackTrace>(player))
            .is_some_and(|trace| !trace.snapshots.is_empty())
    })
}

/// Perpendicular distance of `point` from the chord `a -> b`, falling back to
/// the distance to `a` for a degenerate chord. Shared by [`assert_bend`] and
/// [`assert_flew_curve`]: one measure, the drawn curve and the flown track.
#[cfg(feature = "debug")]
fn perpendicular_distance(point: Vec3, a: Vec3, b: Vec3) -> f32 {
    let chord = b - a;
    let length = chord.length();
    if length <= f32::EPSILON {
        return (point - a).length();
    }
    (point - a).reject_from_normalized(chord / length).length()
}

/// The ribbon on the player ship: at least one curve segment, exactly one
/// guide segment, and nothing hidden.
#[cfg(feature = "debug")]
fn assert_ribbon(world: &mut World) {
    let player = ring::player_root(world).expect("loop_predicted_path: the player ship is present");
    let (mut curve, mut guide, mut hidden) = (0u32, 0u32, 0u32);
    let mut segments = world.query::<(&TrajectoryRibbonSegment, &Visibility)>();
    for (segment, visibility) in segments.iter(world) {
        if segment.ship != player {
            continue;
        }
        if segment.guide {
            guide += 1;
        } else {
            curve += 1;
        }
        if *visibility == Visibility::Hidden {
            hidden += 1;
        }
    }
    info!(
        "loop_predicted_path: ribbon has {curve} curve, {guide} guide, {hidden} hidden segment(s)"
    );
    assert!(
        curve >= 1,
        "loop_predicted_path: the ribbon drew no curve segment"
    );
    assert_eq!(
        guide, 1,
        "loop_predicted_path: the ribbon must draw exactly one guide segment"
    );
    assert_eq!(hidden, 0, "loop_predicted_path: a ribbon segment is hidden");
}

/// The drawn curve's max perpendicular distance off the chord from its own
/// first point to the beacon is at least [`BEND_MIN`].
#[cfg(feature = "debug")]
fn assert_bend(world: &mut World) {
    let player = ring::player_root(world).expect("loop_predicted_path: the player ship is present");
    let beacon =
        ring::entity_by_id(world, BEACON_ID).expect("loop_predicted_path: the beacon is present");
    let beacon_position = world
        .get::<GlobalTransform>(beacon)
        .expect("loop_predicted_path: the beacon has a transform")
        .translation();
    let prediction = world
        .get::<FlightPrediction>(player)
        .expect("loop_predicted_path: the player has a FlightPrediction");
    let chord_start = prediction.points[0];
    let bend = prediction
        .points
        .iter()
        .map(|&point| perpendicular_distance(point, chord_start, beacon_position))
        .fold(0.0_f32, f32::max);
    info!("loop_predicted_path: drawn curve bends {bend:.1} units off the chord to the beacon");
    assert!(
        bend >= BEND_MIN.to_engine(),
        "loop_predicted_path: the drawn curve only bent {bend:.1} units, short of the {:.1} unit \
         floor",
        BEND_MIN.to_engine()
    );
}

/// The follow proof: every short-horizon comparison held (so `trace_track`
/// never panicked), and the flown track itself bends at least
/// [`TRACK_BEND_MIN`] off its own first-to-last chord - the ship actually flew
/// a curve, not a straight line sampled coarsely enough to look like one.
#[cfg(feature = "debug")]
fn assert_flew_curve(world: &mut World) {
    let player = ring::player_root(world).expect("loop_predicted_path: the player ship is present");
    let trace = world
        .get::<TrackTrace>(player)
        .expect("loop_predicted_path: the player carries a TrackTrace");
    assert!(
        trace.follow_checks > 0,
        "loop_predicted_path: the ship was never compared against its own short-horizon forecast"
    );
    let first = *trace
        .flown
        .first()
        .expect("loop_predicted_path: the flown track has a first point");
    let last = *trace
        .flown
        .last()
        .expect("loop_predicted_path: the flown track has a last point");
    let bend = trace
        .flown
        .iter()
        .map(|&point| perpendicular_distance(point, first, last))
        .fold(0.0_f32, f32::max);
    info!(
        "loop_predicted_path: {} follow check(s), worst deviation {:.1}, flown track bends \
         {bend:.1} units",
        trace.follow_checks, trace.worst_follow
    );
    assert!(
        bend >= TRACK_BEND_MIN.to_engine(),
        "loop_predicted_path: the flown track only bent {bend:.1} units, short of the {:.1} unit \
         floor",
        TRACK_BEND_MIN.to_engine()
    );
}

/// Press and release a key action, one frame each, until `landed`.
#[cfg(feature = "debug")]
fn tap(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    label: &str,
    action: &'static str,
    landed: Arc<Predicate>,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script
        .step(format!("press {label}"))
        .on_enter(press_action(action))
        .until(frames(1))
        .add()
        .step(format!("let {label} up"))
        .on_enter(release_action(action))
        .until(landed)
        .deadline(BEAT_DEADLINE_SECS)
        .add()
}

/// The walk: load, lock the beacon, fly the real `G` key, check the drawn
/// curve, then chase the ship and check it flies the curve it drew.
#[cfg(feature = "debug")]
fn path_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the scene")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("show the HUD")
        .on_enter(ring::hud_instrument)
        .until(frames(1))
        .add()
        .step("settle")
        .until(elapsed(SETTLE_SECS))
        .add()
        .step("lock the survey beacon")
        .on_enter(|world: &mut World| {
            let player =
                ring::player_root(world).expect("loop_predicted_path: the player ship is present");
            let beacon = ring::entity_by_id(world, BEACON_ID)
                .expect("loop_predicted_path: the beacon is present");
            world.entity_mut(player).insert(TravelLock(Some(beacon)));
        })
        .until(frames(1))
        .add();
    let script = tap(
        script,
        "the GOTO key",
        "autopilot_goto",
        any_entity::<(With<PlayerSpaceshipMarker>, With<Autopilot>)>(),
    );
    let leg_under_way = and(
        and(
            Arc::new(|world: &World| {
                ring::player_root_ref(world)
                    .and_then(|player| world.get::<FlightPrediction>(player))
                    .is_some_and(|prediction| prediction.points.len() >= 3)
            }) as Arc<Predicate>,
            // The drive phase, not `ring::player_burning`: that waits for a
            // 200 m/s closing speed, which opens the loop deep into the leg.
            Arc::new(|world: &World| ring::autopilot_phase(world) == Some(AutopilotPhase::Burn))
                as Arc<Predicate>,
        ),
        Arc::new(|world: &World| {
            ring::player_root_ref(world).is_some_and(|player| {
                world
                    .try_query::<&TrajectoryRibbonSegment>()
                    .is_some_and(|mut segments| {
                        segments
                            .iter(world)
                            .any(|segment| segment.ship == player && segment.guide)
                    })
            })
        }) as Arc<Predicate>,
    );
    script
        .step("wait for the predicted curve and guide")
        .until(leg_under_way)
        .deadline(PREDICTION_DEADLINE_SECS)
        .add()
        .step("check the drawn curve")
        .on_enter(assert_ribbon)
        .on_enter(assert_bend)
        .until(frames(1))
        .add()
        .step("chase the ship down the curve")
        .on_enter(|world: &mut World| {
            ring::chase(
                world,
                Meters(60.0),
                Meters(160.0),
                Meters(70.0),
                Meters(400.0),
            );
            loop_start(world, LOOP_NAME);
            let player =
                ring::player_root(world).expect("loop_predicted_path: the player ship is present");
            world.entity_mut(player).insert(TrackTrace::default());
        })
        .until(frames(1))
        .add()
        .step("wait for the first forecast snapshot")
        .until(snapshot_ready())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fly the curve")
        .until(frames(RECORD_FRAMES))
        .add()
        .step("check the flown curve")
        .on_enter(assert_flew_curve)
        .until(frames(1))
        .add()
        .step("close the predicted-path loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(LOOP_CLOSE_DEADLINE_SECS)
        .add()
}
