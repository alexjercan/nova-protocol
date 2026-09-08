//! The fireball a destroyed body throws.
//!
//! [`explode`](super::explode) says what the WRECKAGE does - a dead section
//! detaches, takes a kick and a spin, and drifts. That is the aftermath, and on
//! its own it is all a death ever had: a hull hit hard enough to come apart
//! shed pieces in silence, which reads as parts falling off rather than as a
//! ship being killed. This module is the moment itself.
//!
//! # A death is a core and its ejecta, because vacuum has nothing else
//!
//! There is no shock front and no smoke: what a body in vacuum throws is its
//! own vaporised mass, which expands while it cools, and its fragments, which
//! keep going after the light has gone. Those are two different pictures and a
//! quad cannot draw both - a billboard cannot also be a streak - so each death
//! spawns TWO instances, exactly as a torpedo detonation does
//! (`build_default_blast_core_effect` beside `build_default_blast_effect`):
//!
//! - the CORE is camera-facing, brief, and the only part allowed to be bright.
//!   It is a FLASH and not a fireball: nothing out here sustains combustion, so
//!   it is gone inside a third of a second;
//! - the EJECTA is oriented along velocity, so its quads read as tapered
//!   streaks contracting into fragments rather than as a cloud of circles. It
//!   outlives the flash, and it is what the eye follows afterwards.
//!
//! # Two sizes, because a death has two scales
//!
//! A SECTION dying is a compartment going up: a core about the size of the
//! build-grid cell it stood in, over in a third of a second. A ship's
//! INTEGRITY ROOT dying is the whole hull letting go, and it is the only death
//! in the game that is allowed to fill the frame. Each size is its own pair of
//! [`EffectAsset`]s rather than one asset scaled, because hanabi bakes its
//! size and colour gradients into the asset.
//!
//! # A collapse is a chain, and a chain has to be capped
//!
//! Structural collapse destroys every section a hull has left in ONE frame, so
//! the unbudgeted reading of "one fireball per death" is fifty deaths born
//! together. The chain across the hull is exactly what a ship blowing up looks
//! like, so it is kept - up to [`PYRE_FRAME_CAP`] of it - and the root's own
//! fireball is never the one dropped.
//!
//! Engine units throughout: every size, speed and reach below is world units
//! (one is 10 m) and world units per second, because they are measured against
//! hull geometry and avian velocities. The figures in these docs are the
//! METRES they come to, so the two can be checked against each other - an
//! earlier cut read its own comments as metres, and shipped a section fireball
//! of 9 m quads reaching 126 m, which is a small ship exploding rather than a
//! compartment.

use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use super::components::prelude::*;
use crate::{
    integrity::spew::prelude::{inherited_material, CarveDebris},
    lifetime::TempEntity,
    settings::prelude::GraphicsBudget,
    soft_dot::prelude::{declare_soft_dot_slot, soft_dot_modifier, SoftDot},
    transient_light::prelude::LightFlash,
    GameStates,
};

/// `PyrePlugin` and the marker its instances carry.
pub mod prelude {
    pub use super::{PyreEffectMarker, PyrePlugin};
}

/// Tags a live death fireball, so a range can count them. Both halves of one
/// death carry it.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct PyreEffectMarker;

/// Tags a throwaway instance the warm-up spawned to mint a shader.
///
/// Its own marker and not [`PyreEffectMarker`], because a range counting live
/// fireballs must never see one: these draw nothing, emit nothing, and are
/// gone the frame after they are made.
#[derive(Component, Clone, Copy, Debug)]
struct PyreWarmMarker;

/// How many deaths one frame may light.
///
/// A collapsing hull destroys everything it has left at once, and the read
/// wanted is a chain of blasts walking across the wreck rather than a single
/// puff. Six is enough for that on the largest shipped hull and bounds the
/// per-instance GPU buffers a death can allocate in one frame.
///
/// It is a per-FRAME cap and not a per-death one, so a railgun corridor - a
/// rake that condemns thirty cells over several frames - still lights a fire
/// in every one of them. That is the right read for a wound down the length
/// of a hull; what keeps it from becoming a wall is the section pyre's own
/// reach, which is deliberately shorter than the lens is far.
const PYRE_FRAME_CAP: u32 = 6;

/// How many deaths this frame has already lit.
///
/// Reset in [`First`], spent by the observer. A resource and not a `Local`
/// because the observer and the reset are different systems.
#[derive(Resource, Default, Debug)]
struct PyreBudget(u32);

/// The shared graphs. Two per size: the core and its ejecta.
///
/// Warmed on entering [`GameStates::Playing`] by [`warm_the_pyres`] rather than
/// built by [`FromWorld`], so an app with no asset stores and one running at a
/// graphics tier with particles off still build nothing. The slots stay
/// optional because that is what lets those two apps hold the resource without
/// paying for it, and because the warm-up is a system and not a constructor.
#[derive(Resource, Default, Debug)]
struct PyreEffects {
    section: Option<PyrePair>,
    hulk: Option<PyrePair>,
}

/// Which of the two scales a death burns at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PyreSize {
    /// A compartment going up.
    Section,
    /// A whole hull letting go.
    Hulk,
}

impl PyreSize {
    /// The scale a death of this size is authored at.
    fn scale(self) -> PyreScale {
        match self {
            Self::Section => SECTION_PYRE,
            Self::Hulk => HULK_PYRE,
        }
    }

    /// The stem the two graph names are built from, which is also what a
    /// `bevy_hanabi=debug` log prints when a shader is minted.
    fn name(self) -> &'static str {
        match self {
            Self::Section => "section",
            Self::Hulk => "hulk",
        }
    }
}

/// The two graphs one death spawns.
#[derive(Clone, Debug)]
struct PyrePair {
    core: Handle<EffectAsset>,
    ejecta: Handle<EffectAsset>,
}

/// The vaporised mass: a compact camera-facing flare that expands as it cools.
#[derive(Clone, Copy, Debug)]
struct PyreCore {
    /// Peak quad size, world units. The visible ball is wider than this - the
    /// quads drift apart while they burn.
    size: f32,
    /// Slowest and fastest a core quad drifts, world units per second. Small:
    /// what expands here is the fireball, not the debris.
    drift: (f32, f32),
    /// Shortest and longest a core quad lives, seconds.
    life: (f32, f32),
    /// How many quads the core is made of.
    particles: f32,
    /// Buffer the core is given, the next power of two above
    /// [`Self::particles`].
    capacity: u32,
}

/// The fragments: incandescent streaks that leave and keep going.
#[derive(Clone, Copy, Debug)]
struct PyreEjecta {
    /// Peak streak LENGTH along its own velocity, world units.
    length: f32,
    /// Streak width across that, world units. A fraction of the length, or it
    /// stops reading as a fragment and starts reading as a lozenge.
    width: f32,
    /// Slowest and fastest a fragment leaves, world units per second.
    speed: (f32, f32),
    /// Shortest and longest a fragment lives, seconds. Longer than the core's:
    /// the light goes out while the pieces are still travelling.
    life: (f32, f32),
    /// How many fragments the burst throws.
    particles: f32,
    /// Buffer the burst is given, the next power of two above
    /// [`Self::particles`].
    capacity: u32,
}

/// One death's look: its two halves, and how brightly it lights the hulls
/// around it.
#[derive(Clone, Copy, Debug)]
struct PyreScale {
    /// The vaporised mass.
    core: PyreCore,
    /// The fragments it throws.
    ejecta: PyreEjecta,
    /// Peak intensity of the flash, in lumens.
    lumens: f32,
    /// How far the flash reaches, world units.
    light_range: f32,
    /// How long the flash burns, seconds.
    light_secs: f32,
    /// How long the instances are kept alive after the burst, seconds. Longer
    /// than the longest fragment: an emitter despawned early takes its live
    /// particles with it.
    linger: f32,
}

/// A compartment going up. Sized against the build-grid cell it stood in - one
/// world unit, 10 m - so the core is about the cell across and the fragments
/// stay on the cell they came from.
///
/// The ejecta REACH is the number that decides whether a wound reads: 0.5
/// units per second for at most 0.55 s is 0.28 units of travel, 2.8 m, so a
/// railgun corridor - thirty cells going up along one line - draws a line of
/// separate fires rather than the fog an earlier cut put across the whole
/// hull.
const SECTION_PYRE: PyreScale = PyreScale {
    core: PyreCore {
        size: 0.34,
        drift: (0.10, 0.60),
        life: (0.07, 0.20),
        particles: 24.0,
        capacity: 32,
    },
    ejecta: PyreEjecta {
        length: 0.50,
        width: 0.07,
        speed: (0.12, 0.50),
        life: (0.18, 0.55),
        particles: 56.0,
        capacity: 64,
    },
    lumens: 2_500_000.0,
    light_range: 32.0,
    light_secs: 0.22,
    linger: 0.9,
};

/// The whole hull letting go. Sized against a shipped gunship - 85 m stem to
/// stern, 8.5 units - so the core covers the wreck without swallowing the
/// frame: a death the camera cannot see THROUGH is a death nobody can read,
/// and the sections thrown out of it are half of what makes it one.
///
/// The fragments carry it after that, and they are most of the picture: the
/// flash is over in a third of a second, while at up to 8 units per second for
/// 1.7 s these cross about 13 units, 130 m - half again the length of the hull
/// they came off, still leaving the wreck when the light has gone out, which
/// is the order a vacuum burst happens in. Wider than the wreck on purpose: a
/// field of debris that stopped at the hull's own silhouette would read as the
/// ship having merely broken rather than having been destroyed.
const HULK_PYRE: PyreScale = PyreScale {
    core: PyreCore {
        size: 1.30,
        drift: (0.80, 5.00),
        life: (0.12, 0.34),
        particles: 80.0,
        capacity: 128,
    },
    ejecta: PyreEjecta {
        length: 2.20,
        width: 0.20,
        speed: (1.60, 8.00),
        life: (0.45, 1.70),
        particles: 320.0,
        capacity: 512,
    },
    lumens: 60_000_000.0,
    light_range: 170.0,
    light_secs: 0.65,
    linger: 2.6,
};

impl PyreEffects {
    /// The pair for a death, building it on the first one that needs it.
    fn pair(&mut self, size: PyreSize, effects: &mut Assets<EffectAsset>) -> (PyrePair, PyreScale) {
        let (scale, name) = (size.scale(), size.name());
        let slot = match size {
            PyreSize::Section => &mut self.section,
            PyreSize::Hulk => &mut self.hulk,
        };
        let pair = slot
            .get_or_insert_with(|| PyrePair {
                core: effects.add(build_pyre_core(scale.core, &format!("pyre_{name}_core"))),
                ejecta: effects.add(build_pyre_ejecta(
                    scale.ejecta,
                    &format!("pyre_{name}_ejecta"),
                )),
            })
            .clone();
        (pair, scale)
    }
}

/// The CORE's colour: white-hot, and gone.
///
/// Vacuum is the whole argument for this curve. There is no oxidiser out here
/// and nothing to hold pressure, so what a hull throws when it lets go is
/// incandescent for as long as it takes the vapour to thin, and then it is
/// dark - there is no burning fireball to sit and watch. So the alpha is at
/// its highest on the first key and is down to a tenth by the time the quad is
/// half way through its life. Everything after that is the ejecta's.
///
/// Alpha is also what decides whether a death reads as fire or as bubbles.
/// These quads are drawn over each other by the dozen, and at the near-opaque
/// alpha a detonation core can afford - it is normally seen from hundreds of
/// metres away - a death seen from fifty is a heap of flat orange discs. Held
/// low, the same quads sum into a translucent volume with the wreck showing
/// through it.
fn flash_gradient() -> bevy_hanabi::Gradient<Vec4> {
    let mut gradient = bevy_hanabi::Gradient::new();
    gradient.add_key(0.0, Vec4::new(14.0, 12.5, 10.0, 0.55));
    gradient.add_key(0.18, Vec4::new(9.0, 6.0, 2.2, 0.34));
    gradient.add_key(0.50, Vec4::new(3.4, 1.1, 0.20, 0.12));
    gradient.add_key(1.0, Vec4::new(0.5, 0.06, 0.01, 0.0));
    gradient
}

/// The EJECTA's colour: cooler than the flash it left, and it outlives it.
///
/// Cooler on purpose - a hundred streaks at the core's heat wash out the flare
/// they came from - and it holds its alpha longer, because the fragments are
/// what the eye follows once the light has gone. They cool through amber to a
/// dim red and fade rather than cut.
fn ember_gradient() -> bevy_hanabi::Gradient<Vec4> {
    let mut gradient = bevy_hanabi::Gradient::new();
    gradient.add_key(0.0, Vec4::new(6.0, 5.0, 3.4, 0.9));
    gradient.add_key(0.12, Vec4::new(4.2, 2.1, 0.55, 0.8));
    gradient.add_key(0.45, Vec4::new(1.6, 0.42, 0.06, 0.6));
    gradient.add_key(0.80, Vec4::new(0.5, 0.09, 0.02, 0.3));
    gradient.add_key(1.0, Vec4::new(0.10, 0.01, 0.0, 0.0));
    gradient
}

/// A random unit direction, the same three-component draw every burst in the
/// game uses.
fn scatter(writer: &ExprWriter) -> bevy_hanabi::WriterExpr {
    let rand_x = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let rand_y = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let rand_z = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    (writer.lit(Vec3::X) * rand_x + writer.lit(Vec3::Y) * rand_y + writer.lit(Vec3::Z) * rand_z)
        .normalized()
}

/// The core graph: the vaporised mass, camera-facing.
///
/// It grows fast, holds, and thins. The peak is early because a fireball
/// reaches its size while it is still bright and spends the rest of its life
/// fading at roughly that size. What separates a death from a warhead is
/// duration - a magazine and a reactor keep burning after the hit that opened
/// them - so this outlives a detonation core, and nothing else about it is
/// different.
fn build_pyre_core(core: PyreCore, name: &str) -> EffectAsset {
    let spawner = SpawnerSettings::once(core.particles.into()).with_emit_on_start(true);
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        writer
            .lit(core.life.0)
            .uniform(writer.lit(core.life.1))
            .expr(),
    );
    let init_color = SetAttributeModifier::new(Attribute::COLOR, writer.lit(0xFFFFFFFFu32).expr());
    let init_pos = SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());

    // The velocity the wreck carried, written per death. A fireball that does
    // not inherit it hangs where the ship WAS while the pieces fly on out of
    // it, which reads as two unrelated events.
    let base_velocity = writer.add_property("base_velocity", Vec3::ZERO.into());
    let base_velocity = writer.prop(base_velocity);
    let speed = writer.lit(core.drift.0).uniform(writer.lit(core.drift.1));
    let velocity = scatter(&writer) * speed + base_velocity;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    let mut size_gradient = bevy_hanabi::Gradient::new();
    size_gradient.add_key(0.0, Vec3::splat(core.size * 0.28));
    size_gradient.add_key(0.16, Vec3::splat(core.size));
    size_gradient.add_key(0.60, Vec3::splat(core.size * 0.78));
    size_gradient.add_key(1.0, Vec3::ZERO);

    // Round, not rectangular. These are the biggest quads in the frame while
    // they burn, and without the mask a death reads as a cluster of glowing
    // squares whatever the gradient does.
    let mask = soft_dot_modifier(&writer);
    let mut module = writer.finish();
    declare_soft_dot_slot(&mut module);

    EffectAsset::new(core.capacity, spawner, module)
        .with_name(name)
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .init(init_color)
        // Camera-facing, and said in code rather than in a comment: a quad
        // with no orient modifier is expanded along the fixed WORLD axes, so
        // the fireball is drawn edge-on from any camera looking down one.
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient,
            screen_space_size: false,
        })
        .render(mask)
        .render(ColorOverLifetimeModifier {
            gradient: flash_gradient(),
            blend: ColorBlendMode::default(),
            mask: ColorBlendMask::default(),
        })
}

/// The ejecta graph: the fragments, oriented along their own velocity.
///
/// Long, narrow quads become radial incandescent streaks that CONTRACT into
/// points as they cool, which is the same treatment the torpedo blast gives
/// its ejecta and the reason a death does not read as a ball of bubbles. They
/// are ballistic - no drag, because there is nothing to drag against - so the
/// only thing that ends one is its lifetime.
fn build_pyre_ejecta(ejecta: PyreEjecta, name: &str) -> EffectAsset {
    let spawner = SpawnerSettings::once(ejecta.particles.into()).with_emit_on_start(true);
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        writer
            .lit(ejecta.life.0)
            .uniform(writer.lit(ejecta.life.1))
            .expr(),
    );
    let init_color = SetAttributeModifier::new(Attribute::COLOR, writer.lit(0xFFFFFFFFu32).expr());
    let init_pos = SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());

    let base_velocity = writer.add_property("base_velocity", Vec3::ZERO.into());
    let base_velocity = writer.prop(base_velocity);
    let speed = writer
        .lit(ejecta.speed.0)
        .uniform(writer.lit(ejecta.speed.1));
    let velocity = scatter(&writer) * speed + base_velocity;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    // Stretched along X, which the orient modifier puts on the velocity. It
    // reaches its length early and then draws in, so the burst is streaks
    // first and sparks last.
    let streak = |scale: f32| Vec3::new(ejecta.length * scale, ejecta.width, ejecta.width);
    let mut size_gradient = bevy_hanabi::Gradient::new();
    size_gradient.add_key(0.0, streak(0.35));
    size_gradient.add_key(0.10, streak(1.0));
    size_gradient.add_key(0.55, streak(0.55));
    size_gradient.add_key(1.0, Vec3::ZERO);

    // On a velocity-oriented quad the circular mask reads as a tapered streak
    // rather than as a lozenge with corners.
    let mask = soft_dot_modifier(&writer);
    let mut module = writer.finish();
    declare_soft_dot_slot(&mut module);

    EffectAsset::new(ejecta.capacity, spawner, module)
        .with_name(name)
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .init(init_color)
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient,
            screen_space_size: false,
        })
        .render(OrientModifier::new(OrientMode::AlongVelocity))
        .render(mask)
        .render(ColorOverLifetimeModifier {
            gradient: ember_gradient(),
            blend: ColorBlendMode::default(),
            mask: ColorBlendMask::default(),
        })
}

/// Give the frame its fireball allowance back.
fn refill_pyre_budget(mut budget: ResMut<PyreBudget>) {
    budget.0 = 0;
}

/// The two asset stores a fireball needs, or nothing at all.
///
/// Two refusals with one answer, because they have the same consequence. A
/// world with no asset stores has nothing to build a graph in and nothing that
/// could see the result: a headless server, or a test app that added the
/// integrity plugin for its health pipeline alone. A tier with particles off is
/// the spawn-less low-end mode, which is a policy rather than a limitation. An
/// ABSENT budget is a settings-less app, which means full quality. Deaths still
/// happen in all three, they just go unlit.
fn drawable<'w>(
    tier: Option<Res<GraphicsBudget>>,
    effects: Option<ResMut<'w, Assets<EffectAsset>>>,
    images: Option<ResMut<'w, Assets<Image>>>,
) -> Option<(ResMut<'w, Assets<EffectAsset>>, ResMut<'w, Assets<Image>>)> {
    if !tier.as_deref().is_none_or(|tier| tier.particles) {
        return None;
    }
    Some((effects?, images?))
}

/// Mint the graphs, the mask AND the shaders before anything needs them.
///
/// Four `ExprWriter` graphs and the one 128x128 soft-dot mask all four sample -
/// built once here and shared, not per size. Those assets are not the expensive
/// half and never were. `bevy_hanabi` generates a WGSL source from a
/// `CompiledParticleEffect`, and `compile_effects` only visits spawned
/// INSTANCES - adding an [`EffectAsset`] to the store generates nothing.
/// Measured on a hull collapse with `bevy_hanabi=debug`, an asset-only warm-up
/// left five `pyre_section_*` shaders to be minted INSIDE the collapse window,
/// 1.68 ms of main-thread WGSL generation on that frame, and the hulk pair
/// unminted entirely. That frame is the most-watched one in the game and is
/// already doing the most work in it.
///
/// So this spawns one throwaway instance per graph, which is what hanabi's own
/// documentation calls compiling in the background. Each is invisible and free:
/// [`Visibility::Hidden`] so nothing draws, and an [`EffectSpawner`] held
/// inactive so nothing is emitted - the assets are `once` spawners with
/// emit-on-start, so an instance left to its own settings would fire its whole
/// burst on its first tick. They live until [`cool_the_warm_pyres`] takes them
/// away on the next frame's `Update`, which is the earliest point at which
/// `PostUpdate`'s compile and the render world's extract have both had them.
///
/// On entering [`GameStates::Playing`] rather than at startup, for two reasons:
/// the state is never `Playing` on frame one, so the graphics tier a player
/// chose in the menu is settled before this reads it; and a scene load already
/// has a loading screen over it. The sibling
/// `warm_railgun_wake_art` is wired the same way.
fn warm_the_pyres(
    mut commands: Commands,
    effects: Option<ResMut<Assets<EffectAsset>>>,
    images: Option<ResMut<Assets<Image>>>,
    mut pyres: ResMut<PyreEffects>,
    mut soft_dot: ResMut<SoftDot>,
    tier: Option<Res<GraphicsBudget>>,
) {
    let Some((mut effects, mut images)) = drawable(tier, effects, images) else {
        return;
    };
    let dot = soft_dot.handle(&mut images);
    for size in [PyreSize::Section, PyreSize::Hulk] {
        let (pair, _) = pyres.pair(size, &mut effects);
        for handle in [pair.core, pair.ejecta] {
            commands.spawn((
                Name::new("Pyre Warm-Up"),
                PyreWarmMarker,
                ParticleEffect::new(handle),
                EffectMaterial {
                    images: vec![dot.clone()],
                },
                EffectProperties::default(),
                EffectSpawner::default().with_active(false),
                Visibility::Hidden,
            ));
        }
    }
}

/// Take the warm-up's throwaway instances away again.
///
/// In `Update` and not in the same frame's `PostUpdate` or `Last`: hanabi
/// compiles in `PostUpdate` and the render world extracts after the whole main
/// schedule, so an instance spawned from `OnEnter` has to survive its own frame
/// to be both compiled and specialized. [`Ref::is_added`] is what draws that
/// line - on the frame they were spawned these are still new, on the next they
/// are not.
fn cool_the_warm_pyres(mut commands: Commands, warm: Query<(Entity, Ref<PyreWarmMarker>)>) {
    for (entity, marker) in &warm {
        if !marker.is_added() {
            commands.entity(entity).despawn();
        }
    }
}

/// Throw the fireball a death earns.
///
/// Reacts to the destroy marker rather than to a death event of its own, on the
/// same terms as [`explode`](super::explode): the integrity layer decides WHEN
/// something dies, and this decides what that looks like. A mod replacing this
/// observer changes the look and inherits the cap for free.
#[expect(
    clippy::too_many_arguments,
    reason = "one observer assembling a hanabi instance: the graph store, the mask, the frame budget, the tier gate and the queries that place it and say what it is made of"
)]
fn light_the_pyre(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    effects: Option<ResMut<Assets<EffectAsset>>>,
    images: Option<ResMut<Assets<Image>>>,
    mut pyres: ResMut<PyreEffects>,
    mut soft_dot: ResMut<SoftDot>,
    mut budget: ResMut<PyreBudget>,
    tier: Option<Res<GraphicsBudget>>,
    q_dead: Query<(&GlobalTransform, Has<IntegrityRoot>), With<IntegrityDestroyMarker>>,
    q_drift: Query<&avian3d::prelude::LinearVelocity>,
    q_parents: Query<&ChildOf>,
    q_debris: Query<&CarveDebris>,
) {
    let Some((mut effects, mut images)) = drawable(tier, effects, images) else {
        return;
    };

    let entity = add.entity;
    let Ok((frame, root)) = q_dead.get(entity) else {
        // Nothing that carries no transform: a health node hanging off a
        // section, which the section's own fireball already covers.
        return;
    };

    // Rock does not burn. `IntegrityDestroyMarker` is a shared seam - an
    // exhausted asteroid raises it to reuse the destruction cue without opting
    // into the health graph behind it - so the material has the last word on
    // whether a death is a fireball at all. What this throws is a hull's own
    // vaporised mass, and there is none of that in a rock. Asked before the
    // budget is spent: a rock must not cost a section its place in the frame.
    if inherited_material(entity, &q_debris, &q_parents) == CarveDebris::Rock {
        return;
    }

    // The root's fireball is the one the whole death reads as, so it is never
    // the one the cap drops.
    if !root && budget.0 >= PYRE_FRAME_CAP {
        return;
    }
    budget.0 += 1;

    let size = if root {
        PyreSize::Hulk
    } else {
        PyreSize::Section
    };
    let (pair, scale) = pyres.pair(size, &mut effects);
    let drift = inherited_drift(entity, &q_drift, &q_parents);
    let at = frame.translation();
    let dot = soft_dot.handle(&mut images);
    for handle in [pair.core, pair.ejecta] {
        let mut properties = EffectProperties::default();
        properties.set("base_velocity", drift.into());
        commands.spawn((
            Name::new("Pyre Effect"),
            PyreEffectMarker,
            Transform::from_translation(at),
            ParticleEffect::new(handle),
            EffectMaterial {
                images: vec![dot.clone()],
            },
            properties,
            TempEntity(scale.linger),
        ));
    }

    // Asked for, never assumed - the cap may refuse it, and a death that lit
    // nothing is still a death. Amber rather than the core's first white key:
    // the light stands in for the whole burn averaged over its life.
    commands.trigger(LightFlash {
        at,
        color: Color::srgb(1.0, 0.66, 0.32),
        peak_intensity: scale.lumens,
        range: scale.light_range,
        duration: scale.light_secs,
    });
}

/// The velocity the dead body was carrying, from the nearest ancestor that has
/// one.
///
/// A section holds no velocity of its own - it is a child of the ship's rigid
/// body, and avian keeps the velocity there - so this is a walk, not a lookup.
/// Mirrors [`explode`](super::explode)'s inheritance for the same reason.
fn inherited_drift(
    entity: Entity,
    q_drift: &Query<&avian3d::prelude::LinearVelocity>,
    q_parents: &Query<&ChildOf>,
) -> Vec3 {
    let mut current = entity;
    loop {
        if let Ok(drift) = q_drift.get(current) {
            return drift.0;
        }
        let Ok(parent) = q_parents.get(current) else {
            return Vec3::ZERO;
        };
        current = parent.0;
    }
}

/// The fireball half of destruction: what a death LOOKS like at the moment it
/// happens, beside [`explode`](super::explode)'s wreckage.
pub struct PyrePlugin;

impl Plugin for PyrePlugin {
    fn build(&self, app: &mut App) {
        trace!("PyrePlugin: build");

        app.register_type::<PyreEffectMarker>();
        app.init_resource::<PyreEffects>();
        app.init_resource::<PyreBudget>();
        // The mask is shared with the weapon effects, and whichever plugin
        // asks for it first is the one that builds the slot. A ship carrying
        // no armed section still dies, so the pyre cannot rely on a turret
        // having been here.
        app.init_resource::<SoftDot>();
        app.add_systems(OnEnter(GameStates::Playing), warm_the_pyres);
        app.add_systems(First, refill_pyre_budget);
        app.add_systems(Update, cool_the_warm_pyres);
        app.add_observer(light_the_pyre);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::prelude::GraphicsQuality;

    /// The plugin, plus the two asset stores it builds its graphs in. No render
    /// app: an [`EffectAsset`] is data, and what is under test is which
    /// instances a death spawns, not how they draw.
    ///
    /// Built from [`PyrePlugin`] rather than by repeating its wiring, so a
    /// resource the plugin forgets to register fails HERE instead of in the
    /// first crate that adds the plugin for real.
    ///
    /// It is handed to the tests already in `Playing` and one frame past the
    /// transition, which is the state a death happens in: the warm-up runs on
    /// entering it, and its throwaway instances are gone by the time a test
    /// asks what a death spawned.
    fn pyre_app() -> App {
        let mut app = playing_pyre_app();
        app.update();
        app
    }

    /// The same app stopped one frame earlier, on the transition frame itself,
    /// for the tests that are about the warm-up.
    fn playing_pyre_app() -> App {
        playing_pyre_app_at(None)
    }

    /// `tier` is the graphics budget the app runs at. `None` is a
    /// settings-less app, which is the only case the rest of this module's
    /// tests exercise and which means full quality.
    fn playing_pyre_app_at(tier: Option<GraphicsBudget>) -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameStates>();
        app.insert_resource(Assets::<EffectAsset>::default());
        app.insert_resource(Assets::<Image>::default());
        if let Some(tier) = tier {
            app.insert_resource(tier);
        }
        app.add_plugins(PyrePlugin);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app
    }

    /// The graphs and the mask standing in the two stores.
    fn built(app: &App) -> (usize, usize) {
        (
            app.world().resource::<Assets<EffectAsset>>().len(),
            app.world().resource::<Assets<Image>>().len(),
        )
    }

    fn warm_instances(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<PyreWarmMarker>>()
            .iter(app.world())
            .count()
    }

    /// A body the pipeline can kill: it carries the transform the fireball is
    /// placed at.
    fn a_body(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((Transform::default(), GlobalTransform::default()))
            .id()
    }

    fn kill(app: &mut App, entity: Entity) {
        app.world_mut()
            .entity_mut(entity)
            .insert(IntegrityDestroyMarker);
    }

    fn bursts(app: &mut App) -> Vec<Entity> {
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<PyreEffectMarker>>();
        query.iter(app.world()).collect()
    }

    #[test]
    fn a_death_lights_a_core_and_its_ejecta() {
        let mut app = pyre_app();
        let body = a_body(&mut app);
        kill(&mut app, body);
        app.update();

        let lit = bursts(&mut app);
        assert_eq!(lit.len(), 2, "a death is a flash AND the pieces it throws");
        let graphs: Vec<_> = lit
            .iter()
            .map(|&burst| {
                app.world()
                    .get::<ParticleEffect>(burst)
                    .expect("a burst carries its graph")
                    .handle
                    .clone()
            })
            .collect();
        assert_ne!(
            graphs[0], graphs[1],
            "the two halves are different graphs - one billboard cannot also be a streak"
        );
    }

    #[test]
    fn a_hull_and_a_compartment_burn_at_different_scales() {
        let mut app = pyre_app();
        let section = a_body(&mut app);
        kill(&mut app, section);
        let hull = a_body(&mut app);
        app.world_mut().entity_mut(hull).insert(IntegrityRoot);
        kill(&mut app, hull);
        app.update();

        let graphs: std::collections::HashSet<_> = bursts(&mut app)
            .iter()
            .map(|&burst| {
                app.world()
                    .get::<ParticleEffect>(burst)
                    .expect("a burst carries its graph")
                    .handle
                    .clone()
            })
            .collect();
        assert_eq!(
            graphs.len(),
            4,
            "a compartment and a whole hull are two sizes, so four graphs, not two"
        );
    }

    #[test]
    fn the_graphs_are_ready_before_the_first_death_and_no_death_mints_more() {
        let mut app = pyre_app();

        assert_eq!(
            built(&app),
            (4, 1),
            "a core and an ejecta for each of the two sizes, and the one shared mask,              all standing before anything has died"
        );

        for _ in 0..2 {
            let body = a_body(&mut app);
            kill(&mut app, body);
            app.update();
        }
        assert_eq!(
            built(&app),
            (4, 1),
            "the graphs are shared - a death must not mint its own pair"
        );
    }

    /// The half an asset-only warm-up missed. `bevy_hanabi` mints a shader from
    /// a spawned INSTANCE and never from an asset, so a warm-up that only fills
    /// the store leaves the WGSL generation to the collapse frame.
    #[test]
    fn the_warm_up_spawns_an_instance_per_graph_and_takes_them_away_the_next_frame() {
        let mut app = playing_pyre_app();

        assert_eq!(
            warm_instances(&mut app),
            4,
            "an instance per graph is what mints the four shaders",
        );
        assert!(
            bursts(&mut app).is_empty(),
            "a warm instance reached the query a range counts live fireballs with",
        );

        app.update();

        assert_eq!(
            warm_instances(&mut app),
            0,
            "the warm-up's instances outlived the frame they were compiled in",
        );
    }

    /// The warm instances draw nothing and emit nothing. The assets are `once`
    /// spawners with emit-on-start, so an instance left to the asset's own
    /// settings fires its whole burst on its first tick.
    #[test]
    fn a_warm_instance_is_hidden_and_its_spawner_is_held_shut() {
        let mut app = playing_pyre_app();
        let warm: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<PyreWarmMarker>>()
            .iter(app.world())
            .collect();
        assert_eq!(warm.len(), 4, "delivery guard: the warm-up ran");

        for instance in warm {
            let world = app.world();
            assert_eq!(
                world.get::<Visibility>(instance),
                Some(&Visibility::Hidden),
                "a warm instance is drawn",
            );
            assert!(
                !world
                    .get::<EffectSpawner>(instance)
                    .expect("a warm instance states its spawner rather than taking the asset's")
                    .active,
                "a warm instance emits particles",
            );
        }
    }

    /// The spawn-less low tier. Every other test here runs settings-less,
    /// which means FULL quality, so the gate itself went untested - and until
    /// the budget settled before the first frame it was never false in a
    /// shipping app either.
    #[test]
    fn the_low_tier_builds_no_graph_no_mask_and_no_instance() {
        let mut app = playing_pyre_app_at(Some(GraphicsBudget::for_quality(GraphicsQuality::Low)));

        assert_eq!(
            built(&app),
            (0, 0),
            "the spawn-less tier built the graphs and uploaded the mask it will never sample",
        );
        assert_eq!(warm_instances(&mut app), 0, "and warmed them up as well");

        let body = a_body(&mut app);
        kill(&mut app, body);
        app.update();

        assert!(
            bursts(&mut app).is_empty(),
            "the spawn-less tier lit a death",
        );
        assert_eq!(built(&app), (0, 0), "and built the graphs to do it with");
    }

    #[test]
    fn a_node_with_no_transform_lights_nothing() {
        let mut app = pyre_app();
        let node = app.world_mut().spawn_empty().id();
        kill(&mut app, node);
        app.update();

        assert!(
            bursts(&mut app).is_empty(),
            "a health node hanging off a section is covered by the section's own fire"
        );
    }

    #[test]
    fn a_collapse_is_capped_but_the_hull_itself_never_is() {
        let mut app = pyre_app();
        for _ in 0..(PYRE_FRAME_CAP + 4) {
            let section = a_body(&mut app);
            kill(&mut app, section);
        }
        let hull = a_body(&mut app);
        app.world_mut().entity_mut(hull).insert(IntegrityRoot);
        kill(&mut app, hull);
        app.update();

        assert_eq!(
            bursts(&mut app).len() as u32,
            (PYRE_FRAME_CAP + 1) * 2,
            "six compartments walk across the wreck, and the hull's own fire is never the one dropped"
        );
    }

    #[test]
    fn the_allowance_comes_back_the_next_frame() {
        let mut app = pyre_app();
        for _ in 0..(PYRE_FRAME_CAP + 4) {
            let section = a_body(&mut app);
            kill(&mut app, section);
        }
        app.update();
        for _ in 0..(PYRE_FRAME_CAP + 4) {
            let section = a_body(&mut app);
            kill(&mut app, section);
        }
        app.update();

        assert_eq!(
            bursts(&mut app).len() as u32,
            PYRE_FRAME_CAP * 2 * 2,
            "a rake down a hull lights a fire in every frame it condemns cells in"
        );
    }

    #[test]
    fn the_burst_inherits_the_velocity_the_wreck_was_carrying() {
        let mut app = pyre_app();
        let ship = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                avian3d::prelude::LinearVelocity(Vec3::new(0.0, 0.0, -12.0)),
            ))
            .id();
        let section = a_body(&mut app);
        app.world_mut().entity_mut(section).insert(ChildOf(ship));
        kill(&mut app, section);
        app.update();

        for burst in bursts(&mut app) {
            let stored = app
                .world()
                .get::<EffectProperties>(burst)
                .expect("a burst carries its properties")
                .get_stored("base_velocity")
                .expect("and the drift it inherited");
            assert_eq!(
                stored,
                Vec3::new(0.0, 0.0, -12.0).into(),
                "a fireball that stays where the ship WAS reads as a second, unrelated event"
            );
        }
    }

    /// The case every other test here is blind to, because they all hand the
    /// app the two asset stores first: a crate that adds the integrity plugin
    /// for its HEALTH pipeline and never renders anything.
    #[test]
    fn a_death_in_a_world_with_no_asset_stores_is_unlit_and_not_fatal() {
        let mut app = App::new();
        app.add_plugins(PyrePlugin);
        let body = a_body(&mut app);
        kill(&mut app, body);
        app.update();

        assert!(
            bursts(&mut app).is_empty(),
            "there is nothing to build a graph in and nobody to see it"
        );
    }

    /// An exhausted asteroid raises the same destroy marker a section does, to
    /// reuse the cue seam. It must not get the fireball with it.
    #[test]
    fn a_rock_running_out_lights_nothing() {
        let mut app = pyre_app();
        let root = app.world_mut().spawn(CarveDebris::Rock).id();
        let node = a_body(&mut app);
        app.world_mut().entity_mut(node).insert(ChildOf(root));
        kill(&mut app, node);
        app.update();

        assert!(
            bursts(&mut app).is_empty(),
            "a rock has no hull mass to vaporise, so it has no fireball"
        );
    }

    /// The other half of the same rule: anything that does not say what it is
    /// made of is ship, which is what every section in the game relies on.
    #[test]
    fn a_body_that_names_no_material_still_burns() {
        let mut app = pyre_app();
        let body = a_body(&mut app);
        kill(&mut app, body);
        app.update();

        assert_eq!(bursts(&mut app).len(), 2, "a core and its ejecta");
    }
}
