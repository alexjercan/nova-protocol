//! The base game's BLOCK fleet: hand-authored ships assembled from shipped
//! cube prototypes on a cell grid, wearing a derived skin.
//!
//! A block ship is a set of cells plus a handful of placed specials. It uses no
//! modelled part, so a hull is reproducible from this source alone.
//!
//! Every hull names the style its derived skin is clad with. See
//! `base_content::styles`.
//!
//! Cells are BUILD-GRID cells, the one authored coordinate that is not metric:
//! one cell is one world unit is 10 m.

use std::collections::HashSet;

use bevy::prelude::*;
use nova_scenario::prelude::{SectionSource, SpaceshipSectionConfig};
use nova_ship::prelude::{
    BASIC_CONTROLLER_SECTION_ID, BASIC_THRUSTER_SECTION_ID, DOCKING_PORT_SECTION_ID,
    LIGHT_HULL_SECTION_ID, PDC_KINETIC_TURRET_SECTION_ID, REINFORCED_HULL_SECTION_ID,
};

use crate::base_content::{
    sections::VECTOR_THRUSTER_SECTION_ID,
    styles::{ARMOURED_STYLE_ID, INDUSTRIAL_STYLE_ID, SALVAGE_STYLE_ID},
};

/// The drive section ids the block fleet repeats: a pair on the beam, or one
/// on the centreline. Content that patches a drive names these.
const DRIVE_PORT_SECTION_ID: &str = "drive_port";
const DRIVE_STARBOARD_SECTION_ID: &str = "drive_starboard";
const MAIN_DRIVE_SECTION_ID: &str = "main_drive";

/// How far a turret drops into its own cell to put its socket on the plate
/// below: the mount's one link point sits a quarter cell under its centre.
const TURRET_SEAT: Vec3 = Vec3::new(0.0, -0.25, 0.0);

/// The section id every block ship's main flight computer carries, so content
/// can harden or disable one bridge without knowing which hull it is on.
pub const BLOCK_BRIDGE_SECTION_ID: &str = "bridge";

/// The gunship's point-defense mounts, in the order they are placed. Content
/// that sets a magazine per turret walks this rather than naming six strings
/// it would have to keep in step with the hull.
pub const BLOCK_GUNSHIP_TURRET_IDS: [&str; 6] = [
    "pdc_forward_port",
    "pdc_forward_starboard",
    "pdc_aft_port",
    "pdc_aft_starboard",
    "pdc_ventral_port",
    "pdc_ventral_starboard",
];

/// The one point-defense mount the picket carries, so content that arms or
/// disarms it names a section rather than a hull.
pub(crate) const BLOCK_CLEANUP_TURRET_ID: &str = "pdc";
/// The port-flank docking collar the workship and both frame tenders carry.
/// Content that hardens, reads or shoots the hatch names this rather than the
/// hull it is bolted to.
pub const BLOCK_PORT_COLLAR_SECTION_ID: &str = "port_collar";

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

/// A block hull: its cells, its specials, the plate every unclaimed cell is
/// built from, and the style its skin wears.
pub(super) struct BlockShip {
    pub(super) cells: Vec<IVec3>,
    pub(super) specials: Vec<Special>,
    /// The prototype every plain cell takes.
    pub(super) plate: &'static str,
    pub(super) style: &'static str,
}

/// The small unarmed workboat: one hull layer, two sponsons, a dorsal cab and
/// a pair of bell drives. The shortest hull in the fleet.
pub(super) fn utility_cutter() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -3), IVec3::new(3, 1, 6)),
            block(IVec3::new(-2, 0, 0), IVec3::new(1, 1, 3)),
            block(IVec3::new(2, 0, 0), IVec3::new(1, 1, 3)),
            vec![IVec3::new(0, 0, -4), IVec3::new(0, 1, -1)],
        ]),
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -1),
            ),
            cell_part(
                DRIVE_PORT_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 2),
            ),
            cell_part(
                DRIVE_STARBOARD_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 0, 2),
            ),
        ],
        plate: REINFORCED_HULL_SECTION_ID,
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// The freight hull: a flat spine between two cargo shoulders, a stack of
/// containers amidships, and a square transom with one vectoring drive on it.
/// The widest hull in the fleet, so it reads as freight from any backdrop
/// camera angle.
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
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -2),
            ),
            Special {
                id: MAIN_DRIVE_SECTION_ID,
                prototype: VECTOR_THRUSTER_SECTION_ID,
                // Standing off the transom: the drive is three cells square
                // and two deep, so its forward face lands on the z = 3 layer.
                position: Vec3::new(0.0, 0.0, 4.5),
                rotation: Quat::IDENTITY,
            },
        ],
        plate: REINFORCED_HULL_SECTION_ID,
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// A low hull with an open cradle amidships, a raised forward module, an aft
/// deck over two drives, and a docking collar on the port flank.
///
/// The cradle is authored as absence: the cells the two bulwarks stand on,
/// and nothing between them. The collar stands off the flank rather than the
/// bow, so a docking approach comes in alongside.
pub(super) fn utility_workship() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -4), IVec3::new(3, 1, 8)),
            block(IVec3::new(-1, 1, -4), IVec3::new(3, 1, 3)),
            block(IVec3::new(-1, 1, 2), IVec3::new(3, 1, 2)),
            block(IVec3::new(-2, 0, -1), IVec3::new(1, 1, 4)),
            block(IVec3::new(2, 0, -1), IVec3::new(1, 1, 4)),
        ]),
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -3),
            ),
            cell_part(
                DRIVE_PORT_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 3),
            ),
            cell_part(
                DRIVE_STARBOARD_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 0, 3),
            ),
            flank_collar(BLOCK_PORT_COLLAR_SECTION_ID, IVec3::new(-2, 0, -2)),
        ],
        plate: REINFORCED_HULL_SECTION_ID,
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// The frame tender: a long keel under a cargo body, two open frame arches
/// standing over it on rails, a cab forward and a service stack aft, with the
/// same port-flank collar the workship carries.
///
/// The arches are hull cells, not dressing, so the bay between them stays
/// open and the load rides outside the hull volume.
pub(super) fn frame_tender() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -4), IVec3::new(3, 1, 9)),
            block(IVec3::new(-1, 1, -4), IVec3::new(3, 1, 2)),
            block(IVec3::new(-1, 1, -1), IVec3::new(3, 1, 4)),
            block(IVec3::new(-1, 1, 3), IVec3::new(3, 2, 2)),
            // The two arches, and the rails that make them one frame rather
            // than two hoops.
            block(IVec3::new(-2, 1, -1), IVec3::new(1, 2, 1)),
            block(IVec3::new(2, 1, -1), IVec3::new(1, 2, 1)),
            block(IVec3::new(-2, 1, 2), IVec3::new(1, 2, 1)),
            block(IVec3::new(2, 1, 2), IVec3::new(1, 2, 1)),
            block(IVec3::new(-1, 2, -1), IVec3::new(3, 1, 1)),
            block(IVec3::new(-1, 2, 2), IVec3::new(3, 1, 1)),
            block(IVec3::new(-2, 2, 0), IVec3::new(1, 1, 2)),
            block(IVec3::new(2, 2, 0), IVec3::new(1, 1, 2)),
        ]),
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -4),
            ),
            cell_part(
                MAIN_DRIVE_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(0, 0, 4),
            ),
            flank_collar(BLOCK_PORT_COLLAR_SECTION_ID, IVec3::new(-2, 0, -3)),
        ],
        plate: REINFORCED_HULL_SECTION_ID,
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// The frame tender with its stern removed.
///
/// The cell plan drops the drive, the transom under it, the service stack
/// above that, and the starboard half of the aft arch. Everything forward of
/// the tear matches `frame_tender` cell for cell, so the cab and the port
/// collar stay where a docking approach expects them.
pub(super) fn damaged_frame_tender() -> BlockShip {
    BlockShip {
        cells: union(vec![
            // Everything forward of the tear, identical to `frame_tender`.
            block(IVec3::new(-1, 0, -4), IVec3::new(3, 1, 6)),
            block(IVec3::new(-1, 1, -4), IVec3::new(3, 1, 2)),
            // The tear is authored a COLUMN at a time, not as a shorter hull:
            // the derived skin needs an uneven edge to clad a break. The port
            // side runs two cells further aft than the starboard one, and the
            // deck above them stops at a third station.
            block(IVec3::new(-1, 0, 2), IVec3::new(1, 1, 2)),
            block(IVec3::new(1, 0, 2), IVec3::new(1, 1, 1)),
            block(IVec3::new(-1, 1, -1), IVec3::new(2, 1, 4)),
            block(IVec3::new(1, 1, -1), IVec3::new(1, 1, 2)),
            // One plate of the service stack's deck, hanging off the port
            // quarter with nothing around it.
            block(IVec3::new(-1, 1, 3), IVec3::new(1, 1, 1)),
            // The forward arch, whole. The aft one keeps its port leg and one
            // cell of its top bar; the starboard leg, the rest of the bar and
            // the rail between the arches are gone.
            block(IVec3::new(-2, 1, -1), IVec3::new(1, 2, 1)),
            block(IVec3::new(2, 1, -1), IVec3::new(1, 2, 1)),
            block(IVec3::new(-2, 1, 2), IVec3::new(1, 2, 1)),
            block(IVec3::new(-1, 2, -1), IVec3::new(3, 1, 1)),
            block(IVec3::new(-1, 2, 2), IVec3::new(1, 1, 1)),
            block(IVec3::new(-2, 2, 0), IVec3::new(1, 1, 2)),
        ]),
        // No drive: the hull cannot manoeuvre, and the empty transom shows
        // it.
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -4),
            ),
            flank_collar(BLOCK_PORT_COLLAR_SECTION_ID, IVec3::new(-2, 0, -3)),
        ],
        plate: REINFORCED_HULL_SECTION_ID,
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// The armed patrol hull: a two-deck spine over a short ventral keel, stub
/// wings, a dorsal fin, one vectoring drive, and six point-defense mounts
/// covering both hemispheres. About one and a half cutters long.
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
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -1),
            ),
            cell_part(
                "control_aft",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, 1),
            ),
            Special {
                id: MAIN_DRIVE_SECTION_ID,
                prototype: VECTOR_THRUSTER_SECTION_ID,
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
        plate: REINFORCED_HULL_SECTION_ID,
        style: ARMOURED_STYLE_ID,
    }
}

/// A short hull with an outrigger down one flank, a boom up the other, four
/// drives, and two turrets. Its asymmetric backdrop silhouette stays distinct
/// from the gunship.
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
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -2),
            ),
            cell_part(
                DRIVE_PORT_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 3),
            ),
            cell_part(
                "drive_center",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(0, 0, 3),
            ),
            cell_part(
                DRIVE_STARBOARD_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 0, 3),
            ),
            cell_part(
                "drive_outrigger",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-2, 0, 1),
            ),
            turret("pdc_dorsal", IVec3::new(0, 2, -1)),
            turret("pdc_boom", IVec3::new(2, 2, 0)),
        ],
        plate: REINFORCED_HULL_SECTION_ID,
        style: SALVAGE_STYLE_ID,
    }
}

/// The armed picket: a low, symmetric hull with its one gun pushed onto the
/// nose face, where the hull cannot mask its forward arc.
pub(super) fn salvage_picket() -> BlockShip {
    salvage_craft(
        union(vec![
            block(IVec3::new(-1, 0, -3), IVec3::new(3, 1, 7)),
            block(IVec3::new(-2, 0, 0), IVec3::new(5, 1, 3)),
            vec![IVec3::new(0, 0, -4)],
        ]),
        vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, -1),
            ),
            cell_part(
                DRIVE_PORT_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 3),
            ),
            cell_part(
                DRIVE_STARBOARD_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 0, 3),
            ),
            // Rotate the mount's -Y base onto the nose's -Z face. The
            // half-cell mount then centres just outside it.
            part(
                BLOCK_CLEANUP_TURRET_ID,
                PDC_KINETIC_TURRET_SECTION_ID,
                Vec3::new(0.0, 0.0, -4.75),
                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            ),
        ],
    )
}

/// Loose plating: the small piece a debris field is mostly built from.
pub(super) fn carrier_wreck_plate() -> BlockShip {
    wreck(union(vec![
        block(IVec3::new(-1, 0, -2), IVec3::new(3, 1, 5)),
        vec![IVec3::new(1, 1, 0), IVec3::new(-1, 1, -1)],
    ]))
}

/// A hull of thin plating wearing the salvage skin.
fn salvage_craft(cells: Vec<IVec3>, specials: Vec<Special>) -> BlockShip {
    BlockShip {
        cells,
        specials,
        plate: LIGHT_HULL_SECTION_ID,
        style: SALVAGE_STYLE_ID,
    }
}

/// One debris piece: industrial plating with no computer, drive or gun, so it
/// never reports itself neutralized.
fn wreck(cells: Vec<IVec3>) -> BlockShip {
    BlockShip {
        cells,
        specials: vec![],
        plate: REINFORCED_HULL_SECTION_ID,
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// A special placed at an arbitrary pose: a drive standing off a transom, a
/// gun seated on a nose face, a bay sunk into a flank.
fn part(id: &'static str, prototype: &'static str, position: Vec3, rotation: Quat) -> Special {
    Special {
        id,
        prototype,
        position,
        rotation,
    }
}

/// A turret standing in `cell`, seated down onto the top face of the hull cell
/// below it. The mount's socket is a quarter cell off its centre, so the
/// turret sits that far into its own cell rather than floating in the middle
/// of it.
fn turret(id: &'static str, cell: IVec3) -> Special {
    Special {
        id,
        prototype: PDC_KINETIC_TURRET_SECTION_ID,
        position: cell.as_vec3() + TURRET_SEAT,
        rotation: Quat::IDENTITY,
    }
}

/// The same turret hung under a hull cell, rolled over so its socket still
/// faces the plate it stands on.
fn under_turret(id: &'static str, cell: IVec3) -> Special {
    Special {
        id,
        prototype: PDC_KINETIC_TURRET_SECTION_ID,
        position: cell.as_vec3() - TURRET_SEAT,
        rotation: Quat::from_rotation_z(std::f32::consts::PI),
    }
}

/// A docking collar standing off a hull's PORT flank, hatch outboard.
///
/// The port's one blind face is its own `-Z`, so a quarter turn about Y puts
/// the hatch on world `-X` and leaves the `+Z` socket facing the plate
/// inboard of it. `cell` is the collar's own cell, one step outboard of the
/// hull cell it mates to: a collar sunk into the skin would be a hatch that
/// opens into structure.
fn flank_collar(id: &'static str, cell: IVec3) -> Special {
    Special {
        id,
        prototype: DOCKING_PORT_SECTION_ID,
        position: cell.as_vec3(),
        rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
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
        let plate = self.plate;
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
                source: SectionSource::prototype(plate),
            })
            .collect();
        sections.extend(self.specials.iter().map(|part| SpaceshipSectionConfig {
            id: part.id.to_string(),
            position: part.position,
            rotation: part.rotation,
            source: SectionSource::prototype(part.prototype),
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

    /// Every hand-authored block hull, so a fleet-wide test cannot miss one.
    /// The wreck plate is included; `crewed` filters it out where a test
    /// needs a bridge.
    fn fleet() -> Vec<(&'static str, BlockShip)> {
        vec![
            ("cutter", utility_cutter()),
            ("hauler", bulk_hauler()),
            ("workship", utility_workship()),
            ("frame tender", frame_tender()),
            ("gunship", patrol_gunship()),
            ("raider", salvage_raider()),
            ("picket", salvage_picket()),
            ("wreck plate", carrier_wreck_plate()),
        ]
    }

    /// The hulls that carry a bridge: everything but the wreck fragments.
    fn crewed() -> Vec<(&'static str, BlockShip)> {
        fleet()
            .into_iter()
            .filter(|(name, _)| !name.starts_with("wreck"))
            .collect()
    }

    /// Every id in a block hull is unique. Content addresses a section by id,
    /// so a duplicate would resolve to whichever section came first.
    #[test]
    fn every_block_ship_names_each_section_once() {
        for (name, ship) in fleet() {
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

    /// No two sections stand in the same cell: a special REPLACES the plate
    /// it lands on rather than sitting inside it.
    #[test]
    fn no_two_block_sections_share_a_cell() {
        for (name, ship) in fleet() {
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

    /// The picket carries the ONE gun id content names.
    #[test]
    fn the_picket_carries_the_mount_content_names() {
        assert!(
            salvage_picket()
                .sections()
                .iter()
                .any(|section| section.id == BLOCK_CLEANUP_TURRET_ID),
            "the picket has no '{BLOCK_CLEANUP_TURRET_ID}' mount"
        );
    }

    /// A wreck fragment carries plain plating only: no computer, no drive and
    /// no gun, so it never flies, shoots or reports itself neutralized.
    #[test]
    fn a_wreck_fragment_carries_nothing_that_works() {
        for (name, ship) in fleet()
            .into_iter()
            .filter(|(name, _)| name.starts_with("wreck"))
        {
            for section in ship.sections() {
                assert!(
                    matches!(&section.source, SectionSource::Prototype { id, .. } if id == REINFORCED_HULL_SECTION_ID),
                    "'{name}' section '{}' is not plain plating",
                    section.id
                );
            }
        }
    }

    /// Every block ship carries the one bridge id content addresses.
    #[test]
    fn every_block_ship_carries_a_bridge() {
        for (name, ship) in crewed() {
            assert!(
                ship.sections()
                    .iter()
                    .any(|section| section.id == BLOCK_BRIDGE_SECTION_ID),
                "'{name}' has no '{BLOCK_BRIDGE_SECTION_ID}' section"
            );
        }
    }
}
