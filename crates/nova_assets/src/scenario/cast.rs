//! The base campaign's recurring comms voices (task 20260721-160929).
//!
//! The base chain had NO StoryMessage speakers before the voice pass - all
//! narrative rode objective text and outcome banners; the only voiced cast
//! in shipped content was the Ledger mod's (Foreman Okono, Broker Vesh).
//! These are the BASE chain's voices, one constant per speaker so a rename
//! is a one-line change. Names are working placeholders from the arc spike
//! (tasks/20260721-155249/SPIKE.md, Open questions) pending the owner's
//! nod at the flow Finish gate.

/// Captain of the hauler Ceres Queen - the distress call the story hangs
/// on, and the friendly voice of chapters two and three.
pub(crate) const CAPTAIN_HALLORAN: &str = "Capt. Halloran";

/// The gang gunship Rust Tally's channel - chapter two's capital taunts.
pub(crate) const RUST_TALLY: &str = "Rust Tally";

/// Belt traffic control - dispatch connective tissue between the fights.
pub(crate) const BELT_RELAY: &str = "Belt Relay";

/// The gang's boss - chapter three's antagonist voice. Named by Halloran
/// first (the breathe line after Lifeline's first wave), speaks in person
/// from the second wave on.
pub(crate) const TALLYMAN: &str = "The Tallyman";
