use super::*;

pub(super) fn insert_torpedo_section_render(
    add: On<Add, TorpedoSectionBodyMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
        Some(scene) => {
            commands.entity(entity).insert((children![(
                Name::new("Torpedo Section Body"),
                SectionRenderOf(entity),
                WorldAssetRoot(scene.clone()),
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

    if let Some(scene) = &**render_mesh {
        commands.entity(entity).insert((children![(
            Name::new("Torpedo Projectile Body"),
            SectionRenderOf(entity),
            WorldAssetRoot(scene.clone()),
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

/// An expanding, fading sphere that visualizes a blast's area of effect. Unlike the
/// hanabi particle burst (`insert_particle_effect`, wasm-blocked), this is a plain
/// mesh + `StandardMaterial`, so it renders on every target including wasm. It is the
/// blast's actual `radius` made visible: the sphere grows from a point to exactly the
/// blast radius while fading out, so the player sees how far the detonation reached.
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
    add: On<Add, BlastDamageMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sphere_mesh: Local<Option<Handle<Mesh>>>,
    q_blast: Query<(&Transform, &BlastDamageConfig), With<BlastDamageMarker>>,
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
    add: On<Add, BlastDamageMarker>,
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    q_blast: Query<(&Transform, &TorpedoSectionPartOf), With<BlastDamageMarker>>,
    q_config: Query<&TorpedoSectionConfigHelper, With<TorpedoSectionMarker>>,
) {
    let entity = add.entity;
    trace!("insert_particle_effect: entity {:?}", entity);

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
        Some(effect) => effect.clone(),
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
}
