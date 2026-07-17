//! Defines a thruster section for a spaceship, which provides thrust in a specified direction.

use avian3d::prelude::*;
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};
use bevy_common_systems::prelude::*;

use crate::prelude::{AssetRef, SectionDamageClass, SectionInactiveMarker, SectionRenderOf};

pub mod prelude {
    pub use super::{
        thruster_section, ThrusterExhaust, ThrusterExhaustConfig, ThrusterSectionConfig,
        ThrusterSectionInput, ThrusterSectionMagnitude, ThrusterSectionMarker,
        ThrusterSectionPlugin, ThrusterSectionRenderMarker,
    };
}

const THRUSTER_SECTION_DEFAULT_MAGNITUDE: f32 = 1.0;

/// Configuration for a thruster section of a spaceship.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThrusterSectionConfig {
    /// The magnitude of the force produced by this thruster section.
    pub magnitude: f32,
    /// The render mesh of the section, defaults to prototype mesh if None.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh: Option<AssetRef<WorldAsset>>,
    /// The engine-hum loop this thruster contributes to. An authorable
    /// [`AssetRef<AudioSource>`] like the render mesh (task 20260717-101650,
    /// spike 20260717-101524): thrusters sharing a sound share one loop entity
    /// whose volume tracks the loudest ship burning it. AUTHORED-OR-SILENT: a
    /// thruster with no loop_sound hums nothing; base thrusters author
    /// `self://sounds/thruster_loop.wav` via gen_content. Snapshotted
    /// (unresolved) as [`ThrusterSectionLoopSound`]; the audio module resolves
    /// and groups by handle.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub loop_sound: Option<AssetRef<AudioSource>>,
    /// Authorable exhaust cone: where it sits, how it is rotated, and its shape.
    /// `None` uses the default placement/shape (same cone the procedural
    /// thruster always had); set it to align the cone with a custom render mesh.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub exhaust: Option<ThrusterExhaust>,
}

/// A thruster's authored hum, snapshotted UNRESOLVED from
/// [`ThrusterSectionConfig::loop_sound`] by [`thruster_section`]; the audio
/// module resolves it and groups thrusters by handle. `pub(crate)` for the
/// audio module.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct ThrusterSectionLoopSound(#[reflect(ignore)] pub Option<AssetRef<AudioSource>>);

impl Default for ThrusterSectionConfig {
    fn default() -> Self {
        Self {
            magnitude: THRUSTER_SECTION_DEFAULT_MAGNITUDE,
            render_mesh: None,
            loop_sound: None,
            exhaust: None,
        }
    }
}

/// The section's authored exhaust placement/shape, snapshotted from
/// [`ThrusterSectionConfig::exhaust`] so the render observer can spawn the cone.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct ThrusterSectionExhaust(Option<ThrusterExhaust>);

/// Helper function to create an thruster section entity bundle.
pub fn thruster_section(config: ThrusterSectionConfig) -> impl Bundle {
    debug!("thruster_section: config {:?}", config);

    (
        ThrusterSectionMarker,
        SectionDamageClass::Thruster,
        ThrusterSectionMagnitude(config.magnitude),
        ThrusterSectionInput(0.0),
        ThrusterSectionLoopSound(config.loop_sound.clone()),
        ThrusterSectionRenderMesh(config.render_mesh),
        ThrusterSectionExhaust(config.exhaust),
    )
}

/// Configuration for the thruster exhaust shader (cone shape + glow). Authorable
/// via [`ThrusterExhaust::shape`]; `#[serde(default)]` lets a mod set only the
/// fields it cares about.
#[derive(Component, Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ThrusterExhaustConfig {
    pub exhaust_height: f32,
    pub exhaust_radius: f32,
    pub exhaust_max: f32,
    pub exhaust_inner_height: f32,
    pub exhaust_inner_radius: f32,
    pub exhaust_inner_max: f32,
    pub emissive_color: LinearRgba,
    pub emissive_inner_color: LinearRgba,
}

impl Default for ThrusterExhaustConfig {
    fn default() -> Self {
        Self {
            exhaust_height: 0.1,
            exhaust_radius: 0.4,
            exhaust_max: 1.0,
            exhaust_inner_height: 0.05,
            exhaust_inner_radius: 0.1,
            exhaust_inner_max: 0.5,
            emissive_color: LinearRgba::rgb(0.0, 10.0, 10.0),
            emissive_inner_color: LinearRgba::rgb(0.0, 0.0, 10.0),
        }
    }
}

/// Authorable placement + shape of a thruster's exhaust cone. `offset`/`rotation`
/// position and orient the cone relative to the section (the cone is built along
/// +Y); `shape` is the shader/size config. The default matches the procedural
/// thruster's historical hardcoded placement, so an omitted `exhaust` still
/// yields the same cone.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ThrusterExhaust {
    pub offset: Vec3,
    pub rotation: Quat,
    pub shape: ThrusterExhaustConfig,
}

impl Default for ThrusterExhaust {
    fn default() -> Self {
        Self {
            offset: Vec3::new(0.0, 0.0, 0.3),
            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            shape: ThrusterExhaustConfig::default(),
        }
    }
}

/// Marker component for thruster sections.
#[derive(Component, Clone, Debug, Reflect)]
pub struct ThrusterSectionMarker;

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct ThrusterSectionRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

/// The thrust magnitude produced by this thruster section. This is a simple scalar value that can be
/// used to determine the thrust force applied to the ship.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct ThrusterSectionMagnitude(pub f32);

/// The thuster input. Will be a value between 0.0 and 1.0, where 0.0 means no thrust and 1.0 means
/// full thrust.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct ThrusterSectionInput(pub f32);

/// A plugin that enables the ThrusterSection component and its related systems.
#[derive(Default)]
pub struct ThrusterSectionPlugin {
    pub render: bool,
}

impl Plugin for ThrusterSectionPlugin {
    fn build(&self, app: &mut App) {
        debug!("ThrusterSectionPlugin: build");

        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>,
        >::default());

        if self.render {
            app.add_observer(insert_thruster_section_render);
            app.add_observer(insert_thruster_shader);
        }

        app.add_systems(
            Update,
            thruster_shader_update_system.in_set(super::SpaceshipSectionSystems),
        );
        app.add_systems(
            FixedUpdate,
            thruster_impulse_system.in_set(super::SpaceshipSectionSystems),
        );
    }
}

// `pub(crate)` so the flight-control tests can register the real impulse
// system and cover the whole intent -> thrust -> velocity pipeline.
pub(crate) fn thruster_impulse_system(
    q_thruster: Query<
        (
            &Transform,
            &ChildOf,
            &ThrusterSectionMagnitude,
            &ThrusterSectionInput,
        ),
        (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_root: Query<Forces>,
) {
    for (transform, &ChildOf(root), magnitude, input) in &q_thruster {
        let Ok(mut force) = q_root.get_mut(root) else {
            error!(
                "thruster_impulse_system: entity {:?} not found in q_root",
                root
            );
            continue;
        };

        // Compose the burn from the ROOT's raw physics pose and the engine's
        // local mount (sections are direct children of the root), the exact
        // math of the balancer's lever arms in flight.rs. In FixedUpdate,
        // `GlobalTransform` and the child collider's avian pose are at least
        // one tick stale - at speed that trails the hull by `v * dt` and a
        // COM-centered engine torques the ship it must not touch (task
        // 20260711-103527).
        let position = force.position().0;
        let rotation = *force.rotation();
        let thrust_direction = rotation
            .mul_vec3(transform.rotation.mul_vec3(Vec3::NEG_Z))
            .normalize();
        let thrust_force = thrust_direction * **magnitude * input.clamp(0.0, 1.0);
        let world_point = position + rotation.mul_vec3(transform.translation);

        force.apply_linear_impulse_at_point(thrust_force, world_point);
    }
}

#[derive(Component, Clone, Debug, Reflect)]
struct ThrusterSectionExhaustShaderMarker;

fn thruster_shader_update_system(
    q_thruster: Query<
        (&ThrusterSectionInput, Has<SectionInactiveMarker>),
        With<ThrusterSectionMarker>,
    >,
    q_render: Query<
        (
            &MeshMaterial3d<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>,
            &ChildOf,
        ),
        With<ThrusterSectionExhaustShaderMarker>,
    >,
    q_child: Query<&ChildOf>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>>,
) {
    for (material, &ChildOf(parent)) in &q_render {
        let Some((input, inactive)) = find_thruster_section(parent, &q_thruster, &q_child) else {
            error!(
                "thruster_shader_update_system: entity {:?} not found in q_thruster",
                parent
            );
            continue;
        };

        let Some(mut material) = materials.get_mut(&**material) else {
            error!(
                "thruster_shader_update_system: material for entity {:?} not found",
                parent
            );
            continue;
        };

        if inactive {
            material.extension.thruster_input = 0.0;
        } else {
            material.extension.thruster_input = *input;
        }
    }
}

fn find_thruster_section(
    parent: Entity,
    q_thruster: &Query<
        (&ThrusterSectionInput, Has<SectionInactiveMarker>),
        With<ThrusterSectionMarker>,
    >,
    q_child: &Query<&ChildOf>,
) -> Option<(ThrusterSectionInput, bool)> {
    let mut parent = parent;
    loop {
        if let Ok((input, inactive)) = q_thruster.get(parent) {
            return Some((input.clone(), inactive));
        }

        let Ok(ChildOf(grandparent)) = q_child.get(parent) else {
            return None;
        };

        parent = *grandparent;
    }
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct ThrusterSectionRenderMarker;

fn insert_thruster_section_render(
    add: On<Add, ThrusterSectionMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    q_thruster: Query<
        (
            &ThrusterSectionRenderMesh,
            &ThrusterSectionExhaust,
            Has<ThrusterSectionRenderMarker>,
        ),
        With<ThrusterSectionMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_thruster_section_render: entity {:?}", entity);

    let Ok((render_mesh, exhaust, has_render)) = q_thruster.get(entity) else {
        error!(
            "insert_thruster_section_render: entity {:?} not found in q_thruster",
            entity
        );
        return;
    };

    if has_render {
        trace!(
            "insert_thruster_section_render: entity {:?} already has render, skipping",
            entity
        );
        return;
    }

    commands.entity(entity).insert(ThrusterSectionRenderMarker);
    match &**render_mesh {
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            commands.entity(entity).insert((children![(
                Name::new("Thruster Section Body"),
                SectionRenderOf(entity),
                WorldAssetRoot(scene),
            ),],));
        }
        None => {
            commands.entity(entity).insert((children![
                (
                    Name::new("Thruster Section Body (A)"),
                    SectionRenderOf(entity),
                    Mesh3d(meshes.add(Cylinder::new(0.4, 0.4))),
                    MeshMaterial3d(standard_materials.add(Color::srgb(0.8, 0.8, 0.8))),
                    Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
                        .with_translation(Vec3::new(0.0, 0.0, -0.3)),
                ),
                (
                    Name::new("Thruster Section Body (B)"),
                    SectionRenderOf(entity),
                    Mesh3d(meshes.add(Cone::new(0.5, 0.5))),
                    MeshMaterial3d(standard_materials.add(Color::srgb(0.9, 0.3, 0.2))),
                    Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                ),
            ],));
        }
    }

    // Spawn the exhaust cone for EVERY thruster (custom mesh or procedural),
    // using the authored placement/shape or the default. `insert_thruster_shader`
    // turns the ThrusterExhaustConfig into the cone mesh + glow material.
    let exhaust = exhaust.0.clone().unwrap_or_default();
    commands.entity(entity).with_children(|parent| {
        parent.spawn((
            Name::new("Thruster Exhaust"),
            SectionRenderOf(entity),
            exhaust.shape,
            Transform::from_translation(exhaust.offset).with_rotation(exhaust.rotation),
        ));
    });
}

fn insert_thruster_shader(
    add: On<Add, ThrusterExhaustConfig>,
    mut commands: Commands,
    q_config: Query<&ThrusterExhaustConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut exhaust_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>,
    >,
) {
    let entity = add.entity;
    trace!("insert_thruster_shader: entity {:?}", entity);

    let Ok(config) = q_config.get(entity) else {
        error!(
            "insert_thruster_shader: entity {:?} not found in q_config",
            entity
        );
        return;
    };

    let mesh = TriangleMeshBuilder::new_cone(32, 4)
        .with_scale(Vec3::new(
            config.exhaust_radius,
            config.exhaust_height,
            config.exhaust_radius,
        ))
        .build();
    let material = ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 1.0),
            perceptual_roughness: 1.0,
            metallic: 0.0,
            emissive: config.emissive_color,
            ..default()
        },
        extension: ThrusterExhaustMaterial::default()
            .with_exhaust_height(config.exhaust_max)
            .with_exhaust_radius(config.exhaust_radius),
    };

    let inner_mesh = TriangleMeshBuilder::new_cone(32, 4)
        .with_scale(Vec3::new(
            config.exhaust_inner_radius,
            config.exhaust_inner_height,
            config.exhaust_inner_radius,
        ))
        .build();
    let inner_material = ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 1.0),
            perceptual_roughness: 1.0,
            metallic: 0.0,
            emissive: config.emissive_inner_color,
            ..default()
        },
        extension: ThrusterExhaustMaterial::default()
            .with_exhaust_height(config.exhaust_inner_max)
            .with_exhaust_radius(config.exhaust_inner_radius),
    };

    commands.entity(entity).insert((
        ThrusterSectionExhaustShaderMarker,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(exhaust_materials.add(material)),
        children![(
            ThrusterSectionExhaustShaderMarker,
            Transform::from_xyz(0.0, 1e-4, 0.0),
            Mesh3d(meshes.add(inner_mesh)),
            MeshMaterial3d(exhaust_materials.add(inner_material)),
        )],
    ));
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct ThrusterExhaustMaterial {
    #[uniform(100)]
    pub thruster_input: f32,
    #[uniform(100)]
    pub thruster_exhaust_radius: f32,
    #[uniform(100)]
    pub thruster_exhaust_height: f32,
    #[cfg(target_arch = "wasm32")]
    #[uniform(100)]
    _webgl2_padding_16b: u32,
}

impl ThrusterExhaustMaterial {
    pub fn with_exhaust_radius(mut self, radius: f32) -> Self {
        self.thruster_exhaust_radius = radius;
        self
    }

    pub fn with_exhaust_height(mut self, height: f32) -> Self {
        self.thruster_exhaust_height = height;
        self
    }
}

impl MaterialExtension for ThrusterExhaustMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/thruster_exhaust.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/thruster_exhaust.wgsl".into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn spawns_thruster_with_default_config() {
        // Arrange
        let mut app = App::new();
        let id = app
            .world_mut()
            .spawn(thruster_section(ThrusterSectionConfig::default()))
            .id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<ThrusterSectionMarker>(id).is_some());
    }

    #[test]
    fn spawns_thruster_with_custom_scene() {
        // Arrange
        let mut app = App::new();
        let custom_scene = Handle::<WorldAsset>::default();
        let config = ThrusterSectionConfig {
            render_mesh: Some(custom_scene.clone().into()),
            ..default()
        };
        let id = app.world_mut().spawn(thruster_section(config)).id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<ThrusterSectionMarker>(id).is_some());
        let render_mesh = app.world().get::<ThrusterSectionRenderMesh>(id).unwrap();
        assert!(render_mesh.0.is_some());
        assert_eq!(
            render_mesh.0.as_ref().unwrap(),
            &AssetRef::from(custom_scene)
        );
    }
}
