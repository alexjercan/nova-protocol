//! The hulls of the stations generated worlds place, built on the cell grid
//! from shipped prototypes. Three layouts, each with its own look:
//!
//! - the RING: a plated hub on four truss spokes inside a ring, a docking
//!   collar at the end of every spoke and guns on the hub and the ring;
//! - the SPIRE: three plated decks stacked on truss columns, collars round
//!   the middle deck, guns on the top and the bottom deck, and solar wings;
//! - the YARD: a long spine between two end blocks, rows of solar panels
//!   either side, collars at both ends and on the core's flanks.
//!
//! Cells are BUILD-GRID cells: one cell is one world unit is 10 m. Compass
//! names read the XZ plane from above: north is `-Z`, east is `+X`.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use nova_scenario::prelude::{SectionSource, SpaceshipSectionConfig};
use nova_ship::prelude::{
    BASIC_CONTROLLER_SECTION_ID, DOCKING_PORT_SECTION_ID, PDC_KINETIC_TURRET_SECTION_ID,
    REINFORCED_HULL_SECTION_ID,
};

use crate::base_content::{
    sections::{SOLAR_PANEL_SECTION_ID, TRUSS_HULL_SECTION_ID},
    ships::BLOCK_BRIDGE_SECTION_ID,
};

/// How many cells out a ring spoke node stands from the hub centre.
const RING_RADIUS: i32 = 10;

/// How many truss cells a straight ring run lays either side of a node.
const RING_RUN: i32 = 4;

/// How many solar panels a ring mast carries either side of its head.
const MAST_PANELS: i32 = 4;

/// How far above and below the core deck the spire's other two decks stand.
const SPIRE_DECK_STEP: i32 = 8;

/// How many solar panels long a spire wing row is.
const SPIRE_WING_PANELS: i32 = 5;

/// How far out along the spine the yard's end blocks start.
const YARD_END: i32 = 10;

/// The spine cells a yard panel row branches off, either side of the core.
const YARD_ROWS: [i32; 3] = [3, 5, 7];

/// How many solar panels long a yard row is.
const YARD_ROW_PANELS: i32 = 7;

/// How far a turret drops into its own cell to put its socket on the plate
/// below. The same seat the block fleet uses.
const TURRET_SEAT: Vec3 = Vec3::new(0.0, -0.25, 0.0);

/// The four spoke directions, by compass name, as the unit cell step out.
const SPOKES: [(&str, IVec3); 4] = [
    ("north", IVec3::NEG_Z),
    ("east", IVec3::X),
    ("south", IVec3::Z),
    ("west", IVec3::NEG_X),
];

/// The four diagonals, by compass name, as the signs of their X and Z.
const QUADRANTS: [(&str, i32, i32); 4] = [
    ("northeast", 1, -1),
    ("southeast", 1, 1),
    ("southwest", -1, 1),
    ("northwest", -1, -1),
];

/// A station hull under construction: the section list, in placement order.
#[derive(Default)]
struct Layout {
    sections: Vec<SpaceshipSectionConfig>,
}

impl Layout {
    fn place(&mut self, id: String, prototype: &str, position: Vec3, rotation: Quat) {
        self.sections.push(SpaceshipSectionConfig {
            id,
            position,
            rotation,
            source: SectionSource::prototype(prototype),
        });
    }

    /// A reinforced plate filling `cell`.
    fn plate(&mut self, id: String, cell: IVec3) {
        self.place(
            id,
            REINFORCED_HULL_SECTION_ID,
            cell.as_vec3(),
            Quat::IDENTITY,
        );
    }

    /// A solid block of plates from `min` to `max` inclusive, named
    /// `{name}_{n}`, with a flight computer called `computer` at `heart`.
    fn deck(&mut self, name: &str, min: IVec3, max: IVec3, computer: &str, heart: IVec3) {
        let mut plate = 0;
        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let cell = IVec3::new(x, y, z);
                    if cell == heart {
                        self.place(
                            computer.to_string(),
                            BASIC_CONTROLLER_SECTION_ID,
                            cell.as_vec3(),
                            Quat::IDENTITY,
                        );
                    } else {
                        self.plate(format!("{name}_{plate}"), cell);
                        plate += 1;
                    }
                }
            }
        }
    }

    /// A truss cell at `cell` whose two sockets lie along `axis`.
    fn truss(&mut self, id: String, cell: IVec3, axis: IVec3) {
        self.place(id, TRUSS_HULL_SECTION_ID, cell.as_vec3(), truss_turn(axis));
    }

    /// A gun standing in `cell`, seated down onto the plate below it.
    fn dorsal_gun(&mut self, id: String, cell: IVec3) {
        self.place(
            id,
            PDC_KINETIC_TURRET_SECTION_ID,
            cell.as_vec3() + TURRET_SEAT,
            Quat::IDENTITY,
        );
    }

    /// A gun hung in `cell`, rolled over onto the plate above it.
    fn ventral_gun(&mut self, id: String, cell: IVec3) {
        self.place(
            id,
            PDC_KINETIC_TURRET_SECTION_ID,
            cell.as_vec3() - TURRET_SEAT,
            Quat::from_rotation_z(PI),
        );
    }

    /// A docking collar in `cell` with its hatch facing `out`.
    fn collar(&mut self, id: String, cell: IVec3, out: IVec3) {
        self.place(id, DOCKING_PORT_SECTION_ID, cell.as_vec3(), hatch_turn(out));
    }

    /// `count` solar panels from `first` on, one cell further along `X` by
    /// `step` each, named `{prefix}{n}`.
    fn panels(&mut self, prefix: &str, first: IVec3, step: i32, count: i32) {
        for index in 0..count {
            self.place(
                format!("{prefix}{index}"),
                SOLAR_PANEL_SECTION_ID,
                (first + IVec3::X * step * index).as_vec3(),
                Quat::IDENTITY,
            );
        }
    }
}

/// The ring station's section list, in a stable order.
pub(super) fn ring_station() -> Vec<SpaceshipSectionConfig> {
    let mut layout = Layout::default();
    layout.deck(
        "hub",
        IVec3::NEG_ONE,
        IVec3::ONE,
        BLOCK_BRIDGE_SECTION_ID,
        IVec3::ZERO,
    );

    for (name, out) in SPOKES {
        for step in 2..RING_RADIUS {
            layout.truss(format!("spoke_{name}_{step}"), out * step, out);
        }
        // Each node carries a flight computer of its own, so the station is
        // not brain-dead the moment one hit reaches the hub.
        let node = out * RING_RADIUS;
        layout.place(
            format!("computer_{name}"),
            BASIC_CONTROLLER_SECTION_ID,
            node.as_vec3(),
            Quat::IDENTITY,
        );
        layout.collar(format!("collar_{name}"), node + out, out);
        // The ring runs at right angles to the spoke, both ways from the node.
        let across = IVec3::new(-out.z, 0, out.x);
        for step in 1..=RING_RUN {
            for (side, sign) in [("a", 1), ("b", -1)] {
                layout.truss(
                    format!("ring_{name}_{side}{step}"),
                    node + across * sign * step,
                    across,
                );
            }
        }
    }

    // Each diagonal is a plated stair from the end of one straight run to the
    // start of the next, so every step mates face to face.
    for (name, sx, sz) in QUADRANTS {
        for (step, (x, z)) in stair().into_iter().enumerate() {
            layout.plate(format!("ring_{name}_{step}"), IVec3::new(sx * x, 0, sz * z));
        }
        let middle = IVec3::new(sx * 7, 0, sz * 7);
        layout.dorsal_gun(format!("pdc_{name}_dorsal"), middle + IVec3::Y);
        layout.ventral_gun(format!("pdc_{name}_ventral"), middle - IVec3::Y);
    }

    // Four hub guns on opposite corners, two up and two down, so every face
    // of the hub is covered by at least one.
    for (name, x, z) in [("northeast", 1, -1), ("southwest", -1, 1)] {
        layout.dorsal_gun(format!("pdc_hub_{name}_dorsal"), IVec3::new(x, 2, z));
    }
    for (name, x, z) in [("southeast", 1, 1), ("northwest", -1, -1)] {
        layout.ventral_gun(format!("pdc_hub_{name}_ventral"), IVec3::new(x, -2, z));
    }

    // The solar masts: two truss cells off the hub's top and bottom centre, a
    // plated head, and a row of panels either side of it.
    for (name, up) in [("dorsal", 1), ("ventral", -1)] {
        for step in 2..=3 {
            layout.truss(
                format!("mast_{name}_{step}"),
                IVec3::Y * up * step,
                IVec3::Y,
            );
        }
        let head = IVec3::Y * 4 * up;
        layout.plate(format!("mast_{name}_head"), head);
        layout.panels(
            &format!("solar_{name}_east"),
            head + IVec3::X,
            1,
            MAST_PANELS,
        );
        layout.panels(
            &format!("solar_{name}_west"),
            head - IVec3::X,
            -1,
            MAST_PANELS,
        );
    }

    layout.sections
}

/// The spire station's section list, in a stable order.
pub(super) fn spire_station() -> Vec<SpaceshipSectionConfig> {
    let mut layout = Layout::default();
    let upper = IVec3::Y * SPIRE_DECK_STEP;
    let lower = -upper;
    for (name, heart, computer) in [
        ("core", IVec3::ZERO, BLOCK_BRIDGE_SECTION_ID),
        ("upper", upper, "computer_upper"),
        ("lower", lower, "computer_lower"),
    ] {
        layout.deck(
            name,
            heart - IVec3::ONE,
            heart + IVec3::ONE,
            computer,
            heart,
        );
    }

    // Four truss columns up each corner between the decks.
    for (name, sx, sz) in QUADRANTS {
        for step in 2..=SPIRE_DECK_STEP - 2 {
            layout.truss(
                format!("column_{name}_up{step}"),
                IVec3::new(sx, step, sz),
                IVec3::Y,
            );
            layout.truss(
                format!("column_{name}_down{step}"),
                IVec3::new(sx, -step, sz),
                IVec3::Y,
            );
        }
        layout.dorsal_gun(format!("pdc_upper_{name}"), upper + IVec3::new(sx, 2, sz));
        layout.ventral_gun(format!("pdc_lower_{name}"), lower + IVec3::new(sx, -2, sz));
    }

    for (name, out) in SPOKES {
        layout.collar(format!("collar_{name}"), out * 2, out);
    }

    // A solar wing three rows wide off each flank of the top and bottom
    // decks: two truss cells out, then a row of panels.
    for (deck, heart) in [("upper", upper), ("lower", lower)] {
        for (side, sx) in [("east", 1), ("west", -1)] {
            for row in -1..=1 {
                let base = heart + IVec3::new(0, 0, row);
                let tag = format!("wing_{deck}_{side}_{}", row + 1);
                for step in 2..=3 {
                    layout.truss(
                        format!("{tag}_truss{step}"),
                        base + IVec3::X * sx * step,
                        IVec3::X,
                    );
                }
                layout.panels(
                    &format!("{tag}_solar"),
                    base + IVec3::X * sx * 4,
                    sx,
                    SPIRE_WING_PANELS,
                );
            }
        }
    }

    layout.sections
}

/// The yard station's section list, in a stable order.
pub(super) fn yard_station() -> Vec<SpaceshipSectionConfig> {
    let mut layout = Layout::default();
    layout.deck(
        "core",
        IVec3::new(-1, 0, -1),
        IVec3::new(1, 1, 1),
        BLOCK_BRIDGE_SECTION_ID,
        IVec3::new(0, 1, 0),
    );
    for (name, out) in [("east", IVec3::X), ("west", IVec3::NEG_X)] {
        layout.collar(format!("collar_{name}"), out * 2, out);
    }

    for (name, sz) in [("bow", -1), ("stern", 1)] {
        let near = YARD_END * sz;
        let far = (YARD_END + 2) * sz;
        layout.deck(
            &format!("{name}_block"),
            IVec3::new(-1, 0, near.min(far)),
            IVec3::new(1, 1, near.max(far)),
            &format!("computer_{name}"),
            IVec3::new(0, 1, (YARD_END + 1) * sz),
        );
        layout.collar(
            format!("collar_{name}"),
            IVec3::new(0, 0, (YARD_END + 3) * sz),
            IVec3::Z * sz,
        );
        let gun_z = (YARD_END + 1) * sz;
        for (side, sx) in [("east", 1), ("west", -1)] {
            layout.dorsal_gun(
                format!("pdc_{name}_{side}_dorsal"),
                IVec3::new(sx, 2, gun_z),
            );
            layout.ventral_gun(
                format!("pdc_{name}_{side}_ventral"),
                IVec3::new(sx, -1, gun_z),
            );
        }

        // The spine: truss, with a plate wherever a panel row branches off,
        // because a truss cell mates only along the spine.
        for step in 2..YARD_END {
            let cell = IVec3::Z * sz * step;
            if YARD_ROWS.contains(&step) {
                layout.plate(format!("spine_{name}_{step}"), cell);
                for (side, sx) in [("east", 1), ("west", -1)] {
                    let tag = format!("row_{name}{step}_{side}");
                    for out in 1..=2 {
                        layout.truss(
                            format!("{tag}_truss{out}"),
                            cell + IVec3::X * sx * out,
                            IVec3::X,
                        );
                    }
                    layout.panels(
                        &format!("{tag}_solar"),
                        cell + IVec3::X * sx * 3,
                        sx,
                        YARD_ROW_PANELS,
                    );
                }
            } else {
                layout.truss(format!("spine_{name}_{step}"), cell, IVec3::Z);
            }
        }
    }

    layout.sections
}

/// The cells of one ring diagonal stair in the `+X +Z` quadrant, from the
/// end of the south run's east half to the end of the east run's south half.
fn stair() -> Vec<(i32, i32)> {
    let start = RING_RUN + 1;
    let mut cells = vec![(start, RING_RADIUS)];
    let (mut x, mut z) = (start, RING_RADIUS);
    while (x, z) != (RING_RADIUS, start) {
        z -= 1;
        cells.push((x, z));
        x += 1;
        cells.push((x, z));
    }
    cells
}

/// The rotation that lays a truss cell's two sockets along `axis`. The truss
/// mates along its own `Z`.
fn truss_turn(axis: IVec3) -> Quat {
    if axis.x != 0 {
        Quat::from_rotation_y(FRAC_PI_2)
    } else if axis.y != 0 {
        Quat::from_rotation_x(FRAC_PI_2)
    } else {
        Quat::IDENTITY
    }
}

/// The rotation that opens a docking collar's hatch along `out`. The hatch is
/// the port's own `-Z`.
fn hatch_turn(out: IVec3) -> Quat {
    Quat::from_rotation_y((-out.x as f32).atan2(-out.z as f32))
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use nova_ship::prelude::{
        derive_link_point_graph, exit_normal, placement_blocks_an_exit, PlacedPart,
        PlacedSectionLinkPoints, SectionFootprint, SectionLinkPoints,
    };

    use super::*;

    /// Every layout, with the guns, computers and collars it carries.
    fn layouts() -> Vec<(&'static str, Vec<SpaceshipSectionConfig>, [usize; 3])> {
        vec![
            ("ring", ring_station(), [12, 5, 4]),
            ("spire", spire_station(), [8, 3, 4]),
            ("yard", yard_station(), [8, 3, 4]),
        ]
    }

    fn prototype(section: &SpaceshipSectionConfig) -> &str {
        section.source.prototype_id()
    }

    /// Each station is one structure: every section, guns and collars
    /// included, joins one connected mate graph.
    #[test]
    fn every_station_layout_mates_into_one_structure() {
        let catalog = crate::generation::build_section_catalog();
        for (name, sections, _) in layouts() {
            let sockets: Vec<_> = sections
                .iter()
                .map(|section| {
                    let config = catalog
                        .iter()
                        .find(|config| config.base.id == prototype(section))
                        .unwrap_or_else(|| {
                            panic!("{name} '{}' names a missing prototype", section.id)
                        });
                    SectionLinkPoints(config.base.link_points.clone())
                })
                .collect();
            let placed: Vec<_> = sections
                .iter()
                .zip(&sockets)
                .map(|(section, link_points)| PlacedSectionLinkPoints {
                    position: section.position,
                    rotation: section.rotation,
                    link_points,
                })
                .collect();
            let mates = derive_link_point_graph(&placed)
                .unwrap_or_else(|errors| panic!("the {name} does not mate: {errors:?}"));
            assert!(mates.len() >= sections.len() - 1, "{name}");
        }
    }

    /// No section stands in or sockets into a gun's or a collar's lane, so
    /// every turret can fire and every collar can take a ship.
    #[test]
    fn no_station_layout_blocks_an_exit() {
        let catalog = crate::generation::build_section_catalog();
        for (name, sections, [guns, _, collars]) in layouts() {
            let place = |section: &SpaceshipSectionConfig| {
                let config = catalog
                    .iter()
                    .find(|config| config.base.id == prototype(section))
                    .expect("every station section is in the catalog");
                PlacedPart {
                    position: section.position,
                    rotation: section.rotation,
                    link_points: config.base.link_points.as_slice(),
                    footprint: *SectionFootprint::from_collider(
                        config.base.collider.unwrap_or_default(),
                    ),
                    exit: exit_normal(&config.kind),
                }
            };
            let mut exits = 0;
            for (index, section) in sections.iter().enumerate() {
                let part = place(section);
                if part.exit.is_none() {
                    continue;
                }
                exits += 1;
                let rest: Vec<PlacedPart> = sections
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != index)
                    .map(|(_, other)| place(other))
                    .collect();
                assert!(
                    !placement_blocks_an_exit(&rest, &part),
                    "{name} '{}' has a blocked exit",
                    section.id
                );
            }
            assert_eq!(exits, guns + collars, "{name}");
        }
    }

    /// Each station carries the guns, flight computers and collars its role
    /// reads, every id once and no two sections in one cell, and reaches no
    /// more than 400 m from its root, the cap a generated body may declare.
    #[test]
    fn every_station_layout_carries_its_guns_computers_and_collars() {
        for (name, sections, [guns, computers, collars]) in layouts() {
            let count = |id: &str| {
                sections
                    .iter()
                    .filter(|section| prototype(section) == id)
                    .count()
            };
            assert_eq!(count(PDC_KINETIC_TURRET_SECTION_ID), guns, "{name}");
            assert_eq!(count(BASIC_CONTROLLER_SECTION_ID), computers, "{name}");
            assert_eq!(count(DOCKING_PORT_SECTION_ID), collars, "{name}");

            let ids: HashSet<&str> = sections.iter().map(|section| section.id.as_str()).collect();
            assert_eq!(ids.len(), sections.len(), "{name} repeats a section id");
            let mut seats: HashMap<IVec3, &str> = HashMap::new();
            for section in &sections {
                let rounded = section.position.round();
                if !section.position.abs_diff_eq(rounded, 1e-5) {
                    continue;
                }
                if let Some(other) = seats.insert(rounded.as_ivec3(), &section.id) {
                    panic!("{name} '{}' stands in '{other}'", section.id);
                }
            }

            let reach_m = sections
                .iter()
                .map(|section| (section.position.abs() + Vec3::splat(0.5)).length())
                .fold(0.0, f32::max)
                * 10.0;
            assert!(reach_m <= 400.0, "{name} reaches {reach_m} m");
        }
    }

    /// The ring spans 150 to 250 m across, the size the owner asked for.
    #[test]
    fn the_ring_station_spans_150_to_250_m() {
        let span_m = (ring_station()
            .iter()
            .map(|section| section.position.x)
            .fold(f32::MIN, f32::max)
            * 2.0
            + 1.0)
            * 10.0;
        assert!((150.0..=250.0).contains(&span_m), "spans {span_m} m");
    }
}
