//! The directional-HUD widget family around the ship: an orbiting cone +
//! shaded sphere driven by a world vector. Two sources ship today - the
//! original velocity readout (white/blue) and the gravity indicator
//! (yellow, pointing down the dominant well's pull, hidden in flat
//! space; task 20260710-201514, replacing the SOI shell the user cut).
//! The module keeps its historical name to avoid churn; a rename to
//! direction_hud is fair game in a cleanup pass.

use avian3d::prelude::*;
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};
use bevy_common_systems::prelude::*;

use crate::{flight::prelude::*, gravity::prelude::*};

pub mod prelude {
    pub use super::{
        velocity_hud, VelocityHudConfig, VelocityHudIndicatorMarker, VelocityHudMarker,
        VelocityHudPalette, VelocityHudPlugin, VelocityHudSource, VelocityHudTargetEntity,
    };
}

/// What the widget's vector means - which world quantity feeds the cone
/// direction and the sphere shading.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum VelocityHudSource {
    /// The target's linear velocity (the original readout).
    #[default]
    Velocity,
    /// The pull of the target's dominant gravity well; the widget hides
    /// itself in flat space.
    Gravity,
}

/// The widget's colors, so the two sources never read as one quantity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
pub struct VelocityHudPalette {
    /// The orbiting cone.
    pub indicator: Color,
    /// The shaded sphere's base tint.
    pub sphere: Color,
}

impl Default for VelocityHudPalette {
    fn default() -> Self {
        // The original velocity colors.
        Self {
            indicator: Color::srgba(1.0, 1.0, 1.0, 1.0),
            sphere: Color::srgba(0.0, 0.5, 1.0, 0.2),
        }
    }
}

impl VelocityHudPalette {
    /// Gravity yellow (user request 2026-07-10): same shader, different
    /// color, so pull and velocity never read as the same thing.
    pub const GRAVITY: Self = Self {
        indicator: Color::srgba(1.0, 0.9, 0.2, 1.0),
        sphere: Color::srgba(1.0, 0.8, 0.1, 0.15),
    };

    /// The velocity widget while the autopilot flies: the flight
    /// computer's nav-cyan family (NAV_CYAN's rgb), so an engaged ship
    /// reads as computer-flown from the sphere alone (task
    /// 20260710-234115). Values are a starting point for the by-eye pass.
    pub const ENGAGED: Self = Self {
        indicator: Color::srgba(0.3, 0.9, 1.0, 1.0),
        sphere: Color::srgba(0.3, 0.9, 1.0, 0.2),
    };
}

/// The palette the velocity widget should wear right now. Pure so the
/// engaged/manual decision is unit-testable apart from the material
/// plumbing.
pub(crate) fn desired_velocity_palette(engaged: bool) -> VelocityHudPalette {
    if engaged {
        VelocityHudPalette::ENGAGED
    } else {
        VelocityHudPalette::default()
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct VelocityHudMarker;

#[derive(Component, Debug, Clone, Reflect)]
pub struct VelocityHudIndicatorMarker;

#[derive(Component, Debug, Clone, Reflect)]
pub struct VelocityHudSphereMarker;

#[derive(Clone, Debug)]
pub struct VelocityHudConfig {
    pub radius: f32,
    pub sharpness: f32,
    pub target: Entity,
    pub source: VelocityHudSource,
    pub palette: VelocityHudPalette,
}

impl Default for VelocityHudConfig {
    fn default() -> Self {
        Self {
            radius: 5.0,
            sharpness: 10.0,
            target: Entity::PLACEHOLDER,
            source: VelocityHudSource::default(),
            palette: VelocityHudPalette::default(),
        }
    }
}

pub fn velocity_hud(config: VelocityHudConfig) -> impl Bundle {
    debug!("velocity_hud: config {:?}", config);

    // The gravity variant starts hidden until the feeder proves the ship
    // is in a well, so a spawn in flat space never flashes the widget.
    let visibility = match config.source {
        VelocityHudSource::Velocity => Visibility::Visible,
        VelocityHudSource::Gravity => Visibility::Hidden,
    };

    (
        Name::new("VelocityHUD"),
        VelocityHudMarker,
        VelocityHudTargetEntity(config.target),
        VelocityHudSharpness(config.sharpness),
        config.source,
        config.palette,
        DirectionalSphereOrbit {
            radius: config.radius,
            ..default()
        },
        Transform::default(),
        visibility,
    )
}

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct VelocityHudTargetEntity(Entity);

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct VelocityHudSharpness(pub f32);

#[derive(Default)]
pub struct VelocityHudPlugin;

impl Plugin for VelocityHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("VelocityHudPlugin: build");

        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, DirectionMagnitudeMaterial>,
        >::default());
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>,
        >::default());

        // The gravity variant reads the gravity tunables; init here too
        // so the widget stands alone (idempotent with NovaGravityPlugin).
        app.init_resource::<GravitySettings>();
        app.register_type::<VelocityHudSource>();
        app.register_type::<VelocityHudPalette>();

        app.add_observer(insert_velocity_hud_indicator_system);
        app.add_observer(insert_velocity_hud_sphere_system);

        app.add_systems(
            Update,
            (
                update_velocity_hud_input,
                sync_orbit_state,
                sync_engaged_palette,
                direction_shader_update_system,
            )
                .in_set(super::NovaHudSystems),
        );
    }
}

/// The gravity indicator's shader magnitude: the felt pull normalized by
/// the strength cap, so a surface-strength well reads full scale. Pure
/// for unit testing.
pub(crate) fn gravity_indicator_magnitude(accel: f32, max_surface_gravity: f32) -> f32 {
    (accel / max_surface_gravity.max(1e-3)).clamp(0.0, 1.0)
}

/// The widget's world vector for this frame: the source quantity's
/// direction, plus whether the widget should show at all (the gravity
/// variant hides in flat space).
fn source_vector(
    source: VelocityHudSource,
    target: Entity,
    q_velocity: &Query<&LinearVelocity>,
    q_ship: &Query<(&Position, Option<&DominantWell>)>,
    q_wells: &Query<&Position, With<GravityWell>>,
) -> Option<Vec3> {
    match source {
        VelocityHudSource::Velocity => q_velocity
            .get(target)
            .ok()
            .map(|velocity| velocity.normalize_or_zero()),
        VelocityHudSource::Gravity => {
            let (position, dominant) = q_ship.get(target).ok()?;
            let well_position = q_wells.get(**dominant?).ok()?;
            (**well_position - **position).try_normalize()
        }
    }
}

fn update_velocity_hud_input(
    mut q_hud: Query<
        (
            &mut DirectionalSphereOrbitInput,
            &mut Visibility,
            &VelocityHudSource,
            &VelocityHudTargetEntity,
        ),
        With<VelocityHudMarker>,
    >,
    q_velocity: Query<&LinearVelocity>,
    q_ship: Query<(&Position, Option<&DominantWell>)>,
    q_wells: Query<&Position, With<GravityWell>>,
) {
    for (mut hud_input, mut visibility, &source, target) in q_hud.iter_mut() {
        match source_vector(source, **target, &q_velocity, &q_ship, &q_wells) {
            Some(dir) => {
                **hud_input = dir;
                // Only the gravity variant toggles itself; the velocity
                // readout is always on and never wrote Visibility before.
                if source == VelocityHudSource::Gravity && *visibility != Visibility::Inherited {
                    *visibility = Visibility::Inherited;
                }
            }
            None if source == VelocityHudSource::Gravity => {
                if *visibility != Visibility::Hidden {
                    *visibility = Visibility::Hidden;
                }
            }
            None => error!(
                "update_velocity_hud_input: entity {:?} not found in q_velocity",
                target
            ),
        }
    }
}

fn sync_orbit_state(
    mut q_orbit: Query<
        (
            &mut Transform,
            &DirectionalSphereOrbitOutput,
            &VelocityHudTargetEntity,
        ),
        (
            Changed<DirectionalSphereOrbitOutput>,
            With<VelocityHudMarker>,
        ),
    >,
    q_target: Query<&Transform, Without<VelocityHudMarker>>,
) {
    for (mut transform, output, target) in &mut q_orbit {
        let Ok(target_transform) = q_target.get(**target) else {
            error!(
                "sync_orbit_state: entity {:?} not found in q_target",
                target
            );
            continue;
        };

        let origin = target_transform.translation;
        let dir = **output;
        transform.translation = origin + dir;
        transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, dir.normalize_or_zero());
    }
}

fn direction_shader_update_system(
    gravity_settings: Res<GravitySettings>,
    q_target: Query<&LinearVelocity>,
    q_ship: Query<(&Position, Option<&DominantWell>)>,
    q_wells: Query<(&Position, &GravityWell)>,
    q_hud: Query<(Entity, &VelocityHudSource, &VelocityHudTargetEntity), With<VelocityHudMarker>>,
    q_render: Query<(
        &MeshMaterial3d<ExtendedMaterial<StandardMaterial, DirectionMagnitudeMaterial>>,
        &ChildOf,
    )>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, DirectionMagnitudeMaterial>>>,
) {
    for (material, &ChildOf(parent)) in &q_render {
        let Ok((_, &source, target)) = q_hud.get(parent) else {
            error!(
                "direction_shader_update_system: parent entity {:?} not found in q_hud",
                parent
            );
            continue;
        };

        let magnitude = match source {
            VelocityHudSource::Velocity => {
                let Ok(velocity) = q_target.get(**target) else {
                    error!(
                        "direction_shader_update_system: entity {:?} not found in q_target",
                        target
                    );
                    continue;
                };
                velocity.length() / 100.0
            }
            VelocityHudSource::Gravity => {
                // The felt pull at the ship, normalized by the cap. A
                // hidden widget (flat space) keeps its last value; the
                // root Visibility already hides it.
                let felt = q_ship.get(**target).ok().and_then(|(position, dominant)| {
                    let (well_position, well) = q_wells.get(**dominant?).ok()?;
                    let r = position.distance(**well_position);
                    Some(well_accel(
                        well.mu,
                        r,
                        well.body_radius,
                        well.soi_radius,
                        gravity_settings.fade_fraction,
                        gravity_settings.surface_margin,
                    ))
                });
                match felt {
                    Some(accel) => {
                        gravity_indicator_magnitude(accel, gravity_settings.max_surface_gravity)
                    }
                    None => continue,
                }
            }
        };

        let Some(mut material) = materials.get_mut(&**material) else {
            error!(
                "direction_shader_update_system: material for entity {:?} not found",
                parent
            );
            continue;
        };

        material.extension.magnitude_input = magnitude;
    }
}

/// Tint the velocity widget by who is flying: nav-cyan while the
/// target's [`Autopilot`] is engaged, the default white/blue in manual.
/// The palette component is the seam - materials are only touched on a
/// state flip (a per-frame write would re-upload the assets), and the
/// spawn-time palette converges on the first run, so a ship that spawns
/// already engaged wears the right colors from frame one. Gravity-source
/// widgets report the world, not control, and are never tinted.
fn sync_engaged_palette(
    q_autopilot: Query<(), With<Autopilot>>,
    mut q_hud: Query<
        (
            &mut VelocityHudPalette,
            &VelocityHudSource,
            &VelocityHudTargetEntity,
            Option<&Children>,
        ),
        With<VelocityHudMarker>,
    >,
    q_indicator: Query<
        &MeshMaterial3d<ExtendedMaterial<StandardMaterial, DirectionMagnitudeMaterial>>,
        With<VelocityHudIndicatorMarker>,
    >,
    q_sphere: Query<
        &MeshMaterial3d<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>,
        With<VelocityHudSphereMarker>,
    >,
    mut indicator_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, DirectionMagnitudeMaterial>>,
    >,
    mut sphere_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>,
    >,
) {
    for (mut palette, &source, target, children) in &mut q_hud {
        if source != VelocityHudSource::Velocity {
            continue;
        }
        let desired = desired_velocity_palette(q_autopilot.get(**target).is_ok());
        if *palette == desired {
            continue;
        }
        *palette = desired;
        for &child in children.into_iter().flatten() {
            if let Ok(material) = q_indicator.get(child) {
                if let Some(mut material) = indicator_materials.get_mut(&**material) {
                    material.base.base_color = desired.indicator;
                }
            }
            if let Ok(material) = q_sphere.get(child) {
                if let Some(mut material) = sphere_materials.get_mut(&**material) {
                    material.base.base_color = desired.sphere;
                }
            }
        }
    }
}

fn insert_velocity_hud_indicator_system(
    add: On<Add, VelocityHudMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut direction_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, DirectionMagnitudeMaterial>>,
    >,
    q_hud: Query<&VelocityHudPalette, With<VelocityHudMarker>>,
) {
    let entity = add.entity;
    trace!("insert_velocity_hud_indicator_system: entity {:?}", entity);

    let palette = q_hud.get(entity).copied().unwrap_or_default();
    commands.entity(entity).with_child((
        Name::new("VelocityHUD Indicator"),
        VelocityHudIndicatorMarker,
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        Mesh3d(meshes.add(Cone::new(0.2, 0.1))),
        MeshMaterial3d(
            direction_materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: palette.indicator,
                    perceptual_roughness: 1.0,
                    metallic: 0.0,
                    ..default()
                },
                extension: DirectionMagnitudeMaterial::default()
                    .with_max_height(1.0)
                    .with_radius(0.2),
            }),
        ),
    ));
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct DirectionMagnitudeMaterial {
    #[uniform(100)]
    pub magnitude_input: f32,
    #[uniform(100)]
    pub radius: f32,
    #[uniform(100)]
    pub max_height: f32,
    #[cfg(target_arch = "wasm32")]
    #[uniform(100)]
    _webgl2_padding_16b: u32,
}

impl DirectionMagnitudeMaterial {
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_max_height(mut self, height: f32) -> Self {
        self.max_height = height;
        self
    }
}

impl MaterialExtension for DirectionMagnitudeMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/directional_magnitude.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/directional_magnitude.wgsl".into()
    }
}

fn insert_velocity_hud_sphere_system(
    add: On<Add, VelocityHudMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut direction_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>,
    >,
    q_hud: Query<
        (
            &DirectionalSphereOrbit,
            &VelocityHudSharpness,
            &VelocityHudPalette,
        ),
        With<VelocityHudMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_velocity_hud_sphere_system: entity {:?}", entity);

    let Ok((orbit, sharpness, palette)) = q_hud.get(entity) else {
        error!(
            "insert_velocity_hud_sphere_system: entity {:?} not found in q_hud",
            entity
        );
        return;
    };

    let radius = orbit.radius;
    let mesh = TriangleMeshBuilder::new_octahedron(6).build();

    commands.entity(entity).with_child((
        Name::new("VelocityHUD Sphere"),
        VelocityHudSphereMarker,
        Transform::from_translation(Vec3::new(0.0, 0.0, radius)).with_scale(Vec3::splat(radius)),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(
            direction_materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: palette.sphere,
                    perceptual_roughness: 1.0,
                    metallic: 0.0,
                    alpha_mode: AlphaMode::Blend,
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                },
                extension: DirectionSphereMaterial::default()
                    .with_radius(radius)
                    .with_sharpness(**sharpness),
            }),
        ),
    ));
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct DirectionSphereMaterial {
    #[uniform(100)]
    pub radius: f32,
    #[uniform(100)]
    pub sharpness: f32,
    #[cfg(target_arch = "wasm32")]
    #[uniform(100)]
    _webgl2_padding_16b1: u32,
    #[cfg(target_arch = "wasm32")]
    #[uniform(100)]
    _webgl2_padding_16b2: u32,
}

impl DirectionSphereMaterial {
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_sharpness(mut self, sharpness: f32) -> Self {
        self.sharpness = sharpness;
        self
    }
}

impl MaterialExtension for DirectionSphereMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/directional_sphere.wgsl".into()
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn gravity_indicator_magnitude_normalizes_by_the_cap() {
        assert_eq!(gravity_indicator_magnitude(2.5, 5.0), 0.5);
        assert_eq!(gravity_indicator_magnitude(10.0, 5.0), 1.0, "clamped");
        assert_eq!(gravity_indicator_magnitude(0.0, 5.0), 0.0);
        // A degenerate cap must not divide by zero.
        assert_eq!(gravity_indicator_magnitude(1.0, 0.0), 1.0);
    }

    fn spawn_widget(world: &mut World, target: Entity, source: VelocityHudSource) -> Entity {
        world
            .spawn((
                velocity_hud(VelocityHudConfig {
                    target,
                    source,
                    ..default()
                }),
                // Provided by the bcs orbit plugin in the real app.
                DirectionalSphereOrbitInput(Vec3::ZERO),
            ))
            .id()
    }

    #[test]
    fn gravity_widget_points_down_the_well_and_hides_in_flat_space() {
        let mut world = World::new();
        let gravity = GravitySettings::default();
        let well = world
            .spawn((
                Position(Vec3::ZERO),
                GravityWell::from_surface_gravity(3.0, 20.0, &gravity),
            ))
            .id();
        let ship = world
            .spawn((Position(Vec3::new(50.0, 0.0, 0.0)), DominantWell(well)))
            .id();
        let widget = spawn_widget(&mut world, ship, VelocityHudSource::Gravity);

        // Spawns hidden until the feeder proves the ship is in a well.
        assert_eq!(
            *world.entity(widget).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );

        world.run_system_once(update_velocity_hud_input).unwrap();
        let input = world
            .entity(widget)
            .get::<DirectionalSphereOrbitInput>()
            .unwrap();
        assert!(
            (**input - Vec3::NEG_X).length() < 1e-5,
            "the indicator points down the pull, got {:?}",
            **input
        );
        assert_eq!(
            *world.entity(widget).get::<Visibility>().unwrap(),
            Visibility::Inherited
        );

        // Flat space: the widget hides instead of pointing at nothing.
        world.entity_mut(ship).remove::<DominantWell>();
        world.run_system_once(update_velocity_hud_input).unwrap();
        assert_eq!(
            *world.entity(widget).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );
    }

    fn palette_world() -> World {
        let mut world = World::new();
        world
            .init_resource::<Assets<ExtendedMaterial<StandardMaterial, DirectionMagnitudeMaterial>>>();
        world
            .init_resource::<Assets<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>>();
        world
    }

    fn palette_of(world: &World, widget: Entity) -> VelocityHudPalette {
        *world.entity(widget).get::<VelocityHudPalette>().unwrap()
    }

    #[test]
    fn desired_velocity_palette_picks_by_engagement() {
        assert_eq!(desired_velocity_palette(true), VelocityHudPalette::ENGAGED);
        assert_eq!(
            desired_velocity_palette(false),
            VelocityHudPalette::default()
        );
    }

    #[test]
    fn velocity_palette_follows_the_autopilot() {
        let mut world = palette_world();
        // The ship spawns already engaged: the widget must converge on
        // the first run, not wait for a state flip.
        let ship = world
            .spawn((
                LinearVelocity(Vec3::ZERO),
                Autopilot::engage(AutopilotAction::Stop),
            ))
            .id();
        let widget = spawn_widget(&mut world, ship, VelocityHudSource::Velocity);
        assert_eq!(palette_of(&world, widget), VelocityHudPalette::default());

        world.run_system_once(sync_engaged_palette).unwrap();
        assert_eq!(palette_of(&world, widget), VelocityHudPalette::ENGAGED);

        // Disengage: back to the manual white/blue.
        world.entity_mut(ship).remove::<Autopilot>();
        world.run_system_once(sync_engaged_palette).unwrap();
        assert_eq!(palette_of(&world, widget), VelocityHudPalette::default());
    }

    #[test]
    fn engaging_rewrites_the_child_materials() {
        let mut world = palette_world();
        let ship = world.spawn(LinearVelocity(Vec3::ZERO)).id();
        let widget = spawn_widget(&mut world, ship, VelocityHudSource::Velocity);

        // Hand-build the children the On<Add> observers would attach in
        // the real app, with live material assets.
        let indicator_handle = world
            .resource_mut::<Assets<ExtendedMaterial<StandardMaterial, DirectionMagnitudeMaterial>>>(
            )
            .add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: VelocityHudPalette::default().indicator,
                    ..default()
                },
                extension: DirectionMagnitudeMaterial::default(),
            });
        let sphere_handle = world
            .resource_mut::<Assets<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>>()
            .add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: VelocityHudPalette::default().sphere,
                    ..default()
                },
                extension: DirectionSphereMaterial::default(),
            });
        world.spawn((
            VelocityHudIndicatorMarker,
            MeshMaterial3d(indicator_handle.clone()),
            ChildOf(widget),
        ));
        world.spawn((
            VelocityHudSphereMarker,
            MeshMaterial3d(sphere_handle.clone()),
            ChildOf(widget),
        ));

        world
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        world.run_system_once(sync_engaged_palette).unwrap();

        let indicator_color = world
            .resource::<Assets<ExtendedMaterial<StandardMaterial, DirectionMagnitudeMaterial>>>()
            .get(&indicator_handle)
            .unwrap()
            .base
            .base_color;
        assert_eq!(indicator_color, VelocityHudPalette::ENGAGED.indicator);
        let sphere_color = world
            .resource::<Assets<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>>()
            .get(&sphere_handle)
            .unwrap()
            .base
            .base_color;
        assert_eq!(sphere_color, VelocityHudPalette::ENGAGED.sphere);
    }

    #[test]
    fn gravity_palette_never_tints() {
        let mut world = palette_world();
        let ship = world
            .spawn((
                LinearVelocity(Vec3::ZERO),
                Autopilot::engage(AutopilotAction::Stop),
            ))
            .id();
        let widget = world
            .spawn((
                velocity_hud(VelocityHudConfig {
                    target: ship,
                    source: VelocityHudSource::Gravity,
                    palette: VelocityHudPalette::GRAVITY,
                    ..default()
                }),
                DirectionalSphereOrbitInput(Vec3::ZERO),
            ))
            .id();

        world.run_system_once(sync_engaged_palette).unwrap();
        assert_eq!(
            palette_of(&world, widget),
            VelocityHudPalette::GRAVITY,
            "the gravity indicator reports the world, not who is flying"
        );
    }

    #[test]
    fn velocity_widget_behavior_is_unchanged() {
        let mut world = World::new();
        let ship = world.spawn(LinearVelocity(Vec3::new(0.0, 0.0, -8.0))).id();
        let widget = spawn_widget(&mut world, ship, VelocityHudSource::Velocity);

        world.run_system_once(update_velocity_hud_input).unwrap();
        let input = world
            .entity(widget)
            .get::<DirectionalSphereOrbitInput>()
            .unwrap();
        assert!((**input - Vec3::NEG_Z).length() < 1e-5);
        assert_eq!(
            *world.entity(widget).get::<Visibility>().unwrap(),
            Visibility::Visible,
            "the velocity readout never toggles itself"
        );
    }
}
