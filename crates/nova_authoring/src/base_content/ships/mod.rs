//! Shipped semantic craft assemblies, their section prototypes, and the ship
//! CONTENT entries a scenario spawns them by.
//!
//! A grade is a build-time knob, not a spawn-time one: the raider corvette
//! carries thinner plating and mounts that are quicker to shoot off, which is a
//! different ship to fight and to read about, so it is a second CATALOG entry
//! rather than a flag a scenario flips. Two entries cost one line each here and no machinery
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

mod block;
mod cargo_a;
mod cargo_b;
mod racer;
mod shared;

pub(crate) use block::{BLOCK_BRIDGE_SECTION_ID, BLOCK_GUNSHIP_TURRET_IDS};
pub(crate) use cargo_a::CARGOA_TURRET_IDS;
use shared::{Ordnance, ShipGrade};

/// The id the player-grade CargoA corvette is spawned by.
pub(crate) const CARGOA_SHIP_ID: &str = "cargoa";
/// The id the scavenger-grade CargoA corvette is spawned by: thinner plating,
/// flimsier gun mounts, a softer flight computer. The gun itself is the same
/// shared PDC every craft carries.
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

/// The id the block-built utility cutter is spawned by: the small unarmed
/// workboat, and the base game's plainest craft.
pub(crate) const BLOCK_CUTTER_SHIP_ID: &str = "block_cutter";
/// The id the block-built bulk hauler is spawned by: unarmed freight on one
/// vectoring drive.
pub(crate) const BLOCK_HAULER_SHIP_ID: &str = "block_hauler";
/// The id the block-built patrol gunship is spawned by: armoured, six point
/// defense mounts, the fleet's warship.
pub(crate) const BLOCK_GUNSHIP_SHIP_ID: &str = "block_gunship";
/// The id the block-built salvage raider is spawned by: the same tonnage worn
/// down to two guns and a scrap boom, in the scavenger look.
pub(crate) const BLOCK_RAIDER_SHIP_ID: &str = "block_raider";

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
pub(crate) fn ship_catalog(assets: &BaseContentAssets) -> Vec<ShipConfig> {
    vec![
        ship(assets, RACER_SHIP_ID, "Racer Yacht", racer::sections()),
        ship(
            assets,
            CARGOB_SHIP_ID,
            "CargoB Hauler",
            cargo_b::sections(Ordnance::Serpent),
        ),
        ship(
            assets,
            CARGOB_LANCE_SHIP_ID,
            "CargoB Hauler (Lance)",
            cargo_b::sections(Ordnance::Lance),
        ),
        ship(
            assets,
            CARGOA_SHIP_ID,
            "CargoA Corvette",
            cargo_a::sections(ShipGrade::Player),
        ),
        ship(
            assets,
            CARGOA_RAIDER_SHIP_ID,
            "CargoA Raider Corvette",
            cargo_a::sections(ShipGrade::Enemy),
        ),
        block_ship(
            assets,
            BLOCK_CUTTER_SHIP_ID,
            "Utility Cutter",
            block::utility_cutter(),
        ),
        block_ship(
            assets,
            BLOCK_HAULER_SHIP_ID,
            "Bulk Hauler",
            block::bulk_hauler(),
        ),
        block_ship(
            assets,
            BLOCK_GUNSHIP_SHIP_ID,
            "Patrol Gunship",
            block::patrol_gunship(),
        ),
        block_ship(
            assets,
            BLOCK_RAIDER_SHIP_ID,
            "Salvage Raider",
            block::salvage_raider(),
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
/// engine's collapse threshold and goes unclad, so the hull is its sections
/// plus the one voice no section can own: the ship coming apart.
fn ship(
    assets: &BaseContentAssets,
    id: &str,
    name: &str,
    sections: Vec<SpaceshipSectionConfig>,
) -> ShipConfig {
    ShipConfig {
        id: id.to_string(),
        name: name.to_string(),
        hull: ShipHull {
            sections,
            collapse_sound: Some(assets.ship_collapse_sound.clone()),
            ..Default::default()
        },
    }
}

/// One catalog entry over a BLOCK hull. The same entry as above plus the two
/// fields the modelled fleet cannot have: a derived skin, which reads a hull as
/// unit cells, and the style it wears.
fn block_ship(
    assets: &BaseContentAssets,
    id: &str,
    name: &str,
    design: block::BlockShip,
) -> ShipConfig {
    let style = design.style.to_string();
    let mut config = ship(assets, id, name, design.sections());
    config.hull.skin = true;
    config.hull.style = Some(style);
    config
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec3;
    use nova_scenario::prelude::SectionSource;
    use nova_ship::prelude::{
        cardinal_axis, derive_link_point_graph, snap_placement, unit_cube_link_points,
        PlacedSectionLinkPoints, SectionLinkPoints,
    };

    use super::{cargo_a::*, cargo_b::*, racer::*, shared::*, BaseContentAssets};

    /// Every shipped ship, ASSEMBLED, derives one connected structural graph.
    ///
    /// Checked on the assembly rather than on the part specs, because the specs
    /// are no longer the whole story: a turret mount contributes no prototype
    /// of its own, so its sockets come from the shared PDC and its pose is
    /// derived from the face it stands on. Only the assembled ship exercises
    /// that, and only the assembled ship is what the game spawns.
    ///
    /// `derive_link_point_graph` is all-or-nothing: one mount whose socket
    /// misses by more than the mate epsilon leaves it in its own component, the
    /// whole ship comes back `Disconnected`, and section integrity falls back
    /// to EMPTY adjacency - under which any single section death severs the
    /// entire hull into loose wrecks rather than shearing off what hung on it.
    /// A silent tenth of a unit is enough to do it.
    #[test]
    fn every_shipped_ship_has_one_connected_mate_graph() {
        let catalog = crate::generation::build_section_catalog();
        for ship in super::ship_catalog(&BaseContentAssets::from_paths()) {
            let sockets: Vec<_> = ship
                .hull
                .sections
                .iter()
                .map(|section| {
                    let SectionSource::Prototype(id) = &section.source else {
                        panic!(
                            "ship '{}' section '{}' is not a prototype",
                            ship.id, section.id
                        )
                    };
                    let prototype = catalog
                        .iter()
                        .find(|candidate| candidate.base.id == *id)
                        .unwrap_or_else(|| {
                            panic!("ship '{}' names missing prototype '{id}'", ship.id)
                        });
                    SectionLinkPoints(prototype.base.link_points.clone())
                })
                .collect();
            let placed: Vec<_> = ship
                .hull
                .sections
                .iter()
                .zip(&sockets)
                .map(|(section, sockets)| PlacedSectionLinkPoints {
                    position: section.position,
                    rotation: section.rotation,
                    link_points: sockets,
                })
                .collect();
            // Deriving at all IS the assertion: the call returns `Disconnected`
            // unless every section - turret mounts included - joined the one
            // component. The count guard only catches a graph that connected
            // through some accident while leaving a section dangling.
            let mates = derive_link_point_graph(&placed)
                .unwrap_or_else(|errors| panic!("ship '{}' does not mate: {errors:?}", ship.id));
            assert!(
                mates.len() >= ship.hull.sections.len() - 1,
                "ship '{}' holds together on only {} mates",
                ship.id,
                mates.len(),
            );
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
            // The meshed parts only: a turret mount contributes no prototype,
            // so it has no sockets of its own to mate with.
            for index in 0..specs.len() - 2 {
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
