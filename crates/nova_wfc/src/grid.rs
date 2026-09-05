//! The block of cells a hull is collapsed in.

use bevy::prelude::*;

/// The six cardinal faces of a grid cell, in the order the tile face arrays
/// use. The layout is what makes `face ^ 1` the OPPOSITE face, which is the
/// only index arithmetic the adjacency rule needs.
pub(crate) const FACES: [Vec3; 6] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
];

/// Slack for the socket geometry the grid reads: positions and normals come
/// out of a quaternion multiply, so nothing lands exactly on 0.5.
pub const GRID_EPSILON: f32 = 1e-4;

/// Reciprocal of the quantum a derived placement is snapped onto - a power of
/// two, so the snap itself is exact.
///
/// Half a cell comes back out of a quaternion multiply as 0.49999997, and the
/// section then sits 6e-8 off its cell centre. That is invisible on screen and
/// fatal to the overlap lint, which compares centre distances against a sum of
/// half-extents with a strict `<`: two DIAGONAL neighbours a hair under one
/// cell apart read as interpenetrating. Snapping puts the exact value back.
const PLACEMENT_SNAP: f32 = 4096.0;

/// A rectangular block of cells with a place in ship space.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Grid {
    pub(crate) size: UVec3,
    /// Ship-space centre of cell `(0, 0, 0)`.
    pub(crate) origin: Vec3,
}

impl Grid {
    /// The half grid a mirrored hull's structural collapse runs on. It starts
    /// half a cell off the centreline, so `x = 0` is the mirror plane and the
    /// two halves meet flush.
    pub(crate) fn starboard_half(half_width: u32, height: u32, length: u32) -> Self {
        Self {
            size: UVec3::new(half_width, height, length),
            origin: Vec3::new(
                0.5,
                -(height as f32 - 1.0) * 0.5,
                -(length as f32 - 1.0) * 0.5,
            ),
        }
    }

    pub(crate) fn cells(&self) -> usize {
        (self.size.x * self.size.y * self.size.z) as usize
    }

    pub(crate) fn index(&self, x: usize, y: usize, z: usize) -> usize {
        (x * self.size.y as usize + y) * self.size.z as usize + z
    }

    pub(crate) fn coords(&self, cell: usize) -> (usize, usize, usize) {
        let (height, length) = (self.size.y as usize, self.size.z as usize);
        (
            cell / (height * length),
            (cell / length) % height,
            cell % length,
        )
    }

    pub(crate) fn centre(&self, cell: usize) -> Vec3 {
        let (x, y, z) = self.coords(cell);
        self.origin + Vec3::new(x as f32, y as f32, z as f32)
    }

    /// The neighbour across one face, or `None` at the grid edge.
    pub(crate) fn neighbour(&self, cell: usize, face: usize) -> Option<usize> {
        let (x, y, z) = self.coords(cell);
        let step = |value: usize, delta: i32, limit: u32| {
            let next = value as i32 + delta;
            (next >= 0 && (next as u32) < limit).then_some(next as usize)
        };
        match face {
            0 => step(x, 1, self.size.x).map(|x| self.index(x, y, z)),
            1 => step(x, -1, self.size.x).map(|x| self.index(x, y, z)),
            2 => step(y, 1, self.size.y).map(|y| self.index(x, y, z)),
            3 => step(y, -1, self.size.y).map(|y| self.index(x, y, z)),
            4 => step(z, 1, self.size.z).map(|z| self.index(x, y, z)),
            _ => step(z, -1, self.size.z).map(|z| self.index(x, y, z)),
        }
    }
}

/// Put a derived placement back on the grid's exact arithmetic, see
/// [`PLACEMENT_SNAP`].
pub(crate) fn snapped(value: Vec3) -> Vec3 {
    (value * PLACEMENT_SNAP).round() / PLACEMENT_SNAP
}

/// The index in [`FACES`] of the cardinal direction `direction` points along,
/// or `None` if it points somewhere between two of them.
pub(crate) fn face_index(direction: Vec3) -> Option<usize> {
    FACES.iter().position(|face| face.dot(direction) > 0.999)
}

/// The face index a reflection through `x = 0` sends `face` to.
pub(crate) fn mirror_face(face: usize) -> usize {
    match face / 2 {
        0 => face ^ 1,
        _ => face,
    }
}

/// The rotation a part reflected through `x = 0` wears.
///
/// A reflection flips handedness, so it is not a rotation - but conjugating a
/// rotation by one is (the axis reflects, the angle negates), which for a
/// quaternion is just negating y and z.
pub(crate) fn mirrored(rotation: Quat) -> Quat {
    Quat::from_xyzw(rotation.x, -rotation.y, -rotation.z, rotation.w)
}
