//! Render and particle systems for the torpedo bay and its in-flight
//! projectile, gated behind the section plugin's `render` flag.

use std::f32::consts::PI;

use bevy::platform::collections::HashMap;

use super::*;
use crate::sections::nose_cone_mesh;

pub(super) fn insert_torpedo_section_render(
    add: On<Add, TorpedoSectionBodyMarker>,
    mut commands: Commands,
    placeholder: Res<PlaceholderArt>,
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
                Mesh3d(placeholder.body.clone()),
                MeshMaterial3d(placeholder.structure_material.clone()),
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

/// The built-in torpedo body: one mesh, and one material per TINT.
///
/// A launched torpedo authors no `projectile_render_mesh` (both shipped bays
/// leave it `None`), so this IS the warhead the player shoots at. It replaces a
/// flat-ended cylinder that was rebuilt - mesh AND material - on every launch
/// and never freed.
///
/// The mesh is built nose down -Y: the body rides the torpedo's CONTROLLER
/// section, whose authored `Transform` turns the section a quarter turn about X
/// (`shoot_spawn_projectile`), which lands -Y on the torpedo's own -Z, the way
/// it flies.
///
/// The MATERIAL is shared by tint rather than minted per launch. A distinct
/// asset is extracted, prepared, bound and written every frame however many
/// entities share its value, so a private material per torpedo put the frame's
/// asset work on the size of the salvo - and, because
/// [`SectionCracks`](crate::sections::damage_cracks::prelude::SectionCracks)
/// keys its bucket materials on the SOURCE, it multiplied that by the crack
/// buckets as well. A tint is a property of the ordnance TYPE, so a salvo of
/// one type is one material.
#[derive(Resource, Debug)]
pub(crate) struct DefaultTorpedoRender {
    mesh: Handle<Mesh>,
    /// One warhead material per tint, keyed by the colour's bit pattern the way
    /// `ExhaustMeshes` keys its flames. Held STRONG and never evicted: the set
    /// is bounded by the ordnance types a mod authors, and keeping a tint alive
    /// is also what keeps its crack buckets from being rebuilt on the next
    /// salvo.
    bodies: HashMap<[u32; 4], Handle<StandardMaterial>>,
}

impl FromWorld for DefaultTorpedoRender {
    fn from_world(world: &mut World) -> Self {
        // 10 m long over a 3.2 m body, the same envelope as the cylinder it
        // replaces: a third of the length is nose, so the warhead reads as
        // ordnance rather than as pipe.
        let mesh = nose_cone_mesh(0.16, 0.65, 0.35).rotated_by(Quat::from_rotation_x(PI));
        Self {
            mesh: world.resource_mut::<Assets<Mesh>>().add(mesh),
            bodies: HashMap::default(),
        }
    }
}

impl DefaultTorpedoRender {
    /// The warhead material for `tint`, building it the first time a torpedo of
    /// that colour launches.
    fn body_material(
        &mut self,
        tint: Color,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        let colour = LinearRgba::from(tint);
        let key = [
            colour.red.to_bits(),
            colour.green.to_bits(),
            colour.blue.to_bits(),
            colour.alpha.to_bits(),
        ];
        self.bodies
            .entry(key)
            .or_insert_with(|| materials.add(tint))
            .clone()
    }

    /// How many distinct warhead tints have a material.
    #[cfg(test)]
    fn tints(&self) -> usize {
        self.bodies.len()
    }
}

pub(super) fn insert_torpedo_controller_render(
    add: On<Add, TorpedoControllerMarker>,
    mut commands: Commands,
    mut default_render: ResMut<DefaultTorpedoRender>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_controller: Query<&ChildOf, With<TorpedoControllerMarker>>,
    q_torpedo: Query<
        (&TorpedoProjectileRenderMesh, Option<&TorpedoType>),
        With<TorpedoProjectileMarker>,
    >,
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

    let Ok((render_mesh, torpedo_type)) = q_torpedo.get(*torpedo) else {
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

    // The type's tint, so two ordnance types read apart in the frame BEFORE
    // their flight paths have had time to diverge. A torpedo spawned with no
    // type - a bare test fixture - keeps the old neutral grey.
    let tint = torpedo_type
        .map(|torpedo_type| torpedo_type.tint)
        .unwrap_or(Color::srgb(0.8, 0.8, 0.8));
    let material = default_render.body_material(tint, &mut materials);
    commands.entity(entity).insert((
        Mesh3d(default_render.mesh.clone()),
        MeshMaterial3d(material),
    ));
}

/// The generated blast burst, held so it is built ONCE.
///
/// A detonation must not author its own: the graph is identical every time -
/// hanabi keys its shader cache on the generated WGSL, so a per-blast asset
/// recompiles nothing and buys nothing, while a salvo pays a whole `ExprWriter`
/// build and a fresh [`BLAST_CAPACITY`] buffer per warhead.
///
/// Lazy rather than [`FromWorld`], so an app that never detonates anything -
/// and one running at a graphics tier with particles off - builds nothing.
#[derive(Resource, Default, Debug)]
pub(crate) struct DefaultBlastEffect {
    ejecta: Option<Handle<EffectAsset>>,
    core: Option<Handle<EffectAsset>>,
}

impl DefaultBlastEffect {
    /// The shared fallback ejecta burst, building it on the first blast that
    /// needs it.
    fn handle(&mut self, effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
        self.ejecta
            .get_or_insert_with(|| effects.add(build_default_blast_effect()))
            .clone()
    }

    /// The shared fallback core-and-shell burst, built on the same terms.
    fn core_handle(&mut self, effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
        self.core
            .get_or_insert_with(|| effects.add(build_default_blast_core_effect()))
            .clone()
    }
}

/// Particle capacity of the built-in blast, which is a per-INSTANCE GPU buffer:
/// the [`EffectAsset`] is shared, one allocation per detonation is not.
///
/// DERIVED, not picked. The burst emits once on start and is never `reset`, so
/// an instance holds exactly the 400 particles it was spawned with. This is the
/// next power of two above that.
///
/// An authored `blast_effect` brings its own capacity and ignores this.
const BLAST_CAPACITY: u32 = 512;

/// The generated blast burst, built once and shared by every detonation.
///
/// Authoring `blast_effect` on the bay overrides it; this is the fallback.
fn build_default_blast_effect() -> EffectAsset {
    let spawner = SpawnerSettings::once(400.0.into())
        // In this case we want to emit on start to create an instantaneous explosion
        .with_emit_on_start(true);

    let writer = ExprWriter::new();

    let age = writer.lit(0.).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // A vacuum burst is a brief flash followed by fast incandescent ejecta,
    // not a lingering atmospheric cloud. Shorter lives also reduce overdraw.
    let lifetime = writer.lit(0.18).uniform(writer.lit(0.8)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // HDR white-gold gives the compact core enough bloom to read at combat
    // distance. It cools through amber to dim red without a smoke phase.
    let mut color_gradient = bevy_hanabi::Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(6.0, 5.6, 4.2, 1.0));
    color_gradient.add_key(0.08, Vec4::new(4.0, 2.1, 0.55, 0.95));
    color_gradient.add_key(0.45, Vec4::new(1.4, 0.35, 0.06, 0.55));
    color_gradient.add_key(1.0, Vec4::new(0.18, 0.02, 0.01, 0.0));

    let color_over_lifetime = ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::default(),
        mask: ColorBlendMask::default(),
    };

    let init_color = SetAttributeModifier::new(Attribute::COLOR, writer.lit(0xFFFFFFFFu32).expr());

    // Long, narrow quads become radial incandescent streaks when oriented to
    // velocity. They contract into fragments instead of swelling into a ball.
    let mut size_gradient = bevy_hanabi::Gradient::new();
    size_gradient.add_key(0.0, Vec3::new(0.55, 0.12, 0.12));
    size_gradient.add_key(0.06, Vec3::new(0.8, 0.14, 0.14));
    size_gradient.add_key(0.25, Vec3::new(0.34, 0.07, 0.07));
    size_gradient.add_key(1.0, Vec3::ZERO);

    let size_over_lifetime = SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    };

    // Position: explosion center
    let init_pos = SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());

    // The velocity the warhead arrived with, written per detonation. A blast
    // that does not carry it expands about a point the torpedo was only
    // passing through, which reads as a firework rather than as an impact -
    // and on an intercept, where the closing speed is most of the motion in
    // the frame, it reads as the wrong thing entirely.
    let base_velocity = writer.add_property("base_velocity", Vec3::ZERO.into());
    let base_velocity = writer.prop(base_velocity);

    // Velocity: spherical random burst
    let rand_x = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let rand_y = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let rand_z = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);

    let dir =
        writer.lit(Vec3::X) * rand_x + writer.lit(Vec3::Y) * rand_y + writer.lit(Vec3::Z) * rand_z;

    // Normalize before applying an intentionally broad speed range. The faster
    // front gives the brief burst reach without adding particles or lifetime.
    let speed = writer.lit(12.0).uniform(writer.lit(60.0));
    let velocity = dir.normalized() * speed + base_velocity;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    // Round, not rectangular. On a velocity-oriented quad the circular mask
    // reads as a tapered streak rather than as a lozenge with corners, which
    // is what a fragment leaving a fireball should look like close up.
    let mask = soft_dot_modifier(&writer);
    let mut module = writer.finish();
    declare_soft_dot_slot(&mut module);

    EffectAsset::new(BLAST_CAPACITY, spawner, module)
        .with_name("spawn_on_blast_explosion")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .init(init_color)
        .render(size_over_lifetime)
        .render(OrientModifier::new(OrientMode::AlongVelocity))
        .render(mask)
        .render(color_over_lifetime)
}

/// Particle capacity of the built-in blast CORE, a per-instance buffer on the
/// same terms as [`BLAST_CAPACITY`]. Derived the same way: the burst emits
/// [`BLAST_CORE_PARTICLES`] once and is never reset.
const BLAST_CORE_CAPACITY: u32 = 64;

/// How many soft billboards make the core and its shell.
///
/// Small on purpose. These are the LARGEST quads in the effect and they all
/// overlap at the centre, so this is the overdraw-bound half of a detonation
/// while the ejecta is the particle-count-bound half. Forty is enough to read
/// as a filled sphere rather than as separate blobs; more only pays more fill
/// for the same picture.
const BLAST_CORE_PARTICLES: f32 = 40.0;

/// The generated blast CORE: the flash and the cooling shell the ejecta leaves
/// from, built once and shared by every detonation.
///
/// Two assets and not one, because the two halves want opposite things from
/// every modifier a single graph could only set once. The ejecta is fast, thin
/// and oriented ALONG its velocity, which is what makes a streak; the core is
/// slow, wide and camera-facing, which is what makes a fireball. Orientation
/// alone forces the split - a billboard cannot also be a streak - and once the
/// split exists the lifetimes, sizes and speeds separate too.
///
/// Physically this stands for the fireball rather than a shock wave: vacuum has
/// nothing to carry a shock, so what expands is the warhead's own vaporised
/// mass, and it cools as it thins. That is why the size gradient grows and the
/// colour falls at the same time, and why it is over inside a third of a
/// second - there is no atmosphere to keep it burning.
fn build_default_blast_core_effect() -> EffectAsset {
    let spawner = SpawnerSettings::once(BLAST_CORE_PARTICLES.into()).with_emit_on_start(true);

    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());

    // Shorter than the ejecta's tail: the fireball is gone while the fragments
    // are still travelling, which is the order a vacuum burst happens in.
    let lifetime = writer.lit(0.12).uniform(writer.lit(0.32)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Blinding white through amber to nothing. The first key is well past 1.0
    // in every channel so the core blooms into a flare rather than reading as
    // a white circle, and the last is transparent so the shell thins out
    // instead of cutting off.
    let mut color_gradient = bevy_hanabi::Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(14.0, 13.0, 11.0, 1.0));
    color_gradient.add_key(0.12, Vec4::new(9.0, 6.0, 2.4, 0.95));
    color_gradient.add_key(0.40, Vec4::new(3.2, 1.0, 0.16, 0.6));
    color_gradient.add_key(1.0, Vec4::new(0.4, 0.05, 0.01, 0.0));
    let color_over_lifetime = ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::default(),
        mask: ColorBlendMask::default(),
    };
    let init_color = SetAttributeModifier::new(Attribute::COLOR, writer.lit(0xFFFFFFFFu32).expr());

    // Grows, then thins. The peak is early: a fireball reaches its size while
    // it is still bright and spends the rest of its life fading at roughly
    // that size, so a gradient that keeps growing to the end reads as a
    // balloon being inflated.
    let mut size_gradient = bevy_hanabi::Gradient::new();
    size_gradient.add_key(0.0, Vec3::splat(1.6));
    size_gradient.add_key(0.18, Vec3::splat(4.2));
    size_gradient.add_key(0.55, Vec3::splat(3.4));
    size_gradient.add_key(1.0, Vec3::ZERO);
    let size_over_lifetime = SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    };

    let init_pos = SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());

    let base_velocity = writer.add_property("base_velocity", Vec3::ZERO.into());
    let base_velocity = writer.prop(base_velocity);

    let rand_x = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let rand_y = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let rand_z = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let dir =
        writer.lit(Vec3::X) * rand_x + writer.lit(Vec3::Y) * rand_y + writer.lit(Vec3::Z) * rand_z;

    // An order of magnitude under the ejecta. The shell is what the fragments
    // are seen leaving THROUGH, so it has to stay behind them.
    let speed = writer.lit(1.5).uniform(writer.lit(9.0));
    let velocity = dir.normalized() * speed + base_velocity;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    // The core is where an untextured quad is most obviously a box: it is the
    // biggest, brightest, closest-to-camera thing in the frame. Without the
    // mask a detonation reads as a glowing square whatever the gradient does.
    let mask = soft_dot_modifier(&writer);
    let mut module = writer.finish();
    declare_soft_dot_slot(&mut module);

    EffectAsset::new(BLAST_CORE_CAPACITY, spawner, module)
        .with_name("spawn_on_blast_core")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .init(init_color)
        // The core is the camera-facing half of the detonation, and saying so
        // in a doc comment did not make it so: a quad with no orient modifier
        // is expanded along the fixed WORLD axes, so the fireball was drawn
        // edge-on from any camera looking down one of them.
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
        .render(size_over_lifetime)
        .render(mask)
        .render(color_over_lifetime)
}

/// How bright a detonation lights the hulls around it, in lumens.
///
/// Sized against the blast radius rather than picked: `PointLight` falls off
/// with the square of distance, so a light meant to still read at the 30-unit
/// edge of the shipped warhead's damage sphere has to be large at its centre.
/// This lands a little under bevy's own outdoor-sun illuminance at that edge,
/// which is the intent - at the rim of a nuclear fireball, being lit like
/// daylight is the understatement.
const BLAST_LIGHT_LUMENS: f32 = 40_000_000.0;

/// How far the detonation light reaches, in world units.
///
/// Three times the shipped 30-unit blast radius. The light is the only part of
/// a detonation that is meant to be seen from outside it: the fireball says
/// something went off there, and the light says it went off near YOU.
const BLAST_LIGHT_RANGE: f32 = 90.0;

/// How long the detonation light burns, in seconds.
///
/// Shorter than the core it belongs to. The flash has to be over before the
/// fireball is, or the last frames read as a lamp hanging in the debris.
const BLAST_LIGHT_SECS: f32 = 0.16;

pub(super) fn insert_particle_effect(
    add: On<Add, NovaBlast>,
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    mut images: ResMut<Assets<Image>>,
    mut default_blast: ResMut<DefaultBlastEffect>,
    mut soft_dot: ResMut<SoftDot>,
    asset_server: Res<AssetServer>,
    budget: Option<Res<GraphicsBudget>>,
    q_blast: Query<(&Transform, &TorpedoSectionPartOf, Option<&BlastMomentum>), With<NovaBlast>>,
    q_config: Query<&TorpedoSectionConfigHelper, With<TorpedoSectionMarker>>,
) {
    let entity = add.entity;
    trace!("insert_particle_effect: entity {:?}", entity);

    // Low graphics tier is spawn-less: skip the hanabi blast entirely. Absent
    // budget (settings-less app) means full quality.
    if !budget.as_deref().is_none_or(|b| b.particles) {
        return;
    }

    let Ok((blast_transform, TorpedoSectionPartOf(torpedo_section), momentum)) =
        q_blast.get(entity)
    else {
        // A blast no torpedo bay owns - a scripted detonation, a range rig, a
        // mod spawning `nova_blast` directly. It is a real blast and it damages
        // normally; it just has no authored bay behind it to take a particle
        // effect from. Same class as the missing config below, not an error.
        debug!(
            "insert_particle_effect: blast {:?} has no owning bay; particle omitted",
            entity
        );
        return;
    };

    let Ok(config) = q_config.get(*torpedo_section) else {
        // A launched torpedo can outlive the bay that supplied its optional
        // particle effect. The blast remains real; only that authored look is
        // unavailable after its owner has gone.
        debug!(
            "insert_particle_effect: source section {:?} is gone; particle omitted",
            torpedo_section
        );
        return;
    };

    let at = blast_transform.translation;
    let inherited = momentum.map_or(Vec3::ZERO, |momentum| **momentum);
    let mut properties = EffectProperties::default();
    properties.set("base_velocity", inherited.into());

    // An AUTHORED effect replaces the whole detonation look, core included: a
    // mod that wrote its own blast graph did not ask for the built-in fireball
    // to be composited under it. So the core rides the fallback path only.
    match &config.blast_effect {
        Some(asset_ref) => {
            commands.spawn((
                Name::new("Blast Effect"),
                TorpedoBlastEffectMarker,
                Transform::from_translation(at),
                ParticleEffect::new(asset_ref.resolve(&asset_server)),
                properties,
                TempEntity(2.0),
            ));
        }
        None => {
            // Both fallback graphs declare the mask slot, so both instances
            // must bind it. An AUTHORED effect above gets no material: its
            // graph declares whatever slots it wants and binding an image it
            // never asked for is not ours to do.
            let mask = EffectMaterial {
                images: vec![soft_dot.handle(&mut images)],
            };
            commands.spawn((
                Name::new("Blast Effect"),
                TorpedoBlastEffectMarker,
                Transform::from_translation(at),
                ParticleEffect::new(default_blast.handle(&mut effects)),
                mask.clone(),
                properties.clone(),
                TempEntity(2.0),
            ));
            commands.spawn((
                Name::new("Blast Core Effect"),
                TorpedoBlastEffectMarker,
                Transform::from_translation(at),
                ParticleEffect::new(default_blast.core_handle(&mut effects)),
                mask,
                properties,
                // Shorter than the ejecta's: the core is finished inside a
                // third of a second and nothing is left to draw after it.
                TempEntity(1.0),
            ));
        }
    }

    // Asked for, never assumed - the budget may refuse it, and a detonation
    // that lit nothing is still a detonation. Gated with the particles above
    // rather than separately: a tier that spawns no fireball has nothing for a
    // light to be the flash OF.
    commands.trigger(LightFlash {
        at,
        // Warmer than the core's first key. The light stands in for the whole
        // fireball averaged over its life, and that average is amber, not the
        // white of its first two frames.
        color: Color::srgb(1.0, 0.72, 0.38),
        peak_intensity: BLAST_LIGHT_LUMENS,
        range: BLAST_LIGHT_RANGE,
        duration: BLAST_LIGHT_SECS,
    });
}

/// Brightness of the drive light-off, in lumens.
///
/// Four hundred times smaller than the detonation's, which is the point: a
/// torpedo motor catching is a torch, not a warhead, and the two must not read
/// as the same event at a distance. Enough to put a moving highlight on the
/// launching ship's hull for a few frames as the torpedo pulls away.
const IGNITION_LIGHT_LUMENS: f32 = 100_000.0;

/// How far the ignition light reaches, in world units. Short: it is meant to
/// light the bay it just left and nothing further.
const IGNITION_LIGHT_RANGE: f32 = 18.0;

/// How long the ignition light burns, in seconds. It is a light-off, not a
/// running engine - the exhaust takes over from here.
const IGNITION_LIGHT_SECS: f32 = 0.22;

/// Light the bay when a torpedo's drive catches.
///
/// Answers [`TorpedoIgnited`] rather than watching the component, so gameplay
/// says WHEN and this says what it looks like. Unbudgeted on the particle tier
/// unlike the blast's flash: there is no fireball here for a light to be the
/// flash of, the light IS the effect, and the transient-light cap is already
/// its own budget.
pub(super) fn on_torpedo_ignition(ignited: On<TorpedoIgnited>, mut commands: Commands) {
    trace!("on_torpedo_ignition: torpedo {:?}", ignited.torpedo);
    commands.trigger(LightFlash {
        at: ignited.at,
        // The exhaust's own colour, so the light and the plume it comes from
        // agree.
        color: Color::srgb(1.0, 0.55, 0.15),
        peak_intensity: IGNITION_LIGHT_LUMENS,
        range: IGNITION_LIGHT_RANGE,
        duration: IGNITION_LIGHT_SECS,
    });
}

/// Particle capacity of the built-in launch puff, a per-INSTANCE GPU buffer
/// held by every bay spawner in the scene.
///
/// DERIVED, not picked. The puff bursts 80 particles per launch and a particle
/// lives at most 0.35 s, so a bay holds `80 x fire_rate x 0.35` at once - 28 at
/// the 1 round a second every shipped bay fires at. Sized well above that
/// because the burst is `reset`-driven and a mod may author a faster bay: this
/// covers 18 launches a second before a particle is dropped.
///
/// An authored `launch_effect` brings its own capacity and ignores this.
const LAUNCH_PUFF_CAPACITY: u32 = 512;

/// The generated launch puff, built once and shared by every bay.
///
/// Authoring `launch_effect` on the bay overrides it; this is the fallback.
fn build_default_launch_puff_effect() -> EffectAsset {
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

    let init_pos = SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());

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

    EffectAsset::new(LAUNCH_PUFF_CAPACITY, spawner, writer.finish())
        .with_name("torpedo_launch_burst")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        // The puff is a cloud seen from wherever the camera is. Without this a
        // hanabi quad is expanded on the fixed WORLD axes, so it was a
        // billboard only from the directions nobody was looking from.
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
        .render(size_over_lifetime)
        .render(color_over_lifetime)
}

/// The shared fallback launch puff. Every bay spawner used to mint its own and
/// they were byte-identical - the launch direction and the ship's motion both
/// arrive through properties, so there was never anything per-bay in the asset.
#[derive(Resource, Default)]
pub(crate) struct DefaultLaunchPuffEffect(Option<Handle<EffectAsset>>);

impl DefaultLaunchPuffEffect {
    /// The shared puff, building it on the first bay that needs it.
    fn handle(&mut self, effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
        self.0
            .get_or_insert_with(|| effects.add(build_default_launch_puff_effect()))
            .clone()
    }
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
    mut default_puff: ResMut<DefaultLaunchPuffEffect>,
    asset_server: Res<AssetServer>,
    budget: Option<Res<GraphicsBudget>>,
    q_effect: Query<&TorpedoSectionSpawnerEffect, With<TorpedoSectionSpawnerMarker>>,
) {
    let entity = add.entity;
    trace!("insert_torpedo_spawner_effect: entity {:?}", entity);

    // Low graphics tier is spawn-less: skip the launch-burst hanabi. Absent
    // budget (settings-less app) means full quality.
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
        None => default_puff.handle(&mut effects),
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
    q_projectile: Query<
        (&TorpedoSectionSpawnerEntity, Option<&ProjectileOwner>),
        With<TorpedoProjectileMarker>,
    >,
    q_ship_velocity: Query<&LinearVelocity>,
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
    // missing-effect branch below would `error!` on every launch.
    if !budget.as_deref().is_none_or(|b| b.particles) {
        return;
    }

    let Ok((spawner, owner)) = q_projectile.get(projectile) else {
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
    // The charge leaves with the SHIP. Pinned to zero the puff hung at the
    // point in space the bay had been at, which at any closing speed is a cloud
    // the launching ship drives out of - the same defect the muzzle flash and
    // the blast both carried.
    let base_velocity = owner
        .and_then(|owner| q_ship_velocity.get(**owner).ok())
        .map_or(Vec3::ZERO, |velocity| velocity.0);
    properties.set("base_velocity", base_velocity.into());

    effect_spawner.reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The warhead must point the way the torpedo flies. The body mesh is built
    /// in the CONTROLLER's frame, which `shoot_spawn_projectile` mounts a
    /// quarter turn about X - so the mesh axis and the flight axis are only
    /// related through that rotation, and a sign error here flies the torpedo
    /// tail-first or sideways with nothing else to catch it.
    #[test]
    fn the_torpedo_body_points_along_the_torpedo_forward() {
        use bevy::mesh::VertexAttributeValues;

        // The authored controller mount, verbatim from `shoot_spawn_projectile`.
        let controller = Quat::from_euler(EulerRot::XYZ, std::f32::consts::FRAC_PI_2, 0.0, 0.0);
        let mesh = nose_cone_mesh(0.16, 0.65, 0.35).rotated_by(Quat::from_rotation_x(PI));
        let positions: Vec<Vec3> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(p)) => {
                p.iter().copied().map(Vec3::from_array).collect()
            }
            other => panic!("unexpected positions: {other:?}"),
        };

        // The tip, in the torpedo root's frame.
        let tip = positions
            .iter()
            .map(|&p| controller * p)
            .min_by(|a, b| a.z.total_cmp(&b.z))
            .expect("a vertex");

        // A torpedo flies down its own -Z (its thruster child sits at +Z), and
        // the leading point is the nose on the axis.
        assert!(tip.z < 0.0, "the nose leads: {tip:?}");
        assert!(
            tip.x.abs() < 1e-5 && tip.y.abs() < 1e-5,
            "the leading vertex is the cone tip on the flight axis: {tip:?}"
        );
    }

    /// Every launch reuses one body mesh. A torpedo is not fired at a bullet's
    /// rate, but the old path built a mesh per launch and never freed it.
    #[test]
    fn the_torpedo_body_mesh_is_built_once() {
        use bevy::asset::AssetPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_resource::<DefaultTorpedoRender>();
        app.add_observer(insert_torpedo_controller_render);
        app.update();

        let meshes_before = app.world().resource::<Assets<Mesh>>().len();
        assert_eq!(meshes_before, 1, "one shared body mesh is built");

        let mut launched = Vec::new();
        for _ in 0..8 {
            let torpedo = app
                .world_mut()
                .spawn((TorpedoProjectileMarker, TorpedoProjectileRenderMesh(None)))
                .id();
            let controller = app
                .world_mut()
                .spawn((TorpedoControllerMarker, ChildOf(torpedo)))
                .id();
            app.update();
            launched.push(controller);
        }

        assert_eq!(
            app.world().resource::<Assets<Mesh>>().len(),
            meshes_before,
            "launching must not add mesh assets"
        );
        let handles: Vec<_> = launched
            .iter()
            .map(|&e| app.world().get::<Mesh3d>(e).expect("body mesh").0.clone())
            .collect();
        assert!(
            handles.iter().all(|h| *h == handles[0]),
            "every torpedo shares one body mesh handle"
        );
    }

    /// Two torpedo types must be tellable apart IN THE AIR, and the body colour
    /// is the half of that a player reads before the flight path has drawn
    /// itself. Sharing a material by tint must not cost that: a second tint is
    /// still a second material.
    #[test]
    fn a_torpedo_flies_in_its_own_types_colour() {
        use bevy::asset::AssetPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_resource::<DefaultTorpedoRender>();
        app.add_observer(insert_torpedo_controller_render);
        app.update();

        let flown_in = |app: &mut App, tint: Color| {
            let torpedo = app
                .world_mut()
                .spawn((
                    TorpedoProjectileMarker,
                    TorpedoProjectileRenderMesh(None),
                    TorpedoType {
                        name: "Test".to_string(),
                        tint,
                    },
                ))
                .id();
            let controller = app
                .world_mut()
                .spawn((TorpedoControllerMarker, ChildOf(torpedo)))
                .id();
            app.update();
            let handle = app
                .world()
                .get::<MeshMaterial3d<StandardMaterial>>(controller)
                .expect("body material")
                .0
                .clone();
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&handle)
                .expect("material")
                .base_color
        };

        let lance = Color::srgb(0.7, 0.78, 0.86);
        let serpent = Color::srgb(0.95, 0.45, 0.1);
        assert_eq!(flown_in(&mut app, lance), lance);
        assert_eq!(flown_in(&mut app, serpent), serpent);
        assert_eq!(
            app.world().resource::<DefaultTorpedoRender>().tints(),
            2,
            "two tints are two materials"
        );
    }

    /// A salvo of one ordnance type is ONE warhead material however many tubes
    /// fired it. The frame is what this buys: a distinct material is extracted,
    /// prepared, bound and written every frame however many entities share it,
    /// and `damage_cracks` keys its bucket materials on the SOURCE - so a
    /// private material per launch cost the frame the salvo size, times the
    /// crack buckets each warhead reached.
    #[test]
    fn a_salvo_of_one_type_shares_one_body_material() {
        use bevy::asset::AssetPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_resource::<DefaultTorpedoRender>();
        app.add_observer(insert_torpedo_controller_render);
        app.update();

        let mut launched = Vec::new();
        for _ in 0..64 {
            let torpedo = app
                .world_mut()
                .spawn((
                    TorpedoProjectileMarker,
                    TorpedoProjectileRenderMesh(None),
                    TorpedoType {
                        name: "Lance".to_string(),
                        tint: Color::srgb(0.7, 0.78, 0.86),
                    },
                ))
                .id();
            let controller = app
                .world_mut()
                .spawn((TorpedoControllerMarker, ChildOf(torpedo)))
                .id();
            app.update();
            launched.push(controller);
        }

        assert_eq!(
            app.world().resource::<Assets<StandardMaterial>>().len(),
            1,
            "sixty-four launches of one type build one material"
        );
        let handles: Vec<_> = launched
            .iter()
            .map(|&e| {
                app.world()
                    .get::<MeshMaterial3d<StandardMaterial>>(e)
                    .expect("body material")
                    .0
                    .clone()
            })
            .collect();
        assert!(
            handles.iter().all(|h| *h == handles[0]),
            "every torpedo of one type shares one material handle"
        );
    }

    /// The torpedo bay reads its `render_mesh_transform` STRAIGHT OFF THE CONFIG
    /// (unlike hull/thruster/controller which snapshot it into a component), so
    /// this exercises that distinct path end to end: the authored transform must
    /// land on the meshed body render child, identity when unset.
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
            app.init_resource::<PlaceholderArt>();
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
            ..default()
        };
        let got = child_transform(Some(authored));
        assert_eq!(got.translation, authored.position);
        assert!(got.rotation.abs_diff_eq(authored.rotation, 1e-5));

        assert_eq!(child_transform(None), Transform::IDENTITY);
    }
}
