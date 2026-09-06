//! Narrative channels: WHERE a story line was heard, as authored content.
//!
//! The comms panel draws a line differently depending on the channel it came
//! down - the work channel is addressed to this ship, the crew are in the room,
//! and the guard channel is traffic nobody sent you. That difference is the
//! whole point of the panel, and until this module it was a closed enum in game
//! code with a lookup table hanging off it. A mod could not add a channel, only
//! borrow one of ours.
//!
//! A channel is content for the same reason a style or an impact row is. It
//! carries no behaviour - every one of its fields is a presentation fact the
//! panel reads - so an enum bought nothing but a place for mods to be locked
//! out of. A campaign that wants a distress band, a corporate net or a
//! smuggler's private channel authors one and its lines are drawn in it.
//!
//! It lives here rather than beside the panel because both ends need it: the
//! HUD reads a resolved channel off each line, and the scenario layer resolves
//! the id a cue names. `StoryFeed` is split the same way and for the same
//! reason - the HUD cannot depend on the scenario crate.

use bevy::prelude::*;
pub use nova_ui::hud::ChipTone;

/// The authored channel, the merged catalog, and the ids the base game ships.
///
/// [`ChipTone`] rides along because it is the type of an authored field: a
/// crate that writes a channel must be able to name its tone without taking a
/// dependency on the widget crate the palette lives in.
pub mod prelude {
    pub use super::{
        ChipTone, GameChannels, NarrativeChannelConfig, CHANNEL_COMMS, CHANNEL_CREW, CHANNEL_GUARD,
    };
}

/// The work channel: traffic addressed to this ship.
pub const CHANNEL_COMMS: &str = "comms";

/// Inside the hull, off the radio entirely - the crew talking to each other.
pub const CHANNEL_CREW: &str = "crew";

/// The guard channel: everybody's channel, nobody's conversation.
pub const CHANNEL_GUARD: &str = "guard";

/// One authored narrative channel: how a line heard on it is drawn.
///
/// Not a faction and not a speaker. The same person reaches the cockpit down
/// two different channels and the difference matters - Meridian Control on the
/// work channel is addressed to you, and the same desk read over the guard
/// channel is something you are overhearing.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NarrativeChannelConfig {
    /// Stable content id, and what a cue names. The overlay key: a mod
    /// re-declaring this id replaces the channel in place, so a total
    /// conversion can restyle the work channel without touching a line of
    /// dialogue.
    pub id: String,
    /// The chip family the card is drawn in - its frame, its header, and
    /// (lifted toward white) the line the player reads. One field rather than
    /// four colours, so a mod's channel lands inside the CRT palette instead of
    /// beside it.
    pub tone: ChipTone,
    /// The channel name shown beside the speaker.
    ///
    /// An override in the sense the authoring rule reserves `Option` for:
    /// absent means the channel needs no saying. Tag the ones the player has to
    /// tell apart from the panel's own default voice - a tag on every line
    /// distinguishes it from nothing.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub tag: Option<String>,
    /// How strongly the card is drawn, 0 to 1. A line the ship was sent is at
    /// full strength; a fragment the cockpit merely CAUGHT is not.
    ///
    /// One dial rather than a second set of colours: it fades frame, header and
    /// body together, so a weak channel stays recognisably itself.
    pub signal_strength: f32,
}

impl NarrativeChannelConfig {
    /// An untagged channel at full signal: the plain case, which a caller then
    /// adjusts with [`Self::with_tag`] and [`Self::with_signal_strength`].
    pub fn new(id: impl Into<String>, tone: ChipTone) -> Self {
        Self {
            id: id.into(),
            tone,
            tag: None,
            signal_strength: 1.0,
        }
    }

    /// Name the channel beside the speaker.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Draw the channel at less than full strength.
    pub fn with_signal_strength(mut self, strength: f32) -> Self {
        self.signal_strength = strength;
        self
    }

    /// The colour of the line the player actually reads: the tone's accent as
    /// long-form reading copy.
    pub fn body(&self) -> Color {
        self.tone.body()
    }
}

/// The merged channel catalog, in registration order (base then mods),
/// overlaid last-wins by [`NarrativeChannelConfig::id`].
///
/// Inserted by the content merge and init'd empty, so a rig that loads no
/// content resolves nothing rather than panicking on a missing resource.
#[derive(Resource, Clone, Debug, Default)]
pub struct GameChannels(pub Vec<NarrativeChannelConfig>);

impl GameChannels {
    /// The channel with this id, or `None` if nothing authored it.
    ///
    /// A cue naming an unauthored channel is refused at lint and again at load,
    /// so a `None` here means a rig that skipped the catalog rather than a
    /// scenario that reached the player.
    pub fn get_channel(&self, id: &str) -> Option<&NarrativeChannelConfig> {
        self.0.iter().find(|channel| channel.id == id)
    }

    /// The channel to DRAW a line on, for a caller that has a line either way.
    ///
    /// An unauthored id is refused at lint and again at load, so reaching the
    /// fallback means a rig that never merged content. It draws the line on the
    /// HUD's default voice and logs, because a story line the player never sees
    /// is a worse failure than one drawn plainly - and silence would leave the
    /// rig looking like it works.
    pub fn resolve(&self, id: &str) -> NarrativeChannelConfig {
        self.get_channel(id).cloned().unwrap_or_else(|| {
            error!("narrative cue names channel '{id}', which no content authors");
            NarrativeChannelConfig {
                id: id.to_string(),
                tone: ChipTone::default(),
                tag: None,
                signal_strength: 1.0,
            }
        })
    }
}
