//! Every line chapter one says, and every objective it posts, as named
//! constants.
//!
//! The event graph next door is about WHEN things happen; this module is about
//! what is said. Nothing in the graph reads a line's text, so a dialogue pass
//! is an edit to this file alone.
//!
//! Provenance: the season outline (`web/src/lore/seasons/season-1.md`) and the
//! opening episode (`web/src/comics/season-1/episode-1/`). The comic is the
//! approved script, so the lines the two tellings share are the comic's own,
//! word for word - Nadia's hail, Rina's answer, Elena's instruction. The rest
//! is bridging written in the same register, because a ship being flown says
//! less than a page with three panels on it.
//!
//! The voice is a working crew: short, practical, nobody explains anything to
//! anybody who would already know it. A player who has read the book should
//! recognise the hour; a player who has not should be able to fly the whole
//! chapter as a rescue and miss nothing they need.
//!
//! Two rules the playtest put here, both about reading while flying:
//!
//! - A LINE is one thought. The comms card holds for eight seconds and three
//!   of them can be on screen at once, so a line that needs longer than that
//!   is two lines, not one card held open. Nothing below runs past about
//!   ninety characters, and the long ones are split at the full stop the
//!   thought already has.
//! - An OBJECTIVE is the goal and nothing else. How to do it belongs to
//!   whoever says it out loud, and the key belongs to the keybind chip the
//!   handler spotlights in the same breath - a scenario spotlight draws that
//!   chip even for a verb that is not available yet, so the card never has to
//!   spell a key the HUD is already showing.

// --- the voices --------------------------------------------------------------

/// The captain. The player flies the ship; these are the words the captain
/// puts on the channel while they do.
pub(super) const JONAH: &str = "Jonah";
/// Chief engineer, and the one who reads a damaged hull for a living.
pub(super) const LEILA: &str = "Leila";
/// Pilot and navigator: the lane, the intercept, and what both cost.
pub(super) const TOMAS: &str = "Tomas";
/// Work operations: the load, and the people coming across.
pub(super) const RINA: &str = "Rina";
/// Systems and first aid: the traffic nobody else heard.
pub(super) const SAMIR: &str = "Samir";

/// Gantry's captain, on the radio for the whole chapter.
pub(super) const NADIA: &str = "Nadia - Gantry";
/// Clearwell's manager at Baikal, and the shore end of the decision.
pub(super) const ELENA: &str = "Elena - Baikal";

/// The face each voice wears on the comms panel.
///
/// The art is the BOOK's: `scripts/generate-campaign-portraits.py` reads every
/// skin, hair and coat colour below out of the same palette the encyclopedia
/// portrait studies are drawn from, so these seven are the story's people in
/// the game's medium rather than a second cast that happens to share their
/// names. An unlisted speaker draws the panel's fallback tile.
pub(super) fn portrait(speaker: &str) -> Option<&'static str> {
    Some(match speaker {
        JONAH => "jonah",
        LEILA => "leila",
        TOMAS => "tomas",
        RINA => "rina",
        SAMIR => "samir",
        NADIA => "nadia",
        ELENA => "elena",
        _ => return None,
    })
}

// --- the opening -------------------------------------------------------------

pub(super) const OPEN_CARD_PLACE: &str = "SATURN, THE INNER LANES";
pub(super) const OPEN_CARD_WHEN: &str = "2078";
pub(super) const OPEN_CARD_NOTE: &str = "Kaveri, homebound for Baikal with a pump assembly.";

pub(super) const OPEN_LOAD: &str = "Load's secured, covers on. Passage is clear.";
pub(super) const OPEN_LANE: &str = "Rock is thick on the direct line.";
pub(super) const OPEN_CHOICE: &str = "I can take us around it, or thread it and save us the hour.";
pub(super) const OPEN_COST: &str =
    "Baikal loses production for every hour that assembly is out here. Thread it.";
pub(super) const OPEN_ORDER: &str = "Thread it. Slowly - there is only the one assembly.";
/// The handover, in two cards: what the ship is, and then how to fly it.
///
/// One card carried both and ran to 128 characters, which is a card nobody
/// finishes before the next one lands on top of it.
pub(super) const OPEN_MARKS: &str = "Marks are up. She is carrying something today.";
pub(super) const OPEN_HANDOVER: &str =
    "Slide her across with the thrusters - [SHIFT] and the mouse. Do not swing her.";

/// The goal. The thrusters are Tomas's line above, and [SHIFT] is on the RCS
/// chip the same handler spotlights.
pub(super) const OBJ_TEXT_LANE: &str = "Thread the lane home.";

// --- the lane ----------------------------------------------------------------

pub(super) const LANE_ONE: &str = "LANE-1. Next one is off to port and a little high.";
pub(super) const LANE_TWO: &str =
    "That is the way. The cradle does not care for being turned with two tonnes on it.";
pub(super) const LANE_THREE: &str = "Tight through there. Nothing touched.";
pub(super) const LANE_FOUR: &str = "LANE-4, and clear of the worst of it. Baikal in four hours.";

// --- the distress call -------------------------------------------------------

pub(super) const CALL_TRAFFIC: &str = "Traffic. Distress, broad channel, and it is close.";
pub(super) const CALL_MAYDAY: &str = "Gantry, requesting assistance. Main propulsion disabled.";
pub(super) const CALL_KNOWN: &str = "That's the captain from Aquila.";
pub(super) const CALL_ANSWER: &str = "Gantry, Kaveri. We hear you. How many aboard?";
pub(super) const CALL_THREE: &str =
    "Three. All alive. Owen's hurt. We've isolated the damaged spaces.";
pub(super) const CALL_COST: &str =
    "We can reach them before their reserves run out. The scheduled recovery can't.";
pub(super) const CALL_PRICE: &str = "It costs us the repair plan. Ebro's load won't be ready.";
pub(super) const CALL_RINA: &str = "Then let's get them off it.";
/// Elena's refusal, split at its own full stop - the comic's wording either
/// way, because the approved script is the approved script.
pub(super) const CALL_REFUSAL: &str = "Kaveri, Baikal. I asked EarthWorks to cover the diversion.";
pub(super) const CALL_REED: &str =
    "Reed has three of his own people on that hull, and the answer was no.";
pub(super) const CALL_BACKING: &str = "Clearwell will cover it. Bring them home.";
pub(super) const CALL_DECISION: &str = "Tomas. Take the intercept.";

pub(super) const OBJ_TEXT_REACH: &str = "Come about and close on Gantry.";

// --- the approach ------------------------------------------------------------

pub(super) const NEAR_SIGHT: &str = "Gantry in sight. Matching their motion.";
pub(super) const NEAR_PORT_CHECK: &str = "Gantry, did the fault reach your docking equipment?";
pub(super) const NEAR_PORT_SOUND: &str =
    "Not that branch. The collar on our port side is sound, and we can hold attitude.";
/// The commitment, and the one thing the card cannot leave to the HUD: the
/// docking verb reads the radar lock, so a captain who never locks Gantry can
/// follow every other instruction exactly and have nothing happen. The captain
/// gives it as the order it is, rather than the card carrying a third clause.
pub(super) const NEAR_COMMIT: &str = "Understood, we come to you. Tomas - hold her on the radar.";

/// The goal. The lock is the line above and [D] is the DOCK chip, lit by the
/// same handler that posts this.
pub(super) const OBJ_TEXT_DOCK: &str = "Bring Kaveri's collar onto Gantry's port side.";
/// The same ask, for a captain who let go early.
pub(super) const OBJ_TEXT_DOCK_AGAIN: &str = "Get back on Gantry's collar.";

// --- the transfer ------------------------------------------------------------

pub(super) const DOCK_SEAL: &str = "Connection secure. Checking the seal.";
pub(super) const OBJ_TEXT_HOLD: &str = "Hold the clamp while Gantry's crew come across.";

pub(super) const HOLD_OWEN: &str = "I'm Rina. Tell me what he can manage, and we move together.";
pub(super) const HOLD_ABOARD: &str = "Owen is aboard and supported. Ivo behind him.";
pub(super) const HOLD_ALL_THREE: &str = "That's everyone. Three aboard Kaveri.";
/// A captain who released the clamp with people still on the other hull.
pub(super) const HOLD_EARLY_RELEASE: &str = "We have not got them all. Get back on that collar.";

pub(super) const OBJ_TEXT_RELEASE: &str = "Let go and take them home.";

// --- the homecoming ----------------------------------------------------------

pub(super) const WON_LINE: &str = "Baikal, Kaveri. We have all three. Coming home.";
/// Trimmed rather than split: the outro chain is the shared pacing helper's,
/// and it carries one tease line between the win and the banner.
pub(super) const OUTRO_TEASE: &str =
    "The load is late and Foundation has already called. That one is mine. Come home.";
pub(super) const OUTRO_BANNER: &str =
    "Three people are coming home. Ebro's load is not, and Clearwell carries the cost.";

// --- the defeats -------------------------------------------------------------

pub(super) const DEFEAT_KAVERI: &str = "Kaveri broke up in the rock, with the assembly aboard.";
pub(super) const DEFEAT_GANTRY: &str = "Gantry came apart with its crew still on it.";
