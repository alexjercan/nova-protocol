//! The base campaign's voices, and the portrait pass that gives them faces.
//!
//! One constant per speaker, so a rename is a one-line change. The names are
//! the story's: the cutter's three, the two people on the Meridian who talk to
//! them, the guard channel nobody listens to, and afterwards, nobody.
//!
//! A speaker is a NAME, not a channel. Which channel a line goes out on is
//! chosen at the line - `comms` for the work circuit, `crew` for the cabin,
//! `guard` for the traffic that is not addressed to this ship - so the same
//! person can be on the radio in one beat and in the room in the next.

use bevy::prelude::Image;
use nova_gameplay::prelude::AssetRef;
use nova_scenario::prelude::*;

use crate::base_content::assets::CampaignPortraits;

/// The carrier's own name, used in objective and banner text. The ship is a
/// character in the shift and a grave afterwards, so it is named in one
/// place.
pub(crate) const CARRIER_NAME: &str = "Meridian";

/// The player's ship, named in objective text and used as its scenario id. A
/// working cutter with a crew on it, not a generic `player_spaceship`: the
/// chapter's ending is about people, and the ship has to be one of them.
pub(crate) const CUTTER_NAME: &str = "Cutter One";

/// Yusra Demir: Meridian Control. The work channel for the whole shift, and
/// the last voice off the carrier. She is the reason the guard channel reads
/// as wrong when it finally says something - it is not her.
pub(crate) const DEMIR: &str = "Demir";

/// Aurelio Brandt: Meridian's deck chief. Taught Pell to fit boats and Okoro
/// after him, and owns the renewal paperwork the shift's joke is about.
pub(crate) const BRANDT: &str = "Brandt";

/// Nadia Halloran: the seat beside the player's. Runs the checklist, reads the
/// tape, says what everyone is thinking and does nothing about it.
pub(crate) const HALLORAN: &str = "Halloran";

/// Bastian Okoro: down the back with the crates. Learned this deck from a dead
/// man and stops at his plaque without saying so.
pub(crate) const OKORO: &str = "Okoro";

/// The player's own comms label: a plain "You", not a callsign. The captain
/// has no name in this story and does not need one - they are the only person
/// on the crew with an Earth to go back to, and that is their whole character.
pub(crate) const PLAYER: &str = "You";

/// The guard channel, which is not addressed to anyone on this ship.
///
/// It carries Fleet challenge traffic in fragments all shift and nobody cares.
/// It is the same label, with the same face, that reads the clause over the
/// Meridian - which is the point: the channel that was noise for an hour is
/// the channel that kills four hundred and twelve people.
pub(crate) const GUARD_VOICE: &str = "Unknown Signal";

/// The wreck's automatic beacon, the only thing still transmitting at the end
/// of the chapter and the reason the next one happens.
pub(crate) const BEACON: &str = "Automated Beacon";

fn portrait(portraits: &CampaignPortraits, speaker: &str) -> Option<AssetRef<Image>> {
    let found = match speaker {
        DEMIR => &portraits.meridian_control,
        BRANDT => &portraits.deck_chief,
        HALLORAN => &portraits.copilot,
        OKORO => &portraits.engineer,
        PLAYER => &portraits.player,
        BEACON => &portraits.automated_beacon,
        GUARD_VOICE => &portraits.unknown_channel,
        _ => return None,
    };
    Some(found.clone())
}

/// Attach the base campaign's shared speaker portraits to every narrative cue,
/// however deeply nested, while leaving unknown and preview-only speakers on
/// the HUD fallback.
pub(crate) fn apply_portraits(portraits: &CampaignPortraits, events: &mut [ScenarioEventConfig]) {
    for event in events {
        for action in &mut event.actions {
            action.walk_mut(&mut |action| {
                if let EventActionConfig::NarrativeCue(cue) = action {
                    if cue.icon.is_none() {
                        cue.icon = portrait(portraits, &cue.speaker);
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{base_content::assets::BaseContentAssets, scenario_helpers::prelude::*};

    /// Every voice the campaign speaks with resolves to its OWN portrait. A
    /// `self://` reference only resolves inside a mod bundle, so this also
    /// pins that the generated set is the one place those sentinels are
    /// written.
    #[test]
    fn every_campaign_voice_has_its_own_portrait() {
        let portraits = BaseContentAssets::from_paths().portraits;
        let path = |speaker| {
            portrait(&portraits, speaker)
                .unwrap_or_else(|| panic!("'{speaker}' has no portrait"))
                .path()
                .expect("the generated set is authored as paths")
                .to_string()
        };
        let mut faces = [DEMIR, BRANDT, HALLORAN, OKORO, PLAYER, BEACON, GUARD_VOICE].map(path);
        let count = faces.len();
        faces.sort();
        let mut distinct = faces.to_vec();
        distinct.dedup();
        assert_eq!(distinct.len(), count, "each voice has its own face");
        assert!(
            portrait(&portraits, "PREVIEW").is_none(),
            "preview scaffolding must retain the HUD fallback"
        );
    }

    /// A line inside a scene is a line. The strike is authored as one
    /// cinematic, so a portrait pass that only walked `Sequence` would leave
    /// every voice in the chapter's ending on the fallback face.
    #[test]
    fn a_line_buried_inside_a_scene_still_gets_a_face() {
        let portraits = BaseContentAssets::from_paths().portraits;
        let mut events = vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: true,
            filters: vec![],
            actions: vec![cinematic(
                "strike",
                true,
                vec![step(
                    0.0,
                    vec![sequence(
                        "inner",
                        vec![step(0.0, vec![guard(GUARD_VOICE, "...roster...")])],
                    )],
                )],
            )],
        }];

        apply_portraits(&portraits, &mut events);

        let mut icons = Vec::new();
        events[0].actions[0].walk(&mut |action| {
            if let EventActionConfig::NarrativeCue(cue) = action {
                icons.push(cue.icon.clone());
            }
        });
        assert_eq!(icons.len(), 1, "the buried line was not reached");
        assert!(
            icons[0].is_some(),
            "a line inside a scene kept the fallback"
        );
    }
}
