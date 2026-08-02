//! The reel driver, driven through a real `App`, in its own test BINARY.
//!
//! Not lib tests: the reel must arm `NOVA_REEL` and pin `NOVA_SHOT_DIR`
//! process-wide for the whole binary, and `NOVA_SHOT_DIR` would then leak into
//! the `screenshot` and `reel` modules' unit tests. Only the pure
//! `capture_path` case stays in the module.

use std::{
    path::PathBuf,
    sync::{Mutex, Once},
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
    time::TimeUpdateStrategy,
};
use nova_autopilot::{
    completion::{self, HarnessCompletion},
    reel::{ReelBeat, ScreenshotReelPlugin, REEL_ENV, SHOT_DIR_ENV},
};

/// Where this binary's beats stage their PNGs. Under the system temp dir so a
/// parallel run of another binary cannot collide.
fn shot_dir() -> PathBuf {
    std::env::temp_dir().join("nova_autopilot_reel_tests")
}

/// Arm the reel for the whole test binary and pin the shot dir. Set once, never
/// removed and never with a second value, so the parallel test threads cannot
/// observe disagreeing environments.
fn arm() {
    static ARM: Once = Once::new();
    ARM.call_once(|| {
        std::env::set_var(REEL_ENV, "1");
        std::env::set_var(SHOT_DIR_ENV, shot_dir());
    });
}

/// Frames are 1/60s of MANUAL time, so the run does not depend on how fast the
/// test host is.
const FRAME: Duration = Duration::from_nanos(16_666_667);

/// Headless rig: minimal app, deterministic clock, no render app - so nothing
/// triggers [`ScreenshotCaptured`] on its own, which is what lets a test
/// observe the capture spawn and then synthesize the capture itself. No
/// `StatesPlugin`: unlike the single-shot driver the reel is not state-generic.
fn app(plugin: ScreenshotReelPlugin) -> App {
    arm();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
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

/// Every outstanding capture request in the world.
fn capture_entities(app: &mut App) -> Vec<Entity> {
    let mut query = app.world_mut().query_filtered::<Entity, With<Screenshot>>();
    query.iter(app.world()).collect()
}

/// Stand in for the render app a headless test does not have: trigger the
/// capture observers (so `save_to_disk` writes the PNG and the driver learns it
/// landed) and despawn the request, which the render app does for real once it
/// has served it.
fn land_capture(app: &mut App, entity: Entity) {
    app.world_mut().trigger(ScreenshotCaptured {
        entity,
        image: tiny_image(),
    });
    app.world_mut().entity_mut(entity).despawn();
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
fn reel_beats_are_serialized_on_capture() {
    // A process-static log: the `apply` hooks are `Fn` closures, so they cannot
    // own state an assertion outside the app also reads. Only this test touches
    // it.
    static APPLIED: Mutex<Vec<usize>> = Mutex::new(Vec::new());
    fn applied() -> Vec<usize> {
        APPLIED.lock().expect("apply log").clone()
    }

    let names = ["serial-0.png", "serial-1.png", "serial-2.png"];
    for name in names {
        let _ = std::fs::remove_file(shot_dir().join(name));
    }
    let beats = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            ReelBeat::new(*name)
                .settle_frames(2)
                .apply(move |_world| APPLIED.lock().expect("apply log").push(index))
        })
        .collect();
    let mut app = app(ScreenshotReelPlugin::new(beats));

    for beat in 0..names.len() {
        if beat > 0 {
            // The frame after a capture lands only advances the index; the
            // next beat enters on the frame after that.
            run(&mut app, 1);
            assert_eq!(
                applied().len(),
                beat,
                "the advance frame does not apply beat {beat} yet"
            );
        }
        // Frame 1 applies the beat, then `settle_frames` frames pass before the
        // capture spawns.
        run(&mut app, 1);
        assert_eq!(
            applied().len(),
            beat + 1,
            "beat {beat} applies on entry, and no later beat has applied yet"
        );
        run(&mut app, 1);
        assert!(
            capture_entities(&mut app).is_empty(),
            "beat {beat} captures nothing before its settle frames have passed"
        );

        run(&mut app, 1);
        let captures = capture_entities(&mut app);
        assert_eq!(
            captures.len(),
            1,
            "exactly one capture request is outstanding at a time"
        );

        // Several frames with the PNG still in flight: the reel must not apply
        // the next beat or spawn a second capture until this one lands.
        run(&mut app, 5);
        assert_eq!(
            capture_entities(&mut app).len(),
            1,
            "no second capture spawns while the first is still in flight"
        );
        assert_eq!(
            applied().len(),
            beat + 1,
            "and the next beat does not apply either"
        );

        land_capture(&mut app, captures[0]);
    }

    // The advance off the last landed capture reports done.
    run(&mut app, 1);
    assert_eq!(
        applied(),
        vec![0, 1, 2],
        "the beats ran in list order, exactly once each"
    );
    for name in names {
        let path = shot_dir().join(name);
        assert!(
            path.exists(),
            "beat PNG {name} staged under the armed shot dir: {}",
            path.display()
        );
        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn reel_waits_for_the_scene_to_be_ready() {
    static APPLIED: Mutex<usize> = Mutex::new(0);

    #[derive(Resource)]
    struct SceneReady(bool);

    let mut app = app(
        ScreenshotReelPlugin::new(vec![ReelBeat::new("waits-for-ready.png")
            .settle_frames(2)
            .apply(|_world| *APPLIED.lock().expect("apply count") += 1)])
        .ready(|world| world.resource::<SceneReady>().0),
    );
    app.insert_resource(SceneReady(false));

    // Well past this beat's apply frame plus its settle frames.
    run(&mut app, 30);
    assert_eq!(
        *APPLIED.lock().expect("apply count"),
        0,
        "an unready scene does not enter the first beat"
    );
    assert!(
        capture_entities(&mut app).is_empty(),
        "and captures nothing, however many frames pass"
    );
    assert!(
        app.world()
            .resource::<HarnessCompletion>()
            .is_pending(completion::REEL),
        "the reel holds its completion open while it waits"
    );

    app.world_mut().resource_mut::<SceneReady>().0 = true;
    run(&mut app, 1);
    assert_eq!(
        *APPLIED.lock().expect("apply count"),
        1,
        "the first beat enters as soon as the scene reports ready"
    );
    run(&mut app, 2);
    assert_eq!(
        capture_entities(&mut app).len(),
        1,
        "and settles and captures from there"
    );

    let _ = std::fs::remove_file(shot_dir().join("waits-for-ready.png"));
}

#[test]
fn reel_negotiates_completion() {
    let path = shot_dir().join("negotiates.png");
    let _ = std::fs::remove_file(&path);
    let mut app = app(ScreenshotReelPlugin::new(vec![ReelBeat::new(
        "negotiates.png",
    )
    .settle_frames(2)]));
    // A second collector, still pending when the reel finishes: it is what
    // proves the reel reported done rather than exiting on its own clock.
    completion::register(&mut app, "slower");

    // Frame 1 enters the beat, frames 2-3 settle and capture.
    let observed = run(&mut app, 3);
    assert!(
        observed.is_empty(),
        "no exit while the reel is still running"
    );
    let captures = capture_entities(&mut app);
    assert_eq!(captures.len(), 1, "the beat spawned its capture");
    assert!(
        app.world()
            .resource::<HarnessCompletion>()
            .is_pending(completion::REEL),
        "the reel stays pending until its PNG has landed"
    );

    land_capture(&mut app, captures[0]);
    let observed = run(&mut app, 2);
    assert!(
        !app.world()
            .resource::<HarnessCompletion>()
            .is_pending(completion::REEL),
        "the landed capture reports the reel done"
    );
    assert!(
        observed.is_empty(),
        "and the reel writes NO exit itself: `slower` is still pending, so a \
         self-written AppExit::Success would cut it off"
    );

    app.world_mut()
        .resource_mut::<HarnessCompletion>()
        .done("slower");
    let observed = run(&mut app, 1);
    assert_eq!(
        observed,
        vec![AppExit::Success],
        "the watcher exits once the LAST collector reports done"
    );
    assert!(path.exists(), "and the beat's PNG is on disk");
    let _ = std::fs::remove_file(&path);
}
