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
//! 2. click New Ship - which keeps the editor-preview controller fix (task 20260706-212909) honest:
//!    a live controller on the non-physics preview root used to flood the log with "root not found"
//!    every frame, so this run staying quiet is the regression check;
//! 3. arm a hull through the parts gallery - the editor's only parts picker;
//! 4. place TWO sections by clicking the ship through the real picking pipeline (avian's
//!    physics-picking backend raycasts the pointer to a hit, and the editor's own
//!    `on_click_spaceship_section` observer places the section);
//! 5. click Select / Rebind and click the ship again - select mode must place NOTHING;
//! 6. click Delete Section and click the ship again - the count drops back;
//! 7. walk the gallery end to end - browse (the tiles are up), filter (typing narrows the grid to
//!    one part), focus (the card names that part), select (Place arms the tool and closes the
//!    gallery) and place (a click on the ship builds the part the gallery picked);
//! 8. meet both placement REFUSALS in words and in pixels - an occupied socket, and a drive
//!    aimed up a lane the hull already stands beside, which is `nova_ship`'s clearance rule and
//!    the same one the ship generator collapses under;
//! 9. turn the SKIN on and watch it follow the build - the bare ship, the same ship clad from
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

/// The hull the run builds with. The same id `create_new_spaceship` seeds a
/// hull ship from, so a catalog that dropped it has already broken the editor.
#[cfg(feature = "debug")]
const HULL_PROTOTYPE: &str = "reinforced_hull_section";

/// A viewport point (logical px) with neither the ship nor a rail panel under
/// it, on the 1024x768 window the app opens. Pointing here is how a beat puts
/// the ghost away without disarming the part it is holding.
#[cfg(feature = "debug")]
const EMPTY_SPACE: Vec2 = Vec2::new(760.0, 640.0);

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
        .click_a_widget("editor: click New Ship", "Create New Spaceship Button V2")
        // The preview spawning is what the run waits for, and nothing else has
        // to be waited for after it: the first click on the ship holds until the
        // EDITOR has solved a placement there, which cannot happen before avian
        // has prepared the collider that click's ray must hit.
        .step("editor: the new ship is up")
        .until(sections_number(1))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("editor: the ship came up with a controller")
        .on_enter(|world: &mut World| {
            // `create_new_spaceship_with_controller` spawns the preview with
            // exactly its controller section - the editor's own marker types are
            // crate-private, so the section COUNT is what an example can see, and
            // it is the same claim: the ship exists and it is not empty.
            let sections = count_sections(world);
            assert_eq!(
                sections, 1,
                "New Ship must create a ship carrying its controller section"
            );
            let hull = hull_section_name(world).expect("the catalog lists a hull section");
            nova_probe::probe_marker(
                world,
                "outcome: new ship carries its controller",
                serde_json::json!({}),
            );
            info!("editor: ship created, will build with `{hull}`");
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
        .click_a_widget("editor: click Select / Rebind", "Select Section Button")
        // The armed tool is READ now rather than inferred: the rail chip
        // disarming the placement tool is a claim this beat makes, where it
        // used to be something the next two clicks had to imply by building
        // nothing.
        .step("editor: the editor put the part down")
        .until(editor_tool_is(EditorTool::Select))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click_the_ship(
            "editor: click the ship in select mode",
            the_pointer_is_on_the_ship(),
            sections_grew_by(0),
        )
        .step("editor: select mode placed nothing")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().sections;
            let now = count_sections(world);
            assert_eq!(
                now, before,
                "select mode must not place anything; the same click built a \
                 section a moment ago"
            );
            nova_probe::probe_marker(
                world,
                "outcome: select mode places nothing",
                serde_json::json!({}),
            );
            info!("editor: select mode is inert for placement ({now} sections)");
        })
        .add()
        .click_a_widget("editor: click Delete Section", "Delete Section Button")
        .step("editor: the delete tool is armed")
        .until(editor_tool_is(EditorTool::Delete))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click_the_ship(
            "editor: click the ship in delete mode",
            the_pointer_is_on_the_ship(),
            sections_shrank_by(1),
        )
        .step("editor: the count dropped back")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorWalk>().sections;
            let now = count_sections(world);
            assert_eq!(
                now,
                before - 1,
                "a click in delete mode must remove exactly the section under \
                 the pointer"
            );
            nova_probe::probe_marker(
                world,
                "outcome: delete removes the section clicked",
                serde_json::json!({}),
            );
            info!("editor: deleted a section ({before} -> {now})");
        })
        .add()
        // The parts gallery, walked the way a player walks it: browse, filter,
        // focus, select, place. Every beat is a real gesture on a real widget;
        // what it WAITS on is the gallery's own state, so a grid that never
        // narrowed fails at the filter rather than at whatever the next Enter
        // happened to arm.
        .click_a_widget("editor: open the parts gallery", "Parts Gallery Category")
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
            for stat in ["hp", "size", "sockets"] {
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
            let status = subtree_text(world, "Placement Status");
            info!("editor: the editor refuses with {refusal:?}, status reads {status:?}");
            assert!(
                refusal.is_some_and(|reason| reason.contains("occupied")),
                "an occupied socket must be refused; the editor decided {refusal:?}"
            );
            assert!(
                status.iter().any(|line| line.contains("occupied")),
                "and the builder must be told so in words; the status read {status:?}"
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
            let status = subtree_text(world, "Placement Status");
            info!("editor: the editor refuses with {refusal:?}, status reads {status:?}");
            assert!(
                refusal.is_some_and(|reason| reason.contains("block")),
                "a drive whose plume would fire into the ship's own plating must \
                 be refused; the editor decided {refusal:?}"
            );
            assert!(
                status.iter().any(|line| line.contains("block")),
                "and the builder must be told so in words; the status read {status:?}"
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

/// The display name of any hull section in the catalog (the section the run places).
#[cfg(feature = "debug")]
fn hull_section_name(world: &World) -> Option<String> {
    world
        .resource::<GameSections>()
        .iter()
        .find(|section| matches!(section.kind, SectionKind::Hull(_)))
        .map(|section| section.base.name.clone())
}

/// Count the preview ship's sections.
///
/// `&World` for the same reason [`count_named_with_prefix`] is: the beats read
/// it and the predicates below wait on it, and one counter cannot disagree with
/// itself.
#[cfg(feature = "debug")]
fn count_sections(world: &World) -> usize {
    world
        .try_query_filtered::<(), With<SectionMarker>>()
        .map_or(0, |mut sections| sections.iter(world).count())
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

/// Advance once the preview ship carries exactly `count` sections.
#[cfg(feature = "debug")]
fn sections_number(count: usize) -> Wait {
    std::sync::Arc::new(move |world: &World| count_sections(world) == count)
}

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

/// Advance once the flown ship carries every section the editor built.
///
/// The hand-off, waited on rather than settled for: Play tears the preview down
/// and the scenario spawns the real ship a while later, and "a while" is a
/// property of the machine.
#[cfg(feature = "debug")]
fn the_flown_ship_is_whole() -> Wait {
    std::sync::Arc::new(|world: &World| {
        let built = world.resource::<EditorWalk>().sections;
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
    let mut q_sections = world.query_filtered::<&GlobalTransform, With<SectionMarker>>();
    // The LOWEST-posed section rather than the first one the query yields:
    // query order is an archetype detail that changes between runs, and a beat
    // that clicks a different section each time proves nothing (see
    // `placed_sections`).
    let section_pos = q_sections
        .iter(world)
        .map(GlobalTransform::translation)
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
        self.click_a_widget(
            &format!("{label}: open the gallery"),
            "Parts Gallery Category",
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
