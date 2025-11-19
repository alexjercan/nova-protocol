// TODO: move this to `bevy_common_systems` as a small game

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
use rand::prelude::*;

#[derive(Parser)]
#[command(name = "07b_slicer")]
#[command(version = "1.0.0")]
#[command(about = "A simple example showing how to play with the mesh slicer in nova_protocol", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStates::Playing), setup_scenario);

    app.add_observer(on_click_damage_health);
    // app.add_observer(on_fragment_added);
}

fn setup_scenario(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(test_scenario(&game_assets)));
}

fn on_click_damage_health(
    click: On<Pointer<Press>>,
    mut commands: Commands,
    q_health: Query<&Health>,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let entity = click.entity;

    if q_health.get(entity).is_ok() {
        commands.trigger(HealthApplyDamage {
            target: click.entity,
            source: None,
            amount: 100.0,
        });
    }
}

// fn on_fragment_added(
//     add: On<Add, FragmentMeshMarker>,
//     mut commands: Commands,
//     mut materials: ResMut<Assets<StandardMaterial>>,
// ) {
//     commands.entity(add.entity).insert((
//         ExplodableEntityMarker,
//         MeshMaterial3d(materials.add(Color::srgb(rand::random(), rand::random(), rand::random()))),
//         Health::new(10.0),
//     ));
// }

pub fn test_scenario(game_assets: &GameAssets) -> ScenarioConfig {
    let mut rng = rand::rng();

    let mut objects = Vec::new();
    for i in 0..20 {
        let pos = Vec3::new(
            rng.random_range(-100.0..100.0),
            rng.random_range(-20.0..20.0),
            rng.random_range(-100.0..100.0),
        );
        let radius = rng.random_range(1.0..3.0);
        let texture = game_assets.asteroid_texture.clone();

        objects.push(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: format!("asteroid_{}", i),
                name: format!("Asteroid {}", i),
                position: pos,
                rotation: Quat::IDENTITY,
                health: 100.0,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig { radius, texture }),
        });
    }

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .collect::<_>(),
    }];

    ScenarioConfig {
        id: "test_scenario".to_string(),
        name: "Test Scenario".to_string(),
        description: "A test scenario.".to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}
