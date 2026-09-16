//! The colour a narrative cue is drawn in, and the one it is drawn in by
//! default.
//!
//! A story line carries ONE presentation fact: an accent. The comms panel
//! derives everything it paints from it - the speaker, the fallback icon, the
//! border, and the reading copy lifted toward white - so a cue that wants a
//! corporate net, a distress band or a smuggler's private voice names a colour
//! and is drawn in it.
//!
//! It lives here rather than beside the panel because both ends need it: the
//! HUD paints a card off the accent on each line, and the scenario layer
//! defaults the field a cue left out. `StoryFeed` is split the same way and for
//! the same reason - the HUD cannot depend on the scenario crate.

use bevy::prelude::*;

/// The accent every ordinary narrative cue is drawn in, and the HUD's own
/// palette of them - re-exported here because a cue that wants the flight
/// readout's green or the objective amber should name the HUD's colour rather
/// than a second copy of the hex.
pub mod prelude {
    pub use nova_ui::hud::ChipTone;

    pub use super::default_comms_accent;
}

/// The comms blue: what a cue that names no accent is drawn in.
///
/// A function rather than a constant because it is the `serde` default for a
/// cue's `accent` field, and because almost every line in the game is ordinary
/// comms traffic - authoring the colour on all of them would be noise around
/// the few that differ.
pub fn default_comms_accent() -> Color {
    nova_ui::theme::BLUE
}
