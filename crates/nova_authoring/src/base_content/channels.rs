//! The base game's narrative channels: the three places a line is heard from.
//!
//! Three rows, because the cutter has three: the radio traffic a shift is paid
//! to answer, the two people in the cabin, and everything else on the air. The
//! panel draws a card in whichever the line arrived on, so the split is what
//! makes a fragment read as OVERHEARD rather than addressed.
//!
//! Only the guard channel is tagged. The work channel is what the comms panel
//! IS, and the crew are in the room; tagging either would put a label on every
//! line to distinguish it from nothing. Signal strength does the same job the
//! other way - a line the ship was sent is at full strength, and a fragment the
//! cockpit merely caught is not.
//!
//! Adding a channel to this game is adding a row here and naming it on a cue.
//! Nothing else changes, and a mod that wants the work channel in its own
//! colours re-declares this one row's id.

use nova_gameplay::prelude::{
    ChipTone, NarrativeChannelConfig, CHANNEL_COMMS, CHANNEL_CREW, CHANNEL_GUARD,
};

/// How faintly a guard-channel card is drawn against a line the ship was sent.
const GUARD_SIGNAL_STRENGTH: f32 = 0.7;

/// Every built-in channel, in the order the file carries them.
pub(crate) fn channel_catalog() -> Vec<NarrativeChannelConfig> {
    vec![
        NarrativeChannelConfig::new(CHANNEL_COMMS, ChipTone::Comms),
        NarrativeChannelConfig::new(CHANNEL_CREW, ChipTone::Phosphor),
        NarrativeChannelConfig::new(CHANNEL_GUARD, ChipTone::Amber)
            .with_tag("GUARD")
            .with_signal_strength(GUARD_SIGNAL_STRENGTH),
    ]
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// A channel is told apart by the colour it is drawn in before it is read.
    /// Two channels sharing a tone would make the panel's whole reason for
    /// existing invisible - the player would have to read the tag to know who
    /// was talking to them.
    #[test]
    fn no_two_base_channels_share_a_tone() {
        let catalog = channel_catalog();
        let tones: HashSet<ChipTone> = catalog.iter().map(|channel| channel.tone).collect();
        assert_eq!(
            tones.len(),
            catalog.len(),
            "two base channels are drawn in the same tone"
        );
    }

    /// The tag marks a line the ship was NOT sent. On every channel it marks
    /// nothing.
    #[test]
    fn only_the_guard_channel_is_tagged_and_only_it_is_weak() {
        for channel in channel_catalog() {
            let overheard = channel.id == CHANNEL_GUARD;
            assert_eq!(
                channel.tag.is_some(),
                overheard,
                "channel '{}' tags the wrong way round",
                channel.id
            );
            assert_eq!(
                channel.signal_strength < 1.0,
                overheard,
                "channel '{}' is drawn at the wrong strength",
                channel.id
            );
        }
    }
}
