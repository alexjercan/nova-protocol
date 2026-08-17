//! Carving a rock: the signed field behind an asteroid, and the remesh that
//! follows a hit.
//!
//! An asteroid is the one body in the game with nothing to hide behind. A ship
//! carves through its cladding and stops at the structure underneath, because a
//! plate is one cell thick and the hull it is bolted to is a glTF model nothing
//! can cut. A rock is solid all the way down, so a carve here can go as deep as
//! the hit deserves - which is what makes it the honest test of the whole idea.
//!
//! # Seeded from the shipped silhouette, not from a fresh shape
//!
//! The field is `|p| - radius(p/|p|)` on the asteroid's OWN
//! [`RockHeight`] sampler and its OWN seed, which is the same function
//! `apply_noise` displaced the shipped mesh by. So the first remesh reproduces
//! the rock that was already there rather than swapping in a different one:
//! what changes is the crater, not the rock.
//!
//! # Built on the first hit and never before
//!
//! Seeding costs tens of thousands of noise samples, and a scenario can hold a
//! field of a hundred rocks that are never touched. So the grid is allocated
//! the first time a rock is actually marked; an unshot asteroid costs exactly
//! what it always did. After seeding, nothing resamples the noise: a carve
//! touches the cells its sphere reaches, and the remesh reads the stored grid.
//!
//! # What it swaps, and what it must not break
//!
//! A remesh replaces the drawn `Mesh3d` and rebuilds the trimesh `Collider` -
//! the same `trimesh_from_mesh` call the spawn path uses, so mass-from-volume
//! stays honest and a carved rock weighs what is left of it. `BodyRadius` is
//! re-derived and only ever SHRINKS, which is what keeps gravity spheres of
//! influence and orbit bands valid without recomputing them: everything sized
//! off a rock's surface was authored against a bigger rock than the one that is
//! there now.

use avian3d::prelude::{Collider, ColliderDensity};
use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::BodyRadius;

use super::{
    asteroid::{AsteroidMarker, AsteroidRadius, AsteroidSeed},
    asteroid_surface::prelude::{RockHeight, ROCK_SURFACE_MIN},
};

/// `AsteroidField` and `AsteroidCarvePlugin`.
pub mod prelude {
    pub use super::{AsteroidCarvePlugin, AsteroidField};
}

/// Cells per axis in a rock's field.
///
/// The design's wasm cap, used everywhere rather than scaled by rock size, and
/// deliberately: it is the resolution a single-threaded browser can remesh
/// inside a frame, and a rock that looked coarser on the web than on the
/// desktop would be a different rock. `33^3` corners is 140 KB per carved rock,
/// paid only by rocks that are hit.
///
/// Coarseness is also the ART. A finer grid does not make a better rock, it
/// makes a smoother one, and the game is flat-shaded facets throughout.
const FIELD_RESOLUTION: usize = 32;

/// How much room past the rock's own surface the grid covers.
///
/// Only just over 1: carving never adds material, so the surface can never
/// reach further out than the pristine silhouette already does. The margin is
/// for the one cell of slop a sign change needs on the outside.
const FIELD_MARGIN: f32 = 1.08;

/// The signed field a carved asteroid is meshed from, in the mesh node's own
/// UNIT space.
///
/// Unit space and not world space, because that is the frame the node's mesh
/// and collider already live in - the node carries `Transform::from_scale(radius)`
/// - and it is the frame [`DamageMarks`] on the same entity are recorded in.
/// Nothing here has to know how big the rock is.
#[derive(Component, Debug)]
pub struct AsteroidField {
    /// The solid, carved by every mark seen so far.
    field: SignedField,
    /// A fingerprint of the mark list the solid was last carved by, so a rock
    /// is remeshed when its marks move and not otherwise.
    ///
    /// NOT a count. Past the mark budget a hit MERGES into its nearest
    /// neighbour, growing that mark's radius without growing the list, and a
    /// count would call that "nothing new" and never carve it.
    carved: u64,
}

impl AsteroidField {
    /// A fingerprint of `marks`: enough to tell a list that moved from one that
    /// did not.
    ///
    /// Every field of every mark goes in, because a merge changes a radius in
    /// place. Cheap - the list is capped at a couple of dozen - and it does not
    /// have to be collision-free, only different when something changed.
    fn signature(marks: &DamageMarks) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for mark in &marks.0 {
            for value in [mark.at.x, mark.at.y, mark.at.z, mark.radius] {
                hash ^= u64::from(value.to_bits());
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        hash
    }
}

impl AsteroidField {
    /// The solid the rock currently is, in unit space.
    pub fn solid(&self) -> &SignedField {
        &self.field
    }
}

/// The pristine field of a rock with this `seed`: the analytic version of the
/// silhouette `apply_noise` builds.
///
/// The shipped mesh is a unit sphere whose vertices `apply_noise` pushes out
/// to `radius(direction)`, so the surface is `|p| = radius(p/|p|)` and the
/// signed distance is what this returns. The mesh and the field are two
/// readings of ONE function rather than two shapes that have to be kept in
/// step.
///
/// The near/far shortcut is not an approximation of the surface, it is a bound
/// on it: no rock's surface comes closer in than `ROCK_SURFACE_MIN` or reaches
/// past the grid, so outside that shell the sign is settled without asking the
/// noise. The noise is the only expensive part, and this halves how often it
/// is asked.
fn pristine_field(seed: u32, half_extent: f32) -> SignedField {
    let rock = RockHeight::default().with_seed(seed).sampler();
    SignedField::sample(FIELD_RESOLUTION, half_extent, |at| {
        let radius = at.length();
        // Inside the smallest the surface can be, or outside the largest: the
        // sign is settled and the exact value only has to be conservative.
        if radius <= ROCK_SURFACE_MIN {
            return radius - ROCK_SURFACE_MIN;
        }
        if radius >= half_extent {
            return radius - half_extent;
        }
        radius - rock.radius(at / radius)
    })
}

/// Give a rock its field the first time it is marked, then keep its mesh,
/// collider and published radius in step with the marks.
///
/// One remesh per rock per frame however many hits landed: a frame's worth of
/// marks is one list, and one list is one carve. The coalescing the design
/// asked for falls out of that rather than needing a queue.
///
/// NOT filtered on `Changed<DamageMarks>`, deliberately. Seeding the grid takes
/// a frame of its own - the insert lands on the next flush - and by then the
/// marks have not changed again, so a change filter would seed every rock and
/// carve none of them. The fingerprint compare that replaces it is two integers
/// per rock per frame.
///
/// SYNCHRONOUS. The design's plan is to run the remesh and the collider build
/// on the async compute pool with at most one job in flight per rock; that is
/// worth doing when the numbers say so, and the numbers have to come first.
/// This logs what each stage costs so the decision is made on measurements
/// rather than on the estimate that motivated the plan.
#[expect(
    clippy::type_complexity,
    reason = "the query carries the whole node: marks, field, parent and drawn mesh"
)]
fn carve_asteroid_fields(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut q_nodes: Query<(
        Entity,
        &DamageMarks,
        Option<&mut AsteroidField>,
        &ChildOf,
        Option<&Mesh3d>,
    )>,
    q_asteroid: Query<(&AsteroidSeed, &AsteroidRadius, &BodyRadius), With<AsteroidMarker>>,
) {
    for (node, marks, field, ChildOf(root), mesh) in &mut q_nodes {
        if marks.0.is_empty() {
            continue;
        }
        let Ok((seed, nominal, body)) = q_asteroid.get(*root) else {
            continue;
        };
        // How far the pristine surface reaches in the node's own unit space:
        // the published world radius over the authored one, which is exactly
        // the factor the spawn path derived it with.
        let unit_extent = (body.0 / nominal.0.max(f32::EPSILON)).max(1.0);

        let mut field = match field {
            Some(field) => field,
            None => {
                let started = std::time::Instant::now();
                let seeded = pristine_field(seed.0, unit_extent * FIELD_MARGIN);
                debug!(
                    "carve_asteroid_fields: seeded {node:?} at {FIELD_RESOLUTION}^3 in {:.1} ms",
                    started.elapsed().as_secs_f32() * 1000.0
                );
                commands.entity(node).insert(AsteroidField {
                    field: seeded,
                    carved: 0,
                });
                // The insert lands next flush, and this rock is carved the
                // frame after. One frame of lag on the first hit only, and it
                // keeps the seeding cost out of the same frame as the first
                // remesh rather than paying for both at once.
                continue;
            }
        };

        let signature = AsteroidField::signature(marks);
        if field.carved == signature {
            continue;
        }
        field.carved = signature;
        // EVERY mark, not just the ones that look new. Subtraction is a max, so
        // re-applying one that is already in the solid changes nothing, and
        // that idempotence is what lets this skip tracking which is which -
        // including the merge case, where a mark's radius grew in place.
        for mark in &marks.0 {
            field.field.subtract_sphere(mark.at, mark.radius);
        }

        let started = std::time::Instant::now();
        let surface = field.field.surface().build();
        let remeshed = started.elapsed();

        let started = std::time::Instant::now();
        let collider = Collider::trimesh_from_mesh(&surface);
        let rebuilt = started.elapsed();

        let Some(collider) = collider else {
            // A rock carved into something parry will not accept is a rock
            // that keeps the shape it had. Never leave a body without a
            // collider: it would fall out of the world and stop stopping
            // rounds.
            warn!("carve_asteroid_fields: {node:?} carved into an unusable collider, kept");
            continue;
        };

        let surviving = field.field.surface_radius();
        debug!(
            "carve_asteroid_fields: {node:?} remesh {:.1} ms, collider {:.1} ms, \
             {} tri(s), unit radius {surviving:.2}",
            remeshed.as_secs_f32() * 1000.0,
            rebuilt.as_secs_f32() * 1000.0,
            surface.indices().map_or(0, |i| i.len() / 3),
        );

        let mut node = commands.entity(node);
        node.insert(collider);
        // The density rides along unchanged, so avian re-derives mass from the
        // volume that is actually left: a carved rock is a lighter rock.
        node.insert(ColliderDensity(1.0));
        match mesh {
            Some(_) => {
                node.insert(Mesh3d(meshes.add(surface)));
            }
            // Headless: the node never had a drawn mesh and must not grow one.
            None => {}
        }

        // Only ever DOWN. Everything sized off a rock's surface - standoff
        // distances, orbit clearances, the sphere of influence - was authored
        // against the pristine radius, so shrinking keeps every one of those
        // valid and growing would silently invalidate them.
        let shrunk = nominal.0 * surviving;
        if shrunk < body.0 {
            commands.entity(*root).insert(BodyRadius(shrunk));
        }
    }
}

/// Gives asteroids a carvable field and remeshes them as they are hit.
#[derive(Default, Clone, Debug)]
pub struct AsteroidCarvePlugin {
    /// Whether a remesh replaces the DRAWN mesh as well as the collider.
    ///
    /// The collider half is gameplay and runs headless: a crater a server
    /// cannot see is still a crater a round can fly into.
    pub render: bool,
}

impl Plugin for AsteroidCarvePlugin {
    fn build(&self, app: &mut App) {
        debug!("AsteroidCarvePlugin: build");

        let _ = self.render;
        app.add_systems(Update, carve_asteroid_fields);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seeded field has to be the rock that is already on screen, not a new
    /// one. Both the shipped mesh and the field are read off the SAME
    /// `RockHeight` sampler, so this pins the translation between them: the
    /// meshed surface must sit within a cell of where the sampler says the
    /// rock's surface is.
    #[test]
    fn the_seeded_field_reproduces_the_shipped_silhouette() {
        let seed = 4242;
        let rock = RockHeight::default().with_seed(seed).sampler();
        let field = pristine_field(seed, 7.0);
        let mesh = field.surface().build();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("a seeded field meshes");
        };

        assert!(!positions.is_empty(), "the rock has a surface");
        for position in positions {
            let at = Vec3::from_array(*position);
            let radius = at.length();
            let expected = rock.radius(at / radius);
            assert!(
                (radius - expected).abs() < field.cell_size(),
                "a vertex sat at {radius} where the rock's surface is {expected}"
            );
        }
    }

    /// The rule that keeps gravity and navigation valid without recomputing
    /// them: carving removes material, so a rock's published radius can only
    /// ever fall.
    #[test]
    fn a_carved_rock_never_grows() {
        let mut field = pristine_field(7, 6.5);
        let mut previous = field.surface_radius();
        assert!(previous > 1.0, "delivery guard: the rock has a surface");

        for step in 0..4 {
            let angle = step as f32 * 1.3;
            field.subtract_sphere(Vec3::new(angle.cos(), angle.sin(), 0.0) * previous, 1.5);
            let now = field.surface_radius();
            assert!(now <= previous + 1e-4, "grew at step {step}");
            previous = now;
        }
    }
}
