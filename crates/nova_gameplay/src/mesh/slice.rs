//! Triangle-vs-plane geometry, the kernel behind
//! [`TriangleMeshBuilder::slice`](super::builder::TriangleMeshBuilder::slice).
//!
//! Pure math over [`Triangle3d`] and a plane: no builder, no `Mesh`, no assets.
//! Every entry point is total - a degenerate or parallel input yields a finite
//! result rather than a panic or a NaN, because slicing runs on arbitrary
//! game meshes (see [`super::explode`]).

use bevy::prelude::*;

/// Compute intersection between an edge and a plane.
///
/// The result is always finite. If the edge is (nearly) parallel to the
/// plane the crossing is undefined and a division would yield inf/NaN, so we
/// fall back to the edge midpoint. The parameter is also clamped to the
/// segment so numerical overshoot cannot push the vertex off the edge.
fn edge_plane_intersection(a: Vec3, b: Vec3, plane_point: Vec3, plane_normal: Vec3) -> Vec3 {
    let ab = b - a;
    let denom = ab.dot(plane_normal);

    if denom.abs() < 1e-6 {
        return a + ab * 0.5;
    }

    let t = ((plane_point - a).dot(plane_normal) / denom).clamp(0.0, 1.0);

    a + ab * t
}

/// Result of slicing a triangle against a plane.
pub(super) enum TriangleSliceResult {
    Single(Triangle3d),
    Split(Triangle3d, Triangle3d, Triangle3d),
}

/// Slice a triangle along a plane.
///
/// Returns a tuple containing the slice result and a boolean indicating
/// whether the lonely vertex is on the positive side of the plane.
pub(super) fn triangle_slice(
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

    /// An edge parallel to the plane makes the denominator zero; the fallback
    /// midpoint must keep the result finite instead of producing inf/NaN.
    #[test]
    fn test_edge_plane_intersection_parallel_is_finite() {
        let a = Vec3::new(0.0, 1.0, 0.0);
        let b = Vec3::new(1.0, 1.0, 0.0);
        let plane_point = Vec3::ZERO;
        let plane_normal = Vec3::new(0.0, 1.0, 0.0); // parallel to edge AB

        let p = edge_plane_intersection(a, b, plane_point, plane_normal);

        assert!(
            p.is_finite(),
            "parallel edge intersection must be finite, got {p:?}"
        );
    }
}
