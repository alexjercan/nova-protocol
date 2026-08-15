//! controller_section: the controller section's PD attitude control.
//!
//! One minimal ship (controller + hull, no player input) chases a slowly
//! rotating attitude command written straight into
//! [`ControllerSectionRotationInput`] - the same seam the mouse, the AI and
//! the autopilot write. Watch the hull swing to track the command arrow;
//! the section under test is the PD that turns "desired rotation" into
//! torque.
//!
//! The scripted run walks FOUR named invariants across three rounds and two
//! rig layouts, so "the PD tracks" is not one lucky sample:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: attitude command swept` | the command swept, and sits clear of the spawn attitude, so a frozen hull cannot pass |
//! | 2 | `outcome: attitude tracks` | the hull is inside [`TRACK_TOLERANCE_RAD`] of it |
//! | 3 | `outcome: attitude tracks on rig b` | it still is on a DIFFERENT inertia tensor |
//! | 4 | `outcome: attitude reconverges after reload` | a fresh rig re-converges from identity |
//!
//! Every beat waits on a world value - the rig being up, the command having
//! swept clear of where that rig spawned - never on a runway, so a slow load
//! delays the walk instead of truncating it. The beats deliberately do NOT wait
//! on the tracking error: that is what the asserts decide, and a beat sharing
//! their constant would make them unfailable.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example controller_section --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `attitude probe: rig a tracks the command`,
//! #           `attitude probe: the reload tracks the command`,
//! #           `attitude probe: rig b tracks the command`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use std::sync::Arc;

#[cfg(feature = "debug")]
use avian3d::prelude::Rotation;
use bevy::{color::palettes::tailwind, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "controller_section")]
#[command(version = "1.0.0")]
#[command(about = "PD attitude control: a minimal ship chases a rotating attitude command", long_about = None)]
struct Cli;

/// How fast the commanded attitude sweeps, radians per second. Slow enough
/// that a healthy PD tracks with a small lag; fast enough that a dead PD
/// falls a full radian behind within the smoke window.
const COMMAND_RAD_PER_SEC: f32 = 0.35;

/// Which rig layout a load builds. The two differ in their INERTIA TENSOR -
/// layout B hangs a second hull off the roll axis - so a PD that only tracks
/// because it was tuned for one specific mass distribution fails round three.
#[derive(Clone, Copy)]
enum Layout {
    /// Controller + one hull, in a line along +Z.
    A,
    /// Layout A plus a hull mounted beside the first one, off the +Z axis.
    #[cfg(feature = "debug")]
    B,
}

impl Layout {
    /// The scenario id this layout loads under. Distinct ids so the run log
    /// says which rig is live.
    fn scenario_id(self) -> &'static str {
        match self {
            Layout::A => "controller_rig_a",
            #[cfg(feature = "debug")]
            Layout::B => "controller_rig_b",
        }
    }
}

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PERF_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(attitude_script());
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

/// The scripted run: converge on rig A, reload it and converge again, then
/// swap to rig B's inertia tensor and converge a third time.
///
/// Not the stock `nova_autopilot()` preset: every beat waits on the value it
/// depends on - the rig being up, the command having swept clear of that rig's
/// spawn attitude - so a slow load delays the walk instead of truncating it.
/// The per-step deadlines are what the runway used to be, and they NAME the
/// beat that stalled; their sum (86s) stays well under
/// `DEFAULT_DEADLINE_SECS` (120s) so a named stall wins the race against the
/// generic collector deadline.
#[cfg(feature = "debug")]
fn attitude_script() -> Script {
    let script = Script::new()
        .step("load rig a")
        .enter(GameStates::Loading)
        .until(rig_present())
        .deadline(20.0)
        .add()
        .step("sweep the command clear of rig a's spawn attitude")
        .until(command_delivered())
        .deadline(12.0)
        .add()
        .step("assert rig a tracks")
        .on_enter(assert_rig_a_tracks)
        .add();
    let script = then_reload(script, Layout::A)
        .step("sweep the command clear of the reloaded rig's spawn attitude")
        .until(command_delivered())
        .deadline(12.0)
        .add()
        .step("assert the reload tracks")
        .on_enter(assert_reload_tracks)
        .add();
    then_reload(script, Layout::B)
        .step("sweep the command clear of rig b's spawn attitude")
        .until(command_delivered())
        .deadline(12.0)
        .add()
        // Last beat: the driver reports done after it, so the run ends on the
        // assertion rather than idling out a runway.
        .step("assert rig b tracks")
        .on_enter(assert_rig_b_tracks)
        .add()
}

/// The reload gate, appended to `script`: trigger the load, then wait for a rig
/// that is DEMONSTRABLY the fresh one - a root entity that is not the one the
/// reload replaced.
///
/// Not "wait for the rig to be gone, then back": there is no such frame.
/// `on_load_scenario` queues the scoped despawns and fires `OnStartEvent` on
/// the SAME `Commands` (nova_scenario/src/loader/lifecycle.rs:162-253), so the
/// teardown and the respawn land in one flush and `not(rig_present())` never
/// observes true - it stalls out its deadline. A despawned entity's id never
/// comes back (avian and bevy bump the generation), so "a root that is not the
/// replaced one" is the observable form of the same claim.
#[cfg(feature = "debug")]
fn then_reload(script: Script, layout: Layout) -> Script {
    script
        .step("reload the rig")
        .on_enter(move |world: &mut World| load_layout(world, layout))
        .until(rig_reset())
        .deadline(15.0)
        .add()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_rig);
    app.add_systems(Update, (drive_command, draw_command_arrow));
    #[cfg(feature = "debug")]
    {
        app.init_resource::<RigEpoch>();
        app.add_systems(Update, track_rig_epoch);
    }
}

/// Which rig root is live and when it appeared, so both script gates read a
/// PER-ROUND quantity instead of the absolute clock.
///
/// The command keeps sweeping across a reload (a fresh rig has to catch up to
/// wherever it got to, which is what invariant 4 is about), so the command's
/// own angle from identity wraps through zero every ~18s and is not a usable
/// guard. Measured from `spawned_at` it grows monotonically inside a round.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct RigEpoch {
    /// The live rig root, and `Time::elapsed_secs` when it appeared.
    root: Option<Entity>,
    spawned_at: f32,
    /// The attitude that rig appeared at. Read from the world rather than
    /// assumed to be identity, so [`command_offset_from_spawn`] measures the
    /// separation a frozen hull would actually show.
    spawn_rotation: Quat,
    /// The root a reload is replacing; [`rig_reset`] waits for `root` to become
    /// something else.
    replacing: Option<Entity>,
}

/// Stamp each freshly spawned rig root into [`RigEpoch`].
#[cfg(feature = "debug")]
fn track_rig_epoch(
    time: Res<Time>,
    mut epoch: ResMut<RigEpoch>,
    q_root: Query<(Entity, Option<&Rotation>), Added<SpaceshipRootMarker>>,
) {
    for (root, rotation) in &q_root {
        epoch.root = Some(root);
        epoch.spawned_at = time.elapsed_secs();
        epoch.spawn_rotation = rotation.map_or(Quat::IDENTITY, |r| r.0);
    }
}

fn setup_rig(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(attitude_rig(
        &game_assets,
        &sections,
        Layout::A,
    )));
}

/// Re-trigger the scenario load with `layout`, from a script beat.
#[cfg(feature = "debug")]
fn load_layout(world: &mut World, layout: Layout) {
    let config = {
        let game_assets = world.resource::<GameAssets>();
        let sections = world.resource::<GameSections>();
        attitude_rig(game_assets, sections, layout)
    };
    // Record the root the reload is about to tear down, so `rig_reset` can tell
    // the replacement apart from it.
    let mut epoch = world.resource_mut::<RigEpoch>();
    epoch.replacing = epoch.root;
    info!("attitude probe: loading {}", layout.scenario_id());
    world.trigger(LoadScenario(config));
}

/// The rig scenario: one sectioned ship with no player and no AI - rotation
/// authority belongs to this example's command writer alone.
fn attitude_rig(
    game_assets: &GameAssets,
    sections: &GameSections,
    layout: Layout,
) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };

    // Only layout B pushes onto this, and layout B is debug-only.
    #[cfg_attr(
        not(feature = "debug"),
        expect(unused_mut, reason = "the only push is behind `debug`")
    )]
    let mut ship_sections = vec![
        at("controller", "basic_controller_section", Vec3::ZERO),
        at("hull", "reinforced_hull_section", Vec3::new(0.0, 0.0, 1.0)),
    ];
    #[cfg(feature = "debug")]
    if matches!(layout, Layout::B) {
        // Mounted BESIDE the first hull (adjacent, so the structure stays
        // connected) rather than behind it: an off-axis mass moves the
        // inertia tensor off the rig-A diagonal, which is the point of the
        // second layout.
        ship_sections.push(at(
            "hull_offaxis",
            "reinforced_hull_section",
            Vec3::new(1.0, 0.0, 1.0),
        ));
    }

    let ship = SpaceshipConfig {
        collapse_threshold: None,
        allegiance: None,
        controller: SpaceshipController::None,
        sections: ship_sections,
    };

    ScenarioConfig {
        description: "A minimal ship chasing a rotating attitude command.".to_string(),
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
            layout.scenario_id().to_string(),
            "Controller Section Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The commanded attitude at time `t`: a steady yaw sweep. Pure so the demo
/// writer and the probe agree on it by construction.
fn command_at(elapsed: f32) -> Quat {
    Quat::from_rotation_y(elapsed * COMMAND_RAD_PER_SEC)
}

/// Write the rotating command into the controller section's input - the
/// seam every rotation authority (mouse, AI, autopilot) writes.
fn drive_command(
    time: Res<Time>,
    mut q_input: Query<&mut ControllerSectionRotationInput, With<ControllerSectionMarker>>,
) {
    for mut input in &mut q_input {
        input.0 = command_at(time.elapsed_secs());
    }
}

/// Show the command as an arrow from the ship, so the chase is visible.
fn draw_command_arrow(
    time: Res<Time>,
    mut gizmos: Gizmos,
    q_ship: Query<&Transform, With<SpaceshipRootMarker>>,
) {
    for transform in &q_ship {
        let dir = command_at(time.elapsed_secs()) * Vec3::NEG_Z;
        gizmos.arrow(
            transform.translation,
            transform.translation + dir * 6.0,
            tailwind::CYAN_400,
        );
    }
}

/// Tracking error tolerance, radians. A healthy PD (frequency 4, damping 4)
/// tracks the slow sweep with a fraction of this; a dead or misfiring PD
/// falls behind by the full swept angle. Well above the f32 quat noise
/// floor (~1e-3 rad).
#[cfg(feature = "debug")]
const TRACK_TOLERANCE_RAD: f32 = 0.35;

/// Delivery guard, part one: how far the command must have SWEPT since this rig
/// spawned before "the hull tracks it" means anything - i.e. how much chasing
/// the PD has been asked to do this round. Comfortably above
/// [`TRACK_TOLERANCE_RAD`], so clearing it is a real chase rather than noise.
///
/// Arc length travelled, so this is monotonic and usable as a beat. It does NOT
/// bound the command's distance from the spawn attitude - that is part two,
/// [`command_offset_from_spawn`], and the guard needs both.
#[cfg(feature = "debug")]
const COMMAND_SWEEP_GUARD_RAD: f32 = 1.2;

/// A rig ship exists. The rigs run `SpaceshipController::None`, so this is
/// the root marker rather than `player_ship_present()`.
#[cfg(feature = "debug")]
fn rig_present() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    any_entity::<With<SpaceshipRootMarker>>()
}

/// Angle between the rig's live attitude and the command it is chasing, or
/// `None` while no rig is up.
#[cfg(feature = "debug")]
fn tracking_error(world: &World) -> Option<f32> {
    let command = command_at(world.get_resource::<Time>()?.elapsed_secs());
    let mut query = world.try_query_filtered::<&Rotation, With<SpaceshipRootMarker>>()?;
    let rotation = query.iter(world).next()?;
    Some(rotation.0.angle_between(command))
}

/// How far the command has swept since the live rig spawned - i.e. away from
/// the identity attitude that rig started at.
///
/// The INTEGRATED angle, not `angle_between(command, IDENTITY)`: the latter is
/// periodic, peaks at pi and wraps back under [`COMMAND_SWEEP_GUARD_RAD`] for a
/// ~7s window every turn, so a rig that spawned inside that window could never
/// clear the guard and the beat would stall out its whole deadline. Rate times
/// in-round time is monotonic by construction.
#[cfg(feature = "debug")]
fn sweep_since_spawn(world: &World) -> Option<f32> {
    let epoch = world.get_resource::<RigEpoch>()?;
    epoch.root?;
    let elapsed = world.get_resource::<Time>()?.elapsed_secs();
    Some((elapsed - epoch.spawned_at) * COMMAND_RAD_PER_SEC)
}

/// How far the command currently sits from the attitude the live rig spawned
/// at.
///
/// The delivery guard needs BOTH this and [`sweep_since_spawn`], and they are
/// different claims. Arc length travelled is monotonic, so it is the usable
/// BEAT - but the command keeps its absolute phase across a reload, so a rig
/// that spawns at phase ~-1.2 rad has the command back on top of the spawn
/// attitude exactly as `swept` clears its guard, and a dead PD parked where it
/// spawned would satisfy invariants 1 and 2 together. This is the separation
/// that excludes it: above [`TRACK_TOLERANCE_RAD`] a hull frozen at spawn is
/// outside the tracking tolerance by construction.
#[cfg(feature = "debug")]
fn command_offset_from_spawn(world: &World) -> Option<f32> {
    let epoch = world.get_resource::<RigEpoch>()?;
    epoch.root?;
    let elapsed = world.get_resource::<Time>()?.elapsed_secs();
    Some(command_at(elapsed).angle_between(epoch.spawn_rotation))
}

/// How much margin the beat's offset clause carries over the assert's, in
/// seconds of sweep. The autopilot enters the next step on the FOLLOWING frame
/// (`nova_autopilot::autopilot`: "entry gets its own frame"), and the offset is
/// a triangle wave, so a beat that opened on the DESCENDING edge at exactly the
/// assert's threshold would hand the assert a value one frame lower. Half a
/// second of sweep is orders of magnitude more than a frame at any framerate
/// this runs at.
#[cfg(feature = "debug")]
const OFFSET_BEAT_MARGIN_SECS: f32 = 0.5;

/// The beat every round waits on: the command has swept far enough this round,
/// and it now sits clear of the attitude the rig spawned at.
///
/// Both clauses are about the COMMAND - the stimulus. Deliberately nothing
/// about the hull: gating on `tracking_error < TRACK_TOLERANCE_RAD` would be
/// the same comparison [`assert_tracking`] makes with the same constant, which
/// makes that assert unfailable and turns a dead PD into a deadline stall on
/// this beat's name instead of the message that explains it. The beat delivers
/// the stimulus; whether the hull FOLLOWED it is the assertions' question.
///
/// The offset clause is strictly stronger than the assert's by
/// [`OFFSET_BEAT_MARGIN_SECS`], so the assert cannot fail on a moment this beat
/// would have opened on.
///
/// No settle clause: the sweep guard IS the PD's catch-up runway. A round's
/// clock starts at the rig's spawn and the guard needs
/// [`COMMAND_SWEEP_GUARD_RAD`] (1.2 rad) at [`COMMAND_RAD_PER_SEC`]
/// (0.35 rad/s), so the beat cannot open until 3.4s of chasing have passed -
/// longer than the transient, and longer than any fixed hold worth writing.
#[cfg(feature = "debug")]
fn command_delivered() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        sweep_since_spawn(world).is_some_and(|swept| swept > COMMAND_SWEEP_GUARD_RAD)
            && command_offset_from_spawn(world).is_some_and(|offset| {
                offset > TRACK_TOLERANCE_RAD + COMMAND_RAD_PER_SEC * OFFSET_BEAT_MARGIN_SECS
            })
    })
}

/// The reload gate's beat: a rig root is up and it is NOT the one the reload
/// tore down.
#[cfg(feature = "debug")]
fn rig_reset() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<RigEpoch>(|epoch| epoch.root.is_some() && epoch.root != epoch.replacing)
}

/// Re-read both values at assert time: the delivery guard first (the command
/// is far enough from identity that tracking it means something), then the
/// tolerance. Returns `(swept, error)` for the timeline.
#[cfg(feature = "debug")]
fn assert_tracking(world: &mut World, round: &str) -> (f32, f32) {
    let swept = sweep_since_spawn(world).expect("attitude probe: the rig ship must exist");
    assert!(
        swept > COMMAND_SWEEP_GUARD_RAD,
        "attitude probe ({round}): the command should have swept well away from \
         the attitude this rig spawned at (delivery guard), got {swept:.3} rad"
    );
    let offset = command_offset_from_spawn(world).expect("attitude probe: the rig ship must exist");
    assert!(
        offset > TRACK_TOLERANCE_RAD,
        "attitude probe ({round}): the command is only {offset:.3} rad from the \
         attitude this rig spawned at (need more than {TRACK_TOLERANCE_RAD}); a \
         hull frozen at spawn would clear the tracking tolerance for free"
    );
    let error = tracking_error(world).expect("attitude probe: the rig ship must exist");
    assert!(
        error < TRACK_TOLERANCE_RAD,
        "attitude probe ({round}): hull is {error:.3} rad off the command \
         (tolerance {TRACK_TOLERANCE_RAD}); the PD is not tracking"
    );
    info!(
        "attitude probe: {round} tracks the command ({error:.3} rad lag, \
         command swept {swept:.3} rad since the rig spawned, now {offset:.3} rad \
         off the spawn attitude)"
    );
    (swept, error)
}

/// Round 1 on rig A: invariant 1 (the command really swept away from where the
/// hull spawned) and invariant 2 (the hull is tracking it).
#[cfg(feature = "debug")]
fn assert_rig_a_tracks(world: &mut World) {
    let (swept, error) = assert_tracking(world, "rig a");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: attitude command swept",
        serde_json::json!({ "t": elapsed, "swept_rad": swept }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: attitude tracks",
        serde_json::json!({ "t": elapsed, "error_rad": error }),
    );
}

/// Round 2, same layout: invariant 4 - a FRESH rig, spawned at identity while
/// the command is already most of a turn away, pulls back onto it.
#[cfg(feature = "debug")]
fn assert_reload_tracks(world: &mut World) {
    let (_, error) = assert_tracking(world, "the reload");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: attitude reconverges after reload",
        serde_json::json!({ "t": elapsed, "error_rad": error }),
    );
}

/// Round 3 on layout B: invariant 3 - the same PD gains hold on a different
/// inertia tensor, so round 1 was not a fit to one mass distribution.
#[cfg(feature = "debug")]
fn assert_rig_b_tracks(world: &mut World) {
    let (_, error) = assert_tracking(world, "rig b");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: attitude tracks on rig b",
        serde_json::json!({ "t": elapsed, "error_rad": error }),
    );
}
