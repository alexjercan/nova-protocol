//! controller_section: the controller section's PD attitude control.
//!
//! One minimal ship (controller + hull, no player input) chases a slowly
//! rotating attitude command written straight into
//! [`ControllerSectionRotationInput`] - the same seam the mouse, the AI and
//! the autopilot write. Watch the hull swing to track the command arrow;
//! the section under test is the PD that turns "desired rotation" into
//! torque.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example controller_section --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `attitude probe: hull tracks the command`,
//! #           `autopilot: cycle complete, no panic`
//! ```

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

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: the probe asserts the section's whole
    // point - the hull actually converges on the commanded attitude.
    #[cfg(feature = "debug")]
    {
        app.init_resource::<AttitudeProbe>();
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PERF_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        app.add_plugins(nova_probe::nova_frametime());
        app.add_plugins(nova_autopilot().input(autopilot_attitude_probe));
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_rig);
    app.add_systems(Update, (drive_command, draw_command_arrow));
}

fn setup_rig(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(attitude_rig(&game_assets, &sections)));
}

/// The rig scenario: one sectioned ship with no player and no AI - rotation
/// authority belongs to this example's command writer alone.
fn attitude_rig(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    let ship = SpaceshipConfig {
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
        ],
    };

    ScenarioConfig {
        id: "controller_rig".to_string(),
        name: "Controller Section Rig".to_string(),
        description: "A minimal ship chasing a rotating attitude command.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![EventActionConfig::SpawnScenarioObject(
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
        }],
        ..Default::default()
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

/// Stage tracker for the attitude probe.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct AttitudeProbe {
    first_playing_at: Option<f32>,
    asserted: bool,
}

/// Tracking error tolerance, radians. A healthy PD (frequency 4, damping 4)
/// tracks the slow sweep with a fraction of this; a dead or misfiring PD
/// falls behind by the full swept angle. Well above the f32 quat noise
/// floor (~1e-3 rad).
#[cfg(feature = "debug")]
const TRACK_TOLERANCE_RAD: f32 = 0.35;

/// Autopilot script: after the PD has had a couple of seconds to chase,
/// assert the hull's attitude tracks the command. Delivery guard: the
/// command must have swept at least a radian by then, so a hull frozen at
/// spawn cannot pass.
#[cfg(feature = "debug")]
fn autopilot_attitude_probe(world: &mut World, elapsed: f32) {
    // Backstop before the state gate: if the window is about to close and
    // the probe never completed (loading ate the window, a stage stalled),
    // fail loudly instead of vacuously passing.
    if elapsed > nova_protocol::nova_debug::harness::NOVA_AUTOPILOT_SECS - 0.3
        && !world.resource::<AttitudeProbe>().asserted
    {
        panic!("attitude probe: never completed within the autopilot window");
    }
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    let probe = world.resource::<AttitudeProbe>();
    if probe.asserted {
        return;
    }
    let Some(first) = probe.first_playing_at else {
        world.resource_mut::<AttitudeProbe>().first_playing_at = Some(elapsed);
        return;
    };
    if elapsed < first + 2.5 {
        return;
    }

    let command = command_at(world.resource::<Time>().elapsed_secs());
    let swept = command.angle_between(Quat::IDENTITY);
    assert!(
        swept > 0.6,
        "attitude probe: the command should have swept well away from \
         identity by now (delivery guard), got {swept:.3} rad"
    );

    let rotation = {
        let mut q = world.query_filtered::<&Rotation, With<SpaceshipRootMarker>>();
        q.single(world)
            .expect("attitude probe: the rig ship must exist")
            .0
    };
    let error = rotation.angle_between(command);
    assert!(
        error < TRACK_TOLERANCE_RAD,
        "attitude probe: hull is {error:.3} rad off the command \
         (tolerance {TRACK_TOLERANCE_RAD}); the PD is not tracking"
    );
    info!("attitude probe: hull tracks the command ({error:.3} rad lag)");
    world.resource_mut::<AttitudeProbe>().asserted = true;
    // Timeline beat (task 20260719-210450): the PD tracked, with the lag
    // on the record.
    nova_probe::probe_marker(
        world,
        "outcome: attitude tracks",
        serde_json::json!({ "t": elapsed, "error_rad": error }),
    );
}
