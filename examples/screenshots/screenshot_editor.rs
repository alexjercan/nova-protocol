//! screenshot_editor: the sandbox editor with a ship BUILT in it
//! (`feature-editor.png`), driven through the shipped app (`editor_app`).
//!
//! The ship is built by real pointer gestures over the real picking pipeline, so
//! a build that stopped placing sections fails the run instead of shooting an
//! empty editor.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - build the ship, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_editor --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_editor --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::PhysicsGizmos;
#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_debug::prelude::capturing;
#[cfg(feature = "debug")]
use nova_protocol::nova_debug::harness::Predicate;
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other menu walks. Script-only, so the
// whole module sits behind one gate here.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::{
    block_centre, block_named, canvas_middle, count_sections, pose_editor_camera,
    the_build_camera_is_posed, the_editor_can_play, the_editor_is_inside_a_ship, the_skin_is_on,
    Gestures,
};

#[derive(Parser)]
#[command(name = "screenshot_editor")]
#[command(version = "1.0.0")]
#[command(about = "Capture the sandbox editor with a ship built in it. Autopilot-only: a scripted pointer build over the real editor", long_about = None)]
struct Cli;

#[cfg(feature = "debug")]
const EDITOR_LOOP: &str = "landing-editor-build";
#[cfg(feature = "debug")]
const EDITOR_SKIN_LOOP: &str = "news-0110-editor-skin";
/// The script half: leaving the ship through the tree, switching to EVENTS,
/// and building a handler off the block palette. A still of the finished script
/// says what was authored; only a loop says that it was authored by pointing
/// at rows rather than by typing a file.
#[cfg(feature = "debug")]
const EDITOR_EVENTS_LOOP: &str = "news-0120-editor-events";

/// How long the sandbox range is given to come up and settle.
///
/// A frame-counted settle under a real-time deadline: the two only agree at an
/// assumed frame rate, and CI's software rasterizer is far under it. The
/// range - sixty-four scattered rocks, three pickets and a skybox - measured
/// 1.8 frames a second on lavapipe, where the editor scene before it managed
/// 3.8, so the settle needs about fifty seconds there and the shared
/// thirty-second deadline killed the walk mid-settle. A deadline is a HANG
/// detector, not a frame-rate gate, so it is sized to outlast the slowest
/// healthy settle; a run on a real GPU spends the frames in under two seconds
/// and never sees it.
#[cfg(feature = "debug")]
const SANDBOX_SETTLE_FRAMES: u32 = SETTLE_FRAMES * 3;
#[cfg(feature = "debug")]
const SANDBOX_DEADLINE_SECS: f32 = 90.0;

/// What the inspector's Id row is showing.
///
/// The claim the picker beat rests on: `click_named` warns and continues when a
/// widget is missing, so a walk that only PRESSED the option would pass with
/// nothing written.
#[cfg(feature = "debug")]
fn the_id_row_reads(id: &'static str) -> std::sync::Arc<Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query::<&Text>()
            .is_some_and(|mut texts| texts.iter(world).any(|text| text.0 == id))
            && !ui_node_present("Pick Body")(world)
    })
}

/// Chip `index` of the filter the walk just dropped.
#[cfg(feature = "debug")]
fn filter_chip(world: &World, index: usize) -> String {
    format!(
        "Chip {}/{} {index}",
        new_handler(world),
        selected_block(world)
    )
}

/// Press and release chip `index` of the filter the walk just dropped. A
/// chip opens on release, so a press alone leaves the list shut.
#[cfg(feature = "debug")]
trait BlockChipWalk {
    fn open_block_chip(self, label: &str, index: usize, landed: &'static str) -> Self;
}

#[cfg(feature = "debug")]
impl BlockChipWalk for nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    fn open_block_chip(self, label: &str, index: usize, landed: &'static str) -> Self {
        self.step(format!("{label} is drawn"))
            .until(std::sync::Arc::new(move |world: &World| {
                ui_node_present(filter_chip(world, index))(world)
            }))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("point at {label}"))
            .on_enter(move |world: &mut World| hover_named(filter_chip(world, index))(world))
            .until(frames(3))
            .add()
            .step(format!("press {label}"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(3))
            .add()
            .step(format!("release {label}"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(ui_node_present(landed))
            .diagnose(move |world: &World| {
                let chip = filter_chip(world, index);
                let names = world
                    .try_query::<&Name>()
                    .map(|mut names| {
                        names
                            .iter(world)
                            .filter(|name| name.starts_with("Pick") || name.contains(" Option "))
                            .map(|name| name.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                format!("{} {names:?}", pointer_hover_diagnosis(chip)(world))
            })
            .deadline(STEP_DEADLINE_SECS)
            .add()
    }
}

/// The handler the walk dropped, whose slots the later drags aim at.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct NewHandler(String);

#[cfg(feature = "debug")]
fn selected_block(world: &World) -> String {
    world
        .resource::<EditorProbe>()
        .shell
        .selected_block
        .clone()
        .expect("the dropped block is selected")
}

#[cfg(feature = "debug")]
fn new_handler(world: &World) -> String {
    world.resource::<NewHandler>().0.clone()
}

#[cfg(feature = "debug")]
fn undo_reads(label: &'static str) -> std::sync::Arc<Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| probe.next_undo.as_deref() == Some(label))
    })
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game/binary runs (main menu over the ambience backdrop).
    let mut app = editor_app(true, None);
    app.insert_resource(EditorOpensScratch);

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets on the menu/editor
            // teardown) into panics so the run fails loudly on them.
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // Clean frames at a known 16:9 size: force the window resolution and drop
        // the dev overlays. The HUD chrome is re-hidden right before the capture
        // (entering the editor re-raises it).
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(editor_script());
    }

    app.run()
}

#[cfg(feature = "debug")]
fn set_colliders(world: &mut World, enabled: bool) {
    world
        .resource_mut::<GizmoConfigStore>()
        .config_mut::<PhysicsGizmos>()
        .0
        .enabled = enabled;
}

/// The driven walk: menu -> editor -> build a ship -> shoot it.
#[cfg(feature = "debug")]
fn editor_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // The HUD is re-raised by entering the editor, so it is dropped again right
    // before the shot rather than once at `Startup`. `shoot` itself is the
    // capture gate: unarmed, this whole walk runs and writes nothing.
    let shot = |path: &'static str| {
        move |world: &mut World| {
            hide_hud(world);
            shoot(world, path);
        }
    };

    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The editor: reached the way a player reaches it. `click` waits for the
        // button to lay out, so the menu needs no settle of its own.
        .click("leave for the editor", "Sandbox Button")
        .step("reach the editor")
        .until(state_is(GameStates::Playing))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("pose the editor camera off the axis")
        .on_enter(pose_editor_camera)
        .until(the_build_camera_is_posed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Through the top bar, because that is where Add lives now: the row
        // does not exist until its menu is open.
        .click("open the Add menu", "Add Menu Button")
        .click("open Add > Ship", "Add Menu Ship")
        .click("create the ship", "Add New Ship")
        .step("the blank ship is entered")
        .until(the_editor_is_inside_a_ship())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // A blank ship is FOUNDED: arm a controller and click empty space,
        // which drops it at the ship's own origin. The first `place` beat
        // after this covers its collider - the editor cannot solve a mate
        // before avian has prepared the collider that aim's ray has to hit.
        .arm("arm the controller", "basic_controller_section")
        .found("found the ship")
        // Build it: arm a part through the gallery, then click the ship itself
        // through the real picking pipeline. A placed section mates the socket
        // nearest the pointer, so each beat below names the section it grows
        // from and the face it grows out of.
        .arm("arm the hull", "reinforced_hull_section")
        .place("hull ahead of the controller", Vec3::ZERO, Vec3::X)
        .place("hull ahead of that", Vec3::new(1.0, 0.0, 0.0), Vec3::X)
        .arm("arm the thruster", "basic_thruster_section")
        .place("thruster on the tail", Vec3::new(2.0, 0.0, 0.0), Vec3::X);

    if capturing() {
        script = script
            .step("open the editor build loop")
            .on_enter(|world| loop_start(world, EDITOR_LOOP))
            .add();
    }

    script = script
        .click("open the Ship tab", "Browser Tab Ship")
        .click("derive the ship skin", "Ship Skin Toggle")
        .step("let the skin close around the structure")
        .until(the_skin_is_on())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .arm("arm the turret", "pdc_kinetic_turret_section")
        .place("turret on the spine", Vec3::new(1.0, 0.0, 0.0), Vec3::Y)
        .step("the ship was built")
        .on_enter(|world: &mut World| {
            // The founding controller plus the four sections the gestures
            // placed. A short count means a click missed its face and the shot
            // shows a thinner ship than the figure claims.
            let sections = count_sections(world);
            assert_eq!(
                sections, 5,
                "the build gestures must leave a controller, two hulls, a \
                 thruster and a turret on the preview ship"
            );
            info!("editor build: {sections} sections on the preview ship");
        })
        .add()
        // Put the part down before the shot: a builder holding one sees every
        // free socket drawn on the ship, which is the right answer while
        // building and pure clutter in a figure of the finished thing. Escape
        // is the gesture now that select mode is the default rather than a
        // rail button. The tool actually disarming is its own claim:
        // `editor_placement_clear()` below is true either way - it holds when
        // nothing is armed OR when nothing is under the pointer - so a missed
        // Escape would sail through it the moment the pointer reached empty
        // space, with the part still in hand and its link-point clutter still
        // in the figure.
        .step("press Escape to put the part down")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        // Park the pointer clear of the ship: hovering a section raises the
        // placement GHOST, and a translucent extra section is exactly the thing
        // a reader would mistake for part of the built ship. The editor SAYS
        // when the ghost is away - with nothing under the pointer there is no
        // placement to solve.
        .step("park the pointer clear of the ship")
        .on_enter(|world: &mut World| move_cursor(Vec2::new(1400.0, 960.0))(world))
        .until(editor_placement_clear())
        .deadline(STEP_DEADLINE_SECS)
        .add();

    if capturing() {
        script = script
            .step("hold the completed build")
            .until(elapsed(0.8))
            .add()
            .step("close the editor build loop")
            .on_enter(|world| loop_end(world, EDITOR_LOOP))
            .until(loop_written(EDITOR_LOOP))
            .deadline(60.0)
            .add();
    }

    if capturing() {
        script = script
            .click("hold the Ship tab open", "Browser Tab Ship")
            .step("open the editor skin loop")
            .on_enter(|world| loop_start(world, EDITOR_SKIN_LOOP))
            .add()
            .click("remove the derived skin", "Ship Skin Toggle")
            .step("hold the bare structure")
            .until(elapsed(0.8))
            .add()
            .click("restore the derived skin", "Ship Skin Toggle")
            .step("let the skin close")
            .until(elapsed(0.8))
            .add()
            .click("select the civilian style", "Style: Civilian")
            .step("hold the civilian style")
            .until(elapsed(0.8))
            .add()
            .click("select the armoured style", "Style: Armoured")
            .step("hold the armoured style")
            .until(elapsed(0.8))
            .add()
            .click("select the salvage style", "Style: Salvage")
            .step("hold the salvage style")
            .until(elapsed(0.8))
            .add()
            .click("restore the industrial style", "Style: Industrial")
            .step("hold the industrial style")
            .until(elapsed(0.8))
            .add()
            .step("close the editor skin loop")
            .on_enter(|world| loop_end(world, EDITOR_SKIN_LOOP))
            .until(loop_written(EDITOR_SKIN_LOOP))
            .deadline(60.0)
            .add();
    }

    // The last steps prove the finished build can leave the editor and capture
    // the ordinary free-flight range that players receive from Play.
    let mut script = script
        .step("capture the editor with the built ship")
        .on_enter(shot("feature-editor.png"))
        .until(shot_written("feature-editor.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("capture the clean collider comparison half")
        .on_enter(shot("news-0110-collider-before.png"))
        .until(shot_written("news-0110-collider-before.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("show the authored colliders")
        .on_enter(|world| set_colliders(world, true))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the diagnostic collider half")
        .on_enter(shot("news-0110-collider-after.png"))
        .until(shot_written("news-0110-collider-after.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("hide the collider diagnostic")
        .on_enter(|world| set_colliders(world, false))
        .add();

    if capturing() {
        script = script
            .step("open the editor events loop")
            .on_enter(|world| loop_start(world, EDITOR_EVENTS_LOOP))
            .add();
    }

    script = script
        // Play compiles the document from the scenario node, so the walk
        // steps out of the ship through the tree's root row first: one click
        // on a row you are inside is the way back out.
        .click("leave the ship for the hand-off", "Back Button")
        // The script half of the document, authored the way a builder does it:
        // blocks off the palette, and the ship named through the picker rather
        // than typed. The walk asserts the picked id landed, because a chip
        // that opens a list and writes nothing is the failure this whole path
        // is built to make impossible.
        .click("show the events half of the tree", "View Tab Events")
        .filter_palette("On Enter")
        .drag_block(
            "add a handler",
            block_named("Palette On Enter"),
            canvas_middle(),
            |held| held,
            undo_reads("Add On Enter"),
        )
        .step("keep the handler's id")
        .on_enter(|world: &mut World| {
            let hat = selected_block(world);
            world.insert_resource(NewHandler(hat));
        })
        .add()
        .filter_palette("Entity")
        .drag_block(
            "scope the handler to one ship",
            block_named("Palette Entity"),
            block_centre(|world| format!("Slot {} filter 0", new_handler(world))),
            |held| held,
            undo_reads("Add Entity"),
        )
        .open_block_chip("the id list", 1, "Ref Option player_spaceship")
        // The list holds more ids than it shows; typing brings the row up.
        .step("type the ship's id into the list")
        .on_enter(type_text("player_spaceship"))
        .until(frames(6))
        .add()
        .click(
            "name the ship the document holds",
            "Ref Option player_spaceship",
        )
        .step("the id row took the ship that was picked")
        .until(the_id_row_reads("player_spaceship"))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .filter_palette("Debug Message")
        .drag_block(
            "give the handler something to do",
            block_named("Palette Debug Message"),
            block_centre(|world| format!("Slot {} 0", new_handler(world))),
            |held| held,
            undo_reads("Add Debug Message"),
        )
        // A CONDITION, last, so the capture is of the condition being authored.
        // It arrives holding `0 == 0`, and the comparison is then changed in
        // its own list on the block.
        .filter_palette("==")
        .drag_block(
            "add the filter that compares",
            block_named("Palette =="),
            block_centre(|world| format!("Slot {} filter 1", new_handler(world))),
            |held| held,
            undo_reads("Add =="),
        )
        .open_block_chip("the comparison list", 2, "Choice Option >")
        .click("compare the other way", "Choice Option >");

    if capturing() {
        script = script
            .step("close the editor events loop")
            .on_enter(|world| loop_end(world, EDITOR_EVENTS_LOOP))
            .until(loop_written(EDITOR_EVENTS_LOOP))
            .deadline(60.0)
            .add();
    }

    script
        .step("capture the authored script")
        .on_enter(shot("feature-editor-events.png"))
        .until(shot_written("feature-editor-events.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("Play is reachable")
        .until(the_editor_can_play())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .click("launch the built ship", "Play Button")
        .step("reach the sandbox range")
        .until(and(player_ship_present(), frames(SANDBOX_SETTLE_FRAMES)))
        .deadline(SANDBOX_DEADLINE_SECS)
        .add()
        // Scenario entry synchronizes the debug layer once more. Disable the
        // diagnostic after that synchronization so this player-facing shot is
        // the ordinary sandbox view.
        .step("hide sandbox collider diagnostics")
        .on_enter(|world| set_colliders(world, false))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the sandbox range")
        .on_enter(|world| shoot(world, "wiki-sandbox-range.png"))
        .until(shot_written("wiki-sandbox-range.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
