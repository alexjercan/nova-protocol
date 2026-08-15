//! The SHAPE of a hull shell tile: eight boundary samples, and the mesh they
//! define.
//!
//! A shell is one cell of a ship's outer skin. Its top is a height field
//! sampled on the BOUNDARY of the cell's outward face at eight points - the
//! four corners and the four edge midpoints - each standing on one of the five
//! quarter cells between the floor and the ceiling. Those eight digits are the
//! whole shape, and they are what a shell's id spells:
//!
//! ```text
//!            m0
//!        c0------c1
//!        |        |
//!     m3 |        | m1        shell_<c0 c1 c2 c3>_<m0 m1 m2 m3>
//!        |        |
//!        c3------c2
//!            m2
//! ```
//!
//! Every sample is SHARED with the cells around it - a corner with the four
//! that meet at it, a midpoint with the two that meet across it - so every
//! neighbour builds the same boundary points out of its own state and two
//! surfaces meet exactly. That is what makes a run of tiles one continuous hull
//! rather than a heap of plates, and it is why nothing has to generate a seam
//! between two tiles in the same plane: there is no gap to fill.
//!
//! The MIDDLE of the top is shared with nobody, so it is free. It rides at the
//! mean of the eight boundary samples, which is the one choice that leaves a
//! ramp flat: corners at floor, floor, full, full with the midpoints their
//! straight line asks for average to exactly the height the plane passes the
//! middle at. The minimum would crater it and the maximum would tent it.
//!
//! A quarter turn about the out axis is free to whatever places the tile, so
//! two settings that differ by one are ONE shape - see
//! [`ShellShape::canonical`]. Neither reflection nor height is a symmetry: a
//! section is placed with a rotation and nothing else, so a shape and its
//! mirror are two shapes, and a bump is not a dent.
//!
//! Meshes are BUILT, not shipped, and built on demand. Nothing enumerates the
//! shapes: a whole ship touches a couple of dozen of them, so a tile costs less
//! to generate than to load and the size of the space it is drawn from does not
//! matter.

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

use crate::sections::link_points::{unit_cube_link_points, LinkPoint};

/// The heights a boundary sample can stand at, indexed by the digit that names
/// it: quarter cells, from the floor to a whole cell.
///
/// QUARTERS and not the three the corners themselves use, because the corners
/// are not the only samples. The midpoint of an edge climbing from the floor to
/// the half cell belongs at the mean of its two corners, and that mean is a
/// quarter. With three heights it had to round back down to the floor, which
/// sagged the middle of every rim edge and grew a facet on every tile at the
/// edge of a skin.
pub const SAMPLE_HEIGHTS: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// The digit of a sample standing a full cell high, which is what it takes to
/// close against structure a whole cell tall.
pub const FULL: u8 = 4;

/// The digit of a sample standing half a cell high - where a face-centre socket
/// sits, and the height a run of skin travels at.
pub const HALF: u8 = 2;

/// Half the cell, which is the tile's reach from its own centre on every axis.
/// The cell floor is at `-REACH` and its ceiling at `+REACH` - `pub(crate)`
/// because [`super::shell_skin`] builds a collider round the tile and has to
/// stand it on the same floor the mesh does.
pub(crate) const REACH: f32 = 0.5;

/// Every shell id opens with this.
pub const SHELL_ID_PREFIX: &str = "shell_";

/// Separates the corner round from the midpoint round in an id.
const ROUND_SEPARATOR: char = '_';

/// The four corners of the outward face, as signs on x and z, in the order an
/// id reads them. A quarter turn about the out axis is one step along this
/// list.
const FACE_CORNERS: [(f32, f32); 4] = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)];

/// The face a shell bolts down through. It keeps the whole cell floor whatever
/// its top is doing, so this socket is on every tile.
const BOLT_NORMAL: Vec3 = Vec3::NEG_Y;

/// Enough to tell the six cardinal directions apart. The normals compared
/// against it are authored constants, so nothing here is close to the limit.
const SOCKET_EPSILON: f32 = 1e-4;

/// The surface roles a generated tile is cut into, one mesh each.
///
/// A role per mesh rather than a colour per vertex, which is what the authored
/// `.glb` tiles already do - one primitive per material. Vertex colours would
/// be one mesh and one material for the whole family, but bevy's PBR fragment
/// ASSIGNS `base_color` from the vertex colour instead of multiplying by it,
/// and [`super::damage_tint`] grades a section by writing that same
/// `base_color`. A vertex-coloured shell would quietly stop reading as damaged
/// or destroyed, which is a gameplay signal and not just a look. Nor would it
/// buy anything: `damage_tint` hands every section a private clone of its
/// material, so sections do not batch with each other whatever the mesh does.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
pub enum ShellSurface {
    /// The cell floor the tile bolts down through. Never seen - it is against
    /// the section it clads.
    Floor,
    /// The sides, which a neighbouring tile is usually up against.
    Wall,
    /// The face shown to vacuum, which is the only one that is always in shot.
    Top,
}

impl ShellSurface {
    /// The flat colour a tile wears where nobody has dressed it. LINEAR values,
    /// so they sit well below what they read as on screen - a cool dark slate
    /// against the warm grey of the structure underneath.
    pub fn colour(self) -> Color {
        match self {
            Self::Floor => Color::linear_rgb(0.030, 0.033, 0.042),
            Self::Wall => Color::linear_rgb(0.088, 0.104, 0.138),
            Self::Top => Color::linear_rgb(0.105, 0.125, 0.165),
        }
    }
}

/// One shell tile's shape: the eight heights on the boundary of its outward
/// face.
///
/// `corners[i]` is the corner at [`FACE_CORNERS`]`[i]`, and `midpoints[i]` is
/// the sample halfway along the edge that runs from corner `i` to corner
/// `i + 1`. Every digit is an index into [`SAMPLE_HEIGHTS`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShellShape {
    /// The four corner heights, read round the face.
    pub corners: [u8; 4],
    /// The four edge-midpoint heights, one per edge in the same order.
    pub midpoints: [u8; 4],
}

impl ShellShape {
    /// A shape from its eight digits, or `None` if any of them names no height.
    pub fn new(corners: [u8; 4], midpoints: [u8; 4]) -> Option<Self> {
        let sane = |round: [u8; 4]| round.iter().all(|h| (*h as usize) < SAMPLE_HEIGHTS.len());
        (sane(corners) && sane(midpoints)).then_some(Self { corners, midpoints })
    }

    /// The shape a shell id spells, or `None` if the id is not one.
    pub fn from_id(id: &str) -> Option<Self> {
        let (corners, midpoints) = id
            .strip_prefix(SHELL_ID_PREFIX)?
            .split_once(ROUND_SEPARATOR)?;
        Self::new(round_digits(corners)?, round_digits(midpoints)?)
    }

    /// The id that spells this shape. The prototype id and the mesh key are
    /// this one string, so a shape and what it loads cannot drift apart.
    pub fn id(&self) -> String {
        let spell =
            |round: [u8; 4]| -> String { round.iter().map(|h| char::from(b'0' + h)).collect() };
        format!(
            "{SHELL_ID_PREFIX}{}{ROUND_SEPARATOR}{}",
            spell(self.corners),
            spell(self.midpoints),
        )
    }

    /// This shape turned `steps` quarter turns about the out axis.
    ///
    /// Both rounds turn TOGETHER: a corner and the midpoint of the edge leaving
    /// it are one thing, and shuffling them apart would be a different shape.
    pub fn turned(&self, steps: usize) -> Self {
        Self {
            corners: std::array::from_fn(|i| self.corners[(i + steps) % 4]),
            midpoints: std::array::from_fn(|i| self.midpoints[(i + steps) % 4]),
        }
    }

    /// The lowest of the four turns of this shape.
    ///
    /// A quarter turn about the out axis is free to whatever places a tile, so
    /// two shapes sharing this ARE one shape at two orientations and only one
    /// of them is ever built.
    ///
    /// REFLECTION is deliberately not a symmetry here, though the square has
    /// it. A section is placed with a rotation and nothing else, and no
    /// rotation turns a shape into its mirror image - folding the two together
    /// would halve the roster and leave one of every chiral pair impossible to
    /// place. Height is not a symmetry either: a bump is not a dent, so nothing
    /// here touches the digits themselves.
    pub fn canonical(&self) -> Self {
        (0..4)
            .map(|steps| self.turned(steps))
            .min()
            .expect("four quarter turns")
    }

    /// Whether the tile's surface reaches the MIDDLE of the side that edge
    /// `edge` stands on - which is exactly where that side's socket sits: the
    /// face centre, half a cell up.
    ///
    /// The wall on that side rises from the whole base edge to the boundary
    /// above it, and at the middle of the face the boundary IS the midpoint
    /// sample. So there is material at the socket precisely when the midpoint
    /// stands at half a cell or higher.
    ///
    /// Because the midpoint is shared with the tile across that edge, the two
    /// can never disagree about whether there is a socket between them.
    pub fn side_is_clad(&self, edge: usize) -> bool {
        self.midpoints[edge] >= HALF
    }

    /// The sockets this shape's own surface can back: the floor, and the sides
    /// its walls actually reach the middle of.
    ///
    /// Filtered out of the unit cube's six rather than written out, so the
    /// surviving ids, positions and normals stay the exact ones every other
    /// cube-shaped section mates through. Nothing sockets the out face: a skin
    /// is the outermost thing on a ship, so a section mated there would be
    /// built on the far side of the hull it clads.
    pub fn link_points(&self) -> Vec<LinkPoint> {
        let offered: Vec<Vec3> = std::iter::once(BOLT_NORMAL)
            .chain(
                (0..4)
                    .filter(|edge| self.side_is_clad(*edge))
                    .map(edge_normal),
            )
            .collect();
        unit_cube_link_points()
            .into_iter()
            .filter(|point| {
                offered
                    .iter()
                    .any(|normal| point.normal.abs_diff_eq(*normal, SOCKET_EPSILON))
            })
            .collect()
    }

    /// How much of its cell the tile fills, from a bare stud to a solid block.
    ///
    /// The mean of the eight boundary samples. A tile is a height field over
    /// the whole cell floor, so the mean height its surface stands at IS the
    /// share of the cell underneath it - which is why this and
    /// [`centre_height`](Self::centre_height) are one number and one function.
    ///
    /// The shape whose whole boundary is on the floor is the exception, and has
    /// to be: its mean is zero and its solid is not. It reads as half a cell,
    /// so an isolated clad cell comes out a stud rather than an invisible
    /// section - and so a caller scaling health or a collider by this never
    /// gets a plate born dead inside a box of no volume.
    pub fn volume(&self) -> f32 {
        if self.corners == [0; 4] && self.midpoints == [0; 4] {
            return SAMPLE_HEIGHTS[HALF as usize];
        }
        let total: f32 = self
            .corners
            .iter()
            .chain(self.midpoints.iter())
            .map(|h| SAMPLE_HEIGHTS[*h as usize])
            .sum();
        total / 8.0
    }

    /// Where the middle of the top sits, as a height in cells: the mean of the
    /// eight boundary samples, which is exactly [`volume`](Self::volume). See
    /// the module note for why the mean and not something else.
    pub fn centre_height(&self) -> f32 {
        self.volume()
    }

    /// The top boundary as a closed polyline: corner, midpoint, corner,
    /// midpoint, round the face. This is the whole contract with the
    /// neighbours - they build the same points out of their own digits and have
    /// to arrive at the same places.
    pub fn boundary(&self) -> [Vec3; 8] {
        std::array::from_fn(|step| {
            let edge = step / 2;
            if step % 2 == 0 {
                let (x, z) = FACE_CORNERS[edge];
                Vec3::new(x * REACH, self.height(self.corners[edge]), z * REACH)
            } else {
                let (ax, az) = FACE_CORNERS[edge];
                let (bx, bz) = FACE_CORNERS[(edge + 1) % 4];
                Vec3::new(
                    (ax + bx) * REACH * 0.5,
                    self.height(self.midpoints[edge]),
                    (az + bz) * REACH * 0.5,
                )
            }
        })
    }

    /// A sample's height in the tile's own frame, where the cell floor is at
    /// `-REACH` and the cell ceiling at `+REACH`.
    fn height(&self, sample: u8) -> f32 {
        -REACH + SAMPLE_HEIGHTS[sample as usize]
    }

    /// The tile as flat-shaded meshes in the section's local frame: the cell,
    /// out along `+Y`, floor at `-Y`. One mesh per [`ShellSurface`] that has
    /// any area, so each can carry its own material and be graded by damage.
    ///
    /// A surface with nothing to draw is left out rather than returned empty -
    /// a tile whose sides all die to the floor has no wall at all.
    pub fn surfaces(&self) -> Vec<(ShellSurface, Mesh)> {
        let boundary = self.boundary();
        let floor = -REACH;
        let footprint = |point: Vec3| Vec3::new(point.x, floor, point.z);
        let mut faces = MeshFaces::default();

        // The bolt face. The whole cell floor, always: it is flush against
        // whatever it mates and it is the one part of the shape that never
        // moves. Fanned off its own centre over the SAME eight footprint points
        // the walls stand on, so no wall ever meets a floor edge that has no
        // vertex where the wall has one - a T-junction like that cracks along
        // the seam as soon as the two surfaces are interpolated apart.
        for step in 0..8 {
            faces.add(
                ShellSurface::Floor,
                &[
                    Vec3::new(0.0, floor, 0.0),
                    footprint(boundary[step]),
                    footprint(boundary[(step + 1) % 8]),
                ],
                Vec3::NEG_Y,
            );
        }

        // The walls, one per HALF edge: corner to midpoint, then midpoint to
        // corner. Splitting at the midpoint is what keeps each piece convex -
        // a midpoint below the line between its corners makes the whole side a
        // V, and a fan over that would paper straight across the notch. Where
        // half a side is entirely on the floor its piece has no area and drops
        // out, which is the tile ending at nothing rather than at a step.
        for step in 0..8 {
            let (here, there) = (boundary[step], boundary[(step + 1) % 8]);
            faces.add(
                ShellSurface::Wall,
                &[footprint(here), footprint(there), there, here],
                edge_normal(step / 2),
            );
        }

        // The top, fanned off its own middle. A fan only ever adds a vertex
        // INSIDE the face, so the boundary stays exactly where the neighbours'
        // is.
        let middle = Vec3::new(0.0, floor + self.centre_height(), 0.0);
        for step in 0..8 {
            faces.add(
                ShellSurface::Top,
                &[middle, boundary[step], boundary[(step + 1) % 8]],
                Vec3::Y,
            );
        }

        faces.build()
    }
}

/// Four heights spelled by one round of an id.
///
/// The alphabet IS the radix, so anything naming no height is no round at
/// all - a digit above [`FULL`] included, since there is nothing above a whole
/// cell for it to mean.
fn round_digits(round: &str) -> Option<[u8; 4]> {
    let radix = SAMPLE_HEIGHTS.len() as u32;
    let digits: Option<Vec<u8>> = round
        .chars()
        .map(|digit| digit.to_digit(radix).map(|height| height as u8))
        .collect();
    digits?.try_into().ok()
}

/// The outward direction of the SIDE that edge `edge` stands on.
///
/// Edge `i` runs from corner `i` to corner `i + 1`, and adding the two corner
/// directions cancels the axis they share and leaves the one they do not.
fn edge_normal(edge: usize) -> Vec3 {
    let (ax, az) = FACE_CORNERS[edge];
    let (bx, bz) = FACE_CORNERS[(edge + 1) % 4];
    Vec3::new(ax + bx, 0.0, az + bz) * 0.5
}

/// Flat-shaded triangles under construction, kept apart by surface role: one
/// vertex per corner per facet, so every facet keeps its own normal.
///
/// Insertion order decides the order [`ShellShape::surfaces`] returns, which
/// keeps a shape's meshes in the same order every time it is built - a cache
/// keyed on the shape would otherwise hand out handles in an order that
/// depended on hashing.
#[derive(Default)]
struct MeshFaces {
    roles: Vec<ShellSurface>,
    positions: Vec<Vec<[f32; 3]>>,
    normals: Vec<Vec<[f32; 3]>>,
}

impl MeshFaces {
    /// Triangulate one planar convex loop into `role`'s mesh, wound so it
    /// points `outward`.
    ///
    /// Repeated points are dropped first, so a quad whose top edge has
    /// collapsed to a point comes back as the triangle it really is - which is
    /// most of this family, since a wall collapses wherever its side dies to
    /// the floor. Winding is CHECKED rather than reasoned about: these tiles
    /// carry a dozen distinct face orientations between them, and hand-ordering
    /// each one is how a model ends up with a black facet nobody notices.
    fn add(&mut self, role: ShellSurface, loop_: &[Vec3], outward: Vec3) {
        let mut unique: Vec<Vec3> = Vec::with_capacity(loop_.len());
        for point in loop_ {
            if !unique.iter().any(|kept| kept.abs_diff_eq(*point, 1e-6)) {
                unique.push(*point);
            }
        }
        if unique.len() < 3 {
            return;
        }
        if (unique[1] - unique[0])
            .cross(unique[2] - unique[0])
            .dot(outward)
            < 0.0
        {
            unique.reverse();
        }
        let normal = (unique[1] - unique[0])
            .cross(unique[2] - unique[0])
            .normalize_or_zero();

        let slot = match self.roles.iter().position(|kept| *kept == role) {
            Some(slot) => slot,
            None => {
                self.roles.push(role);
                self.positions.push(Vec::new());
                self.normals.push(Vec::new());
                self.roles.len() - 1
            }
        };
        for step in 1..unique.len() - 1 {
            for point in [unique[0], unique[step], unique[step + 1]] {
                self.positions[slot].push(point.to_array());
                self.normals[slot].push(normal.to_array());
            }
        }
    }

    fn build(self) -> Vec<(ShellSurface, Mesh)> {
        self.roles
            .into_iter()
            .zip(self.positions)
            .zip(self.normals)
            .map(|((role, positions), normals)| {
                let indices: Vec<u32> = (0..positions.len() as u32).collect();
                let mesh = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::default(),
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
                .with_inserted_indices(Indices::U32(indices));
                (role, mesh)
            })
            .collect()
    }
}

/// `ShellShape`, `ShellSurface`, `SAMPLE_HEIGHTS`, `SHELL_ID_PREFIX`, `FULL`
/// and `HALF`.
pub mod prelude {
    pub use super::{ShellShape, ShellSurface, FULL, HALF, SAMPLE_HEIGHTS, SHELL_ID_PREFIX};
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    /// How far off a plane or an edge a reading may be and still count.
    const MESH_EPSILON: f32 = 1e-4;

    fn shape(corners: [u8; 4], midpoints: [u8; 4]) -> ShellShape {
        ShellShape::new(corners, midpoints).expect("a legal shape")
    }

    /// Every facet of a shape, across all of its surface meshes. The tile is
    /// one solid cut into meshes by material, so anything asking whether it is
    /// closed, or where it has material, has to see all of them at once.
    fn facets(shape: &ShellShape) -> Vec<[Vec3; 3]> {
        shape
            .surfaces()
            .iter()
            .flat_map(|(_, mesh)| triangles(mesh))
            .collect()
    }

    fn triangles(mesh: &Mesh) -> Vec<[Vec3; 3]> {
        let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("a generated shell carries float positions");
        };
        positions
            .chunks_exact(3)
            .map(|corner| std::array::from_fn(|i| Vec3::from_array(corner[i])))
            .collect()
    }

    /// The samples a CORNER may take: the floor, the height a skin runs at, and
    /// the whole cell. A corner reads structure, and structure is either there
    /// or it is not, so the quarters between are a midpoint's business.
    const COARSE: [u8; 3] = [0, HALF, FULL];

    /// The shapes the geometry tests sweep: every corner setting over
    /// [`COARSE`], against every setting of the four midpoints over the whole
    /// alphabet. A midpoint is a mean, so it can land anywhere.
    ///
    /// A spread rather than the whole space, and deliberately so. Nothing
    /// enumerates the shapes in production - meshes are built on demand and a
    /// whole ship touches a couple of dozen - so the space is far larger than
    /// it is worth building a mesh for. What breaks a mesh builder is which
    /// samples sit above which, and this reaches every one of those.
    fn sample_shapes() -> Vec<ShellShape> {
        let (coarse, alphabet) = (COARSE.len() as u32, SAMPLE_HEIGHTS.len() as u32);
        let mut shapes: Vec<ShellShape> = Vec::new();
        let mut seen = HashSet::new();
        for corner_raw in 0..coarse.pow(4) {
            let corners = std::array::from_fn(|i| {
                COARSE[((corner_raw / coarse.pow(i as u32)) % coarse) as usize]
            });
            for mid_raw in 0..alphabet.pow(4) {
                let midpoints =
                    std::array::from_fn(|i| ((mid_raw / alphabet.pow(i as u32)) % alphabet) as u8);
                let canonical = ShellShape { corners, midpoints }.canonical();
                if seen.insert(canonical) {
                    shapes.push(canonical);
                }
            }
        }
        shapes.sort_unstable();
        shapes
    }

    /// An id and the shape it spells each other.
    #[test]
    fn every_id_parses_back_into_the_shape_it_was_built_from() {
        for shape in sample_shapes() {
            let id = shape.id();
            assert_eq!(
                ShellShape::from_id(&id),
                Some(shape),
                "`{id}` parses back into a different shape",
            );
        }
        assert_eq!(
            ShellShape::from_id("shell_0000"),
            None,
            "one round is no id"
        );
        assert_eq!(
            ShellShape::from_id("shell_0005_0000"),
            None,
            "nothing stands above a whole cell"
        );
        assert_eq!(ShellShape::from_id("hull_cube"), None, "not a shell at all");
    }

    /// Every generated tile is a CLOSED SOLID.
    ///
    /// Every edge shared by exactly two triangles. A hole would show as a
    /// see-through facet the moment the tile is lit, and it is the failure a
    /// generated mesh is most likely to have: the walls collapse to nothing
    /// wherever a side dies to the floor, and the top fan has to close over
    /// exactly what is left. Checked over the whole spread rather than a few
    /// tidy shapes, because the ones that break are the lopsided ones nobody
    /// would pick by hand.
    #[test]
    fn every_generated_tile_is_a_closed_solid() {
        for shape in sample_shapes() {
            let mut edges: HashMap<([i64; 3], [i64; 3]), usize> = HashMap::new();
            let key = |point: Vec3| -> [i64; 3] {
                std::array::from_fn(|axis| (point[axis] * 1e6).round() as i64)
            };
            for triangle in facets(&shape) {
                for step in 0..3 {
                    let (a, b) = (key(triangle[step]), key(triangle[(step + 1) % 3]));
                    *edges
                        .entry(if a < b { (a, b) } else { (b, a) })
                        .or_default() += 1;
                }
            }
            let unpaired: Vec<_> = edges.iter().filter(|(_, n)| **n != 2).collect();
            assert!(
                unpaired.is_empty(),
                "`{}` has {} unpaired edge(s) - not a closed solid",
                shape.id(),
                unpaired.len(),
            );
        }
    }

    /// No tile leaves its own cell.
    ///
    /// One tile per cell is what keeps a skin free of interpenetration, and it
    /// only holds if the geometry stays inside the box the collider claims.
    #[test]
    fn no_generated_tile_leaves_its_cell() {
        for shape in sample_shapes() {
            for triangle in facets(&shape) {
                for point in triangle {
                    assert!(
                        point.abs().max_element() <= REACH + MESH_EPSILON,
                        "`{}` reaches {point:?}, outside its own cell",
                        shape.id(),
                    );
                }
            }
        }
    }

    /// A tile sockets a side exactly where its own mesh reaches it.
    ///
    /// The socket set is derived from the digits; this reads the answer back
    /// off the TRIANGLES instead, so the declaration cannot drift from the
    /// model. A socket the mesh cannot back invites a builder to mate a section
    /// against thin air; a missing one hides a seam the tile really could have
    /// closed.
    #[test]
    fn a_shell_sockets_a_side_exactly_where_its_own_mesh_reaches_it() {
        for shape in sample_shapes() {
            let facets = facets(&shape);
            let sockets: Vec<String> = shape
                .link_points()
                .into_iter()
                .map(|point| point.id)
                .collect();
            for point in unit_cube_link_points() {
                if point.normal.dot(Vec3::Y) > 0.5 {
                    continue; // Nothing ever sockets the face shown to space.
                }
                let declared = sockets.contains(&point.id);
                let modelled = mesh_covers(&facets, point.position, point.normal);
                assert_eq!(
                    declared,
                    modelled,
                    "`{}` {} `{}`, and its mesh {} material there",
                    shape.id(),
                    if declared {
                        "sockets"
                    } else {
                        "does not socket"
                    },
                    point.id,
                    if modelled { "has" } else { "has no" },
                );
            }
        }
    }

    /// Whether any facet lying in the plane of `normal`'s face covers `point`.
    fn mesh_covers(facets: &[[Vec3; 3]], point: Vec3, normal: Vec3) -> bool {
        let axis = (0..3)
            .max_by(|a, b| normal[*a].abs().total_cmp(&normal[*b].abs()))
            .expect("three axes");
        let (u, v) = match axis {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
        let flat = |p: Vec3| Vec2::new(p[u], p[v]);
        facets.iter().any(|facet| {
            facet
                .iter()
                .all(|corner| (corner[axis] - point[axis]).abs() <= MESH_EPSILON)
                && encloses(facet.map(flat), flat(point))
        })
    }

    /// Point in triangle, edges INCLUDED: a socket often sits exactly on the
    /// crest of the wall it stands on, and that is contact rather than a miss.
    /// Degenerate facets are refused - every sign is zero on one, which would
    /// otherwise read as covering the whole plane.
    fn encloses(triangle: [Vec2; 3], point: Vec2) -> bool {
        let signs: [f32; 3] = std::array::from_fn(|corner| {
            let (from, to) = (triangle[corner], triangle[(corner + 1) % 3]);
            (to - from).perp_dot(point - from)
        });
        if signs.iter().all(|area| area.abs() <= MESH_EPSILON) {
            return false;
        }
        signs.iter().all(|area| *area >= -MESH_EPSILON)
            || signs.iter().all(|area| *area <= MESH_EPSILON)
    }

    /// Two tiles across a seam agree about the socket between them.
    ///
    /// The rule reads only the midpoint, which is the sample the two SHARE, so
    /// the answer cannot depend on which side is asking. Without that a skin
    /// could butt two tiles together where only one was willing to mate.
    #[test]
    fn two_tiles_across_a_seam_agree_about_the_socket_between_them() {
        let alphabet = SAMPLE_HEIGHTS.len() as u8;
        for near in 0..alphabet {
            for far in 0..alphabet {
                for mid in 0..alphabet {
                    // One tile's edge 0, and the same seam as the neighbour
                    // reads it: same midpoint, corners the other way round.
                    let mine = shape([near, far, 0, 0], [mid, 0, 0, 0]);
                    let theirs = shape([far, near, 0, 0], [mid, 0, 0, 0]);
                    assert_eq!(
                        mine.side_is_clad(0),
                        theirs.side_is_clad(0),
                        "corners {near} and {far} over a {mid} midpoint read \
                         differently from the two sides of the seam",
                    );
                }
            }
        }
    }

    /// A tile is cut into meshes by MATERIAL, and only where it has area.
    ///
    /// One mesh per role is what lets a shell keep being graded by damage - see
    /// [`ShellSurface`]. A role with nothing to draw has to be absent rather
    /// than empty, or a caller spawns an entity holding a mesh of no triangles.
    #[test]
    fn a_tile_carries_one_mesh_per_surface_it_actually_has() {
        let roles = |shape: &ShellShape| -> Vec<ShellSurface> {
            shape.surfaces().into_iter().map(|(role, _)| role).collect()
        };

        // A stud's boundary is all on the floor, so its walls have no height.
        let stud = shape([0; 4], [0; 4]);
        assert_eq!(roles(&stud), vec![ShellSurface::Floor, ShellSurface::Top]);

        // A full block closes every side, so it carries all three.
        let block = shape([FULL; 4], [FULL; 4]);
        assert_eq!(
            roles(&block),
            vec![ShellSurface::Floor, ShellSurface::Wall, ShellSurface::Top],
        );

        for shape in sample_shapes() {
            let surfaces = shape.surfaces();
            let mut seen = HashSet::new();
            for (role, mesh) in &surfaces {
                assert!(
                    seen.insert(*role),
                    "`{}` splits {role:?} in two",
                    shape.id()
                );
                assert!(
                    !triangles(mesh).is_empty(),
                    "`{}` carries an empty {role:?} mesh",
                    shape.id(),
                );
            }
            // The floor is the whole cell floor on every tile, and the top
            // always closes over it, so those two are never missing.
            assert!(seen.contains(&ShellSurface::Floor), "`{}`", shape.id());
            assert!(seen.contains(&ShellSurface::Top), "`{}`", shape.id());
        }
    }

    /// A ramp comes out FLAT.
    ///
    /// This is what pins the centre to the mean of the eight. A run of skin
    /// climbing a step is the commonest thing the family does, and a centre
    /// anywhere else would put a crease down the middle of every one of them.
    #[test]
    fn a_ramp_is_flat_through_the_middle() {
        // Floor along one edge, full along the opposite one, midpoints where
        // the straight line between each pair of corners puts them.
        let ramp = shape([0, 0, FULL, FULL], [0, HALF, FULL, HALF]);
        assert!(
            (ramp.centre_height() - SAMPLE_HEIGHTS[HALF as usize]).abs() < MESH_EPSILON,
            "a ramp's middle sits at {} rather than half a cell",
            ramp.centre_height(),
        );
    }

    /// The shape with nothing on its boundary is still something to look at.
    ///
    /// The mean would be zero, which is a tile of no volume - an invisible
    /// section holding a cell. It rides at half a cell instead, so an isolated
    /// clad cell comes out a stud.
    #[test]
    fn a_boundary_entirely_on_the_floor_still_has_a_shape() {
        let stud = shape([0; 4], [0; 4]);
        assert_eq!(stud.centre_height(), SAMPLE_HEIGHTS[HALF as usize]);
        let peak = facets(&stud)
            .iter()
            .flatten()
            .map(|point| point.y)
            .fold(f32::MIN, f32::max);
        assert!(
            (peak - (-REACH + SAMPLE_HEIGHTS[HALF as usize])).abs() < MESH_EPSILON,
            "the stud peaks at {peak} rather than half a cell up",
        );
        assert_eq!(
            stud.link_points().len(),
            1,
            "a stud touches nothing but the floor it stands on",
        );
    }

    /// The shapes the old dead-edge scheme could not express are real now.
    ///
    /// These are the ones that motivated freeing the midpoints. Each was
    /// previously either unreachable or drawn as something else entirely.
    #[test]
    fn the_shapes_a_free_midpoint_unlocks_are_distinct() {
        // A corner at half with both its edges on the floor is a TRIANGLE
        // rising out of one corner, not a square with a gradient over it.
        let spike = shape([0, 0, 0, HALF], [0; 4]);
        assert!(spike.centre_height() < SAMPLE_HEIGHTS[HALF as usize] * 0.5);

        // Corners full with every edge midpoint on the floor: the block with a
        // valley cut across it, which is what a one-cell concave notch wants.
        let notch = shape([FULL; 4], [0; 4]);
        assert_eq!(notch.link_points().len(), 1, "its sides die at the middle");
        assert!(notch.centre_height() < SAMPLE_HEIGHTS[FULL as usize]);

        // And it is a different shape from the full block, which the old
        // scheme conflated it with.
        let block = shape([FULL; 4], [FULL; 4]);
        assert_ne!(notch.canonical(), block.canonical());
        assert_eq!(
            block.link_points().len(),
            5,
            "a full block closes every side"
        );
    }
}
