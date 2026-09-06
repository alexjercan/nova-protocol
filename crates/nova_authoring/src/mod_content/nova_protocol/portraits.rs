//! The story's comms portraits, one per speaker.

use bevy::prelude::Image;
use nova_gameplay::prelude::AssetRef;

use super::MOD_ID;

/// The speaker portraits the story attaches to its lines.
///
/// Grouped rather than passed loose because every story builder needs the
/// whole set, and because the set is what a caller has to source: the RON
/// generator writes the mod's own `self://` and `dep://base/` refs, and an
/// in-process caller (the preview benches) loads the same files by their
/// shipped paths. A `self://` sentinel that reaches the asset server outside
/// a bundle loads nothing.
#[derive(Clone)]
pub struct CampaignPortraits {
    /// The carrier's duty channel.
    pub meridian_control: AssetRef<Image>,
    /// The player's supervisor on the maintenance deck.
    pub deck_chief: AssetRef<Image>,
    /// The seat beside the player's, on both the work and cabin channels.
    pub copilot: AssetRef<Image>,
    /// Down the back with the crates.
    pub engineer: AssetRef<Image>,
    /// The player's own comms label. The base game's face, shared with the
    /// training range.
    pub player: AssetRef<Image>,
    /// The wreck's automatic beacon.
    pub automated_beacon: AssetRef<Image>,
    /// The guard channel: Fleet challenge traffic all shift, and the voice that
    /// reads the clause over the Meridian.
    pub unknown_channel: AssetRef<Image>,
}

impl CampaignPortraits {
    /// Generation source: the refs the mod's generated files carry. The
    /// story's own faces resolve against its bundle, the player's against
    /// base.
    pub(crate) fn authored() -> Self {
        Self::from_paths(
            |name| format!("self://portraits/{name}.png"),
            "dep://base/portraits/player.png".to_string(),
        )
    }

    /// The shipped files a running app loads the same faces from, for a
    /// bench that builds the story in-process rather than through the merge.
    pub fn shipped() -> Self {
        Self::from_paths(
            |name| format!("mods/{MOD_ID}/portraits/{name}.png"),
            "base/portraits/player.png".to_string(),
        )
    }

    fn from_paths(own: impl Fn(&str) -> String, player: String) -> Self {
        let portrait = |name: &str| AssetRef::from(own(name));
        Self {
            meridian_control: portrait("meridian-control"),
            deck_chief: portrait("deck-chief"),
            copilot: portrait("copilot"),
            engineer: portrait("engineer"),
            player: AssetRef::from(player),
            automated_beacon: portrait("automated-beacon"),
            unknown_channel: portrait("unknown-channel"),
        }
    }
}
