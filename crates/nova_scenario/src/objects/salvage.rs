//! Salvage crate scenario object (task 20260712-093044, spike
//! docs/spikes/20260712-092926-starter-scenario.md): a minimal proximity
//! pickup. The crate is a small tumbling prop that doubles as its own
//! trigger area - flying through it fires `OnEnter` under the crate's
//! scenario id, and the scenario script pairs that with
//! `DespawnScenarioObject` plus whatever counting it wants. No inventory:
//! "collected" is scenario state, which keeps pickup consequences in
//! scenario data instead of a hardcoded item system.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        salvage_crate_scenario_object, SalvageCrateConfig, SalvageCratePlugin,
        SALVAGE_CRATE_TYPE_NAME,
    };
}

pub const SALVAGE_CRATE_TYPE_NAME: &str = "salvage_crate";

/// Tumble rate (radians/second) of the crate's render child.
const CRATE_TUMBLE_RAD_PER_SEC: f32 = 0.6;

/// Crate body color: bright against grey rock (layer-0 conveyance - the
/// prop advertises itself; spike, "Conveying objectives").
const CRATE_COLOR: Color = Color::srgb(1.0, 0.75, 0.15);

/// Faint self-glow so crates read in shadowed debris.
const CRATE_EMISSIVE: f32 = 2.0;

#[derive(Clone, Debug)]
pub struct SalvageCrateConfig {
    /// Edge length of the crate's visible box (world units).
    pub size: f32,
    /// Pickup radius: the sensor sphere that counts as "collected".
    pub area_radius: f32,
}

pub fn salvage_crate_scenario_object(config: SalvageCrateConfig) -> impl Bundle {
    debug!("salvage_crate_scenario_object: config {:?}", config);

    (
        SalvageCrateMarker,
        EntityTypeName::new(SALVAGE_CRATE_TYPE_NAME),
        SalvageCrateSize(config.size),
        // The pickup volume: the crate IS its own trigger area, so OnEnter
        // fires under its scenario id via the area plugin.
        ScenarioAreaMarker,
        Collider::sphere(config.area_radius),
        Sensor,
        // On rails: a sensor-only collider contributes no mass, so a
        // Dynamic crate would be an avian zero-mass warning; the visual
        // tumble is a render-child animation, not physics.
        RigidBody::Static,
    )
}

/// Marks a salvage crate root.
#[derive(Component, Clone, Debug, Reflect)]
pub struct SalvageCrateMarker;

/// Render input: the crate's visible edge length.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct SalvageCrateSize(pub f32);

/// The tumble driver on the crate's render child: axis and phase seeded
/// from the entity index so a cluster of crates does not spin in lockstep.
#[derive(Component, Clone, Debug, Reflect)]
pub struct CrateTumble {
    pub axis: Vec3,
}

pub struct SalvageCratePlugin {
    pub render: bool,
}

impl Plugin for SalvageCratePlugin {
    fn build(&self, app: &mut App) {
        debug!("SalvageCratePlugin: build");

        if self.render {
            app.add_observer(insert_crate_render);
            app.add_systems(Update, tumble_crates);
        }
    }
}

/// The visible crate: a bright box child, tumbling for life. A child (not
/// the root) so the pickup collider on the root keeps its own radius and
/// the tumble never rotates the sensor.
fn insert_crate_render(
    add: On<Add, SalvageCrateMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_crate: Query<&SalvageCrateSize, With<SalvageCrateMarker>>,
) {
    let entity = add.entity;
    let Ok(size) = q_crate.get(entity) else {
        error!(
            "insert_crate_render: entity {:?} not found in q_crate",
            entity
        );
        return;
    };

    let material = materials.add(StandardMaterial {
        base_color: CRATE_COLOR,
        emissive: CRATE_COLOR.to_linear() * CRATE_EMISSIVE,
        ..default()
    });

    // A per-crate tumble axis from the entity index - decorrelated without
    // rand (scenario spawn order is stable, aesthetics only).
    let seed = entity.index_u32() as f32;
    let axis = Vec3::new((seed * 0.7).sin(), (seed * 1.3).cos(), (seed * 2.1).sin())
        .try_normalize()
        .unwrap_or(Vec3::Y);

    commands.entity(entity).insert(children![(
        Name::new("SalvageCrateBox"),
        Transform::default(),
        Mesh3d(meshes.add(Cuboid::from_length(**size))),
        MeshMaterial3d(material),
        CrateTumble { axis },
    )]);
}

/// Spin each crate's render child around its tumble axis.
fn tumble_crates(time: Res<Time>, mut q_tumble: Query<(&CrateTumble, &mut Transform)>) {
    for (tumble, mut transform) in &mut q_tumble {
        transform.rotate(Quat::from_axis_angle(
            tumble.axis,
            CRATE_TUMBLE_RAD_PER_SEC * time.delta_secs(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The crate's contract: a static sensor trigger (the pickup volume,
    /// firing OnEnter under its scenario id) that never collides physically
    /// and never moves - the ScenarioAreaMarker + Sensor + Static trio.
    #[test]
    fn crate_is_a_static_sensor_trigger() {
        let mut world = World::new();
        let entity = world
            .spawn(salvage_crate_scenario_object(SalvageCrateConfig {
                size: 1.5,
                area_radius: 6.0,
            }))
            .id();

        assert!(world.get::<SalvageCrateMarker>(entity).is_some());
        assert!(world.get::<ScenarioAreaMarker>(entity).is_some());
        assert!(world.get::<Sensor>(entity).is_some());
        assert!(world.get::<Collider>(entity).is_some());
        assert!(matches!(
            world.get::<RigidBody>(entity),
            Some(RigidBody::Static)
        ));
    }

    /// The pickup path end to end through real physics AND the real event
    /// pipeline: a moving body entering the crate's sensor fires OnEnter
    /// under the CRATE's scenario id, an EventHandler filtered on
    /// (crate id, other id) matches it, and its VariableSet action lands in
    /// NovaEventWorld - the exact chain a scenario's salvage beat runs on.
    /// Delivery guard: the variable starts false and only the handler can
    /// flip it, so a quiet sensor fails the test.
    #[test]
    fn entering_the_crate_sensor_drives_the_scenario_event_pipeline() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.02,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_plugins(super::super::area::ScenarioAreaPlugin);
        app.finish();

        // The handler a scenario would register: OnEnter(crate_1, player) ->
        // picked_up = true.
        let mut handler = EventHandler::<NovaEventWorld>::from(crate::events::EventConfig::OnEnter);
        handler.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
            id: Some("crate_1".to_string()),
            other_id: Some("player_spaceship".to_string()),
            ..default()
        }));
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "picked_up".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(handler);

        app.world_mut().spawn((
            salvage_crate_scenario_object(SalvageCrateConfig {
                size: 1.5,
                area_radius: 6.0,
            }),
            EntityId::new("crate_1".to_string()),
            Transform::from_translation(Vec3::ZERO),
        ));
        // The mover: a player-ship stand-in flying straight through the
        // pickup volume.
        app.world_mut().spawn((
            EntityId::new("player_spaceship".to_string()),
            EntityTypeName::new("spaceship".to_string()),
            RigidBody::Dynamic,
            Collider::sphere(0.5),
            ColliderDensity(1.0),
            Transform::from_translation(Vec3::new(0.0, 0.0, 12.0)),
            LinearVelocity(Vec3::new(0.0, 0.0, -60.0)),
        ));

        assert!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("picked_up")
                .is_none(),
            "delivery guard: the variable does not exist until the handler runs"
        );

        // 12u at 60 u/s: inside the 6u sensor within ~0.15s; run half a
        // second of fixed ticks so collision, event queue and handler all
        // get their turns.
        for _ in 0..25 {
            app.update();
        }

        assert_eq!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("picked_up"),
            Some(&VariableLiteral::Boolean(true)),
            "the crate's OnEnter drove the filtered handler's action"
        );
    }

    /// The tumble animates the RENDER CHILD, not the trigger root: the
    /// sensor sphere must not inherit a spin (and the root must not need a
    /// mass). Runs the real observer + system on a rendered crate.
    #[test]
    fn tumble_rotates_the_render_child_only() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
        ));
        app.init_asset::<StandardMaterial>();
        app.add_observer(insert_crate_render);
        app.add_systems(Update, tumble_crates);

        let root = app
            .world_mut()
            .spawn(salvage_crate_scenario_object(SalvageCrateConfig {
                size: 1.5,
                area_radius: 6.0,
            }))
            .id();

        // Two updates: one to flush the observer's child spawn, one to tick
        // the tumble with a nonzero delta.
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(5));
        app.update();

        let children = app.world().get::<Children>(root).expect("render child");
        let child = children.iter().next().expect("one child");
        let child_rotation = app.world().get::<Transform>(child).unwrap().rotation;
        assert!(
            child_rotation.angle_between(Quat::IDENTITY) > 0.0,
            "the render child tumbles"
        );
        let root_rotation = app.world().get::<Transform>(root);
        assert!(
            root_rotation.is_none() || root_rotation.unwrap().rotation == Quat::IDENTITY,
            "the trigger root does not rotate"
        );
    }
}
