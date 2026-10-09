//! Carving a rock: the signed field behind an asteroid, and the remesh that
//! follows a hit.
//!
//! An asteroid is the one body in the game with nothing to hide behind. A ship
//! carves through its cladding and stops at the structure underneath, because a
//! plate is one cell thick and the hull it is bolted to is a glTF model nothing
//! can cut. A rock is solid all the way down, so a carve here can go as deep as
//! the hit deserves - which is what makes it the honest test of the whole idea.
//!
//! # The field IS the rock
//!
//! [`pristine_field`] is the only description of an asteroid's shape.
//! [`pristine_rock_mesh`] is that field meshed, and it is what the spawn path
//! draws and collides with; the reseed on the first hit calls the same function
//! with the same seed and gets the same grid back. So a hit changes the CRATER
//! and nothing else.
//!
//! ONE shape, not two. A subdivided octahedron displaced by the noise for the
//! shipped mesh and this field for the carved one agree to within a cell, which
//! is not the same as agreeing: the first hit moves the silhouette and changes
//! the size of every facet on it, and that pop shows on a rock the shot had
//! barely scratched.
//!
//! # Kept only while it is needed
//!
//! The grid is 140 KB on an arena rock and 275 KB on the biggest one the cap
//! allows, and a scenario scatters a hundred rocks most of which are never
//! touched, so the spawn path meshes the field and DROPS it. The first hit pays
//! to build it again - tens to hundreds of thousands of noise samples - and from
//! then on nothing resamples: a carve touches the cells its sphere reaches, and
//! the remesh reads the stored grid.
//!
//! # Nothing here happens in the frame that asked for it
//!
//! The seed and the remesh both run on the async compute pool, one job at a
//! time per rock, and the rock keeps drawing the surface it already had until
//! one lands. What the main thread pays is the sphere subtraction the mark
//! itself reaches and the swap when the job comes back. A rock is therefore a
//! frame or two behind the shot that hit it, which is the same staleness the
//! volume throttle below allows and is what keeps a held burst off the frame
//! rate.
//!
//! The swap is PLACEMENT and nothing else. Everything a carve produces -
//! including the pieces it cut free - arrives built, because the only thing
//! that ever asks a grid a question is the worker that already holds one:
//! [`CarveApplyReport`] counts the grids that reach the main thread, and one
//! per rock is the whole budget.
//!
//! On wasm there is no worker to move to - `AsyncComputeTaskPool` there is the
//! browser's own task queue on the one thread the page has - so the split buys
//! no parallelism. It still buys the frame: the work lands between ticks with
//! the rest of the carve rather than inside the system that swaps the result
//! in, and it is one job instead of one job plus a per-piece tail.
//!
//! # What it swaps, and what it must not break
//!
//! A remesh replaces the drawn `Mesh3d` and rebuilds the `Collider` as a
//! TRIMESH. That is also where a rock stops being collided against as a hull:
//! the spawn path hulls the pristine ball because a whole field of trimeshes
//! is unaffordable, and a hole is precisely the shape a hull cannot hold. Mass
//! comes from the surface either way, so a carved rock weighs what is left of
//! it. `BodyRadius` is
//! re-derived and only ever SHRINKS, so later contact and clearance checks
//! cannot assume a larger surface than the rock still has.

use avian3d::prelude::{AngularVelocity, Collider, ColliderDensity, LinearVelocity, RigidBody};
// Bevy's platform Instant, not std's - `std::time::Instant::now` panics
// on wasm32-unknown-unknown, which this crate ships to.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    platform::time::Instant,
    prelude::*,
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
};
use nova_events::prelude::{CommandsGameEventExt, *};
use nova_gameplay::prelude::*;
use nova_ship::prelude::{BodyRadius, RadarOccluder};

use super::{
    asteroid::{AsteroidMarker, AsteroidRadius, AsteroidSeed, AsteroidTexture},
    asteroid_kind::prelude::{asteroid_kind_look, AsteroidKind, AsteroidKindId},
    asteroid_surface::prelude::{AsteroidSurfaceMaterial, AsteroidSurfaceMaterialExt, RockHeight},
};

/// `AsteroidField`, its seed request and remesh event, `AsteroidCarvePlugin`,
/// the rock mesh they share with the spawn path, and the rock chunk
/// freeze/thaw pair with `RockChunkSurface`, `FrozenRockChunk` and
/// `FrozenChunkMesh`.
pub mod prelude {
    pub use super::{
        freeze_rock_chunk, pristine_rock_mesh, thaw_rock_chunk, AsteroidCarvePlugin, AsteroidField,
        AsteroidFieldSeedRequest, AsteroidRemeshed, CarveApplyReport, FrozenChunkMesh,
        FrozenRockChunk, RockChunkSurface,
    };
}

/// How wide one field cell is, in WORLD units.
///
/// The cell is fixed in the WORLD and the cell COUNT is derived from it, which
/// is the opposite way round from how this started. A crater is a world-sized
/// thing - a 4-damage PDC round carves a 0.62 unit sphere whatever it lands on -
/// so a grid whose cells grew with the rock could not draw that round's hole on
/// anything big, and a fixed count meant exactly that: 32 cells across a
/// radius-3 rock is a 1.02 unit cell, four times the round that is being fired
/// at it. Half a unit puts the shipped PDC round at about 1.2 cells across on
/// every rock in a fight.
///
/// Coarseness is still the ART. This is not a resolution knob for prettier
/// rocks - a finer grid only makes a smoother one - it is the size of the
/// smallest hole the game has to be able to show.
const FIELD_CELL_WORLD: f32 = 0.5;

/// The most cells a rock's field may have per axis.
///
/// A cap on the CELL COUNT is a cap on a frame, because everything about a
/// field is `count^3`: the seed, the corner scans, the remesh, the collider.
/// At `64^3` one carve measured 43 ms of main-thread work on one desktop
/// core - 17 to remesh 28,000 triangles, 11 to rebuild the collider, 3 to test
/// connectivity and the rest in whole-grid scans - a first hit seeded in 19 ms,
/// and a rock cost 39 ms to spawn. Held PDC fire pays the carve on every second
/// frame, which is what put the asteroid field at 25 fps. `40^3` is a quarter of
/// the cells.
///
/// It BINDS above about radius 1.8, and what it costs there is the cell: a
/// radius-3 rock grids at 0.82 units rather than 0.5, so one PDC round is under
/// a cell and only sustained fire - whose mark GROWS where it is held - opens a
/// hole. That is the trade this number exists to make, and it is why it is not
/// a resolution knob: raising it buys smoother rocks and a frame nothing else
/// can pay for. `41^3` corners is 275 KB per carved rock, paid only by rocks
/// that are hit.
const FIELD_RESOLUTION_MAX: usize = 40;

/// The fewest cells a rock's field may have per axis.
///
/// A rock is noise on a 4-unit base swinging 1.6 units either way, so its relief
/// is about a quarter of its own reach. At 16 cells one cell is about a quarter
/// of that relief, which still leaves every lobe and hollow in the silhouette:
/// swept over twelve seeds, the meshed reach holds inside 4% of what a 64-cell
/// grid measures down to 16 cells and starts losing peaks below it. It binds
/// under about radius 0.7 - smaller than anything a shipped scenario scatters -
/// so it is a floor for mods and for debris, not a size the game authors.
const FIELD_RESOLUTION_MIN: usize = 16;

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
/// and collider already live in (the node carries
/// `Transform::from_scale(radius)`), and it is the frame [`DamageMarks`] on the
/// same entity are recorded in. Nothing here has to know how big the rock is.
#[derive(Component, Debug)]
pub struct AsteroidField {
    /// The solid, carved by every mark seen so far.
    field: SignedField,
    /// A fingerprint of the mark list already applied to `field`.
    ///
    /// NOT a count. Repeated fire grows a mark's radius without growing the
    /// list, and a count would call that "nothing new" and never carve it.
    applied: u64,
    /// Mark signature whose candidate was last attempted; `None` after a
    /// [`mine`](Self::mine), which changes the solid without changing the
    /// marks.
    ///
    /// A rejected surface waits for another mark or mining pulse rather than
    /// rebuilding the same unusable collider every frame.
    attempted: Option<u64>,
    /// How much solid `field` holds, in the grid's own cubic units.
    ///
    /// CARRIED, not measured. `subtract_sphere` reports exactly what it took by
    /// the same count [`SignedField::solid_volume`] uses, so the guard below is
    /// a compare rather than a whole-grid scan every rock pays every frame.
    volume: f32,
    /// Quantized solid volume at the last successful mesh and collider swap.
    ///
    /// Marks smaller than a grid cell still accumulate in `field`, but work
    /// waits until a corner changes sign and the grid can draw the result.
    meshed_volume: f32,
}

impl AsteroidField {
    /// A fingerprint of `marks`: enough to tell a list that moved from one that
    /// did not.
    ///
    /// Every field of every mark goes in, because a merge changes a radius in
    /// place. Cheap - the list is capped at a couple of dozen - and it does not
    /// have to be collision-free, only different when something changed.
    ///
    /// Never persisted and never compared across processes: a signature is only
    /// ever weighed against another signature taken by this same function in
    /// this same session, so unlike the seeds and looks [`Fnv64`] also derives,
    /// the exact value here is free to move with the hash.
    fn signature(marks: &DamageMarks) -> u64 {
        let mut hash = Fnv64::new();
        for mark in &marks.0 {
            for value in [mark.at.x, mark.at.y, mark.at.z, mark.radius] {
                hash = hash.write(&value.to_bits().to_le_bytes());
            }
        }
        hash.finish()
    }
}

impl AsteroidField {
    /// The solid the rock currently is, in unit space.
    pub fn solid(&self) -> &SignedField {
        &self.field
    }

    /// Take a mining sphere at `at` with `radius`, both in the field's unit
    /// space, and return how many corners it flipped from solid to empty.
    ///
    /// Only material still in the field counts: a corner a hit or an earlier
    /// pulse already emptied flips nothing. The change waits for the next
    /// remesh like a hit does, and [`AsteroidRemeshed`] says when it is drawn
    /// and collided with.
    pub fn mine(&mut self, at: Vec3, radius: f32) -> u32 {
        let flipped = self.field.subtract_sphere(at, radius);
        self.volume -= corner_volume(&self.field, flipped);
        if flipped > 0 {
            self.attempted = None;
        }
        flipped
    }
}

/// The cubic unit-space volume of `corners` cells of `field`.
fn corner_volume(field: &SignedField, corners: u32) -> f32 {
    corners as f32 * field.cell_size().powi(3)
}

/// A frozen asteroid's whole carved solid: the field itself and its remesh
/// bookkeeping, for [`crate::objects::asteroid::freeze_asteroid`] and
/// [`crate::objects::asteroid::thaw_asteroid`].
///
/// `pub(crate)`, not re-exported: the carve invariants
/// (`applied`/`attempted`/`meshed_volume` agreeing with `volume`) are this
/// module's to keep, so only the snapshot and its two restore paths cross the
/// module boundary, never the raw fields.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub(crate) struct AsteroidFieldSnapshot {
    field: SignedField,
    applied: u64,
    attempted: Option<u64>,
    volume: f32,
    meshed_volume: f32,
}

impl AsteroidField {
    /// Snapshot this field's whole state, for a freeze.
    pub(crate) fn snapshot(&self) -> AsteroidFieldSnapshot {
        AsteroidFieldSnapshot {
            field: self.field.clone(),
            applied: self.applied,
            attempted: self.attempted,
            volume: self.volume,
            meshed_volume: self.meshed_volume,
        }
    }
}

impl AsteroidFieldSnapshot {
    /// The surface this field meshes to, and the trimesh that surface makes,
    /// built with the calls `carve_surface` makes for a remesh. Pure, so a
    /// sector worker builds it before the main thread thaws the rock. `None`
    /// for the collider when the surface makes no usable trimesh.
    pub(crate) fn geometry(&self) -> (Mesh, Option<Collider>) {
        let surface = self.field.surface().build();
        let collider = Collider::trimesh_from_mesh(&surface);
        (surface, collider)
    }

    /// Rebuild the field exactly as it was snapshotted: no remesh pending.
    pub(crate) fn restored(self) -> AsteroidField {
        AsteroidField {
            field: self.field,
            applied: self.applied,
            attempted: self.attempted,
            volume: self.volume,
            meshed_volume: self.meshed_volume,
        }
    }

    /// Rebuild the field with its remesh forced to queue again: used when a
    /// remesh was in flight at freeze and its task was dropped rather than
    /// carried over. The solid already holds every mark - marks apply in
    /// place every frame, never on the worker - so `volume` is already short
    /// of `meshed_volume`; only `attempted` has to clear, since the live
    /// field set it to the in-flight candidate's signature to stop a second
    /// task from queuing behind the first one.
    pub(crate) fn restored_pending_remesh(self) -> AsteroidField {
        AsteroidField {
            field: self.field,
            applied: self.applied,
            attempted: None,
            volume: self.volume,
            meshed_volume: self.meshed_volume,
        }
    }
}

/// Asks for the pristine field of a rock node that no hit has marked, so a
/// mining pulse can carve it. Removed when the seed task starts.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AsteroidFieldSeedRequest;

/// A rock node's carve became observable: a remesh swapped in a validated
/// collider and mesh, or the rock ran out of material and is being despawned.
///
/// Triggered through `Commands` after the swap's inserts, so an observer sees
/// the new collider, and before the despawn, so an exhausted node still
/// exists.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct AsteroidRemeshed {
    /// The rock's mesh node, which carries the field.
    pub entity: Entity,
    /// True when the rock ran out of material and is being despawned.
    pub exhausted: bool,
}

/// The worst frame the swap-in has had, and what it took delivery of.
///
/// [`collect_asteroid_remeshes`] is the one step the module's off-thread
/// promise cannot cover - a finished carve has to become observable somewhere -
/// so the promise it makes instead is that the swap does no WORK. `grids` is
/// what witnesses that, and it is a count rather than a clock: a
/// [`SignedField`] is a quarter of a megabyte and every question you can ask
/// one is a scan of all of it, so a swap holding more grids than it has rocks
/// is a swap doing the worker's geometry on the main thread. That is invisible
/// in the result - the same rock, the same pieces, the same collider - and
/// differs only in which frame pays for it.
///
/// Ranked by `grids` and then by `millis`, so the record survives the frames
/// after it.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct CarveApplyReport {
    /// Finished carves that frame took delivery of.
    pub delivered: usize,
    /// Whole grids they handed it. ONE per delivery: the rock's own new solid.
    /// A piece arrives as finished geometry and brings no grid with it.
    pub grids: usize,
    /// Bodies that frame cut free.
    pub pieces: usize,
    /// What that frame's swap cost, in milliseconds. A fact about the HOST that
    /// ran it - record it, never gate on it.
    pub millis: f32,
}

impl CarveApplyReport {
    /// Keep `frame` if it is worse than what is already recorded.
    fn worst(&mut self, frame: Self) {
        if (frame.grids, frame.millis) > (self.grids, self.millis) {
            *self = frame;
        }
    }
}

/// Cells per axis for a rock whose field spans `half_extent` in its own unit
/// space and is drawn at `radius`.
///
/// The rock's own space and the world differ by exactly `radius` - the mesh node
/// carries `Transform::from_scale(radius)` - so this is the one place the two
/// meet: pick the count that puts a [`FIELD_CELL_WORLD`] cell across the world
/// extent, then clamp. Pure and cheap, so a caller can ask what a rock will cost
/// before paying for it.
fn field_resolution(half_extent: f32, radius: f32) -> usize {
    let across = 2.0 * half_extent * radius;
    ((across / FIELD_CELL_WORLD).round() as usize).clamp(FIELD_RESOLUTION_MIN, FIELD_RESOLUTION_MAX)
}

/// The pristine field of a rock with this `seed`, in the mesh node's own unit
/// space.
///
/// The ONE description of a rock's shape. The drawn mesh, the collider and the
/// carve field all come off this, so a rock cannot be one shape before it is hit
/// and another one after: [`pristine_rock_mesh`] is this function meshed, and
/// the reseed on the first hit is this function called again with the same seed.
/// Nothing has to be kept in step because there is nothing to keep in step with.
///
/// The near/far shortcut is not an approximation of the surface, it is a bound
/// on it: THIS rock's surface never comes closer in than its own nearest reach
/// or past its own furthest, so outside that shell the sign is settled without
/// asking the noise. The noise is the only expensive part, and the shell is
/// about a third of the grid.
pub(super) fn pristine_field(seed: u32, radius: f32) -> SignedField {
    let rock = RockHeight::default().with_seed(seed).sampler();
    let (nearest, furthest) = rock.reach();
    // The domain has to contain the whole rock: a surface that reached past it
    // would be clipped flat against the grid wall.
    let half_extent = furthest * FIELD_MARGIN;
    SignedField::sample(field_resolution(half_extent, radius), half_extent, |at| {
        let radius = at.length();
        // Inside the nearest the surface comes, or outside the furthest it
        // reaches: the sign is settled, and the value only has to carry it.
        if radius <= nearest {
            return radius - nearest;
        }
        if radius >= furthest {
            return radius - furthest;
        }
        radius - rock.radius(at / radius)
    })
}

/// The mesh a pristine rock with this `seed` is drawn and collided with.
///
/// Meshed from the same field a carve reads, so a rock's first hit changes the
/// CRATER and nothing else. The alternative - a subdivided octahedron displaced
/// by the same noise - was a different shape at a different triangle density,
/// and swapping one for the other on the first hit was a visible pop: the
/// silhouette moved by up to a cell and every facet in the rock changed size.
///
/// `radius` is what the rock is DRAWN at, and it is needed here for the same
/// reason: the grid is sized in world units, so the mesh a rock ships with is
/// the mesh its own field makes at its own size.
pub fn pristine_rock_mesh(seed: u32, radius: f32) -> Mesh {
    pristine_field(seed, radius).surface().build()
}

/// Everything a piece needs to know about the body it is leaving.
struct Parent {
    /// The mesh node itself, which is the body a crumb is announced against.
    node: Entity,
    /// The mesh node's frame: what turns a point in the field's unit space into
    /// a place in the world.
    frame: GlobalTransform,
    /// The body's own centre, which is what a piece's lever arm is measured
    /// from.
    centre: Vec3,
    /// How the body is moving, in world units per second.
    linear: Vec3,
    /// How the body is turning, in radians per second.
    angular: Vec3,
    /// The rock's own material, which its pieces wear too.
    ///
    /// The triplanar shader samples by the body's own LOCAL position, and a
    /// piece is a new body with a new origin - so a piece reads the rock's grain
    /// from a different place than the rock does. For a rock that is invisible:
    /// the grain is noise, and one patch of it looks like any other. What it
    /// buys is that the piece keeps sampling in ITS own space as it tumbles,
    /// which is what makes the texture sit still on it.
    ///
    /// `None` headless, where nothing is drawn at all.
    material: Option<MeshMaterial3d<AsteroidSurfaceMaterial>>,
    /// The rock's own kind, texture and seed, copied onto every piece it
    /// throws - see [`RockChunkSurface`].
    surface: RockChunkSurface,
}

/// One piece a carve cut free, already measured and - where it is big enough to
/// be a body - already meshed and collided.
///
/// Built on the worker that still holds the grid, and it is the whole reason a
/// piece does not travel as one. Every question you can ask a [`SignedField`]
/// is a scan of all of it: where its surface sits, how much it holds, what it
/// meshes to. Asked on the main thread, once per piece, that was a whole frame:
/// three rocks landing together with five pieces between them measured 17.5 ms
/// against a 0.02 ms median.
struct CarvedPiece {
    /// Its middle, in the PARENT's unit space, which is where it has to appear.
    at: Vec3,
    /// How much material it holds, in WORLD cubic units.
    volume: f32,
    /// Its geometry about its own middle, or `None` when it is too small to be
    /// a body and goes out as dust.
    body: Option<PieceBody>,
}

/// What a piece is drawn and collided with, in ITS own space.
struct PieceBody {
    /// The surface, recentred on the piece's own middle.
    mesh: Mesh,
    /// The collider that surface makes.
    collider: Collider,
}

/// Measure one severed island and, if it is worth simulating, build its body.
/// Pure, and run off the main thread.
///
/// A piece big enough to matter is meshed by the SAME surface nets the rock is,
/// off its own field - so it is exactly the geometry that left the rock, not an
/// approximation of it and not a generic lump. Recentred on its own middle,
/// because it is about to be a body and a body's origin should be inside it.
///
/// A piece SMALLER than [`CHUNK_MIN_VOLUME`] gets no body and is announced as a
/// carve instead, which turns it into dust. A cut across a rock does not end at
/// a clean line: it leaves crumbs all round the rim where the slab thinned out,
/// and a run of the gallery produced eighteen of them off one cut. Eighteen
/// rigid bodies of a few cells each is litter that costs a solver step; the same
/// eighteen as puffs of dust is what a cut through rock looks like anyway.
///
/// `cubic_scale` turns the grid's own cubic units into world ones. The
/// threshold is a world size and the grid does not know how big the rock it
/// came off is drawn, so the caller has to say.
fn sever_piece(island: &SignedField, cubic_scale: f32) -> Option<CarvedPiece> {
    let at = island.surface_centre()?;
    let volume = island.solid_volume() * cubic_scale;
    if volume < CHUNK_MIN_VOLUME {
        return Some(CarvedPiece {
            at,
            volume,
            body: None,
        });
    }

    let mut mesh = island.surface().build();
    // About its own middle: the field meshes in the ROCK's space, and a body
    // drawn far from its own origin tumbles about a point outside itself.
    mesh.translate_by(-at);

    let Some(collider) = chunk_collider(&mesh) else {
        trace!("sever_piece: a piece had no usable bounds, dropped");
        return None;
    };

    Some(CarvedPiece {
        at,
        volume,
        body: Some(PieceBody { mesh, collider }),
    })
}

/// Put every severed piece into the world.
///
/// Placement only: the geometry arrived built ([`sever_piece`]), so what is
/// left here is one transform and one spawn per piece.
///
/// Velocity is `v + omega x r`: a piece off a tumbling rock carries the tumble,
/// which is what makes it read as material that came loose rather than as
/// something spawned nearby. The spin it inherits outright - a rigid body's
/// pieces all turn at the body's rate.
fn throw_severed_pieces(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    parent: &Parent,
    pieces: Vec<CarvedPiece>,
) -> usize {
    let (scale, rotation, _) = parent.frame.to_scale_rotation_translation();
    let mut thrown = 0;

    for piece in pieces {
        let at = parent.frame.transform_point(piece.at);
        let Some(body) = piece.body else {
            commands.trigger(CarveSpew {
                entity: parent.node,
                at,
                // The crumb's own size, so the dust it becomes is the size of
                // the thing that crumbled.
                radius: (piece.volume * 3.0 / (4.0 * std::f32::consts::PI)).cbrt(),
                // A crumb is severed material, not a weapon's impact, and it is
                // seen leaving whatever cut the rock. Kinetic is the class
                // whose look IS chips off a solid, which is what a crumb is;
                // this is not a claim about what fired.
                kind: DamageType::Kinetic,
            });
            continue;
        };

        let spawned = spawn_carved_chunk(
            commands,
            ChunkSpawn {
                name: "Severed Rock".to_string(),
                mesh: meshes.add(body.mesh),
                transform: Transform {
                    translation: at,
                    rotation,
                    scale,
                },
                velocity: parent.linear + parent.angular.cross(at - parent.centre),
                spin: parent.angular,
                collider: body.collider,
            },
        );
        // A chunk is handed back undressed - see `spawn_carved_chunk`.
        if let Some(material) = parent.material.clone() {
            commands.entity(spawned).insert(material);
        }
        // Copied so a save can rebuild the chunk's material after its own
        // parent rock is gone - see `RockChunkSurface`.
        commands.entity(spawned).insert(parent.surface.clone());
        // Rock stops radio whether or not it is still attached to the rock it
        // came off. Cover a player shoots loose is still cover, and without
        // this a lock held through a severed island read straight through it.
        //
        // Here and not in `spawn_carved_chunk`: that module also throws metal
        // off a dying hull, and a ship's debris is not the radar shadow its
        // hull was. The occluder rides the collider, and the chunk grows its
        // own on this entity once it is clear of the parent.
        commands.entity(spawned).insert(RadarOccluder);
        thrown += 1;
    }

    thrown
}

/// Everything a remesh built off one candidate, waiting for the main thread to
/// make it observable.
///
/// The whole product of a carve travels in one piece, because the pieces have
/// to become visible together: a mesh without its collider is a rock rounds
/// fly through, and an island thrown without the parent's new surface is a
/// chunk that leaves a hole nothing filled.
struct CarvedSurface {
    /// The solid the surface was built from, islands already taken out.
    ///
    /// The ONE grid a swap takes delivery of. A piece brings none - see
    /// [`CarvedPiece`] and [`CarveApplyReport`].
    field: SignedField,
    /// The pieces the carve cut free, already built.
    pieces: Vec<CarvedPiece>,
    /// The surface that solid meshes to.
    surface: Mesh,
    /// Its trimesh, or `None` when the surface makes no usable collider.
    collider: Option<Collider>,
    /// How much solid is left, in the grid's own cubic units.
    volume: f32,
    /// How far that surface reaches, in the grid's own space.
    surviving: f32,
}

/// The pristine field a rock is waiting on, in flight on the compute pool.
#[derive(Component)]
pub(crate) struct AsteroidFieldSeeding(Task<SignedField>);

/// The remesh a rock has in flight, at most one at a time.
///
/// While it is here the rock keeps the mesh and collider it already had, and
/// takes no new work: the marks that land meanwhile are re-applied to whatever
/// the task hands back. That staleness is the same one the volume throttle
/// already allows, and it is what a carve costs instead of a frame.
#[derive(Component)]
pub(crate) struct AsteroidRemesh(Task<CarvedSurface>);

/// Split, mesh and collide one candidate solid AND everything it cut free.
/// Pure, and run off the main thread.
///
/// Everything: the swap-in that takes this result has to be able to place it
/// without asking a grid anything. `cubic_scale` is there for the same reason -
/// it is what turns a piece's volume into the world units its threshold is
/// written in.
///
/// `tracked` is the caller's running volume, which is right whenever nothing
/// severed - splitting is the one operation that removes material without
/// reporting how much, so it is the one case that pays for a scan.
fn carve_surface(
    node: Entity,
    mut candidate: SignedField,
    tracked: f32,
    cubic_scale: f32,
) -> CarvedSurface {
    let started = Instant::now();
    let islands = candidate.split_off_islands();
    let severed = started.elapsed();

    let volume = match islands.is_empty() {
        true => tracked,
        false => candidate.solid_volume(),
    };

    let started = Instant::now();
    let pieces: Vec<CarvedPiece> = islands
        .iter()
        .filter_map(|island| sever_piece(island, cubic_scale))
        .collect();
    let pieced = started.elapsed();

    let started = Instant::now();
    let built = candidate.surface();
    // Off the DRAWN surface, not off a second whole-grid pass over the cell
    // vertices it was just built from: same answer, and the margin keeps the
    // solid clear of the domain wall where a cell has no quad.
    let surviving = built
        .triangles
        .iter()
        .flat_map(|triangle| triangle.vertices)
        .fold(0.0f32, |furthest, vertex| furthest.max(vertex.length()));
    let surface = built.build();
    let remeshed = started.elapsed();

    let started = Instant::now();
    let collider = Collider::trimesh_from_mesh(&surface);
    let rebuilt = started.elapsed();

    // Per NODE. `collect_asteroid_remeshes` carries the frame's totals; this is
    // the breakdown for one grid, which only a reader chasing a specific carve
    // wants.
    trace!(
        "carve_surface: {node:?} sever {:.1} ms, pieces {:.1} ms, remesh {:.1} ms, \
         collider {:.1} ms, {} piece(s), {} tri(s), unit radius {surviving:.2}",
        severed.as_secs_f32() * 1000.0,
        pieced.as_secs_f32() * 1000.0,
        remeshed.as_secs_f32() * 1000.0,
        rebuilt.as_secs_f32() * 1000.0,
        pieces.len(),
        surface.indices().map_or(0, |i| i.len() / 3),
    );

    CarvedSurface {
        field: candidate,
        pieces,
        surface,
        collider,
        volume,
        surviving,
    }
}

/// Put the pristine field of every rock that has just been marked, or that a
/// miner asked for with [`AsteroidFieldSeedRequest`], in flight.
///
/// A rock is gridded only once it is shot at or mined, so this is where a
/// scenario's untouched hundred stay free. The seed is tens of thousands of
/// noise samples and it goes to the pool for the same reason the remesh does.
fn seed_asteroid_fields(
    mut commands: Commands,
    q_nodes: Query<
        (
            Entity,
            &DamageMarks,
            &ChildOf,
            Has<AsteroidFieldSeedRequest>,
        ),
        (Without<AsteroidField>, Without<AsteroidFieldSeeding>),
    >,
    q_asteroid: Query<(&AsteroidSeed, &AsteroidRadius), With<AsteroidMarker>>,
) {
    for (node, marks, ChildOf(root), requested) in &q_nodes {
        if marks.0.is_empty() && !requested {
            continue;
        }
        let Ok((seed, nominal)) = q_asteroid.get(*root) else {
            continue;
        };
        let (seed, radius) = (seed.0, nominal.0);
        let task = AsyncComputeTaskPool::get().spawn(async move { pristine_field(seed, radius) });
        commands
            .entity(node)
            .remove::<AsteroidFieldSeedRequest>()
            .insert(AsteroidFieldSeeding(task));
    }
}

/// Give a rock the field its seed task finished.
fn collect_asteroid_field_seeds(
    mut commands: Commands,
    mut q_seeding: Query<(Entity, &mut AsteroidFieldSeeding, &ChildOf)>,
    q_asteroid: Query<&AsteroidRadius, With<AsteroidMarker>>,
) {
    for (node, mut seeding, ChildOf(root)) in &mut q_seeding {
        let Some(seeded) = block_on(poll_once(&mut seeding.0)) else {
            continue;
        };
        let nominal = q_asteroid.get(*root).map_or(1.0, |radius| radius.0);
        trace!(
            "collect_asteroid_field_seeds: {node:?} at {}^3 ({:.2}u cells)",
            seeded.resolution(),
            seeded.cell_size() * nominal,
        );
        let volume = seeded.solid_volume();
        commands
            .entity(node)
            .remove::<AsteroidFieldSeeding>()
            .insert(AsteroidField {
                field: seeded,
                applied: 0,
                attempted: None,
                volume,
                meshed_volume: volume,
            });
    }
}

/// Carve every mark into the rock's own field, and put a remesh in flight when
/// the grid has lost a cell to a mark or a mining pulse.
///
/// Marks accumulate on every hit, but remeshing waits until the grid loses a
/// cell. A change the field cannot yet draw must not pay for connectivity,
/// surface generation or a collider rebuild.
///
/// NOT filtered on `Changed<DamageMarks>`, deliberately. Seeding the grid takes
/// a frame of its own - the insert lands on the next flush - and by then the
/// marks have not changed again, so a change filter would seed every rock and
/// carve none of them. The fingerprint compare that replaces it is a bounded
/// mark-list hash per rock per frame.
///
/// What is left here is the carve itself: a sphere subtraction over the cells
/// one mark reaches, which is bounded by the mark and not by the grid.
fn carve_asteroid_fields(
    mut commands: Commands,
    mut q_nodes: Query<
        (Entity, &DamageMarks, &mut AsteroidField, &GlobalTransform),
        Without<AsteroidRemesh>,
    >,
) {
    for (node, marks, mut field, frame) in &mut q_nodes {
        // A mined rock may carry no marks at all; its empty list still has a
        // signature, so the check below still sees the pulse.
        let signature = AsteroidField::signature(marks);
        if field.applied != signature {
            field.applied = signature;
            // EVERY mark, not just the ones that look new. Subtraction is a
            // max, so re-applying one already in the solid changes nothing -
            // and reports taking nothing, which is what keeps `volume` exact.
            for mark in &marks.0 {
                let flipped = field.field.subtract_sphere(mark.at, mark.radius);
                field.volume -= corner_volume(&field.field, flipped);
            }
        }

        // A changed distance below the grid's sign boundary cannot change the
        // surface topology. Keep accumulating it, but do not pay connectivity,
        // surface generation and collider rebuild until at least one cell is
        // observably gone.
        if field.attempted == Some(signature) || field.volume >= field.meshed_volume {
            continue;
        }
        field.attempted = Some(signature);

        // Work on a candidate. Splitting mutates a field; doing it to the live
        // one before collider validation can spawn duplicate islands and leave
        // the old mesh around a different internal solid.
        let candidate = field.field.clone();
        let tracked = field.volume;
        // The node's own scale, which is fixed at spawn: the grid is the rock's
        // unit space and the piece threshold is a world size, so the worker has
        // to be told the ratio it cannot see.
        let (scale, _, _) = frame.to_scale_rotation_translation();
        let cubic_scale = (scale.x * scale.y * scale.z).abs();
        let task = AsyncComputeTaskPool::get()
            .spawn(async move { carve_surface(node, candidate, tracked, cubic_scale) });
        commands.entity(node).insert(AsteroidRemesh(task));
    }
}

/// Swap in the surface a remesh finished: the drawn mesh, the collider, the
/// pieces it cut free, and the radius everything else measures the rock by.
///
/// This is the only place a carve becomes observable, so it is also where a
/// rock that ran out of material dies.
#[expect(
    clippy::type_complexity,
    reason = "the query carries the whole node: task, field, parent, frame and drawn art"
)]
fn collect_asteroid_remeshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut report: ResMut<CarveApplyReport>,
    mut q_remeshing: Query<(
        Entity,
        &mut AsteroidRemesh,
        &mut AsteroidField,
        &ChildOf,
        &GlobalTransform,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<AsteroidSurfaceMaterial>>,
    )>,
    q_asteroid: Query<
        (
            &AsteroidRadius,
            &BodyRadius,
            Option<&EntityId>,
            Option<&EntityTypeName>,
            &AsteroidKind,
            &AsteroidTexture,
            &AsteroidSeed,
        ),
        With<AsteroidMarker>,
    >,
    q_motion: Query<(
        &GlobalTransform,
        Option<&LinearVelocity>,
        Option<&AngularVelocity>,
    )>,
) {
    let started = Instant::now();
    let mut frame_cost = CarveApplyReport::default();

    for (node, mut remeshing, mut field, ChildOf(root), frame, mesh, chunk_material) in
        &mut q_remeshing
    {
        let Some(carved) = block_on(poll_once(&mut remeshing.0)) else {
            continue;
        };
        frame_cost.delivered += 1;
        // The rock's own new solid, and nothing else - see `CarveApplyReport`.
        frame_cost.grids += 1;
        commands.entity(node).remove::<AsteroidRemesh>();
        let Ok((nominal, body, id, type_name, kind, texture, seed)) = q_asteroid.get(*root) else {
            continue;
        };

        let (scale, _, _) = frame.to_scale_rotation_translation();
        let cubic_scale = (scale.x * scale.y * scale.z).abs();
        let remaining_world = carved.volume * cubic_scale;

        let (centre, linear, angular) = match q_motion.get(*root) {
            Ok((body, linear, angular)) => (
                body.translation(),
                linear.map_or(Vec3::ZERO, |velocity| velocity.0),
                angular.map_or(Vec3::ZERO, |velocity| velocity.0),
            ),
            Err(_) => (frame.translation(), Vec3::ZERO, Vec3::ZERO),
        };
        let parent = Parent {
            node,
            frame: *frame,
            centre,
            linear,
            angular,
            material: chunk_material.cloned(),
            surface: RockChunkSurface {
                kind: kind.0.clone(),
                texture: texture.0.clone(),
                seed: seed.0,
            },
        };

        if remaining_world < CHUNK_MIN_VOLUME || carved.surface.count_vertices() == 0 {
            trace!(
                "collect_asteroid_remeshes: {node:?} exhausted at {remaining_world:.2} cubic units"
            );
            // The candidate is terminal, so its islands commit together with the
            // root's destruction. Nothing is thrown for the solid that is left:
            // the hit that took it already threw the dust it was priced for, and
            // a second puff for the same material is the same round paid twice.
            frame_cost.pieces +=
                throw_severed_pieces(&mut commands, &mut meshes, &parent, carved.pieces);
            commands.trigger(AsteroidRemeshed {
                entity: node,
                exhausted: true,
            });
            // Reuse the common destruction cue seam without opting into its
            // health or random-fragment finale.
            commands.entity(node).insert(IntegrityDestroyMarker);
            if let (Some(id), Some(type_name)) = (id, type_name) {
                commands.fire::<OnDestroyedEvent>(OnDestroyedEventInfo {
                    id: id.to_string(),
                    type_name: type_name.to_string(),
                });
            }
            commands.entity(*root).try_despawn();
            continue;
        }
        let Some(collider) = carved.collider else {
            warn!(
                "collect_asteroid_remeshes: {node:?} candidate collider was unusable; \
                 kept prior state"
            );
            continue;
        };

        // Validation succeeded. Only now may the field and its pieces become
        // observable.
        frame_cost.pieces +=
            throw_severed_pieces(&mut commands, &mut meshes, &parent, carved.pieces);
        field.field = carved.field;
        field.volume = carved.volume;
        field.meshed_volume = carved.volume;
        // Marks that landed while the task was in flight were held off the live
        // field, so the whole list is re-applied to the solid that came back.
        field.applied = 0;

        let remeshed = AsteroidRemeshed {
            entity: node,
            exhausted: false,
        };
        let mut node = commands.entity(node);
        node.insert(collider);
        // The density rides along unchanged, so avian re-derives mass from the
        // volume that is actually left: a carved rock is a lighter rock.
        node.insert(ColliderDensity(1.0));
        // A `None` here is headless: the node never had a drawn mesh and must
        // not grow one.
        if mesh.is_some() {
            node.insert(Mesh3d(meshes.add(carved.surface)));
        }

        // Only ever DOWN. Everything sized off a rock's surface - standoff
        // distances, orbit clearances, the sphere of influence - was authored
        // against the pristine radius, so shrinking keeps every one of those
        // valid and growing would silently invalidate them.
        let shrunk = nominal.0 * carved.surviving;
        if shrunk < body.0 {
            commands.entity(*root).insert(BodyRadius(shrunk));
        }
        commands.trigger(remeshed);
    }

    if frame_cost.delivered > 0 {
        frame_cost.millis = started.elapsed().as_secs_f32() * 1000.0;
        debug!(
            "collect_asteroid_remeshes: {} delivery(s), {} grid(s), {} piece(s), {:.2} ms",
            frame_cost.delivered, frame_cost.grids, frame_cost.pieces, frame_cost.millis,
        );
        report.worst(frame_cost);
    }
}

// ---- Rock chunk freeze/thaw ----
//
// A chunk's field was built once by `sever_piece` and dropped the same frame
// (see the module docs' "Kept only while it is needed") - by the time a save
// freezes a live chunk there is no field left to re-mesh from, only the mesh
// it was already meshed into. So a chunk record keeps the raw vertex data
// (`FrozenChunkMesh`) rather than a field snapshot, unlike `FrozenAsteroid`'s
// `CarvedState`.

/// A carved rock chunk's own kind, texture and silhouette seed: the three
/// things that drive its material and are not derivable from the chunk's
/// mesh once it is a body of its own.
///
/// Copied from the parent rock's `AsteroidKind`/`AsteroidTexture`/
/// `AsteroidSeed` at spawn (`collect_asteroid_remeshes`, `throw_severed_pieces`),
/// because a resumed chunk may have no live parent rock left to clone a
/// material handle off of - see `thaw_rock_chunk`.
#[derive(Component, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RockChunkSurface {
    kind: AsteroidKindId,
    texture: AssetRef<Image>,
    seed: u32,
}

/// A chunk's drawn surface, as raw vertex data a save can keep.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FrozenChunkMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

/// A carved rock chunk's own state, as a save keeps it.
///
/// Built by [`freeze_rock_chunk`]; consumed once by [`thaw_rock_chunk`]. No
/// `visibility` field: nothing ever hides a live chunk (unlike a mining
/// beam or a stowed turret), so there is no state here to lose.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FrozenRockChunk {
    surface: RockChunkSurface,
    mesh: FrozenChunkMesh,
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
    linear: Vec3,
    angular: Vec3,
    /// [`ChunkGrace::remaining`] while the chunk is still drifting clear of
    /// what it left; `None` once it has landed (gone `RigidBody::Dynamic`
    /// and shed its `ChunkGrace`).
    grace: Option<f32>,
}

impl FrozenRockChunk {
    /// Check a record read from a file before a thaw draws it.
    ///
    /// # Errors
    ///
    /// A non-finite pose or velocity; a non-finite or negative `grace`; an
    /// empty mesh; a mesh whose positions and normals disagree in length; a
    /// mesh index count that is not a multiple of 3; a mesh index past its
    /// positions; or a non-finite mesh position or normal.
    pub fn validate(&self) -> Result<(), String> {
        if !(self.translation.is_finite()
            && self.rotation.is_finite()
            && self.scale.is_finite()
            && self.linear.is_finite()
            && self.angular.is_finite())
        {
            return Err("a rock chunk's pose or velocity is not finite".to_string());
        }
        if let Some(grace) = self.grace {
            if !(grace.is_finite() && grace >= 0.0) {
                return Err(format!(
                    "a rock chunk's grace of {grace} is not finite and non-negative"
                ));
            }
        }
        if self.mesh.positions.is_empty() {
            return Err("a rock chunk's mesh has no positions".to_string());
        }
        if self.mesh.positions.len() != self.mesh.normals.len() {
            return Err(format!(
                "a rock chunk's mesh has {} positions but {} normals",
                self.mesh.positions.len(),
                self.mesh.normals.len()
            ));
        }
        if self.mesh.indices.len() % 3 != 0 {
            return Err(format!(
                "a rock chunk's mesh has {} indices, which is not a multiple of 3",
                self.mesh.indices.len()
            ));
        }
        let vertex_count = self.mesh.positions.len();
        for &index in &self.mesh.indices {
            if index as usize >= vertex_count {
                return Err(format!(
                    "a rock chunk's mesh index {index} names a vertex past its {vertex_count} positions"
                ));
            }
        }
        if self
            .mesh
            .positions
            .iter()
            .any(|position| !Vec3::from_array(*position).is_finite())
            || self
                .mesh
                .normals
                .iter()
                .any(|normal| !Vec3::from_array(*normal).is_finite())
        {
            return Err("a rock chunk's mesh has a non-finite position or normal".to_string());
        }
        Ok(())
    }
}

/// `mesh`'s position, normal and index attributes, copied out for a save.
///
/// # Panics
///
/// If `mesh` carries no `Float32x3` `ATTRIBUTE_POSITION`, no `Float32x3`
/// `ATTRIBUTE_NORMAL`, or no index buffer: every chunk mesh is built by
/// `TriangleMeshBuilder::build` (`sever_piece`), which always writes all
/// three in that form, so a gap here means `entity` is not really a chunk.
fn frozen_chunk_mesh(entity: Entity, mesh: &Mesh) -> FrozenChunkMesh {
    let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(values)) => values.clone(),
        _ => panic!(
            "freeze_rock_chunk: chunk {entity}'s mesh carries no Float32x3 ATTRIBUTE_POSITION"
        ),
    };
    let normals = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(VertexAttributeValues::Float32x3(values)) => values.clone(),
        _ => {
            panic!("freeze_rock_chunk: chunk {entity}'s mesh carries no Float32x3 ATTRIBUTE_NORMAL")
        }
    };
    let indices = match mesh.indices() {
        Some(Indices::U32(indices)) => indices.clone(),
        Some(Indices::U16(indices)) => indices.iter().map(|&index| u32::from(index)).collect(),
        None => panic!("freeze_rock_chunk: chunk {entity}'s mesh carries no index buffer"),
    };
    FrozenChunkMesh {
        positions,
        normals,
        indices,
    }
}

/// The rock chunk `entity` is drawn and collided with, as a value a save can
/// keep.
///
/// # Errors
///
/// Never, today: a chunk carries no durable reference a save has to resolve
/// and nothing about it runs as a multi-frame process the way a raking
/// slug's tip does. The `Result` matches the other transient freezes
/// (`freeze_shed_fixture`, `freeze_detached_piece`) so a caller can treat
/// every transient kind the same way.
///
/// # Panics
///
/// If `entity` carries no [`RockChunkSurface`], [`Mesh3d`], [`Transform`],
/// [`LinearVelocity`] or [`AngularVelocity`] - `throw_severed_pieces` and
/// `land_carved_chunks` always leave a chunk with all five - or if its
/// `Mesh3d` handle names no live [`Mesh`]. A gap in any of these is a
/// programming error, not a save-time condition.
pub fn freeze_rock_chunk(
    world: &World,
    entity: Entity,
) -> Result<FrozenRockChunk, TransientFreezeFault> {
    let surface = world
        .get::<RockChunkSurface>(entity)
        .unwrap_or_else(|| panic!("freeze_rock_chunk: chunk {entity} carries no RockChunkSurface"))
        .clone();
    let transform = world
        .get::<Transform>(entity)
        .copied()
        .unwrap_or_else(|| panic!("freeze_rock_chunk: chunk {entity} carries no Transform"));
    let linear = world
        .get::<LinearVelocity>(entity)
        .unwrap_or_else(|| panic!("freeze_rock_chunk: chunk {entity} carries no LinearVelocity"))
        .0;
    let angular = world
        .get::<AngularVelocity>(entity)
        .unwrap_or_else(|| panic!("freeze_rock_chunk: chunk {entity} carries no AngularVelocity"))
        .0;
    let mesh_handle = &world
        .get::<Mesh3d>(entity)
        .unwrap_or_else(|| panic!("freeze_rock_chunk: chunk {entity} carries no Mesh3d"))
        .0;
    let mesh = world
        .resource::<Assets<Mesh>>()
        .get(mesh_handle)
        .unwrap_or_else(|| panic!("freeze_rock_chunk: chunk {entity}'s Mesh3d names no live Mesh"));
    let mesh = frozen_chunk_mesh(entity, mesh);
    let grace = world.get::<ChunkGrace>(entity).map(ChunkGrace::remaining);

    Ok(FrozenRockChunk {
        surface,
        mesh,
        translation: transform.translation,
        rotation: transform.rotation,
        scale: transform.scale,
        linear,
        angular,
        grace,
    })
}

/// `record`'s vertex data as a fresh [`Mesh`], built the same way
/// `TriangleMeshBuilder::build` does.
///
/// # Panics
///
/// If `record.positions` is empty, if `record.normals` disagrees with it in
/// length, or if an index in `record.indices` names a vertex past the end of
/// `record.positions`. [`FrozenRockChunk::validate`] already rejects a saved
/// chunk with any of these shapes before a thaw ever reaches here, so a
/// panic means the thaw was handed a chunk that skipped that check, not a
/// save-time condition.
fn rebuild_chunk_mesh(record: &FrozenChunkMesh) -> Mesh {
    assert!(
        !record.positions.is_empty(),
        "thaw_rock_chunk: a chunk record's mesh has no positions"
    );
    assert_eq!(
        record.positions.len(),
        record.normals.len(),
        "thaw_rock_chunk: a chunk record's positions and normals disagree in length"
    );
    let vertex_count = record.positions.len() as u32;
    assert!(
        record.indices.iter().all(|&index| index < vertex_count),
        "thaw_rock_chunk: a chunk record's indices name a vertex past its positions"
    );

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, record.positions.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, record.normals.clone())
    .with_inserted_indices(Indices::U32(record.indices.clone()))
}

/// Rebuild `record` as a live chunk: the exact mesh it was drawn with, a
/// fresh collider from that mesh, and the material a live rock of the saved
/// kind, texture and seed wears - built with the SAME helpers
/// `insert_asteroid_render` (`asteroid.rs`) builds one with, since a resumed
/// chunk has no live parent rock to clone a material handle off of.
///
/// `meshes`, `materials` and `asset_server` are explicit arguments rather
/// than system params so this stays unit-testable against bare `Assets`
/// fixtures. The caller borrows them once (a `SystemState` over the whole
/// resumed spawn) and applies the pass once, so indices and refs stay
/// stable.
///
/// # Panics
///
/// If `record.mesh` is invalid (see [`rebuild_chunk_mesh`]), if the rebuilt
/// mesh has no usable bounds for [`chunk_collider`], or if
/// `record.surface.kind` is not a known asteroid kind: a save never keeps
/// any of these, so a gap here is corrupt or hand-edited data. No
/// placeholder art.
pub fn thaw_rock_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<AsteroidSurfaceMaterial>,
    asset_server: &AssetServer,
    record: &FrozenRockChunk,
) -> Entity {
    let mesh = rebuild_chunk_mesh(&record.mesh);
    let collider = chunk_collider(&mesh)
        .unwrap_or_else(|| panic!("thaw_rock_chunk: a chunk's rebuilt mesh has no usable bounds"));
    let look = asteroid_kind_look(&record.surface.kind).unwrap_or_else(|| {
        panic!(
            "thaw_rock_chunk: a chunk is made of '{}', which is not a kind",
            record.surface.kind
        )
    });
    let image = record.surface.texture.resolve(asset_server);
    let material = AsteroidSurfaceMaterial {
        base: StandardMaterial::default(),
        extension: AsteroidSurfaceMaterialExt::new(image, &look, record.surface.seed),
    };

    let entity = spawn_carved_chunk(
        commands,
        ChunkSpawn {
            name: "Severed Rock".to_string(),
            mesh: meshes.add(mesh),
            transform: Transform {
                translation: record.translation,
                rotation: record.rotation,
                scale: record.scale,
            },
            velocity: record.linear,
            spin: record.angular,
            collider: collider.clone(),
        },
    );

    commands.entity(entity).insert((
        record.surface.clone(),
        MeshMaterial3d(materials.add(material)),
        RadarOccluder,
    ));

    match record.grace {
        Some(remaining) => {
            commands
                .entity(entity)
                .insert(ChunkGrace::resumed(collider, remaining));
        }
        None => {
            commands.entity(entity).remove::<ChunkGrace>().insert((
                RigidBody::Dynamic,
                collider,
                GravityAffected,
            ));
        }
    }

    entity
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
        trace!("AsteroidCarvePlugin: build");

        let _ = self.render;
        app.init_resource::<CarveApplyReport>();
        // Collect first, so a task started last frame has had a whole frame to
        // finish and the rock it belongs to is free to take the next mark in
        // the same update.
        app.add_systems(
            Update,
            (
                collect_asteroid_field_seeds,
                collect_asteroid_remeshes,
                seed_asteroid_fields,
                carve_asteroid_fields,
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in rock surface for a test that does not care which kind,
    /// texture or seed its chunk carries - only that it carries one.
    fn test_rock_chunk_surface() -> RockChunkSurface {
        RockChunkSurface {
            kind: AsteroidKindId::from(crate::objects::asteroid_kind::prelude::KIND_ROCK),
            texture: AssetRef::from("base/textures/asteroid_rock.png"),
            seed: 7,
        }
    }

    /// The seeded field has to be the rock that is already on screen, not a new
    /// one. Both the shipped mesh and the field are read off the SAME
    /// `RockHeight` sampler, so this pins the translation between them: the
    /// meshed surface must sit within a cell of where the sampler says the
    /// rock's surface is.
    #[test]
    fn the_seeded_field_reproduces_the_shipped_silhouette() {
        let seed = 4242;
        let rock = RockHeight::default().with_seed(seed).sampler();
        let field = pristine_field(seed, 1.5);
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

    /// A severed piece has to land where it was, at the size it was, carrying
    /// the motion it had. Everything about it is read off the rock's frame, so
    /// a piece off a rock at the far end of a scenario must not appear at the
    /// world origin at unit scale.
    ///
    /// Across the thread split on purpose: the island is measured and built the
    /// way the worker builds it, and only the result is handed to the placement
    /// half. A piece that survived the crossing wrongly would land exactly like
    /// one the placement half got wrong.
    #[test]
    fn a_severed_piece_carries_the_rock_it_left() {
        use bevy::ecs::system::RunSystemOnce;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();

        // A rock 100 units out, drawn at 4x, spinning about +Y.
        let scale = 4.0;
        let centre = Vec3::new(100.0, 0.0, 0.0);
        let angular = Vec3::Y * 2.0;
        // One island: a ball sitting off the rock's own middle in unit space.
        let offset = Vec3::new(2.0, 0.0, 0.0);
        let island = SignedField::sample(16, 4.0, move |at| at.distance(offset) - 1.0);
        let cell = island.cell_size();
        // What the worker hands over, built exactly as the carve builds it.
        let piece = sever_piece(&island, scale * scale * scale).expect("the island is a body");
        assert!(
            piece.body.is_some(),
            "delivery guard: the island is big enough to be a body, not dust"
        );
        // The throw CONSUMES its pieces; the closure it runs inside is FnMut.
        let mut piece = Some(piece);

        let rock = app.world_mut().spawn_empty().id();
        app.world_mut()
            .run_system_once(
                move |mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>| {
                    throw_severed_pieces(
                        &mut commands,
                        &mut meshes,
                        &Parent {
                            node: rock,
                            frame: GlobalTransform::from(
                                Transform::from_translation(centre).with_scale(Vec3::splat(scale)),
                            ),
                            centre,
                            linear: Vec3::Z * 5.0,
                            angular,
                            material: None,
                            surface: test_rock_chunk_surface(),
                        },
                        vec![piece.take().expect("the throw runs once")],
                    );
                },
            )
            .expect("the throw runs");

        let mut q_pieces = app
            .world_mut()
            .query_filtered::<(&Transform, &LinearVelocity), With<CarvedChunkMarker>>();
        let pieces: Vec<(Transform, Vec3)> = q_pieces
            .iter(app.world())
            .map(|(transform, velocity)| (*transform, velocity.0))
            .collect();

        assert_eq!(pieces.len(), 1, "the island became a body");
        let (transform, velocity) = pieces[0];
        let expected = centre + offset * scale;
        assert!(
            transform.translation.distance(expected) < cell * scale,
            "the piece landed at {} rather than {expected}",
            transform.translation
        );
        assert_eq!(
            transform.scale,
            Vec3::splat(scale),
            "and is drawn at the rock's own scale, not at unit scale"
        );
        // v + omega x r: the rock's drift plus the speed the spin was already
        // carrying that point at. Dropping the second term is what makes a
        // piece off a tumbling rock look spawned rather than shed.
        let expected = Vec3::Z * 5.0 + angular.cross(expected - centre);
        assert!(
            velocity.distance(expected) < 1.0,
            "the piece left at {velocity} rather than {expected}"
        );
        assert!(
            expected.distance(Vec3::Z * 5.0) > 1.0,
            "delivery guard: the spin contributes something to measure"
        );
    }

    /// Cover you shot loose is still cover. A severed island is rock standing
    /// in the same place it stood a frame ago, and a lock held through it read
    /// straight through because only the parent's hull wore the occluder.
    #[test]
    fn a_severed_island_still_stops_radio() {
        use bevy::ecs::system::RunSystemOnce;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();

        let island = SignedField::sample(16, 4.0, |at| at.distance(Vec3::new(2.0, 0.0, 0.0)) - 1.0);
        let piece = sever_piece(&island, 1.0).expect("the island is a body");
        assert!(
            piece.body.is_some(),
            "delivery guard: the island is a body, not dust"
        );
        let mut piece = Some(piece);

        let rock = app.world_mut().spawn_empty().id();
        app.world_mut()
            .run_system_once(
                move |mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>| {
                    throw_severed_pieces(
                        &mut commands,
                        &mut meshes,
                        &Parent {
                            node: rock,
                            frame: GlobalTransform::IDENTITY,
                            centre: Vec3::ZERO,
                            linear: Vec3::ZERO,
                            angular: Vec3::ZERO,
                            material: None,
                            surface: test_rock_chunk_surface(),
                        },
                        vec![piece.take().expect("the throw runs once")],
                    );
                },
            )
            .expect("the throw runs");

        let mut q_pieces = app
            .world_mut()
            .query_filtered::<Has<RadarOccluder>, With<CarvedChunkMarker>>();
        let occluding: Vec<bool> = q_pieces.iter(app.world()).collect();
        assert_eq!(occluding, vec![true], "a thrown island is opaque to radar");
    }

    /// The rule that keeps gravity and navigation valid without recomputing
    /// them: carving removes material, so a rock's published radius can only
    /// ever fall.
    #[test]
    fn a_carved_rock_never_grows() {
        let mut field = pristine_field(7, 1.5);
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

    /// The rule the whole grid rests on: a rock is gridded in WORLD units, so
    /// the crater a shipped round carves is representable on every rock in a
    /// fight rather than only on the small ones.
    ///
    /// The cap is part of the claim, not an exception to it: past about radius
    /// 1.8 the cell grows, and what that costs has to be visible here rather
    /// than discovered when a big rock stops taking marks.
    #[test]
    fn a_rock_is_gridded_in_world_units_until_the_cap_binds() {
        let half_extent = RockHeight::default()
            .with_seed(20260817)
            .sampler()
            .reach()
            .1
            * FIELD_MARGIN;
        let cell =
            |radius: f32| 2.0 * half_extent * radius / field_resolution(half_extent, radius) as f32;

        // Every rock a shipped scenario scatters up to where the cap binds.
        for radius in [0.8f32, 1.2, 1.5, 1.8] {
            let cell = cell(radius);
            assert!(
                (cell - FIELD_CELL_WORLD).abs() < FIELD_CELL_WORLD * 0.05,
                "radius {radius} grids at a {cell:.3}u cell, not {FIELD_CELL_WORLD}"
            );
            // And the round that has to show on it.
            assert!(mark_radius(4.0) > cell * 0.5);
        }

        // Past the cap the cell grows and one PDC round is under it, so a
        // bigger rock is marked by held fire rather than by a round.
        let biggest = cell(3.0);
        assert_eq!(field_resolution(half_extent, 3.0), FIELD_RESOLUTION_MAX);
        assert!(
            biggest > FIELD_CELL_WORLD && biggest < 1.0,
            "a radius-3 rock grids at {biggest:.3}u"
        );

        // And the floor holds a silhouette on something smaller than anything
        // authored.
        assert_eq!(field_resolution(half_extent, 0.1), FIELD_RESOLUTION_MIN);
    }

    /// An authored mobile rock that runs out of material takes everything that
    /// named it: its node, gravity marker and scenario id, with one
    /// `OnDestroyed`. The rock dies through the carve chain - seed, carve,
    /// remesh - and not through a despawn the test issues.
    #[test]
    fn an_exhausted_authored_rock_removes_its_gravity_marker_and_id() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };

        use crate::{actions::scoped_entities, prelude::*, test_support::drain_spawns};

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
        app.init_asset::<Mesh>();
        app.add_plugins((
            AsteroidPlugin { render: false },
            AsteroidCarvePlugin { render: false },
        ));
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        let destroyed = Arc::new(AtomicUsize::new(0));
        let heard = destroyed.clone();
        app.add_observer(move |event: On<GameEvent>| {
            if event.event().name() == OnDestroyedEvent::name() {
                heard.fetch_add(1, Ordering::SeqCst);
            }
        });

        let authored = ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "rock".to_string(),
                name: "Rock".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                kind: KIND_ROCK.into(),
                destroy_sound: None,
                radius: Meters(200.0),
                texture: AssetRef::default(),
                initial_velocity: MetersPerSecond3::ZERO,
                seed: None,
                lock_signature: None,
            }),
        };
        authored.action(
            &mut app.world_mut().resource_mut::<NovaEventWorld>(),
            &GameEventInfo::default(),
        );
        drain_spawns(app.world_mut());
        app.update();

        let [root] = scoped_entities(app.world_mut(), "rock")[..] else {
            panic!("the authored id must resolve to one rock");
        };
        assert!(app.world().get::<GravityAffected>(root).is_some());
        assert!(app.world().get::<GravityWell>(root).is_none());
        let children: Vec<Entity> = app
            .world()
            .get::<Children>(root)
            .expect("the rock has its collider node")
            .iter()
            .collect();
        let node = children
            .iter()
            .copied()
            .find(|child| app.world().get::<DamageMarks>(*child).is_some())
            .expect("the collider node takes marks");

        // One mark wider than any rock reaches in its own unit space.
        app.world_mut()
            .get_mut::<DamageMarks>(node)
            .expect("the node takes marks")
            .0
            .push(DamageMark {
                at: Vec3::ZERO,
                radius: 2.0 * ASTEROID_GEOMETRIC_FACTOR_MAX,
            });

        // The seed and the remesh run on the task pool. The cap names a hung
        // chain; it is not a budget.
        for _ in 0..10_000 {
            if app.world().get_entity(root).is_err() {
                break;
            }
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(
            app.world().get_entity(root).is_err(),
            "the exhausted rock must despawn"
        );
        // Frames after the death, so a second OnDestroyed would be heard.
        app.update();
        app.update();

        for child in children {
            assert!(app.world().get_entity(child).is_err(), "{child} survived");
        }
        assert_eq!(
            app.world_mut()
                .query_filtered::<(), With<GravityAffected>>()
                .iter(app.world())
                .count(),
            0,
            "the mobile rock's gravity marker dies with its body"
        );
        assert!(scoped_entities(app.world_mut(), "rock").is_empty());
        assert_eq!(destroyed.load(Ordering::SeqCst), 1);
    }

    // ---- Rock chunk freeze/thaw ----

    /// A resumed rock chunk has to look and move exactly like the live one it
    /// was: same geometry, the same collider bounds rebuilt from it, the same
    /// material inputs, the same visibility, and the same grace. Covers both
    /// ends of `FrozenRockChunk::grace` - a chunk still drifting clear, and
    /// one that has already landed.
    ///
    /// The chunk is thrown through the real `throw_severed_pieces`, the same
    /// production path `a_severed_piece_carries_the_rock_it_left` exercises -
    /// not a reimplementation of it.
    #[cfg(feature = "serde")]
    #[test]
    fn a_resumed_rock_chunk_has_its_mesh_material_and_grace() {
        use bevy::ecs::system::RunSystemOnce;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Image>();
        app.init_asset::<AsteroidSurfaceMaterial>();

        let surface = test_rock_chunk_surface();
        let island = SignedField::sample(16, 4.0, |at| at.distance(Vec3::new(2.0, 0.0, 0.0)) - 1.0);
        let piece = sever_piece(&island, 1.0).expect("the island is a body");
        assert!(
            piece.body.is_some(),
            "delivery guard: the island is a body, not dust"
        );
        let mut piece = Some(piece);

        let rock = app.world_mut().spawn_empty().id();
        let thrown_surface = surface.clone();
        app.world_mut()
            .run_system_once(
                move |mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>| {
                    throw_severed_pieces(
                        &mut commands,
                        &mut meshes,
                        &Parent {
                            node: rock,
                            frame: GlobalTransform::IDENTITY,
                            centre: Vec3::ZERO,
                            linear: Vec3::new(1.0, 0.0, 0.0),
                            angular: Vec3::Y,
                            material: None,
                            surface: thrown_surface.clone(),
                        },
                        vec![piece.take().expect("the throw runs once")],
                    );
                },
            )
            .expect("the throw runs");

        let chunk = app
            .world_mut()
            .query_filtered::<Entity, With<CarvedChunkMarker>>()
            .iter(app.world())
            .next()
            .expect("the island became a body");

        let live_grace = app
            .world()
            .get::<ChunkGrace>(chunk)
            .expect("a freshly thrown chunk is still in grace")
            .remaining();
        // No required-component chain inserts `Visibility` on a chunk, so a
        // live one never carries one either - the absence IS the proof that
        // nothing hides a chunk (see `FrozenRockChunk`'s doc).
        let live_visibility = app.world().get::<Visibility>(chunk).copied();
        let live_mesh = {
            let handle = &app.world().get::<Mesh3d>(chunk).unwrap().0;
            app.world()
                .resource::<Assets<Mesh>>()
                .get(handle)
                .expect("the thrown chunk's mesh handle resolves")
                .clone()
        };
        let live_bounds = nova_gameplay::integrity::chunk::mesh_bounds(&live_mesh)
            .expect("the thrown chunk's mesh has bounds");

        let frozen = freeze_rock_chunk(app.world(), chunk).expect("a chunk always freezes");
        assert_eq!(
            frozen.grace,
            Some(live_grace),
            "a chunk still in grace freezes its remaining window"
        );

        let ron = ron::to_string(&frozen).expect("serialize");
        println!("a rock chunk record is {} RON bytes", ron.len());
        let record: FrozenRockChunk = ron::from_str(&ron).expect("deserialize");

        app.world_mut().entity_mut(chunk).despawn();

        let thaw = |world: &mut World, record: FrozenRockChunk| -> Entity {
            world
                .run_system_once(
                    move |mut commands: Commands,
                          mut meshes: ResMut<Assets<Mesh>>,
                          mut materials: ResMut<Assets<AsteroidSurfaceMaterial>>,
                          asset_server: Res<AssetServer>| {
                        thaw_rock_chunk(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &asset_server,
                            &record,
                        )
                    },
                )
                .expect("the thaw runs")
        };

        let thawed = thaw(app.world_mut(), record.clone());
        app.world_mut().flush();

        let thawed_mesh = {
            let handle = &app.world().get::<Mesh3d>(thawed).unwrap().0;
            app.world()
                .resource::<Assets<Mesh>>()
                .get(handle)
                .expect("the thawed chunk's mesh handle resolves")
                .clone()
        };
        let thawed_extracted = frozen_chunk_mesh(thawed, &thawed_mesh);
        assert_eq!(
            thawed_extracted.positions, record.mesh.positions,
            "positions did not round-trip"
        );
        assert_eq!(
            thawed_extracted.normals, record.mesh.normals,
            "normals did not round-trip"
        );
        assert_eq!(
            thawed_extracted.indices, record.mesh.indices,
            "indices did not round-trip"
        );

        let thawed_bounds = nova_gameplay::integrity::chunk::mesh_bounds(&thawed_mesh)
            .expect("the thawed chunk's mesh has bounds");
        assert_eq!(
            thawed_bounds, live_bounds,
            "the rebuilt collider's bounds (same mesh, same chunk_collider) must match the live \
             chunk's"
        );

        assert!(
            app.world()
                .get::<MeshMaterial3d<AsteroidSurfaceMaterial>>(thawed)
                .is_some(),
            "a thaw must build and insert a material, not leave the chunk undressed"
        );
        assert_eq!(
            asteroid_kind_look(&record.surface.kind),
            asteroid_kind_look(&surface.kind),
            "the material's kind look did not round-trip"
        );
        assert_eq!(
            record.surface.seed, surface.seed,
            "the material's seed did not round-trip"
        );
        assert_eq!(
            record.surface.texture.path(),
            surface.texture.path(),
            "the material's texture path did not round-trip"
        );

        assert_eq!(
            app.world().get::<Visibility>(thawed).copied(),
            live_visibility,
            "visibility did not round-trip"
        );
        assert_eq!(
            app.world()
                .get::<ChunkGrace>(thawed)
                .map(ChunkGrace::remaining),
            Some(live_grace),
            "grace remaining did not round-trip"
        );
        assert!(
            app.world()
                .get::<RigidBody>(thawed)
                .is_none_or(|body| *body == RigidBody::Kinematic),
            "a chunk still in grace must not already be dynamic"
        );

        // ---- Landed case: no ChunkGrace, a real collider, RigidBody::Dynamic ----

        let landed_record = FrozenRockChunk {
            grace: None,
            ..record.clone()
        };
        let landed = thaw(app.world_mut(), landed_record);
        app.world_mut().flush();

        assert_eq!(
            app.world().get::<RigidBody>(landed).copied(),
            Some(RigidBody::Dynamic),
            "a landed chunk must thaw straight to a dynamic body"
        );
        assert!(
            app.world().get::<Collider>(landed).is_some(),
            "a landed chunk must thaw with a real collider"
        );
        assert!(
            app.world().get::<ChunkGrace>(landed).is_none(),
            "a landed chunk must carry no ChunkGrace"
        );
    }
}
