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
            "Welcome to Lessons! Each lesson is a bite-sized piece of information about the \
             game. Some lessons include a short scenario where you can practice what they teach.",
            &[],
            "wiki/getting-started",
            None,
            &[],
            &[],
        ),
        lesson(
            "start_units",
            StartHere,
            15,
            "Distances and speeds",
            still(
                "start_units",
                "the HUD in a quiet cruise with a target range reading in kilometers beside a \
                 speed chip reading in meters a second",
            ),
            "Distances are shown in meters below one kilometer and in kilometers above it. Speeds \
             are shown in meters per second.",
            &[],
            "wiki/glossary#units",
            None,
            &[],
            &["One cube section is 10 meters on each side."],
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
            "The velocity sphere shows the direction your ship is moving.",
            &[],
            "wiki/hud",
            None,
            &[],
            &["The velocity sphere shows the direction your ship is moving."],
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
            "The labels at the bottom of the screen show the actions your ship can take right now.",
            &[],
            "wiki/hud#flight-readouts",
            Some(DRILL_AUTOPILOT_ID),
            &[],
            &[
                "The labels at the bottom of the screen show the actions your ship can take right now.",
            ],
        ),
        lesson(
            "start_camera",
            StartHere,
            30,
            "Looking around",
            looping("start_camera", "the camera orbiting a stationary ship"),
            "The mouse controls the camera and normally turns the ship with it. Hold free look to \
             turn the camera without turning the ship.",
            &["camera_rotate", "free_look"],
            "wiki/keybinds#targeting-and-camera",
            None,
            &[],
            &["Hold free look to turn the camera without turning the ship."],
        ),
        lesson(
            "start_markers",
            StartHere,
            35,
            "Who is who",
            still(
                "start_markers",
                "four hulls in one row at the same range against black, three of them with \
                 their markers up: a green triangle over a wingman, a red one over a raider, a \
                 grey one over an unaligned hauler, and the nearest hull - the reader's own - \
                 wearing nothing",
            ),
            "A small triangle above a ship shows its allegiance: green for allies, red for \
             hostiles, and grey for neutral ships. Your own ship has no triangle.",
            &[],
            "wiki/hud#allegiance-markers",
            None,
            &[],
            &[
                "Triangles mark allegiance: green for allies, red for hostiles, and grey for \
                 neutral ships.",
            ],
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
            "The HUD starts On. Toggle it to Cinematic for a clean screen, then toggle it again to \
             restore the HUD.",
            &["hud_cinematic"],
            "wiki/hud#what-is-on-screen-and-when",
            None,
            &[],
            &["Cinematic hides the HUD for a clean view."],
        ),
        lesson(
            "start_pause",
            StartHere,
            45,
            "Pausing and retrying",
            still(
                "start_pause",
                "the pause overlay over a live flight, its Resume, Retry, Settings, Back to Main \
                 Menu and Exit rows stacked on the frozen world",
            ),
            "Pausing freezes the world. From the pause menu, you can resume, retry, change settings, \
             return to the main menu, or exit. Retry restarts the current scenario from the beginning.",
            &[],
            "wiki/getting-started#launch-and-start",
            None,
            &[],
            &[],
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
            "Your main thrusters push your ship forward. Turn toward where you want to go, then \
             thrust. To stop, turn around and thrust against your movement.",
            &["main_drive"],
            "wiki/flight-autopilot#manual-flight",
            Some(DRILL_MOMENTUM_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_MOMENTUM_ID],
            &["Main thrusters only push your ship forward."],
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
            "Ships conserve momentum. Releasing the thrusters does not slow you down. Your ship \
             keeps moving until another force changes its speed or direction.",
            &["main_drive"],
            "wiki/flight-autopilot#manual-flight",
            Some(DRILL_MOMENTUM_ID),
            &[DRILL_MOMENTUM_ID],
            &["Releasing the thrusters does not slow your ship down."],
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
            "STOP calculates the minimum action needed to bring your ship to rest. It uses \
             thrusters that already face the right direction and turns the ship only when needed.",
            &["autopilot_stop"],
            "wiki/flight-autopilot#the-autopilot-flies-the-hull",
            Some(DRILL_STOP_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_STOP_ID],
            &["STOP calculates the minimum action needed to bring your ship to rest."],
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
            "CANCEL ends the current autopilot maneuver and returns control to you. Manual flight \
             input also cancels autopilot. Moving the mouse does not.",
            &["autopilot_off", "main_drive", "rcs_modifier"],
            "wiki/flight-autopilot#the-autopilot-flies-the-hull",
            Some(DRILL_STOP_ID),
            &[],
            &[
                "Moving the mouse does not cancel an autopilot maneuver.",
                "Manual flight input cancels autopilot.",
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
            "Hold the RCS modifier and move the mouse to push your ship sideways without turning \
             it. Use the scroll wheel to move up or down. RCS is useful for small corrections.",
            &["rcs_modifier", "rcs_aim"],
            "wiki/flight-autopilot#rcs-fine-docking-thrusters",
            Some(DRILL_STOP_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_STOP_ID],
            &["RCS moves your ship sideways, up, or down without turning it."],
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
            "Gravity pulls your ship toward nearby planets and other large bodies. Heavy and \
             light ships fall at the same rate. The pull ends when you leave the body's sphere of \
             influence.",
            &[],
            "wiki/gravity-wells#sphere-of-influence",
            Some(DRILL_AUTOPILOT_ID),
            &[],
            &["Heavy and light ships fall at the same rate."],
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
            "Mark a contact, then use GOTO to fly toward it automatically. Your ship stops 500 \
             meters from the target's surface.",
            &["autopilot_goto"],
            "wiki/flight-autopilot#the-autopilot-flies-the-hull",
            Some(DRILL_AUTOPILOT_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_AUTOPILOT_ID],
            &["GOTO stops 500 meters from the target's surface."],
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
            "GOTO starts braking early enough to stop near the target. Your ship turns around, \
             uses its main thrusters to slow down, then uses RCS to settle into position.",
            &["autopilot_goto"],
            "wiki/flight-autopilot#the-autopilot-flies-the-hull",
            Some(DRILL_AUTOPILOT_ID),
            &[],
            &["GOTO starts braking early enough to stop near its target."],
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
            "ORBIT circles the gravity well your ship is inside, not the contact you marked. It \
             only works inside a sphere of influence and continues until you cancel it.",
            &["autopilot_orbit"],
            "wiki/gravity-wells#the-dominant-well",
            Some(DRILL_AUTOPILOT_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_AUTOPILOT_ID],
            &["ORBIT circles the gravity well you are inside, not your marked contact."],
        ),
        lesson(
            "flight_dock",
            Flight,
            65,
            "DOCK",
            looping(
                "flight_dock",
                "a tender creeping the last meters onto a moored spar, the last tick going off \
                 the ticked line between the two port crosses, then the dock taken - the sight \
                 goes out and the two hulls hold as one",
            ),
            "DOCK connects your ship to the ship under your travel lock. Both ships need a docking \
             port. When you are close enough, the docking sight helps you align the two ports.",
            &["dock"],
            "wiki/hud#docking-sight",
            None,
            &[],
            &["Both ships need a docking port before they can dock."],
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
            "Hold radar to scan and lock a contact. With weapons lowered, it creates a white travel \
             lock for the autopilot. With weapons raised, it creates a red combat lock for your \
             weapons.",
            &["radar_hold", "radar_clear"],
            "wiki/targeting-radar#holding-to-sweep",
            Some(DRILL_GUNNERY_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_GUNNERY_ID],
            &[
                "Lowered weapons create a travel lock. Raised weapons create a combat lock.",
            ],
        ),
        lesson(
            "combat_lock_ranges",
            Combat,
            12,
            "How far you can lock",
            still(
                "combat_lock_ranges",
                "the radar reticle on a distant gunship with its range printed under the bracket, \
                 and a second hull further out still unlocked",
            ),
            "Lock range depends on the target's radar signature. Large ships can be locked from \
             farther away than small ships. No target can be locked beyond 200 kilometers.",
            &["radar_hold"],
            "wiki/targeting-radar#lock-ranges",
            None,
            &[],
            &["Larger ships can be locked from farther away than smaller ships."],
        ),
        lesson(
            "combat_allegiance",
            Combat,
            15,
            "Who shoots whom",
            still(
                "combat_allegiance",
                "a gunship holding station in a rock pocket with a green triangle over a friendly \
                 gunship, a red one over a raider and a grey one over an unaligned salvage hull",
            ),
            "Ships can be allied, hostile, or neutral to each other. A projectile keeps its \
             shooter's allegiance after it is fired, so your own weapons cannot damage your ship.",
            &[],
            "wiki/factions#the-relation-model",
            None,
            &[],
            &["A projectile keeps its shooter's allegiance after it is fired."],
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
            "Hold combat stance to raise your weapons. Radar creates a combat lock, your weapons \
             become active, and the mouse aims your turrets instead of turning the ship.",
            &["combat_stance"],
            "wiki/targeting-radar#stances-and-slots",
            Some(DRILL_GUNNERY_ID),
            &[TUTORIAL_SCENARIO_ID, DRILL_GUNNERY_ID],
            &[
                "Raising weapons moves the mouse from steering the ship to aiming its turrets.",
            ],
        ),
        lesson(
            "combat_cover",
            Combat,
            25,
            "Cover and the firing line",
            looping(
                "combat_cover",
                "a gunship's tracers crossing to a boulder and dying on it while the red-marked \
                 hostile beyond the stone sits untouched",
            ),
            "Asteroids and other obstacles block both weapons and radar. Put one between your ship \
             and a hostile to break its line of fire and radar lock.",
            &[],
            "wiki/combat-weapons#cover-line-of-fire",
            None,
            &[],
            &["Cover blocks both weapons and radar."],
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
            "Hold a combat lock steady, then select a section of the target. Your turrets aim at \
             that section. Destroying drives removes thrust, while destroying the last flight \
             computer leaves the ship drifting.",
            &["component_next", "component_prev"],
            "wiki/targeting-radar#per-section-fine-lock",
            Some(DRILL_GUNNERY_ID),
            &[DRILL_GUNNERY_ID],
            &["Destroying a ship's drives removes thrust, not its current speed."],
        ),
        lesson(
            "combat_reaches",
            Combat,
            32,
            "How far each weapon reaches",
            still(
                "combat_reaches",
                "a gunship firing its full battery down a long lane, the tracer streams ending in \
                 open space well short of the hostile hull they are laid on, and one of that \
                 hull's torpedoes already past the point the rounds stop",
            ),
            "A weapon's reach comes from its projectile speed and lifetime. PDC rounds reach about \
             2 kilometers, railgun rounds reach 18 kilometers, and torpedoes reach about 30 \
             kilometers.",
            &[],
            "wiki/combat-weapons#three-reaches",
            None,
            &[],
            &["Projectile speed and lifetime determine how far a weapon reaches."],
        ),
        lesson(
            "combat_damage_types",
            Combat,
            35,
            "Kinetic and Pierce",
            looping(
                "combat_damage_types",
                "two identical gun rigs firing down two spaced stacks of hull plates, the \
                 Kinetic lane chipping only the plate it cannot destroy and the Pierce lane \
                 chipping every plate in its stack",
            ),
            "Kinetic and Pierce deal the same damage, but travel through a ship differently. \
             Kinetic continues only after destroying a section. Pierce can damage several sections \
             as it passes through them.",
            &[],
            "wiki/combat-weapons#damage-types",
            None,
            &[],
            &[
                "Kinetic must destroy a section to continue. Pierce can pass through several \
                 sections.",
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
            "A turret can rotate all the way around but cannot aim below its mounting surface. Your \
             own hull does not block its rounds.",
            &[],
            "wiki/sections/turret#what-it-can-bear-on",
            None,
            &[TUTORIAL_SCENARIO_ID, DRILL_GUNNERY_ID],
            &["Turrets can rotate all the way around but cannot aim below their mounts."],
        ),
        lesson(
            "combat_barrel_discipline",
            Combat,
            42,
            "Barrel discipline",
            looping(
                "combat_barrel_discipline",
                "a gunship's dorsal mounts pouring rounds at one bearing, the streams cutting out \
                 the instant the aim swings across and the barrels chase it, and coming back the \
                 moment they settle",
            ),
            "A weapon fires only when its barrel is aimed closely enough at the target. It stops \
             firing while the mount turns, preventing shots that would miss.",
            &[],
            "wiki/sections/turret#barrel-discipline",
            None,
            &[],
            &["Weapons stop firing while their mounts turn toward the target."],
        ),
        lesson(
            "combat_magazines",
            Combat,
            45,
            "Reloading magazines",
            looping(
                "combat_magazines",
                "a gunship's dorsal mounts firing, the ammo ring beside each one losing pips \
                 through the burst and coming back full on one batch a few frames after the \
                 trigger comes up",
            ),
            "Weapons automatically refill their magazines after a short time without firing. \
             Firing again restarts the wait. Long bursts can empty a magazine temporarily, but \
             ammunition never runs out permanently.",
            &[],
            "wiki/combat-weapons#magazines",
            Some(DRILL_GUNNERY_ID),
            &[],
            &["Weapon magazines refill after a short time without firing."],
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
            "Torpedoes require a combat lock. After launch, they drift clear before their engines \
             start and only arm at a safe distance. A torpedo that hits too close to its launcher \
             is a dud.",
            &[],
            "wiki/sections/torpedo-bay",
            None,
            &[],
            &["Torpedoes only arm after moving a safe distance from their launcher."],
        ),
        lesson(
            "combat_torpedo_types",
            Combat,
            55,
            "Serpent and Lance",
            looping(
                "combat_torpedo_types",
                "two torpedoes on the same run, filmed from beside the straight one: the pale \
                 Lance holds its guidance line dead still in frame while the orange Serpent swings \
                 down and away from it on the corkscrew and drops further behind every frame",
            ),
            "Serpent torpedoes weave, making them harder for point defense to hit. Lance torpedoes \
             fly straight and faster, making them easier to hit but giving defenders less time.",
            &[],
            "wiki/sections/torpedo-bay#the-two-run-ins",
            None,
            &[],
            &["Serpents weave. Lances fly straight and faster."],
        ),
        lesson(
            "combat_point_defense",
            Combat,
            60,
            "Your battery defends itself",
            still(
                "combat_point_defense",
                "a gunship with its weapons lowered working its own mounts against an inbound \
                 salvo, a thin line running from each engaged mount out to the torpedo it picked \
                 and its rounds streaming along that line",
            ),
            "With weapons lowered and no combat lock, idle turrets automatically fire at incoming \
             torpedoes. Raise your weapons or take a combat lock to control every turret yourself.",
            &["combat_stance"],
            "wiki/combat-weapons#your-own-battery",
            None,
            &[],
            &["Idle turrets automatically defend your ship from incoming torpedoes."],
        ),
        lesson(
            "combat_railgun",
            Combat,
            70,
            "The hull is the aim",
            looping(
                "combat_railgun",
                "a gunboat committing its spinal railgun on a patrol gunship a hundred and fifty \
                 meters off, the bore sight thread thickening across the gap as the charge runs, \
                 and then the shell opening the target hull into tumbling plates",
            ),
            "A railgun cannot turn. It fires along the direction of the hull face where you mounted \
             it, so you aim by turning the entire ship. Lowering your weapons cancels a charging \
             shot.",
            &[],
            "wiki/sections/railgun#committing-the-shot",
            None,
            &[],
            &["A railgun fires in the direction of the hull face where it is mounted."],
        ),
        lesson(
            "combat_bore_sight",
            Combat,
            80,
            "The bore sight",
            looping(
                "combat_bore_sight",
                "a railgun's sight line arriving over the frame edge and crossing a patrol \
                 gunship, with a blue ring on a section the shot would destroy, and the ring \
                 walking the length of the hull as the gun is aimed off the spine and back",
            ),
            "The bore sight shows the railgun's path through its target. A ring marks each section \
             the shot would destroy, and the line becomes thicker while the railgun charges.",
            &[],
            "wiki/sections/railgun#the-bore-sight",
            None,
            &[],
            &["The bore sight marks every section the railgun shot would destroy."],
        ),
        lesson(
            "combat_collapse",
            Combat,
            90,
            "When a ship comes apart",
            looping(
                "combat_collapse",
                "one siege railgun shell boring a corridor through a block of reinforced hull, and \
                 the block then letting go of every section it has left and opening into a \
                 drifting cloud of plates",
            ),
            "A ship collapses when less than one twentieth of its original structure remains. A \
             ship is neutralized when it loses every weapon or its last flight computer.",
            &[],
            "wiki/ships#taking-a-ship-apart",
            Some(DRILL_GUNNERY_ID),
            &[],
            &[
                "A ship collapses when less than one twentieth of its original structure remains.",
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
            "Ships are built from sections connected socket to socket. These sockets are the only \
             structural links between sections. The build grid uses cells that are 10 meters on \
             each side.",
            &[],
            "wiki/sections#what-every-section-shares",
            None,
            &[],
            &["Sections connect socket to socket."],
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
            "Every section adds mass that your thrusters must move and stop. With the same \
             thrusters, a heavier ship accelerates more slowly. Adding more thrusters can increase \
             its acceleration.",
            &[],
            "wiki/sections/thruster",
            None,
            &[],
            &["Heavier ships need more thrust to accelerate at the same rate."],
        ),
        lesson(
            "build_readout",
            Shipbuilding,
            25,
            "The build readout",
            still(
                "build_readout",
                "the editor rail's stat block on a half-built hull: Turn, Mass, Thrust, HP and \
                 Parts down one column with the limit note under them",
            ),
            "The editor shows your ship's turn rate, mass, thrust, health, and section count. A \
             note below these values explains what currently limits its turn rate.",
            &[],
            "wiki/keybinds#the-build-readout",
            None,
            &[],
            &["The build readout shows what limits your ship's turn rate."],
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
            "The flight computer balances your thrusters to prevent them from spinning the ship. A \
             lopsided or damaged ship can still fly straight, but it may have less available thrust.",
            &[],
            "wiki/sections/thruster",
            None,
            &[],
            &[
                "The flight computer balances thrusters to keep your ship flying straight.",
            ],
        ),
        lesson(
            "build_turning",
            Shipbuilding,
            35,
            "What decides your turn rate",
            still(
                "build_turning",
                "a long spine in the editor, the rail printing its turn ceiling with the \
                 structure-limited note under it",
            ),
            "Turn rate is limited by your flight computers, the ship's mass, and the distance of \
             its outermost sections. Long or heavy ships usually turn more slowly.",
            &[],
            "wiki/sections/controller#what-sets-how-hard-a-ship-turns",
            None,
            &[],
            &["Long or heavy ships usually turn more slowly."],
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
            "Test a new ship before using it in a scenario. Accelerate to speed, then use STOP. \
             This shows how quickly the ship accelerates and how much room it needs to slow down.",
            &[],
            "wiki/flight-autopilot#the-hull-decides-the-handling",
            None,
            &[],
            &["Test a new ship to learn how quickly it accelerates and stops."],
        ),
        lesson(
            "build_stacking",
            Shipbuilding,
            50,
            "A second flight computer",
            still(
                "build_stacking",
                "two flight computers on one spine in the editor, both of them listed in the \
                 scene tree, and the rail naming the structure as what holds the turn rate down",
            ),
            "Most ships are already limited by their structure, so another flight computer does \
             not make them turn faster. It improves turning precision and keeps the ship \
             controllable if another computer is destroyed.",
            &[],
            "wiki/sections/controller#stacking-controllers",
            None,
            &[],
            &["Extra flight computers improve precision and provide a backup."],
        ),
        lesson(
            "build_weapon_mounts",
            Shipbuilding,
            60,
            "Where a weapon can point",
            still(
                "build_weapon_mounts",
                "a turret bolted on a hull's top face and a railgun on its nose, the turret's \
                 card open in the inspector",
            ),
            "A turret can aim in most directions but cannot aim below its mount. A railgun cannot \
             turn and only fires outward from the hull face where it is mounted.",
            &[],
            "wiki/sections/turret#what-it-can-bear-on",
            None,
            &[],
            &["Weapon placement determines where the weapon can fire."],
        ),
        lesson(
            "build_docking_port",
            Shipbuilding,
            70,
            "Bolting on a docking port",
            still(
                "build_docking_port",
                "a docking port seated on a hull face in the editor, its capture envelope open \
                 in the inspector",
            ),
            "Base ships do not include docking ports, so you must add one before a ship can dock. \
             Keep the port's hatch facing outward and clear of other sections.",
            &[],
            "wiki/sections/docking#variants",
            None,
            &[],
            &["A ship needs a docking port before it can dock."],
        ),
        lesson(
            "build_dock_envelope",
            Shipbuilding,
            75,
            "Making a dock",
            looping(
                "build_dock_envelope",
                "a tender lining up on a moored spar: the docking sight's face plates square up \
                 and go green, then the gap line as the hull brakes inside the capture distance, \
                 and a DOCK chip joins the verb row",
            ),
            "To dock, bring the ports within 10 meters, face them toward each other, and reduce the \
             ships' relative movement and spin. Once docked, your ship's steering and thrusters are \
             disabled.",
            &["dock"],
            "wiki/sections/docking#flying-the-approach",
            None,
            &[],
            &["Docking requires two aligned ports and very little relative movement."],
        ),
        lesson(
            "build_skin",
            Shipbuilding,
            80,
            "Ship skin",
            looping(
                "build_skin",
                "a hull standing bare on the editor stage, and Ship Skin switched on: plating \
                 closes over the structure it was derived from, cell by cell",
            ),
            "Ship Skin previews the plating generated from your ship's structure. It updates \
             automatically while you build and does not place additional sections. The setting \
             remains active when you enter Play.",
            &[],
            "wiki/keybinds#the-inspector",
            None,
            &[],
            &["Ship Skin is generated automatically from your ship's structure."],
        ),
        lesson(
            "build_generate",
            Shipbuilding,
            90,
            "Generate a hull",
            looping(
                "build_generate",
                "a three-part stub in the editor with seed 2026 typed in the Generate block, then \
                 the hull Generate rolls in its place",
            ),
            "Generate replaces the current ship with a hull built from a seed and the section types \
             you selected. The result uses ordinary sections that you can continue editing.",
            &[],
            "wiki/keybinds#generate-a-hull",
            None,
            &[],
            &[
                "Generate replaces the current ship with a hull built from your selected seed.",
            ],
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
            "Opening NOVA OS freezes the game. Combat, physics, and projectiles stop while you read \
             information or give commands. The game resumes when you close it.",
            &["novaos_toggle"],
            "wiki/nova-os#opening-and-closing",
            None,
            &[],
            &["Opening NOVA OS freezes the game until you close it."],
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
            "Type commands at the `nova>` prompt. Completion suggests commands, subcommands, section \
             codes, and contact codes. An unknown command turns red so you can correct it before \
             running it.",
            &[],
            "wiki/nova-os#the-terminal",
            None,
            &[],
            &["Prompt completion suggests commands and codes from the current game."],
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
            "Drag to rotate or pan the ship model. Use reframe to return it to its starting \
             position. Moving the model does not move your ship.",
            &["novaos_reframe"],
            "wiki/nova-os#the-ship",
            None,
            &[],
            &["Moving the NOVA OS model does not move your ship."],
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
            "`help` lists every command. `log` shows the flight log, `objectives` shows current \
             objectives, and `clear` resets the prompt. `map` and `ship` open apps, while `exit` \
             returns to flight.",
            &[],
            "wiki/nova-os#command-reference",
            None,
            &[],
            &["Type `help` to list every available command."],
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
            "The MAP app plots nearby contacts and gives each one a short code. Select a contact to \
             see its name, type, range, and bearing. GOTO engages the autopilot without creating a \
             radar lock.",
            &["novaos_next", "novaos_prev", "map_goto"],
            "wiki/nova-os#the-map",
            None,
            &[],
            &[
                "MAP can send your ship toward a contact without creating a radar lock.",
            ],
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
            "Ship sections use short codes such as `THR-1` and `PDC-1`. Select a section to repair \
             its integrity or reload its magazine. Both actions happen immediately.",
            &["novaos_next", "novaos_prev", "ship_repair", "ship_reload"],
            "wiki/nova-os#the-ship",
            None,
            &[],
            &["Select a section in the SHIP app to repair or reload it."],
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
            "Thrusters, turrets, and torpedo bays have per-ship controls that do not appear in \
             Settings. Select a section and start rebind, then press the new key or mouse button. \
             Reserved flight controls cannot be used.",
            &["ship_rebind", "novaos_next", "novaos_prev"],
            "wiki/nova-os#rebinding-a-section",
            None,
            &[],
            &["Section controls are set per ship in the SHIP app."],
        ),
        lesson(
            "novaos_shell",
            NovaOs,
            55,
            "The command shell",
            still(
                "novaos_shell",
                "the same monitor on its cmd> prompt over the main menu, with a run of `status` \
                 answered under it",
            ),
            "The command shell opens over the menu or editor without requiring a ship. It provides \
             commands for the current run, ships, bindings, and settings. Cheat commands must be \
             armed first, which permanently marks the run.",
            &[],
            "wiki/commands#the-commands",
            None,
            &[],
            &["Arming cheat commands permanently marks the current run."],
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
            "A scenario creates one flight with its own world and objectives. A campaign is a group \
             of scenarios played in order. Mods can add new scenarios and campaigns or replace \
             existing ones.",
            &[],
            "wiki/scenarios#what-a-scenario-places",
            None,
            &[],
            &[
                "A scenario is one flight. A campaign is a group of scenarios played in order.",
            ],
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
            "Ships, weapons, scenarios, and other game content come from files that mods can add or \
             replace. The base game is also a mod, so you can inspect its files and learn from \
             them.",
            &[],
            "create/mod-files",
            None,
            &[],
            &["The base game is a mod, and you can inspect all of its content files."],
        ),
        lesson(
            "advanced_ai_flight",
            Advanced,
            25,
            "How an enemy flies",
            looping(
                "advanced_ai_flight",
                "a raider running in on a parked gunship across a rock field, nose already on it, \
                 closing across the frame from one cell to the next",
            ),
            "Enemy ships use the same flight system as your ship. They approach until reaching \
             their preferred distance, then circle while keeping their weapons pointed toward you.",
            &[],
            "wiki/factions#what-allegiance-drives",
            None,
            &[TUTORIAL_SCENARIO_ID],
            &["Enemy ships use the same flight system as your ship."],
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
            "Settings lets you rebind flight, targeting, camera, and interface controls. Section \
             controls are set separately for each ship. Lessons always show your current bindings.",
            &["main_drive"],
            "wiki/settings#controls",
            None,
            &[],
            &[
                "Lessons show your current bindings, including any changes made in Settings.",
            ],
        ),
        lesson(
            "advanced_mouse",
            Advanced,
            35,
            "Mouse sensitivity",
            looping(
                "advanced_mouse",
                "the settings screen on its MOUSE group: three sensitivity sliders and no \
                 bindings, with the Look handle dragged up to 300% and down to 100% as its \
                 readout counts",
            ),
            "Settings has separate sensitivity sliders for Look, RCS, and Free Camera. Look affects \
             steering, free look, and turret aim. RCS affects mouse translation. These settings do \
             not change gamepad sensitivity.",
            &[],
            "wiki/settings#mouse",
            None,
            &[],
            &["Look, RCS, and Free Camera have separate mouse sensitivity settings."],
        ),
        lesson(
            "advanced_graphics",
            Advanced,
            40,
            "Graphics presets",
            still(
                "advanced_graphics",
                "the settings screen on its graphics row with the Low, Medium and High preset \
                 picked out",
            ),
            "Low, Medium, and High presets trade visual detail for performance. High includes \
             camera shake and hit flashes. Medium removes camera shake. Low also removes hit \
             flashes and particles and renders the world at a lower resolution.",
            &[],
            "wiki/settings#graphics-quality",
            None,
            &[],
            &["Lower graphics presets remove visual effects to improve performance."],
        ),
        lesson(
            "advanced_audio",
            Advanced,
            45,
            "Audio mix",
            still(
                "advanced_audio",
                "the settings screen on its audio tab: the Master, Interface, World and Music \
                 sliders with their levels as whole percents",
            ),
            "Master, Interface, World, and Music have separate volume controls. Interface covers \
             menus, the HUD, and the flight computer. World covers sounds in the game world. Master \
             changes the final volume of every channel.",
            &[],
            "wiki/settings#audio",
            None,
            &[],
            &[
                "Master volume changes every channel while preserving their individual levels.",
            ],
        ),
    ]
}
