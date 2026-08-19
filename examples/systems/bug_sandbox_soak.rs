//! bug_sandbox_soak: the editor sandbox, entered the way a player enters it, then
//! LEFT ALONE.
//!
//! The walk is the owner's - main menu, Sandbox, New Ship, Play - and then the
//! script does nothing at all for [`SOAK_SECS`]. Sitting still IS the gesture:
//! the range exists because the sandbox held its frame rate for a few seconds
//! after Play and then fell to 2 FPS on its own, with no input, no spawning,
//! and a flat entity count.
//!
//! It ASSERTS one thing and RECORDS the rest. The one thing is structural:
//! every rock in the settled field collides as a HULL. avian sleeps only
//! TOUCHING contact pairs, so a belt whose neighbours overlap at the AABB and
//! never meet keeps every one of those pairs in the ACTIVE set and re-manifolds
//! them every step forever - and trimesh against trimesh is the most expensive
//! manifold parry can be asked for. That is the defect, and a rock's collider
//! shape is the same fact on any box.
//!
//! It does NOT assert TIME. Milliseconds are a statement about the host, not
//! about the code: load on this box moved these numbers 4x, and a timing gate
//! on a shared runner generates false alarms instead of catching regressions.
//! The step cost, the narrow-phase cost, the contact pairs and the constraints
//! they hold are recorded as probe evidence and flagged past
//! [`FIXED_TIMESTEP_MS`] / [`IDLE_CONTACT_NOTICE_MS`] with a WARN for a
//! reviewer to judge. They are how the collapse was diagnosed and they stay;
//! they are not a verdict. Same reading as the probe's own
//! `fps_within_baseline`. Do not turn them back into asserts.
//!
//! The frame numbers underneath, for reading a flagged run: bevy's fixed loop
//! catches up on an overrun, and `Time<Virtual>`'s `max_delta` lets one
//! rendered frame pay for sixteen steps at the shipped 64 Hz. A step 1.4x over
//! the timestep therefore does not cost 1.4x - it saturates the accumulator and
//! pins the frame at sixteen steps, which is the 2 FPS floor and the reason F1
//! back to the editor "fixes" it instantly.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=400 \
//!   cargo run --example bug_sandbox_soak --features debug
//! # look for: `sandbox_soak: N unshot rocks, none colliding as a trimesh`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::Collider;
#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "bug_sandbox_soak")]
#[command(version = "1.0.0")]
#[command(about = "The editor sandbox, entered and then left alone. Autopilot-only correctness range - sitting still IS the gesture", long_about = None)]
struct Cli;

/// Soak a SHIPPED scenario by id instead of walking the editor, so the
/// sandbox's numbers can be read beside another range's.
#[cfg(feature = "debug")]
const SCENARIO_ENV: &str = "NOVA_SOAK_SCENARIO";

/// Override [`SOAK_SECS`], for proving a curve over minutes rather than the
/// seconds the claims need.
#[cfg(feature = "debug")]
const SOAK_SECS_ENV: &str = "NOVA_SOAK_SECS";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    #[cfg(feature = "debug")]
    let comparison = std::env::var(SCENARIO_ENV).ok().filter(|id| !id.is_empty());
    #[cfg(not(feature = "debug"))]
    let comparison: Option<String> = None;

    // The same editor app the game binary runs - not a bespoke copy.
    let mut app = editor_app(true, comparison.clone().map(StartupScenario::Id));

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.init_resource::<StepTally>();
        app.add_systems(FixedUpdate, count_fixed_steps);
        app.add_systems(Update, (count_frames, sample_the_soak).chain());
        app.add_plugins(match comparison {
            Some(_) => scenario_script(),
            None => sandbox_script(),
        });
    }

    app.run()
}

/// Frames a beat waits for a gesture to land: the picking backend needs a
/// frame to raycast the new pointer position and the editor's observers a
/// frame to react.
#[cfg(feature = "debug")]
const SETTLE: u32 = 10;

/// Frames the run waits after creating a ship, for the preview to spawn.
#[cfg(feature = "debug")]
const SHIP_SETTLE: u32 = 40;

/// How long the run sits still. Long enough that a curve which is going to
/// slide has slid: the collapse this range pins was complete within six
/// seconds of the field spawning, and a slower box only reaches it sooner.
#[cfg(feature = "debug")]
const SOAK_SECS: f32 = 45.0;

/// Seconds between samples.
#[cfg(feature = "debug")]
const SAMPLE_SECS: f64 = 5.0;

/// The walk: menu -> Sandbox -> New Ship -> Play -> sit still -> verdict.
#[cfg(feature = "debug")]
fn sandbox_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("soak: reach the main menu")
        .until(state_is(GameStates::MainMenu))
        .deadline(90.0)
        .add()
        .step("soak: let the menu lay out")
        .until(frames(SETTLE))
        .add()
        .step("soak: click Sandbox")
        .on_enter(click_named("Sandbox Button"))
        .until(frames(SETTLE))
        .add()
        // The menu buttons act on `Activate`, which fires on RELEASE over the
        // same widget - so a click is two beats.
        .step("soak: release Sandbox")
        .on_enter(release_mouse(MouseButton::Left))
        .until(state_is(GameStates::Playing))
        .deadline(90.0)
        .add()
        .step("soak: let the editor lay out")
        .until(frames(SETTLE))
        .add()
        .step("soak: click New Ship")
        .on_enter(click_named("Create New Spaceship Button V2"))
        .until(frames(SETTLE))
        .add()
        .step("soak: release New Ship")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SHIP_SETTLE))
        .add()
        .step("soak: press Play")
        .on_enter(click_named("Play Button"))
        .until(frames(SETTLE))
        .add()
        .step("soak: release Play")
        .on_enter(release_mouse(MouseButton::Left))
        .until(player_ship_present())
        .deadline(120.0)
        .add()
        .step("soak: sit still")
        .until(elapsed(soak_secs()))
        .deadline(soak_secs() + 120.0)
        .add()
        .step("soak: read the settled scene")
        .on_enter(read_the_settled_scene)
        .until(frames(1))
        .add()
}

/// The comparison walk: a shipped scenario booted straight past the menu, then
/// the same sit-still and the same verdict.
#[cfg(feature = "debug")]
fn scenario_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("soak: load the range")
        .until(player_ship_present())
        .deadline(120.0)
        .add()
        .step("soak: sit still")
        .until(elapsed(soak_secs()))
        .deadline(soak_secs() + 120.0)
        .add()
        .step("soak: read the settled scene")
        .on_enter(read_the_settled_scene)
        .until(frames(1))
        .add()
}

/// [`SOAK_SECS`], or the [`SOAK_SECS_ENV`] override.
#[cfg(feature = "debug")]
fn soak_secs() -> f32 {
    std::env::var(SOAK_SECS_ENV)
        .ok()
        .and_then(|secs| secs.parse().ok())
        .unwrap_or(SOAK_SECS)
}

/// Fixed steps and rendered frames since the last sample, so a sample can say
/// how many physics steps one frame paid for.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct StepTally {
    steps: u32,
    frames: u32,
}

#[cfg(feature = "debug")]
fn count_fixed_steps(mut tally: ResMut<StepTally>) {
    tally.steps += 1;
}

#[cfg(feature = "debug")]
fn count_frames(mut tally: ResMut<StepTally>) {
    tally.frames += 1;
}

/// A smoothed avian diagnostic by path, or zero when it has no reading yet.
#[cfg(feature = "debug")]
fn avian_ms(world: &World, path: &str) -> f64 {
    world
        .resource::<bevy::diagnostic::DiagnosticsStore>()
        .iter()
        .find(|diagnostic| diagnostic.path().as_str() == path)
        .and_then(bevy::diagnostic::Diagnostic::smoothed)
        .unwrap_or_default()
}

/// avian's whole-step time, milliseconds.
#[cfg(feature = "debug")]
const STEP_TIME: &str = "avian/total_step_time";

/// avian's narrow phase, milliseconds: the manifold pass over every ACTIVE
/// contact pair.
#[cfg(feature = "debug")]
const UPDATE_CONTACTS: &str = "avian/collision/update_contacts";

/// The shipped fixed timestep, in milliseconds. What a recorded step cost is
/// READ against: a step over it cannot be paid for at rate, and bevy's catch-up
/// turns the overrun into a saturated accumulator rather than a proportional
/// slowdown. A reference for a reviewer, never a gate - see the module doc.
#[cfg(feature = "debug")]
const FIXED_TIMESTEP_MS: f64 = 1000.0 / 64.0;

/// Where a recorded narrow-phase cost over an untouched field is worth a WARN.
/// Three orders below the timestep and two above what the fix measures, so a
/// flag means the SHAPE of the work changed rather than that the host was busy.
/// Same nature as `FPS_WARN_THRESHOLD_PCT`: one tunable, and never a failure.
#[cfg(feature = "debug")]
const IDLE_CONTACT_NOTICE_MS: f64 = 5.0;

/// Every rock in the settled field, and how many collide as a TRIMESH.
///
/// `None` when the world carries no asteroid at all, which is a run that
/// measured an empty scene. The collider hangs on the rock's collider node,
/// a child of the marked root.
#[cfg(feature = "debug")]
fn rock_collider_shapes(world: &World) -> Option<(usize, usize)> {
    let mut roots = world.try_query_filtered::<&Children, With<AsteroidMarker>>()?;
    let (mut rocks, mut trimeshes) = (0, 0);
    for children in roots.iter(world) {
        for child in children.iter() {
            let Some(collider) = world.get::<Collider>(child) else {
                continue;
            };
            rocks += 1;
            if collider.shape().as_trimesh().is_some() {
                trimeshes += 1;
            }
        }
    }
    Some((rocks, trimeshes))
}

/// Assert the collider shape the fix installed, and RECORD what the settled
/// scene costs. The split is the point: one is a fact about the code, the rest
/// are facts about the box that ran it.
#[cfg(feature = "debug")]
fn read_the_settled_scene(world: &mut World) {
    let step_ms = avian_ms(world, STEP_TIME);
    let contacts_ms = avian_ms(world, UPDATE_CONTACTS);
    let pairs = avian_ms(world, "avian/collision/contact_count");
    let constraints = avian_ms(world, "avian/solver/contact_constraint_count");

    let (rocks, trimeshes) =
        rock_collider_shapes(world).expect("the settled sandbox carries an asteroid field");
    assert!(
        rocks > 0,
        "the soak found no rock collider at all - the field never spawned, and \
         a run that read an empty scene proves nothing"
    );
    assert_eq!(
        trimeshes, 0,
        "{trimeshes} of {rocks} unshot rocks collide as their drawn surface; \
         avian holds every never-touching pair in the active set and \
         re-manifolds it each step, and trimesh against trimesh is the most \
         expensive manifold there is"
    );
    nova_probe::probe_marker(
        world,
        "outcome: every unshot rock collides as a hull",
        serde_json::json!({ "rocks": rocks, "trimesh_rocks": trimeshes }),
    );

    nova_probe::probe_marker(
        world,
        "outcome: the idle contact cost is recorded",
        serde_json::json!({
            "update_contacts_ms": contacts_ms,
            "contact_pairs": pairs,
            "contact_constraints": constraints,
            "notice_ms": IDLE_CONTACT_NOTICE_MS,
        }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the settled step cost is recorded",
        serde_json::json!({ "step_ms": step_ms, "timestep_ms": FIXED_TIMESTEP_MS }),
    );

    info!("sandbox_soak: {rocks} unshot rocks, none colliding as a trimesh");
    info!(
        "sandbox_soak: step {step_ms:.2} ms against a {FIXED_TIMESTEP_MS:.2} ms \
         timestep, narrow phase {contacts_ms:.2} ms over {pairs:.0} pairs \
         holding {constraints:.0} constraints"
    );
    if contacts_ms >= IDLE_CONTACT_NOTICE_MS || step_ms >= FIXED_TIMESTEP_MS {
        warn!(
            "sandbox_soak: the settled scene reads expensive ({contacts_ms:.2} ms \
             narrow phase, {step_ms:.2} ms step). Host-noisy by nature - judge it \
             against a quiet run before believing it"
        );
    }
}

/// One line per [`SAMPLE_SECS`]: the frame, the live entity count, how many
/// physics steps one frame paid for, and what a step cost. The entity count is
/// there because "the population is growing" is the first thing a collapse
/// that takes time looks like, and here it is flat.
#[cfg(feature = "debug")]
fn sample_the_soak(world: &mut World, mut next: Local<f64>) {
    use bevy::diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin};

    let now = world.resource::<Time>().elapsed_secs_f64();
    if now < *next {
        return;
    }
    *next = now + SAMPLE_SECS;

    let store = world.resource::<DiagnosticsStore>();
    let fps = store
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(Diagnostic::smoothed)
        .unwrap_or_default();
    let frame_ms = store
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(Diagnostic::smoothed)
        .unwrap_or_default();

    // `Entities::len` is the high-water mark of allocated rows rather than the
    // live count, so sum the archetypes.
    let entities: u32 = world
        .archetypes()
        .iter()
        .map(bevy::ecs::archetype::Archetype::len)
        .sum();

    let step_ms = avian_ms(world, STEP_TIME);
    let contacts_ms = avian_ms(world, UPDATE_CONTACTS);
    let pairs = avian_ms(world, "avian/collision/contact_count");

    let mut tally = world.resource_mut::<StepTally>();
    let (steps, frames) = (tally.steps, tally.frames.max(1));
    tally.steps = 0;
    tally.frames = 0;
    let per_frame = f64::from(steps) / f64::from(frames);

    info!(
        "soak {now:7.1}s | fps {fps:6.1} | frame {frame_ms:8.1} ms | entities {entities} | \
         steps/frame {per_frame:6.2} | step {step_ms:6.2} ms | contacts {contacts_ms:6.2} ms \
         over {pairs:.0} pairs"
    );
}
