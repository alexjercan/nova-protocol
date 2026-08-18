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
//! It used to be two shapes: a subdivided octahedron displaced by the noise for
//! the shipped mesh, and this field for the carved one. They agreed to within a
//! cell, which is not the same as agreeing - the first hit on a rock moved its
//! silhouette and changed the size of every facet on it, and that pop was
//! visible on a rock the shot had barely scratched.
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
//! # What it swaps, and what it must not break
//!
//! A remesh replaces the drawn `Mesh3d` and rebuilds the trimesh `Collider` -
//! the same `trimesh_from_mesh` call the spawn path uses, so mass-from-volume
//! stays honest and a carved rock weighs what is left of it. `BodyRadius` is
//! re-derived and only ever SHRINKS, which is what keeps gravity spheres of
//! influence and orbit bands valid without recomputing them: everything sized
//! off a rock's surface was authored against a bigger rock than the one that is
//! there now.

use avian3d::prelude::{AngularVelocity, Collider, ColliderDensity, LinearVelocity};
use bevy::prelude::*;
use nova_events::prelude::{CommandsGameEventExt, *};
use nova_gameplay::prelude::*;
use nova_ship::prelude::BodyRadius;

use super::{
    asteroid::{AsteroidMarker, AsteroidRadius, AsteroidSeed},
    asteroid_surface::prelude::{AsteroidSurfaceMaterial, RockHeight},
};

/// `AsteroidField`, `AsteroidCarvePlugin` and the rock mesh they share with the
/// spawn path.
pub mod prelude {
    pub use super::{pristine_rock_mesh, AsteroidCarvePlugin, AsteroidField};
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
/// At `64^3` one carve measured 43 ms of main-thread work on one desktop core
/// - 17 to remesh 28,000 triangles, 11 to rebuild the collider, 3 to test
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
/// and collider already live in - the node carries `Transform::from_scale(radius)`
/// - and it is the frame [`DamageMarks`] on the same entity are recorded in.
/// Nothing here has to know how big the rock is.
#[derive(Component, Debug)]
pub struct AsteroidField {
    /// The solid, carved by every mark seen so far.
    field: SignedField,
    /// A fingerprint of the mark list already applied to `field`.
    ///
    /// NOT a count. Repeated fire grows a mark's radius without growing the
    /// list, and a count would call that "nothing new" and never carve it.
    applied: u64,
    /// Mark signature whose candidate was last attempted.
    ///
    /// A rejected surface waits for another mark rather than rebuilding the
    /// same unusable collider every frame.
    attempted: u64,
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
}

/// Put every severed piece into the world.
///
/// A piece big enough to be worth simulating becomes a body of its own, meshed
/// by the SAME surface nets the rock is, off its own field - so it is exactly
/// the geometry that left the rock, not an approximation of it and not a
/// generic lump. Recentred on its own middle, because it is about to be a body
/// and a body's origin should be inside it.
///
/// Velocity is `v + omega x r`: a piece off a tumbling rock carries the tumble,
/// which is what makes it read as material that came loose rather than as
/// something spawned nearby. The spin it inherits outright - a rigid body's
/// pieces all turn at the body's rate.
///
/// A piece SMALLER than [`CHUNK_MIN_VOLUME`] is announced as a carve instead,
/// which turns it into dust. A cut across a rock does not end at a clean line:
/// it leaves crumbs all round the rim where the slab thinned out, and a run of
/// the gallery produced eighteen of them off one cut. Eighteen rigid bodies of
/// a few cells each is litter that costs a solver step; the same eighteen as
/// puffs of dust is what a cut through rock looks like anyway.
fn throw_severed_pieces(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    parent: &Parent,
    islands: &[SignedField],
) {
    // Volumes are measured in the rock's own unit space, so they have to be
    // scaled into world units before the world's threshold means anything.
    let (scale, rotation, _) = parent.frame.to_scale_rotation_translation();
    let cubic = scale.x * scale.y * scale.z;

    for island in islands {
        let Some(middle) = island.surface_centre() else {
            continue;
        };
        let at = parent.frame.transform_point(middle);
        let volume = island.solid_volume() * cubic;
        if volume < CHUNK_MIN_VOLUME {
            commands.trigger(CarveSpew {
                entity: parent.node,
                at,
                // The crumb's own size, so the dust it becomes is the size of
                // the thing that crumbled.
                radius: (volume * 3.0 / (4.0 * std::f32::consts::PI)).cbrt(),
            });
            continue;
        }

        let mut mesh = island.surface().build();
        // About its own middle: the field meshes in the ROCK's space, and a
        // body drawn far from its own origin tumbles about a point outside
        // itself.
        mesh.translate_by(-middle);

        let Some(collider) = chunk_collider(&mesh) else {
            debug!("throw_severed_pieces: a piece had no usable bounds, dropped");
            continue;
        };

        let piece = spawn_carved_chunk(
            commands,
            ChunkSpawn {
                name: "Severed Rock".to_string(),
                mesh: meshes.add(mesh),
                transform: Transform {
                    translation: at,
                    rotation,
                    scale,
                },
                velocity: parent.linear + parent.angular.cross(at - parent.centre),
                spin: parent.angular,
                collider,
            },
        );
        // A chunk is handed back undressed - see `spawn_carved_chunk`.
        if let Some(material) = parent.material.clone() {
            commands.entity(piece).insert(material);
        }
    }
}

/// Give a rock its field the first time it is marked, then keep its mesh,
/// collider and published radius in step with the marks.
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
/// SYNCHRONOUS. The design's plan is to run the remesh and the collider build
/// on the async compute pool with at most one job in flight per rock; that is
/// worth doing when the numbers say so, and the numbers have to come first.
/// This logs what each stage costs so the decision is made on measurements
/// rather than on the estimate that motivated the plan.
#[expect(
    clippy::type_complexity,
    reason = "the query carries the whole node: marks, field, parent, frame and drawn art"
)]
fn carve_asteroid_fields(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut q_nodes: Query<(
        Entity,
        &DamageMarks,
        Option<&mut AsteroidField>,
        &ChildOf,
        &GlobalTransform,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<AsteroidSurfaceMaterial>>,
    )>,
    q_asteroid: Query<
        (
            &AsteroidSeed,
            &AsteroidRadius,
            &BodyRadius,
            Option<&EntityId>,
            Option<&EntityTypeName>,
        ),
        With<AsteroidMarker>,
    >,
    q_motion: Query<(
        &GlobalTransform,
        Option<&LinearVelocity>,
        Option<&AngularVelocity>,
    )>,
) {
    for (node, marks, field, ChildOf(root), frame, mesh, chunk_material) in &mut q_nodes {
        if marks.0.is_empty() {
            continue;
        }
        let Ok((seed, nominal, body, id, type_name)) = q_asteroid.get(*root) else {
            continue;
        };

        let mut field = match field {
            Some(field) => field,
            None => {
                let started = std::time::Instant::now();
                let seeded = pristine_field(seed.0, nominal.0);
                debug!(
                    "carve_asteroid_fields: seeded {node:?} at {}^3 ({:.2}u cells) in {:.1} ms",
                    seeded.resolution(),
                    seeded.cell_size() * nominal.0,
                    started.elapsed().as_secs_f32() * 1000.0
                );
                let meshed_volume = seeded.solid_volume();
                commands.entity(node).insert(AsteroidField {
                    field: seeded,
                    applied: 0,
                    attempted: 0,
                    meshed_volume,
                });
                // The insert lands next flush, and this rock is carved the
                // frame after. One frame of lag on the first hit only, and it
                // keeps the seeding cost out of the same frame as the first
                // remesh rather than paying for both at once.
                continue;
            }
        };

        let signature = AsteroidField::signature(marks);
        if field.applied != signature {
            field.applied = signature;
            // EVERY mark, not just the ones that look new. Subtraction is a
            // max, so re-applying one already in the solid changes nothing.
            for mark in &marks.0 {
                field.field.subtract_sphere(mark.at, mark.radius);
            }
        }

        // A changed distance below the grid's sign boundary cannot change the
        // surface topology. Keep accumulating it, but do not pay connectivity,
        // surface generation and collider rebuild until at least one cell is
        // observably gone.
        if field.field.solid_volume() >= field.meshed_volume || field.attempted == signature {
            continue;
        }
        field.attempted = signature;

        // Work on a candidate. Splitting mutates a field; doing it to the live
        // one before collider validation can spawn duplicate islands and leave
        // the old mesh around a different internal solid.
        let carve_started = std::time::Instant::now();
        let mut candidate = field.field.clone();
        let started = std::time::Instant::now();
        let islands = candidate.split_off_islands();
        let severed = started.elapsed();
        if !islands.is_empty() {
            debug!(
                "carve_asteroid_fields: {node:?} found {} severed piece(s) in {:.1} ms",
                islands.len(),
                severed.as_secs_f32() * 1000.0
            );
        }

        let (scale, _, _) = frame.to_scale_rotation_translation();
        let cubic_scale = (scale.x * scale.y * scale.z).abs();
        let remaining_world = candidate.solid_volume() * cubic_scale;
        let started = std::time::Instant::now();
        let surface = candidate.surface().build();
        let remeshed = started.elapsed();

        let started = std::time::Instant::now();
        let collider = Collider::trimesh_from_mesh(&surface);
        let rebuilt = started.elapsed();

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
        };

        if remaining_world < CHUNK_MIN_VOLUME || surface.count_vertices() == 0 {
            debug!("carve_asteroid_fields: {node:?} exhausted at {remaining_world:.2} cubic units");
            // The candidate is terminal, so its islands and final dust commit
            // together with the root's destruction.
            throw_severed_pieces(&mut commands, &mut meshes, &parent, &islands);
            if remaining_world > 0.0 {
                commands.trigger(CarveSpew {
                    entity: node,
                    at: candidate
                        .surface_centre()
                        .map_or(frame.translation(), |at| frame.transform_point(at)),
                    radius: (remaining_world * 3.0 / (4.0 * std::f32::consts::PI)).cbrt(),
                });
            }
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
        let Some(collider) = collider else {
            warn!(
                "carve_asteroid_fields: {node:?} candidate collider was unusable; kept prior state"
            );
            continue;
        };

        // Validation succeeded. Only now may the field and its islands become
        // observable.
        throw_severed_pieces(&mut commands, &mut meshes, &parent, &islands);
        let surviving = candidate.surface_radius();
        field.field = candidate;
        field.meshed_volume = field.field.solid_volume();
        debug!(
            "carve_asteroid_fields: {node:?} carve {:.1} ms (sever {:.1}, remesh {:.1}, \
             collider {:.1}), {} tri(s), unit radius {surviving:.2}",
            carve_started.elapsed().as_secs_f32() * 1000.0,
            severed.as_secs_f32() * 1000.0,
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
                        },
                        std::slice::from_ref(&island),
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
}
