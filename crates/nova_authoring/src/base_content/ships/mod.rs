//! Shipped semantic craft assemblies, their section prototypes, and the ship
//! CONTENT entries a scenario spawns them by.
//!
//! A grade is a build-time knob, not a spawn-time one: the raider corvette
//! carries thinner plating and scavenger-grade guns, which is a different ship
//! to fight and to read about, so it is a second CATALOG entry rather than a
//! flag a scenario flips. Two entries cost one line each here and no machinery
//! anywhere else.
//!
//! ORDNANCE is the same shape of knob. The two cargo-B entries differ only in
//! which torpedo their pods load, and that decides whether the ship a player
//! meets is one their point defense can answer - which is exactly "a different
//! ship to fight" again.

use nova_scenario::prelude::{
    SectionModification, ShipConfig, ShipHull, ShipSectionModification, ShipSource,
    SpaceshipSectionConfig,
};
use nova_ship::prelude::SectionConfig;

use super::assets::BaseContentAssets;

mod cargo_a;
mod cargo_b;
mod racer;
mod shared;

pub(crate) use cargo_a::CARGOA_TURRET_IDS;
use shared::{Ordnance, ShipGrade};

/// The id the player-grade CargoA corvette is spawned by.
pub(crate) const CARGOA_SHIP_ID: &str = "cargoa";
/// The id the scavenger-grade CargoA corvette is spawned by: thinner plating,
/// light turrets, a softer flight computer.
pub(crate) const CARGOA_RAIDER_SHIP_ID: &str = "cargoa_raider";
/// The id the CargoB torpedo hauler is spawned by: weaving Serpents in the
/// tubes, which is the escalation a defender cannot screen.
pub(crate) const CARGOB_SHIP_ID: &str = "cargob";
/// The id the CargoB is spawned by when its tubes carry straight-running
/// LANCES. The same hull, guns and rack; the ordnance is the whole difference,
/// and it is the campaign's difficulty setting for a player's first torpedo
/// fight (see `sections::ordnance`).
pub(crate) const CARGOB_LANCE_SHIP_ID: &str = "cargob_lance";
/// The id the unarmed Racer yacht is spawned by.
pub(crate) const RACER_SHIP_ID: &str = "racer";

/// The section id every shipped craft's flight computer carries - what a
/// scenario aims a spawn-time controller modification at.
pub(crate) const FUSELAGE_SECTION_ID: &str = "fuselage";

/// Semantic Racer, CargoB, and CargoA part prototypes in generated-content order.
pub(crate) fn semantic_part_prototypes(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    let mut sections = racer::prototypes_for(assets);
    sections.extend(cargo_b::prototypes_for(assets));
    sections.extend(cargo_a::prototypes_for(assets));
    sections
}

/// Every shipped ship, in stable generated-content order.
pub(crate) fn ship_catalog() -> Vec<ShipConfig> {
    vec![
        ship(RACER_SHIP_ID, "Racer Yacht", racer::sections()),
        ship(
            CARGOB_SHIP_ID,
            "CargoB Hauler",
            cargo_b::sections(Ordnance::Serpent),
        ),
        ship(
            CARGOB_LANCE_SHIP_ID,
            "CargoB Hauler (Lance)",
            cargo_b::sections(Ordnance::Lance),
        ),
        ship(
            CARGOA_SHIP_ID,
            "CargoA Corvette",
            cargo_a::sections(ShipGrade::Player),
        ),
        ship(
            CARGOA_RAIDER_SHIP_ID,
            "CargoA Raider Corvette",
            cargo_a::sections(ShipGrade::Enemy),
        ),
    ]
}

/// A spawn of one CATALOG ship, by id.
pub(crate) fn hull(id: &str) -> ShipSource {
    ShipSource::Prototype(id.to_string())
}

/// A ONE-OFF hull, authored inline: a scripted battery that is a single tube, a
/// derelict that is five plates. Anything a second scenario would want gets a
/// catalog entry instead.
pub(crate) fn inline_hull(sections: Vec<SpaceshipSectionConfig>) -> ShipSource {
    ShipSource::Inline(ShipHull {
        sections,
        ..Default::default()
    })
}

/// One spawn-time delta aimed at a named section of the resolved hull.
pub(crate) fn on_section(
    section: &str,
    modifications: Vec<SectionModification>,
) -> ShipSectionModification {
    ShipSectionModification {
        section: section.to_string(),
        modifications,
    }
}

/// One catalog entry over a built section list. Every shipped ship takes the
/// engine's collapse threshold and goes unclad, so the hull is its sections.
fn ship(id: &str, name: &str, sections: Vec<SpaceshipSectionConfig>) -> ShipConfig {
    ShipConfig {
        id: id.to_string(),
        name: name.to_string(),
        hull: ShipHull {
            sections,
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec3;
    use nova_ship::prelude::{
        cardinal_axis, derive_link_point_graph, snap_placement, unit_cube_link_points,
        PlacedSectionLinkPoints, SectionLinkPoints,
    };

    use super::{cargo_a::*, cargo_b::*, racer::*, shared::*};

    #[test]
    fn every_parts_ship_has_one_connected_mate_graph() {
        for (specs, edges) in [
            (RACER_PARTS.as_slice(), RACER_EDGES.as_slice()),
            (CARGOB_PARTS.as_slice(), CARGOB_EDGES.as_slice()),
            (CARGOA_PARTS.as_slice(), CARGOA_EDGES.as_slice()),
        ] {
            let points: Vec<_> = specs
                .iter()
                .enumerate()
                .map(|(index, _)| SectionLinkPoints(link_points(specs, edges, index)))
                .collect();
            let placed: Vec<_> = specs
                .iter()
                .zip(&points)
                .map(|(spec, points)| PlacedSectionLinkPoints {
                    position: spec.center(),
                    rotation: spec.rotation(),
                    link_points: points,
                })
                .collect();
            let mates = derive_link_point_graph(&placed).unwrap();
            assert_eq!(mates.len(), edges.len());
        }
    }

    /// The claim the whole snap exists for: a part cut off ONE craft mates onto
    /// a plain cube square, not at whatever angle its neighbour on that craft
    /// happened to sit at.
    ///
    /// The cargob's torpedo pod is the case the owner hit - its fuselage socket
    /// used to point 36 degrees off -X, and the pod arrived on a hull tilted by
    /// exactly that much. Every socket of every shipped part is checked, on
    /// every face of the cube, because "only parts from the same ship fit" was
    /// the shape of the bug.
    #[test]
    fn every_semantic_part_mates_square_onto_a_plain_cube() {
        for (specs, edges) in [
            (RACER_PARTS.as_slice(), RACER_EDGES.as_slice()),
            (CARGOB_PARTS.as_slice(), CARGOB_EDGES.as_slice()),
            (CARGOA_PARTS.as_slice(), CARGOA_EDGES.as_slice()),
        ] {
            for index in 0..specs.len() {
                for socket in link_points(specs, edges, index) {
                    for face in unit_cube_link_points() {
                        let (_, rotation) = snap_placement(face.position, face.normal, &socket, 0);

                        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                            let placed = rotation * axis;
                            assert!(
                                placed.abs_diff_eq(cardinal_axis(placed), 1e-4),
                                "part {index} socket `{}` arrived tilted ({placed:?}) on \
                                 the cube's `{}` face",
                                socket.id,
                                face.id,
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn part_mesh_offsets_preserve_recipe_assembly_bounds() {
        for specs in [&RACER_PARTS[..7], &CARGOB_PARTS[..7], &CARGOA_PARTS[..7]] {
            for spec in specs {
                let rendered_min = spec.center() + spec.mesh_offset() + spec.bbox_min;
                let rendered_max = spec.center() + spec.mesh_offset() + spec.bbox_max;
                assert_eq!(rendered_min, spec.origin + spec.bbox_min);
                assert_eq!(rendered_max, spec.origin + spec.bbox_max);
            }
        }
    }
}
