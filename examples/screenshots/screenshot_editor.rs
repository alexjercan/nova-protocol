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
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other menu walks. Script-only, so the
// whole module sits behind one gate here.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::{
    count_sections, pose_editor_camera, the_build_camera_is_posed, the_editor_can_play,
    the_editor_is_inside_a_ship, the_skin_is_on, Gestures, STEP_DEADLINE_SECS,
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

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game/binary runs (main menu over the ambience backdrop).
    let mut app = editor_app(true, None);

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
        .click("create the ship", "Add Ship Button")
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
        // in the figure (review a4a6 R4).
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
    script
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
        .add()
        // Play compiles the document from the scenario node, so the walk
        // steps out of the ship through the tree's root row first: one click
        // on a row you are inside is the way back out.
        .click("leave the ship for the hand-off", "Scene Row scenario")
        .step("Play is reachable")
        .until(the_editor_can_play())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .click("launch the built ship", "Play Button")
        .step("reach the sandbox range")
        .until(and(player_ship_present(), frames(SETTLE_FRAMES * 3)))
        .deadline(STEP_DEADLINE_SECS)
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
