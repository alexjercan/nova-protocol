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
//! afterwards, so grey cubes on top of the fire add nothing and read as litter.
//! Nor does a big hit become feedback-less: a warhead that SEVERS a body throws
//! real severed geometry, meshed off the carve field in the body's own
//! material, which is a different effect with a different meaning
//! ([`chunk`](super::chunk)).
//!
//! Engine units throughout: chip sizes, crater radii and throw speeds are
//! world units (one is 10 m) and world units per second, because they are
//! measured against carve-field geometry and avian velocities.
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
//! # The body says what it is made of
//!
//! A rock and a hull announce a carve identically, so for as long as one look
//! was shared an asteroid threw gunmetal chips. [`CarveDebris`] sits on the
//! BODY and this observer reads it, which is the only arrangement that does not
//! force `integrity` to enumerate the kinds of body the layers above it will
//! invent. A body that says nothing is plate, so every ship stayed correct
//! without being touched.
//!
//! Metal is also HOT. A chip is cut, not picked up, so it leaves near-white and
//! cools to gunmetal in under a second - which is the difference between debris
//! and litter, and what the cold grey cube got wrong.
//!
//! # Why shards are not physical debris
//!
//! Shards are `Kinematic` and carry NO collider, exactly as damage sparks do,
//! and for a stronger reason. They are born INSIDE the body they came off - a
//! crater's shards start in the hull's own collider - so a dynamic body with a
//! collider would spawn interpenetrating and the solver would resolve that by
//! shoving the two apart. A ship would kick itself sideways every time it was
//! shot, which is a physics bug wearing a costume.
//!
//! A chunk has the same problem and solves it differently: it starts kinematic
//! and grows its collider once it has drifted clear. It can afford to, because
//! it is meant to still be there when it lands.

use avian3d::prelude::{AngularVelocity, LinearVelocity, RigidBody};
use bevy::{platform::collections::HashMap, prelude::*};

use super::carve::prelude::CarveSpew;
use crate::{damage::prelude::DamageType, prelude::TempEntity};

/// `CarveDebris`, `CarveShardMarker` and `CarveSpewPlugin`.
pub mod prelude {
    pub use super::{CarveDebris, CarveShardMarker, CarveSpewPlugin};
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
    /// cube thrown at 2 to 6.5 u/s and gone in 2.5 seconds. Nothing reads that
    /// difference, and the curve's other end did real harm: a ram or a scripted
    /// mega-hit carves several units, and a fifth of that is a cube the size of
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

/// How fast a shard tumbles, in radians per second.
const SPEW_SPIN: f32 = 7.0;

/// How long a shard lives. Long enough to be seen leaving and to sell the
/// direction, short enough that a long fight leaves no litter.
const SHARD_LIFETIME_SECS: f32 = 2.5;

/// Marks a shard thrown by a carve, so a range can count them.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct CarveShardMarker;

/// The one mesh and one material every shard is drawn with.
///
/// Built once and shared, for the reason the spark assets are: a firefight
/// carves several craters a second across a formation, and minting a mesh and
/// material per shard would pile up assets for the rest of the scenario, none
/// of them ever freed. Shards vary by SCALE on their transform instead.
///
/// Built on the FIRST CARVE rather than at plugin build, and that is not
/// laziness for its own sake: this plugin ships inside `NovaIntegrityPlugin`,
/// which a headless test app adds without any asset stores at all, and an
/// `init_resource` here would panic every one of them before a single carve
/// happened.
struct ShardAssets {
    mesh: Handle<Mesh>,
    /// The cooling ramp, hottest first. One entry means a material that never
    /// glowed and so never cools.
    ramp: Vec<Handle<StandardMaterial>>,
}

/// The shard assets for every material seen so far, minted on first use.
///
/// A MAP and not a pair of fields, so a third material is one arm in
/// [`shard_assets`] rather than an edit here as well - the same shape
/// `DefaultTorpedoRender` uses to key warhead materials by tint.
#[derive(Resource, Default)]
struct DebrisLooks(HashMap<CarveDebris, ShardAssets>);

/// Seconds a hot chip takes to reach the cold end of its ramp.
///
/// Under a third of [`SHARD_LIFETIME_SECS`]. The glow is what says the chip was
/// just CUT, and a chip still glowing when it drifts out of frame reads as a
/// firefly instead.
const SHARD_COOL_SECS: f32 = 0.8;

/// How many materials the cooling ramp is cut into.
///
/// Discrete because the material is SHARED - a per-shard material would mint an
/// asset per chip and never free it - so cooling is a swap between a handful of
/// shared handles rather than a value animated per entity. Five is where the
/// steps stop being visible as pops at [`SHARD_COOL_SECS`].
const SHARD_COOL_STEPS: usize = 5;

/// How hot a chip is at the instant it is cut. Well past 1.0 so it blooms.
const SHARD_HOT_EMISSIVE: LinearRgba = LinearRgba::new(7.0, 3.2, 0.9, 1.0);

/// Mint the shared shard assets for one material.
fn shard_assets(
    debris: CarveDebris,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> ShardAssets {
    // A unit cube, scaled per shard. Not a sphere: a chip off a hull is flat
    // and angular, and the flat-shaded look the rest of the game is built on
    // has no way to draw a small sphere that does not read as a ball bearing.
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    let ramp = match debris {
        CarveDebris::Metal => (0..SHARD_COOL_STEPS)
            .map(|step| {
                // Squared, so most of the glow is gone in the first step or
                // two: metal loses heat fastest when it is hottest, and a
                // linear ramp reads as a chip being dimmed by a knob.
                let remaining = 1.0 - step as f32 / (SHARD_COOL_STEPS - 1) as f32;
                let heat = remaining * remaining;
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.30, 0.30, 0.33),
                    // Channel by channel, and NOT `SHARD_HOT_EMISSIVE * heat`:
                    // scaling the whole colour scales its alpha too, and the
                    // cold end of the ramp would be a transparent black rather
                    // than the ordinary opaque black a material that emits
                    // nothing carries.
                    emissive: LinearRgba::new(
                        SHARD_HOT_EMISSIVE.red * heat,
                        SHARD_HOT_EMISSIVE.green * heat,
                        SHARD_HOT_EMISSIVE.blue * heat,
                        1.0,
                    ),
                    perceptual_roughness: 0.9,
                    metallic: 0.3,
                    ..default()
                })
            })
            .collect(),
        CarveDebris::Rock => vec![materials.add(StandardMaterial {
            base_color: Color::srgb(0.34, 0.29, 0.25),
            perceptual_roughness: 1.0,
            metallic: 0.0,
            ..default()
        })],
    };

    ShardAssets { mesh, ramp }
}

/// A chip still losing its cutting heat. Absent on a material that never
/// glowed, so [`cool_carve_shards`] does no work for rock.
#[derive(Component, Clone, Copy, Debug)]
struct ShardCooling {
    debris: CarveDebris,
    age: f32,
    step: usize,
}

/// Walk every hot chip down its ramp, swapping to the next shared material as
/// it cools.
fn cool_carve_shards(
    time: Res<Time>,
    looks: Res<DebrisLooks>,
    mut q_shard: Query<(&mut ShardCooling, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let delta = time.delta_secs();
    for (mut cooling, mut material) in &mut q_shard {
        cooling.age += delta;
        let Some(assets) = looks.0.get(&cooling.debris) else {
            continue;
        };
        let last = assets.ramp.len().saturating_sub(1);
        if last == 0 {
            continue;
        }
        let t = (cooling.age / SHARD_COOL_SECS).clamp(0.0, 1.0);
        let step = shard_cool_step(t, assets.ramp.len());
        if step != cooling.step {
            cooling.step = step;
            material.0 = assets.ramp[step].clone();
        }
    }
}

/// Which ramp entry a chip `t` of the way through its cooling is on.
///
/// Pure, so the ramp can be read without a running app.
fn shard_cool_step(t: f32, steps: usize) -> usize {
    let last = steps.saturating_sub(1);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "t is clamped to 0..=1, so the product is inside `last`"
    )]
    let step = (t.clamp(0.0, 1.0) * last as f32).round() as usize;
    step.min(last)
}

/// Which way the `nth` shard off a crater at `at` is thrown.
///
/// DETERMINISTIC, like the spark spread and for the same reasons: the same
/// fight throws the same debris twice, which is what a re-run capture and a
/// replay both want, and it keeps this off the global RNG.
///
/// Spread in a cone about `outward` rather than over a sphere. Material knocked
/// off a surface goes mostly the way the hit pushed it, and the outward
/// direction is also the only one that does not fire shards back through the
/// body they came off.
fn shard_throw(outward: Vec3, at: Vec3, nth: usize) -> (Vec3, f32) {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in
        at.x.to_bits()
            .to_le_bytes()
            .iter()
            .chain(at.y.to_bits().to_le_bytes().iter())
            .chain(at.z.to_bits().to_le_bytes().iter())
            .chain((nth as u32).to_le_bytes().iter())
    {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }

    // Three independent fractions out of the one hash: the turn about the
    // outward axis, how far off it to lean, and how hard to throw.
    let turn = (hash >> 20) as f32 / 4096.0 * std::f32::consts::TAU;
    let lean = ((hash >> 8) & 0xfff) as f32 / 4096.0 * SPEW_CONE;
    let speed = SPEW_SPEED_MIN + (hash & 0xff) as f32 / 256.0 * (SPEW_SPEED_MAX - SPEW_SPEED_MIN);

    // Any axis not parallel to `outward` builds the basis; `any_orthonormal_pair`
    // picks one without a degenerate case to guard.
    let (right, up) = outward.any_orthonormal_pair();
    let direction = (outward * lean.cos() + (right * turn.cos() + up * turn.sin()) * lean.sin())
        .normalize_or(outward);
    (direction, speed)
}

/// Throws the chips an impact knocked off.
pub struct CarveSpewPlugin;

impl Plugin for CarveSpewPlugin {
    fn build(&self, app: &mut App) {
        trace!("CarveSpewPlugin: build");

        app.register_type::<CarveShardMarker>();
        app.register_type::<CarveDebris>();
        app.init_resource::<DebrisLooks>();
        app.add_observer(spew_carved_material);
        app.add_systems(Update, cool_carve_shards);
    }
}

fn spew_carved_material(
    spew: On<CarveSpew>,
    mut commands: Commands,
    mut looks: ResMut<DebrisLooks>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    q_body: Query<&GlobalTransform>,
    q_debris: Query<&CarveDebris>,
) {
    // The class decides first, so a warhead costs nothing at all here.
    let Some(look) = shard_look(spew.kind) else {
        return;
    };

    // A world with no asset stores has nothing to draw with and nothing that
    // could see the result: a headless server, or a test app that added the
    // integrity plugin for its health pipeline alone. Carving still happens
    // there, it just goes unseen.
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    // OUT of the body, which for a crater means away from the body's own
    // origin. A hit dead on the centre has no outward direction to speak of, so
    // it falls back to up rather than to a zero vector nothing can be built on.
    let outward = q_body.get(spew.entity).map_or(Vec3::Y, |frame| {
        (spew.at - frame.translation()).normalize_or(Vec3::Y)
    });

    // The body decides what it is made of; anything silent is plate.
    let debris = q_debris.get(spew.entity).copied().unwrap_or_default();
    let look = debris.shape(look);

    let assets = looks
        .0
        .entry(debris)
        .or_insert_with(|| shard_assets(debris, &mut meshes, &mut materials));
    let mesh = assets.mesh.clone();
    let material = assets.ramp[0].clone();
    let cools = assets.ramp.len() > 1;

    let count = look.count(spew.radius);
    trace!(
        "spew_carved_material: {count} {debris:?} shard(s) off {:?} at {} ({:?})",
        spew.entity,
        spew.at,
        spew.kind
    );

    for nth in 0..count {
        let (direction, speed) = shard_throw(outward, spew.at, nth);
        let speed = speed * debris.speed_scale();
        let mut shard = commands.spawn((
            Name::new("Carve Shard"),
            CarveShardMarker,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            // Started at the crater's LIP rather than its centre, so a shard is
            // not drawn inside the material it supposedly just left.
            Transform::from_translation(spew.at + direction * spew.radius * 0.5)
                .with_scale(Vec3::splat(look.size)),
            // Kinematic and WITHOUT a collider - see the module docs. It is
            // born inside the hull it came off, and a dynamic collider there
            // would shove the ship every time it was shot.
            RigidBody::Kinematic,
            LinearVelocity(direction * speed),
            AngularVelocity(direction.any_orthogonal_vector() * SPEW_SPIN),
            TempEntity(SHARD_LIFETIME_SECS),
        ));
        // Only a material with a ramp carries the cooling: rock never glowed,
        // so it costs no component and the cooling system skips it entirely
        // rather than iterating it to do nothing.
        if cools {
            shard.insert(ShardCooling {
                debris,
                age: 0.0,
                step: 0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::{
        carve::prelude::{DamageMark, DamageMarks},
        chunk::prelude::{CarvedChunkMarker, CHUNK_MIN_VOLUME},
    };

    /// Every class that throws anything, so a new damage type cannot be added
    /// without a test noticing what it does.
    const THROWING: [(DamageType, ShardLook); 2] = [
        (DamageType::Kinetic, KINETIC_SHARDS),
        (DamageType::Pierce, PIERCE_SHARDS),
    ];

    fn spew_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_plugins(CarveSpewPlugin);
        app
    }

    fn shards(app: &mut App) -> Vec<(Vec3, Vec3)> {
        app.world_mut()
            .query_filtered::<(&Transform, &LinearVelocity), With<CarveShardMarker>>()
            .iter(app.world())
            .map(|(transform, velocity)| (transform.translation, velocity.0))
            .collect()
    }

    /// Throw one crater of `radius` off a body at the origin and report the
    /// shards it left.
    fn carve(app: &mut App, kind: DamageType, radius: f32) -> Vec<(Vec3, Vec3)> {
        carve_body(app, kind, radius, None);
        shards(app)
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

    /// Every shard's material handle.
    fn shard_materials(app: &mut App) -> Vec<Handle<StandardMaterial>> {
        app.world_mut()
            .query_filtered::<&MeshMaterial3d<StandardMaterial>, With<CarveShardMarker>>()
            .iter(app.world())
            .map(|material| material.0.clone())
            .collect()
    }

    /// THE ruling this module now implements: chips are an IMPACT effect. A
    /// bullet of either type chips what it hits; a warhead's own fireball is
    /// the cue for a blast, so it throws nothing on top of it.
    #[test]
    fn a_bullet_chips_what_it_hits_and_a_warhead_does_not() {
        for (kind, _) in THROWING {
            let mut app = spew_app();
            assert!(
                !carve(&mut app, kind, 0.6).is_empty(),
                "{kind:?} threw nothing off a bullet-sized crater"
            );
        }

        let mut app = spew_app();
        assert!(
            carve(&mut app, DamageType::Explosive, 3.0).is_empty(),
            "a blast littered its own fireball with chips"
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
                    !carve(&mut app, kind, radius).is_empty(),
                    "{kind:?} at {radius} threw nothing"
                );
                let scales: Vec<f32> = app
                    .world_mut()
                    .query_filtered::<&Transform, With<CarveShardMarker>>()
                    .iter(app.world())
                    .map(|transform| transform.scale.x)
                    .collect();
                for scale in scales {
                    assert!(
                        (scale - look.size).abs() < 1e-6,
                        "{kind:?} at radius {radius} drew a {scale}u chip"
                    );
                }
            }
        }
    }

    /// The line between decoration and material. A shard has no collider and no
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

    /// THE claim: material comes off, and it comes off OUTWARD. A shard thrown
    /// back through the body it left would read as the hull swallowing its own
    /// wreckage.
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

        let thrown = shards(&mut app);
        assert!(!thrown.is_empty(), "a carve throws something");
        for (position, velocity) in thrown {
            assert!(
                velocity.dot(Vec3::X) > 0.0,
                "a shard flew back into the hull: {velocity}"
            );
            assert!(
                position.x >= at.x,
                "a shard started inside the material it left: {position}"
            );
        }
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
            assert!(!thrown.is_empty(), "radius {radius} threw no dust");
        }
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

    /// Shards are debris, not litter: they clear themselves, so a long fight
    /// cannot leave a cloud of them hanging over the field.
    #[test]
    fn every_shard_clears_itself() {
        let mut app = spew_app();
        let rock = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        app.world_mut().trigger(CarveSpew {
            entity: rock,
            at: Vec3::Y * 2.0,
            radius: 0.8,
            kind: DamageType::Kinetic,
        });
        app.update();

        let mut q = app
            .world_mut()
            .query_filtered::<Option<&TempEntity>, With<CarveShardMarker>>();
        let lifetimes: Vec<_> = q.iter(app.world()).collect();
        assert!(!lifetimes.is_empty(), "delivery guard: it spewed");
        for temp in lifetimes {
            assert!(temp.is_some(), "a shard must despawn itself");
        }
    }

    /// The same hit throws the same debris twice, which is what a re-run
    /// capture and a replay both want.
    #[test]
    fn the_same_crater_throws_the_same_shards() {
        let once = shard_throw(Vec3::X, Vec3::new(1.0, 2.0, 3.0), 2);
        let again = shard_throw(Vec3::X, Vec3::new(1.0, 2.0, 3.0), 2);
        assert_eq!(once, again);

        let elsewhere = shard_throw(Vec3::X, Vec3::new(1.0, 2.0, 3.5), 2);
        assert_ne!(
            once, elsewhere,
            "two different craters must not throw identically"
        );
    }

    /// The ruling the owner took: a rock must not throw the hull's chips. Both
    /// bodies take the identical carve, and the only thing that differs is what
    /// the body said it was made of.
    #[test]
    fn a_rock_and_a_hull_throw_different_debris() {
        let mut plate = spew_app();
        carve_body(&mut plate, DamageType::Kinetic, 0.6, None);
        let plate_material = shard_materials(&mut plate);

        let mut rock = spew_app();
        carve_body(&mut rock, DamageType::Kinetic, 0.6, Some(CarveDebris::Rock));
        let rock_material = shard_materials(&mut rock);

        assert!(!plate_material.is_empty() && !rock_material.is_empty());
        let plate_asset = plate.world().resource::<Assets<StandardMaterial>>();
        let rock_asset = rock.world().resource::<Assets<StandardMaterial>>();
        let plate_look = plate_asset.get(&plate_material[0]).expect("plate look");
        let rock_look = rock_asset.get(&rock_material[0]).expect("rock look");
        assert_ne!(plate_look.base_color, rock_look.base_color);
        assert!(
            plate_look.emissive.red > 0.0,
            "a freshly cut chip of plate is incandescent"
        );
        assert_eq!(
            rock_look.emissive,
            LinearRgba::BLACK,
            "rock does not glow when it is hit"
        );
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
        assert!(shards(&mut rock_app).len() > shards(&mut plate_app).len());
    }

    /// Only a material with a ramp carries the cooling component, so rock costs
    /// the cooling system nothing at all.
    #[test]
    fn only_a_hot_chip_carries_its_cooling() {
        let mut plate = spew_app();
        carve_body(&mut plate, DamageType::Kinetic, 0.6, None);
        let hot = plate
            .world_mut()
            .query_filtered::<(), (With<CarveShardMarker>, With<ShardCooling>)>()
            .iter(plate.world())
            .count();
        assert!(hot > 0, "a chip of plate cools");

        let mut rock = spew_app();
        carve_body(&mut rock, DamageType::Kinetic, 0.6, Some(CarveDebris::Rock));
        let cold = rock
            .world_mut()
            .query_filtered::<(), (With<CarveShardMarker>, With<ShardCooling>)>()
            .iter(rock.world())
            .count();
        assert_eq!(cold, 0, "rock never glowed, so it has nothing to cool");
    }

    /// The ramp starts hot, ends cold, and clamps at both ends.
    #[test]
    fn the_cooling_ramp_runs_hot_to_cold_and_clamps() {
        assert_eq!(shard_cool_step(0.0, 5), 0);
        assert_eq!(shard_cool_step(1.0, 5), 4);
        assert_eq!(shard_cool_step(-1.0, 5), 0);
        assert_eq!(shard_cool_step(2.0, 5), 4);
        assert!(shard_cool_step(0.25, 5) < shard_cool_step(0.75, 5));
        // A one-entry ramp has nowhere to go.
        assert_eq!(shard_cool_step(1.0, 1), 0);
    }

    /// A hot chip actually swaps material as it flies.
    ///
    /// The clock is DRIVEN, not read: `MinimalPlugins` runs `Time` off the real
    /// wall clock, so forty updates of a test binary advance it by microseconds
    /// and nothing would ever cool. A warm-up tick goes first because bevy's
    /// first manual step is dt 0.
    #[test]
    fn a_hot_chip_swaps_to_a_cooler_material_as_it_flies() {
        let mut app = spew_app();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(50),
        ));
        app.update();

        carve_body(&mut app, DamageType::Kinetic, 0.6, None);
        let at_birth = shard_materials(&mut app);
        assert!(!at_birth.is_empty());

        // Past the far end of the ramp, and still well inside the shard's own
        // lifetime so there is something left to read.
        for _ in 0..25 {
            app.update();
        }

        let cooled = shard_materials(&mut app);
        assert!(!cooled.is_empty(), "the shard outlives its cooling");
        assert_ne!(at_birth[0], cooled[0], "the chip is on a cooler material");
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(
            materials.get(&cooled[0]).expect("cold look").emissive,
            LinearRgba::BLACK,
            "the far end of the ramp is not glowing at all"
        );
    }
}
