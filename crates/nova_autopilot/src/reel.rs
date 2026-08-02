//! Multi-shot reel driver: step an ordered list of beats, capturing a PNG at
//! each one, then report done to the [`completion`] protocol.
//!
//! [`ScreenshotReelPlugin`] is the multi-shot sibling of
//! [`ScreenshotPlugin`](crate::screenshot::ScreenshotPlugin) (one shot, then
//! done): it owns the *cadence* - wait for the host app to be ready, apply the
//! beat, settle, capture, wait for the PNG to land, advance - while the scene
//! itself is the host app's business. Captures are serialized: a beat waits for
//! its PNG to be on disk before the next one starts, so a shot is never taken
//! mid-transition.
//!
//! It is inert unless the [`REEL_ENV`] environment variable is set. Relative
//! beat paths resolve under [`SHOT_DIR_ENV`], so a whole reel stages into one
//! folder; an absolute path is used as-is.
//!
//! ## The two hooks
//!
//! This crate knows nothing about the host app's scene, so everything
//! app-shaped is a caller-supplied closure:
//!
//! - [`ready`](ScreenshotReelPlugin::ready): a predicate gating the FIRST beat
//!   until the scene is live (assets loaded, camera spawned). Unset means
//!   immediately ready. It has no timeout of its own - a scene that never
//!   becomes ready is caught by the completion deadline, which error-exits
//!   naming `reel` as the laggard.
//! - [`apply`](ReelBeat::apply): per-beat setup with full world access (pose a
//!   camera, flip a UI state). It runs on beat entry, one frame before the
//!   settle count starts, so the change is applied before anything settles.
//!
//! [`hide_overlay`](ScreenshotReelPlugin::hide_overlay) is the same startup
//! hook [`ScreenshotPlugin`](crate::screenshot::ScreenshotPlugin) takes: clear
//! whatever dev chrome would otherwise be in frame.
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use nova_autopilot::reel::{ReelBeat, ScreenshotReelPlugin};
//!
//! # fn build(app: &mut App) {
//! app.add_plugins(
//!     ScreenshotReelPlugin::new(vec![
//!         ReelBeat::new("wide.png").apply(|_world| { /* pose the camera */ }),
//!         ReelBeat::new("close.png").settle_frames(60),
//!     ])
//!     .ready(|world| world.get_resource::<Time>().is_some()),
//! );
//! # }
//! ```

use std::{path::PathBuf, sync::Arc};

use bevy::{
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured},
    window::PrimaryWindow,
};

use crate::completion;

/// Environment variable that arms the reel driver. Distinct from
/// [`SCREENSHOT_ENV`](crate::screenshot::SCREENSHOT_ENV) (the single-shot
/// driver) so a reel run and a one-off capture never fight over the window.
pub const REEL_ENV: &str = "NOVA_REEL";

/// Environment variable naming the directory relative beat paths resolve
/// under. Unset (or empty), a relative path stays relative to the CWD.
pub const SHOT_DIR_ENV: &str = "NOVA_SHOT_DIR";

/// Default reel capture resolution: 1920x1080, the 16:9 the web figures and
/// thumbnails use (thumbnails share this capture and are sized down in CSS).
pub const REEL_CAPTURE_RESOLUTION: (f32, f32) = (1920.0, 1080.0);

/// Frames a beat settles before capturing when it does not override the count:
/// long enough for a camera move, a UI transition, and one-shot layout to
/// finish rendering.
const DEFAULT_SETTLE_FRAMES: u32 = 30;

/// Caller hook run with full world access: a per-beat `apply`, or the startup
/// overlay hide.
type WorldHookFn = dyn Fn(&mut World) + Send + Sync;

/// Caller predicate gating the first beat. `&World`, not `&mut World`: a
/// predicate that mutates is a beat, not a gate.
type ReadyFn = dyn Fn(&World) -> bool + Send + Sync;

/// One beat of a screenshot reel: run the [`apply`](Self::apply) hook, wait
/// [`settle_frames`](Self::settle_frames) for the scene to render, then capture
/// the primary window to `path`. Relative paths resolve under [`SHOT_DIR_ENV`],
/// so a whole reel stages into one folder.
#[derive(Clone)]
pub struct ReelBeat {
    /// Frames to let the scene render after applying before capturing.
    pub settle_frames: u32,
    /// Output PNG path (relative paths resolve under [`SHOT_DIR_ENV`]).
    pub path: String,
    apply: Option<Arc<WorldHookFn>>,
}

impl ReelBeat {
    /// A beat that settles the default 30 frames and captures to `path`, with
    /// no setup of its own (so it keeps the previous beat's framing).
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            settle_frames: DEFAULT_SETTLE_FRAMES,
            path: path.into(),
            apply: None,
        }
    }

    /// Set the per-beat setup hook: an exclusive system run once on beat entry,
    /// before the settle count starts. This crate cannot know how the host app
    /// frames a shot, so the caller supplies the closure.
    pub fn apply(mut self, f: impl Fn(&mut World) + Send + Sync + 'static) -> Self {
        self.apply = Some(Arc::new(f));
        self
    }

    /// Override the settle-frame count for this beat.
    pub fn settle_frames(mut self, frames: u32) -> Self {
        self.settle_frames = frames;
        self
    }
}

/// Env-gated plugin that walks an ordered list of [`ReelBeat`]s.
///
/// When [`REEL_ENV`] is set the plugin forces the primary window to the capture
/// resolution, waits for the [`ready`](Self::ready) predicate, then runs each
/// beat in turn and reports [`completion::REEL`] done once the last PNG has
/// landed. When the env var is unset - or the beat list is empty - it adds
/// nothing and registers nothing.
pub struct ScreenshotReelPlugin {
    beats: Vec<ReelBeat>,
    resolution: (f32, f32),
    ready: Option<Arc<ReadyFn>>,
    hide_overlay: Option<Arc<WorldHookFn>>,
}

impl ScreenshotReelPlugin {
    /// Build a reel from an ordered list of beats, capturing at the default
    /// [`REEL_CAPTURE_RESOLUTION`].
    pub fn new(beats: Vec<ReelBeat>) -> Self {
        Self {
            beats,
            resolution: REEL_CAPTURE_RESOLUTION,
            ready: None,
            hide_overlay: None,
        }
    }

    /// Override the capture resolution (the primary window is forced to this
    /// size at startup so every beat is captured at a known aspect).
    pub fn resolution(mut self, width: f32, height: f32) -> Self {
        self.resolution = (width, height);
        self
    }

    /// Set the readiness predicate gating the first beat: the reel idles until
    /// it returns `true`. A caller that omits it starts on the first frame.
    pub fn ready(mut self, f: impl Fn(&World) -> bool + Send + Sync + 'static) -> Self {
        self.ready = Some(Arc::new(f));
        self
    }

    /// Set the overlay-hide hook: an exclusive `Startup` system that clears
    /// whatever dev overlay (inspector window, gizmos, diagnostics UI) would
    /// otherwise be in frame. The same shape as
    /// [`ScreenshotPlugin::hide_overlay`](crate::screenshot::ScreenshotPlugin::hide_overlay).
    pub fn hide_overlay(mut self, f: impl Fn(&mut World) + Send + Sync + 'static) -> Self {
        self.hide_overlay = Some(Arc::new(f));
        self
    }
}

/// The forced capture resolution; drives the startup window resize.
#[derive(Resource)]
struct ReelWindowSize(f32, f32);

/// Internal driver state; kept out of the prelude per the crate conventions.
#[derive(Resource)]
struct ReelState {
    beats: Vec<ReelBeat>,
    index: usize,
    settled: u32,
    applied: bool,
    capturing: bool,
    done: bool,
}

/// Flipped by the per-capture `ScreenshotCaptured` observer so the driver knows
/// the PNG has landed and it is safe to advance (or report done).
#[derive(Resource, Default)]
struct ReelCaptureDone(bool);

impl Plugin for ScreenshotReelPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var(REEL_ENV).is_err() {
            return;
        }
        if self.beats.is_empty() {
            warn!("ScreenshotReelPlugin: {REEL_ENV} set but no beats; doing nothing");
            return;
        }
        debug!(
            "ScreenshotReelPlugin: build ({REEL_ENV} active, {} beats)",
            self.beats.len()
        );
        app.insert_resource(ReelState {
            beats: self.beats.clone(),
            index: 0,
            settled: 0,
            applied: false,
            capturing: false,
            done: false,
        });
        app.insert_resource(ReelWindowSize(self.resolution.0, self.resolution.1));
        app.init_resource::<ReelCaptureDone>();
        app.add_systems(Startup, reel_resize_window);
        if let Some(hide_overlay) = self.hide_overlay.clone() {
            app.add_systems(Startup, move |world: &mut World| hide_overlay(world));
        }
        let ready = self.ready.clone();
        app.add_systems(Update, move |world: &mut World| {
            reel_drive(world, ready.as_deref())
        });
        completion::register(app, completion::REEL);
    }
}

/// Force the primary window to the reel's capture resolution at startup, so
/// every beat lands at a known aspect (mirrors the single-shot driver's
/// resize). Non-resizable so a tiling WM cannot reflow it mid-reel.
fn reel_resize_window(
    size: Res<ReelWindowSize>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(size.0, size.1);
        window.resizable = false;
    }
}

/// Capture the primary window to `path` (relative paths resolve under
/// [`SHOT_DIR_ENV`]), creating the parent directory if needed. Public for
/// capture examples that script their own beats from an autopilot closure
/// rather than from a [`ReelBeat`] list, and so want the capture primitive
/// without the cadence.
pub fn capture_window(world: &mut World, path: &str) {
    let resolved = capture_path(path);
    create_capture_dir(&resolved);
    world
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(resolved));
}

/// Create the parent directory of a resolved capture path. A failure warns and
/// continues: the capture request is still worth making, and `save_to_disk`
/// reports the write failure itself.
fn create_capture_dir(resolved: &std::path::Path) {
    if let Some(parent) = resolved.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                warn!("reel: could not create capture dir {parent:?}: {error}");
            }
        }
    }
}

/// Resolve an output path under [`SHOT_DIR_ENV`]. Only RELATIVE paths are
/// joined - an absolute path passes through unchanged, so a caller naming an
/// exact file still gets it.
fn capture_path(path: &str) -> PathBuf {
    let path = std::path::Path::new(path);
    match std::env::var(SHOT_DIR_ENV) {
        Ok(dir) if !dir.is_empty() && !path.is_absolute() => std::path::Path::new(&dir).join(path),
        _ => path.to_path_buf(),
    }
}

/// Step the reel: ready -> apply -> settle -> capture -> (await the PNG) ->
/// advance -> report done. Exclusive because the `apply` hook and the capture
/// spawn both need `&mut World`.
fn reel_drive(world: &mut World, ready: Option<&ReadyFn>) {
    if world.resource::<ReelState>().done {
        return;
    }
    // Wait for the host app to say the scene is live before the first beat.
    if let Some(ready) = ready {
        if !ready(world) {
            return;
        }
    }

    world.resource_scope(|world, mut state: Mut<ReelState>| {
        let index = state.index;
        let beat = state.beats[index].clone();

        // Waiting on the previous capture to land before advancing.
        if state.capturing {
            if !world.resource::<ReelCaptureDone>().0 {
                return;
            }
            world.resource_mut::<ReelCaptureDone>().0 = false;
            state.capturing = false;
            state.applied = false;
            state.settled = 0;
            state.index += 1;
            if state.index >= state.beats.len() {
                info!("reel: {} beats captured, reporting done", state.beats.len());
                // NOTE: report done, never `AppExit::Success` - the completion
                // watcher owns the exit, so a slower collector is not cut off.
                world
                    .resource_mut::<completion::HarnessCompletion>()
                    .done(completion::REEL);
                state.done = true;
            }
            return;
        }

        // Apply on beat entry, then give the change a frame before settling.
        if !state.applied {
            if let Some(apply) = &beat.apply {
                apply(world);
            }
            state.applied = true;
            state.settled = 0;
            return;
        }

        // Let the scene render before capturing.
        state.settled += 1;
        if state.settled < beat.settle_frames {
            return;
        }

        let path = capture_path(&beat.path);
        create_capture_dir(&path);
        info!(
            "reel: beat {}/{} capturing -> {}",
            index + 1,
            state.beats.len(),
            path.display()
        );
        world
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path))
            .observe(
                |_: On<ScreenshotCaptured>, mut done: ResMut<ReelCaptureDone>| {
                    done.0 = true;
                },
            );
        state.capturing = true;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: only the pure path resolution is tested here. The App-driven tests
    // live in `tests/reel.rs`, because they must arm `NOVA_REEL` and pin
    // `NOVA_SHOT_DIR` process-wide, and `NOVA_SHOT_DIR` would then leak into
    // the other modules' unit tests in THIS binary.

    /// The shot-dir join itself is covered by `tests/reel.rs` (the beat PNGs
    /// land under the armed dir); pin the no-env and absolute cases here so the
    /// helper does not accidentally rewrite them.
    #[test]
    fn reel_capture_path_leaves_bare_and_absolute_paths_alone() {
        // Read, never set: mutating the env would race the other tests sharing
        // this binary's process.
        if std::env::var(SHOT_DIR_ENV).is_err() {
            assert_eq!(
                capture_path("feature-gravity.png"),
                PathBuf::from("feature-gravity.png"),
                "with no shot dir a relative path stays relative"
            );
        }
        assert_eq!(
            capture_path("/shots/a.png"),
            PathBuf::from("/shots/a.png"),
            "an absolute path passes through whatever the env says"
        );
    }
}
