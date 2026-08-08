//! Turret render children: joint meshes, the projectile mesh, and the
//! muzzle-flash and bullet-trail effects.

use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use super::*;

pub(super) fn on_projectile_marker_effect(
    add: On<Add, TurretBulletProjectileMarker>,
    budget: Option<Res<GraphicsBudget>>,
    q_projectile: Query<&TurretSectionMuzzleEntity, With<TurretBulletProjectileMarker>>,
    mut q_effect: Query<
        (&mut EffectProperties, &mut EffectSpawner, &ChildOf),
        (
            With<TurretSectionBarrelMuzzleEffectMarker>,
            Without<TurretSectionBarrelMuzzleMarker>,
        ),
    >,
    // We are using TransformHelper here because we need to compute the global transform; And it
    // should be fine, since it will not be called frequently.
    transform_helper: TransformHelper,
) {
    let projectile = add.entity;
    trace!("on_projectile_marker: entity {:?}", projectile);

    // On the Low tier `insert_turret_barrel_muzzle_effect` never spawned the muzzle
    // effect, so there is nothing to reset - skip before the lookup, otherwise the
    // missing-effect branch below would `error!` on every shot.
    if !budget.as_deref().is_none_or(|b| b.particles) {
        return;
    }

    let Ok(muzzle) = q_projectile.get(projectile) else {
        error!(
            "on_projectile_marker: entity {:?} not found in q_projectile",
            projectile
        );
        return;
    };

    let Ok(muzzle_transform) = transform_helper.compute_global_transform(**muzzle) else {
        error!(
            "on_projectile_marker_effect: entity {:?} global transform not found",
            **muzzle
        );
        return;
    };

    // Spawn the effect muzzle
    let Some((mut properties, mut effect_spawner, _)) = q_effect
        .iter_mut()
        .find(|(_, _, &ChildOf(parent))| parent == **muzzle)
    else {
        error!(
            "on_shoot_spawn_projectile: effect for muzzle {:?} not found",
            **muzzle
        );
        return;
    };

    let normal = muzzle_transform.forward();

    let p: f32 = rand::random();

    let (r, g, b) = if p < 0.4 {
        let r = 255;
        let g = 240 + rand::random_range(0..16);
        let b = 200 + rand::random_range(0..56);
        (r, g, b)
    } else if p < 0.75 {
        let r = 255;
        let g = rand::random_range(100..180);
        let b = 0;
        (r, g, b)
    } else if p < 0.95 {
        let r = 255;
        let g = rand::random_range(50..120);
        let b = 0;
        (r, g, b)
    } else {
        let val = rand::random_range(30..80);
        (val, val, val)
    };
    let color = 0xFF000000u32 | ((b as u32) << 16) | ((g as u32) << 8) | (r as u32);
    properties.set("spawn_color", color.into());

    // Set the collision normal
    let normal = normal.normalize();
    properties.set("normal", normal.into());

    let base_velocity = Vec3::ZERO;
    properties.set("base_velocity", base_velocity.into());

    // Spawn the particles
    effect_spawner.reset();
}

/// The default bullet's mesh and material, built once.
///
/// The `None` arm of [`insert_projectile_render`] is the shipped path - every
/// stock turret authors no projectile mesh - and the default turret fires 100
/// rounds/s per muzzle, so allocating there allocated a mesh and a material per
/// shot.
///
/// Built through [`FromWorld`] rather than a `Startup` system on purpose: the
/// observer below takes it as a plain `Res`, so a turret that spawns before a
/// startup command flush would miss the resource and hard-error under the
/// `FallbackErrorHandler(panic)` the autopilot and probe runs install.
#[derive(Resource, Debug)]
pub(crate) struct DefaultProjectileRender {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

impl FromWorld for DefaultProjectileRender {
    fn from_world(world: &mut World) -> Self {
        let mesh = world
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::new(0.05, 0.05, 0.3));
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(Color::srgb(1.0, 0.9, 0.2));
        Self { mesh, material }
    }
}

pub(super) fn insert_projectile_render(
    add: On<Add, TurretBulletProjectileMarker>,
    mut commands: Commands,
    default_render: Res<DefaultProjectileRender>,
    asset_server: Res<AssetServer>,
    q_render_mesh: Query<&BulletProjectileRenderMesh>,
) {
    let entity = add.entity;
    trace!("insert_projectile_render: entity {:?}", entity);

    let Ok(render_mesh) = q_render_mesh.get(entity) else {
        error!(
            "insert_projectile_render: entity {:?} not found in q_render_mesh",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(asset_ref) => {
            let scene_handle = asset_ref.resolve(&asset_server);
            commands.entity(entity).insert((children![(
                Name::new("Bullet Projectile Render"),
                WorldAssetRoot(scene_handle),
            ),],));
        }
        None => {
            commands.entity(entity).insert((children![(
                Name::new("Bullet Projectile Render"),
                Mesh3d(default_render.mesh.clone()),
                MeshMaterial3d(default_render.material.clone()),
            ),],));
        }
    }
}

/// Generic joint render (replaces the four per-type render observers). Fires on
/// `Add, TurretJointMarker` (gated by `self.render`). If the joint authored a
/// mesh, spawn a `WorldAssetRoot` child; otherwise spawn a small generic
/// default primitive so an unmeshed joint is still visible. The old bespoke
/// per-type placeholder art (ridged yaw/pitch cylinders, layered barrel shape)
/// is dropped in favor of one default; shipped turrets author GLB meshes so the
/// visible game is unchanged.
pub(super) fn insert_turret_joint_render(
    add: On<Add, TurretJointMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    q_joint: Query<
        (
            &TurretSectionPartOf,
            &TurretJointRenderMesh,
            &TurretJointRenderMeshTransform,
            Has<TurretSectionBarrelMuzzleMarker>,
        ),
        With<TurretJointMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_turret_joint_render: entity {:?}", entity);

    let Ok((turret, render_mesh, render_mesh_transform, is_muzzle)) = q_joint.get(entity) else {
        error!(
            "insert_turret_joint_render: entity {:?} not found in q_joint",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            // Authored render-mesh transform, or identity (mesh at the joint
            // origin) when unset. It lives on the mesh CHILD, so it moves only
            // the art, never the joint's kinematic frame.
            let transform = render_mesh_transform
                .map(RenderMeshTransform::to_transform)
                .unwrap_or_default();
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Joint"),
                transform,
                SectionRenderOf(**turret),
                WorldAssetRoot(scene),
            ),],));
        }
        // A muzzle is an invisible fire point (the original never rendered it);
        // only a STRUCTURAL unmeshed joint (the base plate) gets a default
        // primitive so the mount is not floating meshes with a gap under it. The
        // shape matches the pre-refactor base plate (a wide flat disc slightly
        // above the joint origin) so an unmeshed base looks exactly as it did.
        None if !is_muzzle => {
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Joint"),
                Transform::from_xyz(0.0, 0.05, 0.0),
                SectionRenderOf(**turret),
                Mesh3d(meshes.add(Cylinder::new(0.5, 0.1))),
                MeshMaterial3d(materials.add(Color::srgb(0.25, 0.25, 0.25))),
            ),],));
        }
        None => {}
    }
}

pub(super) fn insert_turret_barrel_muzzle_effect(
    add: On<Add, TurretSectionBarrelMuzzleMarker>,
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    asset_server: Res<AssetServer>,
    budget: Option<Res<GraphicsBudget>>,
    q_effect: Query<&TurretSectionBarrelMuzzleEffect, With<TurretSectionBarrelMuzzleMarker>>,
) {
    let entity = add.entity;
    trace!("insert_turret_barrel_muzzle_effect: entity {:?}", entity);

    // Low graphics tier is spawn-less: skip the muzzle-flash hanabi. Absent
    // budget (settings-less app) means full quality.
    if !budget.as_deref().is_none_or(|b| b.particles) {
        return;
    }

    let Ok(effect_handle) = q_effect.get(entity) else {
        error!(
            "insert_turret_barrel_muzzle_effect: entity {:?} not found in q_effect",
            entity
        );
        return;
    };

    match &**effect_handle {
        Some(asset_ref) => {
            let effect = asset_ref.resolve(&asset_server);
            commands.entity(entity).insert((children![(
                Name::new("Muzzle Effect"),
                TurretSectionBarrelMuzzleEffectMarker,
                ParticleEffect::new(effect),
                EffectProperties::default(),
            ),],));
        }
        None => {
            let spawner = SpawnerSettings::once(100.0.into())
                // NOTE: do not emit on instantiation - the muzzle flash only
                // fires when the shot calls reset().
                .with_emit_on_start(false);

            let writer = ExprWriter::new();

            let age = writer.lit(0.).expr();
            let init_age = SetAttributeModifier::new(Attribute::AGE, age);

            // Give a bit of variation by randomizing the lifetime per particle
            let lifetime = writer.lit(0.01).uniform(writer.lit(0.1)).expr();
            let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

            // Attribute::COLOR saves the property value PER PARTICLE at spawn,
            // so a later property change leaves live particles alone.
            let spawn_color = writer.add_property("spawn_color", 0xFFFFFFFFu32.into());
            let color = writer.prop(spawn_color).expr();
            let init_color = SetAttributeModifier::new(Attribute::COLOR, color);

            let normal = writer.add_property("normal", Vec3::ZERO.into());
            let normal = writer.prop(normal);

            let base_velocity = writer.add_property("base_velocity", Vec3::ZERO.into());
            let base_velocity = writer.prop(base_velocity);

            let pos = writer.lit(Vec3::ZERO);
            let init_pos = SetAttributeModifier::new(Attribute::POSITION, pos.expr());

            // A random direction mostly along the muzzle normal, with a little
            // spread - cheaper than bounding the spray with a KillAabbModifier,
            // which would spawn particles only to kill them.
            let spread_x = (writer.rand(ScalarType::Float) - writer.lit(0.5)) * writer.lit(0.2);
            let spread_y = (writer.rand(ScalarType::Float) - writer.lit(0.5)) * writer.lit(0.2);
            let spread_z = (writer.rand(ScalarType::Float) - writer.lit(0.5)) * writer.lit(0.2);
            let spread = writer.lit(Vec3::X) * spread_x
                + writer.lit(Vec3::Y) * spread_y
                + writer.lit(Vec3::Z) * spread_z;
            let speed = writer.rand(ScalarType::Float) * writer.lit(5.0);
            let velocity = (normal + spread * writer.lit(2.5)).normalized() * speed;
            let velocity = velocity + base_velocity;
            let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

            let effect = effects.add(
                EffectAsset::new(32768, spawner, writer.finish())
                    .with_name("spawn_on_command")
                    .init(init_pos)
                    .init(init_vel)
                    .init(init_age)
                    .init(init_lifetime)
                    .init(init_color)
                    // Set a size of 3 (logical) pixels, constant in screen space, independent of projection
                    .render(SetSizeModifier {
                        size: Vec3::splat(3.).into(),
                    })
                    .render(ScreenSpaceSizeModifier),
            );

            commands.entity(entity).insert((children![(
                Name::new("Muzzle Effect"),
                TurretSectionBarrelMuzzleEffectMarker,
                ParticleEffect::new(effect),
                EffectProperties::default(),
            ),],));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{config::default_joint_speed, test_support::*},
        *,
    };

    /// The `None` arm is the shipped path and the default turret fires 100
    /// rounds/s per muzzle. Every bullet must reuse the one shared mesh and
    /// material instead of adding two assets per shot.
    ///
    /// The first bullet is spawned BEFORE any update, which is what a `Startup`
    /// system could not serve: the resource has to exist the moment the
    /// observer runs.
    #[test]
    fn default_projectile_render_allocates_no_assets_per_shot() {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_resource::<DefaultProjectileRender>();
        app.add_observer(insert_projectile_render);
        app.update();

        let assets_now = |app: &App| {
            (
                app.world().resource::<Assets<Mesh>>().len(),
                app.world().resource::<Assets<StandardMaterial>>().len(),
            )
        };
        let before = assets_now(&app);
        assert_eq!(
            before,
            (1, 1),
            "exactly one shared mesh + material is built"
        );

        // No update has flushed yet on this entity's behalf beyond the one
        // above; the observer must already find the resource.
        app.world_mut().spawn((
            TurretBulletProjectileMarker,
            BulletProjectileRenderMesh(None),
        ));
        app.update();
        assert_eq!(
            assets_now(&app),
            before,
            "a bullet spawned with no startup flush reuses the shared assets"
        );

        for _ in 0..64 {
            app.world_mut().spawn((
                TurretBulletProjectileMarker,
                BulletProjectileRenderMesh(None),
            ));
            app.update();
        }

        assert_eq!(
            assets_now(&app),
            before,
            "firing must not add mesh/material assets"
        );

        // ... and every bullet actually got the shared handles.
        let world = app.world_mut();
        let mut q =
            world.query_filtered::<(&Mesh3d, &MeshMaterial3d<StandardMaterial>), With<Name>>();
        let children: Vec<_> = q
            .iter(world)
            .map(|(m, s)| (m.0.clone(), s.0.clone()))
            .collect();
        // 64 in the loop plus the pre-flush bullet above.
        assert_eq!(children.len(), 65, "one render child per bullet");
        assert!(
            children.iter().all(|h| *h == children[0]),
            "every bullet shares the same mesh and material handle"
        );
    }

    #[test]
    fn every_turret_joint_render_child_is_parented_to_its_joint() {
        // BASE-FLOATING REGRESSION: the base (and every unmeshed fixed joint)
        // renders a default primitive as a CHILD of the joint entity. If that
        // render child is not actually parented (ChildOf == joint), it drifts to
        // world origin instead of riding the ship - the "base floats" report.
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_observer(insert_turret_section);
        app.add_observer(insert_turret_joint_render);

        // Place the turret far from the world origin, like a section on a flying
        // ship. If a render child is detached, its GlobalTransform stays near the
        // origin - hundreds of units from where the turret actually is.
        let ship_pos = Vec3::new(100.0, 50.0, 200.0);
        let turret = app
            .world_mut()
            .spawn((turret_section(TurretSectionConfig::default()),))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert(Transform::from_translation(ship_pos));
        app.world_mut().flush();
        app.update(); // propagate transforms

        for joint in joint_entities(&app) {
            let render_children: Vec<Entity> = app
                .world()
                .get::<Children>(joint)
                .map(|c| {
                    c.iter()
                        .filter(|&e| app.world().get::<SectionRenderOf>(e).is_some())
                        .collect()
                })
                .unwrap_or_default();
            // A muzzle is an invisible fire point (no render child); every other
            // joint renders and must be parented on the ship.
            if app
                .world()
                .get::<TurretSectionBarrelMuzzleMarker>(joint)
                .is_some()
            {
                assert!(
                    render_children.is_empty(),
                    "muzzle joint {joint:?} should be invisible but has a render child"
                );
                continue;
            }
            assert!(
                !render_children.is_empty(),
                "joint {joint:?} has no render child"
            );
            for rc in render_children {
                let parent = app.world().get::<ChildOf>(rc).map(|c| c.0);
                assert_eq!(
                    parent,
                    Some(joint),
                    "render child {rc:?} is not parented to its joint {joint:?}"
                );
                // The whole turret assembly spans ~2 units; a correctly mounted
                // render child sits within that of the turret's world position.
                let world = app
                    .world()
                    .get::<GlobalTransform>(rc)
                    .map(|g| g.translation())
                    .unwrap_or(Vec3::ZERO);
                assert!(
                    world.distance(ship_pos) < 5.0,
                    "render child {rc:?} of joint {joint:?} is at {world:?}, {} units \
                     from the turret at {ship_pos:?} - it floats",
                    world.distance(ship_pos)
                );
            }
        }

        // FOLLOW CHECK: move the turret (like a flying ship) and confirm every
        // render child rode along instead of staying behind at the old spot - a
        // detached child "floats" in place while the ship flies off.
        let moved = Vec3::new(-400.0, 900.0, -50.0);
        app.world_mut()
            .entity_mut(turret)
            .insert(Transform::from_translation(moved));
        app.update();
        for joint in joint_entities(&app) {
            let render_children: Vec<Entity> = app
                .world()
                .get::<Children>(joint)
                .map(|c| {
                    c.iter()
                        .filter(|&e| app.world().get::<SectionRenderOf>(e).is_some())
                        .collect()
                })
                .unwrap_or_default();
            for rc in render_children {
                let world = app
                    .world()
                    .get::<GlobalTransform>(rc)
                    .map(|g| g.translation())
                    .unwrap_or(Vec3::ZERO);
                assert!(
                    world.distance(moved) < 5.0,
                    "render child {rc:?} of joint {joint:?} did not follow the turret \
                     to {moved:?} (it is at {world:?}) - it floats"
                );
            }
        }
    }

    /// A meshed joint's render child carries the authored
    /// `render_mesh_transform`, and a meshed joint that omits it gets an
    /// identity transform - the pre-feature behavior. This is
    /// the load-bearing wiring: the transform must land on the mesh CHILD, not
    /// the joint entity (whose transform is the kinematic frame).
    #[test]
    fn render_mesh_transform_positions_the_meshed_render_child() {
        use bevy::asset::AssetPlugin;

        // A one-joint turret (fixed root + a muzzle leaf so it is a valid
        // turret) whose root carries a mesh and the given transform.
        let turret_with = |xf: Option<RenderMeshTransform>| TurretSectionConfig {
            root: TurretJoint {
                offset: Vec3::ZERO,
                axis: None,
                speed: default_joint_speed(),
                min: None,
                max: None,
                render_mesh: Some(AssetRef::from("gltf/turret-yaw-01.glb#Scene0".to_string())),
                render_mesh_transform: xf,
                muzzle: None,
                children: vec![TurretJoint {
                    offset: Vec3::new(0.0, 0.0, -0.5),
                    axis: None,
                    speed: default_joint_speed(),
                    min: None,
                    max: None,
                    render_mesh: None,
                    render_mesh_transform: None,
                    muzzle: Some(MuzzleConfig {
                        fire_rate: 100.0,
                        muzzle_effect: None,
                    }),
                    children: vec![],
                }],
            },
            ..Default::default()
        };

        // The single WorldAssetRoot (meshed) render child's local Transform.
        let meshed_child_transform = |xf: Option<RenderMeshTransform>| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();
            app.init_asset::<WorldAsset>();
            app.add_observer(insert_turret_section);
            app.add_observer(insert_turret_joint_render);
            app.world_mut().spawn((turret_section(turret_with(xf)),));
            app.world_mut().flush();
            app.update();

            let world = app.world_mut();
            let mut q =
                world.query_filtered::<&Transform, (With<SectionRenderOf>, With<WorldAssetRoot>)>();
            let found: Vec<Transform> = q.iter(world).copied().collect();
            assert_eq!(found.len(), 1, "exactly one meshed render child expected");
            found[0]
        };

        // Authored transform lands verbatim on the mesh child.
        let authored = RenderMeshTransform {
            position: Vec3::new(0.1, 0.2, 0.3),
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        };
        let got = meshed_child_transform(Some(authored));
        assert_eq!(got.translation, authored.position);
        assert!(
            got.rotation.abs_diff_eq(authored.rotation, 1e-5),
            "render child rotation {:?} != authored {:?}",
            got.rotation,
            authored.rotation
        );
        assert_eq!(
            got.scale,
            Vec3::ONE,
            "render transform must not touch scale"
        );

        // No authored transform => identity child (unchanged pre-feature look).
        let got = meshed_child_transform(None);
        assert_eq!(got, Transform::IDENTITY);
    }
}
