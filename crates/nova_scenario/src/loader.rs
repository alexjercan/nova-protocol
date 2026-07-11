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

/// Event that is triggered once a scenario has been successfully loaded. Carries a snapshot of
/// the loaded scenario's init status so consumers (e.g. the autopilot/screenshot smoke harness)
/// can assert on it and so scenario init is easier to debug.
#[derive(Event, Clone, Debug)]
pub struct ScenarioLoaded {
    /// The id of the scenario that was loaded.
    pub scenario_id: ScenarioId,
    /// The number of event handlers registered for the scenario (one per `ScenarioEventConfig`).
    pub handler_count: usize,
    /// The number of scenario objects the scenario will spawn, counted from the
    /// `SpawnScenarioObject` actions across all of its events.
    pub object_count: usize,
}

impl ScenarioLoaded {
    /// Build the load-status snapshot from a scenario config. The counts come straight from the
    /// config: one handler per event, and one object per `SpawnScenarioObject` action.
    fn from_config(scenario: &ScenarioConfig) -> Self {
        let object_count = scenario
            .events
            .iter()
            .flat_map(|event| event.actions.iter())
            .filter(|action| matches!(action, EventActionConfig::SpawnScenarioObject(_)))
            .count();
        Self {
            scenario_id: scenario.id.clone(),
            handler_count: scenario.events.len(),
            object_count,
        }
    }
}

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

/// Tear down the currently-loaded scenario: clear the event world and despawn every
/// scenario-scoped entity (despawn is recursive, so their children go too). Shared by
/// both the unload path and the load path (which tears the old scenario down before
/// spawning the next), so teardown is identical no matter how a scenario ends.
fn teardown_scenario_entities(
    commands: &mut Commands,
    q_scoped: &Query<Entity, With<ScenarioScopedMarker>>,
    world: &mut NovaEventWorld,
) {
    world.clear();
    for entity in q_scoped.iter() {
        commands.entity(entity).despawn();
    }
}

fn unload_scenario(
    _: On<UnloadScenario>,
    mut commands: Commands,
    q_scoped: Query<Entity, With<ScenarioScopedMarker>>,
    mut current_scenario: ResMut<CurrentScenario>,
    mut world: ResMut<NovaEventWorld>,
) {
    teardown_scenario_entities(&mut commands, &q_scoped, &mut world);
    **current_scenario = None;
}

fn on_load_scenario(
    load: On<LoadScenario>,
    mut commands: Commands,
    mut current_scenario: ResMut<CurrentScenario>,
    q_scoped: Query<Entity, With<ScenarioScopedMarker>>,
    mut world: ResMut<NovaEventWorld>,
) {
    teardown_scenario_entities(&mut commands, &q_scoped, &mut world);

    let scenario = (**load).clone();
    **current_scenario = Some(scenario.clone());
    debug!("on_load_scenario: scenario {:?}", scenario.name);

    // Setup Scenario Camera. `SfxListenerMarker` makes this the explicit
    // SFX/juice listener (attenuation, camera shake, flash facing); the editor
    // camera deliberately never carries it.
    commands.spawn((
        ScenarioScopedMarker,
        ScenarioCameraMarker,
        SfxListenerMarker,
        Name::new("Scenario Camera"),
        Camera3d::default(),
        PostProcessingCamera,
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

    // Trigger ScenarioLoaded event with a snapshot of the init status.
    let loaded = ScenarioLoaded::from_config(&scenario);
    debug!(
        "on_load_scenario: loaded scenario '{}' with {} handler(s) and {} object(s)",
        loaded.scenario_id, loaded.handler_count, loaded.object_count
    );
    commands.trigger(loaded);

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

fn on_next_input(
    _: On<Start<NextScenarioInput>>,
    mut world: ResMut<super::world::NovaEventWorld>,
    pause: Res<State<PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == PauseStates::Paused {
        return;
    }

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
    _remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    camera: Single<Entity, With<SpaceshipCameraController>>,
) {
    trace!("on_player_spaceship_destroyed: switching camera back to WASD");

    let camera = camera.into_inner();

    commands
        .entity(camera)
        .remove::<SpaceshipCameraController>()
        .insert(WASDCameraController);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_object_action() -> EventActionConfig {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "obj".to_string(),
                name: "Obj".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius: 1.0,
                texture: Handle::default(),
                health: 1.0,
                surface_gravity: None,
            }),
        })
    }

    fn event_with(actions: Vec<EventActionConfig>) -> ScenarioEventConfig {
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions,
        }
    }

    fn scenario_with(id: &str, events: Vec<ScenarioEventConfig>) -> ScenarioConfig {
        ScenarioConfig {
            id: id.to_string(),
            name: "Test Scenario".to_string(),
            description: "For tests".to_string(),
            cubemap: Handle::default(),
            events,
        }
    }

    #[test]
    fn snapshot_reports_id_and_handler_count() {
        // One ScenarioLoaded handler_count per event, regardless of the actions inside.
        let scenario = scenario_with(
            "asteroid_field",
            vec![event_with(vec![]), event_with(vec![]), event_with(vec![])],
        );

        let loaded = ScenarioLoaded::from_config(&scenario);

        assert_eq!(loaded.scenario_id, "asteroid_field");
        assert_eq!(loaded.handler_count, 3);
    }

    #[test]
    fn snapshot_counts_spawn_object_actions_across_events() {
        // object_count counts SpawnScenarioObject actions everywhere, and ignores other
        // action kinds (here a bare DebugMessage-free event and a mixed one).
        let scenario = scenario_with(
            "mixed",
            vec![
                event_with(vec![spawn_object_action(), spawn_object_action()]),
                event_with(vec![]),
                event_with(vec![spawn_object_action()]),
            ],
        );

        let loaded = ScenarioLoaded::from_config(&scenario);

        assert_eq!(loaded.handler_count, 3);
        assert_eq!(loaded.object_count, 3);
    }

    #[test]
    fn empty_scenario_reports_zero_counts() {
        let loaded = ScenarioLoaded::from_config(&scenario_with("empty", vec![]));

        assert_eq!(loaded.scenario_id, "empty");
        assert_eq!(loaded.handler_count, 0);
        assert_eq!(loaded.object_count, 0);
    }
}
