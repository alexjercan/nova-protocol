//! Turret render children: joint meshes, the projectile mesh, and the
//! muzzle-flash and bullet-trail effects.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use super::*;
use crate::sections::nose_cone_mesh;

pub(super) fn on_projectile_marker_effect(
    add: On<Add, TurretBulletProjectileMarker>,
    budget: Option<Res<GraphicsBudget>>,
    q_projectile: Query<
        (&TurretSectionMuzzleEntity, Option<&ProjectileOwner>),
        With<TurretBulletProjectileMarker>,
    >,
    q_ship_velocity: Query<&LinearVelocity>,
    q_direct_mesh: Query<(), With<Mesh3d>>,
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

    // Diagnostic rounds can carry their complete art directly. They have no
    // authored turret or muzzle effect to reset.
    if q_direct_mesh.contains(projectile) {
        return;
    }

    // On the Low tier `insert_turret_barrel_muzzle_effect` never spawned the muzzle
    // effect, so there is nothing to reset - skip before the lookup, otherwise the
    // missing-effect branch below would `error!` on every shot.
    if !budget.as_deref().is_none_or(|b| b.particles) {
        return;
    }

    let Ok((muzzle, owner)) = q_projectile.get(projectile) else {
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

    // The gas leaves with the SHIP, not with the round and not with the world.
    // Pinned to zero it smeared off the barrel the moment the firing ship had
    // any speed of its own, which at combat closing speeds is always - the same
    // defect the blast carried until it was given the warhead's momentum.
    //
    // The ship's linear velocity and not the muzzle POINT's: the turret's own
    // rotation adds a term far below anything a flash lasting 0.05 s could
    // show, and reading it would cost a transform chain walk per shot at 100
    // rounds a second.
    let base_velocity = owner
        .and_then(|owner| q_ship_velocity.get(**owner).ok())
        .map_or(Vec3::ZERO, |velocity| velocity.0);
    properties.set("base_velocity", base_velocity.into());

    // Spawn the particles
    effect_spawner.reset();
}

/// How hard a round's own colour burns past white, so a meter-long body still
/// reads as a tracer at engagement range and picks up the camera's bloom. In
/// the same family as the thruster plume (10/5/0) and the blast shell (4/1.6/0.3).
const ROUND_EMISSIVE_GAIN: f32 = 6.0;

/// One damage type's built-in round art. Handed out as clones - never rebuilt.
#[derive(Debug, Clone)]
struct ProjectileArt {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    /// Half the mesh's drawn length, in world units. Half because
    /// [`nose_cone_mesh`] centres the body on its own midpoint, so this is the
    /// distance from the round's position to its nose - which is what
    /// [`stretch_round_tracers`] has to hold still.
    half_length: f32,
}

/// The built-in bullet art, one mesh and one material per [`DamageType`], built
/// once.
///
/// The `None` arm of [`insert_projectile_render`] is the shipped path - every
/// stock turret authors no projectile mesh - and the default turret fires 100
/// rounds/s per muzzle, so allocating there allocated a mesh and a material per
/// shot.
///
/// Keyed by damage type rather than by turret because the fired type comes from
/// the runtime `LoadedBullet` slot, not from the authored config: a turret that
/// swaps ammo swaps what its rounds look like. The per-turret
/// `projectile_render_mesh` GLB stays the escape hatch for a bespoke round.
///
/// Built through [`FromWorld`] rather than a `Startup` system on purpose: the
/// observer below takes it as a plain `Res`, so a turret that spawns before a
/// startup command flush would miss the resource and hard-error under the
/// `FallbackErrorHandler(panic)` the autopilot and probe runs install.
#[derive(Resource, Debug)]
pub(crate) struct DefaultProjectileRender {
    kinetic: ProjectileArt,
    pierce: ProjectileArt,
    explosive: ProjectileArt,
}

impl DefaultProjectileRender {
    /// The art a round of `kind` flies with. Exhaustive on purpose: a new
    /// damage type must choose a silhouette rather than inherit one.
    fn art(&self, kind: DamageType) -> &ProjectileArt {
        match kind {
            DamageType::Kinetic => &self.kinetic,
            DamageType::Pierce => &self.pierce,
            DamageType::Explosive => &self.explosive,
        }
    }
}

/// A round, nose down -Z: a projectile's transform IS its direction of travel
/// (`muzzle_direction = rotation * NEG_Z` in `shoot_spawn_projectile`), so the
/// shared +Y body is turned onto that axis once, at build time.
fn round_mesh(radius: f32, body_length: f32, nose_length: f32) -> Mesh {
    nose_cone_mesh(radius, body_length, nose_length)
        .rotated_by(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
}

/// A round's material: the damage type's HUD colour ([`damage_type_color`]), so
/// a round in flight matches the ammo pip that loaded it, burned bright enough
/// to read as a tracer.
fn round_material(kind: DamageType) -> StandardMaterial {
    let color = damage_type_color(kind);
    let linear = color.to_linear();
    StandardMaterial {
        base_color: color,
        emissive: LinearRgba::rgb(
            linear.red * ROUND_EMISSIVE_GAIN,
            linear.green * ROUND_EMISSIVE_GAIN,
            linear.blue * ROUND_EMISSIVE_GAIN,
        ),
        ..default()
    }
}

impl FromWorld for DefaultProjectileRender {
    fn from_world(world: &mut World) -> Self {
        // Silhouette carries the read: at 2 km a round is a couple of pixels
        // wide, so only length, thickness and colour survive. All three are
        // shorter than the 3 m box they replace.
        //
        // Kinetic is a blunt slug: thick, stubby, barely tapered.
        // Pierce is a long fine dart: a third the thickness, twice the length,
        // and most of that length is the point.
        // Explosive is a squat shell - the widest body, the least travel.
        //
        // The mesh and the half-length come off the SAME three numbers here,
        // and that is the point of building them together: the tracer stretch
        // holds the nose of the drawn body still, so a length read from
        // anywhere but the mesh it belongs to would pin the streak to a point
        // the round is not at.
        world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
            let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
            let mut art = |kind, radius, body_length: f32, nose_length: f32| ProjectileArt {
                mesh: meshes.add(round_mesh(radius, body_length, nose_length)),
                material: materials.add(round_material(kind)),
                half_length: (body_length + nose_length) / 2.0,
            };
            Self {
                kinetic: art(DamageType::Kinetic, 0.025, 0.09, 0.03),
                pierce: art(DamageType::Pierce, 0.015, 0.13, 0.09),
                explosive: art(DamageType::Explosive, 0.035, 0.05, 0.03),
            }
        })
    }
}

pub(super) fn insert_projectile_render(
    add: On<Add, TurretBulletProjectileMarker>,
    mut commands: Commands,
    default_render: Res<DefaultProjectileRender>,
    asset_server: Res<AssetServer>,
    q_render_mesh: Query<(&BulletProjectileRenderMesh, Option<&ProjectileDamage>)>,
    q_direct_mesh: Query<(), With<Mesh3d>>,
) {
    let entity = add.entity;
    trace!("insert_projectile_render: entity {:?}", entity);

    // A diagnostic or authored one-off can supply its render components on the
    // projectile itself instead of asking this observer for turret-owned art.
    if q_direct_mesh.contains(entity) {
        return;
    }

    let Ok((render_mesh, damage)) = q_render_mesh.get(entity) else {
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
            // The round carries the type it was fired as; a projectile with no
            // authored damage at all is a bare test spawn, not a shot.
            let art = default_render.art(damage.map_or(DamageType::Kinetic, |d| d.kind));
            commands.entity(entity).insert((children![(
                Name::new("Bullet Projectile Render"),
                Mesh3d(art.mesh.clone()),
                MeshMaterial3d(art.material.clone()),
                RoundTracer {
                    half_length: art.half_length,
                    max_length: TRACER_MAX_LENGTH,
                },
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
    placeholder: Res<PlaceholderArt>,
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

    // Authored render-mesh transform, or identity (mesh at the joint origin)
    // when unset. It lives on the mesh CHILD, so it moves only the art, never
    // the joint's kinematic frame.
    let transform = render_mesh_transform
        .map(RenderMeshTransform::to_transform)
        .unwrap_or_default();

    match &**render_mesh {
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
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
        //
        // The authored transform applies HERE too, and has to: the plate is a
        // full unit across, so a turret assembled at any other size than the
        // unit cube it was drawn for wore a hull-sized dinner plate under it.
        // Composed rather than replaced, so the plate keeps its own lift and
        // scales with the rest of the assembly.
        None if !is_muzzle => {
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Joint"),
                transform.mul_transform(Transform::from_xyz(0.0, 0.05, 0.0)),
                SectionRenderOf(**turret),
                Mesh3d(placeholder.turret_plate.clone()),
                MeshMaterial3d(placeholder.turret_plate_material.clone()),
            ),],));
        }
        None => {}
    }
}

/// Longest a PDC tracer may be drawn, in world units.
///
/// The stretch is taken from the frame's own delta, so a stall would draw a
/// streak as long as the stall was: a quarter-second hitch puts a 25 unit bar
/// of light across the screen for one frame, which reads as a rendering fault
/// rather than as a gun. This clamps it to a length a round could plausibly
/// have crossed.
///
/// It is the DEFAULT, not the rule: the clamp scales with the round's speed, so
/// a 1500 u/s lance slug carries its own on [`RoundTracer::max_length`]. A
/// clamp sized for a 100 u/s round would erase the streak of a round fifteen
/// times faster.
pub(crate) const TRACER_MAX_LENGTH: f32 = 4.0;

/// How much of a frame's travel a round is drawn across.
///
/// A camera's shutter is open for part of its frame, not all of it - the film
/// standard is half - so the streak a moving object leaves is shorter than the
/// ground it covered. Here the fraction also does a second job: the shipped PDC
/// leaves 1.0 unit between rounds and covers 1.67 units a frame at 60 fps, so
/// drawing the whole of it overlaps every round with both its neighbours and
/// fuses the burst into an unbroken rod. The gap has to close enough to read as
/// one stream and stay open enough to read as separate rounds.
///
/// Below about 30 fps the streaks meet and the stream does go solid. That is
/// the correct way round: a longer exposure IS more blur.
const TRACER_SHUTTER: f32 = 0.35;

/// How long a round's body is drawn this frame, in world units.
///
/// A round crosses more ground between two drawn frames than the gun leaves
/// between rounds - 1.67 units at 60 fps against 1.0 unit of spacing at 100
/// rounds a second - so no fire rate makes a stream that reads as continuous
/// while each round is drawn at its own 0.12 units. Lowering the rate widens
/// the gap and makes it worse, which is why the answer here is the tracer and
/// not the cadence.
///
/// Drawing a frame of travel closes the gap by construction and at ANY frame
/// rate: what the eye is missing is exactly the distance the round covered
/// while the shutter was open. Never shorter than the round itself, so a slow
/// round keeps its silhouette instead of collapsing to a point.
///
/// A FRACTION of that travel, per [`TRACER_SHUTTER`], because a whole frame of
/// it is too much: 1.67 units drawn into 1.0 unit of spacing overlaps every
/// round with the two beside it and fuses the burst into an unbroken rod. A
/// rod is a laser. This gun fires rounds and has to look like it.
fn tracer_length(speed: f32, delta_secs: f32, natural_length: f32, max_length: f32) -> f32 {
    (speed * delta_secs * TRACER_SHUTTER).clamp(natural_length, max_length.max(natural_length))
}

/// The render child of a round wearing the BUILT-IN art, and how long that art
/// is.
///
/// Only the built-in art wears it. An authored `projectile_render_mesh` chose
/// its own look, there is no length this crate could read off a scene handle
/// before it loads, and a mod that wanted a streak would author one.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct RoundTracer {
    /// Half the drawn length of the unstretched art, in world units - the
    /// distance from the round's position to its nose.
    pub(crate) half_length: f32,
    /// Longest this round's streak may be drawn, in world units. Sized to the
    /// round's own speed: see [`TRACER_MAX_LENGTH`].
    pub(crate) max_length: f32,
}

/// Stretch each built-in round's art back along its own flight, so a burst
/// reads as a stream rather than as a dotted line. See [`tracer_length`].
///
/// On the RENDER clock and in `Update`, not on the sweep's fixed clock: the gap
/// being closed is between two DRAWN frames, so a machine drawing at 20 fps
/// needs three times the streak a machine at 60 fps needs from the same round
/// at the same speed.
///
/// The round itself is untouched. It carries no collider and is swept by hand
/// (`nova_gameplay::rounds`), so nothing about what its art MEASURES can reach
/// what it HITS - which is the whole reason stretching it is free.
pub(super) fn stretch_round_tracers(
    time: Res<Time>,
    q_round: Query<&RoundVelocity>,
    mut q_tracer: Query<(&mut Transform, &RoundTracer, &ChildOf)>,
) {
    let delta = time.delta_secs();
    for (mut transform, tracer, &ChildOf(round)) in &mut q_tracer {
        let Ok(velocity) = q_round.get(round) else {
            continue;
        };
        let natural = tracer.half_length * 2.0;
        if natural <= 0.0 {
            continue;
        }
        let stretch = tracer_length(velocity.length(), delta, natural, tracer.max_length) / natural;
        transform.scale.z = stretch;
        // The art is centred on the round, so scaling alone would push the nose
        // out ahead of where the round actually is - the streak would arrive
        // before the round did, and a PDC would appear to hit early. Sliding
        // the art back by exactly what the nose gained pins the nose to the
        // round and spends the whole stretch on a tail. Local +Z is backwards:
        // `round_mesh` turns the nose onto -Z.
        transform.translation.z = (stretch - 1.0) * tracer.half_length;
    }
}

/// How many particles one shot throws.
///
/// A sixth of what the screen-space flash spent. That version needed a hundred
/// 3-pixel dots to fill anything, because each dot covered nine pixels however
/// near or far the camera was; a world-space quad the width of the bore covers
/// the same picture with a handful, and it grows when you fly past it.
///
/// It was 32 while the flash was invisible for an unrelated reason (see
/// [`MUZZLE_LIFETIME_MAX`]), which is not a count anybody could have judged.
/// Once the flash reached the screen, 32 a frame against a 0.12 s life put
/// about 230 overlapping quads inside one 0.55-unit ball and cost 1.0 ms of
/// the range's frame. Half of them draw the same picture.
const MUZZLE_PARTICLES_PER_SHOT: f32 = 16.0;

/// Shortest a muzzle particle lives, in seconds. See [`MUZZLE_LIFETIME_MAX`]
/// for why the pair is as long as it is.
const MUZZLE_LIFETIME_MIN: f32 = 0.05;

/// Longest a muzzle particle lives, in seconds.
///
/// A real muzzle flash is over in a millisecond or two, and that is not a
/// duration a frame can draw. A particle is first RENDERED one simulation step
/// after it is born, so its gradients are already sampled a whole frame in: a
/// flash that does not outlive two frames is only ever drawn on its way out.
/// The first version of this graph lived 0.01 s to 0.05 s and was a dim orange
/// smear at the bore for precisely that reason - the bright head of its colour
/// curve existed and was never once put on screen.
///
/// So the flash is given the three to seven frames the eye actually has. It
/// still does not become a plume: in vacuum there is no air to push against
/// and nothing to keep the propellant burning, and the size and colour curves
/// below take it to nothing well inside that. At 100 rounds a second the
/// barrel does refire before this decays, and the overlap is the point - a
/// sustained burst should read as one ball of gas burning at the bore for as
/// long as the trigger is down, not as separate puffs.
const MUZZLE_LIFETIME_MAX: f32 = 0.12;

/// Fastest a muzzle particle leaves the barrel, in units/second.
///
/// Slow enough that the longest-lived particle travels under two units before
/// it dies. The gas has to stay a cone at the bore; anything faster throws the
/// tail far enough downrange to be read as a second, weaker tracer.
const MUZZLE_SPEED_MAX: f32 = 14.0;

/// How wide the flash is at birth, in world units.
///
/// World-space, so it is the SAME object from any distance and shrinks to
/// nothing as the camera pulls out. The screen-space dots it replaces stayed
/// three pixels across at any range, which turned a gun two kilometers away
/// into a cloud of confetti larger than the ship firing it.
const MUZZLE_SIZE: f32 = 0.55;

/// Particle capacity of the built-in muzzle flash, which is a per-INSTANCE GPU
/// buffer: the [`EffectAsset`] is shared, one allocation per barrel is not.
///
/// DERIVED, not picked. A spawner emits its `once` count at most one time per
/// tick however many shots asked for it, so a barrel holds
/// `MUZZLE_PARTICLES_PER_SHOT x frame_rate x MUZZLE_LIFETIME_MAX` at once, not
/// one burst per round. At 60 fps that peaks at 115. This is that, doubled and
/// rounded up to a power of two - an eighth of what the hundred-dot version
/// reserved.
///
/// An authored `muzzle_effect` brings its own capacity and ignores this.
const MUZZLE_FLASH_CAPACITY: u32 = 256;

/// The generated muzzle flash, built once and shared by every barrel.
///
/// Every barrel used to mint its own, and they were byte-identical: the
/// direction, the colour and the ship's own velocity all arrive through
/// hanabi PROPERTIES at runtime, so nothing about the graph is per-barrel.
/// A clad warship carries a lot of barrels.
///
/// # One graph, two readings
///
/// A muzzle flash is a bright ball at the bore and a thin cone of gas leaving
/// it, and here those are not two effects - they are the two ends of one speed
/// distribution. The speed is the PRODUCT of two random draws, so most
/// particles barely clear the barrel and pile into the ball while the tail of
/// the distribution strings the rest out ahead as the cone. Splitting them into
/// two assets, as the blast does, would double the per-barrel buffer to draw
/// the same picture; the blast pays that because its two halves want opposite
/// ORIENTATIONS, and both halves of a muzzle flash are billboards.
fn build_default_muzzle_effect() -> EffectAsset {
    let spawner = SpawnerSettings::once(MUZZLE_PARTICLES_PER_SHOT.into())
        // Do not emit on instantiation - the muzzle flash only
        // fires when the shot calls reset().
        .with_emit_on_start(false);

    let writer = ExprWriter::new();

    let age = writer.lit(0.).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Give a bit of variation by randomizing the lifetime per particle
    let lifetime = writer
        .lit(MUZZLE_LIFETIME_MIN)
        .uniform(writer.lit(MUZZLE_LIFETIME_MAX))
        .expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Attribute::COLOR saves the property value PER PARTICLE at spawn,
    // so a later property change leaves live particles alone.
    let spawn_color = writer.add_property("spawn_color", 0xFFFFFFFFu32.into());
    let color = writer.prop(spawn_color).expr();
    let init_color = SetAttributeModifier::new(Attribute::COLOR, color);

    // MODULATE and not the default overwrite, so the shot's own colour - drawn
    // per shot in `on_projectile_marker_effect` - survives the fade instead of
    // being replaced by it. The first key is well past 1.0 in every channel so
    // the flash blooms rather than reading as a flat disc, and the last is
    // transparent so the gas thins out instead of cutting off.
    let mut color_gradient = bevy_hanabi::Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(12.0, 12.0, 12.0, 1.0));
    color_gradient.add_key(0.25, Vec4::new(5.0, 5.0, 5.0, 0.85));
    color_gradient.add_key(1.0, Vec4::ZERO);
    let color_over_lifetime = ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::Modulate,
        mask: ColorBlendMask::default(),
    };

    // Full width immediately: a flash has no rise. It only shrinks.
    let mut size_gradient = bevy_hanabi::Gradient::new();
    size_gradient.add_key(0.0, Vec3::splat(MUZZLE_SIZE));
    size_gradient.add_key(0.3, Vec3::splat(MUZZLE_SIZE * 0.6));
    size_gradient.add_key(1.0, Vec3::ZERO);
    let size_over_lifetime = SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    };

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
    // Two draws multiplied, per the graph's docs: one uniform draw spreads the
    // particles evenly along the barrel's axis, which reads as a jet.
    let speed = writer.rand(ScalarType::Float)
        * writer.rand(ScalarType::Float)
        * writer.lit(MUZZLE_SPEED_MAX);
    let velocity = (normal + spread * writer.lit(2.5)).normalized() * speed;
    let velocity = velocity + base_velocity;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    // Round, not rectangular. A PDC is fought at ranges where the camera can
    // sit a couple of meters off the barrel, and a square is the one shape a
    // ball of burning gas is not.
    let mask = soft_dot_modifier(&writer);
    let mut module = writer.finish();
    declare_soft_dot_slot(&mut module);

    EffectAsset::new(MUZZLE_FLASH_CAPACITY, spawner, module)
        .with_name("spawn_on_command")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .init(init_color)
        // A hanabi quad with no orient modifier is expanded along the fixed
        // WORLD axes, not the camera's, so it is a billboard only from the
        // directions the camera happens not to be looking from. The old
        // screen-space sizing hid the omission by expanding the quad in screen
        // space instead, so dropping it made this mandatory.
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
        .render(size_over_lifetime)
        .render(mask)
        .render(color_over_lifetime)
}

/// The generated muzzle flash, held so it is built ONCE. Authoring an
/// effect on the barrel overrides it; this is the fallback.
///
/// Lazy rather than [`FromWorld`], so an app with no turrets - or one at a
/// graphics tier with particles off - builds nothing.
#[derive(Resource, Default, Debug)]
pub(crate) struct DefaultMuzzleEffect(Option<Handle<EffectAsset>>);

impl DefaultMuzzleEffect {
    /// The shared flash, building it on the first barrel that needs it.
    fn handle(&mut self, effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
        self.0
            .get_or_insert_with(|| effects.add(build_default_muzzle_effect()))
            .clone()
    }
}

pub(super) fn insert_turret_barrel_muzzle_effect(
    add: On<Add, TurretSectionBarrelMuzzleMarker>,
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    mut default_muzzle: ResMut<DefaultMuzzleEffect>,
    mut images: ResMut<Assets<Image>>,
    mut soft_dot: ResMut<SoftDot>,
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

    // The fallback graph declares the round-billboard slot, so its instance
    // must bind it. An AUTHORED effect gets no material: its graph declares
    // whatever slots it wants and binding an image it never asked for is not
    // ours to do.
    let (effect, mask) = match &**effect_handle {
        Some(asset_ref) => (asset_ref.resolve(&asset_server), None),
        None => (
            default_muzzle.handle(&mut effects),
            Some(EffectMaterial {
                images: vec![soft_dot.handle(&mut images)],
            }),
        ),
    };
    // Spawned as its own entity rather than through `children!` because the
    // mask is conditional and `Option<B>` is not a `Bundle`.
    let child = commands
        .spawn((
            Name::new("Muzzle Effect"),
            TurretSectionBarrelMuzzleEffectMarker,
            ParticleEffect::new(effect),
            EffectProperties::default(),
            ChildOf(entity),
        ))
        .id();
    if let Some(mask) = mask {
        commands.entity(child).insert(mask);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{config::default_joint_speed, test_support::*},
        *,
    };

    /// A bullet app with only the round-render observer and its resource.
    #[test]
    fn a_muzzle_flash_outlives_the_frame_that_draws_it() {
        // A particle is first DRAWN one simulation step after it is born, so
        // its gradients are already sampled a frame in. A flash that does not
        // outlive two frames is therefore only ever put on screen on its way
        // out, which is what made the first version of this graph a dim smear
        // at the bore however bright the head of its colour curve was.
        let frame = 1.0 / 60.0;
        assert!(
            MUZZLE_LIFETIME_MIN > frame * 2.0,
            "the shortest-lived muzzle particle must survive the frame that first draws it"
        );
        const { assert!(MUZZLE_LIFETIME_MAX > MUZZLE_LIFETIME_MIN) };
    }

    #[test]
    fn the_muzzle_buffer_holds_a_sustained_burst() {
        // One burst per FRAME and not per round: a spawner emits its `once`
        // count at most once a tick however many shots called `reset()`, so a
        // 100-round-a-second barrel does not fill this ten times faster than a
        // 10-round-a-second one.
        let alive = MUZZLE_PARTICLES_PER_SHOT * 60.0 * MUZZLE_LIFETIME_MAX;
        assert!(
            alive <= f64::from(MUZZLE_FLASH_CAPACITY) as f32,
            "{alive} live particles do not fit in {MUZZLE_FLASH_CAPACITY}"
        );
    }

    fn round_render_app() -> App {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        // The AUTHORED arm loads a scene through the asset server, which panics
        // on an uninitialised asset type.
        app.init_asset::<WorldAsset>();
        app.init_resource::<DefaultProjectileRender>();
        app.add_observer(insert_projectile_render);
        app
    }

    /// The kinetic round the PDC fires: 0.12 units long at 100 units/s, drawn
    /// at 60 fps. This is the exact case `## Findings` measured - 1.0 unit of
    /// spacing between rounds and 1.67 units of travel per frame - and the
    /// drawn body has to land between the two: long enough that the burst
    /// reads as one stream, short enough that consecutive rounds stay separate
    /// rather than fusing into a rod.
    #[test]
    fn a_tracer_is_drawn_between_its_own_length_and_the_gap_to_the_next_round() {
        let natural = 0.09 + 0.03;
        let spacing = 1.0;
        let drawn = tracer_length(100.0, 1.0 / 60.0, natural, TRACER_MAX_LENGTH);
        assert!(
            drawn > natural * 4.0,
            "the streak must be several times the round to close the gap, got {drawn}"
        );
        assert!(
            drawn < spacing,
            "a streak past the spacing merges the burst into a rod, got {drawn}"
        );
    }

    /// Frame-rate correctness is the whole claim: the SAME round at the SAME
    /// speed must be drawn longer on a slower machine, because the gap it has
    /// to cover is the one between two drawn frames.
    #[test]
    fn a_slower_frame_draws_a_longer_tracer() {
        let natural = 0.12;
        let fast = tracer_length(100.0, 1.0 / 120.0, natural, TRACER_MAX_LENGTH);
        let slow = tracer_length(100.0, 1.0 / 30.0, natural, TRACER_MAX_LENGTH);
        assert!(slow > fast, "{slow} must exceed {fast}");
        assert!(
            (slow / fast - 4.0).abs() < 1e-4,
            "four times the frame is four times the streak, got {}",
            slow / fast
        );
    }

    #[test]
    fn a_round_that_is_barely_moving_keeps_its_own_silhouette() {
        let natural = 0.12;
        assert_eq!(
            tracer_length(0.0, 1.0 / 60.0, natural, TRACER_MAX_LENGTH),
            natural
        );
        assert_eq!(
            tracer_length(1.0, 1.0 / 60.0, natural, TRACER_MAX_LENGTH),
            natural
        );
    }

    /// A hitch must not put a bar of light across the screen. The stretch is
    /// read off the frame's own delta, so without the clamp a quarter-second
    /// stall would draw a 25 unit streak for one frame.
    #[test]
    fn a_stalled_frame_cannot_draw_a_bar_across_the_screen() {
        assert_eq!(
            tracer_length(100.0, 0.25, 0.12, TRACER_MAX_LENGTH),
            TRACER_MAX_LENGTH
        );
    }

    /// The nose is the round. Scaling the art alone would push it out ahead of
    /// the position the sweep actually resolves hits at, so a PDC would look
    /// like it connected before it did; the offset exists to pin it.
    #[test]
    fn stretching_a_tracer_pins_its_nose_to_the_round() {
        let mut app = round_render_app();
        app.add_systems(Update, stretch_round_tracers);
        // `MinimalPlugins` runs `Time` off the wall clock, which would advance
        // by microseconds across a test binary's updates and stretch nothing.
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 60.0),
        ));
        // Bevy's first manual tick is dt 0, so the stretch would be a no-op.
        app.update();

        let round = app
            .world_mut()
            .spawn((
                TurretBulletProjectileMarker,
                BulletProjectileRenderMesh(None),
                ProjectileDamage::new(4.0, DamageType::Kinetic),
                RoundVelocity(Vec3::NEG_Z * 100.0),
            ))
            .id();
        app.update();

        let world = app.world_mut();
        let child = world
            .entity(round)
            .get::<Children>()
            .and_then(|children| children.first().copied())
            .expect("the round got its render child");
        let tracer = *world
            .entity(child)
            .get::<RoundTracer>()
            .expect("built-in art wears the tracer marker");
        let transform = *world
            .entity(child)
            .get::<Transform>()
            .expect("the render child has a transform");

        let natural = tracer.half_length * 2.0;
        let drawn = natural * transform.scale.z;
        assert!(
            (drawn - 100.0 / 60.0 * TRACER_SHUTTER).abs() < 1e-4,
            "the body must be drawn one shutter of travel long, got {drawn}"
        );
        // Local -Z is forward: `round_mesh` turns the nose onto it.
        let nose = transform.translation.z - tracer.half_length * transform.scale.z;
        assert!(
            (nose + tracer.half_length).abs() < 1e-5,
            "the nose must stay where the unstretched art put it, got {nose}"
        );
    }

    /// The lance's slug is the fastest round in the game, and the streak is
    /// most of what a player ever sees of it. A clamp sized for the PDC would
    /// draw it shorter than its own body.
    #[test]
    fn a_lance_slug_streaks_further_than_a_pdc_round_may() {
        let natural = 0.8;
        let pdc_clamped = tracer_length(1500.0, 1.0 / 60.0, natural, TRACER_MAX_LENGTH);
        assert_eq!(
            pdc_clamped, TRACER_MAX_LENGTH,
            "the PDC clamp truncates a lance slug to {TRACER_MAX_LENGTH}"
        );

        let slug = tracer_length(1500.0, 1.0 / 60.0, natural, 40.0);
        assert!(
            slug > pdc_clamped,
            "a lance slug must outrun the PDC clamp, got {slug}"
        );
        // A frame of travel is 25 units; the shutter draws a fraction of it.
        assert!(
            slug > natural * 4.0 && slug < 25.0,
            "the streak closes the gap without spanning the whole frame, got {slug}"
        );
    }

    /// An authored round chose its own look and there is no length to measure
    /// on a scene handle, so the stretch must leave it alone.
    #[test]
    fn an_authored_round_gets_no_tracer_stretch() {
        let mut app = round_render_app();
        let round = app
            .world_mut()
            .spawn((
                TurretBulletProjectileMarker,
                BulletProjectileRenderMesh(Some("rounds/dart.glb".into())),
                RoundVelocity(Vec3::NEG_Z * 100.0),
            ))
            .id();
        app.update();

        let world = app.world_mut();
        let child = world
            .entity(round)
            .get::<Children>()
            .and_then(|children| children.first().copied())
            .expect("the round got its render child");
        assert!(
            world.entity(child).get::<RoundTracer>().is_none(),
            "authored art must not be stretched"
        );
    }

    /// The `None` arm is the shipped path and the default turret fires 100
    /// rounds/s per muzzle. Every bullet must reuse its damage type's shared
    /// mesh and material instead of adding two assets per shot - the whole
    /// point of keying the art by TYPE rather than building it per round.
    ///
    /// The first bullet is spawned BEFORE any update, which is what a `Startup`
    /// system could not serve: the resource has to exist the moment the
    /// observer runs.
    #[test]
    fn default_projectile_render_allocates_no_assets_per_shot() {
        let mut app = round_render_app();
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
            (3, 3),
            "one shared mesh + material per damage type, and no more"
        );

        // Alternate the fired type: an ammo swap mid-burst must not allocate
        // either.
        let kinds = [
            DamageType::Kinetic,
            DamageType::Pierce,
            DamageType::Explosive,
        ];
        let fire = |app: &mut App, i: usize| {
            app.world_mut().spawn((
                TurretBulletProjectileMarker,
                BulletProjectileRenderMesh(None),
                ProjectileDamage::new(4.0, kinds[i % kinds.len()]),
            ));
        };

        // No update has flushed yet on this entity's behalf beyond the one
        // above; the observer must already find the resource.
        fire(&mut app, 0);
        app.update();
        assert_eq!(
            assets_now(&app),
            before,
            "a bullet spawned with no startup flush reuses the shared assets"
        );

        for i in 1..65 {
            fire(&mut app, i);
            app.update();
        }

        assert_eq!(
            assets_now(&app),
            before,
            "firing must not add mesh/material assets"
        );

        // ... and every bullet actually got its type's shared handles.
        let world = app.world_mut();
        let mut q =
            world.query_filtered::<(&Mesh3d, &MeshMaterial3d<StandardMaterial>), With<Name>>();
        let children: Vec<_> = q
            .iter(world)
            .map(|(m, s)| (m.0.clone(), s.0.clone()))
            .collect();
        // 64 in the loop plus the pre-flush bullet above.
        assert_eq!(children.len(), 65, "one render child per bullet");
        let distinct: std::collections::HashSet<_> = children.iter().cloned().collect();
        assert_eq!(
            distinct.len(),
            kinds.len(),
            "bullets share exactly one handle pair per damage type"
        );
    }

    /// A kinetic slug and a penetrator must not look alike: that is the change.
    #[test]
    fn each_damage_type_flies_a_distinct_round() {
        let mut app = round_render_app();
        app.update();

        let render_of = |app: &mut App, kind: DamageType| {
            let bullet = app
                .world_mut()
                .spawn((
                    TurretBulletProjectileMarker,
                    BulletProjectileRenderMesh(None),
                    ProjectileDamage::new(4.0, kind),
                ))
                .id();
            app.update();
            let child = app.world().get::<Children>(bullet).expect("render child")[0];
            let mesh = app.world().get::<Mesh3d>(child).expect("mesh").0.clone();
            let material = app
                .world()
                .get::<MeshMaterial3d<StandardMaterial>>(child)
                .expect("material")
                .0
                .clone();
            (mesh, material)
        };

        let kinetic = render_of(&mut app, DamageType::Kinetic);
        let pierce = render_of(&mut app, DamageType::Pierce);
        assert_ne!(kinetic.0, pierce.0, "the slug and the dart differ in shape");
        assert_ne!(kinetic.1, pierce.1, "and in colour");

        // The colour is the HUD's, so a round in flight matches the ammo pip
        // that loaded it.
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(
            materials.get(&pierce.1).expect("material").base_color,
            damage_type_color(DamageType::Pierce)
        );
    }

    /// The round is built nose-forward: a projectile flies down its own -Z
    /// (`muzzle_direction = rotation * NEG_Z`), so a mesh pointing any other way
    /// flies backwards or sideways.
    #[test]
    fn round_mesh_points_its_nose_down_negative_z() {
        use bevy::mesh::VertexAttributeValues;

        let (radius, body, nose) = (0.015, 0.13, 0.09);
        let mesh = round_mesh(radius, body, nose);
        let positions: Vec<Vec3> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(p)) => {
                p.iter().copied().map(Vec3::from_array).collect()
            }
            other => panic!("unexpected positions: {other:?}"),
        };
        let min = positions
            .iter()
            .copied()
            .reduce(Vec3::min)
            .expect("vertices");
        let max = positions
            .iter()
            .copied()
            .reduce(Vec3::max)
            .expect("vertices");
        let extent = max - min;

        // Long axis is Z, the round spans exactly body + nose, and it is a dart
        // rather than a box.
        assert!((extent.z - (body + nose)).abs() < 1e-5, "{extent:?}");
        assert!((extent.x - radius * 2.0).abs() < 2e-3, "{extent:?}");
        assert!(extent.z > extent.x * 4.0, "a dart, not a box: {extent:?}");

        // The leading vertex - the cone tip - is on the axis, so the nose points
        // the way the round travels.
        let tip = positions
            .iter()
            .copied()
            .min_by(|a, b| a.z.total_cmp(&b.z))
            .expect("a vertex");
        assert!(
            tip.x.abs() < 1e-5 && tip.y.abs() < 1e-5,
            "the leading vertex is the cone tip on the axis, not a body corner: {tip:?}"
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
        app.init_resource::<PlaceholderArt>();
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
                name: None,
                offset: Vec3::ZERO,
                axis: None,
                speed: default_joint_speed(),
                min: None,
                max: None,
                render_mesh: Some(AssetRef::from("gltf/turret-yaw-01.glb#Scene0".to_string())),
                render_mesh_transform: xf,
                muzzle: None,
                children: vec![TurretJoint {
                    name: None,
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
            app.init_resource::<PlaceholderArt>();
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
            scale: Vec3::splat(0.5),
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
            got.scale, authored.scale,
            "the authored scale sizes the art"
        );

        // No authored transform => identity child (unchanged pre-feature look).
        let got = meshed_child_transform(None);
        assert_eq!(got, Transform::IDENTITY);
    }

    /// The default base plate obeys the authored transform too.
    ///
    /// It is not authored art - an unmeshed structural joint gets a primitive a
    /// full unit across - so before this it stayed unit-sized whatever the rest
    /// of the assembly was scaled to, and a half-size turret wore a hull-sized
    /// dinner plate. It keeps its own small lift, which has to SCALE with the
    /// plate rather than survive it.
    #[test]
    fn the_default_base_plate_is_sized_by_the_authored_transform() {
        let plate_transform = |xf: Option<RenderMeshTransform>| {
            let turret = TurretSectionConfig {
                root: TurretJoint {
                    name: None,
                    offset: Vec3::ZERO,
                    axis: None,
                    speed: default_joint_speed(),
                    min: None,
                    max: None,
                    // Unmeshed and not a muzzle: the default-plate branch.
                    render_mesh: None,
                    render_mesh_transform: xf,
                    muzzle: None,
                    children: vec![TurretJoint {
                        name: None,
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

            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();
            app.init_asset::<WorldAsset>();
            app.init_resource::<PlaceholderArt>();
            app.add_observer(insert_turret_section);
            app.add_observer(insert_turret_joint_render);
            app.world_mut().spawn((turret_section(turret),));
            app.world_mut().flush();
            app.update();

            let world = app.world_mut();
            let mut q = world.query_filtered::<&Transform, (With<SectionRenderOf>, With<Mesh3d>)>();
            let found: Vec<Transform> = q.iter(world).copied().collect();
            assert_eq!(found.len(), 1, "exactly one default plate expected");
            found[0]
        };

        // Unauthored: the plate sits where it always did.
        let plain = plate_transform(None);
        assert_eq!(plain.translation, Vec3::new(0.0, 0.05, 0.0));
        assert_eq!(plain.scale, Vec3::ONE);

        // Half size: the plate halves, and so does its lift.
        let half = plate_transform(Some(RenderMeshTransform {
            scale: Vec3::splat(0.5),
            ..default()
        }));
        assert_eq!(half.scale, Vec3::splat(0.5));
        assert_eq!(half.translation, Vec3::new(0.0, 0.025, 0.0));
    }
}
