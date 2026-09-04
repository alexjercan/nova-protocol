//! A piece of a body that came off and is still a body: real geometry, real
//! mass, its own place in the world.
//!
//! One thing produces one: a carve that SEVERS material - eats through the neck
//! between two lobes until the far lobe is no longer attached to anything. A
//! deep hole does not, however big it is, because the material a crater removes
//! was never sitting there as an object. Whoever cut the piece free hands it
//! here with a mesh, a place, and the velocity it inherited.
//!
//! # Born inside the body it left
//!
//! A chunk starts exactly where it was when it was part of the parent, which
//! means inside the parent's collider. A dynamic body spawned interpenetrating
//! another is a problem the solver fixes by shoving them apart, hard, and that
//! reads as a rock kicking its own debris across the screen.
//!
//! So a chunk spends its first [`CHUNK_GRACE_SECS`] as a KINEMATIC body with no
//! collider - it drifts out under the velocity it was given, touching nothing -
//! and only then becomes dynamic and grows its collider. By then it is clear.
//! [`ChunkGrace`] is public for the same reason: a section that detaches when it
//! dies ([`explode`](super::explode)) is born inside the ship it was bolted to
//! and needs exactly this window.
//!
//! This matters most for SHIPS, whose sections carry convex colliders with a
//! real inside. A chunk only ever comes off a rock that has been carved, and a
//! carved rock's collider is a trimesh - a shell rather than a solid - so a
//! chunk born inside one has nothing to resolve against; the grace costs it
//! nothing either way.
//!
//! # A crowd of them lands a few at a time
//!
//! The grace is a fixed window, so pieces born together come out of it
//! together. One collapse sheds hundreds of them in a single command flush, and
//! half a second later every one of those inserts a collider and a dynamic body
//! on the SAME frame - still stacked where they were bolted, so the solver
//! meets the whole population as contacts on that frame as well.
//!
//! [`land_carved_chunks`] therefore lands at most
//! [`CHUNK_ACTIVATIONS_PER_FRAME`] of them per frame and leaves the rest for
//! the next. That does not reduce the work, it spreads it, which is what a
//! frame-time tail cares about - and it is the safe direction to move in,
//! because a piece that waits is a piece that stays kinematic and colliderless,
//! which is what the grace was for in the first place. Nothing may LAND EARLY;
//! landing late costs only that the piece cannot be flown into yet.
//!
//! # Not the same thing as a shard
//!
//! [`spew`](super::spew) throws shards: kinematic, colliderless, short-lived,
//! purely a read on the hit. A shard never becomes physical. A chunk is the
//! opposite claim - this is material, it has volume, and you can fly into it.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::lifetime::TempEntity;

/// `CarvedChunkMarker`, `CarvedChunkPlugin`, `ChunkGrace`, `ChunkSpawn`,
/// `chunk_collider` and `spawn_carved_chunk`.
pub mod prelude {
    pub use super::{
        chunk_collider, spawn_carved_chunk, CarvedChunkMarker, CarvedChunkPlugin, ChunkGrace,
        ChunkSpawn, CHUNK_MIN_VOLUME,
    };
}

/// The least material a SEVERED piece has to hold before it is worth
/// simulating, in cubic world units.
///
/// Asked of a piece that is already free, never of a crater: a cut across a
/// rock does not end at a clean line, it leaves crumbs all round the rim where
/// the slab thinned out, and eighteen rigid bodies of a few cells each is
/// litter that costs a solver step. Under this a crumb goes out as
/// [`spew`](super::spew) dust instead.
///
/// One cubic unit is 8 hit points at the cladding's toughness
/// ([`mark_radius`](super::carve::mark_radius)), so it is also the scale at
/// which a piece is worth a ship noticing.
pub const CHUNK_MIN_VOLUME: f32 = 1.0;

/// How long a chunk drifts before it becomes a physical body.
///
/// Long enough to clear the collider it was born inside at the speed a carve
/// throws it, short enough that a player cannot see the moment it starts
/// colliding. See the module docs for why it is not zero.
pub const CHUNK_GRACE_SECS: f32 = 0.5;

/// The most pieces that may become physical in one frame.
///
/// Sized so that a single hit is never delayed at all and a collapse is fully
/// physical about half a second after its first piece lands: a hull raked by a
/// siege lance sheds some 700 pieces into one flush, which at this rate is
/// about thirty frames of landings rather than one. See the module docs for why
/// waiting is safe and landing early would not be.
const CHUNK_ACTIVATIONS_PER_FRAME: usize = 24;

/// How long a chunk survives before it despawns.
///
/// The same span a detached wreck gets ([`explode`](super::explode)), so the
/// field clears at one rate whatever put debris on it. A chunk is debris: an
/// unattended scene that keeps carving rocks - a menu backdrop, a long mission -
/// would otherwise accumulate physics bodies without bound.
pub const CHUNK_LIFETIME_SECS: f32 = 30.0;

/// The thinnest a chunk's collider may be, in the mesh's own units.
///
/// A piece cut thin enough to come away coplanar hulls into a shape with no
/// volume. parry returns that hull rather than declining it, avian gives a
/// volumeless dynamic body zero mass AND zero inertia, and the solver divides by
/// it: the body's swept AABB comes back NaN and avian asserts on it frames
/// later, deep inside `update_solver_body_aabbs`
/// (`assertion failed: b.min.cmple(b.max).all()`).
///
/// A rock is a blob and most pieces of one have volume, so this is the floor
/// under the flattest crumbs a cut shaves off the rim rather than a common case.
const CHUNK_MIN_THICKNESS: f32 = 0.02;

/// The most points [`chunk_collider`] hands to the convex hull.
///
/// A piece coming off a body is an unwelded TRIANGLE SOUP - `TriangleMeshBuilder`
/// emits three positions per triangle and shares none - so every corner reaches
/// parry twice more than it needs to. And the hull is not linear in the count:
/// 34 points cost 7 us, 2020 cost 139 us and 5268 cost 1488 us, so the handful
/// of dense pieces a cut frees dominate a whole frame's colliders.
///
/// A hull is a SHELL, and a strided sample across the surface describes the same
/// shell. Priced over 924 sampled pieces the mean fell from 94.0 us to 16.4 us,
/// and MORE of them came back with usable mass (681 against 673) - fewer
/// near-duplicate points is fewer degenerate faces for parry to reject.
const HULL_POINT_CAP: usize = 64;

/// Tags a piece that came off a body under fire and is now a body of its own.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct CarvedChunkMarker;

/// A piece still drifting clear of the body it came off, and the collider it
/// gets when it has.
///
/// Public because a carved chunk is not the only thing born inside its parent -
/// see the module docs.
#[derive(Component, Clone, Debug)]
pub struct ChunkGrace {
    /// The collider to grow once the grace runs out.
    collider: Collider,
    /// Seconds left of it.
    remaining: f32,
}

impl ChunkGrace {
    /// The window a piece born inside its parent drifts through before it
    /// becomes physical and grows `collider`.
    pub fn new(collider: Collider) -> Self {
        Self {
            collider,
            remaining: CHUNK_GRACE_SECS,
        }
    }
}

/// Everything a caller has to decide about a chunk. The rest - lifetime, the
/// grace window, the body type - is this module's.
pub struct ChunkSpawn {
    /// What the chunk is called in the inspector.
    pub name: String,
    /// Its geometry, in ITS OWN space: centred on the piece, not on the body it
    /// was cut out of.
    pub mesh: Handle<Mesh>,
    /// Where the piece is, which is where it was when it came off.
    pub transform: Transform,
    /// The velocity it leaves with, in world units per second. Callers that
    /// have a parent to inherit from should pass `v + omega x r`, so a chunk
    /// off a tumbling rock carries the tumble.
    pub velocity: Vec3,
    /// How it tumbles, in radians per second.
    pub spin: Vec3,
    /// The collider it grows once it is clear.
    pub collider: Collider,
}

/// Put a chunk in the world, and hand it back UNDRESSED.
///
/// The caller inserts the material, because there is no one type to take: a
/// rock's pieces want its triplanar `ExtendedMaterial` and a section's want a
/// plain `StandardMaterial`. Handing back the entity is what keeps this module
/// out of that question entirely - and a chunk with no material inserted is
/// drawn as nothing, so it is not a question a caller can forget.
///
/// Kinematic and colliderless to begin with; [`land_carved_chunks`] makes it
/// physical once it has drifted clear. See the module docs.
pub fn spawn_carved_chunk(commands: &mut Commands, spawn: ChunkSpawn) -> Entity {
    trace!(
        "spawn_carved_chunk: {} at {}",
        spawn.name,
        spawn.transform.translation
    );

    commands
        .spawn((
            CarvedChunkMarker,
            Name::new(spawn.name),
            Mesh3d(spawn.mesh),
            spawn.transform,
            RigidBody::Kinematic,
            LinearVelocity(spawn.velocity),
            AngularVelocity(spawn.spin),
            ChunkGrace::new(spawn.collider),
            TempEntity(CHUNK_LIFETIME_SECS),
        ))
        .id()
}

/// A collider avian can actually simulate for `mesh`, or `None` when the mesh
/// has no bounds to work from and there is nothing to spawn.
///
/// Prefers the true convex hull, and falls back to the mesh's own bounding box
/// padded to [`CHUNK_MIN_THICKNESS`] whenever that hull would leave the body
/// without mass. MASS is the property tested rather than the vertex layout,
/// because it is the one the solver divides by - a hull can be judged
/// non-degenerate by its bounds and still come back with none.
///
/// A slightly boxy piece inside debris that lasts a few seconds is not
/// something a player can see. A NaN rigid body is a crash.
pub fn chunk_collider(mesh: &Mesh) -> Option<Collider> {
    let (centre, half) = mesh_bounds(mesh)?;

    if half.min_element() > CHUNK_MIN_THICKNESS {
        if let Some(hull) = hull_points(mesh).and_then(Collider::convex_hull) {
            if hull.mass_properties(1.0).mass > 0.0 {
                return Some(hull);
            }
        }
    }

    // Offset by the bounds' centre: a piece is cut off a bigger mesh, so its
    // geometry sits wherever it sat in the original and a box centred on the
    // entity would not cover it.
    let padded = half.max(Vec3::splat(CHUNK_MIN_THICKNESS));
    Some(Collider::compound(vec![(
        centre,
        Quat::IDENTITY,
        Collider::cuboid(padded.x * 2.0, padded.y * 2.0, padded.z * 2.0),
    )]))
}

/// At most [`HULL_POINT_CAP`] of `mesh`'s positions, evenly strided, or `None`
/// when it carries none to sample.
///
/// Only ever called after [`mesh_bounds`] has accepted the same mesh, so every
/// position here is already known finite.
fn hull_points(mesh: &Mesh) -> Option<Vec<Vec3>> {
    use bevy::mesh::VertexAttributeValues;
    let VertexAttributeValues::Float32x3(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
    else {
        return None;
    };

    let stride = positions.len().div_ceil(HULL_POINT_CAP).max(1);
    Some(
        positions
            .iter()
            .step_by(stride)
            .map(|position| Vec3::from_array(*position))
            .collect(),
    )
}

/// A mesh's local centre and half extents, or `None` when it carries no finite
/// positions to measure.
///
/// Read straight off the position attribute rather than through bevy's
/// `compute_aabb`, so the finite check is on the numbers this module is about
/// to hand to the physics engine.
pub fn mesh_bounds(mesh: &Mesh) -> Option<(Vec3, Vec3)> {
    use bevy::mesh::VertexAttributeValues;
    let VertexAttributeValues::Float32x3(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
    else {
        return None;
    };

    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for position in positions {
        let point = Vec3::from_array(*position);
        if !point.is_finite() {
            return None;
        }
        min = min.min(point);
        max = max.max(point);
    }
    (min.cmple(max).all()).then(|| ((min + max) * 0.5, (max - min) * 0.5))
}

/// Make chunks physical once they have drifted clear of the body they came
/// off, at most [`CHUNK_ACTIVATIONS_PER_FRAME`] of them per frame.
fn land_carved_chunks(
    time: Res<Time>,
    mut commands: Commands,
    mut q_chunks: Query<(Entity, &mut ChunkGrace)>,
) {
    let step = time.delta_secs();
    let mut landed = 0usize;
    for (entity, mut grace) in &mut q_chunks {
        grace.remaining -= step;
        if grace.remaining > 0.0 {
            continue;
        }
        // Past its grace but past the frame's allowance too, so it drifts one
        // more frame. It keeps its `ChunkGrace`, and with it the collider it is
        // owed. Not a `break`: every remaining piece's own clock has to keep
        // running down, or a queue behind the allowance would hold pieces back
        // that had not even reached their grace yet.
        if landed >= CHUNK_ACTIVATIONS_PER_FRAME {
            continue;
        }
        landed += 1;
        trace!("land_carved_chunks: {entity:?} is clear, going dynamic");
        commands
            .entity(entity)
            .insert((RigidBody::Dynamic, grace.collider.clone()))
            .remove::<ChunkGrace>();
    }
}

/// Turns drifting chunks into physical ones.
pub struct CarvedChunkPlugin;

impl Plugin for CarvedChunkPlugin {
    fn build(&self, app: &mut App) {
        trace!("CarvedChunkPlugin: build");

        app.register_type::<CarvedChunkMarker>();
        app.add_systems(Update, land_carved_chunks);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn chunk_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // A fixed step, so the grace runs down by a known amount per update
        // rather than by however long the test took to get here.
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs_f32(CHUNK_GRACE_SECS * 0.6),
        ));
        app.add_plugins(CarvedChunkPlugin);
        app
    }

    fn a_chunk(commands: &mut Commands) -> Entity {
        spawn_carved_chunk(
            commands,
            ChunkSpawn {
                name: "Test Chunk".to_string(),
                mesh: Handle::default(),
                transform: Transform::default(),
                velocity: Vec3::X * 4.0,
                spin: Vec3::Y,
                collider: Collider::sphere(0.5),
            },
        )
    }

    /// THE reason the grace exists. A chunk is born inside the collider it came
    /// off, and a dynamic body spawned interpenetrating another gets shoved out
    /// hard enough to read as the parent kicking its own debris.
    #[test]
    fn a_fresh_chunk_touches_nothing() {
        let mut app = chunk_app();
        let chunk = app
            .world_mut()
            .run_system_once(|mut commands: Commands| a_chunk(&mut commands))
            .expect("the spawn runs");
        app.update();

        assert_eq!(
            app.world().get::<RigidBody>(chunk),
            Some(&RigidBody::Kinematic),
            "a fresh chunk drifts rather than colliding"
        );
        assert!(
            app.world().get::<Collider>(chunk).is_none(),
            "and has nothing to collide WITH yet"
        );
        assert_eq!(
            app.world().get::<LinearVelocity>(chunk).map(|v| v.0),
            Some(Vec3::X * 4.0),
            "but it is already leaving"
        );
    }

    /// And it does not stay a ghost: once it is clear it is a real body a ship
    /// can fly into.
    #[test]
    fn a_chunk_that_has_drifted_clear_becomes_physical() {
        let mut app = chunk_app();
        let chunk = app
            .world_mut()
            .run_system_once(|mut commands: Commands| a_chunk(&mut commands))
            .expect("the spawn runs");

        // A warm-up tick: the first update of a manually stepped clock reports
        // a delta of zero, so nothing would come off the grace. Then two steps
        // of 0.6 of the window carries it past.
        app.update();
        app.update();
        app.update();

        assert_eq!(
            app.world().get::<RigidBody>(chunk),
            Some(&RigidBody::Dynamic)
        );
        assert!(
            app.world().get::<Collider>(chunk).is_some(),
            "a landed chunk is something you can hit"
        );
    }

    /// How many pieces are physical bodies now.
    fn physical(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), (With<CarvedChunkMarker>, With<Collider>)>()
            .iter(app.world())
            .count()
    }

    /// THE ceiling. A hull raked by a siege lance sheds hundreds of pieces into
    /// one command flush, all carrying the same grace, so without a per-frame
    /// allowance every one of them grows a collider on the same frame and the
    /// solver meets the lot of them at once.
    #[test]
    fn a_crowd_of_pieces_lands_a_few_at_a_time() {
        let crowd = CHUNK_ACTIVATIONS_PER_FRAME * 3;
        let mut app = chunk_app();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                for _ in 0..crowd {
                    a_chunk(&mut commands);
                }
            })
            .expect("the spawns run");

        // The first manual step reports a delta of zero, then two steps of 0.6
        // of the window carry every piece past its grace on the same frame.
        app.update();
        app.update();
        app.update();
        assert_eq!(
            physical(&mut app),
            CHUNK_ACTIVATIONS_PER_FRAME,
            "one frame landed more than its allowance"
        );

        // And the piece that waited is still safe to wait: the grace's own
        // contract is that nothing is physical inside the body it came off.
        let waiting: Vec<RigidBody> = app
            .world_mut()
            .query_filtered::<&RigidBody, With<ChunkGrace>>()
            .iter(app.world())
            .copied()
            .collect();
        assert_eq!(waiting.len(), crowd - CHUNK_ACTIVATIONS_PER_FRAME);
        for body in waiting {
            assert_eq!(body, RigidBody::Kinematic, "a waiting piece went physical");
        }

        app.update();
        app.update();
        assert_eq!(physical(&mut app), crowd, "and every piece does land");
    }

    /// Chunks are debris, not scenery: a scene that keeps carving rocks must
    /// not accumulate physics bodies without bound.
    #[test]
    fn every_chunk_clears_itself() {
        let mut app = chunk_app();
        let chunk = app
            .world_mut()
            .run_system_once(|mut commands: Commands| a_chunk(&mut commands))
            .expect("the spawn runs");
        app.update();

        assert!(app.world().get::<TempEntity>(chunk).is_some());
    }

    /// A sphere as an unwelded TRIANGLE SOUP - the shape every piece off a body
    /// arrives in. Three positions per triangle, nothing shared.
    fn dense_soup() -> Mesh {
        let (rings, segments) = (16usize, 32usize);
        let at = |ring: usize, segment: usize| {
            let phi = std::f32::consts::PI * ring as f32 / rings as f32;
            let theta = std::f32::consts::TAU * segment as f32 / segments as f32;
            [phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()]
        };

        let mut positions: Vec<[f32; 3]> = Vec::new();
        for ring in 0..rings {
            for segment in 0..segments {
                let (a, b) = (at(ring, segment), at(ring, segment + 1));
                let (c, d) = (at(ring + 1, segment), at(ring + 1, segment + 1));
                positions.extend([a, c, b, b, c, d]);
            }
        }
        let indices: Vec<u32> = (0..positions.len() as u32).collect();

        Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(bevy::mesh::Indices::U32(indices))
    }

    /// The hull is priced on POINTS, super-linearly, and a soup hands it the
    /// same corner three times. Sampling must still describe the PIECE: a hull
    /// with mass, and one TIGHTER than the box the fallback would have used.
    #[test]
    fn a_dense_soup_is_hulled_from_a_bounded_sample() {
        let mesh = dense_soup();
        let points = mesh.count_vertices();
        assert!(
            points > HULL_POINT_CAP * 3,
            "delivery guard: the fixture must out-run the cap, got {points} points"
        );

        let collider = chunk_collider(&mesh).expect("a dense piece still gets a collider");
        let mass = collider.mass_properties(1.0).mass;
        assert!(
            mass > 0.0,
            "a body with no mass makes the solver produce NaN"
        );
        // A unit sphere is 4/3 pi against its 8-unit box. A mass anywhere near
        // 8 is the cuboid fallback, which would mean the sampled hull had been
        // rejected rather than used.
        assert!(
            mass < 6.0,
            "the capped sample fell back to a bounding box: mass {mass}"
        );
    }

    /// A FLAT piece - the shape ship art is full of and an asteroid never is.
    /// Its convex hull has no volume, which is the zero-mass NaN body.
    #[test]
    fn a_flat_piece_still_gets_a_collider_with_mass() {
        let mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-1.0f32, 0.0, -1.0],
                [1.0, 0.0, -1.0],
                [1.0, 0.0, 1.0],
                [-1.0, 0.0, 1.0],
            ],
        )
        .with_inserted_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 0, 2, 3]));

        let bare = Collider::convex_hull_from_mesh(&mesh)
            .expect("parry hulls a flat panel rather than declining");
        assert_eq!(
            bare.mass_properties(1.0).mass,
            0.0,
            "delivery guard: a coplanar hull is exactly the zero-mass case"
        );

        let collider = chunk_collider(&mesh).expect("flat art still gets a collider");
        assert!(
            collider.mass_properties(1.0).mass > 0.0,
            "a body with no mass makes the solver produce NaN"
        );
    }
}
