//! F12 screenshot hotkey.
//!
//! Under the `debug` feature, pressing [`SCREENSHOT_KEYCODE`] (F12) captures the
//! primary window and writes it to the user's Downloads directory as
//! `<unix-millis>.png`. It reuses Bevy's `Screenshot::primary_window()` +
//! `save_to_disk` primitive (the same one the reel harness and the `Screenshot`
//! scenario action use), driven straight from a `Commands` system rather than an
//! exclusive-`World` one.
//!
//! The capture is deliberately NOT gated by [`DebugEnabled`](crate::DebugEnabled)
//! (the F11 overlay toggle): a screenshot should work whether or not the dev
//! overlays are currently shown, so the system runs every frame in `Update`
//! outside the [`DebugSystems`](crate::DebugSystems) run condition.

use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bevy::{
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
};

/// The keycode that captures a screenshot. F1 is the editor toggle and F11 is
/// [`DEBUG_TOGGLE_KEYCODE`](crate::DEBUG_TOGGLE_KEYCODE); F12 is free.
pub const SCREENSHOT_KEYCODE: KeyCode = KeyCode::F12;

/// A plugin that saves a screenshot to Downloads when [`SCREENSHOT_KEYCODE`] is
/// pressed. Added by [`DebugPlugin`](crate::DebugPlugin), so it only exists in a
/// `--features debug` build.
pub struct ScreenshotHotkeyPlugin;

impl Plugin for ScreenshotHotkeyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, capture_screenshot_on_key);
    }
}

/// Build the timestamped screenshot filename from a duration since the Unix
/// epoch. Unix millis keeps rapid successive presses from colliding and sorts
/// chronologically; a human-readable date would need `chrono` and timezone
/// handling, which is out of scope for the debug convenience.
fn screenshot_filename(since_epoch: Duration) -> String {
    format!("{}.png", since_epoch.as_millis())
}

/// Resolve the destination path: the user's Downloads directory (falling back to
/// the current directory when the platform reports none) joined with the
/// timestamped filename for the current wall-clock time.
fn downloads_screenshot_path() -> PathBuf {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let dir = dirs::download_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.join(screenshot_filename(since_epoch))
}

/// On [`SCREENSHOT_KEYCODE`], spawn a primary-window capture that saves to the
/// resolved Downloads path. `create_dir_all` is defensive (Downloads normally
/// exists) and mirrors the harness `capture_window`.
fn capture_screenshot_on_key(mut commands: Commands, keyboard: Res<ButtonInput<KeyCode>>) {
    if !keyboard.just_pressed(SCREENSHOT_KEYCODE) {
        return;
    }

    let path = downloads_screenshot_path();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                warn!("screenshot: could not create dir {:?}: {error}", parent);
            }
        }
    }

    info!("screenshot: capturing primary window -> {}", path.display());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The filename is the Unix-millis timestamp with a `.png` extension.
    #[test]
    fn screenshot_filename_is_millis_dot_png() {
        assert_eq!(screenshot_filename(Duration::from_millis(0)), "0.png");
        assert_eq!(
            screenshot_filename(Duration::from_millis(1_737_027_685_123)),
            "1737027685123.png"
        );
        // Sub-millisecond precision truncates to whole millis.
        assert_eq!(screenshot_filename(Duration::from_micros(1_500)), "1.png");
    }

    /// Pressing F12 spawns exactly one `Screenshot` capture entity; with no press
    /// the same rig spawns none (so the press, not the rig, is what fires).
    #[test]
    fn f12_spawns_a_screenshot_capture() {
        fn screenshots(app: &mut App) -> usize {
            app.world_mut()
                .query::<&Screenshot>()
                .iter(app.world())
                .count()
        }

        // No press: nothing captured.
        let mut idle = App::new();
        idle.init_resource::<ButtonInput<KeyCode>>();
        idle.add_systems(Update, capture_screenshot_on_key);
        idle.update();
        assert_eq!(
            screenshots(&mut idle),
            0,
            "no key press must capture nothing"
        );

        // F12 pressed: exactly one capture entity.
        let mut pressed = App::new();
        pressed.init_resource::<ButtonInput<KeyCode>>();
        pressed.add_systems(Update, capture_screenshot_on_key);
        pressed
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(SCREENSHOT_KEYCODE);
        pressed.update();
        assert_eq!(
            screenshots(&mut pressed),
            1,
            "F12 must spawn one primary-window capture"
        );
    }
}
