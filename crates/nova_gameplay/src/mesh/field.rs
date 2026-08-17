//! A solid described as a SIGNED FIELD on a grid, and the surface it defines.
//!
//! The representation a carvable body needs, and the reason a height field
//! could not be one: a value per grid CORNER says how far that point is outside
//! the solid (positive) or inside it (negative), so the solid may be any shape
//! at all - concave, holed, hollowed. Subtracting a sphere from it is one line
//! of arithmetic, which is exactly what a hit does.
//!
//! # Seeded analytically, carved incrementally
//!
//! The field is SAMPLED once from a function - for an asteroid,
//! `|p| - (1 + height(p/|p|))`, the same noise its pristine mesh was displaced
//! by, so the seeded field and the shipped silhouette are the same shape. After
//! that nothing resamples: a carve touches only the cells its sphere reaches,
//! and remeshing reads the stored grid. The expensive step happens once, on the
//! first hit, and only for bodies that are ever hit at all.
//!
//! # Meshed by NAIVE SURFACE NETS
//!
//! One vertex per sign-changing cell, placed at the average of its edge
//! crossings, and a quad across every sign-changing grid edge. Chosen over
//! marching cubes because it has no 256-case table and produces no
//! near-degenerate slivers - slivers hurt the look and make a worse trimesh
//! collider. Dual contouring was rejected: it wants hermite data and QEF solves
//! to recover sharp features that a noise blob does not have.
//!
//! The output is a [`TriangleMeshBuilder`], which is what keeps a carved rock
//! looking like an uncarved one: it emits unindexed triangles with per-FACE
//! normals and the same planar UVs the shipped mesh already uses. The coarse
//! faceting is the art direction, not an artifact - do not smooth the normals,
//! and do not raise the resolution to "fix" the blobbiness.

use bevy::{math::Vec3, prelude::Triangle3d};

use super::builder::TriangleMeshBuilder;

/// The eight corners of a cell, as offsets, in the order the edge table names
/// them.
const CELL_CORNERS: [[usize; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [0, 1, 0],
    [1, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [0, 1, 1],
    [1, 1, 1],
];

/// The twelve edges of a cell, as pairs of indices into [`CELL_CORNERS`].
const CELL_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (2, 3),
    (4, 5),
    (6, 7),
    (0, 2),
    (1, 3),
    (4, 6),
    (5, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// A solid stored as one signed distance per grid corner.
///
/// Negative is INSIDE. The grid spans `[-half_extent, half_extent]` on every
/// axis with `resolution` cells per axis, so it holds `(resolution + 1)^3`
/// corner samples.
#[derive(Clone, Debug)]
pub struct SignedField {
    resolution: usize,
    half_extent: f32,
    corners: Vec<f32>,
}

impl SignedField {
    /// Sample `field` onto a fresh grid.
    ///
    /// `field` answers the signed distance at a point: negative inside the
    /// solid, positive outside. It is called once per corner and never again,
    /// which is what makes an expensive analytic field (a fourteen-octave
    /// noise graph, say) affordable.
    pub fn sample(resolution: usize, half_extent: f32, field: impl Fn(Vec3) -> f32) -> Self {
        let resolution = resolution.max(1);
        let stride = resolution + 1;
        let mut grid = Self {
            resolution,
            half_extent,
            corners: vec![0.0; stride * stride * stride],
        };
        for z in 0..stride {
            for y in 0..stride {
                for x in 0..stride {
                    let at = grid.corner_position(x, y, z);
                    let index = grid.corner_index(x, y, z);
                    grid.corners[index] = field(at);
                }
            }
        }
        grid
    }

    /// How many cells the grid has per axis.
    pub fn resolution(&self) -> usize {
        self.resolution
    }

    /// How far the grid reaches from the origin on each axis.
    pub fn half_extent(&self) -> f32 {
        self.half_extent
    }

    /// The width of one cell.
    pub fn cell_size(&self) -> f32 {
        self.half_extent * 2.0 / self.resolution as f32
    }

    /// Take a sphere out of the solid: `d(p) = max(d(p), radius - |p - centre|)`.
    ///
    /// The union of a solid and the COMPLEMENT of a ball, which for signed
    /// fields is a max. Only the cells the sphere reaches are visited, so a
    /// pinprick on a big rock costs almost nothing however large the grid is.
    ///
    /// Monotone by construction: a max can only raise a value, and raising a
    /// value can only remove material. A carve can never grow a body back.
    pub fn subtract_sphere(&mut self, centre: Vec3, radius: f32) {
        if radius <= 0.0 {
            return;
        }
        let stride = self.resolution + 1;
        // One cell of slop, so the corners just outside the sphere - the ones
        // whose edges the surface actually crosses - are updated too.
        let reach = radius + self.cell_size();
        let lower = self.corner_of(centre - Vec3::splat(reach));
        let upper = self.corner_of(centre + Vec3::splat(reach));

        for z in lower[2]..=upper[2].min(stride - 1) {
            for y in lower[1]..=upper[1].min(stride - 1) {
                for x in lower[0]..=upper[0].min(stride - 1) {
                    let at = self.corner_position(x, y, z);
                    let carved = radius - at.distance(centre);
                    let index = self.corner_index(x, y, z);
                    self.corners[index] = self.corners[index].max(carved);
                }
            }
        }
    }

    /// The surface of the solid, as flat-shaded triangles in the grid's own
    /// space.
    ///
    /// Empty when the solid has no surface in the domain at all - carved away
    /// entirely, or never seeded. A caller that draws the result has to handle
    /// that rather than assume a mesh comes back.
    pub fn surface(&self) -> TriangleMeshBuilder {
        let cells = self.resolution;
        let mut vertices: Vec<Option<Vec3>> = vec![None; cells * cells * cells];

        for z in 0..cells {
            for y in 0..cells {
                for x in 0..cells {
                    if let Some(vertex) = self.cell_vertex(x, y, z) {
                        vertices[(z * cells + y) * cells + x] = Some(vertex);
                    }
                }
            }
        }

        let mut builder = TriangleMeshBuilder::new_empty();
        let stride = cells + 1;
        for z in 0..stride {
            for y in 0..stride {
                for x in 0..stride {
                    for axis in 0..3 {
                        self.quad_across_edge(&vertices, [x, y, z], axis, &mut builder);
                    }
                }
            }
        }
        builder
    }

    /// How far the surface reaches from the origin, or `0.0` when there is no
    /// surface left.
    ///
    /// What a caller shrinks a body's published radius by. Read off the CELL
    /// vertices rather than off the corners, so it measures the surface that is
    /// actually drawn.
    pub fn surface_radius(&self) -> f32 {
        let cells = self.resolution;
        let mut furthest: f32 = 0.0;
        for z in 0..cells {
            for y in 0..cells {
                for x in 0..cells {
                    if let Some(vertex) = self.cell_vertex(x, y, z) {
                        furthest = furthest.max(vertex.length());
                    }
                }
            }
        }
        furthest
    }

    /// The one vertex a cell contributes, at the average of its edge crossings,
    /// or `None` when the surface does not pass through it.
    fn cell_vertex(&self, x: usize, y: usize, z: usize) -> Option<Vec3> {
        let mut sum = Vec3::ZERO;
        let mut crossings = 0u32;
        for (a, b) in CELL_EDGES {
            let (ax, ay, az) = (
                x + CELL_CORNERS[a][0],
                y + CELL_CORNERS[a][1],
                z + CELL_CORNERS[a][2],
            );
            let (bx, by, bz) = (
                x + CELL_CORNERS[b][0],
                y + CELL_CORNERS[b][1],
                z + CELL_CORNERS[b][2],
            );
            let da = self.corners[self.corner_index(ax, ay, az)];
            let db = self.corners[self.corner_index(bx, by, bz)];
            if da.is_sign_negative() == db.is_sign_negative() {
                continue;
            }
            // Where along the edge the field passes zero. The denominator
            // cannot vanish: the two values differ in sign, so they differ.
            let along = da / (da - db);
            sum += self
                .corner_position(ax, ay, az)
                .lerp(self.corner_position(bx, by, bz), along);
            crossings += 1;
        }
        (crossings > 0).then(|| sum / crossings as f32)
    }

    /// Emit the quad joining the four cells around one sign-changing grid edge.
    ///
    /// Wound so the face points from inside to outside - the corner with the
    /// negative value is behind the surface - which is what makes the flat
    /// per-face normals come out pointing away from the solid.
    fn quad_across_edge(
        &self,
        vertices: &[Option<Vec3>],
        corner: [usize; 3],
        axis: usize,
        builder: &mut TriangleMeshBuilder,
    ) {
        let cells = self.resolution;
        let stride = cells + 1;
        let mut next = corner;
        next[axis] += 1;
        if next[axis] >= stride {
            return;
        }

        let here = self.corners[self.corner_index(corner[0], corner[1], corner[2])];
        let there = self.corners[self.corner_index(next[0], next[1], next[2])];
        if here.is_sign_negative() == there.is_sign_negative() {
            return;
        }

        // The four cells sharing this edge sit at offsets 0 and -1 along the
        // two axes the edge does not run along.
        let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
        if corner[axis] >= cells || corner[u] == 0 || corner[u] > cells - 1 {
            return;
        }
        if corner[v] == 0 || corner[v] > cells - 1 {
            return;
        }

        let mut quad = [Vec3::ZERO; 4];
        // Wound counter-clockwise about +axis, so a solid on the negative side
        // of the edge faces outward along it.
        for (slot, (du, dv)) in [(0, 0), (0, -1i32), (-1, -1), (-1, 0)]
            .into_iter()
            .enumerate()
        {
            let mut cell = corner;
            cell[u] = (cell[u] as i32 + du) as usize;
            cell[v] = (cell[v] as i32 + dv) as usize;
            let index = (cell[2] * cells + cell[1]) * cells + cell[0];
            match vertices[index] {
                Some(vertex) => quad[slot] = vertex,
                // A sign-changing edge always leaves a vertex in all four of
                // its cells, so this cannot fire; bail rather than draw a
                // triangle through the origin if it ever does.
                None => return,
            }
        }

        // `here` negative means the solid is on the LOW side of the edge, so
        // the surface faces along +axis and the quad has to be wound to match;
        // the other way round flips it. Which of the two orders is which is
        // not something to reason about from the axis permutation - it is
        // pinned by the outward-normal test, which is the only thing that can
        // tell an inside-out mesh from a correct one.
        let (a, b, c, d) = match here.is_sign_negative() {
            true => (quad[3], quad[2], quad[1], quad[0]),
            false => (quad[0], quad[1], quad[2], quad[3]),
        };
        builder.add_triangle(Triangle3d::new(a, b, c));
        builder.add_triangle(Triangle3d::new(a, c, d));
    }

    /// Where a grid corner sits in the grid's own space.
    fn corner_position(&self, x: usize, y: usize, z: usize) -> Vec3 {
        let step = self.cell_size();
        Vec3::new(
            -self.half_extent + x as f32 * step,
            -self.half_extent + y as f32 * step,
            -self.half_extent + z as f32 * step,
        )
    }

    /// The flat index of a grid corner.
    fn corner_index(&self, x: usize, y: usize, z: usize) -> usize {
        let stride = self.resolution + 1;
        (z * stride + y) * stride + x
    }

    /// The grid corner at or below `at` on every axis, clamped into the domain.
    fn corner_of(&self, at: Vec3) -> [usize; 3] {
        let step = self.cell_size();
        let last = self.resolution;
        std::array::from_fn(|axis| {
            let along = (at[axis] + self.half_extent) / step;
            (along.floor().max(0.0) as usize).min(last)
        })
    }
}

#[cfg(test)]
mod tests {
    use bevy::mesh::{Mesh, MeshBuilder, VertexAttributeValues};

    use super::*;

    /// A ball of `radius` about the origin.
    fn ball(radius: f32) -> impl Fn(Vec3) -> f32 {
        move |at: Vec3| at.length() - radius
    }

    fn triangles(builder: &TriangleMeshBuilder) -> Vec<[Vec3; 3]> {
        let mesh = builder.build();
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("a built mesh has positions");
        };
        positions
            .chunks_exact(3)
            .map(|face| std::array::from_fn(|slot| Vec3::from_array(face[slot])))
            .collect()
    }

    /// THE claim the representation exists for: it draws a closed surface where
    /// the field changes sign, and that surface is where the solid says it is.
    #[test]
    fn a_ball_meshes_as_a_shell_at_its_own_radius() {
        let field = SignedField::sample(24, 4.0, ball(2.0));
        let faces = triangles(&field.surface());

        assert!(!faces.is_empty(), "a ball has a surface");
        for face in &faces {
            for vertex in face {
                let off = (vertex.length() - 2.0).abs();
                assert!(
                    off < field.cell_size(),
                    "a vertex sat {off} off the ball's own radius"
                );
            }
        }
    }

    /// Winding is not cosmetic: an inside-out mesh renders as a hole and builds
    /// a collider whose inside and outside are swapped. Every face of a ball
    /// has to point away from its centre.
    #[test]
    fn every_face_of_a_ball_points_outward() {
        let field = SignedField::sample(20, 4.0, ball(2.0));
        for face in triangles(&field.surface()) {
            let centroid = (face[0] + face[1] + face[2]) / 3.0;
            let normal = (face[1] - face[0]).cross(face[2] - face[0]);
            assert!(
                normal.dot(centroid) > 0.0,
                "a face pointed inward at {centroid}"
            );
        }
    }

    /// A carve takes a BITE, and the bite is where the sphere was. This is the
    /// thing a height field structurally could not do.
    #[test]
    fn a_subtracted_sphere_leaves_a_hollow_where_it_landed() {
        let mut field = SignedField::sample(24, 4.0, ball(2.0));
        let before = triangles(&field.surface()).len();

        // A bite out of the +X pole.
        field.subtract_sphere(Vec3::new(2.0, 0.0, 0.0), 1.0);
        let faces = triangles(&field.surface());

        assert!(!faces.is_empty(), "the ball still has a surface");
        assert!(
            faces.len() > before,
            "a crater ADDS surface: {before} -> {}",
            faces.len()
        );
        // Nothing survives at the middle of the bite, and the far pole is
        // untouched.
        let bite = Vec3::new(2.0, 0.0, 0.0);
        let nearest = faces
            .iter()
            .flatten()
            .map(|vertex| vertex.distance(bite))
            .fold(f32::MAX, f32::min);
        assert!(
            nearest > 0.5,
            "material survived inside the carved sphere, {nearest} from its centre"
        );
        let far = faces
            .iter()
            .flatten()
            .map(|vertex| vertex.distance(Vec3::new(-2.0, 0.0, 0.0)))
            .fold(f32::MAX, f32::min);
        assert!(far < field.cell_size(), "the far side was disturbed");
    }

    /// Carving only ever removes, however many spheres land and however they
    /// overlap. A body that grew back under fire would be the bug this whole
    /// representation is meant to make impossible.
    #[test]
    fn carving_only_ever_removes_material() {
        let mut field = SignedField::sample(20, 4.0, ball(2.5));
        let mut previous = field.surface_radius();

        for step in 0..6 {
            let angle = step as f32;
            field.subtract_sphere(Vec3::new(angle.cos(), angle.sin(), 0.0) * 2.5, 0.8);
            let now = field.surface_radius();
            assert!(
                now <= previous + 1e-4,
                "the surface grew at step {step}: {previous} -> {now}"
            );
            previous = now;
        }
    }

    /// A body carved away to nothing has no surface, and the caller has to be
    /// able to tell rather than being handed a mesh of no triangles that looks
    /// like a build failure.
    #[test]
    fn a_solid_carved_away_entirely_has_no_surface() {
        let mut field = SignedField::sample(16, 4.0, ball(1.0));
        field.subtract_sphere(Vec3::ZERO, 3.0);

        assert!(triangles(&field.surface()).is_empty());
        assert_eq!(field.surface_radius(), 0.0);
    }

    /// A carve reaches exactly as far as its sphere. The cost of a pinprick on
    /// a big rock has to stay a pinprick, which is what lets the grid be large.
    #[test]
    fn a_carve_leaves_the_far_side_of_the_grid_alone() {
        let mut field = SignedField::sample(16, 4.0, ball(2.0));
        let far = field.corner_index(1, 1, 1);
        let before = field.corners[far];

        field.subtract_sphere(Vec3::new(2.0, 0.0, 0.0), 0.5);

        assert_eq!(field.corners[far], before);
    }
}
