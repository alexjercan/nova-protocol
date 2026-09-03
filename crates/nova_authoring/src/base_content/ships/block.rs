//! The base game's BLOCK fleet: hand-authored ships assembled from shipped
//! cube prototypes on a cell grid, wearing a derived skin.
//!
//! Nothing here reaches for a modelled part. A block ship is a set of cells and
//! a handful of placed specials, so it is readable in a diff, reproducible from
//! the source, and buildable by anyone with the editor and no art pipeline -
//! which is the whole point of making it the base game's own identity. The
//! Kenney semantic fleet beside it says the opposite thing on purpose: that a
//! mod can bring its own GLBs.
//!
//! The STYLE is the fleet's vocabulary. `armoured` is military, `salvage` is
//! scavenged, `industrial` is working freight, and a backdrop reads which is
//! which before anything shoots. See `base_content::styles`.
//!
//! Cells are BUILD-GRID cells, the one authored coordinate that is not metric:
//! one cell is one world unit is 10 m.

use std::collections::HashSet;

use bevy::prelude::*;
use nova_scenario::prelude::{SectionSource, SpaceshipSectionConfig};

use crate::base_content::styles::{ARMOURED_STYLE_ID, INDUSTRIAL_STYLE_ID, SALVAGE_STYLE_ID};

/// The plain structural cell every block hull is mostly made of.
const HULL: &str = "reinforced_hull_section";
const CONTROLLER: &str = "basic_controller_section";
const THRUSTER: &str = "basic_thruster_section";
const VECTOR_THRUSTER: &str = "vector_thruster_section";
const PDC: &str = "pdc_kinetic_turret_section";

/// How far a turret drops into its own cell to put its socket on the plate
/// below: the mount's one link point sits a quarter cell under its centre.
const TURRET_SEAT: Vec3 = Vec3::new(0.0, -0.25, 0.0);

/// The section id every block ship's main flight computer carries, so content
/// can harden or disable one bridge without knowing which hull it is on.
pub(crate) const BLOCK_BRIDGE_SECTION_ID: &str = "bridge";

/// The gunship's point-defense mounts, in the order they are placed. Content
/// that sets a magazine per turret walks this rather than naming six strings
/// it would have to keep in step with the hull.
pub(crate) const BLOCK_GUNSHIP_TURRET_IDS: [&str; 6] = [
    "pdc_forward_port",
    "pdc_forward_starboard",
    "pdc_aft_port",
    "pdc_aft_starboard",
    "pdc_ventral_port",
    "pdc_ventral_starboard",
];

/// One placed part that is not a plain hull cell. A special whose position
/// lands exactly on a cell REPLACES that cell; one placed off the grid (a
/// turret seated on a face, a drive standing off the transom) is added beside
/// it.
#[derive(Clone, Copy)]
pub(super) struct Special {
    id: &'static str,
    prototype: &'static str,
    position: Vec3,
    rotation: Quat,
}

/// A block hull: its cells, its specials, and the style its skin wears.
pub(super) struct BlockShip {
    pub(super) cells: Vec<IVec3>,
    pub(super) specials: Vec<Special>,
    pub(super) style: &'static str,
}

/// The small unarmed workboat: one hull layer, two sponsons, a dorsal cab and
/// a pair of bell drives. The smallest thing in the fleet that still reads as
/// a crewed ship rather than a drone, and the shape the campaign's opening
/// cutter is drawn from.
pub(super) fn utility_cutter() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -3), IVec3::new(3, 1, 6)),
            block(IVec3::new(-2, 0, 0), IVec3::new(1, 1, 3)),
            block(IVec3::new(2, 0, 0), IVec3::new(1, 1, 3)),
            vec![IVec3::new(0, 0, -4), IVec3::new(0, 1, -1)],
        ]),
        specials: vec![
            cell_part(BLOCK_BRIDGE_SECTION_ID, CONTROLLER, IVec3::new(0, 1, -1)),
            cell_part("drive_port", THRUSTER, IVec3::new(-1, 0, 2)),
            cell_part("drive_starboard", THRUSTER, IVec3::new(1, 0, 2)),
        ],
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// The freight hull: a flat spine between two cargo shoulders, a stack of
/// containers amidships, and a square transom with one vectoring drive on it.
/// The widest ship in the fleet, so it reads as cargo from any angle a
/// backdrop camera can take.
pub(super) fn bulk_hauler() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -3), IVec3::new(3, 1, 7)),
            block(IVec3::new(-3, 0, -1), IVec3::new(2, 1, 3)),
            block(IVec3::new(2, 0, -1), IVec3::new(2, 1, 3)),
            block(IVec3::new(-1, 1, -1), IVec3::new(3, 1, 3)),
            // The transom: a full three-by-three face is what a vectoring
            // drive mates onto, and the only reason this hull has a ventral
            // layer at all.
            block(IVec3::new(-1, -1, 3), IVec3::new(3, 3, 1)),
            vec![IVec3::new(0, 0, -4), IVec3::new(0, 1, -2)],
        ]),
        specials: vec![
            cell_part(BLOCK_BRIDGE_SECTION_ID, CONTROLLER, IVec3::new(0, 1, -2)),
            Special {
                id: "main_drive",
                prototype: VECTOR_THRUSTER,
                // Standing off the transom: the drive is three cells square
                // and two deep, so its forward face lands on the z = 3 layer.
                position: Vec3::new(0.0, 0.0, 4.5),
                rotation: Quat::IDENTITY,
            },
        ],
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// The military patrol boat: a two-deck fighting spine over a short ventral
/// keel, stub wings, a dorsal fin, one vectoring drive, and six point-defense
/// mounts covering both hemispheres. About a cutter and a half long - small
/// enough to read as a patrol boat rather than a capital, and the base game's
/// answer to "what does a warship look like here".
pub(super) fn patrol_gunship() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -2), IVec3::new(3, 2, 5)),
            // The keel, which squares the transom off for the drive and gives
            // the ventral turrets something to stand under.
            block(IVec3::new(-1, -1, 1), IVec3::new(3, 1, 2)),
            block(IVec3::new(-2, 0, 0), IVec3::new(1, 1, 2)),
            block(IVec3::new(2, 0, 0), IVec3::new(1, 1, 2)),
            block(IVec3::new(0, 2, -1), IVec3::new(1, 1, 3)),
            vec![
                IVec3::new(0, 0, -3),
                IVec3::new(0, 1, -3),
                IVec3::new(0, 0, -4),
            ],
        ]),
        specials: vec![
            cell_part(BLOCK_BRIDGE_SECTION_ID, CONTROLLER, IVec3::new(0, 1, -1)),
            cell_part("control_aft", CONTROLLER, IVec3::new(0, 1, 1)),
            Special {
                id: "main_drive",
                prototype: VECTOR_THRUSTER,
                position: Vec3::new(0.0, 0.0, 3.5),
                rotation: Quat::IDENTITY,
            },
            turret(BLOCK_GUNSHIP_TURRET_IDS[0], IVec3::new(-1, 2, -2)),
            turret(BLOCK_GUNSHIP_TURRET_IDS[1], IVec3::new(1, 2, -2)),
            turret(BLOCK_GUNSHIP_TURRET_IDS[2], IVec3::new(-1, 2, 1)),
            turret(BLOCK_GUNSHIP_TURRET_IDS[3], IVec3::new(1, 2, 1)),
            under_turret(BLOCK_GUNSHIP_TURRET_IDS[4], IVec3::new(-1, -2, 2)),
            under_turret(BLOCK_GUNSHIP_TURRET_IDS[5], IVec3::new(1, -2, 2)),
        ],
        style: ARMOURED_STYLE_ID,
    }
}

/// The scavenger: a shorter hull with an outrigger down one flank and a scrap
/// boom up the other, four mismatched bell drives where the yard could weld
/// them, and two turrets bolted on where they fitted rather than where they
/// cover. Deliberately asymmetric - the silhouette is the faction.
pub(super) fn salvage_raider() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -2), IVec3::new(3, 1, 6)),
            block(IVec3::new(-1, 1, -2), IVec3::new(3, 1, 3)),
            block(IVec3::new(-2, 0, -1), IVec3::new(1, 1, 3)),
            block(IVec3::new(2, 1, -1), IVec3::new(1, 1, 2)),
            vec![IVec3::new(0, 0, -3)],
        ]),
        specials: vec![
            cell_part(BLOCK_BRIDGE_SECTION_ID, CONTROLLER, IVec3::new(0, 1, -2)),
            cell_part("drive_port", THRUSTER, IVec3::new(-1, 0, 3)),
            cell_part("drive_center", THRUSTER, IVec3::new(0, 0, 3)),
            cell_part("drive_starboard", THRUSTER, IVec3::new(1, 0, 3)),
            cell_part("drive_outrigger", THRUSTER, IVec3::new(-2, 0, 1)),
            turret("pdc_dorsal", IVec3::new(0, 2, -1)),
            turret("pdc_boom", IVec3::new(2, 2, 0)),
        ],
        style: SALVAGE_STYLE_ID,
    }
}

/// A turret standing in `cell`, seated down onto the top face of the hull cell
/// below it. The mount's socket is a quarter cell off its centre, so the
/// turret sits that far into its own cell rather than floating in the middle
/// of it.
fn turret(id: &'static str, cell: IVec3) -> Special {
    Special {
        id,
        prototype: PDC,
        position: cell.as_vec3() + TURRET_SEAT,
        rotation: Quat::IDENTITY,
    }
}

/// The same turret hung under a hull cell, rolled over so its socket still
/// faces the plate it stands on.
fn under_turret(id: &'static str, cell: IVec3) -> Special {
    Special {
        id,
        prototype: PDC,
        position: cell.as_vec3() - TURRET_SEAT,
        rotation: Quat::from_rotation_z(std::f32::consts::PI),
    }
}

/// A special that occupies one whole cell, replacing the plate that would
/// otherwise be there.
fn cell_part(id: &'static str, prototype: &'static str, cell: IVec3) -> Special {
    Special {
        id,
        prototype,
        position: cell.as_vec3(),
        rotation: Quat::IDENTITY,
    }
}

impl BlockShip {
    /// The section list a catalog entry is built from: one plate per cell no
    /// special claimed, then the specials themselves.
    pub(super) fn sections(self) -> Vec<SpaceshipSectionConfig> {
        let claimed: HashSet<IVec3> = self
            .specials
            .iter()
            .filter_map(|part| {
                let rounded = part.position.round();
                part.position
                    .abs_diff_eq(rounded, 1e-5)
                    .then_some(rounded.as_ivec3())
            })
            .collect();

        let mut sections: Vec<_> = self
            .cells
            .into_iter()
            .filter(|cell| !claimed.contains(cell))
            .enumerate()
            .map(|(index, cell)| SpaceshipSectionConfig {
                id: format!("plate_{index}"),
                position: cell.as_vec3(),
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype(HULL.to_string()),
                modifications: vec![],
            })
            .collect();
        sections.extend(self.specials.iter().map(|part| SpaceshipSectionConfig {
            id: part.id.to_string(),
            position: part.position,
            rotation: part.rotation,
            source: SectionSource::Prototype(part.prototype.to_string()),
            modifications: vec![],
        }));
        sections
    }
}

/// A solid box of cells: `size` cells from `origin`, inclusive of the origin.
fn block(origin: IVec3, size: IVec3) -> Vec<IVec3> {
    (0..size.x)
        .flat_map(|x| {
            (0..size.y).flat_map(move |y| (0..size.z).map(move |z| origin + IVec3::new(x, y, z)))
        })
        .collect()
}

/// Every cell of every part, each one once and in first-seen order, so a hull
/// built from overlapping boxes stays a stable section list.
fn union(parts: Vec<Vec<IVec3>>) -> Vec<IVec3> {
    let mut seen = HashSet::new();
    parts
        .into_iter()
        .flatten()
        .filter(|cell| seen.insert(*cell))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Every id in a block hull is unique. A duplicate would be a section
    /// content silently addresses the wrong one of, and the two places that
    /// name a block section by id - the gauntlet's magazines and the duel's
    /// hardened bridge - would each hit whichever came first.
    #[test]
    fn every_block_ship_names_each_section_once() {
        for (name, ship) in [
            ("cutter", utility_cutter()),
            ("hauler", bulk_hauler()),
            ("gunship", patrol_gunship()),
            ("raider", salvage_raider()),
        ] {
            let sections = ship.sections();
            let ids: HashSet<_> = sections.iter().map(|section| section.id.as_str()).collect();
            assert_eq!(
                ids.len(),
                sections.len(),
                "'{name}' repeats a section id across {} sections",
                sections.len()
            );
        }
    }

    /// No two sections stand in the same cell. A special REPLACES the plate it
    /// lands on rather than sitting inside it, which is the one rule the cell
    /// grid has and the one a hand-authored hull breaks by moving a drive.
    #[test]
    fn no_two_block_sections_share_a_cell() {
        for (name, ship) in [
            ("cutter", utility_cutter()),
            ("hauler", bulk_hauler()),
            ("gunship", patrol_gunship()),
            ("raider", salvage_raider()),
        ] {
            let mut seats: HashMap<IVec3, &str> = HashMap::new();
            for section in &ship.sections() {
                let rounded = section.position.round();
                if !section.position.abs_diff_eq(rounded, 1e-5) {
                    continue;
                }
                if let Some(other) = seats.insert(rounded.as_ivec3(), &section.id) {
                    panic!("'{name}' seats '{}' on top of '{other}'", section.id);
                }
            }
        }
    }

    /// The gunship carries exactly the turrets content walks by name.
    #[test]
    fn the_gunship_carries_every_turret_content_addresses() {
        let sections = patrol_gunship().sections();
        for turret in BLOCK_GUNSHIP_TURRET_IDS {
            assert!(
                sections.iter().any(|section| section.id == turret),
                "the gunship is missing turret '{turret}'"
            );
        }
    }

    /// Every block ship carries the one bridge id content addresses.
    #[test]
    fn every_block_ship_carries_a_bridge() {
        for (name, ship) in [
            ("cutter", utility_cutter()),
            ("hauler", bulk_hauler()),
            ("gunship", patrol_gunship()),
            ("raider", salvage_raider()),
        ] {
            assert!(
                ship.sections()
                    .iter()
                    .any(|section| section.id == BLOCK_BRIDGE_SECTION_ID),
                "'{name}' has no '{BLOCK_BRIDGE_SECTION_ID}' section"
            );
        }
    }
}
