//! A triangle mesh builder for Bevy.
//!
//! This module provides utilities to create, manipulate, and convert triangle-based 3D meshes.
//! It supports:
//! - Creating basic primitives like octahedrons
//! - Subdividing faces for higher resolution
//! - Applying procedural noise to vertices
//! - Slicing meshes along planes
//! - Generating normals and UVs
//! - Converting to and from `Mesh`
//!
//! Example usage:
//!
//! ```rust
//! let mut builder = TriangleMeshBuilder::new_octahedron(2);
//! builder.apply_noise(&my_noise_fn);
//! let mesh = builder.build();
//! ```
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};
use noise::NoiseFn;

use crate::meth::prelude::*;

pub mod prelude {
    pub use super::TriangleMeshBuilder;
}

/// A triangle mesh builder that stores a collection of 3D triangles.
#[derive(Clone, Debug, Default)]
pub struct TriangleMeshBuilder {
    pub triangles: Vec<Triangle3d>,
}

impl TriangleMeshBuilder {
    /// Create an empty mesh builder with no triangles.
    pub fn new_empty() -> Self {
        Self {
            triangles: Vec::new(),
        }
    }

    /// Create a subdivided octahedron mesh with a given resolution.
    ///
    /// Each triangular face is recursively subdivided `resolution` times.
    pub fn new_octahedron(resolution: u32) -> Self {
        let mut builder = TriangleMeshBuilder::default();

        let up = Vec3::Y;
        let down = -Vec3::Y;
        let left = -Vec3::X;
        let right = Vec3::X;
        let forward = Vec3::Z;
        let back = -Vec3::Z;

        let faces = [
            (up, back, left),
            (up, right, back),
            (up, forward, right),
            (up, left, forward),
            (down, left, back),
            (down, back, right),
            (down, right, forward),
            (down, forward, left),
        ];

        for (a, b, c) in faces {
            builder.subdivide_face(a, b, c, resolution);
        }

        builder
    }

    /// Add a triangle to the mesh.
    pub fn add_triangle(&mut self, t: Triangle3d) -> &mut Self {
        self.triangles.push(t);
        self
    }

    /// Apply procedural noise to all vertices using a 3D noise function.
    ///
    /// The noise value is added along the normalized vertex direction.
    pub fn apply_noise(&mut self, noise_fn: &impl NoiseFn<f64, 3>) -> &mut Self {
        let (vertices, indices) = self.vertices_and_indices();

        let height_values = vertices
            .iter()
            .map(|&p| noise_fn.get([p.x as f64, p.y as f64, p.z as f64]) as f32)
            .collect::<Vec<_>>();

        let positions = vertices
            .iter()
            .zip(height_values.iter())
            .map(|(pos, height)| pos + pos.normalize() * *height)
            .collect::<Vec<_>>();

        self.triangles = indices
            .chunks(3)
            .map(|c| {
                Triangle3d::new(
                    positions[c[0] as usize],
                    positions[c[1] as usize],
                    positions[c[2] as usize],
                )
            })
            .collect::<Vec<_>>();

        self
    }

    /// Slice the mesh along a plane defined by `plane_normal` and `plane_point`.
    ///
    /// Returns `Some((positive_side, negative_side))` if the slice produces
    /// two non-empty meshes, otherwise `None`.
    pub fn slice(&self, plane_normal: Vec3, plane_point: Vec3) -> Option<(Self, Self)> {
        let triangles = self.triangles.clone();

        let mut positive_mesh_builder = TriangleMeshBuilder::default();
        let mut negative_mesh_builder = TriangleMeshBuilder::default();

        let mut boundary = vec![];
        for tri in triangles {
            match triangle_slice(tri, plane_normal, plane_point) {
                (TriangleSliceResult::Single(tri), true) => {
                    positive_mesh_builder.add_triangle(tri);
                }
                (TriangleSliceResult::Single(tri), false) => {
                    negative_mesh_builder.add_triangle(tri);
                }
                (TriangleSliceResult::Split(single, first, second), true) => {
                    boundary.push(single.vertices[2]);
                    boundary.push(single.vertices[1]);

                    positive_mesh_builder.add_triangle(single);
                    negative_mesh_builder.add_triangle(first);
                    negative_mesh_builder.add_triangle(second);
                }
                (TriangleSliceResult::Split(single, first, second), false) => {
                    boundary.push(single.vertices[1]);
                    boundary.push(single.vertices[2]);

                    negative_mesh_builder.add_triangle(single);
                    positive_mesh_builder.add_triangle(first);
                    positive_mesh_builder.add_triangle(second);
                }
            }
        }

        positive_mesh_builder.fill_boundary(&boundary);
        negative_mesh_builder.fill_boundary(&boundary.iter().rev().cloned().collect::<Vec<_>>());

        if positive_mesh_builder.is_empty() || negative_mesh_builder.is_empty() {
            return None;
        }

        Some((positive_mesh_builder, negative_mesh_builder))
    }

    /// Fill a boundary with triangles to close holes after slicing.
    ///
    /// Assumes the boundary is a polygon and fills triangles toward its centroid.
    pub fn fill_boundary(&mut self, boundary: &[Vec3]) -> &Self {
        if boundary.len() < 3 {
            return self;
        }

        let center = boundary.iter().fold(Vec3::ZERO, |acc, v| acc + v) / (boundary.len() as f32);

        let reordered = boundary.to_vec();
        for i in (0..reordered.len()).step_by(2) {
            let a = reordered[i];
            let b = reordered[i + 1];

            let t = Triangle3d::new(a, b, center);

            self.add_triangle(t);
        }

        self
    }

    /// Extract all vertices and triangle indices from the mesh.
    pub fn vertices_and_indices(&self) -> (Vec<Vec3>, Vec<u32>) {
        let mut base = 0;
        let mut vertices = vec![];
        let mut indices = vec![];

        for t in &self.triangles {
            vertices.push(t.vertices[0]);
            vertices.push(t.vertices[1]);
            vertices.push(t.vertices[2]);

            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);

            base += 3;
        }

        (vertices, indices)
    }

    /// Compute per-vertex normals based on triangle faces.
    pub fn normals(&self) -> Vec<Vec3> {
        let mut normals = vec![];

        for t in &self.triangles {
            let normal = t.normal().unwrap_or(Dir3::Y).into();

            normals.push(normal);
            normals.push(normal);
            normals.push(normal);
        }

        normals
    }

    /// Compute simple planar UVs for the mesh.
    pub fn uvs(&self) -> Vec<Vec2> {
        let mut uvs = vec![];

        for t in &self.triangles {
            let a = t.vertices[0];
            let b = t.vertices[1];
            let c = t.vertices[2];

            let u_axis = (b - a).normalize();
            let v_axis = t.normal().unwrap_or(Dir3::Y).cross(u_axis).normalize();

            for v in [a, b, c] {
                let local = v - a;
                uvs.push(Vec2::new(local.dot(u_axis), local.dot(v_axis)));
            }
        }

        uvs
    }

    /// Returns true if there are no triangles in the mesh.
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// Recursively subdivide a triangle face to increase mesh resolution.
    fn subdivide_face(&mut self, a: Vec3, b: Vec3, c: Vec3, depth: u32) {
        if depth == 0 {
            self.add_triangle(Triangle3d::new(a, b, c));
        } else {
            let ab = slerp(a, b, 0.5);
            let bc = slerp(b, c, 0.5);
            let ca = slerp(c, a, 0.5);

            // Recursively subdivide into four smaller triangles
            self.subdivide_face(a, ab, ca, depth - 1);
            self.subdivide_face(b, bc, ab, depth - 1);
            self.subdivide_face(c, ca, bc, depth - 1);
            self.subdivide_face(ab, bc, ca, depth - 1);
        }
    }
}

impl MeshBuilder for TriangleMeshBuilder {
    /// Build a Bevy Mesh from the triangle mesh builder.
    fn build(&self) -> Mesh {
        let (vertices, indices) = self.vertices_and_indices();
        let normals = self.normals();
        let uvs = self.uvs();

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vertices.iter().map(|v| [v.x, v.y, v.z]).collect::<Vec<_>>(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            normals.iter().map(|n| [n.x, n.y, n.z]).collect::<Vec<_>>(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            uvs.iter().map(|u| [u.x, u.y]).collect::<Vec<_>>(),
        )
        .with_inserted_indices(Indices::U32(indices.to_vec()))
    }
}

impl From<Mesh> for TriangleMeshBuilder {
    /// Convert a Bevy Mesh into a TriangleMeshBuilder.
    fn from(mesh: Mesh) -> Self {
        let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
            VertexAttributeValues::Float32x3(vals) => {
                vals.iter().map(|v| Vec3::from(*v)).collect::<Vec<_>>()
            }
            _ => panic!("Unsupported position format"),
        };

        let triangles = match mesh.indices().unwrap() {
            Indices::U32(indices) => indices.to_vec(),
            Indices::U16(indices) => indices.iter().map(|&i| i as u32).collect::<Vec<_>>(),
        }
        .chunks(3)
        .map(|c| {
            Triangle3d::new(
                positions[c[0] as usize],
                positions[c[1] as usize],
                positions[c[2] as usize],
            )
        })
        .collect::<Vec<_>>();

        Self { triangles }
    }
}

/// Compute intersection between an edge and a plane.
fn edge_plane_intersection(a: Vec3, b: Vec3, plane_point: Vec3, plane_normal: Vec3) -> Vec3 {
    let ab = b - a;
    let t = (plane_point - a).dot(plane_normal) / ab.dot(plane_normal);

    a + ab * t
}

/// Result of slicing a triangle against a plane.
enum TriangleSliceResult {
    Single(Triangle3d),
    Split(Triangle3d, Triangle3d, Triangle3d),
}

/// Slice a triangle along a plane.
///
/// Returns a tuple containing the slice result and a boolean indicating
/// whether the lonely vertex is on the positive side of the plane.
fn triangle_slice(
    tri: Triangle3d,
    plane_normal: Vec3,
    plane_point: Vec3,
) -> (TriangleSliceResult, bool) {
    let d0 = plane_normal.dot(tri.vertices[0] - plane_point);
    let d1 = plane_normal.dot(tri.vertices[1] - plane_point);
    let d2 = plane_normal.dot(tri.vertices[2] - plane_point);

    let sides = [d0 >= 0.0, d1 >= 0.0, d2 >= 0.0];

    if sides[0] && sides[1] && sides[2] {
        (TriangleSliceResult::Single(tri), true)
    } else if !sides[0] && !sides[1] && !sides[2] {
        (TriangleSliceResult::Single(tri), false)
    } else {
        let lonely_index = if sides[0] == sides[1] {
            2
        } else if sides[0] == sides[2] {
            1
        } else {
            0
        };
        let (lonely, first, second) = match lonely_index {
            0 => (tri.vertices[0], tri.vertices[2], tri.vertices[1]),
            1 => (tri.vertices[1], tri.vertices[0], tri.vertices[2]),
            2 => (tri.vertices[2], tri.vertices[1], tri.vertices[0]),
            _ => unreachable!(),
        };

        let lonely_side = sides[lonely_index];
        let first_int = edge_plane_intersection(lonely, first, plane_point, plane_normal);
        let second_int = edge_plane_intersection(lonely, second, plane_point, plane_normal);

        let single = Triangle3d::new(lonely, second_int, first_int);
        let tri1 = Triangle3d::new(first, first_int, second);
        let tri2 = Triangle3d::new(second, first_int, second_int);

        (TriangleSliceResult::Split(single, tri1, tri2), lonely_side)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_edge_plane_intersection() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let plane_point = Vec3::new(0.5, 0.0, 0.0);
        let plane_normal = Vec3::new(1.0, 0.0, 0.0);

        let intersection = edge_plane_intersection(a, b, plane_point, plane_normal);

        assert_eq!(intersection, Vec3::new(0.5, 0.0, 0.0));
    }

    #[test]
    fn test_triangle_slice() {
        let tri = Triangle3d::new(
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
        );
        let plane_point = Vec3::new(0.0, 0.0, 0.0);
        let plane_normal = Vec3::new(0.0, 1.0, 0.0);

        let (result, is_positive) = triangle_slice(tri, plane_normal, plane_point);

        assert!(
            matches!(result, TriangleSliceResult::Split(_, _, _)),
            "Expected triangle to be split"
        );
        assert!(is_positive, "Expected lonely vertex to be on positive side");
    }
}
