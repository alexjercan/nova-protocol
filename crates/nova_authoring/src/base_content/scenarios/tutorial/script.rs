//! Every line Basic Training says, and every objective it posts, as named
//! constants.
//!
//! The event graph next door is about WHEN things happen; this module is
//! about what is said. Nothing in the graph reads a line's text and no two
//! beats share a constant, so a rewrite here is a rewrite here.
//!
//! The voice is Range Control: a Fleet training range talking a cadet through
//! a qualification card. No story, no stakes beyond the card. The one thing
//! the lines carry besides instruction is that the Fleet is a thing worth
//! joining.

/// The range officer: the only voice on the channel besides the cadet's.
pub(super) const RANGE_CONTROL: &str = "Range Control";
/// The cadet's own comms label.
pub(super) const PLAYER: &str = "You";

// --- the briefing ------------------------------------------------------------

/// The card the range opens on, over the shot of the trainer on the line.
pub(super) const OPEN_CARD_PLACE: &str = "FLEET GUNNERY RANGE";
pub(super) const OPEN_CARD_WHEN: &str = "QUALIFICATION DAY";
pub(super) const OPEN_CARD_NOTE: &str = "Trainer Seven, on the line.";

pub(super) const BRIEF_HELLO: &str =
    "Trainer Seven, Range Control. Morning, cadet. You are on the gunnery range and the \
     range is cold.";
pub(super) const BRIEF_CARD: &str =
    "Today is your qualification: fly the pattern, put five target hulks on the scrap \
     list, then deal with two live drones.";
pub(super) const BRIEF_READY: &str = "Trainer Seven copies. Ready on the line.";
pub(super) const BRIEF_HELM: &str =
    "Helm is yours. [W] opens the throttle, the mouse steers. Burn to mark ALPHA, dead \
     ahead.";

pub(super) const OBJ_TEXT_BURN: &str =
    "Burn to mark ALPHA - [W] opens the throttle, the mouse steers.";

// --- the pattern -------------------------------------------------------------

pub(super) const STOP_LINE: &str =
    "On the mark. Now kill the drift: press [X] and the flight computer flies you to a \
     stop. Hands off until it is done.";
pub(super) const OBJ_TEXT_STOP: &str = "Press [X] and let STOP bring Trainer Seven to rest.";

pub(super) const RCS_LINE: &str =
    "At rest. Thrusters next: hold [SHIFT] and the mouse slides the hull instead of \
     turning it. Slide across to mark BRAVO.";
pub(super) const OBJ_TEXT_RCS: &str =
    "Hold [SHIFT] and move the mouse to slide across to mark BRAVO.";

// --- the autopilot -----------------------------------------------------------

pub(super) const NAV_LINE: &str =
    "That is the pattern. Now the flight computer. Keep weapons down, put the planetoid \
     off the far corner in front of you, and hold [CTRL] until the white travel lock \
     takes.";
pub(super) const OBJ_TEXT_NAV: &str =
    "Weapons down: hold [CTRL] on the planetoid until the white travel lock takes.";
/// A cadet who swept with weapons raised: the red lock is the gun's, and the
/// computer flies only the white one.
pub(super) const NAV_COMBAT_NUDGE: &str =
    "Red is the gun's lock, cadet. The computer flies only the white one. Weapons down \
     and sweep the planetoid again.";

pub(super) const GOTO_LINE: &str =
    "Locked. Press [G] and the computer flies the leg: burn, flip, and a stop just off \
     the rock. Hands off the stick until it is parked - a manual input takes the ship \
     back.";
pub(super) const OBJ_TEXT_GOTO: &str = "Press [G] and let the computer fly you to the planetoid.";

pub(super) const ORBIT_LINE: &str =
    "Clean arrival, and you are in its pull. Press [O]: the computer circularises and \
     holds the orbit for you.";
pub(super) const OBJ_TEXT_ORBIT: &str =
    "Press [O] and hold orbit until the computer calls it stable.";

pub(super) const RETURN_LINE: &str =
    "Orbit is stable. Enjoy the view, then bring it home: mark CHARLIE is up on the \
     line. Travel-lock it - wait for it to clear the rock if you cannot see it - and \
     [G]. The computer drops the orbit on its own.";
pub(super) const OBJ_TEXT_RETURN: &str =
    "Travel-lock mark CHARLIE and press [G] to fly back to the line.";
/// The same slip on the leg home.
pub(super) const RETURN_COMBAT_NUDGE: &str =
    "White lock for the computer, cadet, not red. Weapons down and sweep CHARLIE again.";

// --- the radar ---------------------------------------------------------------

pub(super) const LOCK_LINE: &str =
    "Back on the line. Now the gun's lock: weapons up - hold [Right Mouse] - then put \
     Target 1 in front of you and hold [CTRL] until the red lock takes.";
pub(super) const OBJ_TEXT_LOCK: &str =
    "Weapons up ([Right Mouse]) and lock Target 1 - hold [CTRL] until the lock is red.";
/// A cadet who swept with weapons lowered: the white travel lock is real but
/// feeds no gun, and the card waits for the red one.
pub(super) const LOCK_TRAVEL_NUDGE: &str =
    "White is a travel lock, cadet - it flies you places, it shoots nothing. Weapons \
     up, hold [Right Mouse], and sweep Target 1 again for the red one.";

// --- the line ----------------------------------------------------------------

pub(super) const FIRE_LINE: &str =
    "Lock is good. Range is hot. Hold [Left Mouse] and the gun follows the lock - keep \
     the rounds on it until it comes apart.";
pub(super) const OBJ_TEXT_FIRE: &str =
    "Hold [Left Mouse] and put rounds into Target 1 until it comes apart.";

pub(super) const SCRAP_LINE: &str =
    "Target 1 is scrap. Four more on the line - they are marked. Work them in any \
     order.";
/// The same hand-off for a cadet who shot Target 1 apart before the lock
/// landed. The lesson is not repeated, and the card moves on.
pub(super) const SCRAP_EARLY_LINE: &str =
    "Target 1 is scrap - no lock, but scrap. We will call that initiative. Four more \
     on the line, marked. Work them in any order.";
pub(super) const OBJ_TEXT_LINE: &str = "Destroy every target on the line.";

// --- the drones --------------------------------------------------------------

pub(super) const LIVE_LINE: &str =
    "Line is clear. Now the part that shoots back: two range drones are going live. \
     Weapons free, cadet.";
pub(super) const OBJ_TEXT_LIVE: &str = "Defeat both range drones.";
/// A crippled drone coasted off the range: the boundary took it, and it
/// counts.
pub(super) const DRONE_ADRIFT_LINE: &str =
    "That drone is adrift and off the range - the recovery tug has it. It counts, \
     cadet.";

// --- the outro ---------------------------------------------------------------

pub(super) const WON_LINE: &str = "Both drones down. Range is cold.";
pub(super) const OUTRO_TEASE: &str =
    "Qualification logged, cadet. Welcome to the Fleet. Report to the Scenarios board \
     for your first posting.";
pub(super) const OUTRO_BANNER: &str = "Trainer Seven qualified. Welcome to the Fleet.";

// --- the defeats -------------------------------------------------------------

pub(super) const DEFEAT_DESTROYED: &str = "Trainer Seven broke up on the range.";
pub(super) const DEFEAT_NEUTRALIZED: &str =
    "Trainer Seven is disarmed. The range recovers what is left.";
