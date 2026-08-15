//! Built-in section-prototype content.
//!
//! `standard` owns generic mountable modules; semantic body-part prototypes
//! live with their complete craft definitions under `ships`. This module joins
//! both sources into the one generated section catalog.
//!
//! CLADDING is not here and is not a prototype at all. A ship's skin is DERIVED
//! from the structure it wraps - see `nova_ship`'s `shell_skin` - so no id names
//! a plate and nothing places one by hand.

use nova_ship::prelude::SectionConfig;

use super::{assets::BaseContentAssets, ships};

mod standard;

pub(crate) use standard::{turret_joint_tree, UNIT_TURRET_MOUNT, UNIT_TURRET_SCALE};

/// Generic hull, controller, thruster, turret, and torpedo prototypes.
pub(crate) fn standard_section_prototypes(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    standard::standard_section_prototypes(assets)
}

/// Complete built-in prototype catalog in stable generated-content order.
pub(crate) fn section_catalog(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    let mut sections = standard_section_prototypes(assets);
    sections.extend(ships::semantic_part_prototypes(assets));
    sections
}
