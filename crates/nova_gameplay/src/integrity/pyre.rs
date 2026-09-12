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
//! # Two sizes, and the hull one is the size of its hull
//!
//! A SECTION dying is a compartment going up: a core about the size of the
//! build-grid cell it stood in, over in a third of a second. That is the same
//! event on every ship, because a cell is the same size on every ship. A
//! ship's INTEGRITY ROOT dying is the whole hull letting go, and it is the
//! only death in the game that is allowed to fill the frame - so it is drawn
//! at the size of the hull it came out of, from that root's
//! [`IntegrityEnvelope`] against the gunship [`HULK_PYRE`] was cut on.
//!
//! Each size is its own pair of [`EffectAsset`]s, and the hull scale is a
//! per-instance PROPERTY multiplied into both, because hanabi bakes a gradient
//! into the asset: a baked size curve cannot be multiplied by anything, so
//! every size curve here is an expression over the particle's own age instead.
//!
//! # A collapse is a chain, and a chain is a SAMPLE of the collapse
//!
//! Structural collapse destroys every section a hull has left in ONE frame, so
//! the unbudgeted reading of "one fireball per death" is two thousand deaths
//! born together. The chain across the hull is exactly what a ship blowing up
//! looks like, so it is kept - [`frame_cap`] of it, which grows with the root
//! of the batch - and the root's own fireball is never one of the ones dropped.
//!
//! Which of them burn cannot be answered while they are still arriving: the
//! first six of a carrier collapse are six fires in one shoulder, because the
//! destruction pass walks the section graph. So compartment deaths QUEUE, and
//! [`spend_the_pyre_queue`] picks them apart at the end of the frame.
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
use nova_events::prelude::*;

use super::components::prelude::*;
use crate::{
    integrity::spew::prelude::{inherited_material, CarveDebris},
    lifetime::TempEntity,
    settings::prelude::{GraphicsBudget, SettingsSystems},
    soft_dot::prelude::{declare_soft_dot_slot, soft_dot_modifier, SoftDot},
    transient_light::prelude::LightFlash,
};

/// `PyrePlugin` and the marker its instances carry.
pub mod prelude {
    pub use super::{PyreEffectMarker, PyrePlugin};
}

/// Tags a live death fireball, so a range can count them and say what it was
/// lit at. Both halves of one death carry it.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct PyreEffectMarker {
    /// Whether this is a whole hull letting go rather than one compartment.
    pub hulk: bool,
    /// How much bigger than the hull the graph was cut against the dead body
    /// was: every length in this instance is multiplied by it, and its flash
    /// is brightened by its square. One for a section, whatever its ship.
    pub scale: f32,
}

impl Default for PyreEffectMarker {
    /// A section-sized death at the authored size, which is what a fireball is
    /// before anything says otherwise.
    fn default() -> Self {
        Self {
            hulk: false,
            scale: 1.0,
        }
    }
}

/// Tags a throwaway instance the warm-up spawned to mint a shader.
///
/// Its own marker and not [`PyreEffectMarker`], because a range counting live
/// fireballs must never see one: these draw nothing, emit nothing, and are
/// gone the frame after they are made.
#[derive(Component, Clone, Copy, Debug)]
struct PyreWarmMarker;

/// The name the per-instance spatial scale is written under, in both graphs.
const PYRE_SCALE_PROPERTY: &str = "hull_scale";

/// The hull [`HULK_PYRE`] was cut against: a shipped gunship, 55.2 m of
/// structural arm.
///
/// A hull death is that fireball multiplied by how much bigger the dead hull
/// is, so the gunship is the one hull whose death is exactly as authored. One
/// reference and not a table, because the figures below are a LOOK and a look
/// is tuned once.
const PYRE_REFERENCE_RADIUS: Meters = Meters(55.2);

/// The fewest compartment deaths one frame lights.
///
/// A collapsing hull destroys everything it has left at once, and the read
/// wanted is a chain of blasts walking across the wreck rather than a single
/// puff. Six is that chain on the hull the look was cut on, and it is what
/// every ordinary frame gets: a railgun corridor - a rake that condemns thirty
/// cells over several frames - lights a fire in every one of them.
const PYRE_FRAME_FLOOR: usize = 6;

/// The most, whatever dies.
///
/// This is the GPU bound: every lit death allocates its own pair of
/// per-instance buffers in the frame it is born, and past four dozen the chain
/// stops reading as more fires and starts costing the frame they are supposed
/// to sell.
const PYRE_FRAME_CEILING: usize = 48;

/// The batch [`PYRE_FRAME_FLOOR`] is the right chain for: the 53 sections of
/// the gunship [`HULK_PYRE`] was cut on.
///
/// A whole gunship going up at once lights six. Everything else is that read
/// held at a bigger or smaller size - see [`frame_cap`].
const PYRE_BATCH_REFERENCE: usize = 53;

/// How many of the frame's compartment deaths are lit.
///
/// `6 * sqrt(condemned / 53)`, bounded by [`PYRE_FRAME_FLOOR`] and
/// [`PYRE_FRAME_CEILING`]. The ROOT of the batch and not the batch: a wreck
/// twice the size is read by eye as roughly twice as much fire, not as forty
/// times as much, and the chain is a sample of the collapse rather than a
/// drawing of it. A carrier shedding all 2 081 of its sections lights 38, a
/// 720-cell corridor 23, and anything at or under the gunship's own count the
/// six it always had.
fn frame_cap(condemned: usize) -> usize {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a section count, and the result is a count of fires to light"
    )]
    let scaled = PYRE_FRAME_FLOOR as f32 * (condemned as f32 / PYRE_BATCH_REFERENCE as f32).sqrt();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ceil of a positive float bounded by the clamp below"
    )]
    let cap = scaled.ceil() as usize;
    cap.clamp(PYRE_FRAME_FLOOR, PYRE_FRAME_CEILING)
}

/// One compartment death waiting for the rest of the frame's batch.
///
/// Everything a fireball is built from, taken where the death happened: the
/// body itself is despawned in the same frame, so the request cannot hold an
/// entity.
#[derive(Clone, Copy, Debug)]
struct PyreRequest {
    /// Where it burns, world space.
    at: Vec3,
    /// The velocity the wreck was carrying.
    drift: Vec3,
}

/// The compartment deaths this frame has raised, spent at the end of it.
///
/// A collapse is one frame's worth of deaths, and which six of two thousand to
/// light cannot be answered while they are still arriving - taking the first
/// six draws the fire wherever the destruction pass happened to walk first,
/// which on a hull graph is one corner of the wreck. So the observer QUEUES,
/// and [`spend_the_pyre_queue`] answers it once the batch is whole.
#[derive(Resource, Default, Debug)]
struct PyreQueue(Vec<PyreRequest>);

/// The shared graphs. Two per size: the core and its ejecta.
///
/// Warmed by [`warm_the_pyres`] rather than built by [`FromWorld`], so an app
/// with no asset stores and one running at a graphics tier with particles off
/// still build nothing. The slots stay optional because that is what lets
/// those two apps hold the resource without paying for it, and because the
/// warm-up is a system and not a constructor.
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

/// The whole hull letting go, at the size of the gunship it was cut on - 90 m
/// stem to stern, 9 units - so the core covers THAT wreck without swallowing
/// the frame: a death the camera cannot see THROUGH is a death nobody can
/// read, and the sections thrown out of it are half of what makes it one.
///
/// Every length below is multiplied per death by how much bigger the dead hull
/// is than [`PYRE_REFERENCE_RADIUS`], so the figures here are what a gunship
/// gets and the band each one covers is in [`hulk_scale`].
///
/// The fragments carry it after that, and they are most of the picture: the
/// flash is over in a third of a second, while at up to 8 units per second for
/// 1.7 s these cross about 13 units - 130 m of reach out of the root, still
/// leaving the wreck when the light has gone out, which is the order a vacuum
/// burst happens in.
///
/// The reach is the number that carries it: the fragments must get PAST the
/// silhouette, because a debris field that stopped at the hull's own outline
/// says the ship has merely broken rather than been destroyed. At up to 8
/// units per second for 1.7 s a gunship's cross about 13 units, 130 m, out of
/// a hull 9 units long. A carrier is 37 units stem to stern - `block_warship`
/// is 22 - and gets the same picture only because the scale carries the speeds
/// with it. A flat cut lit the middle of a wreck whose ends the fragments
/// never got near.
///
/// Those spans are outer FACE to outer face, so each includes the overhang a
/// multi-cell drive has past its own centre cell - half a unit on the gunship
/// and the warship, a whole one on the carrier. A centre-to-centre span reads
/// each of them short.
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
    ///
    /// The graphs alone. [`PyreSize::scale`] is the one place a caller asks
    /// what a size is authored at, so adding a size cannot leave two answers
    /// to disagree.
    fn pair(&mut self, size: PyreSize, effects: &mut Assets<EffectAsset>) -> PyrePair {
        let (scale, name) = (size.scale(), size.name());
        let slot = match size {
            PyreSize::Section => &mut self.section,
            PyreSize::Hulk => &mut self.hulk,
        };
        slot.get_or_insert_with(|| PyrePair {
            core: effects.add(build_pyre_core(scale.core, &format!("pyre_{name}_core"))),
            ejecta: effects.add(build_pyre_ejecta(
                scale.ejecta,
                &format!("pyre_{name}_ejecta"),
            )),
        })
        .clone()
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

/// One piecewise-linear curve over a particle's own normalised age, built as
/// an expression rather than baked into the asset.
///
/// The same keys a [`bevy_hanabi::Gradient`] would hold and the same straight
/// lines between them, written in code because a gradient cannot be multiplied
/// by a per-instance property and a fireball has to be the size of the hull
/// that threw it. `keys` are `(age, value)` in ascending age.
fn key_curve(writer: &ExprWriter, age: &WriterExpr, keys: &[(f32, f32)]) -> WriterExpr {
    let mut value = writer.lit(keys[0].1);
    for pair in keys.windows(2) {
        let ((from, was), (to, becomes)) = (pair[0], pair[1]);
        let ramp = ((age.clone() - writer.lit(from)) * writer.lit(1.0 / (to - from))).saturate();
        value = value + ramp * writer.lit(becomes - was);
    }
    value
}

/// A particle's age over its lifetime, the input every curve here is read at.
fn normalised_age(writer: &ExprWriter) -> WriterExpr {
    writer.attr(Attribute::AGE) / writer.attr(Attribute::LIFETIME)
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
    // How much bigger than the hull this graph was cut against the dead one
    // was, written per death. Every LENGTH here is multiplied by it and no
    // duration is, so a capital's death is the same event at the size of a
    // capital.
    let hull_scale = writer.add_property(PYRE_SCALE_PROPERTY, 1.0f32.into());
    let speed = writer.lit(core.drift.0).uniform(writer.lit(core.drift.1));
    let velocity = scatter(&writer) * speed * writer.prop(hull_scale) + base_velocity;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    let age = normalised_age(&writer);
    let size = key_curve(
        &writer,
        &age,
        &[
            (0.0, core.size * 0.28),
            (0.16, core.size),
            (0.60, core.size * 0.78),
            (1.0, 0.0),
        ],
    ) * writer.prop(hull_scale);
    let update_size = SetAttributeModifier::new(Attribute::SIZE, size.expr());

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
        .update(update_size)
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
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
    let hull_scale = writer.add_property(PYRE_SCALE_PROPERTY, 1.0f32.into());
    let speed = writer
        .lit(ejecta.speed.0)
        .uniform(writer.lit(ejecta.speed.1));
    let velocity = scatter(&writer) * speed * writer.prop(hull_scale) + base_velocity;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    // Stretched along X, which the orient modifier puts on the velocity. It
    // reaches its length early and then draws in, so the burst is streaks
    // first and sparks last. The width holds while the length moves, until
    // both go to nothing at the end of the streak's life.
    let age = normalised_age(&writer);
    let length = key_curve(
        &writer,
        &age,
        &[
            (0.0, ejecta.length * 0.35),
            (0.10, ejecta.length),
            (0.55, ejecta.length * 0.55),
            (1.0, 0.0),
        ],
    );
    let width = key_curve(
        &writer,
        &age,
        &[(0.0, ejecta.width), (0.55, ejecta.width), (1.0, 0.0)],
    );
    let streak = (length * writer.prop(hull_scale)).vec3(
        width.clone() * writer.prop(hull_scale),
        width * writer.prop(hull_scale),
    );
    let update_size = SetAttributeModifier::new(Attribute::SIZE3, streak.expr());

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
        .update(update_size)
        .render(OrientModifier::new(OrientMode::AlongVelocity))
        .render(mask)
        .render(ColorOverLifetimeModifier {
            gradient: ember_gradient(),
            blend: ColorBlendMode::default(),
            mask: ColorBlendMask::default(),
        })
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
/// In `Update`, on the first frame that has all three of a cold store
/// ([`pyres_are_cold`]), a tier that draws particles at all
/// ([`the_tier_draws_particles`]) and a camera to render from
/// ([`a_view_exists`]). The settings have settled long before that - they are
/// applied in `PostStartup` and again on change, both inside
/// [`SettingsSystems`] - so this never builds the graphs a spawn-less run
/// exists to skip, and a tier RAISED from the pause overlay re-opens the gate
/// without re-entering a state, which is how a budget change is reached by no
/// `OnEnter`.
///
/// As early as that, and not on entering `Playing`: the main-menu backdrop is
/// a scenario whose whole loop is a torpedo erasing a ship, so an earlier cut
/// that warmed on `OnEnter(Playing)` left the first deaths a player ever sees
/// to mint their own shaders. That backdrop raises its camera frames before
/// its first death, so waiting for a view still wins the race it has to win.
///
/// The view is not a preference, and this is why. `bevy_hanabi` gives every
/// live instance a row in one `BufferTable` of draw arguments, and reconciles
/// that table in `prepare_gpu_resources`, which returns EARLY while no view
/// exists. Four rows taken and given back with no view in between leave the
/// table holding four pending row writes against a row count that has fallen
/// back to three, and the first frame that does have a view then allocates a
/// 60-byte buffer and writes 80 bytes into it: `slice offset 0 size 80 is out
/// of range for buffer of size 60`, a panic in the render schedule at boot.
/// `PostStartup` is before any scenario has spawned a camera, so warming there
/// is exactly that shape - which is why this waits for a view instead.
///
/// A hidden instance warms the WGSL and the two COMPUTE pipelines, and not the
/// render one. `compile_effects` generates the source for hidden instances on
/// purpose - that is the background compile hanabi documents - and neither
/// `allocate_effects` nor `prepare_init_update_pipelines` has a visibility
/// gate, so the init and update pipelines are specialized from these. The
/// render pipeline is specialized only in `emit_sorted_draw`, from a view's
/// `RenderVisibleEntities`, which a hidden entity is never in; hanabi in fact
/// drops it a stage earlier, where `prepare_batch_inputs` skips an invisible
/// effect whose asset is `SimulationCondition::WhenVisible` (the default, and
/// what these assets take). So the first death of a run still creates one
/// render pipeline, and `nova_core` asks for `synchronous_pipeline_compilation`
/// on every backend, so that creation is paid on the render thread rather than
/// on a task. Warming it would take a VISIBLE instance, which a warm-up is
/// not: these are hidden exactly so that nothing draws them.
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
        let pair = pyres.pair(size, &mut effects);
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

/// Whether the graphs are still unbuilt, which is the only state a re-warm has
/// anything to do in.
///
/// Once both pairs stand the shaders behind them are minted, and running the
/// warm-up again would spawn four instances to build nothing. A tier LOWERED
/// mid-run therefore keeps the graphs it already has: they are four assets and
/// a mask, and dropping them while a fireball is still drawing from one is a
/// bigger question than the memory is worth.
fn pyres_are_cold(pyres: Res<PyreEffects>) -> bool {
    pyres.section.is_none() || pyres.hulk.is_none()
}

/// Whether the budget in force draws particles at all.
///
/// [`drawable`] asks the same question of the same resource and is what
/// actually refuses the work; this is here so the question is asked by a run
/// condition too. A spawn-less run leaves [`pyres_are_cold`] true for its whole
/// length, so without this the warm-up would be woken on every frame of it to
/// decide again that it has nothing to do. A settings-less app is full quality.
fn the_tier_draws_particles(tier: Option<Res<GraphicsBudget>>) -> bool {
    tier.as_deref().is_none_or(|tier| tier.particles)
}

/// Whether there is a camera for the render world to build a view from.
///
/// [`warm_the_pyres`] states what a view has to do with a warm-up that draws
/// nothing: hanabi only reconciles its table of draw arguments on a frame that
/// has one, and instances that appear and vanish entirely between such frames
/// leave it inconsistent enough to panic. An inactive camera produces no view,
/// so it does not count.
fn a_view_exists(cameras: Query<&Camera>) -> bool {
    cameras.iter().any(|camera| camera.is_active)
}

/// Take the warm-up's throwaway instances away again.
///
/// In `Update` and not in the same frame's `PostUpdate` or `Last`: hanabi
/// compiles in `PostUpdate` and the render world extracts after the whole main
/// schedule, so a warm instance has to survive the frame it was spawned in to
/// be compiled and to reach the render world at all - which is what mints its
/// WGSL and specializes its two compute pipelines. [`Ref::is_added`] is what
/// draws that line - on the frame they were spawned these are still new, on
/// the next they are not.
///
/// That line only falls where it says after [`warm_the_pyres`], which spawns in
/// the same schedule: the ordering is what puts a sync point between the two,
/// so the instances are here to be judged new on their own frame rather than
/// first seen, and skipped, on the frame after.
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
    mut queue: ResMut<PyreQueue>,
    tier: Option<Res<GraphicsBudget>>,
    q_dead: Query<
        (
            &GlobalTransform,
            Has<IntegrityRoot>,
            Option<&IntegrityEnvelope>,
        ),
        With<IntegrityDestroyMarker>,
    >,
    q_drift: Query<&avian3d::prelude::LinearVelocity>,
    q_parents: Query<&ChildOf>,
    q_debris: Query<&CarveDebris>,
) {
    let Some((mut effects, mut images)) = drawable(tier, effects, images) else {
        return;
    };

    let entity = add.entity;
    let Ok((frame, root, envelope)) = q_dead.get(entity) else {
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

    let request = PyreRequest {
        at: frame.translation(),
        drift: inherited_drift(entity, &q_drift, &q_parents),
    };
    if !root {
        // One of possibly two thousand. Which of them burn is decided at the
        // end of the frame, when the batch is whole.
        queue.0.push(request);
        return;
    }

    // The hull's own fireball is the one the whole death reads as, so it is
    // never queued and never dropped - and it is the size of THAT hull, where
    // a compartment is the size of the cell it stood in on any ship.
    let hull_scale = hulk_scale(envelope.map(|envelope| **envelope));
    let pair = pyres.pair(PyreSize::Hulk, &mut effects);
    let dot = soft_dot.handle(&mut images);
    burn(
        &mut commands,
        &pair,
        &dot,
        request,
        PyreSize::Hulk,
        hull_scale,
    );
}

/// Light the compartment deaths this frame earned, spread over the wreck.
///
/// In [`Last`], because that is where the frame's destruction batch is whole:
/// a marker raised anywhere from [`First`] to here is in it.
fn spend_the_pyre_queue(
    mut commands: Commands,
    effects: Option<ResMut<Assets<EffectAsset>>>,
    images: Option<ResMut<Assets<Image>>>,
    mut pyres: ResMut<PyreEffects>,
    mut soft_dot: ResMut<SoftDot>,
    tier: Option<Res<GraphicsBudget>>,
    mut queue: ResMut<PyreQueue>,
) {
    let condemned = std::mem::take(&mut queue.0);
    if condemned.is_empty() {
        return;
    }
    let Some((mut effects, mut images)) = drawable(tier, effects, images) else {
        return;
    };
    let pair = pyres.pair(PyreSize::Section, &mut effects);
    let dot = soft_dot.handle(&mut images);
    for lit in spread(&condemned, frame_cap(condemned.len())) {
        burn(
            &mut commands,
            &pair,
            &dot,
            condemned[lit],
            PyreSize::Section,
            1.0,
        );
    }
}

/// Which of the frame's deaths to light, as indices into `condemned`.
///
/// Farthest-point sampling: the first is the death furthest from the batch's
/// own centre, and every one after it is whichever is furthest from all of
/// those already taken. That is what makes a chain read as a wreck coming
/// apart - six fires in one shoulder of a carrier read as one hit, and the
/// deaths arrive in graph order, which IS one corner first.
///
/// Deterministic and allocation-bounded: ties go to the earlier death, and the
/// walk is `keep` passes over a distance table rather than a sort of the
/// batch.
fn spread(condemned: &[PyreRequest], keep: usize) -> Vec<usize> {
    if condemned.len() <= keep {
        return (0..condemned.len()).collect();
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a count of deaths in one frame, dividing their summed positions"
    )]
    let middle = condemned.iter().map(|death| death.at).sum::<Vec3>() / condemned.len() as f32;
    let furthest = |from: &[f32]| {
        from.iter()
            .enumerate()
            .fold((0usize, f32::NEG_INFINITY), |best, (index, &distance)| {
                if distance > best.1 {
                    (index, distance)
                } else {
                    best
                }
            })
            .0
    };

    let mut reach: Vec<f32> = condemned
        .iter()
        .map(|death| death.at.distance_squared(middle))
        .collect();
    let mut lit = Vec::with_capacity(keep);
    for _ in 0..keep {
        let next = furthest(&reach);
        lit.push(next);
        let taken = condemned[next].at;
        reach[next] = f32::NEG_INFINITY;
        for (index, left) in reach.iter_mut().enumerate() {
            if *left > f32::NEG_INFINITY {
                *left = left.min(condemned[index].at.distance_squared(taken));
            }
        }
    }
    lit
}

/// Spawn one death: the two instances, and the light it throws.
fn burn(
    commands: &mut Commands,
    pair: &PyrePair,
    dot: &Handle<Image>,
    request: PyreRequest,
    size: PyreSize,
    hull_scale: f32,
) {
    let scale = size.scale();
    for handle in [pair.core.clone(), pair.ejecta.clone()] {
        let mut properties = EffectProperties::default();
        properties.set("base_velocity", request.drift.into());
        properties.set(PYRE_SCALE_PROPERTY, hull_scale.into());
        commands.spawn((
            Name::new("Pyre Effect"),
            PyreEffectMarker {
                hulk: size == PyreSize::Hulk,
                scale: hull_scale,
            },
            Transform::from_translation(request.at),
            ParticleEffect::new(handle),
            EffectMaterial {
                images: vec![dot.clone()],
            },
            properties,
            TempEntity(scale.linger),
        ));
    }

    // Amber rather than the core's first white key: the light stands in for
    // the whole burn averaged over its life. It grows with the fireball, and
    // its lumens with the SQUARE of it, because lumens stand in for a burning
    // surface and a surface goes up with the square of what it wraps.
    commands.trigger(LightFlash {
        at: request.at,
        color: Color::srgb(1.0, 0.66, 0.32),
        peak_intensity: scale.lumens * hull_scale * hull_scale,
        range: scale.light_range * hull_scale,
        duration: scale.light_secs,
    });
}

/// How much bigger than the reference hull a dying body is.
///
/// `envelope` is the body's live [`IntegrityEnvelope`], world units. A body
/// that publishes none has never said how big it is, and gets the reference
/// death rather than no death: the figure is a multiplier on a LOOK, so the
/// safe answer is the one the look was authored at.
///
/// Unclamped in both directions. The shipped hulls run from `block_skiff` at
/// 48.3 m of containment radius, 0.88, to `block_carrier` at 194.3 m, 3.52 -
/// and a hull outside that band is a hull this look was never cut for, which
/// is a reason to see it at its own size rather than at a bound.
fn hulk_scale(envelope: Option<f32>) -> f32 {
    match envelope {
        Some(envelope) if envelope > 0.0 => envelope / PYRE_REFERENCE_RADIUS.to_engine(),
        _ => 1.0,
    }
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
        app.init_resource::<PyreQueue>();
        // The mask is shared with the weapon effects, and whichever plugin
        // asks for it first is the one that builds the slot. A ship carrying
        // no armed section still dies, so the pyre cannot rely on a turret
        // having been here.
        app.init_resource::<SoftDot>();
        app.add_systems(
            Update,
            warm_the_pyres
                .after(SettingsSystems)
                .run_if(pyres_are_cold)
                .run_if(the_tier_draws_particles)
                .run_if(a_view_exists),
        );
        app.add_systems(Last, spend_the_pyre_queue);
        app.add_systems(Update, cool_the_warm_pyres.after(warm_the_pyres));
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
    /// It is handed to the tests one frame past the warm-up, which is where a
    /// death happens: the graphs stand, and the warm-up's throwaway instances
    /// are gone by the time a test asks what a death spawned.
    fn pyre_app() -> App {
        let mut app = warm_pyre_app();
        app.update();
        app
    }

    /// The same app stopped one frame earlier, on the warm-up frame itself, for
    /// the tests that are about the warm-up.
    fn warm_pyre_app() -> App {
        pyre_app_at(None)
    }

    /// `tier` is the graphics budget the app runs at. `None` is a
    /// settings-less app, which is the only case the rest of this module's
    /// tests exercise and which means full quality.
    ///
    /// The camera is not decoration: the warm-up waits for one, so an app
    /// without it never warms at all.
    fn pyre_app_at(tier: Option<GraphicsBudget>) -> App {
        let mut app = viewless_pyre_app_at(tier);
        app.world_mut().spawn(Camera::default());
        app.update();
        app
    }

    /// The same app with nothing to render from, for the tests that are about
    /// the wait itself.
    fn viewless_pyre_app_at(tier: Option<GraphicsBudget>) -> App {
        let mut app = App::new();
        app.insert_resource(Assets::<EffectAsset>::default());
        app.insert_resource(Assets::<Image>::default());
        if let Some(tier) = tier {
            app.insert_resource(tier);
        }
        app.add_plugins(PyrePlugin);
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
        a_body_at(app, Vec3::ZERO)
    }

    /// The same, somewhere in particular. The global transform is written
    /// rather than propagated: these apps run no transform pass.
    fn a_body_at(app: &mut App, at: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                Transform::from_translation(at),
                GlobalTransform::from_translation(at),
            ))
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

    /// What every live fireball says it was lit at.
    fn lit_at(app: &mut App) -> Vec<PyreEffectMarker> {
        bursts(app)
            .iter()
            .map(|&burst| {
                *app.world()
                    .get::<PyreEffectMarker>(burst)
                    .expect("a burst carries its marker")
            })
            .collect()
    }

    /// The flashes the deaths asked for. The transient-light module is not in
    /// this app, so the request is caught here instead of looked for as a
    /// light.
    #[derive(Resource, Default)]
    struct Flashes(Vec<LightFlash>);

    /// [`pyre_app`] with the flash requests kept, which only the tests about
    /// the light need.
    fn flash_watching_pyre_app() -> App {
        let mut app = pyre_app();
        app.init_resource::<Flashes>();
        app.add_observer(|flash: On<LightFlash>, mut flashes: ResMut<Flashes>| {
            flashes.0.push(*flash);
        });
        app
    }

    /// A hull of `envelope` world units that is about to let go.
    fn a_hull_reaching(app: &mut App, envelope: f32) -> Entity {
        let hull = a_body(app);
        app.world_mut()
            .entity_mut(hull)
            .insert((IntegrityRoot, IntegrityEnvelope(envelope)));
        hull
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
    fn a_hull_burns_at_the_size_of_the_hull() {
        let mut app = pyre_app();
        let hull = a_hull_reaching(&mut app, 2.0 * PYRE_REFERENCE_RADIUS.to_engine());
        kill(&mut app, hull);
        app.update();

        let lit = lit_at(&mut app);
        assert_eq!(lit.len(), 2, "a death is a flash AND the pieces it throws");
        for burst in lit {
            assert!(burst.hulk, "a root letting go is the whole hull");
            assert!(
                (burst.scale - 2.0).abs() < 1.0e-5,
                "a hull twice the reference burns twice the size, got {}",
                burst.scale
            );
        }
    }

    #[test]
    fn a_compartment_is_the_same_size_whatever_ship_it_is_part_of() {
        let mut app = pyre_app();
        let section = a_body(&mut app);
        app.world_mut()
            .entity_mut(section)
            .insert(IntegrityEnvelope(20.0 * PYRE_REFERENCE_RADIUS.to_engine()));
        kill(&mut app, section);
        app.update();

        for burst in lit_at(&mut app) {
            assert!(!burst.hulk, "a node that is not the root is a compartment");
            assert!(
                (burst.scale - 1.0).abs() < 1.0e-5,
                "a build-grid cell is one cell on a carrier too, got {}",
                burst.scale
            );
        }
    }

    #[test]
    fn a_hull_that_says_no_size_burns_as_it_was_authored() {
        let mut app = pyre_app();
        let hull = a_body(&mut app);
        app.world_mut().entity_mut(hull).insert(IntegrityRoot);
        kill(&mut app, hull);
        app.update();

        for burst in lit_at(&mut app) {
            assert!(
                (burst.scale - 1.0).abs() < 1.0e-5,
                "an unmeasured hull gets the authored death, not no death, got {}",
                burst.scale
            );
        }
    }

    #[test]
    fn a_bigger_hull_lights_further_and_brighter_by_the_square() {
        let mut app = flash_watching_pyre_app();
        let hull = a_hull_reaching(&mut app, 3.0 * PYRE_REFERENCE_RADIUS.to_engine());
        kill(&mut app, hull);
        app.update();

        let flashes = &app.world().resource::<Flashes>().0;
        assert_eq!(flashes.len(), 1, "one death, one flash");
        let flash = flashes[0];
        assert!(
            (flash.range - 3.0 * HULK_PYRE.light_range).abs() < 1.0e-3,
            "the flash reaches as far as the fireball is wide, got {}",
            flash.range
        );
        assert!(
            (flash.peak_intensity - 9.0 * HULK_PYRE.lumens).abs() < 1.0,
            "lumens go up with the burning SURFACE, so with the square, got {}",
            flash.peak_intensity
        );
        assert!(
            (flash.duration - HULK_PYRE.light_secs).abs() < 1.0e-5,
            "a bigger hull does not burn for longer, got {}",
            flash.duration
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
        let mut app = warm_pyre_app();

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

    /// Every warm instance takes a row in the one table of draw arguments
    /// `bevy_hanabi` keeps, and hanabi reconciles that table only on a frame
    /// that has a view. Instances that come and go entirely between such frames
    /// leave it holding more pending row writes than the buffer it then
    /// allocates has room for, which is a panic in the render schedule on the
    /// first frame a camera appears. So the warm-up waits for one.
    #[test]
    fn the_warm_up_waits_for_a_camera_and_runs_on_the_frame_one_arrives() {
        let mut app = viewless_pyre_app_at(None);
        app.update();
        app.world_mut().spawn(Camera {
            is_active: false,
            ..default()
        });
        app.update();

        assert_eq!(
            warm_instances(&mut app),
            0,
            "a camera that renders nothing was taken for a view",
        );
        assert_eq!(built(&app), (0, 0), "and the graphs were minted against it");

        app.world_mut().spawn(Camera::default());
        app.update();

        assert_eq!(
            warm_instances(&mut app),
            4,
            "the frame a camera arrived left the pyres cold",
        );
        assert_eq!(
            built(&app),
            (4, 1),
            "a core and an ejecta for each of the two sizes, and the one shared mask",
        );
    }

    /// The warm instances draw nothing and emit nothing. The assets are `once`
    /// spawners with emit-on-start, so an instance left to the asset's own
    /// settings fires its whole burst on its first tick.
    #[test]
    fn a_warm_instance_is_hidden_and_its_spawner_is_held_shut() {
        let mut app = warm_pyre_app();
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
        let mut app = pyre_app_at(Some(GraphicsBudget::for_quality(GraphicsQuality::Low)));

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
        for _ in 0..(PYRE_FRAME_FLOOR + 4) {
            let section = a_body(&mut app);
            kill(&mut app, section);
        }
        let hull = a_body(&mut app);
        app.world_mut().entity_mut(hull).insert(IntegrityRoot);
        kill(&mut app, hull);
        app.update();

        assert_eq!(
            bursts(&mut app).len(),
            (PYRE_FRAME_FLOOR + 1) * 2,
            "six compartments walk across the wreck, and the hull's own fire is never the one dropped"
        );
    }

    #[test]
    fn the_allowance_comes_back_the_next_frame() {
        let mut app = pyre_app();
        for _ in 0..(PYRE_FRAME_FLOOR + 4) {
            let section = a_body(&mut app);
            kill(&mut app, section);
        }
        app.update();
        for _ in 0..(PYRE_FRAME_FLOOR + 4) {
            let section = a_body(&mut app);
            kill(&mut app, section);
        }
        app.update();

        assert_eq!(
            bursts(&mut app).len(),
            PYRE_FRAME_FLOOR * 2 * 2,
            "a rake down a hull lights a fire in every frame it condemns cells in"
        );
    }

    #[test]
    fn a_bigger_collapse_lights_more_fires_by_the_root_of_it() {
        assert_eq!(
            frame_cap(0),
            PYRE_FRAME_FLOOR,
            "a frame that condemned nothing still answers with the floor",
        );
        assert_eq!(
            frame_cap(PYRE_BATCH_REFERENCE),
            PYRE_FRAME_FLOOR,
            "the hull the chain was tuned on lights the chain it was tuned at",
        );
        assert_eq!(frame_cap(720), 23, "a 720-cell corridor");
        assert_eq!(frame_cap(2_081), 38, "a whole carrier letting go");
        assert_eq!(
            frame_cap(1_000_000),
            PYRE_FRAME_CEILING,
            "the GPU bound is a bound, whatever died",
        );
    }

    #[test]
    fn a_compartment_waits_for_the_batch_and_a_hull_never_does() {
        let mut app = pyre_app();
        let section = a_body(&mut app);
        kill(&mut app, section);
        let hull = a_body(&mut app);
        app.world_mut().entity_mut(hull).insert(IntegrityRoot);
        kill(&mut app, hull);

        assert_eq!(
            lit_at(&mut app).len(),
            2,
            "the hull's own death is the one the whole thing reads as, so it never waits",
        );
        app.update();
        assert_eq!(
            lit_at(&mut app).len(),
            4,
            "and the compartment burns once the frame's batch is whole",
        );
    }

    #[test]
    fn the_chain_walks_the_whole_wreck_rather_than_the_corner_it_arrived_in() {
        const CELLS: usize = 60;
        let mut app = pyre_app();
        // Condemned in graph order along one line, which is the order a
        // collapse really raises them in.
        for cell in 0..CELLS {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a cell index used as a position in a 60-cell fixture"
            )]
            let section = a_body_at(&mut app, Vec3::X * cell as f32);
            kill(&mut app, section);
        }
        app.update();

        let mut lit: Vec<f32> = bursts(&mut app)
            .iter()
            .map(|&burst| {
                app.world()
                    .get::<Transform>(burst)
                    .expect("a burst carries its transform")
                    .translation
                    .x
            })
            .collect();
        lit.sort_by(f32::total_cmp);
        lit.dedup();
        assert_eq!(
            lit.len(),
            frame_cap(CELLS),
            "a fire per lit cell, each of them drawn once as a core and once as ejecta",
        );
        #[expect(
            clippy::cast_precision_loss,
            reason = "a 60-cell fixture measured against its own length"
        )]
        let span = (CELLS - 1) as f32;
        assert!(
            lit[0] <= 1.0 && lit[lit.len() - 1] >= span - 1.0,
            "the chain stopped short of the ends of the wreck: {lit:?}",
        );
        let closest = lit
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .fold(f32::INFINITY, f32::min);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count of fires in a 60-cell fixture"
        )]
        let crowded = span / (frame_cap(CELLS) * 3) as f32;
        assert!(
            closest > crowded,
            "two of the chain burned on top of each other, {closest} apart: {lit:?}",
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

    /// The gap an `OnEnter` warm-up left. A player who starts on the
    /// spawn-less tier and raises it from the PAUSE overlay never re-enters a
    /// state, so the budget itself has to be what warms the graphs - otherwise
    /// the next death pays the whole cold path.
    #[test]
    fn a_tier_raised_mid_run_warms_the_graphs_the_low_tier_skipped() {
        let mut app = pyre_app_at(Some(GraphicsBudget::for_quality(GraphicsQuality::Low)));
        app.update();
        assert_eq!(
            built(&app),
            (0, 0),
            "delivery guard: the spawn-less tier built something",
        );

        *app.world_mut().resource_mut::<GraphicsBudget>() =
            GraphicsBudget::for_quality(GraphicsQuality::High);
        app.update();

        assert_eq!(
            built(&app),
            (4, 1),
            "the raised tier left the four graphs and the mask cold",
        );
        assert_eq!(
            warm_instances(&mut app),
            4,
            "and left the shaders behind them to the first death",
        );
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
