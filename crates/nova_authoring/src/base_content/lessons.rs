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
            "You can turn the camera without turning the ship. Use it to look \
             around and find a contact before you choose a heading.",
            &["camera_rotate", "free_look"],
            "wiki/keybinds#targeting-and-camera",
            None,
            &[],
            &["The camera turns without turning the ship. Use it to look around first."],
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
            "STOP hands the ship to the flight computer. The computer turns the \
             ship around and thrusts until your speed reads zero. That takes \
             time and distance, so start it early.",
            &["autopilot_stop"],
            "wiki/flight-autopilot#the-autopilot-flies-the-hull",
            Some(DRILL_STOP_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_STOP_ID],
            &["STOP is not instant. The flight computer thrusts until your speed reads zero."],
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
            "GOTO flies the ship to your current mark and stops there. Mark a \
             contact with the radar first, then give the order. Use it for long \
             transits.",
            &["autopilot_goto"],
            "wiki/flight-autopilot#the-autopilot-flies-the-hull",
            Some(DRILL_AUTOPILOT_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_AUTOPILOT_ID],
            &["GOTO flies to your mark and stops there. Use it for long transits."],
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
            "ORBIT flies a circle around your mark and holds it. It keeps the \
             target in view while you do something else. Taking the controls \
             back turns the flight computer off.",
            &["autopilot_orbit", "autopilot_off"],
            "wiki/gravity-wells#the-dominant-well",
            Some(DRILL_AUTOPILOT_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_AUTOPILOT_ID],
            &["Taking the controls back turns the flight computer off."],
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
            "Hold radar to sweep for contacts and mark one. A tap clears the \
             mark. Autopilot orders and weapon locks both work from the contact \
             you mark here.",
            &["radar_hold"],
            "wiki/targeting-radar#holding-to-sweep",
            Some(DRILL_GUNNERY_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_GUNNERY_ID],
            &[],
        ),
        lesson(
            "combat_stance",
            Combat,
            20,
            "Raising weapons",
            looping(
                "combat_stance",
                "weapons coming up and the flight computer changing how it flies",
            ),
            "Raising weapons changes how the flight computer flies. Instead of \
             taking the shortest line to your mark, it holds the nose where the \
             guns can reach the target.",
            &["combat_stance"],
            "wiki/targeting-radar#stances-and-slots",
            Some(DRILL_GUNNERY_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_GUNNERY_ID],
            &["Raising weapons changes how the flight computer flies the ship."],
        ),
        lesson(
            "combat_components",
            Combat,
            30,
            "Lock a component",
            looping(
                "combat_components",
                "the lock stepping from a whole ship to its drive, then its turret",
            ),
            "A lock can target one section instead of the whole ship. Step it \
             onto the drive to stop the ship moving, or onto a turret to stop \
             it shooting back.",
            &["component_next", "component_prev"],
            "wiki/targeting-radar#per-section-fine-lock",
            Some(DRILL_GUNNERY_ID),
            &[DRILL_GUNNERY_ID],
            &["A lock can target one section of a ship, such as its drive or a turret."],
        ),
        lesson(
            "combat_turrets",
            Combat,
            40,
            "Turret arcs",
            still("combat_turrets", "turret firing arcs drawn over a ship"),
            "A turret only fires within its own arc. Your own ship blocks part \
             of that arc, so a turret cannot shoot a target hidden behind your \
             drive.",
            &[],
            "wiki/sections/turret",
            None,
            &[TUTORIAL_SCENARIO_ID, DRILL_GUNNERY_ID],
            &["A turret only fires within its arc. Your own ship blocks part of it."],
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
            "A torpedo needs a combat lock. The bay drops it cold and the drive \
             catches a moment later, then it steers itself onto the lock. It \
             will not arm until it is well clear of you.",
            &[],
            "wiki/sections/torpedo-bay",
            None,
            &[],
            &["A torpedo needs a combat lock, and it will not arm close to your ship."],
        ),
        lesson(
            "build_sections",
            Shipbuilding,
            10,
            "Ships are built from sections",
            still(
                "build_sections",
                "the editor with one section selected on a frame",
            ),
            "Ships are built from sections bolted to a frame, and each section \
             does one job. Every section you add costs mass, so nothing on a \
             ship is there only for looks.",
            &[],
            "wiki/sections#what-every-section-shares",
            None,
            &[],
            &[],
        ),
        lesson(
            "build_mass",
            Shipbuilding,
            20,
            "Mass and thrust",
            still(
                "build_mass",
                "the same frame with a heavier loadout, its numbers beside it",
            ),
            "Every section you add is mass the drive has to move and stop. The \
             same drive handles a light ship and a heavy ship very differently. \
             The clearest sign is how long STOP takes.",
            &[],
            "wiki/sections#what-every-section-shares",
            None,
            &[],
            &["Every section you add is mass the drive has to move and stop."],
        ),
        lesson(
            "build_balance",
            Shipbuilding,
            30,
            "Thruster placement",
            still(
                "build_balance",
                "thrusters on a lopsided frame firing at different throttles",
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
                "a newly built ship leaving the yard under its own drive",
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
            "NOVA OS runs while you fly. Open it to read contacts, check your \
             ship and give orders. The ship keeps flying while the screen is \
             open.",
            &["novaos_toggle"],
            "wiki/nova-os#opening-and-closing",
            None,
            &[],
            &["NOVA OS runs while you fly. Open it to read contacts and give orders."],
        ),
        lesson(
            "novaos_view",
            NovaOs,
            20,
            "Turning the model",
            looping(
                "novaos_view",
                "the ship model turning under the cursor, then reframing",
            ),
            "Drag to turn and pan the ship model, and use reframe to put it \
             back where it started. Moving the model does not move the ship. \
             The ship keeps flying while you look.",
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
            still("novaos_contacts", "the contacts list with one entry selected"),
            "The contacts list shows what is known about each contact. \
             Selecting one here sets the same mark the flight computer uses for \
             GOTO and ORBIT.",
            &["novaos_next", "novaos_prev"],
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
                "the scenario picker with a campaign expanded",
            ),
            "A scenario is a single flight. A campaign is a set of scenarios \
             played in order. Both are loaded from content files, so a mod can \
             add or replace them.",
            &[],
            "wiki/scenarios#browsing-and-replaying-scenarios",
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
            "You can rebind every control in Settings. Lessons read your \
             current bindings, so the key each lesson shows is the key you have \
             set.",
            &["main_drive"],
            "wiki/settings#controls",
            None,
            &[],
            &[],
        ),
    ]
}
