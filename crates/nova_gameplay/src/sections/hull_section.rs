//! Module for defining hull sections in a 3D environment using Bevy and Avian3D.

use bevy::prelude::*;

use crate::prelude::{
    AssetRef, RenderMeshTransform, SectionDamageClass, SectionRenderMeshTransform, SectionRenderOf,
};

/// Glob-import surface: `use nova_gameplay::sections::hull_section::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{hull_section, HullSectionConfig, HullSectionMarker, HullSectionPlugin};
}

/// Configuration for a hull section.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HullSectionConfig {
    /// The render mesh of the hull section, defaults to a cuboid of size 1x1x1.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh: Option<AssetRef<WorldAsset>>,
    /// Optional transform (position + rotation) applied to the hull's render
    /// mesh only. None = the mesh sits at the section origin (unchanged).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_transform: Option<RenderMeshTransform>,
}

/// Helper function to create a hull section entity bundle.
pub fn hull_section(config: HullSectionConfig) -> impl Bundle {
    debug!("hull_section: config {:?}", config);

    (
        HullSectionMarker,
        SectionDamageClass::Hull,
        HullSectionRenderMesh(config.render_mesh),
        SectionRenderMeshTransform(config.render_mesh_transform),
    )
}

/// Marker component for hull sections.
#[derive(Component, Clone, Debug, Reflect)]
pub struct HullSectionMarker;

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct HullSectionRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

/// A plugin that enables the HullSection component and its related systems.
#[derive(Default)]
pub struct HullSectionPlugin {
    /// Whether to spawn the section's render mesh (false on headless servers).
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
    asset_server: Res<AssetServer>,
    q_hull: Query<(&HullSectionRenderMesh, &SectionRenderMeshTransform), With<HullSectionMarker>>,
) {
    let entity = add.entity;
    trace!("insert_hull_section_render: entity {:?}", entity);

    let Ok((render_mesh, render_mesh_transform)) = q_hull.get(entity) else {
        error!(
            "insert_hull_section_render: entity {:?} not found in q_hull",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            // Authored render-mesh transform (identity when unset). Applied to
            // the mesh CHILD, so it moves the art only, never the collider.
            let transform = render_mesh_transform
                .map(RenderMeshTransform::to_transform)
                .unwrap_or_default();
            commands.entity(entity).insert((children![(
                Name::new("Hull Section Body"),
                transform,
                SectionRenderOf(entity),
                WorldAssetRoot(scene),
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
        let custom_scene = Handle::<WorldAsset>::default();
        let config = HullSectionConfig {
            render_mesh: Some(custom_scene.clone().into()),
            render_mesh_transform: None,
        };
        let id = app.world_mut().spawn(hull_section(config)).id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<HullSectionMarker>(id).is_some());
        let render_mesh = app.world().get::<HullSectionRenderMesh>(id).unwrap();
        assert!(render_mesh.0.is_some());
        assert_eq!(
            render_mesh.0.as_ref().unwrap(),
            &AssetRef::from(custom_scene)
        );
    }

    /// The authored `render_mesh_transform` lands on the hull's meshed render
    /// child (task 20260718-121205), and an unset one leaves it at identity -
    /// the same shared wiring as the turret, exercised on a section-kind path.
    #[test]
    fn render_mesh_transform_positions_the_hull_render_child() {
        use bevy::asset::AssetPlugin;

        let child_transform = |xf: Option<RenderMeshTransform>| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();
            app.init_asset::<WorldAsset>();
            app.add_observer(insert_hull_section_render);
            app.world_mut().spawn(hull_section(HullSectionConfig {
                render_mesh: Some(AssetRef::from("gltf/hull-01.glb#Scene0".to_string())),
                render_mesh_transform: xf,
            }));
            app.world_mut().flush();
            app.update();

            let world = app.world_mut();
            let mut q = world.query_filtered::<&Transform, With<SectionRenderOf>>();
            let found: Vec<Transform> = q.iter(world).copied().collect();
            assert_eq!(found.len(), 1, "one meshed hull render child expected");
            found[0]
        };

        let authored = RenderMeshTransform {
            position: Vec3::new(0.2, -0.1, 0.4),
            rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        };
        let got = child_transform(Some(authored));
        assert_eq!(got.translation, authored.position);
        assert!(got.rotation.abs_diff_eq(authored.rotation, 1e-5));

        assert_eq!(child_transform(None), Transform::IDENTITY);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn hull_config_serde_omits_unset_render_mesh_transform() {
        // A hull that omits the field does not serialize it (RON parity), and a
        // set one round-trips.
        let plain = HullSectionConfig::default();
        let ron = ron::ser::to_string(&plain).expect("serialize");
        assert!(
            !ron.contains("render_mesh_transform"),
            "unset must not serialize: {ron}"
        );

        let authored = HullSectionConfig {
            render_mesh: None,
            render_mesh_transform: Some(RenderMeshTransform {
                position: Vec3::new(1.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
            }),
        };
        let ron = ron::ser::to_string(&authored).expect("serialize");
        let back: HullSectionConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.render_mesh_transform, authored.render_mesh_transform);
    }
}
