//! Render and particle systems for the torpedo bay and its in-flight
//! projectile, gated behind the section plugin's `render` flag.

use super::*;

pub(super) fn insert_torpedo_section_render(
    add: On<Add, TorpedoSectionBodyMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    q_section: Query<&TorpedoSectionConfigHelper, With<TorpedoSectionMarker>>,
    q_body: Query<&TorpedoSectionPartOf, With<TorpedoSectionBodyMarker>>,
) {
    let entity = add.entity;
    trace!("insert_torpedo_section_render: entity {:?}", entity);

    let Ok(part_of) = q_body.get(entity) else {
        error!(
            "insert_torpedo_section_render: entity {:?} not found in q_body",
            entity
        );
        return;
    };

    let Ok(config) = q_section.get(**part_of) else {
        error!(
            "insert_torpedo_section_render: entity {:?} not found in q_section",
            entity
        );
        return;
    };
    let render_mesh = &config.render_mesh;

    match render_mesh {
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            // Authored render-mesh transform (identity when unset), on the mesh
            // CHILD so it moves the art only.
            let transform = config
                .render_mesh_transform
                .map(RenderMeshTransform::to_transform)
                .unwrap_or_default();
            commands.entity(entity).insert((children![(
                Name::new("Torpedo Section Body"),
                transform,
                SectionRenderOf(entity),
                WorldAssetRoot(scene),
            ),],));
        }
        None => {
            commands.entity(entity).insert((children![(
                Name::new("Torpedo Section Body"),
                SectionRenderOf(entity),
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                MeshMaterial3d(materials.add(Color::srgb(0.8, 0.8, 0.8))),
            ),],));
        }
    }
}

pub(super) fn insert_torpedo_render(
    add: On<Add, TorpedoProjectileMarker>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_projectile: Query<&TorpedoProjectileRenderMesh, With<TorpedoProjectileMarker>>,
) {
    let entity = add.entity;
    trace!("insert_torpedo_render: entity {:?}", entity);

    let Ok(render_mesh) = q_projectile.get(entity) else {
        error!(
            "insert_torpedo_render: entity {:?} not found in q_projectile",
            entity
        );
        return;
    };

    if let Some(asset_ref) = &**render_mesh {
        let scene = asset_ref.resolve(&asset_server);
        commands.entity(entity).insert((children![(
            Name::new("Torpedo Projectile Body"),
            SectionRenderOf(entity),
            WorldAssetRoot(scene),
        ),],));
    }
}

pub(super) fn insert_torpedo_controller_render(
    add: On<Add, TorpedoControllerMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_controller: Query<&ChildOf, With<TorpedoControllerMarker>>,
    q_torpedo: Query<&TorpedoProjectileRenderMesh, With<TorpedoProjectileMarker>>,
) {
    let entity = add.entity;
    trace!("insert_torpedo_controller_render: entity {:?}", entity);

    let Ok(ChildOf(torpedo)) = q_controller.get(entity) else {
        error!(
            "insert_torpedo_controller_render: entity {:?} not found in q_controller",
            entity
        );
        return;
    };

    let Ok(render_mesh) = q_torpedo.get(*torpedo) else {
        error!(
            "insert_torpedo_controller_render: entity {:?} not found in q_torpedo",
            *torpedo
        );
        return;
    };

    if render_mesh.is_some() {
        // If the torpedo has a render mesh, we skip rendering the controller
        return;
    }

    commands.entity(entity).insert((
        Mesh3d(meshes.add(Cylinder::new(0.2, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.8, 0.8))),
    ));
}

/// An expanding, fading sphere that visualizes a blast's area of effect. It
/// complements the hanabi detonation burst (`insert_particle_effect`): where the
/// burst is spray, this plain mesh + `StandardMaterial` is the blast's actual
/// `radius` made visible - the sphere grows from a point to exactly the blast
/// radius while fading out, so the player sees how far the detonation reached.
/// Being a mesh (no compute), it also stays visible if particles are ever off.
#[derive(Component, Debug, Clone, Reflect)]
pub(super) struct BlastRadiusVisual {
    /// Full blast radius the sphere expands to reach, in world units.
    radius: f32,
    /// Seconds elapsed since the detonation.
    elapsed: f32,
    /// Total lifetime of the visual, in seconds.
    duration: f32,
    /// This visual's own material, faded each frame and freed on despawn.
    material: Handle<StandardMaterial>,
}

/// Spawn the wasm-safe expanding-sphere blast visual when a blast sensor appears.
///
/// The sphere mesh is a unit sphere shared across all blasts (cached in a `Local`);
/// only the material is per-instance so each blast can fade independently.
pub(super) fn insert_blast_radius_visual(
    add: On<Add, NovaBlast>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sphere_mesh: Local<Option<Handle<Mesh>>>,
    q_blast: Query<(&Transform, &NovaBlast)>,
) {
    let entity = add.entity;
    trace!("insert_blast_radius_visual: entity {:?}", entity);

    let Ok((blast_transform, config)) = q_blast.get(entity) else {
        error!(
            "insert_blast_radius_visual: entity {:?} not found in q_blast",
            entity
        );
        return;
    };

    let mesh = sphere_mesh
        .get_or_insert_with(|| meshes.add(Sphere::new(1.0)))
        .clone();

    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.55, 0.15, 0.35),
        emissive: LinearRgba::rgb(4.0, 1.6, 0.3),
        alpha_mode: bevy::prelude::AlphaMode::Blend,
        // Render both faces so the shell is visible from inside the blast too.
        cull_mode: None,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Name::new("Blast Radius Visual"),
        BlastRadiusVisual {
            radius: config.radius,
            elapsed: 0.0,
            duration: 0.4,
            material: material.clone(),
        },
        Mesh3d(mesh),
        MeshMaterial3d(material),
        // Start at a point; `animate_blast_radius_visual` grows it to `radius`.
        Transform::from_translation(blast_transform.translation).with_scale(Vec3::ZERO),
    ));
}

/// Base alpha of the blast shell at the start of its life (before it fades out).
const BLAST_VISUAL_BASE_ALPHA: f32 = 0.35;

/// The blast visual's world radius and fade factor at normalized time `t` in `[0, 1]`.
///
/// The radius follows an ease-out cubic (a quick punch outward that settles at the
/// full `radius`); the fade goes linearly from 1 (opaque) at `t = 0` to 0 at `t = 1`.
/// Pure, so the growth/fade curve is unit-tested without a render world.
fn blast_visual_step(radius: f32, t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - t).powi(3);
    (radius * eased, 1.0 - t)
}

/// Expand each blast visual out to its radius while fading it, then despawn it (and
/// free its one-off material so the assets do not accumulate over a long session).
pub(super) fn animate_blast_radius_visual(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q_visual: Query<(Entity, &mut Transform, &mut BlastRadiusVisual)>,
) {
    for (entity, mut transform, mut visual) in &mut q_visual {
        visual.elapsed += time.delta_secs();
        let t = visual.elapsed / visual.duration;

        if t >= 1.0 {
            materials.remove(&visual.material);
            commands.entity(entity).despawn();
            continue;
        }

        let (scale, fade) = blast_visual_step(visual.radius, t);
        transform.scale = Vec3::splat(scale);

        // Fade the shell (and its glow) to nothing over the lifetime.
        if let Some(mut material) = materials.get_mut(&visual.material) {
            material
                .base_color
                .set_alpha(BLAST_VISUAL_BASE_ALPHA * fade);
            material.emissive = LinearRgba::rgb(4.0 * fade, 1.6 * fade, 0.3 * fade);
        }
    }
}

pub(super) fn insert_particle_effect(
    add: On<Add, NovaBlast>,
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    asset_server: Res<AssetServer>,
    budget: Option<Res<GraphicsBudget>>,
    q_blast: Query<(&Transform, &TorpedoSectionPartOf), With<NovaBlast>>,
    q_config: Query<&TorpedoSectionConfigHelper, With<TorpedoSectionMarker>>,
) {
    let entity = add.entity;
    trace!("insert_particle_effect: entity {:?}", entity);

    // Low graphics tier is spawn-less: skip the hanabi blast entirely (task
    // 20260525-133013). Absent budget (settings-less app) means full quality.
    if !budget.as_deref().is_none_or(|b| b.particles) {
        return;
    }

    let Ok((blast_transform, TorpedoSectionPartOf(torpedo_section))) = q_blast.get(entity) else {
        error!(
            "insert_particle_effect: entity {:?} not found in q_blast",
            entity
        );
        return;
    };

    let Ok(config) = q_config.get(*torpedo_section) else {
        error!(
            "insert_turret_barrel_muzzle_effect: entity {:?} not found in q_effect",
            entity
        );
        return;
    };

    let effect = match &config.blast_effect {
        Some(asset_ref) => asset_ref.resolve(&asset_server),
        None => {
            let spawner = SpawnerSettings::once(400.0.into())
                // In this case we want to emit on start to create an instantaneous explosion
                .with_emit_on_start(true);

            let writer = ExprWriter::new();

            let age = writer.lit(0.).expr();
            let init_age = SetAttributeModifier::new(Attribute::AGE, age);

            // Lifetime: explosion should be fast but noticeable
            let lifetime = writer.lit(0.25).uniform(writer.lit(1.5)).expr();
            let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

            // Color over lifetime
            let mut color_gradient = bevy_hanabi::Gradient::new();
            // t=0: bright yellow/white
            color_gradient.add_key(0.0, Vec4::new(1.0, 0.95, 0.7, 1.0));
            // mid: hot orange
            color_gradient.add_key(0.3, Vec4::new(1.0, 0.6, 0.1, 0.7));
            // end: dark, almost transparent smoke
            color_gradient.add_key(1.0, Vec4::new(0.1, 0.1, 0.1, 0.0));

            let color_over_lifetime = ColorOverLifetimeModifier {
                gradient: color_gradient,
                blend: ColorBlendMode::default(),
                mask: ColorBlendMask::default(),
            };

            let init_color =
                SetAttributeModifier::new(Attribute::COLOR, writer.lit(0xFFFFFFFFu32).expr());

            // Size over lifetime: fast expansion then shrink/fade
            let mut size_gradient = bevy_hanabi::Gradient::new();
            size_gradient.add_key(0.0, Vec3::splat(0.02)); // just spawned
            size_gradient.add_key(0.1, Vec3::splat(0.2)); // big boom
            size_gradient.add_key(0.5, Vec3::splat(0.25)); // lingering cloud
            size_gradient.add_key(1.0, Vec3::splat(0.0)); // disappear

            let size_over_lifetime = SizeOverLifetimeModifier {
                gradient: size_gradient,
                screen_space_size: false,
            };

            // Position: explosion center
            let init_pos =
                SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());

            // Velocity: spherical random burst
            let rand_x = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
            let rand_y = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
            let rand_z = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);

            let dir = writer.lit(Vec3::X) * rand_x
                + writer.lit(Vec3::Y) * rand_y
                + writer.lit(Vec3::Z) * rand_z;

            let speed = writer.lit(20.0).uniform(writer.lit(30.0));
            let velocity = dir * speed;
            let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

            effects.add(
                EffectAsset::new(32768, spawner, writer.finish())
                    .with_name("spawn_on_blast_explosion")
                    .init(init_pos)
                    .init(init_vel)
                    .init(init_age)
                    .init(init_lifetime)
                    .init(init_color)
                    .render(size_over_lifetime)
                    .render(color_over_lifetime),
            )
        }
    };

    commands.spawn(((
        Name::new("Blast Effect"),
        TorpedoBlastEffectMarker,
        Transform::from_translation(blast_transform.translation),
        ParticleEffect::new(effect),
        EffectProperties::default(),
        TempEntity(2.0),
    ),));
}

/// Build the launch particle burst on the bay spawner when the spawner entity is
/// added. Mirrors the turret's `insert_turret_barrel_muzzle_effect`: a
/// spawn-on-command effect (emit-on-start `false`) parented to the spawner, so
/// `on_torpedo_launch_effect` can fire it with `EffectSpawner::reset()`. When the
/// config supplies a `launch_effect` we use it; otherwise we build a default
/// cold white-blue propellant flash sprayed forward along the launch tube.
pub(super) fn insert_torpedo_spawner_effect(
    add: On<Add, TorpedoSectionSpawnerMarker>,
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    asset_server: Res<AssetServer>,
    budget: Option<Res<GraphicsBudget>>,
    q_effect: Query<&TorpedoSectionSpawnerEffect, With<TorpedoSectionSpawnerMarker>>,
) {
    let entity = add.entity;
    trace!("insert_torpedo_spawner_effect: entity {:?}", entity);

    // Low graphics tier is spawn-less: skip the launch-burst hanabi (task
    // 20260525-133013). Absent budget (settings-less app) means full quality.
    if !budget.as_deref().is_none_or(|b| b.particles) {
        return;
    }

    let Ok(effect_handle) = q_effect.get(entity) else {
        error!(
            "insert_torpedo_spawner_effect: entity {:?} not found in q_effect",
            entity
        );
        return;
    };

    let effect = match &**effect_handle {
        Some(asset_ref) => asset_ref.resolve(&asset_server),
        None => {
            // Emit a fixed-size burst only when reset() is called (per launch),
            // never automatically on spawn.
            let spawner = SpawnerSettings::once(80.0.into()).with_emit_on_start(false);

            let writer = ExprWriter::new();

            let age = writer.lit(0.).expr();
            let init_age = SetAttributeModifier::new(Attribute::AGE, age);

            // A short-lived puff, with per-particle variation so it does not read
            // as a single hard flash.
            let lifetime = writer.lit(0.1).uniform(writer.lit(0.35)).expr();
            let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

            // Cold propellant flash: bright white-blue core fading to a dim blue
            // haze, distinct from the turret's hot-orange muzzle flash.
            let mut color_gradient = bevy_hanabi::Gradient::new();
            color_gradient.add_key(0.0, Vec4::new(0.8, 0.9, 1.0, 1.0));
            color_gradient.add_key(0.3, Vec4::new(0.3, 0.5, 1.0, 0.8));
            color_gradient.add_key(1.0, Vec4::new(0.05, 0.05, 0.2, 0.0));
            let color_over_lifetime = ColorOverLifetimeModifier {
                gradient: color_gradient,
                blend: ColorBlendMode::default(),
                mask: ColorBlendMask::default(),
            };

            // A small world-space puff that expands then fades, so it reads at the
            // bay's scale rather than as a cluster of screen-space dots.
            let mut size_gradient = bevy_hanabi::Gradient::new();
            size_gradient.add_key(0.0, Vec3::splat(0.03));
            size_gradient.add_key(0.2, Vec3::splat(0.22));
            size_gradient.add_key(0.6, Vec3::splat(0.18));
            size_gradient.add_key(1.0, Vec3::splat(0.0));
            let size_over_lifetime = SizeOverLifetimeModifier {
                gradient: size_gradient,
                screen_space_size: false,
            };

            let init_pos =
                SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());

            // Launch direction, set per shot from the spawner's forward (`up`) axis.
            let normal = writer.add_property("normal", Vec3::ZERO.into());
            let normal = writer.prop(normal);

            // Ship motion the burst rides along with, set per shot.
            let base_velocity = writer.add_property("base_velocity", Vec3::ZERO.into());
            let base_velocity = writer.prop(base_velocity);

            // Forward-biased cone: mostly along the launch normal with a little
            // spread, so the flash sprays out of the tube.
            let spread_x = (writer.rand(ScalarType::Float) - writer.lit(0.5)) * writer.lit(0.4);
            let spread_y = (writer.rand(ScalarType::Float) - writer.lit(0.5)) * writer.lit(0.4);
            let spread_z = (writer.rand(ScalarType::Float) - writer.lit(0.5)) * writer.lit(0.4);
            let spread = writer.lit(Vec3::X) * spread_x
                + writer.lit(Vec3::Y) * spread_y
                + writer.lit(Vec3::Z) * spread_z;
            let speed = writer.rand(ScalarType::Float) * writer.lit(8.0) + writer.lit(4.0);
            let velocity = (normal + spread).normalized() * speed + base_velocity;
            let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

            effects.add(
                EffectAsset::new(32768, spawner, writer.finish())
                    .with_name("torpedo_launch_burst")
                    .init(init_pos)
                    .init(init_vel)
                    .init(init_age)
                    .init(init_lifetime)
                    .render(size_over_lifetime)
                    .render(color_over_lifetime),
            )
        }
    };

    commands.entity(entity).insert((children![(
        Name::new("Torpedo Launch Effect"),
        TorpedoSectionSpawnerEffectMarker,
        ParticleEffect::new(effect),
        EffectProperties::default(),
    ),],));
}

/// Fire the bay's launch burst when a torpedo projectile is spawned. Mirrors the
/// turret's `on_projectile_marker_effect`: the projectile carries its spawner
/// entity, so we look up that spawner's child effect, point the burst along the
/// spawner's launch axis, and `reset()` the spawner to emit one puff.
pub(super) fn on_torpedo_launch_effect(
    add: On<Add, TorpedoProjectileMarker>,
    budget: Option<Res<GraphicsBudget>>,
    q_projectile: Query<&TorpedoSectionSpawnerEntity, With<TorpedoProjectileMarker>>,
    mut q_effect: Query<
        (&mut EffectProperties, &mut EffectSpawner, &ChildOf),
        (
            With<TorpedoSectionSpawnerEffectMarker>,
            Without<TorpedoSectionSpawnerMarker>,
        ),
    >,
    // TransformHelper computes the spawner's global transform; only runs once per
    // shot, so the cost is fine.
    transform_helper: TransformHelper,
) {
    let projectile = add.entity;
    trace!("on_torpedo_launch_effect: entity {:?}", projectile);

    // On the Low tier `insert_torpedo_spawner_effect` never spawned the launch
    // effect, so there is nothing to reset - skip before the lookup, otherwise the
    // missing-effect branch below would `error!` on every launch (task 20260525-133013).
    if !budget.as_deref().is_none_or(|b| b.particles) {
        return;
    }

    let Ok(spawner) = q_projectile.get(projectile) else {
        error!(
            "on_torpedo_launch_effect: entity {:?} not found in q_projectile",
            projectile
        );
        return;
    };

    let Ok(spawner_transform) = transform_helper.compute_global_transform(**spawner) else {
        error!(
            "on_torpedo_launch_effect: entity {:?} global transform not found",
            **spawner
        );
        return;
    };

    let Some((mut properties, mut effect_spawner, _)) = q_effect
        .iter_mut()
        .find(|(_, _, &ChildOf(parent))| parent == **spawner)
    else {
        error!(
            "on_torpedo_launch_effect: effect for spawner {:?} not found",
            **spawner
        );
        return;
    };

    // The launch axis is the spawner's forward (`up`), matching the direction
    // `shoot_spawn_projectile` gives the torpedo. `up()` is already a unit `Dir3`.
    let normal = spawner_transform.up();
    properties.set("normal", Vec3::from(normal).into());
    // Currently always zero; a hook to later ride the burst along ship motion.
    properties.set("base_velocity", Vec3::ZERO.into());

    effect_spawner.reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blast_visual_starts_at_a_point_fully_opaque() {
        // t = 0: no size yet, full opacity - the flash begins as a bright point.
        let (scale, fade) = blast_visual_step(6.0, 0.0);
        assert_eq!(scale, 0.0);
        assert_eq!(fade, 1.0);
    }

    #[test]
    fn blast_visual_ends_at_full_radius_fully_faded() {
        // t = 1: the sphere has reached exactly the blast radius and faded out.
        let (scale, fade) = blast_visual_step(6.0, 1.0);
        assert!((scale - 6.0).abs() < 1e-6, "scale should reach the radius");
        assert_eq!(fade, 0.0);
    }

    #[test]
    fn blast_visual_grows_and_fades_monotonically() {
        // Sampling forward in time, the shell only ever grows and only ever fades.
        let radius = 6.0;
        let mut prev = blast_visual_step(radius, 0.0);
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            let step = blast_visual_step(radius, t);
            assert!(step.0 >= prev.0, "radius must not shrink");
            assert!(step.1 <= prev.1, "opacity must not increase");
            assert!(step.0 <= radius + 1e-6, "never exceeds the blast radius");
            prev = step;
        }
    }

    #[test]
    fn blast_visual_step_clamps_out_of_range_time() {
        let radius = 6.0;
        assert_eq!(
            blast_visual_step(radius, -0.5),
            blast_visual_step(radius, 0.0)
        );
        assert_eq!(
            blast_visual_step(radius, 2.0),
            blast_visual_step(radius, 1.0)
        );
    }

    /// The torpedo bay reads its `render_mesh_transform` STRAIGHT OFF THE CONFIG
    /// (unlike hull/thruster/controller which snapshot it into a component), so
    /// this exercises that distinct path end to end: the authored transform must
    /// land on the meshed body render child, identity when unset (task
    /// 20260718-121205).
    #[test]
    fn render_mesh_transform_positions_the_torpedo_body_render_child() {
        use bevy::asset::AssetPlugin;

        let child_transform = |xf: Option<RenderMeshTransform>| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();
            app.init_asset::<WorldAsset>();
            // insert_torpedo_section spawns the body; the render observer meshes it.
            app.add_observer(insert_torpedo_section);
            app.add_observer(insert_torpedo_section_render);
            app.world_mut().spawn((
                TorpedoSectionMarker,
                Transform::default(),
                TorpedoSectionConfigHelper(TorpedoSectionConfig {
                    render_mesh: Some(AssetRef::from("gltf/torpedo-bay-01.glb#Scene0".to_string())),
                    render_mesh_transform: xf,
                    ..Default::default()
                }),
            ));
            app.world_mut().flush();
            app.update();

            let world = app.world_mut();
            let mut q = world.query_filtered::<&Transform, With<SectionRenderOf>>();
            let found: Vec<Transform> = q.iter(world).copied().collect();
            assert_eq!(
                found.len(),
                1,
                "one meshed torpedo body render child expected"
            );
            found[0]
        };

        let authored = RenderMeshTransform {
            position: Vec3::new(0.0, 0.3, -0.2),
            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_4),
        };
        let got = child_transform(Some(authored));
        assert_eq!(got.translation, authored.position);
        assert!(got.rotation.abs_diff_eq(authored.rotation, 1e-5));

        assert_eq!(child_transform(None), Transform::IDENTITY);
    }
}
