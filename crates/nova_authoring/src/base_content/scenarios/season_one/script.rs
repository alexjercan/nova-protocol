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

// --- the opening -------------------------------------------------------------

pub(super) const OPEN_CARD_PLACE: &str = "SATURN, THE INNER LANES";
pub(super) const OPEN_CARD_WHEN: &str = "2078";
pub(super) const OPEN_CARD_NOTE: &str = "Kaveri, homebound for Baikal with a pump assembly.";

pub(super) const OPEN_LOAD: &str = "Load's secured, covers on. Passage is clear.";
pub(super) const OPEN_LANE: &str =
    "Rock is thick on the direct line. I can take us around it, or thread it and save us \
     the hour.";
pub(super) const OPEN_COST: &str =
    "Baikal loses production for every hour that assembly is out here. Thread it.";
pub(super) const OPEN_ORDER: &str = "Thread it. Slowly - there is only the one assembly.";
pub(super) const OPEN_HANDOVER: &str =
    "Marks are up. She is carrying something today, so slide her across with the \
     thrusters - [SHIFT] and the mouse. Do not swing her.";

pub(super) const OBJ_TEXT_LANE: &str =
    "Thread the lane home - hold [SHIFT] and the mouse to slide between the marks.";

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
    "We can reach them before their reserves run out. The scheduled recovery can't. It \
     costs us the repair plan - Ebro's load won't be ready.";
pub(super) const CALL_RINA: &str = "Then let's get them off it.";
pub(super) const CALL_REFUSAL: &str =
    "Kaveri, Baikal. I asked EarthWorks to cover the diversion. Reed has three of his own \
     people on that hull, and the answer was no.";
pub(super) const CALL_BACKING: &str = "Clearwell will cover it. Bring them home.";
pub(super) const CALL_DECISION: &str = "Tomas. Take the intercept.";

pub(super) const OBJ_TEXT_REACH: &str = "Come about and close on Gantry.";

// --- the approach ------------------------------------------------------------

pub(super) const NEAR_SIGHT: &str = "Gantry in sight. Matching their motion.";
pub(super) const NEAR_PORT_CHECK: &str = "Gantry, did the fault reach your docking equipment?";
pub(super) const NEAR_PORT_SOUND: &str =
    "Not that branch. The collar on our port side is sound, and we can hold attitude.";
pub(super) const NEAR_COMMIT: &str = "Understood. We come to you.";

/// The whole of the docking ask, in the order it has to be done: the verb
/// reads the radar lock, so a card that says only "press [D]" is a card a
/// captain can follow exactly and have nothing happen.
pub(super) const OBJ_TEXT_DOCK: &str =
    "Hold Gantry on the radar, bring Kaveri's collar onto her port side slowly, \
     and press [D].";
/// The same ask, for a captain who let go early.
pub(super) const OBJ_TEXT_DOCK_AGAIN: &str = "Get back on Gantry's collar and press [D].";

// --- the transfer ------------------------------------------------------------

pub(super) const DOCK_SEAL: &str = "Connection secure. Checking the seal.";
pub(super) const OBJ_TEXT_HOLD: &str = "Hold the clamp while Gantry's crew come across.";

pub(super) const HOLD_OWEN: &str = "I'm Rina. Tell me what he can manage, and we move together.";
pub(super) const HOLD_ABOARD: &str = "Owen is aboard and supported. Ivo behind him.";
pub(super) const HOLD_ALL_THREE: &str = "That's everyone. Three aboard Kaveri.";
/// A captain who released the clamp with people still on the other hull.
pub(super) const HOLD_EARLY_RELEASE: &str = "We have not got them all. Get back on that collar.";

pub(super) const OBJ_TEXT_RELEASE: &str = "Release the clamp ([D]) and take them home.";

// --- the homecoming ----------------------------------------------------------

pub(super) const WON_LINE: &str = "Baikal, Kaveri. We have all three. Coming home.";
pub(super) const OUTRO_TEASE: &str =
    "Mara has already called from Foundation. The load is late, the penalty stands, and \
     that one is mine to answer. Come home.";
pub(super) const OUTRO_BANNER: &str =
    "Three people are coming home. Ebro's load is not, and Clearwell carries the cost.";

// --- the defeats -------------------------------------------------------------

pub(super) const DEFEAT_KAVERI: &str = "Kaveri broke up in the rock, with the assembly aboard.";
pub(super) const DEFEAT_GANTRY: &str = "Gantry came apart with its crew still on it.";
