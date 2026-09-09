//! loop_death_compare: one hull cell killed on a small bare hull - the
//! v0.13.0 half of the news page's section-death split.
//!
//! A controller core with eight hull cells around it in a plate, pinned still
//! under the hero-shot rig with one corner toward the lens. The script holds
//! on the intact plate, kills that corner through the production damage path
//! ([`HealthApplyDamage`], the same event a round delivers), and holds on the
//! aftermath: the flash, the ejecta that outlives it and the light it throws
//! on the cells beside it. The v0.12.0 half is the content-machine capsule
//! scene `death-compare`, the same hull under the same rig from the same
//! lens, killed on the same beat; the split is composed by hand from the two.
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the plate, kill the corner, exit.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also record the death as
//!   `news-0130-death-after.webm` (staged under `NOVA_CAPTURE_DIR`).

#[path = "shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use nova_protocol::prelude::*;

const SUBJECT_ID: &str = "pyre_subject";
/// The cell that dies: the plate's corner nearest the lens.
const KILLED_CELL: &str = "cell_2_2";

/// Where the lens stands and what it looks at, in meters: three-quarter
/// above the killed corner, close enough that the flash's reach across the
/// plate fills the middle half of the frame.
const EYE: Meters3 = Meters3::new(30.0, 24.0, 40.0);
const LOOK: Meters3 = Meters3::new(3.0, 0.0, 3.0);

/// A beat of the intact plate before the hit, so the death lands inside the
/// loop rather than on frame one.
const INTACT_SECS: f32 = 1.5;
/// How long the loop holds on the aftermath: past the flash and into the
/// ejecta's own travel.
const AFTERMATH_SECS: f32 = 5.0;

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new().with_game_plugins(plate_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(death_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn plate_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_plate);
    app.add_systems(Update, pin_plate);
}

fn load_plate(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(plate(&game_assets, &sections)));
}

fn plate(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let subject = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: SUBJECT_ID.to_string(),
            name: "Plate".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            hull: ShipSource::Inline(ShipHull {
                sections: plate_sections(sections),
                collapse_threshold: Some(0.0),
                ..default()
            }),
            ..default()
        }),
    };
    ScenarioConfig {
        description: "A bare plate of hull cells, one corner about to die".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: std::iter::once(EventActionConfig::SpawnScenarioObject(subject))
                .chain(ThreePointRig::around("pyre", Meters3::ZERO, 1.0).actions())
                .collect(),
        }],
        ..ScenarioConfig::new(
            "loop_death_compare".to_string(),
            "Section Death".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The controller core at the centre and a hull cell on every other cell of a
/// 3x3 plate, named by column and row so the corner has a name.
fn plate_sections(sections: &GameSections) -> Vec<SpaceshipSectionConfig> {
    let mut hull = vec![section(
        sections,
        "core",
        "basic_controller_section",
        Vec3::ZERO,
    )];
    for x in -1..=1 {
        for z in -1..=1 {
            if x == 0 && z == 0 {
                continue;
            }
            hull.push(section(
                sections,
                &format!("cell_{}_{}", x + 1, z + 1),
                "light_hull_section",
                Vec3::new(x as f32, 0.0, z as f32),
            ));
        }
    }
    hull
}

/// One catalog section inline at a build-grid cell.
fn section(sections: &GameSections, id: &str, kind: &str, cell: Vec3) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id: id.to_string(),
        position: cell,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(
            sections
                .get_section(kind)
                .unwrap_or_else(|| panic!("loop_death_compare: no section '{kind}' in the catalog"))
                .clone(),
        ),
        modifications: vec![],
    }
}

/// Pin the plate still the frame it spawns: the death is the only thing that
/// moves in this take.
fn pin_plate(mut commands: Commands, roots: Query<Entity, Added<SpaceshipRootMarker>>) {
    for root in &roots {
        commands
            .entity(root)
            .insert(avian3d::prelude::RigidBody::Static);
    }
}

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "news-0130-death-after";

/// The plate is in the world.
#[cfg(feature = "debug")]
fn plate_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut ships| ships.iter(world).any(|id| id.0 == SUBJECT_ID))
    })
}

/// The scenario is built: its spawn queue has drained and its art is in. The
/// spawn queue lands over several frames and the rig's lights come after the
/// scene, so a loop opened on the first cell would open unlit.
#[cfg(feature = "debug")]
fn settled() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let queue_drained = world
            .get_resource::<NovaEventWorld>()
            .is_some_and(|scenario| !scenario.is_settling());
        let art_in = !world
            .get_resource::<ScenarioPreload>()
            .is_some_and(|preload| preload.is_pending());
        queue_drained && art_in
    })
}

#[cfg(feature = "debug")]
fn frame_plate(world: &mut World) {
    hide_hud(world);
    pose_camera(world, EYE, LOOK);
}

/// Put the corner down through the production damage path.
#[cfg(feature = "debug")]
fn kill_corner(world: &mut World) {
    let Some(node) = kit::section_health(world, SUBJECT_ID, KILLED_CELL) else {
        panic!("loop_death_compare: no health node under '{KILLED_CELL}'");
    };
    world.trigger(HealthApplyDamage {
        entity: node,
        source: None,
        amount: 1.0e6,
    });
    info!("loop_death_compare: killed '{KILLED_CELL}'");
}

#[cfg(feature = "debug")]
fn death_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the plate")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            and(plate_present(), settled()),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the corner")
        .on_enter(frame_plate)
        .until(elapsed(0.5))
        .add()
        .step("open the loop on the intact plate")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .until(elapsed(INTACT_SECS))
        .add()
        .step("kill the corner and hold on the aftermath")
        .on_enter(kill_corner)
        .until(elapsed(AFTERMATH_SECS))
        .add()
        .step("close the loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}
