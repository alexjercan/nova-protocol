//! The pure text layer: what the modal's title, status line and section kinds
//! SAY.
//!
//! No `Commands` and no queries, so every string is testable without an app.
//!
//! Touch this module when changing the wording of the modal chrome.

use nova_gameplay::prelude::*;

use super::components::*;

/// The modal's title, on the header brand plate.
pub(crate) const NOVA_COMMANDS_TITLE: &str = "NOVA COMMANDS";

/// The FPS number formatted to a FIXED width so the topbar does not reflow when
/// the reading changes digit count (owner playtest: 100 -> 99 must not shift the
/// layout). Right-aligned to 3 chars in the monospace topbar font, so ` 99` and
/// `100` occupy the same width; `--` before the first reading pads the same way.
pub(crate) fn nova_os_fps_segment(fps: Option<u32>) -> String {
    match fps {
        Some(fps) => format!("{fps:>3}"),
        None => format!("{:>3}", "--"),
    }
}

/// The modal's topbar status line: the arming state and link, plus a live FPS
/// segment. The FPS is rehomed here from the flight status bar, which hides
/// while the modal is open; `fps` is the smoothed frame rate rounded to a whole
/// number, or `None` before the diagnostic has a reading (shown as `--`).
pub(crate) fn nova_os_status_text(armed: bool, fps: Option<u32>) -> String {
    let fps = nova_os_fps_segment(fps);
    format!(
        "{}{NOVA_OS_TOPBAR_FPS_MARKER}{fps}",
        command_topbar_head(armed)
    )
}

/// The head of the modal topbar: the one piece of state a player must be able
/// to see at a glance, whether this run was armed.
pub(crate) fn command_topbar_head(armed: bool) -> String {
    let cheats = if armed { "ON " } else { "OFF" };
    format!("CHEATS: {cheats}     LINK: LOCAL")
}

impl NovaOsFlightLog {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.active_objective_entries.clear();
        self.previous_active.clear();
        self.seen_story = 0;
    }
}

pub(crate) fn section_kind_from_markers(
    class: Option<&SectionClass>,
    hull: bool,
    controller: bool,
    thruster: bool,
    turret: bool,
    torpedo: bool,
) -> Option<SectionClass> {
    if let Some(class) = class {
        return Some(*class);
    }
    if hull {
        Some(SectionClass::Hull)
    } else if controller {
        Some(SectionClass::Controller)
    } else if thruster {
        Some(SectionClass::Thruster)
    } else if turret {
        Some(SectionClass::Turret)
    } else if torpedo {
        Some(SectionClass::Torpedo)
    } else {
        None
    }
}

/// The separator that fronts the FPS segment in the topbar status line. The drive
/// system rewrites everything from this marker on, leaving the `CHEATS:`/`LINK:`
/// head untouched.
pub(crate) const NOVA_OS_TOPBAR_FPS_MARKER: &str = "     FPS: ";

/// The smoothed frame rate rounded to a whole number, or `None` before the
/// diagnostic has a reading. Reuses Bevy's `FrameTimeDiagnosticsPlugin::FPS`
/// smoothed value - the exact source the flight status bar's FPS item read
/// (bcs `status_fps_value_fn`) - so the number on the topbar matches the one the
/// hidden status bar would show.
pub(crate) fn nova_os_diagnostic_fps(
    diagnostics: &bevy::diagnostic::DiagnosticsStore,
) -> Option<u32> {
    diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .map(|fps| fps.round() as u32)
}

/// Rewrite only the `FPS: <n>` tail of a topbar status line, preserving the
/// `CHEATS:`/`LINK:` head. Falls back to appending the segment if a line somehow
/// lacks it (e.g. an older spawn), so the FPS never silently goes missing.
pub(crate) fn topbar_line_with_fps(current: &str, fps: Option<u32>) -> String {
    let head = current
        .split_once(NOVA_OS_TOPBAR_FPS_MARKER)
        .map(|(head, _)| head)
        .unwrap_or(current);
    let fps = nova_os_fps_segment(fps);
    format!("{head}{NOVA_OS_TOPBAR_FPS_MARKER}{fps}")
}
