//! screenshot_menu: the main menu over the ambience backdrop
//! (`tutorial-menu.png`), the Settings panel open over it
//! (`wiki-settings.png`) and its Controls tab (`wiki-controls.png`), driven
//! through the shipped app (`editor_app`).
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - walk the menu, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write each PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_menu --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_menu --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other menu walks. Script-only, so the
// whole module sits behind one gate here.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::Gestures;

/// The backdrop this walk shoots against.
const BACKDROP: &str = "menu_gauntlet";

/// The one ship the gauntlet is built around, and the subject of the frame
/// behind the menu.
#[cfg(feature = "debug")]
const BACKDROP_SHIP_ID: &str = "gauntlet_ship";

/// Where the camera stands relative to that gunship, in the HULL's own frame:
/// off the starboard bow (`-Z` is ahead of it), raised, about 170 m out.
///
/// The hull frame rather than world axes, because the gunship flies a patrol
/// and a world-axis offset photographs whatever end the current leg presents -
/// the first cut of this shot came out stern-on, a drive bell filling the
/// middle of the menu. Riding the hull always gives the same three-quarter
/// bow. A `block_gunship` is roughly 110 m stem to stern, and 170 m of
/// standoff makes a frame 250 m wide: the sections and their mounts stay
/// separable, with the rock band still around them.
#[cfg(feature = "debug")]
const BACKDROP_EYE: Meters3 = Meters3::new(130.0, 50.0, -95.0);

#[derive(Parser)]
#[command(name = "screenshot_menu")]
#[command(version = "1.0.0")]
#[command(about = "Capture the main menu and its Settings panel. Autopilot-only: a scripted pointer walk over the real menu", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // Pin the backdrop before the app boots. Menu entry draws one of the four
    // shipped backdrops at RANDOM, so an unpinned capture is a different scene
    // every run and the page's figure note can only describe whichever one it
    // drew.
    //
    // The gauntlet, because it is the backdrop built around ONE named ship:
    // a gunship holding a station against torpedo waves. The other three put
    // their traffic on a route or on a dogfight, so the hull in shot is
    // whichever one the act has carried into view.
    //
    // Set here rather than exported by the capture loop: the packager runs
    // every still producer through one generic command, so a producer that
    // needs an environment carries it itself.
    std::env::set_var(MENU_BACKDROP_ENV, BACKDROP);

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
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets on the menu
            // teardown) into panics so the run fails loudly on them.
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // Clean frames at a known 16:9 size: force the window resolution and drop
        // the dev overlays.
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(menu_script());
    }

    app.run()
}

/// The driven walk: menu -> Settings -> Controls, one shot per state.
#[cfg(feature = "debug")]
fn menu_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // The HUD chrome is dropped right before every shot rather than once at
    // `Startup`, because other states re-raise it. `shoot` itself is the capture
    // gate: unarmed, this whole walk runs and writes nothing.
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
        // The backdrop's own SetCamera is an ESTABLISHING shot - 2.6 km out,
        // where the gunship is a speck over a rock band. The menu is the
        // reader's first sight of the fleet, so move in on the hull.
        .step("frame the backdrop's gunship")
        .on_enter(frame_backdrop_ship)
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
        .add()
        .step("capture the settings panel")
        .on_enter(shot("wiki-settings.png"))
        .until(shot_written("wiki-settings.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The panel opens on Audio, so the tab the range exists for - the
        // rebindable keycap rows - is one click further in and needs its own
        // shot. The wiki's Controls section reads on that picture.
        .click("open the Controls tab", "Settings Tab: Controls")
        .step("settle the controls tab")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the controls tab")
        .on_enter(shot("wiki-controls.png"))
        .until(shot_written("wiki-controls.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("close Settings", "Settings Back Button")
        .click("start a new game", "New Game Button")
        .step("reach the first flight")
        .until(state_is(GameStates::Playing))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}

/// Bolt the backdrop camera to the gauntlet's gunship.
///
/// An anchor rather than a fixed pose: the gunship flies a patrol six
/// waypoints wide, so a pose solved at this instant is a picture of where the
/// ship USED to be by the time the PNG lands.
#[cfg(feature = "debug")]
fn frame_backdrop_ship(world: &mut World) {
    let ship = named_ship(world, BACKDROP_SHIP_ID).unwrap_or_else(|| {
        panic!("screenshot_menu: backdrop '{BACKDROP}' spawned no ship '{BACKDROP_SHIP_ID}'")
    });
    let camera = {
        let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
        query.iter(world).next()
    };
    let camera = camera.expect("screenshot_menu: the backdrop staged no scenario camera");
    world
        .entity_mut(camera)
        .remove::<WASDCameraController>()
        // The two overrides derive the same transform, so the backdrop's own
        // fixed pose has to go or it fights the anchor every frame.
        .remove::<ScriptedCameraPose>()
        .insert(ScriptedCameraAnchor {
            anchor: ship,
            offset: BACKDROP_EYE,
            frame: CameraOffsetFrame::Local,
            look_at: ScriptedCameraLookAt::Anchor,
        });
}

/// The spawned ship root carrying `id`.
#[cfg(feature = "debug")]
fn named_ship(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    query
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}
