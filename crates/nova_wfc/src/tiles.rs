//! Reading the section catalog as a set of grid tiles.
//!
//! The ADJACENCY RULE is the catalog's own link points: a prototype earns a
//! place on the grid by its sockets alone, and two tiles may sit face to face
//! exactly when the sockets they turn to each other agree. Nothing in this
//! module authors a rule; it reads one off content that already exists.

use bevy::prelude::*;
use nova_ship::prelude::{
    cell_grid_fit, cube_rotations, mirror_face, oriented_part, CellGridFault, GameSections,
    OrientedPart, SectionCollider, SectionConfig, CELL_FACES, GRID_EPSILON,
};

use crate::{
    grid::{face_index, snapped},
    plan::{WfcPart, WfcPlan, WfcZone},
};

/// Index of the vacuum tile, which [`build`] always puts first.
pub(crate) const VACUUM: usize = 0;

/// One part of the plan's draw, resolved against the catalog.
pub(crate) struct Family {
    /// The catalog section id.
    pub(crate) prototype: String,
    /// Draw weight for the PART, spent across whatever orientations of it are
    /// legal in a given cell.
    pub(crate) weight: f32,
    /// The only face this part may fire through, in [`CELL_FACES`] order, or `None`
    /// for a part that may point any way the mating rule allows.
    pub(crate) aim: Option<usize>,
    /// The only region of the hull this part may stand in, or `None` for a part
    /// free to stand anywhere the mating rule allows.
    pub(crate) zone: Option<WfcZone>,
}

/// What one cell may hold.
pub(crate) struct Tile {
    /// The prototype placed in the cell, or `None` for vacuum.
    pub(crate) part: Option<TileBody>,
    /// Which [`Family`] this tile is an orientation of, or `None` for vacuum.
    /// Draw weights are authored per PART, so the draw needs to know which
    /// tiles are the same part wearing different rotations.
    pub(crate) family: Option<usize>,
    /// Per-face sockets this tile presents to its neighbours, in
    /// [`CELL_FACES`] order, read off the rotated link points.
    pub(crate) faces: [bool; 6],
    /// The face this CELL fires, launches or exhausts through, in
    /// [`CELL_FACES`] order, or `None` for a cell that does none of those. It
    /// is the cell's clearance: nothing may stand in front of it.
    ///
    /// A big drive's nozzle is as wide as the drive, so every cell of its
    /// exhaust face carries this and every cell of the layers in front of it
    /// carries `None`.
    pub(crate) exit: Option<usize>,
    /// The face the PART this tile belongs to fires through, on every one of
    /// its cells, or `None` for a part that fires nowhere.
    ///
    /// Which way a part POINTS is a property of the part, and the plan's
    /// aim rules on it ([`Family::aim`]). Reading `exit` for that was a bug a
    /// one-cell part could not show: on a multi-cell part only the muzzle
    /// layer carries an exit, so an aimed drive had its whole mount layer
    /// struck as "pointing nowhere", and propagation then took the muzzle
    /// layer with it - the part could stand in no cell of any grid.
    pub(crate) aims: Option<usize>,
    /// Per-face, the tile index of the NEXT SEGMENT of the same multi-cell
    /// part, or `None` on a face that is the part's own surface. A joint face
    /// accepts exactly its partner: not vacuum, not another part, not another
    /// copy of the same part ([`compatible`]).
    pub(crate) joints: [Option<usize>; 6],
    /// Whether this tile emits the placed section. Every single-cell tile
    /// does; of a multi-cell part's segments exactly one does, and the rest
    /// only occupy their cells.
    pub(crate) emits: bool,
    /// How many cells the whole PART spans, in GRID axes at this tile's
    /// rotation. `UVec3::ONE` for a part that fills one cell.
    ///
    /// The seeding reads it off the upright tile to know how deep into the
    /// hull the stern drive reaches ([`crate::collapse::Collapse::seed_stern`]).
    pub(crate) span: UVec3,
    /// Whether the body reaches the `-x` face of its cell, so that on the
    /// centreline it meets its own reflection flush ([`seam_allows`]).
    ///
    /// A single-cell tile answers with its within-cell offset: the half-size
    /// mount stands on a boundary rather than in the middle, and a pair of
    /// those on the seam would meet base to base. A segment of a multi-cell
    /// part fills its cell by construction - the footprint is whole cells -
    /// and its own `offset` points at the part's centre, which is not a
    /// within-cell offset at all.
    pub(crate) seam_flush: bool,
}

impl Tile {
    pub(crate) fn is_solid(&self) -> bool {
        self.part.is_some()
    }
}

/// A prototype at one of the rotations the grid can use it at.
#[derive(Clone)]
pub(crate) struct TileBody {
    pub(crate) prototype: String,
    pub(crate) rotation: Quat,
    /// Where inside its cell the section sits, so that every socket it kept
    /// lands exactly on the cell face it points through. Zero for a part that
    /// fills its cell; a quarter cell for the half-size PDC mount, which
    /// stands on the boundary rather than in the middle.
    pub(crate) offset: Vec3,
}

/// Vacuum: the tile a cell holds when it holds nothing.
fn vacuum_tile() -> Tile {
    Tile {
        part: None,
        family: None,
        faces: [false; 6],
        exit: None,
        aims: None,
        joints: [None; 6],
        emits: false,
        span: UVec3::ONE,
        seam_flush: false,
    }
}

/// Build the tile set: vacuum first ([`VACUUM`]), then every distinct
/// ORIENTATION each drawn prototype can stand at, with the families the draw
/// prices them by.
///
/// Every ORIENTATION is one of the 24 [`cube_rotations`], read as grid cells
/// by [`oriented_part`].
///
/// Deduplicating by SOCKET PATTERN instead would be tempting - it is all the
/// collapse can tell apart - and it was wrong. Sockets do not determine a
/// shape. A ramp blind on +Y and +X is the same socket pattern whether it lies
/// on its -Y face sloping toward +X or on its -X face sloping toward +Y, and
/// those are different models; keeping one and throwing away the other let the
/// generator plant ramps on their sides. The camera can tell orientations apart
/// even where the rule cannot, so they are kept apart.
///
/// `Err` carries the line the caller shows: a plan naming a part the
/// catalog does not hold, one whose geometry cannot be mirrored, or one whose
/// exit lane the grid cannot keep clear.
pub(crate) fn build(
    sections: &GameSections,
    plan: &WfcPlan,
) -> Result<(Vec<Tile>, Vec<Family>), String> {
    let mut tiles = vec![vacuum_tile()];
    let mut families = Vec::new();

    for part in &drawn_and_seeded(plan) {
        let config = sections.get_section(&part.prototype).ok_or_else(|| {
            format!(
                "the hull plan draws '{}', which the catalog does not hold",
                part.prototype
            )
        })?;
        if !mirror_symmetric(config) {
            // The structural half is built once and reflected, so every part
            // in it has to survive the reflection.
            return Err(format!(
                "'{}' has an x-asymmetric socket set, so a mirrored copy of it would not \
                 carry the same sockets",
                part.prototype
            ));
        }

        let family = families.len();
        families.push(resolved(part)?);
        // A part off the grid keeps its family and simply has no tiles, so the
        // seed can name it. A part whose lane the grid cannot keep clear is a
        // fault: drawing it silently would drop a weapon or drive the plan names.
        match cell_grid_fit(config) {
            Ok(()) => {}
            Err(fault @ (CellGridFault::ObliqueExit | CellGridFault::SocketOnExitFace)) => {
                return Err(format!("'{}' {fault}", part.prototype));
            }
            Err(_) => continue,
        }

        for rotation in cube_rotations() {
            let oriented = oriented_part(config, rotation)
                .expect("cell_grid_fit above already accepted this prototype");
            let group = if oriented.cells.len() == 1 {
                vec![tile(family, config, &oriented)]
            } else {
                segment_tiles(family, &config.base.id, &oriented)
            };
            let base = tiles.len();
            for mut tile in group {
                for joint in &mut tile.joints {
                    *joint = joint.map(|local| base + local);
                }
                tiles.push(tile);
            }
        }
    }

    Ok((tiles, families))
}

/// Every prototype the plan NAMES, in one list: the parts it draws, then
/// any role it seeds that the draw does not already price.
///
/// A seeded role needs tiles as much as a drawn one does - the collapse assigns
/// it by hand - but it is not a draw, so it joins at weight zero and is never
/// offered. Without this a plan that seeds a part it does not also list
/// fails deep in the seed with "cannot stand on the grid at all", which names
/// the symptom rather than the omission.
fn drawn_and_seeded(plan: &WfcPlan) -> Vec<WfcPart> {
    let mut parts = plan.parts.clone();
    let keel = &plan.keel;
    let seeded = [
        keel.hull.as_str(),
        keel.bridge.as_str(),
        keel.stern_deck.as_str(),
        keel.stern_drive.as_str(),
    ]
    .into_iter()
    .chain(keel.bow_gun.as_deref());
    for prototype in seeded {
        if parts.iter().any(|part| part.prototype == prototype) {
            continue;
        }
        parts.push(WfcPart {
            prototype: prototype.to_string(),
            weight: 0.0,
            aim: None,
            zone: None,
        });
    }
    parts
}

/// One authored draw entry with its aim resolved to a [`CELL_FACES`] index.
fn resolved(part: &WfcPart) -> Result<Family, String> {
    let aim = match part.aim {
        Some(aim) => Some(face_index(aim.normal()).ok_or_else(|| {
            format!(
                "'{}' aims down {:?}, which is not a cardinal face",
                part.prototype, aim
            )
        })?),
        None => None,
    };
    Ok(Family {
        prototype: part.prototype.clone(),
        weight: part.weight,
        aim,
        zone: part.zone,
    })
}

/// Read one prototype that [`cell_grid_fit`] accepts, at one rotation, as a
/// grid tile, from the single cell [`oriented_part`] reads it as.
fn tile(family: usize, config: &SectionConfig, part: &OrientedPart) -> Tile {
    let cell = &part.cells[0];
    Tile {
        part: Some(TileBody {
            prototype: config.base.id.clone(),
            rotation: part.rotation,
            offset: part.origin,
        }),
        family: Some(family),
        faces: cell.faces,
        exit: cell.exit,
        aims: part.aims,
        joints: [None; 6],
        emits: true,
        span: UVec3::ONE,
        seam_flush: part.origin.x.abs() < GRID_EPSILON,
    }
}

/// Read one MULTI-CELL prototype that [`cell_grid_fit`] accepts, at one
/// rotation, as a BLOCK of segment tiles, from the block [`oriented_part`]
/// reads it as. Returned joints are LOCAL segment indices; [`build`] remaps
/// them once the segments' final indices exist.
///
/// The block is held together by [`Tile::joints`], which name the one tile each
/// face may sit against. A cell that draws a corner segment forces its whole
/// block through propagation, or the domain empties and the seed is refused.
///
/// One segment emits the placed section: the one at the part's own local
/// origin, carrying the offset that puts the part's centre back over the
/// block. The rest only occupy their cells.
fn segment_tiles(family: usize, id: &str, part: &OrientedPart) -> Vec<Tile> {
    let mut segments: Vec<Tile> = part
        .cells
        .iter()
        .map(|cell| Tile {
            part: Some(TileBody {
                prototype: id.to_string(),
                rotation: part.rotation,
                offset: snapped(part.origin - cell.cell.as_vec3()),
            }),
            family: Some(family),
            faces: cell.faces,
            exit: cell.exit,
            aims: part.aims,
            joints: [None; 6],
            emits: cell.local == UVec3::ZERO,
            span: part.span,
            seam_flush: true,
        })
        .collect();

    for (index, cell) in part.cells.iter().enumerate() {
        for (face, offset) in CELL_FACES.into_iter().enumerate() {
            let neighbor = cell.cell.as_vec3() + offset;
            if neighbor.cmplt(Vec3::ZERO).any() || neighbor.cmpge(part.span.as_vec3()).any() {
                continue;
            }
            let neighbor = neighbor.as_uvec3();
            segments[index].joints[face] =
                part.cells.iter().position(|other| other.cell == neighbor);
        }
    }
    segments
}

/// Whether a prototype's socket set survives the centreline mirror.
///
/// The structural half is built once and reflected through `x = 0`, and a
/// reflection is not a rotation - it can only be re-expressed as one for a part
/// whose sockets come in mirrored pairs. Every shipped structural prototype
/// does; the refusal is what stops a mod part that does not from being silently
/// mirrored into a ship whose halves no longer mate.
///
/// POSITION counts, not just which way a socket points. On a one-cell part the
/// two say the same thing, because a socket sits on the axis it points down.
/// A part several cells ACROSS can carry a socket over one of its own cells and
/// not over that cell's reflection, and only the starboard half is ever solved
/// - so the port copy would present sockets in cells nothing checked.
fn mirror_symmetric(config: &SectionConfig) -> bool {
    let reflected = |vector: Vec3| Vec3::new(-vector.x, vector.y, vector.z);
    config.base.link_points.iter().all(|point| {
        config.base.link_points.iter().any(|other| {
            other
                .normal
                .abs_diff_eq(reflected(point.normal), GRID_EPSILON)
                && other
                    .position
                    .abs_diff_eq(reflected(point.position), GRID_EPSILON)
        })
    })
}

/// Half-extents of a collider's AABB after `rotation`, which for the cardinal
/// rotations the grid uses is just a permutation of the authored ones.
pub fn rotated_half_extents(collider: SectionCollider, rotation: Quat) -> Vec3 {
    collider.rotated_aabb_half_extents(rotation)
}

/// THE RULE. A socket may never press into a face that has none, and nothing
/// may stand in front of an exit.
///
/// Two sockets facing each other is precisely what `derive_link_point_graph`
/// will later call a mate, and one socket against a face that cannot answer it
/// is the thing that must never be built. A socket facing VACUUM is fine, for
/// skin as much as for structure: most of a hull is exposed, and a tile at the
/// rim of the skin has sockets nothing answers.
///
/// Where NEITHER face has a socket the two may touch. A face without a socket
/// is not a keep-out zone, it is a face with nothing to mate: a drive's flank
/// is the side of a cylinder and a mount's is its housing, and two of those
/// resting against each other is an engine cluster rather than a fault.
///
/// CLEARANCE is the second clause, and it is why the first is not enough. A
/// muzzle, a nozzle and a bay's mouth are all faces with nothing to mate, so
/// the mating rule alone let a drive exhaust into the flank of the drive beside
/// it and a bay fire into a mount's housing. What may not be there is read off
/// the part's KIND (`exit_normal`) rather than guessed from the absent socket,
/// because a drive's flank has no socket either and nothing comes out of it.
///
/// This is only the LOCAL half of clearance - the one cell a binary rule can
/// see. The rest of the lane, and the cladding the lane must not be given, is
/// [`crate::collapse::Collapse::erode_blocked_exits`].
pub(crate) fn compatible(tiles: &[Tile], here: usize, face: usize, there: usize) -> bool {
    // A joint face is the INSIDE of a multi-cell part, and it accepts exactly
    // the part's own next segment - before the solid early-out, because vacuum
    // across a joint would be half a part.
    if let Some(partner) = tiles[here].joints[face] {
        return there == partner;
    }
    if let Some(partner) = tiles[there].joints[face ^ 1] {
        return here == partner;
    }
    let (here, there) = (&tiles[here], &tiles[there]);
    if !here.is_solid() || !there.is_solid() {
        return true;
    }
    if here.exit == Some(face) || there.exit == Some(face ^ 1) {
        return false;
    }
    here.faces[face] == there.faces[face ^ 1]
}

/// The centreline face, which has no neighbour cell because its neighbour is
/// this cell's own reflection. What the reflection turns toward the seam is
/// the mirror of this tile's own -x face, which carries a socket exactly when
/// this tile's -x face does - so [`compatible`] against the mirror collapses
/// to a rule about one tile: a solid on the centreline must carry a -x socket.
///
/// It must also FILL its cell. A half-size mount on the centreline would meet
/// its own reflection base to base - legal by the rule and absurd on a ship.
pub(crate) fn seam_allows(tile: &Tile) -> bool {
    if tile.part.is_none() {
        return true;
    }
    tile.faces[1] && tile.seam_flush
}

/// The tile that places `prototype` unrotated - the orientation that faces the
/// way the ship does, which for the drive is the nozzle pointing aft.
pub(crate) fn upright_tile(tiles: &[Tile], prototype: &str) -> Option<usize> {
    tiles.iter().position(|tile| {
        tile.emits
            && tile.part.as_ref().is_some_and(|part| {
                part.prototype == prototype
                    && part.rotation.abs_diff_eq(Quat::IDENTITY, GRID_EPSILON)
            })
    })
}

/// One placed part of the WHOLE ship - both halves, in cells - as the skin
/// reads it.
pub(crate) struct ShipCell<'a> {
    pub(crate) cell: IVec3,
    pub(crate) prototype: &'a str,
    pub(crate) faces: [bool; 6],
    pub(crate) exit: Option<usize>,
}

/// The mirror of one placed cell: the port copy of a starboard one.
pub(crate) fn mirror_cell<'a>(placed: &ShipCell<'a>) -> ShipCell<'a> {
    ShipCell {
        cell: IVec3::new(-1 - placed.cell.x, placed.cell.y, placed.cell.z),
        prototype: placed.prototype,
        faces: std::array::from_fn(|face| placed.faces[mirror_face(face)]),
        exit: placed.exit.map(mirror_face),
    }
}
