//! The base game's impact table: what a round sounds like against what it hit.
//!
//! Four rows, and the shape is the point. Three are DEFAULTS - one per damage
//! type, with no material named - because the table falls back once and the
//! round is the half that always exists. Only stone earns a second row, because
//! it is the only thing in the base game that is not ship plate.
//!
//! Adding a material to this game is adding rows here and tagging the objects
//! made of it. Nothing else changes: no new asset kind, no code path, and a mod
//! overriding one pair re-declares that row's id alone.

use nova_gameplay::prelude::{DamageType, ImpactSoundConfig, MATERIAL_ROCK};

use super::assets::BaseContentAssets;

/// Every built-in impact row, in the order the file carries them.
pub(crate) fn impact_table(assets: &BaseContentAssets) -> Vec<ImpactSoundConfig> {
    vec![
        ImpactSoundConfig {
            id: "impact_kinetic".to_string(),
            damage: DamageType::Kinetic,
            material: None,
            sound: assets.impact_kinetic_sound.clone(),
        },
        ImpactSoundConfig {
            id: "impact_kinetic_rock".to_string(),
            damage: DamageType::Kinetic,
            material: Some(MATERIAL_ROCK.to_string()),
            sound: assets.impact_kinetic_rock_sound.clone(),
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
    ]
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
