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

use crate::base_content::{
    sections,
    styles::{ARMOURED_STYLE_ID, INDUSTRIAL_STYLE_ID, SALVAGE_STYLE_ID},
};

/// The plain structural cell every block hull is mostly made of.
const HULL: &str = "reinforced_hull_section";
/// The thin plate the scavenged fleet is built from: the same cell, at the
/// grade a yard that welds what it finds can afford.
const LIGHT_HULL: &str = "light_hull_section";
const CONTROLLER: &str = "basic_controller_section";
const THRUSTER: &str = "basic_thruster_section";
const VECTOR_THRUSTER: &str = "vector_thruster_section";
const CAPITAL_THRUSTER: &str = "capital_thruster_section";
const PDC: &str = "pdc_kinetic_turret_section";
const TORPEDO: &str = "torpedo_section";
const SIEGE_TORPEDO: &str = "heavy_torpedo_section";
/// The capital-grade lance, mounted by exactly one hull in the fleet: the
/// stolen warship's two spinal guns. A separate PROTOTYPE rather than a
/// per-spawn override, so the standard lance every other ship carries is
/// untouched and the heavy one is a thing you can see in the catalog.
const SIEGE_RAILGUN: &str = sections::SIEGE_RAILGUN_LANCE_SECTION_ID;

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

/// The stolen warship's two spinal lances, port then starboard. The campaign
/// fires them by name, one deliberate shot at a time.
pub(crate) const BLOCK_WARSHIP_RAILGUN_IDS: [&str; 2] = ["railgun_port", "railgun_starboard"];

/// The stolen warship's six flank siege bays, port fore-to-aft then starboard.
pub(crate) const BLOCK_WARSHIP_BAY_IDS: [&str; 6] = [
    "bay_port_forward",
    "bay_port_midships",
    "bay_port_aft",
    "bay_starboard_forward",
    "bay_starboard_midships",
    "bay_starboard_aft",
];

/// The single point-defense mount each armed cleanup craft carries, and the
/// cleanup leader's one torpedo bay. One id apiece, so content that arms or
/// disarms the search group names a section rather than a hull.
pub(crate) const BLOCK_CLEANUP_TURRET_ID: &str = "pdc";
/// The cleanup leader's flank bay.
pub(crate) const BLOCK_CLEANUP_BAY_ID: &str = "torpedo_bay";

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
    /// The prototype every plain cell takes. Reinforced for a working or
    /// military hull; light for the scavenged fleet, which is thin on purpose.
    pub(super) plate: &'static str,
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
        plate: HULL,
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
        plate: HULL,
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
        plate: HULL,
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
        plate: HULL,
        style: SALVAGE_STYLE_ID,
    }
}

/// The campaign's home: an Earth industrial carrier, and by a wide margin the
/// largest thing the base game spawns.
///
/// An elongated refinery spine buried between two seven-deck cargo shoulders,
/// with a dorsal superstructure, a ventral keel and a broad transom carrying
/// two capital drives. Both shoulders are cut with a vertical cutter berth: the
/// port berth is empty, and the starboard one holds a cutter that is CARRIER
/// STRUCTURE rather than a second ship - its thin axis points outboard, it
/// stands one cell proud of the shoulder, and two docking lugs join it back on.
///
/// The scale is the point. At thirty-three cells long against the cutter's ten
/// it reads as the thing the player's boat is carried BY, so losing it is a
/// place, not a set piece prop.
pub(super) fn industrial_carrier() -> BlockShip {
    let mut cells = union(vec![
        block(IVec3::new(-2, -1, -16), IVec3::new(5, 3, 33)),
        block(IVec3::new(-5, -3, -11), IVec3::new(3, 7, 23)),
        block(IVec3::new(3, -3, -11), IVec3::new(3, 7, 23)),
        block(IVec3::new(-3, 2, -7), IVec3::new(7, 1, 15)),
        block(IVec3::new(-2, 3, -9), IVec3::new(5, 2, 19)),
        block(IVec3::new(-1, 5, -5), IVec3::new(3, 2, 11)),
        block(IVec3::new(-1, -3, -9), IVec3::new(3, 2, 19)),
        block(IVec3::new(-5, -2, 13), IVec3::new(11, 5, 5)),
    ]);
    cells.retain(|cell| {
        !(cell.x.abs() == 5 && (-2..=2).contains(&cell.y) && (-4..=2).contains(&cell.z))
    });
    // The berthed cutter, laid on its side in the starboard recess: the
    // workboat's own cell plan, with width standing vertical and its
    // nose-to-stern axis still the carrier's.
    cells.extend(
        utility_cutter()
            .cells
            .into_iter()
            .map(|local| IVec3::new(6 + local.y, -local.x, local.z)),
    );
    cells.extend([IVec3::new(5, -1, 0), IVec3::new(5, 1, 0)]);

    let berthed = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    BlockShip {
        cells,
        specials: vec![
            cell_part(BLOCK_BRIDGE_SECTION_ID, CONTROLLER, IVec3::new(0, 6, -2)),
            // Nine more computers down the spine and through both shoulders. A
            // hull this long turns on what its computers can ask of it, and a
            // ship the scenario flies by order needs the authority to still be
            // there after the first shot lands.
            cell_part("control_forward", CONTROLLER, IVec3::new(0, 0, -14)),
            cell_part("control_forward_mid", CONTROLLER, IVec3::new(0, 0, -8)),
            cell_part("control_midships", CONTROLLER, IVec3::new(0, 0, 0)),
            cell_part("control_aft_mid", CONTROLLER, IVec3::new(0, 0, 8)),
            cell_part("control_aft", CONTROLLER, IVec3::new(0, 0, 15)),
            cell_part("control_port_forward", CONTROLLER, IVec3::new(-1, 0, -11)),
            cell_part(
                "control_starboard_forward",
                CONTROLLER,
                IVec3::new(1, 0, -11),
            ),
            cell_part("control_port_aft", CONTROLLER, IVec3::new(-1, 0, 11)),
            cell_part("control_starboard_aft", CONTROLLER, IVec3::new(1, 0, 11)),
            part(
                "berth_cutter_drive_port",
                THRUSTER,
                Vec3::new(6.0, 1.0, 2.0),
                berthed,
            ),
            part(
                "berth_cutter_drive_starboard",
                THRUSTER,
                Vec3::new(6.0, -1.0, 2.0),
                berthed,
            ),
            part(
                "capital_drive_port",
                CAPITAL_THRUSTER,
                Vec3::new(-3.0, 0.0, 19.0),
                Quat::IDENTITY,
            ),
            part(
                "capital_drive_starboard",
                CAPITAL_THRUSTER,
                Vec3::new(3.0, 0.0, 19.0),
                Quat::IDENTITY,
            ),
        ],
        plate: HULL,
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// The stolen Earth warship: a long five-wide fighting spine with the width
/// saved for its engine transom, two spinal lances embedded in the prow, three
/// flush siege bays down each flank, and ten point-defense mounts covering both
/// hemispheres.
///
/// It is the only capital combatant in the base fleet and it is deliberately
/// out of the player's league - the campaign's opening exists to be watched,
/// not fought. Each weapon volume is CARVED from the hull so the muzzles sit
/// flush with the skin instead of hanging off it.
pub(super) fn stolen_warship() -> BlockShip {
    let mut cells = union(vec![
        block(IVec3::new(-2, -1, 0), IVec3::new(5, 3, 14)),
        block(IVec3::new(-2, -1, -6), IVec3::new(5, 3, 6)),
        block(IVec3::new(-1, 2, 2), IVec3::new(3, 1, 7)),
        block(IVec3::new(-3, -1, 11), IVec3::new(1, 3, 3)),
        block(IVec3::new(3, -1, 11), IVec3::new(1, 3, 3)),
    ]);
    cells.retain(|cell| {
        let bay = cell.y == 0 && [-2, 2, 6].contains(&cell.z) && matches!(cell.x.abs(), 1 | 2);
        let lance = cell.y == 0 && (-6..=-4).contains(&cell.z) && cell.x.abs() == 1;
        !bay && !lance
    });

    let yaw = std::f32::consts::FRAC_PI_2;
    BlockShip {
        cells,
        specials: vec![
            cell_part(BLOCK_BRIDGE_SECTION_ID, CONTROLLER, IVec3::new(0, 2, 3)),
            cell_part("control_bow", CONTROLLER, IVec3::new(0, 0, -5)),
            cell_part("control_fore", CONTROLLER, IVec3::new(0, 0, -2)),
            cell_part("control_forward", CONTROLLER, IVec3::new(0, 0, 0)),
            cell_part("control_forward_mid", CONTROLLER, IVec3::new(0, 0, 3)),
            cell_part("control_midships", CONTROLLER, IVec3::new(0, 0, 6)),
            cell_part("control_mid_aft", CONTROLLER, IVec3::new(0, 0, 8)),
            cell_part("control_aft_mid", CONTROLLER, IVec3::new(0, 0, 10)),
            cell_part("control_aft", CONTROLLER, IVec3::new(0, 0, 12)),
            cell_part("control_stern", CONTROLLER, IVec3::new(0, 0, 13)),
            part(
                "drive_port",
                VECTOR_THRUSTER,
                Vec3::new(-2.0, 0.0, 14.5),
                Quat::IDENTITY,
            ),
            part(
                "drive_starboard",
                VECTOR_THRUSTER,
                Vec3::new(2.0, 0.0, 14.5),
                Quat::IDENTITY,
            ),
            flank_bay(BLOCK_WARSHIP_BAY_IDS[0], -1.5, -2.0, yaw),
            flank_bay(BLOCK_WARSHIP_BAY_IDS[1], -1.5, 2.0, yaw),
            flank_bay(BLOCK_WARSHIP_BAY_IDS[2], -1.5, 6.0, yaw),
            flank_bay(BLOCK_WARSHIP_BAY_IDS[3], 1.5, -2.0, -yaw),
            flank_bay(BLOCK_WARSHIP_BAY_IDS[4], 1.5, 2.0, -yaw),
            flank_bay(BLOCK_WARSHIP_BAY_IDS[5], 1.5, 6.0, -yaw),
            part(
                BLOCK_WARSHIP_RAILGUN_IDS[0],
                SIEGE_RAILGUN,
                Vec3::new(-1.0, 0.0, -5.0),
                Quat::IDENTITY,
            ),
            part(
                BLOCK_WARSHIP_RAILGUN_IDS[1],
                SIEGE_RAILGUN,
                Vec3::new(1.0, 0.0, -5.0),
                Quat::IDENTITY,
            ),
            turret("pdc_forward_port", IVec3::new(-2, 2, 1)),
            turret("pdc_forward_starboard", IVec3::new(2, 2, 1)),
            turret("pdc_dorsal_port", IVec3::new(-1, 3, 4)),
            turret("pdc_dorsal_starboard", IVec3::new(1, 3, 4)),
            turret("pdc_aft_port", IVec3::new(-2, 2, 10)),
            turret("pdc_aft_starboard", IVec3::new(2, 2, 10)),
            under_turret("pdc_ventral_forward_port", IVec3::new(-2, -2, 2)),
            under_turret("pdc_ventral_forward_starboard", IVec3::new(2, -2, 2)),
            under_turret("pdc_ventral_aft_port", IVec3::new(-2, -2, 10)),
            under_turret("pdc_ventral_aft_starboard", IVec3::new(2, -2, 10)),
        ],
        plate: HULL,
        style: ARMOURED_STYLE_ID,
    }
}

/// The cleanup group's unarmed needle: a narrow sensor prow reaching ahead of
/// two exposed machinery shoulders. It searches; it cannot answer.
pub(super) fn salvage_skiff() -> BlockShip {
    salvage_craft(
        union(vec![
            block(IVec3::new(0, 0, -4), IVec3::new(1, 1, 8)),
            block(IVec3::new(-1, 0, -1), IVec3::new(3, 1, 4)),
            vec![IVec3::new(-2, 0, 1), IVec3::new(2, 0, 1)],
        ]),
        vec![
            cell_part(BLOCK_BRIDGE_SECTION_ID, CONTROLLER, IVec3::new(0, 1, -1)),
            cell_part("drive_port", THRUSTER, IVec3::new(-1, 0, 3)),
            cell_part("drive_starboard", THRUSTER, IVec3::new(1, 0, 3)),
        ],
    )
}

/// The cleanup group's unarmed fork tug: twin recovery booms on a broad drive
/// crossbar. The one that carries away what the search finds.
pub(super) fn salvage_tug() -> BlockShip {
    salvage_craft(
        union(vec![
            block(IVec3::new(-2, 0, 1), IVec3::new(5, 1, 3)),
            block(IVec3::new(-2, 0, -4), IVec3::new(2, 1, 5)),
            block(IVec3::new(1, 0, -4), IVec3::new(2, 1, 5)),
            block(IVec3::new(-1, 1, 1), IVec3::new(3, 1, 2)),
        ]),
        vec![
            cell_part(BLOCK_BRIDGE_SECTION_ID, CONTROLLER, IVec3::new(0, 1, 1)),
            cell_part("drive_port", THRUSTER, IVec3::new(-2, 0, 3)),
            cell_part("drive_starboard", THRUSTER, IVec3::new(2, 0, 3)),
        ],
    )
}

/// The cleanup group's armed picket: a low, balanced hull with its one gun
/// pushed onto the nose face, where the hull cannot mask its forward arc.
pub(super) fn salvage_picket() -> BlockShip {
    salvage_craft(
        union(vec![
            block(IVec3::new(-1, 0, -3), IVec3::new(3, 1, 7)),
            block(IVec3::new(-2, 0, 0), IVec3::new(5, 1, 3)),
            vec![IVec3::new(0, 0, -4)],
        ]),
        vec![
            cell_part(BLOCK_BRIDGE_SECTION_ID, CONTROLLER, IVec3::new(0, 0, -1)),
            cell_part("drive_port", THRUSTER, IVec3::new(-1, 0, 3)),
            cell_part("drive_starboard", THRUSTER, IVec3::new(1, 0, 3)),
            // Rotate the mount's -Y base onto the nose's -Z face. The
            // half-cell mount then centres just outside it.
            part(
                BLOCK_CLEANUP_TURRET_ID,
                PDC,
                Vec3::new(0.0, 0.0, -4.75),
                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            ),
        ],
    )
}

/// The cleanup group's armed claw: a port machinery pod against a long
/// starboard grapple arm, with the gun riding the arm. Asymmetric on purpose -
/// the silhouette is the faction.
pub(super) fn salvage_claw() -> BlockShip {
    salvage_craft(
        union(vec![
            block(IVec3::new(0, 0, -4), IVec3::new(1, 1, 9)),
            block(IVec3::new(-2, 0, -1), IVec3::new(2, 1, 5)),
            block(IVec3::new(1, 0, -2), IVec3::new(3, 1, 1)),
            vec![IVec3::new(3, 0, -3), IVec3::new(-1, 1, 1)],
        ]),
        vec![
            cell_part(BLOCK_BRIDGE_SECTION_ID, CONTROLLER, IVec3::new(-1, 1, 1)),
            cell_part("drive_spine", THRUSTER, IVec3::new(0, 0, 4)),
            cell_part("drive_pod", THRUSTER, IVec3::new(-2, 0, 4)),
            cell_part("drive_grapple", THRUSTER, IVec3::new(3, 0, -1)),
            turret(BLOCK_CLEANUP_TURRET_ID, IVec3::new(3, 1, -2)),
        ],
    )
}

/// The cleanup group's leader: a heavier salvage hull with one dorsal gun, one
/// flank Serpent bay and a vectoring drive. The only ordnance in the group, and
/// the reason an unarmed cutter runs instead of hiding.
pub(super) fn salvage_leader() -> BlockShip {
    let mut cells = union(vec![
        block(IVec3::new(-2, 0, -3), IVec3::new(5, 1, 8)),
        block(IVec3::new(-1, 0, -5), IVec3::new(3, 1, 2)),
        block(IVec3::new(-1, -1, 2), IVec3::new(3, 3, 3)),
        block(IVec3::new(-1, 1, -1), IVec3::new(3, 1, 3)),
    ]);
    // The bay's own volume, carved out of the port flank so its muzzle sits
    // flush with the skin.
    cells.retain(|cell| !(cell.y == 0 && cell.z == 0 && matches!(cell.x, -2 | -1)));
    salvage_craft(
        cells,
        vec![
            cell_part(BLOCK_BRIDGE_SECTION_ID, CONTROLLER, IVec3::new(0, 1, -1)),
            cell_part("control_aft", CONTROLLER, IVec3::new(0, 1, 2)),
            part(
                "main_drive",
                VECTOR_THRUSTER,
                Vec3::new(0.0, 0.0, 5.5),
                Quat::IDENTITY,
            ),
            turret(BLOCK_CLEANUP_TURRET_ID, IVec3::new(0, 2, 1)),
            part(
                BLOCK_CLEANUP_BAY_ID,
                TORPEDO,
                Vec3::new(-1.5, 0.0, 0.0),
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            ),
        ],
    )
}

/// The carrier's bridge tower, sheared off whole: the biggest recognizable
/// piece left, and the one the search starts at.
pub(super) fn carrier_wreck_bridge() -> BlockShip {
    wreck(union(vec![
        block(IVec3::new(-2, -1, -3), IVec3::new(5, 3, 7)),
        block(IVec3::new(-1, 2, -1), IVec3::new(3, 2, 4)),
        block(IVec3::new(0, 4, 0), IVec3::new(1, 2, 2)),
    ]))
}

/// A length of the refinery spine, open at both ends.
pub(super) fn carrier_wreck_spine() -> BlockShip {
    wreck(union(vec![
        block(IVec3::new(-1, -1, -4), IVec3::new(3, 3, 7)),
        block(IVec3::new(-3, 0, 1), IVec3::new(7, 1, 3)),
    ]))
}

/// A cargo shoulder, torn along the deck it was welded to.
pub(super) fn carrier_wreck_shoulder() -> BlockShip {
    wreck(union(vec![
        block(IVec3::new(-3, -1, -2), IVec3::new(7, 2, 5)),
        block(IVec3::new(-1, 1, -1), IVec3::new(3, 2, 3)),
    ]))
}

/// Loose plating: the small pieces, and most of what a debris field is.
pub(super) fn carrier_wreck_plate() -> BlockShip {
    wreck(union(vec![
        block(IVec3::new(-1, 0, -2), IVec3::new(3, 1, 5)),
        vec![IVec3::new(1, 1, 0), IVec3::new(-1, 1, -1)],
    ]))
}

/// One scavenged craft: the salvage fleet's thin plating and scavenged skin
/// over an authored cell plan.
fn salvage_craft(cells: Vec<IVec3>, specials: Vec<Special>) -> BlockShip {
    BlockShip {
        cells,
        specials,
        plate: LIGHT_HULL,
        style: SALVAGE_STYLE_ID,
    }
}

/// One piece of the dead carrier: industrial plating with nothing left that
/// works - no computer, no drive, no gun. It cannot be neutralized because it
/// was never a combatant; it is scenery with a collider.
fn wreck(cells: Vec<IVec3>) -> BlockShip {
    BlockShip {
        cells,
        specials: vec![],
        plate: HULL,
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// A torpedo bay sunk flush into a flank, its muzzle facing outboard.
fn flank_bay(id: &'static str, x: f32, z: f32, yaw: f32) -> Special {
    Special {
        id,
        prototype: SIEGE_TORPEDO,
        position: Vec3::new(x, 0.0, z),
        rotation: Quat::from_rotation_y(yaw),
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
                source: SectionSource::Prototype(plate.to_string()),
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

    /// Every hand-authored block hull, so a test that must hold for the whole
    /// fleet cannot silently miss the newest ship. A wreck fragment is in here
    /// too: it carries no bridge, which is exactly why its exclusion is
    /// written down once rather than per test.
    fn fleet() -> Vec<(&'static str, BlockShip)> {
        vec![
            ("cutter", utility_cutter()),
            ("hauler", bulk_hauler()),
            ("gunship", patrol_gunship()),
            ("raider", salvage_raider()),
            ("carrier", industrial_carrier()),
            ("warship", stolen_warship()),
            ("skiff", salvage_skiff()),
            ("tug", salvage_tug()),
            ("picket", salvage_picket()),
            ("claw", salvage_claw()),
            ("cleanup leader", salvage_leader()),
            ("wreck bridge", carrier_wreck_bridge()),
            ("wreck spine", carrier_wreck_spine()),
            ("wreck shoulder", carrier_wreck_shoulder()),
            ("wreck plate", carrier_wreck_plate()),
        ]
    }

    /// The hulls with a crew: everything but the carrier's dead fragments,
    /// which have no computer, no drive and no gun by design.
    fn crewed() -> Vec<(&'static str, BlockShip)> {
        fleet()
            .into_iter()
            .filter(|(name, _)| !name.starts_with("wreck"))
            .collect()
    }

    /// Every id in a block hull is unique. A duplicate would be a section
    /// content silently addresses the wrong one of, and the two places that
    /// name a block section by id - the gauntlet's magazines and the duel's
    /// hardened bridge - would each hit whichever came first.
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

    /// No two sections stand in the same cell. A special REPLACES the plate it
    /// lands on rather than sitting inside it, which is the one rule the cell
    /// grid has and the one a hand-authored hull breaks by moving a drive.
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

    /// The campaign fires the warship's lances and bays BY ID, one at a time,
    /// so a rename here would silently turn the opening set piece into a ship
    /// sitting still with its guns cold.
    #[test]
    fn the_warship_carries_every_weapon_the_opening_fires_by_name() {
        let sections = stolen_warship().sections();
        for weapon in BLOCK_WARSHIP_RAILGUN_IDS
            .iter()
            .chain(BLOCK_WARSHIP_BAY_IDS.iter())
        {
            assert!(
                sections.iter().any(|section| section.id == *weapon),
                "the warship is missing '{weapon}'"
            );
        }
    }

    /// The heavy lance is the STOLEN WARSHIP's, and nothing else in the fleet
    /// carries one. The opening set piece needs a gun that opens a carrier in
    /// one shot; every other lance in the game is the catalog's standard one,
    /// and this is what keeps those two facts from drifting into each other.
    #[test]
    fn only_the_stolen_warship_mounts_the_siege_lance() {
        for (name, ship) in fleet() {
            let siege = ship
                .sections()
                .into_iter()
                .filter(|section| {
                    matches!(&section.source, SectionSource::Prototype(id) if id == SIEGE_RAILGUN)
                })
                .count();
            let expected = if name == "warship" {
                BLOCK_WARSHIP_RAILGUN_IDS.len()
            } else {
                0
            };
            assert_eq!(
                siege, expected,
                "'{name}' mounts {siege} siege lances, expected {expected}"
            );
        }
    }

    /// The three armed cleanup craft each carry the ONE gun id content names,
    /// and the leader carries the group's only bay.
    #[test]
    fn every_armed_cleanup_craft_carries_the_mount_content_names() {
        for (name, ship) in [
            ("picket", salvage_picket()),
            ("claw", salvage_claw()),
            ("cleanup leader", salvage_leader()),
        ] {
            let sections = ship.sections();
            assert!(
                sections
                    .iter()
                    .any(|section| section.id == BLOCK_CLEANUP_TURRET_ID),
                "'{name}' has no '{BLOCK_CLEANUP_TURRET_ID}' mount"
            );
        }
        assert!(
            salvage_leader()
                .sections()
                .iter()
                .any(|section| section.id == BLOCK_CLEANUP_BAY_ID),
            "the cleanup leader has no '{BLOCK_CLEANUP_BAY_ID}'"
        );
    }

    /// A wreck fragment is scenery: no computer, no drive, no gun. If one grew
    /// a working part it would start flying, shooting, or reporting itself
    /// neutralized in the middle of a search.
    #[test]
    fn a_carrier_wreck_fragment_carries_nothing_that_works() {
        for (name, ship) in fleet()
            .into_iter()
            .filter(|(name, _)| name.starts_with("wreck"))
        {
            for section in ship.sections() {
                assert!(
                    matches!(&section.source, SectionSource::Prototype(id) if id == HULL),
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
