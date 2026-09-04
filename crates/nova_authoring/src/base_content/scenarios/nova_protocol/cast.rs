//! The base campaign's comms voices.
//!
//! One constant per speaker, so a rename is a one-line change. These are
//! working placeholders pending the owner's final dialogue pass: the campaign
//! is about an engineer who loses a ship, so the voices are the people that
//! ship is made of - a supervisor, a duty channel, and afterwards, nobody.

/// The carrier's own name, used in objective and banner text. The ship is a
/// character in chapter one and a grave in chapter two, so it is named in one
/// place.
pub(crate) const CARRIER_NAME: &str = "Meridian";

/// The player's supervisor on the maintenance deck: the whole of chapter one's
/// coaching, and the last voice off the ship.
pub(crate) const DECK_CHIEF: &str = "Deck Chief";

/// The carrier's duty channel - traffic, work orders, the sound of a working
/// hull. Its silence is chapter one's ending.
pub(crate) const CONTROL: &str = "Meridian Control";

/// The player's own comms label: a plain "You", not a callsign - neutral and
/// reusable, so the campaign's player voice commits to no name. The player is
/// the [`CUTTER_NAME`] cutter's captain, and the two voices below are their
/// crew.
pub(crate) const PLAYER: &str = "You";

/// The player's ship, named in objective text and used as its scenario id. A
/// working cutter with a crew on it, not a generic `player_spaceship`: the
/// chapter's ending is about people, and the ship has to be one of them.
pub(crate) const CUTTER_NAME: &str = "Cutter";

/// The seat beside the player's. Runs the checklist, reads the tape, and is
/// the one who suggests the thing nobody put on the work sheet.
pub(crate) const COPILOT: &str = "Copilot";

/// The copilot on the cutter's private crew channel rather than the Meridian
/// work circuit.
pub(crate) const COPILOT_CABIN: &str = "Copilot - Cabin";

/// Down the back with the crates. Says what the crew is thinking and is never
/// on the record.
pub(crate) const ENGINEER: &str = "Engineer";

/// The wreck's automatic beacon, the only thing still transmitting at the end
/// of chapter one and the reason chapter two happens.
pub(crate) const BEACON: &str = "Automated Beacon";

/// The cleanup group's leader in chapter two. It is never named and never
/// explains itself - the campaign's antagonists are, for now, a channel.
pub(crate) const CLEANUP_LEADER: &str = "Unknown Channel";
