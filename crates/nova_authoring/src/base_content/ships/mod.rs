//! The base game's shipped craft and the ship CONTENT entries a scenario
//! spawns them by.
//!
//! Every one of them is BLOCK-BUILT: cells on a grid wearing a derived skin,
//! with no modelled part.
//!
//! A hull variant is a second CATALOG entry, not a spawn-time flag. Thinner
//! plating or a missing drive changes the section list, so it is authored once
//! here rather than patched at every spawn.

use nova_scenario::prelude::{
    SectionId, SectionSource, ShipDesign, ShipDesignPrototype, ShipDesignSource,
    ShipIntegrityConfig, ShipPresentationConfig, SpaceshipSectionConfig,
    SpaceshipSectionConfigPatch,
};
use nova_ship::prelude::{
    AmmoCapacity, SectionConfigPatch, SectionKindPatch, TurretSectionConfigPatch,
};

use super::assets::BaseContentAssets;

mod block;

pub(crate) use block::BLOCK_CLEANUP_TURRET_ID;
pub use block::{BLOCK_BRIDGE_SECTION_ID, BLOCK_GUNSHIP_TURRET_IDS, BLOCK_PORT_COLLAR_SECTION_ID};

/// The id the block-built utility cutter is spawned by: the small unarmed
/// workboat.
pub const BLOCK_CUTTER_SHIP_ID: &str = "block_cutter";
/// The id the block-built bulk hauler is spawned by: unarmed freight on one
/// vectoring drive.
pub const BLOCK_HAULER_SHIP_ID: &str = "block_hauler";
/// The id the block-built civilian workship is spawned by: an open work
/// cradle, a port-flank docking collar, and no gun. Season one's player
/// hull.
pub const BLOCK_WORKSHIP_SHIP_ID: &str = "block_workship";
/// The id the block-built frame tender is spawned by: a cargo body under two
/// open gantry arches, carrying the same collar the workship docks on.
pub const BLOCK_FRAME_TENDER_SHIP_ID: &str = "block_frame_tender";
/// The id the damaged frame tender is spawned by: the same hull with its
/// stern and its main drive gone. A second CATALOG entry for the reason the
/// module doc gives.
pub const BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID: &str = "block_frame_tender_damaged";

/// The id the block-built patrol gunship is spawned by: armoured, with six
/// point-defense mounts.
pub const BLOCK_GUNSHIP_SHIP_ID: &str = "block_gunship";
/// The id the block-built picket is spawned by: one nose gun on thin
/// plating.
pub const BLOCK_PICKET_SHIP_ID: &str = "block_picket";
/// The id loose debris plating is spawned by: no computer, drive or gun.
pub const BLOCK_WRECK_PLATE_SHIP_ID: &str = "block_wreck_plate";

/// Every shipped ship, in stable generated-content order.
pub(crate) fn ship_catalog(assets: &BaseContentAssets) -> Vec<ShipDesignPrototype> {
    vec![
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
            BLOCK_WORKSHIP_SHIP_ID,
            "Utility Workship",
            block::utility_workship(),
        ),
        block_ship(
            assets,
            BLOCK_FRAME_TENDER_SHIP_ID,
            "Frame Tender",
            block::frame_tender(),
        ),
        block_ship(
            assets,
            BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID,
            "Frame Tender: Damaged",
            block::damaged_frame_tender(),
        ),
        block_ship(
            assets,
            BLOCK_GUNSHIP_SHIP_ID,
            "Patrol Gunship",
            block::patrol_gunship(),
        ),
        block_ship(
            assets,
            BLOCK_PICKET_SHIP_ID,
            "Salvage Picket",
            block::salvage_picket(),
        ),
        block_ship(
            assets,
            BLOCK_WRECK_PLATE_SHIP_ID,
            "Carrier Wreck: Plating",
            block::carrier_wreck_plate(),
        ),
    ]
}

/// A spawn of one CATALOG design, by id, with nothing patched.
pub fn design(id: &str) -> ShipDesignSource {
    ShipDesignSource::prototype(id)
}

/// A spawn of one CATALOG design with this spawn's own section patches.
pub(crate) fn patched_design(
    id: &str,
    patches: impl IntoIterator<Item = (SectionId, SpaceshipSectionConfigPatch)>,
) -> ShipDesignSource {
    ShipDesignSource::Prototype {
        id: id.to_string(),
        section_patches: patches.into_iter().collect(),
    }
}

/// An inline copy of the raider hull with its own collapse threshold, which
/// is a design-level field no spawn patch can reach.
pub(crate) fn inline_raider(
    assets: &BaseContentAssets,
    collapse_threshold: f32,
) -> ShipDesignSource {
    let design = block::salvage_raider();
    let style = design.style.to_string();
    ShipDesignSource::Inline(ShipDesign {
        sections: design.sections(),
        integrity: ShipIntegrityConfig {
            collapse_threshold: Some(collapse_threshold),
        },
        presentation: ShipPresentationConfig {
            skin: true,
            style: Some(style),
            collapse_sound: Some(assets.ship_collapse_sound.clone()),
            ..default_presentation(assets)
        },
    })
}

/// A ONE-OFF design, authored inline: a scripted battery that is a single tube,
/// a derelict that is five plates. Anything a second scenario would want gets a
/// catalog entry instead.
pub(crate) fn inline_design(sections: Vec<SpaceshipSectionConfig>) -> ShipDesignSource {
    ShipDesignSource::Inline(ShipDesign {
        sections,
        ..Default::default()
    })
}

/// Every section id the picket's catalog entry carries, plates included, for
/// content that modifies the whole hull at spawn rather than one named part.
pub(crate) fn picket_section_ids() -> Vec<String> {
    block::salvage_picket()
        .sections()
        .into_iter()
        .map(|section| section.id)
        .collect()
}

/// One spawn-time patch aimed at a named section of the resolved design.
pub(crate) fn on_section(
    section: &str,
    patch: SpaceshipSectionConfigPatch,
) -> (SectionId, SpaceshipSectionConfigPatch) {
    (section.to_string(), patch)
}

/// A spawn patch that gives one section its own health and changes nothing
/// else about it.
pub(crate) fn section_health(health: f32) -> SpaceshipSectionConfigPatch {
    SpaceshipSectionConfigPatch {
        config: SectionConfigPatch {
            health: Some(health),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Put one section patch on a design source, whichever shape it has.
///
/// A prototype spawn carries the patch in its own spawn layer, over whatever
/// the catalog design says. An inline design has no spawn layer, so the patch
/// lands on the section itself - on its catalog reference when the section
/// names a prototype, and on the config when the section is written out in
/// full.
pub(crate) fn patched_section(
    design: ShipDesignSource,
    section: &str,
    patch: SpaceshipSectionConfigPatch,
) -> ShipDesignSource {
    match design {
        ShipDesignSource::Prototype {
            id,
            mut section_patches,
        } => {
            section_patches.insert(section.to_string(), patch);
            ShipDesignSource::Prototype {
                id,
                section_patches,
            }
        }
        ShipDesignSource::Inline(mut design) => {
            for entry in &mut design.sections {
                if entry.id != section {
                    continue;
                }
                if let Some(position) = patch.position {
                    entry.position = position;
                }
                if let Some(rotation) = patch.rotation {
                    entry.rotation = rotation;
                }
                match &mut entry.source {
                    SectionSource::Prototype {
                        patch: reference, ..
                    } => reference.clone_from(&patch.config),
                    SectionSource::Inline(config) => patch
                        .config
                        .apply(config)
                        .expect("a shipped inline patch names its own section's kind"),
                }
            }
            ShipDesignSource::Inline(design)
        }
    }
}

/// A spawn patch that gives one TURRET section its own magazine and changes
/// nothing else about it.
pub(crate) fn turret_magazine(rounds: u32) -> SpaceshipSectionConfigPatch {
    SpaceshipSectionConfigPatch {
        config: SectionConfigPatch {
            kind: Some(SectionKindPatch::Turret(TurretSectionConfigPatch {
                ammunition: Some(AmmoCapacity::Limited(rounds)),
                ..Default::default()
            })),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// The sound set every shipped design carries. These sounds belong to the
/// game rather than to one hull, so they are authored once here.
pub(crate) fn default_presentation(assets: &BaseContentAssets) -> ShipPresentationConfig {
    ShipPresentationConfig {
        collapse_sound: Some(assets.ship_collapse_sound.clone()),
        lock_on_sound: Some(assets.ship_lock_on_sound.clone()),
        lock_off_sound: Some(assets.ship_lock_off_sound.clone()),
        radar_deny_sound: Some(assets.ship_radar_deny_sound.clone()),
        radar_retarget_sound: Some(assets.ship_radar_retarget_sound.clone()),
        safety_on_sound: Some(assets.ship_safety_on_sound.clone()),
        warn_lock_sound: Some(assets.ship_warn_lock_sound.clone()),
        ammo_dry_sound: Some(assets.ship_ammo_dry_sound.clone()),
        warn_hull_sound: Some(assets.ship_warn_hull_sound.clone()),
        rcs_loop_sound: Some(assets.ship_rcs_loop_sound.clone()),
        ..ShipPresentationConfig::default()
    }
}

/// One catalog entry over a built section list. Every shipped ship keeps the
/// engine's default collapse threshold.
fn ship(
    assets: &BaseContentAssets,
    id: &str,
    name: &str,
    sections: Vec<SpaceshipSectionConfig>,
) -> ShipDesignPrototype {
    ShipDesignPrototype {
        id: id.to_string(),
        name: name.to_string(),
        design: ShipDesign {
            sections,
            presentation: default_presentation(assets),
            ..Default::default()
        },
    }
}

/// One catalog entry over a BLOCK hull: the entry above plus the two fields a
/// cell-built hull carries, a derived skin and the style it wears.
fn block_ship(
    assets: &BaseContentAssets,
    id: &str,
    name: &str,
    design: block::BlockShip,
) -> ShipDesignPrototype {
    let style = design.style.to_string();
    let mut config = ship(assets, id, name, design.sections());
    config.design.presentation.skin = true;
    config.design.presentation.style = Some(style);
    config
}

#[cfg(test)]
mod tests {
    use nova_scenario::prelude::SectionSource;
    use nova_ship::prelude::{derive_link_point_graph, PlacedSectionLinkPoints, SectionLinkPoints};

    use super::BaseContentAssets;

    /// Every shipped ship, ASSEMBLED, derives one connected structural graph.
    ///
    /// Checked on the assembly rather than on the block designs, because the
    /// designs are not the whole story: a turret stands on a face, so its pose
    /// is derived from the cell it bolts to. Only the assembled ship exercises
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
                .design
                .sections
                .iter()
                .map(|section| {
                    let SectionSource::Prototype { id, .. } = &section.source else {
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
                .design
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
                mates.len() >= ship.design.sections.len() - 1,
                "ship '{}' holds together on only {} mates",
                ship.id,
                mates.len(),
            );
        }
    }
}
