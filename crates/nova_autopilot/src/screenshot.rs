//! Viewport-capture driver: force a window size, reach a state, snap a PNG.
//!
//! [`ScreenshotPlugin`] is an env-gated sibling of
//! [`AutopilotPlugin`](crate::autopilot::AutopilotPlugin) that shares the same
//! state-driver idea: it advances the game to a named state, waits for it to
//! settle, then captures the primary window to a file and reports done to the
//! [`completion`] protocol. It makes the "seeing the screen" workflow
//! reusable -- it caught a real responsive-layout regression (shop buttons
//! below the fold at phone width) that every non-visual check missed.
//!
//! It is inert unless the [`SCREENSHOT_ENV`] environment variable is set. A
//! value of the form `WxH` (for example `390x844`) overrides the window
//! resolution before the capture, which is exactly the phone-width check that
//! found the regression:
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use nova_autopilot::screenshot::ScreenshotPlugin;
//!
//! #[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
//! enum GameState {
//!     #[default]
//!     Menu,
//!     Playing,
//!     GameOver,
//! }
//!
//! # fn build(app: &mut App) {
//! app.add_plugins(
//!     ScreenshotPlugin::new(GameState::Playing)
//!         .settle_frames(12)
//!         .path("shot.png"),
//! );
//! # }
//! ```
//!
//! Then `NOVA_SHOT=390x844 cargo run --example my_game` writes `shot.png` at
//! phone width and exits.
//!
//! Hiding a dev overlay before the capture is the host app's business, not
//! this crate's: pass a closure to
//! [`hide_overlay`](ScreenshotPlugin::hide_overlay) and it runs once at
//! startup, before any capture, with full world access.

use std::sync::Arc;

use bevy::{
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured},
    state::state::FreelyMutableState,
    window::PrimaryWindow,
};

use crate::{autopilot::AUTOPILOT_ENV, completion};

/// Environment variable that arms the single-shot screenshot driver. Unset,
/// the plugin adds nothing at all. A `WxH` value also overrides the window
/// resolution; any other value is a plain toggle.
pub const SCREENSHOT_ENV: &str = "NOVA_SHOT";

/// Safety bound: if the target state is not reached within this many frames the
/// driver exits with an error instead of hanging forever (for example if the
/// game keeps overriding `NextState` back to a menu). At 60 fps this is ~30s.
pub const MAX_WAIT_FRAMES: u32 = 1800;

/// Caller hook run once at startup with full world access, before any capture.
type OverlayHideFn = dyn Fn(&mut World) + Send + Sync;

/// Env-gated plugin that advances to a state, settles, and captures a PNG.
///
/// Build it with the target [`State`] and, optionally, a settle-frame count,
/// output path, default resolution, and overlay-hide hook. When
/// [`SCREENSHOT_ENV`] is set the plugin (optionally) resizes the primary
/// window, sets `NextState` to the target, waits
/// [`settle_frames`](Self::settle_frames) frames after the state is entered,
/// spawns a [`Screenshot`] with [`save_to_disk`], and reports done to the
/// [`completion`] protocol once the capture lands on disk. When the env var is
/// unset it adds nothing.
///
/// Distinct from Bevy's internal `bevy::render::view::screenshot::ScreenshotPlugin`
/// (the render-side plugin that services capture requests); this is the
/// headless-verification driver that drives one. Neither is in `bevy::prelude`,
/// so there is no name collision, but do not confuse the two.
pub struct ScreenshotPlugin<S: States + FreelyMutableState> {
    state: S,
    settle_frames: u32,
    path: String,
    resolution: Option<(f32, f32)>,
    hide_overlay: Option<Arc<OverlayHideFn>>,
}

impl<S: States + FreelyMutableState> ScreenshotPlugin<S> {
    /// Create a screenshot driver targeting `state`, with sensible defaults
    /// (8 settle frames, `screenshot.png`, no resolution override, no overlay
    /// hook).
    pub fn new(state: S) -> Self {
        Self {
            state,
            settle_frames: 8,
            path: "screenshot.png".to_string(),
            resolution: None,
            hide_overlay: None,
        }
    }

    /// Number of frames to wait after the target state is entered before
    /// capturing, so animations and one-shot layout settle. Defaults to 8.
    pub fn settle_frames(mut self, frames: u32) -> Self {
        self.settle_frames = frames;
        self
    }

    /// Output path for the PNG. Defaults to `screenshot.png`.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Default window resolution to force before capture. The `WxH` value of
    /// the [`SCREENSHOT_ENV`] env var, if present, takes precedence over this.
    pub fn resolution(mut self, width: f32, height: f32) -> Self {
        self.resolution = Some((width, height));
        self
    }

    /// Set the overlay-hide hook: an exclusive `Startup` system that clears
    /// whatever dev overlay (inspector window, gizmos, diagnostics UI) would
    /// otherwise obscure the captured frame. This crate cannot know what those
    /// are, so the host app supplies the closure; a caller that omits it gets
    /// a capture with its overlays in frame and no error.
    pub fn hide_overlay(mut self, f: impl Fn(&mut World) + Send + Sync + 'static) -> Self {
        self.hide_overlay = Some(Arc::new(f));
        self
    }
}

/// Internal driver state; kept out of the prelude per the crate conventions.
#[derive(Resource)]
struct ScreenshotConfig<S: States + FreelyMutableState> {
    state: S,
    settle_frames: u32,
    path: String,
    resolution: Option<(f32, f32)>,
    advanced: bool,
    frames_in_state: u32,
    waited_frames: u32,
    shot_taken: bool,
}

impl<S: States + FreelyMutableState> Plugin for ScreenshotPlugin<S> {
    fn build(&self, app: &mut App) {
        let Ok(env_value) = std::env::var(SCREENSHOT_ENV) else {
            return;
        };

        // NOTE: both drivers write `NextState` and would fight over it;
        // the autopilot wins and the screenshot driver stands down.
        if std::env::var(AUTOPILOT_ENV).is_ok() {
            warn!(
                "ScreenshotPlugin: both {SCREENSHOT_ENV} and {AUTOPILOT_ENV} are set; \
                 deferring to the autopilot and doing nothing"
            );
            return;
        }

        let resolution = parse_resolution(&env_value).or(self.resolution);

        debug!(
            "ScreenshotPlugin: build ({SCREENSHOT_ENV} active, target {:?}, resolution {:?})",
            self.state, resolution
        );

        app.insert_resource(ScreenshotConfig::<S> {
            state: self.state.clone(),
            settle_frames: self.settle_frames,
            path: self.path.clone(),
            resolution,
            advanced: false,
            frames_in_state: 0,
            waited_frames: 0,
            shot_taken: false,
        });

        if resolution.is_some() {
            app.add_systems(Startup, resize_window::<S>);
        }
        if let Some(hide_overlay) = self.hide_overlay.clone() {
            app.add_systems(Startup, move |world: &mut World| hide_overlay(world));
        }
        app.add_systems(Update, screenshot_drive::<S>);
        completion::register(app, completion::SCREENSHOT);
    }
}

/// Force the primary window to the configured resolution at startup.
fn resize_window<S: States + FreelyMutableState>(
    config: Res<ScreenshotConfig<S>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Some((width, height)) = config.resolution else {
        return;
    };
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(width, height);
        // NOTE: pin the size, or a tiling/reflowing WM resizes the window
        // back and the responsive-layout capture measures the wrong width.
        window.resizable = false;
        trace!("screenshot: forced window resolution to {width}x{height}");
    }
}

/// Advance to the target state, count settled frames, then capture and report.
fn screenshot_drive<S: States + FreelyMutableState>(
    mut commands: Commands,
    mut config: ResMut<ScreenshotConfig<S>>,
    state: Res<State<S>>,
    mut next: ResMut<NextState<S>>,
    mut exit: MessageWriter<AppExit>,
) {
    if config.shot_taken {
        return;
    }

    if !config.advanced {
        if state.get() != &config.state {
            next.set(config.state.clone());
        }
        config.advanced = true;
        trace!("screenshot: advancing to {:?}", config.state);
        return;
    }

    if state.get() != &config.state {
        config.waited_frames += 1;
        if config.waited_frames >= MAX_WAIT_FRAMES {
            error!(
                "screenshot: target state {:?} not reached within {MAX_WAIT_FRAMES} frames; \
                 giving up",
                config.state
            );
            config.shot_taken = true;
            // NOTE: an unreachable target is an ABORT, not a completion -
            // error exits do not negotiate with the other collectors.
            exit.write(AppExit::error());
        }
        return;
    }

    config.frames_in_state += 1;
    if config.frames_in_state < config.settle_frames {
        return;
    }

    let path = config.path.clone();
    info!(
        "screenshot: capturing {:?} after {} settled frames -> {path}",
        config.state, config.frames_in_state
    );

    // NOTE: `save_to_disk` writes the PNG synchronously in its observer, so
    // the second observer only reports done once the frame is on disk.
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path))
        .observe(
            |_: On<ScreenshotCaptured>, mut completion: ResMut<completion::HarnessCompletion>| {
                info!("screenshot: capture complete");
                completion.done(completion::SCREENSHOT);
            },
        );

    config.shot_taken = true;
}

/// Parse a `WxH` resolution string (case-insensitive `x`), for example
/// `800x600`. Returns `None` when the string is not a valid `WxH` pair of
/// positive dimensions, so a plain toggle value like `1` (or a nonsense
/// `0x0` / `-5x10`) leaves the resolution unchanged.
fn parse_resolution(value: &str) -> Option<(f32, f32)> {
    let (width, height) = value.split_once(['x', 'X'])?;
    let width: f32 = width.trim().parse().ok()?;
    let height: f32 = height.trim().parse().ok()?;
    (width > 0.0 && height > 0.0).then_some((width, height))
}

#[cfg(test)]
mod tests {
    use super::parse_resolution;

    // NOTE: only the pure parser is tested here. The App-driven tests live in
    // `tests/screenshot.rs`, because THIS binary's `autopilot` tests arm
    // `NOVA_AUTOPILOT` process-wide and the driver would stand down.

    #[test]
    fn parses_valid_wxh() {
        assert_eq!(parse_resolution("800x600"), Some((800.0, 600.0)));
        assert_eq!(parse_resolution("390x844"), Some((390.0, 844.0)));
        assert_eq!(parse_resolution("1024X768"), Some((1024.0, 768.0)));
        assert_eq!(parse_resolution(" 640 x 480 "), Some((640.0, 480.0)));
    }

    #[test]
    fn rejects_non_resolution_values() {
        assert_eq!(parse_resolution("1"), None, "a plain toggle value");
        assert_eq!(parse_resolution(""), None);
        assert_eq!(parse_resolution("wide"), None);
        assert_eq!(parse_resolution("800x"), None);
        assert_eq!(parse_resolution("x600"), None);
        assert_eq!(parse_resolution("800x600x3"), None);
    }

    /// Zero or negative dimensions would be nonsense fed to `resolution.set`.
    #[test]
    fn rejects_non_positive_dimensions() {
        assert_eq!(parse_resolution("0x0"), None);
        assert_eq!(parse_resolution("800x0"), None);
        assert_eq!(parse_resolution("-5x10"), None);
        assert_eq!(parse_resolution("10x-5"), None);
    }
}
