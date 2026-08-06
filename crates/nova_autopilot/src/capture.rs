//! The capture primitive: write the primary window to a PNG, and the
//! environment contract that arms a capturing run.
//!
//! There is ONE capture idiom in the fleet. A capturing example is an ordinary
//! autopilot script whose steps call [`capture_window`] from an `on_enter`
//! hook, so the act, the framing and the shot read top-to-bottom in the step
//! list. (A multi-shot reel driver used to own that cadence itself, with the
//! beat list built away from the script that produced the state each beat
//! framed; it was deleted for this.)
//!
//! [`ScreenshotPlugin`](crate::screenshot::ScreenshotPlugin) is the one
//! remaining capture DRIVER, and it stays: it is a single settled-frame shot
//! with no script at all, for an example that just wants a picture of itself.
//!
//! Nothing here is armed on its own. [`capturing`] reports whether
//! [`CAPTURE_ENV`] is set, and a script uses it to decide whether it is on the
//! capture path or the smoke path - the same file drives both.
//!
//! Relative paths resolve under [`SHOT_DIR_ENV`], so a whole run stages into
//! one folder; an absolute path is used as-is.

use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
};

/// Environment variable that puts a run on the CAPTURE path: its scripts take
/// their shots and settle long enough for the PNGs to land. Unset, the same
/// script is the smoke path - it walks the whole thing and captures nothing.
///
/// Distinct from [`SCREENSHOT_ENV`](crate::screenshot::SCREENSHOT_ENV) (the
/// single settled-frame driver) so a scripted capture run and a one-off
/// snapshot never fight over the window.
pub const CAPTURE_ENV: &str = "NOVA_CAPTURE";

/// Environment variable naming the directory relative capture paths resolve
/// under. Unset (or empty), a relative path stays relative to the CWD.
pub const SHOT_DIR_ENV: &str = "NOVA_SHOT_DIR";

/// Default capture resolution: 1920x1080, the 16:9 the web figures and
/// thumbnails use (thumbnails share this capture and are sized down in CSS).
pub const CAPTURE_RESOLUTION: (f32, f32) = (1920.0, 1080.0);

/// Whether this run is on the capture path ([`CAPTURE_ENV`] is set).
///
/// A script reads it ONCE while building its steps rather than per frame: the
/// answer decides which settle values the whole walk uses, and a run that
/// changed its mind halfway would produce shots settled to the smoke path's
/// budget.
pub fn capturing() -> bool {
    std::env::var_os(CAPTURE_ENV).is_some()
}

/// Capture the primary window to `path` (relative paths resolve under
/// [`SHOT_DIR_ENV`]), creating the parent directory if needed.
///
/// ASYNCHRONOUS: it spawns a bare `Screenshot` request, and the PNG lands at
/// the end of a later frame. Nothing here registers a completion collector, so
/// a caller must hold its step open long enough for the write to finish before
/// navigating away or ending the run.
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
                warn!("capture: could not create capture dir {parent:?}: {error}");
            }
        }
    }
}

/// Resolve an output path under [`SHOT_DIR_ENV`].
fn capture_path(path: &str) -> PathBuf {
    resolve_capture_path(std::env::var(SHOT_DIR_ENV).ok().as_deref(), path)
}

/// The path resolution itself, with the shot dir passed in rather than read:
/// only RELATIVE paths are joined - an absolute path passes through unchanged,
/// so a caller naming an exact file still gets it. Split from [`capture_path`]
/// so every branch is testable without mutating the process environment, which
/// would race the other tests sharing this binary.
fn resolve_capture_path(shot_dir: Option<&str>, path: &str) -> PathBuf {
    let path = std::path::Path::new(path);
    match shot_dir {
        Some(dir) if !dir.is_empty() && !path.is_absolute() => std::path::Path::new(dir).join(path),
        _ => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every branch of the resolution, asserted against an explicit shot dir
    /// instead of whatever the ambient env happens to hold.
    #[test]
    fn capture_path_joins_only_relative_paths_under_the_shot_dir() {
        assert_eq!(
            resolve_capture_path(None, "feature-gravity.png"),
            PathBuf::from("feature-gravity.png"),
            "with no shot dir a relative path stays relative"
        );
        assert_eq!(
            resolve_capture_path(Some(""), "feature-gravity.png"),
            PathBuf::from("feature-gravity.png"),
            "an empty shot dir is the same as none, not a join onto nothing"
        );
        assert_eq!(
            resolve_capture_path(Some("/shots"), "feature-gravity.png"),
            PathBuf::from("/shots/feature-gravity.png"),
            "a relative path resolves under the shot dir"
        );
        assert_eq!(
            resolve_capture_path(Some("/shots"), "/elsewhere/a.png"),
            PathBuf::from("/elsewhere/a.png"),
            "an absolute path passes through whatever the shot dir says"
        );
        assert_eq!(
            resolve_capture_path(None, "/elsewhere/a.png"),
            PathBuf::from("/elsewhere/a.png"),
            "an absolute path is untouched with no shot dir either"
        );
    }
}
