//! A section of a spaceship that can control its rotation using a PD controller.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;

use crate::prelude::SectionRenderOf;

pub mod prelude {
    pub use super::{
        controller_section, ControllerSectionConfig, ControllerSectionMarker,
        ControllerSectionPlugin, ControllerSectionRotationInput,
    };
}

/// Configuration for a controller section.
#[derive(Clone, Debug, Reflect)]
pub struct ControllerSectionConfig {
    /// The frequency of the PD controller in Hz.
    pub frequency: f32,
    /// The damping ratio of the PD controller.
    pub damping_ratio: f32,
    /// The maximum torque that can be applied by the PD controller.
    pub max_torque: f32,
    /// The render mesh of the hull section, defaults to a cuboid of size 1x1x1.
    pub render_mesh: Option<Handle<Scene>>,
}

impl Default for ControllerSectionConfig {
    fn default() -> Self {
        Self {
            frequency: 2.0,
            damping_ratio: 2.0,
            max_torque: 1.0,
            render_mesh: None,
        }
    }
}

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct ControllerSectionRenderMesh(Option<Handle<Scene>>);

/// Helper function to create a controller section entity bundle.
pub fn controller_section(config: ControllerSectionConfig) -> impl Bundle {
    debug!("controller_section: config {:?}", config);

    (
        ControllerSectionMarker,
        PDController {
            frequency: config.frequency,
            damping_ratio: config.damping_ratio,
            max_torque: config.max_torque,
        },
        ControllerSectionRotationInput::default(),
        ControllerSectionRenderMesh(config.render_mesh),
    )
}

/// Marker component for controller sections.
#[derive(Component, Clone, Debug, Reflect)]
pub struct ControllerSectionMarker;

/// The desired rotation of the controller section, in world space.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut, Reflect)]
pub struct ControllerSectionRotationInput(pub Quat);

/// A plugin that will enable the ControllerSection.
#[derive(Default)]
pub struct ControllerSectionPlugin {
    pub render: bool,
}

impl Plugin for ControllerSectionPlugin {
    fn build(&self, app: &mut App) {
        debug!("ControllerSectionPlugin: build");

        app.add_observer(insert_controller_section_target);

        app.add_systems(
            Update,
            update_controller_section_rotation_input.in_set(super::SpaceshipSectionSystems),
        );

        app.add_systems(
            FixedUpdate,
            sync_controller_section_forces.in_set(super::SpaceshipSectionSystems),
        );

        app.configure_sets(
            FixedUpdate,
            PDControllerSystems::Sync.before(super::SpaceshipSectionSystems),
        );

        if self.render {
            app.add_observer(insert_controller_section_render);
        }
    }
}

fn update_controller_section_rotation_input(
    mut q_controller: Query<(&mut PDControllerInput, &ControllerSectionRotationInput)>,
) {
    for (mut input, desired_rotation) in &mut q_controller {
        **input = **desired_rotation;
    }
}

fn sync_controller_section_forces(
    mut q_root: Query<Forces>,
    q_controller: Query<(&PDControllerOutput, &PDControllerTarget)>,
) {
    for (output, target) in &q_controller {
        if let Ok(mut forces) = q_root.get_mut(**target) {
            forces.apply_torque(**output);
        }
    }
}

fn insert_controller_section_target(
    add: On<Add, ControllerSectionMarker>,
    mut commands: Commands,
    q_controller: Query<&ChildOf, With<ControllerSectionMarker>>,
) {
    let entity = add.entity;
    trace!("insert_controller_section_target: entity {:?}", entity);
    let Ok(ChildOf(root)) = q_controller.get(entity) else {
        error!(
            "insert_controller_section_target: entity {:?} not found in q_controller",
            entity
        );
        return;
    };

    commands.entity(entity).insert(PDControllerTarget(*root));
}

fn insert_controller_section_render(
    add: On<Add, ControllerSectionMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_controller: Query<&ControllerSectionRenderMesh, With<ControllerSectionMarker>>,
) {
    let entity = add.entity;
    trace!("insert_controller_section_render: entity {:?}", entity);

    let Ok(render_mesh) = q_controller.get(entity) else {
        error!(
            "insert_controller_section_render: entity {:?} not found in q_controller",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(scene) => {
            commands.entity(entity).insert((children![(
                Name::new("Controller Section Body"),
                SectionRenderOf(entity),
                SceneRoot(scene.clone()),
            ),],));
        }
        None => {
            commands.entity(entity).insert((children![
                (
                    Name::new("Controller Section Body (A)"),
                    SectionRenderOf(entity),
                    Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                    MeshMaterial3d(materials.add(Color::srgb(0.2, 0.7, 0.9))),
                ),
                (
                    Name::new("Controller Section Window (B)"),
                    SectionRenderOf(entity),
                    Mesh3d(meshes.add(Cylinder::new(0.2, 0.1))),
                    MeshMaterial3d(materials.add(Color::srgb(0.9, 0.9, 1.0))),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                )
            ],));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawns_controller_with_default_config() {
        // Arrange
        let mut app = App::new();
        let id = app
            .world_mut()
            .spawn(controller_section(ControllerSectionConfig::default()))
            .id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<ControllerSectionMarker>(id).is_some());
    }

    #[test]
    fn spawns_controller_with_custom_scene() {
        // Arrange
        let mut app = App::new();
        let custom_scene = Handle::<Scene>::default();
        let config = ControllerSectionConfig {
            render_mesh: Some(custom_scene.clone()),
            ..Default::default()
        };
        let id = app.world_mut().spawn(controller_section(config)).id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<ControllerSectionMarker>(id).is_some());
        let render_mesh = app.world().get::<ControllerSectionRenderMesh>(id).unwrap();
        assert!(render_mesh.0.is_some());
        assert_eq!(render_mesh.0.as_ref().unwrap(), &custom_scene);
    }
}
