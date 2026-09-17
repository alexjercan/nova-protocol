//! system_chapter_one: the campaign's first chapter, flown end to end.
//!
//! Every other `systems/` range builds its own fixture in Rust. This one does
//! the opposite on purpose: it boots the SHIPPED scenario
//! (`season_one_chapter_one`, the base bundle's only campaign chapter today)
//! and walks it from the opening scene to the Victory banner, so what is
//! graded is the content a player loads rather than a model of it. A chapter
//! is a state machine wearing dialogue - one `beat` variable, fifteen handlers
//! and five cards - and the failure it is exposed to is not a crash but a beat
//! that never arms, a card that never comes down, or a clamp that completes a
//! card nobody posted.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the opening scene hands the helm back and posts the lane` | the way out of the cinematic resumes control and leaves exactly the lane card and the lane's first gate |
//! | 2 | `outcome: each mark comes down as the next one goes up` | the lane arms one gate at a time, four times, never the same one twice, and leaves none behind |
//! | 3 | `outcome: the collar is staged inside the capture envelope` | the pose the range flies Kaveri into is eligible by the mechanic's own reading - gap, facing and closing rate |
//! | 4 | `outcome: the clamp completes the docking card and starts the transfer` | `DOCK` on the staged pair joins both hulls and takes the docking card down |
//! | 5 | `outcome: letting go and clamping again leaves no card behind` | a release and a re-clamp inside the same breath post no card and complete none |
//! | 6 | `outcome: every card posts and completes in the authored order` | the whole objective-panel transcript, `+id` / `-id`, matches the chapter's authored order |
//! | 7 | `outcome: the chapter ends in victory with the board clear and both hulls alive` | the run ends on `Victory`, with no card left up and neither ship gone |
//!
//! The range STAGES two things the chapter expects a pilot to do, and grades
//! neither: it teleports Kaveri between the lane's gates instead of flying six
//! kilometres of rock by hand, and it writes the travel lock the docking verb
//! reads instead of earning one on the radar. Everything after the staging is
//! the real path - the real `DOCK` key, the real capture search, the real
//! handlers - which is what makes the markers mean anything.
//!
//! The world clock runs [`WORLD_SPEED`] times over. The chapter is about two
//! minutes of authored dialogue and the range makes no timing claim about any
//! of it, so the dilation costs nothing and keeps the walk inside the
//! harness's own completion deadline.
//!
//! "No error line is logged" is not a marker here. `probe run` reads the run
//! log for exactly that (its `log_clean` check), and a range cannot honestly
//! grade its own process's log from inside it.
//!
//! Headless smoke test (no display needed):
//!
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_chapter_one --features debug
//! # look for: `chapter one: PASS the chapter is won, board clear`
//! ```

use clap::Parser;

#[derive(Parser)]
#[command(name = "system_chapter_one")]
#[command(version = "1.0.0")]
#[command(about = "Season one chapter one, flown from the opening scene to the banner. Autopilot-only correctness range", long_about = None)]
struct Cli;

#[cfg(not(feature = "debug"))]
fn main() {
    let _ = Cli::parse();
    eprintln!("system_chapter_one walks the chapter on the debug-only autopilot;");
    eprintln!("run it with --features debug");
}

#[cfg(feature = "debug")]
use std::sync::Arc;

#[cfg(feature = "debug")]
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
#[cfg(feature = "debug")]
use bevy::{ecs::system::RunSystemOnce, prelude::*};
#[cfg(feature = "debug")]
use nova_protocol::{
    nova_debug::harness::{AutopilotPlugin, Predicate},
    nova_gameplay::transform::prelude::PointRotationOutput,
    prelude::*,
};

/// The shipped chapter this range flies.
#[cfg(feature = "debug")]
const CHAPTER: &str = "season_one_chapter_one";

/// The two hulls, by the scenario ids the chapter's own handlers name them by.
#[cfg(feature = "debug")]
const ID_KAVERI: &str = "kaveri";
#[cfg(feature = "debug")]
const ID_GANTRY: &str = "gantry";

/// The chapter's own beat counter, and the ladder it climbs.
///
/// Mirrored rather than imported: the numbers live in `nova_authoring`'s
/// private content builders, and a range that reached into them would be
/// grading the builder instead of the shipped file. A renumbered chapter
/// stalls the step that waits for the beat it lost, and the abort names that
/// step.
#[cfg(feature = "debug")]
const VAR_BEAT: &str = "beat";
/// The opening scene is running.
#[cfg(feature = "debug")]
const BEAT_OPEN: f64 = 1.0;
/// One beat per mark of the working lane.
#[cfg(feature = "debug")]
const LANE_BEATS: [f64; 4] = [2.0, 3.0, 4.0, 5.0];
/// The lane is behind them and the distress call is running.
#[cfg(feature = "debug")]
const BEAT_CALL: f64 = 6.0;
/// The diversion: come about and close on Gantry.
#[cfg(feature = "debug")]
const BEAT_REACH: f64 = 7.0;
/// Standing off Gantry with the docking card up.
#[cfg(feature = "debug")]
const BEAT_DOCK: f64 = 8.0;
/// Let go with people still to come: the breath before the card comes back.
#[cfg(feature = "debug")]
const BEAT_REGRIP: f64 = 8.5;
/// Clamped.
#[cfg(feature = "debug")]
const BEAT_HOLD: f64 = 9.0;
/// Clamped, with the hold card up and people crossing.
#[cfg(feature = "debug")]
const BEAT_TRANSFER: f64 = 10.0;
/// All three aboard: let go.
#[cfg(feature = "debug")]
const BEAT_RELEASE: f64 = 11.0;
/// The epilogue.
#[cfg(feature = "debug")]
const BEAT_OUTRO: f64 = 12.0;
/// The banner.
#[cfg(feature = "debug")]
const BEAT_WON: f64 = 13.0;

/// The chapter's cards, by the ids it posts them under.
#[cfg(feature = "debug")]
const OBJ_LANE: &str = "lane";
#[cfg(feature = "debug")]
const OBJ_REACH: &str = "reach";
#[cfg(feature = "debug")]
const OBJ_DOCK: &str = "dock";
#[cfg(feature = "debug")]
const OBJ_HOLD: &str = "hold";
#[cfg(feature = "debug")]
const OBJ_RELEASE: &str = "release";

/// The objective-panel transcript the walk must write, in order: `+id` as a
/// card goes up, `-id` as it comes down. The hold card appears twice because
/// the range deliberately lets go halfway through the evacuation and clamps
/// again, which starts the transfer over.
#[cfg(feature = "debug")]
fn expected_board() -> Vec<String> {
    [
        OBJ_LANE,
        OBJ_REACH,
        OBJ_DOCK,
        OBJ_HOLD,
        OBJ_HOLD,
        OBJ_RELEASE,
    ]
    .into_iter()
    .flat_map(|id| [format!("+{id}"), format!("-{id}")])
    .collect()
}

/// The transcript as it must read the moment the regrip is over: everything
/// above up to and including the hold card the early release took down.
#[cfg(feature = "debug")]
fn expected_board_at_regrip() -> Vec<String> {
    expected_board().into_iter().take(8).collect()
}

/// The suffix every arrival gate's scenario id carries. The lane's beacon
/// stands on the same place and is a sensor too, so the suffix - not the
/// marker - is what tells the two apart.
#[cfg(feature = "debug")]
const GATE_SUFFIX: &str = "_gate";

/// The player action the docking verb is bound to. The same key lets go
/// again, which is why every tap below is a press AND a release.
#[cfg(feature = "debug")]
const DOCK_ACTION: &str = "dock";

/// Where Kaveri is put down when it reaches Gantry: off the stranded hull's
/// port flank, which is the side its surviving collar is on. Well inside the
/// arrival volume and a long way outside either hull.
#[cfg(feature = "debug")]
const STANDOFF: Meters3 = Meters3::new(-400.0, 0.0, 0.0);

/// The face gap the two collars are staged at, engine units: half the
/// authored one-cell capture distance. Eligible with room to spare, and half a
/// cell of daylight between two hulls that must not touch.
#[cfg(feature = "debug")]
const FACE_GAP: f32 = 0.5;

/// How much faster than real time the world runs once the chapter is live.
///
/// The chapter is about two minutes of authored dialogue with nothing for a
/// pilot to do in most of it. Nothing here is a timing claim - every wait is
/// on a variable the chapter itself wrote - so the clock is dilated rather
/// than sat through. The probe's frame-time pass is turned off for the same
/// reason: a dilated clock is not a frame budget anyone should read.
#[cfg(feature = "debug")]
const WORLD_SPEED: f32 = 8.0;

/// Frames a settle step runs for. Long enough that the commands an action
/// group queues - a spawned sensor, a posted card, a declared outcome - have
/// all landed before the next step reads the world.
#[cfg(feature = "debug")]
const SETTLE: u32 = 12;

/// Frames the docking key is held. Long enough that the input pass cannot
/// miss the edge, short enough that the release is unambiguous.
#[cfg(feature = "debug")]
const TAP_FRAMES: u32 = 3;

/// Seconds the boot step gets. The chapter spawns two hulls, three procedural
/// moons and a scatter field behind an asset load; it is the longest wait in
/// the run and the only one that is not dialogue.
#[cfg(feature = "debug")]
const LOAD_DEADLINE_SECS: f32 = 120.0;

/// What the walk saw, kept so a report reads the whole run rather than the
/// frame it happens to fire on.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Walk {
    /// Every change to the objective panel, in order (see [`expected_board`]).
    board: Vec<String>,
    /// The cards showing at the last frame the recorder read them.
    showing: Vec<String>,
    /// The arrival gates the lane was flown through, in the order they opened.
    gates: Vec<String>,
    /// How many gates stood open at each of those arrivals.
    open_at_arrival: Vec<usize>,
}

#[cfg(feature = "debug")]
fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = editor_app(false, Some(StartupScenario::Id(CHAPTER.to_string())));

    app.init_resource::<Walk>();
    app.add_systems(Update, record_the_board);

    // The probe's timeline and invariant feed: without it every assert below
    // still fires, but the markers beside them reach no report and `probe run`
    // grades the range unprobeable.
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());

    app.add_plugins(script());
    app.run()
}

/// The walk, beat by beat.
#[cfg(feature = "debug")]
fn script() -> AutopilotPlugin<GameStates> {
    let mut script = AutopilotPlugin::<GameStates>::new()
        // The chapter is live when its own OnStart has run: the hull is up and
        // the beat counter is seeded. Waiting on the state alone would race
        // the spawns.
        .step("chapter one: boot into the chapter")
        .until(and(
            player_ship_present(),
            scenario_variable_is(VAR_BEAT, BEAT_OPEN),
        ))
        .deadline(LOAD_DEADLINE_SECS)
        .add()
        // The opening scene is skippable and this range lets it play: what it
        // hands back on the way out is claim 1, and the skip path leaves
        // through the same handler.
        .step("chapter one: watch the opening scene out")
        .on_enter(dilate_the_clock)
        .until(scenario_variable_is(VAR_BEAT, LANE_BEATS[0]))
        .deadline(STEP_DEADLINE_SECS)
        .add();
    script = report(script, "the helm", report_the_helm);

    // The lane: four marks, each armed by the arrival at the one before it.
    // The range steers at whichever gate is open rather than at a coordinate,
    // so a re-laid lane is still walked.
    let arrivals = [LANE_BEATS[1], LANE_BEATS[2], LANE_BEATS[3], BEAT_CALL];
    for (index, arrived) in arrivals.into_iter().enumerate() {
        script = settle(script, &format!("the lane's mark {}", index + 1))
            .step(format!("chapter one: fly the lane, leg {}", index + 1))
            .on_enter(fly_to_the_open_gate)
            .until(scenario_variable_is(VAR_BEAT, arrived))
            .deadline(STEP_DEADLINE_SECS)
            .add();
    }
    script = report(script, "the lane", report_the_lane);

    script = script
        // The call, the refusal, and Clearwell covering it: the one stretch of
        // the chapter with nothing to fly.
        .step("chapter one: hear the call out")
        .until(scenario_variable_is(VAR_BEAT, BEAT_REACH))
        .deadline(STEP_DEADLINE_SECS)
        .add();
    script = settle(script, "Gantry's arrival volume")
        .step("chapter one: come about on Gantry")
        .on_enter(stand_off_gantry)
        .until(scenario_variable_is(VAR_BEAT, BEAT_DOCK))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("chapter one: line Kaveri up on the collar")
        .on_enter(line_up_on_the_collar)
        .until(frames(SETTLE))
        .deadline(BEAT_DEADLINE_SECS)
        .add();
    script = report(script, "the envelope", report_the_envelope);

    script = tap_the_dock_key(
        script,
        "clamp on",
        scenario_variable_is(VAR_BEAT, BEAT_HOLD),
    );
    script = report(script, "the clamp", report_the_clamp);

    script = script
        .step("chapter one: wait for the first transfer beat")
        .until(scenario_variable_is(VAR_BEAT, BEAT_TRANSFER))
        .deadline(STEP_DEADLINE_SECS)
        .add();

    // The breath: let go with people still to come, and clamp again before the
    // ask can land. The chapter parks on a beat of its own for exactly this,
    // and claim 5 is that the panel comes out of it unchanged.
    script = tap_the_dock_key(
        script,
        "let go early",
        scenario_variable_is(VAR_BEAT, BEAT_REGRIP),
    );
    script = tap_the_dock_key(
        script,
        "clamp again",
        scenario_variable_is(VAR_BEAT, BEAT_HOLD),
    );
    script = report(script, "the regrip", report_the_regrip);

    script = script
        .step("chapter one: wait for all three aboard")
        .until(scenario_variable_is(VAR_BEAT, BEAT_RELEASE))
        .deadline(STEP_DEADLINE_SECS)
        .add();

    script = tap_the_dock_key(
        script,
        "let go for home",
        scenario_variable_is(VAR_BEAT, BEAT_OUTRO),
    );

    script = script
        .step("chapter one: the epilogue runs")
        .until(scenario_variable_is(VAR_BEAT, BEAT_WON))
        .deadline(STEP_DEADLINE_SECS)
        .add();
    report(script, "the chapter", report_the_chapter)
}

/// Let the world catch up before the next step reads it.
///
/// A scenario action does not change the world where it runs: it queues a
/// command. A step that read the panel, or steered at a gate, on the frame
/// after the beat changed would be reading the world one action group early.
#[cfg(feature = "debug")]
fn settle(script: AutopilotPlugin<GameStates>, what: &str) -> AutopilotPlugin<GameStates> {
    script
        .step(format!("chapter one: let {what} land"))
        .until(frames(SETTLE))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
}

/// Settle, then make one claim about what settled.
#[cfg(feature = "debug")]
fn report(
    script: AutopilotPlugin<GameStates>,
    what: &str,
    claim: impl Fn(&mut World) + Send + Sync + 'static,
) -> AutopilotPlugin<GameStates> {
    settle(script, what)
        .step(format!("chapter one: report {what}"))
        .on_enter(claim)
        .add()
}

/// Press and release the docking key, and hold until the press has earned
/// something.
///
/// Two steps, because a press and its release are two frames and the verb
/// fires on the edge: the same key docks and undocks, so a key left held would
/// be the next tap's press.
#[cfg(feature = "debug")]
fn tap_the_dock_key(
    script: AutopilotPlugin<GameStates>,
    what: &str,
    until: Arc<Predicate>,
) -> AutopilotPlugin<GameStates> {
    script
        .step(format!("chapter one: {what} - press [D]"))
        .on_enter(press_action(DOCK_ACTION))
        .until(frames(TAP_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step(format!("chapter one: {what} - let [D] go"))
        .on_enter(release_action(DOCK_ACTION))
        .until(until)
        .deadline(BEAT_DEADLINE_SECS)
        .add()
}

// --- the world reads ---------------------------------------------------------

/// Run the world at [`WORLD_SPEED`], once the chapter is live so the load
/// itself is untouched.
#[cfg(feature = "debug")]
fn dilate_the_clock(world: &mut World) {
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(WORLD_SPEED);
    info!("chapter one: the world clock runs {WORLD_SPEED}x for the walk");
}

/// Keep the objective panel's transcript: what went up, what came down, in the
/// order the chapter did it.
///
/// A change detector rather than a reading of the final state, because the
/// claim is the ORDER - a card that posts and completes inside one beat is
/// invisible to anything that only reads the end.
#[cfg(feature = "debug")]
fn record_the_board(objectives: Option<Res<GameObjectives>>, mut walk: ResMut<Walk>) {
    let Some(objectives) = objectives else {
        return;
    };
    if !objectives.is_changed() {
        return;
    }
    let showing: Vec<String> = objectives
        .objectives
        .iter()
        .map(|objective| objective.id.clone())
        .collect();
    let mut completed: Vec<String> = walk
        .showing
        .iter()
        .filter(|id| !showing.contains(id))
        .cloned()
        .collect();
    let mut posted: Vec<String> = showing
        .iter()
        .filter(|id| !walk.showing.contains(id))
        .cloned()
        .collect();
    // One frame can carry both. Completions first, each side sorted, so the
    // transcript is a function of the world rather than of iteration order.
    completed.sort();
    posted.sort();
    for id in completed {
        walk.board.push(format!("-{id}"));
    }
    for id in posted {
        walk.board.push(format!("+{id}"));
    }
    walk.showing = showing;
}

/// The ship hull wearing this scenario id, or a named panic.
#[cfg(feature = "debug")]
fn hull(world: &mut World, id: &str) -> Entity {
    world
        .try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()
        .and_then(|mut hulls| {
            hulls
                .iter(world)
                .find(|(_, entity_id)| entity_id.0 == id)
                .map(|(entity, _)| entity)
        })
        .unwrap_or_else(|| panic!("chapter one: no hull called '{id}' is in the world"))
}

/// Every arrival gate standing open right now, as (scenario id, place).
#[cfg(feature = "debug")]
fn open_gates(world: &mut World) -> Vec<(String, Vec3)> {
    let Some(mut gates) =
        world.try_query_filtered::<(&EntityId, &Transform), With<ScenarioAreaMarker>>()
    else {
        return Vec::new();
    };
    let mut open: Vec<(String, Vec3)> = gates
        .iter(world)
        .filter(|(id, _)| id.0.ends_with(GATE_SUFFIX))
        .map(|(id, transform)| (id.0.clone(), transform.translation))
        .collect();
    open.sort_by(|left, right| left.0.cmp(&right.0));
    open
}

/// Put a hull down at a pose, at rest, and tell its computer that is where it
/// meant to be.
///
/// Both avian's pose and the render transform, because avian owns the former
/// and nothing has propagated the latter yet; both velocities, because a body
/// carried into a new place at its old speed is not the same staging.
///
/// And the helm, which is the part a teleport cannot skip. A live hull holds
/// an attitude COMMAND on its controller section and torques itself back onto
/// it every tick, so a hull turned by hand and left alone flies straight back
/// to the way it was pointing - through whatever is now beside it. Re-parking
/// the command is the same thing docking does when it takes the helm.
#[cfg(feature = "debug")]
fn place(world: &mut World, ship: Entity, position: Vec3, rotation: Quat) {
    world.entity_mut(ship).insert((
        Position(position),
        Rotation(rotation),
        Transform::from_translation(position).with_rotation(rotation),
        LinearVelocity(Vec3::ZERO),
        AngularVelocity(Vec3::ZERO),
    ));
    park_the_helm(world, ship, rotation);
}

/// Point every live computer on `ship` at `rotation`, and the aim the player's
/// half of that command is slewed toward.
///
/// Two writes, because the player's helm is two things. The computer holds the
/// command the PD controller flies; the camera rig holds where the pilot is
/// LOOKING, and every tick the ship is under the mouse the command is stepped
/// back toward it. Writing only the command would be overwritten within the
/// frame; writing only the aim would slew the hull round over a second or two
/// instead of putting it there. The range only ever places the player's hull,
/// so the one rig in the world is the one being placed.
#[cfg(feature = "debug")]
fn park_the_helm(world: &mut World, ship: Entity, rotation: Quat) {
    if let Some(mut computers) =
        world.try_query::<(&ChildOf, &mut ControllerSectionRotationInput)>()
    {
        for (mount, mut command) in computers.iter_mut(world) {
            if mount.0 == ship {
                command.0 = rotation;
            }
        }
    }
    if let Some(mut rigs) = world.try_query_filtered::<&mut PointRotationOutput, (
        With<SpaceshipCameraInputMarker>,
        With<SpaceshipCameraNormalInputMarker>,
    )>() {
        for mut aim in rigs.iter_mut(world) {
            aim.0 = rotation;
        }
    }
}

/// Fly the leg: put Kaveri in whichever arrival gate the chapter has open, and
/// record what was open when it did.
#[cfg(feature = "debug")]
fn fly_to_the_open_gate(world: &mut World) {
    let gates = open_gates(world);
    let (id, at) = gates
        .first()
        .cloned()
        .expect("chapter one: the lane arms a gate before it asks the captain for one");
    let kaveri = hull(world, ID_KAVERI);
    place(world, kaveri, at, Quat::IDENTITY);

    let mut walk = world.resource_mut::<Walk>();
    assert!(
        !walk.gates.contains(&id),
        "chapter one: '{id}' was flown to twice - the lane re-armed a mark it had taken down"
    );
    walk.open_at_arrival.push(gates.len());
    walk.gates.push(id);
}

/// Arrive in Gantry's volume, standing off its port flank.
#[cfg(feature = "debug")]
fn stand_off_gantry(world: &mut World) {
    let gantry = hull(world, ID_GANTRY);
    let at = world
        .get::<Position>(gantry)
        .expect("chapter one: Gantry is a physics body")
        .0;
    let kaveri = hull(world, ID_KAVERI);
    place(world, kaveri, at + STANDOFF.to_engine(), Quat::IDENTITY);
}

/// What the mechanic's own port search reads on the two hulls, plus the pose
/// that would put Kaveri's collar square on Gantry's.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct Staging {
    kaveri: Entity,
    gantry: Entity,
    /// Where Kaveri has to be, and how it has to be turned, for the two
    /// collars to face each other across [`FACE_GAP`].
    position: Vec3,
    rotation: Quat,
    /// What the pair reads right now: the face gap, how opposed the two axes
    /// are, and whether `DOCK` would be offered on it.
    gap: f32,
    opposition: f32,
    eligible: bool,
    envelope: DockingEnvelope,
}

/// Read the pair through [`DockingPorts`] - the same search the verb and the
/// docking sight use, so the range cannot stage against one set of numbers and
/// then be graded against another.
#[cfg(feature = "debug")]
fn read_the_staging(
    ports: DockingPorts,
    hulls: Query<(Entity, &EntityId, &Position, &Rotation), With<SpaceshipRootMarker>>,
) -> Option<Staging> {
    let find = |wanted: &str| {
        hulls
            .iter()
            .find(|(_, id, ..)| id.0 == wanted)
            .map(|(entity, _, position, rotation)| (entity, position.0, rotation.0))
    };
    let (kaveri, kaveri_at, kaveri_facing) = find(ID_KAVERI)?;
    let (gantry, ..) = find(ID_GANTRY)?;
    let pair = ports.nearest_pair(kaveri, gantry)?;

    // Square on, half a cell out: turn Kaveri until its own port axis is the
    // opposite of Gantry's, then slide the whole hull so its port face lands
    // on the mating point. Roll is free - the capture ignores it, because a
    // cylindrical collar has nothing to key.
    let turn = Quat::from_rotation_arc(pair.first.axis, -pair.second.axis);
    let mating = pair.second.face + pair.second.axis * FACE_GAP;
    Some(Staging {
        kaveri,
        gantry,
        position: mating - turn * (pair.first.face - kaveri_at),
        rotation: turn * kaveri_facing,
        gap: pair.gap,
        opposition: pair.opposition,
        eligible: pair.is_eligible(),
        envelope: pair.envelope,
    })
}

#[cfg(feature = "debug")]
fn staging(world: &mut World) -> Staging {
    world
        .run_system_once(read_the_staging)
        .expect("chapter one: the port search runs")
        .expect("chapter one: both hulls carry a free docking port")
}

/// Bring Kaveri's collar onto Gantry's, and hold Gantry on the radar.
///
/// The lock is staged, not earned: taking one is a dwell on the radar, which
/// this range does not grade. The verb READS the lock, so without it the key
/// press would be a silent no-op and the step waiting for the clamp would
/// stall against nothing.
#[cfg(feature = "debug")]
fn line_up_on_the_collar(world: &mut World) {
    let staged = staging(world);
    place(world, staged.kaveri, staged.position, staged.rotation);
    world
        .entity_mut(staged.kaveri)
        .insert(TravelLock(Some(staged.gantry)));
}

// --- the claims --------------------------------------------------------------

/// The cards showing right now.
#[cfg(feature = "debug")]
fn showing(world: &World) -> Vec<String> {
    world
        .resource::<GameObjectives>()
        .objectives
        .iter()
        .map(|objective| objective.id.clone())
        .collect()
}

#[cfg(feature = "debug")]
fn report_the_helm(world: &mut World) {
    // The identity check the other ranges get from `assert_scenario_loaded`.
    // That plugin reads the load on the way INTO `Playing`, and a startup
    // scenario is queued and built after the state is already there, so the
    // chapter has to say its own name here instead.
    let loaded = world
        .get_resource::<CurrentScenario>()
        .and_then(|scenario| scenario.0.as_ref().map(|config| config.id.clone()))
        .expect("chapter one: a scenario is loaded");
    assert_eq!(
        loaded, CHAPTER,
        "chapter one: the walk must be flying the chapter it names"
    );
    let suspended = world
        .get_resource::<PlayerControlSuspended>()
        .is_some_and(PlayerControlSuspended::is_suspended);
    assert!(
        !suspended,
        "chapter one: the opening scene kept the helm - every way out of it must resume control"
    );
    let cards = showing(world);
    assert_eq!(
        cards,
        vec![OBJ_LANE.to_string()],
        "chapter one: the scene ends on the lane card and nothing else"
    );
    let gates = open_gates(world);
    assert_eq!(
        gates.len(),
        1,
        "chapter one: the scene ends with the lane's first gate up and no other: {gates:?}"
    );
    let first = gates[0].0.clone();
    nova_probe::probe_marker(
        world,
        "outcome: the opening scene hands the helm back and posts the lane",
        serde_json::json!({ "scenario": loaded, "card": OBJ_LANE, "gate": first }),
    );
}

#[cfg(feature = "debug")]
fn report_the_lane(world: &mut World) {
    let walk = world.resource::<Walk>();
    let gates = walk.gates.clone();
    let open = walk.open_at_arrival.clone();
    assert_eq!(
        gates.len(),
        LANE_BEATS.len(),
        "chapter one: the lane is four marks long: {gates:?}"
    );
    assert!(
        open.iter().all(|count| *count == 1),
        "chapter one: the lane must arm exactly one gate at a time: {open:?}"
    );
    let left_open = open_gates(world);
    assert!(
        left_open.is_empty(),
        "chapter one: the lane takes its last mark down behind it: {left_open:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: each mark comes down as the next one goes up",
        serde_json::json!({ "gates": gates }),
    );
}

#[cfg(feature = "debug")]
fn report_the_envelope(world: &mut World) {
    let staged = staging(world);
    assert!(
        staged.eligible,
        "chapter one: the staged pair must be one the verb would act on: gap {:.2} of {:.2}, \
         opposition {:.3}",
        staged.gap, staged.envelope.capture_distance, staged.opposition
    );
    let off_axis_deg = (-staged.opposition).clamp(-1.0, 1.0).acos().to_degrees();
    nova_probe::probe_marker(
        world,
        "outcome: the collar is staged inside the capture envelope",
        serde_json::json!({
            "gap_m": Meters::from_engine(staged.gap).get(),
            "capture_m": Meters::from_engine(staged.envelope.capture_distance).get(),
            "off_axis_deg": off_axis_deg,
        }),
    );
}

#[cfg(feature = "debug")]
fn report_the_clamp(world: &mut World) {
    for id in [ID_KAVERI, ID_GANTRY] {
        let ship = hull(world, id);
        assert!(
            world.get::<DockedShip>(ship).is_some(),
            "chapter one: '{id}' must know it is held once the chapter says it is clamped"
        );
    }
    let board = world.resource::<Walk>().board.clone();
    assert_eq!(
        board.last().map(String::as_str),
        Some("-dock"),
        "chapter one: the clamp takes the docking card down: {board:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the clamp completes the docking card and starts the transfer",
        serde_json::json!({ "board": board }),
    );
}

#[cfg(feature = "debug")]
fn report_the_regrip(world: &mut World) {
    let board = world.resource::<Walk>().board.clone();
    assert_eq!(
        board,
        expected_board_at_regrip(),
        "chapter one: a release and a re-clamp inside the same breath must post no card and \
         complete none"
    );
    assert!(
        showing(world).is_empty(),
        "chapter one: nothing is asked of a captain who is already back on the collar"
    );
    nova_probe::probe_marker(
        world,
        "outcome: letting go and clamping again leaves no card behind",
        serde_json::json!({ "board": board }),
    );
}

#[cfg(feature = "debug")]
fn report_the_chapter(world: &mut World) {
    let board = world.resource::<Walk>().board.clone();
    assert_eq!(
        board,
        expected_board(),
        "chapter one: the objective panel must read the chapter's authored order"
    );
    nova_probe::probe_marker(
        world,
        "outcome: every card posts and completes in the authored order",
        serde_json::json!({ "board": board }),
    );

    let outcome = world
        .get_resource::<CurrentOutcome>()
        .and_then(|outcome| outcome.0.clone())
        .expect("chapter one: the chapter declares an outcome");
    assert_eq!(
        outcome.outcome,
        ScenarioOutcomeKind::Victory,
        "chapter one: bringing three people off Gantry is a win"
    );
    let left_up = showing(world);
    assert!(
        left_up.is_empty(),
        "chapter one: the banner comes up on a clear board: {left_up:?}"
    );
    // Both hulls are still there to be found: a chapter that wins by losing
    // one of them is the failure this reading exists for.
    for id in [ID_KAVERI, ID_GANTRY] {
        let _ = hull(world, id);
    }
    nova_probe::probe_marker(
        world,
        "outcome: the chapter ends in victory with the board clear and both hulls alive",
        serde_json::json!({
            "banner": outcome.message,
            "last_card": OBJ_RELEASE,
            "reached": OBJ_REACH,
        }),
    );
    info!("chapter one: PASS the chapter is won, board clear");
}
