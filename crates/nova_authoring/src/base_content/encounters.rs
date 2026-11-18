//! Built-in ENCOUNTER content: what the open world may put ahead of the
//! player on a long flight.
//!
//! `content gen` serializes this builder into
//! `assets/base/encounters/base.content.ron`. The weights are provisional:
//! the two finds a hull can answer without a fight outweigh the ambush.

use nova_gameplay::prelude::{EncounterDesign, EncounterPlayerType, EncounterType, HailJobType};

/// The base game's encounters.
pub(crate) fn encounter_catalog() -> Vec<EncounterDesign> {
    vec![
        EncounterDesign {
            id: "base/distress_call".into(),
            weight: 3,
            player: EncounterPlayerType::Any,
            kind: EncounterType::Hail {
                job: HailJobType::Distress,
                line: "Mayday, mayday. Our drive is dead and we are adrift. Any ship, please dock \
                       with us."
                    .to_string(),
            },
        },
        EncounterDesign {
            id: "base/supply_call".into(),
            weight: 2,
            player: EncounterPlayerType::Any,
            kind: EncounterType::Hail {
                job: HailJobType::Supply,
                line: "Any ship, please respond. We are stranded and out of supplies. Can you \
                       bring some?"
                    .to_string(),
            },
        },
        // A hull with no weapon cannot answer raiders, so it never meets them.
        EncounterDesign {
            id: "base/ambush".into(),
            weight: 2,
            player: EncounterPlayerType::Armed,
            kind: EncounterType::Ambush,
        },
        EncounterDesign {
            id: "base/derelict_find".into(),
            weight: 3,
            player: EncounterPlayerType::Any,
            kind: EncounterType::Derelict,
        },
    ]
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::encounter_faults;

    use super::*;

    #[test]
    fn base_encounters_have_no_faults_and_no_id_twice() {
        let catalog = encounter_catalog();
        for (index, encounter) in catalog.iter().enumerate() {
            assert_eq!(
                encounter_faults(encounter),
                Vec::<String>::new(),
                "{}",
                encounter.id
            );
            assert!(
                catalog[..index]
                    .iter()
                    .all(|other| other.id != encounter.id),
                "{} is defined twice",
                encounter.id
            );
        }
    }

    #[test]
    fn a_base_hail_line_fits_90_characters() {
        for encounter in encounter_catalog() {
            if let EncounterType::Hail { line, .. } = &encounter.kind {
                assert!(line.chars().count() <= 90, "{}: {line}", encounter.id);
            }
        }
    }
}
