//! Shared hand-authored structures for the First Shift visual benches.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use nova_protocol::prelude::*;

const HULL: &str = "reinforced_hull_section";
const CONTROLLER: &str = "basic_controller_section";
const THRUSTER: &str = "basic_thruster_section";
const VECTOR_THRUSTER: &str = "vector_thruster_section";
const CAPITAL_THRUSTER: &str = "capital_thruster_section";
const TORPEDO: &str = "heavy_torpedo_section";
const SALVAGE_TORPEDO: &str = "torpedo_section";
const PDC: &str = "pdc_kinetic_turret_section";
const RAILGUN: &str = "railgun_lance_section";
const LIGHT_HULL: &str = "light_hull_section";

const INDUSTRIAL: &str = "industrial";
const SALVAGE: &str = "salvage";
const ARMOURED: &str = "armoured";

#[derive(Clone, Copy)]
struct Special {
    id: &'static str,
    prototype: &'static str,
    position: Vec3,
    rotation: Quat,
}

pub fn maintenance_cutter() -> ShipHull {
    let cells = maintenance_cutter_cells();
    grid_hull(
        cells,
        &[
            special("controller", CONTROLLER, IVec3::new(0, 1, -1)),
            special("drive_port", THRUSTER, IVec3::new(-1, 0, 2)),
            special("drive_starboard", THRUSTER, IVec3::new(1, 0, 2)),
        ],
        INDUSTRIAL,
    )
}

fn maintenance_cutter_cells() -> Vec<IVec3> {
    union(vec![
        block(IVec3::new(-1, 0, -3), IVec3::new(3, 1, 6)),
        block(IVec3::new(-2, 0, 0), IVec3::new(1, 1, 3)),
        block(IVec3::new(2, 0, 0), IVec3::new(1, 1, 3)),
        vec![IVec3::new(0, 0, -4), IVec3::new(0, 1, -1)],
    ])
}

/// An unarmed needle skiff whose narrow sensor prow searches ahead of two
/// exposed machinery shoulders.
pub fn salvage_skiff() -> ShipHull {
    let cells = union(vec![
        block(IVec3::new(0, 0, -4), IVec3::new(1, 1, 8)),
        block(IVec3::new(-1, 0, -1), IVec3::new(3, 1, 4)),
        vec![IVec3::new(-2, 0, 1), IVec3::new(2, 0, 1)],
    ]);
    salvage_hull(
        cells,
        &[
            special("controller", CONTROLLER, IVec3::new(0, 1, -1)),
            special("drive_port", THRUSTER, IVec3::new(-1, 0, 3)),
            special("drive_starboard", THRUSTER, IVec3::new(1, 0, 3)),
        ],
    )
}

/// An unarmed fork tug with twin recovery booms and a broad drive crossbar.
pub fn salvage_tug() -> ShipHull {
    let cells = union(vec![
        block(IVec3::new(-2, 0, 1), IVec3::new(5, 1, 3)),
        block(IVec3::new(-2, 0, -4), IVec3::new(2, 1, 5)),
        block(IVec3::new(1, 0, -4), IVec3::new(2, 1, 5)),
        block(IVec3::new(-1, 1, 1), IVec3::new(3, 1, 2)),
    ]);
    salvage_hull(
        cells,
        &[
            special("controller", CONTROLLER, IVec3::new(0, 1, 1)),
            special("drive_port", THRUSTER, IVec3::new(-2, 0, 3)),
            special("drive_starboard", THRUSTER, IVec3::new(2, 0, 3)),
        ],
    )
}

/// A low, balanced armed picket with one PDC pushed onto the nose where the
/// hull cannot mask its forward firing arc.
pub fn salvage_picket() -> ShipHull {
    let cells = union(vec![
        block(IVec3::new(-1, 0, -3), IVec3::new(3, 1, 7)),
        block(IVec3::new(-2, 0, 0), IVec3::new(5, 1, 3)),
        vec![IVec3::new(0, 0, -4)],
    ]);
    salvage_hull(
        cells,
        &[
            special("controller", CONTROLLER, IVec3::new(0, 0, -1)),
            special("drive_port", THRUSTER, IVec3::new(-1, 0, 3)),
            special("drive_starboard", THRUSTER, IVec3::new(1, 0, 3)),
            Special {
                id: "pdc",
                prototype: PDC,
                // Rotate the mount's -Y base to face +Z against the nose's
                // -Z socket. The 0.5-cell mount centers outside that face.
                position: Vec3::new(0.0, 0.0, -4.75),
                rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            },
        ],
    )
}

/// An asymmetric armed search craft built around a port machinery pod and a
/// long starboard grapple arm.
pub fn salvage_claw() -> ShipHull {
    let cells = union(vec![
        block(IVec3::new(0, 0, -4), IVec3::new(1, 1, 9)),
        block(IVec3::new(-2, 0, -1), IVec3::new(2, 1, 5)),
        block(IVec3::new(1, 0, -2), IVec3::new(3, 1, 1)),
        vec![IVec3::new(3, 0, -3), IVec3::new(-1, 1, 1)],
    ]);
    salvage_hull(
        cells,
        &[
            special("controller", CONTROLLER, IVec3::new(-1, 1, 1)),
            special("drive_spine", THRUSTER, IVec3::new(0, 0, 4)),
            special("drive_pod", THRUSTER, IVec3::new(-2, 0, 4)),
            special("drive_grapple", THRUSTER, IVec3::new(3, 0, -1)),
            pdc("pdc", IVec3::new(3, 1, -2), Vec3::NEG_Y * 0.25),
        ],
    )
}

/// The cleanup leader: a heavier salvage hull with one dorsal PDC and one
/// flank-mounted standard Serpent torpedo bay.
pub fn salvage_leader() -> ShipHull {
    let mut cells = union(vec![
        block(IVec3::new(-2, 0, -3), IVec3::new(5, 1, 8)),
        block(IVec3::new(-1, 0, -5), IVec3::new(3, 1, 2)),
        block(IVec3::new(-1, -1, 2), IVec3::new(3, 3, 3)),
        block(IVec3::new(-1, 1, -1), IVec3::new(3, 1, 3)),
    ]);
    cells.retain(|cell| !(cell.y == 0 && cell.z == 0 && matches!(cell.x, -2 | -1)));
    salvage_hull(
        cells,
        &[
            special("controller_fore", CONTROLLER, IVec3::new(0, 1, -1)),
            special("controller_aft", CONTROLLER, IVec3::new(0, 1, 2)),
            Special {
                id: "vector_drive",
                prototype: VECTOR_THRUSTER,
                position: Vec3::new(0.0, 0.0, 5.5),
                rotation: Quat::IDENTITY,
            },
            pdc("pdc", IVec3::new(0, 2, 1), Vec3::NEG_Y * 0.25),
            Special {
                id: "torpedo_bay",
                prototype: SALVAGE_TORPEDO,
                position: Vec3::new(-1.5, 0.0, 0.0),
                rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            },
        ],
    )
}

fn salvage_hull(cells: Vec<IVec3>, specials: &[Special]) -> ShipHull {
    grid_hull_from(cells, specials, SALVAGE, LIGHT_HULL)
}

/// Disconnected industrial assemblies from the destroyed carrier. Each entry
/// is internally connected so it behaves as one floating wreck fragment.
pub fn carrier_wreck_fragments() -> Vec<ShipHull> {
    let fragments = vec![
        union(vec![
            block(IVec3::new(-2, -1, -3), IVec3::new(5, 3, 7)),
            block(IVec3::new(-1, 2, -1), IVec3::new(3, 2, 4)),
            block(IVec3::new(0, 4, 0), IVec3::new(1, 2, 2)),
        ]),
        block(IVec3::new(-2, -2, -3), IVec3::new(4, 5, 7)),
        block(IVec3::new(-1, -2, -3), IVec3::new(3, 5, 6)),
        union(vec![
            block(IVec3::new(-3, -1, -2), IVec3::new(7, 2, 5)),
            block(IVec3::new(-1, 1, -1), IVec3::new(3, 2, 3)),
        ]),
        union(vec![
            block(IVec3::new(-1, -1, -4), IVec3::new(3, 3, 7)),
            block(IVec3::new(-3, 0, 1), IVec3::new(7, 1, 3)),
        ]),
        block(IVec3::new(-1, -1, -2), IVec3::new(3, 2, 5)),
        block(IVec3::new(0, -1, -3), IVec3::new(2, 3, 6)),
        union(vec![
            block(IVec3::new(-2, 0, -2), IVec3::new(5, 1, 3)),
            block(IVec3::new(-2, 0, 1), IVec3::new(2, 2, 3)),
        ]),
        union(vec![
            block(IVec3::new(-1, -1, -2), IVec3::new(2, 3, 4)),
            block(IVec3::new(1, 0, 0), IVec3::new(3, 1, 2)),
        ]),
        block(IVec3::new(-2, 0, -1), IVec3::new(5, 1, 3)),
        block(IVec3::new(-1, -1, -2), IVec3::new(2, 2, 5)),
        union(vec![
            block(IVec3::new(-2, 0, 0), IVec3::new(5, 1, 2)),
            block(IVec3::new(1, -1, -2), IVec3::new(2, 2, 3)),
        ]),
        block(IVec3::new(0, 0, -1), IVec3::new(1, 1, 3)),
        block(IVec3::new(-1, 0, 0), IVec3::new(3, 1, 1)),
        block(IVec3::new(0, -1, -1), IVec3::new(1, 2, 3)),
        block(IVec3::new(-1, 0, -1), IVec3::new(2, 1, 3)),
        union(vec![
            block(IVec3::new(-1, 0, -1), IVec3::new(3, 1, 2)),
            vec![IVec3::new(1, 1, 0)],
        ]),
        union(vec![
            block(IVec3::new(0, -1, -2), IVec3::new(1, 3, 4)),
            vec![IVec3::new(1, 0, 1)],
        ]),
        block(IVec3::new(-1, -1, 0), IVec3::new(3, 2, 1)),
        union(vec![
            block(IVec3::new(-1, 0, -2), IVec3::new(2, 1, 4)),
            vec![IVec3::new(1, 0, -2)],
        ]),
        block(IVec3::new(0, 0, -2), IVec3::new(1, 2, 5)),
        block(IVec3::new(-2, 0, 0), IVec3::new(5, 1, 1)),
        union(vec![
            block(IVec3::new(-1, -1, -1), IVec3::new(2, 2, 3)),
            vec![IVec3::new(1, 0, 1)],
        ]),
        block(IVec3::new(-1, 0, -2), IVec3::new(3, 1, 5)),
        union(vec![
            block(IVec3::new(-2, 0, 0), IVec3::new(4, 1, 2)),
            block(IVec3::new(1, -1, 1), IVec3::new(1, 3, 2)),
        ]),
        block(IVec3::new(-1, -1, -1), IVec3::new(3, 3, 2)),
        union(vec![
            block(IVec3::new(0, -1, -2), IVec3::new(1, 2, 5)),
            block(IVec3::new(-2, 0, 1), IVec3::new(3, 1, 2)),
        ]),
        block(IVec3::new(-2, 0, -1), IVec3::new(4, 2, 3)),
    ];

    fragments
        .into_iter()
        .map(|cells| grid_hull(cells, &[], INDUSTRIAL))
        .collect()
}

pub fn industrial_carrier() -> ShipHull {
    // An elongated refinery spine buried between two massive cargo shoulders. The
    // middle is eleven cells wide and seven high: large enough that the cutter
    // reads as a carried workboat rather than as a peer flying beside it.
    let mut cells = union(vec![
        block(IVec3::new(-2, -1, -16), IVec3::new(5, 3, 33)),
        block(IVec3::new(-5, -3, -11), IVec3::new(3, 7, 23)),
        block(IVec3::new(3, -3, -11), IVec3::new(3, 7, 23)),
        block(IVec3::new(-3, 2, -7), IVec3::new(7, 1, 15)),
        block(IVec3::new(-2, 3, -9), IVec3::new(5, 2, 19)),
        block(IVec3::new(-1, 5, -5), IVec3::new(3, 2, 11)),
        block(IVec3::new(-1, -3, -9), IVec3::new(3, 2, 19)),
        // Broad five-cell-high transom for the pair of capital drives.
        block(IVec3::new(-5, -2, 13), IVec3::new(11, 5, 5)),
    ]);
    // Vertical cutter berths cut into both shoulders. The port berth remains
    // empty. The starboard cutter is carrier structure, not another ship: its
    // thin local Y axis points outboard, its width stands vertical, and its
    // nose-to-stern axis remains aligned with the carrier. It stands one cell
    // proud of the shoulder and connects through two small docking lugs, so its
    // outline and aft drive bells remain distinct from the carrier skin.
    cells.retain(|cell| {
        !(cell.x.abs() == 5 && (-2..=2).contains(&cell.y) && (-4..=2).contains(&cell.z))
    });
    cells.extend(
        maintenance_cutter_cells()
            .into_iter()
            .map(|local| IVec3::new(6 + local.y, -local.x, local.z)),
    );
    cells.extend([IVec3::new(5, -1, 0), IVec3::new(5, 1, 0)]);
    let docked_rotation = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    grid_hull(
        cells,
        &[
            special("bridge", CONTROLLER, IVec3::new(0, 6, -2)),
            special("control_forward", CONTROLLER, IVec3::new(0, 0, -14)),
            special("control_forward_mid", CONTROLLER, IVec3::new(0, 0, -8)),
            special("control_midships", CONTROLLER, IVec3::new(0, 0, 0)),
            special("control_aft_mid", CONTROLLER, IVec3::new(0, 0, 8)),
            special("control_aft", CONTROLLER, IVec3::new(0, 0, 15)),
            special("control_port_forward", CONTROLLER, IVec3::new(-1, 0, -11)),
            special(
                "control_starboard_forward",
                CONTROLLER,
                IVec3::new(1, 0, -11),
            ),
            special("control_port_aft", CONTROLLER, IVec3::new(-1, 0, 11)),
            special("control_starboard_aft", CONTROLLER, IVec3::new(1, 0, 11)),
            Special {
                id: "docked_cutter_drive_port",
                prototype: THRUSTER,
                position: Vec3::new(6.0, 1.0, 2.0),
                rotation: docked_rotation,
            },
            Special {
                id: "docked_cutter_drive_starboard",
                prototype: THRUSTER,
                position: Vec3::new(6.0, -1.0, 2.0),
                rotation: docked_rotation,
            },
            Special {
                id: "capital_drive_port",
                prototype: CAPITAL_THRUSTER,
                position: Vec3::new(-3.0, 0.0, 19.0),
                rotation: Quat::IDENTITY,
            },
            Special {
                id: "capital_drive_starboard",
                prototype: CAPITAL_THRUSTER,
                position: Vec3::new(3.0, 0.0, 19.0),
                rotation: Quat::IDENTITY,
            },
        ],
        INDUSTRIAL,
    )
}

pub fn stolen_warship() -> ShipHull {
    // A long five-wide fighting spine with width reserved for the engine
    // transom. The old seven-wide slab made the ship read thick beside the
    // carrier instead of fast and military.
    let mut cells = union(vec![
        block(IVec3::new(-2, -1, 0), IVec3::new(5, 3, 14)),
        block(IVec3::new(-2, -1, -6), IVec3::new(5, 3, 6)),
        block(IVec3::new(-1, 2, 2), IVec3::new(3, 1, 7)),
        block(IVec3::new(-3, -1, 11), IVec3::new(1, 3, 3)),
        block(IVec3::new(3, -1, 11), IVec3::new(1, 3, 3)),
    ]);
    // Carve exact weapon volumes out of the hull. Each lateral bay consumes
    // two cells and mates to the centreline cell, leaving its muzzle flush
    // with the flank. Each three-cell lance sits inside the prow, whose cheeks
    // and upper/lower decks now reach the muzzle plane instead of ending three
    // cells behind it.
    cells.retain(|cell| {
        let side_bay = cell.y == 0 && [-2, 2, 6].contains(&cell.z) && matches!(cell.x.abs(), 1 | 2);
        let railgun = cell.y == 0 && (-6..=-4).contains(&cell.z) && cell.x.abs() == 1;
        !side_bay && !railgun
    });
    let pdc_seat = Vec3::NEG_Y * 0.25;
    let underside_pdc_seat = Vec3::Y * 0.25;
    grid_hull(
        cells,
        &[
            special("bridge", CONTROLLER, IVec3::new(0, 2, 3)),
            special("control_fore", CONTROLLER, IVec3::new(0, 0, -2)),
            special("control_midships", CONTROLLER, IVec3::new(0, 0, 6)),
            special("control_aft", CONTROLLER, IVec3::new(0, 0, 12)),
            special("control_bow", CONTROLLER, IVec3::new(0, 0, -5)),
            special("control_forward", CONTROLLER, IVec3::new(0, 0, 0)),
            special("control_forward_mid", CONTROLLER, IVec3::new(0, 0, 3)),
            special("control_mid_aft", CONTROLLER, IVec3::new(0, 0, 8)),
            special("control_aft_mid", CONTROLLER, IVec3::new(0, 0, 10)),
            special("control_stern", CONTROLLER, IVec3::new(0, 0, 13)),
            Special {
                id: "vector_drive_port",
                prototype: VECTOR_THRUSTER,
                position: Vec3::new(-2.0, 0.0, 14.5),
                rotation: Quat::IDENTITY,
            },
            Special {
                id: "vector_drive_starboard",
                prototype: VECTOR_THRUSTER,
                position: Vec3::new(2.0, 0.0, 14.5),
                rotation: Quat::IDENTITY,
            },
            side_bay(
                "bastion_bay_port_forward",
                -1.5,
                -2.0,
                std::f32::consts::FRAC_PI_2,
            ),
            side_bay(
                "bastion_bay_port_midships",
                -1.5,
                2.0,
                std::f32::consts::FRAC_PI_2,
            ),
            side_bay(
                "bastion_bay_port_aft",
                -1.5,
                6.0,
                std::f32::consts::FRAC_PI_2,
            ),
            side_bay(
                "bastion_bay_starboard_forward",
                1.5,
                -2.0,
                -std::f32::consts::FRAC_PI_2,
            ),
            side_bay(
                "bastion_bay_starboard_midships",
                1.5,
                2.0,
                -std::f32::consts::FRAC_PI_2,
            ),
            side_bay(
                "bastion_bay_starboard_aft",
                1.5,
                6.0,
                -std::f32::consts::FRAC_PI_2,
            ),
            Special {
                id: "railgun_port",
                prototype: RAILGUN,
                position: Vec3::new(-1.0, 0.0, -5.0),
                rotation: Quat::IDENTITY,
            },
            Special {
                id: "railgun_starboard",
                prototype: RAILGUN,
                position: Vec3::new(1.0, 0.0, -5.0),
                rotation: Quat::IDENTITY,
            },
            pdc("pdc_forward_port", IVec3::new(-2, 2, 1), pdc_seat),
            pdc("pdc_forward_starboard", IVec3::new(2, 2, 1), pdc_seat),
            pdc("pdc_aft_port", IVec3::new(-2, 2, 10), pdc_seat),
            pdc("pdc_aft_starboard", IVec3::new(2, 2, 10), pdc_seat),
            pdc("pdc_dorsal_port", IVec3::new(-1, 3, 4), pdc_seat),
            pdc("pdc_dorsal_starboard", IVec3::new(1, 3, 4), pdc_seat),
            underside_pdc(
                "pdc_ventral_forward_port",
                IVec3::new(-2, -2, 2),
                underside_pdc_seat,
            ),
            underside_pdc(
                "pdc_ventral_forward_starboard",
                IVec3::new(2, -2, 2),
                underside_pdc_seat,
            ),
            underside_pdc(
                "pdc_ventral_aft_port",
                IVec3::new(-2, -2, 10),
                underside_pdc_seat,
            ),
            underside_pdc(
                "pdc_ventral_aft_starboard",
                IVec3::new(2, -2, 10),
                underside_pdc_seat,
            ),
        ],
        ARMOURED,
    )
}

fn side_bay(id: &'static str, x: f32, z: f32, yaw: f32) -> Special {
    Special {
        id,
        prototype: TORPEDO,
        position: Vec3::new(x, 0.0, z),
        rotation: Quat::from_rotation_y(yaw),
    }
}

fn pdc(id: &'static str, cell: IVec3, seat: Vec3) -> Special {
    Special {
        id,
        prototype: PDC,
        position: cell.as_vec3() + seat,
        rotation: Quat::IDENTITY,
    }
}

fn underside_pdc(id: &'static str, cell: IVec3, seat: Vec3) -> Special {
    Special {
        id,
        prototype: PDC,
        position: cell.as_vec3() + seat,
        rotation: Quat::from_rotation_z(std::f32::consts::PI),
    }
}

fn special(id: &'static str, prototype: &'static str, cell: IVec3) -> Special {
    Special {
        id,
        prototype,
        position: cell.as_vec3(),
        rotation: Quat::IDENTITY,
    }
}

fn grid_hull(cells: Vec<IVec3>, specials: &[Special], style: &str) -> ShipHull {
    grid_hull_from(cells, specials, style, HULL)
}

fn grid_hull_from(
    cells: Vec<IVec3>,
    specials: &[Special],
    style: &str,
    hull_prototype: &str,
) -> ShipHull {
    let replaced: HashMap<IVec3, &Special> = specials
        .iter()
        .filter_map(|part| {
            let rounded = part.position.round();
            part.position
                .abs_diff_eq(rounded, 1e-5)
                .then_some((rounded.as_ivec3(), part))
        })
        .collect();

    let mut sections: Vec<_> = cells
        .into_iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            if replaced.contains_key(&cell) {
                return None;
            }
            Some(SpaceshipSectionConfig {
                id: format!("hull_{index}"),
                position: cell.as_vec3(),
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype(hull_prototype.to_string()),
                modifications: vec![],
            })
        })
        .collect();

    sections.extend(specials.iter().map(|part| SpaceshipSectionConfig {
        id: part.id.to_string(),
        position: part.position,
        rotation: part.rotation,
        source: SectionSource::Prototype(part.prototype.to_string()),
        modifications: vec![],
    }));

    ShipHull {
        sections,
        skin: true,
        style: Some(style.to_string()),
        ..default()
    }
}

fn block(origin: IVec3, size: IVec3) -> Vec<IVec3> {
    (0..size.x)
        .flat_map(|x| {
            (0..size.y).flat_map(move |y| (0..size.z).map(move |z| origin + IVec3::new(x, y, z)))
        })
        .collect()
}

fn union(parts: Vec<Vec<IVec3>>) -> Vec<IVec3> {
    let mut seen = HashSet::new();
    parts
        .into_iter()
        .flatten()
        .filter(|cell| seen.insert(*cell))
        .collect()
}
