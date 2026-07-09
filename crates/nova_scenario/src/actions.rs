use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use nova_events::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        base_scenario_object, BaseScenarioObjectConfig, DebugMessageActionConfig,
        EventActionConfig, NextScenarioActionConfig, ObjectiveActionConfig,
        ObjectiveCompleteActionConfig, ScenarioAreaConfig, ScenarioObjectConfig,
        ScenarioObjectKind, VariableSetActionConfig,
    };
}

#[derive(Clone, Debug)]
pub enum EventActionConfig {
    DebugMessage(DebugMessageActionConfig),
    VariableSet(VariableSetActionConfig),
    Objective(ObjectiveActionConfig),
    ObjectiveComplete(ObjectiveCompleteActionConfig),
    SpawnScenarioObject(ScenarioObjectConfig),
    CreateScenarioArea(ScenarioAreaConfig),
    NextScenario(NextScenarioActionConfig),
}

impl EventAction<NovaEventWorld> for EventActionConfig {
    fn action(&self, world: &mut NovaEventWorld, info: &GameEventInfo) {
        match self {
            EventActionConfig::DebugMessage(config) => {
                config.action(world, info);
            }
            EventActionConfig::VariableSet(config) => {
                config.action(world, info);
            }
            EventActionConfig::Objective(config) => {
                config.action(world, info);
            }
            EventActionConfig::ObjectiveComplete(config) => {
                config.action(world, info);
            }
            EventActionConfig::SpawnScenarioObject(config) => {
                config.action(world, info);
            }
            EventActionConfig::CreateScenarioArea(config) => {
                config.action(world, info);
            }
            EventActionConfig::NextScenario(config) => {
                config.action(world, info);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct VariableSetActionConfig {
    pub key: String,
    pub expression: VariableExpressionNode,
}

impl EventAction<NovaEventWorld> for VariableSetActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        match self.expression.evaluate(world) {
            Ok(literal) => {
                world.insert_variable(self.key.clone(), literal);
            }
            Err(e) => {
                error!(
                    "VariableSetActionConfig: failed to evaluate expression for key '{}': {:?}",
                    self.key, e
                );
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct DebugMessageActionConfig {
    pub message: String,
}

impl EventAction<NovaEventWorld> for DebugMessageActionConfig {
    fn action(&self, _: &mut NovaEventWorld, _: &GameEventInfo) {
        debug!("Event Action Message: {}", self.message);
    }
}

#[derive(Clone, Debug, Default)]
pub struct NextScenarioActionConfig {
    pub scenario_id: String,
    pub linger: bool,
}

impl EventAction<NovaEventWorld> for NextScenarioActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        debug!(
            "NextScenario: queuing scenario '{}' (linger: {})",
            self.scenario_id, self.linger
        );
        world.next_scenario = Some(self.clone());
    }
}

/// A scenario action that adds an objective to the HUD.
///
/// The objective *data* (id + message) is the generic `bevy_common_systems` `Objective`, but
/// this scenario-action wrapper stays nova-local because it implements the (foreign)
/// `EventAction` trait - which the orphan rule forbids implementing on the foreign
/// `Objective` type directly.
#[derive(Clone, Debug)]
pub struct ObjectiveActionConfig {
    /// Opaque identifier, used to complete/remove the objective later.
    pub id: String,
    /// The text shown in the objectives HUD.
    pub message: String,
}

impl ObjectiveActionConfig {
    /// Construct from string slices.
    pub fn new(id: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            message: message.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ObjectiveActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.push_objective(self.clone());
    }
}

#[derive(Clone, Debug)]
pub struct ObjectiveCompleteActionConfig {
    pub id: String,
}

impl EventAction<NovaEventWorld> for ObjectiveCompleteActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.remove_objective(&self.id);
    }
}

#[derive(Clone, Debug)]
pub struct ScenarioObjectConfig {
    pub base: BaseScenarioObjectConfig,
    pub kind: ScenarioObjectKind,
}

#[derive(Clone, Debug)]
pub struct BaseScenarioObjectConfig {
    pub id: String,
    pub name: String,
    pub position: Vec3,
    pub rotation: Quat,
}

pub fn base_scenario_object(config: &BaseScenarioObjectConfig) -> impl Bundle {
    (
        ScenarioScopedMarker,
        Name::new(config.name.clone()),
        EntityId::new(config.id.clone()),
        Transform::from_translation(config.position).with_rotation(config.rotation),
        RigidBody::Dynamic,
        // Physics advances Transform only on fixed ticks (64 Hz by
        // default); everything
        // watched by the render-rate camera must interpolate between them or
        // it stair-steps. Invisible while the chase camera was bolted rigidly
        // to the ship (both stepped together), but the camera smoothing from
        // the flight-feel retune eases at render rate and exposed the steps
        // as twitch (task 20260709-160753).
        TransformInterpolation,
        Visibility::Visible,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The behavior the component buys (task 20260709-160753): a moving
    /// scenario body's Transform advances on EVERY render frame, not just on
    /// fixed physics ticks. 4 ms frames against the 15.6 ms tick mean at
    /// most one tick lands inside any 3-frame span - without easing at
    /// least two consecutive frames would show identical translations.
    #[test]
    fn scenario_bodies_move_between_fixed_ticks() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        // Mirrors the integrity physics harness: MeshPlugin because avian's
        // collider-from-mesh backend reads AssetEvent<Mesh> at startup.
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.004,
        )));
        app.finish();

        let body = app
            .world_mut()
            .spawn((
                base_scenario_object(&BaseScenarioObjectConfig {
                    id: "mover".to_string(),
                    name: "Mover".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                }),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
                LinearVelocity(Vec3::X * 10.0),
            ))
            .id();

        // Warm up past two fixed ticks so the easing has start+end states.
        for _ in 0..10 {
            app.update();
        }

        // Four consecutive 4 ms frames: with easing every frame advances the
        // translation; stair-stepping would repeat a value.
        let mut positions = Vec::new();
        for _ in 0..4 {
            app.update();
            positions.push(app.world().get::<Transform>(body).unwrap().translation.x);
        }
        for pair in positions.windows(2) {
            assert!(
                pair[1] > pair[0],
                "translation must advance every render frame, got {positions:?}"
            );
        }
    }

    /// Every dynamic scenario body must interpolate its Transform between
    /// fixed physics ticks, or it stair-steps under the smoothed chase
    /// camera (task 20260709-160753).
    #[test]
    fn scenario_objects_interpolate_their_transforms() {
        let mut world = World::new();
        let entity = world
            .spawn(base_scenario_object(&BaseScenarioObjectConfig {
                id: "test".to_string(),
                name: "Test".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            }))
            .id();
        assert!(world.get::<TransformInterpolation>(entity).is_some());
    }
}

#[derive(Clone, Debug)]
pub enum ScenarioObjectKind {
    Asteroid(AsteroidConfig),
    Spaceship(SpaceshipConfig),
}

impl EventAction<NovaEventWorld> for ScenarioObjectConfig {
    fn action(&self, world: &mut NovaEventWorld, _info: &GameEventInfo) {
        let config = self.clone();
        debug!("SpawnScenarioObject: spawning '{}'", config.base.id);

        world.push_command(move |commands| {
            let mut entity_commands = commands.spawn(base_scenario_object(&config.base));

            match &config.kind {
                ScenarioObjectKind::Asteroid(config) => {
                    entity_commands.insert(asteroid_scenario_object(config.clone()));
                }
                ScenarioObjectKind::Spaceship(config) => {
                    entity_commands.insert(spaceship_scenario_object(config.clone()));
                }
            }
        });
    }
}

#[derive(Clone, Debug)]
pub struct ScenarioAreaConfig {
    pub id: String,
    pub name: String,
    pub position: Vec3,
    pub rotation: Quat,
    pub radius: f32,
}

impl EventAction<NovaEventWorld> for ScenarioAreaConfig {
    fn action(&self, world: &mut NovaEventWorld, _info: &GameEventInfo) {
        let config = self.clone();
        debug!(
            "CreateScenarioArea: creating area '{}' (radius: {})",
            config.id, config.radius
        );

        world.push_command(move |commands| {
            commands.spawn((
                ScenarioScopedMarker,
                ScenarioAreaMarker,
                Name::new(config.name.clone()),
                EntityId::new(config.id.clone()),
                Transform::from_translation(config.position).with_rotation(config.rotation),
                RigidBody::Static,
                Collider::sphere(config.radius),
                Sensor,
                Visibility::Visible,
            ));
        });
    }
}
