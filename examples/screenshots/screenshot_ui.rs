//! screenshot_ui: capture the UI/state-dependent web screenshots that a
//! pure scene capture cannot make - the menu, its two panels and the editor -
//! by driving the shipped app (`editor_app`) through an autopilot script.
//!
//! The pure-3D scene shots live in `screenshot_scene`; this example covers the
//! shots that need a real game state and UI up:
//!
//! | shot                             | state                                  |
//! |----------------------------------|----------------------------------------|
//! | `tutorial-menu.png`              | the main menu over the ambience backdrop |
//! | `wiki-settings.png`              | the Settings panel open over the menu   |
//! | `news-090-scenario-campaigns.png` | the Scenarios picker, a campaign chapter selected |
//! | `feature-editor.png`             | the sandbox editor with a ship BUILT in it |
//!
//! Every gesture is a real pointer gesture, the way `examples/ui/menu_scenarios.rs`
//! does it: the pointer is moved to the widget's own screen position (resolved
//! from its `Name`) and pressed and released there. Nothing is reached by
//! triggering its observer, so a panel that stopped opening on a click fails the
//! run instead of quietly producing the previous shot again. Widgets act on
//! `Activate`, which fires on RELEASE over the same node, so every click here
//! spans two beats.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole walk, reach
//!   Playing, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture each beat's PNG (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_ui --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_ui --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::widget::Selected;

#[derive(Parser)]
#[command(name = "screenshot_ui")]
#[command(version = "1.0.0")]
#[command(about = "Capture the menu/editor web screenshots via the shipped app", long_about = None)]
struct Cli;

/// Seconds a step may sit before it is called a stall. Sized with headroom for a
/// slow software-rendered CI GPU (llvmpipe), where every beat costs more
/// wall-clock than on a real GPU. An expiry is an error exit naming the step,
/// which is the point: a walk that never reaches the menu or the editor now
/// fails loudly instead of idling out a fixed window.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// The campaign chapter the Scenarios shot selects.
///
/// The picker's own subject in that figure is the CAMPAIGN grouping - the `[-]`
/// header with its chapters indented under it - so the selection has to be a
/// chapter rather than one of the uncampaigned scenarios in the tail, and a
/// chapter partway down rather than the first, so the header above it is
/// visibly its parent. `assets/base/campaigns/nova_protocol.content.ron` lists
/// it; campaigns render expanded unless collapsed, so nothing has to open it.
#[cfg(feature = "debug")]
const CAMPAIGN_CHAPTER_ROW: &str = "Scenario Row: broadside";

/// Where the editor camera is put before the ship is built.
///
/// The editor's own camera sits at `(0, 5, 10)` looking down the -Z axis, dead
/// on: from there every SIDE face of a section is exactly edge-on, so the
/// picking ray can only ever hit a top or a front face and a ship can only be
/// built as a stack toward the lens. Off the axis, the +X, +Y and +Z faces are
/// all hittable, which is what lets the build below lay out a spine.
#[cfg(feature = "debug")]
const EDITOR_EYE: Vec3 = Vec3::new(4.44, 3.17, 7.25);

/// What the editor camera looks at.
///
/// NOT the built ship's centre: the rail and the component drawer own the left
/// 430 px of the frame, so a ship centred in the WINDOW sits left of centre in
/// the picture. The look point is pushed a little over a unit along the camera's
/// screen-left, which slides the ship the same distance right into the free part
/// of the frame.
#[cfg(feature = "debug")]
const EDITOR_LOOK: Vec3 = Vec3::new(0.62, 0.25, 0.5);

/// Frames a pointer gesture on the SHIP gets: the picking backend needs a
/// frame to raycast the new pointer position and the editor's observers a
/// frame to react. Never shorter than the walk's other settles - a click that
/// lands before the raycast has moved places nothing, and this is the one part
/// of the walk that silently produces a thinner ship rather than failing.
#[cfg(feature = "debug")]
const GESTURE_FRAMES: u32 = 12;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game/binary runs (main menu over the ambience backdrop).
    let mut app = editor_app(true);

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PERF_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets on the menu/editor
            // teardown) into panics so the run fails loudly on them (as 12 does).
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // Clean frames at a known 16:9 size: force the window resolution and drop
        // the dev overlays. The HUD chrome is re-hidden right before each capture
        // (entering the editor re-raises it).
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(ui_script());
    }

    app.run()
}

/// The whole driven walk: menu -> Settings -> Scenarios -> editor, capturing one
/// shot per state.
#[cfg(feature = "debug")]
fn ui_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // The HUD is re-raised by entering the editor, so it is dropped again right
    // before every shot rather than once at `Startup`. `shoot` itself is the
    // capture gate: unarmed, this whole walk runs and writes nothing.
    let shot = |path: &'static str| {
        move |world: &mut World| {
            hide_hud(world);
            shoot(world, path);
        }
    };

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the menu and its ambience backdrop")
        .until(frames(SETTLE_FRAMES))
        .add()
        // Hide the HUD first, and let the PNG land BEFORE navigating away:
        // clicking on in the same frame captured a black mid-teardown frame.
        .step("capture the main menu")
        .on_enter(shot("tutorial-menu.png"))
        .until(shot_written("tutorial-menu.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The Settings panel: an overlay over the menu, not its own state.
        .click("open Settings", "Settings Button")
        .step("settle the settings panel")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The panel is toggled by Visibility alone, so this has to assert
        // VISIBLE, not merely laid out: the shot is otherwise the main menu
        // again under a different name.
        .step("the settings panel is up")
        .on_enter(assert_named_visible("Settings Panel"))
        .until(frames(1))
        .add()
        .step("capture the settings panel")
        .on_enter(shot("wiki-settings.png"))
        .until(shot_written("wiki-settings.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("close Settings", "Settings Back Button")
        // The Scenarios picker: the campaign header and its indented chapters.
        .click("open Scenarios", "Scenarios Button")
        .step("settle the scenarios picker")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("select a campaign chapter", CAMPAIGN_CHAPTER_ROW)
        .step("settle the selected chapter's details pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the chapter is the selected row")
        .on_enter(|world: &mut World| {
            // The picker's OWN record of which click landed
            // (`select_scenario_row` inserts `Selected` on the clicked row and
            // removes it from every other), not a restatement of what the beat
            // intended. A missed click leaves the details pane on whatever was
            // selected before, and the shot would still look plausible.
            let selected = world
                .query_filtered::<&Name, With<Selected>>()
                .iter(world)
                .any(|name| name.as_str() == CAMPAIGN_CHAPTER_ROW);
            assert!(
                selected,
                "the click on `{CAMPAIGN_CHAPTER_ROW}` never landed: the picker \
                 has not marked that row Selected, so the details pane shows the \
                 PREVIOUS selection"
            );
        })
        .until(frames(1))
        .add()
        .step("capture the scenarios picker")
        .on_enter(shot("news-090-scenario-campaigns.png"))
        .until(shot_written("news-090-scenario-campaigns.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("leave the picker", "Scenarios Back Button")
        // The editor: reached the way a player reaches it.
        .click("leave for the editor", "Sandbox Button")
        .step("reach the editor")
        .until(and(state_is(GameStates::Playing), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("pose the editor camera off the axis")
        .on_enter(pose_editor_camera)
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("create the ship", "Create New Spaceship Button V2")
        .step("settle the new ship and its colliders")
        .until(frames(SETTLE_FRAMES))
        .add()
        // Build it: arm a card, then click the ship itself through the real
        // picking pipeline. A placed section lands at the hit section's position
        // plus the hit FACE normal, so each beat below names the section it
        // grows from and the face it grows out of.
        .click("arm the hull", "Reinforced Hull Section")
        .place("hull ahead of the controller", Vec3::ZERO, Vec3::X)
        .place("hull ahead of that", Vec3::new(1.0, 0.0, 0.0), Vec3::X)
        .click("arm the thruster", "Basic Thruster Section")
        .place("thruster on the tail", Vec3::new(2.0, 0.0, 0.0), Vec3::X)
        .click("arm the turret", "Better Turret Section")
        .place("turret on the spine", Vec3::new(1.0, 0.0, 0.0), Vec3::Y)
        .step("the ship was built")
        .on_enter(|world: &mut World| {
            // The controller the New Ship button spawns, plus the four sections
            // the gestures placed. A short count means a click missed its face
            // and the shot shows a thinner ship than the figure claims.
            let sections = count_sections(world);
            assert_eq!(
                sections, 5,
                "the build gestures must leave a controller, two hulls, a \
                 thruster and a turret on the preview ship"
            );
            info!("editor build: {sections} sections on the preview ship");
        })
        .until(frames(1))
        .add()
        // Park the pointer clear of the ship: hovering a section raises the
        // placement GHOST, and a translucent extra section is exactly the thing
        // a reader would mistake for part of the built ship.
        .step("park the pointer clear of the ship")
        .on_enter(|world: &mut World| move_cursor(Vec2::new(1720.0, 960.0))(world))
        .until(frames(SETTLE_FRAMES))
        .add()
        // The last step holds until the PNG is on disk, so the driver cannot
        // report done out from under a pending write.
        .step("capture the editor with the built ship")
        .on_enter(shot("feature-editor.png"))
        .until(shot_written("feature-editor.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// Put the editor's camera on [`EDITOR_EYE`] and PIN it there.
///
/// Pinned with a `ScriptedCameraPose` - the same component the scenario
/// `SetCamera` action and the scene captures use - rather than by writing the
/// Transform once. The editor camera is a free-fly WASD camera, and that
/// controller's state machine rewrites the Transform every frame whether or not
/// any key is down (removing the component does not stop it: its private state
/// survives), so a one-shot set is gone by the next frame. It was, and every
/// section the build placed landed on the face the ORIGINAL pose saw.
#[cfg(feature = "debug")]
fn pose_editor_camera(world: &mut World) {
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the editor is up, so it has a 3D camera");
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: EDITOR_EYE,
        look_at: EDITOR_LOOK,
    });
}

/// Count the preview ship's sections.
#[cfg(feature = "debug")]
fn count_sections(world: &mut World) -> usize {
    let mut q = world.query_filtered::<(), With<SectionMarker>>();
    q.iter(world).count()
}

/// Where to point the pointer to hit the `face` face of the section mounted at
/// `section`, in logical pixels.
///
/// Aims just INSIDE the face (sections are one unit apart, so a face sits half a
/// unit out): a ray to a point on the face plane itself grazes it, and the
/// editor places nothing without a hit normal. `world_to_viewport` answers in
/// logical pixels, which is the space [`move_cursor`] takes.
#[cfg(feature = "debug")]
fn aim_at_face(world: &mut World, section: Vec3, face: Vec3) -> Vec2 {
    let target = section + face * 0.49;
    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the editor is up, so it has a 3D camera");
    let camera = world
        .get::<Camera>(camera_entity)
        .expect("a 3D camera has a Camera");
    let camera_transform = world
        .get::<GlobalTransform>(camera_entity)
        .expect("a camera has a global transform");
    camera
        .world_to_viewport(camera_transform, target)
        .unwrap_or_else(|err| panic!("the point {target} must be on screen for the shot: {err}"))
}

/// The two gesture shapes the walk is written in: a click on a WIDGET, and a
/// click on the SHIP.
///
/// An extension trait rather than free functions so a gesture reads in the
/// script as one line, the way `examples/ui/editor.rs` writes its ship clicks -
/// a press and a release spelled out at every call site buried what the walk
/// actually does.
#[cfg(feature = "debug")]
trait Gestures {
    /// Press and release the pointer over the named widget. Widgets act on
    /// `Activate`, which fires on RELEASE over the same node.
    fn click(self, label: &str, name: &str) -> Self;

    /// Place a section on the ship: aim at the `face` face of the section
    /// mounted at `on`, press, release. The editor acts on `Pointer<Press>`, so
    /// the press does the work and the release only lets go.
    fn place(self, label: &str, on: Vec3, face: Vec3) -> Self;
}

#[cfg(feature = "debug")]
impl Gestures for nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    fn click(self, label: &str, name: &str) -> Self {
        self.step(format!("{label}: press"))
            .on_enter(click_named(name.to_string()))
            .until(frames(GESTURE_FRAMES))
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(GESTURE_FRAMES))
            .add()
    }

    fn place(self, label: &str, on: Vec3, face: Vec3) -> Self {
        self.step(format!("{label}: aim"))
            .on_enter(move |world: &mut World| {
                let at = aim_at_face(world, on, face);
                move_cursor(at)(world);
            })
            .until(frames(GESTURE_FRAMES))
            .add()
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(GESTURE_FRAMES))
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(GESTURE_FRAMES))
            .add()
            // Per-gesture, not just the total at the end: a short final count
            // says the build missed, this says WHICH gesture missed.
            .step(format!("{label}: count"))
            .on_enter(|world: &mut World| {
                let sections = count_sections(world);
                info!("editor build: {sections} sections");
            })
            .until(frames(1))
            .add()
    }
}
