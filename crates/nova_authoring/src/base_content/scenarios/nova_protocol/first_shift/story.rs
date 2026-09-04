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
pub(super) const OBJ_TEXT_CRATE_FIRST: &str = "Recover the first manifested crate.";

pub(super) const CRATE_ENGINEER_FIRST_SECURE: &str =
    "One secure. Seal is intact and the tag matches. Second mark is deeper in the plate, so \
     keep it slow.";
pub(super) const OBJ_TEXT_CRATE_SECOND: &str = "Recover the second manifested crate.";
pub(super) const CRATE_ENGINEER_SECOND_SECURE: &str = "Two secure. Both manifests are clean.";

// --- the targeting computer --------------------------------------------------

pub(super) const LOCK_CHIEF: &str =
    "The third crate is outside the plate. Control has laid a route around the survey body.";
pub(super) const OBJ_TEXT_LOCK: &str = "Lock TRANSIT 1 - hold [CTRL].";

pub(super) const GOTO_COPILOT: &str =
    "Transit One locked. One box remains on the maintenance release: guidance and automatic \
     braking. Give the leg to the computer.";
pub(super) const OBJ_TEXT_GOTO: &str = "Press [G] and let the computer fly to TRANSIT 1.";

pub(super) const TRANSIT_COPILOT_CLEAN: &str =
    "First solution is clean. Turnaround, braking, and arrival are all inside limits.";
pub(super) const TRANSIT_ENGINEER_ONE_MORE: &str = "One more before you sign it?";
pub(super) const TRANSIT_COPILOT_ONE_MORE: &str = "One more.";
pub(super) const OBJ_TEXT_TRANSIT: &str = "Lock TRANSIT 2 and press [G].";

pub(super) const TRANSIT_COPILOT_RELEASE: &str =
    "Second arrival clean. Guidance and automatic braking are inside limits. That's the \
     maintenance release.";

// --- the detour --------------------------------------------------------------

pub(super) const DETOUR_COPILOT: &str =
    "Survey body's between us and Meridian. We're outside their sightline.";
pub(super) const DETOUR_ENGINEER_TEST: &str = "One system left to test.";
pub(super) const DETOUR_COPILOT_NOT_LISTED: &str = "Orbit hold wasn't on the maintenance release.";
pub(super) const DETOUR_ENGINEER_GRAVITY: &str = "Then call it an unscheduled gravity check.";
pub(super) const DETOUR_PLAYER_DONUT: &str = "It's a donut.";
pub(super) const DETOUR_ENGINEER_DOCUMENTED: &str = "A documented donut.";
pub(super) const OBJ_TEXT_DETOUR: &str = "Fly to the inspection planetoid.";

pub(super) const ORBIT_COPILOT: &str =
    "Gravity's on the hull. Lock the survey body and give orbit hold to the computer.";
pub(super) const OBJ_TEXT_ORBIT: &str = "Press [O] and complete one orbit.";

/// Said while the ring is holding itself and the workload is nothing, which is
/// the only place in the shift a crew moment fits.
pub(super) const ORBIT_ENGINEER_VIEW: &str = "Look at that. Whole belt turning under us.";
pub(super) const ORBIT_COPILOT_STEADY: &str =
    "Orbit hold is steady. New manifold isn't fighting the correction.";
pub(super) const ORBIT_ENGINEER_LOG: &str = "Put it on the release as a gravity-load check.";
pub(super) const ORBIT_PLAYER_LOG: &str = "It's going in the log as a donut.";

// --- back to work ------------------------------------------------------------

pub(super) const RETURN_CONTROL: &str = "Cutter One, Meridian Control. Explain the orbit.";
pub(super) const RETURN_ENGINEER_VISIBLE: &str = "They can see us again.";
pub(super) const RETURN_PLAYER_CHECK: &str =
    "Cutter One was completing an unscheduled guidance check.";
pub(super) const RETURN_CONTROL_FILED: &str = "Your maintenance release was filed six minutes ago.";
pub(super) const RETURN_COPILOT_FAST: &str = "That was fast.";
pub(super) const RETURN_CHIEF: &str =
    "We get under way inside the hour, and the bonus needs a clean manifest. Get back on the \
     plate and bring me that third crate.";
pub(super) const OBJ_TEXT_RETURN: &str = "Lock and GOTO back to the work site.";

pub(super) const SEARCH_COPILOT: &str =
    "Back on the plate. Third crate's tag is weak, off to starboard and close in.";
pub(super) const OBJ_TEXT_SEARCH: &str = "Recover the last crate.";

// --- the run home ------------------------------------------------------------

pub(super) const HOME_ENGINEER_SECURE: &str =
    "Third crate secure. Tag is valid. Still no manifest.";
pub(super) const HOME_CHIEF: &str =
    "Three for three and the sheet is clean. Bring Cutter One to the outer hold and we'll \
     walk you in.";
pub(super) const HOME_COPILOT_TIME: &str = "And with minutes to spare.";
pub(super) const HOME_ENGINEER_TIME: &str = "You don't know how many.";
pub(super) const OBJ_TEXT_HOME: &str = "Lock and GOTO to the Meridian outer hold.";

// --- the attack --------------------------------------------------------------

pub(super) const ATTACK_CONTROL_PLUME: &str =
    "Cutter One, hold at the outer mark. We have a drive plume clearing the large body and \
     no transponder.";
pub(super) const OBJ_TEXT_WITNESS: &str = "Hold the mark. Do not close.";

pub(super) const ATTACK_COPILOT_SILENT: &str =
    "No squawk. No running lights. And it's still accelerating.";

pub(super) const ATTACK_PLAYER_MILITARY: &str =
    "Control, that's an Earth Navy hull, but it isn't broadcasting a fleet code. It's turning \
     toward you.";

pub(super) const ATTACK_CONTROL_CHALLENGE: &str =
    "Unidentified warship, this is Earthworks carrier Meridian. Civilian registry. We are \
     unarmed. Identify yourself.";

pub(super) const ATTACK_CHIEF_TURNING: &str =
    "Control, its bow is coming around. Those are rail apertures.";

// --- the silence -------------------------------------------------------------

pub(super) const OBJ_TEXT_SILENCE: &str = "Hold position and keep the channel open.";

pub(super) const AFTER_COPILOT_CHANNEL: &str = "Carrier channel is gone.";
pub(super) const AFTER_PLAYER_CALL: &str = "Meridian Control, Cutter One. Respond.";
pub(super) const AFTER_ENGINEER_SIGNAL: &str =
    "Wait. I still have one carrier signal. Weak, but it's running.";

pub(super) const OUTRO_BEACON: &str =
    "ANY VESSEL. ANY VESSEL. THIS IS MERIDIAN. HULL BREACH ALL DECKS. SURVIVORS UNKNOWN.";
pub(super) const OUTRO_BANNER: &str =
    "The Meridian is gone. Something in the wreck is still transmitting.";
pub(super) const OBJ_TEXT_DONE: &str = "First shift complete. The beacon is still running.";

// --- losing ------------------------------------------------------------------

pub(super) const DEFEAT_DESTROYED: &str = "Your cutter broke apart in the belt.";
pub(super) const DEFEAT_NEUTRALIZED: &str =
    "Nothing left to fly with - you drift derelict in the belt.";
