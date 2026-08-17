//! Turning an AUTHORED mesh into a solid a carve can cut.
//!
//! [`SignedField`] describes a solid analytically for asteroids - the same noise
//! the rock's own mesh comes from - and there is nothing analytic behind a ship
//! section. A section is a glTF model an artist made, so the only way to get a
//! field out of one is to ask the geometry itself: for every grid corner, how
//! far is it from the surface, and is it inside or outside?
//!
//! parry answers both at once. A `TriMesh` carries a QBVH, so a point query is
//! a tree descent rather than a scan of every triangle, and with
//! `TriMeshFlags::ORIENTED` its projection reports which side it landed on.
//!
//! # Inside is only a question a CLOSED surface can answer
//!
//! And that is the whole difficulty. "Inside" means the surface separates space
//! into two parts, which needs a surface with no boundary: every edge shared by
//! exactly two triangles. Authored art is under no obligation to be that. A face
//! left open because nothing can see it, a panel modelled as a single sheet, a
//! shape assembled from overlapping boxes - all of them look right and none of
//! them has an inside.
//!
//! parry will not tell you. It computes pseudo-normals from whatever it is given
//! and reports an `is_inside` that is simply wrong in the neighbourhood of a
//! hole. So [`is_closed`] checks first, and [`field_from_mesh`] refuses rather
//! than producing a solid whose sign flips in places nobody looked at.
//!
//! MEASURED against the shipped section art (2026-08-18). Of the five glTF
//! models, three have no boundary edge at all - `hull-01`, `torpedo-bay-01`,
//! `turret-pitch-01` - and two do: `turret-barrel-01` has 48 and `turret-yaw-01`
//! has 2. Only `turret-pitch-01` is a clean 2-manifold; the other two carry
//! non-manifold edges where panels meet. The procedural cut-cube parts most
//! sections actually draw with are all closed. Building a 32^3 field off one
//! costs 5-20 ms native, which is the same order as an asteroid's own field.

use avian3d::parry::{
    query::PointQueryWithLocation,
    shape::{TriMesh, TriMeshFlags},
};
use bevy::{
    math::Vec3,
    mesh::{Indices, Mesh, VertexAttributeValues},
    prelude::*,
};

use super::field::SignedField;

/// How far past a mesh's own bounds the field's domain reaches, as a fraction.
///
/// The surface has to sit strictly inside the grid: a sign change ON the
/// boundary has no cell outside it to pair with, so the mesher would leave that
/// face open.
const MESH_FIELD_MARGIN: f32 = 1.1;

/// How close two positions have to be to count as the same vertex, in the
/// mesh's own units.
///
/// glTF splits vertices per normal and per UV, so a cube arrives as 24 vertices
/// and every one of its edges looks like a boundary until they are welded. This
/// is a WELD tolerance and not a repair: it joins what the exporter split, and
/// it will not close a hole the artist left.
const WELD_EPSILON: f32 = 1e-4;

/// Whether `mesh` is a closed surface: every edge shared by exactly two
/// triangles.
///
/// The precondition for asking anything about inside and outside. Vertices are
/// welded by position first, because a glTF mesh's index buffer describes the
/// RENDER topology - split at every crease and UV seam - rather than the shape's.
///
/// Only boundary edges are counted, not non-manifold ones. A surface with an
/// edge shared by four triangles still separates space; a surface with an edge
/// shared by one does not, and that is the case this is here to catch.
pub fn is_closed(mesh: &Mesh) -> bool {
    let Some((_, indices)) = welded(mesh) else {
        return false;
    };

    let mut edges: std::collections::HashMap<(u32, u32), u32> = std::collections::HashMap::new();
    for face in indices.chunks_exact(3) {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            if a == b {
                continue;
            }
            *edges.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    !edges.is_empty() && edges.values().all(|shared| *shared >= 2)
}

/// Sample `mesh` onto a signed grid of `resolution` cells per axis, or `None`
/// when the mesh is not something a solid can be read out of.
///
/// `None` for three reasons, and all of them mean the same thing - there is no
/// honest answer, so no answer is given:
///
/// - the mesh has no triangles to query;
/// - it is not CLOSED, so inside is undefined (see [`is_closed`]);
/// - parry declines to build a `TriMesh` from it.
///
/// The grid is centred on the MESH's own bounds rather than on the origin,
/// because authored art is modelled wherever the artist put it and a grid
/// centred on the origin would spend most of its cells on empty space. The
/// caller gets the offset back so it can place what it meshes.
pub fn field_from_mesh(mesh: &Mesh, resolution: usize) -> Option<(SignedField, Vec3)> {
    let (positions, indices) = welded(mesh)?;
    if indices.len() < 3 {
        return None;
    }
    if !is_closed(mesh) {
        debug!("field_from_mesh: the mesh is not closed, so it has no inside");
        return None;
    }

    let faces: Vec<[u32; 3]> = indices
        .chunks_exact(3)
        .map(|face| [face[0], face[1], face[2]])
        .collect();
    let trimesh = TriMesh::with_flags(
        positions.clone(),
        faces,
        // ORIENTED is what makes a projection report which SIDE it landed on,
        // which is the only reason parry is here rather than a plain distance.
        TriMeshFlags::ORIENTED | TriMeshFlags::MERGE_DUPLICATE_VERTICES,
    )
    .ok()?;

    let (centre, half) = bounds(&positions)?;
    let half_extent = half.max_element() * MESH_FIELD_MARGIN;

    let field = SignedField::sample(resolution, half_extent, |at| {
        // The grid is centred on the mesh's own bounds, so a sample has to be
        // put back into the mesh's frame before it is asked about.
        let query = at + centre;
        // The LOCATION-aware projection, not the plain one: parry only applies
        // the pseudo-normals - the whole reason `ORIENTED` is set - on this
        // path, so `project_local_point` comes back with `is_inside` false
        // everywhere and a field with no interior at all.
        let projection = trimesh.project_local_point_and_get_location(query, false).0;
        let distance = projection.point.distance(query);
        match projection.is_inside {
            true => -distance,
            false => distance,
        }
    });
    Some((field, centre))
}

/// A mesh's triangle list with vertices welded by position, or `None` when it
/// carries no usable geometry.
fn welded(mesh: &Mesh) -> Option<(Vec<Vec3>, Vec<u32>)> {
    let VertexAttributeValues::Float32x3(raw) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)? else {
        return None;
    };

    // Quantised to the weld tolerance and keyed on the bits, so two vertices an
    // exporter split apart come back together without a distance search.
    let mut keys: std::collections::HashMap<[i64; 3], u32> = std::collections::HashMap::new();
    let mut positions: Vec<Vec3> = Vec::new();
    let mut remap: Vec<u32> = Vec::with_capacity(raw.len());
    for point in raw {
        let at = Vec3::from_array(*point);
        if !at.is_finite() {
            return None;
        }
        let key = [
            (at.x / WELD_EPSILON).round() as i64,
            (at.y / WELD_EPSILON).round() as i64,
            (at.z / WELD_EPSILON).round() as i64,
        ];
        let next = keys.len() as u32;
        let index = *keys.entry(key).or_insert(next);
        if index == next {
            positions.push(at);
        }
        remap.push(index);
    }

    let indices: Vec<u32> = match mesh.indices() {
        Some(Indices::U32(indices)) => indices.iter().map(|i| remap[*i as usize]).collect(),
        Some(Indices::U16(indices)) => indices.iter().map(|i| remap[*i as usize]).collect(),
        // Unindexed: the positions ARE the triangle list.
        None => remap.clone(),
    };
    // Degenerate faces the weld collapsed carry no surface and would confuse
    // the edge count.
    let indices: Vec<u32> = indices
        .chunks_exact(3)
        .filter(|face| face[0] != face[1] && face[1] != face[2] && face[0] != face[2])
        .flatten()
        .copied()
        .collect();
    Some((positions, indices))
}

/// The centre and half extents of a set of points.
fn bounds(positions: &[Vec3]) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for at in positions {
        min = min.min(*at);
        max = max.max(*at);
    }
    min.cmple(max)
        .all()
        .then(|| ((min + max) * 0.5, (max - min) * 0.5))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::builder::TriangleMeshBuilder;

    /// An open sheet: two triangles making a square, every edge a boundary.
    fn open_sheet() -> Mesh {
        Mesh::new(
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
        .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
    }

    /// THE precondition. A closed surface has an inside; an open one does not,
    /// and no amount of querying it will produce one.
    #[test]
    fn a_closed_mesh_is_told_from_an_open_one() {
        assert!(
            is_closed(&TriangleMeshBuilder::new_octahedron(2).build()),
            "a subdivided octahedron is a closed surface"
        );
        assert!(
            is_closed(&Mesh::from(Cuboid::new(1.0, 1.0, 1.0))),
            "and so is a cube, once its render-split vertices are welded"
        );
        assert!(
            !is_closed(&open_sheet()),
            "a sheet is not - every one of its edges is a boundary"
        );
    }

    /// The refusal is the feature: a field over an open mesh would have a sign
    /// that flips wherever the hole is, and nothing downstream could tell.
    #[test]
    fn an_open_mesh_yields_no_field_at_all() {
        assert!(field_from_mesh(&open_sheet(), 8).is_none());
    }

    /// And a closed one yields the solid it describes: inside is inside,
    /// outside is outside, and the surface comes back where it went in.
    #[test]
    fn a_closed_mesh_yields_the_solid_it_describes() {
        let mesh = Mesh::from(Cuboid::new(2.0, 2.0, 2.0));
        let (field, centre) = field_from_mesh(&mesh, 24).expect("a cube is a solid");
        assert_eq!(centre, Vec3::ZERO, "a cube models about its own middle");

        let remeshed = field.surface().build();
        let VertexAttributeValues::Float32x3(positions) = remeshed
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("positions")
        else {
            panic!("a built mesh has positions");
        };
        assert!(!positions.is_empty(), "the cube has a surface");
        for point in positions {
            let at = Vec3::from_array(*point).abs();
            assert!(
                (at.max_element() - 1.0).abs() < field.cell_size(),
                "a vertex sat at {at}, off the cube's own face"
            );
        }
    }

    /// The point of doing this at all: a solid read off authored art carves the
    /// same way an analytic one does.
    #[test]
    fn a_solid_read_off_a_mesh_can_be_carved() {
        let mesh = TriangleMeshBuilder::new_octahedron(2).build();
        let (mut field, _) = field_from_mesh(&mesh, 24).expect("a ball is a solid");
        let before = field.solid_volume();

        field.subtract_sphere(Vec3::X, 0.6);

        assert!(
            field.solid_volume() < before,
            "carving an authored solid removes material like any other"
        );
    }
}
