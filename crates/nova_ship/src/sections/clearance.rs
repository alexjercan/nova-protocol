//! EXIT CLEARANCE: nothing may stand in the space a part fires, launches or
//! exhausts into.
//!
//! A muzzle, a nozzle and a bay's mouth are all faces with nothing to mate, so
//! the mating rule alone says nothing about them - it is happy to let a drive
//! exhaust into the flank of the drive beside it. What may not be there is read
//! off the part's KIND ([`exit_normal`]), because a drive's flank carries no
//! socket either and nothing comes out of it.
//!
//! A lane cell has to be VOID, and void is two claims rather than one:
//!
//! - it holds no STRUCTURE. A hull slab two cells in front of a muzzle is where
//!   a torpedo is born ([`TorpedoSectionConfig::spawn_offset`] is cells, not a
//!   direction).
//! - nothing beside it OFFERS A SOCKET INTO IT. This is the half that cannot be
//!   fixed anywhere else. The skin covers every face of structure that would
//!   otherwise look at vacuum, so a hull standing DIAGONALLY off a muzzle
//!   demands a plate in the muzzle's own cell. Either the skin closes over the
//!   lane and the bay fires into its own plating, or the skin refuses the cell -
//!   a blind face keeps it out - and that hull face is left bare to space.
//!   There is no third answer, so the structure must not be there.
//!
//! Stated over SOCKETS rather than over plates on purpose: a socket offered
//! into a cell is exactly what [`cladding_cells`] reads to decide the cell is
//! clad, so this is the skin's own rule without a caller having to know what a
//! plate is. It is also why the ship is read into a [`SkinStructure`] here - the
//! skin and this rule want the same two facts about a cell.
//!
//! ONE copy, two callers. The generator ([`examples/playable/wfc_ships.rs`])
//! collapses ships that cannot break it, and the editor refuses a placement that
//! would. Two copies of a constraint this subtle drift apart.
//!
//! [`TorpedoSectionConfig::spawn_offset`]: crate::sections::torpedo_section::prelude::TorpedoSectionConfig::spawn_offset
//! [`cladding_cells`]: super::shell_skin::cladding_cells

use bevy::{platform::collections::HashSet, prelude::*};

use crate::sections::{
    base_section::prelude::{SectionConfig, SectionKind},
    shell_skin::{lattice_phase, read_cells, PlacedPart, SkinStructure, FACES},
};

/// The prelude: `exit_normal`, `SectionExit`, `BlockedExit`,
/// `BlockedExitReason`, `ShipExit`, `blocked_exits`, `exit_lanes` and
/// `placement_blocks_an_exit`.
pub mod prelude {
    pub use super::{
        blocked_exits, exit_lanes, exit_normal, placement_blocks_an_exit, BlockedExit,
        BlockedExitReason, SectionExit, ShipExit,
    };
}

/// Which way a part's business points, in the part's OWN frame, or `None` for a
/// part whose whole surface is structure.
///
/// Read off the KIND, which is where the content already says it, and never
/// guessed from an absent socket: a drive's flank carries no socket either and
/// nothing comes out of it.
pub fn exit_normal(kind: &SectionKind) -> Option<Vec3> {
    match kind {
        // The tube fires down `spawn_offset`, and the torpedo is BORN at the
        // far end of it rather than at the muzzle.
        SectionKind::Torpedo(bay) => Some(bay.spawn_offset.normalize()),
        // Thrust is -Z (`thruster_impulse_system`), so the bell opens on +Z and
        // the plume leaves through it.
        SectionKind::Thruster(_) => Some(Vec3::Z),
        // The mount bolts down by -Y and the gun stands on top of it. A turret
        // traverses, so this is the one direction it can always fire.
        SectionKind::Turret(_) => Some(Vec3::Y),
        SectionKind::Hull(_) | SectionKind::Controller(_) => None,
    }
}

/// Which way a live section fires, in its OWN frame.
///
/// Put on a section when it is spawned from a [`SectionConfig`], because that is
/// the last point anything knows its KIND: a section entity carries its sockets
/// and its collider as components and nothing that says what sort of part it is.
/// The skin reads it back off the ship to decide which cells may not be clad.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct SectionExit(pub Vec3);

impl SectionExit {
    /// The exit a section built from `config` carries, or `None` for a part
    /// whose whole surface is structure.
    pub fn of(config: &SectionConfig) -> Option<Self> {
        exit_normal(&config.kind).map(Self)
    }
}

/// An exit, in the cells a ship is read into: the cell the part stands in and
/// the face of that cell it fires through, indexed as [`FACES`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShipExit {
    /// The cell the part stands in.
    pub cell: IVec3,
    /// The face it fires through.
    pub out: usize,
}

/// Why a lane cell is not void.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockedExitReason {
    /// The lane cell holds structure, and the shot is fired into it.
    Structure,
    /// A section beside the lane offers a socket into it, so the skin will
    /// close the lane over and the shot is fired into its own ship's plating.
    Cladding,
}

/// One exit that cannot fire where it stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BlockedExit {
    /// The cell holding the part that cannot fire.
    pub part: IVec3,
    /// The cell of its lane that is not void.
    pub lane: IVec3,
    /// The cell that blocks it - the lane cell itself when it holds structure,
    /// and the neighbour demanding cladding when it does not.
    pub blocker: IVec3,
    /// Which of the two claims about a void lane cell fails.
    pub reason: BlockedExitReason,
}

/// The cells an exit fires through: straight out from the part until the lane
/// leaves the ship.
///
/// A COLUMN, not a cone and not one cell. One cell is not enough for any of the
/// three: a torpedo is BORN two cells out, a plume is drawn several cells long,
/// and a round leaves down the whole line. A cone would be guessing at a
/// traverse the caller cannot know.
///
/// It stops one cell outside the ship's own extent, because that is as far as a
/// blocked lane can reach: past there no neighbour of a lane cell can hold
/// structure, so nothing can be in the way and nothing can demand cladding.
fn lane(reach: (IVec3, IVec3), exit: ShipExit) -> impl Iterator<Item = IVec3> {
    let step = FACES[exit.out];
    (1..)
        .map(move |ahead| exit.cell + step * ahead)
        .take_while(move |cell| cell.cmpge(reach.0).all() && cell.cmple(reach.1).all())
}

/// How far out a lane is worth walking: the extent of the ship's filled cells,
/// grown by the one cell a plate stands in. Past that no neighbour of a lane
/// cell can hold structure, so nothing can be in the way and nothing can demand
/// cladding.
fn reach(structure: &SkinStructure) -> (IVec3, IVec3) {
    structure
        .filled_cells()
        .fold((IVec3::MAX, IVec3::MIN), |(low, high), cell| {
            (low.min(cell - IVec3::ONE), high.max(cell + IVec3::ONE))
        })
}

/// Every cell claimed by an exit lane.
///
/// Handed to whatever else moves structure around, so that filling a dent
/// cannot re-block a lane that has already been cleared.
pub fn exit_lanes(structure: &SkinStructure, exits: &[ShipExit]) -> HashSet<IVec3> {
    let reach = reach(structure);
    exits.iter().flat_map(|exit| lane(reach, *exit)).collect()
}

/// Every exit on a ship that cannot fire where it stands, one finding per lane
/// cell that is not void.
pub fn blocked_exits(structure: &SkinStructure, exits: &[ShipExit]) -> Vec<BlockedExit> {
    let reach = reach(structure);
    let mut blocked = Vec::new();
    for exit in exits {
        for cell in lane(reach, *exit) {
            if structure.filled(cell) {
                blocked.push(BlockedExit {
                    part: exit.cell,
                    lane: cell,
                    blocker: cell,
                    reason: BlockedExitReason::Structure,
                });
            }
            for (face, direction) in FACES.iter().copied().enumerate() {
                let beside = cell + direction;
                if structure.offers(beside, face ^ 1) == Some(true) {
                    blocked.push(BlockedExit {
                        part: exit.cell,
                        lane: cell,
                        blocker: beside,
                        reason: BlockedExitReason::Cladding,
                    });
                }
            }
        }
    }
    blocked
}

/// A set of placed parts read into the cells the skin buckets them into, plus
/// the exits they carry.
///
/// The lattice is passed in rather than read here, so that two readings of the
/// same ship - with a part and without it - land in the same cells. A ship has
/// no grid, and the offset its sections share moves when the set does.
///
/// The reading itself is the SKIN's ([`read_cells`]), which is the whole point:
/// the lateral clause below is the skin's own rule, so the two must bucket a
/// ship the same way or they are arguing about different ships.
fn read_ship(parts: &[PlacedPart], phase: Vec3) -> (SkinStructure, Vec<ShipExit>) {
    let (structure, cells) = read_cells(parts, phase);
    let exits = parts
        .iter()
        .zip(cells)
        .filter_map(|(part, cell)| {
            let out = super::shell_skin::face_index(part.rotation * part.exit?)?;
            Some(ShipExit { cell, out })
        })
        .collect();
    (structure, exits)
}

/// Whether adding `part` to `ship` would leave something unable to fire.
///
/// The question a BUILDER asks, and the answer is about the part rather than
/// about the ship it joins: a hull that already cannot fire is not made worse
/// by the next slab, and refusing every placement on a ship with one bad bay on
/// it would be unbuildable. So the findings are counted twice, with the part
/// and without it, and only a NEW one refuses.
///
/// Both readings share one lattice, taken over the whole set. Reading them
/// apart would let the extra part move the cells everything else stands in, and
/// the difference would be about the bucketing rather than about the placement.
pub fn placement_blocks_an_exit(ship: &[PlacedPart], part: &PlacedPart) -> bool {
    let mut whole: Vec<PlacedPart> = ship.to_vec();
    whole.push(part.clone());
    let phase = lattice_phase(whole.iter().map(|part| part.position));

    let (before, exits) = read_ship(ship, phase);
    let (after, with) = read_ship(&whole, phase);
    blocked_exits(&after, &with).len() > blocked_exits(&before, &exits).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sections::{
        link_points::{prelude::LinkPoint, unit_cube_link_points},
        shell_skin::derive_skin,
    };

    /// A hull cube: sockets on all six faces, nothing to fire.
    fn hull(position: Vec3) -> PlacedPart<'static> {
        static POINTS: std::sync::OnceLock<Vec<LinkPoint>> = std::sync::OnceLock::new();
        PlacedPart {
            position,
            rotation: Quat::IDENTITY,
            link_points: POINTS.get_or_init(unit_cube_link_points),
            exit: None,
        }
    }

    /// A bay: one socket on the face it bolts down through, firing the other
    /// way, with the torpedo born two cells out.
    fn bay(position: Vec3, rotation: Quat) -> PlacedPart<'static> {
        static POINTS: std::sync::OnceLock<Vec<LinkPoint>> = std::sync::OnceLock::new();
        PlacedPart {
            position,
            rotation,
            link_points: POINTS.get_or_init(|| {
                vec![LinkPoint {
                    id: "base".to_string(),
                    position: Vec3::NEG_Y * 0.5,
                    normal: Vec3::NEG_Y,
                }]
            }),
            exit: Some(Vec3::Y),
        }
    }

    /// A slab in front of a muzzle is refused, and the same slab behind it is
    /// not.
    #[test]
    fn a_part_may_not_be_built_in_front_of_a_muzzle() {
        let ship = vec![hull(Vec3::ZERO), bay(Vec3::Y, Quat::IDENTITY)];
        assert!(
            placement_blocks_an_exit(&ship, &hull(Vec3::Y * 3.0)),
            "a slab in the lane is where the torpedo is born",
        );
        assert!(
            !placement_blocks_an_exit(&ship, &hull(Vec3::NEG_Y)),
            "a slab BEHIND the bay is nothing to do with its lane",
        );
    }

    /// A part standing DIAGONALLY off a muzzle is refused too.
    ///
    /// The subtle half, and the owner's original report. Nothing is in the lane,
    /// but the hull beside it offers a socket into the lane cell, so the skin
    /// closes the lane over and the bay fires into its own plating.
    #[test]
    fn a_part_diagonally_off_a_muzzle_is_refused_for_the_cladding_it_demands() {
        let ship = vec![hull(Vec3::ZERO), bay(Vec3::Y, Quat::IDENTITY)];
        assert!(
            placement_blocks_an_exit(&ship, &hull(Vec3::new(1.0, 2.0, 0.0))),
            "a hull one cell off the lane demands cladding inside it",
        );
    }

    /// A ship that is already broken does not refuse everything.
    ///
    /// A builder has to be able to carry on - and to fix it by taking the part
    /// away, which is the other half of the same claim.
    #[test]
    fn a_ship_that_already_cannot_fire_still_takes_a_placement() {
        let blocked = vec![
            hull(Vec3::ZERO),
            bay(Vec3::Y, Quat::IDENTITY),
            hull(Vec3::Y * 3.0),
        ];
        assert!(
            !placement_blocks_an_exit(&blocked, &hull(Vec3::X)),
            "a slab on the far flank is not what blocks that bay",
        );
    }

    /// Two drives flank to flank are legal; one exhausting into the other's
    /// side is not.
    #[test]
    fn a_drive_may_stand_beside_a_drive_but_not_in_front_of_one() {
        let quarter = std::f32::consts::FRAC_PI_2;
        let ship = vec![hull(Vec3::ZERO), bay(Vec3::Y, Quat::IDENTITY)];
        // Beside it, firing the same way: a bank of nozzles.
        assert!(
            !placement_blocks_an_exit(&ship, &bay(Vec3::new(1.0, 1.0, 0.0), Quat::IDENTITY)),
            "two drives side by side is a bank, not a fault",
        );
        // Turned across the first one's lane, so its own body is in the way.
        assert!(
            placement_blocks_an_exit(&ship, &bay(Vec3::Y * 2.0, Quat::from_rotation_z(quarter))),
            "a part standing in the lane blocks it whichever way it faces",
        );
    }

    /// The skin may plate a fitting's FLANK and still not its LANE.
    ///
    /// This rule and the skin pull opposite ways over one [`SkinStructure`]:
    /// the skin covers every face of structure that would otherwise look at
    /// vacuum, and this rule wants a column in front of every muzzle left
    /// empty. Narrowing the skin's refusal from a fitting's five blind faces to
    /// its one exit face is only safe if the lane is held by something else -
    /// and it is, by the socket clause below, which is why a hull DIAGONALLY
    /// off a muzzle is refused. Checked over a whole derivation rather than
    /// argued.
    #[test]
    fn the_skin_plates_around_a_muzzle_without_closing_its_lane() {
        let mut ship: Vec<PlacedPart> = (-1..=1)
            .flat_map(|x| (-1..=1).map(move |z| hull(Vec3::new(x as f32, 0.0, z as f32))))
            .collect();
        ship.push(bay(Vec3::Y, Quat::IDENTITY));

        let phase = lattice_phase(ship.iter().map(|part| part.position));
        let (structure, exits) = read_ship(&ship, phase);
        assert!(
            blocked_exits(&structure, &exits).is_empty(),
            "a bay on the middle of a flat deck can fire",
        );

        let lanes = exit_lanes(&structure, &exits);
        assert!(!lanes.is_empty(), "the bay has a lane to hold");
        let clad: Vec<IVec3> = derive_skin(&structure)
            .iter()
            .map(|plate| plate.cell)
            .collect();
        assert!(
            clad.contains(&IVec3::new(1, 1, 0)),
            "the deck beside a muzzle is plating, not a hole",
        );
        for cell in lanes {
            assert!(
                !clad.contains(&cell),
                "{cell:?} is in the bay's lane and the skin closed over it",
            );
        }
    }
}
