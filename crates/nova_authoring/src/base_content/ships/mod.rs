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

#[cfg(test)]
pub(crate) use block::MAIN_DRIVE_SECTION_ID;
pub(crate) use block::PICKET_GUN_SECTION_ID;
pub use block::{
    BLOCK_BRIDGE_SECTION_ID, BLOCK_PORT_COLLAR_SECTION_ID, LINE_WARSHIP_PDC_IDS,
    MINING_TUG_EMITTER_ID,
};

/// The industrial start hull: a work boat with a mining emitter on a bow
/// boom, a cargo intake and a flank collar. Unarmed.
pub const MINING_TUG_SHIP_ID: &str = "mining_tug";
/// The scavenger start hull: the biggest start hold, a cargo intake, one
/// point-defense mount, a mining emitter and a flank collar.
pub const SALVAGE_HAULER_SHIP_ID: &str = "salvage_hauler";
/// The transport start hull: a stack and a pod of light plate on nine bell
/// drives, a cargo intake, a flank collar, unarmed.
pub const LIGHT_FREIGHTER_SHIP_ID: &str = "light_freighter";
/// The bounty hunter start hull: two point-defense mounts and a cargo intake.
pub const INTERCEPTOR_SHIP_ID: &str = "interceptor";
/// The pirate start hull: two point-defense mounts and a cargo intake.
pub const RAIDER_SHIP_ID: &str = "raider";
/// The military start hull: two point-defense mounts, a torpedo bay and a
/// cargo intake on light plate.
pub const PATROL_PICKET_SHIP_ID: &str = "patrol_picket";
/// The tutorial range's hull: one nose point-defense mount on light plate.
/// Scenario-only, with no cargo intake.
pub const TRAINING_PICKET_SHIP_ID: &str = "training_picket";
/// The Line Warship class, the military goal: six point-defense mounts, a
/// railgun, two torpedo bays and two collars.
pub const LINE_WARSHIP_SHIP_ID: &str = "line_warship";
/// A yard-fitted freighter with its stern and drive gone: a derelict to dock
/// with.
pub const FREIGHTER_DERELICT_SHIP_ID: &str = "freighter_derelict";
/// Loose debris plating: no computer, drive or gun.
pub const DEBRIS_PLATE_SHIP_ID: &str = "debris_plate";

/// Every shipped ship, in stable generated-content order.
pub(crate) fn ship_catalog(assets: &BaseContentAssets) -> Vec<ShipDesignPrototype> {
    [
        (MINING_TUG_SHIP_ID, "Mining Tug", block::mining_tug()),
        (
            SALVAGE_HAULER_SHIP_ID,
            "Salvage Hauler",
            block::salvage_hauler(),
        ),
        (
            LIGHT_FREIGHTER_SHIP_ID,
            "Light Freighter",
            block::light_freighter(),
        ),
        (INTERCEPTOR_SHIP_ID, "Interceptor", block::interceptor()),
        (RAIDER_SHIP_ID, "Raider", block::raider()),
        (
            PATROL_PICKET_SHIP_ID,
            "Patrol Picket",
            block::patrol_picket(),
        ),
        (
            TRAINING_PICKET_SHIP_ID,
            "Training Picket",
            block::training_picket(),
        ),
        (LINE_WARSHIP_SHIP_ID, "Line Warship", block::line_warship()),
        (
            FREIGHTER_DERELICT_SHIP_ID,
            "Light Freighter: Derelict",
            block::freighter_derelict(),
        ),
        (DEBRIS_PLATE_SHIP_ID, "Debris Plate", block::debris_plate()),
    ]
    .into_iter()
    .map(|(id, name, design)| block_ship(assets, id, name, design))
    .collect()
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
        id: id.into(),
        section_patches: patches.into_iter().collect(),
    }
}

/// An inline copy of the raider hull with its own collapse threshold, which
/// is a design-level field no spawn patch can reach.
pub(crate) fn inline_raider(
    assets: &BaseContentAssets,
    collapse_threshold: f32,
) -> ShipDesignSource {
    let design = block::raider();
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

/// Every section id the training picket's catalog entry carries, plates
/// included, for content that modifies the whole hull at spawn rather than one
/// named part.
pub(crate) fn picket_section_ids() -> Vec<String> {
    block::training_picket()
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

/// The waystation's hull, for the station catalog: the block hull with the
/// skin, the style and the voice every shipped design carries.
pub(crate) fn waystation_hull(assets: &BaseContentAssets) -> ShipDesign {
    block_hull(assets, block::waystation())
}

/// A BLOCK hull as a design: its section list, a derived skin, the style it
/// wears and the shipped voice.
fn block_hull(assets: &BaseContentAssets, design: block::BlockShip) -> ShipDesign {
    let style = design.style.to_string();
    let mut presentation = default_presentation(assets);
    presentation.skin = true;
    presentation.style = Some(style);
    ShipDesign {
        sections: design.sections(),
        presentation,
        ..Default::default()
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
    ShipDesignPrototype {
        id: id.into(),
        name: name.to_string(),
        design: block_hull(assets, design),
    }
}

#[cfg(test)]
mod tests {
    use nova_scenario::prelude::SectionSource;
    use nova_ship::prelude::{
        derive_link_point_graph, ship_build_fault, PlacedSectionLinkPoints, RefitPlacedSection,
        SectionKind, SectionLinkPoints,
    };

    use super::*;

    /// Every shipped ship but the two that are wreckage on purpose passes the
    /// generator's post-build checks as authored, and carries a docking port:
    /// every station service needs a dock.
    #[test]
    fn every_crewed_ship_passes_the_post_build_checks_and_docks() {
        let catalog =
            crate::base_content::sections::section_catalog(&BaseContentAssets::from_paths());
        for ship in ship_catalog(&BaseContentAssets::from_paths())
            .into_iter()
            .filter(|ship| {
                ![FREIGHTER_DERELICT_SHIP_ID, DEBRIS_PLATE_SHIP_ID].contains(&ship.id.as_str())
            })
        {
            let placed: Vec<RefitPlacedSection<'_>> = ship
                .design
                .sections
                .iter()
                .map(|section| RefitPlacedSection {
                    position: section.position,
                    rotation: section.rotation,
                    config: catalog
                        .iter()
                        .find(|config| config.base.id == section.source.prototype_id())
                        .unwrap_or_else(|| panic!("'{}' names a missing part", ship.id)),
                })
                .collect();
            assert_eq!(ship_build_fault(&placed), None, "'{}'", ship.id);
            assert!(
                placed
                    .iter()
                    .any(|section| matches!(section.config.kind, SectionKind::Docking(_))),
                "'{}' has no docking port",
                ship.id
            );
        }
    }

    /// Every shipped ship, ASSEMBLED, derives one connected structural graph.
    ///
    /// Checked on the assembly rather than on the block designs, because the
    /// design sections alone are insufficient: a turret stands on a face, so its pose
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
        for ship in ship_catalog(&BaseContentAssets::from_paths()) {
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
