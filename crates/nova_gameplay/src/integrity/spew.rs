//! The chips an IMPACT knocked off a body, seen leaving.
//!
//! Keyed on the WEAPON CLASS that paid for the carve, never on how big the hit
//! was: a round striking metal chips it, and that is a property of the round.
//! Change this module when a damage type needs debris of its own.
//!
//! # Why a bullet chips and a blast does not
//!
//! A crater is a hole where solid was, and solid does not vanish. For a BULLET
//! nothing else says so: the geometry changes between one frame and the next,
//! which reads as a rendering glitch rather than as a hit, and the chips are
//! what make the change legible - something came off, and it went that way.
//!
//! A warhead is not short of that cue. Its own fireball covers the frames in
//! which the geometry changes, and the crater it opens is permanent evidence
//! afterwards, so grey chips on top of the fire add nothing and read as litter.
//! Nor does a big hit become feedback-less: a warhead that SEVERS a body throws
//! real severed geometry, meshed off the carve field in the body's own
//! material, which is a different effect with a different meaning
//! ([`chunk`](super::chunk)).
//!
//! Engine units throughout: chip sizes, crater radii and throw speeds are
//! world units (one is 10 m) and world units per second, because they are
//! measured against carve-field geometry.
//!
//! # Dust, and only dust
//!
//! What this throws is decoration and nothing else. The material a hemisphere
//! of carving removes is not waiting in the hole as a lump to be pushed out -
//! it is pulverised across the crater floor - so a body invented to stand for
//! it is decoration the solver then carries for as long as it lives.
//!
//! Real geometry leaves a body only where a carve actually SEVERED it. Then the
//! piece is a thing that existed, with the mass and the tumble it carried out.
//! That is the asteroid's own carve path, and it is decided there because only
//! the body being cut knows whether anything came away. A deep hole and a
//! severing cut look identical from here.
//!
//! A carve announces what it took and nothing about how it should look, so a
//! mod that wants a puff, sparks, or no debris at all replaces this observer
//! rather than patching the carve.
//!
//! # A chip is a particle, not an entity
//!
//! Chips are `bevy_hanabi` particles, and the game is the same without them.
//! They were once kinematic bodies - a cube mesh, a velocity and a 2.5 s timer
//! each - and a busy 4v4 held eight thousand of them at a time, nine in every
//! ten rigid bodies in the world. Every one was a solver body avian integrated
//! per substep, a candidate every ship's sensor sweep walked every frame, a
//! mesh instance the renderer extracted and specialised, and a despawn. That
//! was most of the frame the wider firing cone cost.
//!
//! Now a carve AIMS an emitter and the GPU does the rest: the cone, the speed,
//! the tumble, the cooling and the end of life are all in the effect graph,
//! and no CPU-side entity stands for a chip. Three things are traded for that.
//! Particles are unlit, so the cold colours are authored at the value a lit
//! plate reads at rather than as an albedo. The spray is GPU random rather
//! than hashed off the crater, so two runs of one fight throw different chips.
//! And a range counts what was THROWN ([`CarveShardTally`]) rather than what is
//! in flight, because the flight is on the GPU.
//!
//! # What one FRAME may throw
//!
//! One carve's worth of chips is never the problem: seven chips off a crater is
//! what a hit looks like. A capital hull opening is a different event - a siege
//! lance rakes a corridor and hundreds of craters announce themselves in ONE
//! command flush, each asking for its clamped maximum.
//!
//! An emitter is aimed at ONE crater a frame, so the ceiling is the pool: a
//! material has at most [`SHARD_EMITTERS`] emitters, grown on demand and kept,
//! and the craters that arrive after every one of them has fired this frame go
//! unchipped. That is the right thing to drop: a frame over the pool is one in
//! which dozens of craters opened together, it is already throwing real severed
//! geometry, and nobody can count chips in it.
//!
//! # The body says what it is made of
//!
//! A rock and a hull announce a carve identically, so for as long as one look
//! was shared an asteroid threw gunmetal chips. [`CarveDebris`] sits on the
//! BODY and this observer reads it, which is the only arrangement that does not
//! force `integrity` to enumerate the kinds of body the layers above it will
//! invent. A body that says nothing is plate, so every ship stayed correct
//! without being touched.
//!
//! Read through [`inherited_material`], never off the spewing entity alone: a
//! carve is announced against the mesh NODE that carries the damage marks,
//! while an asteroid declares its material on the root above it.
//!
//! Metal is also HOT. A chip is cut, not picked up, so it leaves near-white and
//! cools to gunmetal in under a second - which is the difference between debris
//! and litter, and what the cold grey cube got wrong.

use std::f32::consts::TAU;

use bevy::{platform::collections::HashMap, prelude::*};
use bevy_hanabi::prelude::*;

use super::carve::prelude::CarveSpew;
use crate::{
    damage::prelude::DamageType,
    settings::prelude::{GraphicsBudget, SettingsSystems},
};

/// `CarveDebris`, `CarveShardEmitterMarker`, `CarveShardTally`,
/// `CarveSpewPlugin` and the walk that reads a body's material.
pub mod prelude {
    pub use super::{
        inherited_material, CarveDebris, CarveShardEmitterMarker, CarveShardTally, CarveSpewPlugin,
    };
}

/// What a body is MADE OF, read off the body a carve took material from.
///
/// A ship and an asteroid announce a carve identically - the same `CarveSpew`,
/// with the same fields - so before this existed a rock threw the hull's
/// gunmetal chips. The split cannot live in the event, because a carve says
/// what it took and nothing about how that should look, and it cannot live in
/// a match on a body-type enum either: `integrity` does not know what kinds of
/// body exist, and every layer that invents one would have to come back and
/// add an arm.
///
/// So the BODY declares it, and this observer reads whatever is there.
/// Anything without one is plate, which keeps every existing ship correct
/// without touching it.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component)]
pub enum CarveDebris {
    /// Ship plate. Freshly cut metal is INCANDESCENT, so a chip leaves
    /// near-white and cools to gunmetal while it flies.
    #[default]
    Metal,
    /// Rock. It does not glow, it spalls in greater number and it leaves
    /// slower, because what comes off a hit rock is dust and grit rather than
    /// a cut piece of plate.
    Rock,
}

impl CarveDebris {
    /// Every material, in the order the warm-up mints them.
    const ALL: [Self; 2] = [Self::Metal, Self::Rock];

    /// How this material spalls, given the weapon class's baseline look.
    fn shape(self, look: ShardLook) -> ShardLook {
        match self {
            Self::Metal => look,
            Self::Rock => ShardLook {
                size: look.size * 0.75,
                per_unit_radius: look.per_unit_radius * 1.8,
                fewest: look.fewest * 2,
                most: (look.most * 9) / 5,
            },
        }
    }

    /// How fast its chips leave, as a factor on the shared speed range.
    fn speed_scale(self) -> f32 {
        match self {
            Self::Metal => 1.0,
            // Grit off a rock carries less of the round's energy than a cut
            // piece of plate does, and it is what makes rock read as heavy.
            Self::Rock => 0.55,
        }
    }

    /// The stem of the effect's name, which is also what a `bevy_hanabi=debug`
    /// log prints when its shader is minted.
    fn name(self) -> &'static str {
        match self {
            Self::Metal => "metal",
            Self::Rock => "rock",
        }
    }
}

/// What a body is made of, from the nearest ancestor that says, `entity`
/// itself included.
///
/// A WALK and not a lookup, and the difference is player-visible. An asteroid
/// declares [`CarveDebris::Rock`] on its root while everything that announces a
/// carve names the mesh NODE beneath it - the node is where `DamageMarks` ride,
/// so it is the entity a crater is announced against and the entity a severed
/// crumb hangs off. Read flat, every one of those came back as the
/// [`CarveDebris::Metal`] default and shooting a rock threw hot gunmetal chips.
///
/// Anything silent all the way up is plate, which is what keeps every ship
/// correct without being touched.
///
/// One home for two callers: this and
/// [`pyre`](super::pyre), which asks the same question to decide that a rock
/// has no hull mass to vaporise and so no fireball.
pub fn inherited_material(
    entity: Entity,
    q_debris: &Query<&CarveDebris>,
    q_parents: &Query<&ChildOf>,
) -> CarveDebris {
    let mut current = entity;
    loop {
        if let Ok(debris) = q_debris.get(current) {
            return *debris;
        }
        let Ok(parent) = q_parents.get(current) else {
            return CarveDebris::Metal;
        };
        current = parent.0;
    }
}

/// What one weapon class's chips are.
///
/// Held per class rather than per module so the classes can diverge; see
/// [`shard_look`] for why they are separate entries even while they agree.
/// Everything NOT here - how fast a chip leaves, how wide the spray is, how it
/// tumbles, how long it lives - describes material leaving a surface rather
/// than the weapon that put it there, so it stays shared. A class that needs
/// one of those to itself grows a field here.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ShardLook {
    /// How big one chip is, in world units.
    ///
    /// A CONSTANT and not a fraction of the crater, because across the whole
    /// shipped bullet catalog the fraction was a constant wearing a curve. A
    /// kinetic PDC round pays 4 damage scaled by a speed curve clamped to
    /// `[0.25, 2.0]`, so its crater runs 0.39 to 0.78 units; the pierce PDC pays
    /// a flat 2 and cuts 0.49. At the fifth of a crater the old rule took, that
    /// whole band is 0.09 to 0.17 units of chip - under a factor of two, on a
    /// chip thrown at 2 to 6.5 u/s and gone in 2.5 seconds. Nothing reads that
    /// difference, and the curve's other end did real harm: a ram or a scripted
    /// mega-hit carves several units, and a fifth of that is a chip the size of
    /// the sections it just hit, which is why a ceiling had to be bolted on. A
    /// constant IS the ceiling.
    size: f32,
    /// How many chips one unit of crater radius is worth, between the bounds.
    per_unit_radius: f32,
    /// The fewest chips a carve throws, however small the crater.
    ///
    /// Two and not one: a single chip reads as a stray particle, a pair reads
    /// as something breaking.
    fewest: usize,
    /// The most a carve throws, however big the crater. A heavy hit should look
    /// expensive, not fill the screen with litter.
    most: usize,
}

impl ShardLook {
    /// How many chips a crater of `radius` throws.
    ///
    /// Pure, so the curve can be read without a running app, and clamped at
    /// both ends: the floor is what stops a graze reading as a single stray
    /// particle, the ceiling is what stops a ram filling the screen.
    ///
    /// This is where the SIZE of a hit still reads. A shipped PDC round throws
    /// the floor's two chips; a ram, which pays a crater an order of magnitude
    /// wider, throws the ceiling's seven.
    fn count(&self, radius: f32) -> usize {
        ((radius * self.per_unit_radius).round() as usize).clamp(self.fewest, self.most)
    }
}

/// The chips a KINETIC hit throws: the middle of the band the shipped PDC
/// rounds cut, two of them for a bullet and up to seven for a ram.
const KINETIC_SHARDS: ShardLook = ShardLook {
    size: 0.12,
    per_unit_radius: 4.0,
    fewest: 2,
    most: 7,
};

/// The chips a PIERCE hit throws.
///
/// Every number is kinetic's, written out rather than shared, and that is the
/// point: a penetrator wants debris of its own eventually, and when it does the
/// change is a number in this block. Do NOT fold it back into kinetic's arm
/// because the two currently agree.
const PIERCE_SHARDS: ShardLook = ShardLook {
    size: 0.12,
    per_unit_radius: 4.0,
    fewest: 2,
    most: 7,
};

/// What `kind` throws, or `None` for a class that throws nothing.
///
/// ONE ENTRY PER DAMAGE TYPE. Kinetic and Pierce hold identical values today
/// and are still two entries, so giving a penetrator its own look is editing
/// [`PIERCE_SHARDS`] rather than taking a branch apart.
///
/// Explosive throws nothing at all - see the module docs. That is a third
/// entry and not a special case, so changing one's mind about a warhead is the
/// same size of edit as changing one's mind about a penetrator.
fn shard_look(kind: DamageType) -> Option<ShardLook> {
    match kind {
        DamageType::Kinetic => Some(KINETIC_SHARDS),
        DamageType::Pierce => Some(PIERCE_SHARDS),
        DamageType::Explosive => None,
    }
}

/// How fast shards leave, in units per second.
const SPEW_SPEED_MIN: f32 = 2.0;
const SPEW_SPEED_MAX: f32 = 6.5;

/// How wide the spray is around the outward direction, in radians. Not a full
/// hemisphere: material knocked off a surface goes mostly the way the hit
/// pushed it, and a full dome reads as an explosion.
const SPEW_CONE: f32 = 0.9;

/// How fast a shard tumbles, in radians per second. On a camera-facing quad
/// that is a spin in the picture plane, which is what a tumbling chip reads as
/// from any distance a chip is seen at.
const SPEW_SPIN: f32 = 7.0;

/// How long a shard lives. Long enough to be seen leaving and to sell the
/// direction, short enough that a long fight leaves no litter.
const SHARD_LIFETIME_SECS: f32 = 2.5;

/// The last part of a chip's life over which it shrinks away, as a fraction
/// of [`SHARD_LIFETIME_SECS`]. A chip that stops existing between two frames
/// pops; one that draws in to nothing has left.
const SHARD_FADE: f32 = 0.2;

/// Seconds a hot chip takes to reach the cold end of its ramp.
///
/// Under a third of [`SHARD_LIFETIME_SECS`]. The glow is what says the chip was
/// just CUT, and a chip still glowing when it drifts out of frame reads as a
/// firefly instead.
const SHARD_COOL_SECS: f32 = 0.8;

/// How many keys the cooling ramp is cut into.
///
/// The ramp is a piecewise-linear gradient over the chip's age, and five keys
/// is where the straight lines between them stop showing against the square
/// law they approximate.
const SHARD_COOL_KEYS: usize = 5;

/// How hot a chip is at the instant it is cut, ADDED to the cold colour. Well
/// past 1.0 so it blooms. Zero alpha, so adding it leaves the chip opaque.
const SHARD_HOT: Vec4 = Vec4::new(7.0, 3.2, 0.9, 0.0);

/// Gunmetal, as the sun lights it. Particles are unlit, so this is the value a
/// lit plate READS at - a mid grey, faintly blue - and not the albedo the old
/// material carried, which unlit was a black chip on black space.
const METAL_COLD: Vec4 = Vec4::new(0.30, 0.30, 0.34, 1.0);

/// Rock, on the same terms: the lit value of the asteroid's own brown.
const ROCK_COLD: Vec4 = Vec4::new(0.24, 0.19, 0.15, 1.0);

/// The most emitters one material keeps, which is the most craters of that
/// material one FRAME chips.
///
/// Clear of what a firefight opens and well under what a collapse asks for. A
/// hot 4v4 opens some twenty craters a frame at its peak; a capital hull raked
/// by a siege lance opens them in the thousands over one collapse. The pool is
/// grown to what the busiest frame so far needed and kept, so a quiet run
/// never mints most of these.
const SHARD_EMITTERS: usize = 64;

/// How many emitters a material is warmed WITH, before any crater opens.
///
/// More than one, and not for the shader: minting the shader takes one. A lone
/// gun plinking one rock hits it about once a frame, and one emitter re-aimed
/// every frame holds a whole lifetime of bursts in its own buffer - so the
/// first bursts rotate through a few emitters instead of piling into one.
const SHARD_EMITTER_FLOOR: usize = 4;

/// The buffer each emitter's chips live in.
///
/// An emitter is re-aimed as often as the pool rotates back to it, and every
/// burst it has thrown in the last [`SHARD_LIFETIME_SECS`] is still in this
/// buffer. At 60 frames a second a single emitter fired every frame holds 150
/// bursts; at the seven chips of a metal ceiling that is 1050, and a rock's
/// twelve is 1800. Over the floor the rotation divides that by
/// [`SHARD_EMITTER_FLOOR`]. A burst that finds the buffer full is clipped
/// rather than refused, which on a lone rock at a very high frame rate is a
/// few grit particles fewer.
const SHARD_CAPACITY: u32 = 1024;

/// The names the per-crater values are written under, in both graphs.
///
/// `bevy_hanabi` adds only the emitter's TRANSLATION to a global-space
/// particle, never its rotation, so the cone's basis is handed to the graph
/// as three vectors rather than as the emitter's orientation.
const OUTWARD_PROPERTY: &str = "outward";
const RIGHT_PROPERTY: &str = "right";
const UP_PROPERTY: &str = "up";
/// How far off the crater's centre a chip starts, world units: its lip.
const LIP_PROPERTY: &str = "lip";
/// How big a chip is, world units. Per crater and not per graph, so the class
/// table stays the one place a chip's size is authored.
const CHIP_PROPERTY: &str = "chip";

/// How many chips carves have asked the GPU for, over the whole run.
///
/// The count a range reads, because the chips themselves are on the GPU and
/// nothing on the CPU stands for one. Cumulative, so a probe takes a mark and
/// reads the difference; a frame's own share is that difference across one
/// update.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarveShardTally {
    /// Chips thrown so far. A carve the pool refused adds nothing.
    pub thrown: u64,
}

/// Marks one of the pooled emitters carves are aimed through, so a range can
/// count the pool. NOT a chip: chips are particles and have no entity.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct CarveShardEmitterMarker;

/// One material's emitters, and where the rotation through them stands.
struct ShardPool {
    /// The graph every emitter in this pool instances.
    effect: Handle<EffectAsset>,
    /// The emitters, in the order they were minted. Grown by a carve that
    /// finds every one of them fired this frame, up to [`SHARD_EMITTERS`], and
    /// never shrunk.
    emitters: Vec<Entity>,
    /// The next emitter to aim. A ROTATION rather than a scan from the front,
    /// so consecutive frames spread their bursts across the pool instead of
    /// re-aiming the first emitter every frame and filling its buffer alone.
    next: usize,
    /// How many emitters this FRAME has fired. Reset in `First`.
    fired: usize,
}

/// The pool for every material, minted on first use.
///
/// A MAP and not a pair of fields, so a third material is one arm in
/// [`build_shard_effect`] rather than an edit here as well - the same shape
/// `DefaultTorpedoRender` uses to key warhead materials by tint.
///
/// Warmed by [`warm_the_shards`] rather than built by [`FromWorld`], so an app
/// with no asset store and one running at a graphics tier with particles off
/// still build nothing.
#[derive(Resource, Default)]
struct DebrisLooks(HashMap<CarveDebris, ShardPool>);

impl DebrisLooks {
    /// The pool for `debris`, building its graph on the first call.
    fn pool(&mut self, debris: CarveDebris, effects: &mut Assets<EffectAsset>) -> &mut ShardPool {
        self.0.entry(debris).or_insert_with(|| ShardPool {
            effect: effects.add(build_shard_effect(debris)),
            emitters: Vec::new(),
            next: 0,
            fired: 0,
        })
    }
}

/// Hand every pool its emitters back.
///
/// In `First`, so a carve announced from `FixedUpdate` draws on the same
/// allowance as one announced from `Update`, and a FRAME rather than a fixed
/// step because `bevy_hanabi` samples an emitter once a frame: two bursts
/// aimed through one emitter in one frame are one burst, the second.
fn open_the_emitter_budget(mut looks: ResMut<DebrisLooks>) {
    for pool in looks.0.values_mut() {
        pool.fired = 0;
    }
}

/// The cooling ramp, over a chip's normalised age.
///
/// Metal starts at [`SHARD_HOT`] over its cold colour and is cold by
/// [`SHARD_COOL_SECS`], squared so most of the glow is gone in the first step
/// or two: metal loses heat fastest when it is hottest, and a linear ramp
/// reads as a chip being dimmed by a knob. Rock is one colour from birth: it
/// never glowed.
fn shard_gradient(debris: CarveDebris) -> bevy_hanabi::Gradient<Vec4> {
    let mut gradient = bevy_hanabi::Gradient::new();
    match debris {
        CarveDebris::Metal => {
            let cool = SHARD_COOL_SECS / SHARD_LIFETIME_SECS;
            for key in 0..SHARD_COOL_KEYS {
                let t = key as f32 / (SHARD_COOL_KEYS - 1) as f32;
                let heat = (1.0 - t) * (1.0 - t);
                gradient.add_key(cool * t, METAL_COLD + SHARD_HOT * heat);
            }
            gradient.add_key(1.0, METAL_COLD);
        }
        CarveDebris::Rock => {
            gradient.add_key(0.0, ROCK_COLD);
            gradient.add_key(1.0, ROCK_COLD);
        }
    }
    gradient
}

/// The idle spawner every emitter is minted with: a single burst, held until
/// a carve resets it. Its count is overwritten per crater.
fn idle_spawner() -> SpawnerSettings {
    SpawnerSettings::once(1.0.into()).with_emit_on_start(false)
}

/// One material's graph.
///
/// Everything the old per-shard entity carried is here: the crater's outward
/// cone, the speed band, the lip a chip starts on, the tumble, the lifetime,
/// the cooling. The values that change per crater arrive as properties, the
/// ones that describe the material are literals. Global space, so a chip is
/// detached from its emitter the instant it is thrown and the emitter can be
/// aimed at the next crater while it flies.
fn build_shard_effect(debris: CarveDebris) -> EffectAsset {
    let writer = ExprWriter::new();

    let outward = writer.prop(writer.add_property(OUTWARD_PROPERTY, Vec3::Y.into()));
    let right = writer.prop(writer.add_property(RIGHT_PROPERTY, Vec3::X.into()));
    let up = writer.prop(writer.add_property(UP_PROPERTY, Vec3::Z.into()));
    let lip = writer.prop(writer.add_property(LIP_PROPERTY, 0.0f32.into()));
    let chip = writer.add_property(CHIP_PROPERTY, KINETIC_SHARDS.size.into());

    // Three independent draws: the turn about the outward axis, how far off
    // it to lean, and how hard to throw. The same cone the CPU used to hash
    // off the crater, in the GPU's random.
    let turn = writer.rand(ScalarType::Float) * writer.lit(TAU);
    let lean = writer.rand(ScalarType::Float) * writer.lit(SPEW_CONE);
    let sideways = (right * turn.clone().cos() + up * turn.sin()) * lean.clone().sin();
    let direction = outward * lean.cos() + sideways;
    let speed = writer
        .lit(SPEW_SPEED_MIN * debris.speed_scale())
        .uniform(writer.lit(SPEW_SPEED_MAX * debris.speed_scale()));

    // Started at the crater's LIP rather than its centre, so a chip is not
    // drawn inside the material it supposedly just left.
    let init_pos = SetAttributeModifier::new(Attribute::POSITION, (direction.clone() * lip).expr());
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, (direction * speed).expr());
    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
    let init_lifetime =
        SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(SHARD_LIFETIME_SECS).expr());
    let init_color = SetAttributeModifier::new(Attribute::COLOR, writer.lit(0xFFFFFFFFu32).expr());
    // Which way round the chip starts, so a burst is not seven aligned squares.
    let init_facing = SetAttributeModifier::new(
        Attribute::F32_0,
        (writer.rand(ScalarType::Float) * writer.lit(TAU)).expr(),
    );

    // One size for the whole flight, drawn in over the last fifth of it.
    let age = writer.attr(Attribute::AGE) / writer.attr(Attribute::LIFETIME);
    let remaining = ((writer.lit(1.0) - age) * writer.lit(1.0 / SHARD_FADE)).saturate();
    let update_size =
        SetAttributeModifier::new(Attribute::SIZE, (writer.prop(chip) * remaining).expr());

    let spin = (writer.attr(Attribute::F32_0)
        + writer.attr(Attribute::AGE) * writer.lit(SPEW_SPIN))
    .expr();

    EffectAsset::new(SHARD_CAPACITY, idle_spawner(), writer.finish())
        .with_name(format!("carve_shards_{}", debris.name()))
        .with_simulation_space(SimulationSpace::Global)
        // Solid. A chip is a piece of something, and an opaque quad depth-tests
        // against the hull it left instead of blending over it.
        .with_alpha_mode(bevy_hanabi::AlphaMode::Opaque)
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .init(init_color)
        .init(init_facing)
        .update(update_size)
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane).with_rotation(spin))
        .render(ColorOverLifetimeModifier {
            gradient: shard_gradient(debris),
            blend: ColorBlendMode::default(),
            mask: ColorBlendMask::default(),
        })
}

/// One crater's burst, as the emitter is aimed at it.
#[derive(Clone, Copy, Debug)]
struct Burst {
    /// The crater's centre, world space.
    at: Vec3,
    /// OUT of the body the crater is in.
    outward: Vec3,
    /// The crater's lip: how far from `at` a chip starts.
    lip: f32,
    /// How big a chip is.
    chip: f32,
    /// How many chips.
    count: usize,
}

impl Burst {
    /// Aim an emitter at this crater and arm it. The burst leaves on the
    /// frame's spawner tick.
    fn aim(
        &self,
        transform: &mut Transform,
        properties: &mut EffectProperties,
        spawner: &mut EffectSpawner,
    ) {
        transform.translation = self.at;
        // Any axis not parallel to `outward` builds the basis;
        // `any_orthonormal_pair` picks one without a degenerate case to guard.
        let (right, up) = self.outward.any_orthonormal_pair();
        properties.set(OUTWARD_PROPERTY, self.outward.into());
        properties.set(RIGHT_PROPERTY, right.into());
        properties.set(UP_PROPERTY, up.into());
        properties.set(LIP_PROPERTY, self.lip.into());
        properties.set(CHIP_PROPERTY, self.chip.into());
        spawner.settings.set_count((self.count as f32).into());
        spawner.reset();
    }

    /// A fresh emitter, already aimed here.
    fn emitter(&self, effect: Handle<EffectAsset>) -> impl Bundle {
        let mut transform = Transform::IDENTITY;
        let mut properties = EffectProperties::default();
        let mut spawner = EffectSpawner::new(&idle_spawner());
        self.aim(&mut transform, &mut properties, &mut spawner);
        (
            Name::new("Carve Shard Emitter"),
            CarveShardEmitterMarker,
            ParticleEffect::new(effect),
            transform,
            properties,
            spawner,
        )
    }
}

/// A fresh emitter aimed at nothing, for the warm-up.
fn idle_emitter(effect: Handle<EffectAsset>) -> impl Bundle {
    (
        Name::new("Carve Shard Emitter"),
        CarveShardEmitterMarker,
        ParticleEffect::new(effect),
        EffectProperties::default(),
        EffectSpawner::new(&idle_spawner()),
    )
}

/// The asset store a chip needs, or nothing at all.
///
/// Two refusals with one answer, because they have the same consequence. A
/// world with no effect store has nothing to build a graph in and nothing that
/// could see the result: a headless server, or a test app that added the
/// integrity plugin for its health pipeline alone. A tier with particles off is
/// the spawn-less low-end mode, which is a policy rather than a limitation. An
/// ABSENT budget is a settings-less app, which means full quality. Carving
/// still happens in all three, it just goes unseen. The same gate
/// [`pyre`](super::pyre) keeps for its fireballs.
fn drawable<'w>(
    tier: Option<Res<GraphicsBudget>>,
    effects: Option<ResMut<'w, Assets<EffectAsset>>>,
) -> Option<ResMut<'w, Assets<EffectAsset>>> {
    if !tier.as_deref().is_none_or(|tier| tier.particles) {
        return None;
    }
    effects
}

/// Mint the graphs, the shaders AND the first emitters before any crater opens.
///
/// The graphs are not the expensive half. `bevy_hanabi` generates a WGSL
/// source from a `CompiledParticleEffect`, and only for spawned INSTANCES, so
/// without this the first bullet of the first fight would mint its shader
/// inside the frame the hit lands in. [`pyre`](super::pyre) measured that
/// shape on its fireballs.
///
/// Unlike the pyre's, these instances are not throwaways: they are the first
/// [`SHARD_EMITTER_FLOOR`] emitters of each pool, visible and idle, and the
/// first carves are aimed through them. Visible, so the render pipeline is
/// specialised here too - an emitter with nothing in flight draws nothing and
/// costs one empty dispatch.
///
/// In `Update`, on the first frame that has all three of a cold pool
/// ([`shards_are_cold`]), a tier that draws particles at all
/// ([`the_tier_draws_particles`]) and a camera to render from
/// ([`a_view_exists`]); the pyre states why the last two are conditions
/// rather than preferences.
fn warm_the_shards(
    mut commands: Commands,
    effects: Option<ResMut<Assets<EffectAsset>>>,
    mut looks: ResMut<DebrisLooks>,
    tier: Option<Res<GraphicsBudget>>,
) {
    let Some(mut effects) = drawable(tier, effects) else {
        return;
    };
    for debris in CarveDebris::ALL {
        let pool = looks.pool(debris, &mut effects);
        while pool.emitters.len() < SHARD_EMITTER_FLOOR {
            let emitter = commands.spawn(idle_emitter(pool.effect.clone())).id();
            pool.emitters.push(emitter);
        }
    }
}

/// Whether any pool is still under its floor, which is the only state a
/// re-warm has anything to do in.
fn shards_are_cold(looks: Res<DebrisLooks>) -> bool {
    CarveDebris::ALL.iter().any(|debris| {
        looks
            .0
            .get(debris)
            .is_none_or(|pool| pool.emitters.len() < SHARD_EMITTER_FLOOR)
    })
}

/// Whether the budget in force draws particles at all. A settings-less app is
/// full quality.
fn the_tier_draws_particles(tier: Option<Res<GraphicsBudget>>) -> bool {
    tier.as_deref().is_none_or(|tier| tier.particles)
}

/// Whether there is a camera for the render world to build a view from. An
/// inactive camera produces no view, so it does not count.
fn a_view_exists(cameras: Query<&Camera>) -> bool {
    cameras.iter().any(|camera| camera.is_active)
}

/// Throws the chips an impact knocked off.
pub struct CarveSpewPlugin;

impl Plugin for CarveSpewPlugin {
    fn build(&self, app: &mut App) {
        trace!("CarveSpewPlugin: build");

        app.register_type::<CarveShardEmitterMarker>();
        app.register_type::<CarveDebris>();
        app.init_resource::<DebrisLooks>();
        app.init_resource::<CarveShardTally>();
        app.add_observer(spew_carved_material);
        app.add_systems(First, open_the_emitter_budget);
        app.add_systems(
            Update,
            warm_the_shards
                .after(SettingsSystems)
                .run_if(shards_are_cold)
                .run_if(the_tier_draws_particles)
                .run_if(a_view_exists),
        );
    }
}

/// Aim the next free emitter at the crater.
#[expect(
    clippy::too_many_arguments,
    reason = "one observer aiming a hanabi instance: the pool, the tally, the tier gate, the graph store and the queries that place it and say what it is made of"
)]
fn spew_carved_material(
    spew: On<CarveSpew>,
    mut commands: Commands,
    mut looks: ResMut<DebrisLooks>,
    mut tally: ResMut<CarveShardTally>,
    tier: Option<Res<GraphicsBudget>>,
    effects: Option<ResMut<Assets<EffectAsset>>>,
    q_body: Query<&GlobalTransform>,
    q_debris: Query<&CarveDebris>,
    q_parents: Query<&ChildOf>,
    mut q_emitter: Query<
        (&mut Transform, &mut EffectProperties, &mut EffectSpawner),
        With<CarveShardEmitterMarker>,
    >,
) {
    // The class decides first, so a warhead costs nothing at all here.
    let Some(look) = shard_look(spew.kind) else {
        return;
    };
    let Some(mut effects) = drawable(tier, effects) else {
        return;
    };

    // OUT of the body, which for a crater means away from the body's own
    // origin. A hit dead on the centre has no outward direction to speak of, so
    // it falls back to up rather than to a zero vector nothing can be built on.
    let outward = q_body.get(spew.entity).map_or(Vec3::Y, |frame| {
        (spew.at - frame.translation()).normalize_or(Vec3::Y)
    });

    // The body decides what it is made of; anything silent is plate. A walk,
    // because a rock says so on its root and is carved on the node beneath it.
    let debris = inherited_material(spew.entity, &q_debris, &q_parents);
    let look = debris.shape(look);
    let burst = Burst {
        at: spew.at,
        outward,
        lip: spew.radius * 0.5,
        chip: look.size,
        count: look.count(spew.radius),
    };

    let pool = looks.pool(debris, &mut effects);
    if pool.fired < pool.emitters.len() {
        // The rotation's next emitter has not fired this frame.
        let index = pool.next % pool.emitters.len();
        pool.next = (index + 1) % pool.emitters.len();
        let emitter = pool.emitters[index];
        match q_emitter.get_mut(emitter) {
            Ok((mut transform, mut properties, mut spawner)) => {
                burst.aim(&mut transform, &mut properties, &mut spawner);
            }
            // Taken away from under the pool - a scene torn down around it -
            // so its replacement is minted in its slot, aimed at this crater.
            Err(_) if commands.get_entity(emitter).is_err() => {
                pool.emitters[index] = commands.spawn(burst.emitter(pool.effect.clone())).id();
            }
            // Minted this frame and not yet applied. The next frame has it.
            Err(_) => {
                trace!(
                    "spew_carved_material: {:?} goes unchipped, its emitter is still being minted",
                    spew.entity
                );
                return;
            }
        }
    } else if pool.emitters.len() < SHARD_EMITTERS {
        // Every emitter has fired this frame and the pool has room to grow.
        let emitter = commands.spawn(burst.emitter(pool.effect.clone())).id();
        pool.emitters.push(emitter);
        pool.next = 0;
    } else {
        trace!(
            "spew_carved_material: {:?} goes unchipped, the frame's emitters are spent",
            spew.entity
        );
        return;
    }
    pool.fired += 1;
    tally.thrown += burst.count as u64;

    trace!(
        "spew_carved_material: {} {debris:?} shard(s) off {:?} at {} ({:?})",
        burst.count,
        spew.entity,
        spew.at,
        spew.kind
    );
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::{Collider, RigidBody};

    use super::*;
    use crate::{
        integrity::{
            carve::prelude::{DamageMark, DamageMarks},
            chunk::prelude::{CarvedChunkMarker, CHUNK_MIN_VOLUME},
        },
        settings::prelude::GraphicsQuality,
    };

    /// Every class that throws anything, so a new damage type cannot be added
    /// without a test noticing what it does.
    const THROWING: [(DamageType, ShardLook); 2] = [
        (DamageType::Kinetic, KINETIC_SHARDS),
        (DamageType::Pierce, PIERCE_SHARDS),
    ];

    /// The plugin plus the effect store it builds its graphs in. No render
    /// app and no camera: an [`EffectAsset`] is data, and what is under test
    /// is which emitters a carve aims, not how they draw.
    fn spew_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(Assets::<EffectAsset>::default());
        app.add_plugins(CarveSpewPlugin);
        app
    }

    /// The same with a camera, one frame past the warm-up, for the tests
    /// about the pool the warm-up seeds.
    fn viewed_spew_app() -> App {
        let mut app = spew_app();
        app.world_mut().spawn(Camera::default());
        app.update();
        app
    }

    /// One emitter as a carve left it: where it stands, which way it throws,
    /// how big and how many, and whether it is armed to fire at all.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Aimed {
        at: Vec3,
        outward: Vec3,
        lip: f32,
        chip: f32,
        count: f32,
        armed: bool,
    }

    fn property<T>(properties: &EffectProperties, name: &str) -> Option<T>
    where
        bevy_hanabi::graph::Value: TryInto<T>,
    {
        properties
            .get_stored(name)
            .and_then(|value| value.try_into().ok())
    }

    /// Every emitter standing, in pool order of entity.
    fn emitters(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<CarveShardEmitterMarker>>()
            .iter(app.world())
            .collect()
    }

    /// Every emitter a carve has aimed. Armed stays true in these apps: no
    /// hanabi tick ever completes the burst.
    fn aimed(app: &mut App) -> Vec<Aimed> {
        app.world_mut()
            .query_filtered::<
                (&Transform, &EffectProperties, &EffectSpawner),
                With<CarveShardEmitterMarker>,
            >()
            .iter(app.world())
            .filter_map(|(transform, properties, spawner)| {
                let CpuValue::Single(count) = spawner.settings.count() else {
                    return None;
                };
                Some(Aimed {
                    at: transform.translation,
                    outward: property(properties, OUTWARD_PROPERTY)?,
                    lip: property(properties, LIP_PROPERTY)?,
                    chip: property(properties, CHIP_PROPERTY)?,
                    count,
                    armed: !spawner.has_completed(),
                })
            })
            .filter(|aimed| aimed.armed)
            .collect()
    }

    fn thrown(app: &App) -> u64 {
        app.world().resource::<CarveShardTally>().thrown
    }

    /// Throw one crater of `radius` off a body at the origin and report how
    /// many chips it asked for.
    fn carve(app: &mut App, kind: DamageType, radius: f32) -> u64 {
        let before = thrown(app);
        carve_body(app, kind, radius, None);
        thrown(app) - before
    }

    /// Throw one crater off a body optionally declaring what it is made of.
    fn carve_body(app: &mut App, kind: DamageType, radius: f32, debris: Option<CarveDebris>) {
        let mut body = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::ZERO));
        if let Some(debris) = debris {
            body.insert(debris);
        }
        let body = body.id();
        app.world_mut().trigger(CarveSpew {
            entity: body,
            at: Vec3::X * 3.0,
            radius,
            kind,
        });
        app.update();
    }

    /// The graph every emitter instances, which is one per material.
    fn effects(app: &mut App) -> Vec<Handle<EffectAsset>> {
        app.world_mut()
            .query_filtered::<&ParticleEffect, With<CarveShardEmitterMarker>>()
            .iter(app.world())
            .map(|effect| effect.handle.clone())
            .collect()
    }

    fn effect_name(app: &App, handle: &Handle<EffectAsset>) -> String {
        app.world()
            .resource::<Assets<EffectAsset>>()
            .get(handle)
            .expect("an emitter instances a graph that stands")
            .name
            .clone()
    }

    /// THE ruling this module implements: chips are an IMPACT effect. A bullet
    /// of either type chips what it hits; a warhead's own fireball is the cue
    /// for a blast, so it throws nothing on top of it.
    #[test]
    fn a_bullet_chips_what_it_hits_and_a_warhead_does_not() {
        for (kind, _) in THROWING {
            let mut app = spew_app();
            assert!(
                carve(&mut app, kind, 0.6) > 0,
                "{kind:?} threw nothing off a bullet-sized crater"
            );
            assert_eq!(aimed(&mut app).len(), 1, "{kind:?} aimed no emitter");
        }

        let mut app = spew_app();
        assert_eq!(
            carve(&mut app, DamageType::Explosive, 3.0),
            0,
            "a blast littered its own fireball with chips"
        );
        assert!(
            emitters(&mut app).is_empty(),
            "a blast minted an emitter it has no use for"
        );
    }

    /// The two bullet classes agree TODAY and are configured apart, which is
    /// the whole structure: this test is a one-line delete on the day a
    /// penetrator earns debris of its own, and nothing else has to move.
    #[test]
    fn pierce_throws_exactly_what_kinetic_does_for_now() {
        assert_eq!(PIERCE_SHARDS, KINETIC_SHARDS);

        let mut kinetic_app = spew_app();
        let mut pierce_app = spew_app();
        assert_eq!(
            carve(&mut kinetic_app, DamageType::Kinetic, 0.6),
            carve(&mut pierce_app, DamageType::Pierce, 0.6)
        );
        assert_eq!(aimed(&mut kinetic_app), aimed(&mut pierce_app));
    }

    /// A bigger crater throws more, and neither end runs away: one chip reads
    /// as a stray particle, fifty read as litter. This is where the SIZE of a
    /// hit still reads, now that a chip is one size.
    #[test]
    fn a_bigger_crater_throws_more_shards_but_never_a_swarm() {
        for (kind, look) in THROWING {
            assert_eq!(look.count(0.0), look.fewest, "{kind:?} floor");
            assert!(look.count(0.5) >= look.fewest);
            assert!(look.count(0.5) <= look.count(1.5));
            assert_eq!(look.count(100.0), look.most, "{kind:?} ceiling");
        }
    }

    /// A chip is one size whatever hole it came out of, so no hit can grow one.
    /// The crater-proportional rule this replaced put section-sized cubes
    /// beside a hull the moment something paid a crater several units across.
    #[test]
    fn a_shard_is_the_same_size_however_big_the_hit_was() {
        for (kind, look) in THROWING {
            for radius in [0.15f32, 0.6, 8.0, 50.0] {
                let mut app = spew_app();
                assert!(
                    carve(&mut app, kind, radius) > 0,
                    "{kind:?} at {radius} threw nothing"
                );
                for aimed in aimed(&mut app) {
                    assert!(
                        (aimed.chip - look.size).abs() < 1e-6,
                        "{kind:?} at radius {radius} drew a {}u chip",
                        aimed.chip
                    );
                }
            }
        }
    }

    /// The line between decoration and material. A chip has no collider and no
    /// mass, so it must never reach the size at which a piece is worth
    /// simulating as a body of its own - clearly under the line rather than
    /// beside it, at half the side of that cube and an eighth of its volume. At
    /// the line itself a chip is a cube the size of a ship section, which reads
    /// as the hull coming apart rather than as a hit on it.
    #[test]
    fn no_class_throws_a_shard_the_size_of_real_material() {
        let chunk_side = CHUNK_MIN_VOLUME.cbrt();
        for (kind, look) in THROWING {
            assert!(
                look.size * 2.0 <= chunk_side,
                "{kind:?} draws a {}u chip, and material starts at {chunk_side}",
                look.size
            );
        }
    }

    /// THE claim: material comes off, and it comes off OUTWARD. The cone is
    /// built on the GPU about the axis the emitter is handed, so the axis is
    /// what a test can hold: out of the body, from the crater, off its lip.
    #[test]
    fn every_shard_leaves_the_body_it_came_off() {
        let mut app = spew_app();
        let hull = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::ZERO))
            .id();

        // A crater on the +X face.
        let at = Vec3::new(3.0, 0.0, 0.0);
        app.world_mut().trigger(CarveSpew {
            entity: hull,
            at,
            radius: 1.0,
            kind: DamageType::Kinetic,
        });
        app.update();

        let aimed = aimed(&mut app);
        assert_eq!(aimed.len(), 1, "a carve aims one emitter");
        let burst = aimed[0];
        assert!(
            burst.outward.dot(Vec3::X) > 0.99,
            "the chips are thrown back into the hull: {}",
            burst.outward
        );
        assert_eq!(burst.at, at, "the emitter stands off the crater");
        assert!(
            burst.lip > 0.0 && burst.lip <= 1.0,
            "a chip starts inside the material it left, or clear of the crater: {}",
            burst.lip
        );
    }

    /// THE line this module draws. A hole is a hole however deep it goes: a
    /// carve throws dust and never a body, because nothing here can tell a deep
    /// crater from a cut that severed something. Bodies come from the carve
    /// path that knows what it cut free.
    #[test]
    fn every_carve_throws_dust_and_never_a_body() {
        for radius in [0.1f32, 1.0, 8.0, 50.0] {
            let mut app = spew_app();
            let thrown = carve(&mut app, DamageType::Kinetic, radius);

            let bodies = app
                .world_mut()
                .query_filtered::<(), With<CarvedChunkMarker>>()
                .iter(app.world())
                .count();
            assert_eq!(bodies, 0, "radius {radius} threw {bodies} body(s)");
            assert!(thrown > 0, "radius {radius} threw no dust");
        }
    }

    /// The reason chips are particles. A fight's worth of craters leaves the
    /// solver, the sensor sweep and the mesh extractor NOTHING: no rigid body,
    /// no collider, no mesh per chip, and a pool that stops growing.
    #[test]
    fn a_fight_of_carves_leaves_the_solver_and_the_renderer_nothing_to_carry() {
        let mut app = spew_app();
        let hull = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        for frame in 0..40 {
            for nth in 0..3 {
                app.world_mut().trigger(CarveSpew {
                    entity: hull,
                    at: Vec3::new(3.0 + nth as f32, frame as f32, 0.0),
                    radius: 0.6,
                    kind: DamageType::Kinetic,
                });
            }
            app.update();
        }
        assert!(thrown(&app) >= 240, "delivery guard: it spewed");

        let bodies = app
            .world_mut()
            .query_filtered::<(), With<RigidBody>>()
            .iter(app.world())
            .count();
        let colliders = app
            .world_mut()
            .query_filtered::<(), With<Collider>>()
            .iter(app.world())
            .count();
        let meshes = app
            .world_mut()
            .query_filtered::<(), With<Mesh3d>>()
            .iter(app.world())
            .count();
        assert_eq!(bodies, 0, "a chip put a body in the solver");
        assert_eq!(colliders, 0, "a chip put a collider in the broad phase");
        assert_eq!(meshes, 0, "a chip put a mesh instance in the extractor");
        assert_eq!(
            emitters(&mut app).len(),
            3,
            "the pool grew past what the busiest frame needed"
        );
    }

    /// Repeated fire pays for more material, so it keeps announcing a carve
    /// without spending another mark slot.
    #[test]
    fn a_hit_into_an_existing_crater_still_takes_material() {
        let mut marks = DamageMarks::default();
        marks.add(
            DamageMark {
                at: Vec3::ZERO,
                radius: 1.0,
            },
            1.0,
        );
        assert!(marks.add(
            DamageMark {
                at: Vec3::X * 0.1,
                radius: 0.2,
            },
            1.0,
        ));
        assert_eq!(marks.0.len(), 1);
        assert!(marks.0[0].radius > 1.0);
    }

    /// Announce `craters` carves into one frame, the way a command flush does,
    /// and report how many chips were asked for.
    ///
    /// Every crater here is wide enough to want the ceiling, so the only thing
    /// that can hold the count down is the pool.
    fn rake_one_frame(app: &mut App, craters: usize) -> u64 {
        let before = thrown(app);
        let hull = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        for nth in 0..craters {
            app.world_mut().trigger(CarveSpew {
                entity: hull,
                at: Vec3::X * (3.0 + nth as f32),
                radius: 2.0,
                kind: DamageType::Kinetic,
            });
        }
        app.update();
        thrown(app) - before
    }

    /// A lone crater is untouched by the pool, and that is the point of
    /// putting the ceiling on the frame: what one hit throws is what a hit
    /// looks like, whatever else is happening. And one burst fits its buffer.
    #[test]
    fn one_carve_still_throws_everything_its_crater_is_worth() {
        for (kind, look) in THROWING {
            let mut app = spew_app();
            assert_eq!(
                carve(&mut app, kind, 100.0),
                look.most as u64,
                "{kind:?} lost chips off a crater nothing was competing with"
            );
            for debris in CarveDebris::ALL {
                assert!(
                    debris.shape(look).most as u32 <= SHARD_CAPACITY,
                    "{kind:?} off {debris:?} cannot fit one burst in an emitter"
                );
            }
        }
    }

    /// THE ceiling. Hundreds of craters announcing themselves in one command
    /// flush is what a capital hull coming apart does, and the frame that has
    /// to aim their emitters is the one that cannot afford a pool that size.
    #[test]
    fn one_frame_cannot_be_made_to_throw_more_than_its_pool() {
        let mut app = spew_app();
        let thrown = rake_one_frame(&mut app, 200);
        assert_eq!(thrown, (SHARD_EMITTERS * KINETIC_SHARDS.most) as u64);
        assert_eq!(emitters(&mut app).len(), SHARD_EMITTERS);
    }

    /// And the emitters come back: a long fight is a sequence of frames, so
    /// nothing here starves a firefight of debris, and the pool does not grow
    /// past its ceiling to do it.
    #[test]
    fn the_next_frame_gets_its_emitters_back() {
        let mut app = spew_app();
        let ceiling = (SHARD_EMITTERS * KINETIC_SHARDS.most) as u64;
        assert_eq!(rake_one_frame(&mut app, 200), ceiling);
        assert_eq!(
            rake_one_frame(&mut app, 200),
            ceiling,
            "the second frame threw nothing of its own"
        );
        assert_eq!(emitters(&mut app).len(), SHARD_EMITTERS);
    }

    /// Consecutive frames rotate through the pool rather than re-aiming the
    /// first emitter every time, which is what keeps a lone rock's bursts out
    /// of one buffer.
    #[test]
    fn a_crater_a_frame_rotates_through_the_pool() {
        let mut app = viewed_spew_app();
        let hull = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        for frame in 0..SHARD_EMITTER_FLOOR {
            app.world_mut().trigger(CarveSpew {
                entity: hull,
                at: Vec3::new(3.0, frame as f32, 0.0),
                radius: 0.6,
                kind: DamageType::Kinetic,
            });
            app.update();
        }
        let aimed = aimed(&mut app);
        assert_eq!(
            aimed.len(),
            SHARD_EMITTER_FLOOR,
            "one crater a frame wore one emitter out instead of rotating"
        );
        let heights: std::collections::BTreeSet<i32> =
            aimed.iter().map(|burst| burst.at.y as i32).collect();
        assert_eq!(
            heights.len(),
            SHARD_EMITTER_FLOOR,
            "two frames aimed one emitter"
        );
    }

    /// The warm-up seeds every pool to its floor the moment there is a view,
    /// idle, and asks the GPU for nothing while it does.
    #[test]
    fn the_warm_up_seeds_the_pool_when_a_view_exists() {
        let mut app = viewed_spew_app();
        assert_eq!(
            emitters(&mut app).len(),
            SHARD_EMITTER_FLOOR * CarveDebris::ALL.len()
        );
        assert!(aimed(&mut app).is_empty(), "the warm-up armed an emitter");
        assert_eq!(thrown(&app), 0, "the warm-up counted chips it never threw");
        app.update();
        assert_eq!(
            emitters(&mut app).len(),
            SHARD_EMITTER_FLOOR * CarveDebris::ALL.len(),
            "the warm-up ran again over a pool already at its floor"
        );

        let mut viewless = spew_app();
        viewless.update();
        assert!(
            emitters(&mut viewless).is_empty(),
            "the warm-up ran with nothing to render from"
        );
    }

    /// The spawn-less tier: nothing minted, nothing aimed, nothing counted.
    #[test]
    fn a_tier_without_particles_throws_nothing() {
        let mut app = spew_app();
        app.insert_resource(GraphicsBudget::for_quality(GraphicsQuality::Low));
        app.world_mut().spawn(Camera::default());
        app.update();
        assert!(emitters(&mut app).is_empty(), "the warm-up ran on Low");
        assert_eq!(carve(&mut app, DamageType::Kinetic, 0.6), 0);
        assert!(
            emitters(&mut app).is_empty(),
            "a carve minted an emitter on Low"
        );
        assert_eq!(
            app.world().resource::<Assets<EffectAsset>>().len(),
            0,
            "a graph was built for a tier that draws none"
        );
    }

    /// A headless app - the integrity plugin added for its health pipeline
    /// alone - carves in silence rather than panicking on a missing store.
    #[test]
    fn a_world_without_an_effect_store_carves_in_silence() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CarveSpewPlugin);
        assert_eq!(carve(&mut app, DamageType::Kinetic, 0.6), 0);
        assert!(emitters(&mut app).is_empty());
    }

    /// The ruling the owner took: a rock must not throw the hull's chips. Both
    /// bodies take the identical carve, and the only thing that differs is what
    /// the body said it was made of.
    #[test]
    fn a_rock_and_a_hull_throw_different_debris() {
        let mut plate = spew_app();
        carve_body(&mut plate, DamageType::Kinetic, 0.6, None);
        let plate_effects = effects(&mut plate);

        let mut rock = spew_app();
        carve_body(&mut rock, DamageType::Kinetic, 0.6, Some(CarveDebris::Rock));
        let rock_effects = effects(&mut rock);

        assert_eq!(plate_effects.len(), 1);
        assert_eq!(rock_effects.len(), 1);
        assert_eq!(effect_name(&plate, &plate_effects[0]), "carve_shards_metal");
        assert_eq!(effect_name(&rock, &rock_effects[0]), "carve_shards_rock");
    }

    /// Metal leaves incandescent and is gunmetal before a third of its life is
    /// gone; rock is its own brown from the first frame and never blooms.
    #[test]
    fn metal_cools_from_white_hot_to_gunmetal_and_rock_never_glows() {
        let metal = shard_gradient(CarveDebris::Metal);
        let cut = metal.sample(0.0);
        assert!(
            cut.x > 1.0 && cut.y > 1.0,
            "a freshly cut chip of plate is not incandescent: {cut}"
        );
        let cool = SHARD_COOL_SECS / SHARD_LIFETIME_SECS;
        assert!(cool < 1.0 / 3.0, "the glow outlasts a third of the flight");
        let cooled = metal.sample(cool);
        assert!(
            (cooled - METAL_COLD).abs().max_element() < 1e-5,
            "the chip is still glowing at the cold end of its ramp: {cooled}"
        );
        assert_eq!(metal.sample(1.0), METAL_COLD, "gunmetal does not drift");
        let mut last = cut.x;
        for step in 1..=10 {
            let now = metal.sample(cool * step as f32 / 10.0).x;
            assert!(now <= last + 1e-5, "the chip warmed back up while flying");
            last = now;
        }
        assert!(
            (metal.sample(cool * 0.25).x - METAL_COLD.x) < (cut.x - METAL_COLD.x) * 0.6,
            "the glow leaves slower than a square law: a chip dimmed by a knob"
        );
        for key in metal.keys() {
            assert!(
                (key.value.w - 1.0).abs() < 1e-6,
                "a chip of plate went see-through"
            );
        }

        let rock = shard_gradient(CarveDebris::Rock);
        for key in rock.keys() {
            assert_eq!(key.value, ROCK_COLD, "rock changed colour while flying");
            assert!(key.value.max_element() <= 1.0, "rock glows when it is hit");
        }
    }

    /// An unmarked body is plate, so every ship that existed before
    /// [`CarveDebris`] did keeps throwing exactly what it threw.
    #[test]
    fn a_body_that_says_nothing_throws_plate() {
        assert_eq!(CarveDebris::default(), CarveDebris::Metal);
        assert_eq!(CarveDebris::Metal.shape(KINETIC_SHARDS), KINETIC_SHARDS);
        assert!((CarveDebris::Metal.speed_scale() - 1.0).abs() < f32::EPSILON);
    }

    /// Rock spalls in greater number and leaves slower, which is what makes it
    /// read as heavy rather than as a hull painted brown.
    #[test]
    fn rock_spalls_more_and_slower_than_plate() {
        let plate = CarveDebris::Metal.shape(KINETIC_SHARDS);
        let rock = CarveDebris::Rock.shape(KINETIC_SHARDS);
        assert!(rock.count(0.6) > plate.count(0.6));
        assert!(rock.count(100.0) > plate.count(100.0), "and at the ceiling");
        assert!(rock.size < plate.size);
        assert!(CarveDebris::Rock.speed_scale() < CarveDebris::Metal.speed_scale());

        let mut plate_app = spew_app();
        carve_body(&mut plate_app, DamageType::Kinetic, 0.6, None);
        let mut rock_app = spew_app();
        carve_body(
            &mut rock_app,
            DamageType::Kinetic,
            0.6,
            Some(CarveDebris::Rock),
        );
        assert!(thrown(&rock_app) > thrown(&plate_app));
        assert!(aimed(&mut rock_app)[0].chip < aimed(&mut plate_app)[0].chip);
    }

    /// The split a flat lookup on the spewing entity cannot see, and the one
    /// every asteroid in the game is shaped like: the material is declared on
    /// the ROOT, while carves are announced against the mesh node beneath it -
    /// the node is where the damage marks ride, and where a severed crumb hangs
    /// off. Read flat, every crater on a rock threw hot gunmetal.
    #[test]
    fn a_carve_announced_from_a_child_of_a_rock_still_throws_rock() {
        let mut rock = spew_app();
        let root = rock.world_mut().spawn(CarveDebris::Rock).id();
        let node = rock
            .world_mut()
            .spawn((ChildOf(root), GlobalTransform::IDENTITY))
            .id();
        rock.world_mut().trigger(CarveSpew {
            entity: node,
            at: Vec3::X * 3.0,
            radius: 0.6,
            kind: DamageType::Kinetic,
        });
        rock.update();

        let mut plate = spew_app();
        carve_body(&mut plate, DamageType::Kinetic, 0.6, None);
        assert!(
            thrown(&rock) > thrown(&plate),
            "the crater on the rock threw a plate's chip count",
        );
        let effects = effects(&mut rock);
        assert_eq!(effects.len(), 1);
        assert_eq!(
            effect_name(&rock, &effects[0]),
            "carve_shards_rock",
            "shooting the asteroid sprayed hot gunmetal off it"
        );
    }

    /// An emitter a scene tore down under the pool is replaced in its slot
    /// rather than aimed at, so a carve after a teardown still chips.
    #[test]
    fn an_emitter_torn_down_under_the_pool_is_replaced() {
        let mut app = spew_app();
        carve_body(&mut app, DamageType::Kinetic, 0.6, None);
        let gone = emitters(&mut app);
        assert_eq!(gone.len(), 1);
        app.world_mut().despawn(gone[0]);
        app.update();

        let before = thrown(&app);
        carve_body(&mut app, DamageType::Kinetic, 0.6, None);
        assert!(
            thrown(&app) > before,
            "the carve after the teardown went unchipped"
        );
        let fresh = emitters(&mut app);
        assert_eq!(
            fresh.len(),
            1,
            "the pool leaked a slot or grew a second one"
        );
        assert_ne!(fresh[0], gone[0]);
        assert_eq!(aimed(&mut app).len(), 1);
    }
}
