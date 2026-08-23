//! stress_point_defense: one hull's battery working a stream of inbound
//! ordnance, at a scale no fight will ever reach.
//!
//! The TARGETING and POINT-DEFENSE chain under load, which nothing else in the
//! suite exercises: the radar scanner walks every lockable body every frame, the
//! per-turret assignment sorts a threat list per hull and hands each mount its
//! own torpedo, and the mounts the Flight Computer holds aim and fire on their
//! own. Every round it spends is a physics body, so the range is also the
//! instrument for the projectile broad phase - `stress_bullets` fills the sky
//! from a held trigger, this one fills it the way a fight does.
//!
//! Three knobs, and the range reports each separately so a reader can tell which
//! one a cost belongs to:
//!
//! - [`DEFENSE_MOUNTS`] - mounts on the defender. Drives the assignment's turret
//!   loop and, through them, the rounds in the sky.
//! - [`TORPEDO_BAYS`] - the launch rate, and through it the live torpedo
//!   population. Drives the radar scanner and the threat list.
//! - the rounds and the colliders they add are DERIVED, recorded per cycle, and
//!   the thing the broad phase actually carries.
//!
//! Both are overridable per run ([`scale_param`]) so the case can be swept
//! across scales in one build without moving what it asserts. Two further
//! knobs measure rather than scale: `NOVA_STRESS_PD_VIEW` moves the camera
//! ([`View`]), because what is inside the frustum is drawn and what is outside
//! it is not; `NOVA_STRESS_PD_FRAME_MS` ([`frame_floor_ms`]) holds each frame
//! open, which turns any box into a slow one and is how the counts below are
//! shown not to follow the frame rate.
//!
//! What it claims, beside the frame time:
//!
//! - both hulls stand up WHOLE - exactly the mounts and bays they were built
//!   with, so a run that quietly lost half its battery cannot report an easy
//!   number;
//! - the computer TOOK the battery - every mount resolves to
//!   [`MountAuthority::FlightComputer`], which is the precondition for any of
//!   this measuring point defense at all;
//! - the envelope FILLED - inbound torpedoes inside [`PD_ENVELOPE`] reach the
//!   range's scale, so the threat list is a load and not a formality;
//! - the mounts were WORKING - a peak count of mounts holding an assignment;
//! - the battery CONNECTED - torpedoes were shot down, so the chain ran end to
//!   end rather than aiming at things it never reached;
//! - the sky FILLED with rounds, which is the broad-phase load the case exists
//!   to grade;
//! - the sky CLEARS once the tubes close, and the teardown leaves NOTHING.
//!
//! Nothing here asserts a millisecond. The frame time is recorded by the
//! capture; what the range pins are counts - mounts, threats, rounds, colliders.
//!
//! **The torpedoes are aimed PAST the hull, not at it.** A leaker would sever a
//! section, and the scene would then measure a different ship for the rest of
//! the window - which is exactly the run-to-run drift this case exists to
//! remove. Every lane crosses the defender's transverse plane on a ring
//! ([`GATE_NEAR`]..[`GATE_FAR`]) well inside the point-defense envelope and well
//! clear of the hull, so the battery gets a real crossing target and the hull is
//! never touched. The lane is a pure function of the tube it left, so there is
//! no RNG in the layout at all.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example stress_point_defense --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `stress_point_defense: N mounts up against M bays`,
//! #           `stress_point_defense: peak N rounds in the sky`,
//! #           `stress_point_defense: teardown returned to baseline`,
//! #           `autopilot: cycle complete, no panic`
//!
//! # with the frame-time capture armed:
//! cargo run --features debug probe run stress_point_defense
//!
//! # swept, one scale per run:
//! NOVA_STRESS_PD_MOUNTS=24 cargo run --features debug probe run stress_point_defense
//! ```

// The BVH census the broad-phase claim rides on, and nothing else needs it: the
// census only exists under the harness.
#[cfg(feature = "debug")]
use avian3d::prelude::Collider;
use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "stress_point_defense")]
#[command(version = "1.0.0")]
#[command(about = "Stress range: one battery working a stream of inbound torpedoes. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// ABSURD SCALE, and deliberately so: this number must never reflect real
/// content. The shipped corvette carries a handful of mounts; a hull that is
/// nothing but battery is a load the code has to survive, not a ship anyone will
/// fly. Read it as a ceiling the assignment is proven under, never as a balance
/// figure.
const DEFENSE_MOUNTS: usize = 12;

/// ABSURD SCALE (see [`DEFENSE_MOUNTS`]): one tube per second each, forever, out
/// of a wall of them. A rack that decides a fight is six torpedoes total.
///
/// This is the knob that sets the live torpedo population: a bay launches once a
/// second and a lane takes about ten seconds to fly, so the sky settles around
/// ten torpedoes per bay and the range reaches its scale within one lane's
/// flight time instead of spending the completion budget climbing to it.
const TORPEDO_BAYS: usize = 12;

/// Environment override for a scale knob, so one build can be swept.
///
/// A MEASUREMENT knob, not content: the defaults above are what the range
/// asserts against, and an override moves every floor with it. Named
/// `NOVA_STRESS_PD_<KNOB>` beside the harness's own `NOVA_PROBE_*` parameters.
fn scale_param(knob: &str, default: usize) -> usize {
    std::env::var(format!("NOVA_STRESS_PD_{knob}"))
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

/// Mounts on the defender this run.
fn mounts() -> usize {
    static CACHED: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *CACHED.get_or_init(|| scale_param("MOUNTS", DEFENSE_MOUNTS))
}

/// Tubes on the launcher this run.
fn bays() -> usize {
    static CACHED: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *CACHED.get_or_init(|| scale_param("BAYS", TORPEDO_BAYS))
}

/// Floor on how long one frame takes, in milliseconds
/// (`NOVA_STRESS_PD_FRAME_MS`, 0 = free-running).
///
/// A MEASUREMENT knob, and the INSTRUMENT for frame-rate invariance: a slow
/// host is a long frame, so pacing the frame from inside the range reproduces
/// one on any box. The point-defence chain must land the same COUNTS at 8 ms
/// and at 33 ms a frame; a count that tracks this knob is a system deciding on
/// the render clock and spending on the physics clock.
///
/// It is a floor, not a cap: a frame that already overruns is left alone, so
/// the knob only ever makes the run slower and never hides real cost.
fn frame_floor_ms() -> u64 {
    static CACHED: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *CACHED.get_or_init(|| {
        std::env::var("NOVA_STRESS_PD_FRAME_MS")
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok())
            .unwrap_or(0)
    })
}

/// Hold each frame open to [`frame_floor_ms`], so the run reads as a host of
/// that speed. Registered in `Last`, after everything the frame does.
fn pace_the_frame(mut last: Local<Option<std::time::Instant>>) {
    let floor = std::time::Duration::from_millis(frame_floor_ms());
    let now = std::time::Instant::now();
    if let Some(start) = *last {
        if let Some(remaining) = floor.checked_sub(now.duration_since(start)) {
            std::thread::sleep(remaining);
        }
    }
    *last = Some(std::time::Instant::now());
}

/// Where the scenario camera stands, and what it frames.
///
/// A MEASUREMENT knob (`NOVA_STRESS_PD_VIEW`), like the two scale knobs. The
/// frame is view-dependent - a torpedo off screen is culled and costs nothing to
/// draw - so a capture that does not say where the camera pointed has not said
/// what it measured.
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    /// The loader's own pose, and the default: the battery framed from +Z, with
    /// the launcher and most of each lane BEHIND the camera.
    Battery,
    /// The whole range end on, from behind the battery: every lane, every
    /// torpedo and the launcher wall are all inside the frustum at once.
    Lanes,
    /// Empty sky. Nothing the range spawns is inside the frustum, so what is
    /// left is the cost of simulating the scene without drawing it.
    Away,
}

/// Camera pose for this run.
fn view() -> View {
    static CACHED: std::sync::OnceLock<View> = std::sync::OnceLock::new();
    *CACHED.get_or_init(|| match std::env::var("NOVA_STRESS_PD_VIEW").as_deref() {
        Ok("lanes") => View::Lanes,
        Ok("away") => View::Away,
        _ => View::Battery,
    })
}

/// The camera pose each [`View`] stands at, as (eye, target).
fn view_pose() -> (Vec3, Vec3) {
    match view() {
        View::Battery => (Vec3::new(0.0, 10.0, 20.0), Vec3::ZERO),
        // Far enough back that the launcher at LAUNCHER_STANDOFF and the whole
        // gate ring fit in a 45 deg vertical FOV.
        View::Lanes => (Vec3::new(0.0, 20.0, -160.0), Vec3::new(0.0, 0.0, 110.0)),
        View::Away => (Vec3::new(0.0, 10.0, 20.0), Vec3::new(0.0, 4000.0, 20.0)),
    }
}

/// The turret content id every mount is built from.
const MOUNT_SECTION: &str = PDC_KINETIC_TURRET_SECTION_ID;

/// The bay content id every tube is built from: the shipped Serpent bay, so the
/// weave point defense actually has to solve is the shipped one.
const BAY_SECTION: &str = "torpedo_section";

/// Where a mount's centre sits above the spine cell it stands on: half a cell
/// out to that cell's top face, then a quarter for the PDC's own base plate.
const MOUNT_SEAT: f32 = 0.75;

/// The point-defence envelope the defender is authored with.
///
/// AUTHORED rather than inherited so the range can name the number its own
/// threat census counts against; it matches the engine default, and authoring it
/// is what keeps the two from drifting apart silently.
const PD_ENVELOPE: f32 = 150.0;

/// How far downrange the launcher sits, along +Z from the defender at the
/// origin. Outside [`PD_ENVELOPE`], so the battery never engages the tubes
/// themselves - the range measures point defence against ORDNANCE.
const LAUNCHER_STANDOFF: f32 = 220.0;

/// The gate ring: how near and how far a lane crosses the defender's transverse
/// plane.
///
/// Inside [`PD_ENVELOPE`] at both ends, so every torpedo is a threat for the
/// whole crossing; [`GATE_NEAR`] is far enough out that the warhead's 30 u blast
/// cannot reach the hull even at its closest.
const GATE_NEAR: f32 = 60.0;
/// The outer gate radius (see [`GATE_NEAR`]).
const GATE_FAR: f32 = 110.0;

/// The arc of the gate ring, as a fraction of a half turn measured up from the
/// hull's own plane.
///
/// Every gate sits ABOVE the spine, because every mount stands on it: a lane
/// under the hull is one no mount can depress far enough to reach, and a range
/// whose battery cannot bear measures an idle scene. The ends are pulled off the
/// horizon so the shallowest lane is still ~27 deg up.
const GATE_ARC_START: f32 = 0.15;
/// The end of the gate arc (see [`GATE_ARC_START`]).
const GATE_ARC_END: f32 = 0.85;

/// How far past the gate a torpedo's aim point sits.
///
/// It fuzes half a blast radius short of that point, so this is what bounds the
/// live population: a lane is the standoff plus this, and the flight time across
/// it times the launch rate IS the number of torpedoes in the sky.
const LANE_OVERRUN: f32 = 60.0;

/// The scenario id the range loads under.
const SCENARIO_ID: &str = "stress_point_defense";

/// The first step's name, so `loop_from` restarts the cycle at the spawn
/// without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "spawn the range";

#[cfg(feature = "debug")]
const POINT_DEFENSE_LOOP: &str = "news-0110-point-defense";

/// How many frames the saturated scene is held for.
///
/// A FRAME count, not a duration, because the frame-time window is one too: the
/// measured hold has to outlast the capture's warm-up plus its window, or the
/// window would close over the drain and measure the scene coming apart. An
/// unarmed run holds only long enough to sample the steady state.
#[cfg(feature = "debug")]
fn hold_frames() -> u32 {
    if nova_probe::probe_armed() {
        CAPTURE_HOLD_FRAMES
    } else {
        HOLD_FRAMES
    }
}

/// Hold length for a run with no capture armed: enough frames for the peaks to
/// settle, and no more.
#[cfg(feature = "debug")]
const HOLD_FRAMES: u32 = 120;

/// Hold length for a MEASURED run: the capture's 180 warm-up plus its 900-frame
/// window, plus margin for the frames the ready gate and the reload spend.
#[cfg(feature = "debug")]
const CAPTURE_HOLD_FRAMES: u32 = 1600;

/// Settle window between the unload and the baseline assertion.
#[cfg(feature = "debug")]
const TEARDOWN_SETTLE_SECS: f32 = 0.5;

/// Deadlines, in the clamped sim seconds every gate here counts in. Generous for
/// llvmpipe, where a frame of this scene is hundreds of milliseconds.
#[cfg(feature = "debug")]
const SPAWN_DEADLINE_SECS: f32 = 60.0;
/// The fill gate's budget: a lane needs ~2.5 s to reach the envelope and the
/// population then climbs at the launch rate.
#[cfg(feature = "debug")]
const FILL_DEADLINE_SECS: f32 = 90.0;
/// Once the tubes close nothing new launches, so every torpedo left flies its
/// lane out and fuzes, and the rounds expire on their own 2 s lifetime.
#[cfg(feature = "debug")]
const DRAIN_DEADLINE_SECS: f32 = 60.0;

/// The fill gate and the scale claim: how many inbound torpedoes have to be
/// inside the envelope at once, per bay.
///
/// Under the settled numbers a lane spends ~9 s of its ~10 s flight inside
/// [`PD_ENVELOPE`], so a bay launching once a second contributes ~9 - the floor
/// is set at two thirds of that, because point defence removes some of them and
/// how many is the thing being measured.
///
/// This gate also opens the capture window, so it is what makes the window
/// start on a saturated scene. The cost is that it bounds the sweep: measured
/// yield falls as the battery outguns the launcher, so
/// `NOVA_STRESS_PD_MOUNTS=24` against twelve bays never fills the envelope at
/// all and stalls on the step deadline. Sweeping mounts UP means raising bays
/// with them.
///
/// FOUR, not six. Six was drawn against a battery that only held its trigger a
/// third of the time because its barrel advanced on the render clock; a battery
/// that works kills enough of the stream that twelve bays settle at 5.0-5.6 a
/// bay and the old gate is unreachable at any frame rate. The floor keeps a 25%
/// margin under the measured settle.
#[cfg(feature = "debug")]
const INBOUND_PER_BAY: usize = 4;

/// The scale claim on the ROUNDS: how many have to be in the sky at once, per
/// mount.
///
/// A PDC fires 100 rounds/s with a 2 s round lifetime, so a mount that holds its
/// trigger down saturates at ~200. What the battery actually reaches is the
/// FRACTION of the time its barrels bear, and that fraction no longer depends on
/// the host: the whole gun - assignment, intercept solve, hinge, joint pose and
/// spawner - runs on the fixed clock, so a run buys the same number of decisions
/// per second of SIMULATED time whatever the frame rate.
///
/// The floor is kept LOOSE against the measured yield on purpose. It is a scale
/// claim - "the battery reached a load worth measuring" - not a yield
/// regression, and a floor drawn tight against one machine's number would fail
/// on a scene the owner deliberately rebalanced.
#[cfg(feature = "debug")]
const ROUNDS_PER_MOUNT: usize = 40;

/// The claim that the battery was WORKING: how many mounts must hold an
/// assignment at the peak, as a fraction of the battery.
#[cfg(feature = "debug")]
const WORKING_MOUNTS_NUMERATOR: usize = 1;
/// Denominator of the working-mounts fraction (see
/// [`WORKING_MOUNTS_NUMERATOR`]).
#[cfg(feature = "debug")]
const WORKING_MOUNTS_DENOMINATOR: usize = 2;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins(|app: &mut App| {
            app.init_resource::<TubesOpen>();
            app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
            app.add_systems(
                Update,
                (
                    author_the_envelope,
                    point_the_camera,
                    hold_the_tubes,
                    commit_fresh_torpedoes,
                ),
            );
        })
        .build();

    if frame_floor_ms() > 0 {
        app.add_systems(Last, pace_the_frame);
    }

    #[cfg(feature = "debug")]
    {
        app.init_resource::<Peaks>();
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin);
        app.add_systems(Update, (track_peaks, trace_the_census).chain());
        app.add_systems(FixedUpdate, track_trigger_duty);
        app.add_observer(count_intercepts);
        app.add_observer(count_rounds_fired);
        // The picture is taken at the range's PEAK. `nova_screenshot`
        // appends its beat to whatever it is handed, and the beats after
        // this call drain and tear the range down - a shot behind them
        // photographs an empty world.
        app.add_plugins(
            nova_screenshot(
                nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                    // Both hulls are spawned by an OnStart handler, so waiting for
                    // the full section count is also the gate a looped reload needs:
                    // the old cycle's entities outlive the load replacing them.
                    .step(LOAD_STEP)
                    .enter(GameStates::Loading)
                    .until(the_range_is_up())
                    .deadline(SPAWN_DEADLINE_SECS)
                    .add()
                    // The scene is live again: close the reload interval so a frame
                    // capture excludes it. A no-op on the first cycle.
                    .step("close the reload interval")
                    .on_enter(nova_probe::capture_reload_end)
                    .on_enter(assert_the_range_stood_up)
                    .add()
                    // NO input is held, and that is the point: an unlocked, unraised
                    // player battery is exactly the state the Flight Computer takes,
                    // so the range measures the computer's own chain rather than a
                    // held trigger.
                    .step("open the tubes")
                    .on_enter(open_the_tubes)
                    .until(the_envelope_is_full())
                    .deadline(FILL_DEADLINE_SECS)
                    .add()
                    .step("open the point-defense loop")
                    .on_enter(mark_the_envelope_full)
                    .on_enter(hide_hud)
                    .on_enter(|world| loop_start(world, POINT_DEFENSE_LOOP))
                    .until(frames(1))
                    .add()
                    .step("record the battery dividing the salvo")
                    .until(elapsed(4.0))
                    .add()
                    .step("close the point-defense loop")
                    .on_enter(|world| loop_end(world, POINT_DEFENSE_LOOP))
                    .until(loop_written(POINT_DEFENSE_LOOP))
                    .deadline(60.0)
                    .add()
                    .step("hold the saturation")
                    .until(frames(hold_frames()))
                    .add()
                    .step("assert the range reached its scale")
                    .on_enter(assert_the_computer_took_the_battery)
                    .on_enter(assert_the_envelope_filled)
                    .on_enter(assert_the_mounts_were_working)
                    .on_enter(assert_the_battery_connected)
                    .on_enter(assert_the_sky_filled)
                    .add(),
            )
            .step("let the sky drain")
            .on_enter(close_the_tubes)
            .until(nothing_in_flight())
            .deadline(DRAIN_DEADLINE_SECS)
            .add()
            .step("assert the sky drained")
            .on_enter(assert_the_sky_drained)
            .add()
            .step("tear the range down")
            .on_enter(tear_the_range_down)
            .until(elapsed(TEARDOWN_SETTLE_SECS))
            .add()
            .step("check the world came back to baseline")
            .on_enter(assert_back_to_baseline)
            .add()
            .loop_from(LOAD_STEP)
            .on_loop(respawn_or_finish_capture),
        );
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        // The window opens on the SATURATED scene, not on `Playing`: the fill is
        // a ramp whose length depends on how fast the host ran it, so a fixed
        // warm-up from `Playing` would buy an arbitrary slice of the climb.
        app.add_plugins(nova_probe::NovaProbePlugin::default().ready_frametime(envelope_is_full));
    }

    app.run()
}

#[cfg(feature = "debug")]
fn respawn_or_finish_capture(world: &mut World) {
    if capturing() {
        world.write_message(AppExit::Success);
    } else {
        respawn_the_range(world);
    }
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(range_scenario(&game_assets, &sections)));
}

/// The defender: a spine of hull cells, one mount standing on each, with the
/// flight computer taking the middle cell.
///
/// The row shares faces with its neighbours and with the computer, so the
/// integrity graph gets one connected structure - a cloud of islands would spend
/// the run coming apart instead of shooting. The PDC bolts down by its base
/// plate and by nothing else, which is why each mount needs a cell of its own to
/// stand on rather than chaining side to side.
fn battery(sections: &GameSections) -> SpaceshipConfig {
    let count = mounts();
    let cell = |i: usize| i as f32 - (count as f32) / 2.0;
    let computer = count / 2;
    let mut specs = vec![SectionSpec::new(
        "controller",
        BASIC_CONTROLLER_SECTION_ID,
        Vec3::new(cell(computer), 0.0, 0.0),
    )];
    specs.extend((0..count).filter(|&i| i != computer).map(|i| {
        SectionSpec::new(
            format!("spine_{i}"),
            REINFORCED_HULL_SECTION_ID,
            Vec3::new(cell(i), 0.0, 0.0),
        )
    }));
    specs.extend((0..count).map(|i| {
        SectionSpec::new(
            format!("mount_{i}"),
            MOUNT_SECTION,
            Vec3::new(cell(i), MOUNT_SEAT, 0.0),
        )
    }));

    SpaceshipConfig {
        allegiance: Some(Allegiance::Player),
        ..fixtures::ship(
            sections,
            SpaceshipController::Player(PlayerControllerConfig {
                // NO bindings. The battery is never the player's: with nothing
                // locked and nothing raised every mount falls to the Flight
                // Computer, which is the subject.
                input_mapping: default(),
                speed_cap: None,
                // Saturation harness: the magazine and its reload cadence are
                // not the subject, the rounds in the sky are.
                infinite_ammo: true,
            }),
            &specs,
        )
    }
}

/// Where bay `i` sits on the launcher: a square wall of tubes centred on the
/// hull axis, so every muzzle opens into empty space instead of into the section
/// in front of it.
fn bay_position(i: usize) -> Vec3 {
    let side = (bays() as f32).sqrt().ceil();
    let half = (side - 1.0) / 2.0;
    Vec3::new(
        (i % side as usize) as f32 - half,
        (i / side as usize) as f32 - half,
        0.0,
    )
}

/// The launcher: a wall of tubes with a flight computer bolted to its edge, and
/// NOBODY aboard.
///
/// Controller-less on purpose. An AI would decide when to shoot, how to
/// maneuver and what to shoot at, and every one of those decisions is a source
/// of run-to-run drift in a case whose whole value is that it has none. An
/// unpiloted emplacement fires because the range holds its tubes open, and does
/// nothing else.
///
/// The magazine is stripped from every tube: `infinite_ammo` is a PLAYER-side
/// cheat and this hull has no player, so the bays are authored without one
/// instead.
fn launcher(sections: &GameSections) -> SpaceshipConfig {
    let count = bays();
    let side = (count as f32).sqrt().ceil();
    let half = (side - 1.0) / 2.0;
    let mut specs = vec![SectionSpec::new(
        "controller",
        BASIC_CONTROLLER_SECTION_ID,
        Vec3::new(-half - 1.0, -half, 0.0),
    )];
    specs.extend((0..count).map(|i| SectionSpec::new(bay_slot(i), BAY_SECTION, bay_position(i))));

    let mut config = SpaceshipConfig {
        allegiance: Some(Allegiance::Enemy),
        ..fixtures::ship(sections, SpaceshipController::None, &specs)
    };
    if let ShipSource::Inline(hull) = &mut config.hull {
        for section in &mut hull.sections {
            if let SectionSource::Inline(section) = &mut section.source {
                if let SectionKind::Torpedo(bay) = &mut section.kind {
                    bay.ammo_capacity = None;
                }
            }
        }
    }
    config
}

/// The per-ship slot id of bay `i`.
fn bay_slot(i: usize) -> String {
    format!("bay_{i}")
}

/// The range: the battery at the origin and the launcher downrange of it.
fn range_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let objects = vec![
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "battery".to_string(),
                name: "Battery".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(battery(sections)),
        },
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "launcher".to_string(),
                name: "Launcher".to_string(),
                position: launcher_origin(),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(launcher(sections)),
        },
    ];

    ScenarioConfig {
        description: "A battery working a stream of inbound torpedoes.".to_string(),
        hidden: true,
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        events: fixtures::spawn_on_start(
            [
                objects,
                ThreePointRig::around("lights", Vec3::ZERO, 24.0).objects(),
            ]
            .concat(),
        ),
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Stress: Point Defense".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Where the launcher stands.
fn launcher_origin() -> Vec3 {
    Vec3::new(0.0, 0.0, LAUNCHER_STANDOFF)
}

/// Author the defender's point-defence envelope, once, on the hull the scenario
/// spawned.
fn author_the_envelope(
    mut commands: Commands,
    q_defender: Query<
        Entity,
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            Without<AIPointDefenseRange>,
        ),
    >,
) {
    for defender in &q_defender {
        commands
            .entity(defender)
            .insert(AIPointDefenseRange(PD_ENVELOPE));
    }
}

/// Pin the scenario camera at this run's [`View`].
///
/// Every frame rather than once: the loader spawns the camera with a
/// `WASDCameraController`, so a stray input would otherwise move the subject
/// mid-capture, and holding the pose costs one entity's transform.
fn point_the_camera(mut q_camera: Query<&mut Transform, With<ScenarioCameraMarker>>) {
    let (eye, target) = view_pose();
    for mut transform in &mut q_camera {
        let pinned = Transform::from_translation(eye).looking_at(target, Vec3::Y);
        if *transform != pinned {
            *transform = pinned;
        }
    }
}

/// Whether the tubes are open. A resource rather than a held button: the
/// launcher has no controller, so the range writes the bay input itself.
#[derive(Resource, Default)]
struct TubesOpen(bool);

/// Hold every tube's trigger while the range wants ordnance in the sky.
///
/// The bay's own gates still apply - the fire cooldown decides the cadence -
/// so this only keeps the input down. An unpiloted hull carries no `WeaponsHot`,
/// so the safety never touches it.
fn hold_the_tubes(open: Res<TubesOpen>, mut q_bay: Query<&mut TorpedoSectionInput>) {
    for mut input in &mut q_bay {
        // Change-detection hygiene: only write on a real edge.
        if **input != open.0 {
            **input = open.0;
        }
    }
}

/// Commit every fresh torpedo to a point down its own lane.
///
/// A torpedo's target is decided exactly once, right after launch: the player
/// commits from the crosshair lock and the AI from its own `AITarget`. Neither
/// runs on an unpiloted emplacement, so this does that one write and the
/// guidance, the weave, the arming and the fuze run themselves.
///
/// The lane is a pure function of the tube the torpedo left, so the range
/// replays identically with no RNG in it: the tube's offset across the wall
/// picks the gate's bearing, its offset up the wall picks the gate's radius, and
/// the aim point is the gate carried [`LANE_OVERRUN`] further along the same
/// line. The torpedo therefore crosses the defender's plane ON the gate.
fn commit_fresh_torpedoes(
    mut commands: Commands,
    q_fresh: Query<
        (Entity, &Transform),
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoTargetChosen>,
            Without<TorpedoTargetEntity>,
        ),
    >,
) {
    for (torpedo, transform) in &q_fresh {
        let from = transform.translation;
        let gate = gate_for(from - launcher_origin());
        let lane = (gate - from).normalize_or(Vec3::NEG_Z);
        commands.entity(torpedo).insert((
            TorpedoTargetChosen,
            TorpedoTargetPosition(gate + lane * LANE_OVERRUN),
        ));
    }
}

/// The point on the defender's transverse plane a torpedo launched at `offset`
/// from the launcher's own origin crosses.
fn gate_for(offset: Vec3) -> Vec3 {
    let side = (bays() as f32).sqrt().ceil();
    let span = side.max(1.0);
    let unit = |value: f32| ((value / span) + 0.5).clamp(0.0, 1.0);
    let angle =
        std::f32::consts::PI * (GATE_ARC_START + (GATE_ARC_END - GATE_ARC_START) * unit(offset.x));
    let radius = GATE_NEAR + (GATE_FAR - GATE_NEAR) * unit(offset.y);
    Vec3::new(radius * angle.cos(), radius * angle.sin(), 0.0)
}

/// The high-water marks of one cycle, so every scale claim is about the PEAK
/// rather than about whatever happened to be alive on the one frame an assertion
/// looked, and the intercept tally that says the chain ran end to end.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Peaks {
    /// Torpedoes inside the defender's envelope: the threat list's own size.
    inbound: usize,
    /// Mounts holding an assignment: the assignment's own output.
    working: usize,
    /// Point-defence rounds in the sky: the broad phase's load.
    rounds: usize,
    /// Every collider in the world, which is what the broad phase indexes.
    colliders: usize,
    /// Torpedoes the battery shot down this cycle.
    intercepts: usize,
    /// Rounds the battery SPENT this cycle, counted at spawn.
    ///
    /// The peak above is a high-water mark sampled per FRAME, so it reads
    /// higher on a host that samples more often; this is a spawn tally and
    /// carries no such artefact. Rounds per second of SIMULATED time and hits
    /// per round are both derived from it, and they are what separate "the
    /// battery shot more" from "the battery aimed better".
    fired: usize,
    /// Frames the range has run. Divided by simulated seconds it IS the frame
    /// rate, which is the independent variable every count here is graded
    /// against.
    frames: usize,
    /// Simulated seconds elapsed when the tubes opened, and when the envelope
    /// filled: the fill is the range's own frame-rate canary.
    tubes_open_secs: f32,
    full_secs: f32,
    /// [`Peaks::frames`] when the tubes opened, so every rate below is measured
    /// over the ENGAGED window rather than diluted by the load and the warm-up.
    open_frames: usize,
    /// Fixed steps the range has run, and the mount-steps the battery held its
    /// trigger down for.
    ///
    /// The DISCRIMINATING pair. Rounds are spent per fixed step, so
    /// `fired / steps` is the quantity a frame rate must not move. Splitting it
    /// by the trigger tally says WHICH gate moved it: a duty cycle that tracks
    /// the frame rate is the `Update`-side trigger, and a flat duty cycle with
    /// a frame-rate-dependent `fired / trigger_mount_steps` is the barrel's own
    /// bearing gate inside the fixed-step spawner.
    steps: usize,
    trigger_mount_steps: usize,
    open_steps: usize,
    /// Summed [`muzzle_aim_error`] (degrees) over every engaged mount-step, and
    /// the sample count.
    ///
    /// The ROOT reading. A mount fires only inside [`TURRET_ON_TARGET_RAD`]
    /// (0.92 deg), so the mean tracking residual beside that number says
    /// whether the battery is gated by geometry or by cadence - and whether the
    /// residual moves with the frame rate, which is the defect itself.
    aim_error_sum: f64,
    aim_samples: usize,
}

/// Track the high-water marks. A cheap census per frame, which is what it takes
/// to see a peak a per-step sample would miss between frames.
#[cfg(feature = "debug")]
fn track_peaks(
    q_defender: Query<&Transform, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_torpedoes: Query<&Transform, With<TorpedoProjectileMarker>>,
    q_assignments: Query<&TurretDefenseTarget>,
    q_rounds: Query<(), With<TurretBulletProjectileMarker>>,
    q_colliders: Query<(), With<Collider>>,
    mut peaks: ResMut<Peaks>,
) {
    peaks.frames += 1;
    if let Ok(defender) = q_defender.single() {
        let inbound = q_torpedoes
            .iter()
            .filter(|torpedo| torpedo.translation.distance(defender.translation) <= PD_ENVELOPE)
            .count();
        peaks.inbound = peaks.inbound.max(inbound);
    }
    let working = q_assignments
        .iter()
        .filter(|assignment| assignment.is_some())
        .count();
    peaks.working = peaks.working.max(working);
    peaks.rounds = peaks.rounds.max(q_rounds.iter().count());
    peaks.colliders = peaks.colliders.max(q_colliders.iter().count());
}

/// Trace the census once a simulated second, so a run that never reaches its
/// assertions still says what it was doing. A stalled fill is exactly the
/// frame-rate symptom, and it aborts before any marker is written.
///
/// `debug!`, so a normal run stays quiet: reach it with
/// `RUST_LOG=info,stress_point_defense=debug`.
#[cfg(feature = "debug")]
fn trace_the_census(time: Res<Time<Virtual>>, peaks: Res<Peaks>, mut next: Local<f32>) {
    let now = time.elapsed_secs();
    if now < *next {
        return;
    }
    *next = now + 1.0;
    let aim = peaks.aim_error_sum / peaks.aim_samples.max(1) as f64;
    debug!(
        "stress_point_defense: t={now:.1} inbound_peak={} intercepts={} fired={} frames={} \
         steps={} trigger_mount_steps={} aim_err_deg={aim:.3}",
        peaks.inbound,
        peaks.intercepts,
        peaks.fired,
        peaks.frames,
        peaks.steps,
        peaks.trigger_mount_steps
    );
}

/// Census the battery's trigger on the FIXED clock, which is the clock the
/// rounds are actually spent on.
#[cfg(feature = "debug")]
fn track_trigger_duty(
    q_mounts: Query<
        (
            &TurretSectionInput,
            &TurretDefenseTarget,
            &TurretSectionAimPoint,
            &TurretSectionMuzzleEntity,
        ),
        With<PointDefenseMount>,
    >,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    mut peaks: ResMut<Peaks>,
) {
    peaks.steps += 1;
    for (input, assignment, aim_point, muzzle) in &q_mounts {
        if **input {
            peaks.trigger_mount_steps += 1;
        }
        // Only ENGAGED mounts: an unassigned mount points wherever it was left
        // and its error is not a tracking residual.
        let (Some(_), Some(aim), Ok(muzzle)) = (**assignment, **aim_point, q_muzzle.get(**muzzle))
        else {
            continue;
        };
        peaks.aim_error_sum += f64::from(
            muzzle_aim_error(muzzle.forward().into(), muzzle.translation(), aim).to_degrees(),
        );
        peaks.aim_samples += 1;
    }
}

/// Count every torpedo the battery kills. An observer rather than a census: the
/// marker lives for one schedule pass before the reaper removes the torpedo, so
/// a per-frame count would miss most of them.
#[cfg(feature = "debug")]
fn count_intercepts(_shot_down: On<Add, TorpedoShotDownMarker>, mut peaks: ResMut<Peaks>) {
    peaks.intercepts += 1;
}

/// Tally every round the battery spends, at the frame it is spawned.
#[cfg(feature = "debug")]
fn count_rounds_fired(_fired: On<Add, TurretBulletProjectileMarker>, mut peaks: ResMut<Peaks>) {
    peaks.fired += 1;
}

/// How many point-defence rounds are alive right now.
#[cfg(feature = "debug")]
fn live_rounds(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<TurretBulletProjectileMarker>())
        .count()
}

/// How many torpedoes are alive right now.
#[cfg(feature = "debug")]
fn live_torpedoes(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<TorpedoProjectileMarker>())
        .count()
}

/// Live sections and ship roots - what the teardown claim covers beside the
/// projectiles.
#[cfg(feature = "debug")]
fn live_structure(world: &World) -> (usize, usize) {
    let sections = world
        .iter_entities()
        .filter(|entity| entity.contains::<SectionMarker>())
        .count();
    let roots = world
        .iter_entities()
        .filter(|entity| entity.contains::<SpaceshipRootMarker>())
        .count();
    (sections, roots)
}

/// The sections both hulls carry between them: the defender's spine, its mounts
/// and its computer, plus the launcher's wall and computer.
#[cfg(feature = "debug")]
fn sections_on_the_range() -> usize {
    2 * mounts() + 1 + bays()
}

/// Advance once both hulls are in the world.
#[cfg(feature = "debug")]
fn the_range_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| live_structure(world).0 >= sections_on_the_range())
}

/// The envelope holds the range's whole scale at once.
#[cfg(feature = "debug")]
fn envelope_is_full(world: &World) -> bool {
    world
        .get_resource::<Peaks>()
        .is_some_and(|peaks| peaks.inbound >= INBOUND_PER_BAY * bays())
}

/// [`envelope_is_full`] as a script predicate.
#[cfg(feature = "debug")]
fn the_envelope_is_full() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(envelope_is_full)
}

/// Nothing left in the sky: no torpedo under guidance, no round in flight.
#[cfg(feature = "debug")]
fn nothing_in_flight() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| live_torpedoes(world) == 0 && live_rounds(world) == 0)
}

/// Both hulls are EXACTLY what they were built with. A range that lost mounts or
/// tubes would still fill a sky, only differently, and report a number for a
/// scene nobody authored.
#[cfg(feature = "debug")]
fn assert_the_range_stood_up(world: &mut World) {
    let (sections, roots) = live_structure(world);
    let up_mounts = world
        .iter_entities()
        .filter(|entity| entity.contains::<TurretSectionMarker>())
        .count();
    let up_bays = world
        .iter_entities()
        .filter(|entity| entity.contains::<TorpedoSectionMarker>())
        .count();
    assert_eq!(
        (sections, roots, up_mounts, up_bays),
        (sections_on_the_range(), 2, mounts(), bays()),
        "stress_point_defense: the range must stand up as exactly two hulls of \
         {} sections carrying {} mounts and {} bays",
        sections_on_the_range(),
        mounts(),
        bays()
    );
    nova_probe::probe_marker(
        world,
        "outcome: both hulls stood up whole",
        serde_json::json!({ "sections": sections, "mounts": up_mounts, "bays": up_bays }),
    );
    info!("stress_point_defense: {up_mounts} mounts up against {up_bays} bays");
}

/// The precondition for everything else: nobody is holding the battery, so the
/// Flight Computer has all of it. A mount the player still held would be aimed
/// by the crosshair feed instead of by the assignment, and the range would
/// measure the wrong chain.
#[cfg(feature = "debug")]
fn assert_the_computer_took_the_battery(world: &mut World) {
    let mut query = world.query::<&PointDefenseMount>();
    let held = query
        .iter(world)
        .filter(|mount| mount.authority == MountAuthority::FlightComputer)
        .count();
    assert_eq!(
        held,
        mounts(),
        "stress_point_defense: only {held} of {} mounts fell to the Flight \
         Computer - an unheld battery must be the computer's outright",
        mounts()
    );
    nova_probe::probe_marker(
        world,
        "outcome: the computer took every mount",
        serde_json::json!({ "held": held }),
    );
}

/// The scale claim on the THREATS: the envelope actually filled with inbound
/// ordnance, so the threat list and the radar scanner are a load.
#[cfg(feature = "debug")]
fn assert_the_envelope_filled(world: &mut World) {
    let floor = INBOUND_PER_BAY * bays();
    let peak = world.resource::<Peaks>().inbound;
    assert!(
        peak >= floor,
        "stress_point_defense: peaked at only {peak} inbound torpedoes inside \
         the envelope (want >= {floor}) - the launcher never reached the \
         range's scale"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the envelope filled with inbound ordnance",
        serde_json::json!({ "peak": peak, "floor": floor, "bays": bays() }),
    );
    info!("stress_point_defense: peak {peak} inbound inside the envelope");
}

/// The assignment claim: the battery was WORKING, not standing idle beside a
/// stream it never picked up.
#[cfg(feature = "debug")]
fn assert_the_mounts_were_working(world: &mut World) {
    let floor = mounts() * WORKING_MOUNTS_NUMERATOR / WORKING_MOUNTS_DENOMINATOR;
    let peak = world.resource::<Peaks>().working;
    assert!(
        peak >= floor,
        "stress_point_defense: only {peak} of {} mounts ever held an assignment \
         (want >= {floor}) - the assignment pass found nothing its mounts could \
         reach",
        mounts()
    );
    nova_probe::probe_marker(
        world,
        "outcome: the battery was working the stream",
        serde_json::json!({ "peak": peak, "floor": floor, "mounts": mounts() }),
    );
    info!("stress_point_defense: peak {peak} mounts working a torpedo");
}

/// The end-to-end claim: rounds actually reached torpedoes. Aim and trigger can
/// both run forever without ever connecting, and a range that only aimed would
/// measure two thirds of the chain while reading as if it measured all of it.
#[cfg(feature = "debug")]
fn assert_the_battery_connected(world: &mut World) {
    let now = world.resource::<Time<Virtual>>().elapsed_secs();
    let peaks = world.resource::<Peaks>();
    let (intercepts, fired, opened) = (peaks.intercepts, peaks.fired, peaks.tubes_open_secs);
    let fill = peaks.full_secs - opened;
    let frames = peaks.frames - peaks.open_frames;
    let steps = (peaks.steps - peaks.open_steps).max(1);
    let trigger_mount_steps = peaks.trigger_mount_steps;
    assert!(
        intercepts > 0,
        "stress_point_defense: the battery shot nothing down - the chain aimed \
         and fired but never connected"
    );
    // Per SIMULATED second and per ROUND, because both are what a frame rate
    // must not move. A wall-clock rate would only restate how fast the box ran.
    let engaged = (now - opened).max(f32::EPSILON);
    let rounds_per_sec = fired as f32 / engaged;
    let hits_per_round = intercepts as f32 / fired.max(1) as f32;
    let fps = frames as f32 / engaged;
    let rounds_per_step = fired as f32 / steps as f32;
    let duty = trigger_mount_steps as f32 / (steps * mounts()) as f32;
    let frames_per_step = frames as f32 / steps as f32;
    let aim_error_deg = peaks.aim_error_sum / peaks.aim_samples.max(1) as f64;
    nova_probe::probe_marker(
        world,
        "outcome: the battery shot torpedoes down",
        serde_json::json!({
            "intercepts": intercepts,
            "fired": fired,
            "engaged_secs": engaged,
            "rounds_per_sec": rounds_per_sec,
            "hits_per_round": hits_per_round,
            "fill_secs": fill,
            "fps": fps,
            "rounds_per_step": rounds_per_step,
            "trigger_duty": duty,
            "frames_per_step": frames_per_step,
            "aim_error_deg": aim_error_deg,
        }),
    );
    info!(
        "stress_point_defense: {intercepts} torpedoes shot down, {fired} rounds spent over \
         {engaged:.1} s ({rounds_per_sec:.0} rounds/s, {hits_per_round:.4} hits/round), \
         fill {fill:.1} s, {fps:.0} fps"
    );
    info!(
        "stress_point_defense: {frames_per_step:.2} frames/step, {rounds_per_step:.2} \
         rounds/step, trigger duty {duty:.3}, mean aim error {aim_error_deg:.3} deg (gate \
         {:.3} deg)",
        TURRET_ON_TARGET_RAD.to_degrees()
    );
}

/// The scale claim on the ROUNDS: how many were in the sky at once, which is
/// what the sweep in `nova_gameplay::rounds` is graded against.
///
/// A round is NOT a physics body and no longer appears in the collider census
/// beside it - the census now counts only what the broad phase indexes, which
/// is ships, sections, torpedoes and rocks. Rounds cost one shape cast each
/// instead, so this count and that one are two separate loads rather than one
/// number counted twice.
#[cfg(feature = "debug")]
fn assert_the_sky_filled(world: &mut World) {
    let floor = ROUNDS_PER_MOUNT * mounts();
    let peaks = world.resource::<Peaks>();
    let (peak, colliders) = (peaks.rounds, peaks.colliders);
    assert!(
        peak >= floor,
        "stress_point_defense: peaked at only {peak} rounds in the sky (want >= \
         {floor}) - the battery never reached the range's scale"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the sky filled with point-defense rounds",
        serde_json::json!({
            "peak": peak,
            "floor": floor,
            "mounts": mounts(),
            "colliders": colliders,
        }),
    );
    info!("stress_point_defense: peak {peak} rounds in the sky, {colliders} colliders");
}

/// Open the tubes, and stamp the simulated clock the fill starts on.
#[cfg(feature = "debug")]
fn open_the_tubes(world: &mut World) {
    world.resource_mut::<TubesOpen>().0 = true;
    let now = world.resource::<Time<Virtual>>().elapsed_secs();
    let mut peaks = world.resource_mut::<Peaks>();
    peaks.tubes_open_secs = now;
    peaks.open_frames = peaks.frames;
    peaks.open_steps = peaks.steps;
}

/// Stamp the simulated clock the envelope filled on. The FILL is the range's
/// own frame-rate canary: it is the launch rate against the intercept rate, so
/// a chain that defends better on a faster host takes visibly longer here.
#[cfg(feature = "debug")]
fn mark_the_envelope_full(world: &mut World) {
    let now = world.resource::<Time<Virtual>>().elapsed_secs();
    let mut peaks = world.resource_mut::<Peaks>();
    peaks.full_secs = now;
    let fill = now - peaks.tubes_open_secs;
    info!("stress_point_defense: the envelope filled in {fill:.1} s of simulated time");
}

/// Close the tubes. Nothing else stops the launcher: it has no controller, so
/// the range's own hold is the only thing keeping its triggers down.
#[cfg(feature = "debug")]
fn close_the_tubes(world: &mut World) {
    world.resource_mut::<TubesOpen>().0 = false;
}

/// The lifecycle claim: the sky came back to EXACTLY zero on the projectiles'
/// own fuzes and lifetimes, before anything was torn down. Asserted BEFORE the
/// teardown on purpose - the unload also takes every projectile, so asserting
/// after it would hide a stuck one rather than prove it drained.
#[cfg(feature = "debug")]
fn assert_the_sky_drained(world: &mut World) {
    let (torpedoes, rounds) = (live_torpedoes(world), live_rounds(world));
    assert_eq!(
        (torpedoes, rounds),
        (0, 0),
        "stress_point_defense: {torpedoes} torpedoes and {rounds} rounds still \
         in flight after the drain gate opened"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the sky drained to nothing",
        serde_json::json!({ "torpedoes": torpedoes, "rounds": rounds }),
    );
    info!("stress_point_defense: the sky drained to zero");
}

/// Drop the whole scene. `UnloadScenario` guarantees every scenario-scoped
/// entity goes, which is exactly the claim the next step checks.
#[cfg(feature = "debug")]
fn tear_the_range_down(world: &mut World) {
    nova_probe::probe_marker(world, "stress: teardown", serde_json::json!({}));
    world.trigger(UnloadScenario);
}

/// The teardown claim: nothing survived.
#[cfg(feature = "debug")]
fn assert_back_to_baseline(world: &mut World) {
    let (torpedoes, rounds) = (live_torpedoes(world), live_rounds(world));
    let (sections, roots) = live_structure(world);
    assert_eq!(
        (torpedoes, rounds, sections, roots),
        (0, 0, 0, 0),
        "stress_point_defense: {torpedoes} torpedoes, {rounds} rounds, \
         {sections} sections and {roots} ship roots survived UnloadScenario - \
         the range did not return to baseline"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the teardown left nothing behind",
        serde_json::json!({
            "torpedoes": torpedoes,
            "rounds": rounds,
            "sections": sections,
            "roots": roots,
        }),
    );
    info!("stress_point_defense: teardown returned to baseline");
}

/// Rebuild the range for the next capture cycle, the way the loop point does.
/// The peaks reset with it: every scale claim is per-cycle, so a later cycle
/// that shot nothing down must not pass on an earlier one's high-water mark.
#[cfg(feature = "debug")]
fn respawn_the_range(world: &mut World) {
    let (Some(game_assets), Some(sections)) = (
        world.get_resource::<GameAssets>().cloned(),
        world.get_resource::<GameSections>().cloned(),
    ) else {
        return;
    };
    *world.resource_mut::<Peaks>() = Peaks::default();
    nova_probe::capture_reload_begin(world);
    world.trigger(LoadScenario(range_scenario(&game_assets, &sections)));
}
