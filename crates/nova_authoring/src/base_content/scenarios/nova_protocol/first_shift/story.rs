//! Every line An Ordinary Shift says, and every objective it posts, as named
//! constants.
//!
//! The event graph next door is about WHEN things happen; this module is about
//! what is said. Splitting them is what makes a pacing pass and a dialogue
//! pass two separate edits instead of one careful walk through a thousand-line
//! file.
//!
//! The shift carries four things the rest of the story needs, and it carries
//! them as work talk rather than as exposition: the plaque in the boat bay,
//! section nine and the renewal that signs itself, the junk nobody owns, and a
//! guard channel that crackles all shift and is ignored. The last one is the
//! only one that has to land, because the guard channel is what reads the
//! clause over the Meridian an hour later.
//!
//! Nothing in the graph reads a line's text and no two beats share a constant,
//! so a rewrite here is a rewrite here.

// --- the launch --------------------------------------------------------------

pub(super) const OPEN_CONTROL_CLEAR: &str =
    "Cutter One, Meridian Control. Boat bay is clear. You are released.";
pub(super) const OPEN_COPILOT_GREEN: &str = "Clamps are open. Drive and thrusters read green.";

/// The plaque for Pell, on the boat bay wall under the company's banner. The
/// captain walks past it every shift and the reader who has seen the prologue
/// knows what it is; nobody in the cockpit explains it, because to them it is
/// four years old.
pub(super) const OPEN_ENGINEER_PLAQUE: &str = "You stopped at the plaque again.";
pub(super) const OPEN_PLAYER_EVERY_SHIFT: &str = "I stop at it every shift.";
pub(super) const OPEN_ENGINEER_TAUGHT: &str =
    "Brandt taught him to fit boats. Then he taught me, same deck.";
pub(super) const OPEN_COPILOT_BANNER: &str =
    "A banner the length of the bay, and one plaque under it. Nobody ever hangs those the \
     other way round.";

pub(super) const OPEN_CHIEF_CARD: &str =
    "Cutter One, deck. You're flying a re-qualification card today - the yard had your port \
     thrusters open.";
pub(super) const OPEN_CONTROL_SHEET: &str =
    "Work sheet after that: tag the hulls on Plate Seven, then pull the cache logged off the \
     far side of the survey body.";
pub(super) const OPEN_PLAYER_COPY: &str = "Cutter One copies. Card, two hulls, one cache.";
pub(super) const OPEN_CONTROL_DEADLINE: &str =
    "Correct. Meridian gets under way in fifty-six minutes. Don't make me come looking for \
     you.";
pub(super) const OPEN_COPILOT_PRIVATE: &str = "She says that like they'd leave without us.";
pub(super) const OPEN_ENGINEER_LEAVE: &str = "They'd leave without you.";
pub(super) const OPEN_COPILOT_CRUEL: &str = "Cruel.";
pub(super) const OPEN_ENGINEER_RIG: &str = "Put us alongside. I'll handle the tags.";
pub(super) const OPEN_COPILOT_MARK: &str =
    "Mark ahead, seventeen hundred metres. Easy on the drive - we're still inside Meridian's \
     paint budget.";

pub(super) const OBJ_TEXT_BURN: &str = "Burn to the work mark.";

// --- the card ----------------------------------------------------------------

pub(super) const TRIM_COPILOT_STOP: &str =
    "Good. This is as clear a spot as we're going to get. Bring us to a full stop first. The \
     card starts from rest.";
pub(super) const OBJ_TEXT_STOP: &str = "Press [X] and let STOP bring Cutter One to rest.";

pub(super) const TRIM_ENGINEER_WHAT_TEST: &str = "What are we doing?";
pub(super) const TRIM_COPILOT_MAINTENANCE: &str =
    "Port RCS manifold came out of the yard four days ago. The card says I fly a box on it \
     before it counts as flown.";
pub(super) const TRIM_ENGINEER_DOUBT: &str = "And the yard says it's fine.";
pub(super) const TRIM_COPILOT_YARD: &str =
    "The yard says everything is fine. The yard is on a bonus.";

/// The first of the shift's guard-channel fragments. It is never addressed to
/// this ship and never completes a sentence, and the crew's whole reaction is
/// to name it and go back to work - which is the training the strike uses.
pub(super) const TRIM_GUARD_CHALLENGE: &str = "- vessel this net - state registry and -";
pub(super) const TRIM_ENGINEER_GUARD: &str = "Guard channel again.";
pub(super) const TRIM_COPILOT_FLEET: &str = "Fleet's always looking for something. Let them look.";

pub(super) const TRIM_ENGINEER_RUN_IT: &str = "Fly your box.";
pub(super) const TRIM_COPILOT_BOX: &str =
    "Four marks. Out to A, up to B, across to C, then back down to D. Bring us home without \
     any drift and the card closes.";
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
    "Thruster box is closed. First hull tag is on Plate Seven's near edge. Bring it in.";
pub(super) const OBJ_TEXT_CRATE_FIRST: &str = "Recover the first hull tag.";

/// The junk site in one exchange: the plate is somebody's whole company, and
/// nobody can say whose. A plaque, and then junk.
pub(super) const CRATE_ENGINEER_FIRST_SECURE: &str =
    "One aboard. Tag reads, but there's no owner on it. Second one's deeper in the plate, so \
     keep it slow.";
pub(super) const OBJ_TEXT_CRATE_SECOND: &str = "Recover the second hull tag.";
pub(super) const CRATE_ENGINEER_SECOND_SECURE: &str = "Two aboard. No name on that one either.";
pub(super) const CRATE_COPILOT_OWNED: &str =
    "Somebody owned all this. Whole outfit, out here, for years.";
pub(super) const CRATE_ENGINEER_NOBODY: &str = "And now it's a plate number.";

// --- the targeting computer --------------------------------------------------

pub(super) const LOCK_CHIEF: &str =
    "The cache is outside the plate. Control has laid a route around the survey body.";
pub(super) const OBJ_TEXT_LOCK: &str = "Lock TRANSIT 1 - hold [CTRL].";

pub(super) const GOTO_COPILOT: &str =
    "Transit One locked. Last box on the card is guidance and automatic braking. Give the leg \
     to the computer.";
pub(super) const OBJ_TEXT_GOTO: &str = "Press [G] and let the computer fly to TRANSIT 1.";

pub(super) const TRANSIT_COPILOT_CLEAN: &str =
    "First solution is clean. Turnaround, braking, and arrival are all inside limits.";
pub(super) const TRANSIT_ENGINEER_ONE_MORE: &str = "One more before she signs it off?";
pub(super) const TRANSIT_COPILOT_ONE_MORE: &str = "One more.";
pub(super) const OBJ_TEXT_TRANSIT: &str = "Lock TRANSIT 2 and press [G].";

pub(super) const TRANSIT_COPILOT_RELEASE: &str =
    "Second arrival clean. Guidance and automatic braking inside limits. Deck, Cutter One is \
     re-qualified.";

/// Section nine: the renewal clause, and the standing joke of the shift. The
/// captain is the only one of the three who still finds it funny, because for
/// the captain the clause is a delay and not a sentence. Under section nine,
/// and never said out loud on a good day, is the Nova Protocol.
pub(super) const TRANSIT_CHIEF_RENEWAL: &str =
    "Logged. And your renewal is still on my desk - nine days now.";
pub(super) const TRANSIT_PLAYER_NINE: &str = "Section nine again.";
pub(super) const TRANSIT_CHIEF_NINE: &str =
    "Section nine again. Six days and it signs itself. Come down and do it properly.";

// --- the detour --------------------------------------------------------------

pub(super) const DETOUR_COPILOT: &str =
    "Survey body's between us and Meridian. We're outside their sightline.";
pub(super) const DETOUR_ENGINEER_TEST: &str = "One system left to test.";
pub(super) const DETOUR_COPILOT_NOT_LISTED: &str = "Orbit hold wasn't on the card.";
pub(super) const DETOUR_ENGINEER_GRAVITY: &str = "Then call it an unscheduled gravity check.";
pub(super) const DETOUR_PLAYER_DONUT: &str = "It's a donut.";
pub(super) const DETOUR_ENGINEER_DOCUMENTED: &str = "A documented donut.";
pub(super) const OBJ_TEXT_DETOUR: &str = "Fly to the inspection planetoid.";

pub(super) const ORBIT_COPILOT: &str =
    "Gravity's on the hull. Lock the survey body and give orbit hold to the computer.";
pub(super) const OBJ_TEXT_ORBIT: &str = "Press [O] and complete one orbit.";

/// Said while the ring is holding itself and the workload is nothing, which is
/// the only place in the shift a crew moment fits. This is where the renewal
/// stops being paperwork: the captain believes in rotation, and the other two
/// stopped believing in it years ago and go on flying with them anyway.
pub(super) const ORBIT_ENGINEER_VIEW: &str = "Look at that. Whole belt turning under us.";
pub(super) const ORBIT_COPILOT_THIRD: &str = "That renewal is your third.";
pub(super) const ORBIT_PLAYER_LAST: &str = "My third and my last. I'm on the rotation list.";
pub(super) const ORBIT_ENGINEER_SAID: &str = "You said that about the second.";
pub(super) const ORBIT_PLAYER_MEANT: &str = "I meant it about the second.";
pub(super) const ORBIT_COPILOT_NOBODY: &str =
    "Nobody has rotated off this ship since I signed. Not one name.";
pub(super) const ORBIT_PLAYER_FIRST: &str = "Then I'll be the first. Somebody has to be.";
pub(super) const ORBIT_COPILOT_SURE: &str = "Sure.";

// --- back to work ------------------------------------------------------------

pub(super) const RETURN_CONTROL: &str = "Cutter One, Meridian Control. Explain the orbit.";
pub(super) const RETURN_ENGINEER_VISIBLE: &str = "They can see us again.";
pub(super) const RETURN_PLAYER_CHECK: &str =
    "Cutter One was completing an unscheduled guidance check.";
pub(super) const RETURN_CONTROL_FILED: &str = "Your card was closed six minutes ago.";
pub(super) const RETURN_COPILOT_FAST: &str = "That was fast.";
pub(super) const RETURN_CHIEF: &str =
    "We get under way inside the hour, and the bonus needs a clean sheet. Get back on the \
     plate and bring me that cache.";
pub(super) const OBJ_TEXT_RETURN: &str = "Lock and GOTO back to the work site.";

pub(super) const SEARCH_COPILOT: &str =
    "Back on the plate. Cache tag is weak, off to starboard and close in.";
pub(super) const OBJ_TEXT_SEARCH: &str = "Recover the cache.";

// --- the run home ------------------------------------------------------------

pub(super) const HOME_ENGINEER_SECURE: &str =
    "Cache is aboard. Seal's intact. No name on this one either.";
pub(super) const HOME_CHIEF: &str =
    "Three for three and the sheet is clean. Bring Cutter One to the outer hold and we'll \
     walk you in.";
pub(super) const HOME_COPILOT_TIME: &str = "And with minutes to spare.";
pub(super) const HOME_ENGINEER_TIME: &str = "You don't know how many.";

/// The last time the guard channel is ignored.
pub(super) const HOME_GUARD_TRAFFIC: &str = "- any vessel this net - Saturn traffic - respond -";
pub(super) const HOME_COPILOT_STILL: &str = "Still at it.";
pub(super) const HOME_PLAYER_NOT_OURS: &str = "Not our net. Take us home.";
pub(super) const OBJ_TEXT_HOME: &str = "Lock and GOTO to the Meridian outer hold.";

// --- the strike --------------------------------------------------------------

/// The card the strike opens on, over the first shot.
///
/// It says only what the shift has already established - where the cutter is
/// parked, that the carrier is unarmed survey plant, and that it is due to
/// leave - so it sets the frame without spending Demir's challenge, which is
/// where the number four hundred and twelve is meant to land.
pub(super) const ATTACK_CARD_PLACE: &str = "MERIDIAN, OUTER HOLD";
pub(super) const ATTACK_CARD_WHEN: &str = "END OF SHIFT - THREE KILOMETRES OFF THE CARRIER";
pub(super) const ATTACK_CARD_NOTE: &str = "Earthworks survey carrier. Unarmed, and due under way.";

pub(super) const ATTACK_CONTROL_PLUME: &str =
    "Cutter One, hold at the outer mark. We have a drive plume clearing the large body and \
     no transponder.";
pub(super) const OBJ_TEXT_WITNESS: &str = "Hold the mark. Do not close.";

pub(super) const ATTACK_COPILOT_SILENT: &str =
    "No squawk. No running lights. And it's still accelerating.";

pub(super) const ATTACK_PLAYER_MILITARY: &str =
    "Control, that's a Fleet hull, and it isn't broadcasting a fleet code. It's turning \
     toward you.";

/// The number, said once, while it is still a crew list.
pub(super) const ATTACK_CONTROL_CHALLENGE: &str =
    "Unidentified warship, this is Earthworks carrier Meridian. Civilian registry, four \
     hundred and twelve aboard. We are unarmed. Identify yourself.";

/// What the guard channel is actually reading is the clause, in the desk's own
/// dead language: Meridian Control, section nine, roster closed, no recovery.
/// Out here, on a work channel, through a body of rock, the cutter gets one
/// word of it - and the crew have spent the whole shift learning to ignore
/// exactly this.
pub(super) const ATTACK_GUARD_CLAUSE: &str = "- roster -";
pub(super) const ATTACK_ENGINEER_AGAIN: &str = "Say again?";
pub(super) const ATTACK_COPILOT_NOT_CONTROL: &str =
    "That's not Control. That's coming off the guard channel.";

pub(super) const ATTACK_CHIEF_TURNING: &str =
    "Control, its bow is coming around. Those are rail apertures.";

// --- the silence -------------------------------------------------------------

pub(super) const OBJ_TEXT_SILENCE: &str = "Hold position and keep the channel open.";

pub(super) const AFTER_COPILOT_CHANNEL: &str = "Carrier channel's gone.";
pub(super) const AFTER_PLAYER_CALL: &str = "Meridian Control, Cutter One. Demir, respond.";
pub(super) const AFTER_ENGINEER_SIGNAL: &str =
    "Wait. I still have one carrier signal. Weak, but it's running.";

/// The strike's last move and the next chapter's opening position, said rather
/// than staged: the warship leaves at once, its boats stay to sweep the field,
/// and the only thing still transmitting out here is the wreck.
pub(super) const AFTER_COPILOT_BOATS: &str =
    "And small craft. Four of them, out of the warship and into the field.";
pub(super) const AFTER_PLAYER_DARK: &str = "Then we go dark and we go into the junk.";

pub(super) const OUTRO_BEACON: &str =
    "ANY VESSEL. ANY VESSEL. THIS IS MERIDIAN. HULL BREACH ALL DECKS. SURVIVORS UNKNOWN.";
pub(super) const OUTRO_BANNER: &str =
    "The Meridian is gone with four hundred and twelve aboard. Something in the wreck is \
     still transmitting.";
pub(super) const OBJ_TEXT_DONE: &str = "The shift is over. The beacon is still running.";

// --- losing ------------------------------------------------------------------

pub(super) const DEFEAT_DESTROYED: &str = "Your cutter broke apart in the belt.";
pub(super) const DEFEAT_NEUTRALIZED: &str =
    "Nothing left to fly with - you drift derelict in the belt.";
