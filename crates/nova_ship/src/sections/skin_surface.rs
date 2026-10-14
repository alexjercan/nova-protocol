//! The SURFACE GRAPH: a derived skin read as a quad graph, one cell per plate.
//!
//! A greeble solver needs to know which plate lies across each edge of each
//! plate, and which edge of that neighbour faces back. The derivation already
//! decides what lies across every edge to shape the plate; this module keeps
//! the plate it finds there instead of only the height.
//!
//! Each cell has four edge slots in a frame that turns the same way on every
//! face: `up` is [`link_point_up`] of the out face, `right` is `up x out`, and
//! the slots are numbered up, right, down, left. Seen from outside, every face
//! reads clockwise. The plate's own rotation is not used, because it is chosen
//! to make the shape canonical, and `face_plane` is not used, because its axis
//! pair mirrors on half the faces.
//!
//! The graph holds no geometry. A solver reads the links and the per-cell key
//! only.

use bevy::{platform::collections::HashMap, prelude::*};
use nova_gameplay::prelude::Fnv64;

use crate::sections::{
    link_points::link_point_up,
    shell_skin::{exit_pocket, face_plane, walls, SkinPlate, SkinStructure, FACES},
};

/// The prelude: `skin_surface`, `slot_direction`, `SkinSurface`, `SurfaceCell`,
/// `SurfaceLink` and `SURFACE_SLOTS`.
pub mod prelude {
    pub use super::{
        skin_surface, slot_direction, SkinSurface, SurfaceCell, SurfaceLink, SURFACE_SLOTS,
    };
}

/// The edge slots of one surface cell, in the order up, right, down, left.
pub const SURFACE_SLOTS: usize = 4;

/// The far end of one edge slot: the neighbour cell and the slot of that
/// neighbour that faces back across the same edge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SurfaceLink {
    /// Index of the neighbour cell in [`SkinSurface::cells`].
    pub cell: usize,
    /// The neighbour's slot, numbered as [`SURFACE_SLOTS`].
    pub slot: u8,
}

/// One cell of the surface graph. Cell `i` is plate `i` of the plates the graph
/// was built from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SurfaceCell {
    /// A hash of the plate's cell and out face. A seeded draw keyed on it does
    /// not move when cells are added or removed elsewhere in the graph.
    pub key: u64,
    /// The neighbour across each slot, or `None` where no plate continues the
    /// surface.
    pub links: [Option<SurfaceLink>; SURFACE_SLOTS],
}

/// A derived skin as a quad graph. Links are mutual: if cell `p` slot `k` names
/// cell `q` slot `m`, then cell `q` slot `m` names cell `p` slot `k`.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SkinSurface {
    cells: Vec<SurfaceCell>,
}

impl SkinSurface {
    /// The cells, in plate order.
    pub fn cells(&self) -> &[SurfaceCell] {
        &self.cells
    }

    /// The number of cells.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether the graph has no cells.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// The link across `slot` of `cell`.
    pub fn link(&self, cell: usize, slot: usize) -> Option<SurfaceLink> {
        self.cells[cell].links[slot]
    }
}

/// The in-plane step across `slot` of a cell whose plate shows `out`, with
/// slots numbered up, right, down, left.
pub fn slot_direction(out: IVec3, slot: usize) -> IVec3 {
    let up = link_point_up(out.as_vec3()).round().as_ivec3();
    let right = up.cross(out);
    [up, right, -up, -right][slot]
}

/// The surface graph of `plates`, derived from `structure`.
///
/// Across each slot the neighbour is found by the same branches
/// `boundary_heights` reads, in its order:
///
/// - structure the skin climbs: the plate one cell up the wall, else the plate
///   on top of a wall one cell high;
/// - other structure: none, the skin stops there;
/// - a plate in the next cell: that plate, in the same plane or turned;
/// - an exit pocket: none;
/// - open space: the plate round a convex fold, else none.
///
/// A link found from one side only is made mutual when the far slot is free.
/// Mutual links are kept first, then one-sided ones in plate and slot order;
/// a far slot another plate holds leaves this slot a border.
pub fn skin_surface(structure: &SkinStructure, plates: &[SkinPlate]) -> SkinSurface {
    let at: HashMap<IVec3, usize> = plates
        .iter()
        .enumerate()
        .map(|(index, plate)| (plate.cell, index))
        .collect();
    let raw: Vec<[Option<usize>; SURFACE_SLOTS]> = plates
        .iter()
        .map(|plate| {
            std::array::from_fn(|slot| {
                let across = slot_direction(out_of(plate), slot);
                neighbour(structure, plates, &at, plate, across)
            })
        })
        .collect();

    let mut links = vec![[None; SURFACE_SLOTS]; plates.len()];
    for mutual_only in [true, false] {
        for (cell, slots) in raw.iter().enumerate() {
            for (slot, far) in slots.iter().enumerate() {
                let Some(far) = *far else { continue };
                let back = back_slot(&plates[cell], slot, &plates[far]);
                let named = raw[far][back];
                let agrees = match mutual_only {
                    true => named == Some(cell),
                    false => named.is_none() || named == Some(cell),
                };
                if agrees && links[cell][slot].is_none() && links[far][back].is_none() {
                    links[cell][slot] = Some(SurfaceLink {
                        cell: far,
                        slot: back as u8,
                    });
                    links[far][back] = Some(SurfaceLink {
                        cell,
                        slot: slot as u8,
                    });
                }
            }
        }
    }

    SkinSurface {
        cells: plates
            .iter()
            .zip(links)
            .map(|(plate, links)| SurfaceCell {
                key: plate_key(plate),
                links,
            })
            .collect(),
    }
}

/// The direction a plate shows to space: from the structure it bolts to into
/// its own cell.
fn out_of(plate: &SkinPlate) -> IVec3 {
    plate.cell - plate.anchor
}

/// The plate the derivation finds across the edge of `plate` that steps
/// `across`, before the links are made mutual.
fn neighbour(
    structure: &SkinStructure,
    plates: &[SkinPlate],
    at: &HashMap<IVec3, usize>,
    plate: &SkinPlate,
    across: IVec3,
) -> Option<usize> {
    let facing = |cell: IVec3, out: IVec3| {
        at.get(&cell)
            .copied()
            .filter(|index| out_of(&plates[*index]) == out)
    };
    let (cell, n) = (plate.cell, out_of(plate));
    let next = cell + across;
    if structure.filled(next) {
        let face = FACES
            .iter()
            .position(|face| *face == n)
            .expect("a plate shows a cardinal face");
        let (u, v) = face_plane(face);
        match walls(structure, next, u, v) {
            true => facing(cell + n, -across).or_else(|| facing(next + n, n)),
            false => None,
        }
    } else if let Some(index) = at.get(&next) {
        Some(*index)
    } else if exit_pocket(structure, next) {
        None
    } else {
        facing(cell - n + across, across)
    }
}

/// The slot of `far` that faces back toward the edge of `near` across `slot`:
/// the one pointing most toward that edge's midpoint, measured from the
/// middle of `far`'s mounting face. A tie goes to the slot that points most
/// toward `near`'s cell, then to the lower slot.
///
/// Doubled coordinates keep the half-cell points on the integer grid.
fn back_slot(near: &SkinPlate, slot: usize, far: &SkinPlate) -> usize {
    let (near_out, far_out) = (out_of(near), out_of(far));
    let edge = near.cell * 2 - near_out + slot_direction(near_out, slot);
    let centre = far.cell * 2 - far_out;
    let (offset, toward) = (edge - centre, near.cell - far.cell);
    let mut best = (0, i32::MIN);
    for candidate in 0..SURFACE_SLOTS {
        let direction = slot_direction(far_out, candidate);
        let score = 4 * direction.dot(offset) + direction.dot(toward);
        if score > best.1 {
            best = (candidate, score);
        }
    }
    best.0
}

/// [`Fnv64`] over the plate's cell and out face.
fn plate_key(plate: &SkinPlate) -> u64 {
    let out = out_of(plate);
    let mut hash = Fnv64::new();
    for value in [
        plate.cell.x,
        plate.cell.y,
        plate.cell.z,
        out.x,
        out.y,
        out.z,
    ] {
        hash = hash.write(&value.to_le_bytes());
    }
    hash.finish()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::sections::shell_skin::derive_skin;

    fn hull(cells: &[IVec3]) -> SkinStructure {
        let mut structure = SkinStructure::default();
        for cell in cells {
            structure.insert(*cell, [true; 6]);
        }
        structure
    }

    /// A 3x3 deck at `y = 0`, `x` in `0..3`, with a wall `height` cells high
    /// standing on its `x = 0, z = 0` cell.
    fn deck_with_wall(height: i32) -> Vec<IVec3> {
        let deck = (0..3).flat_map(|x| (-1..=1).map(move |z| IVec3::new(x, 0, z)));
        let wall = (1..=height).map(|y| IVec3::new(0, y, 0));
        deck.chain(wall).collect()
    }

    fn surface_of(structure: &SkinStructure) -> (Vec<SkinPlate>, SkinSurface) {
        let plates = derive_skin(structure);
        let surface = skin_surface(structure, &plates);
        (plates, surface)
    }

    fn index_of(plates: &[SkinPlate], cell: IVec3) -> usize {
        plates
            .iter()
            .position(|plate| plate.cell == cell)
            .unwrap_or_else(|| panic!("a plate stands in {cell}"))
    }

    fn assert_mutual(surface: &SkinSurface) {
        for (cell, data) in surface.cells().iter().enumerate() {
            for (slot, link) in data.links.iter().enumerate() {
                if let Some(link) = link {
                    assert_eq!(
                        surface.link(link.cell, link.slot as usize),
                        Some(SurfaceLink {
                            cell,
                            slot: slot as u8
                        }),
                        "cell {cell} slot {slot} names a slot that does not name it back",
                    );
                }
            }
        }
    }

    /// Every corner of the graph as the set of (cell, corner) pairs that meet
    /// there, with corner `k` of a cell between its slots `k` and `k + 1`.
    ///
    /// Both faces of an edge run it in opposite directions, so corner `k` of a
    /// cell is corner `m` of the cell across slot `k + 1`, where `m` is the
    /// slot facing back. A walk that meets a border is an open corner.
    fn corners(surface: &SkinSurface) -> Vec<(BTreeSet<(usize, usize)>, bool)> {
        let mut seen = BTreeSet::new();
        let mut corners = Vec::new();
        for cell in 0..surface.len() {
            for corner in 0..SURFACE_SLOTS {
                if seen.contains(&(cell, corner)) {
                    continue;
                }
                let mut fan = BTreeSet::new();
                let (mut at, mut k) = (cell, corner);
                let closed = loop {
                    fan.insert((at, k));
                    match surface.link(at, (k + 1) % SURFACE_SLOTS) {
                        None => break false,
                        Some(link) => {
                            (at, k) = (link.cell, link.slot as usize);
                            if (at, k) == (cell, corner) {
                                break true;
                            }
                        }
                    }
                };
                seen.extend(fan.iter().copied());
                corners.push((fan, closed));
            }
        }
        corners
    }

    #[test]
    fn a_lone_cube_is_six_linked_cells_meeting_three_at_each_of_eight_corners() {
        let (plates, surface) = surface_of(&hull(&[IVec3::ZERO]));

        assert_eq!(plates.len(), 6);
        assert_eq!(surface.len(), 6);
        assert!(
            surface
                .cells()
                .iter()
                .all(|cell| cell.links.iter().all(Option::is_some)),
            "every slot of a lone cube is linked round a convex fold",
        );
        assert_mutual(&surface);
        let corners = corners(&surface);
        assert_eq!(corners.len(), 8);
        assert!(
            corners
                .iter()
                .all(|(fan, closed)| *closed && fan.len() == 3),
            "every cube corner closes over three cells: {corners:?}",
        );
    }

    #[test]
    fn an_l_shape_links_the_deck_to_the_wall_across_the_inner_corner() {
        // A 3x3 deck against a wall two cells high. The deck plate at
        // (1, 1, 0) faces up and the wall plate above it at (1, 2, 0) faces +X.
        let structure = hull(&deck_with_wall(2));
        let (plates, surface) = surface_of(&structure);
        let deck = index_of(&plates, IVec3::new(1, 1, 0));
        let wall = index_of(&plates, IVec3::new(1, 2, 0));
        assert_eq!(out_of(&plates[deck]), IVec3::Y);
        assert_eq!(out_of(&plates[wall]), IVec3::X);

        let toward_wall = (0..SURFACE_SLOTS)
            .find(|slot| slot_direction(IVec3::Y, *slot) == IVec3::NEG_X)
            .expect("an up face has a slot toward -X");
        let down_from_wall = (0..SURFACE_SLOTS)
            .find(|slot| slot_direction(IVec3::X, *slot) == IVec3::NEG_Y)
            .expect("a +X face has a slot toward -Y");
        assert_eq!(
            surface.link(deck, toward_wall),
            Some(SurfaceLink {
                cell: wall,
                slot: down_from_wall as u8
            }),
            "the deck's edge against the wall links to the wall plate's lower edge",
        );
        assert_mutual(&surface);
    }

    #[test]
    fn an_l_shape_with_a_wall_one_cell_high_links_the_deck_to_the_wall_top() {
        let structure = hull(&deck_with_wall(1));
        let (plates, surface) = surface_of(&structure);
        let deck = index_of(&plates, IVec3::new(1, 1, 0));
        let top = index_of(&plates, IVec3::new(0, 2, 0));
        assert_eq!(out_of(&plates[deck]), IVec3::Y);
        assert_eq!(out_of(&plates[top]), IVec3::Y);

        let slot_toward = |direction: IVec3| {
            (0..SURFACE_SLOTS)
                .find(|slot| slot_direction(IVec3::Y, *slot) == direction)
                .expect("an up face has a slot each way")
        };
        assert_eq!(
            surface.link(deck, slot_toward(IVec3::NEG_X)),
            Some(SurfaceLink {
                cell: top,
                slot: slot_toward(IVec3::X) as u8
            }),
            "the deck steps up onto the top of the wall",
        );
        assert_mutual(&surface);
    }

    #[test]
    fn a_bare_face_leaves_the_slots_round_it_unlinked() {
        let mut structure = SkinStructure::default();
        let mut faces = [true; 6];
        faces[0] = false;
        structure.insert(IVec3::ZERO, faces);
        let (plates, surface) = surface_of(&structure);

        assert_eq!(plates.len(), 5, "a bare face carries no plate");
        let unlinked: Vec<(IVec3, IVec3)> = surface
            .cells()
            .iter()
            .enumerate()
            .flat_map(|(cell, data)| {
                let plate = plates[cell];
                data.links
                    .iter()
                    .enumerate()
                    .filter(|(_, link)| link.is_none())
                    .map(move |(slot, _)| (plate.cell, slot_direction(out_of(&plate), slot)))
            })
            .collect();
        assert_eq!(
            unlinked.len(),
            4,
            "the four plates round the bare face each end there: {unlinked:?}",
        );
        assert!(
            unlinked.iter().all(|(_, across)| *across == IVec3::X),
            "each open slot steps toward the bare face: {unlinked:?}",
        );
        assert_mutual(&surface);
    }
}
