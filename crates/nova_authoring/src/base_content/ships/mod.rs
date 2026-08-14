//! Shipped semantic craft assemblies and their section prototypes.

use nova_scenario::prelude::{SectionModification, SpaceshipSectionConfig};
use nova_ship::prelude::SectionConfig;

use super::assets::BaseContentAssets;

mod cargo_a;
mod cargo_b;
mod racer;
mod shared;

pub(crate) use cargo_a::CARGOA_TURRET_IDS;
pub(crate) use shared::ShipGrade;

/// Semantic Racer, CargoB, and CargoA part prototypes in generated-content order.
pub(crate) fn semantic_part_prototypes(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    let mut sections = racer::prototypes_for(assets);
    sections.extend(cargo_b::prototypes_for(assets));
    sections.extend(cargo_a::prototypes_for(assets));
    sections
}

pub(crate) fn racer_sections() -> Vec<SpaceshipSectionConfig> {
    racer::sections()
}

pub(crate) fn cargob_sections() -> Vec<SpaceshipSectionConfig> {
    cargo_b::sections()
}

pub(crate) fn cargoa_sections(
    grade: ShipGrade,
    controller_modifications: Vec<SectionModification>,
) -> Vec<SpaceshipSectionConfig> {
    cargo_a::sections(grade, controller_modifications)
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
