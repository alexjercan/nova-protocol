//! The screenshot driver, driven through a real `App`, in its own test BINARY.
//!
//! Not lib tests: the lib-test binary's `autopilot` tests arm `NOVA_AUTOPILOT`
//! process-wide, and the screenshot driver stands down whenever that is set -
//! so every App-driven screenshot assertion would silently test an empty
//! plugin. Only the pure `parse_resolution` unit tests stay in the module.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Once,
    },
    time::Duration,
};

use bevy::{
    asset::RenderAssetUsages,
    image::Image,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    state::app::StatesPlugin,
    time::TimeUpdateStrategy,
    window::PrimaryWindow,
};
use nova_autopilot::{
    completion::{self, HarnessCompletion},
    screenshot::{ScreenshotPlugin, MAX_WAIT_FRAMES, SCREENSHOT_ENV},
};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum TestState {
    #[default]
    Boot,
    Playing,
}

/// Arm the driver for the whole test binary, with a `WxH` value so the env's
/// resolution override is exercised for real. Set once, never removed and
/// never with a second value, so the parallel test threads cannot observe
/// disagreeing environments. Tests that spawn no window are unaffected:
/// `resize_window` no-ops when there is none.
fn arm() {
    static ARM: Once = Once::new();
    ARM.call_once(|| std::env::set_var(SCREENSHOT_ENV, "390x844"));
}

/// Frames are 1/60s of MANUAL time, so the run does not depend on how fast the
/// test host is.
const FRAME: Duration = Duration::from_nanos(16_666_667);

/// Headless rig: minimal app, real state machine, deterministic clock. There is
/// no render app, so nothing triggers [`ScreenshotCaptured`] on its own - which
/// is what lets a test observe the capture spawn and then synthesize the
/// capture itself.
fn app(plugin: ScreenshotPlugin<TestState>) -> App {
    arm();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
    app.init_state::<TestState>();
    app.add_plugins(plugin);
    app
}

fn exits(app: &mut App) -> Vec<AppExit> {
    app.world_mut()
        .resource_mut::<Messages<AppExit>>()
        .drain()
        .collect()
}

/// Run `frames` frames, collecting every [`AppExit`] written along the way.
/// Drained per frame: `Messages` is double-buffered, so an exit written mid-run
/// is gone by the time a later frame reads it.
fn run(app: &mut App, frames: usize) -> Vec<AppExit> {
    let mut observed = Vec::new();
    for _ in 0..frames {
        app.update();
        observed.extend(exits(app));
    }
    observed
}

/// The entity the driver spawned its capture request on, if any.
fn capture_entity(app: &mut App) -> Option<Entity> {
    let mut query = app.world_mut().query_filtered::<Entity, With<Screenshot>>();
    query.iter(app.world()).next()
}

/// A unique temp path per test, so parallel runs never collide.
fn shot_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("nova_autopilot_{name}.png"))
}

/// A 1x1 white image: the smallest thing `save_to_disk` can round-trip through
/// `try_into_dynamic`.
fn tiny_image() -> Image {
    Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    )
}

#[test]
fn screenshot_env_resolution_pins_the_primary_window() {
    arm();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
    app.init_state::<TestState>();
    // Spawned BEFORE the plugin: `resize_window` is a `Startup` system, so the
    // window has to exist by the first frame.
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.add_plugins(ScreenshotPlugin::new(TestState::Playing));

    app.update();

    let mut windows = app.world_mut().query::<&Window>();
    let window = windows.iter(app.world()).next().expect("primary window");
    assert_eq!(
        (window.resolution.width(), window.resolution.height()),
        (390.0, 844.0),
        "the armed WxH value pins the primary window before capture"
    );
    assert!(
        !window.resizable,
        "an unpinned window lets a reflowing WM resize it, and the \
         responsive-layout capture then measures the wrong width"
    );
}

#[test]
fn screenshot_reports_done_after_settling() {
    let path = shot_path("settle");
    let _ = std::fs::remove_file(&path);
    let mut app = app(ScreenshotPlugin::new(TestState::Playing)
        .settle_frames(4)
        .path(path.to_str().expect("utf-8 temp path")));

    // Frame 1 advances, frame 2 sees the applied transition and counts the
    // first settled frame, so the capture spawns on frame 5.
    run(&mut app, 4);
    assert!(
        capture_entity(&mut app).is_none(),
        "no capture before the configured settled frames have passed"
    );

    run(&mut app, 1);
    let entity = capture_entity(&mut app).expect("the capture spawns on the settle frame");
    assert!(
        app.world()
            .resource::<HarnessCompletion>()
            .is_pending(completion::SCREENSHOT),
        "the driver holds its completion open until the capture lands"
    );

    // Stand in for the render app a headless test does not have: both observers
    // then run for real, `save_to_disk` first.
    app.world_mut().trigger(ScreenshotCaptured {
        entity,
        image: tiny_image(),
    });

    assert!(
        path.exists(),
        "the PNG is on disk once the capture observers have run"
    );
    assert!(
        !app.world()
            .resource::<HarnessCompletion>()
            .is_pending(completion::SCREENSHOT),
        "and only THEN does the run report done"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn unreached_target_state_error_exits() {
    let mut app = app(ScreenshotPlugin::new(TestState::Playing));
    // `PostUpdate`, so this write lands after the driver's `Update` one and is
    // the value the next frame's `StateTransition` applies: the target is
    // provably never reached, rather than racing the driver inside `Update`.
    app.add_systems(PostUpdate, |mut next: ResMut<NextState<TestState>>| {
        next.set(TestState::Boot)
    });

    let observed = run(&mut app, MAX_WAIT_FRAMES as usize + 2);

    assert_eq!(observed.len(), 1, "the give-up exits exactly once");
    assert_ne!(
        observed[0],
        AppExit::Success,
        "an unreachable target state ABORTS instead of hanging forever"
    );
    assert!(
        capture_entity(&mut app).is_none(),
        "and it captures nothing"
    );
}

#[test]
fn hide_overlay_hook_runs_before_the_capture() {
    // A process-static counter: the hook is a `Fn` closure, so it cannot own
    // state an assertion outside the app also has to read. Only this test
    // touches it.
    static RUNS: AtomicUsize = AtomicUsize::new(0);

    // No `.path()`: this test never triggers `ScreenshotCaptured`, so
    // `save_to_disk` never runs and no file is written under any path.
    let mut app = app(ScreenshotPlugin::new(TestState::Playing)
        .settle_frames(2)
        .hide_overlay(|_world| {
            RUNS.fetch_add(1, Ordering::SeqCst);
        }));

    run(&mut app, 1);
    let before_any_capture = RUNS.load(Ordering::SeqCst);

    run(&mut app, 5);
    assert!(
        capture_entity(&mut app).is_some(),
        "the run reached the capture, so the ordering claim is about a capture \
         that actually happened"
    );
    assert_eq!(
        before_any_capture, 1,
        "the hook runs at startup, before any capture could spawn"
    );
    assert_eq!(
        RUNS.load(Ordering::SeqCst),
        1,
        "and exactly once for the whole run"
    );
}
