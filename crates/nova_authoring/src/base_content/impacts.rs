//! The base game's impact table: what a round sounds like against what it hit.
//!
//! The shape is the point. Three rows are DEFAULTS - one per damage type, with
//! no material named - because the table falls back once and the round is the
//! half that always exists. Ship plate is what those defaults were recorded
//! against, so a section needs no row of its own.
//!
//! The rest is stone. An asteroid's material id is its KIND
//! ([`ASTEROID_KINDS`]), so the table needs a Kinetic row per kind or a rock
//! rings like a bulkhead. All four rock kinds take the same sample: the base
//! game records two impacts, plate and stone, and nickel-iron is stone as far
//! as this library goes. `plain` is a rock too - it is the control for how a
//! rock is SHADED, not a different substance - so it takes stone as well.
//!
//! Adding a material to this game is adding rows here and tagging the objects
//! made of it. Nothing else changes: no new asset kind, no code path, and a mod
//! that wants nickel-iron to ring re-declares that one row's id alone.

use nova_gameplay::prelude::{DamageType, ImpactSoundConfig};
use nova_scenario::prelude::ASTEROID_KINDS;

use super::assets::BaseContentAssets;

/// Every built-in impact row, in the order the file carries them: the three
/// damage-type defaults, then one stone row per asteroid kind.
pub(crate) fn impact_table(assets: &BaseContentAssets) -> Vec<ImpactSoundConfig> {
    let mut table = vec![
        ImpactSoundConfig {
            id: "impact_kinetic".to_string(),
            damage: DamageType::Kinetic,
            material: None,
            sound: assets.impact_kinetic_sound.clone(),
        },
        ImpactSoundConfig {
            id: "impact_pierce".to_string(),
            damage: DamageType::Pierce,
            material: None,
            sound: assets.impact_pierce_sound.clone(),
        },
        ImpactSoundConfig {
            id: "impact_explosive".to_string(),
            damage: DamageType::Explosive,
            material: None,
            sound: assets.impact_explosive_sound.clone(),
        },
    ];
    table.extend(ASTEROID_KINDS.iter().map(|kind| ImpactSoundConfig {
        id: format!("impact_kinetic_{kind}"),
        damage: DamageType::Kinetic,
        material: Some((*kind).to_string()),
        sound: assets.impact_kinetic_rock_sound.clone(),
    }));
    table
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_damage_type_has_a_default_row_so_no_round_is_silent() {
        let table = impact_table(&BaseContentAssets::from_paths());
        for kind in [
            DamageType::Kinetic,
            DamageType::Pierce,
            DamageType::Explosive,
        ] {
            assert!(
                table
                    .iter()
                    .any(|row| row.damage == kind && row.material.is_none()),
                "{kind:?} has no default row, so anything it hits is silent"
            );
        }
    }

    /// A rock's material id IS its kind id, so a kind without a row rings like
    /// ship plate. This is the join that breaks silently: adding a kind is a
    /// render change, and nothing else in the build would notice the sound.
    #[test]
    fn every_asteroid_kind_has_a_row_so_no_rock_sounds_like_hull() {
        let table = impact_table(&BaseContentAssets::from_paths());
        for kind in ASTEROID_KINDS {
            assert!(
                table.iter().any(|row| row.damage == DamageType::Kinetic
                    && row.material.as_deref() == Some(kind)),
                "{kind} has no kinetic row, so a rock of it sounds like a bulkhead"
            );
        }
    }

    #[test]
    fn no_two_rows_claim_the_same_id_or_the_same_pair() {
        let table = impact_table(&BaseContentAssets::from_paths());
        let ids: HashSet<_> = table.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids.len(), table.len(), "an id is the overlay key");
        let pairs: HashSet<_> = table
            .iter()
            .map(|row| (format!("{:?}", row.damage), row.material.clone()))
            .collect();
        assert_eq!(
            pairs.len(),
            table.len(),
            "two rows for one pair means the second can never win"
        );
    }
}
