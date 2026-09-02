//! The wake a slug leaves: ionized rail material hanging in the frame behind
//! the shot, and the light the slug throws on the hulls it passes.
//!
//! A slug is a dart and a stretched tracer, and both are gone the frame after
//! it hits. The wake is what lets the shot leave energy in the frame: a soft
//! cyan haze the slug's path is written into, thinning over half a second,
//! and sparse violet filaments running through it. It is ionized material in
//! vacuum and not smoke, so it neither billows nor drifts on a wind.
//!
//! # Why the wake is not a child of the slug
//!
//! The slug is despawned at impact. A particle effect parented to it would
//! take its live particles with it, and the whole point of the wake is to
//! outlive the thing that left it. So each layer is its own entity riding the
//! slug's transform ([`follow_railgun_wakes`]), and when the slug goes the
//! emitter stops spawning and lingers for one lifetime before it despawns.
//!
//! # Why particles are spread along the ground covered, not left at a point
//!
//! At 1500 u/s a slug crosses more than twenty units between two fixed steps.
//! A spawner that emits where the emitter IS draws the wake as a row of
//! puffs one step apart. Instead every frame's particles are placed a random
//! fraction of the way back along the segment the slug covered since the
//! last spawn, and born that much older ([`count_railgun_wake_spawns`]), so
//! the wake is one continuous line at any speed and any frame rate.
//!
//! Hanabi's global simulation space adds the emitter's TRANSLATION to a new
//! particle and nothing else, so everything the graph is handed is in world
//! orientation: the segment back to the last spawn, and the axis of flight.
//!
//! # The light
//!
//! A real point light rides the slug, so a hull the shot passes is lit by
//! it. It takes one of the [`GraphicsBudget::transient_lights`] slots on the
//! same terms as a flash - refused when the cap is full - and gives it back
//! when the slug despawns. Tuned on `railgun_wake_bench`.

use bevy::ecs::system::SystemParam;
use bevy_hanabi::{
    graph::expr::PropertyHandle, Attribute, CompiledParticleEffect, EffectAsset, EffectMaterial,
    EffectProperties, EffectSpawner, ExprWriter, OrientMode, OrientModifier, ParticleEffect,
    ScalarType, SetAttributeModifier, SpawnerSettings,
};
use nova_gameplay::transient_light::prelude::{CappedLight, TransientLight};

use super::*;

/// How the wake is drawn. The weapon fixes it at [`Default`]; the bench
/// slides it live.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailgunWakeTuning {
    /// Seconds a haze particle lives. The wake's length is this times the
    /// slug's speed: half a second is 750 units behind the shipped lance.
    pub lifetime: f32,
    /// Haze particles per world unit of flight.
    pub density: f32,
    /// A haze particle's size at full growth, world units.
    pub width: f32,
    /// Multiplier on the haze's HDR colour. Zero draws none.
    pub haze_intensity: f32,
    /// Multiplier on the filaments' HDR colour. Zero draws none.
    pub filament_intensity: f32,
    /// Spread each frame's particles along the ground covered since the
    /// last spawn. Off only to show the clustering it removes.
    pub spread: bool,
}

impl Default for RailgunWakeTuning {
    fn default() -> Self {
        Self {
            lifetime: 0.5,
            density: 6.0,
            width: 1.5,
            haze_intensity: 1.0,
            filament_intensity: 1.0,
            spread: true,
        }
    }
}

/// The two halves of the wake.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RailgunWakeLayer {
    /// Camera-facing soft dots, expanding and fading in place.
    Haze,
    /// Velocity-oriented streaks running back and forth along the line.
    Filaments,
}

/// The particles a haze emitter may hold at once, a per-instance GPU buffer.
///
/// The shipped tuning holds 4500 over the fastest lance's wake; past the
/// capacity hanabi drops spawns, which shows as a wake thinning at its tail.
const HAZE_CAPACITY: u32 = 8192;

/// The particles a filament emitter may hold, on the same terms.
const FILAMENT_CAPACITY: u32 = 2048;

/// Filaments per haze particle. The filaments are the sparse half by design:
/// at one to one they read as a second, busier haze.
const FILAMENT_RATIO: f32 = 0.25;

/// Filament lifetime as a fraction of the haze's. A discharge is over before
/// the gas it ran through has thinned.
const FILAMENT_LIFE: f32 = 0.7;

/// A filament's thickness, world units. Thin against the haze, but not so
/// thin that it is under a pixel from a chase camera.
const FILAMENT_THICKNESS: f32 = 0.2;

/// How long a retired emitter lingers past its longest particle, seconds.
const LINGER_MARGIN: f32 = 0.05;

/// Brightness of the light riding the slug, in lumens.
pub const RAILGUN_SLUG_LIGHT_LUMENS: f32 = 300_000.0;

/// How far the slug's light reaches, in world units.
pub const RAILGUN_SLUG_LIGHT_RANGE: f32 = 25.0;

/// The light riding a slug. A child of the slug, so it dies with it and the
/// slot it holds is released on the same frame.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct RailgunSlugLight;

/// A wake emitter following a slug.
#[derive(Component, Debug)]
pub struct RailgunWakeEmitter {
    /// The slug it rides. Retires when this is gone.
    pub slug: Entity,
    /// Which half of the wake this draws.
    pub layer: RailgunWakeLayer,
    /// The values it draws with, read every frame so a change reaches
    /// particles already in flight.
    pub tuning: RailgunWakeTuning,
    /// Where the slug was when this emitter last spawned particles. The next
    /// spawn is spread from here to where the slug is now.
    anchor: Vec3,
    /// Seconds since `anchor` was set, so the particles spread back along
    /// the segment are born that much older.
    anchor_age: f32,
    /// Fractional particles carried to the next frame.
    remainder: f32,
    retiring: bool,
}

/// The shared wake graphs, built on the first slug that needs them.
///
/// Lazy rather than [`FromWorld`], so an app that never fires a lance - and
/// one on a graphics tier with particles off - builds nothing. One graph per
/// layer for every slug: what differs per shot arrives through properties.
#[derive(Resource, Default, Debug)]
pub struct RailgunWakeArt {
    haze: Option<Handle<EffectAsset>>,
    filaments: Option<Handle<EffectAsset>>,
}

impl RailgunWakeArt {
    fn handle(
        &mut self,
        layer: RailgunWakeLayer,
        effects: &mut Assets<EffectAsset>,
    ) -> Handle<EffectAsset> {
        match layer {
            RailgunWakeLayer::Haze => self
                .haze
                .get_or_insert_with(|| effects.add(build_haze_effect()))
                .clone(),
            RailgunWakeLayer::Filaments => self
                .filaments
                .get_or_insert_with(|| effects.add(build_filament_effect()))
                .clone(),
        }
    }
}

/// Everything spawning a wake needs, so the observer and the bench call one
/// seam.
#[derive(SystemParam)]
pub struct RailgunWakeSpawner<'w> {
    art: ResMut<'w, RailgunWakeArt>,
    effects: ResMut<'w, Assets<EffectAsset>>,
    images: ResMut<'w, Assets<Image>>,
    soft_dot: ResMut<'w, SoftDot>,
}

impl RailgunWakeSpawner<'_> {
    /// Spawn both layers of a wake following `slug`, which is at `transform`.
    pub fn spawn(
        &mut self,
        commands: &mut Commands,
        slug: Entity,
        transform: Transform,
        tuning: RailgunWakeTuning,
    ) {
        let mask = self.soft_dot.handle(&mut self.images);
        for layer in [RailgunWakeLayer::Haze, RailgunWakeLayer::Filaments] {
            let mut properties = EffectProperties::default();
            properties.set("back", Vec3::ZERO.into());
            properties.set("axis", Vec3::Z.into());
            properties.set("frame_dt", 0.0f32.into());
            properties.set("life", layer_lifetime(&tuning, layer).into());
            properties.set("width", tuning.width.into());
            properties.set("intensity", layer_intensity(&tuning, layer).into());
            commands.spawn((
                Name::new(format!("Railgun Wake {layer:?}")),
                RailgunWakeEmitter {
                    slug,
                    layer,
                    tuning,
                    anchor: transform.translation,
                    anchor_age: 0.0,
                    remainder: 0.0,
                    retiring: false,
                },
                transform,
                ParticleEffect::new(self.art.handle(layer, &mut self.effects)),
                EffectMaterial {
                    images: vec![mask.clone()],
                },
                properties,
            ));
        }
    }
}

/// The particle lifetime one layer gets.
fn layer_lifetime(tuning: &RailgunWakeTuning, layer: RailgunWakeLayer) -> f32 {
    match layer {
        RailgunWakeLayer::Haze => tuning.lifetime,
        RailgunWakeLayer::Filaments => tuning.lifetime * FILAMENT_LIFE,
    }
}

fn layer_intensity(tuning: &RailgunWakeTuning, layer: RailgunWakeLayer) -> f32 {
    match layer {
        RailgunWakeLayer::Haze => tuning.haze_intensity,
        RailgunWakeLayer::Filaments => tuning.filament_intensity,
    }
}

/// Particles one layer spawns per world unit of flight.
fn layer_density(tuning: &RailgunWakeTuning, layer: RailgunWakeLayer) -> f32 {
    match layer {
        RailgunWakeLayer::Haze => tuning.density,
        RailgunWakeLayer::Filaments => tuning.density * FILAMENT_RATIO,
    }
}

/// Particles a layer owes for `covered` units of flight, carrying the
/// fraction it cannot spawn to the next frame.
///
/// Pure, so the spread's arithmetic can be read without a running app: the
/// count is whole, the remainder is what was left under one particle, and
/// nothing is lost between frames whatever their length.
fn owed_particles(covered: f32, per_unit: f32, remainder: f32) -> (u32, f32) {
    let wanted = (covered * per_unit + remainder).max(0.0);
    let count = wanted.floor();
    (count as u32, wanted - count)
}

/// Ride each emitter on its slug and hand it the frame's properties; retire
/// the ones whose slug has gone.
pub(super) fn follow_railgun_wakes(
    mut commands: Commands,
    time: Res<Time>,
    q_slug: Query<&Transform, With<RailgunSlugProjectileMarker>>,
    mut q_emitter: Query<
        (
            Entity,
            &mut RailgunWakeEmitter,
            &mut Transform,
            &mut EffectProperties,
            Option<&mut EffectSpawner>,
        ),
        Without<RailgunSlugProjectileMarker>,
    >,
) {
    for (entity, mut emitter, mut transform, mut properties, spawner) in &mut q_emitter {
        let Ok(slug) = q_slug.get(emitter.slug) else {
            if !emitter.retiring {
                emitter.retiring = true;
                if let Some(mut spawner) = spawner {
                    spawner.active = false;
                }
                let linger = layer_lifetime(&emitter.tuning, emitter.layer) * 1.3 + LINGER_MARGIN;
                commands.entity(entity).insert(TempEntity(linger));
            }
            continue;
        };

        *transform = *slug;
        emitter.anchor_age += time.delta_secs();
        // World vectors, because that is the frame the particles are born in
        // (see the module docs). Zero puts every spawn at the emitter, which
        // is the clustering the spread exists to remove.
        let back = if emitter.tuning.spread {
            emitter.anchor - slug.translation
        } else {
            Vec3::ZERO
        };
        // +Z in the slug's frame is behind it: the direction the wake runs.
        let axis = slug.rotation * Vec3::Z;
        properties.set("back", back.into());
        properties.set("axis", axis.into());
        properties.set("frame_dt", emitter.anchor_age.into());
        properties.set(
            "life",
            layer_lifetime(&emitter.tuning, emitter.layer).into(),
        );
        properties.set("width", emitter.tuning.width.into());
        properties.set(
            "intensity",
            layer_intensity(&emitter.tuning, emitter.layer).into(),
        );
    }
}

/// Say how many particles each emitter spawns this frame: the density times
/// the ground its slug covered since the last spawn.
///
/// After hanabi's own tick, which wrote the asset's rate (zero) and would
/// overwrite this if it ran second. Only a READY effect is charged: a spawn
/// count set on one still compiling is dropped by the renderer, and the
/// ground it covered would be lost from the wake - at 1500 u/s that is the
/// first twenty-odd units of every shot.
pub(super) fn count_railgun_wake_spawns(
    mut q_emitter: Query<(
        &mut RailgunWakeEmitter,
        &Transform,
        &CompiledParticleEffect,
        &mut EffectSpawner,
    )>,
) {
    for (mut emitter, transform, compiled, mut spawner) in &mut q_emitter {
        if emitter.retiring || !spawner.active || !compiled.is_ready() {
            spawner.spawn_count = 0;
            continue;
        }
        let covered = (transform.translation - emitter.anchor).length();
        let per_unit = layer_density(&emitter.tuning, emitter.layer);
        let (count, remainder) = owed_particles(covered, per_unit, emitter.remainder);
        emitter.remainder = remainder;
        spawner.spawn_count = count;
        emitter.anchor = transform.translation;
        emitter.anchor_age = 0.0;
    }
}

/// Give a slug its light, if the shared cap has a slot for it.
///
/// One QUEUED command, as the flash observer does it: a volley spawns several
/// slugs inside one flush, and a child spawned through `Commands` is invisible
/// to a query until the next one, so every slug in the volley would count the
/// same number and every one would be lit. Queued commands run in order
/// against the real world, so the second slug sees the first one's light.
pub(super) fn light_railgun_slug(commands: &mut Commands, slug: Entity) {
    commands.queue(move |world: &mut World| {
        let cap = world.get_resource::<GraphicsBudget>().map_or_else(
            || GraphicsBudget::default().transient_lights,
            |budget| budget.transient_lights,
        );
        let lit = world
            .query_filtered::<(), Or<(With<TransientLight>, With<CappedLight>)>>()
            .iter(world)
            .count();
        if lit >= cap {
            trace!("light_railgun_slug: {lit} already lit, cap {cap} - unlit slug");
            return;
        }
        // The slug can be gone before this runs: a shot into a hull at the
        // muzzle is hit and despawned in the tick it was born.
        if world.get_entity(slug).is_err() {
            return;
        }
        world.spawn((
            Name::new("Railgun Slug Light"),
            RailgunSlugLight,
            CappedLight,
            ChildOf(slug),
            PointLight {
                color: RAIL_GLOW_COLOR,
                intensity: RAILGUN_SLUG_LIGHT_LUMENS,
                range: RAILGUN_SLUG_LIGHT_RANGE,
                radius: 0.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
        ));
    });
}

/// The properties every wake graph takes. Both layers share them so one
/// [`follow_railgun_wakes`] serves both.
struct WakeProperties {
    /// From the emitter back to where it last spawned, world space.
    back: PropertyHandle,
    /// Unit vector along the wake, world space (behind the slug).
    axis: PropertyHandle,
    /// Seconds since the emitter was at the far end of `back`.
    frame_dt: PropertyHandle,
    life: PropertyHandle,
    width: PropertyHandle,
    intensity: PropertyHandle,
}

impl WakeProperties {
    fn declare(writer: &ExprWriter) -> Self {
        Self {
            back: writer.add_property("back", Vec3::ZERO.into()),
            axis: writer.add_property("axis", Vec3::Z.into()),
            frame_dt: writer.add_property("frame_dt", 0.0f32.into()),
            life: writer.add_property("life", 0.5f32.into()),
            width: writer.add_property("width", 1.0f32.into()),
            intensity: writer.add_property("intensity", 1.0f32.into()),
        }
    }
}

/// A random vector with each component in `[-0.5, 0.5]`.
fn jitter(writer: &ExprWriter) -> bevy_hanabi::WriterExpr {
    (writer.rand(ScalarType::Float) - writer.lit(0.5)).vec3(
        writer.rand(ScalarType::Float) - writer.lit(0.5),
        writer.rand(ScalarType::Float) - writer.lit(0.5),
    )
}

/// `intensity` spread over RGB with alpha left alone.
fn gain(writer: &ExprWriter, props: &WakeProperties) -> bevy_hanabi::WriterExpr {
    writer
        .prop(props.intensity)
        .vec3(writer.prop(props.intensity), writer.prop(props.intensity))
        .vec4_xyz_w(writer.lit(1.0))
}

/// The wake's haze: camera-facing soft dots, left in world space, expanding
/// and fading.
///
/// Two random fractions are written to attributes first and read back, so
/// the position and the age of one particle agree on where along the
/// segment it was born - a `rand()` used twice is two numbers. Size and
/// colour are functions of age in the update pass, so a tuning change
/// reaches particles already in flight.
fn build_haze_effect() -> EffectAsset {
    let writer = ExprWriter::new();
    let props = WakeProperties::declare(&writer);

    let along = SetAttributeModifier::new(Attribute::F32_0, writer.rand(ScalarType::Float).expr());
    let seed = SetAttributeModifier::new(Attribute::F32_1, writer.rand(ScalarType::Float).expr());
    let t = writer.attr(Attribute::F32_0);
    let j = writer.attr(Attribute::F32_1);

    let scatter = jitter(&writer) * writer.prop(props.width) * writer.lit(0.4);
    let init_pos = SetAttributeModifier::new(
        Attribute::POSITION,
        (writer.prop(props.back) * t.clone() + scatter).expr(),
    );
    let init_age =
        SetAttributeModifier::new(Attribute::AGE, (writer.prop(props.frame_dt) * t).expr());
    let init_life = SetAttributeModifier::new(
        Attribute::LIFETIME,
        (writer.prop(props.life) * (writer.lit(0.7) + j * writer.lit(0.6))).expr(),
    );
    // A slow drift in a random direction. The size curve does most of the
    // expanding; this stops the haze reading as a row of beads.
    let drift = jitter(&writer).normalized()
        * (writer.lit(0.5) + writer.rand(ScalarType::Float) * writer.lit(2.5));
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, drift.expr());

    let age = writer.attr(Attribute::AGE) / writer.attr(Attribute::LIFETIME);
    let grown = writer.lit(0.35)
        + writer.lit(0.65) * age.clone().smoothstep(writer.lit(0.0), writer.lit(1.0));
    let size = writer.prop(props.width)
        * grown
        * (writer.lit(0.7) + writer.attr(Attribute::F32_1) * writer.lit(0.6));
    let update_size = SetAttributeModifier::new(Attribute::SIZE, size.expr());

    // White-cyan cooling to a dim blue. Alpha rides the mix to zero so the
    // haze thins out rather than cutting off.
    let hot = writer.lit(Vec4::new(1.2, 2.6, 3.2, 0.5));
    let cold = writer.lit(Vec4::new(0.25, 0.55, 1.4, 0.0));
    let fade = writer.lit(1.0) - age.clone();
    let color = hot.mix(cold, age) * gain(&writer, &props) * fade;
    let update_color = SetAttributeModifier::new(Attribute::HDR_COLOR, color.expr());

    let mask = soft_dot_modifier(&writer);
    let mut module = writer.finish();
    declare_soft_dot_slot(&mut module);

    EffectAsset::new(HAZE_CAPACITY, SpawnerSettings::rate(0.0.into()), module)
        .with_name("railgun_wake_haze")
        .with_alpha_mode(bevy_hanabi::AlphaMode::Add)
        .init(along)
        .init(seed)
        .init(init_pos)
        .init(init_age)
        .init(init_life)
        .init(init_vel)
        .update(update_size)
        .update(update_color)
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
        .render(mask)
}

/// The wake's filaments: short velocity-oriented streaks running back and
/// forth along the line, flickering.
///
/// The same properties and the same segment spread as the haze. What is
/// different is everything that makes a discharge rather than a gas: a
/// streak along its own motion, a motion along the wake with a random sign,
/// and a brightness that strobes on the simulation clock.
fn build_filament_effect() -> EffectAsset {
    let writer = ExprWriter::new();
    let props = WakeProperties::declare(&writer);

    let along = SetAttributeModifier::new(Attribute::F32_0, writer.rand(ScalarType::Float).expr());
    let phase = SetAttributeModifier::new(Attribute::F32_1, writer.rand(ScalarType::Float).expr());
    let t = writer.attr(Attribute::F32_0);

    let scatter = jitter(&writer) * writer.prop(props.width) * writer.lit(0.6);
    let init_pos = SetAttributeModifier::new(
        Attribute::POSITION,
        (writer.prop(props.back) * t.clone() + scatter).expr(),
    );
    let init_age =
        SetAttributeModifier::new(Attribute::AGE, (writer.prop(props.frame_dt) * t).expr());
    let init_life = SetAttributeModifier::new(
        Attribute::LIFETIME,
        (writer.prop(props.life)
            * (writer.lit(0.5) + writer.rand(ScalarType::Float) * writer.lit(0.5)))
        .expr(),
    );
    // Along the line either way, fast, with a sideways kick: the streak's
    // orientation follows this, so a filament lies along the wake and leans.
    let sign = (writer.rand(ScalarType::Float) - writer.lit(0.5)).sign();
    let run = sign * (writer.lit(10.0) + writer.rand(ScalarType::Float) * writer.lit(30.0));
    let kick = jitter(&writer) * writer.lit(10.0);
    let init_vel = SetAttributeModifier::new(
        Attribute::VELOCITY,
        (writer.prop(props.axis) * run + kick).expr(),
    );

    let age = writer.attr(Attribute::AGE) / writer.attr(Attribute::LIFETIME);
    let phase_at = writer.attr(Attribute::F32_1);
    // Long on X (the velocity axis), thin across; shortening as it dies.
    let length = (writer.lit(1.5) + phase_at.clone() * writer.lit(2.5))
        * (writer.lit(1.0) - age.clone() * writer.lit(0.6));
    let thickness = writer.lit(FILAMENT_THICKNESS);
    let update_size = SetAttributeModifier::new(
        Attribute::SIZE3,
        length.vec3(thickness.clone(), thickness).expr(),
    );

    // A strobe on the simulation clock, per particle phase, on a bit over
    // half the time and never fully off.
    let strobe = (writer.time() * writer.lit(20.0) + phase_at * writer.lit(7.0))
        .fract()
        .smoothstep(writer.lit(0.35), writer.lit(0.45));
    let flicker = writer.lit(0.25) + writer.lit(0.75) * strobe;
    let violet = writer.lit(Vec4::new(2.2, 1.4, 6.0, 1.0));
    let fade = writer.lit(1.0) - age;
    let color = violet * gain(&writer, &props) * flicker * fade;
    let update_color = SetAttributeModifier::new(Attribute::HDR_COLOR, color.expr());

    let mask = soft_dot_modifier(&writer);
    let mut module = writer.finish();
    declare_soft_dot_slot(&mut module);

    EffectAsset::new(FILAMENT_CAPACITY, SpawnerSettings::rate(0.0.into()), module)
        .with_name("railgun_wake_filaments")
        .with_alpha_mode(bevy_hanabi::AlphaMode::Add)
        .init(along)
        .init(phase)
        .init(init_pos)
        .init(init_age)
        .init(init_life)
        .init(init_vel)
        .update(update_size)
        .update(update_color)
        .render(OrientModifier::new(OrientMode::AlongVelocity))
        .render(mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_owes_the_density_times_the_ground_covered() {
        assert_eq!(owed_particles(10.0, 6.0, 0.0), (60, 0.0));
    }

    #[test]
    fn the_fraction_under_one_particle_carries_to_the_next_frame() {
        let (count, remainder) = owed_particles(0.25, 1.0, 0.0);
        assert_eq!((count, remainder), (0, 0.25));
        let (count, remainder) = owed_particles(0.25, 1.0, remainder);
        assert_eq!((count, remainder), (0, 0.5));
        assert_eq!(owed_particles(0.5, 1.0, remainder), (1, 0.0));
    }

    #[test]
    fn a_still_slug_owes_nothing() {
        assert_eq!(owed_particles(0.0, 6.0, 0.0), (0, 0.0));
    }

    #[test]
    fn the_filaments_are_the_sparse_short_lived_half() {
        let tuning = RailgunWakeTuning::default();
        assert!(
            layer_density(&tuning, RailgunWakeLayer::Filaments)
                < layer_density(&tuning, RailgunWakeLayer::Haze)
        );
        assert!(
            layer_lifetime(&tuning, RailgunWakeLayer::Filaments)
                < layer_lifetime(&tuning, RailgunWakeLayer::Haze)
        );
    }

    #[test]
    fn the_shipped_wake_fits_its_buffers_behind_the_fastest_lance() {
        let tuning = RailgunWakeTuning::default();
        let length = tuning.lifetime * 1500.0;
        let haze = length * layer_density(&tuning, RailgunWakeLayer::Haze);
        let filaments =
            length * FILAMENT_LIFE * layer_density(&tuning, RailgunWakeLayer::Filaments);
        assert!(
            haze <= HAZE_CAPACITY as f32,
            "haze {haze} over {HAZE_CAPACITY}"
        );
        assert!(
            filaments <= FILAMENT_CAPACITY as f32,
            "filaments {filaments} over {FILAMENT_CAPACITY}"
        );
    }

    #[test]
    fn both_graphs_build() {
        let haze = build_haze_effect();
        let filaments = build_filament_effect();
        assert_eq!(haze.capacity(), HAZE_CAPACITY);
        assert_eq!(filaments.capacity(), FILAMENT_CAPACITY);
    }
}
