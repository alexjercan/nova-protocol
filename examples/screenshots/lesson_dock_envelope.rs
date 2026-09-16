//! lesson_dock_envelope: the SHIPBUILDING lesson about the capture envelope -
//! the four things a dock asks for at once, and the instrument that grades
//! them while you fly.
//!
//! One producer, one sheet. The lesson lists four gates (10 m of face gap, 15
//! degrees of facing, under 5 m/s of closing rate, under 5 degrees a second of
//! relative spin), and a still can only ever show one moment of one of them.
//! What a pilot actually reads is the docking sight CHANGING COLOUR as each
//! gate comes good, so this lesson was flipped from `still(...)` to
//! `looping(...)` in `crates/nova_authoring/src/base_content/lessons.rs` and
//! its alt text rewritten to describe the footage.
//!
//! The twenty cells hold, in order:
//! - a locked pair well out of square, every part of the sight in nav cyan;
//! - the tender turning until the two port axes are opposed, which turns both
//!   face plates green ([`DockingPair::facing_holds`]);
//! - the turn settling, which drops the relative spin back under its gate;
//! - the gap running down through the last capture ticks - still cyan, because
//!   the run-in is faster than the closing-rate gate allows;
//! - the brake, which is the frame the gap line and its ticks go green as well
//!   ([`DockingPair::gap_holds`] and [`DockingPair::motion_holds`] together).
//!
//! ## Why the approach is flown rather than posed
//!
//! Every number the sight grades is a MEASUREMENT of two rigid bodies:
//! `DockingPorts::nearest_pair` reads avian's own poses and velocities. A
//! sheet that moved the hull by writing its transform would show a sight
//! colouring against a relative speed of zero - the gate the lesson is about
//! would be met by the fact that nothing was really moving. So the tender
//! flies: [`measure_the_pair`] reads the same pair the instrument draws,
//! [`aim_the_helm`] points the ship's own mouse rig at the attitude that
//! opposes the two ports, and [`fly_the_approach`] holds the closing burn. The
//! turn is therefore the SHIP's - commanded heading, computer turn rate, PD
//! loop - and the gates in the footage are graded against motion that really
//! happened.
//!
//! The turn is also what buys the spin gate. While the hull is swinging round
//! to square, its relative spin is far over the envelope's 5 deg/s, so the gap
//! line stays cyan - a hull that is still turning may not dock however close it
//! is - and the line only goes green once the helm has settled AND the last
//! capture distance has gone. [`aim_the_helm`] carries the account of the
//! hand-written angular velocity that came first and lost a fight with the
//! ship's own helm.
//!
//! ## What the row does that the lesson did not ask for
//!
//! The keybind dock offers DOCK exactly when a pair would be ACCEPTED - its
//! availability runs `DockingPorts::best_candidate`, the same search the
//! command runs (`nova_ship/src/input/player/hints.rs`). So the verb row grows
//! a `D DOCK` chip on the same cell the sight goes green, and the sheet ends
//! with the offer on screen. The alt text says so: a reader who has met
//! `start_verbs` is being shown the two halves of one state.
//!
//! ## Why the clamp is not in the sheet
//!
//! The twenty cells are spent on the four gates coming good, which is the
//! lesson's own sentence. Taking the dock inside the recording would spend the
//! last cells on the joint and, worse, would REMOVE the sight: a held hull has
//! nothing left to line up, so `sync_docking_sight` stops drawing the very
//! instrument this lesson is about. The sheet ends on the frame a pilot is
//! flying for, with every part of the sight green.
//!
//! ## Why the camera leaves the cockpit
//!
//! This is the one lesson in my set whose subject is in WORLD space and spans
//! two hulls. The player's own chase camera stands astern of the tender and
//! looks down its bow, which foreshortens the gap line to a dot and hides the
//! near face plate inside the hull it is drawn on. The eye is therefore pinned
//! off the quarter ([`SHEET_EYE`]), high enough to look down the line and far
//! enough across it that both crosses read as crosses. The HUD stays up: the
//! lock bracket on the spar is what says which hull the sight belongs to.
//!
//! The scene is the `docking_approach` playable's, staged for a camera rather
//! than for a pilot: the same two hand-built hulls, one port each, with the
//! spar moored close enough that the whole approach fits in two seconds.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - lock, fly the approach, check
//!   the gates, exit clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_dock_envelope --features debug
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID};

#[derive(Parser)]
#[command(name = "lesson_dock_envelope")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's docking envelope demonstration", long_about = None)]
struct Cli;

/// The sheet this tiles: "Making a dock".
#[cfg(feature = "debug")]
const ENVELOPE_LESSON: &str = "build_dock_envelope";

/// Scenario id of the hull the sheet flies.
const TENDER_ID: &str = "envelope_tender";
/// Scenario id of the hull it is lining up on.
const SPAR_ID: &str = "envelope_spar";

/// How far ahead the spar is moored, in meters.
///
/// Both port faces stand 15 m off their hull's origin and the mooring's own
/// error swings the far one out, which leaves the sheet opening on a face gap
/// of about 16 m - a gap and a half of the 10 m capture distance. Tuned
/// against the recorded approach rather than computed: at the run-in speed the
/// gap gate then closes about three fifths of the way through the twenty
/// cells, with the plates already green and the spin just settled, so the last
/// third of the loop is the frame a pilot is flying for.
const SPAR_RANGE: f32 = 43.0;

/// How far the spar is moored off square, in degrees.
///
/// The error the tender has to turn out. Twice the 15 degree facing gate, so
/// the plates spend the first cells cyan and the reader sees them change
/// rather than finding them already green.
const SPAR_YAW_ERROR: f32 = 24.0;

/// A little pitch on the mooring as well, in degrees, so the error the tender
/// turns out is not a flat yaw a reader could mistake for a camera angle.
const SPAR_PITCH_ERROR: f32 = 5.0;

/// How fast the tender runs the gap down, in meters a second.
///
/// Deliberately OVER the envelope's 5 m/s ceiling. A run-in that was already
/// legal would leave the closing-rate gate out of the lesson entirely: here
/// the hull arrives inside the capture distance still going too fast to be
/// allowed to dock, which is the cells where the gap line is cyan with
/// everything else green, and the reader sees what that gate is for.
#[cfg(feature = "debug")]
const RUN_SPEED: MetersPerSecond = MetersPerSecond(9.0);

/// What it brakes to at the capture distance, in meters a second.
///
/// Well inside the ceiling. The brake is triggered by the gap gate itself
/// (`fly_the_approach`), not by a cell count, so the footage is a pilot who
/// closes on the ticks and eases off when the last one goes rather than a
/// stopwatch that happens to agree with the geometry.
#[cfg(feature = "debug")]
const CREEP_SPEED: MetersPerSecond = MetersPerSecond(2.0);

/// Which way is up when the helm is pointed at the spar.
///
/// World up, so the hull arrives square with the scene rather than clocked to
/// wherever the turn left it. Roll is outside the capture envelope, so this
/// picks the look of the shot and nothing else.
#[cfg(feature = "debug")]
const HELM_UP: Vec3 = Vec3::Y;

/// The face gap the tender stops closing at, in meters.
///
/// Well inside the 10 m capture distance and well outside a collision: the
/// sheet's last cells hold the frame a pilot would press the dock key in.
#[cfg(feature = "debug")]
const HOLD_GAP: f32 = 6.0;

/// Cells the sheet holds the locked, unmoving pair before the approach starts.
#[cfg(feature = "debug")]
const LEAD_CELLS: u32 = 2;

/// Seconds the script holds the radar key on the spar.
///
/// Past the gesture threshold and the acquisition dwell at this range, the way
/// `docking_approach` holds it - the lock is what picks the pair the sight
/// draws, so a walk that tapped it would record an empty sky.
#[cfg(feature = "debug")]
const LOCK_HOLD: f32 = 3.0;

/// Where the sheet's eye stands, in meters: off the tender's starboard
/// quarter and above the line, so the gap line runs across the cell and both
/// face plates read as crosses rather than as edges.
#[cfg(feature = "debug")]
const SHEET_EYE: Meters3 = Meters3::new(82.0, 30.0, 30.0);

/// What it looks at: a point on the approach line between the two hulls, so
/// the pair takes the middle of the cell with the sight between them.
#[cfg(feature = "debug")]
const SHEET_LOOK: Meters3 = Meters3::new(0.0, 0.0, -24.0);

/// What the sight sees, and the two orders that answer it.
///
/// Written by [`measure_the_pair`] from the same `DockingPair` the instrument
/// draws, and read by [`fly_the_approach`] and by the walk's own assertions -
/// so the footage, the flying and the check are all grading one measurement.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone, Copy, Default)]
struct Approach {
    /// Face gap, in meters.
    gap: f32,
    /// How far the two port axes are from opposed, in degrees.
    facing: f32,
    /// How fast the two hulls are closing, in meters a second.
    closing: f32,
    /// How fast they are turning relative to each other, in degrees a second.
    spin: f32,
    /// Whether the faces are near enough.
    gap_holds: bool,
    /// Whether the axes are opposed enough.
    facing_holds: bool,
    /// Whether the pair is calm enough.
    motion_holds: bool,
    /// Unit direction from the tender's face to the spar's, engine axes.
    closing_axis: Vec3,
    /// The direction the tender's bow has to point for the two port axes to be
    /// opposed - what the helm is aimed at.
    opposed: Vec3,
}

#[cfg(feature = "debug")]
impl Approach {
    /// Every gate the dock asks for, holding at once.
    fn holds(&self) -> bool {
        self.gap_holds && self.facing_holds && self.motion_holds
    }
}

/// Set while the tender is flying the approach. Absent, the hull sits where
/// the scenario put it - which is what the lock beats and the framing beats
/// need.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct Approaching;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(envelope_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.init_resource::<Approach>();
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(dock_envelope_script());
        // The pair is measured on every frame the walk is armed, so the beats
        // can wait on a gate rather than on a stopwatch; the orders it writes
        // only reach the hull once the approach has started.
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.add_systems(
                Update,
                (
                    measure_the_pair,
                    fly_the_approach.run_if(resource_exists::<Approaching>),
                )
                    .chain(),
            );
        }
    }

    app.run()
}

fn envelope_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_envelope);
}

fn load_envelope(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(envelope_scene(&game_assets)));
}

/// One prototype section at a build cell, square with the hull.
fn part(id: &str, prototype: &str, cell: Vec3) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id: id.to_string(),
        position: cell,
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(prototype),
    }
}

/// The tender: a bow port, a spine, two shoulder plates and a drive.
///
/// The port stands at the very bow with nothing in front of it, which is the
/// port's own clearance rule - its face is what the capture is measured from.
fn tender() -> ShipDesign {
    clad(vec![
        part(
            "dock_bow",
            DOCKING_PORT_SECTION_ID,
            Vec3::new(0.0, 0.0, -1.0),
        ),
        part("bridge", BASIC_CONTROLLER_SECTION_ID, Vec3::ZERO),
        part("shoulder_port", LIGHT_HULL_SECTION_ID, Vec3::NEG_X),
        part("shoulder_starboard", LIGHT_HULL_SECTION_ID, Vec3::X),
        part("drive", BASIC_THRUSTER_SECTION_ID, Vec3::Z),
    ])
}

/// The spar: a moored stack with one port on its near end and nothing that
/// flies. It is the thing you dock WITH, not a second ship.
fn spar() -> ShipDesign {
    clad(vec![
        part(
            "dock_fore",
            DOCKING_PORT_SECTION_ID,
            Vec3::new(0.0, 0.0, -1.0),
        ),
        part("midships", REINFORCED_HULL_SECTION_ID, Vec3::ZERO),
        part("aft", REINFORCED_HULL_SECTION_ID, Vec3::Z),
        part("mast_high", LIGHT_HULL_SECTION_ID, Vec3::Y),
        part("mast_low", LIGHT_HULL_SECTION_ID, Vec3::NEG_Y),
    ])
}

/// A hand-built cell list wearing the derived skin, the way every block hull
/// in the fleet is dressed. A design that leaves this off renders as bare
/// cells, which is a look the game ships nowhere.
fn clad(sections: Vec<SpaceshipSectionConfig>) -> ShipDesign {
    ShipDesign {
        sections,
        presentation: ShipPresentationConfig {
            skin: true,
            style: Some("industrial".to_string()),
            ..ShipPresentationConfig::base_voice()
        },
        ..default()
    }
}

fn ship_object(
    id: &str,
    name: &str,
    position: Meters3,
    rotation: Quat,
    controller: SpaceshipController,
    design: ShipDesign,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller,
            design: ShipDesignSource::Inline(design),
            ..default()
        }),
    })
}

/// The tender at the origin and the spar moored ahead of it, out of square.
fn envelope_scene(game_assets: &GameAssets) -> ScenarioConfig {
    let spar_position = Meters3::new(0.0, 0.0, -SPAR_RANGE);
    let tender = ship_object(
        TENDER_ID,
        "Tender",
        Meters3::ZERO,
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig::default()),
        tender(),
    );
    let spar = ship_object(
        SPAR_ID,
        "Mooring Spar",
        spar_position,
        Quat::from_rotation_y(std::f32::consts::PI + SPAR_YAW_ERROR.to_radians())
            * Quat::from_rotation_x(SPAR_PITCH_ERROR.to_radians()),
        SpaceshipController::None,
        spar(),
    );

    ScenarioConfig {
        description: "A ported tender lining up on a moored spar".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![tender, spar],
                ThreePointRig::around("envelope", spar_position, 8.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "lesson_dock_envelope".to_string(),
            "Docking Envelope".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Read the pair the sight is drawing, and work out the two orders that fly it
/// into the envelope.
///
/// The same `DockingPorts::nearest_pair` the instrument calls, so the sheet
/// cannot be flown against one set of numbers while it is coloured against
/// another.
#[cfg(feature = "debug")]
fn measure_the_pair(
    ports: DockingPorts,
    player: Query<(Entity, &TravelLock), (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    mut approach: ResMut<Approach>,
) {
    let Ok((ship, lock)) = player.single() else {
        return;
    };
    let Some(target) = lock.0 else {
        return;
    };
    let Some(pair) = ports.nearest_pair(ship, target) else {
        return;
    };

    // Where the bow has to point for the two axes to be opposed. The port is
    // mounted square on the bow, so the port's axis IS the hull's forward and
    // the helm can be given this directly.
    let opposed = -pair.second.axis;
    let error = pair.first.axis.angle_between(opposed);

    *approach = Approach {
        gap: Meters::from_engine(pair.gap).get(),
        facing: error.to_degrees(),
        closing: MetersPerSecond::from_engine(pair.relative_speed).get(),
        spin: pair.relative_spin.to_degrees(),
        gap_holds: pair.gap_holds(),
        facing_holds: pair.facing_holds(),
        motion_holds: pair.motion_holds(),
        closing_axis: (pair.second.face - pair.first.face).normalize_or_zero(),
        opposed,
    };
    // One line per frame, off by default: the cell a gate changes on is what
    // the staging constants are tuned against, and a capture is too slow to
    // watch. `RUST_LOG=info,lesson_dock_envelope=debug` prints the approach.
    debug!(
        "dock approach: gap {:.1} m ({}), facing {:.1} deg ({}), closing {:.1} m/s, spin {:.1} \
         deg/s ({})",
        approach.gap,
        approach.gap_holds,
        approach.facing,
        approach.facing_holds,
        approach.closing,
        approach.spin,
        approach.motion_holds
    );
}

/// Hold the closing burn on the hull.
///
/// A velocity rather than a transform: the closing-rate gate is a measurement
/// of how the two bodies are MOVING, and a hull teleported down the line would
/// be graded as a hull at rest. The TURN is not written here at all - it is
/// the ship's own helm, aimed by [`aim_the_helm`].
#[cfg(feature = "debug")]
fn fly_the_approach(
    approach: Res<Approach>,
    mut player: Query<
        &mut avian3d::prelude::LinearVelocity,
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    // Three regimes, in the order a pilot flies them: run the gap down, brake
    // to a legal closing rate once the last capture tick has gone, and stop
    // inside the hold gap - a tender that kept closing would ram the spar
    // inside the sheet.
    let speed = if approach.gap <= HOLD_GAP {
        MetersPerSecond::ZERO
    } else if approach.gap_holds {
        CREEP_SPEED
    } else {
        RUN_SPEED
    };
    let closing = approach.closing_axis * speed.to_engine();
    for mut linear in &mut player {
        linear.0 = closing;
    }
}

/// Point the helm at the spar: the rig the player's mouse turns, written to
/// the attitude that opposes the two port axes.
///
/// The hull is NOT turned here. `update_controller_target_rotation_torque`
/// slews the ship's commanded heading toward this rig at the computer's own
/// turn rate and the PD loop flies it, so the spin in the footage is the
/// ship's, it eases in and out the way a real correction does, and it stops
/// when the hull has actually arrived. The first cut wrote an angular velocity
/// straight onto the body instead and spent the whole sheet fighting this helm
/// - the facing error stalled five degrees out with the hull still swinging at
/// ten degrees a second, because the mouse rig was still pointed where the
/// walk had left it.
#[cfg(feature = "debug")]
fn aim_the_helm(world: &mut World) {
    let opposed = world.resource::<Approach>().opposed;
    let heading = Transform::default().looking_to(opposed, HELM_UP).rotation;
    let rigs: Vec<Entity> = world
        .query_filtered::<Entity, With<PointRotationOutput>>()
        .iter(world)
        .collect();
    for rig in rigs {
        world.entity_mut(rig).insert((
            PointRotation {
                initial_rotation: heading,
            },
            PointRotationOutput(heading),
        ));
    }
}

/// Raise the instruments and drop the fps/version bar, which names the build
/// the capture came from.
#[cfg(feature = "debug")]
fn raise_the_instruments(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::On;
    }
    hide_status_bar(world);
}

/// Advance once the lock has landed on the spar - the selector the sight and
/// the DOCK verb both read.
#[cfg(feature = "debug")]
fn the_spar_is_locked() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(locked) = world
            .try_query_filtered::<&TravelLock, With<PlayerSpaceshipMarker>>()
            .and_then(|mut ships| ships.iter(world).next().and_then(|lock| lock.0))
        else {
            return false;
        };
        world
            .get::<EntityId>(locked)
            .is_some_and(|id| id.0 == SPAR_ID)
    })
}

/// Advance once every gate the dock asks for is holding at once.
///
/// The beat that flies the approach waits on this AS WELL AS on the sheet,
/// which is what makes the smoke path mean something: unarmed, there is no
/// sheet to wait for, so a beat held only on `sheet_written` would end the
/// walk on the frame the approach began and hand the closing check a hull
/// that had not moved yet.
#[cfg(feature = "debug")]
fn every_gate_holds() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<Approach>(|approach| approach.holds())
}

/// Advance once the sight is actually being drawn.
#[cfg(feature = "debug")]
fn the_sight_is_drawn() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query::<&DockingSightPart>()
            .is_some_and(|mut parts| parts.iter(world).next().is_some())
    })
}

/// Check the sheet closed on the frame the lesson describes, and say what the
/// four gates read.
#[cfg(feature = "debug")]
fn the_envelope_holds(world: &mut World) {
    let approach = *world.resource::<Approach>();
    info!(
        "lesson dock: gap {:.1} m, facing {:.1} deg, closing {:.1} m/s, spin {:.1} deg/s",
        approach.gap, approach.facing, approach.closing, approach.spin
    );
    assert!(
        approach.holds(),
        "the sheet must end inside the envelope: gap {:.1} m (holds {}), facing {:.1} deg \
         (holds {}), motion {:.1} m/s and {:.1} deg/s (holds {})",
        approach.gap,
        approach.gap_holds,
        approach.facing,
        approach.facing_holds,
        approach.closing,
        approach.spin,
        approach.motion_holds
    );
}

/// Load, lock the spar, frame the pair, and record the approach coming good.
#[cfg(feature = "debug")]
fn dock_envelope_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the tender and the spar")
        .enter(GameStates::Loading)
        .until(and(state_is(GameStates::Playing), player_ship_present()))
        .deadline(30.0)
        .add()
        .step("settle the scene")
        .on_enter(raise_the_instruments)
        .until(elapsed(1.0))
        .add()
        // The lock is a HOLD, and it happens while the camera is still the
        // player's own: the radar picks the body nearest the camera's aim ray,
        // and the sheet's eye is off the quarter where that ray points at
        // nothing.
        .step("hold the radar onto the spar")
        .on_enter(press_action("radar_hold"))
        .until(elapsed(LOCK_HOLD))
        .add()
        .step("release the radar")
        .on_enter(release_action("radar_hold"))
        .until(and(the_spar_is_locked(), the_sight_is_drawn()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("stand the eye off the quarter")
        .on_enter(|world: &mut World| pose_camera(world, SHEET_EYE, SHEET_LOOK))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("open the sheet on the pair out of square")
        .on_enter(|world: &mut World| sheet_start(world, ENVELOPE_LESSON, LESSON_GRID))
        .until(frames(LEAD_CELLS))
        .add()
        .step("fly the approach")
        .on_enter(aim_the_helm)
        .on_enter(|world: &mut World| world.insert_resource(Approaching))
        .until(and(sheet_written(ENVELOPE_LESSON), every_gate_holds()))
        .deadline(60.0)
        .add()
        .step("every gate holds")
        .on_enter(the_envelope_holds)
        .add()
}
