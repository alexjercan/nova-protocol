//! The marker components naming every node of the NOVA OS tree.
//!
//! Spawn and the reconcilers never share a path expression: spawn attaches a
//! marker, the reconcilers query for it, so moving a node in the hierarchy
//! costs nothing.
//!
//! Touch this module when adding a node another system has to find.

use bevy::prelude::*;
use nova_gameplay::{objectives::Objective, prelude::*};

use super::style::*;

/// The NOVA OS UI root whose visibility is driven by [`NovaOsOpenness`].
#[derive(Component)]
pub(crate) struct NovaOsRootMarker;

/// The single physical NOVA OS monitor root.
#[derive(Component)]
pub(crate) struct NovaOsMonitorMarker;

/// The recessed physical bezel around the phosphor screen.
#[derive(Component)]
pub(crate) struct NovaOsBezelMarker;

/// The active green phosphor screen surface.
#[derive(Component)]
pub(crate) struct NovaOsScreenMarker;

/// The terminal placeholder content under the CRT overlay stack.
#[derive(Component)]
pub(crate) struct NovaOsTerminalContentMarker;

/// The persistent header bar (`<header>`): brand/breadcrumb on the left, the
/// close control + ship/FPS status on the right. Always on screen at a fixed
/// height; the old name is kept so the topbar is still one queryable node.
#[derive(Component)]
pub(crate) struct NovaOsTopbarMarker;

/// The persistent middle region (`<main>`) between the header and the footer.
/// It swaps between the terminal surface and a launched app body; the app root
/// is spawned as an absolute-fill child of this node.
#[derive(Component)]
pub(crate) struct NovaOsMainMarker;

/// The lit square lamp to the left of the NOVA OS brand.
#[derive(Component)]
pub(crate) struct NovaOsLampMarker;

/// The header brand/breadcrumb text (`NOVA OS <ver> // SHELL` at the prompt,
/// `NOVA OS <ver> // APPS / <ID>` while an app owns the screen).
#[derive(Component)]
pub(crate) struct NovaOsBrandMarker;

/// Right-side status text row in the header (`SHIP: .. LINK: .. FPS: ..`).
#[derive(Component)]
pub(crate) struct NovaOsStatusMarker;

/// The single terminal surface that fills the monitor screen.
#[derive(Component)]
pub(crate) struct NovaOsTerminalSurfaceMarker;

/// Scrollback rows printed by the NOVA OS terminal shell.
#[derive(Component)]
pub(crate) struct NovaOsTerminalScrollbackMarker;

/// Prompt row at the bottom of the terminal surface.
#[derive(Component)]
pub(crate) struct NovaOsPromptRowMarker;

/// Horizontal command-entry line inside the prompt strip.
#[derive(Component)]
pub(crate) struct NovaOsPromptInputLineMarker;

/// The fixed amber `nova>` prompt prefix.
#[derive(Component)]
pub(crate) struct NovaOsPromptPrefixMarker;

/// Remaining-width input lane that owns typed prompt and ghost completion.
#[derive(Component)]
pub(crate) struct NovaOsPromptInputWrapMarker;

/// Typed prompt text LEFT of the caret, owned by the terminal shell.
#[derive(Component)]
pub(crate) struct NovaOsTerminalPromptMarker;

/// Typed prompt text RIGHT of the caret (empty when the caret is at the end).
#[derive(Component)]
pub(crate) struct NovaOsTerminalPromptAfterMarker;

/// The block caret rendered between the before/after prompt text.
#[derive(Component)]
pub(crate) struct NovaOsTerminalCaretMarker;

/// Hint/status line owned by the terminal shell.
#[derive(Component)]
pub(crate) struct NovaOsTerminalHintMarker;

/// Ghost completion suffix rendered inline beside the typed prompt.
#[derive(Component)]
pub(crate) struct NovaOsTerminalGhostMarker;

/// The footer hint row from the PoC.
#[derive(Component)]
pub(crate) struct NovaOsFooterHintsMarker;

/// One of the four moulded corner screws on the casing (PoC `.screw`).
#[derive(Component)]
pub(crate) struct NovaOsScrewMarker;

/// The top-centre vent strip on the casing (PoC `.vents`).
#[derive(Component)]
pub(crate) struct NovaOsVentMarker;

/// The inset moulding-seam outline inside the casing (PoC `.case::after`).
#[derive(Component)]
pub(crate) struct NovaOsSeamMarker;

/// The bottom casing chin strip below the bezel (PoC `.chin`).
#[derive(Component)]
pub(crate) struct NovaOsChinMarker;

/// The recessed brand plate on the chin's left (PoC `.plate`).
#[derive(Component)]
pub(crate) struct NovaOsBrandPlateMarker;

/// The controls row on the chin's right, carrying the BRIGHT/SCAN knobs and the
/// SND/PWR buttons.
#[derive(Component)]
pub(crate) struct NovaOsControlsRowMarker;

/// Player-tunable NOVA OS monitor hardware state, driven by the physical chin
/// controls: the BRIGHT/SCAN knob detents and the SND
/// speaker toggle. Persisted via the settings store (nova_menu) so the dial
/// positions and mute survive a restart. Defaults mirror the PoC boot state:
/// BRIGHT detent 1 (= neutral 1.0), SCAN detent 2 (the shipped scanline look),
/// sound ON.
#[derive(Resource, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Resource)]
pub struct NovaOsMonitorSettings {
    /// BRIGHT knob detent, an index into `NOVA_OS_BRIGHT_DETENTS`.
    pub bright_detent: usize,
    /// SCAN knob detent, an index into `NOVA_OS_SCAN_DETENTS`.
    pub scan_detent: usize,
    /// Whether the monitor speaker is armed (the SND button), gating the cues
    /// and the ambient bed.
    pub sound_enabled: bool,
}

impl Default for NovaOsMonitorSettings {
    fn default() -> Self {
        Self {
            bright_detent: NOVA_OS_BRIGHT_DEFAULT_DETENT,
            scan_detent: NOVA_OS_SCAN_DEFAULT_DETENT,
            sound_enabled: true,
        }
    }
}

impl NovaOsMonitorSettings {
    /// The brightness multiply for the current BRIGHT detent. Clamps a
    /// possibly-corrupt persisted index into range.
    pub fn brightness(&self) -> f32 {
        NOVA_OS_BRIGHT_DETENTS[self.bright_detent.min(NOVA_OS_BRIGHT_DETENTS.len() - 1)]
    }

    /// The scanline strength for the current SCAN detent. Clamps a
    /// possibly-corrupt persisted index into range.
    pub fn scanline_strength(&self) -> f32 {
        NOVA_OS_SCAN_DETENTS[self.scan_detent.min(NOVA_OS_SCAN_DETENTS.len() - 1)]
    }

    /// Clamp possibly-corrupt persisted detent indices into range. The readers
    /// above clamp too, so the screen always looks right, but [`Self::cycle`]
    /// wraps modulo the detent count - an out-of-range index would make the
    /// next click jump from brightest to dimmest.
    pub fn clamp_detents(&mut self) {
        self.bright_detent = self.bright_detent.min(NOVA_OS_BRIGHT_DETENTS.len() - 1);
        self.scan_detent = self.scan_detent.min(NOVA_OS_SCAN_DETENTS.len() - 1);
    }

    /// The dial-pointer angle for a knob's current detent.
    pub(crate) fn dial_angle(&self, knob: NovaOsKnob) -> f32 {
        let index = match knob {
            NovaOsKnob::Bright => self.bright_detent,
            NovaOsKnob::Scan => self.scan_detent,
        };
        NOVA_OS_KNOB_ANGLES[index.min(NOVA_OS_KNOB_ANGLES.len() - 1)]
    }

    /// Advance a knob to its next detent, wrapping (PoC `(index + 1) % len`).
    pub(crate) fn cycle(&mut self, knob: NovaOsKnob) {
        match knob {
            NovaOsKnob::Bright => {
                self.bright_detent = (self.bright_detent + 1) % NOVA_OS_BRIGHT_DETENTS.len();
            }
            NovaOsKnob::Scan => {
                self.scan_detent = (self.scan_detent + 1) % NOVA_OS_SCAN_DETENTS.len();
            }
        }
    }
}

/// Which chin knob a button/dial belongs to.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NovaOsKnob {
    /// The BRIGHT knob (screen brightness multiply).
    Bright,
    /// The SCAN knob (scanline depth).
    Scan,
}

/// The rotating dial pointer inside a knob (a child of the knob button). Carries
/// the [`NovaOsKnob`] it belongs to so the sync system rotates the right one.
#[derive(Component)]
pub(crate) struct NovaOsKnobDialMarker;

/// The SND speaker toggle button on the chin.
#[derive(Component)]
pub(crate) struct NovaOsSoundButtonMarker;

/// The lit/unlit green bulb inside the SND button: its colour (not the label
/// text) is the on/off state.
#[derive(Component)]
pub(crate) struct NovaOsSoundIndicatorMarker;

/// The PWR power LED: green while the monitor is on, flashing orange while it
/// powers down (driven by [`super::shell::drive_nova_os_power_led`]).
#[derive(Component)]
pub(crate) struct NovaOsPowerLedMarker;

/// The PWR button on the chin (the diegetic twin of the `exit` command).
#[derive(Component)]
pub(crate) struct NovaOsPowerButtonMarker;

/// The bright phosphor rim tracing the screen edge (PoC `.rim` line/glow pair).
#[derive(Component)]
pub(crate) struct NovaOsPhosphorRimMarker;

/// The glass specular sheen laid over the screen (PoC `.glass`).
#[derive(Component)]
pub(crate) struct NovaOsGlassMarker;

/// The root of the active NOVA OS app, spawned as a sibling of the terminal
/// content while `TerminalMode::App` is active. Carries the running app's id so
/// [`super::shell::sync_nova_os_app_ui`] can tell a launch/exit/switch apart from a no-op.
#[derive(Component)]
pub(crate) struct NovaOsAppRoot {
    pub(crate) id: &'static str,
}

/// The `[ ESC ]` close control in the persistent header, shown only while an app
/// owns the screen; clicking it exits the app back to the terminal, mirroring the
/// Escape route.
#[derive(Component)]
pub(crate) struct NovaOsAppCloseMarker;

/// The dim full-screen backdrop behind the panel.
#[derive(Component)]
pub(crate) struct NovaOsBackdropMarker;

/// The container the objectives-section lines are (re)built into.
#[derive(Component)]
pub(crate) struct NovaOsObjectivesListMarker;

/// Scrollable viewport around a NOVA OS row list.
#[derive(Component)]
pub(crate) struct NovaOsScrollViewportMarker;

/// One objective row in the NOVA OS's mission-log list.
#[derive(Component)]
pub(crate) struct NovaOsObjectiveRowMarker;

/// Objective id copied onto each NOVA OS row for rebuild and tests.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) struct NovaOsObjectiveId(pub(crate) String);

/// Whether a row is still active or retained as completed history.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NovaOsObjectiveRowStatus {
    Active,
}

/// The small status glyph at the start of a NOVA OS objective row.
#[derive(Component)]
pub(crate) struct NovaOsObjectiveGlyphMarker;

/// The text entity for a NOVA OS objective row.
#[derive(Component)]
pub(crate) struct NovaOsObjectiveTextMarker;

/// Thin overlay used as a completed row's line-through.
#[cfg(test)]
#[derive(Component)]
pub(crate) struct NovaOsObjectiveStrikeMarker;

/// Styled empty-state row for the objective list.
#[derive(Component)]
pub(crate) struct NovaOsObjectiveEmptyMarker;

/// The container the combined left-panel flight log is rebuilt into.
#[derive(Component)]
pub(crate) struct NovaOsFlightLogListMarker;

/// One row in the left-panel combined flight log stream.
#[derive(Component)]
pub(crate) struct NovaOsFlightLogRowMarker;

/// Text entity for a combined flight log row.
#[derive(Component)]
pub(crate) struct NovaOsFlightLogTextMarker;

/// Styled empty-state row for the combined flight log.
#[derive(Component)]
pub(crate) struct NovaOsFlightLogEmptyMarker;

/// Icon semantics for a combined flight log row.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NovaOsFlightLogIconMarker {
    pub(crate) kind: NovaOsFlightLogIconKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NovaOsFlightLogIconKind {
    CommsAuthored,
    Fallback,
    Objective,
}

/// Openness in `[0, 1]`: 0 fully closed (off-screen past the panel's edge), 1
/// fully open (flush with that edge). Eased toward the state-driven target with
/// real time so it keeps moving while the sim is frozen.
#[derive(Component, Default)]
pub(crate) struct NovaOsOpenness(pub(super) f32);

/// True after the user requested close; gameplay remains paused until the
/// real-time close animation reaches zero.
#[derive(Resource, Default)]
pub(crate) struct NovaOsCloseTransition {
    pub(crate) closing: bool,
}

/// NovaOs-local combined flight log derived from [`StoryFeed`] and
/// [`GameObjectives`].
///
/// The monitor placeholder keeps the historical stream: comms rows plus
/// objective posted/completed rows, in the order the HUD observes them.
/// Objective text updates edit the open posted row rather than appending
/// duplicate events.
#[derive(Resource, Default, Debug, Clone)]
pub(crate) struct NovaOsFlightLog {
    pub(crate) entries: Vec<NovaOsFlightLogEntry>,
    pub(crate) active_objective_entries: Vec<NovaOsFlightLogActiveObjective>,
    pub(crate) previous_active: Vec<Objective>,
    pub(crate) seen_story: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NovaOsFlightLogActiveObjective {
    pub(crate) id: String,
    pub(crate) entry_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NovaOsFlightLogEntry {
    pub(crate) kind: NovaOsFlightLogEntryKind,
    pub(crate) objective_id: Option<String>,
    pub(crate) speaker: Option<String>,
    pub(crate) message: String,
    pub(crate) icon: Option<AssetRef<Image>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NovaOsFlightLogEntryKind {
    Comms,
    ObjectivePosted,
    ObjectiveCompleted,
}

/// The live degauss pulse. [`super::shell::sync_nova_os_app_ui`] resets
/// `remaining` to [`NOVA_OS_DEGAUSS_DURATION`] on every real app
/// launch/exit/switch (the same transitions that play the `NovaOsCoil` coil
/// sound, so the wobble+flash lands with the thump); [`super::crt::animate_nova_os_crt`]
/// bleeds it down by real time and feeds the `remaining / DURATION` envelope into
/// the CRT `degauss` uniform. Real time because the sim clock is frozen while the
/// computer is open.
#[derive(Resource, Default, Debug)]
pub(crate) struct NovaOsDegauss {
    pub(crate) remaining: f32,
}

impl NovaOsDegauss {
    /// Kick the coil: restart the full pulse.
    pub(crate) fn pulse(&mut self) {
        self.remaining = NOVA_OS_DEGAUSS_DURATION;
    }

    /// The 0..1 shader envelope for the current `remaining`.
    pub(crate) fn envelope(&self) -> f32 {
        (self.remaining / NOVA_OS_DEGAUSS_DURATION).clamp(0.0, 1.0)
    }
}
