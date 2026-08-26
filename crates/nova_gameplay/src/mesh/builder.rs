//! A triangle mesh builder for Bevy.
//!
//! This module provides utilities to create, manipulate, and convert triangle-based 3D meshes.
//! It supports:
//! - Creating basic primitives like octahedrons
//! - Subdividing faces for higher resolution
//! - Applying procedural noise to vertices
//! - Generating normals and UVs
//! - Converting to and from `Mesh`
//!
//! Example usage:
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_gameplay::mesh::prelude::*;
//! # let my_noise_fn = noise::Fbm::<noise::Perlin>::new(0);
//! let mut builder = TriangleMeshBuilder::new_octahedron(2);
//! builder.apply_noise(&my_noise_fn);
//! let mesh = builder.build();
//! ```
//!
//! Nova owns this because every procedural mesh in the game is grown here -
//! asteroids, thruster exhausts and the velocity indicator - so subdivision
//! depth and noise shaping are art decisions.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};
use noise::NoiseFn;

use crate::math::prelude::*;

/// The `TriangleMeshBuilder`.
pub mod prelude {
    pub use super::TriangleMeshBuilder;
}

/// A triangle mesh builder that stores a collection of 3D triangles.
#[derive(Clone, Debug, Default)]
pub struct TriangleMeshBuilder {
    /// The triangles accumulated so far, in local space.
    pub triangles: Vec<Triangle3d>,
}

impl TriangleMeshBuilder {
    /// Create an empty mesh builder with no triangles.
    pub fn new_empty() -> Self {
        Self {
            triangles: Vec::new(),
        }
    }

    /// Create a builder from anything convertible into a [`Mesh`].
    pub fn new<M>(mesh: M) -> Self
    where
        M: Into<Mesh>,
    {
        Self::from(mesh.into())
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
            builder.subdivide_face_sphere(a, b, c, resolution);
        }

        builder
    }

    /// Create a cone with a given number of radial and height subdivisions.
    ///
    /// - `radial_subdivisions`: number of slices around the circumference (>= 3)
    /// - `height_subdivisions`: number of slices along the height (>= 1)
    ///
    /// Cone tip is at (0, +1, 0), base center at (0, 0, 0), base radius = 1.0.
    pub fn new_cone(radial_subdivisions: u32, height_subdivisions: u32) -> Self {
        let radial = radial_subdivisions.max(3);
        let vertical = height_subdivisions.max(1);

        let mut builder = TriangleMeshBuilder::new_empty();

        let tip = Vec3::new(0.0, 1.0, 0.0);
        let base_center = Vec3::new(0.0, 0.0, 0.0);
        let base_radius = 1.0;

        for i in 0..radial {
            let theta0 = (i as f32 / radial as f32) * std::f32::consts::TAU;
            let theta1 = ((i + 1) as f32 / radial as f32) * std::f32::consts::TAU;

            let dir0 = Vec3::new(theta0.cos(), 0.0, theta0.sin());
            let dir1 = Vec3::new(theta1.cos(), 0.0, theta1.sin());

            for v in 0..vertical {
                let t0 = v as f32 / vertical as f32;
                let t1 = (v + 1) as f32 / vertical as f32;

                let p00 = Vec3::lerp(tip, base_center + dir0 * base_radius, t0);
                let p01 = Vec3::lerp(tip, base_center + dir1 * base_radius, t0);
                let p10 = Vec3::lerp(tip, base_center + dir0 * base_radius, t1);
                let p11 = Vec3::lerp(tip, base_center + dir1 * base_radius, t1);

                builder.add_triangle(Triangle3d::new(p00, p10, p11));
                builder.add_triangle(Triangle3d::new(p00, p11, p01));
            }
        }

        for i in 0..radial {
            let theta0 = (i as f32 / radial as f32) * std::f32::consts::TAU;
            let theta1 = ((i + 1) as f32 / radial as f32) * std::f32::consts::TAU;

            let p0 = base_center + Vec3::new(theta0.cos(), 0.0, theta0.sin()) * base_radius;
            let p1 = base_center + Vec3::new(theta1.cos(), 0.0, theta1.sin()) * base_radius;

            builder.add_triangle(Triangle3d::new(base_center, p1, p0));
        }

        builder
    }

    /// Scale every vertex component-wise.
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        for tri in &mut self.triangles {
            for v in &mut tri.vertices {
                *v *= scale;
            }
        }
        self
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

            // `normalize_or_zero`, not `normalize` - slicing readily produces
            // degenerate triangles whose zero-length edges would yield NaN UVs.
            let u_axis = (b - a).normalize_or_zero();
            let v_axis = t
                .normal()
                .map(|n| n.cross(u_axis))
                .unwrap_or(Vec3::ZERO)
                .normalize_or_zero();

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
    fn subdivide_face_sphere(&mut self, a: Vec3, b: Vec3, c: Vec3, depth: u32) {
        if depth == 0 {
            self.add_triangle(Triangle3d::new(a, b, c));
        } else {
            let ab = slerp(a, b, 0.5);
            let bc = slerp(b, c, 0.5);
            let ca = slerp(c, a, 0.5);

            self.subdivide_face_sphere(a, ab, ca, depth - 1);
            self.subdivide_face_sphere(b, bc, ab, depth - 1);
            self.subdivide_face_sphere(c, ca, bc, depth - 1);
            self.subdivide_face_sphere(ab, bc, ca, depth - 1);
        }
    }

    fn subdivide_face_plane(&mut self, a: Vec3, b: Vec3, c: Vec3, depth: u32) {
        if depth == 0 {
            self.add_triangle(Triangle3d::new(a, b, c));
        } else {
            let ab = (a + b) * 0.5;
            let bc = (b + c) * 0.5;
            let ca = (c + a) * 0.5;

            self.subdivide_face_plane(a, ab, ca, depth - 1);
            self.subdivide_face_plane(b, bc, ab, depth - 1);
            self.subdivide_face_plane(c, ca, bc, depth - 1);
            self.subdivide_face_plane(ab, bc, ca, depth - 1);
        }
    }

    /// Subdivide the entire mesh to increase resolution.
    pub fn subdivide(&mut self, depth: u32) -> &mut Self {
        let triangles = self.triangles.clone();
        self.triangles.clear();

        for tri in triangles {
            self.subdivide_face_plane(tri.vertices[0], tri.vertices[1], tri.vertices[2], depth);
        }

        self
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

impl TriangleMeshBuilder {
    /// Fallibly convert a Bevy `Mesh` into a `TriangleMeshBuilder`.
    ///
    /// Returns `None` instead of panicking when the mesh cannot be
    /// interpreted as a triangle list: no `Float32x3` position attribute, a
    /// position attribute in another format, or no index buffer. Indices
    /// that reference a missing vertex, and a trailing partial triangle, are
    /// skipped rather than panicking.
    ///
    /// Use this for meshes of unknown provenance (for example the arbitrary
    /// mesh handed to the explode plugin). The `From<Mesh>` impl is a
    /// convenience wrapper for meshes known to be well-formed (such as the
    /// output of [`TriangleMeshBuilder::build`]).
    pub fn try_from_mesh(mesh: &Mesh) -> Option<Self> {
        let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION)? {
            VertexAttributeValues::Float32x3(vals) => {
                vals.iter().map(|v| Vec3::from(*v)).collect::<Vec<_>>()
            }
            _ => return None,
        };

        let indices = match mesh.indices()? {
            Indices::U32(indices) => indices.to_vec(),
            Indices::U16(indices) => indices.iter().map(|&i| i as u32).collect::<Vec<_>>(),
        };

        let triangles = indices
            .as_chunks::<3>()
            .0
            .iter()
            .filter_map(|c| {
                Some(Triangle3d::new(
                    *positions.get(c[0] as usize)?,
                    *positions.get(c[1] as usize)?,
                    *positions.get(c[2] as usize)?,
                ))
            })
            .collect::<Vec<_>>();

        Some(Self { triangles })
    }
}

impl From<Mesh> for TriangleMeshBuilder {
    /// Convert a well-formed Bevy `Mesh` into a `TriangleMeshBuilder`.
    ///
    /// Panics if the mesh is not a `Float32x3`-positioned indexed triangle
    /// list. For untrusted meshes use [`TriangleMeshBuilder::try_from_mesh`],
    /// which returns `None` instead.
    fn from(mesh: Mesh) -> Self {
        Self::try_from_mesh(&mesh).expect(
            "TriangleMeshBuilder::from: mesh must have Float32x3 positions and indices; \
             use try_from_mesh for untrusted meshes",
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// A fully collapsed triangle (all vertices equal) has zero-length edges,
    /// so a plain `normalize` in the UV basis would return NaN.
    #[test]
    fn test_uvs_degenerate_triangle_are_finite() {
        let p = Vec3::new(1.0, 2.0, 3.0);
        let builder = TriangleMeshBuilder {
            triangles: vec![Triangle3d::new(p, p, p)],
        };

        for uv in builder.uvs() {
            assert!(uv.is_finite(), "degenerate UV must be finite, got {uv:?}");
        }
    }

    /// A position-only mesh with no index buffer must decline, not panic.
    #[test]
    fn test_try_from_mesh_without_indices_returns_none() {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        );

        assert!(TriangleMeshBuilder::try_from_mesh(&mesh).is_none());
    }
}
