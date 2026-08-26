//! system_ship_editor: the ship editor, BUILT and INSPECTED through synthesized pointer input.
//!
//! This runs the exact same editor the `nova_protocol` binary launches (via the shared
//! [`editor_app`]), just with the autopilot + screenshot harness attached. Every beat is a real
//! gesture at a real screen position - the run clicks the menu, the rail buttons and the gallery
//! tiles by `Name`, and it clicks the ship itself by projecting a section onto the viewport. No
//! editor code is reached by triggering its observer or by inserting its state component.
//!
//! The arc, one beat per gesture:
//!
//! 1. click Sandbox in the main menu, which is also the smoke coverage for the menu itself;
//! 2. click Add Ship - a BLANK ship, entered - then arm a controller and FOUND it with a click
//!    on empty space, which lands the first section at the ship's own origin. The founded
//!    controller also keeps the editor-preview controller fix (task 20260706-212909) honest: a
//!    live controller on the non-physics preview root used to flood the log every frame, so this
//!    run staying quiet is the regression check;
//! 3. arm a hull through the parts gallery - the editor's only parts picker;
//! 4. place TWO sections by clicking the ship through the real picking pipeline (avian's
//!    physics-picking backend raycasts the pointer to a hit, and the editor's own
//!    `on_click_spaceship_section` observer places the section);
//! 5. Escape puts the part down, and a click on the ship in select mode places NOTHING - it
//!    SELECTS the section under the pointer, the same mark its tree row would take;
//! 6. press Del - what the select click MARKED is what goes, at whatever depth it sits, and the
//!    count drops back;
//! 7. walk the gallery end to end - browse (the tiles are up), filter (typing narrows the grid to
//!    one part), focus (the card names that part), select (Place arms the tool and closes the
//!    gallery) and place (a click on the ship builds the part the gallery picked);
//! 8. meet both placement REFUSALS in words and in pixels - an occupied socket, and a drive
//!    aimed up a lane the hull already stands beside, which is `nova_ship`'s clearance rule and
//!    the same one the ship generator collapses under;
//! 9. place a WORLD object from the rail's palette, TYPE A NEW RADIUS INTO ITS INSPECTOR and
//!    delete it again - the scenario node edits the range it stands on, not just the ships parked
//!    on it, and a placed object is authorable rather than stuck with what the palette handed it;
//! 10. turn the SKIN on and watch it follow the build - the bare ship, the same ship clad from
//!    its own structure, and the cladding reflowing around a hull that is still in the
//!    builder's hand. Play then proves the toggle rode the hand-off: the flown ship wears it.
//!
//! Controls (interactive run): use the on-screen buttons to create ships and place sections.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_ship_editor --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `editor: ...` verdict lines per beat,
//! #           `autopilot: cycle complete, no panic`
//! ```

// Only the debug-gated autopilot below names bevy types directly.
#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_ship_editor")]
#[command(version = "1.0.0")]
#[command(about = "The nova_protocol ship editor, built and inspected by synthesized pointer input. Autopilot-only correctness range - run the game to use the editor", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same editor app the game/binary runs - not a bespoke copy.
    let mut app = editor_app(true, None);

    // Headless smoke-test harness: inert in a normal run (gated on NOVA_AUTOPILOT).
    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env): run
        // timeline + engine-bound invariants + frame-time capture.
        //
        // The capture is GATED on the flown range, and that gate is why
        // measuring here is worth anything. [`FLOWN_RANGE`] is the scenario the
        // editor hands off to on Play, so it is the range a player is most
        // likely to be sitting in - and nothing else in the catalog can load
        // it, because the editor registers it at hand-off instead of shipping
        // it in `GameScenarios`. Ungated, the window closed while the walk was
        // still clicking gallery tiles: the row said `system_ship_editor` and meant
        // the editor's build UI.
        app.add_plugins(
            nova_probe::NovaProbePlugin::default().ready_frametime(|world: &World| {
                world.get_resource::<CurrentScenario>().is_some_and(|live| {
                    live.0
                        .as_ref()
                        .is_some_and(|scenario| scenario.id == FLOWN_RANGE)
                })
            }),
        );
        app.init_resource::<EditorWalk>();
        app.add_plugins(nova_screenshot(editor_script()));
        framelog(&mut app);
    }

    app.run()
}

/// Arms the per-frame diagnostic below. Off by default: it writes a line every
/// frame, which is a diagnostic instrument and not a thing a smoke run wants.
#[cfg(feature = "debug")]
const FRAMELOG_ENV: &str = "NOVA_EDITOR_FRAMELOG";

/// Per-frame wall time and fixed-step count, so a slow beat can be told from a
/// slow editor.
///
/// The frame-time CAPTURE reports one mean and one max over a 900-frame window
/// and cannot say which gesture paid for them. This writes a line per frame,
/// which the `autopilot: step ... begins` lines split into beats.
///
/// `Time<Real>` on purpose: `Time<Virtual>` is clamped by `max_delta`, so a
/// frame that took a second of wall clock reports 250 ms there.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct FrameLog {
    /// Fixed steps run since the last report.
    steps: u32,
    /// Rendered frames since the run started.
    index: u32,
}

#[cfg(feature = "debug")]
fn framelog(app: &mut App) {
    if std::env::var(FRAMELOG_ENV).is_err() {
        return;
    }
    app.init_resource::<FrameLog>();
    app.add_systems(FixedUpdate, |mut log: ResMut<FrameLog>| log.steps += 1);
    app.add_systems(Last, report_frame);
}

#[cfg(feature = "debug")]
fn report_frame(world: &mut World) {
    let delta = world.resource::<Time<Real>>().delta_secs_f64() * 1000.0;
    // `Entities::len` is the high-water mark of allocated rows rather than the
    // live count, so sum the archetypes (same reading as `bug_sandbox_soak`).
    let entities: u32 = world
        .archetypes()
        .iter()
        .map(bevy::ecs::archetype::Archetype::len)
        .sum();
    let step_ms = world
        .resource::<bevy::diagnostic::DiagnosticsStore>()
        .iter()
        .find(|diagnostic| diagnostic.path().as_str() == "avian/total_step_time")
        .and_then(bevy::diagnostic::Diagnostic::smoothed)
        .unwrap_or_default();
    let mut log = world.resource_mut::<FrameLog>();
    let steps = std::mem::take(&mut log.steps);
    log.index += 1;
    let index = log.index;
    info!(
        "framelog f={index} ms={delta:.1} steps={steps} entities={entities} step_ms={step_ms:.2}"
    );
}

/// The scenario Play hands off to: the editor's own open range, registered at
/// hand-off rather than shipped, so no id-driven rig can reach it. Named here
/// because the frame-time capture waits for it - see the probe wiring above.
#[cfg(feature = "debug")]
const FLOWN_RANGE: &str = "editor_sandbox";

/// In-step seconds a beat of this walk gets before the run gives up on it.
///
/// THE ONLY NUMBER LEFT. Every wait here is a condition on the app - the widget
/// laid out, the picking pointer registering the press, the editor solving a
/// placement, the section landing - so this is a backstop rather than a settle:
/// a healthy run satisfies each beat within a frame or two of the gesture and
/// never comes near it. That is why it can be generous against a
/// software-rendered CI GPU without slowing a good run down, where the frame
/// count it replaced was paid in full by every beat on every machine.
///
/// What it buys is the diagnostic. A frame count sails past whatever it was
/// guessing at, and the run then fails several beats later on a snapshot
/// assertion that raced; an unmet condition fails AT its beat, naming it - "the
/// editor never solved a placement there" instead of "expected 5 sections, got
/// 4".
#[cfg(feature = "debug")]
const BEAT_DEADLINE_SECS: f32 = 20.0;

/// How far, in pixels, the stage drag carries a grabbed node.
///
/// Short on purpose. The transform rig is scaled to a fixed share of the view,
/// so its innermost handle geometry stands about 44 pixels off the node's
/// origin at ANY camera distance; a longer sweep from the middle of a node
/// crosses a ring and turns the ship instead of sliding it. Even this much is
/// tens of world units out where the whole range is framed.
#[cfg(feature = "debug")]
const DRAG_SPAN: f32 = 25.0;

/// In-step seconds the two asset-gated beats get: reaching the main menu, and
/// reaching gameplay through it. Sized to outlast a cold load on a
/// software-rendered CI GPU, and kept under the harness completion deadline so
/// a stall names the beat rather than tripping the generic hang detector.
#[cfg(feature = "debug")]
const BOOT_DEADLINE_SECS: f32 = 90.0;

/// In-step seconds the hand-off beats get: Play tears the editor down and loads
/// a scenario, which is a second asset-gated wait rather than a gesture.
#[cfg(feature = "debug")]
const PLAY_DEADLINE_SECS: f32 = 60.0;

/// The hull the run builds with.
#[cfg(feature = "debug")]
const HULL_PROTOTYPE: &str = "reinforced_hull_section";

/// The part the run FOUNDS the blank ship with. A controller, so the founded
/// ship is also the regression check for the editor-preview controller fix.
#[cfg(feature = "debug")]
const CONTROLLER_PROTOTYPE: &str = "basic_controller_section";

/// The top bar's Add menu, the door every "one more node" gesture goes through.
#[cfg(feature = "debug")]
const MENU_ADD: &str = "Add Menu Button";

/// The top bar's File menu: the verbs that touch the saved file.
#[cfg(feature = "debug")]
const MENU_FILE: &str = "File Menu Button";

/// The top bar's View menu, where the camera gestures are listed.
#[cfg(feature = "debug")]
const MENU_VIEW: &str = "View Menu Button";

/// The Scene tree's row names all start with this, then the node's own id.
#[cfg(feature = "debug")]
const SCENE_ROW: &str = "Scene Row ";

/// The top bar's Edit menu: what can be done to the selection.
#[cfg(feature = "debug")]
const MENU_EDIT: &str = "Edit Menu Button";

/// The top bar's Ship menu: the verbs of the ship the editor is inside.
#[cfg(feature = "debug")]
const MENU_SHIP: &str = "Ship Menu Button";

/// A viewport point (logical px) with neither the ship nor a docked panel
/// under it, on the 1024x768 window the app opens: low on the screen, and
/// inside the 210..724 band the rail and the Inspector leave the stage.
/// Pointing here is how a beat puts the ghost away without disarming the part
/// it is holding.
#[cfg(feature = "debug")]
const EMPTY_SPACE: Vec2 = Vec2::new(560.0, 640.0);

/// What a beat measured, so a later beat can say whether the gesture changed
/// anything.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct EditorWalk {
    /// The hull section the run places, resolved from the catalog once the
    /// component cards are up.
    hull: String,
    /// Live section count, stamped by the beat before the one that checks it.
    sections: usize,
    /// Gallery tiles on screen, stamped before the filter beat narrows them.
    tiles: usize,
    /// Mates the editor's assembled ship derives, to compare against the flown
    /// ship's.
    mates: usize,
    /// Skin plates on the build, stamped before the beat that drags a part
    /// through them.
    plates: usize,
    /// The id a select-mode click marked, so the beat that presses Del can say
    /// the tree lost THAT node rather than merely one fewer.
    marked: String,
    /// The edited ship's section ids, stamped before the run leaves it. Ids are
    /// the document's own key, so coming back to the same ones is what says the
    /// tree survived the trip.
    ids: Vec<String>,
    /// Where the first ship stood before the drag beat grabbed it.
    first_ship_at: Vec3,
    /// Where the camera stood before a framing gesture, so the beat after can
    /// say the gesture moved it.
    camera_at: Vec3,
    /// The id of the object the picker beats placed. Stamped rather than
    /// guessed from the stem: the sandbox range already stands two beacons of
    /// its own, so "no beacon in the document" is never true.
    placed: String,
    /// What the rock's Position row read before one of its three boxes was
    /// retyped, so the beat after can say the other two stood still.
    position: String,
    /// What the beacon's colour row read before the picker touched it.
    colour: String,
    /// Where the floating picker stood before the beat that drags it.
    window_at: Vec2,
    /// The scenario node's whole listing, stamped before the run saves it. The
    /// document that comes back off disk has to be this one, id for id.
    document: Vec<String>,
    /// The same listing's poses, so the round trip is proved on WHERE each node
    /// stands as well as on which nodes there are.
    document_at: Vec<(String, Vec3)>,
}

/// The whole driven run.
///
/// Every beat waits on a CONDITION - a widget laid out, the picking pointer
/// registering a press, the editor solving a placement, a section landing - and
/// carries a deadline. Nothing here counts frames: a frame is not a unit of
/// work, and the same `frames(10)` was about 20 ms on a workstation and about
/// 600 ms on lavapipe while saying nothing about whether the editor had
/// finished reacting to anything.
///
/// A gesture beat and its VERDICT beat stay separate: the wait is what makes
/// the verdict safe to read, and a verdict that panics names the beat it
/// belongs to instead of stalling the step it was folded into.
#[cfg(feature = "debug")]
fn editor_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("editor: reach the main menu")
        .until(state_is(GameStates::MainMenu))
        .deadline(BOOT_DEADLINE_SECS)
        .add()
        .click_a_widget("editor: click Sandbox", "Sandbox Button")
        .step("editor: Sandbox reached gameplay")
        .until(state_is(GameStates::Playing))
        .deadline(BOOT_DEADLINE_SECS)
        .add()
        .click_a_menu_item("editor: click Add > New Ship", MENU_ADD, "Add Ship Button")
        .step("editor: the blank ship is up and entered")
        .until(inside_a_ship_of(0))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // A blank ship has no view for the placement ray to hit, so the first
        // part FOUNDS it: armed from the gallery, dropped on empty space,
        // landing at the ship's own origin.
        .arm_from_the_gallery("editor: arm the controller", CONTROLLER_PROTOTYPE)
        .step("editor: point at empty space")
        .on_enter(|world: &mut World| {
            move_cursor(EMPTY_SPACE)(world);
            stamp_sections(world);
        })
        .add()
        .press_and_release("editor: found the ship", sections_grew_by(1))
        .step("editor: the founding section is at the origin")
        .on_enter(|world: &mut World| {
            let section = world
                .resource::<EditorProbe>()
                .ship
                .first()
                .expect("the founding click placed a section")
                .clone();
            assert_eq!(
                section.id,
                format!("{CONTROLLER_PROTOTYPE}_1"),
                "the founding section's id is minted from its prototype"
            );
            assert_eq!(
                section.position,
                Vec3::ZERO,
                "the first part founds the ship at its own origin"
            );
            let hull = hull_section_name(world).expect("the catalog lists a hull section");
            nova_probe::probe_marker(
                world,
                "outcome: a blank ship is founded at its origin",
                serde_json::json!({}),
            );
            info!(
                "editor: founded the ship with `{}`, will build with `{hull}`",
                section.id
            );
            world.resource_mut::<EditorWalk>().hull = hull;
        })
        .add()
        // Arm the hull through the gallery - the editor's only parts picker now
        // that the component drawer is gone.
        .arm_from_the_gallery("editor: arm the hull", HULL_PROTOTYPE)
        .step("editor: stamp the count before building")
        .on_enter(stamp_sections)
        .add()
        .click_the_ship(
            "editor: place the first section",
            editor_placement_solved(),
            sections_grew_by(1),
        )
        .click_the_ship(
            "editor: place the second section",
            editor_placement_solved(),
            sections_grew_by(2),
        )
        .step("editor: two sections were placed")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().sections;
            let now = count_sections(world);
            assert_eq!(
                now,
                before + 2,
                "two pointer clicks on the ship must place two sections"
            );
            nova_probe::probe_marker(
                world,
                "outcome: two clicks place two sections",
                serde_json::json!({}),
            );
            info!("editor: placed 2 sections ({before} -> {now})");
            stamp_sections(world);
        })
        .add()
        // Escape is how a builder puts the part down now that select mode is
        // the default rather than a rail button. The armed tool is READ rather
        // than inferred: the probe reporting Select is the claim.
        .step("editor: press Escape to put the part down")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .click_the_ship(
            "editor: click the ship in select mode",
            the_pointer_is_on_the_ship(),
            sections_grew_by(0),
        )
        .step("editor: select mode selected instead of placing")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().sections;
            let now = count_sections(world);
            assert_eq!(
                now, before,
                "select mode must not place anything; the same click built a \
                 section a moment ago"
            );
            let marked = world.resource::<EditorProbe>().selected_node.clone();
            let marked =
                marked.expect("a click in select mode marks the section under the pointer");
            world.resource_mut::<EditorWalk>().marked = marked.clone();
            nova_probe::probe_marker(
                world,
                "outcome: select mode selects, and places nothing",
                serde_json::json!({}),
            );
            info!("editor: select mode marked '{marked}' and placed nothing ({now} sections)");
        })
        .add()
        // Delete acts on the SELECTION now, not on a brush: what the click
        // above marked is what Del takes off, and it takes it off at whatever
        // depth the mark sits.
        .step("editor: press Del to delete the marked section")
        .on_enter(press_key(KeyCode::Delete))
        .until(sections_shrank_by(1))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: release Del")
        .on_enter(release_key(KeyCode::Delete))
        .add()
        .step("editor: the marked section is the one that went")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().sections;
            let marked = world.resource::<EditorWalk>().marked.clone();
            let now = count_sections(world);
            assert_eq!(
                now,
                before - 1,
                "Del must remove exactly the section the click marked"
            );
            assert!(
                !edited_section_ids(world).contains(&marked),
                "the tree still carries '{marked}', the id Del was aimed at"
            );
            assert_eq!(
                world.resource::<EditorProbe>().selected_node,
                None,
                "deleting the marked node must leave nothing marked"
            );
            nova_probe::probe_marker(
                world,
                "outcome: Del removes the marked section",
                serde_json::json!({}),
            );
            info!("editor: Del removed '{marked}' ({before} -> {now})");
        })
        .add()
        // The parts gallery, walked the way a player walks it: browse, filter,
        // focus, select, place. Every beat is a real gesture on a real widget;
        // what it WAITS on is the gallery's own state, so a grid that never
        // narrowed fails at the filter rather than at whatever the next Enter
        // happened to arm.
        .click_a_menu_item("editor: open the parts gallery", MENU_SHIP, "Parts Item")
        .step("editor: the gallery is browsing the catalog")
        .until(and(editor_gallery_open(), some_gallery_tiles()))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the gallery lists the catalog")
        .on_enter(|world: &mut World| {
            let tiles = count_named_with_prefix(world, GALLERY_TILE);
            assert!(
                tiles > 0,
                "opening the gallery must list the browsable prototypes"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the gallery lists the catalog",
                serde_json::json!({}),
            );
            info!("editor: gallery is up with {tiles} tiles");
            world.resource_mut::<EditorWalk>().tiles = tiles;
        })
        .add()
        // The two gallery figures. `shoot` writes nothing unless NOVA_CAPTURE
        // is armed, and `shot_written` is inert in the same case, so the smoke
        // run walks these beats without waiting on a file.
        .step("editor: shoot the gallery grid")
        .on_enter(|world: &mut World| shoot(world, "editor-gallery.png"))
        .until(shot_written("editor-gallery.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // Typing reaches the filter only once the field has the caret. `/` is
        // the keyboard way to give it one - the mouse way (a click on the
        // field) is what `arm_from_the_gallery` drives.
        .step("editor: put the caret in the filter")
        .on_enter(press_key(KeyCode::Slash))
        .until(editor_filter_focused())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: release /")
        .on_enter(release_key(KeyCode::Slash))
        .add()
        .step("editor: filter the gallery")
        .on_enter(|world: &mut World| {
            let needle = filter_needle(world);
            type_text(needle)(world);
        })
        .until(the_gallery_narrowed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the filter narrowed the grid to the hull")
        .on_enter(|world: &mut World| {
            let hull = world.resource::<EditorWalk>().hull.clone();
            let before = world.resource::<EditorWalk>().tiles;
            let now = count_named_with_prefix(world, GALLERY_TILE);
            assert!(
                now < before,
                "typing a name into the gallery must narrow it ({before} -> {now})"
            );
            assert!(
                ui_node_rect(world, &format!("{GALLERY_TILE}{hull}")).is_some(),
                "the filtered grid must still list `{hull}`"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the filter narrows the gallery",
                serde_json::json!({}),
            );
            info!("editor: filter narrowed the gallery ({before} -> {now} tiles)");
        })
        .add()
        .step("editor: focus the filtered tile")
        .on_enter(|world: &mut World| {
            let hull = world.resource::<EditorWalk>().hull.clone();
            click_named(format!("{GALLERY_TILE}{hull}"))(world);
        })
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: release the tile")
        .on_enter(release_mouse(MouseButton::Left))
        .until(and(
            pointer_released(),
            ui_node_present("Gallery Focus Card"),
        ))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the focus card names the part and reads its stats")
        .on_enter(|world: &mut World| {
            let hull = world.resource::<EditorWalk>().hull.clone();
            let lines = subtree_text(world, "Gallery Focus Card");
            assert!(
                lines.contains(&hull),
                "the focus card must name `{hull}`; it read {lines:?}"
            );
            for stat in ["HP", "Size", "Sockets"] {
                assert!(
                    lines.iter().any(|line| line == stat),
                    "the focus card must read `{stat}`; it read {lines:?}"
                );
            }
            nova_probe::probe_marker(
                world,
                "outcome: the focus card names the part",
                serde_json::json!({}),
            );
            info!("editor: focus card reads `{hull}`");
        })
        .add()
        .step("editor: shoot the gallery focus card")
        .on_enter(|world: &mut World| shoot(world, "editor-gallery-focus.png"))
        .until(shot_written("editor-gallery-focus.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click_a_widget("editor: place the focused part", "Gallery Place Button")
        .step("editor: the gallery closed and armed the tool")
        .until(and(editor_gallery_closed(), editor_part_armed()))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the build view is back")
        .on_enter(|world: &mut World| {
            assert!(
                ui_node_rect(world, "Parts Gallery").is_none(),
                "placing from the gallery must close it"
            );
            stamp_sections(world);
        })
        .add()
        .click_the_ship(
            "editor: place the gallery's pick",
            editor_placement_solved(),
            sections_grew_by(1),
        )
        .step("editor: the gallery's pick was placed")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().sections;
            let now = count_sections(world);
            assert_eq!(
                now,
                before + 1,
                "the part selected in the gallery must be the one a click builds"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the gallery pick builds",
                serde_json::json!({}),
            );
            info!("editor: placed the gallery's pick ({before} -> {now})");
            stamp_sections(world);
        })
        .add()
        // The menu figure, taken with a part still in hand: the Ship menu is
        // where the pose verbs live, and this is the one state that shows both
        // halves of a menu row's tail - a live key chip and a greyed one.
        .click_a_widget("editor: drop the Ship menu", MENU_SHIP)
        .step("editor: shoot the Ship menu")
        .on_enter(|world: &mut World| shoot(world, "editor-menu.png"))
        .until(shot_written("editor-menu.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("editor: close the menu")
        .on_enter(press_key(KeyCode::Escape))
        .add()
        .step("editor: release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .until(no_menu_is_open())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Snapping: the roll is the one degree of freedom a mate leaves, so a
        // rolled part must still mate. R turns the ghost a quarter turn about
        // the mating axis before the click commits it. The pose is applied in
        // the same frame the press is read, and the click that follows waits for
        // a SOLVED placement, so there is nothing here to settle for.
        .step("editor: roll the ghost a quarter turn")
        .on_enter(press_key(KeyCode::KeyR))
        .add()
        .step("editor: release R")
        .on_enter(release_key(KeyCode::KeyR))
        .add()
        .click_the_ship(
            "editor: place the rolled part",
            editor_placement_solved(),
            sections_grew_by(1),
        )
        .step("editor: the ship is one structure, with a rolled mate in it")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().sections;
            let now = count_sections(world);
            assert_eq!(now, before + 1, "the rolled part was built");
            assert!(
                sections_with_a_rotation(world) > 0,
                "R must leave a section rolled off its parent's frame"
            );

            // The claim snapping exists to make: what the editor built is a
            // ship the RUNTIME derivation accepts - one connected structure,
            // every mate unambiguous.
            let mates = mate_graph(world, None).unwrap_or_else(|error| {
                panic!("the assembled ship must derive one connected graph: {error}")
            });
            assert!(
                mates >= now - 1,
                "a connected ship of {now} sections needs at least {} mates, derived {mates}",
                now - 1
            );
            nova_probe::probe_marker(
                world,
                "outcome: the build derives one connected graph",
                serde_json::json!({}),
            );
            info!("editor: {now} sections, {mates} mates, one connected structure");
            world.resource_mut::<EditorWalk>().mates = mates;
        })
        .add()
        // The FAST path through the same picker: Tab opens it, and a part is
        // taken by pointing at it and pressing Q. No focus card, no Place
        // button, no click. This is the shape a builder repeats all session, so
        // it is walked as a whole gesture rather than trusted to unit tests.
        .step("editor: open the gallery with Tab")
        .on_enter(press_key(KeyCode::Tab))
        .until(and(editor_gallery_open(), some_gallery_tiles()))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: release Tab")
        .on_enter(release_key(KeyCode::Tab))
        .add()
        .step("editor: Tab put the gallery up")
        .on_enter(|world: &mut World| {
            assert!(
                ui_node_rect(world, "Parts Gallery").is_some(),
                "Tab must open the parts gallery from the build view"
            );
            stamp_sections(world);
        })
        .add()
        // The pipette reads the tile under the POINTER, so the hover is the
        // thing to wait on: Q on a tile the picking backend has not hovered yet
        // takes nothing at all, silently.
        .step("editor: hover a tile")
        .on_enter(|world: &mut World| {
            let hull = world.resource::<EditorWalk>().hull.clone();
            hover_named(format!("{GALLERY_TILE}{hull}"))(world);
        })
        .until(the_hull_tile_is_hovered())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: take it with Q")
        .on_enter(press_key(KeyCode::KeyQ))
        .until(and(editor_gallery_closed(), editor_part_armed()))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: release Q")
        .on_enter(release_key(KeyCode::KeyQ))
        .add()
        .step("editor: Q closed the gallery holding that part")
        .on_enter(|world: &mut World| {
            assert!(
                ui_node_rect(world, "Parts Gallery").is_none(),
                "taking a part with Q must hand the builder back to the ship"
            );
        })
        .add()
        .click_the_ship(
            "editor: place the part Q took",
            editor_placement_solved(),
            sections_grew_by(1),
        )
        .step("editor: the pipette's part was placed")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().sections;
            let now = count_sections(world);
            assert_eq!(
                now,
                before + 1,
                "hover + Q must arm the hovered part, so the next click builds it"
            );
            nova_probe::probe_marker(
                world,
                "outcome: hover and Q arm the part",
                serde_json::json!({}),
            );
            info!("editor: placed the part Q took ({before} -> {now})");
        })
        .add()
        // The SKIN, which is the one part of the build view nobody builds:
        // cladding derived from the structure under it. Three figures - the
        // bare ship, the same ship clad, and the cladding closing around a hull
        // that is still in the builder's hand.
        //
        // The hull the pipette took stays armed throughout. Pointing AWAY from
        // the ship is what puts the ghost away for the first two figures, and
        // the editor SAYS when it has: with nothing under the pointer there is
        // no placement to solve, so no ghost and no status line.
        .step("editor: look away from the ship")
        .on_enter(move_cursor(EMPTY_SPACE))
        .until(editor_placement_clear())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: shoot the bare build")
        .on_enter(|world: &mut World| {
            assert_eq!(
                count_plates(world),
                0,
                "nothing is clad until the toggle asks for it"
            );
            shoot(world, "editor-skin-off.png");
        })
        .until(shot_written("editor-skin-off.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click_a_widget("editor: click Ship Skin", "Ship Skin Toggle")
        .step("editor: the skin closed over the build")
        .until(the_skin_is_on())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the build came up clad")
        .on_enter(|world: &mut World| {
            let plates = count_plates(world);
            assert!(
                plates > 0,
                "the toggle must clad the ship on the stage, derived from the \
                 structure the builder assembled"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the skin clads the build",
                serde_json::json!({}),
            );
            info!("editor: the skin laid {plates} plates over the build");
            world.resource_mut::<EditorWalk>().plates = plates;
            stamp_sections(world);
            shoot(world, "editor-skin.png");
        })
        .until(shot_written("editor-skin.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The feature itself: a hull held against the ship is structure the
        // skin can already see, so the cladding closes around it BEFORE the
        // click that would commit it. The pointer goes to the same face the PDC
        // mounts on two beats later, which is this run's own evidence that the
        // aim lands on a socket a part can take.
        .step("editor: hold the hull against the ship")
        .on_enter(|world: &mut World| {
            let (centre, _) = aim_at_a_visible_face(world).expect("a section faces the camera");
            move_cursor(centre)(world);
        })
        .until(the_skin_reflowed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the skin reflowed around the part in hand")
        .on_enter(|world: &mut World| {
            let settled = world.resource::<EditorWalk>().plates;
            let now = count_plates(world);
            assert_ne!(
                now, settled,
                "the part under the pointer must reflow the skin around it \
                 ({settled} plates with it out of the way)"
            );
            assert_eq!(
                count_sections(world),
                world.resource::<EditorWalk>().sections,
                "and it must reflow without anything being BUILT - the click \
                 has not happened yet"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the skin reflows around the held part",
                serde_json::json!({}),
            );
            info!("editor: the ghost reflowed the skin ({settled} -> {now} plates)");
            shoot(world, "editor-skin-drag.png");
        })
        .until(shot_written("editor-skin-drag.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The refusal path needs a socket that is taken but still POINTABLE, so
        // the run mounts the compact PDC on a hull face: it fills that face's
        // socket while covering only a corner of the face. It is also the part
        // that proves a mount authored at its own size mates against a unit
        // cube at all - which is what `box_link_points` is for.
        .arm_from_the_gallery(
            "editor: arm the shared PDC turret",
            "pdc_kinetic_turret_section",
        )
        .step("editor: aim at the face the camera can see")
        .on_enter(|world: &mut World| {
            let (centre, _) = aim_at_a_visible_face(world).expect("a section faces the camera");
            move_cursor(centre)(world);
            stamp_sections(world);
        })
        .until(editor_placement_solved())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .press_and_release("editor: press on the free face", sections_grew_by(1))
        .step("editor: the module mounted on that face")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().sections;
            let now = count_sections(world);
            assert_eq!(
                now,
                before + 1,
                "a mount authored at its own size must mate onto a unit-cube \
                 hull face - one turret for every craft is the whole point"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the shared mount fits a hull face",
                serde_json::json!({}),
            );
            info!("editor: mounted the shared PDC ({before} -> {now})");
        })
        .add()
        // An ANSWER is what this waits for, either one: `or` holds the beat
        // until the solver has spoken, and the verdict below is what says which
        // answer it had to be.
        .step("editor: aim at the same socket, now occupied")
        .on_enter(|world: &mut World| {
            let (_, off_centre) = aim_at_a_visible_face(world).expect("the same face is up");
            move_cursor(off_centre)(world);
        })
        .until(or(editor_placement_solved(), editor_placement_refused()))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the occupied socket says so")
        .on_enter(|world: &mut World| {
            let refusal = placement_refusal(world);
            let callout = subtree_text(world, "Placement Callout");
            info!("editor: the editor refuses with {refusal:?}, the callout reads {callout:?}");
            assert!(
                refusal.is_some_and(|reason| reason.contains("occupied")),
                "an occupied socket must be refused; the editor decided {refusal:?}"
            );
            // BESIDE THE GHOST, and naming the key that clears it: the words
            // are what a builder acts on, and a verdict alone is a compiler
            // error.
            assert!(
                callout
                    .iter()
                    .any(|line| line.contains("occupied") && line.contains('F')),
                "the callout must name the fault and its key; it read {callout:?}"
            );
            stamp_sections(world);
        })
        .add()
        .press_and_release("editor: press on the occupied socket", sections_grew_by(0))
        .step("editor: the refused click built nothing")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().sections;
            let now = count_sections(world);
            assert_eq!(
                now, before,
                "a refused placement must not build; the same gesture built a \
                 section a moment ago"
            );
            nova_probe::probe_marker(
                world,
                "outcome: an occupied socket refuses",
                serde_json::json!({}),
            );
            info!("editor: the occupied socket refused the click ({now} sections)");

            // The graph as BUILT, the PDC mount included - what the flown
            // ship has to re-derive from the flat saved poses.
            let mates = mate_graph(world, None).unwrap_or_else(|error| {
                panic!("the assembled ship must derive one connected graph: {error}")
            });
            info!("editor: the finished ship derives {mates} mates over {now} sections");
            world.resource_mut::<EditorWalk>().mates = mates;
        })
        .add()
        // The refusal figure: the ghost at the pose a click would build, its
        // bounds box red, and the reason under the ship.
        .step("editor: shoot the refused placement")
        .on_enter(|world: &mut World| {
            let (_, off_centre) = aim_at_a_visible_face(world).expect("the same face is up");
            move_cursor(off_centre)(world);
            shoot(world, "editor-placement-refused.png");
        })
        .until(shot_written("editor-placement-refused.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The OTHER refusal a builder can meet, and the one the GENERATOR is
        // held to as well: a part that fires may not be built where it cannot.
        //
        // The seeded hull has nowhere like that yet, so the run builds one: a
        // two-cell tower beside the upper arm. Anything mounted on the arm's
        // roof then fires up a lane that tower stands beside, and the skin would
        // close the lane over rather than leave the tower's own face bare.
        .arm_from_the_gallery("editor: arm the hull again", HULL_PROTOTYPE)
        .place_on_the_face(
            "editor: raise a tower, first course",
            Vec3::new(0.0, 1.5, 1.0),
        )
        .place_on_the_face(
            "editor: raise a tower, second course",
            Vec3::new(0.0, 2.5, 1.0),
        )
        // A DRIVE rather than a hull slab: this is the case where the ghost is
        // the thing that fires, and a slab has no lane of its own. The REFUSAL
        // is the wait, so an editor that solved this pose fails here, naming
        // the beat, instead of downstream on a count that came out right for
        // the wrong reason.
        .arm_from_the_gallery("editor: arm a drive", "basic_thruster_section")
        .step("editor: aim the drive up the tower's lane")
        .on_enter(|world: &mut World| {
            let at = aim_at_world(world, Vec3::new(0.0, 1.5, 2.0))
                .expect("the roof of the upper arm is on screen");
            move_cursor(at)(world);
            stamp_sections(world);
        })
        .until(editor_placement_refused())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: a drive that cannot fire says so")
        .on_enter(|world: &mut World| {
            let refusal = placement_refusal(world);
            let callout = subtree_text(world, "Placement Callout");
            info!("editor: the editor refuses with {refusal:?}, the callout reads {callout:?}");
            assert!(
                refusal.is_some_and(|reason| reason.contains("block")),
                "a drive whose plume would fire into the ship's own plating must \
                 be refused; the editor decided {refusal:?}"
            );
            assert!(
                callout.iter().any(|line| line.contains("block")),
                "and the builder must be told so in words; it read {callout:?}"
            );
            shoot(world, "editor-placement-blocked-exit.png");
        })
        .until(shot_written("editor-placement-blocked-exit.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .press_and_release("editor: press on the blocked lane", sections_grew_by(0))
        .step("editor: the blocked exit built nothing either")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().sections;
            let now = count_sections(world);
            assert_eq!(
                now, before,
                "a drive with nowhere to fire must not be built; the editor and \
                 the generator hold one rule between them"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a blocked drive lane refuses",
                serde_json::json!({}),
            );
            info!("editor: the blocked exit refused the click ({now} sections)");

            // The tower moved the graph on, so the figure the flown ship is
            // compared against is re-taken here.
            let mates = mate_graph(world, None).unwrap_or_else(|error| {
                panic!("the assembled ship must derive one connected graph: {error}")
            });
            world.resource_mut::<EditorWalk>().mates = mates;
            info!("editor: the finished ship derives {mates} mates over {now} sections");
            stamp_sections(world);
        })
        .add()
        // Add means "one more node HERE", and here changes. Inside a ship the
        // world palette is greyed - it used to answer Asteroid by putting a
        // rock in a world the stage had stopped showing - and the same menu
        // offers the ship's parts instead.
        .click_a_widget("editor: drop the Add menu inside a ship", MENU_ADD)
        .step("editor: Add offers parts, not rocks, inside a ship")
        .on_enter(|world: &mut World| {
            assert!(
                widget_is_disabled(world, "Add Asteroid"),
                "the world palette must be greyed inside a ship"
            );
            assert!(
                widget_is_disabled(world, "Add Ship Button"),
                "Add > New Ship inside a ship used to move the builder to a \
                 different ship without saying so"
            );
            assert!(
                !widget_is_disabled(world, "Add Structure Item"),
                "and the parts palette must be live in the same menu"
            );
            nova_probe::probe_marker(
                world,
                "outcome: Add obeys the context",
                serde_json::json!({}),
            );
            info!("editor: inside a ship, Add offers parts and greys the world");
        })
        .add()
        // The menu is already down from the beat above, so this clicks the ROW
        // rather than reopening the menu - a second press on the bar button
        // would close it.
        .click_a_widget("editor: browse Add > Structure", "Add Structure Item")
        .step("editor: the gallery opened narrowed to one kind")
        .on_enter(|world: &mut World| {
            // The grid shows one PAGE, so a count says nothing about the
            // filter. WHAT is listed does: every tile up must be a hull.
            let hulls: Vec<String> = world
                .resource::<GameSections>()
                .iter()
                .filter(|section| matches!(section.kind, SectionKind::Hull(_)))
                .map(|section| section.base.name.clone())
                .collect();
            let listed = visible_names_with_prefix(world, GALLERY_TILE);
            assert!(!listed.is_empty(), "a named kind must still list something");
            let strays: Vec<&String> = listed.iter().filter(|name| !hulls.contains(name)).collect();
            assert!(
                strays.is_empty(),
                "a row that names a kind must open the gallery ON that kind; \
                 the grid also listed {strays:?}"
            );
            nova_probe::probe_marker(
                world,
                "outcome: an Add row opens the gallery on its kind",
                serde_json::json!({}),
            );
            info!(
                "editor: Add > Structure opened the gallery on {} hull tiles",
                listed.len()
            );
        })
        .until(and(editor_gallery_open(), some_gallery_tiles()))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Widen it again before leaving: the category is what the NEXT browse
        // opens on, by design, and the beats after this one want the catalog.
        .click_a_widget(
            "editor: widen the gallery back to All",
            "Gallery Category All",
        )
        .step("editor: close the gallery")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_gallery_closed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        // Two ships in one session, which is the whole reason the editor keeps
        // a document rather than a build state. The run leaves the ship it
        // built, adds a second one beside it, walks back into the first, and
        // finds the ids it left - entity-independent, as a saved file needs.
        .step("editor: stamp the ids before leaving the ship")
        .on_enter(|world: &mut World| {
            let ids = edited_section_ids(world);
            assert!(
                ids.len() > 1,
                "the run must have a built ship to leave, not a bare seed"
            );
            info!("editor: leaving a ship of {} sections: {ids:?}", ids.len());
            world.resource_mut::<EditorWalk>().ids = ids;
        })
        .add()
        // Add > New Ship is a scenario-context action, so the run steps out
        // through the tree's root row first - the same door a builder uses.
        .click_a_widget("editor: leave to add a second ship", "Scene Row scenario")
        .step("editor: back at the scenario node to add a ship")
        .until(at_the_scenario_node())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click_a_menu_item(
            "editor: click Add > New Ship again",
            MENU_ADD,
            "Add Ship Button",
        )
        .step("editor: the second ship is up and entered")
        .until(inside_a_ship_of(0))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: a second ship did not replace the first")
        .on_enter(|world: &mut World| {
            let probe = world.resource::<EditorProbe>();
            let second = probe.inside.clone().expect("the new ship was entered");
            let first = world.resource::<EditorWalk>().ids.clone();
            assert!(
                probe.ship.is_empty(),
                "a new ship starts blank - its first part is the builder's decision"
            );
            assert!(
                !probe.can_play,
                "Play is refused inside a ship - it compiles the whole document"
            );
            assert_eq!(
                probe.visible_ships,
                vec!["ship_2".to_string()],
                "inside a ship the stage shows only that ship"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a second ship stands beside the first",
                serde_json::json!({}),
            );
            info!(
                "editor: inside '{second}', and the first ship's {} sections are elsewhere",
                first.len()
            );
        })
        .add()
        // The second ship is BUILT ON, in its own space. Founding drops a
        // controller at ship_2's origin (x=24 in the WORLD), and the next
        // placement must mate the socket under the pointer - the world-space
        // hit against ship-local sockets used to make an offset ship accept
        // exactly one link point, wherever the pointer was.
        .arm_from_the_gallery("editor: arm the second controller", CONTROLLER_PROTOTYPE)
        .step("editor: point at empty space beside the second ship")
        .on_enter(|world: &mut World| {
            move_cursor(EMPTY_SPACE)(world);
            stamp_sections(world);
        })
        .add()
        .press_and_release("editor: found the second ship", sections_grew_by(1))
        .arm_from_the_gallery("editor: arm a hull for the second ship", HULL_PROTOTYPE)
        .place_on_the_face(
            "editor: roof a hull onto the second ship",
            Vec3::new(24.0, 0.49, 0.0),
        )
        .step("editor: the second ship built in its own space")
        .on_enter(|world: &mut World| {
            let probe = world.resource::<EditorProbe>();
            assert_eq!(probe.inside.as_deref(), Some("ship_2"));
            let roof = probe
                .ship
                .iter()
                .find(|section| section.prototype == HULL_PROTOTYPE)
                .expect("the roof landed on the second ship")
                .clone();
            assert_eq!(
                roof.position,
                Vec3::Y,
                "the mate solves in the ship's OWN space, on the socket the \
                 pointer aimed at"
            );
            nova_probe::probe_marker(
                world,
                "outcome: an offset ship builds in its own space",
                serde_json::json!({}),
            );
            info!("editor: ship_2 took a roof at {:?}", roof.position);
        })
        .add()
        // Out through the tree's root row.
        .click_a_widget("editor: leave the second ship", "Scene Row scenario")
        .step("editor: the scenario node holds both ships")
        .until(at_the_scenario_node())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: both ships are listed and Play is live again")
        .on_enter(|world: &mut World| {
            let probe = world.resource::<EditorProbe>();
            for ship in ["ship_1", "ship_2"] {
                assert!(
                    probe.context_nodes.iter().any(|node| node == ship),
                    "the document holds the ship that was built and the one \
                     beside it: {:?}",
                    probe.context_nodes
                );
            }
            // And the world they stand in: the sandbox's objects are document
            // nodes now, not constants baked into the hand-off.
            assert!(
                probe.context_nodes.iter().any(|node| node == "planetoid"),
                "the scenario node lists the world too: {:?}",
                probe.context_nodes
            );
            assert!(
                probe.ship.is_empty(),
                "outside a ship there is no edited ship to report"
            );
            assert!(probe.can_play, "Play is the scenario node's gesture");
            // The two the run built, standing among the range's own hulls -
            // the hulks and pickets are ship nodes of this document too.
            for ship in ["ship_1", "ship_2", "picket_warden"] {
                assert!(
                    probe.visible_ships.iter().any(|node| node == ship),
                    "the scenario node puts every ship back on the stage: {:?}",
                    probe.visible_ships
                );
            }
            let listed = probe.context_nodes.clone();
            nova_probe::probe_marker(
                world,
                "outcome: the scenario node lists both ships",
                serde_json::json!({}),
            );
            info!("editor: back at the scenario node, listing {listed:?}");
        })
        .add()
        // The world is a document too, not just the ships in it. The Add menu
        // places a rock in front of the camera, the tree lists it beside the
        // ships, and Delete takes it back off.
        .click_a_menu_item("editor: place an asteroid", MENU_ADD, "Add Asteroid")
        .step("editor: the placed rock is a marked node in the tree")
        .until(an_object_was_placed("asteroid"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the rock stands out in front of the camera")
        .on_enter(|world: &mut World| {
            let probe = world.resource::<EditorProbe>();
            let placed = probe
                .selected_node
                .clone()
                .expect("placing an object marks it");
            let at = probe
                .node_positions
                .iter()
                .find(|(id, _)| *id == placed)
                .map(|(_, at)| *at)
                .unwrap_or_else(|| panic!("the placed object stands somewhere: {placed}"));
            assert!(
                at.length() > 1.0,
                "a placed object lands where the camera was looking, not on \
                 the origin ({at:?})"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the palette places a world object",
                serde_json::json!({}),
            );
            info!("editor: placed {placed} at {at:?}");
        })
        .add()
        // And the Inspector edits it. The rows are read off the rock's OWN
        // config by reflection - no editor code names `radius` - so typing into
        // one is the proof that a placed object is authorable and not just
        // stock. The caret is put at the end and the old value cleared, because
        // the click lands the caret wherever the pixels fell.
        .click_a_widget(
            "editor: reach for the rock's radius",
            "Inspector Field Radius",
        )
        .step("editor: the radius field has the caret")
        .until(editor_field_focused())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: retype the radius")
        .on_enter(|world: &mut World| {
            press_edit_key(Key::End)(world);
            for _ in 0..8 {
                press_edit_key(Key::Backspace)(world);
            }
            type_text("18")(world);
            press_edit_key(Key::Enter)(world);
        })
        .until(editor_inspector_reads("Radius", "18"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the typed radius is the rock's own config now")
        .on_enter(|world: &mut World| {
            let probe = world.resource::<EditorProbe>();
            assert!(
                !probe.inspector_focused,
                "committing the field gives the caret - and the editor's \
                 single-letter keys - back"
            );
            let rows = probe.inspector.clone();
            // The rest of the config is still listed beside it: the panel walks
            // the WHOLE config, not the one field that was typed - `Mass` is
            // there and empty, which is an `Option` the rock never set.
            for label in ["Name", "Position", "Radius", "Mass"] {
                assert!(
                    rows.iter().any(|(row, _)| row == label),
                    "the rock's panel lists {label}: {rows:?}"
                );
            }
            nova_probe::probe_marker(
                world,
                "outcome: the inspector writes a placed object's config",
                serde_json::json!({}),
            );
            info!("editor: the rock's inspector reads {rows:?}");
        })
        .add()
        // A radius has a floor, and the box is where it is enforced. A negative
        // one used to be taken here and found out at spawn time, with the range
        // already flying.
        .click_a_widget(
            "editor: reach for the radius again",
            "Inspector Field Radius",
        )
        .step("editor: the radius field has the caret again")
        .until(editor_field_focused())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: type a radius that is not one")
        .on_enter(|world: &mut World| {
            press_edit_key(Key::End)(world);
            for _ in 0..8 {
                press_edit_key(Key::Backspace)(world);
            }
            type_text("-5")(world);
            press_edit_key(Key::Enter)(world);
        })
        .until(the_field_refuses("Inspector Field Radius", "min 0"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the refusal is on the box, and the rock is untouched")
        .on_enter(|world: &mut World| {
            assert_eq!(
                inspector_reading(world, "Radius"),
                "18",
                "a refused number leaves the document as it was"
            );
            assert_eq!(
                field_text(world, "Inspector Field Radius"),
                "-5",
                "the refused number stays in the box to be corrected"
            );
            assert_eq!(
                text_of(world, "Inspector Unit Radius"),
                "min 0",
                "the reason stands where the unit does"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a number under its floor is refused where it is typed",
                serde_json::json!({}),
            );
            info!("editor: the radius box refused -5, kept it, and said why");
        })
        .add()
        // The pose is three boxes, and each one writes ONE number: this types
        // into the middle box and the beat after says the other two did not
        // move. A single `x, y, z` field could not make that claim.
        .step("editor: stamp the rock's position")
        .on_enter(|world: &mut World| {
            let reading = inspector_reading(world, "Position");
            world.resource_mut::<EditorWalk>().position = reading;
        })
        .add()
        .click_a_widget(
            "editor: reach for the rock's height",
            "Inspector Field Position Y",
        )
        .step("editor: the height box has the caret")
        .until(editor_field_focused())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: retype the height")
        .on_enter(|world: &mut World| {
            press_edit_key(Key::End)(world);
            for _ in 0..12 {
                press_edit_key(Key::Backspace)(world);
            }
            type_text("12")(world);
            press_edit_key(Key::Enter)(world);
        })
        .until(the_rocks_height_reads("12"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: only the height moved")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().position.clone();
            let now = inspector_reading(world, "Position");
            let parts: Vec<&str> = now.split(", ").collect();
            let was: Vec<&str> = before.split(", ").collect();
            assert_eq!(parts.len(), 3, "a position reads as three numbers: {now}");
            assert_eq!(parts[0], was[0], "X stood still: {before} -> {now}");
            assert_eq!(parts[2], was[2], "Z stood still: {before} -> {now}");
            nova_probe::probe_marker(
                world,
                "outcome: one axis box writes one number",
                serde_json::json!({}),
            );
            info!("editor: the rock's position went {before} -> {now}");
        })
        .add()
        .click_a_menu_item("editor: delete the placed rock", MENU_EDIT, "Delete Item")
        .step("editor: the rock is gone and nothing is marked")
        .until(no_object_named("asteroid"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: deleting the rock left the rest of the world standing")
        .on_enter(|world: &mut World| {
            let probe = world.resource::<EditorProbe>();
            assert!(
                probe.selected_node.is_none(),
                "the deleted node cannot stay marked: {:?}",
                probe.selected_node
            );
            assert!(
                probe.context_nodes.iter().any(|node| node == "planetoid"),
                "Delete takes the selection, not the document: {:?}",
                probe.context_nodes
            );
            nova_probe::probe_marker(
                world,
                "outcome: Delete removes a world object",
                serde_json::json!({}),
            );
            info!("editor: deleted the placed asteroid");
        })
        .add()
        // A COLOUR is the one value in a config nobody can author by reading
        // it, so it gets a window of its own. The beacon is the shipped object
        // that has one.
        .click_a_menu_item("editor: place a beacon", MENU_ADD, "Add Beacon")
        .step("editor: the beacon is the marked node")
        .until(an_object_was_placed("beacon"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: stamp the beacon's colour")
        .on_enter(|world: &mut World| {
            let reading = inspector_reading(world, "Color");
            let placed = world
                .resource::<EditorProbe>()
                .selected_node
                .clone()
                .expect("placing an object marks it");
            let mut walk = world.resource_mut::<EditorWalk>();
            walk.colour = reading;
            walk.placed = placed;
        })
        .add()
        .click_a_widget("editor: press the colour swatch", "Inspector Swatch Color")
        .step("editor: the picker floats over the stage")
        .until(ui_node_present("Colour Window"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: stamp where the picker stands")
        .on_enter(|world: &mut World| {
            let at = window_position(world).expect("the picker is on screen");
            world.resource_mut::<EditorWalk>().window_at = at;
        })
        .add()
        // Dragged by its BAR, which is what makes it a window rather than a
        // panel that happens to float.
        .step("editor: aim at the picker's bar")
        .on_enter(hover_named("Colour Window Bar"))
        .until(the_pointer_is_on("Colour Window Bar"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: grab the bar")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: pull the window across the stage")
        .on_enter(|world: &mut World| {
            let at = ui_node_centre(world, "Colour Window Bar").expect("the bar is on screen");
            move_cursor(at - Vec2::new(120.0, -40.0))(world);
        })
        .until(the_picker_moved())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: let the window go")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // And it EDITS. A click on the middle of a channel's track snaps that
        // channel to half, which the row behind the window reads back as new
        // hex - the window and the row are one edit made two ways.
        .click_a_widget(
            "editor: pull the green channel down",
            "Colour Window Slider G",
        )
        .step("editor: the beacon's colour changed under the picker")
        .until(the_colour_row_changed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the picker wrote the beacon's own config")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().colour.clone();
            let now = inspector_reading(world, "Color");
            let moved = world.resource::<EditorWalk>().window_at;
            nova_probe::probe_marker(
                world,
                "outcome: a floating picker edits the colour of a scenario object",
                serde_json::json!({}),
            );
            info!("editor: the beacon's colour went {before} -> {now}, picker at {moved:?}");
        })
        .add()
        .click_a_widget("editor: close the picker", "Colour Window Close")
        .step("editor: the picker is gone")
        .until(nova_autopilot::predicate::not(ui_node_present(
            "Colour Window",
        )))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click_a_menu_item("editor: delete the placed beacon", MENU_EDIT, "Delete Item")
        .step("editor: the beacon is off the stage")
        .until(the_placed_node_is_gone())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // A world click out here SELECTS: the viewport and the tree answer a
        // click the same way, and the tree is the door.
        .step("editor: aim at the first ship")
        .on_enter(|world: &mut World| {
            let at = aim_at_the_first_ship(world)
                .expect("the first ship is on screen at the scenario node");
            move_cursor(at)(world);
        })
        .until(the_pointer_is_on_the_ship())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The pointer carries the node across to the rail: resting on a hull
        // out here lights the row that names it, so a tree of minted ids can be
        // read against the stage without clicking anything.
        .step("editor: the stage hover reaches the tree")
        .until(the_hovered_node_is("ship_1"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: exactly the pointed-at ship's row is lit")
        .on_enter(|world: &mut World| {
            assert!(
                world.resource::<EditorProbe>().selected_node.is_none(),
                "the hover beats run on an unselected tree, or the tint proves nothing"
            );
            let lit = row_tint(world, "Scene Row ship_1");
            let dark = row_tint(world, "Scene Row ship_2");
            assert!(lit > 0.0, "the hovered ship's row is painted, got {lit}");
            assert!(dark <= 0.0, "and its neighbour is not, got {dark}");
            nova_probe::probe_marker(
                world,
                "outcome: pointing at a ship lights its row in the tree",
                serde_json::json!({}),
            );
            info!("editor: ship_1's row is lit from the stage ({lit} against {dark})");
        })
        .add()
        .press_and_release(
            "editor: click the first ship's body",
            the_first_ship_is_selected(),
        )
        .step("editor: the world click selected without entering")
        .on_enter(|world: &mut World| {
            let probe = world.resource::<EditorProbe>();
            assert_eq!(probe.selected_node.as_deref(), Some("ship_1"));
            assert!(
                probe.inside.is_none(),
                "a world click at the scenario node selects; entering is the tree's gesture"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a world click selects the ship",
                serde_json::json!({}),
            );
            info!("editor: clicked ship_1 in the world - selected, not entered");
        })
        .add()
        // Grab the ship's BODY and slide it on the ground plane - the gesture
        // that predates the handles. The pointer is still over ship_1 from the
        // select, and the rig leaves the middle of a node hollow so the body
        // under it is still what a press there lands on.
        .step("editor: grab the first ship")
        .on_enter(|world: &mut World| {
            let at = first_ship_position(world);
            world.resource_mut::<EditorWalk>().first_ship_at = at;
            press_mouse(MouseButton::Left)(world);
        })
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: drag it across the plane")
        .on_enter(|world: &mut World| {
            let at = aim_at_the_first_ship(world).expect("the grabbed ship is on screen");
            // Inside the hollow the whole way. The rig is scaled to a fixed
            // share of the VIEW, so its rings sit at a constant screen radius
            // however far the camera stands off - and a drag long enough to
            // cross one hands the gesture to the handle it crossed.
            move_cursor(at + Vec2::new(DRAG_SPAN, 0.0))(world);
        })
        .until(the_first_ship_moved())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: release the drag")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the ship moved on the plane and stayed selected")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().first_ship_at;
            let now = first_ship_position(world);
            let marked = world.resource::<EditorProbe>().selected_node.clone();
            assert!(
                now.distance(before) > 0.5,
                "the drag must move the ship ({before:?} -> {now:?})"
            );
            assert!(
                (now.y - before.y).abs() < 1e-3,
                "a ground-plane drag never changes altitude"
            );
            assert_eq!(
                marked.as_deref(),
                Some("ship_1"),
                "grabbing a ship is also pointing at it"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a drag slides the ship on the ground plane",
                serde_json::json!({}),
            );
            info!("editor: dragged ship_1 from {before:?} to {now:?}");
        })
        .add()
        // The handles: the ground-plane drag has no way to say "up", and the
        // gizmo's Y arrow is the whole of that answer. It is on the stage
        // because ship_1 is selected - which is what the click above did.
        .step("editor: the handles came up on the selected ship")
        .until(editor_gizmo_on("ship_1"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: aim at the lift handle")
        .on_enter(|world: &mut World| {
            let at = aim_at_the_named(world, "Gizmo Tip Y")
                .expect("the Y arrowhead is on screen beside the selected ship");
            world.resource_mut::<EditorWalk>().first_ship_at = first_ship_position(world);
            move_cursor(at)(world);
        })
        .until(the_pointer_is_on_a_handle())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: grab the lift handle")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: pull the handle up the screen")
        .on_enter(|world: &mut World| {
            let at =
                aim_at_the_named(world, "Gizmo Tip Y").expect("the grabbed handle is on screen");
            // Up the SCREEN is up the +Y axis from this camera, which looks
            // slightly down at the stage.
            move_cursor(at - Vec2::new(0.0, 110.0))(world);
        })
        .until(the_first_ship_rose())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: let the handle go")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the handle lifted the ship off the plane")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().first_ship_at;
            let now = first_ship_position(world);
            assert!(
                now.y - before.y > 0.5,
                "the Y handle must lift the ship ({before:?} -> {now:?})"
            );
            assert!(
                (now.x - before.x).abs() < 0.5 && (now.z - before.z).abs() < 0.5,
                "and lift it ALONE - one handle is one axis ({before:?} -> {now:?})"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the Y handle moves the ship off the ground plane",
                serde_json::json!({}),
            );
            info!("editor: lifted ship_1 from {before:?} to {now:?}");
        })
        .add()
        // Framing: the ship has been dragged and lifted out from under the
        // camera, so putting the camera back on it is a gesture with something
        // to prove.
        .step("editor: stamp where the camera stands")
        .on_enter(|world: &mut World| {
            let at = camera_position(world).expect("the editor camera is up");
            world.resource_mut::<EditorWalk>().camera_at = at;
        })
        .add()
        .click_a_menu_item(
            "editor: frame the selection",
            MENU_VIEW,
            "Frame Selection Item",
        )
        .step("editor: the camera went to the ship")
        .until(the_camera_frames_the_first_ship())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: View > Frame Selection put the camera on the marked node")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().camera_at;
            let now = camera_position(world).expect("the editor camera is up");
            assert!(
                now.distance(before) > 0.5,
                "the menu row must move the camera ({before:?} -> {now:?})"
            );
            assert_eq!(
                world.resource::<EditorProbe>().selected_node.as_deref(),
                Some("ship_1"),
                "framing looks at the selection; it does not change it"
            );
            nova_probe::probe_marker(
                world,
                "outcome: Frame Selection puts the camera on the marked node",
                serde_json::json!({}),
            );
            info!("editor: framed ship_1, camera {before:?} -> {now:?}");
        })
        .add()
        // The axis views, on the same selection: a mate is checked by looking
        // down an axis, so the row has to actually put the camera on one.
        .click_a_menu_item("editor: look straight down", MENU_VIEW, "Top View Item")
        .step("editor: the camera stands on the top axis")
        .until(the_camera_looks_down_on_the_first_ship())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click_a_menu_item("editor: back to the stage view", MENU_VIEW, "Iso View Item")
        .step("editor: the camera came off the top axis")
        .until(the_camera_frames_the_first_ship())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The stage figure. Taken HERE because this is the one beat where the
        // grid, the handles and a marked ship are all on screen at once, which
        // is the picture every change to the stage has to be judged against.
        .step("editor: shoot the stage")
        .on_enter(|world: &mut World| {
            assert_eq!(
                world.resource::<EditorProbe>().selected_node.as_deref(),
                Some("ship_1"),
                "the stage figure wants the handles up, so it wants a selection"
            );
            shoot(world, "editor-stage.png");
        })
        .until(shot_written("editor-stage.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // And back in - two clicks on the ship's tree row, because entering is
        // the gesture that hides the rest of the document.
        .double_click_a_widget("editor: enter through the tree", "Scene Row ship_1")
        .step("editor: back inside the first ship")
        .until(back_inside_the_stamped_ship())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Entering ISOLATES: the tree holds the way out, this ship and its
        // parts, and not one row of the range standing around it - a click on
        // a beacon from inside a ship means nothing, so there is nothing to
        // click.
        .step("editor: the tree isolates the entered ship")
        .on_enter(|world: &mut World| {
            let rows = scene_row_names(world);
            assert!(
                rows.iter().any(|row| row == "Scene Row ship_1"),
                "the entered ship is in the tree: {rows:?}"
            );
            assert!(
                rows.iter().any(|row| row == "Scene Row scenario"),
                "and so is the way back out: {rows:?}"
            );
            assert!(
                !rows.iter().any(|row| {
                    row.starts_with("Scene Row beacon")
                        || row.starts_with("Scene Row hulk")
                        || row == "Scene Row ship_2"
                }),
                "and nothing else: {rows:?}"
            );
            nova_probe::probe_marker(
                world,
                "outcome: entering a node isolates it in the tree",
                serde_json::json!({}),
            );
            info!("editor: inside ship_1 the tree lists {rows:?}");
        })
        .add()
        // A 150px row clips its id, so resting on one reveals the whole thing
        // and the word its icon stands for.
        .step("editor: rest on the turret's row")
        .on_enter(hover_named("Scene Row pdc_kinetic_turret_section_7"))
        .until(ui_node_present("Scene Row Hint"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the hint says what the row could not")
        .on_enter(|world: &mut World| {
            let hint = named_text(world, "Scene Row Hint Id");
            assert_eq!(
                hint.as_deref(),
                Some("pdc_kinetic_turret_section_7"),
                "the hint reveals the id the row clipped"
            );
            let kind = named_text(world, "Scene Row Hint Kind");
            assert_eq!(kind.as_deref(), Some("TURRET"), "and what its icon meant");
            // The other half of the cross-highlight: the row hands the node to
            // the stage, which is what `draw_node_marks` boxes.
            assert_eq!(
                world.resource::<EditorProbe>().hovered_node.as_deref(),
                Some("pdc_kinetic_turret_section_7"),
                "resting on a row marks that part on the stage"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a tree row reveals its kind and its whole id on hover",
                serde_json::json!({}),
            );
            info!("editor: the row hint reads {kind:?} {hint:?}");
        })
        .add()
        .step("editor: leave the row")
        .on_enter(hover_named("Editor Inspector"))
        .until(nova_autopilot::predicate::not(ui_node_present(
            "Scene Row Hint",
        )))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The turret is the deepest config in the game - its fire rate lives on
        // a muzzle, on a joint, inside `root.children` - so it is what proves
        // both that the walk reaches into a list and that the panel can be
        // scrolled down to what the walk found there.
        .click_a_widget(
            "editor: inspect the turret",
            "Scene Row pdc_kinetic_turret_section_7",
        )
        // The Key row IS the rebind. It used to be dead text beside a verb in
        // the top bar, which left a builder reading a binding on one surface
        // and arming it on another.
        .click_a_widget("editor: press the turret's Key row", "Inspector Key")
        .step("editor: the turret waits for its key")
        .until(a_section_awaits_its_key())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: give it J")
        .on_enter(press_key(KeyCode::KeyJ))
        .until(the_key_row_reads("J"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the binding landed on the section it was read off")
        .on_enter(|world: &mut World| {
            release_key(KeyCode::KeyJ)(world);
            nova_probe::probe_marker(
                world,
                "outcome: the Key row arms the rebind and takes the key",
                serde_json::json!({}),
            );
            info!("editor: the turret's Key row armed the capture and took J");
        })
        .add()
        // The turret's FIRST screen is what it does, not what it declares: the
        // joint tree, the render mesh transforms and the per-joint offsets are
        // not questions a scenario asks.
        .step("editor: the turret opens on what it does")
        .on_enter(|world: &mut World| {
            let labels = inspector_labels(world);
            assert!(
                labels.iter().any(|label| label == "Fire Rate"),
                "the curated turret keeps the number it is authored by: {labels:?}"
            );
            assert!(
                !labels.iter().any(|label| label == "Offset"),
                "and drops the joint plumbing: {labels:?}"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the inspector opens on the fields the kind is authored through",
                serde_json::json!({}),
            );
            info!("editor: the turret's first screen is {} rows", labels.len());
        })
        .add()
        // And the way past it. Everything the walk can reach is one View menu
        // item away, which is also what makes the panel deep enough to scroll.
        .click_a_menu_item("editor: ask for every field", MENU_VIEW, "All Fields Item")
        .step("editor: the whole config is back")
        .until(the_inspector_has_a_row("Offset"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: aim at the inspector")
        .on_enter(hover_named("Inspector List"))
        .until(the_inspector_overflows())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: roll the wheel down the panel")
        .on_enter(scroll_lines(-12.0))
        .until(the_inspector_scrolled())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the turret's joint tree can be reached to the bottom")
        .on_enter(|world: &mut World| {
            info!(
                "editor: the inspector scrolled {:.0}px down the turret's joint tree",
                inspector_scroll(world)
            );
        })
        .add()
        // The tree's own view, and the way an event will be wired: a filter
        // names its node by id, so the rail has to be able to say the ids.
        .click_a_menu_item("editor: ask for the ids", MENU_VIEW, "Ids Item")
        .step("editor: the tree reads as ids")
        .until(the_tree_reads_as_ids())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: an event could be wired off the rail")
        .on_enter(|world: &mut World| {
            let labels = scene_row_labels(world);
            nova_probe::probe_marker(
                world,
                "outcome: the tree can be read as the ids an event names",
                serde_json::json!({}),
            );
            info!("editor: the rail reads {labels:?}");
        })
        .add()
        .click_a_menu_item("editor: back to the names", MENU_VIEW, "Ids Item")
        .step("editor: the ids survived the trip out and back")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().ids.clone();
            let now = edited_section_ids(world);
            assert_eq!(
                now, before,
                "a section's id is the document's key and owes nothing to the \
                 entity it landed on, so re-entry finds the ids it left"
            );
            nova_probe::probe_marker(
                world,
                "outcome: section ids survive exit and re-entry",
                serde_json::json!({}),
            );
            info!("editor: re-entered the first ship on the same ids: {now:?}");
        })
        .add()
        // Play only from the scenario node, so the run steps back out to press
        // it. The button is greyed inside a ship and the observer refuses too.
        .click_a_widget("editor: leave for the hand-off", "Scene Row scenario")
        .step("editor: Play is reachable")
        .until(at_the_scenario_node())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The root is a node like any other, and the panel on it used to be a
        // titled empty box - which reads as the panel breaking every time you
        // leave a ship.
        .step("editor: the root says what the document holds")
        .on_enter(|world: &mut World| {
            let ships = inspector_reading(world, "Ships");
            let objects = inspector_reading(world, "Objects");
            let flown = inspector_reading(world, "Player Ship");
            // The two the run built, standing among the stock range's own
            // hulls - the hulks and pickets are ship nodes of this document
            // too, so the count is not the walk's alone.
            assert!(
                ships.parse::<usize>().is_ok_and(|ships| ships >= 2),
                "the run built two designs and the root counts them: {ships}"
            );
            assert_ne!(objects, "0", "and stands them on a range with things on it");
            assert_ne!(
                flown, "none",
                "and one of the two is the ship Play would hand over"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the scenario node reports the document",
                serde_json::json!({}),
            );
            info!("editor: the root reads {ships} ship(s), {objects} object(s), flown by {flown}");
        })
        .add()
        // A hull the RANGE came with is a ship of this document, and the tree
        // is the door into it: two clicks on a picket's row go inside it,
        // exactly as they do on a ship the run built. It used to be an opaque
        // object - a double click framed it, and the panel read out its whole
        // flattened spawn config.
        .double_click_a_widget("editor: enter a seeded picket", "Scene Row picket_warden")
        .step("editor: inside the hull the range came with")
        .until(inside_the_node("picket_warden"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: a seeded hull opens, and opens as a ship")
        .on_enter(|world: &mut World| {
            let sections = world.resource::<EditorProbe>().context_nodes.clone();
            assert!(
                !sections.is_empty(),
                "a picket's sections are nodes of its own: {sections:?}"
            );
            let driver = inspector_reading(world, "Driver");
            let side = inspector_reading(world, "Allegiance");
            assert_eq!(driver, "AI", "a picket has a live pilot");
            assert_eq!(
                side, "Neutral",
                "and the side that makes it dormant is on the panel that names it"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a seeded hull is entered and inspected as a ship",
                serde_json::json!({}),
            );
            info!("editor: inside picket_warden - {sections:?}, driven by {driver}, {side}");
        })
        .add()
        .click_a_widget("editor: back out to the range", "Scene Row scenario")
        .step("editor: back at the scenario node")
        .until(at_the_scenario_node())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The round trip. The document the run built - two ships and the
        // range around them - goes to disk as a mod bundle, gets thrown away,
        // and comes back. Play then flies what came BACK, so every assert
        // after this point is an assert about the reloaded document.
        .step("editor: stamp the document before it is saved")
        .on_enter(|world: &mut World| {
            let probe = world.resource::<EditorProbe>();
            let (nodes, poses) = (probe.context_nodes.clone(), probe.node_positions.clone());
            assert!(
                nodes.iter().any(|node| node == "ship_2"),
                "the run built a second ship, so the document to save holds one"
            );
            let walk = &mut world.resource_mut::<EditorWalk>();
            walk.document = nodes.clone();
            walk.document_at = poses;
            info!("editor: about to save {} node(s): {nodes:?}", nodes.len());
        })
        .add()
        .click_a_menu_item("editor: save the document", MENU_FILE, "Save Item")
        .step("editor: the save reported itself")
        .until(the_status_reads("saved"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The complaint this answers: a range saved here did not appear in the
        // Scenarios list until the game was quit and started again, because
        // content is merged once at boot. The save asks for the reload itself,
        // so nothing is pressed here - the beat waits for the registry to hold
        // what was just written.
        .step("editor: the saved range is in the game's scenarios")
        .until(the_scenarios_hold(SAVED_RANGE_ID))
        .deadline(RELOAD_DEADLINE_SECS)
        .add()
        .step("editor: a save reaches the game without a restart")
        .on_enter(|world: &mut World| {
            nova_probe::probe_marker(
                world,
                "outcome: a saved range is playable without restarting",
                serde_json::json!({}),
            );
            info!("editor: the reload put '{SAVED_RANGE_ID}' in the scenarios list");
        })
        .add()
        // Thrown away on purpose: a walk cannot restart the process, so New
        // Scenario is what stands in for one. It reseeds the stock range, which
        // has no `ship_2` in it - so a `ship_2` after the Open can only have
        // come off disk.
        // Neither of these two verbs acts on the row any more: a document has
        // no undo, so the row asks and the window's own button is what goes
        // through with it. The first press proves the ASK - the ship is still
        // there while the question is up.
        .click_a_menu_item("editor: start over", MENU_FILE, "New Scenario Item")
        .step("editor: the question is up and nothing is gone yet")
        .on_enter(|world: &mut World| {
            assert!(
                ui_node_rect(world, "Confirm Window").is_some(),
                "a verb with no undo must ask before it runs"
            );
            let nodes = world.resource::<EditorProbe>().context_nodes.clone();
            assert!(
                nodes.iter().any(|node| node == "ship_2"),
                "and the document must still be standing while it asks; the \
                 stage held {nodes:?}"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a destructive verb asks first",
                serde_json::json!({}),
            );
            info!("editor: New Scenario asked before throwing the document away");
        })
        .add()
        .click_a_widget("editor: confirm the discard", "Confirm Discard Button")
        .step("editor: the built document is gone")
        .until(no_object_named("ship_2"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click_a_menu_item("editor: open the saved document", MENU_FILE, "Open Item")
        .click_a_widget("editor: confirm the open", "Confirm Discard Button")
        .step("editor: the saved document came back")
        .until(the_status_reads("opened"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the document came back node for node, pose for pose")
        .on_enter(|world: &mut World| {
            let walk = world.resource::<EditorWalk>();
            let (before, before_at) = (walk.document.clone(), walk.document_at.clone());
            let probe = world.resource::<EditorProbe>();
            assert_eq!(
                probe.context_nodes, before,
                "a saved document is its ids: what was written is what opens, \
                 in the same order, owing nothing to the entities it left on"
            );
            for (id, was) in &before_at {
                let now = probe
                    .node_positions
                    .iter()
                    .find(|(node, _)| node == id)
                    .map(|(_, at)| *at)
                    .unwrap_or_else(|| panic!("the reopened document must still hold {id}"));
                assert!(
                    now.distance(*was) < 1e-3,
                    "{id} stood at {was:?} when it was saved and at {now:?} after \
                     the round trip"
                );
            }
            nova_probe::probe_marker(
                world,
                "outcome: the document survives a save and an open",
                serde_json::json!({}),
            );
            info!(
                "editor: reopened {} node(s) on the same ids and poses",
                before.len()
            );
        })
        .add()
        // An id minted AFTER the load must clear the ones that came off disk.
        // The ordinal is the editor's own counter, and a fresh document starts
        // it at zero - so a rock placed now colliding with a loaded id is the
        // whole failure this beat is here to catch.
        .click_a_menu_item(
            "editor: place a rock after the load",
            MENU_ADD,
            "Add Asteroid",
        )
        .step("editor: the id minted after a load clears the loaded ones")
        .until(an_object_was_placed("asteroid"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: nothing was minted twice")
        .on_enter(|world: &mut World| {
            let nodes = world.resource::<EditorProbe>().context_nodes.clone();
            let mut seen: Vec<&String> = nodes.iter().collect();
            seen.sort();
            let before = seen.len();
            seen.dedup();
            assert_eq!(
                before,
                seen.len(),
                "every node id in a document is unique, loaded or minted: {nodes:?}"
            );
            let placed = nodes
                .iter()
                .find(|node| node.starts_with("asteroid"))
                .cloned()
                .unwrap_or_default();
            nova_probe::probe_marker(
                world,
                "outcome: ids minted after a load do not collide",
                serde_json::json!({}),
            );
            info!("editor: minted {placed} beside the loaded ids");
        })
        .add()
        .click_a_menu_item("editor: take the rock back off", MENU_EDIT, "Delete Item")
        .step("editor: the document is the saved one again")
        .until(no_object_named("asteroid"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Play: the ship the editor assembled becomes the ship that flies, and
        // the runtime derives the SAME graph from the flat saved poses.
        .click_a_widget("editor: press Play", "Play Button")
        .step("editor: the built ship flies")
        .until(player_ship_present())
        .deadline(PLAY_DEADLINE_SECS)
        .add()
        // The hand-off itself, waited on rather than settled for: the flown ship
        // is whole when it carries every section the editor built, and clad when
        // the derived plates are back.
        .step("editor: the flown ship is whole and clad")
        .until(and(the_flown_ship_is_whole(), the_skin_is_on()))
        .deadline(PLAY_DEADLINE_SECS)
        .add()
        // The DOCUMENT flies, not just the player's design: the AI ship built
        // beside it stands on the range too, whole.
        .step("editor: the second ship stands on the range")
        .until(the_second_ship_flies())
        .deadline(PLAY_DEADLINE_SECS)
        .add()
        .step("editor: the flown ship derives the same mate graph")
        .on_enter(|world: &mut World| {
            let built = world.resource::<EditorWalk>().mates;
            let root = player_root(world);
            let flown = mate_graph(world, Some(root))
                .unwrap_or_else(|error| panic!("the spawned ship must derive a graph: {error}"));
            assert_eq!(
                flown, built,
                "the flown ship must re-derive the mate graph the editor built"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the flown ship re-derives the graph",
                serde_json::json!({}),
            );
            info!("editor: the flown ship carries the same {flown} mates");

            // What you see is what you fly: the toggle rides the hand-off, so
            // a ship built clad is spawned clad.
            let plates = count_plates(world);
            assert!(
                plates > 0,
                "the ship was built with its skin on, so the flown one wears it"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the flown ship wears the skin",
                serde_json::json!({}),
            );
            info!("editor: the flown ship came up in {plates} plates");
        })
        .add()
}

/// Name prefix every gallery tile carries; the part's display name follows it.
#[cfg(feature = "debug")]
const GALLERY_TILE: &str = "Gallery Tile ";

/// The sections of ONE ship, as the pure link-point derivation takes them.
///
/// Works on the editor's preview ship and on the flown one alike: both are a
/// root plus section children carrying local poses and authored sockets, which
/// is all the derivation reads.
///
/// `None` takes every section in the world, which is only the same thing in
/// the EDITOR scene: the preview is the only ship there. In flight it is not -
/// the sandbox spawns target hulks and pickets whose sections carry the same
/// marker - and the derivation demands ONE connected structure, so a
/// world-wide sweep out there hands it several ships at once and is rejected.
///
/// Sorted by POSE, not left in query order. A ship is a set of sections and
/// nothing about it says which comes first, so the ECS answers in whatever order
/// its archetypes happen to hold - an order that changes between runs of the
/// same binary. Every caller that then picks ONE section out of this list would
/// inherit that coin flip, and [`aim_at_a_visible_face`] did: the run mounted its
/// turret on a different face from one run to the next, and the beats downstream
/// that name a face by its coordinates found nothing there. Sorting here is what
/// makes the ship the run builds the SAME ship every time.
#[cfg(feature = "debug")]
fn placed_sections(world: &mut World, root: Entity) -> Vec<(Transform, SectionLinkPoints)> {
    let mut sections: Vec<_> = world
        .query_filtered::<(&Transform, &SectionLinkPoints, &ChildOf), With<SectionMarker>>()
        .iter(world)
        .filter(|(_, _, ChildOf(parent))| *parent == root)
        .map(|(transform, points, _)| (*transform, points.clone()))
        .collect();
    sections.sort_by(|(a, _), (b, _)| by_pose(a.translation, b.translation));
    sections
}

/// A total order on positions, for sorts and for breaking ties that would
/// otherwise fall back to query order.
///
/// `total_cmp` rather than `partial_cmp`: a NaN coordinate must still order
/// somewhere rather than silently collapse two sections into "equal".
#[cfg(feature = "debug")]
fn by_pose(a: Vec3, b: Vec3) -> std::cmp::Ordering {
    a.x.total_cmp(&b.x)
        .then_with(|| a.y.total_cmp(&b.y))
        .then_with(|| a.z.total_cmp(&b.z))
}

/// The flown player ship's root.
#[cfg(feature = "debug")]
fn player_root(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .iter(world)
        .next()
        .expect("the beat before this one waited for the player ship")
}

/// How many mates the runtime derivation finds over one ship's sections, or
/// the errors that rejected it.
/// The sections of the ship being EDITED, read off the public
/// [`EditorProbe`] rather than off the scene.
///
/// The scene stopped being able to answer this: what carries `SectionMarker` in
/// the editor is a render-only view whose own transform is identity, and the
/// pose lives on the document node above it. The probe reports the ship in the
/// edit context, which also means a second ship on the stage cannot leak into a
/// derivation that demands ONE connected structure.
///
/// Link points come from the public catalog, keyed by the prototype the probe
/// names - the same catalog the editor placed the section out of.
#[cfg(feature = "debug")]
fn edited_sections(world: &mut World) -> Vec<(Transform, SectionLinkPoints)> {
    let Some(probe) = world.get_resource::<EditorProbe>() else {
        return Vec::new();
    };
    let ship = probe.ship.clone();
    let Some(catalog) = world.get_resource::<GameSections>() else {
        return Vec::new();
    };
    let mut sections: Vec<_> = ship
        .iter()
        .filter_map(|section| {
            let config = catalog.get_section(&section.prototype)?;
            Some((
                Transform::from_translation(section.position).with_rotation(section.rotation),
                SectionLinkPoints(config.base.link_points.clone()),
            ))
        })
        .collect();
    sections.sort_by(|(a, _), (b, _)| by_pose(a.translation, b.translation));
    sections
}

#[cfg(feature = "debug")]
fn mate_graph(world: &mut World, root: Option<Entity>) -> Result<usize, String> {
    let sections = match root {
        // A flown ship IS its sections: the marker and the pose are the same
        // entity out there.
        Some(root) => placed_sections(world, root),
        None => edited_sections(world),
    };
    let placed: Vec<PlacedSectionLinkPoints> = sections
        .iter()
        .map(|(transform, points)| PlacedSectionLinkPoints {
            position: transform.translation,
            rotation: transform.rotation,
            link_points: points,
        })
        .collect();
    derive_link_point_graph(&placed)
        .map(|mates| mates.len())
        .map_err(|errors| format!("{errors:?}"))
}

/// Sections whose pose is rolled off their parent's frame - what the R key
/// leaves behind.
#[cfg(feature = "debug")]
fn sections_with_a_rotation(world: &mut World) -> usize {
    edited_sections(world)
        .iter()
        .filter(|(transform, _)| transform.rotation.angle_between(Quat::IDENTITY) > 1e-3)
        .count()
}

/// The viewport aim for a point in ship space, so a beat can point at a
/// NAMED face rather than at whatever the camera happens to like.
#[cfg(feature = "debug")]
fn aim_at_world(world: &mut World, point: Vec3) -> Option<Vec2> {
    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()?;
    let camera = world.get::<Camera>(camera_entity)?.clone();
    let camera_transform = *world.get::<GlobalTransform>(camera_entity)?;
    camera
        .world_to_viewport(&camera_transform, point)
        .ok()
        .filter(|aim: &Vec2| aim.x.is_finite() && aim.y.is_finite())
}

/// The camera-facing socket of the section nearest the camera, as two viewport
/// aims: its centre, and a point on the same face 0.35 off centre.
///
/// Both land on the SAME socket - the editor mates the socket nearest the
/// pointer's hit, and on a unit face nothing else is within 0.35 - which is
/// what lets one beat fill that socket and the next meet the refusal, without
/// the second aim being swallowed by the part the first one mounted.
#[cfg(feature = "debug")]
fn aim_at_a_visible_face(world: &mut World) -> Option<(Vec2, Vec2)> {
    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()?;
    let camera = world.get::<Camera>(camera_entity)?.clone();
    let camera_transform = *world.get::<GlobalTransform>(camera_entity)?;
    let eye = camera_transform.translation();

    // The unit-cube hull sections are the ship's body; a mounted module is
    // smaller than a face and must not become the target itself.
    let sections = edited_sections(world);
    let (transform, points) = sections
        .iter()
        .filter(|(_, points)| points.len() >= 6)
        .min_by(|(a, _), (b, _)| {
            a.translation
                .distance_squared(eye)
                .partial_cmp(&b.translation.distance_squared(eye))
                .unwrap_or(std::cmp::Ordering::Equal)
                // Two sections the same distance from the eye must resolve the
                // same way every run; without this the nearer-of-equals is
                // whichever the list happened to hold first.
                .then_with(|| by_pose(a.translation, b.translation))
        })?;

    let (position, normal) = points
        .iter()
        .map(|point| {
            (
                transform.translation + transform.rotation * point.position,
                (transform.rotation * point.normal).normalize(),
            )
        })
        .max_by(|(_, a), (_, b)| {
            let facing = |normal: &Vec3| normal.dot((eye - transform.translation).normalize());
            facing(a)
                .partial_cmp(&facing(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;

    let tangent = normal.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
    Some((
        camera.world_to_viewport(&camera_transform, position).ok()?,
        camera
            .world_to_viewport(&camera_transform, position + tangent * 0.35)
            .ok()?,
    ))
}

/// The text the filter beat types: the first word of the hull's display name,
/// lowercased, so the run proves the filter is case-insensitive as well as
/// narrowing.
#[cfg(feature = "debug")]
fn filter_needle(world: &World) -> String {
    world
        .resource::<EditorWalk>()
        .hull
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_lowercase()
}

/// How many laid-out, visible UI nodes have a name starting with `prefix`.
///
/// `&World`, so the same count backs both the beats that ASSERT on it and the
/// predicates that WAIT on it.
#[cfg(feature = "debug")]
fn count_named_with_prefix(world: &World, prefix: &str) -> usize {
    world
        .try_query::<(&Name, &InheritedVisibility)>()
        .map_or(0, |mut query| {
            query
                .iter(world)
                .filter(|(name, visibility)| visibility.get() && name.as_str().starts_with(prefix))
                .count()
        })
}

/// The visible names carrying `prefix`, with the prefix stripped - what a beat
/// asserting on WHICH tiles are up needs, where a count only says how many.
#[cfg(feature = "debug")]
fn visible_names_with_prefix(world: &mut World, prefix: &str) -> Vec<String> {
    world
        .query::<(&Name, &InheritedVisibility)>()
        .iter(world)
        .filter(|(name, visibility)| visibility.get() && name.as_str().starts_with(prefix))
        .map(|(name, _)| name.as_str()[prefix.len()..].to_string())
        .collect()
}

/// Whether a named widget is currently greyed out. The menus say what the
/// editor CAN do here by greying what it cannot, so a beat asserting context
/// reads the same component the paint does.
#[cfg(feature = "debug")]
fn widget_is_disabled(world: &mut World, name: &str) -> bool {
    world
        .query::<(&Name, Has<bevy::ui::InteractionDisabled>)>()
        .iter(world)
        .find(|(node_name, _)| node_name.as_str() == name)
        .map(|(_, disabled)| disabled)
        .unwrap_or_else(|| panic!("the editor must have a widget named `{name}`"))
}

/// The display name of any hull section in the catalog (the section the run places).
#[cfg(feature = "debug")]
fn hull_section_name(world: &World) -> Option<String> {
    world
        .resource::<GameSections>()
        .iter()
        .find(|section| matches!(section.kind, SectionKind::Hull(_)))
        .map(|section| section.base.name.clone())
}

/// Count the sections of the ship being EDITED.
///
/// The edit context's own list rather than a sweep of every `SectionMarker` in
/// the world: the range's seeded hulks and pickets are ship nodes with sections
/// of their own, so "how many parts are on the ship I am building" stopped
/// being "how many section meshes exist".
///
/// `&World` for the same reason [`count_named_with_prefix`] is: the beats read
/// it and the predicates below wait on it, and one counter cannot disagree with
/// itself.
#[cfg(feature = "debug")]
fn count_sections(world: &World) -> usize {
    world
        .get_resource::<EditorProbe>()
        .map_or(0, |probe| probe.ship.len())
}

/// Count the skin plates on screen - the editor's preview cladding, and after
/// Play the flown ship's real one. Both wear the same marker, which is the
/// point: one derivation, two places.
#[cfg(feature = "debug")]
fn count_plates(world: &World) -> usize {
    world
        .try_query_filtered::<(), With<ShipSkinMarker>>()
        .map_or(0, |mut plates| plates.iter(world).count())
}

/// A predicate over the world, in the shape every `until` takes.
#[cfg(feature = "debug")]
type Wait = std::sync::Arc<nova_protocol::nova_debug::harness::Predicate>;

/// Advance once the ship carries `delta` MORE sections than the last
/// [`stamp_sections`] took.
///
/// `delta` of zero is the wait a click that must build NOTHING takes: by the
/// time the pointer has registered its release the editor has already answered
/// the press, so a count still equal to the stamp is a fact and not a hope.
#[cfg(feature = "debug")]
fn sections_grew_by(delta: usize) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        count_sections(world) == world.resource::<EditorWalk>().sections + delta
    })
}

/// Advance once the ship carries `delta` FEWER sections than the last stamp -
/// what a click in delete mode is for.
#[cfg(feature = "debug")]
fn sections_shrank_by(delta: usize) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        count_sections(world) + delta == world.resource::<EditorWalk>().sections
    })
}

/// The ids of the ship being edited, in the order the document keeps them.
#[cfg(feature = "debug")]
fn edited_section_ids(world: &World) -> Vec<String> {
    world
        .resource::<EditorProbe>()
        .ship
        .iter()
        .map(|section| section.id.clone())
        .collect()
}

/// Advance once the editor is inside SOME ship carrying `sections` of them.
///
/// The count is what tells one ship from another without knowing the minted
/// ids: a freshly seeded ship carries exactly one section, and the ship this
/// run built carries several.
#[cfg(feature = "debug")]
fn inside_a_ship_of(sections: usize) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        let probe = world.resource::<EditorProbe>();
        probe.inside.is_some() && probe.ship.len() == sections
    })
}

/// Advance once the editor is back inside a ship carrying as many sections as
/// the last [`EditorWalk::ids`] stamp took.
#[cfg(feature = "debug")]
fn back_inside_the_stamped_ship() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let probe = world.resource::<EditorProbe>();
        probe.inside.is_some() && probe.ship.len() == world.resource::<EditorWalk>().ids.len()
    })
}

/// Advance once the editor is INSIDE the node called `id`.
#[cfg(feature = "debug")]
fn inside_the_node(id: &'static str) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        world.resource::<EditorProbe>().inside.as_deref() == Some(id)
    })
}

/// The text of the UI node called `name`, if it is on screen.
#[cfg(feature = "debug")]
fn named_text(world: &mut World, name: &str) -> Option<String> {
    let mut query = world.try_query::<(&Name, &Text)>()?;
    query
        .iter(world)
        .find(|(node_name, _)| node_name.as_str() == name)
        .map(|(_, text)| text.0.clone())
}

/// Every Scene tree row on screen, by name.
#[cfg(feature = "debug")]
fn scene_row_names(world: &mut World) -> Vec<String> {
    let Some(mut query) = world.try_query::<(&Name, &InheritedVisibility)>() else {
        return Vec::new();
    };
    let mut rows: Vec<String> = query
        .iter(world)
        .filter(|(name, visibility)| visibility.get() && name.as_str().starts_with(SCENE_ROW))
        .map(|(name, _)| name.as_str().to_string())
        .collect();
    rows.sort();
    rows
}

/// Advance once the editor is out at the scenario node, where Play lives.
#[cfg(feature = "debug")]
fn at_the_scenario_node() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let probe = world.resource::<EditorProbe>();
        probe.inside.is_none() && probe.can_play
    })
}

/// Advance once an object of `stem` is in the document AND marked.
///
/// Matched by PREFIX, not by a whole id: the scenario node mints one ordinal
/// across all of its children, so which number the first placed rock gets
/// depends on how many ships the walk built before it.
#[cfg(feature = "debug")]
fn an_object_was_placed(stem: &'static str) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        let probe = world.resource::<EditorProbe>();
        probe
            .selected_node
            .as_deref()
            .is_some_and(|node| node.starts_with(stem))
            && probe
                .context_nodes
                .iter()
                .any(|node| node.starts_with(stem))
    })
}

/// Advance once the status line starts with `opening`.
///
/// The rail's line is the only visible outcome a file verb has, so this is how
/// a beat learns that Save or Open ran and what it decided. Matched by PREFIX:
/// an Open reports the counts it read, and the counts are what the beat after
/// checks properly.
/// No top-bar menu hangs open.
///
/// A menu is what Escape's first rung is spent on, so a beat that opened one
/// has to be able to see it closed before the next gesture is aimed anywhere.
#[cfg(feature = "debug")]
fn no_menu_is_open() -> Wait {
    std::sync::Arc::new(|world: &World| world.resource::<EditorProbe>().open_menu.is_empty())
}

#[cfg(feature = "debug")]
fn the_status_reads(opening: &'static str) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        world.resource::<EditorProbe>().status.starts_with(opening)
    })
}

/// Advance once no object of `stem` is left in the document.
#[cfg(feature = "debug")]
fn no_object_named(stem: &'static str) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        !world
            .resource::<EditorProbe>()
            .context_nodes
            .iter()
            .any(|node| node.starts_with(stem))
    })
}

/// Advance once the Scene tree's mark sits on the first ship - what a world
/// click at the scenario node does now, where it used to enter.
#[cfg(feature = "debug")]
fn the_first_ship_is_selected() -> Wait {
    std::sync::Arc::new(|world: &World| {
        world.resource::<EditorProbe>().selected_node.as_deref() == Some("ship_1")
    })
}

/// Where the first ship stands on the stage, off the probe's node positions.
#[cfg(feature = "debug")]
fn first_ship_position(world: &World) -> Vec3 {
    world
        .resource::<EditorProbe>()
        .node_positions
        .iter()
        .find(|(id, _)| id == "ship_1")
        .map(|(_, at)| *at)
        .expect("ship_1 is listed at the scenario node")
}

/// Advance once the first ship stands away from the drag stamp - the
/// ground-plane drag doing its work.
#[cfg(feature = "debug")]
fn the_first_ship_moved() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let stamp = world.resource::<EditorWalk>().first_ship_at;
        world
            .resource::<EditorProbe>()
            .node_positions
            .iter()
            .any(|(id, at)| id == "ship_1" && at.distance(stamp) > 0.5)
    })
}

/// Advance once the first ship stands ABOVE the stamp - the Y handle doing its
/// work, and nothing else could have done it: the ground-plane drag holds
/// altitude by construction.
#[cfg(feature = "debug")]
fn the_first_ship_rose() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let stamp = world.resource::<EditorWalk>().first_ship_at;
        world
            .resource::<EditorProbe>()
            .node_positions
            .iter()
            .any(|(id, at)| id == "ship_1" && at.y - stamp.y > 0.5)
    })
}

/// A viewport aim at a NAMED entity in the world - the gizmo's arrowheads,
/// which are the only 3D things this walk points at by name.
///
/// The rig is `pub(crate)` to the editor, so the walk reaches its handles the
/// way anything outside a crate reaches an entity: by the name they carry.
#[cfg(feature = "debug")]
fn aim_at_the_named(world: &mut World, name: &str) -> Option<Vec2> {
    let at = world
        .query::<(&Name, &GlobalTransform)>()
        .iter(world)
        .find(|(named, _)| named.as_str() == name)
        .map(|(_, pose)| pose.translation())?;
    aim_at_world(world, at)
}

/// Advance once the named box is showing `reason` as its refusal.
///
/// Read off the box's own `TextFieldError`, which is what the panel draws: the
/// claim is that the builder who typed it is told, on the thing they typed
/// into.
#[cfg(feature = "debug")]
fn the_field_refuses(name: &'static str, reason: &'static str) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        let Some(entity) = named_entity(world, name) else {
            return false;
        };
        world
            .get::<nova_ui::prelude::TextFieldError>(entity)
            .is_some_and(|error| error.0 == reason)
    })
}

/// What the box named `name` is showing.
#[cfg(feature = "debug")]
fn field_text(world: &World, name: &str) -> String {
    named_entity(world, name)
        .and_then(|entity| world.get::<nova_ui::prelude::TextFieldValue>(entity))
        .map(|value| value.0.clone())
        .unwrap_or_default()
}

/// What the text node named `name` says.
#[cfg(feature = "debug")]
fn text_of(world: &World, name: &str) -> String {
    named_entity(world, name)
        .and_then(|entity| world.get::<Text>(entity))
        .map(|text| text.0.clone())
        .unwrap_or_default()
}

/// Advance once a section is waiting for its new key.
///
/// Read off the CHIP over the part rather than a resource: what the beat
/// before it pressed was a row in the panel, and what proves the two are about
/// the same section is the prompt appearing on that section.
#[cfg(feature = "debug")]
fn a_section_awaits_its_key() -> Wait {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query::<(&Name, &Text)>()
            .is_some_and(|mut chips| {
                chips.iter(world).any(|(name, text)| {
                    name.as_str() == "Section Keybind Label" && text.0 == "press key"
                })
            })
    })
}

/// Advance once the inspected section's Key row reads `wanted`.
#[cfg(feature = "debug")]
fn the_key_row_reads(wanted: &'static str) -> Wait {
    std::sync::Arc::new(move |world: &World| inspector_reading(world, "Key") == wanted)
}

/// Advance once the rock's Position row reads `wanted` in its middle number.
///
/// The MIDDLE one, because the beat that waits on this typed into the Y box:
/// what it is proving is that the box wrote its own component and no other.
#[cfg(feature = "debug")]
fn the_rocks_height_reads(wanted: &'static str) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        inspector_reading(world, "Position").split(", ").nth(1) == Some(wanted)
    })
}

/// Advance once the editor reports `id` as the node under the pointer.
///
/// One resource behind both surfaces, so this is what proves a rail hover and
/// a stage hover are the same fact rather than two highlights that agree by
/// coincidence.
#[cfg(feature = "debug")]
fn the_hovered_node_is(id: &'static str) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        world.resource::<EditorProbe>().hovered_node.as_deref() == Some(id)
    })
}

/// How strongly a named Scene row is painted. A row lit by hover or by
/// selection carries a tinted background; an untouched one is transparent, so
/// the alpha alone says whether the tree answered.
#[cfg(feature = "debug")]
fn row_tint(world: &mut World, name: &str) -> f32 {
    let row = named_entity(world, name).unwrap_or_else(|| panic!("the tree must list `{name}`"));
    world
        .get::<BackgroundColor>(row)
        .map_or(0.0, |paint| paint.0.alpha())
}

/// Advance once the object the picker beats placed is out of the document.
#[cfg(feature = "debug")]
fn the_placed_node_is_gone() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let placed = world.resource::<EditorWalk>().placed.clone();
        !placed.is_empty()
            && !world
                .resource::<EditorProbe>()
                .context_nodes
                .iter()
                .any(|node| *node == placed)
    })
}

/// What the Inspector's row called `label` reads.
#[cfg(feature = "debug")]
fn inspector_reading(world: &World, label: &str) -> String {
    world
        .resource::<EditorProbe>()
        .inspector
        .iter()
        .find(|(row, _)| row == label)
        .map(|(_, reading)| reading.clone())
        .unwrap_or_default()
}

/// Advance once the colour row reads something other than what was stamped.
#[cfg(feature = "debug")]
fn the_colour_row_changed() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let stamp = &world.resource::<EditorWalk>().colour;
        let now = inspector_reading(world, "Color");
        !now.is_empty() && now != *stamp
    })
}

/// Where the floating picker's top-left corner sits, in the pixels its own
/// `Node` is placed with.
#[cfg(feature = "debug")]
fn window_position(world: &mut World) -> Option<Vec2> {
    let window = named_entity(world, "Colour Window")?;
    let node = world.get::<Node>(window)?;
    match (node.left, node.top) {
        (Val::Px(left), Val::Px(top)) => Some(Vec2::new(left, top)),
        _ => None,
    }
}

/// Advance once the picker stands somewhere other than where it was stamped.
#[cfg(feature = "debug")]
fn the_picker_moved() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let stamp = world.resource::<EditorWalk>().window_at;
        let Some(window) = named_entity(world, "Colour Window") else {
            return false;
        };
        let Some(node) = world.get::<Node>(window) else {
            return false;
        };
        match (node.left, node.top) {
            (Val::Px(left), Val::Px(top)) => Vec2::new(left, top).distance(stamp) > 1.0,
            _ => false,
        }
    })
}

/// Advance once the pointer is over the entity called `name`.
///
/// The UI answers a pointer through the same picking pipeline the stage does,
/// so a beat that means to drag a window proves it has the window under the
/// cursor before it presses - exactly as the gizmo beats do.
#[cfg(feature = "debug")]
fn the_pointer_is_on(name: &'static str) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        let Some(hit) = world
            .try_query::<&bevy::picking::pointer::PointerInteraction>()
            .and_then(|mut pointers| {
                pointers
                    .iter(world)
                    .filter_map(|interaction| interaction.get_nearest_hit())
                    .map(|(entity, _)| *entity)
                    .next()
            })
        else {
            return false;
        };
        world
            .get::<Name>(hit)
            .is_some_and(|named| named.as_str() == name)
    })
}

/// Advance once the turret's rows are TALLER than the panel that holds them.
///
/// This is the reason the panel scrolls at all: a turret carries a joint list,
/// and the fire rate sits at the bottom of it. If the rows ever fit, the beat
/// that scrolls below has nothing to prove and says so here instead.
#[cfg(feature = "debug")]
fn the_inspector_overflows() -> Wait {
    std::sync::Arc::new(|world: &World| {
        inspector_list(world).is_some_and(|node| node.content_size().y > node.size().y + 1.0)
    })
}

/// Advance once the panel has actually rolled down. Bevy clamps a scroll to the
/// overflow it has, so a list that fits stays at zero however hard the wheel
/// turns - which makes this the honest end of the gesture.
#[cfg(feature = "debug")]
fn the_inspector_scrolled() -> Wait {
    std::sync::Arc::new(|world: &World| inspector_scroll(world) > 1.0)
}

/// The labels the inspector is showing, in the order it draws them.
#[cfg(feature = "debug")]
fn inspector_labels(world: &World) -> Vec<String> {
    world
        .resource::<EditorProbe>()
        .inspector
        .iter()
        .map(|(label, _)| label.clone())
        .collect()
}

/// The scenario id the editor's save writes its range under.
#[cfg(feature = "debug")]
const SAVED_RANGE_ID: &str = "editor_save";

/// How long a reload gets: it re-reads every content file off disk and the
/// merge runs after they land, so it is several frames rather than one.
#[cfg(feature = "debug")]
const RELOAD_DEADLINE_SECS: f32 = 10.0;

/// Advance once the game's own scenario registry holds `id` - the list the
/// Scenarios picker is drawn from.
#[cfg(feature = "debug")]
fn the_scenarios_hold(id: &'static str) -> Wait {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<GameScenarios>()
            .is_some_and(|scenarios| scenarios.contains_key(id))
    })
}

/// What the Scene rows READ as, in draw order - the label column only.
#[cfg(feature = "debug")]
fn scene_row_labels(world: &World) -> Vec<String> {
    let Some(list) = named_entity(world, "Scene List") else {
        return Vec::new();
    };
    let Some(rows) = world.get::<Children>(list) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let wrapper = world.get::<Children>(row)?.iter().nth(1)?;
            let label = world.get::<Children>(wrapper)?.iter().next()?;
            Some(world.get::<Text>(label)?.0.clone())
        })
        .collect()
}

/// Advance once the rail is drawing ids rather than names.
///
/// The ENTERED ship is the row to read it off: its id is short enough to
/// survive the rail's elide, where a section's `<part>_section_<n>` is not, and
/// the name it wears otherwise (`Ship 1`) is not its id (`ship_1`).
#[cfg(feature = "debug")]
fn the_tree_reads_as_ids() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let Some(inside) = world.resource::<EditorProbe>().inside.clone() else {
            return false;
        };
        scene_row_labels(world).iter().any(|label| *label == inside)
    })
}

/// Advance once the inspector is showing a row called `label`.
#[cfg(feature = "debug")]
fn the_inspector_has_a_row(label: &'static str) -> Wait {
    std::sync::Arc::new(move |world: &World| inspector_labels(world).iter().any(|row| row == label))
}

/// How far down the inspector stands.
#[cfg(feature = "debug")]
fn inspector_scroll(world: &World) -> f32 {
    named_entity(world, "Inspector List")
        .and_then(|entity| world.get::<ScrollPosition>(entity))
        .map_or(0.0, |scroll| scroll.y)
}

/// The laid-out box of the inspector's scrolling list.
#[cfg(feature = "debug")]
fn inspector_list(world: &World) -> Option<&ComputedNode> {
    world.get::<ComputedNode>(named_entity(world, "Inspector List")?)
}

/// The one entity carrying `name`, or nothing.
#[cfg(feature = "debug")]
fn named_entity(world: &World, name: &str) -> Option<Entity> {
    let mut named = world.try_query::<(Entity, &Name)>()?;
    named
        .iter(world)
        .find(|(_, named)| named.as_str() == name)
        .map(|(entity, _)| entity)
}

/// Advance once the pointer is over a gizmo handle rather than over the ship
/// behind it. Two picking backends answer this pointer - the stage's colliders
/// and the handles' meshes - so a beat that means to drag an axis proves which
/// one won before it presses.
#[cfg(feature = "debug")]
fn the_pointer_is_on_a_handle() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let Some(hit) = world
            .try_query::<&bevy::picking::pointer::PointerInteraction>()
            .and_then(|mut pointers| {
                pointers
                    .iter(world)
                    .filter_map(|interaction| interaction.get_nearest_hit())
                    .map(|(entity, _)| *entity)
                    .next()
            })
        else {
            return false;
        };
        world
            .get::<Name>(hit)
            .is_some_and(|name| name.as_str().starts_with("Gizmo "))
    })
}

/// Where the editor camera stands.
#[cfg(feature = "debug")]
fn camera_position(world: &mut World) -> Option<Vec3> {
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()?;
    Some(world.get::<GlobalTransform>(camera)?.translation())
}

/// Advance once the camera is AIMED at the first ship, whatever route put it
/// there. A pose comparison rather than a distance, because "framed" is about
/// what is in the middle of the screen.
#[cfg(feature = "debug")]
fn the_camera_frames_the_first_ship() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let Some((_, at)) = world
            .resource::<EditorProbe>()
            .node_positions
            .iter()
            .find(|(id, _)| id == "ship_1")
        else {
            return false;
        };
        let Some(mut cameras) = world.try_query_filtered::<&GlobalTransform, With<Camera3d>>()
        else {
            return false;
        };
        cameras.iter(world).any(|pose| {
            let towards = *at - pose.translation();
            towards.length() > f32::EPSILON
                && pose.forward().as_vec3().dot(towards.normalize()) > 0.98
        })
    })
}

/// Advance once the camera stands ABOVE the first ship, looking down at it -
/// what View > Top asks for. Level against the height, so the check reads the
/// same whatever the ship's spread backed the camera off to.
#[cfg(feature = "debug")]
fn the_camera_looks_down_on_the_first_ship() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let Some((_, at)) = world
            .resource::<EditorProbe>()
            .node_positions
            .iter()
            .find(|(id, _)| id == "ship_1")
        else {
            return false;
        };
        let Some(mut cameras) = world.try_query_filtered::<&GlobalTransform, With<Camera3d>>()
        else {
            return false;
        };
        cameras.iter(world).any(|pose| {
            let over = pose.translation() - *at;
            // Against the node's own origin, which is not the middle of the
            // hull the view is framing - so the cone is generous and the
            // AIM is what carries the check.
            over.y > 1.0 && over.xz().length() < over.y * 0.35 && pose.forward().y < -0.85
        })
    })
}

/// A viewport aim at the FIRST ship on the stage.
///
/// Reads the scene rather than the probe, because outside a ship the probe
/// reports no ship at all - that is the point of the scoping. Ships are spaced
/// along +X from the origin, so the section view nearest x=0 is on the first
/// one; every ship's sections are on screen out here, and any other rule would
/// pick whichever the archetype walk yielded.
#[cfg(feature = "debug")]
fn aim_at_the_first_ship(world: &mut World) -> Option<Vec2> {
    let nearest = world
        .query_filtered::<&GlobalTransform, With<SectionMarker>>()
        .iter(world)
        .map(GlobalTransform::translation)
        .min_by(|a, b| {
            a.x.abs()
                .partial_cmp(&b.x.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| by_pose(*a, *b))
        })?;
    aim_at_world(world, nearest)
}

/// Advance once the flown PLAYER ship carries every section of the design the
/// run stamped - the ids taken before leaving the first ship, because the
/// world-wide section count now includes the second ship built beside it.
///
/// The hand-off, waited on rather than settled for: Play tears the preview down
/// and the scenario spawns the real ship a while later, and "a while" is a
/// property of the machine.
#[cfg(feature = "debug")]
fn the_flown_ship_is_whole() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let built = world.resource::<EditorWalk>().ids.len();
        let Some(root) = world
            .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
            .and_then(|mut roots| roots.iter(world).next())
        else {
            return false;
        };
        world
            .try_query_filtered::<&ChildOf, With<SectionMarker>>()
            .is_some_and(|mut sections| {
                sections
                    .iter(world)
                    .filter(|ChildOf(parent)| *parent == root)
                    .count()
                    == built
            })
    })
}

/// Advance once the AI design stands on the range: the sandbox lowers every
/// non-empty ship of the document, not just the player's.
///
/// Found by the NAME the ship node carries - `ship_2` is minted "Ship 2" - so
/// this also proves the name a builder can edit is the one the range shows.
#[cfg(feature = "debug")]
fn the_second_ship_flies() -> Wait {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query::<&Name>()
            .is_some_and(|mut names| names.iter(world).any(|name| name.as_str() == "Ship 2"))
    })
}

/// Advance once the derived cladding is on whatever ship is on screen.
#[cfg(feature = "debug")]
fn the_skin_is_on() -> Wait {
    std::sync::Arc::new(|world: &World| count_plates(world) > 0)
}

/// Advance once the cladding has re-derived to a different number of plates
/// than the last stamp - what holding a part against the ship does to it.
#[cfg(feature = "debug")]
fn the_skin_reflowed() -> Wait {
    std::sync::Arc::new(|world: &World| {
        count_plates(world) != world.resource::<EditorWalk>().plates
    })
}

/// Advance once the gallery is listing anything at all - the layout pass behind
/// the state, which the beats that count tiles and hover one both need.
#[cfg(feature = "debug")]
fn some_gallery_tiles() -> Wait {
    std::sync::Arc::new(|world: &World| count_named_with_prefix(world, GALLERY_TILE) > 0)
}

/// Advance once the grid holds fewer tiles than the last stamp - what typing
/// into the filter is for.
#[cfg(feature = "debug")]
fn the_gallery_narrowed() -> Wait {
    std::sync::Arc::new(|world: &World| {
        count_named_with_prefix(world, GALLERY_TILE) < world.resource::<EditorWalk>().tiles
    })
}

/// Advance once the picking pointer is hovering the gallery tile for the hull
/// the run builds with.
///
/// The pipette reads the HOVER, not the selection, so this is the fact Q needs
/// and nothing else says it: the tile is resolved by `Name` at run time, which
/// is why it is a closure here rather than a named-target predicate.
#[cfg(feature = "debug")]
fn the_hull_tile_is_hovered() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let tile = format!("{GALLERY_TILE}{}", world.resource::<EditorWalk>().hull);
        world
            .try_query::<(&Name, &bevy::picking::hover::Hovered)>()
            .is_some_and(|mut query| {
                query
                    .iter(world)
                    .any(|(name, hovered)| name.as_str() == tile && hovered.get())
            })
    })
}

/// Advance once the picking pointer is over one of the ship's sections.
///
/// The aim ack for a click that is NOT placing: select and delete mode solve no
/// placement, so this is all the editor has to say about where the pointer is.
#[cfg(feature = "debug")]
fn the_pointer_is_on_the_ship() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let Some(hit) = world
            .try_query::<&bevy::picking::pointer::PointerInteraction>()
            .and_then(|mut pointers| {
                pointers
                    .iter(world)
                    .filter_map(|interaction| interaction.get_nearest_hit())
                    .map(|(entity, _)| *entity)
                    .next()
            })
        else {
            return false;
        };
        world.get::<SectionMarker>(hit).is_some()
    })
}

/// The refusal the editor is showing, if it is refusing.
///
/// Read off `nova_editor`'s own [`EditorProbe`] rather than scraped out of the
/// status line: what the SOLVER decided and what the builder is told are two
/// claims, and the refusal beats make both.
#[cfg(feature = "debug")]
fn placement_refusal(world: &World) -> Option<&'static str> {
    match world.resource::<EditorProbe>().placement {
        EditorPlacement::Refused { reason, .. } => Some(reason),
        _ => None,
    }
}

/// Record the live section count, so the beat after the next gesture can say
/// what that gesture changed.
#[cfg(feature = "debug")]
fn stamp_sections(world: &mut World) {
    let count = count_sections(world);
    world.resource_mut::<EditorWalk>().sections = count;
}

/// Every line of text under the named UI node, empty when no such node is up.
#[cfg(feature = "debug")]
fn subtree_text(world: &mut World, name: &str) -> Vec<String> {
    let Some(root) = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, node_name)| node_name.as_str() == name)
        .map(|(entity, _)| entity)
    else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(text) = world.get::<Text>(entity) {
            lines.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    lines
}

/// The viewport position (logical px) of a preview section, or `None` until a
/// section, the 3D camera and the window all exist.
///
/// `Camera::world_to_viewport` answers in LOGICAL pixels, which is the space
/// [`move_cursor`] takes - no scale-factor conversion belongs here.
#[cfg(feature = "debug")]
fn aim_at_a_section(world: &mut World) -> Option<Vec2> {
    let mut q_sections =
        world.query_filtered::<(&GlobalTransform, &InheritedVisibility), With<SectionMarker>>();
    // The LOWEST-posed VISIBLE section, rather than the first one the query
    // yields: query order is an archetype detail that changes between runs,
    // and a beat that clicks a different section each time proves nothing (see
    // `placed_sections`). Visible is the half that scopes it to the ship being
    // built - the range's own seeded hulks and pickets are ship nodes with
    // sections of their own, and entering a ship takes them off the stage.
    let section_pos = q_sections
        .iter(world)
        .filter(|(_, visible)| visible.get())
        .map(|(pose, _)| pose.translation())
        .min_by(|a, b| by_pose(*a, *b))?;

    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()?;
    let camera = world.get::<Camera>(camera_entity)?;
    let camera_transform = world.get::<GlobalTransform>(camera_entity)?;
    camera.world_to_viewport(camera_transform, section_pos).ok()
}

/// The gestures this walk is written in, each spelled as beats that WAIT.
///
/// An extension trait rather than free functions so a gesture reads in the
/// script as one line. What every method has in common is the shape: act, then
/// hold until the app has done the thing the act asks for - never until a
/// number of frames has gone by.
#[cfg(feature = "debug")]
trait EditorGestures {
    /// Click a NAMED widget: wait for it to lay out, press it, release it.
    ///
    /// Three beats, each with a condition. The layout wait is the one a frame
    /// count hid: `click_named` warns and CONTINUES when the name resolves to
    /// nothing, so a press fired at a panel that has not laid out yet is a beat
    /// silently lost, and the run fails later somewhere else.
    fn click_a_widget(self, label: &str, name: &str) -> Self;

    /// Press a widget TWICE, close enough together to read as one double click.
    ///
    /// Four beats rather than two presses in one frame: a widget's `Activate`
    /// fires on the RELEASE, and the press that arms it is a command that does
    /// not land until the frame ends - so a press and a release in the same
    /// frame activate nothing at all. Four beats is about a tenth of a second,
    /// well inside the editor's window.
    fn double_click_a_widget(self, label: &str, name: &str) -> Self;

    /// Drop a top-bar menu, then press one of its rows.
    ///
    /// Two clicks because that is the real gesture: a dropdown is `Display::None`
    /// until its button opens it, and a hidden node has no laid-out centre for
    /// the pointer to aim at. A walk that reached the item without opening the
    /// menu would pass over a menu that never drops.
    fn click_a_menu_item(self, label: &str, menu: &str, item: &str) -> Self;

    /// Press and release the pointer where it already is, then hold until
    /// `landed` - what the click was supposed to change.
    fn press_and_release(self, label: &str, landed: Wait) -> Self;

    /// Aim at the ship's nearest section, press, release, and wait for `landed`.
    ///
    /// `aimed` is what says the pointer is where the beat meant it to be: a
    /// solved placement while a part is armed, the pointer being over a section
    /// otherwise. The editor acts on `Pointer<Press>`, so the press is what does
    /// the work and the release only lets go.
    fn click_the_ship(self, label: &str, aimed: Wait, landed: Wait) -> Self;

    /// The same gesture aimed at a NAMED point in ship space, for building a
    /// particular SHAPE rather than whatever face the camera likes.
    fn place_on_the_face(self, label: &str, socket: Vec3) -> Self;

    /// Arm `prototype` through the gallery, by keyboard: open it, type the
    /// catalog id, Enter to focus the tile it left, Enter to place.
    ///
    /// The keyboard path rather than clicking a tile by name: the semantic parts
    /// are named for their ROLE (three craft each carry a "Turret Port"), so a
    /// name is not a unique target where a catalog id is. Every beat waits on
    /// the gallery's own state, so a filter that narrowed to the wrong part
    /// fails at the filter instead of arming something else.
    fn arm_from_the_gallery(self, label: &str, prototype: &str) -> Self;
}

#[cfg(feature = "debug")]
impl EditorGestures for nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    fn click_a_menu_item(self, label: &str, menu: &str, item: &str) -> Self {
        self.click_a_widget(&format!("{label}: open the menu"), menu)
            .click_a_widget(label, item)
    }

    fn double_click_a_widget(self, label: &str, name: &str) -> Self {
        let first = name.to_string();
        let second = name.to_string();
        self.step(format!("{label}: the widget is up"))
            .until(ui_node_present(name.to_string()))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: press"))
            .on_enter(click_named(first))
            .until(pointer_pressed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(pointer_released())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: press again"))
            .on_enter(click_named(second))
            .until(pointer_pressed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release again"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(pointer_released())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }

    fn click_a_widget(self, label: &str, name: &str) -> Self {
        let target = name.to_string();
        self.step(format!("{label}: the widget is up"))
            .until(ui_node_present(name.to_string()))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: press"))
            .on_enter(click_named(target))
            .until(pointer_pressed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // Widgets act on `Activate`, which fires on RELEASE over the same
            // node, so this is the beat that carries the button's effect - and
            // the caller's next beat is where that effect is waited on.
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(pointer_released())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }

    fn press_and_release(self, label: &str, landed: Wait) -> Self {
        self.step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(pointer_released())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: it landed"))
            .until(landed)
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }

    fn click_the_ship(self, label: &str, aimed: Wait, landed: Wait) -> Self {
        self.step(format!("{label}: aim"))
            .on_enter(|world: &mut World| {
                let at = aim_at_a_section(world)
                    .expect("a preview section, the 3D camera and the window are all up");
                move_cursor(at)(world);
            })
            .until(aimed)
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .press_and_release(label, landed)
    }

    fn place_on_the_face(self, label: &str, socket: Vec3) -> Self {
        self.step(format!("{label}: aim"))
            .on_enter(move |world: &mut World| {
                let at = aim_at_world(world, socket).expect("that face is on screen");
                move_cursor(at)(world);
                stamp_sections(world);
            })
            // The EDITOR says the aim landed on a socket it can build on. A
            // frame count could only say that some frames had gone by, and the
            // difference is the whole diagnostic: a face that solves nothing
            // fails here rather than three beats later on a count.
            .until(editor_placement_solved())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .press_and_release(label, sections_grew_by(1))
    }

    fn arm_from_the_gallery(self, label: &str, prototype: &str) -> Self {
        let filter = prototype.to_string();
        let narrowed = prototype.to_string();
        let armed = prototype.to_string();
        self.click_a_menu_item(
            &format!("{label}: open the gallery"),
            MENU_SHIP,
            "Parts Item",
        )
        .step(format!("{label}: the gallery is up"))
        .until(and(editor_gallery_open(), some_gallery_tiles()))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The filter takes the keyboard only once it has the caret; a click on
        // the field is how a mouse user gives it one.
        .click_a_widget(
            &format!("{label}: click the filter field"),
            "Gallery Filter",
        )
        .step(format!("{label}: the filter has the caret"))
        .until(editor_filter_focused())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Typed, then WAITED on: the gallery's selection resolving to this id
        // through the live filter is the honest end of "type enough to leave
        // one tile", and it fails here rather than arming a neighbour.
        .step(format!("{label}: filter to `{prototype}`"))
        .on_enter(move |world: &mut World| type_text(filter.clone())(world))
        .until(editor_gallery_selected(narrowed))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step(format!("{label}: press Enter to focus"))
        .on_enter(press_key(KeyCode::Enter))
        .until(ui_node_present("Gallery Focus Card"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step(format!("{label}: release Enter"))
        .on_enter(release_key(KeyCode::Enter))
        .add()
        .step(format!("{label}: press Enter to place"))
        .on_enter(press_key(KeyCode::Enter))
        .until(and(
            editor_gallery_closed(),
            editor_tool_is(EditorTool::Place(armed)),
        ))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step(format!("{label}: release Enter"))
        .on_enter(release_key(KeyCode::Enter))
        .add()
        .step(format!("{label}: the gallery closed"))
        .on_enter(|world: &mut World| {
            assert!(
                ui_node_rect(world, "Parts Gallery").is_none(),
                "placing from the gallery must close it"
            );
        })
        .add()
    }
}
