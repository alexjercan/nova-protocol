//! Module for defining hull sections in a 3D environment using Bevy and Avian3D.

use bevy::prelude::*;

use crate::prelude::SectionRenderOf;

pub mod prelude {
    pub use super::{hull_section, HullSectionConfig, HullSectionMarker, HullSectionPlugin};
}

/// Configuration for a hull section.
#[derive(Clone, Debug, Default, Reflect)]
pub struct HullSectionConfig {
    /// The render mesh of the hull section, defaults to a cuboid of size 1x1x1.
    pub render_mesh: Option<Handle<WorldAsset>>,
}

/// Helper function to create a hull section entity bundle.
pub fn hull_section(config: HullSectionConfig) -> impl Bundle {
    debug!("hull_section: config {:?}", config);

    (HullSectionMarker, HullSectionRenderMesh(config.render_mesh))
}

/// Marker component for hull sections.
#[derive(Component, Clone, Debug, Reflect)]
pub struct HullSectionMarker;

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct HullSectionRenderMesh(Option<Handle<WorldAsset>>);

/// A plugin that enables the HullSection component and its related systems.
#[derive(Default)]
pub struct HullSectionPlugin {
    pub render: bool,
}

impl Plugin for HullSectionPlugin {
    fn build(&self, app: &mut App) {
        debug!("HullSectionPlugin: build");

        if self.render {
            app.add_observer(insert_hull_section_render);
        }
    }
}

fn insert_hull_section_render(
    add: On<Add, HullSectionMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_hull: Query<&HullSectionRenderMesh, With<HullSectionMarker>>,
) {
    let entity = add.entity;
    trace!("insert_hull_section_render: entity {:?}", entity);

    let Ok(render_mesh) = q_hull.get(entity) else {
        error!(
            "insert_hull_section_render: entity {:?} not found in q_hull",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(scene) => {
            commands.entity(entity).insert((children![(
                Name::new("Hull Section Body"),
                SectionRenderOf(entity),
                WorldAssetRoot(scene.clone()),
            ),],));
        }
        None => {
            commands.entity(entity).insert((children![(
                Name::new("Hull Section Body"),
                SectionRenderOf(entity),
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                MeshMaterial3d(materials.add(Color::srgb(0.8, 0.8, 0.8))),
            ),],));
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn spawns_hull_with_default_config() {
        // Arrange
        let mut app = App::new();
        let id = app
            .world_mut()
            .spawn(hull_section(HullSectionConfig::default()))
            .id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<HullSectionMarker>(id).is_some());
    }

    #[test]
    fn spawns_hull_with_custom_scene() {
        // Arrange
        let mut app = App::new();
        let custom_scene = Handle::<Scene>::default();
        let config = HullSectionConfig {
            render_mesh: Some(custom_scene.clone()),
            ..default()
        };
        let id = app.world_mut().spawn(hull_section(config)).id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<HullSectionMarker>(id).is_some());
        let render_mesh = app.world().get::<HullSectionRenderMesh>(id).unwrap();
        assert!(render_mesh.0.is_some());
        assert_eq!(render_mesh.0.as_ref().unwrap(), &custom_scene);
    }
}
