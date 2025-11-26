/// Scenario loader plugin and related types
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_common_systems::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        CurrentScenario, GameScenarios, LoadScenario, ScenarioConfig, ScenarioEventConfig,
        ScenarioId, ScenarioLoaded, ScenarioLoaderPlugin, ScenarioScopedMarker, UnloadScenario,
    };
}

/// Type alias for Scenario ID
pub type ScenarioId = String;

/// The collection of available game scenarios
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameScenarios(pub HashMap<ScenarioId, ScenarioConfig>);

/// Configuration for a game scenario
#[derive(Clone, Debug)]
pub struct ScenarioConfig {
    /// Unique identifier for the scenario
    pub id: ScenarioId,
    /// The display name of the scenario
    pub name: String,
    /// A brief description of the scenario
    pub description: String,
    /// The cubemap image used for the scenario's skybox
    pub cubemap: Handle<Image>,
    /// Events associated with the scenario
    pub events: Vec<ScenarioEventConfig>,
}

/// Configuration for a scenario event
#[derive(Clone, Debug)]
pub struct ScenarioEventConfig {
    /// The name of the event to listen for
    pub name: EventConfig,
    /// Filters to apply to the event
    pub filters: Vec<EventFilterConfig>,
    /// Actions to perform when the event is triggered
    pub actions: Vec<EventActionConfig>,
}

/// Load a scenario given the configuration (this can be read from the GameScenarios resource).
/// e.g we could display all the scenario names in a menu and load the selected one.
#[derive(Event, Clone, Debug, Deref, DerefMut)]
pub struct LoadScenario(pub ScenarioConfig);

/// Unload the current scenario. This event guarantees that all scenario-scoped entities are
/// removed from the world.
#[derive(Event, Clone, Debug)]
pub struct UnloadScenario;

/// Event that is triggered once a scenario has been successfully loaded.
#[derive(Event, Clone, Debug)]
pub struct ScenarioLoaded;

/// The current loaded scenario, if any. This will contain the scenario configuration.
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct CurrentScenario(pub Option<ScenarioConfig>);

/// Marker that indicates that an entity is scoped to the current scenario.
/// When a scenario is unloaded, all entities with this marker will be despawned.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ScenarioScopedMarker;

pub struct ScenarioLoaderPlugin;

impl Plugin for ScenarioLoaderPlugin {
    fn build(&self, app: &mut App) {
        debug!("ScenarioLoaderPlugin: build");

        app.add_observer(on_player_spaceship_spawned);
        app.add_observer(on_player_spaceship_destroyed);

        app.init_resource::<CurrentScenario>();
        app.add_observer(on_load_scenario);

        app.add_observer(on_add_entity_with::<MeshFragmentMarker>);
        app.add_observer(on_add_entity_with::<TurretBulletProjectileMarker>);
        app.add_observer(on_add_entity_with::<TorpedoProjectileMarker>);

        app.add_input_context::<ScenarioInputMarker>();
        app.add_observer(on_next_input);
        app.add_observer(unload_scenario);
    }
}

fn unload_scenario(
    _: On<UnloadScenario>,
    mut commands: Commands,
    q_scoped: Query<Entity, With<ScenarioScopedMarker>>,
    mut current_scenario: ResMut<CurrentScenario>,
    mut world: ResMut<NovaEventWorld>,
) {
    world.clear();
    **current_scenario = None;
    for entity in q_scoped.iter() {
        commands.entity(entity).despawn();
    }
}

fn on_load_scenario(
    load: On<LoadScenario>,
    mut commands: Commands,
    mut current_scenario: ResMut<CurrentScenario>,
    q_scoped: Query<Entity, With<ScenarioScopedMarker>>,
    mut world: ResMut<NovaEventWorld>,
) {
    world.clear();
    for entity in q_scoped.iter() {
        commands.entity(entity).despawn();
    }

    let scenario = (**load).clone();
    **current_scenario = Some(scenario.clone());
    debug!("on_load_scenario: scenario {:?}", scenario.name);

    // Setup Scenario Camera
    commands.spawn((
        ScenarioScopedMarker,
        ScenarioCameraMarker,
        Name::new("Scenario Camera"),
        Camera3d::default(),
        WASDCameraController,
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        SkyboxConfig {
            cubemap: scenario.cubemap.clone(),
            brightness: 1000.0,
        },
    ));

    // Setup directional light
    commands.spawn((
        ScenarioScopedMarker,
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_2,
            0.0,
            0.0,
        )),
        GlobalTransform::default(),
    ));

    // Setup scenario input context
    commands.spawn((
        ScenarioScopedMarker,
        Name::new(format!("Scenario Input Context: {}", scenario.name)),
        ScenarioInputMarker,
        actions!(
            ScenarioInputMarker[(
                Name::new("Input: Next Scenario"),
                Action::<NextScenarioInput>::new(),
                bindings![KeyCode::Enter, GamepadButton::South]
            )]
        ),
    ));

    // Setup scenario events
    for event in scenario.events.iter() {
        let mut event_handler = EventHandler::<NovaEventWorld>::from(event.name);
        for filter in event.filters.iter() {
            event_handler.add_filter(filter.clone());
        }
        for action in event.actions.iter() {
            event_handler.add_action(action.clone());
        }
        commands.spawn((
            ScenarioScopedMarker,
            Name::new(format!("Event Handler: {:?}", event.name)),
            event_handler,
        ));
    }

    // Trigger ScenarioLoaded event
    commands.trigger(ScenarioLoaded);

    // Fire onstart event
    commands.fire::<OnStartEvent>(OnStartEventInfo);
}

fn on_add_entity_with<T: Component>(
    add: On<Add, T>,
    mut commands: Commands,
    current_scenario: Res<CurrentScenario>,
) {
    if let Some(scenario) = &**current_scenario {
        trace!(
            "on_add_entity_with: Added entity {:?} in scenario {:?}",
            add.entity,
            scenario.name
        );

        commands.entity(add.entity).insert(ScenarioScopedMarker);
    }
}

#[derive(Component, Debug, Clone)]
struct ScenarioInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
struct NextScenarioInput;

fn on_next_input(_: On<Start<NextScenarioInput>>, mut world: ResMut<super::world::NovaEventWorld>) {
    let Some(mut next_scenario) = world.next_scenario.clone() else {
        return;
    };

    next_scenario.linger = false;
    world.next_scenario = Some(next_scenario);
}

#[derive(Component, Debug, Clone)]
struct ScenarioCameraMarker;

fn on_player_spaceship_spawned(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    camera: Single<Entity, With<ScenarioCameraMarker>>,
) {
    trace!("on_player_spaceship_spawned: {:?}", add.entity);

    let camera = camera.into_inner();

    commands
        .entity(camera)
        .remove::<WASDCameraController>()
        .insert(SpaceshipCameraController);
}

fn on_player_spaceship_destroyed(
    add: On<Add, HealthZeroMarker>,
    mut commands: Commands,
    camera: Single<Entity, With<SpaceshipCameraController>>,
    spaceship: Single<Entity, With<PlayerSpaceshipMarker>>,
) {
    trace!("on_player_spaceship_destroyed: {:?}", add.entity);
    if add.entity != spaceship.into_inner() {
        return;
    }

    let camera = camera.into_inner();

    commands
        .entity(camera)
        .remove::<SpaceshipCameraController>()
        .insert(WASDCameraController);
}
