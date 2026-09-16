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
            &["hud_cinematic"],
            "wiki/hud",
            None,
            &[],
            &["The velocity sphere points the way you are moving. That is not always where the nose points."],
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
            &["STOP settles into a deadband, not a dead stop: a sideways drift is handed back at up to 7.5 m/s."],
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
            &["There are two lock slots. Lowered writes the white travel lock, raised the red combat lock."],
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
            &["Raising weapons writes the red combat lock and moves the mouse from the helm to turret aim."],
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
            &["Killing a ship's drives costs it thrust, not speed. Its last flight computer leaves it drifting."],
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
            &["A turret's barrel stops 10 degrees below level, so each mount is blind under its own keel."],
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
            &["A torpedo arms 324 m off a small boat and 494 m off a carrier. Inside that it is a dud."],
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
            &["Sections mate socket to socket, and a socket is the only structural link a hull has."],
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
            &["The 21.00-mass skiff pulls 61 m/s2 on two basic drives; the 41.00-mass tug pulls 31."],
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
            &["The flight computer balances your thrusters, so a lopsided ship still flies straight."],
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
            "novaos_open",
            NovaOs,
            10,
            "Opening NOVA OS",
            looping("novaos_open", "the NOVA OS screen coming up over the cockpit"),
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
            still("advanced_mods", "the mods screen with one enabled mod selected"),
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
            still("advanced_bindings", "the settings screen on its bindings tab"),
            "You can rebind the flight, targeting, camera and interface controls in Settings; the \
             two rows that back out of a screen are fixed, and a section's weapon or thruster \
             trigger is set per ship. Every lesson draws the binding you have now.",
            &["main_drive"],
            "wiki/settings#controls",
            None,
            &[],
            &[],
        ),
    ]
}
