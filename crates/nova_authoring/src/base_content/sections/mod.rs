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

#[cfg(test)]
mod range_tests {
    use nova_ship::prelude::{SectionKind, AI_FIRE_RANGE_FACTOR, AI_STANDOFF_OUTER_EDGE};

    use super::*;

    /// Every authored gun must reach the far edge of the orbit band its own
    /// AI flies, or the ship engages correctly and never pulls the trigger -
    /// no error, no warning, just silence. Reach is
    /// `muzzle_speed * projectile_lifetime` (a turret has no range field),
    /// and the AI fires inside `AI_FIRE_RANGE_FACTOR` of it, so this is the
    /// standing constraint on any lifetime edit - including the shorter
    /// lifetime the 100 u/s guns carry and the longer one the 60 u/s
    /// scavenger guns need to compensate.
    #[test]
    fn every_authored_turret_reaches_past_the_standoff_band() {
        let assets = BaseContentAssets::from_paths();
        let turrets: Vec<(String, f32)> = section_catalog(&assets)
            .into_iter()
            .filter_map(|section| match section.kind {
                SectionKind::Turret(turret) => Some((
                    section.base.id,
                    turret.muzzle_speed * turret.projectile_lifetime * AI_FIRE_RANGE_FACTOR,
                )),
                _ => None,
            })
            .collect();
        assert!(!turrets.is_empty(), "the catalog carries turrets");
        for (id, gate) in turrets {
            assert!(
                gate > AI_STANDOFF_OUTER_EDGE,
                "'{id}' fires out to {gate:.0}u but the standoff band reaches \
                 {AI_STANDOFF_OUTER_EDGE:.0}u - a ship carrying it would orbit \
                 outside its own reach and never fire"
            );
        }
    }
}
