//! The base game's training handbook: the lessons the Lessons screen draws,
//! authored here and shipped as content like everything else.
//!
//! One lesson is one screen - a demonstration, a few lines, the actions it
//! names, a path into the player wiki, and sometimes somewhere to fly. The
//! handbook is the CONDENSED companion: the complete manual is the wiki, and
//! every lesson links into it rather than restating it.
//!
//! Four rules hold this file together:
//!
//! - A lesson never spells a key. It names ACTIONS, and the screen resolves
//!   the player's own binding for each one, so a rebound helm is still
//!   described correctly.
//! - A lesson practises in a RANGE, never in a chapter. `practice` may only
//!   name a `role: Lesson` scenario (`super::scenarios::drills`), which is
//!   what keeps the Practice button from dropping a reader into an hour of
//!   campaign. A lesson with nothing focused to fly carries `practice: None`
//!   rather than the nearest approximation.
//! - A `wiki_path` names a page the manual actually ships, with the anchor of
//!   the heading that carries the claim wherever there is one.
//!   `crates/nova_authoring/tests/lesson_wiki_links.rs` resolves every one of
//!   them against `web/src/`, so a renamed page or a retitled heading fails
//!   there instead of dead-ending a reader.
//! - A claim is written ONCE. A `field_notes` entry is the same fact the
//!   lesson teaches, cut to the two lines the menu card and the loading slot
//!   draw, and it carries its lesson with it so the card can open it.
//!
//! `order` is explicit and spaced by ten, so a lesson can be slipped between
//! two others (or by a mod) without renumbering the file.

use nova_training::prelude::{Lesson, LessonCategory, LessonMedia};

use super::scenarios::{
    drills::{DRILL_AUTOPILOT_ID, DRILL_GUNNERY_ID, DRILL_MOMENTUM_ID, DRILL_STOP_ID},
    tutorial::TUTORIAL_SCENARIO_ID,
};

/// Where a lesson's demonstration lives inside the base bundle. Listed in
/// `assets/base/base.bundle.ron` like every other base resource, so the
/// content gate can prove each one exists and a mod can reuse one through
/// `dep://base/training/<id>.webp`.
///
/// WEBP, not PNG. The pane draws a demonstration at its own width - about 900
/// logical pixels at a 1920 window - so a cell has to carry that many pixels
/// or the screen shows an upscale. At that size a lossless sheet of captured
/// footage is megabytes; the same sheet as WebP is a tenth of it. `nova_menu`
/// turns the decoder on (`bevy/webp`), and nothing else in the pipeline cares
/// which codec a demonstration arrives in.
fn media_path(id: &str) -> String {
    format!("self://training/{id}.webp")
}

/// A still frame.
fn still(id: &str, alt: &str) -> LessonMedia {
    LessonMedia::Image {
        image: media_path(id).into(),
        alt: alt.to_string(),
    }
}

/// A looping demonstration, as a sprite sheet.
///
/// One grid for every loop in the base handbook: 20 frames in a 4x5 sheet at
/// 10 fps, which is TWO SECONDS of motion. The grid is authored rather than
/// inferred because a sheet is just an image - nothing in the file says where
/// the cells are, and a guess would silently cut the frames wrong.
///
/// Two seconds, not one, because a demonstration has to show a THING HAPPEN.
/// One second is enough for a camera to drift over a pose, and not enough to
/// press a key, watch the lock charge and see the bracket land - which is the
/// only reason the loop is there. 10 fps rather than 12 keeps the sheet inside
/// the texture size every target supports.
///
/// Captured footage fills those cells at 960x540 each, so a 3840x2700 sheet -
/// one screen pixel per cell pixel at a 1920 window. A lesson still on
/// placeholder art carries a smaller sheet on the same grid; the cell size
/// follows the file.
fn looping(id: &str, alt: &str) -> LessonMedia {
    LessonMedia::Loop {
        sheet: media_path(id).into(),
        columns: 4,
        rows: 5,
        frames: 20,
        frames_per_second: 10.0,
        alt: alt.to_string(),
    }
}

/// One lesson. Every field is spelled at every call site on purpose: a missing
/// demonstration or a missing wiki path is visible in the table below rather
/// than defaulted in silence.
#[expect(
    clippy::too_many_arguments,
    reason = "one argument per authored field; a struct literal per lesson would be longer and \
              hide nothing"
)]
fn lesson(
    id: &str,
    category: LessonCategory,
    order: i32,
    title: &str,
    media: LessonMedia,
    body: &str,
    actions: &[&str],
    wiki_path: &str,
    practice: Option<&str>,
    proven_by: &[&str],
    field_notes: &[&str],
) -> Lesson {
    Lesson {
        id: id.to_string(),
        category,
        order,
        title: title.to_string(),
        media,
        body: body.to_string(),
        actions: actions.iter().map(|action| (*action).to_string()).collect(),
        wiki_path: wiki_path.to_string(),
        practice: practice.map(str::to_string),
        proven_by: proven_by.iter().map(|id| (*id).to_string()).collect(),
        field_notes: field_notes.iter().map(|n| (*n).to_string()).collect(),
    }
}

/// The base game's shipped lessons, in the order they are authored. The screen
/// sorts by category, then `order`, then id, so this order is for reading the
/// file and nothing else.
pub(crate) fn lesson_catalog() -> Vec<Lesson> {
    use LessonCategory::{Advanced, Combat, Flight, NovaOs, Shipbuilding, StartHere};
    vec![
        lesson(
            "start_welcome",
            StartHere,
            10,
            "How training works",
            still("start_welcome", "the training screen with a lesson open"),
            "Each lesson is one screen: a picture or a short loop, a few lines \
             of text, and the controls it uses. Some lessons open a practice \
             flight. Every lesson links to a longer page on the wiki.",
            &[],
            "wiki/getting-started",
            None,
            &[],
            &[],
        ),
        lesson(
            "start_hud",
            StartHere,
            20,
            "Reading the HUD",
            still(
                "start_hud",
                "the velocity sphere and speed readout drawn around a ship",
            ),
            "The HUD only shows what you need right now. Your speed and a \
             velocity sphere sit around your ship. The sphere points the way \
             you are actually moving, which is not always where the nose \
             points.",
            &[],
            "wiki/hud",
            None,
            &[],
            &[
                "The velocity sphere points the way you are moving. That is not always where the nose points.",
            ],
        ),
        lesson(
            "start_verbs",
            StartHere,
            25,
            "The keybind dock",
            looping(
                "start_verbs",
                "the verb chips along the bottom of a cruising ship, then STOP engaged: its chip \
                 inverts and a CANCEL chip joins the row",
            ),
            "The chip row along the bottom shows the verbs this ship can use right now: STOP, \
             GOTO, ORBIT, CANCEL, RADAR, COMPONENT, RCS and DOCK. Each draws your own keycap; a \
             verb the ship cannot use is off the row, and an engaged one inverts.",
            &[],
            "wiki/hud#flight-readouts",
            Some(DRILL_AUTOPILOT_ID),
            &[],
            &[
                "The chip row draws only the verbs this ship can use now, each with your own keycap.",
            ],
        ),
        lesson(
            "start_camera",
            StartHere,
            30,
            "Looking around",
            looping("start_camera", "the camera orbiting a stationary ship"),
            "The mouse is the helm: in normal flight the camera and the ship turn together. Hold \
             free look and the camera turns on its own while the hull holds its heading - that is \
             how you find a contact before you commit to it.",
            &["camera_rotate", "free_look"],
            "wiki/keybinds#targeting-and-camera",
            None,
            &[],
            &["The mouse is the helm. Hold free look to turn the camera without turning the ship."],
        ),
        lesson(
            "start_cinematic",
            StartHere,
            40,
            "Two HUD levels",
            looping(
                "start_cinematic",
                "a cruising ship with its velocity sphere, speed readout and verb chips drawn, \
                 then the HUD toggle clearing them to a bare view",
            ),
            "The HUD is contextual: a quiet cruise draws the velocity sphere, your speed and the \
             dock's live verbs, and everything else arrives with its moment. The HUD toggle cycles \
             two levels: On, and Cinematic, a clean screen that clears every instrument and chip.",
            &["hud_cinematic"],
            "wiki/hud#what-is-on-screen-and-when",
            None,
            &[],
            &[
                "Cinematic is the HUD's second level: a clean screen with every instrument and chip gone.",
            ],
        ),
        lesson(
            "flight_aim",
            Flight,
            10,
            "Turn, then thrust",
            looping(
                "flight_aim",
                "a ship holding a heading with its main drive lit behind it",
            ),
            "The main drive only pushes in the direction the nose points. Turn \
             to face where you want to go, then thrust. Thrust while you are \
             still turning and you push yourself onto a heading you are \
             leaving.",
            &["main_drive"],
            "wiki/flight-autopilot#manual-flight",
            Some(DRILL_MOMENTUM_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_MOMENTUM_ID],
            &["The drive only pushes where the nose points. Turn first, then thrust."],
        ),
        lesson(
            "flight_momentum",
            Flight,
            20,
            "You keep your speed",
            looping(
                "flight_momentum",
                "thrust released; the ship keeps its speed and heading",
            ),
            "Releasing the drive does not slow you down. Nothing in space does. \
             You keep your current speed and heading until something pushes you \
             the other way.",
            &["main_drive"],
            "wiki/flight-autopilot#manual-flight",
            Some(DRILL_MOMENTUM_ID),
            &[DRILL_MOMENTUM_ID],
            &["Nothing slows you down in space. Release the drive and your speed stays the same."],
        ),
        lesson(
            "flight_speedcap",
            Flight,
            25,
            "The speed cap",
            looping(
                "flight_speedcap",
                "a held burn climbing against the range's soft cap: the speed chip reads 138.7 / \
                 150.0 m/s and settles on 150.0 with the drive still lit",
            ),
            "A scenario can put a soft cap on your manual burn: Basic Training flies under 150 \
             m/s. It caps your TOTAL speed, not the heading you point, so a held throttle tapers \
             off there instead of accelerating forever. A braking burn is never capped.",
            &["main_drive"],
            "wiki/flight-autopilot#manual-flight",
            Some(DRILL_MOMENTUM_ID),
            &[],
            &[
                "A range can cap your manual speed. It caps your total speed, not the heading you point.",
            ],
        ),
        lesson(
            "flight_stop",
            Flight,
            30,
            "The STOP order",
            looping(
                "flight_stop",
                "the flight computer turning the ship around and slowing it to zero",
            ),
            "STOP flips the ship to retrograde and burns the speed off. At rest is a deadband, not \
             a dead stop: a residual the drive already faces is braked to 2 m/s, and a sideways \
             drift is handed back at up to 7.5 m/s.",
            &["autopilot_stop"],
            "wiki/flight-autopilot#the-autopilot-flies-the-hull",
            Some(DRILL_STOP_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_STOP_ID],
            &[
                "STOP settles into a deadband, not a dead stop: a sideways drift is handed back at up to 7.5 m/s.",
            ],
        ),
        lesson(
            "flight_cancel",
            Flight,
            35,
            "Taking the ship back",
            looping(
                "flight_cancel",
                "a braking order running, then the main drive lighting: the AP STOP chip goes \
                 out, the CANCEL chip leaves the dock and the shell turns from nav cyan to blue",
            ),
            "CANCEL drops any engaged maneuver and hands the ship back already moving. So does a \
             main-drive burn, a bound thruster key, entering RCS, or pressing the engaged verb \
             again. Moving the mouse does not: while the computer flies, the mouse is camera only.",
            &["autopilot_off", "main_drive", "rcs_modifier"],
            "wiki/flight-autopilot#the-autopilot-flies-the-hull",
            Some(DRILL_STOP_ID),
            &[],
            &[
                "Moving the mouse does not cancel a maneuver. CANCEL, a burn or a thruster key does.",
            ],
        ),
        lesson(
            "flight_rcs",
            Flight,
            40,
            "Using the RCS thrusters",
            looping(
                "flight_rcs",
                "small thruster bursts moving a ship onto a mark",
            ),
            "Hold the RCS modifier and the mouse drives the small thrusters. \
             They move the ship sideways without turning it. Use them for small \
             corrections the main drive would overshoot.",
            &["rcs_modifier", "rcs_aim"],
            "wiki/flight-autopilot#rcs-fine-docking-thrusters",
            Some(DRILL_STOP_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_STOP_ID],
            &[],
        ),
        lesson(
            "flight_gravity",
            Flight,
            45,
            "Gravity wells",
            looping(
                "flight_gravity",
                "a hull coasting inside a sphere of influence with its drive out: the yellow pull \
                 shell and the blue velocity shell stand off in different directions while the \
                 speed chip climbs from 122 to 139 m/s",
            ),
            "Gravity pulls you inward at a = mu / r^2. mu is the body's number, never your mass, \
             so a laden hauler falls as fast as a stripped fighter. The pull ends at the sphere of \
             influence, where it has decayed to 2.5 m/s2.",
            &[],
            "wiki/gravity-wells#sphere-of-influence",
            Some(DRILL_AUTOPILOT_ID),
            &[],
            &["Gravity is an acceleration, so a laden hauler falls as fast as a stripped fighter."],
        ),
        lesson(
            "flight_goto",
            Flight,
            50,
            "GOTO a mark",
            looping(
                "flight_goto",
                "a ship turning towards a marked contact and flying to it",
            ),
            "GOTO flies the ship to your travel lock and parks a standoff short of it: 500 m of \
             clear space, measured from your own hull face to the target's surface, rather than \
             centre to centre. Mark a contact first, then give the order.",
            &["autopilot_goto"],
            "wiki/flight-autopilot#the-autopilot-flies-the-hull",
            Some(DRILL_AUTOPILOT_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_AUTOPILOT_ID],
            &["GOTO parks 500 m off the target's surface, measured from your own hull face."],
        ),
        lesson(
            "flight_arrival",
            Flight,
            55,
            "The arrival envelope",
            looping(
                "flight_arrival",
                "a GOTO leg flipped retrograde on its braking ramp, drives firing up the track \
                 under an amber AP GOTO - BURN chip, swinging once through ALIGN and back as the \
                 speed falls",
            ),
            "GOTO obeys one rule: at any distance it caps closing speed at what a flip-and-brake \
             from there can still cancel. It flips one swing early, brakes at 85% of the drive's \
             authority down to a 15 m/s floor, then settles on RCS.",
            &["autopilot_goto"],
            "wiki/flight-autopilot#the-autopilot-flies-the-hull",
            Some(DRILL_AUTOPILOT_ID),
            &[],
            &["GOTO never closes faster than a flip-and-brake from where it is can still cancel."],
        ),
        lesson(
            "flight_orbit",
            Flight,
            60,
            "ORBIT a mark",
            looping(
                "flight_orbit",
                "a ship settling into a circle around a planetoid",
            ),
            "ORBIT parks the ship in a circle around the gravity well it is inside, never around \
             your mark. The computer picks a stable ring, holds orbital speed with micro-burns, \
             and never finishes on its own. Outside every sphere of influence the key does \
             nothing.",
            &["autopilot_orbit"],
            "wiki/gravity-wells#the-dominant-well",
            Some(DRILL_AUTOPILOT_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_AUTOPILOT_ID],
            &["ORBIT circles the gravity well you are inside, not the contact you marked."],
        ),
        lesson(
            "combat_radar",
            Combat,
            10,
            "Using the radar",
            looping(
                "combat_radar",
                "the radar marking a contact ahead of the ship: the lock charges, \
                 then the bracket lands on it",
            ),
            "Hold radar to sweep and mark what it settles on. There are two slots: lowered writes \
             the white travel lock the autopilot flies, raised writes the red combat lock the guns \
             use. A tap clears in stages; raised, it only drops the combat lock.",
            &["radar_hold", "radar_clear"],
            "wiki/targeting-radar#holding-to-sweep",
            Some(DRILL_GUNNERY_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_GUNNERY_ID],
            &[
                "There are two lock slots. Lowered writes the white travel lock, raised the red combat lock.",
            ],
        ),
        lesson(
            "combat_allegiance",
            Combat,
            15,
            "Who shoots whom",
            still(
                "combat_allegiance",
                "a mixed field of ships with green, red and grey allegiance triangles over them",
            ),
            "Every ship is Player, Enemy or Neutral, and any pair resolves to Own, Hostile or \
             Neutral. A round copies its shooter's side at launch and keeps it, so your own \
             ordnance never hits you. A triangle over each ship reads green, red or grey.",
            &[],
            "wiki/factions#the-relation-model",
            None,
            &[],
            &["A round takes its shooter's side at launch, so your own ordnance never hits you."],
        ),
        lesson(
            "combat_stance",
            Combat,
            20,
            "Raising weapons",
            looping(
                "combat_stance",
                "a ship's gun mounts rising out of the deck as the weapons come up",
            ),
            "Raising weapons is a held stance. It makes your radar write the red combat lock \
             instead of the white travel lock, makes the guns hot, and moves the mouse off the \
             helm onto turret aim - the hull holds its heading while you aim.",
            &["combat_stance"],
            "wiki/targeting-radar#stances-and-slots",
            Some(DRILL_GUNNERY_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_GUNNERY_ID],
            &[
                "Raising weapons writes the red combat lock and moves the mouse from the helm to turret aim.",
            ],
        ),
        lesson(
            "combat_cover",
            Combat,
            25,
            "Cover and the firing line",
            still(
                "combat_cover",
                "a burst breaking up against a rock between the shooter and a hostile behind it",
            ),
            "Put a rock between you and a hostile and the rounds stop at it - and so does the \
             radar, which needs the same clear line the guns do. A hostile that loses the line \
             loses its pick and goes passive.",
            &[],
            "wiki/combat-weapons#cover-line-of-fire",
            None,
            &[],
            &["A rock on the line eats the round and the radar lock. Cover is a full disengage."],
        ),
        lesson(
            "combat_components",
            Combat,
            30,
            "Lock a component",
            looping(
                "combat_components",
                "the fine lock stepping from one section of a marked ship to the next",
            ),
            "Hold a combat lock steady and you can drill it into one section. Turrets and the \
             viewfinder follow the one you pick. Killing a drive costs thrust, never speed; it is \
             the last flight computer that leaves a hull drifting and tumbling.",
            &["component_next", "component_prev"],
            "wiki/targeting-radar#per-section-fine-lock",
            Some(DRILL_GUNNERY_ID),
            &[DRILL_GUNNERY_ID],
            &[
                "Killing a ship's drives costs it thrust, not speed. Its last flight computer leaves it drifting.",
            ],
        ),
        lesson(
            "combat_damage_types",
            Combat,
            35,
            "Kinetic and Pierce",
            still(
                "combat_damage_types",
                "an amber Kinetic tracer beside a steel-blue Pierce dart raking a stack of \
                 sections",
            ),
            "A damage type is not a multiplier: both hit a hull and a drive for the same number. \
             What changes is travel. Kinetic carries on only through what it destroys; Pierce rakes \
             every section it crosses, spending a power budget per layer.",
            &[],
            "wiki/combat-weapons#damage-types",
            None,
            &[],
            &[
                "A damage type is not a multiplier. Kinetic stops at what it fails to kill; Pierce rakes.",
            ],
        ),
        lesson(
            "combat_turrets",
            Combat,
            40,
            "Turret arcs",
            still(
                "combat_turrets",
                "a ship from above, its turrets swung onto one target and firing",
            ),
            "A turret turns all the way round; its barrel stops 10 degrees below level, so each \
             mount is blind in a cone under its own keel. Nothing else bounds it - your own hull \
             is transparent to your rounds, which pass straight through it.",
            &[],
            "wiki/sections/turret#what-it-can-bear-on",
            None,
            &[TUTORIAL_SCENARIO_ID, DRILL_GUNNERY_ID],
            &[
                "A turret's barrel stops 10 degrees below level, so each mount is blind under its own keel.",
            ],
        ),
        lesson(
            "combat_magazines",
            Combat,
            45,
            "A magazine is a rate limit",
            still(
                "combat_magazines",
                "a turret's ammo ring draining through a burst, then filling again through a \
                 quiet stretch",
            ),
            "No weapon runs out for good. A PDC holds 500 rounds at 100 a second and gets 200 back \
             for every three quiet seconds, all at once. Every shot restarts that interval, so \
             firing in batches sustains 40 rounds a second.",
            &[],
            "wiki/combat-weapons#magazines",
            Some(DRILL_GUNNERY_ID),
            &[],
            &["A magazine is a rate limit, not a budget. Nothing in the game runs out for good."],
        ),
        lesson(
            "combat_torpedoes",
            Combat,
            50,
            "Torpedoes",
            looping(
                "combat_torpedoes",
                "a torpedo dropping clear of a ship, then homing on a locked target",
            ),
            "A torpedo needs a combat lock. The bay drops it cold and the drive lights 0.6 seconds \
             later. It arms only well clear of you: 324 m off a small boat, 494 m off a carrier. \
             Inside that it bounces off as a dud.",
            &[],
            "wiki/sections/torpedo-bay",
            None,
            &[],
            &[
                "A torpedo arms 324 m off a small boat and 494 m off a carrier. Inside that it is a dud.",
            ],
        ),
        lesson(
            "combat_point_defense",
            Combat,
            60,
            "Your battery defends itself",
            still(
                "combat_point_defense",
                "idle mounts swinging onto inbound torpedoes, a thin line drawn from each mount \
                 to its pick",
            ),
            "With no combat lock and your weapons lowered, the flight computer works your idle \
             mounts against inbound torpedoes - no toggle and no key. A thin line runs from each \
             mount to its pick. Lock or raise, and every mount is yours that instant.",
            &["combat_stance"],
            "wiki/combat-weapons#your-own-battery",
            None,
            &[],
            &[
                "Your idle mounts shoot torpedoes down on their own. Lock or raise and they are yours.",
            ],
        ),
        lesson(
            "combat_railgun",
            Combat,
            70,
            "The hull is the aim",
            still(
                "combat_railgun",
                "a railgun charging on a spinal mount, its bore sight line laid across a target \
                 hull",
            ),
            "A railgun has no traverse: it fires down its own axis, so the face you bolt it to is \
             the line. A tap starts a 1.5 second charge that only lowering your weapons aborts; it \
             never re-checks the nose. One shell every 13.5 seconds.",
            &[],
            "wiki/sections/railgun#committing-the-shot",
            None,
            &[],
            &[
                "A railgun fires down its own axis. The face you bolted it to is the line it shoots.",
            ],
        ),
        lesson(
            "combat_collapse",
            Combat,
            90,
            "When a ship comes apart",
            still(
                "combat_collapse",
                "a hull tearing itself apart from the outside in once its structure runs out",
            ),
            "You do not have to shoot every section off. A hull carrying less than a twentieth of \
             the structure it was built with collapses. A ship that loses every weapon, or its last \
             flight computer, is NEUTRALIZED: it keeps its hull and stops answering.",
            &[],
            "wiki/ships#taking-a-ship-apart",
            Some(DRILL_GUNNERY_ID),
            &[],
            &[
                "A hull left under a twentieth of the structure it was built with collapses on its own.",
            ],
        ),
        lesson(
            "build_sections",
            Shipbuilding,
            10,
            "Ships are built from sections",
            still(
                "build_sections",
                "the editor with one section selected on the hull it mates to",
            ),
            "A ship is sections mated socket to socket, and a socket is the only structural link a \
             hull has. Each section does one job and weighs the volume of the box it is hit on. \
             The build grid counts in 10 m cells.",
            &[],
            "wiki/sections#what-every-section-shares",
            None,
            &[],
            &[
                "Sections mate socket to socket, and a socket is the only structural link a hull has.",
            ],
        ),
        lesson(
            "build_mass",
            Shipbuilding,
            20,
            "Mass and thrust",
            still(
                "build_mass",
                "the same hull with a heavier loadout, its numbers beside it",
            ),
            "Every section you add is mass the drive has to move and stop. On the same two basic \
             drives the 21.00-mass skiff pulls 61 m/s2 and the 41.00-mass tug pulls 31. More \
             drives close the gap, but no stack passes 640 m/s2.",
            &[],
            "wiki/sections/thruster",
            None,
            &[],
            &[
                "The 21.00-mass skiff pulls 61 m/s2 on two basic drives; the 41.00-mass tug pulls 31.",
            ],
        ),
        lesson(
            "build_balance",
            Shipbuilding,
            30,
            "Thruster placement",
            still(
                "build_balance",
                "two thrusters on a lopsided hull in the editor, with the ship's turn and \
                 thrust figures beside it",
            ),
            "Thrusters sit wherever you bolt them, so an off-centre burn would \
             spin the ship. The flight computer sets each throttle to cancel \
             that spin. A lopsided or damaged ship still flies straight, with \
             less thrust.",
            &[],
            "wiki/sections/thruster",
            None,
            &[],
            &[
                "The flight computer balances your thrusters, so a lopsided ship still flies straight.",
            ],
        ),
        lesson(
            "build_turning",
            Shipbuilding,
            35,
            "What decides your turn rate",
            still(
                "build_turning",
                "the editor readout on a long hull, its arm and turn rate printed beside the \
                 build",
            ),
            "A hull turns at the lower of two ceilings: what its computers can twist against its \
             mass, and what 8 G allows at the furthest section's face. The base gunship's 55.2 m \
             arm caps it at 1.42 rad/s2, where its computers could push 8.74.",
            &[],
            "wiki/sections/controller#what-sets-how-hard-a-ship-turns",
            None,
            &[],
            &[
                "A hull turns at the lower of two ceilings: computer torque, and 8 G at its furthest face.",
            ],
        ),
        lesson(
            "build_flight_test",
            Shipbuilding,
            40,
            "Fly what you built",
            looping(
                "build_flight_test",
                "a newly built four-block ship running past a rock under its own drive",
            ),
            "Test a new ship before you use it. Take it out, run the drive up \
             to speed, then STOP. That shows you how quickly it accelerates and \
             how long it needs to stop.",
            &[],
            "wiki/flight-autopilot#the-hull-decides-the-handling",
            None,
            &[],
            &[],
        ),
        lesson(
            "build_weapon_mounts",
            Shipbuilding,
            60,
            "Where a weapon can point",
            still(
                "build_weapon_mounts",
                "a turret sweeping its arc on one flank while a railgun holds the line of the \
                 face it sits on",
            ),
            "Where you bolt a weapon decides what it points at. A turret traverses freely and owns \
             58.7 percent of the sky from wherever it sits, blind under its keel. A railgun does \
             not traverse: the face it sits on is its line of fire.",
            &[],
            "wiki/sections/turret#what-it-can-bear-on",
            None,
            &[],
            &[
                "A turret owns 58.7 percent of the sky. A railgun only owns the face you bolted it to.",
            ],
        ),
        lesson(
            "build_docking_port",
            Shipbuilding,
            70,
            "Bolting on a docking port",
            still(
                "build_docking_port",
                "a docking port seated on a hull face in the editor, its hatch face left clear",
            ),
            "No base hull carries a docking port, so a ship that can dock is one you built. The \
             port carries 90 health, the lightest part on a hull, and takes neighbours on every \
             face but the hatch it docks through.",
            &[],
            "wiki/sections/docking#variants",
            None,
            &[],
            &["No base hull carries a docking port. A ship that can dock is one you built."],
        ),
        lesson(
            "build_dock_envelope",
            Shipbuilding,
            75,
            "Making a dock",
            still(
                "build_dock_envelope",
                "two port faces squared up on a final approach, the docking sight green on all \
                 three counts",
            ),
            "Four things at once: the port faces within 10 m, the ports within 15 degrees of \
             opposed, under 5 m/s of closing rate, under 5 degrees a second of relative spin. Roll \
             is ignored. Docked, your drive and helm are inert.",
            &["dock"],
            "wiki/sections/docking#flying-the-approach",
            None,
            &[],
            &["A dock needs 10 m of gap, 15 degrees of facing, and both hulls almost still."],
        ),
        lesson(
            "build_generate",
            Shipbuilding,
            90,
            "Generate a hull",
            still(
                "build_generate",
                "the editor's Generate block with a seed typed and the hull it rolled standing \
                 on the stage",
            ),
            "Generate rolls a whole hull into the ship you are inside, from a seed you can type or \
             reroll, drawing only the sections you ticked. It replaces what that ship holds. The \
             result is ordinary section nodes, already bound to their kind's key.",
            &[],
            "wiki/keybinds#generate-a-hull",
            None,
            &[],
            &["Generate rolls a hull from a seed and REPLACES what the ship you are inside holds."],
        ),
        lesson(
            "novaos_open",
            NovaOs,
            10,
            "Opening NOVA OS",
            looping(
                "novaos_open",
                "the NOVA OS screen coming up over the cockpit",
            ),
            "Opening NOVA OS freezes the game: the clocks stop, so combat, physics and every \
             projectile hold mid-frame while you read contacts, check your ship and give orders. \
             Nothing moves until you close it.",
            &["novaos_toggle"],
            "wiki/nova-os#opening-and-closing",
            None,
            &[],
            &["Opening NOVA OS freezes the game. The clocks stop until you close the screen."],
        ),
        lesson(
            "novaos_terminal",
            NovaOs,
            15,
            "The prompt",
            looping(
                "novaos_terminal",
                "the prompt completing a subcommand: `ship re` carries a dim `load` suffix, Tab \
                 prints the `ship reload` / `ship repair` row and takes the first, Tab again steps \
                 onto `ship repair`, and a space then Tab lists the hull's live section codes",
            ),
            "NOVA OS has a prompt you type at, and it reads nova>. Completion fills in command \
             names, subcommands and the live section or contact codes an argument wants. It keeps \
             200 lines of history and 500 rows of scrollback, and reddens an unknown word.",
            &[],
            "wiki/nova-os#the-terminal",
            None,
            &[],
            &[
                "NOVA OS has a prompt you type at. Completion fills in commands and live section codes.",
            ],
        ),
        lesson(
            "novaos_view",
            NovaOs,
            20,
            "Turning the model",
            looping(
                "novaos_view",
                "the ship schematic turning on the NOVA OS screen, then snapping back to \
                 its framing",
            ),
            "Drag to turn and pan the ship model, and use reframe to put it back where it started. \
             Moving the model does not move the ship. The game is frozen while the computer is \
             open, so you can look as long as you like.",
            &["novaos_reframe"],
            "wiki/nova-os#the-ship",
            None,
            &[],
            &[],
        ),
        lesson(
            "novaos_commands",
            NovaOs,
            25,
            "What you can type",
            looping(
                "novaos_commands",
                "`help` typed at a wiped prompt and run: the registered command list prints under \
                 it, `help` through `exit` and then `map`, `ship` and their subcommands, each with \
                 the line that says what it does",
            ),
            "help lists everything. log prints the flight log, objectives the open objectives, \
             clear wipes back to the boot report. map and ship hand the screen to an app; map view \
             and ship view print the same data as a table. exit returns to flight.",
            &[],
            "wiki/nova-os#command-reference",
            None,
            &[],
            &[
                "help lists every command. map and ship open an app; map view and ship view print a table.",
            ],
        ),
        lesson(
            "novaos_contacts",
            NovaOs,
            30,
            "Reading contacts",
            still(
                "novaos_contacts",
                "the local-space plot with one hostile picked and its range and \
                 bearing under it",
            ),
            "The MAP app plots every contact around you as a labelled blip: SELF, ALLY-1, HOST-1, \
             OBJ-1, AST-1. Pick one and the readout gives its kind, name, range and bearing. \
             Setting GOTO from here engages the autopilot directly, without taking a radar lock.",
            &["novaos_next", "novaos_prev", "map_goto"],
            "wiki/nova-os#the-map",
            None,
            &[],
            &[],
        ),
        lesson(
            "novaos_service",
            NovaOs,
            35,
            "Repair and reload a section",
            looping(
                "novaos_service",
                "the SHIP app's inspector on a damaged PDC-1: repair fills the integrity meter to \
                 `100% [##########]` and prints `repaired PDC-1`, then reload takes the magazine \
                 from `1/500` back to `500/500`",
            ),
            "Sections carry short codes - HULL-1, THR-1, CTL-1, PDC-1, TRB-1 - stable for the \
             session and shared by the app and the prompt. Select one and repair restores its \
             integrity; reload refills a weapon's magazine. Both are instant, and a hull section \
             refuses reload.",
            &["novaos_next", "novaos_prev", "ship_repair", "ship_reload"],
            "wiki/nova-os#the-ship",
            None,
            &[],
            &["Sections carry short codes like THR-1. Select one and repair or reload acts on it."],
        ),
        lesson(
            "novaos_rebind_section",
            NovaOs,
            45,
            "Rebinding a section",
            looping(
                "novaos_rebind_section",
                "the SHIP app's inspector on PDC-1: arming the rebind puts `PRESS A KEY OR MOUSE \
                 BUTTON - ESC CANCELS` across the panel in amber, and the next key takes the \
                 trigger - the binding line turns from `LMB / Right Trigger 2` to `K / Right \
                 Trigger 2`",
            ),
            "Your thrusters, turrets and tubes fire on per-ship triggers that Settings does not \
             list. Select the section in the SHIP app and arm rebind; the next key or mouse button \
             takes over its keyboard trigger. A reserved flight control is refused by name.",
            &["ship_rebind", "novaos_next", "novaos_prev"],
            "wiki/nova-os#rebinding-a-section",
            None,
            &[],
            &["A section's trigger is per ship, not in Settings. Rebind it in the SHIP app."],
        ),
        lesson(
            "advanced_scenarios",
            Advanced,
            10,
            "Scenarios and campaigns",
            still(
                "advanced_scenarios",
                "the scenario picker with a scenario chosen and its briefing \
                 beside it",
            ),
            "A scenario places a world and wires its objectives: rocks, planets, ships, nav \
             beacons, salvage crates and lights. One scenario is one flight; a campaign is a set \
             played in order. Both load from content files, so a mod can add or replace them.",
            &[],
            "wiki/scenarios#what-a-scenario-places",
            None,
            &[],
            &[],
        ),
        lesson(
            "advanced_mods",
            Advanced,
            20,
            "Mods",
            still(
                "advanced_mods",
                "the mods screen with one enabled mod selected",
            ),
            "Ships, weapons and scenarios all come from content files. A mod \
             adds or replaces those files. The base game is itself a mod, so \
             you can open and read every file in it.",
            &[],
            "create/mod-files",
            None,
            &[],
            &["The base game is itself a mod, and you can read every file in it."],
        ),
        lesson(
            "advanced_bindings",
            Advanced,
            30,
            "Rebinding controls",
            still(
                "advanced_bindings",
                "the settings screen on its bindings tab",
            ),
            "You can rebind the flight, targeting, camera and interface controls in Settings; the \
             two rows that back out of a screen are fixed, and a section's weapon or thruster \
             trigger is set per ship. Every lesson draws the binding you have now.",
            &["main_drive"],
            "wiki/settings#controls",
            None,
            &[],
            &[],
        ),
        lesson(
            "advanced_mouse",
            Advanced,
            35,
            "Mouse sensitivity",
            still(
                "advanced_mouse",
                "the settings screen on its MOUSE group, three sliders and no bindings",
            ),
            "The MOUSE group in Settings holds three sliders: Look scales ship steering, free look \
             and turret aim (100-300%, default 200%); RCS scales mouse translation (100-500%, \
             default 100%); Free Camera scales the free camera. Each 100% is its own baseline; the \
             gamepad is untouched.",
            &[],
            "wiki/settings#mouse",
            None,
            &[],
            &["The MOUSE group is three sliders, not bindings, and each 100% is its own baseline."],
        ),
    ]
}
