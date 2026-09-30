//! CELL-GRID FIT: whether a section can stand on a one-cell-per-part grid.
//!
//! A prototype earns a grid place by its sockets alone, so the rule reads
//! content that already exists and authors nothing. Every check here holds or
//! fails the same way at all 24 cube rotations, because a quarter turn only
//! permutes the axes it reads. One answer per prototype is therefore enough.
//!
//! ONE copy, two callers: the WFC tile set builds tiles only for a part that
//! fits, and the open-world part snapshot excludes a part that does not, so an
//! off-grid part can never set a family's weakest tier.
//!
//! [`oriented_part`] reads a part that fits at one of the 24 cube rotations as
//! the cells it fills, the sockets each cell presents and the faces it fires
//! through. The WFC tile set and the open-world ship generator both place
//! parts from that one reading.

use std::fmt;

use bevy::prelude::*;

use crate::sections::{
    base_section::prelude::{SectionConfig, SectionFootprint},
    clearance::prelude::exit_normal,
    shell_skin::PLACEMENT_SNAP,
};

/// The prelude: the fit, the oriented reading, the 24 cube rotations, the
/// face order both use and `GRID_EPSILON`.
pub mod prelude {
    pub use super::{
        cell_face, cell_grid_fit, cube_rotations, mirror_face, mirror_rotation, oriented_part,
        CellGridFault, OrientedCell, OrientedPart, CELL_FACES, GRID_EPSILON,
    };
}

/// Slack for the socket geometry the grid reads: positions and normals come
/// out of a quaternion multiply, so nothing lands exactly on 0.5.
pub const GRID_EPSILON: f32 = 1e-4;

/// Why a section cannot stand on the cell grid, in the order the checks run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellGridFault {
    /// The part has no socket, so it can never mate into one connected ship.
    NoSockets,
    /// A socket points between two cardinal axes.
    ObliqueSocket,
    /// A one-cell part carries a socket off the axis it points down.
    OffAxisSocket,
    /// A one-cell part's sockets need two different within-cell offsets.
    MixedInsets,
    /// A one-cell part's body leaves its cell at the offset its sockets need.
    BodyLeavesCell,
    /// A multi-cell part carries a socket off every face centre of its cells.
    SocketOffCellFace,
    /// The part fires between two cardinal axes, so the grid has no lane in
    /// front of it to keep clear. The content lint rejects it for a
    /// generated-ship catalog, and the WFC plan build refuses a plan that
    /// draws it.
    ObliqueExit,
    /// A socket sits on the face the part fires through, so a hull slab could
    /// be bolted across its muzzle. The content lint rejects it for a
    /// generated-ship catalog, and the WFC plan build refuses a plan that
    /// draws it.
    SocketOnExitFace,
}

impl fmt::Display for CellGridFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoSockets => "carries no socket",
            Self::ObliqueSocket => "carries a socket that is not cardinal",
            Self::OffAxisSocket => "carries a socket off the axis it points down",
            Self::MixedInsets => "carries sockets at two different cell insets",
            Self::BodyLeavesCell => "has a body that leaves its cell",
            Self::SocketOffCellFace => "carries a socket off every cell face centre",
            Self::ObliqueExit => "fires down a direction that is not cardinal",
            Self::SocketOnExitFace => {
                "carries a socket on the face it fires through, so a hull slab could be bolted \
                 across its muzzle"
            }
        })
    }
}

/// Whether `config` can stand on the cell grid.
///
/// A one-cell part (its [`SectionFootprint`] is one cell) must point every
/// socket down a cardinal axis, sit each ON that axis, and be inset from its
/// cell face by the SAME distance, because that common inset is the one
/// placement offset that puts all of them on their cell faces at once. Its
/// body must stay inside the cell at that offset: two parts in different cells
/// then cannot interpenetrate.
///
/// A multi-cell part must carry every socket exactly on the face centre of one
/// of its own cells, so each cell presents the face-centre socket a unit part
/// would.
///
/// Either must fire down one of its own cardinal faces, and neither may carry
/// a socket on the face it fires through.
pub fn cell_grid_fit(config: &SectionConfig) -> Result<(), CellGridFault> {
    let collider = config.base.collider.unwrap_or_default();
    let footprint = SectionFootprint::from_collider(collider).0;
    if config.base.link_points.is_empty() {
        return Err(CellGridFault::NoSockets);
    }
    let mut faces = [false; 6];

    if footprint == UVec3::ONE {
        let mut offset: Option<Vec3> = None;
        for point in &config.base.link_points {
            let face = cell_face(point.normal).ok_or(CellGridFault::ObliqueSocket)?;
            let axis = CELL_FACES[face];
            let inset = point.position.dot(axis);
            if !point.position.abs_diff_eq(axis * inset, GRID_EPSILON) {
                return Err(CellGridFault::OffAxisSocket);
            }
            let candidate = axis * (0.5 - inset);
            match offset {
                Some(previous) if !previous.abs_diff_eq(candidate, GRID_EPSILON) => {
                    return Err(CellGridFault::MixedInsets);
                }
                _ => offset = Some(candidate),
            }
            faces[face] = true;
        }
        let offset = offset.unwrap_or_default();
        if (offset.abs() + collider.aabb_half_extents()).max_element() > 0.5 + GRID_EPSILON {
            return Err(CellGridFault::BodyLeavesCell);
        }
        let exit = exit_normal(&config.kind)
            .map(|exit| cell_face(exit).ok_or(CellGridFault::ObliqueExit))
            .transpose()?;
        if exit.is_some_and(|face| faces[face]) {
            return Err(CellGridFault::SocketOnExitFace);
        }
        return Ok(());
    }

    let extent = footprint.as_vec3();
    let cells = || {
        (0..footprint.x).flat_map(move |x| {
            (0..footprint.y).flat_map(move |y| (0..footprint.z).map(move |z| UVec3::new(x, y, z)))
        })
    };
    // Where one of the part's own cells sits in the part's local space.
    let centre_of = |cell: UVec3| cell.as_vec3() - (extent - Vec3::ONE) * 0.5;
    let mut socketed = Vec::new();
    for point in &config.base.link_points {
        let face = cell_face(point.normal).ok_or(CellGridFault::ObliqueSocket)?;
        let cell = cells()
            .find(|cell| {
                (point.position - centre_of(*cell))
                    .abs_diff_eq(CELL_FACES[face] * 0.5, GRID_EPSILON)
            })
            .ok_or(CellGridFault::SocketOffCellFace)?;
        socketed.push((cell, face));
    }
    if let Some(exit) = exit_normal(&config.kind) {
        let local = cell_face(exit).ok_or(CellGridFault::ObliqueExit)?;
        // The exit is a whole FACE of the block: a big drive's nozzle is as
        // wide as the drive, so no cell on that face may carry a socket there.
        let axis = local / 2;
        let outer = if local % 2 == 0 {
            extent[axis] - 1.0
        } else {
            0.0
        };
        if socketed
            .iter()
            .any(|(cell, face)| *face == local && cell.as_vec3()[axis] == outer)
        {
            return Err(CellGridFault::SocketOnExitFace);
        }
    }
    Ok(())
}

/// The six cardinal directions, with `face ^ 1` the opposite face. Every
/// per-face array this module returns is in this order.
pub const CELL_FACES: [Vec3; 6] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
];

/// The index in [`CELL_FACES`] of the cardinal direction `direction` points
/// along, or `None` for a direction between two axes.
pub fn cell_face(direction: Vec3) -> Option<usize> {
    CELL_FACES
        .iter()
        .position(|face| face.dot(direction) > 0.999)
}

/// The face index a reflection through `x = 0` sends `face` to.
pub fn mirror_face(face: usize) -> usize {
    if face / 2 == 0 {
        face ^ 1
    } else {
        face
    }
}

/// The rotation a part reflected through `x = 0` wears.
///
/// A reflection flips handedness, so it is not a rotation - but conjugating a
/// rotation by one is (the axis reflects, the angle negates), which for a
/// quaternion is just negating y and z.
pub fn mirror_rotation(rotation: Quat) -> Quat {
    Quat::from_xyzw(rotation.x, -rotation.y, -rotation.z, rotation.w)
}

/// The 24 ways a cube can sit, identity first.
///
/// Swept as three quarter-turn axes (64 combinations) in x, y, z order and
/// deduplicated by where each sends the three axes, so a caller that walks
/// them in order always meets the same rotation first.
pub fn cube_rotations() -> Vec<Quat> {
    let quarter = std::f32::consts::FRAC_PI_2;
    let mut seen = Vec::new();
    let mut rotations = Vec::new();
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                let rotation = (Quat::from_rotation_x(x as f32 * quarter)
                    * Quat::from_rotation_y(y as f32 * quarter)
                    * Quat::from_rotation_z(z as f32 * quarter))
                .normalize();
                let basis = [Vec3::X, Vec3::Y, Vec3::Z].map(|axis| cell_face(rotation * axis));
                if !seen.contains(&basis) {
                    seen.push(basis);
                    rotations.push(rotation);
                }
            }
        }
    }
    rotations
}

/// One cell of an [`OrientedPart`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrientedCell {
    /// The cell in the part's own, unturned footprint.
    pub local: UVec3,
    /// The cell in the turned block, from its minimum corner along the grid
    /// axes.
    pub cell: UVec3,
    /// Per face, in [`CELL_FACES`] order, whether the cell presents a socket.
    pub faces: [bool; 6],
    /// The face the cell fires, launches or exhausts through, or `None` for a
    /// cell that does none of those. A multi-cell part fires through a whole
    /// face of its block, so every cell on that face carries it.
    pub exit: Option<usize>,
}

/// A part that [`cell_grid_fit`] accepts, turned by one cube rotation and read
/// as the block of grid cells it fills.
#[derive(Clone, Debug, PartialEq)]
pub struct OrientedPart {
    /// The rotation the part was read at.
    pub rotation: Quat,
    /// How many cells the block spans along each grid axis.
    pub span: UVec3,
    /// Where the section stands, from the centre of the block's minimum cell,
    /// in cells. The block's centre for a part that fills its cells; for a
    /// one-cell part, the inset that puts every socket on its cell face, such
    /// as the quarter cell a half-size mount stands off its cell centre.
    pub origin: Vec3,
    /// The face the part fires through, on every one of its cells, or `None`.
    pub aims: Option<usize>,
    /// Every cell of the block, in the part's own `x`, `y`, `z` cell order.
    pub cells: Vec<OrientedCell>,
}

/// Read `config` at `rotation`, one of [`cube_rotations`], as grid cells.
///
/// Errs with the [`cell_grid_fit`] fault for a part that cannot stand on the
/// grid at any rotation.
///
/// # Panics
///
/// If `rotation` is not a cube rotation.
pub fn oriented_part(
    config: &SectionConfig,
    rotation: Quat,
) -> Result<OrientedPart, CellGridFault> {
    cell_grid_fit(config)?;
    let turned = |direction: Vec3| {
        cell_face(rotation * direction)
            .expect("a cube rotation keeps a cardinal direction cardinal")
    };
    let footprint = SectionFootprint::from_collider(config.base.collider.unwrap_or_default()).0;
    let extent = footprint.as_vec3();
    let span = (rotation * extent).abs().round().as_uvec3();
    let snapped = |value: Vec3| (value * PLACEMENT_SNAP).round() / PLACEMENT_SNAP;

    if footprint == UVec3::ONE {
        let mut faces = [false; 6];
        let mut origin = Vec3::ZERO;
        for point in &config.base.link_points {
            let face = turned(point.normal);
            let axis = CELL_FACES[face];
            origin = axis * (0.5 - (rotation * point.position).dot(axis));
            faces[face] = true;
        }
        let exit = exit_normal(&config.kind).map(turned);
        return Ok(OrientedPart {
            rotation,
            span,
            origin: snapped(origin),
            aims: exit,
            cells: vec![OrientedCell {
                local: UVec3::ZERO,
                cell: UVec3::ZERO,
                faces,
                exit,
            }],
        });
    }

    // Where one of the part's own cells sits, from the part's centre.
    let centre_of = |local: UVec3| local.as_vec3() - (extent - Vec3::ONE) * 0.5;
    let half_span = (span.as_vec3() - Vec3::ONE) * 0.5;
    let mut cells: Vec<OrientedCell> = (0..footprint.x)
        .flat_map(|x| {
            (0..footprint.y).flat_map(move |y| (0..footprint.z).map(move |z| UVec3::new(x, y, z)))
        })
        .map(|local| OrientedCell {
            local,
            cell: snapped(rotation * centre_of(local) + half_span)
                .round()
                .as_uvec3(),
            faces: [false; 6],
            exit: None,
        })
        .collect();

    for point in &config.base.link_points {
        let face =
            CELL_FACES[cell_face(point.normal).expect("the fit found every socket cardinal")];
        let owner = cells
            .iter_mut()
            .find(|cell| {
                (point.position - centre_of(cell.local)).abs_diff_eq(face * 0.5, GRID_EPSILON)
            })
            .expect("the fit put every socket on a face centre of the part's own cells");
        owner.faces[turned(point.normal)] = true;
    }

    let aims = exit_normal(&config.kind).map(|exit| {
        let local = cell_face(exit).expect("the fit found a multi-cell exit cardinal");
        let axis = local / 2;
        let outer = if local % 2 == 0 {
            footprint[axis] - 1
        } else {
            0
        };
        let out = turned(exit);
        for cell in cells.iter_mut().filter(|cell| cell.local[axis] == outer) {
            cell.exit = Some(out);
        }
        out
    });

    Ok(OrientedPart {
        rotation,
        span,
        origin: snapped(half_span),
        aims,
        cells,
    })
}
