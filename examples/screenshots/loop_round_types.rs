//! loop_round_types: synchronized Kinetic and Pierce rounds cross identical
//! layered section targets under the production round-travel rules.

// Only `spawn_comparison_rounds` tints the comparison rounds, and it is
// script-only.
#[cfg(feature = "debug")]
use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_round_types")]
#[command(version = "1.0.0")]
#[command(about = "Capture Kinetic and Pierce travel through layered targets")]
struct Cli;

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "news-0110-round-types";
const KINETIC_X: f32 = -2.5;
const PIERCE_X: f32 = 2.5;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(round_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    let objects = vec![
        layered_target(&sections, "kinetic_target", "Kinetic Target", KINETIC_X),
        layered_target(&sections, "pierce_target", "Pierce Target", PIERCE_X),
    ];
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "Two identical layered targets for the round-type comparison.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                objects
                    .into_iter()
                    .map(EventActionConfig::SpawnScenarioObject)
                    .collect::<Vec<_>>(),
                ThreePointRig::around("round types", Vec3::new(0.0, 0.0, -2.5), 1.5).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "round_type_showcase".to_string(),
            "Round Type Showcase".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

fn layered_target(sections: &GameSections, id: &str, name: &str, x: f32) -> ScenarioObjectConfig {
    let mut specs: Vec<SectionSpec> = (0..6)
        .map(|layer| {
            SectionSpec::new(
                format!("layer_{layer}"),
                LIGHT_HULL_SECTION_ID,
                Vec3::new(0.0, 0.0, -(layer as f32)),
            )
        })
        .collect();
    specs.push(SectionSpec::new(
        "controller",
        BASIC_CONTROLLER_SECTION_ID,
        Vec3::new(0.0, 0.0, -6.0),
    ));
    let mut ship = fixtures::ship(sections, SpaceshipController::None, &specs);
    if let ShipSource::Inline(hull) = &mut ship.hull {
        hull.collapse_threshold = Some(0.0);
    }
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position: Vec3::new(x, 0.0, 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    }
}

#[cfg(feature = "debug")]
fn round_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load both layered targets")
        .enter(GameStates::Loading)
        .until(targets_present())
        .deadline(30.0)
        .add()
        .step("frame the synchronized lanes")
        .on_enter(frame_targets)
        .until(elapsed(0.8))
        .add()
        .step("open the round-type loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .add()
        .step("hold the intact layers")
        .until(frames(20))
        .add()
        .step("fire volley one")
        .on_enter(spawn_comparison_rounds)
        .until(frames(75))
        .add()
        .step("fire volley two")
        .on_enter(spawn_comparison_rounds)
        .until(frames(75))
        .add()
        .step("fire volley three")
        .on_enter(spawn_comparison_rounds)
        .until(frames(75))
        .add()
        .step("fire volley four")
        .on_enter(spawn_comparison_rounds)
        .until(frames(75))
        .add()
        .step("fire volley five")
        .on_enter(spawn_comparison_rounds)
        .until(frames(75))
        .add()
        .step("hold the two outcomes")
        .until(frames(45))
        .add()
        .step("close the round-type loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}

#[cfg(feature = "debug")]
fn targets_present() -> std::sync::Arc<nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<Entity, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).take(2).count() == 2)
    })
}

#[cfg(feature = "debug")]
fn frame_targets(world: &mut World) {
    hide_hud(world);
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(0.2);
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the round showcase has a camera");
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: Vec3::new(6.0, 5.0, 7.0),
        look_at: Vec3::new(0.0, 0.0, -2.5),
    });
}

#[cfg(feature = "debug")]
fn spawn_comparison_rounds(world: &mut World) {
    let mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(0.18).mesh().ico(2).expect("valid round mesh"));
    let kinetic_material = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: tailwind::AMBER_300.into(),
            emissive: LinearRgba::new(8.0, 3.0, 0.2, 1.0),
            ..default()
        });
    let pierce_material = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: tailwind::BLUE_200.into(),
            emissive: LinearRgba::new(1.0, 3.0, 8.0, 1.0),
            ..default()
        });

    let spawn_volley =
        |world: &mut World, x: f32, kind: DamageType, material: Handle<StandardMaterial>| {
            world.spawn((
                Name::new(format!("{kind:?} comparison round")),
                TurretBulletProjectileMarker,
                RoundVelocity(Vec3::new(0.0, 0.0, -100.0)),
                RoundBitten::default(),
                ProjectileDamage::new(
                    if kind == DamageType::Kinetic {
                        100.0
                    } else {
                        20.0
                    },
                    kind,
                ),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(x, 0.0, 2.5),
                TempEntity(8.0),
                Visibility::Visible,
            ));
        };
    spawn_volley(world, KINETIC_X, DamageType::Kinetic, kinetic_material);
    spawn_volley(world, PIERCE_X, DamageType::Pierce, pierce_material);
}
