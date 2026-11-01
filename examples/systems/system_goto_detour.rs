//! system_goto_detour: a GOTO leg planned straight into a rock is routed
//! around it, never touches it, predicts the bend ahead of flying it, and
//! drops its held corner the moment the leg is cancelled.
//!
//! One lane: the fixture skiff at the origin, a nav beacon 3000 m out along
//! -Z and, dead on that line at 1500 m, a single fixed-seed asteroid. The
//! leg's own [`DetourPolicy`] clearance (hull radius +
//! [`FlightSettings::avoid_margin`]) keeps the straight GOTO off the rock's
//! live [`BodyRadius`] - the mesh's derived extent, never the authored nominal
//! radius - and the same planner seeds the player's [`FlightPrediction`], so
//! the shown path bends before the hull itself turns.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a goto leg blocked by a rock is flown around it` | the GOTO completed; [`Autopilot::detour`] held a corner on at least one fixed step; the hull's daylight to the rock never dipped under the resolved avoid margin |
//! | 2 | `outcome: the hull never touches the rock` | zero fixed steps found the hull's own colliders and the rock's touching in avian's `ContactGraph`, over the whole run |
//! | 3 | `outcome: the predicted path bends around the rock` | a `FlightPrediction` was published while the direct leg was blocked; its points kept the same daylight margin and, at their widest, left the origin-beacon chord by more than the hull's own radius |
//! | 4 | `outcome: cancelling the goto drops the corner` | removing [`Autopilot`] mid-detour leaves the hull with no Autopilot and no `ManeuverTelemetry` the next fixed step; a fresh engage back through the rock starts with no held corner and picks one back up within one fixed step |
//! | 5 | `outcome: the detour flight is recorded` | RECORD: hull radius, the rock's live body radius, the resolved avoid margin, the measured minimum daylight, how many distinct corners were held, fixed steps flown, and the largest `FlightPrediction` point count seen |
//!
//! Claim 2 is read off avian's `ContactGraph` rather than
//! `nova_gameplay::integrity::core`'s `trace!`-level "contact impact" line:
//! the same touching-pairs facts `deal_contact_impact_damage` reads before it
//! decides whether a touch is hard enough to cost anything, in the shape
//! `system_collision_damage`'s own census already reads for exactly this
//! comparison.
//!
//! What this range does NOT claim: standalone detour geometry (sphere math,
//! hysteresis, corner hops) already belongs to `navigation.rs`'s own unit
//! tests, and the straight-leg-into-a-body regression belongs to
//! `crates/nova_ship/src/flight/tests/detour.rs`. This range is the production
//! seam alone: one engaged GOTO, one rock, the live telemetry the game itself
//! publishes.
//!
//! The walk also opens the paused Map mid-detour on the first leg, holds it
//! until the frozen hull's route is predicted, and shoots [`MAP_SHOT`]. Armed
//! by `NOVA_CAPTURE` like every other shot; unarmed, the same beats open and
//! close the Map and write nothing, so the probe run walks the identical
//! script. The shot is the one artifact that shows the Map's route bending:
//! the markers above cannot prove a rendered frame.
//!
//! The flown leg is recorded as [`LOOP_NAME`] from a fixed camera abeam the
//! lane. On this lane the planner's corner lies on +Y of the rock, so a camera
//! on +X sees the bend in the frame, where one overhead would see the hull
//! pass across the rock. Armed by `NOVA_CAPTURE` like the shot; unarmed,
//! `loop_start` and `loop_end` are no-ops. An armed run is frame-clocked (see
//! `nova_autopilot::loops`), so every deadline counts rendered frames at 30
//! fps.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_goto_detour --features debug
//! # look for: `goto detour: ...`, `autopilot: cycle complete, no panic`
//! ```
//!
//! Map route frame and detour loop (isolated display, software or real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example system_goto_detour --features debug
//! # writes target/shots/goto-detour-map.png and target/shots/goto-detour.webm
//! ```

#[path = "../shared/dev_fixtures/mod.rs"]
mod dev_fixtures;

#[cfg(feature = "debug")]
use std::sync::Arc;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::nova_interface::pane::InterfacePaneType;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_goto_detour")]
#[command(version = "1.0.0")]
#[command(
    about = "A GOTO leg routed around a sized rock, its predicted path, and the corner it drops on cancel. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// The scenario id the lane's hull is spawned under.
const HULL_ID: &str = "goto_detour_hull";

/// The GOTO leg's target: a beacon 3000 m out along -Z.
const BEACON_ID: &str = "goto_detour_beacon";

/// The rock dead on the line between the hull and the beacon.
const ASTEROID_ID: &str = "goto_detour_rock";

const BEACON_AT: Meters = Meters(3000.0);
const ASTEROID_AT: Meters = Meters(1500.0);

/// The rock's AUTHORED designation radius; the detour clears the live,
/// mesh-derived [`BodyRadius`] instead, which can run 3.5x-6x past this.
const ASTEROID_NOMINAL_RADIUS: Meters = Meters(60.0);

/// Pinned so the generated silhouette, and the derived [`BodyRadius`] that
/// rides on it, is the same every run.
const ASTEROID_SEED: u32 = 20_261_031;

/// In-step seconds the load beat gets.
#[cfg(feature = "debug")]
const LOAD_DEADLINE_SECS: f32 = 30.0;

/// In-step seconds each flown leg gets. See `system_flight_legs`' own note on
/// why a per-beat bound this size survives a software rasterizer.
#[cfg(feature = "debug")]
const LEG_DEADLINE_SECS: f32 = 90.0;

/// In-step seconds the short cancel/re-engage beats get: each waits on one
/// fixed tick, not a flown leg.
#[cfg(feature = "debug")]
const CANCEL_DEADLINE_SECS: f32 = 20.0;

/// In-step seconds the Map beats get: an interface open and a route predicted
/// from the frozen tick, one prediction step per paused frame.
#[cfg(feature = "debug")]
const MAP_DEADLINE_SECS: f32 = 60.0;

/// The Map route frame, under `NOVA_CAPTURE_DIR`.
#[cfg(feature = "debug")]
const MAP_SHOT: &str = "goto-detour-map.png";

/// The flown leg's loop, under `NOVA_CAPTURE_DIR`.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "goto-detour";

/// The leg flies about 26 s of game time, 790 frames at 30 fps; the default
/// 600-frame cap would abort the run. 1200 is 40 s of runway.
#[cfg(feature = "debug")]
const LOOP_FRAME_CAP: u32 = 1200;

/// Rendered-frame seconds the encode gets once the loop closes.
#[cfg(feature = "debug")]
const LOOP_DEADLINE_SECS: f32 = 60.0;

/// The loop camera: abeam the rock on +X, centred above the lane so the rock
/// and the +Y bend both sit in frame. At 2800 m the 45 degree lens spans about
/// 4100 m of the lane and 2300 m of the bend.
#[cfg(feature = "debug")]
const LOOP_CAMERA_AT: Meters3 = Meters3::new(2800.0, 450.0, -1500.0);
#[cfg(feature = "debug")]
const LOOP_LOOK_AT: Meters3 = Meters3::new(0.0, 450.0, -1500.0);

/// How far the game clock may advance in one frame of this harnessed lane.
/// See `system_flight_legs`' own note on the software-rasterizer clamp.
#[cfg(feature = "debug")]
const WALK_MAX_DELTA: std::time::Duration = std::time::Duration::from_secs(2);

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(LoopCapturePlugin::new(LoopProfile {
            frame_cap: LOOP_FRAME_CAP,
            ..default()
        }));
        app.add_plugins(detour_script());
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    #[cfg(feature = "debug")]
    {
        app.init_resource::<Detour>();
        app.add_observer(note_the_hull);
        app.add_observer(note_a_completion);
        // ONCE A FIXED STEP, after the solver, same as `system_flight_legs`:
        // both censuses read decisions `autopilot_system` takes in
        // `FixedUpdate`, and a rendered frame on a software rasterizer holds
        // a dozen of those.
        app.add_systems(
            FixedPostUpdate,
            (census_contacts, census_leg)
                .after(avian3d::prelude::PhysicsSystems::Last)
                .run_if(resource_exists::<Detour>),
        );
        if harness_env_active() {
            app.add_systems(First, hold_the_lane_clock.before(bevy::time::TimeSystems));
        }
        app.add_systems(Startup, force_capture_resolution);
    }
}

/// Held every frame: a scenario load hands `Time<Virtual>` back at its
/// default.
#[cfg(feature = "debug")]
fn hold_the_lane_clock(mut time: ResMut<Time<Virtual>>) {
    if time.max_delta() != WALK_MAX_DELTA {
        time.set_max_delta(WALK_MAX_DELTA);
    }
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(lane(&game_assets)));
}

/// The lane: the beacon out along -Z, the rock dead on the line at half the
/// distance, and the player hull at the origin.
fn lane(game_assets: &GameAssets) -> ScenarioConfig {
    let beacon = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BEACON_ID.to_string(),
            name: "Detour Beacon".to_string(),
            position: Meters3::new(0.0, 0.0, -BEACON_AT.get()),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "DETOUR".to_string(),
            radius: Meters(60.0),
            color: Color::WHITE,
            area_radius: None,
            lock_signature: None,
        }),
    });

    let asteroid = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ASTEROID_ID.to_string(),
            name: "Detour Rock".to_string(),
            position: Meters3::new(0.0, 0.0, -ASTEROID_AT.get()),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: ASTEROID_NOMINAL_RADIUS,
            initial_velocity: MetersPerSecond3::ZERO,
            texture: game_assets.asteroid_texture.clone().into(),
            kind: KIND_ROCK.into(),
            destroy_sound: None,
            lock_signature: None,
            seed: Some(ASTEROID_SEED),
        }),
    });

    // The Map plots a bare nav beacon as nothing, so it draws no route to
    // one: the objective mark makes the beacon a contact the paused Map can
    // draw the GOTO route to. The mark changes no geometry on the leg.
    let objective = EventActionConfig::ObjectiveMarkerAttach(
        ObjectiveMarkerAttachActionConfig::new(BEACON_ID, "DETOUR"),
    );

    let hull = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: HULL_ID.to_string(),
            name: "Detour Hull".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            design: ShipDesignSource::Inline(dev_fixtures::skiff()),
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            allegiance: Some(Allegiance::Player),
            ..default()
        }),
    });

    ScenarioConfig {
        description: "A GOTO leg dead on a rock, flown around it, cancelled and re-engaged."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![beacon, asteroid, objective, hull],
                ThreePointRig::around(
                    "goto detour",
                    Meters3::new(0.0, 0.0, -ASTEROID_AT.get()),
                    60.0,
                )
                .actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "goto_detour".to_string(),
            "Goto Detour Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The scripted run: load the lane, fly the blocked GOTO, read the detour and
/// its prediction, fly a blocked leg back through the rock, cancel it, and
/// prove a fresh engage picks the corner back up.
#[cfg(feature = "debug")]
fn detour_script() -> Script {
    Script::new()
        .step("load the lane")
        .enter(GameStates::Loading)
        .until(lane_staged())
        .deadline(LOAD_DEADLINE_SECS)
        .add()
        .step("engage the goto")
        .on_enter(engage_the_goto)
        .until(detour_is_some())
        .deadline(LEG_DEADLINE_SECS)
        .add()
        // The Map, opened mid-detour: the interface freezes the clock and the
        // route is predicted from the frozen hull, so the frame shows the
        // bend the hull is about to fly.
        .step("press the interface key")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("let the key up and wait for the map")
        .on_enter(release_action("interface_toggle"))
        .until(the_interface_shows(InterfacePaneType::Map))
        .deadline(MAP_DEADLINE_SECS)
        .add()
        .step("wait for the paused route")
        .until(route_predicted())
        .deadline(MAP_DEADLINE_SECS)
        .add()
        .step("settle the map")
        .on_enter(hide_dev_overlays)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the map")
        .on_enter(|world| shoot(world, MAP_SHOT))
        .until(shot_written(MAP_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("press the interface key again")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("let the key up")
        .on_enter(release_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("frame the lane and fly the goto")
        .on_enter(|world| {
            hide_status_bar(world);
            pose_camera(world, LOOP_CAMERA_AT, LOOP_LOOK_AT);
            loop_start(world, LOOP_NAME);
        })
        .until(goto_completed())
        .deadline(LEG_DEADLINE_SECS)
        .add()
        .step("close the detour loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(LOOP_DEADLINE_SECS)
        .add()
        .step("read the detour")
        .on_enter(read_the_detour)
        .add()
        .step("engage the return leg")
        .on_enter(engage_the_return)
        .until(detour_is_some())
        .deadline(LEG_DEADLINE_SECS)
        .add()
        .step("drop the helm")
        .on_enter(cancel_the_helm)
        .until(frames(1))
        .deadline(CANCEL_DEADLINE_SECS)
        .add()
        .step("read the drop")
        .on_enter(read_the_drop)
        .until(frames(1))
        .deadline(CANCEL_DEADLINE_SECS)
        .add()
        .step("read the fresh engage")
        .on_enter(read_the_fresh_engage)
        .add()
        .step("read the contacts")
        .on_enter(read_the_contacts)
        .add()
}

/// Everything the run accumulates.
#[cfg(feature = "debug")]
#[derive(Resource, Debug)]
struct Detour {
    /// The lane's hull root, the rock and the beacon, once staged.
    hull: Option<Entity>,
    asteroid: Option<Entity>,
    beacon: Option<Entity>,
    /// The straight chord the GOTO leg's prediction is read against: the
    /// hull's own start and the beacon's world position, world units.
    chord_start: Vec3,
    chord_end: Vec3,
    /// Whether the GOTO leg's own census window is open. The return leg
    /// (claim 4) does not need it: it reads `Autopilot::detour` directly.
    active: bool,
    /// The production completion seam reported the GOTO complete.
    completed: bool,
    /// Fixed steps the GOTO census ran over.
    steps: usize,
    /// At least one fixed step held a detour corner.
    detour_seen: bool,
    last_corner: Option<Vec3>,
    /// Distinct corners held, in the order they were taken.
    corners: Vec<Vec3>,
    /// The smallest hull-to-rock daylight seen over the GOTO census, world
    /// units.
    min_daylight: f32,
    /// Fixed steps that held a detour corner AND a published
    /// `FlightPrediction`.
    prediction_while_blocked_steps: usize,
    prediction_min_daylight: f32,
    prediction_max_chord_dev: f32,
    prediction_max_points: usize,
    /// Fixed steps, over the WHOLE run, that found the hull and the rock
    /// touching in avian's `ContactGraph`.
    contact_touches: usize,
}

#[cfg(feature = "debug")]
impl Default for Detour {
    fn default() -> Self {
        Self {
            hull: None,
            asteroid: None,
            beacon: None,
            chord_start: Vec3::ZERO,
            chord_end: Vec3::ZERO,
            active: false,
            completed: false,
            steps: 0,
            detour_seen: false,
            last_corner: None,
            corners: Vec::new(),
            min_daylight: f32::INFINITY,
            prediction_while_blocked_steps: 0,
            prediction_min_daylight: f32::INFINITY,
            prediction_max_chord_dev: 0.0,
            prediction_max_points: 0,
            contact_touches: 0,
        }
    }
}

/// The lane's hull root, tracked as the scenario spawns it. An observer, not
/// a poll: see `system_flight_legs`' own note on why.
#[cfg(feature = "debug")]
fn note_the_hull(add: On<Add, PlayerSpaceshipMarker>, mut detour: ResMut<Detour>) {
    detour.hull = Some(add.entity);
}

/// Record the production completion seam reporting the GOTO leg complete.
#[cfg(feature = "debug")]
fn note_a_completion(
    add: On<Add, PlayerAutopilotCompleted>,
    q_completed: Query<&PlayerAutopilotCompleted>,
    mut detour: ResMut<Detour>,
) {
    let Ok(completed) = q_completed.get(add.entity) else {
        return;
    };
    if matches!(completed.action, AutopilotAction::Goto { .. }) {
        info!("goto detour: the GOTO leg reported complete");
        detour.completed = true;
    }
}

/// The scenario object with this id, off a borrowed world.
#[cfg(feature = "debug")]
fn staged(world: &World, id: &str) -> Option<Entity> {
    let mut objects = world.try_query::<(Entity, &EntityId)>()?;
    objects
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

/// The lane is up: the hull has been weighed, and the beacon and the rock are
/// both live with their physics bodies linked.
#[cfg(feature = "debug")]
fn lane_staged() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(hull) = world
            .get_resource::<Detour>()
            .and_then(|detour| detour.hull)
        else {
            return false;
        };
        world.get::<HullRadius>(hull).is_some()
            && world
                .get::<avian3d::prelude::ComputedCenterOfMass>(hull)
                .is_some()
            && staged(world, BEACON_ID).is_some()
            && staged(world, ASTEROID_ID)
                .is_some_and(|rock| world.get::<BodyRadius>(rock).is_some())
    })
}

/// The lane's hull, for a beat that cannot run without one.
#[cfg(feature = "debug")]
fn lane_hull(world: &World) -> Entity {
    world
        .resource::<Detour>()
        .hull
        .expect("goto detour: the lane must have staged a hull")
}

/// Engage the GOTO at the beacon and open the detour census.
#[cfg(feature = "debug")]
fn engage_the_goto(world: &mut World) {
    let hull = lane_hull(world);
    let beacon = staged(world, BEACON_ID).expect("goto detour: the beacon must be staged");
    let asteroid = staged(world, ASTEROID_ID).expect("goto detour: the rock must be staged");
    let beacon_at = world
        .get::<avian3d::prelude::Position>(beacon)
        .map_or(Vec3::ZERO, |position| position.0);
    {
        let mut detour = world.resource_mut::<Detour>();
        detour.beacon = Some(beacon);
        detour.asteroid = Some(asteroid);
        detour.chord_start = Vec3::ZERO;
        detour.chord_end = beacon_at;
        detour.active = true;
    }
    world
        .entity_mut(hull)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: beacon }));
}

/// The production completion seam reported the GOTO complete.
#[cfg(feature = "debug")]
fn goto_completed() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<Detour>(|detour| detour.completed)
}

/// The hull is holding a detour corner right now.
#[cfg(feature = "debug")]
fn detour_is_some() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(hull) = world
            .get_resource::<Detour>()
            .and_then(|detour| detour.hull)
        else {
            return false;
        };
        world
            .get::<Autopilot>(hull)
            .is_some_and(|autopilot| autopilot.detour.is_some())
    })
}

/// The interface is up on `pane`.
#[cfg(feature = "debug")]
fn the_interface_shows(
    pane: InterfacePaneType,
) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world
                .get_resource::<InterfacePaneType>()
                .is_some_and(|shown| *shown == pane)
    })
}

/// The frozen hull has a published route with at least one stroke in it.
#[cfg(feature = "debug")]
fn route_predicted() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(hull) = world
            .get_resource::<Detour>()
            .and_then(|detour| detour.hull)
        else {
            return false;
        };
        world
            .get::<FlightPrediction>(hull)
            .is_some_and(|prediction| prediction.points.len() >= 2)
    })
}

/// The perpendicular distance from `point` to the infinite line through
/// `start` and `end`.
#[cfg(feature = "debug")]
fn chord_deviation(point: Vec3, start: Vec3, end: Vec3) -> f32 {
    let Ok(direction) = Dir3::new(end - start) else {
        return point.distance(start);
    };
    let along = (point - start).dot(*direction);
    let closest = start + *direction * along;
    point.distance(closest)
}

/// Sample the GOTO leg once a fixed step, while its census window is open:
/// the hull-to-rock daylight, whether a corner is held, and - while one is -
/// whatever `FlightPrediction` is published alongside it.
#[cfg(feature = "debug")]
fn census_leg(
    mut detour: ResMut<Detour>,
    q_hull: Query<(
        &avian3d::prelude::Position,
        &avian3d::prelude::Rotation,
        Option<&avian3d::prelude::ComputedCenterOfMass>,
        &HullRadius,
        Option<&Autopilot>,
        Option<&FlightPrediction>,
    )>,
    q_asteroid: Query<(&avian3d::prelude::Position, &BodyRadius)>,
) {
    if !detour.active {
        return;
    }
    let Some(hull) = detour.hull else { return };
    let Some(asteroid) = detour.asteroid else {
        return;
    };
    let Ok((position, rotation, com, hull_radius, autopilot, prediction)) = q_hull.get(hull) else {
        return;
    };
    let Ok((asteroid_position, body_radius)) = q_asteroid.get(asteroid) else {
        return;
    };

    let com_world = position.0 + rotation.0.mul_vec3(com.map_or(Vec3::ZERO, |com| com.0));
    let hull_radius = **hull_radius;
    let body_radius = **body_radius;
    let asteroid_at = asteroid_position.0;

    detour.steps += 1;
    let daylight = com_world.distance(asteroid_at) - body_radius - hull_radius;
    detour.min_daylight = detour.min_daylight.min(daylight);

    let corner = autopilot.and_then(|autopilot| autopilot.detour);
    if corner.is_some() {
        detour.detour_seen = true;
        if detour.last_corner != corner {
            detour.corners.push(corner.expect("checked is_some above"));
        }
    }
    detour.last_corner = corner;

    if corner.is_some() {
        if let Some(prediction) = prediction {
            detour.prediction_while_blocked_steps += 1;
            detour.prediction_max_points =
                detour.prediction_max_points.max(prediction.points.len());
            let (chord_start, chord_end) = (detour.chord_start, detour.chord_end);
            for &point in &prediction.points {
                let point_daylight = point.distance(asteroid_at) - body_radius - hull_radius;
                detour.prediction_min_daylight = detour.prediction_min_daylight.min(point_daylight);
                let deviation = chord_deviation(point, chord_start, chord_end);
                detour.prediction_max_chord_dev = detour.prediction_max_chord_dev.max(deviation);
            }
        }
    }
}

/// Count, once a fixed step over the WHOLE run, whether the hull and the rock
/// are touching in avian's `ContactGraph` - the same touching-pairs facts
/// `deal_contact_impact_damage` reads, filtered to this one pair the way
/// `system_collision_damage`'s own census reads a pair.
#[cfg(feature = "debug")]
fn census_contacts(mut detour: ResMut<Detour>, graph: Res<avian3d::prelude::ContactGraph>) {
    let (Some(hull), Some(asteroid)) = (detour.hull, detour.asteroid) else {
        return;
    };
    let touching = graph
        .iter_active_touching()
        .chain(graph.iter_sleeping_touching())
        .any(|contact| {
            let bodies = (contact.body1, contact.body2);
            bodies == (Some(hull), Some(asteroid)) || bodies == (Some(asteroid), Some(hull))
        });
    if touching {
        detour.contact_touches += 1;
    }
}

/// Close the GOTO leg's census window and read claims 1 and 3: the leg held
/// a corner and kept its margin, and the prediction bent with it.
#[cfg(feature = "debug")]
fn read_the_detour(world: &mut World) {
    let elapsed = world.resource::<Time>().elapsed_secs();
    let margin = world.resource::<FlightSettings>().avoid_margin;
    let (
        hull,
        asteroid,
        steps,
        detour_seen,
        min_daylight,
        prediction_steps,
        prediction_min_daylight,
        prediction_max_chord_dev,
        prediction_max_points,
    ) = {
        let detour = world.resource::<Detour>();
        (
            detour
                .hull
                .expect("goto detour: the lane must have staged a hull"),
            detour
                .asteroid
                .expect("goto detour: the lane must have staged the rock"),
            detour.steps,
            detour.detour_seen,
            detour.min_daylight,
            detour.prediction_while_blocked_steps,
            detour.prediction_min_daylight,
            detour.prediction_max_chord_dev,
            detour.prediction_max_points,
        )
    };
    world.resource_mut::<Detour>().active = false;

    let hull_radius = world.get::<HullRadius>(hull).map_or(0.0, |radius| **radius);
    let body_radius = world
        .get::<BodyRadius>(asteroid)
        .map_or(0.0, |radius| **radius);

    assert!(
        detour_seen,
        "goto detour: the GOTO leg never held a detour corner over {steps} fixed steps; \
         the rock did not block the direct leg"
    );
    assert!(
        min_daylight >= margin,
        "goto detour: the hull's daylight to the rock dipped to {:.2} engine units \
         (margin {:.2}, rock body radius {:.2}, hull radius {:.2})",
        min_daylight,
        margin,
        body_radius,
        hull_radius
    );
    nova_probe::probe_marker(
        world,
        "outcome: a goto leg blocked by a rock is flown around it",
        serde_json::json!({
            "t": elapsed,
            "fixed_steps": steps,
            "detour_seen": detour_seen,
            "min_daylight_m": Meters::from_engine(min_daylight).get(),
            "avoid_margin_m": Meters::from_engine(margin).get(),
        }),
    );

    assert!(
        prediction_steps > 0,
        "goto detour: no FlightPrediction was observed while the direct leg held a detour corner"
    );
    assert!(
        prediction_min_daylight >= margin,
        "goto detour: a predicted point's daylight to the rock dipped to {:.2} engine units \
         (margin {:.2})",
        prediction_min_daylight,
        margin
    );
    assert!(
        prediction_max_chord_dev > hull_radius,
        "goto detour: the predicted path never left the origin-beacon chord by more than the \
         hull radius ({:.2} engine units); the largest deviation seen was {:.2}",
        hull_radius,
        prediction_max_chord_dev
    );
    nova_probe::probe_marker(
        world,
        "outcome: the predicted path bends around the rock",
        serde_json::json!({
            "t": elapsed,
            "prediction_steps_while_blocked": prediction_steps,
            "prediction_min_daylight_m": Meters::from_engine(prediction_min_daylight).get(),
            "prediction_max_chord_deviation_m": Meters::from_engine(prediction_max_chord_dev).get(),
            "prediction_max_points": prediction_max_points,
            "hull_radius_m": Meters::from_engine(hull_radius).get(),
        }),
    );
}

/// Engage a GotoPos back toward the origin, so the same rock blocks the
/// return leg.
#[cfg(feature = "debug")]
fn engage_the_return(world: &mut World) {
    let hull = lane_hull(world);
    world
        .entity_mut(hull)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: Vec3::ZERO,
        }));
}

/// Remove the autopilot mid-detour: a plain production write, no disengage
/// ceremony.
#[cfg(feature = "debug")]
fn cancel_the_helm(world: &mut World) {
    let hull = lane_hull(world);
    world.entity_mut(hull).remove::<Autopilot>();
}

/// The next fixed step after the cancel: no Autopilot, no ManeuverTelemetry.
/// Then engage a fresh action back through the rock and check IT starts with
/// no held corner, before the leg has flown a single tick.
#[cfg(feature = "debug")]
fn read_the_drop(world: &mut World) {
    let hull = lane_hull(world);
    assert!(
        world.get::<Autopilot>(hull).is_none(),
        "goto detour: a cancelled autopilot must leave no Autopilot the next fixed step"
    );
    assert!(
        world.get::<ManeuverTelemetry>(hull).is_none(),
        "goto detour: a cancelled autopilot must leave no ManeuverTelemetry the next fixed step"
    );

    let fresh = Autopilot::engage(AutopilotAction::GotoPos {
        position: Vec3::ZERO,
    });
    assert_eq!(
        fresh.detour, None,
        "goto detour: a freshly engaged autopilot must start with no held corner"
    );
    world.entity_mut(hull).insert(fresh);
}

/// One fixed step after the fresh engage: the leg is blocked by the same rock
/// again, so a corner must be held. Then remove the autopilot and end the
/// run.
#[cfg(feature = "debug")]
fn read_the_fresh_engage(world: &mut World) {
    let elapsed = world.resource::<Time>().elapsed_secs();
    let hull = lane_hull(world);
    let corner = world
        .get::<Autopilot>(hull)
        .and_then(|autopilot| autopilot.detour);
    assert!(
        corner.is_some(),
        "goto detour: a freshly engaged leg flying back through the rock must hold a detour \
         corner within one fixed step"
    );
    world.entity_mut(hull).remove::<Autopilot>();
    nova_probe::probe_marker(
        world,
        "outcome: cancelling the goto drops the corner",
        serde_json::json!({ "t": elapsed, "corner_after_fresh_engage": corner }),
    );
}

/// The last beat: zero contacts over the whole run, and the recorded numbers.
#[cfg(feature = "debug")]
fn read_the_contacts(world: &mut World) {
    let elapsed = world.resource::<Time>().elapsed_secs();
    let (hull, asteroid, touches, steps, corners, min_daylight, prediction_max_points) = {
        let detour = world.resource::<Detour>();
        (
            detour
                .hull
                .expect("goto detour: the lane must have staged a hull"),
            detour
                .asteroid
                .expect("goto detour: the lane must have staged the rock"),
            detour.contact_touches,
            detour.steps,
            detour.corners.len(),
            detour.min_daylight,
            detour.prediction_max_points,
        )
    };
    let hull_radius = world.get::<HullRadius>(hull).map_or(0.0, |radius| **radius);
    let body_radius = world
        .get::<BodyRadius>(asteroid)
        .map_or(0.0, |radius| **radius);
    let margin = world.resource::<FlightSettings>().avoid_margin;

    assert_eq!(
        touches, 0,
        "goto detour: the hull must never touch the rock while flying around it (touched on \
         {touches} fixed steps)"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the hull never touches the rock",
        serde_json::json!({ "t": elapsed, "touching_steps": touches }),
    );

    nova_probe::probe_marker(
        world,
        "outcome: the detour flight is recorded",
        serde_json::json!({
            "t": elapsed,
            "hull_radius_m": Meters::from_engine(hull_radius).get(),
            "rock_body_radius_m": Meters::from_engine(body_radius).get(),
            "avoid_margin_m": Meters::from_engine(margin).get(),
            "min_daylight_m": Meters::from_engine(min_daylight).get(),
            "distinct_corners": corners,
            "fixed_steps": steps,
            "prediction_max_points": prediction_max_points,
        }),
    );
}
