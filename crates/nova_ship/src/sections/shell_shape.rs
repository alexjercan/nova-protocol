//! The SHAPE of a hull shell tile: a plateau, what each edge does, and the
//! mesh that follows from them.
//!
//! A shell is one cell of a ship's outer skin. Its top is a flat PLATEAU at
//! the plate's level - half a cell for a run of skin, a whole cell beside
//! structure the skin climbs - and each of its four edges says how the top
//! leaves the cell on that side:
//!
//! ```text
//!              e0 / f0
//!         s1 +---------+ s0
//!            |         |
//!    e1 / f1 | plateau | e3 / f3     shell_<level>_<feet>_<edges>_<soft>
//!            |         |
//!         s2 +---------+ s3
//!              e2 / f2
//! ```
//!
//! - the FOOT `f` is the height the top comes down to at that edge. A foot at
//!   the level is a flush join; a lower one is a chamfer from the plateau.
//! - the EDGE TYPE `e` is the slope of that chamfer and whatever the edge adds
//!   beyond it: a skirt round a convex hull corner, a wall the plate holds its
//!   level against, or the ramp of a diagonal stair. See [`ShellEdgeType`].
//! - the SOFT CORNER `s` is the height the four cells meeting at that corner
//!   average to. A whole-cell plate is capped by a straight bevel through any
//!   corner the neighbourhood holds lower, so a raised block reads as one
//!   piece rather than as a tower with square shoulders.
//!
//! The top is the lower envelope of straight planes, one for the plateau and
//! one per chamfer, so it is never a point. Two neighbours read the same foot
//! and the same slope on their shared side, which is why their surfaces meet
//! without a seam between them. That replaced a height field sampled at eight
//! boundary points and fanned off a free middle: the fan put a spike wherever a
//! midpoint stood above the two corners beside it.
//!
//! A quarter turn about the out axis is free to whatever places the tile, so
//! two settings that differ by one are ONE shape - see
//! [`ShellShape::canonical`]. Reflection is not a symmetry: a section is placed
//! with a rotation and nothing else, so a shape and its mirror are two shapes.
//!
//! Meshes are BUILT, not shipped, and built on demand. A whole ship touches a
//! few dozen shapes, so a tile costs less to generate than to load.

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

/// The heights a foot or a soft corner can stand at, indexed by the digit that
/// names it: quarter cells, from the floor to a whole cell.
pub const SAMPLE_HEIGHTS: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// The digit of a whole cell: the level of a plate beside structure it climbs.
pub const FULL: u8 = 4;

/// The digit of half a cell: the level a run of skin travels at, and the foot
/// of a chamfer off a whole-cell plate.
pub const HALF: u8 = 2;

/// Half the cell, which is the tile's reach from its own centre on every axis.
/// The cell floor is at `-REACH` and its ceiling at `+REACH` - `pub(crate)`
/// because [`super::shell_skin`] builds a collider round the tile and has to
/// stand it on the same floor the mesh does.
pub(crate) const REACH: f32 = 0.5;

/// Every shell id opens with this.
pub const SHELL_ID_PREFIX: &str = "shell_";

/// How far apart two heights or plane terms may be and still count as one.
/// The inputs are quarter cells and slopes of one to eight, so nothing honest
/// lands anywhere near this.
const EPSILON: f32 = 1e-5;

/// The smallest piece of top worth a facet, in square cells. Smaller pieces
/// are slivers left by two cut lines that cross on the square's own boundary.
const MIN_PIECE_AREA: f32 = 1e-7;

/// The slope of an ordinary chamfer: two cells of height per cell of run, so
/// half a cell of drop takes a quarter of the plate.
const CHAMFER_SLOPE: f32 = 2.0;

/// How many steps a seat's reach is counted in per cell: a seat digit `d`
/// reaches `d / SEAT_STEPS` of a cell from the plate's middle.
pub const SEAT_STEPS: u8 = 64;

/// The slope of a diagonal stair's ramp: one cell of height per cell of run,
/// so a whole-cell plate falls to the floor across its full width.
const RAMP_SLOPE: f32 = 1.0;

/// The slope of a wall cover: steep, so where a plate's rims fall away at a
/// wall corner the cover closes the wall's face with a thin web against it
/// instead of a tooth across the plate.
const WALL_SLOPE: f32 = 8.0;

/// The steepest a plate steepens a chamfer, a ramp end or a corner cap to seat
/// a decoration: a whole cell of drop over one [`SEAT_STEPS`] step. So a
/// plateau may reach to within the drop over this slope of an edge it falls
/// at - a 128th of a cell for half a cell of drop - and a piece that reaches
/// past that does not fit that plate. Near vertical on purpose: a line greeble
/// is authored nearly a cell long so it meets the next plate's, and the plate
/// at the end of its run has to seat it.
const STEEPEST_SLOPE: f32 = SEAT_STEPS as f32;

/// The four corners of the outward face, as signs on x and z, in the order an
/// id reads them. A quarter turn about the out axis is one step along this
/// list, and edge `i` runs from corner `i` to corner `i + 1`.
const FACE_CORNERS: [(f32, f32); 4] = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)];

/// The surface roles a generated tile is cut into, one mesh each.
///
/// A role per mesh rather than a colour per vertex, which is what the authored
/// `.glb` tiles already do - one primitive per material. Vertex colours would
/// be one mesh and one material for the whole family, but bevy's PBR fragment
/// ASSIGNS `base_color` from the vertex colour instead of multiplying by it,
/// and [`super::damage_cracks`] grades a section by writing that same
/// `base_color`. A vertex-coloured shell would quietly stop reading as damaged
/// or destroyed, which is a gameplay signal and not just a look. Nor would it
/// buy anything: `damage_cracks` hands every section a private clone of its
/// material, so sections do not batch with each other whatever the mesh does.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// What one edge of a plate does where the top leaves the cell.
///
/// Read off the cell across the edge by
/// [`boundary_heights`](super::shell_skin::boundary_heights). Every type but
/// [`Wall`](Self::Wall) and [`RampEnd`](Self::RampEnd) chamfers the plateau
/// down to the edge's foot, and does nothing where the foot is the level.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ShellEdgeType {
    /// More skin, structure the skin stops at, the mouth of a fitting, or open
    /// space with no plate round the corner. A chamfer at slope two.
    Plain,
    /// A convex hull edge whose skirt the plate on the side face owns. Shaped
    /// as [`Plain`](Self::Plain); kept apart because the hull falls away
    /// here, which is where a rim piece belongs.
    Fold,
    /// A convex hull edge whose skirt THIS plate owns: a 45 degree face from
    /// the top of the edge down past the cell, to where the plate on the side
    /// face carries on. One plate of the two owns it, so the corner is closed
    /// once.
    Skirt,
    /// The low side of a diagonal stair, or the edge beside such a ramp: a
    /// chamfer at slope one down to the floor.
    Ramp,
    /// The open end of a diagonal stair plate. The top falls into the same
    /// plane as the ramp's end, so the stair's end profile matches the plates
    /// beside it.
    RampEnd,
    /// Structure the plate climbs. The top holds its level along the whole
    /// edge, so a chamfer on a neighbouring edge never bares the structure.
    Wall,
}

impl ShellEdgeType {
    /// The letter an id spells this type with.
    fn letter(self) -> char {
        match self {
            Self::Plain => 'p',
            Self::Fold => 'f',
            Self::Skirt => 's',
            Self::Ramp => 'r',
            Self::RampEnd => 'e',
            Self::Wall => 'w',
        }
    }
}

/// One shell tile's shape: its level, and what each edge and corner does.
///
/// Indexed round the face: `feet[i]` and `edges[i]` are the edge that runs
/// from corner `i` to corner `i + 1`, and `soft_corners[i]` is corner `i`.
/// Every height is a digit of [`SAMPLE_HEIGHTS`]. [`ShellShape::new`] is the
/// one way to build one, and it refuses a combination the mesh has no meaning
/// for.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShellShape {
    /// The plateau height: [`HALF`] for a run of skin, [`FULL`] beside
    /// structure the plate climbs.
    pub level: u8,
    /// The height the top comes down to at each edge.
    pub feet: [u8; 4],
    /// What each edge does.
    pub edges: [ShellEdgeType; 4],
    /// The height the neighbourhood averages to at each corner, clamped to
    /// between [`HALF`] and the level. A corner below the level caps a
    /// whole-cell plate with a bevel; a half-cell plate always reads `HALF`.
    pub soft_corners: [u8; 4],
    /// How far the plateau must reach from the middle toward each edge, in
    /// [`SEAT_STEPS`] of a cell: the seat of the decoration the plate
    /// carries. A chamfer, a ramp end or a corner cap that would cut into
    /// that rectangle is steepened until it clears it; the feet do not move.
    /// All zero on a plate that steepens nothing. See [`ShellShape::seated`].
    pub seat: [u8; 4],
}

impl ShellShape {
    /// A shape from its parts, or `None` where they disagree.
    ///
    /// The level is [`HALF`] or [`FULL`]. A foot stands no higher than the
    /// level, and the edge type fixes it where the type has one meaning: a
    /// [`Plain`](ShellEdgeType::Plain) edge falls at most half a cell, a
    /// [`Wall`](ShellEdgeType::Wall) holds the level, a
    /// [`Fold`](ShellEdgeType::Fold) or [`Skirt`](ShellEdgeType::Skirt) meets
    /// the plate round the corner at [`HALF`], a
    /// [`Ramp`](ShellEdgeType::Ramp) lands wherever the plate below it
    /// stands, and a [`RampEnd`](ShellEdgeType::RampEnd) closes the side of a
    /// ramp that climbs to a wall on a whole-cell plate, down to the floor. A
    /// soft corner sits
    /// between [`HALF`] and the level.
    pub fn new(
        level: u8,
        feet: [u8; 4],
        edges: [ShellEdgeType; 4],
        soft_corners: [u8; 4],
    ) -> Option<Self> {
        let legal_level = level == HALF || level == FULL;
        let legal_edge = |edge: usize| {
            let foot = feet[edge];
            match edges[edge] {
                ShellEdgeType::Plain => foot <= level && foot + HALF >= level,
                ShellEdgeType::Fold | ShellEdgeType::Skirt => foot == HALF,
                ShellEdgeType::Ramp => foot <= level,
                ShellEdgeType::RampEnd => {
                    foot == 0 && level == FULL && ramp_beside(&edges, edge).is_some()
                }
                ShellEdgeType::Wall => foot == level,
            }
        };
        let legal_corner = |corner: usize| (HALF..=level).contains(&soft_corners[corner]);
        (legal_level && (0..4).all(legal_edge) && (0..4).all(legal_corner)).then_some(Self {
            level,
            feet,
            edges,
            soft_corners,
            seat: [0; 4],
        })
    }

    /// The id that spells this shape: the level, the four feet, the four edge
    /// letters and the four soft corners, then the four seat reaches as two
    /// digits each where the plate seats a piece. Names a plate in a dump and
    /// a probe snapshot; nothing keys on it.
    pub fn id(&self) -> String {
        let digits =
            |round: [u8; 4]| -> String { round.map(|h| char::from(b'0' + h)).iter().collect() };
        let letters: String = self.edges.map(ShellEdgeType::letter).iter().collect();
        let seat = match self.seat {
            [0, 0, 0, 0] => String::new(),
            [a, b, c, d] => format!("_{a:02}{b:02}{c:02}{d:02}"),
        };
        format!(
            "{SHELL_ID_PREFIX}{}_{}_{letters}_{}{seat}",
            self.level,
            digits(self.feet),
            digits(self.soft_corners),
        )
    }

    /// This shape turned `steps` quarter turns about the out axis. Edges and
    /// corners turn TOGETHER: edge `i` starts at corner `i`.
    pub fn turned(&self, steps: usize) -> Self {
        Self {
            level: self.level,
            feet: std::array::from_fn(|i| self.feet[(i + steps) % 4]),
            edges: std::array::from_fn(|i| self.edges[(i + steps) % 4]),
            soft_corners: std::array::from_fn(|i| self.soft_corners[(i + steps) % 4]),
            seat: std::array::from_fn(|i| self.seat[(i + steps) % 4]),
        }
    }

    /// The lowest of the four turns of this shape.
    ///
    /// A quarter turn about the out axis is free to whatever places a tile, so
    /// two shapes sharing this ARE one shape at two orientations and only one
    /// of them is ever built. Reflection is deliberately not folded in: no
    /// rotation turns a shape into its mirror image, so folding the two would
    /// leave one of every chiral pair impossible to place.
    pub fn canonical(&self) -> Self {
        (0..4)
            .map(|steps| self.turned(steps))
            .min()
            .expect("four quarter turns")
    }

    /// How much of its cell the tile fills: the mean height of the top over
    /// the cell's footprint, in cells. A skirt reaching past the cell is not
    /// counted; the collider this sizes is the cell's own box.
    pub fn volume(&self) -> f32 {
        let top = self.top_planes();
        self.pieces()
            .iter()
            .map(|piece| polygon_area(&piece.footprint) * top.height(centroid(&piece.footprint)))
            .sum()
    }

    /// Where the middle of the top stands, as a height in cells above the
    /// floor - where a decoration is stood.
    pub fn centre_height(&self) -> f32 {
        self.top_planes().height(Vec2::ZERO)
    }

    /// Whether the whole top inside the cell is ONE PLANE, level or raked.
    ///
    /// A ramp counts: a piece laid on it has as much material under it as one
    /// on a level panel. What a decoration cannot stand on is a crease, and
    /// this is the test for the absence of one. A skirt is outside the cell and
    /// is not part of the seat.
    pub fn is_coplanar(&self) -> bool {
        self.flat_area() >= 1.0 - EPSILON
    }

    /// Whether a piece stood on the middle of the plate, `half` cells each way
    /// along `x` and `z` in the tile's own frame, has ONE PLANE under its
    /// whole footprint: the plane under the middle, the one
    /// [`seat_normal`](Self::seat_normal) beds it on.
    ///
    /// The seat a flat-bottomed decoration needs. Exact rather than sampled:
    /// the seat plane is the top over the rectangle when every lower plane of
    /// the top stands above it, and every wall cover below it, at the four
    /// corners, and each of those differences is linear. A footprint wider
    /// than the cell has no seat.
    pub fn seats(&self, half: Vec2) -> bool {
        if half.max_element() > REACH + EPSILON {
            return false;
        }
        let top = self.top_planes();
        let seat = top.seat_plane();
        let corners = rectangle(half);
        let above = |plane: &Plane| {
            corners
                .iter()
                .all(|c| plane.at(*c) >= seat.at(*c) - EPSILON)
        };
        let below = |plane: &Plane| {
            corners
                .iter()
                .all(|c| plane.at(*c) <= seat.at(*c) + EPSILON)
        };
        top.envelope.iter().chain(&top.caps).all(above) && top.walls.iter().all(below)
    }

    /// This shape reshaped to seat a piece `half` cells each way along `x`
    /// and `z` in the tile's own frame, or `None` when it cannot.
    ///
    /// The plateau grows to cover the footprint, rounded up to a
    /// [`SEAT_STEPS`] step, by steepening every chamfer, ramp end and corner
    /// cap that cuts into it, up to [`STEEPEST_SLOPE`]. The feet stay where
    /// they are, so the plate still meets its neighbours at the same heights
    /// along its edges. A shape that already seats the piece comes back as it
    /// is, so a piece that fits changes nothing. `None` when the footprint is
    /// wider than the cell or even the steepest slope leaves it on more than
    /// one plane.
    pub fn seated(&self, half: Vec2) -> Option<Self> {
        if self.seats(half) {
            return Some(*self);
        }
        if half.max_element() > REACH + EPSILON {
            return None;
        }
        let steps = |reach: f32| (reach * f32::from(SEAT_STEPS) - EPSILON).ceil().max(0.0) as u8;
        let (x, z) = (steps(half.x), steps(half.y));
        let shape = Self {
            seat: [z, x, z, x],
            ..*self
        };
        shape.seats(half).then_some(shape)
    }

    /// How much of the cell footprint the LARGEST flat piece of the top covers,
    /// in square cells: 1.0 for a plate that is one surface, and a quarter for
    /// the plateau of a plate chamfered on every side.
    ///
    /// Measured on the FOOTPRINT rather than on the surface, so it answers how
    /// much room there is under a piece standing here, and a steep facet does
    /// not read as more room than a level one of the same width.
    pub fn flat_area(&self) -> f32 {
        self.largest_flat().1
    }

    /// The unit normal of the LARGEST flat piece of the top, in the tile's own
    /// frame. It always points out of the cell: every plane of the top is a
    /// height over the footprint.
    pub fn top_normal(&self) -> Vec3 {
        self.largest_flat().0.normal()
    }

    /// The normal a DECORATION is stood up on: the plane under the middle of
    /// the plate, which is where [`decor_pose`] stands the piece.
    ///
    /// Where two planes meet under the middle, the flatter one wins, so a piece
    /// on the crest of a chamfer stands up rather than leaning off it.
    ///
    /// [`decor_pose`]: super::skin_decor::decor_pose
    pub fn seat_normal(&self) -> Vec3 {
        self.top_planes().seat_plane().normal()
    }

    /// How far the largest flat piece of the top LEANS off the cell's own out
    /// face, in radians: zero on a level panel, about 0.79 on a stair ramp.
    pub fn tilt(&self) -> f32 {
        self.top_normal().dot(Vec3::Y).clamp(-1.0, 1.0).acos()
    }

    /// The tile as flat-shaded meshes in the section's local frame: the cell,
    /// out along `+Y`, floor at `-Y`. One mesh per [`ShellSurface`] that has
    /// any area, so each can carry its own material and be graded by damage.
    ///
    /// The top is cut into flat pieces, the walls stand from the floor to the
    /// top along each edge through every point a piece meets it at, and the
    /// floor is fanned over the same points - so no facet meets another at a
    /// point the other has no vertex at, and the tile is a closed solid. A
    /// [`Skirt`](ShellEdgeType::Skirt) edge carries a 45 degree face in place
    /// of its wall, out past the cell to the floor.
    pub fn surfaces(&self) -> Vec<(ShellSurface, Mesh)> {
        let top = self.top_planes();
        let pieces = self.pieces();
        let floor = -REACH;
        let lift = |p: Vec2, height: f32| Vec3::new(p.x, floor + height, p.y);
        let height = |p: Vec2| top.height(p);
        let skirt = self.edges.map(|edge| edge == ShellEdgeType::Skirt);
        // Where a skirt's own height stops at `p` on the end of `edge`: its
        // foot, or the top where that is lower.
        let web_foot = |p: Vec2, edge: usize| match skirt[edge] {
            true => height(p).min(SAMPLE_HEIGHTS[self.feet[edge] as usize]),
            false => height(p),
        };
        let mut faces = MeshFaces::default();

        for piece in &pieces {
            let loop_: Vec<Vec3> = piece
                .footprint
                .iter()
                .map(|p| lift(*p, height(*p)))
                .collect();
            faces.add(ShellSurface::Top, &loop_, Vec3::Y);
        }

        let mut ring: Vec<Vec2> = Vec::new();
        for edge in 0..4 {
            let points = edge_points(edge, &pieces);
            let normal = edge_normal(edge);
            let out = Vec2::new(normal.x, normal.z);
            ring.extend_from_slice(&points[..points.len() - 1]);
            if !skirt[edge] {
                // A wall that ends at a skirt's web is split at the skirt's
                // foot, so the two meet vertex to vertex up the corner.
                let last = points.len() - 2;
                for (step, pair) in points.windows(2).enumerate() {
                    let (a, b) = (pair[0], pair[1]);
                    let wa = match step == 0 {
                        true => web_foot(a, (edge + 3) % 4),
                        false => height(a),
                    };
                    let wb = match step == last {
                        true => web_foot(b, (edge + 1) % 4),
                        false => height(b),
                    };
                    faces.add(
                        ShellSurface::Wall,
                        &[lift(a, 0.0), lift(b, 0.0), lift(b, wb), lift(a, wa)],
                        normal,
                    );
                    faces.add(
                        ShellSurface::Wall,
                        &[
                            lift(a, wa),
                            lift(b, wb),
                            lift(b, height(b)),
                            lift(a, height(a)),
                        ],
                        normal,
                    );
                }
                continue;
            }
            // The skirt: from the edge straight out and down at 45 degrees, so
            // it lands as far past the cell as the edge is high. It starts no
            // higher than its foot: where a wall cover lifts the edge above
            // that, a web stands in the edge's plane against the wall instead,
            // or the skirt end would stand a whole cell out as a tooth.
            let skirt_height = |p: Vec2| web_foot(p, edge);
            for pair in points.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let (ha, hb) = (skirt_height(a), skirt_height(b));
                faces.add(
                    ShellSurface::Wall,
                    &[
                        lift(a, ha),
                        lift(b, hb),
                        lift(b, height(b)),
                        lift(a, height(a)),
                    ],
                    normal,
                );
                let (a_out, b_out) = (a + out * ha, b + out * hb);
                faces.add(
                    ShellSurface::Top,
                    &[lift(a, ha), lift(a_out, 0.0), lift(b_out, 0.0), lift(b, hb)],
                    normal + Vec3::Y,
                );
                faces.add(
                    ShellSurface::Floor,
                    &[
                        lift(a, 0.0),
                        lift(b, 0.0),
                        lift(b_out, 0.0),
                        lift(a_out, 0.0),
                    ],
                    Vec3::NEG_Y,
                );
            }
            // Each end is closed by a triangle, unless the next edge round is a
            // skirt too - the bevel below closes that corner instead.
            let tangent = (points[points.len() - 1] - points[0]).normalize();
            for (corner, along, beside) in [
                (points[0], -tangent, (edge + 3) % 4),
                (points[points.len() - 1], tangent, (edge + 1) % 4),
            ] {
                if skirt[beside] {
                    continue;
                }
                let hc = skirt_height(corner);
                faces.add(
                    ShellSurface::Wall,
                    &[
                        lift(corner, 0.0),
                        lift(corner, hc),
                        lift(corner + out * hc, 0.0),
                    ],
                    Vec3::new(along.x, 0.0, along.y),
                );
            }
        }
        for step in 0..ring.len() {
            faces.add(
                ShellSurface::Floor,
                &[
                    lift(Vec2::ZERO, 0.0),
                    lift(ring[step], 0.0),
                    lift(ring[(step + 1) % ring.len()], 0.0),
                ],
                Vec3::NEG_Y,
            );
        }
        // A corner between two skirts closes with one bevel facet over one
        // floor triangle.
        for corner in 0..4 {
            let (from, to) = ((corner + 3) % 4, corner);
            if !(skirt[from] && skirt[to]) {
                continue;
            }
            let c = corner_point(corner);
            let hc = web_foot(c, from);
            let (first, second) = (edge_normal(from), edge_normal(to));
            let (first_out, second_out) = (
                c + Vec2::new(first.x, first.z) * hc,
                c + Vec2::new(second.x, second.z) * hc,
            );
            faces.add(
                ShellSurface::Top,
                &[lift(c, hc), lift(first_out, 0.0), lift(second_out, 0.0)],
                first + second + Vec3::Y,
            );
            faces.add(
                ShellSurface::Floor,
                &[lift(c, 0.0), lift(first_out, 0.0), lift(second_out, 0.0)],
                Vec3::NEG_Y,
            );
        }

        faces.build()
    }

    /// The planes the top is made of, with every fall steepened to clear the
    /// seat where the plate carries one. See [`TopPlanes`].
    ///
    /// The seat plane is read off the top before any steepening, and
    /// steepening never changes it: a fall only rises inside the cell, so the
    /// plane under the middle stays the lowest there.
    fn top_planes(&self) -> TopPlanes {
        let falls = self.falls();
        let derived = falls.top();
        if self.seat == [0; 4] {
            return derived;
        }
        let reach = |edge: usize| f32::from(self.seat[edge]) / f32::from(SEAT_STEPS);
        let corners = [
            Vec2::new(reach(3), reach(0)),
            Vec2::new(-reach(1), reach(0)),
            Vec2::new(-reach(1), -reach(2)),
            Vec2::new(reach(3), -reach(2)),
        ];
        falls.steepened(&derived.seat_plane(), &corners).top()
    }

    /// The top as derived: the plateau, the wall covers, and every plane that
    /// falls from the plateau to an edge or a corner, at its own slope.
    fn falls(&self) -> Falls {
        let level = SAMPLE_HEIGHTS[self.level as usize];
        let mut falls = Falls {
            plateau: Plane {
                offset: level,
                slope: Vec2::ZERO,
            },
            envelope: Vec::new(),
            walls: Vec::new(),
            caps: Vec::new(),
        };
        for edge in 0..4 {
            let out = edge_out(edge);
            let foot = SAMPLE_HEIGHTS[self.feet[edge] as usize];
            let slope = match self.edges[edge] {
                // The cover keeps the level along the wall, so the plate
                // closes against the whole face of the structure there.
                ShellEdgeType::Wall => {
                    falls.walls.push(Plane {
                        offset: level - WALL_SLOPE * REACH,
                        slope: out * WALL_SLOPE,
                    });
                    continue;
                }
                ShellEdgeType::RampEnd => continue,
                ShellEdgeType::Ramp => RAMP_SLOPE,
                ShellEdgeType::Plain | ShellEdgeType::Fold | ShellEdgeType::Skirt => CHAMFER_SLOPE,
            };
            if foot < level - EPSILON {
                falls.envelope.push(Fall {
                    base: Plane {
                        offset: foot,
                        slope: Vec2::ZERO,
                    },
                    run: Plane {
                        offset: REACH,
                        slope: -out,
                    },
                    slope,
                });
            }
        }
        // The open end of a stair plate falls along its end edge into the
        // plane the ramp's end profile lies in: the level at the wall corner,
        // the floor at the ramp corner, rising into the plate at slope two.
        for edge in (0..4).filter(|edge| self.edges[*edge] == ShellEdgeType::RampEnd) {
            if let Some(ramp) = ramp_beside(&self.edges, edge) {
                falls.envelope.push(Fall {
                    base: Plane {
                        offset: level - REACH,
                        slope: -edge_out(ramp),
                    },
                    run: Plane {
                        offset: REACH,
                        slope: -edge_out(edge),
                    },
                    slope: CHAMFER_SLOPE,
                });
            }
        }
        // A whole-cell plate is capped through any corner the neighbourhood
        // holds lower: a straight bevel, never a point, and never into the
        // plateau. Nothing is RAISED to a corner held higher - that stood
        // shards off the ramp corners.
        let uncapped = falls.top();
        for corner in 0..4 {
            let at = corner_point(corner);
            let soft = SAMPLE_HEIGHTS[self.soft_corners[corner] as usize];
            if soft < uncapped.height(at) - EPSILON {
                let (x, z) = FACE_CORNERS[corner];
                falls.caps.push(Fall {
                    base: Plane {
                        offset: soft,
                        slope: Vec2::ZERO,
                    },
                    run: Plane {
                        offset: 2.0 * REACH,
                        slope: -Vec2::new(x, z),
                    },
                    slope: CHAMFER_SLOPE,
                });
            }
        }
        falls
    }

    /// The top cut into flat pieces: the square, split along every line where
    /// two planes of the top cross while both of them ARE the top. Inside a
    /// piece the top is one plane.
    fn pieces(&self) -> Vec<TopPiece> {
        let top = self.top_planes();
        let planes: Vec<Plane> = top.all().copied().collect();
        let square: Vec<Vec2> = (0..4).map(corner_point).collect();
        let mut pieces: Vec<Vec<Vec2>> = vec![square];
        for (i, first) in planes.iter().enumerate() {
            for second in &planes[i + 1..] {
                let (c, k) = (first.offset - second.offset, first.slope - second.slope);
                if k.length() < EPSILON || !top.creases_along(first, c, k) {
                    continue;
                }
                pieces = pieces
                    .into_iter()
                    .flat_map(|piece| [clip(&piece, c, k), clip(&piece, -c, -k)])
                    .filter(|part| part.len() >= 3 && polygon_area(part) > MIN_PIECE_AREA)
                    .collect();
            }
        }
        pieces
            .into_iter()
            .map(|footprint| {
                let middle = centroid(&footprint);
                let height = top.height(middle);
                let plane = planes
                    .iter()
                    .copied()
                    .find(|plane| (plane.at(middle) - height).abs() < EPSILON)
                    .expect("the top is one of its own planes");
                TopPiece { footprint, plane }
            })
            .collect()
    }

    /// The plane of the largest flat piece of the top, and its area.
    fn largest_flat(&self) -> (Plane, f32) {
        let mut flats: Vec<(Plane, f32)> = Vec::new();
        for piece in self.pieces() {
            let area = polygon_area(&piece.footprint);
            match flats.iter_mut().find(|(plane, _)| plane.same(&piece.plane)) {
                Some((_, total)) => *total += area,
                None => flats.push((piece.plane, area)),
            }
        }
        flats
            .into_iter()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .expect("the top covers the cell")
    }
}

/// One plane of the top, as a height above the cell floor over the footprint
/// point `p = (x, z)`: `offset + slope . p`.
#[derive(Clone, Copy, Debug)]
struct Plane {
    offset: f32,
    slope: Vec2,
}

impl Plane {
    fn at(&self, p: Vec2) -> f32 {
        self.offset + self.slope.dot(p)
    }

    fn same(&self, other: &Plane) -> bool {
        (self.offset - other.offset).abs() < EPSILON && self.slope.abs_diff_eq(other.slope, EPSILON)
    }

    /// The unit normal of the surface this plane draws, pointing out.
    fn normal(&self) -> Vec3 {
        Vec3::new(-self.slope.x, 1.0, -self.slope.y).normalize()
    }
}

/// A plane of the top that rises `slope` per unit of `run` above `base`: a
/// chamfer, a ramp, a ramp end or a corner cap.
///
/// `run` is zero along the edge or at the corner the plane falls to, so a
/// steeper slope raises the plane inside the cell and leaves its foot where
/// it was.
#[derive(Clone, Copy, Debug)]
struct Fall {
    base: Plane,
    run: Plane,
    slope: f32,
}

impl Fall {
    fn plane(&self) -> Plane {
        Plane {
            offset: self.base.offset + self.run.offset * self.slope,
            slope: self.base.slope + self.run.slope * self.slope,
        }
    }

    /// This fall steepened until it stands on or above `seat` at each of
    /// `corners`, up to [`STEEPEST_SLOPE`], and never made shallower. A corner
    /// with no run left that the seat still stands above cannot be cleared;
    /// the slope goes to the steepest and [`ShellShape::seats`] refuses it.
    fn clearing(self, seat: &Plane, corners: &[Vec2; 4]) -> Self {
        let needed = corners
            .iter()
            .map(|corner| {
                let (run, rise) = (
                    self.run.at(*corner),
                    seat.at(*corner) - self.base.at(*corner),
                );
                if run > EPSILON {
                    rise / run
                } else if rise > EPSILON {
                    STEEPEST_SLOPE
                } else {
                    self.slope
                }
            })
            .fold(self.slope, f32::max);
        Self {
            slope: needed.min(STEEPEST_SLOPE).max(self.slope),
            ..self
        }
    }
}

/// The top of a shape before it is planes: the plateau, every [`Fall`] off
/// it, the wall covers, and the corner caps.
struct Falls {
    plateau: Plane,
    envelope: Vec<Fall>,
    walls: Vec<Plane>,
    caps: Vec<Fall>,
}

impl Falls {
    fn top(&self) -> TopPlanes {
        TopPlanes {
            envelope: std::iter::once(self.plateau)
                .chain(self.envelope.iter().map(Fall::plane))
                .collect(),
            walls: self.walls.clone(),
            caps: self.caps.iter().map(Fall::plane).collect(),
        }
    }

    /// Every fall but `seat` itself steepened to clear `seat` over the
    /// rectangle `corners` span.
    fn steepened(&self, seat: &Plane, corners: &[Vec2; 4]) -> Self {
        let clear = |fall: &Fall| match fall.plane().same(seat) {
            true => *fall,
            false => fall.clearing(seat, corners),
        };
        Self {
            plateau: self.plateau,
            envelope: self.envelope.iter().map(clear).collect(),
            walls: self.walls.clone(),
            caps: self.caps.iter().map(clear).collect(),
        }
    }
}

/// The top of a shape as three sets of planes: the lower ENVELOPE of the
/// plateau and its chamfers, raised to any WALL cover, and then lowered to any
/// corner CAP.
struct TopPlanes {
    envelope: Vec<Plane>,
    walls: Vec<Plane>,
    caps: Vec<Plane>,
}

impl TopPlanes {
    /// The height of the top above the cell floor at `p`.
    fn height(&self, p: Vec2) -> f32 {
        let envelope = self
            .envelope
            .iter()
            .fold(f32::MAX, |low, plane| low.min(plane.at(p)));
        let walled = self
            .walls
            .iter()
            .fold(envelope, |high, plane| high.max(plane.at(p)));
        self.caps
            .iter()
            .fold(walled, |low, plane| low.min(plane.at(p)))
    }

    fn all(&self) -> impl Iterator<Item = &Plane> {
        self.envelope.iter().chain(&self.walls).chain(&self.caps)
    }

    /// The plane under the middle of the cell, where a decoration stands.
    /// Where two planes meet under the middle, the flatter one.
    fn seat_plane(&self) -> Plane {
        let centre = self.height(Vec2::ZERO);
        self.all()
            .filter(|plane| (plane.at(Vec2::ZERO) - centre).abs() < EPSILON)
            .min_by(|a, b| a.slope.length().total_cmp(&b.slope.length()))
            .copied()
            .expect("the top at the middle is one of its own planes")
    }

    /// Whether the top lies on `plane` somewhere along the line `c + k . p =
    /// 0` inside the cell, where `plane` meets another. Only such a line is a
    /// crease; every other crossing of two planes is hidden under the top.
    ///
    /// The top along the line is straight between the points where a third
    /// plane crosses it, so testing the middle of each stretch is exact.
    fn creases_along(&self, plane: &Plane, c: f32, k: Vec2) -> bool {
        let base = -k * c / k.length_squared();
        let along = k.perp().normalize();
        let Some((enter, leave)) = span_in_square(base, along) else {
            return false;
        };
        let mut stops = vec![enter, leave];
        for other in self.all() {
            // `other` meets `plane` on the line where (other - plane) is zero.
            let rate = (other.slope - plane.slope).dot(along);
            if rate.abs() > EPSILON {
                let t = -(other.at(base) - plane.at(base)) / rate;
                if t > enter && t < leave {
                    stops.push(t);
                }
            }
        }
        stops.sort_by(f32::total_cmp);
        stops.windows(2).any(|pair| {
            let p = base + along * (pair[0] + pair[1]) * 0.5;
            pair[1] - pair[0] > EPSILON && (self.height(p) - plane.at(p)).abs() < EPSILON
        })
    }
}

/// One flat piece of the top: where it stands on the footprint and the plane
/// it lies in.
struct TopPiece {
    footprint: Vec<Vec2>,
    plane: Plane,
}

/// The corners of the rectangle `half` each way along `x` and `z` round the
/// middle of the cell.
fn rectangle(half: Vec2) -> [Vec2; 4] {
    FACE_CORNERS.map(|(x, z)| Vec2::new(x, z) * half)
}

/// Where the line `base + t * along` runs inside the cell's footprint, as the
/// range of `t`, or `None` if it misses.
fn span_in_square(base: Vec2, along: Vec2) -> Option<(f32, f32)> {
    let (mut enter, mut leave) = (f32::MIN, f32::MAX);
    for axis in 0..2 {
        if along[axis].abs() < EPSILON {
            if base[axis].abs() > REACH {
                return None;
            }
            continue;
        }
        let (a, b) = (
            (-REACH - base[axis]) / along[axis],
            (REACH - base[axis]) / along[axis],
        );
        enter = enter.max(a.min(b));
        leave = leave.min(a.max(b));
    }
    (leave - enter > EPSILON).then_some((enter, leave))
}

/// The part of a convex loop where `c + k . p <= 0`.
fn clip(poly: &[Vec2], c: f32, k: Vec2) -> Vec<Vec2> {
    let side = |p: Vec2| c + k.dot(p);
    let mut kept = Vec::new();
    for index in 0..poly.len() {
        let (a, b) = (poly[index], poly[(index + 1) % poly.len()]);
        let (sa, sb) = (side(a), side(b));
        if sa <= EPSILON {
            kept.push(a);
        }
        if (sa < -EPSILON && sb > EPSILON) || (sa > EPSILON && sb < -EPSILON) {
            kept.push(a + (b - a) * (sa / (sa - sb)));
        }
    }
    kept
}

/// The area of a convex loop.
fn polygon_area(poly: &[Vec2]) -> f32 {
    let twice: f32 = (0..poly.len())
        .map(|index| poly[index].perp_dot(poly[(index + 1) % poly.len()]))
        .sum();
    (twice * 0.5).abs()
}

/// The mean of a convex loop's corners, which is inside it.
fn centroid(poly: &[Vec2]) -> Vec2 {
    poly.iter().copied().sum::<Vec2>() / poly.len() as f32
}

/// The ramp a ramp end at `end` closes the side of: the edge beside it that
/// is a [`Ramp`](ShellEdgeType::Ramp) climbing to a
/// [`Wall`](ShellEdgeType::Wall) across the plate.
fn ramp_beside(edges: &[ShellEdgeType; 4], end: usize) -> Option<usize> {
    [(end + 1) % 4, (end + 3) % 4].into_iter().find(|ramp| {
        edges[*ramp] == ShellEdgeType::Ramp && edges[(ramp + 2) % 4] == ShellEdgeType::Wall
    })
}

/// Every point where a piece of the top meets edge `edge`, its two corners
/// included, in order from corner `edge` to corner `edge + 1`.
fn edge_points(edge: usize, pieces: &[TopPiece]) -> Vec<Vec2> {
    let (from, to) = (corner_point(edge), corner_point((edge + 1) % 4));
    let out = edge_out(edge);
    let mut points = vec![from, to];
    for piece in pieces {
        points.extend(
            piece
                .footprint
                .iter()
                .filter(|p| (out.dot(**p) - REACH).abs() < EPSILON),
        );
    }
    let direction = to - from;
    points.sort_by(|p, q| {
        direction
            .dot(*p - from)
            .total_cmp(&direction.dot(*q - from))
    });
    points.dedup_by(|p, q| p.abs_diff_eq(*q, EPSILON));
    points
}

/// Corner `corner` of the footprint, as `(x, z)`.
fn corner_point(corner: usize) -> Vec2 {
    let (x, z) = FACE_CORNERS[corner];
    Vec2::new(x, z) * REACH
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

/// [`edge_normal`] on the footprint, as `(x, z)`.
fn edge_out(edge: usize) -> Vec2 {
    let normal = edge_normal(edge);
    Vec2::new(normal.x, normal.z)
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

/// `ShellShape`, `ShellEdgeType`, `ShellSurface`, `SAMPLE_HEIGHTS`,
/// `SHELL_ID_PREFIX`, `FULL` and `HALF`.
pub mod prelude {
    pub use super::{
        ShellEdgeType, ShellShape, ShellSurface, FULL, HALF, SAMPLE_HEIGHTS, SHELL_ID_PREFIX,
    };
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use ShellEdgeType::{Fold, Plain, Ramp, RampEnd, Skirt, Wall};

    use super::*;

    /// How far off a height or a plane a reading may be and still count.
    const MESH_EPSILON: f32 = 1e-4;

    /// Footprints the sweep seats every shape for, as half extents: a long
    /// piece each way round and a wide square one.
    const SEAT_SAMPLES: [Vec2; 3] = [
        Vec2::new(0.44, 0.17),
        Vec2::new(0.2, 0.48),
        Vec2::new(0.34, 0.34),
    ];

    fn shape(level: u8, feet: [u8; 4], edges: [ShellEdgeType; 4], soft: [u8; 4]) -> ShellShape {
        ShellShape::new(level, feet, edges, soft).expect("a legal shape")
    }

    /// Every facet of a shape, across all of its surface meshes.
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
            .as_chunks::<3>()
            .0
            .iter()
            .map(|corner| std::array::from_fn(|i| Vec3::from_array(corner[i])))
            .collect()
    }

    /// What one edge may be on a plate at `level`, each type with the foot it
    /// allows. A plain edge or a ramp comes down to the floor, to half a cell,
    /// or not at all; the shapes [`ShellShape::new`] refuses drop out of the
    /// sweep.
    fn edge_options(level: u8) -> Vec<(u8, ShellEdgeType)> {
        let mut options: Vec<(u8, ShellEdgeType)> = [0, HALF, level]
            .into_iter()
            .collect::<HashSet<u8>>()
            .into_iter()
            .flat_map(|foot| [(foot, Plain), (foot, Ramp)])
            .collect();
        options.extend([(HALF, Fold), (HALF, Skirt), (0, RampEnd), (level, Wall)]);
        options.sort_unstable();
        options
    }

    /// The shapes the geometry tests sweep: every combination of edge options
    /// at both levels, with at most one ramp that is not a stair's own (one
    /// that climbs to a wall across the plate) - two would take two stairs
    /// coming down onto one plate, which no hull in the catalogue builds - and a
    /// whole-cell plate once with no caps and once with a spread of soft
    /// corners, chosen per combination so the sweep reaches every corner
    /// setting without multiplying out.
    fn sample_shapes() -> Vec<ShellShape> {
        let mut seen = HashSet::new();
        let mut shapes = Vec::new();
        for level in [HALF, FULL] {
            let options = edge_options(level);
            let count = options.len();
            for raw in 0..count.pow(4) {
                let pick: [(u8, ShellEdgeType); 4] =
                    std::array::from_fn(|i| options[(raw / count.pow(i as u32)) % count]);
                let free_ramps = (0..4)
                    .filter(|edge| pick[*edge].1 == Ramp && pick[(edge + 2) % 4].1 != Wall)
                    .count();
                if free_ramps > 1 {
                    continue;
                }
                let feet = pick.map(|(foot, _)| foot);
                let edges = pick.map(|(_, edge)| edge);
                let mut softs = vec![[level; 4]];
                if level == FULL {
                    softs.push(std::array::from_fn(|i| {
                        HALF + ((raw * 7 + i * 5) % 3) as u8
                    }));
                }
                for soft in softs {
                    // Some combinations are not a shape: a plain edge that
                    // falls a whole cell, or a ramp end with no ramp beside it.
                    let Some(built) = ShellShape::new(level, feet, edges, soft) else {
                        continue;
                    };
                    let canonical = built.canonical();
                    if seen.insert(canonical) {
                        shapes.push(canonical);
                    }
                }
            }
        }
        // Every shape again seated for each sample footprint it can take: the
        // steepened falls are new facets, and they owe the same closed,
        // in-cell, spike-free solid.
        let seated: Vec<ShellShape> = shapes
            .iter()
            .flat_map(|shape| SEAT_SAMPLES.map(|half| shape.seated(half)))
            .flatten()
            .filter(|shape| shape.seat != [0; 4] && seen.insert(*shape))
            .collect();
        shapes.extend(seated);
        shapes.sort_unstable();
        shapes
    }

    /// A plate seated for a piece has ONE PLANE under the whole footprint, and
    /// gets there only by rising: no point of its top sinks and every corner
    /// keeps its height, so the feet the neighbours meet do not move. A piece
    /// that would need a fall steeper than [`STEEPEST_SLOPE`] does not fit.
    #[test]
    fn a_seated_plate_seats_the_piece_and_only_rises() {
        const SAMPLES: usize = 21;
        let grid: Vec<Vec2> = (0..SAMPLES * SAMPLES)
            .map(|index| {
                let at = |i: usize| -REACH + 2.0 * REACH * i as f32 / (SAMPLES - 1) as f32;
                Vec2::new(at(index % SAMPLES), at(index / SAMPLES))
            })
            .collect();
        let mut seated = 0;
        for shape in sample_shapes()
            .into_iter()
            .filter(|shape| shape.seat == [0; 4])
        {
            for half in SEAT_SAMPLES {
                let Some(fitted) = shape.seated(half) else {
                    continue;
                };
                seated += usize::from(fitted.seat != [0; 4]);
                assert!(fitted.seats(half), "`{}` for {half}", fitted.id());
                let (before, after) = (shape.top_planes(), fitted.top_planes());
                for p in &grid {
                    assert!(
                        after.height(*p) >= before.height(*p) - MESH_EPSILON,
                        "`{}` sank at {p}",
                        fitted.id(),
                    );
                }
                for corner in 0..4 {
                    let at = corner_point(corner);
                    assert!(
                        (after.height(at) - before.height(at)).abs() < MESH_EPSILON,
                        "`{}` moved corner {corner}",
                        fitted.id(),
                    );
                }
            }
        }
        assert!(seated > 0, "the sweep steepens some shape");

        // A run of skin falling to open space on both sides along z keeps a
        // plateau a quarter cell each way. Seated, it reaches to within a
        // 128th of a cell of the falling edges - half a cell of drop at the
        // steepest slope - and no further; along x, where it does not fall,
        // the plateau already runs the whole cell.
        let spar = shape(HALF, [0, HALF, 0, HALF], [Plain; 4], [HALF; 4]);
        assert!(spar.seats(Vec2::new(0.5, 0.25)));
        assert!(!spar.seats(Vec2::new(0.4, 0.4)));
        let fitted = spar
            .seated(Vec2::new(0.4, 0.49))
            .expect("fits under the steepest slope");
        assert!(fitted.seats(Vec2::new(0.4, 0.49)), "`{}`", fitted.id());
        assert_eq!(
            spar.seated(Vec2::new(0.4, 0.495)),
            None,
            "past the steepest slope"
        );
        assert_eq!(
            spar.seated(Vec2::new(0.51, 0.1)),
            None,
            "wider than the cell"
        );
    }

    /// Every generated tile is a CLOSED SOLID: every edge shared by exactly
    /// two triangles. A hole shows as a see-through facet the moment the tile
    /// is lit, and a skirt or a capped corner is where one would hide.
    #[test]
    fn every_generated_tile_is_a_closed_solid() {
        // Snapped to a 1e-4 grid shifted off the half step: a seat reach is a
        // 64th of a cell, which lands a coordinate exactly halfway between two
        // unshifted grid points, where one ulp of noise picks the side.
        let key = |point: Vec3| -> [i64; 3] {
            std::array::from_fn(|axis| (point[axis] * 1e4 + 0.3183).round() as i64)
        };
        for shape in sample_shapes() {
            let mut edges: HashMap<([i64; 3], [i64; 3]), usize> = HashMap::new();
            for triangle in facets(&shape) {
                for step in 0..3 {
                    let (a, b) = (key(triangle[step]), key(triangle[(step + 1) % 3]));
                    *edges
                        .entry(if a < b { (a, b) } else { (b, a) })
                        .or_default() += 1;
                }
            }
            let unpaired = edges.values().filter(|n| **n != 2).count();
            assert_eq!(unpaired, 0, "`{}` is not a closed solid", shape.id());
        }
    }

    /// A tile stays inside its own cell, but for a skirt: one plate of the two
    /// at a convex corner owns the 45 degree face that closes it, and that face
    /// reaches past the cell over the open side by no more than the edge is
    /// high.
    #[test]
    fn only_a_skirt_reaches_past_the_cell() {
        for shape in sample_shapes() {
            let level = SAMPLE_HEIGHTS[shape.level as usize];
            for point in facets(&shape).into_iter().flatten() {
                for edge in 0..4 {
                    let past = edge_out(edge).dot(Vec2::new(point.x, point.z)) - REACH;
                    if past <= MESH_EPSILON {
                        continue;
                    }
                    assert_eq!(
                        shape.edges[edge],
                        Skirt,
                        "`{}` reaches {point:?} past edge {edge}, which owns no skirt",
                        shape.id(),
                    );
                    assert!(
                        past <= level + MESH_EPSILON,
                        "`{}` reaches {point:?}",
                        shape.id()
                    );
                }
                assert!(
                    (-REACH - MESH_EPSILON..=REACH + MESH_EPSILON).contains(&point.y),
                    "`{}` leaves its cell at {point:?}",
                    shape.id(),
                );
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

        // A lone cell's plate falls to the floor on every side, so its walls
        // have no height.
        let lone = shape(HALF, [0; 4], [Plain; 4], [HALF; 4]);
        assert_eq!(roles(&lone), vec![ShellSurface::Top, ShellSurface::Floor]);

        // A full block closes every side, so it carries all three.
        let block = shape(FULL, [FULL; 4], [Plain; 4], [FULL; 4]);
        assert_eq!(
            roles(&block),
            vec![ShellSurface::Top, ShellSurface::Wall, ShellSurface::Floor],
        );

        for shape in sample_shapes() {
            let mut seen = HashSet::new();
            for (role, mesh) in &shape.surfaces() {
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
            assert!(seen.contains(&ShellSurface::Floor), "`{}`", shape.id());
            assert!(seen.contains(&ShellSurface::Top), "`{}`", shape.id());
        }
    }

    /// No plate has a POINT SPIKE: the highest ground of every top is a flat
    /// or a crest at least a quarter cell across, never a lone point.
    ///
    /// The two spikes the eight-sample skin drew, and what this catches them
    /// as: a midpoint raised between two floor corners peaked at that midpoint,
    /// and a single corner climbing the whole cell over a floor-level rest of
    /// the plate peaked at that corner.
    #[test]
    fn no_plate_has_a_point_spike() {
        const SAMPLES: usize = 41;
        let grid: Vec<Vec2> = (0..SAMPLES * SAMPLES)
            .map(|index| {
                let at = |i: usize| -REACH + 2.0 * REACH * i as f32 / (SAMPLES - 1) as f32;
                Vec2::new(at(index % SAMPLES), at(index / SAMPLES))
            })
            .collect();
        for shape in sample_shapes() {
            let top = shape.top_planes();
            let heights: Vec<f32> = grid.iter().map(|p| top.height(*p)).collect();
            let peak = heights.iter().copied().fold(f32::MIN, f32::max);
            let highest: Vec<Vec2> = grid
                .iter()
                .zip(&heights)
                .filter(|(_, height)| **height > peak - MESH_EPSILON)
                .map(|(p, _)| *p)
                .collect();
            let across = highest
                .iter()
                .flat_map(|a| highest.iter().map(move |b| a.distance(*b)))
                .fold(0.0, f32::max);
            assert!(
                across >= 0.25 - MESH_EPSILON,
                "`{}` peaks in a point {across} cells across",
                shape.id(),
            );
        }
    }

    /// A slab chamfered on both sides keeps a FLAT TOP: the band between the
    /// two chamfers stays at the plate's level the whole width of the cell, so
    /// a run of skin one cell wide is a flat-topped strip and not a tent.
    #[test]
    fn a_slab_chamfered_on_both_sides_keeps_a_flat_top() {
        for (level, soft) in [(HALF, HALF), (FULL, FULL), (FULL, HALF)] {
            let foot = level - HALF;
            let slab = shape(level, [foot, level, foot, level], [Plain; 4], [soft; 4]);
            let top = slab.top_planes();
            let height = SAMPLE_HEIGHTS[level as usize];
            for step in 0..=20 {
                let x = -REACH + step as f32 * 0.05;
                for z in [-0.25, 0.0, 0.25] {
                    assert!(
                        (top.height(Vec2::new(x, z)) - height).abs() < MESH_EPSILON,
                        "`{}` dips off its level at ({x}, {z})",
                        slab.id(),
                    );
                }
            }
            assert!(
                (slab.flat_area() - 0.5).abs() < MESH_EPSILON,
                "`{}`",
                slab.id()
            );
            assert!(
                slab.top_normal().abs_diff_eq(Vec3::Y, MESH_EPSILON),
                "`{}`",
                slab.id()
            );
        }
    }
}
