//! Cell-grid hull assembly for the development fixture fleet.
//!
//! A block hull is a set of BUILD-GRID cells plus a handful of placed
//! specials. One cell is one world unit is 10 m, and it is the one authored
//! coordinate that is not metric.
//!
//! A special whose position lands exactly on a cell REPLACES that cell; one
//! placed off the grid (a turret seated on a face, a drive standing off a
//! transom) is added beside it.

use bevy::prelude::*;
use nova_protocol::prelude::*;

use super::sections;

/// The two large drives. Both are base prototypes, but only the ids engine
/// code names live in `nova_ship`, so a fixture spells these itself.
const VECTOR_THRUSTER_SECTION_ID: &str = "vector_thruster_section";
const CAPITAL_THRUSTER_SECTION_ID: &str = "capital_thruster_section";

/// The style ids the fixture hulls wear. Base content owns the looks; a
/// fixture only names one.
const ARMOURED_STYLE: &str = "armoured";
const INDUSTRIAL_STYLE: &str = "industrial";
const SALVAGE_STYLE: &str = "salvage";

/// How far a turret drops into its own cell to put its socket on the plate
/// below: the mount's one link point sits a quarter cell under its centre.
const TURRET_SEAT: Vec3 = Vec3::new(0.0, -0.25, 0.0);

/// The section id every fixture hull's main flight computer carries, so a
/// scene can harden or disable one bridge without knowing which hull it is on.
pub const BRIDGE_SECTION_ID: &str = "bridge";

/// The warship fixture's two spinal lances, port then starboard.
pub const WARSHIP_RAILGUN_IDS: [&str; 2] = ["railgun_port", "railgun_starboard"];

/// The warship fixture's ten point-defense mounts: dorsal fore, dorsal
/// midships, dorsal aft, then the four ventral mounts.
pub const WARSHIP_TURRET_IDS: [&str; 10] = [
    "pdc_forward_port",
    "pdc_forward_starboard",
    "pdc_dorsal_port",
    "pdc_dorsal_starboard",
    "pdc_aft_port",
    "pdc_aft_starboard",
    "pdc_ventral_forward_port",
    "pdc_ventral_forward_starboard",
    "pdc_ventral_aft_port",
    "pdc_ventral_aft_starboard",
];

/// The warship fixture's six flank siege bays, port fore-to-aft then
/// starboard.
pub const WARSHIP_BAY_IDS: [&str; 6] = [
    "bay_port_forward",
    "bay_port_midships",
    "bay_port_aft",
    "bay_starboard_forward",
    "bay_starboard_midships",
    "bay_starboard_aft",
];

/// The single point-defense mount each armed salvage fixture carries.
pub const CLEANUP_TURRET_ID: &str = "pdc";
/// The cleanup leader fixture's flank bay.
pub const CLEANUP_BAY_ID: &str = "torpedo_bay";

/// One placed part that is not a plain hull cell.
#[derive(Clone)]
pub struct Special {
    id: &'static str,
    source: SectionSource,
    position: Vec3,
    rotation: Quat,
}

/// A block hull: its cells, its specials, the plate every unclaimed cell is
/// built from, and the style its skin wears.
pub struct BlockShip {
    cells: Vec<IVec3>,
    specials: Vec<Special>,
    plate: &'static str,
    style: &'static str,
}

impl BlockShip {
    /// The section list a design is built from: one plate per cell no special
    /// claimed, then the specials themselves.
    pub fn sections(self) -> Vec<SpaceshipSectionConfig> {
        let plate = self.plate;
        let claimed: std::collections::HashSet<IVec3> = self
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
        sections.extend(
            self.specials
                .into_iter()
                .map(|part| SpaceshipSectionConfig {
                    id: part.id.to_string(),
                    position: part.position,
                    rotation: part.rotation,
                    source: part.source,
                }),
        );
        sections
    }

    /// The whole design: sections plus the derived skin and the style it
    /// wears. A `ShipDesign` over a bare section list spawns with
    /// `skin: false` and renders as bare cells.
    pub fn design(self) -> ShipDesign {
        let style = self.style.to_string();
        ShipDesign {
            sections: self.sections(),
            presentation: ShipPresentationConfig {
                skin: true,
                style: Some(style),
                ..presentation()
            },
            ..Default::default()
        }
    }
}

/// The feedback cues a fixture hull speaks with, by the paths base content
/// ships them at. A fixture is not base content, so it names the resolved
/// bundle paths rather than `self://`.
fn presentation() -> ShipPresentationConfig {
    let sound = |name: &str| Some(AssetRef::from(format!("base/sounds/{name}.wav")));
    ShipPresentationConfig {
        collapse_sound: sound("destroy_ship"),
        lock_on_sound: sound("lock_on"),
        lock_off_sound: sound("lock_off"),
        radar_deny_sound: sound("radar_deny"),
        radar_retarget_sound: sound("radar_retarget"),
        safety_on_sound: sound("safety_on"),
        warn_lock_sound: sound("warn_lock"),
        ammo_dry_sound: sound("ammo_dry"),
        warn_hull_sound: sound("warn_hull"),
        rcs_loop_sound: sound("rcs_loop"),
        ..ShipPresentationConfig::default()
    }
}

/// The salvage raider: a shorter hull with an outrigger down one flank and a
/// scrap boom up the other, four bell drives, and two turrets bolted where
/// they fit rather than where they cover.
pub fn salvage_raider() -> BlockShip {
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
                BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -2),
            ),
            cell_part(
                "drive_port",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 3),
            ),
            cell_part(
                "drive_center",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(0, 0, 3),
            ),
            cell_part(
                "drive_starboard",
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
        style: SALVAGE_STYLE,
    }
}

/// The industrial carrier: the largest fixture hull, and the one the size
/// sweeps use as their upper bound.
///
/// An elongated refinery spine buried between two seven-deck cargo shoulders,
/// with a dorsal superstructure, a ventral keel and a broad transom carrying
/// two capital drives. Both shoulders are cut with a vertical berth: the port
/// berth is empty, and the starboard one holds a workboat that is CARRIER
/// STRUCTURE rather than a second ship - its thin axis points outboard, it
/// stands one cell proud of the shoulder, and two lugs join it back on.
pub fn industrial_carrier() -> BlockShip {
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
    // The berthed workboat, laid on its side in the starboard recess: width
    // stands vertical and its nose-to-stern axis is still the carrier's.
    cells.extend(
        berth_insert_cells()
            .into_iter()
            .map(|local| IVec3::new(6 + local.y, -local.x, local.z)),
    );
    cells.extend([IVec3::new(5, -1, 0), IVec3::new(5, 1, 0)]);

    let berthed = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    BlockShip {
        cells,
        specials: vec![
            cell_part(
                BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 6, -2),
            ),
            // Nine more computers down the spine and through both shoulders.
            // A hull this long turns on what its computers can ask of it, so
            // a carrier that has lost one still has the authority to fly.
            cell_part(
                "control_forward",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, -14),
            ),
            cell_part(
                "control_forward_mid",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, -8),
            ),
            cell_part(
                "control_midships",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 0),
            ),
            cell_part(
                "control_aft_mid",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 8),
            ),
            cell_part(
                "control_aft",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 15),
            ),
            cell_part(
                "control_port_forward",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(-1, 0, -11),
            ),
            cell_part(
                "control_starboard_forward",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(1, 0, -11),
            ),
            cell_part(
                "control_port_aft",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(-1, 0, 11),
            ),
            cell_part(
                "control_starboard_aft",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(1, 0, 11),
            ),
            part(
                "berth_drive_port",
                BASIC_THRUSTER_SECTION_ID,
                Vec3::new(6.0, 1.0, 2.0),
                berthed,
            ),
            part(
                "berth_drive_starboard",
                BASIC_THRUSTER_SECTION_ID,
                Vec3::new(6.0, -1.0, 2.0),
                berthed,
            ),
            part(
                "capital_drive_port",
                CAPITAL_THRUSTER_SECTION_ID,
                Vec3::new(-3.0, 0.0, 19.0),
                Quat::IDENTITY,
            ),
            part(
                "capital_drive_starboard",
                CAPITAL_THRUSTER_SECTION_ID,
                Vec3::new(3.0, 0.0, 19.0),
                Quat::IDENTITY,
            ),
        ],
        plate: REINFORCED_HULL_SECTION_ID,
        style: INDUSTRIAL_STYLE,
    }
}

/// The cell plan of the workboat welded into the carrier's starboard berth:
/// one hull layer, two sponsons, a dorsal cab and a bow spur. It is berth
/// structure, so it carries no parts of its own.
fn berth_insert_cells() -> Vec<IVec3> {
    union(vec![
        block(IVec3::new(-1, 0, -3), IVec3::new(3, 1, 6)),
        block(IVec3::new(-2, 0, 0), IVec3::new(1, 1, 3)),
        block(IVec3::new(2, 0, 0), IVec3::new(1, 1, 3)),
        vec![IVec3::new(0, 0, -4), IVec3::new(0, 1, -1)],
    ])
}

/// The capital warship: a long five-wide fighting spine with the width saved
/// for its engine transom, two spinal siege lances embedded in the prow, three
/// flush siege bays down each flank, and ten point-defense mounts covering
/// both hemispheres.
///
/// Each weapon volume is CARVED from the hull so the muzzles sit flush with
/// the skin instead of hanging off it. Its two weapon prototypes are authored
/// INLINE (see [`sections`]): they are fixture ordnance, deliberately outside
/// the balanced catalog, so nothing in the section drawer offers them.
pub fn stolen_warship() -> BlockShip {
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
            cell_part(
                BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 2, 3),
            ),
            cell_part(
                "control_bow",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, -5),
            ),
            cell_part(
                "control_fore",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, -2),
            ),
            cell_part(
                "control_forward",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 0),
            ),
            cell_part(
                "control_forward_mid",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 3),
            ),
            cell_part(
                "control_midships",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 6),
            ),
            cell_part(
                "control_mid_aft",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 8),
            ),
            cell_part(
                "control_aft_mid",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 10),
            ),
            cell_part(
                "control_aft",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 12),
            ),
            cell_part(
                "control_stern",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 13),
            ),
            part(
                "drive_port",
                VECTOR_THRUSTER_SECTION_ID,
                Vec3::new(-2.0, 0.0, 14.5),
                Quat::IDENTITY,
            ),
            part(
                "drive_starboard",
                VECTOR_THRUSTER_SECTION_ID,
                Vec3::new(2.0, 0.0, 14.5),
                Quat::IDENTITY,
            ),
            siege_bay(WARSHIP_BAY_IDS[0], -1.5, -2.0, yaw),
            siege_bay(WARSHIP_BAY_IDS[1], -1.5, 2.0, yaw),
            siege_bay(WARSHIP_BAY_IDS[2], -1.5, 6.0, yaw),
            siege_bay(WARSHIP_BAY_IDS[3], 1.5, -2.0, -yaw),
            siege_bay(WARSHIP_BAY_IDS[4], 1.5, 2.0, -yaw),
            siege_bay(WARSHIP_BAY_IDS[5], 1.5, 6.0, -yaw),
            siege_lance(WARSHIP_RAILGUN_IDS[0], Vec3::new(-1.0, 0.0, -5.0)),
            siege_lance(WARSHIP_RAILGUN_IDS[1], Vec3::new(1.0, 0.0, -5.0)),
            turret(WARSHIP_TURRET_IDS[0], IVec3::new(-2, 2, 1)),
            turret(WARSHIP_TURRET_IDS[1], IVec3::new(2, 2, 1)),
            turret(WARSHIP_TURRET_IDS[2], IVec3::new(-1, 3, 4)),
            turret(WARSHIP_TURRET_IDS[3], IVec3::new(1, 3, 4)),
            turret(WARSHIP_TURRET_IDS[4], IVec3::new(-2, 2, 10)),
            turret(WARSHIP_TURRET_IDS[5], IVec3::new(2, 2, 10)),
            under_turret(WARSHIP_TURRET_IDS[6], IVec3::new(-2, -2, 2)),
            under_turret(WARSHIP_TURRET_IDS[7], IVec3::new(2, -2, 2)),
            under_turret(WARSHIP_TURRET_IDS[8], IVec3::new(-2, -2, 10)),
            under_turret(WARSHIP_TURRET_IDS[9], IVec3::new(2, -2, 10)),
        ],
        plate: REINFORCED_HULL_SECTION_ID,
        style: ARMOURED_STYLE,
    }
}

/// The unarmed salvage skiff: a narrow sensor prow reaching ahead of two
/// exposed machinery shoulders. The smallest fixture hull, and the lower bound
/// of the size sweeps.
pub fn salvage_skiff() -> BlockShip {
    salvage_craft(
        union(vec![
            block(IVec3::new(0, 0, -4), IVec3::new(1, 1, 8)),
            block(IVec3::new(-1, 0, -1), IVec3::new(3, 1, 4)),
            vec![IVec3::new(-2, 0, 1), IVec3::new(2, 0, 1)],
        ]),
        vec![
            cell_part(
                BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -1),
            ),
            cell_part(
                "drive_port",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 3),
            ),
            cell_part(
                "drive_starboard",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 0, 3),
            ),
        ],
    )
}

/// The unarmed salvage tug: twin recovery booms on a broad drive crossbar.
pub fn salvage_tug() -> BlockShip {
    salvage_craft(
        union(vec![
            block(IVec3::new(-2, 0, 1), IVec3::new(5, 1, 3)),
            block(IVec3::new(-2, 0, -4), IVec3::new(2, 1, 5)),
            block(IVec3::new(1, 0, -4), IVec3::new(2, 1, 5)),
            block(IVec3::new(-1, 1, 1), IVec3::new(3, 1, 2)),
        ]),
        vec![
            cell_part(
                BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, 1),
            ),
            cell_part(
                "drive_port",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-2, 0, 3),
            ),
            cell_part(
                "drive_starboard",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(2, 0, 3),
            ),
        ],
    )
}

/// The armed salvage claw: a port machinery pod against a long starboard
/// grapple arm, with the gun riding the arm.
pub fn salvage_claw() -> BlockShip {
    salvage_craft(
        union(vec![
            block(IVec3::new(0, 0, -4), IVec3::new(1, 1, 9)),
            block(IVec3::new(-2, 0, -1), IVec3::new(2, 1, 5)),
            block(IVec3::new(1, 0, -2), IVec3::new(3, 1, 1)),
            vec![IVec3::new(3, 0, -3), IVec3::new(-1, 1, 1)],
        ]),
        vec![
            cell_part(
                BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(-1, 1, 1),
            ),
            cell_part(
                "drive_spine",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(0, 0, 4),
            ),
            cell_part("drive_pod", BASIC_THRUSTER_SECTION_ID, IVec3::new(-2, 0, 4)),
            cell_part(
                "drive_grapple",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(3, 0, -1),
            ),
            turret(CLEANUP_TURRET_ID, IVec3::new(3, 1, -2)),
        ],
    )
}

/// The cleanup leader: a heavier salvage hull with one dorsal gun, one flank
/// Serpent bay and a vectoring drive.
pub fn salvage_leader() -> BlockShip {
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
            cell_part(
                BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -1),
            ),
            cell_part(
                "control_aft",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, 2),
            ),
            part(
                "main_drive",
                VECTOR_THRUSTER_SECTION_ID,
                Vec3::new(0.0, 0.0, 5.5),
                Quat::IDENTITY,
            ),
            turret(CLEANUP_TURRET_ID, IVec3::new(0, 2, 1)),
            part(
                CLEANUP_BAY_ID,
                TORPEDO_SECTION_ID,
                Vec3::new(-1.5, 0.0, 0.0),
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            ),
        ],
    )
}

/// One scavenged craft: thin plating and the salvage skin over an authored
/// cell plan.
fn salvage_craft(cells: Vec<IVec3>, specials: Vec<Special>) -> BlockShip {
    BlockShip {
        cells,
        specials,
        plate: LIGHT_HULL_SECTION_ID,
        style: SALVAGE_STYLE,
    }
}

/// A siege torpedo bay sunk flush into a flank, its muzzle facing outboard.
/// The prototype is authored inline, so the bay travels with the fixture.
fn siege_bay(id: &'static str, x: f32, z: f32, yaw: f32) -> Special {
    Special {
        id,
        source: SectionSource::Inline(sections::siege_torpedo_bay()),
        position: Vec3::new(x, 0.0, z),
        rotation: Quat::from_rotation_y(yaw),
    }
}

/// One spinal siege lance embedded in the prow, authored inline for the same
/// reason the bay is.
fn siege_lance(id: &'static str, position: Vec3) -> Special {
    Special {
        id,
        source: SectionSource::Inline(sections::siege_railgun_lance()),
        position,
        rotation: Quat::IDENTITY,
    }
}

/// A special placed at an arbitrary pose: a drive standing off a transom, a
/// gun seated on a nose face, a bay sunk into a flank.
fn part(id: &'static str, prototype: &'static str, position: Vec3, rotation: Quat) -> Special {
    Special {
        id,
        source: SectionSource::prototype(prototype),
        position,
        rotation,
    }
}

/// A turret standing in `cell`, seated down onto the top face of the hull cell
/// below it.
fn turret(id: &'static str, cell: IVec3) -> Special {
    part(
        id,
        PDC_KINETIC_TURRET_SECTION_ID,
        cell.as_vec3() + TURRET_SEAT,
        Quat::IDENTITY,
    )
}

/// The same turret hung under a hull cell, rolled over so its socket still
/// faces the plate it stands on.
fn under_turret(id: &'static str, cell: IVec3) -> Special {
    part(
        id,
        PDC_KINETIC_TURRET_SECTION_ID,
        cell.as_vec3() - TURRET_SEAT,
        Quat::from_rotation_z(std::f32::consts::PI),
    )
}

/// A special that occupies one whole cell, replacing the plate that would
/// otherwise be there.
fn cell_part(id: &'static str, prototype: &'static str, cell: IVec3) -> Special {
    part(id, prototype, cell.as_vec3(), Quat::IDENTITY)
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
    let mut seen = std::collections::HashSet::new();
    parts
        .into_iter()
        .flatten()
        .filter(|cell| seen.insert(*cell))
        .collect()
}
