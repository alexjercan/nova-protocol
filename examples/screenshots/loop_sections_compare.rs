//! loop_sections_compare: the four remodelled sections on one bench, turning
//! together - the v0.13.0 half of the news page's sections split.
//!
//! The hull cell, the controller core, the PDC mount and the torpedo bay stand
//! as four single-section hulls in a 2x2 grid under the hero-shot light rig,
//! bare, and yaw once around in the loop's length so the loop closes on
//! itself. The v0.12.0 half is the content-machine capsule scene
//! `sections-compare`, which stages the same grid under the same rig from the
//! same lens on the same turn; the split is composed by hand from the two.
//!
//! Every subject is a kinematic body whose yaw is written from the clock, so
//! the turn is the same on both sides of the split to the frame and owes
//! nothing to either release's physics.
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the bench, turn it, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also record the turn as
//!   `news-0130-sections-after.webm` (staged under `NOVA_CAPTURE_DIR`).

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// The four subjects: scenario id, catalog section, and where each stands on
/// the bench, in meters. A 2x2 grid rather than a row, because the split takes
/// the middle half of the frame: the window is nearly square.
const BENCH: [(&str, &str, Meters3); 4] = [
    (
        "bench_hull",
        "light_hull_section",
        Meters3::new(-12.0, 8.0, 0.0),
    ),
    (
        "bench_core",
        "basic_controller_section",
        Meters3::new(12.0, 8.0, 0.0),
    ),
    (
        "bench_pdc",
        "pdc_kinetic_turret_section",
        Meters3::new(-12.0, -10.0, 0.0),
    ),
    (
        "bench_bay",
        "torpedo_section",
        Meters3::new(12.0, -10.0, 0.0),
    ),
];

/// Where the lens stands and what it looks at, in meters. From 58 m the
/// middle half of a 45-degree frame spans about 43 m; the grid spans 34.
const EYE: Meters3 = Meters3::new(0.0, 4.0, 58.0);
const LOOK: Meters3 = Meters3::new(0.0, -1.0, 0.0);

/// How long the loop runs, and so how long one turn takes: the bench yaws
/// exactly once inside the loop, so the last frame hands back to the first.
const LOOP_SECS: f32 = 8.0;
/// The yaw every subject stands at when the turn starts. The working face of
/// a section is -Z and the lens stands at +Z, so a half turn plus a quarter
/// angle opens the turn on the muzzle face and a flank.
const START_YAW: f32 = std::f32::consts::PI - 0.55;

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new().with_game_plugins(bench_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(bench_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn bench_plugin(app: &mut App) {
    app.init_resource::<TurnClock>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_bench);
    app.add_systems(Update, (pin_bench, turn_bench));
}

/// The sim second the turn started on; `None` until the script starts it.
#[derive(Resource, Default)]
struct TurnClock(Option<f32>);

fn load_bench(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(bench(&game_assets, &sections)));
}

fn bench(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    ScenarioConfig {
        description: "Four sections on one bench under one light".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: BENCH
                .iter()
                .map(|(id, section, at)| {
                    EventActionConfig::SpawnScenarioObject(subject(sections, id, section, *at))
                })
                .chain(ThreePointRig::around("bench", Meters3::ZERO, 1.0).actions())
                .collect(),
        }],
        ..ScenarioConfig::new(
            "loop_sections_compare".to_string(),
            "Sections Bench".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// One section as a hull of its own, bare: no skin, so what stands on the
/// bench is the section's own art and nothing closed over it.
fn subject(sections: &GameSections, id: &str, section: &str, at: Meters3) -> ScenarioObjectConfig {
    let config = sections
        .get_section(section)
        .unwrap_or_else(|| panic!("loop_sections_compare: no section '{section}' in the catalog"))
        .clone();
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: id.replace('_', " "),
            position: at,
            rotation: Quat::from_rotation_y(START_YAW),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            hull: ShipSource::Inline(ShipHull {
                sections: vec![SpaceshipSectionConfig {
                    id: "subject".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(config),
                    modifications: vec![],
                }],
                collapse_threshold: Some(0.0),
                ..default()
            }),
            ..default()
        }),
    }
}

/// Pin every bench hull as a kinematic body the frame it spawns: nothing
/// pushes it, and its yaw is written by [`turn_bench`] rather than integrated.
fn pin_bench(mut commands: Commands, roots: Query<Entity, Added<SpaceshipRootMarker>>) {
    for root in &roots {
        commands
            .entity(root)
            .insert(avian3d::prelude::RigidBody::Kinematic);
    }
}

/// Yaw every subject from the clock: one full turn per [`LOOP_SECS`], from
/// [`START_YAW`] at the second the turn started.
fn turn_bench(
    time: Res<Time>,
    clock: Res<TurnClock>,
    mut roots: Query<&mut avian3d::prelude::Rotation, With<SpaceshipRootMarker>>,
) {
    let Some(started) = clock.0 else {
        return;
    };
    let yaw = START_YAW + (time.elapsed_secs() - started) * std::f32::consts::TAU / LOOP_SECS;
    for mut rotation in &mut roots {
        rotation.0 = Quat::from_rotation_y(yaw);
    }
}

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "news-0130-sections-after";

/// Every subject is in the world.
#[cfg(feature = "debug")]
fn bench_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut ships| ships.iter(world).count() == BENCH.len())
    })
}

#[cfg(feature = "debug")]
fn frame_bench(world: &mut World) {
    hide_hud(world);
    pose_camera(world, EYE, LOOK);
}

/// Start the turn and open the loop on the same frame, so the loop's first
/// frame is the turn's first frame.
#[cfg(feature = "debug")]
fn open_turn(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs();
    world.resource_mut::<TurnClock>().0 = Some(now);
    loop_start(world, LOOP_NAME);
}

#[cfg(feature = "debug")]
fn bench_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the bench")
        .enter(GameStates::Loading)
        .until(and(state_is(GameStates::Playing), bench_present()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the bench")
        .on_enter(frame_bench)
        .until(elapsed(0.5))
        .add()
        .step("open the loop and turn the bench once")
        .on_enter(open_turn)
        .until(elapsed(LOOP_SECS))
        .add()
        .step("close the loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}
