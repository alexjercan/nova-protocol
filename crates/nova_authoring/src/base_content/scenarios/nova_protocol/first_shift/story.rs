//! Every line First Shift says, and every objective it posts, as named
//! constants.
//!
//! The event graph next door is about WHEN things happen; this module is about
//! what is said. Splitting them is what makes a pacing pass and a dialogue
//! pass two separate edits instead of one careful walk through a thousand-line
//! file.
//!
//! The dialogue is working scaffolding pending the owner's final pass. It is
//! written to be replaceable: nothing in the graph reads a line's text, and no
//! two beats share a constant.

// --- the launch --------------------------------------------------------------

pub(super) const OPEN_CONTROL_CLEAR: &str =
    "Cutter One, Meridian Control. Bay is clear. You are released.";
pub(super) const OPEN_COPILOT_GREEN: &str = "Clamps are open. Drive and thrusters read green.";
pub(super) const OPEN_CHIEF_MANIFEST: &str =
    "Plate Seven has three recovery crates waiting. Two manifested. Third one turned up after \
     the break.";
pub(super) const OPEN_ENGINEER_FOUND: &str = "Turned up how?";
pub(super) const OPEN_CONTROL_LOOSE: &str = "Loose in the debris. It's marked for recovery.";
pub(super) const OPEN_ENGINEER_MASS: &str = "Mass?";
pub(super) const OPEN_CONTROL_UNKNOWN: &str = "Unknown.";
pub(super) const OPEN_ENGINEER_EXPECTED: &str = "Of course it is.";
pub(super) const OPEN_PLAYER_COPY: &str = "Cutter One copies. Three crates off Seven, one unknown.";
pub(super) const OPEN_CONTROL_DEADLINE: &str =
    "Correct. Meridian gets under way in fifty-six minutes. Don't make me come looking for \
     you.";
pub(super) const OPEN_COPILOT_PRIVATE: &str = "She says that like they'd leave without us.";
pub(super) const OPEN_ENGINEER_LEAVE: &str = "They'd leave without you.";
pub(super) const OPEN_COPILOT_CRUEL: &str = "Cruel.";
pub(super) const OPEN_ENGINEER_RIG: &str = "Put us alongside. I'll handle the crates.";
pub(super) const OPEN_COPILOT_MARK: &str =
    "Mark ahead, seventeen hundred metres. Easy on the drive - we're still inside Meridian's \
     paint budget.";

pub(super) const OBJ_TEXT_BURN: &str = "Burn to the work mark.";

// --- the thrusters -----------------------------------------------------------

pub(super) const TRIM_COPILOT_STOP: &str =
    "Good. This is as clear a spot as we're going to get. Bring us to a full stop first. I \
     want to clear the handling card.";
pub(super) const OBJ_TEXT_STOP: &str = "Press [X] and let STOP bring Cutter One to rest.";

pub(super) const TRIM_ENGINEER_WHAT_TEST: &str = "What test are you running this time?";
pub(super) const TRIM_COPILOT_MAINTENANCE: &str =
    "Yard replaced the port RCS manifold during scheduled maintenance. Computer says \
     everything is green.";
pub(super) const TRIM_ENGINEER_DOUBT: &str = "And you don't believe it.";
pub(super) const TRIM_COPILOT_PROSPECTOR: &str =
    "So did Prospector Six before they lost her in the belt.";
pub(super) const TRIM_ENGINEER_NEWS: &str = "Different company. News said pilot error.";
pub(super) const TRIM_COPILOT_RECORDER: &str =
    "The flight recorder didn't. Port manifold locked open. Nobody aboard survived.";
pub(super) const TRIM_ENGINEER_RUN_IT: &str = "All right. Run your box.";
pub(super) const TRIM_COPILOT_BOX: &str =
    "Four marks. Out to A, up to B, across to C, then back down to D. Bring us home without \
     any drift and I'll clear the card.";
pub(super) const TRIM_COPILOT_FIRST_MARK: &str =
    "Let's move toward A. When the velocity marker turns violet, we're running on RCS.";
pub(super) const OBJ_TEXT_TRIM_LATERAL: &str =
    "Hold [SHIFT], move the mouse, and translate across to TRIM A.";

pub(super) const TRIM_COPILOT_SECOND_AXIS: &str =
    "Good response and no roll. Take us up to B. Let's see if the new manifold fights the \
     vertical bank.";
pub(super) const OBJ_TEXT_TRIM_VERTICAL: &str = "Hold [SHIFT] and move the mouse up toward TRIM B.";

pub(super) const TRIM_COPILOT_BACK_ACROSS: &str =
    "Still clean. Bring us across to C with the same pressure, then watch the drift when you \
     let go.";
pub(super) const OBJ_TEXT_TRIM_RETURN_LATERAL: &str =
    "Use [SHIFT] and the mouse to cross toward TRIM C.";

pub(super) const TRIM_COPILOT_CLOSE_BOX: &str =
    "Port response is even. Bring us down to D and settle where we started.";
pub(super) const OBJ_TEXT_TRIM_RETURN_VERTICAL: &str =
    "Use [SHIFT] and the mouse to descend toward TRIM D.";

pub(super) const TRIM_COPILOT_CLEAN: &str =
    "Back on the mark. No residual drift. That's a clean box.";

// --- the plate ---------------------------------------------------------------

pub(super) const CRATE_CHIEF_FIRST: &str =
    "Handling card closed. First manifested crate is on Plate Seven's near edge. Bring it in.";
pub(super) const OBJ_TEXT_CRATE_FIRST: &str = "Recover the first crate.";

pub(super) const CRATE_ENGINEER_SECOND: &str =
    "One aboard. Next one is well inside the rocks - fly it on the thrusters and take your \
     time.";
pub(super) const OBJ_TEXT_CRATE_SECOND: &str = "Recover the second crate.";

// --- the targeting computer --------------------------------------------------

pub(super) const LOCK_CHIEF: &str =
    "Two. The third one drifted clean out of the plate, so we are going the long way round. \
     Warm the targeting computer and hold your radar on the transit mark - [CTRL].";
pub(super) const OBJ_TEXT_LOCK: &str = "Lock TRANSIT 1 - hold [CTRL].";

pub(super) const GOTO_CHIEF: &str =
    "Locked. Now hand her to the computer - [G]. It flies the leg, you watch the belt.";
pub(super) const OBJ_TEXT_GOTO: &str = "Press [G] and let the computer fly.";

pub(super) const TRANSIT_CHIEF_AGAIN: &str = "Second mark is up. Same again.";
pub(super) const OBJ_TEXT_TRANSIT: &str = "Lock and GOTO to TRANSIT 2.";

// --- the detour --------------------------------------------------------------

pub(super) const DETOUR_COPILOT: &str =
    "Survey body's between us and Meridian. They cannot see us back here.";
pub(super) const DETOUR_ENGINEER: &str =
    "Then do one donut. The computer holds the ring and we still make the last pickup.";
pub(super) const OBJ_TEXT_DETOUR: &str = "Fly to the inspection planetoid.";

pub(super) const ORBIT_COPILOT: &str =
    "That pull you can feel is the body. Lock it and press [O] - the computer finds the ring.";
pub(super) const OBJ_TEXT_ORBIT: &str = "Press [O] and complete one orbit.";

/// Said while the ring is holding itself and the workload is nothing, which is
/// the only place in the shift a crew moment fits.
pub(super) const ORBIT_ENGINEER_VIEW: &str = "Look at that. Whole belt turning under us.";
pub(super) const ORBIT_COPILOT_LOG: &str = "Put it in the log as a sensor check.";
pub(super) const ORBIT_PLAYER_LOG: &str = "It is going in the log as nothing.";

// --- back to work ------------------------------------------------------------

pub(super) const RETURN_CONTROL: &str =
    "Cutter one, Meridian Control. We have you flying rings on the survey body. That is not \
     on today's sheet.";
pub(super) const RETURN_CHIEF: &str =
    "We lift within the hour and the bonus is on a clean manifest. Get back on the plate and \
     find me that third crate.";
pub(super) const OBJ_TEXT_RETURN: &str = "Lock and GOTO back to the work site.";

pub(super) const SEARCH_COPILOT: &str =
    "Signal is weak. It is somewhere off to starboard, close in.";
pub(super) const OBJ_TEXT_SEARCH: &str = "Recover the last crate.";

// --- the run home ------------------------------------------------------------

pub(super) const HOME_CHIEF: &str =
    "Three for three and the sheet is clean. Bring her home - hold on the outer \
     mark and we will walk you in.";
pub(super) const OBJ_TEXT_HOME: &str = "Lock and GOTO to the Meridian outer hold.";

// --- the attack --------------------------------------------------------------

pub(super) const ATTACK_CONTROL_PLUME: &str =
    "Cutter one, stay on the mark. We have a drive plume off the large body and no \
     transponder on it.";
pub(super) const OBJ_TEXT_WITNESS: &str = "Hold the mark. Do not close.";

pub(super) const ATTACK_COPILOT_SILENT: &str = "It is not squawking. It is not slowing either.";

pub(super) const ATTACK_PLAYER_MILITARY: &str =
    "Control, that is a fleet hull. Earth military. It is coming round on you.";

pub(super) const ATTACK_CONTROL_CHALLENGE: &str =
    "Unidentified vessel, this is a civil industrial hull. We are unarmed. Respond.";

pub(super) const ATTACK_CHIEF_TURNING: &str =
    "It is turning, Control. It is putting its whole side on us.";

// --- the silence -------------------------------------------------------------

pub(super) const OBJ_TEXT_SILENCE: &str = "Hold position and keep the channel open.";

pub(super) const AFTER_PLAYER_CARRIER_SIGNAL: &str =
    "There is a carrier signal in there. Weak, but it is running.";

pub(super) const OUTRO_BEACON: &str =
    "ANY VESSEL. ANY VESSEL. THIS IS MERIDIAN. HULL BREACH ALL DECKS. SURVIVORS UNKNOWN.";
pub(super) const OUTRO_BANNER: &str =
    "The Meridian is gone. Something in the wreck is still transmitting.";
pub(super) const OBJ_TEXT_DONE: &str = "First shift complete. The beacon is still running.";

// --- losing ------------------------------------------------------------------

pub(super) const DEFEAT_DESTROYED: &str = "Your cutter broke apart in the belt.";
pub(super) const DEFEAT_NEUTRALIZED: &str =
    "Nothing left to fly with - you drift derelict in the belt.";
