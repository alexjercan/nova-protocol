//! screenshot_goto_moving_target: a GOTO at a hauler that coasts on a circular
//! orbit of a planetoid, shot on the HUD and on the Map to show the forecast
//! curving with a moving, well-pulled target and the dim guide running on to
//! the hauler's live position.
//!
//! The production prediction (`nova_ship::flight::prediction`), the holo
//! ribbon (`nova_hud::holo_instruments`) and the Map route
//! (`nova_interface::map::scene`) draw everything; the script only engages the
//! real `G` key over a travel lock, opens the interface, and shoots.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - fly the leg and check the
//!   target, the ribbon and the followed forecast, writing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the HUD and Map stills
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! The Map route strokes are crate-private to `nova_interface`, so the Map
//! still is checked by eye only. One fixture: it does not prove every moving
//! target.
//!
//! ```text
//! NOVA_CAPTURE_DIR=/tmp/goto-moving-target NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_goto_moving_target --features debug
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
#[cfg(feature = "debug")]
use nova_protocol::nova_interface::pane::InterfacePaneType;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_goto_moving_target")]
#[command(version = "1.0.0")]
#[command(
    about = "Shoot a GOTO forecast curving with an orbiting target, on the HUD and the Map",
    long_about = None
)]
struct Cli;

const PLAYER_ID: &str = "moving_player";
const TARGET_ID: &str = "moving_hauler";

#[cfg(feature = "debug")]
const HUD_SHOT: &str = "goto-moving-target-hud.png";
#[cfg(feature = "debug")]
const MAP_SHOT: &str = "goto-moving-target-map.png";

/// The player's start: outside the hauler's orbit on the hauler's side of the
/// planetoid, so the intercept does not cross the planetoid.
const START: Meters3 = Meters3::new(-3_000.0, 0.0, -4_500.0);
/// The hauler's start: 4 km from the planetoid's centre, clear of its 940.5 m
/// surface and inside the unfaded core of its 4.9 km SOI, where the pull is
/// the full `mu / r^2` a circular orbit needs.
const TARGET_START: Meters3 = Meters3::new(0.0, 0.0, -4_000.0);

/// How long the scene settles before the walk engages anything.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = 0.5;
/// Real-seconds backstop for the first forecast and guide: the predictor runs
/// its 30 s horizon in passes of 120 ticks, and a software-rendered frame can
/// take seconds.
#[cfg(feature = "debug")]
const PREDICTION_DEADLINE_SECS: f32 = 120.0;
/// Simulated seconds the leg flies under the follow check.
#[cfg(feature = "debug")]
const FLY_SECS: f32 = 15.0;
/// Frames the Map pane renders before its still: its offscreen scene builds
/// over frames.
#[cfg(feature = "debug")]
const MAP_SETTLE_FRAMES: u32 = 20;
/// Real-seconds backstop for a still to land once it is requested.
#[cfg(feature = "debug")]
const SHOT_DEADLINE_SECS: f32 = 60.0;

/// The hauler's least speed when the forecast is checked: most of its 122 m/s
/// orbit speed, so a target that was braked or never moved fails.
#[cfg(feature = "debug")]
const TARGET_SPEED_MIN: Meters = Meters(100.0);
/// The drawn curve's least bend off the chord from its first point to the
/// hauler's live position.
#[cfg(feature = "debug")]
const BEND_MIN: Meters = Meters(150.0);
/// How far the flown centre of mass may stray from a forecast no older than
/// [`FOLLOW_HORIZON_SECS`].
#[cfg(feature = "debug")]
const FOLLOW_TOL: Meters = Meters(50.0);
/// The age past which a forecast is no longer checked against the flight.
#[cfg(feature = "debug")]
const FOLLOW_HORIZON_SECS: f32 = 4.0;
/// Seed-to-seed time between two forecast snapshots.
#[cfg(feature = "debug")]
const SNAPSHOT_INTERVAL_SECS: f32 = 2.0;
/// Forecast snapshots the flown window must take: forecasts keep publishing
/// while the hauler coasts, so the target checks do not hide the path.
#[cfg(feature = "debug")]
const SNAPSHOTS_MIN: u32 = 3;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    // NovaMenuPlugin explicitly: the interface's clock hold lives in nova_menu,
    // and `with_game_plugins` turns the menu plugin off. Without the hold the
    // world keeps flying behind the Map with the flight computer gated off.
    let mut app = AppBuilder::new()
        .with_game_plugins((custom_plugin, NovaMenuPlugin))
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(moving_target_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, ring::drive_leg_camera);
        app.add_systems(FixedPostUpdate, refuse_planetoid_contact);
        // `Time<Fixed>` is what `FlightPrediction::seed_time` is measured in;
        // compare only while `Time<Physics>` moves the ship, since a hold's
        // frame still runs its fixed ticks.
        app.add_systems(
            FixedPostUpdate,
            trace_track.run_if(
                any_with_component::<TrackTrace>
                    .and(|physics: Res<Time<Physics>>| !physics.is_paused()),
            ),
        );
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

/// The scene: [`ring::planetoid`]'s well, the player at [`START`] facing the
/// hauler, and the hauler at [`TARGET_START`] on a circular orbit.
fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    let design = |id: &str| {
        ships
            .get_design(&id.into())
            .unwrap_or_else(|| panic!("screenshot_goto_moving_target: unknown ship '{id}'"))
            .design
            .clone()
    };
    let facing = Quat::from_rotation_arc(
        Vec3::NEG_Z,
        (TARGET_START.get() - START.get())
            .try_normalize()
            .expect("screenshot_goto_moving_target: START and TARGET_START must differ"),
    );
    let player = ring::ship(
        PLAYER_ID,
        "Moving Player",
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
        design("block_line_warship"),
    );
    // Circular orbit speed: v = sqrt(mu / r), in engine units because mu is
    // authored in them, then converted at the scenario edge. Tangent to the
    // radius in the orbit plane y = 0.
    let radius = TARGET_START.to_engine().length();
    let orbit_speed = (ring::PLANETOID_MASS / radius).sqrt();
    let hauler = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: TARGET_ID.to_string(),
            name: "Moving Hauler".to_string(),
            position: TARGET_START,
            // Identity, the attitude its idle flight computer holds, so the
            // hauler starts with no turn.
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Neutral),
            design: ShipDesignSource::Inline(design("block_hauler")),
            initial_velocity: MetersPerSecond3::from_engine(Vec3::X * orbit_speed),
            ..default()
        }),
    });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "A planetoid well and a GOTO at a hauler on its orbit.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![ring::planetoid(), player, hauler],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "screenshot_goto_moving_target".to_string(),
            "GOTO Moving Target".to_string(),
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

/// The follow proof `trace_track` builds while the leg flies: the forecasts
/// still young enough to check, how many were taken, and how many
/// comparisons held (a miss is a panic, not a recorded failure).
#[cfg(feature = "debug")]
#[derive(Component, Default)]
struct TrackTrace {
    snapshots: Vec<PathSnapshot>,
    snapshots_taken: u32,
    follow_checks: u32,
    worst_follow: f32,
}

/// Take a forecast snapshot every [`SNAPSHOT_INTERVAL_SECS`] and check every
/// forecast younger than [`FOLLOW_HORIZON_SECS`] against the ship's live
/// centre of mass, from avian `Position`/`Rotation`/`ComputedCenterOfMass`,
/// interpolated at the forecast's own age. A miss past [`FOLLOW_TOL`] panics.
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

    if let Some(prediction) = prediction {
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
            trace.snapshots_taken += 1;
        }
    }

    // The newest snapshot stays, so the next one is still spaced from it.
    let newest = trace.snapshots.last().map(|snapshot| snapshot.seed_time);
    trace.snapshots.retain(|snapshot| {
        Some(snapshot.seed_time) == newest
            || now.saturating_sub(snapshot.seed_time).as_secs_f32() <= FOLLOW_HORIZON_SECS
    });

    let snapshots = trace.snapshots.clone();
    for snapshot in &snapshots {
        let age = now.saturating_sub(snapshot.seed_time).as_secs_f32();
        if age > FOLLOW_HORIZON_SECS {
            continue;
        }
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
            "screenshot_goto_moving_target: the ship strayed {deviation:.1} units from its own \
             {age:.1} s forecast, past the {:.1} unit tolerance",
            FOLLOW_TOL.to_engine()
        );
        trace.follow_checks += 1;
        trace.worst_follow = trace.worst_follow.max(deviation);
    }
}

/// Panic on the first contact between the player and the planetoid. The
/// forecast does not model contacts, so a leg that touches the planetoid makes
/// the fixture invalid rather than the forecast wrong.
#[cfg(feature = "debug")]
fn refuse_planetoid_contact(
    collisions: Collisions,
    q_player: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_id: Query<&EntityId>,
) {
    let Ok(player) = q_player.single() else {
        return;
    };
    for pair in collisions.iter() {
        let bodies = [pair.body1, pair.body2];
        let planetoid = bodies
            .into_iter()
            .flatten()
            .any(|body| q_id.get(body).is_ok_and(|id| id.0 == ring::PLANETOID_ID));
        assert!(
            !(planetoid && bodies.contains(&Some(player))),
            "screenshot_goto_moving_target: the leg touched the planetoid; the fixture is invalid"
        );
    }
}

/// Advance once the player's [`TrackTrace`] holds a first forecast snapshot.
#[cfg(feature = "debug")]
fn snapshot_ready() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        ring::player_root_ref(world)
            .and_then(|player| world.get::<TrackTrace>(player))
            .is_some_and(|trace| trace.snapshots_taken > 0)
    })
}

/// The interface is up on `pane`.
#[cfg(feature = "debug")]
fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world
                .get_resource::<InterfacePaneType>()
                .is_some_and(|shown| *shown == pane)
    })
}

/// Perpendicular distance of `point` from the chord `a -> b`, falling back to
/// the distance to `a` for a degenerate chord.
#[cfg(feature = "debug")]
fn perpendicular_distance(point: Vec3, a: Vec3, b: Vec3) -> f32 {
    let chord = b - a;
    let length = chord.length();
    if length <= f32::EPSILON {
        return (point - a).length();
    }
    (point - a).reject_from_normalized(chord / length).length()
}

/// The hauler coasts: the planetoid's well pulls it, it keeps at least
/// [`TARGET_SPEED_MIN`], and nothing flies it.
#[cfg(feature = "debug")]
fn assert_target_coasts_in_well(world: &mut World) {
    let target = ring::entity_by_id(world, TARGET_ID)
        .expect("screenshot_goto_moving_target: the hauler is present");
    let planetoid = ring::entity_by_id(world, ring::PLANETOID_ID)
        .expect("screenshot_goto_moving_target: the planetoid is present");
    let well = world.get::<DominantWell>(target).map(|well| **well);
    let speed = world
        .get::<LinearVelocity>(target)
        .map_or(0.0, |velocity| velocity.0.length());
    info!(
        "screenshot_goto_moving_target: hauler at {:.1} m/s, pulled by the planetoid: {}",
        MetersPerSecond::from_engine(speed).0,
        well == Some(planetoid)
    );
    assert_eq!(
        well,
        Some(planetoid),
        "screenshot_goto_moving_target: the planetoid must pull the hauler"
    );
    assert!(
        speed >= TARGET_SPEED_MIN.to_engine(),
        "screenshot_goto_moving_target: the hauler only moves {speed:.2} units/s"
    );
    assert!(
        world.get::<Autopilot>(target).is_none(),
        "screenshot_goto_moving_target: nothing may fly the hauler"
    );
}

/// The ribbon on the player ship: at least one curve segment, exactly one
/// guide segment, and nothing hidden.
#[cfg(feature = "debug")]
fn assert_ribbon(world: &mut World) {
    let player =
        ring::player_root(world).expect("screenshot_goto_moving_target: the player is present");
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
        "screenshot_goto_moving_target: ribbon has {curve} curve, {guide} guide, {hidden} hidden \
         segment(s)"
    );
    assert!(
        curve >= 1,
        "screenshot_goto_moving_target: the ribbon drew no curve segment"
    );
    assert_eq!(
        guide, 1,
        "screenshot_goto_moving_target: the ribbon must draw exactly one guide segment"
    );
    assert_eq!(
        hidden, 0,
        "screenshot_goto_moving_target: a ribbon segment is hidden"
    );
}

/// The drawn curve's max perpendicular distance off the chord from its own
/// first point to the hauler's live position is at least [`BEND_MIN`].
#[cfg(feature = "debug")]
fn assert_bend(world: &mut World) {
    let player =
        ring::player_root(world).expect("screenshot_goto_moving_target: the player is present");
    let target = ring::entity_by_id(world, TARGET_ID)
        .expect("screenshot_goto_moving_target: the hauler is present");
    let target_position = world
        .get::<GlobalTransform>(target)
        .expect("screenshot_goto_moving_target: the hauler has a transform")
        .translation();
    let prediction = world
        .get::<FlightPrediction>(player)
        .expect("screenshot_goto_moving_target: the player has a FlightPrediction");
    let chord_start = prediction.points[0];
    let bend = prediction
        .points
        .iter()
        .map(|&point| perpendicular_distance(point, chord_start, target_position))
        .fold(0.0_f32, f32::max);
    info!(
        "screenshot_goto_moving_target: drawn curve bends {bend:.1} units off the chord to the \
         hauler, ends {:?}",
        prediction.end
    );
    assert!(
        bend >= BEND_MIN.to_engine(),
        "screenshot_goto_moving_target: the drawn curve only bent {bend:.1} units, short of the \
         {:.1} unit floor",
        BEND_MIN.to_engine()
    );
}

/// The follow proof: the ship was compared against its young forecasts, every
/// comparison held (so `trace_track` never panicked), and the window took at
/// least [`SNAPSHOTS_MIN`] forecasts.
#[cfg(feature = "debug")]
fn assert_followed(world: &mut World) {
    let player =
        ring::player_root(world).expect("screenshot_goto_moving_target: the player is present");
    let trace = world
        .get::<TrackTrace>(player)
        .expect("screenshot_goto_moving_target: the player carries a TrackTrace");
    info!(
        "screenshot_goto_moving_target: {} forecast snapshot(s), {} follow check(s), worst \
         deviation {:.2} units",
        trace.snapshots_taken, trace.follow_checks, trace.worst_follow
    );
    assert!(
        trace.follow_checks > 0,
        "screenshot_goto_moving_target: the ship was never compared against a young forecast"
    );
    assert!(
        trace.snapshots_taken >= SNAPSHOTS_MIN,
        "screenshot_goto_moving_target: only {} forecast(s) published over the flown window",
        trace.snapshots_taken
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

/// The walk: load, lock the hauler, fly the real `G` key, check the target
/// and the drawn curve, chase the ship under the follow check, shoot the HUD,
/// then open the Map and shoot it.
#[cfg(feature = "debug")]
fn moving_target_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
        .step("lock the hauler")
        .on_enter(|world: &mut World| {
            let player = ring::player_root(world)
                .expect("screenshot_goto_moving_target: the player is present");
            let target = ring::entity_by_id(world, TARGET_ID)
                .expect("screenshot_goto_moving_target: the hauler is present");
            world.entity_mut(player).insert(TravelLock(Some(target)));
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
    let script = script
        .step("wait for the predicted curve and guide")
        .until(leg_under_way)
        .deadline(PREDICTION_DEADLINE_SECS)
        .add()
        .step("check the hauler and the drawn curve")
        .on_enter(assert_target_coasts_in_well)
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
            let player = ring::player_root(world)
                .expect("screenshot_goto_moving_target: the player is present");
            world.entity_mut(player).insert(TrackTrace::default());
        })
        .until(frames(1))
        .add()
        .step("wait for the first forecast snapshot")
        .until(snapshot_ready())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fly the curve")
        .until(elapsed(FLY_SECS))
        .add()
        .step("check the followed forecasts and the hauler")
        .on_enter(assert_followed)
        .on_enter(assert_target_coasts_in_well)
        .on_enter(assert_ribbon)
        .until(frames(1))
        .add()
        .step("shoot the HUD")
        .on_enter(|world: &mut World| shoot(world, HUD_SHOT))
        .until(shot_written(HUD_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add();
    tap(
        script,
        "the interface key",
        "interface_toggle",
        the_interface_shows(InterfacePaneType::Map),
    )
    .step("settle the map")
    .until(frames(MAP_SETTLE_FRAMES))
    .add()
    .step("shoot the map")
    .on_enter(|world: &mut World| shoot(world, MAP_SHOT))
    .until(shot_written(MAP_SHOT))
    .deadline(SHOT_DEADLINE_SECS)
    .add()
}
