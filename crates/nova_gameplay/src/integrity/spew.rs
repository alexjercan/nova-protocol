//! The material a carve took off, seen leaving.
//!
//! A crater is a hole where solid used to be, and solid does not vanish. Without
//! this a hull dents in perfect silence: the geometry changes between one frame
//! and the next, which reads as a rendering glitch rather than as a hit. The
//! shards are what make the change legible - something came off, and it went
//! that way.
//!
//! Sized off the crater, so the read is honest: a bullet chips two flakes, a
//! warhead throws a handful of slabs. Nothing here decides how much material
//! was lost - [`mark_radius`](super::carve::mark_radius) already priced that
//! out of the damage.
//!
//! # Dust, or a piece
//!
//! One decision, made here, on the VOLUME the carve announces: below
//! [`CHUNK_MIN_VOLUME`] the material is pulverised and comes off as shards,
//! above it there is enough of it to be worth simulating and it comes off as a
//! body ([`chunk`](super::chunk)). A PDC round throws dust; a torpedo throws a
//! piece of hull.
//!
//! Both answers arrive through the same event, and that is deliberate: a carve
//! announces what it took and nothing about how it should look, so a mod that
//! wants different thresholds - or no debris at all - replaces this observer
//! rather than patching the carve.
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
use bevy::prelude::*;

use super::{
    carve::prelude::CarveSpew,
    chunk::prelude::{chunk_collider, spawn_carved_chunk, ChunkSpawn, CHUNK_MIN_VOLUME},
    explode::prelude::FragmentMaterial,
};
use crate::{mesh::builder::TriangleMeshBuilder, prelude::TempEntity};

/// `CarveSpewPlugin`.
pub mod prelude {
    pub use super::CarveSpewPlugin;
}

/// The fewest shards a carve throws, however small the crater.
///
/// Two and not one: a single chip reads as a stray particle, a pair reads as
/// something breaking.
const SPEW_MIN: usize = 2;
/// The most a carve throws, however big the crater. A warhead should look
/// expensive, not fill the screen with litter.
const SPEW_MAX: usize = 7;
/// How many shards one unit of crater radius is worth, between those bounds.
const SPEW_PER_UNIT_RADIUS: f32 = 4.0;

/// How big a shard is next to the crater it came out of.
///
/// Well under it: the material a hemisphere's worth of carving removes is
/// spread across the whole crater floor, so shards that each looked a quarter
/// of the hole wide would read as the rock coming apart rather than as chips
/// off it.
const SHARD_OF_CRATER: f32 = 0.22;
/// The smallest shard drawn, in world units. Below this it is a spark, and
/// sparks are a different effect with a different meaning.
const SHARD_MIN_SIZE: f32 = 0.05;

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

/// How big a thrown piece is next to the crater it came out of.
///
/// Bigger than a shard and still well under the crater. The material a carve
/// removes does not come off as one lump the size of the hole - most of it is
/// pulverised - so a piece that looked half the crater wide would read as the
/// body calving rather than as debris off a hit.
const EJECTA_OF_CRATER: f32 = 0.3;

/// The most real pieces one carve throws, however big it was.
///
/// Fewer than the dust it replaces, and for a different reason: every piece is
/// a rigid body the solver carries for half a minute, so this is a budget and
/// not a look. A carve throws two or three of them and the eye reads rubble.
const EJECTA_MAX: usize = 3;

/// How fast a thrown piece tumbles, in radians per second.
const EJECTA_SPIN: f32 = 2.5;

/// How far from round a thrown piece is drawn.
///
/// A lump scaled evenly is a die, however many facets it has. Squashing each
/// piece differently on its own axes is what makes a handful of them read as
/// broken rock rather than as a set of matching props.
const EJECTA_SQUASH: f32 = 0.45;

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
type ShardAssets = (Handle<Mesh>, Handle<StandardMaterial>);

/// Mint the shared shard assets.
fn shard_assets(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> ShardAssets {
    // A unit cube, scaled per shard. Not a sphere: a chip off a hull is flat
    // and angular, and the flat-shaded look the rest of the game is built on
    // has no way to draw a small sphere that does not read as a ball bearing.
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.30, 0.33),
        perceptual_roughness: 0.9,
        metallic: 0.3,
        ..default()
    });
    (mesh, material)
}

/// How many shards a crater of `radius` throws.
///
/// Pure, so the curve can be read without a running app, and clamped at both
/// ends: the floor is what stops a graze reading as a single stray particle,
/// the ceiling is what stops a warhead filling the screen with litter.
pub fn shard_count(radius: f32) -> usize {
    ((radius * SPEW_PER_UNIT_RADIUS).round() as usize).clamp(SPEW_MIN, SPEW_MAX)
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

/// The one mesh and one hull every thrown lump is built from.
///
/// Shared for the reason the shard assets are, and scaled per lump on its own
/// transform. An octahedron rather than a cube: the game is flat-shaded facets
/// and a lump of rock or hull plating is angular but not square.
type LumpAssets = (Handle<Mesh>, avian3d::prelude::Collider);

/// Mint the shared lump mesh and its hull.
fn lump_assets(meshes: &mut Assets<Mesh>) -> LumpAssets {
    let mesh = TriangleMeshBuilder::new_octahedron(1).build();
    // Hulled ONCE, off the unit lump, then scaled by each piece's transform.
    // The unit octahedron has volume, so this never hits the degenerate case
    // `chunk_collider` guards - but it goes through the same function anyway,
    // because there should be one answer to "what collider does a piece get".
    let collider = chunk_collider(&mesh).expect("a unit octahedron has bounds");
    (meshes.add(mesh), collider)
}

/// How many real pieces a crater of `radius` throws.
///
/// The dust curve, capped harder: pieces cost a solver step each for as long as
/// they live, so past a handful the count is a budget question rather than a
/// look one.
pub fn ejecta_count(radius: f32) -> usize {
    shard_count(radius).min(EJECTA_MAX)
}

/// Throw real pieces of the body out of the crater.
///
/// Aimed the same way the dust is - the same deterministic cone, so the same
/// hit throws the same rubble twice - and sized off the crater rather than off
/// the exact volume: `radius` is already the cube root of what came off, so a
/// torpedo's rubble is visibly bigger than a cannon shell's without a second
/// number saying so.
fn throw_ejected_pieces(
    commands: &mut Commands,
    spew: &CarveSpew,
    outward: Vec3,
    (mesh, collider): LumpAssets,
    material: Handle<StandardMaterial>,
) {
    let size = spew.radius * EJECTA_OF_CRATER;
    let count = ejecta_count(spew.radius);
    trace!(
        "spew_carved_material: {count} piece(s) of {size:.2} units off {:?} at {}",
        spew.entity,
        spew.at
    );

    for nth in 0..count {
        let (direction, speed) = shard_throw(outward, spew.at, nth);
        let piece = spawn_carved_chunk(
            commands,
            ChunkSpawn {
                name: "Carve Ejecta".to_string(),
                mesh: mesh.clone(),
                // Started at the crater's LIP, so a piece is not drawn inside
                // the material it just left. Turned to face the way it is
                // going, so a handful of them are not all the same die.
                transform: Transform::from_translation(spew.at + direction * spew.radius * 0.5)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, direction))
                    .with_scale(squashed(size, spew.at, nth)),
                velocity: direction * speed,
                spin: direction.any_orthogonal_vector() * EJECTA_SPIN,
                collider: collider.clone(),
            },
        );
        commands
            .entity(piece)
            .insert(MeshMaterial3d(material.clone()));
    }
}

/// A lump of `size`, squashed differently on each axis.
///
/// Deterministic on the crater and the piece's index, like everything else a
/// carve throws, so the same hit produces the same rubble on a re-run.
fn squashed(size: f32, at: Vec3, nth: usize) -> Vec3 {
    let mut hash: u32 = 0x811c_9dc5 ^ (nth as u32).wrapping_mul(0x9e37_79b9);
    for value in [at.x, at.y, at.z] {
        hash ^= value.to_bits();
        hash = hash.wrapping_mul(0x0100_0193);
    }
    let mut axis = || {
        hash = hash.wrapping_mul(0x0100_0193);
        hash ^= hash >> 15;
        size * (1.0 - EJECTA_SQUASH * (hash >> 16) as f32 / 65_536.0)
    };
    Vec3::new(axis(), axis(), axis())
}

/// Throws what a carve took off: dust below [`CHUNK_MIN_VOLUME`], a real piece
/// above it.
pub struct CarveSpewPlugin;

impl Plugin for CarveSpewPlugin {
    fn build(&self, app: &mut App) {
        debug!("CarveSpewPlugin: build");

        app.register_type::<CarveShardMarker>();
        app.add_observer(spew_carved_material);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the shared assets are minted lazily, so their stores ride along"
)]
fn spew_carved_material(
    spew: On<CarveSpew>,
    mut commands: Commands,
    mut cached: Local<Option<ShardAssets>>,
    mut cached_lump: Local<Option<LumpAssets>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    q_body: Query<(&GlobalTransform, Option<&FragmentMaterial>)>,
) {
    // A world with no asset stores has nothing to draw with and nothing that
    // could see the result: a headless server, or a test app that added the
    // integrity plugin for its health pipeline alone. Carving still happens
    // there, it just goes unseen.
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    let body = q_body.get(spew.entity).ok();
    // OUT of the body, which for a crater means away from the body's own
    // origin. A hit dead on the centre has no outward direction to speak of, so
    // it falls back to up rather than to a zero vector nothing can be built on.
    let outward = body
        .map(|(frame, _)| (spew.at - frame.translation()).normalize_or(Vec3::Y))
        .unwrap_or(Vec3::Y);

    if spew.volume >= CHUNK_MIN_VOLUME {
        let (mesh, collider) = cached_lump
            .get_or_insert_with(|| lump_assets(&mut meshes))
            .clone();
        let grey = cached
            .get_or_insert_with(|| shard_assets(&mut meshes, &mut materials))
            .1
            .clone();
        throw_ejected_pieces(
            &mut commands,
            &spew,
            outward,
            (mesh, collider),
            body.and_then(|(_, material)| material)
                .map_or(grey, |material| material.0.clone()),
        );
        return;
    }

    let (mesh, material) = cached
        .get_or_insert_with(|| shard_assets(&mut meshes, &mut materials))
        .clone();

    let size = (spew.radius * SHARD_OF_CRATER).max(SHARD_MIN_SIZE);
    let count = shard_count(spew.radius);
    trace!(
        "spew_carved_material: {count} shard(s) off {:?} at {}",
        spew.entity,
        spew.at
    );

    for nth in 0..count {
        let (direction, speed) = shard_throw(outward, spew.at, nth);
        commands.spawn((
            Name::new("Carve Shard"),
            CarveShardMarker,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            // Started at the crater's LIP rather than its centre, so a shard is
            // not drawn inside the material it supposedly just left.
            Transform::from_translation(spew.at + direction * spew.radius * 0.5)
                .with_scale(Vec3::splat(size)),
            // Kinematic and WITHOUT a collider - see the module docs. It is
            // born inside the hull it came off, and a dynamic collider there
            // would shove the ship every time it was shot.
            RigidBody::Kinematic,
            LinearVelocity(direction * speed),
            AngularVelocity(direction.any_orthogonal_vector() * SPEW_SPIN),
            TempEntity(SHARD_LIFETIME_SECS),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::{
        carve::prelude::{DamageMark, DamageMarks},
        chunk::prelude::CarvedChunkMarker,
    };

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

    /// A bigger crater throws more, and neither end runs away: one chip reads
    /// as a stray particle, fifty read as litter.
    #[test]
    fn a_bigger_crater_throws_more_shards_but_never_a_swarm() {
        assert_eq!(shard_count(0.0), SPEW_MIN);
        assert!(shard_count(0.5) >= SPEW_MIN);
        assert!(shard_count(0.5) <= shard_count(1.5));
        assert_eq!(shard_count(100.0), SPEW_MAX);
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
            volume: CHUNK_MIN_VOLUME * 0.5,
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

    /// THE line this module draws. A carve that took a hull plate's worth of
    /// material has to leave something a ship can fly into; a carve that took a
    /// PDC round's worth has to leave dust and nothing that has to be
    /// simulated, or a long firefight would fill the field with rubble.
    #[test]
    fn a_big_bite_comes_off_as_a_body_and_a_small_one_as_dust() {
        for (volume, chunks, shards_expected) in [
            (CHUNK_MIN_VOLUME * 0.5, 0, true),
            (CHUNK_MIN_VOLUME * 4.0, ejecta_count(1.0), false),
        ] {
            let mut app = spew_app();
            let rock = app
                .world_mut()
                .spawn(GlobalTransform::from_translation(Vec3::ZERO))
                .id();
            app.world_mut().trigger(CarveSpew {
                entity: rock,
                at: Vec3::X * 3.0,
                radius: 1.0,
                volume,
            });
            app.update();

            let thrown = app
                .world_mut()
                .query_filtered::<(), With<CarvedChunkMarker>>()
                .iter(app.world())
                .count();
            assert_eq!(thrown, chunks, "volume {volume} threw {thrown} piece(s)");
            assert_eq!(
                !shards(&mut app).is_empty(),
                shards_expected,
                "volume {volume} threw the wrong kind of debris"
            );
        }
    }

    /// Real pieces are budgeted harder than dust: each one is a rigid body the
    /// solver carries for half a minute, so a warhead must not be able to throw
    /// as many of them as it throws chips.
    #[test]
    fn a_carve_throws_fewer_real_pieces_than_chips() {
        for radius in [0.1f32, 0.5, 1.0, 3.0, 100.0] {
            assert!(ejecta_count(radius) <= shard_count(radius));
            assert!(ejecta_count(radius) <= EJECTA_MAX, "radius {radius}");
        }
        assert!(ejecta_count(3.0) > 0, "a big carve still throws something");
    }

    /// Rubble is rubble: no two pieces off one crater are the same shape, and
    /// the same crater throws the same rubble twice.
    #[test]
    fn thrown_pieces_are_broken_rock_and_not_matching_props() {
        let at = Vec3::new(1.0, 2.0, 3.0);
        let first = squashed(1.0, at, 0);
        let second = squashed(1.0, at, 1);

        assert_ne!(first, second, "two pieces off one crater are not clones");
        assert_eq!(first, squashed(1.0, at, 0), "and a re-run repeats them");
        for piece in [first, second] {
            assert!(
                piece.max_element() <= 1.0 && piece.min_element() >= 1.0 - EJECTA_SQUASH,
                "a piece is squashed, not resized: {piece}"
            );
        }
    }

    /// A hit into an existing crater takes no material, so it must show none
    /// coming off - otherwise a burst emptied into one hole spews forever.
    #[test]
    fn a_hit_that_carves_nothing_throws_nothing() {
        let mut marks = DamageMarks::default();
        assert!(
            marks.add(DamageMark {
                at: Vec3::ZERO,
                radius: 1.0
            }),
            "the first hit carves"
        );
        assert!(
            !marks.add(DamageMark {
                at: Vec3::X * 0.1,
                radius: 0.2
            }),
            "a hit already inside that crater carves nothing"
        );
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
            volume: CHUNK_MIN_VOLUME * 0.5,
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
}
