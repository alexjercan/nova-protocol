//! Reading the section catalog as a set of grid tiles.
//!
//! The ADJACENCY RULE is the catalog's own link points: a prototype earns a
//! place on the grid by its sockets alone, and two tiles may sit face to face
//! exactly when the sockets they turn to each other agree. Nothing in this
//! module authors a rule; it reads one off content that already exists.

use std::collections::HashSet;

use bevy::prelude::*;
use nova_ship::prelude::{
    exit_normal, GameSections, GrammarPart, GrammarZone, SectionCollider, SectionConfig,
    SectionFootprint, ShipGrammarConfig,
};

use crate::grid::{face_index, mirror_face, snapped, FACES, GRID_EPSILON};

/// Index of the vacuum tile, which [`build`] always puts first.
pub(crate) const VACUUM: usize = 0;

/// One part of the grammar's draw, resolved against the catalog.
pub(crate) struct Family {
    /// The catalog section id.
    pub(crate) prototype: String,
    /// Draw weight for the PART, spent across whatever orientations of it are
    /// legal in a given cell.
    pub(crate) weight: f32,
    /// The only face this part may fire through, in [`FACES`] order, or `None`
    /// for a part that may point any way the mating rule allows.
    pub(crate) aim: Option<usize>,
    /// The only region of the hull this part may stand in, or `None` for a part
    /// free to stand anywhere the mating rule allows.
    pub(crate) zone: Option<GrammarZone>,
}

/// What one cell may hold.
pub(crate) struct Tile {
    /// The prototype placed in the cell, or `None` for vacuum.
    pub(crate) part: Option<TileBody>,
    /// Which [`Family`] this tile is an orientation of, or `None` for vacuum.
    /// Draw weights are authored per PART, so the draw needs to know which
    /// tiles are the same part wearing different rotations.
    pub(crate) family: Option<usize>,
    /// Per-face sockets this tile presents to its neighbours, in [`FACES`]
    /// order, read off the rotated link points.
    pub(crate) faces: [bool; 6],
    /// The face this CELL fires, launches or exhausts through, in [`FACES`]
    /// order, or `None` for a cell that does none of those. It is the cell's
    /// clearance: nothing may stand in front of it.
    ///
    /// A big drive's nozzle is as wide as the drive, so every cell of its
    /// exhaust face carries this and every cell of the layers in front of it
    /// carries `None`.
    pub(crate) exit: Option<usize>,
    /// The face the PART this tile belongs to fires through, on every one of
    /// its cells, or `None` for a part that fires nowhere.
    ///
    /// Which way a part POINTS is a property of the part, and the grammar's
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
/// Rotations are swept coarsely (three quarter-turn axes, 64 combinations) and
/// deduplicated by where they send the part's own axes, which leaves the 24
/// ways a cube can sit.
///
/// Deduplicating by SOCKET PATTERN instead would be tempting - it is all the
/// collapse can tell apart - and it was wrong. Sockets do not determine a
/// shape. A ramp blind on +Y and +X is the same socket pattern whether it lies
/// on its -Y face sloping toward +X or on its -X face sloping toward +Y, and
/// those are different models; keeping one and throwing away the other let the
/// generator plant ramps on their sides. The camera can tell orientations apart
/// even where the rule cannot, so they are kept apart.
///
/// `Err` carries the line the caller shows: a grammar naming a part the
/// catalog does not hold, or one whose geometry cannot be mirrored.
pub(crate) fn build(
    sections: &GameSections,
    grammar: &ShipGrammarConfig,
) -> Result<(Vec<Tile>, Vec<Family>), String> {
    let quarter = std::f32::consts::FRAC_PI_2;
    let mut tiles = vec![vacuum_tile()];
    let mut families = Vec::new();

    for part in &drawn_and_seeded(grammar) {
        let config = sections.get_section(&part.prototype).ok_or_else(|| {
            format!(
                "grammar '{}' draws '{}', which the catalog does not hold",
                grammar.id, part.prototype
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

        let single = SectionFootprint::from_collider(config.base.collider.unwrap_or_default()).0
            == UVec3::ONE;
        let mut seen = HashSet::new();
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let rotation = (Quat::from_rotation_x(x as f32 * quarter)
                        * Quat::from_rotation_y(y as f32 * quarter)
                        * Quat::from_rotation_z(z as f32 * quarter))
                    .normalize();
                    let Some(basis) = rotated_basis(rotation) else {
                        continue;
                    };
                    if !seen.insert(basis) {
                        continue;
                    }
                    let group = if single {
                        tile(family, config, rotation)?.into_iter().collect()
                    } else {
                        segment_tiles(family, config, rotation)?.unwrap_or_default()
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
        }
    }

    Ok((tiles, families))
}

/// Every prototype the grammar NAMES, in one list: the parts it draws, then
/// any role it seeds that the draw does not already price.
///
/// A seeded role needs tiles as much as a drawn one does - the collapse assigns
/// it by hand - but it is not a draw, so it joins at weight zero and is never
/// offered. Without this a grammar that seeds a part it does not also list
/// fails deep in the seed with "cannot stand on the grid at all", which names
/// the symptom rather than the omission.
fn drawn_and_seeded(grammar: &ShipGrammarConfig) -> Vec<GrammarPart> {
    let mut parts = grammar.parts.clone();
    let keel = &grammar.keel;
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
        parts.push(GrammarPart {
            prototype: prototype.to_string(),
            weight: 0.0,
            aim: None,
            zone: None,
        });
    }
    parts
}

/// One authored draw entry with its aim resolved to a [`FACES`] index.
fn resolved(part: &GrammarPart) -> Result<Family, String> {
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

/// Read one prototype at one rotation as a grid tile, or reject it.
///
/// A prototype earns a place on the grid by its sockets alone: each must point
/// down a cardinal axis, sit ON that axis, and be inset from its cell face by
/// the SAME distance as every other socket the part has - because that common
/// inset is the one placement offset that puts all of them on their cell faces
/// at once. A part that cannot satisfy that (an oblique socket, two different
/// insets, a body too big for a cell) is simply not a tile at this rotation,
/// and the generator never sees it. `Err` is reserved for a part that could
/// never tile at ANY rotation, which is an authoring fault.
fn tile(family: usize, config: &SectionConfig, rotation: Quat) -> Result<Option<Tile>, String> {
    let mut faces = [false; 6];
    let mut offset: Option<Vec3> = None;

    for point in &config.base.link_points {
        let Some(face) = face_index(rotation * point.normal) else {
            return Ok(None);
        };
        let axis = FACES[face];
        let position = rotation * point.position;
        let inset = position.dot(axis);
        if !position.abs_diff_eq(axis * inset, GRID_EPSILON) {
            return Ok(None);
        }
        let candidate = axis * (0.5 - inset);
        match offset {
            Some(previous) if !previous.abs_diff_eq(candidate, GRID_EPSILON) => return Ok(None),
            _ => offset = Some(candidate),
        }
        faces[face] = true;
    }

    // A part with no sockets at all can never mate, so it can never be part of
    // one connected ship.
    let Some(offset) = offset else {
        return Ok(None);
    };

    // The body has to stay inside its own cell. This is what makes the
    // overlap lint unreachable by construction rather than by luck: two
    // sections in different cells cannot interpenetrate if neither leaves its
    // cell, and a cell holds one section.
    let half = rotated_half_extents(config.base.collider.unwrap_or_default(), rotation);
    if (offset.abs() + half).max_element() > 0.5 + GRID_EPSILON {
        return Ok(None);
    }

    let exit = exit_normal(&config.kind).and_then(|normal| face_index(rotation * normal));
    if exit.is_some_and(|face| faces[face]) {
        return Err(format!(
            "'{}' carries a socket on the face it fires through, so a hull slab could be \
             bolted across its muzzle",
            config.base.id
        ));
    }

    Ok(Some(Tile {
        part: Some(TileBody {
            prototype: config.base.id.clone(),
            rotation,
            offset: snapped(offset),
        }),
        family: Some(family),
        faces,
        exit,
        aims: exit,
        joints: [None; 6],
        emits: true,
        span: UVec3::ONE,
        seam_flush: offset.x.abs() < GRID_EPSILON,
    }))
}

/// Read one MULTI-CELL prototype at one rotation as a BLOCK of segment tiles,
/// or reject it. Returned joints are LOCAL segment indices; [`build`] remaps
/// them once the segments' final indices exist.
///
/// The bar a segment part has to clear is stricter than [`tile`]'s: every
/// socket must sit exactly on the face centre of one of the part's own cells,
/// so each cell presents the face-centre socket a unit part would. A big drive
/// is authored that way already - one link point per cell of its mount face -
/// so a 3x3x2 drive mates to nine hull cubes and a 5x5x3 to twenty-five, and
/// the adjacency rule needs to know nothing about how big a part is.
///
/// The block is held together by [`Tile::joints`], which name the one tile each
/// face may sit against. A cell that draws a corner segment forces its whole
/// block through propagation, or the domain empties and the seed is refused.
///
/// One segment emits the placed section: the corner at the part's local
/// minimum, carrying the offset that puts the part's centre back over the
/// block. The rest only occupy their cells.
fn segment_tiles(
    family: usize,
    config: &SectionConfig,
    rotation: Quat,
) -> Result<Option<Vec<Tile>>, String> {
    let footprint = SectionFootprint::from_collider(config.base.collider.unwrap_or_default()).0;
    let extent = footprint.as_vec3();
    let turned_span = (rotation * extent).abs().round().as_uvec3();
    let index = |cell: UVec3| ((cell.x * footprint.y + cell.y) * footprint.z + cell.z) as usize;
    let cells = move || {
        (0..footprint.x).flat_map(move |x| {
            (0..footprint.y).flat_map(move |y| (0..footprint.z).map(move |z| UVec3::new(x, y, z)))
        })
    };
    // Where one of the part's own cells sits in the part's local space.
    let centre_of = move |cell: UVec3| cell.as_vec3() - (extent - Vec3::ONE) * 0.5;

    let mut segments: Vec<Tile> = cells()
        .map(|cell| Tile {
            part: Some(TileBody {
                prototype: config.base.id.clone(),
                rotation,
                offset: snapped(rotation * -centre_of(cell)),
            }),
            family: Some(family),
            faces: [false; 6],
            exit: None,
            aims: None,
            joints: [None; 6],
            emits: cell == UVec3::ZERO,
            span: turned_span,
            seam_flush: true,
        })
        .collect();

    for point in &config.base.link_points {
        let Some(face) = face_index(rotation * point.normal) else {
            return Ok(None);
        };
        let position = rotation * point.position;
        let found = cells().find(|cell| {
            (position - rotation * centre_of(*cell)).abs_diff_eq(FACES[face] * 0.5, GRID_EPSILON)
        });
        let Some(cell) = found else {
            return Ok(None);
        };
        segments[index(cell)].faces[face] = true;
    }

    if let Some(exit) = exit_normal(&config.kind) {
        let Some(out) = face_index(rotation * exit) else {
            return Ok(None);
        };
        let Some(local) = face_index(exit) else {
            return Err(format!(
                "'{}' fires down {exit:?}, which is not one of its own cardinal faces",
                config.base.id
            ));
        };
        // The exit is a whole FACE of the block rather than a cell of it: a big
        // drive's nozzle is as wide as the drive. Every cell on that face
        // carries the exit, so every one of them wants its own lane clear.
        let (axis, outer) = (
            local / 2,
            if local % 2 == 0 {
                extent - Vec3::ONE
            } else {
                Vec3::ZERO
            },
        );
        for cell in cells().filter(|cell| cell.as_vec3()[axis] == outer[axis]) {
            if segments[index(cell)].faces[out] {
                return Err(format!(
                    "'{}' carries a socket on the face it fires through, so a hull slab could \
                     be bolted across its muzzle",
                    config.base.id
                ));
            }
            segments[index(cell)].exit = Some(out);
        }
        for segment in &mut segments {
            segment.aims = Some(out);
        }
    }

    for cell in cells() {
        for face in 0..FACES.len() {
            let next = cell.as_vec3() + FACES[face];
            if next.cmplt(Vec3::ZERO).any() || next.cmpge(extent).any() {
                continue;
            }
            let Some(turned) = face_index(rotation * FACES[face]) else {
                return Ok(None);
            };
            segments[index(cell)].joints[turned] = Some(index(next.as_uvec3()));
        }
    }
    Ok(Some(segments))
}

/// Where a rotation sends the three axes, as [`FACES`] indices. Two rotations
/// with the same answer are the same rotation.
fn rotated_basis(rotation: Quat) -> Option<[usize; 3]> {
    Some([
        face_index(rotation * Vec3::X)?,
        face_index(rotation * Vec3::Y)?,
        face_index(rotation * Vec3::Z)?,
    ])
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
