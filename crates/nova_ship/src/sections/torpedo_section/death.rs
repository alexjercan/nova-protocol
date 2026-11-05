//! The look of a torpedo dying without its warhead, answering
//! [`TorpedoKilled`]: a dud is a grey puff of broken casing, an intercept is a
//! small hot pop with a few sparks. Both are far smaller than a detonation, so
//! neither reads as a warhead going off.
//!
//! Gated on [`GraphicsBudget::particles`] like the blast. The sound half lives
//! in `ship_audio` and plays on every tier.

use super::*;

/// Seconds the dud puff entity lives: its longest particle and a margin.
const DUD_PUFF_SECS: f32 = 0.8;

/// Seconds the intercept pop entity lives: its longest particle and a margin.
const INTERCEPT_POP_SECS: f32 = 0.3;

/// Sparks an intercept throws. A handful: the pop is the event, the sparks
/// say it was a hit by something hot.
const INTERCEPT_SPARKS: u32 = 6;

/// Particle capacity of either death effect, a per-instance buffer. Each
/// emits once on start and is never reset, so this is the next power of two
/// above the larger burst.
const DEATH_EFFECT_CAPACITY: u32 = 32;

/// The two shared death effects, built once on the first death that needs
/// each, for the same reason [`DefaultBlastEffect`] is.
#[derive(Resource, Default, Debug)]
pub(super) struct DefaultTorpedoDeathEffects {
    dud: Option<Handle<EffectAsset>>,
    intercept: Option<Handle<EffectAsset>>,
}

impl DefaultTorpedoDeathEffects {
    /// The shared effect for `death`.
    fn handle(
        &mut self,
        death: TorpedoDeathType,
        effects: &mut Assets<EffectAsset>,
    ) -> Handle<EffectAsset> {
        match death {
            TorpedoDeathType::Contact => self
                .dud
                .get_or_insert_with(|| effects.add(build_dud_puff_effect()))
                .clone(),
            TorpedoDeathType::Damage => self
                .intercept
                .get_or_insert_with(|| effects.add(build_intercept_pop_effect()))
                .clone(),
        }
    }
}

/// Draw a torpedo's death: the dud puff or the intercept pop.
///
/// Not throttled: each torpedo is one small effect, and a salvo broken up in
/// one patch of sky should still show every torpedo it lost. The sound is the
/// half that throttles.
pub(super) fn on_torpedo_killed_effect(
    killed: On<TorpedoKilled>,
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    mut images: ResMut<Assets<Image>>,
    mut defaults: ResMut<DefaultTorpedoDeathEffects>,
    mut soft_dot: ResMut<SoftDot>,
    budget: Option<Res<GraphicsBudget>>,
) {
    // Absent budget (settings-less app) means full quality, as for the blast.
    if !budget.as_deref().is_none_or(|b| b.particles) {
        return;
    }
    let (name, lifetime) = match killed.death {
        TorpedoDeathType::Contact => ("Torpedo Dud Puff", DUD_PUFF_SECS),
        TorpedoDeathType::Damage => ("Torpedo Intercept Pop", INTERCEPT_POP_SECS),
    };
    commands.spawn((
        Name::new(name),
        Transform::from_translation(killed.at),
        ParticleEffect::new(defaults.handle(killed.death, &mut effects)),
        EffectMaterial {
            images: vec![soft_dot.handle(&mut images)],
        },
        TempEntity(lifetime),
    ));
    if killed.death == TorpedoDeathType::Damage {
        commands.trigger(ImpactSparks {
            at: killed.at,
            count: INTERCEPT_SPARKS,
            force: 1.0,
        });
    }
}

/// A random unit direction, written into `writer`'s graph.
fn random_direction(writer: &ExprWriter) -> WriterExpr {
    let rand_x = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let rand_y = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let rand_z = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    (writer.lit(Vec3::X) * rand_x + writer.lit(Vec3::Y) * rand_y + writer.lit(Vec3::Z) * rand_z)
        .normalized()
}

/// The dud: a slow grey puff of broken casing, about the torpedo's own size.
///
/// Every colour key stays under 1.0, so nothing blooms. That is the read: a
/// dud is a thing that did NOT go off, and anything bright says it did.
fn build_dud_puff_effect() -> EffectAsset {
    let spawner = SpawnerSettings::once(24.0.into()).with_emit_on_start(true);
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
    let lifetime = writer.lit(0.35).uniform(writer.lit(0.6)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    let mut color_gradient = bevy_hanabi::Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(0.55, 0.55, 0.55, 0.7));
    color_gradient.add_key(0.4, Vec4::new(0.38, 0.38, 0.40, 0.45));
    color_gradient.add_key(1.0, Vec4::new(0.25, 0.25, 0.27, 0.0));
    let color_over_lifetime = ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::default(),
        mask: ColorBlendMask::default(),
    };
    let init_color = SetAttributeModifier::new(Attribute::COLOR, writer.lit(0xFFFFFFFFu32).expr());

    // The torpedo is 2 units long; the puff swells to about that and thins.
    let mut size_gradient = bevy_hanabi::Gradient::new();
    size_gradient.add_key(0.0, Vec3::splat(0.3));
    size_gradient.add_key(0.3, Vec3::splat(0.9));
    size_gradient.add_key(1.0, Vec3::splat(1.2));
    let size_over_lifetime = SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    };

    let init_pos = SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());
    let speed = writer.lit(0.5).uniform(writer.lit(3.0));
    let velocity = random_direction(&writer) * speed;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    let mask = soft_dot_modifier(&writer);
    let mut module = writer.finish();
    declare_soft_dot_slot(&mut module);

    EffectAsset::new(DEATH_EFFECT_CAPACITY, spawner, module)
        .with_name("torpedo_dud_puff")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .init(init_color)
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
        .render(size_over_lifetime)
        .render(mask)
        .render(color_over_lifetime)
}

/// The intercept: a small, sharp, hot pop - the torpedo's fuel and casing
/// flashing as it breaks up. HDR so it blooms a little, but an order of
/// magnitude smaller and shorter than the blast core, so it never reads as a
/// detonation.
fn build_intercept_pop_effect() -> EffectAsset {
    let spawner = SpawnerSettings::once(16.0.into()).with_emit_on_start(true);
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
    let lifetime = writer.lit(0.06).uniform(writer.lit(0.14)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    let mut color_gradient = bevy_hanabi::Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(6.0, 4.5, 2.4, 0.9));
    color_gradient.add_key(0.35, Vec4::new(3.0, 1.2, 0.25, 0.6));
    color_gradient.add_key(1.0, Vec4::new(0.6, 0.1, 0.02, 0.0));
    let color_over_lifetime = ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::default(),
        mask: ColorBlendMask::default(),
    };
    let init_color = SetAttributeModifier::new(Attribute::COLOR, writer.lit(0xFFFFFFFFu32).expr());

    let mut size_gradient = bevy_hanabi::Gradient::new();
    size_gradient.add_key(0.0, Vec3::splat(0.4));
    size_gradient.add_key(0.2, Vec3::splat(1.4));
    size_gradient.add_key(1.0, Vec3::ZERO);
    let size_over_lifetime = SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    };

    let init_pos = SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());
    let speed = writer.lit(4.0).uniform(writer.lit(12.0));
    let velocity = random_direction(&writer) * speed;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    let mask = soft_dot_modifier(&writer);
    let mut module = writer.finish();
    declare_soft_dot_slot(&mut module);

    EffectAsset::new(DEATH_EFFECT_CAPACITY, spawner, module)
        .with_name("torpedo_intercept_pop")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .init(init_color)
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
        .render(size_over_lifetime)
        .render(mask)
        .render(color_over_lifetime)
}
