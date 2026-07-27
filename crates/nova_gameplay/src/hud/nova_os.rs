//! The Tab ship-computer NOVA OS: one inset NOVA OS cockpit monitor that opens
//! on Tab, freezing the sim and freeing the cursor while active. The monitor
//! replaces the old left/right panels with a physical terminal screen: dark
//! casing, hard bezel, green phosphor display and CRT overlays.
//! This module owns the shell, command prompt, scrollback, input handling and
//! terminal content. The existing objectives and combined flight-log data feed
//! read-only terminal commands, but they are no longer visible as permanent
//! monitor panes.
//!
//! # Interaction model
//!
//! Tab opens the NOVA OS by driving the shared [`PauseStates`] axis
//! (`Unpaused -> NovaOs`); once NOVA OS owns the keyboard, Tab completes the
//! terminal prompt instead of closing the monitor. ESC/Start/right-stick close
//! requests are owned here so gameplay stays paused until the close animation
//! reaches zero. The freeze + cursor-free are wired in `nova_menu` on
//! `OnEnter/OnExit(PauseStates::NovaOs)`, reusing the exact hooks the pause
//! overlay uses (see this task's DECISION.md - the NOVA OS is a THIRD variant of
//! the one freeze axis, not a separate freeze). The NOVA OS is inert while the
//! pause menu owns the freeze (`PauseStates::Paused`), which also means a live
//! outcome overlay - which forces `Paused` - implicitly blocks the NOVA OS
//! without this crate depending on `nova_scenario`'s `CurrentOutcome`.
//!
//! # Animation clock
//!
//! The slide is driven by [`Time<Real>`], NOT the bcs `Tween` (which advances
//! on the default `Res<Time>` = `Time<Virtual>`). Opening the NOVA OS PAUSES
//! virtual time, so a virtual-clocked tween would freeze mid-slide; the slide
//! must keep moving while the sim is frozen, so it reads real time
//! (`verify-engine-guarantees-in-source`: bcs `tween::advance_tweens` uses
//! `Res<Time>`).

use bevy::{
    asset::{io::Reader, uuid::Uuid, AssetApp, AssetLoader, LoadContext},
    audio::Volume,
    camera::{visibility::RenderLayers, ImageRenderTarget, NormalizedRenderTarget, RenderTarget},
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    picking::{
        hover::{HoverMap, Hovered},
        pointer::{
            Location, PointerAction, PointerButton, PointerId, PointerInput, PointerLocation,
        },
    },
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType, TextureFormat},
    shader::ShaderRef,
    ui::UiTargetCamera,
    ui_render::prelude::{MaterialNode, UiMaterial, UiMaterialPlugin},
    ui_widgets::{observe, Activate, Button},
};
use bevy_common_systems::prelude::{GameObjectives, Objective, SfxCommandsExt, SoundBank};
use nova_os::prelude::*;
use nova_ui::theme;

use super::NovaHudSystems;
use crate::{
    audio::{
        UiSfx, NOVA_OS_BACK_VOLUME, NOVA_OS_BED_VOLUME, NOVA_OS_COIL_VOLUME, NOVA_OS_ENTER_VOLUME,
        NOVA_OS_ERROR_VOLUME, NOVA_OS_KEY_MIN_INTERVAL, NOVA_OS_KEY_VOLUME, NOVA_OS_OK_VOLUME,
        NOVA_OS_POWER_VOLUME, NOVA_OS_TICK_VOLUME,
    },
    prelude::*,
    settings::{HarnessMute, MasterVolume},
    GameStates, PauseStates,
};

/// Seconds for the monitor to fade/activate fully open (or closed).
const DRAWER_SLIDE_SECS: f32 = 0.22;
/// Backdrop dim at full open. Deepened from the original 0.55 (task
/// 20260724-134335): with the flight HUD hidden while the NOVA OS is open, the
/// backdrop is the ONLY thing separating the NOVA OS from the frozen scene, so
/// it doubles as the "you do not notice the old UI is gone" gray field. The
/// owner chose a deeper gray over a real scene blur at the /flow gate (bevy
/// 0.19 has no UI backdrop-filter; see this task's DECISION.md).
const DRAWER_BACKDROP_ALPHA: f32 = 0.94;
const DRAWER_SECTION_TITLE_FONT_PX: f32 = 14.0;
const DRAWER_LINE_FONT_PX: f32 = 16.0;
/// The staggered boot banner reveals one row this far apart, on real time so it
/// runs while virtual time is frozen (PoC `printBanner`'s ~130 ms cadence).
const NOVA_OS_BOOT_ROW_INTERVAL: f32 = 0.13;
/// The block caret's WIDTH as a fraction of the font size (PoC `.caret` is
/// 0.6em). This is the cursor block's drawn width only - NOT the glyph advance,
/// so it is not used to position the caret (that measures the real text width;
/// see [`position_nova_os_block_caret`]).
const NOVA_OS_CARET_WIDTH_FRACTION: f32 = 0.6;
const DRAWER_ROW_GAP_PX: f32 = 6.0;
const DRAWER_ROW_PADDING_X_PX: f32 = 8.0;
const DRAWER_ROW_PADDING_Y_PX: f32 = 7.0;
const DRAWER_OBJECTIVE_GLYPH_WIDTH_PX: f32 = 18.0;
const DRAWER_LOG_ICON_SIZE_PX: f32 = 20.0;
const DRAWER_SCROLL_LINE_HEIGHT_PX: f32 = 20.0;

/// Horizontal inset from the viewport edge to the physical monitor casing. Kept
/// small so the monitor sits almost at the screen edges (the top status-bar
/// chrome may overlap it - that is intentional).
const NOVA_OS_MONITOR_INSET_X_PX: f32 = 16.0;
/// Vertical inset from the viewport edge to the physical monitor casing.
const NOVA_OS_MONITOR_INSET_Y_PX: f32 = 14.0;
const NOVA_OS_BEZEL_PAD_PX: f32 = 26.0;
const NOVA_OS_SCREEN_PAD_PX: f32 = 18.0;
/// Injection-moulded shell corners: a larger top radius and a tighter bottom,
/// like the PoC `.case` `border-radius: 22px 22px 14px 14px`, scaled up for the
/// full-viewport monitor.
const NOVA_OS_CASE_RADIUS_TOP_PX: f32 = 24.0;
const NOVA_OS_CASE_RADIUS_BOTTOM_PX: f32 = 15.0;
/// Recessed bezel + phosphor-screen corner radii (PoC `.bezel` 16px, screen 12).
const NOVA_OS_BEZEL_RADIUS_PX: f32 = 16.0;
const NOVA_OS_SCREEN_RADIUS_PX: f32 = 12.0;
/// Bottom casing strip under the bezel (PoC `.chin`, ~54px) that carries the
/// brand plate and the reserved controls row.
const NOVA_OS_CHIN_HEIGHT_PX: f32 = 54.0;

/// BRIGHT knob detents (task 20260726-214617): the extra brightness multiply fed
/// to the CRT `brightness` uniform, mirroring the PoC `BRIGHT` array. Index
/// [`NOVA_OS_BRIGHT_DEFAULT_DETENT`] (= 1.0) is the shipped neutral default.
const NOVA_OS_BRIGHT_DETENTS: [f32; 4] = [0.8, 1.0, 1.15, 1.3];
/// SCAN knob detents: the scanline-strength uniform. Index
/// [`NOVA_OS_SCAN_DEFAULT_DETENT`] is [`NOVA_OS_CRT_SCANLINE_STRENGTH`], the
/// shipped default look; 0 turns scanlines off, index 3 is a heavy, obviously
/// aggressive raster (owner call 2026-07-27). (The PoC's [0, 0.18, 0.34, 0.52]
/// were CSS-overlay opacities; the in-game shader's `scanline_strength` darkens
/// far harder, so the range is scaled to it.)
const NOVA_OS_SCAN_DETENTS: [f32; 4] = [0.0, 0.03, NOVA_OS_CRT_SCANLINE_STRENGTH, 0.20];
/// Dial-pointer angle (degrees) per detent index, mirroring the PoC `ANGLES`.
const NOVA_OS_KNOB_ANGLES: [f32; 4] = [-115.0, -38.0, 38.0, 115.0];
/// Default BRIGHT detent index (PoC `brightIndex = 1`, = neutral 1.0).
const NOVA_OS_BRIGHT_DEFAULT_DETENT: usize = 1;
/// Default SCAN detent index (PoC `scanIndex = 2`, = the shipped scanline look).
const NOVA_OS_SCAN_DEFAULT_DETENT: usize = 2;
const NOVA_OS_TERMINAL_PAD_X_PX: f32 = 16.0;
const NOVA_OS_TERMINAL_PAD_Y_PX: f32 = 14.0;
const NOVA_OS_PROMPT_ROW_HEIGHT_PX: f32 = 58.0;
const NOVA_OS_FONT_PATH: &str = "fonts/SGr-IosevkaTerm-Regular.ttc";
const NOVA_OS_BACKDROP: Color = Color::srgb_u8(0, 3, 6);
// Dark-GRAY moulded plastic, matching the PoC `:root` `--case-*` (neutral, not
// blue). `--case-0`, a mid raised body, and `--case-edge`.
const NOVA_OS_CASE: Color = Color::srgb_u8(10, 13, 16);
const NOVA_OS_CASE_RAISED: Color = Color::srgb_u8(16, 22, 27);
const NOVA_OS_CASE_EDGE: Color = Color::srgb_u8(5, 7, 10);
const NOVA_OS_SCREEN: Color = Color::srgb_u8(0, 4, 1);
// Palette lifted from `nova_os_terminal_poc.html`: a hot neon phosphor for the
// prompt, borders and headers; a pale mint for ordinary body text (the HTML
// `--text`), which reads brighter and higher-contrast on the near-black screen
// than the old all-one-green treatment.
const NOVA_OS_PHOSPHOR: Color = Color::srgb_u8(54, 255, 121);
const NOVA_OS_TEXT: Color = Color::srgb_u8(185, 255, 201);
const NOVA_OS_PHOSPHOR_DIM: Color = Color::srgb_u8(95, 238, 137);
const NOVA_OS_PHOSPHOR_MUTED: Color = Color::srgb_u8(70, 207, 118);
const NOVA_OS_INFO: Color = Color::srgb_u8(54, 163, 255);
const NOVA_OS_AMBER: Color = Color::srgb_u8(255, 184, 74);
// Moulded-plastic depth palette (casing gradient stops, screws, seam catch).
// The PoC `.case` body runs a 168deg gradient from a lit top (`--case-3`) down
// through the mid body to an almost-black undercut; these are those `--case-*`
// stops (dark GRAY, not blue).
const NOVA_OS_CASE_LIT: Color = Color::srgb_u8(47, 56, 63);
const NOVA_OS_CASE_MID: Color = Color::srgb_u8(22, 27, 32);
const NOVA_OS_CASE_DEEP: Color = Color::srgb_u8(10, 13, 16);
/// The 1px top light line that catches the moulding lip (PoC `inset 0 1px 0`).
const NOVA_OS_CASE_HIGHLIGHT: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);
/// Screw head shading (PoC `.screw` radial gradient light -> dark).
const NOVA_OS_SCREW_LIT: Color = Color::srgb_u8(89, 101, 110);
const NOVA_OS_SCREW_DARK: Color = Color::srgb_u8(10, 13, 16);
/// Chin-button moulding (PoC `.power-btn` `linear-gradient(180deg,#333c44,#1a2026)`
/// with a 1px inner top-highlight and a near-black outer border): a small raised
/// pill of plastic, lighter than the surrounding case so it reads as a pressable
/// key rather than a painted rectangle.
const NOVA_OS_BUTTON_LIT: Color = Color::srgb_u8(51, 60, 68);
const NOVA_OS_BUTTON_DEEP: Color = Color::srgb_u8(26, 32, 38);
const NOVA_OS_BUTTON_BORDER: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);
/// Knob dial dome (PoC `.dial` `radial-gradient(circle at 34% 28%,#4a555d,#232a30
/// 58%,#0d1114)`): an off-centre highlight over a dark disc gives the moulded
/// rotary its rounded 3D body.
const NOVA_OS_DIAL_LIT: Color = Color::srgb_u8(74, 85, 93);
const NOVA_OS_DIAL_MID: Color = Color::srgb_u8(35, 42, 48);
const NOVA_OS_DIAL_DARK: Color = Color::srgb_u8(13, 17, 20);
/// The PWR button/LED flashes this warm orange while the monitor is powering
/// down (owner playtest: "turn orange and then close"), before the raster
/// collapse finishes the close.
const NOVA_OS_ORANGE: Color = Color::srgb_u8(255, 120, 40);
/// An unlit green bulb: the SND indicator dims to this dark phosphor when muted,
/// so the bulb (not a text swap) carries the on/off state.
const NOVA_OS_BULB_OFF: Color = Color::srgb_u8(18, 34, 22);
const NOVA_OS_CONTENT_Z: i32 = 0;
const NOVA_OS_OVERLAY_Z: i32 = 1;
/// Phosphor rim traces the screen edge above the CRT overlay; the glass sheen is
/// the frontmost surface layer over it.
const NOVA_OS_RIM_Z: i32 = 2;
const NOVA_OS_GLASS_Z: i32 = 3;
/// Blink rate of the terminal caret, in full on/off cycles per second.
const NOVA_OS_CARET_BLINK_HZ: f32 = 1.25;

/// Straight-alpha CRT overlay tint + scanline controls, passed to WGSL. Kept
/// deliberately faint so the overlay never films the text underneath: the tint
/// is a whisper of green, the vignette darkens only the outer edges, and the
/// centre glow is a low bulge that reads as volume rather than a wash (see
/// `assets/shaders/nova_os_crt.wgsl`).
const NOVA_OS_CRT_TINT: LinearRgba = LinearRgba::new(0.212, 1.0, 0.475, 0.03);
const NOVA_OS_CRT_SCANLINE_STRENGTH: f32 = 0.06;
const NOVA_OS_CRT_VIGNETTE_STRENGTH: f32 = 0.55;
/// Centre-peaked phosphor bulge that gives the flat panel its CRT volume and a
/// clearly brighter middle (the HTML radial-gradient centre).
const NOVA_OS_CRT_GLOW_STRENGTH: f32 = 0.07;
const NOVA_OS_CRT_GRAIN_STRENGTH: f32 = 0.03;
/// Barrel-warp amount for the sampling shader: a gentle bow that reads as a tube
/// without pushing corner text past readability (curvature-vs-readability, tuned
/// by playtest). Bloom is the soft green glyph halo.
const NOVA_OS_CRT_WARP: f32 = 0.12;
const NOVA_OS_CRT_BLOOM: f32 = 0.85;

/// Degauss pulse duration in seconds (task 20260727-014148): how long the coil
/// wobble+flash rings out after an app launch/exit/switch. Short enough to feel
/// like a physical coil settle, long enough to read. The envelope is
/// `remaining / NOVA_OS_DEGAUSS_DURATION`, fed to the shader's `degauss` uniform.
const NOVA_OS_DEGAUSS_DURATION: f32 = 0.45;

/// Global stacking-context z for the OPEN NOVA OS: it is a modal, so backdrop and
/// panel rise above the flight HUD chrome (which carries no `GlobalZIndex` = 0).
/// Same modal tier the pause overlay uses (`nova_menu`); the NOVA OS and the
/// pause menu are mutually exclusive `PauseStates` variants, so sharing the tier
/// is fine. The tab handle stays at the HUD z (it is chrome). Task 20260724-121541.
const DRAWER_BACKDROP_Z: i32 = 10;
const DRAWER_PANEL_Z: i32 = 11;
/// z for NOVA OS-exempt diagnostic/status chrome that stays visible while the
/// NOVA OS is open: it must sit above the deepened backdrop so the gray field
/// cannot dim it. Read by status widgets that tag themselves
/// [`super::HudNovaOsExempt`].
pub(crate) const DRAWER_EXEMPT_Z: i32 = 12;

/// The NOVA OS UI root whose visibility is driven by [`NovaOsOpenness`].
#[derive(Component)]
struct NovaOsRootMarker;

/// The single physical NOVA OS monitor root.
#[derive(Component)]
struct NovaOsMonitorMarker;

/// The recessed physical bezel around the phosphor screen.
#[derive(Component)]
struct NovaOsBezelMarker;

/// The active green phosphor screen surface.
#[derive(Component)]
struct NovaOsScreenMarker;

/// The terminal placeholder content under the CRT overlay stack.
#[derive(Component)]
struct NovaOsTerminalContentMarker;

/// The PoC top bar row inside the screen.
#[derive(Component)]
struct NovaOsTopbarMarker;

/// The lit square lamp to the left of the NOVA OS brand.
#[derive(Component)]
struct NovaOsLampMarker;

/// Right-side status text row in the PoC top bar.
#[derive(Component)]
struct NovaOsStatusMarker;

/// The single terminal surface that fills the monitor screen.
#[derive(Component)]
struct NovaOsTerminalSurfaceMarker;

/// Scrollback rows printed by the NOVA OS terminal shell.
#[derive(Component)]
struct NovaOsTerminalScrollbackMarker;

/// Prompt row at the bottom of the terminal surface.
#[derive(Component)]
struct NovaOsPromptRowMarker;

/// Horizontal command-entry line inside the prompt strip.
#[derive(Component)]
struct NovaOsPromptInputLineMarker;

/// The fixed amber `nova>` prompt prefix.
#[derive(Component)]
struct NovaOsPromptPrefixMarker;

/// Remaining-width input lane that owns typed prompt and ghost completion.
#[derive(Component)]
struct NovaOsPromptInputWrapMarker;

/// Typed prompt text LEFT of the caret, owned by the terminal shell.
#[derive(Component)]
struct NovaOsTerminalPromptMarker;

/// Typed prompt text RIGHT of the caret (empty when the caret is at the end).
#[derive(Component)]
struct NovaOsTerminalPromptAfterMarker;

/// The block caret rendered between the before/after prompt text.
#[derive(Component)]
struct NovaOsTerminalCaretMarker;

/// Hint/status line owned by the terminal shell.
#[derive(Component)]
struct NovaOsTerminalHintMarker;

/// Ghost completion suffix rendered inline beside the typed prompt.
#[derive(Component)]
struct NovaOsTerminalGhostMarker;

/// The footer hint row from the PoC.
#[derive(Component)]
struct NovaOsFooterHintsMarker;

/// One of the four moulded corner screws on the casing (PoC `.screw`).
#[derive(Component)]
struct NovaOsScrewMarker;

/// The top-centre vent strip on the casing (PoC `.vents`).
#[derive(Component)]
struct NovaOsVentMarker;

/// The inset moulding-seam outline inside the casing (PoC `.case::after`).
#[derive(Component)]
struct NovaOsSeamMarker;

/// The bottom casing chin strip below the bezel (PoC `.chin`).
#[derive(Component)]
struct NovaOsChinMarker;

/// The recessed brand plate on the chin's left (PoC `.plate`).
#[derive(Component)]
struct NovaOsBrandPlateMarker;

/// The controls row on the chin's right, carrying the BRIGHT/SCAN knobs and the
/// SND/PWR buttons (task 20260726-214617).
#[derive(Component)]
struct NovaOsControlsRowMarker;

/// Player-tunable NOVA OS monitor hardware state, driven by the physical chin
/// controls (task 20260726-214617): the BRIGHT/SCAN knob detents and the SND
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
    /// Whether the monitor speaker is armed (the SND button; consumed by the
    /// NOVA OS sound task 20260726-214639).
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

    /// The dial-pointer angle for a knob's current detent.
    fn dial_angle(&self, knob: NovaOsKnob) -> f32 {
        let index = match knob {
            NovaOsKnob::Bright => self.bright_detent,
            NovaOsKnob::Scan => self.scan_detent,
        };
        NOVA_OS_KNOB_ANGLES[index.min(NOVA_OS_KNOB_ANGLES.len() - 1)]
    }

    /// Advance a knob to its next detent, wrapping (PoC `(index + 1) % len`).
    fn cycle(&mut self, knob: NovaOsKnob) {
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
enum NovaOsKnob {
    /// The BRIGHT knob (screen brightness multiply).
    Bright,
    /// The SCAN knob (scanline depth).
    Scan,
}

/// The rotating dial pointer inside a knob (a child of the knob button). Carries
/// the [`NovaOsKnob`] it belongs to so the sync system rotates the right one.
#[derive(Component)]
struct NovaOsKnobDialMarker;

/// The SND speaker toggle button on the chin.
#[derive(Component)]
struct NovaOsSoundButtonMarker;

/// The lit/unlit green bulb inside the SND button: its colour (not the label
/// text) is the on/off state.
#[derive(Component)]
struct NovaOsSoundIndicatorMarker;

/// The PWR power LED: green while the monitor is on, flashing orange while it
/// powers down (driven by [`drive_nova_os_power_led`]).
#[derive(Component)]
struct NovaOsPowerLedMarker;

/// The PWR button on the chin (the diegetic twin of the `exit` command).
#[derive(Component)]
struct NovaOsPowerButtonMarker;

/// The bright phosphor rim tracing the screen edge (PoC `.rim` line/glow pair).
#[derive(Component)]
struct NovaOsPhosphorRimMarker;

/// The glass specular sheen laid over the screen (PoC `.glass`).
#[derive(Component)]
struct NovaOsGlassMarker;

/// The root of the active NOVA OS app, spawned as a sibling of the terminal
/// content while [`TerminalMode::App`] is active. Carries the running app's id so
/// [`sync_nova_os_app_ui`] can tell a launch/exit/switch apart from a no-op.
#[derive(Component)]
struct NovaOsAppRoot {
    id: &'static str,
}

/// The on-screen close control in an app's chrome bar; clicking it exits the app
/// back to the terminal, mirroring the Escape route.
#[derive(Component)]
struct NovaOsAppCloseMarker;

/// The dim full-screen backdrop behind the panel.
#[derive(Component)]
struct NovaOsBackdropMarker;

/// The container the objectives-section lines are (re)built into.
#[derive(Component)]
struct NovaOsObjectivesListMarker;

/// Scrollable viewport around a NOVA OS row list.
#[derive(Component)]
struct NovaOsScrollViewportMarker;

/// One objective row in the NOVA OS's mission-log list.
#[derive(Component)]
struct NovaOsObjectiveRowMarker;

/// Objective id copied onto each NOVA OS row for rebuild and tests.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
struct NovaOsObjectiveId(String);

/// Whether a row is still active or retained as completed history.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum NovaOsObjectiveRowStatus {
    Active,
}

/// The small status glyph at the start of a NOVA OS objective row.
#[derive(Component)]
struct NovaOsObjectiveGlyphMarker;

/// The text entity for a NOVA OS objective row.
#[derive(Component)]
struct NovaOsObjectiveTextMarker;

/// Thin overlay used as a completed row's line-through.
#[cfg(test)]
#[derive(Component)]
struct NovaOsObjectiveStrikeMarker;

/// Styled empty-state row for the objective list.
#[derive(Component)]
struct NovaOsObjectiveEmptyMarker;

/// The container the combined left-panel flight log is rebuilt into.
#[derive(Component)]
struct NovaOsFlightLogListMarker;

/// One row in the left-panel combined flight log stream.
#[derive(Component)]
struct NovaOsFlightLogRowMarker;

/// Text entity for a combined flight log row.
#[derive(Component)]
struct NovaOsFlightLogTextMarker;

/// Styled empty-state row for the combined flight log.
#[derive(Component)]
struct NovaOsFlightLogEmptyMarker;

/// Icon semantics for a combined flight log row.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct NovaOsFlightLogIconMarker {
    kind: NovaOsFlightLogIconKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NovaOsFlightLogIconKind {
    CommsAuthored,
    Fallback,
    Objective,
}

/// Openness in `[0, 1]`: 0 fully closed (off-screen past the panel's edge), 1
/// fully open (flush with that edge). Eased toward the state-driven target with
/// real time so it keeps moving while the sim is frozen.
#[derive(Component, Default)]
struct NovaOsOpenness(f32);

/// True after the user requested close; gameplay remains paused until the
/// real-time close animation reaches zero.
#[derive(Resource, Default)]
struct NovaOsCloseTransition {
    closing: bool,
}

/// NovaOs-local combined flight log derived from [`StoryFeed`] and
/// [`GameObjectives`].
///
/// The monitor placeholder keeps the historical stream: comms rows plus
/// objective posted/completed rows, in the order the HUD observes them.
/// Objective text updates edit the open posted row rather than appending
/// duplicate events.
#[derive(Resource, Default, Debug, Clone)]
struct NovaOsFlightLog {
    entries: Vec<NovaOsFlightLogEntry>,
    active_objective_entries: Vec<NovaOsFlightLogActiveObjective>,
    previous_active: Vec<Objective>,
    seen_story: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NovaOsFlightLogActiveObjective {
    id: String,
    entry_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct NovaOsFlightLogEntry {
    kind: NovaOsFlightLogEntryKind,
    objective_id: Option<String>,
    speaker: Option<String>,
    message: String,
    icon: Option<AssetRef<Image>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NovaOsFlightLogEntryKind {
    Comms,
    ObjectivePosted,
    ObjectiveCompleted,
}

/// The live degauss pulse (task 20260727-014148). [`sync_nova_os_app_ui`] resets
/// `remaining` to [`NOVA_OS_DEGAUSS_DURATION`] on every real app
/// launch/exit/switch (the same transitions that play the `NovaOsCoil` coil
/// sound, so the wobble+flash lands with the thump); [`animate_nova_os_crt`]
/// bleeds it down by real time and feeds the `remaining / DURATION` envelope into
/// the CRT `degauss` uniform. Real time because the sim clock is frozen while the
/// computer is open.
#[derive(Resource, Default, Debug)]
struct NovaOsDegauss {
    remaining: f32,
}

impl NovaOsDegauss {
    /// Kick the coil: restart the full pulse.
    fn pulse(&mut self) {
        self.remaining = NOVA_OS_DEGAUSS_DURATION;
    }

    /// The 0..1 shader envelope for the current `remaining`.
    fn envelope(&self) -> f32 {
        (self.remaining / NOVA_OS_DEGAUSS_DURATION).clamp(0.0, 1.0)
    }
}

#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
struct NovaOsCrtMaterial {
    #[uniform(0)]
    data: NovaOsCrtUniform,
    /// The offscreen image holding the rendered terminal content. The sampling
    /// shader reads it to bloom the glyphs and barrel-warp the content. A default
    /// (white 1x1) handle in headless rigs keeps the material valid.
    #[texture(1)]
    #[sampler(2)]
    source: Handle<Image>,
}

#[derive(ShaderType, Clone, Debug)]
struct NovaOsCrtUniform {
    tint: LinearRgba,
    /// The CRT panel's pixel size, updated each frame by [`animate_nova_os_crt`]
    /// from the screen node's [`ComputedNode`] so the scanlines/slot-mask + bloom
    /// taps track the real screen size. Zero until the first layout pass feeds it.
    resolution: Vec2,
    scanline_strength: f32,
    vignette_strength: f32,
    glow_strength: f32,
    grain_strength: f32,
    /// Real-time seconds, updated each frame by [`animate_nova_os_crt`] so the
    /// grain shimmers gently.
    time: f32,
    /// Rounded-corner radius in screen pixels. A UI `MaterialNode` is NOT
    /// clipped by its node's [`BorderRadius`], so the shader masks its own
    /// corners to the screen's rounding (no green bleed past the rounded edge).
    /// Zero disables the mask (headless/other rigs).
    corner_radius: f32,
    /// Barrel-distortion amount (0 = flat) - bows the sampled content.
    warp: f32,
    /// Bloom strength (halo of the bright green glyphs).
    bloom: f32,
    /// Power level 0..1 fed from [`NovaOsOpenness`]: 1 full raster, 0 collapsed to
    /// a dying line/dot. Drives the CRT power-on/off collapse.
    power: f32,
    /// Extra brightness multiply (1.0 neutral). Reserved for task 214617's BRIGHT
    /// knob. Appended last so the field order still matches the WGSL struct.
    brightness: f32,
    /// Degauss envelope 0..1 (task 20260727-014148): pulsed to 1 on an app
    /// launch/exit/switch by [`sync_nova_os_app_ui`] via [`NovaOsDegauss`] and
    /// decayed back to 0 by [`animate_nova_os_crt`]. Drives the shader's wobble +
    /// flash. Appended last so the field order still matches the WGSL struct
    /// (trailing `f32` after `brightness` - no alignment hole,
    /// `shader-uniform-field-order-must-match-wgsl`).
    degauss: f32,
}

/// Loads the NOVA OS terminal font, a TrueType Collection (`.ttc`). Bevy's
/// built-in `FontLoader` only advertises `ttf`/`otf`, so without this loader
/// nothing claims the `ttc` extension and the font never loads. It is `pub` so
/// the web `.meta` generator (`nova_meta_gen`) can register the SAME loader
/// type and emit a `...ttc.meta` naming it - otherwise the font ships with no
/// sidecar and, under `AssetMetaCheck::Always`, the web build fails the
/// missing-meta fetch and renders every NOVA OS glyph invisible
/// (task 20260727-172205).
#[derive(Default, TypePath)]
pub struct NovaOsTtcFontLoader;

impl AssetLoader for NovaOsTtcFontLoader {
    type Asset = Font;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(Font::from_bytes(bytes))
    }

    fn extensions(&self) -> &[&str] {
        &["ttc"]
    }
}

impl Default for NovaOsCrtMaterial {
    fn default() -> Self {
        Self {
            data: NovaOsCrtUniform {
                tint: NOVA_OS_CRT_TINT,
                resolution: Vec2::ZERO,
                scanline_strength: NOVA_OS_CRT_SCANLINE_STRENGTH,
                vignette_strength: NOVA_OS_CRT_VIGNETTE_STRENGTH,
                glow_strength: NOVA_OS_CRT_GLOW_STRENGTH,
                grain_strength: NOVA_OS_CRT_GRAIN_STRENGTH,
                time: 0.0,
                corner_radius: NOVA_OS_SCREEN_RADIUS_PX,
                warp: NOVA_OS_CRT_WARP,
                bloom: NOVA_OS_CRT_BLOOM,
                // Start collapsed; the openness driver blooms it on.
                power: 0.0,
                brightness: 1.0,
                // Idle: no degauss until an app launch/exit pulses it.
                degauss: 0.0,
            },
            source: Handle::default(),
        }
    }
}

impl UiMaterial for NovaOsCrtMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/nova_os_crt.wgsl".into()
    }
}

// ---------------------------------------------------------------------------
// Render-to-texture CRT pipeline (task 20260726-193233).
//
// The terminal-content subtree renders to an offscreen image via a dedicated UI
// camera; the screen node then displays that image through the sampling
// `NovaOsCrtMaterial` (bloom + barrel warp + the whole CRT treatment). Interaction
// is preserved by FORWARDING a custom pointer whose location targets the image
// (bevy 0.19 `ui_picking` matches pointers to cameras by render target), plus a
// hover-mirror system because `bevy_picking::update_is_hovered` only tracks the
// mouse pointer. See `tasks/20260726-193233/NOTES.md`.
// ---------------------------------------------------------------------------

/// The dedicated UI camera + content subtree live on this render layer so the
/// image camera draws ONLY the terminal UI, never stray world 2D sprites (the
/// render-scale upscale sprite sits on the default layer 0).
const NOVA_OS_RTT_LAYER: usize = 20;
/// Camera order for the offscreen pass: well before the window/UI cameras so the
/// sampled image is ready when the screen surface reads it.
const NOVA_OS_RTT_CAMERA_ORDER: isize = -20;

#[derive(Component)]
struct NovaOsImageCameraMarker;

#[derive(Component)]
struct NovaOsImageContentRootMarker;

/// The screen-node surface that samples the offscreen image through the CRT shader.
#[derive(Component)]
struct NovaOsSamplingSurfaceMarker;

#[derive(Component)]
struct NovaOsForwardedPointerMarker;

/// Handles/entities of the live NOVA OS's RTT pipeline. Present only on
/// render-capable builds (an `Assets<Image>` + `Assets<NovaOsCrtMaterial>` exist);
/// absent headless, where the terminal renders directly on the screen node.
#[derive(Resource)]
struct NovaOsRtt {
    image: Handle<Image>,
    camera: Entity,
    content_root: Entity,
    pointer: Entity,
}

/// Stable id for the forwarded pointer (one NOVA OS at a time).
fn nova_os_pointer_id() -> PointerId {
    PointerId::Custom(Uuid::from_u128(0x0BADC0DE_CAFE_1234_5678_9ABCDEF01234))
}

fn nova_os_image_target(image: &Handle<Image>) -> NormalizedRenderTarget {
    NormalizedRenderTarget::Image(ImageRenderTarget {
        handle: image.clone(),
        scale_factor: 1.0,
    })
}

fn nova_os_new_target_image(size: UVec2) -> Image {
    Image::new_target_texture(
        size.x.max(1),
        size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    )
}

/// Keep the offscreen image sized to the screen node's physical pixels and the
/// content root sized to match, so window resizes / relayouts never show a
/// stretched frame (mirrors `render_scale.rs`). Deactivate the offscreen pass and
/// hide the content while the NOVA OS is fully closed so it costs nothing.
#[allow(clippy::type_complexity)]
fn reconcile_nova_os_target(
    rtt: Option<Res<NovaOsRtt>>,
    mut images: ResMut<Assets<Image>>,
    q_screen: Query<&ComputedNode, With<NovaOsScreenMarker>>,
    q_openness: Query<&NovaOsOpenness, With<NovaOsRootMarker>>,
    mut q_camera: Query<(&mut Camera, &mut Projection), With<NovaOsImageCameraMarker>>,
    mut q_root: Query<(&mut Node, &mut Visibility), With<NovaOsImageContentRootMarker>>,
) {
    let Some(rtt) = rtt else {
        return;
    };
    let camera = rtt.camera;
    let Ok(computed) = q_screen.single() else {
        return;
    };
    // ComputedNode.size is physical pixels; the image target renders 1:1 at that
    // size (scale_factor 1.0), so content laid out at the image logical size lines
    // up with the sampled surface.
    let desired = computed.size().round().as_uvec2().max(UVec2::ONE);
    let open = q_openness.iter().next().map(|o| o.0).unwrap_or(0.0);

    let needs_resize = images
        .get(&rtt.image)
        .map(|img| img.size() != desired)
        .unwrap_or(true);
    if needs_resize {
        if let Some(mut img) = images.get_mut(&rtt.image) {
            img.resize(bevy::render::render_resource::Extent3d {
                width: desired.x,
                height: desired.y,
                depth_or_array_layers: 1,
            });
        }
        // Force the camera to re-derive its target info after the swap
        // (`bevy-camera-ignores-runtime-rendertarget-swap`).
        if let Ok((_, mut projection)) = q_camera.get_mut(camera) {
            projection.set_changed();
        }
    }

    if let Ok((mut cam, _)) = q_camera.get_mut(camera) {
        // No point rendering the offscreen pass when the NOVA OS is fully closed.
        cam.is_active = open > f32::EPSILON;
    }
    if let Ok((mut node, mut vis)) = q_root.single_mut() {
        node.width = Val::Px(desired.x as f32);
        node.height = Val::Px(desired.y as f32);
        *vis = if open > f32::EPSILON {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Forward the real mouse cursor onto the offscreen image so the terminal UI
/// stays hoverable/clickable through the sampled surface: map the cursor into the
/// screen node's rect, invert the barrel warp, scale into image pixels, write the
/// custom pointer's location, and mirror mouse button presses as `PointerInput`.
#[allow(clippy::type_complexity)]
fn forward_nova_os_pointer(
    rtt: Option<Res<NovaOsRtt>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut mouse_buttons: MessageReader<bevy::input::mouse::MouseButtonInput>,
    q_surface: Query<(&ComputedNode, &UiGlobalTransform), With<NovaOsSamplingSurfaceMarker>>,
    mut q_pointer: Query<&mut PointerLocation, With<NovaOsForwardedPointerMarker>>,
    mut pointer_inputs: MessageWriter<PointerInput>,
    images: Res<Assets<Image>>,
) {
    let Some(rtt) = rtt else {
        return;
    };
    let Ok(mut loc) = q_pointer.get_mut(rtt.pointer) else {
        return;
    };
    let image_size = images
        .get(&rtt.image)
        .map(|i| i.size().as_vec2())
        .unwrap_or(Vec2::ONE);

    let cursor = windows.single().ok().and_then(|w| w.cursor_position());
    let surface = q_surface.single().ok();
    let in_image = match (cursor, surface) {
        (Some(cursor), Some((node, xf))) => {
            let size = node.size();
            let min = xf.translation - size * 0.5;
            let local = (cursor - min) / size.max(Vec2::splat(1.0));
            if local.x < 0.0 || local.x > 1.0 || local.y < 0.0 || local.y > 1.0 {
                None
            } else {
                Some(nova_os_inverse_barrel(local, NOVA_OS_CRT_WARP) * image_size)
            }
        }
        _ => None,
    };

    // Park off-image when the cursor is not over the panel so nothing is hovered.
    let position = in_image.unwrap_or(Vec2::splat(-1000.0));
    loc.location = Some(Location {
        target: nova_os_image_target(&rtt.image),
        position,
    });

    // Mirror mouse buttons onto the forwarded pointer (only meaningful over the
    // panel; harmless otherwise since the position is parked off-image).
    let id = nova_os_pointer_id();
    for ev in mouse_buttons.read() {
        let button = match ev.button {
            MouseButton::Left => PointerButton::Primary,
            MouseButton::Right => PointerButton::Secondary,
            MouseButton::Middle => PointerButton::Middle,
            _ => continue,
        };
        let action = match ev.state {
            ButtonState::Pressed => PointerAction::Press(button),
            ButtonState::Released => PointerAction::Release(button),
        };
        pointer_inputs.write(PointerInput::new(
            id,
            Location {
                target: nova_os_image_target(&rtt.image),
                position,
            },
            action,
        ));
    }
}

/// Inverse of the shader's forward barrel warp, so a hovered on-screen point maps
/// back to the glyph actually under it.
fn nova_os_inverse_barrel(uv: Vec2, amount: f32) -> Vec2 {
    let c = uv - Vec2::splat(0.5);
    let r2 = c.length_squared();
    Vec2::splat(0.5) + c / (1.0 + amount * r2)
}

/// `bevy_picking::update_is_hovered` only mirrors the MOUSE pointer into `Hovered`
/// components, so replicate its ancestor walk for our forwarded pointer - else the
/// terminal's `Hovered`-gated wheel scroll would go dead through the image.
///
/// CRUCIALLY, this only manages `Hovered` on entities rendered THROUGH the image
/// (descendants of the content root). Window-space UI - the chin knobs
/// (task 214617), menus, any `Button` - keep the `Hovered` the MOUSE pointer's
/// `update_is_hovered` owns; touching them here would force `Hovered(false)` every
/// frame the NOVA OS is open (the forwarded pointer's HoverMap targets the image,
/// never the window), fighting the real cursor.
fn mirror_nova_os_hover(
    rtt: Option<Res<NovaOsRtt>>,
    hover_map: Option<Res<HoverMap>>,
    parents: Query<&ChildOf>,
    mut hovers: Query<(Entity, &Hovered)>,
    mut commands: Commands,
) {
    let Some(rtt) = rtt else {
        return;
    };
    let Some(hover_map) = hover_map else {
        return;
    };
    if hovers.is_empty() {
        return;
    }
    let mut hovered_set = bevy::platform::collections::HashSet::new();
    if let Some(hits) = hover_map.get(&nova_os_pointer_id()) {
        for entity in hits.keys() {
            hovered_set.insert(*entity);
            hovered_set.extend(parents.iter_ancestors(*entity));
        }
    }
    for (entity, hovered) in hovers.iter_mut() {
        // Only entities under the offscreen content root are served by the
        // forwarded pointer; never touch window-space `Hovered`.
        let through_image = entity == rtt.content_root
            || parents
                .iter_ancestors(entity)
                .any(|a| a == rtt.content_root);
        if !through_image {
            continue;
        }
        let is_hovering = hovered_set.contains(&entity);
        if hovered.get() != is_hovering {
            commands.entity(entity).insert(Hovered(is_hovering));
        }
    }
}

// Order and summaries mirror `nova_os_terminal_poc.html`'s command list. `map`
// and `ship viewer` from the PoC stay out until their stretch app tasks land.

fn nova_os_ship_name(name: Option<&Name>) -> String {
    name.map(|name| name.as_str().to_uppercase())
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

/// The FPS number formatted to a FIXED width so the topbar does not reflow when
/// the reading changes digit count (owner playtest: 100 -> 99 must not shift the
/// layout). Right-aligned to 3 chars in the monospace topbar font, so ` 99` and
/// `100` occupy the same width; `--` before the first reading pads the same way.
fn nova_os_fps_segment(fps: Option<u32>) -> String {
    match fps {
        Some(fps) => format!("{fps:>3}"),
        None => format!("{:>3}", "--"),
    }
}

/// The NOVA OS topbar status line: ship + link, plus a live FPS segment. The FPS
/// is rehomed here from the flight status bar, which hides while the computer is
/// open (task 20260727-014806); `fps` is the smoothed frame rate rounded to a
/// whole number, or `None` before the diagnostic has a reading (shown as `--`).
fn nova_os_status_text(ship_name: &str, fps: Option<u32>) -> String {
    let fps = nova_os_fps_segment(fps);
    format!("SHIP: {ship_name}     LINK: LOCAL     FPS: {fps}")
}

impl NovaOsFlightLog {
    fn clear(&mut self) {
        self.entries.clear();
        self.active_objective_entries.clear();
        self.previous_active.clear();
        self.seen_story = 0;
    }
}

fn terminal_snapshot_from_world(
    log: &NovaOsFlightLog,
    objectives: &GameObjectives,
    ship_name: Option<&str>,
    ship_sections: &[ShipSectionStatus],
    seen_events: usize,
) -> TerminalCommandSnapshot {
    let unread_events = log.entries.len().saturating_sub(seen_events);
    TerminalCommandSnapshot {
        log_rows: terminal_log_rows(log),
        objective_rows: terminal_objective_rows(objectives),
        ship_rows: terminal_ship_rows(ship_name, ship_sections),
        unread_events,
        unread_hook: nova_os_unread_hook(log, seen_events),
    }
}

/// A short lead-in for the most recent unread flight-log entry, used by the boot
/// banner's unread-events line. `None` when nothing is unread.
fn nova_os_unread_hook(log: &NovaOsFlightLog, seen_events: usize) -> Option<String> {
    log.entries
        .get(seen_events..)
        .and_then(|unread| unread.last())
        .map(|entry| entry.message.clone())
}

fn terminal_log_rows(log: &NovaOsFlightLog) -> Vec<TerminalRow> {
    if log.entries.is_empty() {
        return vec![TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "Flight log is empty.".to_string(),
        }];
    }
    // HTML-style log: each entry gets a 4-digit sequential index prefix
    // (`0001 COMMS ... > ...`, `0003 OBJ + ...`) with no separate header.
    log.entries
        .iter()
        .enumerate()
        .map(|(index, entry)| TerminalRow {
            kind: match entry.kind {
                NovaOsFlightLogEntryKind::Comms => TerminalRowKind::Output,
                NovaOsFlightLogEntryKind::ObjectivePosted => TerminalRowKind::Warn,
                NovaOsFlightLogEntryKind::ObjectiveCompleted => TerminalRowKind::Info,
            },
            text: format!("{:04} {}", index + 1, nova_os_flight_log_text(entry)),
        })
        .collect()
}

fn terminal_objective_rows(objectives: &GameObjectives) -> Vec<TerminalRow> {
    if objectives.objectives.is_empty() {
        return vec![TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "No active objectives.".to_string(),
        }];
    }
    // HTML-style objectives: one `OBJ + <message>` row each, no header.
    objectives
        .objectives
        .iter()
        .map(|objective| TerminalRow {
            kind: TerminalRowKind::Warn,
            text: format!("OBJ + {}", objective.message),
        })
        .collect()
}

#[derive(Debug, Clone)]
struct ShipSectionStatus {
    name: String,
    kind: SectionDamageClass,
    health: Option<Health>,
    inactive: bool,
    zero_health: bool,
    ammo: Option<SectionAmmo>,
}

fn terminal_ship_rows(ship_name: Option<&str>, sections: &[ShipSectionStatus]) -> Vec<TerminalRow> {
    if sections.is_empty() {
        return vec![
            TerminalRow {
                kind: TerminalRowKind::Info,
                text: format!("SHIP {}", terminal_ship_name(ship_name)),
            },
            TerminalRow {
                kind: TerminalRowKind::Dim,
                text: "No live player ship sections detected.".to_string(),
            },
        ];
    }

    let mut rows = vec![
        TerminalRow {
            kind: TerminalRowKind::Info,
            text: format!("SHIP {}", terminal_ship_name(ship_name)),
        },
        TerminalRow {
            kind: TerminalRowKind::Dim,
            text: format!("Sections: {}", sections.len()),
        },
    ];
    for section in sections {
        let status = section_status_label(section);
        rows.push(TerminalRow {
            kind: section_status_row_kind(section),
            text: format!(
                "{} {} - {}{}",
                section_kind_label(section.kind),
                section.name,
                section_health_text(section.health.as_ref()),
                section_ammo_suffix(section.ammo.as_ref())
            ),
        });
        if status != "nominal" {
            rows.push(TerminalRow {
                kind: section_status_row_kind(section),
                text: format!("  status: {status}"),
            });
        }
    }
    rows
}

fn terminal_ship_name(name: Option<&str>) -> String {
    name.map(str::to_uppercase)
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

fn section_kind_label(kind: SectionDamageClass) -> &'static str {
    match kind {
        SectionDamageClass::Hull => "HULL",
        SectionDamageClass::Thruster => "THRUSTER",
        SectionDamageClass::Controller => "CONTROLLER",
        SectionDamageClass::Turret => "TURRET",
        SectionDamageClass::Torpedo => "TORPEDO",
    }
}

fn section_health_text(health: Option<&Health>) -> String {
    match health {
        Some(health) if health.max > 0.0 => {
            format!("{:.0}/{:.0} HP", health.current.max(0.0), health.max)
        }
        Some(health) => format!("{:.0} HP", health.current.max(0.0)),
        None => "HP unknown".to_string(),
    }
}

fn section_ammo_suffix(ammo: Option<&SectionAmmo>) -> String {
    ammo.map(|ammo| format!("; ammo {}/{}", ammo.rounds, ammo.capacity))
        .unwrap_or_default()
}

fn section_status_label(section: &ShipSectionStatus) -> &'static str {
    if section.inactive || section.zero_health {
        return "neutralized";
    }
    let Some(health) = section.health.as_ref() else {
        return "nominal";
    };
    if health.max > 0.0 && health.current / health.max <= 0.25 {
        "critical"
    } else {
        "nominal"
    }
}

fn section_status_row_kind(section: &ShipSectionStatus) -> TerminalRowKind {
    match section_status_label(section) {
        "neutralized" => TerminalRowKind::Error,
        "critical" => TerminalRowKind::Warn,
        _ => TerminalRowKind::Output,
    }
}

/// The reveal's tuck-target rect in logical pixels. This is task 20260721-211520's
/// tween TARGET: the big cockpit objective animates INTO this rect. It is
/// published each frame by `objective_hint` (the minimalist top-right hint
/// replaced the old NOVA OS tab handle as the anchor source - task
/// 20260724-134312). `None` until the hint has laid out at least once (headless
/// rigs without a UI layout pass leave it `None`).
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct NovaOsTabAnchor {
    /// The hint rect in logical window pixels, or `None` before first layout.
    pub rect: Option<Rect>,
}

/// Wires the Tab NOVA OS shell: the toggle, the slide and the objectives section.
/// The reveal's tuck anchor ([`NovaOsTabAnchor`]) is published by `objective_hint`.
/// Registered by [`super::NovaHudPlugin`].
pub struct NovaOsPlugin;

impl Plugin for NovaOsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NovaOsTabAnchor>();
        app.init_resource::<NovaOsFlightLog>();
        app.init_resource::<NovaOsTerminal>();
        app.init_resource::<NovaOsAppRegistry>();
        app.init_resource::<NovaOsCloseTransition>();
        app.init_resource::<NovaOsMonitorSettings>();
        app.init_resource::<NovaOsDegauss>();
        app.register_type::<NovaOsMonitorSettings>();
        app.register_asset_loader(NovaOsTtcFontLoader);
        app.add_plugins(UiMaterialPlugin::<NovaOsCrtMaterial>::default());

        // Tab opens the NOVA OS. It keeps running in all of Playing so an open
        // NOVA OS can reserve Tab for terminal completion instead of closing.
        app.add_systems(
            Update,
            (toggle_nova_os, close_nova_os_from_menu_keys)
                .chain()
                .run_if(in_state(GameStates::Playing)),
        );

        // Shell upkeep while the HUD is live: ease the slide and rebuild the
        // objectives section on change / first spawn. (The reveal's tuck anchor
        // is published by `objective_hint`.)
        app.add_systems(
            Update,
            (
                drive_nova_os_slide,
                (
                    sync_nova_os_logs,
                    rebuild_nova_os_objectives,
                    rebuild_nova_os_flight_log,
                )
                    .chain()
                    .run_if(
                        resource_changed::<GameObjectives>
                            .or_else(resource_changed::<StoryFeed>)
                            .or_else(nova_os_lists_just_spawned),
                    ),
            )
                .in_set(NovaHudSystems),
        );
        // Objective flips announce into the live terminal scrollback the moment
        // they happen while the computer is open (PoC `checkObjectives`). Runs
        // after `sync_nova_os_logs` so it sees the freshly appended entries.
        app.add_systems(
            Update,
            announce_objectives_in_terminal
                .after(sync_nova_os_logs)
                .run_if(resource_changed::<GameObjectives>)
                .in_set(NovaHudSystems),
        );
        app.add_systems(
            Update,
            scroll_nova_os_panels
                .run_if(in_state(PauseStates::NovaOs))
                .run_if(resource_exists::<Messages<bevy::input::mouse::MouseWheel>>)
                .in_set(NovaHudSystems),
        );
        app.add_systems(
            Update,
            (
                sync_nova_os_app_commands.run_if(
                    resource_changed::<NovaOsAppRegistry>.or_else(resource_added::<NovaOsTerminal>),
                ),
                handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
                handle_nova_os_app_keyboard.run_if(in_state(GameStates::Playing)),
                rebuild_terminal_ui
                    .run_if(resource_changed::<NovaOsTerminal>.or_else(terminal_ui_just_spawned)),
                rebuild_nova_os_footer_hints.run_if(
                    resource_changed::<NovaOsTerminal>.or_else(nova_os_footer_just_spawned),
                ),
                sync_nova_os_app_ui.run_if(in_state(PauseStates::NovaOs)),
            )
                .chain()
                .in_set(NovaHudSystems),
        );
        // Blink the caret, shimmer the CRT grain and refresh the topbar FPS on
        // real time (virtual time is paused while the computer is open).
        app.add_systems(
            Update,
            (
                blink_nova_os_caret,
                position_nova_os_block_caret,
                drain_nova_os_boot,
                drive_nova_os_topbar_fps,
                animate_nova_os_crt.run_if(resource_exists::<Assets<NovaOsCrtMaterial>>),
                sync_nova_os_monitor_controls.run_if(resource_changed::<NovaOsMonitorSettings>),
                drive_nova_os_power_led,
                // NOVA OS sound (task 20260726-214639): the power-down sweep on a
                // requested close and the live bed volume / SND mute.
                play_nova_os_power_down,
                apply_nova_os_bed_volume,
            )
                .run_if(in_state(PauseStates::NovaOs))
                .in_set(NovaHudSystems),
        );

        // Power-up sweep + ambient bed on open, bed teardown on close. Reuses the
        // exact OnEnter/OnExit(NovaOs) hooks the freeze axis uses. The first open
        // also kicks off the staggered boot banner, and each close records how
        // many flight-log events have been seen (for the unread-events count).
        app.add_systems(
            OnEnter(PauseStates::NovaOs),
            (start_nova_os_sound, begin_nova_os_boot),
        );
        app.add_systems(
            OnExit(PauseStates::NovaOs),
            (stop_nova_os_bed, mark_nova_os_events_seen),
        );

        // Render-to-texture pipeline: keep the offscreen image sized to the screen
        // (always, so it is ready when the NOVA OS opens), and while the computer is
        // open forward the pointer onto the image + mirror its hover so the
        // terminal stays interactive through the sampled surface. `mirror` runs
        // before the wheel scroll so its `Hovered` gate reads fresh state.
        app.add_systems(
            Update,
            reconcile_nova_os_target
                .run_if(resource_exists::<NovaOsRtt>)
                .in_set(NovaHudSystems),
        );
        app.add_systems(
            Update,
            (forward_nova_os_pointer, mirror_nova_os_hover)
                .chain()
                .before(scroll_nova_os_panels)
                .run_if(in_state(PauseStates::NovaOs))
                .run_if(resource_exists::<NovaOsRtt>)
                .in_set(NovaHudSystems),
        );

        // The NOVA OS is a flight surface: spawn/despawn it with the player ship,
        // like the rest of the HUD.
        app.add_observer(setup_nova_os);
        app.add_observer(remove_nova_os);
    }
}

/// Tab opens the shared freeze axis and becomes autocomplete while open. The
/// gamepad right-stick click still toggles `Unpaused <-> NovaOs`; both inputs are
/// inert while the pause menu owns the freeze (`Paused`) - which is also how a
/// live outcome (it forces `Paused`) blocks the NOVA OS without a cross-crate
/// dependency. The pad button is `RightThumb`, the one free button (task
/// 20260724-134312), mirroring `nova_menu`'s optional-gamepad guard.
fn toggle_nova_os(
    keys: Res<ButtonInput<KeyCode>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    mut close: ResMut<NovaOsCloseTransition>,
) {
    let pad = gamepad
        .map(|g| g.just_pressed(GamepadButton::RightThumb))
        .unwrap_or(false);
    let tab = keys.just_pressed(KeyCode::Tab);
    if !tab && !pad {
        return;
    }
    match current.get() {
        PauseStates::Unpaused => {
            close.closing = false;
            next.set(PauseStates::NovaOs);
        }
        PauseStates::NovaOs if pad && !tab => {
            close.closing = true;
        }
        PauseStates::NovaOs | PauseStates::Paused => {}
    }
}

/// Escape (and gamepad Start) is the single "back" gesture, interpreted here in
/// one place so there is no cross-system race over the key: while an app owns the
/// screen it exits the app back to the terminal; at the prompt it closes the whole
/// computer. Reading `active_mode` (not consuming the key edge elsewhere) is what
/// keeps the two routes from both firing on one press.
/// The looping ambient CRT bed audio entity, spawned while the NOVA OS computer
/// is open (task 20260726-214639). Its OWN marker keeps it out of
/// [`super::super::audio`]'s `pause_loops` thruster/RCS queries, so the sim-freeze
/// loop-pause on `OnEnter(NovaOs)` never silences it - the bed is exempt by
/// construction, not by a guard (`audit-state-gates-on-new-entry-path`). Volume
/// (and the live SND mute) is applied by [`apply_nova_os_bed_volume`].
#[derive(Component)]
struct NovaOsBedSfx;

/// Fire a one-shot NOVA OS terminal cue, honoring the SND toggle
/// ([`NovaOsMonitorSettings::sound_enabled`]). Master volume is applied
/// downstream by the SFX plugin's `SfxMasterVolume` path, like every other cue.
fn play_nova_os_cue(
    commands: &mut Commands,
    bank: &SoundBank<UiSfx>,
    settings: &NovaOsMonitorSettings,
    cue: UiSfx,
    volume: f32,
) {
    if !settings.sound_enabled {
        return;
    }
    commands.play_sfx_volume(bank.get(cue), volume);
}

/// Power-up sweep + start the ambient bed when the computer opens
/// (`OnEnter(NovaOs)`). The bed spawns even when SND is off (silent) so toggling
/// SND on mid-session brings the hum in without reopening.
fn start_nova_os_sound(
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    settings: Res<NovaOsMonitorSettings>,
) {
    let Some(bank) = bank else {
        return;
    };
    play_nova_os_cue(
        &mut commands,
        &bank,
        &settings,
        UiSfx::NovaOsPowerUp,
        NOVA_OS_POWER_VOLUME,
    );
    commands.spawn((
        Name::new("NOVA OS Ambient Bed"),
        NovaOsBedSfx,
        AudioPlayer(bank.get(UiSfx::NovaOsBed)),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
    ));
}

/// Despawn the ambient bed when the computer closes (`OnExit(NovaOs)`, i.e. once
/// the power-down collapse finishes).
fn stop_nova_os_bed(mut commands: Commands, q_bed: Query<Entity, With<NovaOsBedSfx>>) {
    for entity in &q_bed {
        commands.entity(entity).despawn();
    }
}

/// Play the power-down sweep the instant a close is REQUESTED (the rising edge of
/// [`NovaOsCloseTransition::closing`]), so the sweep syncs with the raster
/// collapse that starts then - not `OnExit(NovaOs)`, which fires only after the
/// collapse animation completes.
fn play_nova_os_power_down(
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    settings: Res<NovaOsMonitorSettings>,
    close: Res<NovaOsCloseTransition>,
    mut was_closing: Local<bool>,
) {
    if close.closing && !*was_closing {
        if let Some(bank) = &bank {
            play_nova_os_cue(
                &mut commands,
                bank,
                &settings,
                UiSfx::NovaOsPowerDown,
                NOVA_OS_POWER_VOLUME,
            );
        }
    }
    *was_closing = close.closing;
}

/// Drive the ambient bed sink volume from [`MasterVolume`] and the SND toggle, so
/// muting SND (or the master) silences the hum live without despawning the loop.
/// Uses `output_gain(mute)` like the thruster/RCS loop sinks, so a `HarnessMute`d
/// smoke/probe run silences the bed too (a per-frame sink write bypasses the
/// `GlobalVolume` path that mute otherwise masks).
fn apply_nova_os_bed_volume(
    settings: Res<NovaOsMonitorSettings>,
    master: Option<Res<MasterVolume>>,
    mute: Option<Res<HarnessMute>>,
    mut q_bed: Query<&mut AudioSink, With<NovaOsBedSfx>>,
) {
    let mute = mute.map(|m| *m).unwrap_or_default();
    let master = master.map(|m| m.output_gain(mute)).unwrap_or(1.0);
    let target = nova_os_bed_gain(settings.sound_enabled, master);
    for mut sink in &mut q_bed {
        sink.set_volume(Volume::Linear(target));
    }
}

/// The ambient bed's target sink gain: the base volume scaled by the master
/// output gain, or ZERO when SND is muted. Pure so the SND-off / master / mute
/// silence logic is testable without an `AudioSink` (which needs an audio
/// device). `master` is already the `output_gain(mute)`, so a harness-muted run
/// (master 0) silences the bed too.
fn nova_os_bed_gain(sound_enabled: bool, master: f32) -> f32 {
    if sound_enabled {
        NOVA_OS_BED_VOLUME * master
    } else {
        0.0
    }
}

fn close_nova_os_from_menu_keys(
    keys: Res<ButtonInput<KeyCode>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    current: Res<State<PauseStates>>,
    mut close: ResMut<NovaOsCloseTransition>,
    mut terminal: ResMut<NovaOsTerminal>,
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    settings: Res<NovaOsMonitorSettings>,
) {
    if *current.get() != PauseStates::NovaOs {
        return;
    }
    // This is the ONE owner of the back-out / app-exit gestures (per the
    // `context-key-handled-in-one-owner` lesson): Escape, gamepad Start, and the
    // Ctrl+C / Ctrl+[ app-exit chord are all branched on `active_mode` here, so a
    // single press can never both exit an app and close the NOVA OS.
    let start = gamepad
        .map(|g| g.just_pressed(GamepadButton::Start))
        .unwrap_or(false);
    let escape = keys.just_pressed(KeyCode::Escape) || start;
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    // Ctrl+C / Ctrl+[ exit a running app back to the terminal (PoC's chord).
    let ctrl_exit =
        ctrl && (keys.just_pressed(KeyCode::KeyC) || keys.just_pressed(KeyCode::BracketLeft));
    if !(escape || ctrl_exit) {
        return;
    }

    let exit_app_with_coil = |terminal: &mut NovaOsTerminal, commands: &mut Commands| {
        if terminal.exit_app() {
            // The degauss coil is the app-exit twin of the launch coil.
            if let Some(bank) = &bank {
                play_nova_os_cue(
                    commands,
                    bank,
                    &settings,
                    UiSfx::NovaOsCoil,
                    NOVA_OS_COIL_VOLUME,
                );
            }
        }
    };

    // Shift+Esc is the escape hatch: close the whole computer even from inside an
    // app (PoC's `Shift+Esc`). Plain Escape/Start backs out one level.
    if escape && shift {
        close.closing = true;
        return;
    }
    if ctrl_exit {
        exit_app_with_coil(&mut terminal, &mut commands);
        return;
    }
    match terminal.active_mode() {
        TerminalMode::App { .. } => exit_app_with_coil(&mut terminal, &mut commands),
        TerminalMode::Prompt => close.closing = true,
    }
}

fn handle_terminal_keyboard(
    mut keyboard: MessageReader<KeyboardInput>,
    pause: Res<State<PauseStates>>,
    log: Res<NovaOsFlightLog>,
    objectives: Res<GameObjectives>,
    q_player: Query<
        (Entity, Option<&Name>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_sections: Query<
        (
            &ChildOf,
            Option<&Name>,
            Option<&Health>,
            Option<&SectionDamageClass>,
            Has<SectionInactiveMarker>,
            Has<HealthZeroMarker>,
            Has<HullSectionMarker>,
            Has<ControllerSectionMarker>,
            Has<ThrusterSectionMarker>,
            Has<TurretSectionMarker>,
            Has<TorpedoSectionMarker>,
            Option<&SectionAmmo>,
        ),
        With<SectionMarker>,
    >,
    mut terminal: ResMut<NovaOsTerminal>,
    mut close: ResMut<NovaOsCloseTransition>,
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    settings: Res<NovaOsMonitorSettings>,
    time: Res<Time<Real>>,
    mut last_key_click: Local<Option<f32>>,
    mut q_scrollback: Query<
        (&mut ScrollPosition, Option<&ComputedNode>),
        With<NovaOsTerminalScrollbackMarker>,
    >,
) {
    let nova_os_prompt_active =
        *pause.get() == PauseStates::NovaOs && terminal.active_mode() == TerminalMode::Prompt;
    // The `bank` is absent on rigs without the sound assets (headless), so each
    // branch guards on it and cues are a no-op there.
    let now = time.elapsed_secs();
    for event in keyboard.read() {
        if !nova_os_prompt_active {
            continue;
        }
        if event.state != ButtonState::Pressed {
            continue;
        }
        match &event.logical_key {
            Key::Enter => {
                let (ship_name, sections) = player_ship_snapshot(&q_player, &q_sections);
                let snapshot = terminal_snapshot_from_world(
                    &log,
                    &objectives,
                    ship_name.as_deref(),
                    &sections,
                    terminal.seen_events(),
                );
                let outcome = terminal.submit(&snapshot);
                if let Some(bank) = &bank {
                    // A bare Enter on an empty prompt stays silent (a deliberate
                    // refinement over the PoC, which thunks on every submit).
                    if outcome != TerminalSubmitOutcome::Empty {
                        // The enter "thunk" fires on every real submit; the
                        // outcome then layers ok/error/coil (the Story's cue set).
                        play_nova_os_cue(
                            &mut commands,
                            bank,
                            &settings,
                            UiSfx::NovaOsEnter,
                            NOVA_OS_ENTER_VOLUME,
                        );
                    }
                    let (cue, volume) = match outcome {
                        TerminalSubmitOutcome::Empty => (None, 0.0),
                        TerminalSubmitOutcome::Ran => (Some(UiSfx::NovaOsOk), NOVA_OS_OK_VOLUME),
                        TerminalSubmitOutcome::Errored => {
                            (Some(UiSfx::NovaOsError), NOVA_OS_ERROR_VOLUME)
                        }
                        TerminalSubmitOutcome::Launched => {
                            (Some(UiSfx::NovaOsCoil), NOVA_OS_COIL_VOLUME)
                        }
                    };
                    if let Some(cue) = cue {
                        play_nova_os_cue(&mut commands, bank, &settings, cue, volume);
                    }
                }
            }
            Key::Tab => {
                if terminal.complete() {
                    if let Some(bank) = &bank {
                        play_nova_os_cue(
                            &mut commands,
                            bank,
                            &settings,
                            UiSfx::NovaOsTick,
                            NOVA_OS_TICK_VOLUME,
                        );
                    }
                }
            }
            Key::Backspace => {
                terminal.backspace();
                if let Some(bank) = &bank {
                    play_nova_os_cue(
                        &mut commands,
                        bank,
                        &settings,
                        UiSfx::NovaOsBack,
                        NOVA_OS_BACK_VOLUME,
                    );
                }
            }
            Key::Delete => {
                terminal.delete();
                if let Some(bank) = &bank {
                    play_nova_os_cue(
                        &mut commands,
                        bank,
                        &settings,
                        UiSfx::NovaOsBack,
                        NOVA_OS_BACK_VOLUME,
                    );
                }
            }
            Key::ArrowLeft => terminal.move_cursor_left(),
            Key::ArrowRight => terminal.move_cursor_right(),
            Key::ArrowUp => terminal.history_previous(),
            Key::ArrowDown => terminal.history_next(),
            // Page the scrollback from the keyboard (PoC's PageUp/PageDown): a
            // cockpit player may never have a hand on the mouse. ~0.8 of a
            // viewport per press, clamped like `scroll_nova_os_panels`.
            key @ (Key::PageUp | Key::PageDown) => {
                if let Ok((mut scroll, computed_node)) = q_scrollback.single_mut() {
                    let page = computed_node.map(|node| node.size.y * 0.8).unwrap_or(0.0);
                    let delta = if matches!(key, Key::PageUp) {
                        -page
                    } else {
                        page
                    };
                    scroll.0.y =
                        (scroll.0.y + delta).clamp(0.0, max_nova_os_scroll_y(computed_node));
                }
            }
            Key::Character(_) | Key::Space => {
                if let Some(text) = &event.text {
                    terminal.insert_text(text);
                } else if matches!(event.logical_key, Key::Space) {
                    terminal.insert_text(" ");
                }
                // Typing click, throttled so OS key-repeat cannot machine-gun.
                // The first click always fires (last is `None`).
                if let Some(bank) = &bank {
                    let due = last_key_click
                        .map(|last| now - last >= NOVA_OS_KEY_MIN_INTERVAL)
                        .unwrap_or(true);
                    if due {
                        *last_key_click = Some(now);
                        play_nova_os_cue(
                            &mut commands,
                            bank,
                            &settings,
                            UiSfx::NovaOsKey,
                            NOVA_OS_KEY_VOLUME,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    // The `exit` command requests the same animated close as Esc/Start.
    if terminal.take_pending_close() {
        close.closing = true;
    }
}

/// Mirror the registered apps' launch words into the terminal so parsing,
/// completion and `help` treat them as commands. Reading `app_commands` through
/// the `ResMut` `Deref` does not mark the terminal changed, so once mirrored this
/// early-returns without thrashing `rebuild_terminal_ui`.
fn sync_nova_os_app_commands(
    registry: Res<NovaOsAppRegistry>,
    mut terminal: ResMut<NovaOsTerminal>,
) {
    // Compare through the immutable `Deref` so an up-to-date terminal is never
    // marked changed; only a real change takes the `&mut` path below.
    let up_to_date = terminal.app_commands().len() == registry.len()
        && terminal
            .app_commands()
            .iter()
            .map(|command| command.id)
            .eq(registry.ids());
    if up_to_date {
        return;
    }
    terminal.set_app_commands(registry.commands());
}

/// While an app owns the screen, keyboard input belongs to it: the terminal
/// prompt handler is already inert in app mode, and this feeds each key to the
/// app's own [`NovaOsAppRuntime::handle_key`]. Escape is skipped here because it
/// is the runtime's back gesture (handled once in [`close_nova_os_from_menu_keys`]
/// so it cannot both exit the app and close the NOVA OS on one press); the same is
/// true of the Ctrl+C / Ctrl+[ app-exit chord, so keys pressed while Control is
/// held are skipped here and owned solely by [`close_nova_os_from_menu_keys`].
///
/// An app only receives events on frames where it was ALREADY the live app last
/// frame (`last_app` tracks that). Any transition frame - the launch itself, an
/// app switch, or a Tab that reopens the computer onto a persisted app - drops the
/// event buffer, so the launching keystroke (e.g. the Enter that submitted `map`)
/// never bleeds into the app it just opened.
fn handle_nova_os_app_keyboard(
    mut keyboard: MessageReader<KeyboardInput>,
    pause: Res<State<PauseStates>>,
    keys: Res<ButtonInput<KeyCode>>,
    registry: Res<NovaOsAppRegistry>,
    mut terminal: ResMut<NovaOsTerminal>,
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    settings: Option<Res<NovaOsMonitorSettings>>,
    mut last_app: Local<Option<&'static str>>,
) {
    let in_nova_os = *pause.get() == PauseStates::NovaOs;
    // A held Control turns any key into the app-exit chord, owned by
    // `close_nova_os_from_menu_keys`; the app never sees those keys. This blocks
    // ALL Ctrl+<key> presses from reaching apps, not just Ctrl+C/[; a future app
    // wanting its own Ctrl shortcut must revisit this guard (and the owner) so the
    // exit chord and the shortcut do not both fire on one press.
    let ctrl_held = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let live = match terminal.active_mode() {
        TerminalMode::App { id } if in_nova_os => Some(id),
        _ => None,
    };
    // Only handle input when we were continuously in this same app; otherwise
    // (transition or not-in-an-app) drop the buffer and re-sync.
    let continuous = live.is_some() && live == *last_app;
    *last_app = live;
    if !continuous {
        keyboard.clear();
        return;
    }
    let Some(app) = live.and_then(|id| registry.get(id)) else {
        keyboard.clear();
        return;
    };
    let mut exit = false;
    for event in keyboard.read() {
        if event.state != ButtonState::Pressed
            || ctrl_held
            || matches!(event.logical_key, Key::Escape)
        {
            continue;
        }
        if app.handle_key(&event.logical_key) == NovaOsAppInputOutcome::Exit {
            exit = true;
            break;
        }
    }
    if exit && terminal.exit_app() {
        // Same degauss coil as the Escape / close-control exit routes.
        if let (Some(bank), Some(settings)) = (&bank, &settings) {
            play_nova_os_cue(
                &mut commands,
                bank,
                settings,
                UiSfx::NovaOsCoil,
                NOVA_OS_COIL_VOLUME,
            );
        }
    }
}

/// The app chrome's close control: clicking it returns to the terminal, the same
/// route as Escape, and plays the degauss coil on a real exit.
fn on_nova_os_app_close(
    _activate: On<Activate>,
    mut terminal: ResMut<NovaOsTerminal>,
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    settings: Option<Res<NovaOsMonitorSettings>>,
) {
    if terminal.exit_app() {
        if let (Some(bank), Some(settings)) = (&bank, &settings) {
            play_nova_os_cue(
                &mut commands,
                bank,
                settings,
                UiSfx::NovaOsCoil,
                NOVA_OS_COIL_VOLUME,
            );
        }
    }
}

/// BRIGHT knob click: advance the brightness detent (the dial pointer and the
/// CRT `brightness` uniform follow via [`sync_nova_os_monitor_controls`] /
/// [`animate_nova_os_crt`]).
fn on_nova_os_bright_knob(_activate: On<Activate>, mut settings: ResMut<NovaOsMonitorSettings>) {
    settings.cycle(NovaOsKnob::Bright);
}

/// SCAN knob click: advance the scanline detent.
fn on_nova_os_scan_knob(_activate: On<Activate>, mut settings: ResMut<NovaOsMonitorSettings>) {
    settings.cycle(NovaOsKnob::Scan);
}

/// SND button click: toggle the monitor speaker flag (default ON). The NOVA OS
/// sound task consumes the flag to mute/unmute the bed + cues; the on-screen
/// state reads off the bulb flipping (the label is now a fixed "SND" legend).
fn on_nova_os_sound_button(_activate: On<Activate>, mut settings: ResMut<NovaOsMonitorSettings>) {
    settings.sound_enabled = !settings.sound_enabled;
}

/// PWR button click: drive the existing animated close, the diegetic twin of the
/// `exit` command. Always powers the monitor off (from an app or the prompt).
fn on_nova_os_power_button(_activate: On<Activate>, mut close: ResMut<NovaOsCloseTransition>) {
    close.closing = true;
}

/// Reconcile the chin controls' look with [`NovaOsMonitorSettings`] after a knob
/// turn or SND toggle: rotate each dial pointer to its detent angle, and light /
/// dim the SND bulb. Spawn-time state is set directly in
/// [`spawn_nova_os_knob`]/[`spawn_nova_os_sound_button`]; this handles live
/// changes (gated on `resource_changed`, which also harmlessly re-applies the
/// current state on the init frame). The SND label no longer swaps text: the
/// bulb colour is the only moving part reporting the mute state.
fn sync_nova_os_monitor_controls(
    settings: Res<NovaOsMonitorSettings>,
    mut q_dials: Query<(&NovaOsKnob, &mut UiTransform), With<NovaOsKnobDialMarker>>,
    mut q_sound_indicator: Query<&mut BackgroundColor, With<NovaOsSoundIndicatorMarker>>,
) {
    for (knob, mut transform) in &mut q_dials {
        transform.rotation = Rot2::degrees(settings.dial_angle(*knob));
    }
    let bulb = nova_os_bulb_color(settings.sound_enabled);
    for mut color in &mut q_sound_indicator {
        color.0 = bulb;
    }
}

/// Flash the PWR LED orange while the monitor is powering down, green otherwise
/// (owner playtest: "turn orange and then close"). Runs every frame the NOVA OS
/// is active - the closing flag flips outside `NovaOsMonitorSettings`, so this
/// cannot ride `sync_nova_os_monitor_controls`' `resource_changed` gate.
fn drive_nova_os_power_led(
    close: Res<NovaOsCloseTransition>,
    mut q_led: Query<&mut BackgroundColor, With<NovaOsPowerLedMarker>>,
) {
    let color = if close.closing {
        NOVA_OS_ORANGE
    } else {
        NOVA_OS_PHOSPHOR
    };
    for mut background in &mut q_led {
        background.0 = color;
    }
}

/// Reconcile the on-screen app surface with [`NovaOsTerminal::active_mode`]:
/// launch spawns the app root (chrome + body) and hides the terminal content;
/// exit despawns the app root and reveals the terminal, whose scrollback was
/// never touched. Runs while the computer is open and diff-guards itself, so a
/// NOVA OS reopened onto a persisted app rebuilds the app and a plain reopen keeps
/// the terminal.
fn sync_nova_os_app_ui(
    mut commands: Commands,
    terminal: Res<NovaOsTerminal>,
    registry: Res<NovaOsAppRegistry>,
    asset_server: Option<Res<AssetServer>>,
    rtt: Option<Res<NovaOsRtt>>,
    mut degauss: ResMut<NovaOsDegauss>,
    q_screen: Query<Entity, With<NovaOsScreenMarker>>,
    q_app_root: Query<(Entity, &NovaOsAppRoot)>,
    mut q_content: Query<&mut Visibility, With<NovaOsTerminalContentMarker>>,
) {
    let desired = match terminal.active_mode() {
        TerminalMode::App { id } => Some(id),
        TerminalMode::Prompt => None,
    };
    let current = q_app_root
        .iter()
        .next()
        .map(|(entity, root)| (entity, root.id));
    if desired == current.map(|(_, id)| id) {
        return;
    }

    // A real launch/exit/switch got past the diff-guard: kick the degauss coil so
    // the CRT wobble+flash lands with the `NovaOsCoil` thump the input handlers
    // play on the same transitions (task 20260727-014148).
    degauss.pulse();

    if let Some((entity, _)) = current {
        commands.entity(entity).despawn();
    }
    for mut visibility in &mut q_content {
        *visibility = if desired.is_some() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    // Render-capable: the app surface joins the terminal in the offscreen content
    // root (so it renders through the CRT shader). Headless: onto the screen node.
    let target = match rtt.as_deref() {
        Some(rtt) => Some(rtt.content_root),
        None => q_screen.single().ok(),
    };
    let (Some(id), Some(target)) = (desired, target) else {
        return;
    };
    let Some(app) = registry.get(id) else {
        return;
    };
    let font = nova_os_font(asset_server.as_deref());
    commands.entity(target).with_children(|parent| {
        spawn_nova_os_app(parent, app, font);
    });
}

/// Spawn one app surface: a chrome bar (title + close control) over the app's own
/// body, filling the screen at content depth so the shared CRT overlay still sits
/// on top exactly as it does over the terminal.
fn spawn_nova_os_app(
    screen: &mut ChildSpawnerCommands,
    app: &dyn NovaOsAppRuntime,
    font: Handle<Font>,
) {
    screen
        .spawn((
            Name::new(format!("NovaOsApp:{}", app.id())),
            NovaOsAppRoot { id: app.id() },
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(NOVA_OS_SCREEN),
            ZIndex(NOVA_OS_CONTENT_Z),
        ))
        .with_children(|app_root| {
            app_root
                .spawn((
                    Node {
                        min_height: Val::Px(32.0),
                        padding: UiRect::bottom(Val::Px(10.0)),
                        border: UiRect::bottom(Val::Px(1.0)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(12.0),
                        ..default()
                    },
                    BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.36)),
                ))
                .with_children(|chrome| {
                    chrome.spawn((
                        Text::new(app.title().to_uppercase()),
                        nova_os_text_font(DRAWER_SECTION_TITLE_FONT_PX, font.clone()),
                        TextColor(NOVA_OS_PHOSPHOR),
                    ));
                    chrome.spawn((
                        NovaOsAppCloseMarker,
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(3.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor::all(NOVA_OS_AMBER.with_alpha(0.7)),
                        children![(
                            Text::new("[ ESC ] CLOSE"),
                            nova_os_text_font(11.0, font.clone()),
                            TextColor(NOVA_OS_AMBER),
                        )],
                        observe(on_nova_os_app_close),
                    ));
                });
            app_root
                .spawn((
                    Node {
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ZIndex(NOVA_OS_CONTENT_Z),
                ))
                .with_children(|body| {
                    app.spawn_body(body, font.clone());
                });
        });
}

fn player_ship_snapshot(
    q_player: &Query<
        (Entity, Option<&Name>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_sections: &Query<
        (
            &ChildOf,
            Option<&Name>,
            Option<&Health>,
            Option<&SectionDamageClass>,
            Has<SectionInactiveMarker>,
            Has<HealthZeroMarker>,
            Has<HullSectionMarker>,
            Has<ControllerSectionMarker>,
            Has<ThrusterSectionMarker>,
            Has<TurretSectionMarker>,
            Has<TorpedoSectionMarker>,
            Option<&SectionAmmo>,
        ),
        With<SectionMarker>,
    >,
) -> (Option<String>, Vec<ShipSectionStatus>) {
    let Ok((ship, ship_name)) = q_player.single() else {
        return (None, Vec::new());
    };
    let mut sections: Vec<ShipSectionStatus> = q_sections
        .iter()
        .filter(|(ChildOf(parent), ..)| *parent == ship)
        .filter_map(
            |(
                _,
                name,
                health,
                class,
                inactive,
                zero_health,
                hull,
                controller,
                thruster,
                turret,
                torpedo,
                ammo,
            )| {
                let kind =
                    section_kind_from_markers(class, hull, controller, thruster, turret, torpedo)?;
                Some(ShipSectionStatus {
                    name: name
                        .map(|name| name.as_str().to_string())
                        .unwrap_or_else(|| section_kind_label(kind).to_ascii_lowercase()),
                    kind,
                    health: health.cloned(),
                    inactive,
                    zero_health,
                    ammo: ammo.copied(),
                })
            },
        )
        .collect();
    sections.sort_by(|a, b| {
        section_kind_label(a.kind)
            .cmp(section_kind_label(b.kind))
            .then_with(|| a.name.cmp(&b.name))
    });
    (ship_name.map(|name| name.as_str().to_string()), sections)
}

fn section_kind_from_markers(
    class: Option<&SectionDamageClass>,
    hull: bool,
    controller: bool,
    thruster: bool,
    turret: bool,
    torpedo: bool,
) -> Option<SectionDamageClass> {
    class.copied().or_else(|| {
        if hull {
            Some(SectionDamageClass::Hull)
        } else if controller {
            Some(SectionDamageClass::Controller)
        } else if thruster {
            Some(SectionDamageClass::Thruster)
        } else if turret {
            Some(SectionDamageClass::Turret)
        } else if torpedo {
            Some(SectionDamageClass::Torpedo)
        } else {
            None
        }
    })
}

/// Run condition: the objectives list container was spawned this frame, so its
/// initial contents must be built from the current [`GameObjectives`] even
/// though the resource itself did not change.
fn nova_os_lists_just_spawned(
    q_objectives: Query<(), Added<NovaOsObjectivesListMarker>>,
    q_log: Query<(), Added<NovaOsFlightLogListMarker>>,
) -> bool {
    !q_objectives.is_empty() || !q_log.is_empty()
}

fn terminal_ui_just_spawned(
    q_prompt: Query<(), Added<NovaOsTerminalPromptMarker>>,
    q_scrollback: Query<(), Added<NovaOsTerminalScrollbackMarker>>,
) -> bool {
    !q_prompt.is_empty() || !q_scrollback.is_empty()
}

/// On the FIRST NOVA OS open of a session, kick off the staggered boot banner:
/// clear the scrollback and queue the welcome + unread-events rows for
/// [`drain_nova_os_boot`] to reveal one-by-one (PoC `printBanner`). Subsequent
/// opens keep the scrollback the player left behind.
fn begin_nova_os_boot(mut terminal: ResMut<NovaOsTerminal>, log: Res<NovaOsFlightLog>) {
    if terminal.is_booted() {
        return;
    }
    let unread = log.entries.len().saturating_sub(terminal.seen_events());
    let hook = nova_os_unread_hook(&log, terminal.seen_events());
    terminal.begin_boot(nova_os_boot_banner_rows(unread, hook));
}

/// When the computer closes, remember how many flight-log entries have been seen
/// so a later boot's unread-events count only covers what arrived afterward.
fn mark_nova_os_events_seen(mut terminal: ResMut<NovaOsTerminal>, log: Res<NovaOsFlightLog>) {
    terminal.set_seen_events(log.entries.len());
}

/// Reveal the queued boot-banner rows one-by-one on real time (PoC `printBanner`
/// ~130 ms cadence; virtual time is frozen while the computer is open). Runs only
/// while rows are pending, so once the banner finishes it stops touching the
/// terminal's change detection.
fn drain_nova_os_boot(
    time: Res<Time<Real>>,
    mut terminal: ResMut<NovaOsTerminal>,
    mut elapsed: Local<f32>,
) {
    // Read through the immutable `Deref` so an empty queue does not mark the
    // terminal changed (which would rebuild the UI every idle frame).
    if !terminal.has_pending_boot_rows() {
        *elapsed = 0.0;
        return;
    }
    *elapsed += time.delta_secs();
    while *elapsed >= NOVA_OS_BOOT_ROW_INTERVAL && terminal.reveal_next_boot_row() {
        *elapsed -= NOVA_OS_BOOT_ROW_INTERVAL;
    }
}

fn nova_os_footer_just_spawned(q_footer: Query<(), Added<NovaOsFooterHintsMarker>>) -> bool {
    !q_footer.is_empty()
}

/// Rebuild the footer hint row whenever the active surface changes, so the hints
/// swap per surface (terminal vs a running app). Keyed on `active_mode` via a
/// `Local`, so ordinary prompt edits (which change the terminal resource but not
/// the mode) do not thrash the footer.
fn rebuild_nova_os_footer_hints(
    terminal: Res<NovaOsTerminal>,
    registry: Res<NovaOsAppRegistry>,
    asset_server: Option<Res<AssetServer>>,
    mut commands: Commands,
    q_footer: Query<(Entity, Option<&Children>), With<NovaOsFooterHintsMarker>>,
    mut last_mode: Local<Option<TerminalMode>>,
) {
    if *last_mode == Some(terminal.active_mode()) {
        return;
    }
    *last_mode = Some(terminal.active_mode());
    let hints = nova_os_footer_hints(terminal.active_mode(), &registry);
    let font = nova_os_font(asset_server.as_deref());
    for (footer, children) in &q_footer {
        if let Some(children) = children {
            for &child in children {
                commands.entity(child).despawn();
            }
        }
        commands.entity(footer).with_children(|footer| {
            for &hint in hints {
                footer.spawn((
                    Text::new(hint),
                    nova_os_text_font(11.0, font.clone()),
                    TextColor(NOVA_OS_PHOSPHOR_MUTED),
                ));
            }
        });
    }
}

fn rebuild_terminal_ui(
    mut commands: Commands,
    terminal: Res<NovaOsTerminal>,
    asset_server: Option<Res<AssetServer>>,
    mut q_scrollback: Query<
        (Entity, Option<&Children>, &mut ScrollPosition),
        With<NovaOsTerminalScrollbackMarker>,
    >,
    mut text_targets: ParamSet<(
        Query<(&mut Text, &mut TextColor), With<NovaOsTerminalPromptMarker>>,
        Query<(&mut Text, &mut TextColor), With<NovaOsTerminalPromptAfterMarker>>,
        Query<(&mut Text, &mut TextColor), With<NovaOsTerminalHintMarker>>,
        Query<(&mut Text, &mut TextColor), With<NovaOsTerminalGhostMarker>>,
    )>,
) {
    let font = nova_os_font(asset_server.as_deref());
    if let Ok((list, children, mut scroll)) = q_scrollback.single_mut() {
        if let Some(children) = children {
            for &child in children {
                commands.entity(child).despawn();
            }
        }
        commands.entity(list).with_children(|parent| {
            for row in terminal.scrollback() {
                spawn_terminal_row(parent, row, font.clone());
            }
        });
        scroll.0.y = f32::MAX;
    }

    let prompt_color = prompt_color(&terminal);
    for (mut text, mut color) in &mut text_targets.p0() {
        text.0 = prompt_before_cursor(&terminal);
        color.0 = prompt_color;
    }
    for (mut text, mut color) in &mut text_targets.p1() {
        text.0 = prompt_after_cursor(&terminal);
        color.0 = prompt_color;
    }
    for (mut text, mut color) in &mut text_targets.p2() {
        text.0 = prompt_hint_display(&terminal);
        let hint_color = match terminal.parse_status() {
            TerminalParseStatus::Invalid => theme::semantic::THREAT,
            TerminalParseStatus::ValidPrefix => NOVA_OS_PHOSPHOR_MUTED,
            TerminalParseStatus::Empty | TerminalParseStatus::Valid => NOVA_OS_PHOSPHOR_DIM,
        };
        color.0 = hint_color;
    }
    for (mut text, mut color) in &mut text_targets.p3() {
        text.0 = prompt_completion_ghost(&terminal);
        color.0 = NOVA_OS_TEXT.with_alpha(0.34);
    }
}

/// Blink the terminal caret with a steady on/off cadence, driven by real time
/// so it keeps blinking while the sim is frozen. The caret is a small amber
/// block node, so the blink just toggles its background alpha.
fn blink_nova_os_caret(
    time: Res<Time<Real>>,
    mut q_caret: Query<&mut BackgroundColor, With<NovaOsTerminalCaretMarker>>,
) {
    // On-phase alpha 0.85 (not 1.0): the caret now sits OVER the character at the
    // cursor (the first completion letter), so a translucent block lets that
    // letter read through instead of masking it (PoC `.caret` opacity 0.85).
    let on = (time.elapsed_secs() * NOVA_OS_CARET_BLINK_HZ).fract() < 0.5;
    let color = NOVA_OS_AMBER.with_alpha(if on { 0.85 } else { 0.0 });
    for mut background in &mut q_caret {
        background.0 = color;
    }
}

/// Position the absolute block caret at the MEASURED rendered width of the
/// typed-before text, so it lands exactly on the cursor's character - the first
/// after-cursor / completion-ghost glyph - regardless of the font's real glyph
/// advance. This mirrors the web PoC, which sets `caret.left = measure.offsetWidth`
/// rather than assuming a cell size; a hardcoded `chars * 0.6em` step would drift
/// cumulatively because 0.6em is the caret BLOCK width, not the glyph advance.
/// `ComputedNode::size` is physical px, so scale it back to the logical px that
/// `Node::left` expects.
fn position_nova_os_block_caret(
    q_before: Query<&ComputedNode, With<NovaOsTerminalPromptMarker>>,
    mut q_caret: Query<&mut Node, With<NovaOsTerminalCaretMarker>>,
) {
    let Ok(before) = q_before.single() else {
        return;
    };
    let width = before.size().x * before.inverse_scale_factor();
    for mut node in &mut q_caret {
        node.left = Val::Px(width);
    }
}

/// The separator that fronts the FPS segment in the topbar status line. The drive
/// system rewrites everything from this marker on, leaving the `SHIP:`/`LINK:`
/// head (which never changes after spawn) untouched.
const NOVA_OS_TOPBAR_FPS_MARKER: &str = "     FPS: ";

/// The smoothed frame rate rounded to a whole number, or `None` before the
/// diagnostic has a reading. Reuses Bevy's `FrameTimeDiagnosticsPlugin::FPS`
/// smoothed value - the exact source the flight status bar's FPS item read
/// (bcs `status_fps_value_fn`) - so the number on the topbar matches the one the
/// hidden status bar would show.
fn nova_os_diagnostic_fps(diagnostics: &bevy::diagnostic::DiagnosticsStore) -> Option<u32> {
    diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .map(|fps| fps.round() as u32)
}

/// Rewrite only the `FPS: <n>` tail of a topbar status line, preserving the
/// `SHIP:`/`LINK:` head. Falls back to appending the segment if a line somehow
/// lacks it (e.g. an older spawn), so the FPS never silently goes missing.
fn topbar_line_with_fps(current: &str, fps: Option<u32>) -> String {
    let head = current
        .split_once(NOVA_OS_TOPBAR_FPS_MARKER)
        .map(|(head, _)| head)
        .unwrap_or(current);
    let fps = nova_os_fps_segment(fps);
    format!("{head}{NOVA_OS_TOPBAR_FPS_MARKER}{fps}")
}

/// Refresh the live `FPS: <n>` segment on the NOVA OS topbar each frame while the
/// computer is open. The flight status bar (which normally carries the FPS item)
/// is hidden in `PauseStates::NovaOs`, so this is the only FPS readout on screen
/// then. Runs on the real-time NOVA OS group beside the caret blink because the
/// virtual clock is frozen while the NOVA OS is open.
fn drive_nova_os_topbar_fps(
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut q_status: Query<&mut Text, With<NovaOsStatusMarker>>,
) {
    let fps = nova_os_diagnostic_fps(&diagnostics);
    for mut text in &mut q_status {
        let next = topbar_line_with_fps(&text.0, fps);
        if text.0 != next {
            text.0 = next;
        }
    }
}

/// Feed real-time seconds and the panel pixel size into the CRT material each
/// frame: `time` drives the grain shimmer and `resolution` (from the overlay
/// node's [`ComputedNode`]) makes the scanlines/slot-mask resolution-aware. Real
/// time because the sim clock is frozen while the computer is open.
fn animate_nova_os_crt(
    time: Res<Time<Real>>,
    settings: Res<NovaOsMonitorSettings>,
    mut degauss: ResMut<NovaOsDegauss>,
    mut materials: ResMut<Assets<NovaOsCrtMaterial>>,
    q_openness: Query<&NovaOsOpenness, With<NovaOsRootMarker>>,
    q_surface: Query<
        (&MaterialNode<NovaOsCrtMaterial>, &ComputedNode),
        With<NovaOsSamplingSurfaceMarker>,
    >,
) {
    let seconds = time.elapsed_secs();
    // Bleed the degauss pulse down by real time (the sim clock is frozen while the
    // computer is open) and feed its 0..1 envelope to the shader.
    if degauss.remaining > 0.0 {
        degauss.remaining = (degauss.remaining - time.delta_secs()).max(0.0);
    }
    let degauss_env = degauss.envelope();
    // Feed the eased openness in as the CRT power level: the shader blooms the
    // raster on from a line and collapses it to a dying dot on close.
    let power = q_openness.iter().next().map(|o| o.0).unwrap_or(1.0);
    // The BRIGHT/SCAN chin knobs drive the brightness multiply and scanline
    // depth uniforms (task 20260726-214617).
    let brightness = settings.brightness();
    let scanline_strength = settings.scanline_strength();
    for (node, computed) in &q_surface {
        if let Some(mut material) = materials.get_mut(&node.0) {
            material.data.time = seconds;
            material.data.resolution = computed.size;
            material.data.power = power;
            material.data.brightness = brightness;
            material.data.scanline_strength = scanline_strength;
            material.data.degauss = degauss_env;
        }
    }
}

fn spawn_terminal_row(parent: &mut ChildSpawnerCommands, row: &TerminalRow, font: Handle<Font>) {
    let color = match row.kind {
        TerminalRowKind::Input => NOVA_OS_AMBER,
        TerminalRowKind::Output => NOVA_OS_TEXT,
        TerminalRowKind::Dim => NOVA_OS_PHOSPHOR_DIM,
        TerminalRowKind::Info => NOVA_OS_INFO,
        TerminalRowKind::Warn => NOVA_OS_AMBER,
        TerminalRowKind::Error => theme::semantic::THREAT,
    };
    parent.spawn((
        Text::new(row.text.clone()),
        nova_os_text_font(DRAWER_LINE_FONT_PX, font),
        TextColor(color),
        TextLayout {
            justify: Justify::Left,
            linebreak: LineBreak::WordBoundary,
        },
    ));
}

fn prompt_color(terminal: &NovaOsTerminal) -> Color {
    match terminal.parse_status() {
        TerminalParseStatus::Invalid => theme::semantic::THREAT,
        TerminalParseStatus::Empty
        | TerminalParseStatus::Valid
        | TerminalParseStatus::ValidPrefix => NOVA_OS_PHOSPHOR,
    }
}

fn nova_os_font(asset_server: Option<&AssetServer>) -> Handle<Font> {
    asset_server
        .map(|server| server.load(NOVA_OS_FONT_PATH))
        .unwrap_or_default()
}

fn nova_os_text_font(font_size: f32, font: Handle<Font>) -> TextFont {
    TextFont {
        font: FontSource::Handle(font),
        font_size: FontSize::Px(font_size),
        ..default()
    }
}

/// The prompt/ghost pieces must never wrap: a wrapped ghost is exactly the
/// "completion appears below the line" bug. `NoWrap` keeps every piece on the
/// single input line and lets the wrap node clip horizontally instead.
fn nova_os_prompt_text_layout() -> TextLayout {
    TextLayout {
        justify: Justify::Left,
        linebreak: LineBreak::NoWrap,
    }
}

// The terminal text is drawn crisp, with no per-glyph shadow: Bevy's `TextShadow`
// has no blur, so a faked glow reads as a doubled/offset shadow rather than the
// HTML's soft `text-shadow: 0 0 7px` halo. The screen's phosphor feel comes from
// the CRT overlay (centre glow + grain) instead. A true blurred glow would need a
// render-to-texture bloom pass over the terminal content, out of scope here.

fn scroll_nova_os_panels(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut q_panels: Query<
        (&mut ScrollPosition, Option<&Hovered>, Option<&ComputedNode>),
        With<NovaOsScrollViewportMarker>,
    >,
) {
    use bevy::input::mouse::MouseScrollUnit;

    let dy: f32 = wheel
        .read()
        .map(|ev| match ev.unit {
            MouseScrollUnit::Line => ev.y * DRAWER_SCROLL_LINE_HEIGHT_PX,
            MouseScrollUnit::Pixel => ev.y,
        })
        .sum();
    if dy == 0.0 {
        return;
    }

    let any_hovered = q_panels
        .iter()
        .any(|(_, hovered, _)| hovered.is_some_and(Hovered::get));

    for (mut scroll, hovered, computed_node) in &mut q_panels {
        if any_hovered && !hovered.is_some_and(Hovered::get) {
            continue;
        }
        scroll.0.y = (scroll.0.y - dy).clamp(0.0, max_nova_os_scroll_y(computed_node));
    }
}

fn max_nova_os_scroll_y(computed_node: Option<&ComputedNode>) -> f32 {
    computed_node
        .map(|node| (node.content_size.y - node.size.y + node.scrollbar_size.y).max(0.0))
        .unwrap_or(f32::MAX)
}

/// Ease [`NovaOsOpenness`] toward the state-driven target (1 open, 0 closed)
/// with REAL time, and map it onto the panel offset, the backdrop alpha and
/// both nodes' visibility. Real time because virtual time is paused while the
/// NOVA OS is open (see the module docs).
fn drive_nova_os_slide(
    time: Res<Time<Real>>,
    pause: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    mut close: ResMut<NovaOsCloseTransition>,
    mut q_panel: Query<
        (&mut NovaOsOpenness, &mut Visibility),
        (With<NovaOsRootMarker>, Without<NovaOsBackdropMarker>),
    >,
    mut q_backdrop: Query<
        (&mut BackgroundColor, &mut Visibility),
        (With<NovaOsBackdropMarker>, Without<NovaOsRootMarker>),
    >,
) {
    let nova_os_active = *pause.get() == PauseStates::NovaOs;
    if !nova_os_active {
        close.closing = false;
    }
    let target = if nova_os_active && !close.closing {
        1.0
    } else {
        0.0
    };
    let step = time.delta_secs() / DRAWER_SLIDE_SECS.max(f32::EPSILON);

    // The backdrop tracks the panels' openness; default to the target when no
    // panel exists (headless rigs) so the two stay consistent. Both panels
    // share the same eased openness, so either one is a faithful source.
    let mut openness = target;
    for (mut panel_openness, mut visibility) in &mut q_panel {
        panel_openness.0 = approach(panel_openness.0, target, step);
        openness = panel_openness.0;
        *visibility = visibility_for(panel_openness.0);
    }

    for (mut background, mut visibility) in &mut q_backdrop {
        background.0 = NOVA_OS_BACKDROP.with_alpha(DRAWER_BACKDROP_ALPHA * openness);
        *visibility = visibility_for(openness);
    }

    if nova_os_active && close.closing && openness <= f32::EPSILON {
        close.closing = false;
        next.set(PauseStates::Unpaused);
    }
}

/// Hidden once fully closed (so a closed NOVA OS never eats a raycast), visible
/// otherwise.
fn visibility_for(openness: f32) -> Visibility {
    if openness <= f32::EPSILON {
        Visibility::Hidden
    } else {
        Visibility::Visible
    }
}

/// Move `current` toward `target` by at most `step` (a linear approach; the
/// step is a fraction of the full travel per frame).
fn approach(current: f32, target: f32, step: f32) -> f32 {
    if current < target {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    }
}

/// Update the NOVA OS's combined left-panel flight log from the story feed and
/// active objective list.
fn sync_nova_os_logs(
    story: Res<StoryFeed>,
    objectives: Res<GameObjectives>,
    mut log: ResMut<NovaOsFlightLog>,
) {
    if story.0.len() < log.seen_story {
        log.clear();
    }

    for line in story.0.iter().skip(log.seen_story) {
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::Comms,
            objective_id: None,
            speaker: Some(line.speaker.clone()),
            message: line.text.clone(),
            icon: line.icon.clone(),
        });
    }
    log.seen_story = story.0.len();

    let completed: Vec<Objective> = log
        .previous_active
        .iter()
        .filter(|old| {
            !objectives
                .objectives
                .iter()
                .any(|current| current.id == old.id)
        })
        .cloned()
        .collect();
    for objective in completed {
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::ObjectiveCompleted,
            objective_id: Some(objective.id.clone()),
            speaker: None,
            message: objective.message.clone(),
            icon: None,
        });
        log.active_objective_entries
            .retain(|entry| entry.id != objective.id);
    }

    for objective in &objectives.objectives {
        if let Some(active) = log
            .active_objective_entries
            .iter()
            .find(|entry| entry.id == objective.id)
            .cloned()
        {
            if let Some(entry) = log.entries.get_mut(active.entry_index) {
                entry.message = objective.message.clone();
            }
            continue;
        }

        let entry_index = log.entries.len();
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::ObjectivePosted,
            objective_id: Some(objective.id.clone()),
            speaker: None,
            message: objective.message.clone(),
            icon: None,
        });
        log.active_objective_entries
            .push(NovaOsFlightLogActiveObjective {
                id: objective.id.clone(),
                entry_index,
            });
    }

    log.previous_active = objectives.objectives.clone();
}

/// Announce objective flips into the LIVE terminal scrollback while the computer
/// is open at the prompt (PoC `checkObjectives` pushes an `OBJ x ...` line the
/// moment an objective completes, so the player sees it without typing `log`).
/// Only completions that happen while open are announced; ones that flipped while
/// the computer was closed stay in the flight log (counted by the boot banner's
/// unread-events line instead of dumping on open).
fn announce_objectives_in_terminal(
    log: Res<NovaOsFlightLog>,
    pause: Res<State<PauseStates>>,
    mut terminal: ResMut<NovaOsTerminal>,
    mut announced: Local<Option<usize>>,
) {
    let total = log.entries.len();
    // `None` on the first run (and `min` if the log was cleared) means we start
    // from "everything already seen" - nothing is announced retroactively.
    let from = announced.unwrap_or(total).min(total);
    let open =
        *pause.get() == PauseStates::NovaOs && terminal.active_mode() == TerminalMode::Prompt;
    if open {
        let fresh: Vec<TerminalRow> = log.entries[from..]
            .iter()
            .filter(|entry| entry.kind == NovaOsFlightLogEntryKind::ObjectiveCompleted)
            .map(|entry| TerminalRow {
                kind: TerminalRowKind::Info,
                text: nova_os_flight_log_text(entry),
            })
            .collect();
        // Only touch the scrollback (and so mark the terminal changed, forcing a
        // rebuild that snaps the view to the bottom) when there is actually
        // something to announce - most objective-change frames have no completion.
        if !fresh.is_empty() {
            terminal.extend_scrollback(fresh);
        }
    }
    *announced = Some(total);
}

/// Rebuild the right objectives-section rows from the active objectives list.
fn rebuild_nova_os_objectives(
    mut commands: Commands,
    objectives: Res<GameObjectives>,
    q_list: Query<(Entity, Option<&Children>), With<NovaOsObjectivesListMarker>>,
) {
    let Ok((list, children)) = q_list.single() else {
        return;
    };
    if let Some(children) = children {
        for &child in children {
            commands.entity(child).despawn();
        }
    }
    commands.entity(list).with_children(|parent| {
        if objectives.objectives.is_empty() {
            spawn_nova_os_empty_objective_row(parent);
            return;
        }
        for objective in &objectives.objectives {
            spawn_nova_os_objective_row(parent, objective);
        }
    });
}

fn spawn_nova_os_empty_objective_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Name::new("NovaOsObjectiveEmpty"),
            NovaOsObjectiveEmptyMarker,
            Node {
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                ..default()
            },
            BorderColor::all(theme::BORDER),
            BackgroundColor(theme::PANEL_RAISED.with_alpha(0.45)),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new("No active objectives."),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::TEXT_MUTED),
            ));
        });
}

fn spawn_nova_os_objective_row(parent: &mut ChildSpawnerCommands, objective: &Objective) {
    parent
        .spawn((
            Name::new(format!("NovaOsObjective {}", objective.id)),
            NovaOsObjectiveRowMarker,
            NovaOsObjectiveId(objective.id.clone()),
            NovaOsObjectiveRowStatus::Active,
            Node {
                min_height: Val::Px(34.0),
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(DRAWER_ROW_GAP_PX),
                ..default()
            },
            BorderColor::all(theme::BORDER_BRIGHT),
            BackgroundColor(theme::PANEL_RAISED),
        ))
        .with_children(|row| {
            row.spawn((
                NovaOsObjectiveGlyphMarker,
                Text::new(">"),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::semantic::OBJECTIVE),
                Node {
                    width: Val::Px(DRAWER_OBJECTIVE_GLYPH_WIDTH_PX),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            row.spawn(Node {
                position_type: PositionType::Relative,
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|text_wrap| {
                text_wrap.spawn((
                    NovaOsObjectiveTextMarker,
                    Text::new(objective.message.clone()),
                    TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                    TextLayout {
                        justify: Justify::Left,
                        linebreak: LineBreak::WordBoundary,
                    },
                    TextColor(theme::TEXT),
                ));
            });
        });
}

/// Rebuild the left combined flight-log stream.
fn rebuild_nova_os_flight_log(
    mut commands: Commands,
    log: Res<NovaOsFlightLog>,
    asset_server: Option<Res<AssetServer>>,
    q_list: Query<(Entity, Option<&Children>), With<NovaOsFlightLogListMarker>>,
) {
    let Ok((list, children)) = q_list.single() else {
        return;
    };
    if let Some(children) = children {
        for &child in children {
            commands.entity(child).despawn();
        }
    }
    commands.entity(list).with_children(|parent| {
        if log.entries.is_empty() {
            spawn_nova_os_empty_flight_log_row(parent);
            return;
        }
        for entry in &log.entries {
            spawn_nova_os_flight_log_row(parent, entry, asset_server.as_deref());
        }
    });
}

fn spawn_nova_os_empty_flight_log_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Name::new("NovaOsFlightLogEmpty"),
            NovaOsFlightLogEmptyMarker,
            Node {
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                ..default()
            },
            BorderColor::all(theme::BORDER),
            BackgroundColor(theme::PANEL_RAISED.with_alpha(0.45)),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new("No log entries."),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::TEXT_MUTED),
            ));
        });
}

fn spawn_nova_os_flight_log_row(
    parent: &mut ChildSpawnerCommands,
    entry: &NovaOsFlightLogEntry,
    asset_server: Option<&AssetServer>,
) {
    let icon_kind = match entry.kind {
        NovaOsFlightLogEntryKind::Comms if entry.icon.is_some() => {
            NovaOsFlightLogIconKind::CommsAuthored
        }
        NovaOsFlightLogEntryKind::Comms => NovaOsFlightLogIconKind::Fallback,
        NovaOsFlightLogEntryKind::ObjectivePosted
        | NovaOsFlightLogEntryKind::ObjectiveCompleted => NovaOsFlightLogIconKind::Objective,
    };
    let accent = match entry.kind {
        NovaOsFlightLogEntryKind::Comms => theme::CYAN,
        NovaOsFlightLogEntryKind::ObjectivePosted => theme::semantic::OBJECTIVE,
        NovaOsFlightLogEntryKind::ObjectiveCompleted => theme::semantic::ALLY,
    };

    parent
        .spawn((
            Name::new("NovaOsFlightLogRow"),
            NovaOsFlightLogRowMarker,
            NovaOsFlightLogIconMarker { kind: icon_kind },
            Node {
                min_height: Val::Px(30.0),
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(DRAWER_ROW_GAP_PX),
                ..default()
            },
            BorderColor::all(theme::BORDER),
            BackgroundColor(theme::PANEL_RAISED.with_alpha(0.58)),
        ))
        .with_children(|row| {
            spawn_nova_os_flight_log_icon(row, entry, icon_kind, accent, asset_server);
            row.spawn((
                NovaOsFlightLogTextMarker,
                Text::new(nova_os_flight_log_text(entry)),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::TEXT),
                TextLayout {
                    justify: Justify::Left,
                    linebreak: LineBreak::WordBoundary,
                },
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ));
        });
}

fn spawn_nova_os_flight_log_icon(
    row: &mut ChildSpawnerCommands,
    entry: &NovaOsFlightLogEntry,
    icon_kind: NovaOsFlightLogIconKind,
    accent: Color,
    asset_server: Option<&AssetServer>,
) {
    let node = Node {
        width: Val::Px(DRAWER_LOG_ICON_SIZE_PX),
        height: Val::Px(DRAWER_LOG_ICON_SIZE_PX),
        min_width: Val::Px(DRAWER_LOG_ICON_SIZE_PX),
        border: UiRect::all(Val::Px(theme::BORDER_W)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        flex_shrink: 0.0,
        ..default()
    };
    match (&entry.icon, icon_kind) {
        (Some(icon), NovaOsFlightLogIconKind::CommsAuthored) => {
            row.spawn((
                node,
                ImageNode::new(
                    asset_server
                        .map(|server| icon.resolve(server))
                        .unwrap_or_default(),
                ),
                BorderColor::all(accent),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ));
        }
        _ => {
            row.spawn((
                node,
                BorderColor::all(accent),
                BackgroundColor(accent.with_alpha(0.16)),
            ))
            .with_children(|icon| {
                icon.spawn((
                    Text::new(match icon_kind {
                        NovaOsFlightLogIconKind::Objective => ">",
                        NovaOsFlightLogIconKind::CommsAuthored
                        | NovaOsFlightLogIconKind::Fallback => "#",
                    }),
                    TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                    TextColor(accent),
                ));
            });
        }
    }
}

fn nova_os_flight_log_text(entry: &NovaOsFlightLogEntry) -> String {
    match entry.kind {
        NovaOsFlightLogEntryKind::Comms => format!(
            "COMMS {} > {}",
            entry.speaker.as_deref().unwrap_or("UNKNOWN").to_uppercase(),
            entry.message
        ),
        NovaOsFlightLogEntryKind::ObjectivePosted => format!("OBJ + {}", entry.message),
        NovaOsFlightLogEntryKind::ObjectiveCompleted => format!("OBJ x {}", entry.message),
    }
}

/// Spawn the NOVA OS shell (backdrop plus inset NOVA OS monitor) when the player
/// ship appears - mirrors the other HUD widgets.
fn setup_nova_os(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    mut crt_materials: Option<ResMut<Assets<NovaOsCrtMaterial>>>,
    mut images: Option<ResMut<Assets<Image>>>,
    asset_server: Option<Res<AssetServer>>,
    settings: Option<Res<NovaOsMonitorSettings>>,
    q_spaceship: Query<
        (Entity, Option<&Name>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let Ok((_, ship_name)) = q_spaceship.get(add.entity) else {
        return;
    };
    // The plugin always inits the resource; tolerate its absence so bare-app
    // rigs that only exercise other parts of the shell still spawn.
    let settings = settings.map(|s| *s).unwrap_or_default();
    let font = nova_os_font(asset_server.as_deref());
    let ship_name = nova_os_ship_name(ship_name);

    // Render-to-texture pipeline: on render-capable builds route the terminal
    // content to an offscreen image via a dedicated UI camera, so the screen node
    // can sample it through the CRT shader (bloom + curvature). Headless rigs
    // (no image/material assets) fall back to the terminal directly on the screen.
    let rtt = match (crt_materials.as_deref_mut(), images.as_deref_mut()) {
        (Some(_), Some(images)) => {
            let image = images.add(nova_os_new_target_image(UVec2::new(2, 2)));
            let camera = commands
                .spawn((
                    Name::new("NovaOsImageCamera"),
                    NovaOsImageCameraMarker,
                    Camera2d,
                    Camera {
                        order: NOVA_OS_RTT_CAMERA_ORDER,
                        clear_color: ClearColorConfig::Custom(NOVA_OS_SCREEN),
                        is_active: false,
                        ..default()
                    },
                    RenderTarget::Image(ImageRenderTarget {
                        handle: image.clone(),
                        scale_factor: 1.0,
                    }),
                    // Draw ONLY the terminal UI, never stray world 2D sprites.
                    RenderLayers::layer(NOVA_OS_RTT_LAYER),
                ))
                .id();
            let content_root = commands
                .spawn((
                    Name::new("NovaOsImageContentRoot"),
                    NovaOsImageContentRootMarker,
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        left: Val::Px(0.0),
                        width: Val::Px(2.0),
                        height: Val::Px(2.0),
                        padding: UiRect::all(Val::Px(NOVA_OS_SCREEN_PAD_PX)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(NOVA_OS_SCREEN),
                    UiTargetCamera(camera),
                    RenderLayers::layer(NOVA_OS_RTT_LAYER),
                    Visibility::Hidden,
                ))
                .id();
            let pointer = commands
                .spawn((
                    Name::new("NovaOsForwardedPointer"),
                    NovaOsForwardedPointerMarker,
                    nova_os_pointer_id(),
                    PointerLocation::new(Location {
                        target: nova_os_image_target(&image),
                        position: Vec2::splat(-1000.0),
                    }),
                ))
                .id();
            commands.insert_resource(NovaOsRtt {
                image: image.clone(),
                camera,
                content_root,
                pointer,
            });
            Some((content_root, image))
        }
        _ => {
            commands.remove_resource::<NovaOsRtt>();
            None
        }
    };

    // Dim backdrop behind the panel (hidden until the NOVA OS opens). NO
    // `HudTier`: the NOVA OS is a modal overlay on its own axis, so the
    // grave/tilde HUD-visibility cycle must not touch it - `apply_hud_visibility`
    // force-hides a non-shown Chrome tier every frame (even self-driven ones),
    // which would blank the NOVA OS if the player opened it with the HUD
    // minimized. The panel's visibility is driven entirely by `drive_nova_os_slide`.
    commands.spawn((
        Name::new("NovaOsBackdrop"),
        NovaOsBackdropMarker,
        GlobalZIndex(DRAWER_BACKDROP_Z),
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(NOVA_OS_BACKDROP.with_alpha(0.0)),
    ));

    // (The old flight-view tab handle was removed in task 20260724-134312; the
    // top-right objective hint is the NOVA OS affordance + the reveal's tuck
    // anchor now.)

    // One inset physical monitor. It is hidden until opened by the same
    // real-time openness driver the old NOVA OS panels used.
    commands
        .spawn((
            Name::new("NovaOsMonitor"),
            NovaOsRootMarker,
            NovaOsMonitorMarker,
            NovaOsOpenness(0.0),
            GlobalZIndex(DRAWER_PANEL_Z),
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(NOVA_OS_MONITOR_INSET_Y_PX),
                bottom: Val::Px(NOVA_OS_MONITOR_INSET_Y_PX),
                left: Val::Px(NOVA_OS_MONITOR_INSET_X_PX),
                right: Val::Px(NOVA_OS_MONITOR_INSET_X_PX),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                // Injection-moulded shell: larger top radius, tighter bottom.
                border_radius: BorderRadius {
                    top_left: Val::Px(NOVA_OS_CASE_RADIUS_TOP_PX),
                    top_right: Val::Px(NOVA_OS_CASE_RADIUS_TOP_PX),
                    bottom_left: Val::Px(NOVA_OS_CASE_RADIUS_BOTTOM_PX),
                    bottom_right: Val::Px(NOVA_OS_CASE_RADIUS_BOTTOM_PX),
                },
                ..default()
            },
            BorderColor::all(NOVA_OS_CASE_EDGE),
            // Base fill under the gradient (headless/no-gradient fallback).
            BackgroundColor(NOVA_OS_CASE),
            // Injection-moulded shell: a 168deg body gradient (lit top -> deep
            // undercut) plus a 1px top highlight catching the moulding lip.
            nova_os_case_gradient(),
        ))
        .with_children(|monitor| {
            spawn_nova_os_moulding_seam(monitor);
            spawn_nova_os_casing_screws(monitor);
            spawn_nova_os_casing_vents(monitor);
            monitor
                .spawn((
                    Name::new("NovaOsBezel"),
                    NovaOsBezelMarker,
                    Node {
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        padding: UiRect::all(Val::Px(NOVA_OS_BEZEL_PAD_PX)),
                        border: UiRect::all(Val::Px(1.0)),
                        flex_direction: FlexDirection::Column,
                        border_radius: BorderRadius::all(Val::Px(NOVA_OS_BEZEL_RADIUS_PX)),
                        ..default()
                    },
                    // Recessed bezel lip: dark inner-top shadow, light lower edge.
                    BorderColor {
                        top: Color::srgba(0.0, 0.0, 0.0, 0.6),
                        bottom: Color::srgba(1.0, 1.0, 1.0, 0.06),
                        left: NOVA_OS_CASE_EDGE.with_alpha(0.5),
                        right: NOVA_OS_CASE_EDGE.with_alpha(0.5),
                    },
                    BackgroundColor(NOVA_OS_CASE_RAISED),
                    nova_os_bezel_gradient(),
                ))
                .with_children(|bezel| {
                    bezel
                        .spawn((
                            Name::new("NovaOsScreen"),
                            NovaOsScreenMarker,
                            Node {
                                position_type: PositionType::Relative,
                                flex_grow: 1.0,
                                min_height: Val::Px(0.0),
                                padding: UiRect::all(Val::Px(NOVA_OS_SCREEN_PAD_PX)),
                                border: UiRect::all(Val::Px(1.0)),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(12.0),
                                overflow: Overflow::clip(),
                                border_radius: BorderRadius::all(Val::Px(NOVA_OS_SCREEN_RADIUS_PX)),
                                ..default()
                            },
                            // A dark recess line, not a bright straight phosphor
                            // frame: the glass sits recessed in the bezel and the
                            // crisp glowing edge now comes from the shader's
                            // barrel-bowed rim (see DECISION.md / feedback item 2).
                            BorderColor::all(NOVA_OS_CASE_EDGE.with_alpha(0.85)),
                            BackgroundColor(NOVA_OS_SCREEN),
                        ))
                        .with_children(|screen| {
                            match (&rtt, crt_materials.as_deref_mut()) {
                                (Some((_, image)), Some(crt_materials)) => {
                                    // Screen surface = the offscreen image sampled
                                    // through the CRT shader. Terminal content is
                                    // populated into the content root below.
                                    let mut material = NovaOsCrtMaterial::default();
                                    material.source = image.clone();
                                    let handle = crt_materials.add(material);
                                    screen.spawn((
                                        Name::new("NovaOsCrtSurface"),
                                        NovaOsSamplingSurfaceMarker,
                                        Node {
                                            position_type: PositionType::Absolute,
                                            top: Val::Px(0.0),
                                            bottom: Val::Px(0.0),
                                            left: Val::Px(0.0),
                                            right: Val::Px(0.0),
                                            ..default()
                                        },
                                        MaterialNode(handle),
                                        ZIndex(NOVA_OS_CONTENT_Z),
                                        Pickable::IGNORE,
                                    ));
                                }
                                _ => {
                                    // Headless fallback: terminal directly on-screen.
                                    spawn_nova_os_terminal_content(
                                        screen,
                                        font.clone(),
                                        &ship_name,
                                    );
                                }
                            }
                            spawn_nova_os_phosphor_rim(screen);
                            spawn_nova_os_glass_sheen(screen);
                        });
                });
            spawn_nova_os_chin(monitor, font.clone(), asset_server.as_deref(), &settings);
        });

    // Render-capable: populate the offscreen content root with the terminal (its
    // subtree renders through the image camera, not the window).
    if let Some((content_root, _)) = &rtt {
        commands.entity(*content_root).with_children(|root| {
            spawn_nova_os_terminal_content(root, font.clone(), &ship_name);
        });
    }
}

/// The PoC `.case` body: a 168deg gradient from a lit top through the mid body to
/// an almost-black undercut, with a 1px top highlight catching the moulding lip.
fn nova_os_case_gradient() -> BackgroundGradient {
    BackgroundGradient(vec![
        LinearGradient::degrees(
            168.0,
            vec![
                ColorStop::percent(NOVA_OS_CASE_LIT, 0.0),
                ColorStop::percent(NOVA_OS_CASE_MID, 26.0),
                ColorStop::percent(NOVA_OS_CASE_DEEP, 88.0),
                ColorStop::percent(Color::srgb_u8(4, 6, 8), 100.0),
            ],
        )
        .into(),
        // 1px lit moulding lip along the very top edge.
        LinearGradient::degrees(
            180.0,
            vec![
                ColorStop::px(NOVA_OS_CASE_HIGHLIGHT, 0.0),
                ColorStop::px(NOVA_OS_CASE_HIGHLIGHT, 1.0),
                ColorStop::px(Color::NONE, 1.0),
            ],
        )
        .into(),
    ])
}

/// The PoC `.bezel`: a dark vertical gradient giving the recessed lip its depth.
fn nova_os_bezel_gradient() -> BackgroundGradient {
    BackgroundGradient(vec![LinearGradient::degrees(
        180.0,
        vec![
            ColorStop::percent(Color::srgb_u8(18, 24, 29), 0.0),
            ColorStop::percent(Color::srgb_u8(7, 10, 13), 100.0),
        ],
    )
    .into()])
}

/// Four moulded corner screws (PoC `.screw`): a spherical head via a diagonal
/// light -> dark gradient over a full-radius disc, with a rotated slot line. The
/// slot is a FILLED bar, not a coloured border on a zero-content node, so it
/// dodges the border-collapse trap in the ledger
/// (`bevy-css-border-triangle-needs-contentbox`).
fn spawn_nova_os_casing_screws(parent: &mut ChildSpawnerCommands) {
    const DIAM_PX: f32 = 12.0;
    const INSET_PX: f32 = 15.0;
    for (name, left, top) in [
        ("NovaOsScrewTL", true, true),
        ("NovaOsScrewTR", false, true),
        ("NovaOsScrewBL", true, false),
        ("NovaOsScrewBR", false, false),
    ] {
        let mut node = Node {
            position_type: PositionType::Absolute,
            width: Val::Px(DIAM_PX),
            height: Val::Px(DIAM_PX),
            border: UiRect::all(Val::Px(1.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::MAX,
            ..default()
        };
        if left {
            node.left = Val::Px(INSET_PX);
        } else {
            node.right = Val::Px(INSET_PX);
        }
        if top {
            node.top = Val::Px(INSET_PX);
        } else {
            node.bottom = Val::Px(INSET_PX);
        }
        parent
            .spawn((
                Name::new(name),
                NovaOsScrewMarker,
                node,
                BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                BackgroundColor(NOVA_OS_SCREW_DARK),
                BackgroundGradient(vec![LinearGradient::degrees(
                    135.0,
                    vec![
                        ColorStop::percent(NOVA_OS_SCREW_LIT, 0.0),
                        ColorStop::percent(Color::srgb_u8(27, 33, 38), 62.0),
                        ColorStop::percent(NOVA_OS_SCREW_DARK, 100.0),
                    ],
                )
                .into()]),
                Pickable::IGNORE,
            ))
            .with_children(|screw| {
                screw.spawn((
                    Node {
                        width: Val::Px(8.0),
                        height: Val::Px(1.5),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
                    UiTransform::from_rotation(Rot2::degrees(38.0)),
                    Pickable::IGNORE,
                ));
            });
    }
}

/// The moulding seam running just inside the shell edge (PoC `.case::after`): a
/// 1px rounded outline, light along the top/left, dark along the bottom/right,
/// so the plastic reads as a moulded part with a parting line.
fn spawn_nova_os_moulding_seam(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Name::new("NovaOsMouldingSeam"),
        NovaOsSeamMarker,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            bottom: Val::Px(5.0),
            left: Val::Px(5.0),
            right: Val::Px(5.0),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius {
                top_left: Val::Px(NOVA_OS_CASE_RADIUS_TOP_PX - 4.0),
                top_right: Val::Px(NOVA_OS_CASE_RADIUS_TOP_PX - 4.0),
                bottom_left: Val::Px(NOVA_OS_CASE_RADIUS_BOTTOM_PX - 4.0),
                bottom_right: Val::Px(NOVA_OS_CASE_RADIUS_BOTTOM_PX - 4.0),
            },
            ..default()
        },
        BorderColor {
            top: Color::srgba(1.0, 1.0, 1.0, 0.05),
            left: Color::srgba(1.0, 1.0, 1.0, 0.05),
            bottom: Color::srgba(0.0, 0.0, 0.0, 0.5),
            right: Color::srgba(0.0, 0.0, 0.0, 0.5),
        },
        Pickable::IGNORE,
    ));
}

/// The top-centre vent grille (PoC `.vents`): a centred row of thin dark slats,
/// the case gradient showing through the gaps.
fn spawn_nova_os_casing_vents(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Name::new("NovaOsVents"),
            NovaOsVentMarker,
            Node {
                align_self: AlignSelf::Center,
                height: Val::Px(10.0),
                margin: UiRect::bottom(Val::Px(8.0)),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|vents| {
            for _ in 0..28 {
                vents.spawn((
                    Node {
                        width: Val::Px(4.0),
                        height: Val::Percent(100.0),
                        border_radius: BorderRadius::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
                    Pickable::IGNORE,
                ));
            }
        });
}

/// A faint phosphor halo tracing the screen edge (PoC `.rim`): a wider low-alpha
/// glow under a thin line, two nested rounded-border nodes at the screen
/// rounding. Drawn above the CRT overlay, below the glass. Kept deliberately
/// faint - the crisp, tube-bowed screen edge is now the shader's barrel-warped
/// rim (see DECISION.md); this is only the soft outer bloom, and the headless
/// fallback's sole edge cue.
fn spawn_nova_os_phosphor_rim(screen: &mut ChildSpawnerCommands) {
    for (name, border_px, color) in [
        (
            "NovaOsPhosphorRimGlow",
            3.0,
            NOVA_OS_PHOSPHOR.with_alpha(0.09),
        ),
        (
            "NovaOsPhosphorRimLine",
            1.0,
            NOVA_OS_PHOSPHOR.with_alpha(0.16),
        ),
    ] {
        screen.spawn((
            Name::new(name),
            NovaOsPhosphorRimMarker,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                border: UiRect::all(Val::Px(border_px)),
                border_radius: BorderRadius::all(Val::Px(NOVA_OS_SCREEN_RADIUS_PX)),
                ..default()
            },
            BorderColor::all(color),
            ZIndex(NOVA_OS_RIM_Z),
            Pickable::IGNORE,
        ));
    }
}

/// The glass specular sheen over the screen (PoC `.glass`): a diagonal white
/// gradient fading to clear, plus one soft angled highlight rectangle. The
/// frontmost surface layer; ignores picking so it never eats terminal input.
fn spawn_nova_os_glass_sheen(screen: &mut ChildSpawnerCommands) {
    screen
        .spawn((
            Name::new("NovaOsGlass"),
            NovaOsGlassMarker,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(Val::Px(NOVA_OS_SCREEN_RADIUS_PX)),
                ..default()
            },
            BackgroundGradient(vec![LinearGradient::degrees(
                118.0,
                vec![
                    ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.055), 0.0),
                    ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.016), 17.0),
                    ColorStop::percent(Color::NONE, 33.0),
                ],
            )
            .into()]),
            ZIndex(NOVA_OS_GLASS_Z),
            Pickable::IGNORE,
        ))
        .with_children(|glass| {
            // A soft upper-left reflection. A RADIAL gradient (not a solid fill)
            // fades to transparent at the edges, so it reads as a soft glass
            // catch instead of the hard-edged card a blur-less solid node gives.
            glass.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(6.0),
                    top: Val::Percent(7.0),
                    width: Val::Percent(26.0),
                    height: Val::Percent(40.0),
                    ..default()
                },
                BackgroundGradient(vec![Gradient::from(RadialGradient::new(
                    UiPosition::CENTER,
                    RadialGradientShape::ClosestSide,
                    vec![
                        ColorStop::percent(Color::srgba(0.82, 0.92, 1.0, 0.06), 0.0),
                        ColorStop::percent(Color::srgba(0.82, 0.92, 1.0, 0.02), 55.0),
                        ColorStop::percent(Color::NONE, 100.0),
                    ],
                ))]),
                UiTransform::from_rotation(Rot2::degrees(-14.0)),
                Pickable::IGNORE,
            ));
        });
}

/// The bottom casing chin (PoC `.chin`): the recessed brand plate on the left
/// and a reserved, initially empty controls row on the right (its functional
/// knobs are task 20260726-214617, which depends on this geometry).
fn spawn_nova_os_chin(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    asset_server: Option<&AssetServer>,
    settings: &NovaOsMonitorSettings,
) {
    parent
        .spawn((
            Name::new("NovaOsChin"),
            NovaOsChinMarker,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(NOVA_OS_CHIN_HEIGHT_PX),
                // Wide left/right padding so the plate + controls clear the
                // bottom corner screws (screws inset ~15px, ~12px wide).
                padding: UiRect {
                    left: Val::Px(40.0),
                    right: Val::Px(40.0),
                    top: Val::Px(11.0),
                    bottom: Val::Px(4.0),
                },
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(14.0),
                ..default()
            },
        ))
        .with_children(|chin| {
            chin.spawn((
                Name::new("NovaOsBrandPlate"),
                NovaOsBrandPlateMarker,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(11.0),
                    padding: UiRect {
                        left: Val::Px(11.0),
                        right: Val::Px(14.0),
                        top: Val::Px(7.0),
                        bottom: Val::Px(7.0),
                    },
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
                    ..default()
                },
                // Recessed badge, matching the PoC `.plate`: a base DARKER than
                // the surrounding case, really dark edges, and a top(dark) ->
                // bottom(light-ish grey) gradient with a light lower catch, so it
                // reads pressed a little INTO the plastic (a 3D inset).
                BorderColor {
                    top: Color::srgba(0.0, 0.0, 0.0, 0.8),
                    left: Color::srgb_u8(3, 4, 6),
                    right: Color::srgb_u8(3, 4, 6),
                    bottom: Color::srgba(1.0, 1.0, 1.0, 0.11),
                },
                BackgroundColor(NOVA_OS_CASE_EDGE),
                BackgroundGradient(vec![LinearGradient::degrees(
                    180.0,
                    vec![
                        ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.45), 0.0),
                        ColorStop::percent(Color::srgba(0.82, 0.86, 0.91, 0.16), 100.0),
                    ],
                )
                .into()]),
            ))
            .with_children(|plate| {
                // Logo mark: SVG rendered to a PNG asset (Bevy UI cannot draw SVG).
                if let Some(asset_server) = asset_server {
                    plate.spawn((
                        Name::new("NovaOsBrandMark"),
                        ImageNode::new(asset_server.load("icons/nova_crt_mark.png")),
                        Node {
                            width: Val::Px(22.0),
                            height: Val::Px(22.0),
                            ..default()
                        },
                    ));
                }
                plate
                    .spawn((
                        Name::new("NovaOsBrandText"),
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(3.0),
                            ..default()
                        },
                    ))
                    .with_children(|text| {
                        // Dark glyphs stamped INTO the plastic, with a light catch
                        // along the lower edge (a hard 1px offset, no blur - the
                        // pressed-in look, not the doubled text a blur would give).
                        text.spawn((
                            Name::new("NovaOsBrandWordmark"),
                            Text::new("NOVACRT 9000"),
                            nova_os_text_font(12.0, font.clone()),
                            TextColor(Color::srgb_u8(12, 16, 19)),
                            TextShadow {
                                offset: Vec2::new(0.0, 1.0),
                                color: Color::srgba(1.0, 1.0, 1.0, 0.12),
                            },
                        ));
                        text.spawn((
                            Name::new("NovaOsBrandSpec"),
                            Text::new("P22 GREEN PHOSPHOR . 15 IN . TYPE CQ-4"),
                            nova_os_text_font(8.0, font.clone()),
                            TextColor(Color::srgb_u8(16, 23, 27)),
                            TextShadow {
                                offset: Vec2::new(0.0, 1.0),
                                color: Color::srgba(1.0, 1.0, 1.0, 0.085),
                            },
                        ));
                    });
            });
            // Controls row: the working BRIGHT/SCAN knobs and SND/PWR buttons.
            chin.spawn((
                Name::new("NovaOsControlsRow"),
                NovaOsControlsRowMarker,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexEnd,
                    column_gap: Val::Px(14.0),
                    min_width: Val::Px(120.0),
                    min_height: Val::Px(26.0),
                    ..default()
                },
            ))
            .with_children(|controls| {
                spawn_nova_os_knob(
                    controls,
                    font.clone(),
                    settings,
                    NovaOsKnob::Bright,
                    "BRIGHT",
                );
                spawn_nova_os_knob(controls, font.clone(), settings, NovaOsKnob::Scan, "SCAN");
                spawn_nova_os_sound_button(controls, font.clone(), settings);
                spawn_nova_os_power_button(controls, font.clone());
            });
        });
}

/// One rotary knob (BRIGHT or SCAN): a clickable dial that cycles its 4 detents
/// on each press (PoC `.knob`), the pointer rotating to the detent angle, with a
/// small caption beneath. Spawns with the dial already rotated to the current
/// detent so a reopen shows the saved position; live turns are re-synced by
/// [`sync_nova_os_monitor_controls`].
fn spawn_nova_os_knob(
    controls: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    settings: &NovaOsMonitorSettings,
    knob: NovaOsKnob,
    caption: &str,
) {
    let mut knob_cmd = controls.spawn((
        Name::new(format!("NovaOsKnob({caption})")),
        knob,
        Button,
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(3.0),
            ..default()
        },
    ));
    // Each knob cycles its own detent; the observer type differs per knob, so
    // attach it via EntityCommands rather than a shared bundle.
    match knob {
        NovaOsKnob::Bright => knob_cmd.observe(on_nova_os_bright_knob),
        NovaOsKnob::Scan => knob_cmd.observe(on_nova_os_scan_knob),
    };
    knob_cmd.with_children(|knob_node| {
        // The dial face: a dark moulded disc with a raised rim.
        knob_node
            .spawn((
                Name::new("NovaOsKnobDial"),
                NovaOsKnobDialMarker,
                knob,
                Node {
                    width: Val::Px(26.0),
                    height: Val::Px(26.0),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(NOVA_OS_DIAL_DARK),
                // Domed knob body: an off-centre highlight (PoC `circle at 34% 28%`)
                // falling to the dark disc rim reads as a rounded rotary.
                BackgroundGradient(vec![Gradient::from(RadialGradient::new(
                    UiPosition::anchor(Vec2::new(-0.16, -0.22)),
                    RadialGradientShape::ClosestSide,
                    vec![
                        ColorStop::percent(NOVA_OS_DIAL_LIT, 0.0),
                        ColorStop::percent(NOVA_OS_DIAL_MID, 58.0),
                        ColorStop::percent(NOVA_OS_DIAL_DARK, 100.0),
                    ],
                ))]),
                // A near-black inner rim (PoC `inset 0 0 0 1px rgba(0,0,0,0.8)`).
                BorderColor::all(NOVA_OS_BUTTON_BORDER),
                UiTransform::from_rotation(Rot2::degrees(settings.dial_angle(knob))),
                // The knob click is owned by the parent Button; the dial and
                // its pointer must not intercept the pick.
                Pickable::IGNORE,
            ))
            .with_children(|dial| {
                // Pointer: a bright phosphor tick near the top, sweeping as the
                // dial rotates around its centre.
                dial.spawn((
                    Name::new("NovaOsKnobPointer"),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(2.0),
                        height: Val::Px(9.0),
                        top: Val::Px(2.0),
                        left: Val::Px(11.0),
                        ..default()
                    },
                    BackgroundColor(NOVA_OS_PHOSPHOR),
                    Pickable::IGNORE,
                ));
            });
        knob_node.spawn((
            Name::new("NovaOsKnobCaption"),
            Text::new(caption),
            nova_os_text_font(7.0, font),
            TextColor(NOVA_OS_PHOSPHOR_MUTED),
            Pickable::IGNORE,
        ));
    });
}

/// The SND speaker toggle (PoC `#soundBtn`): flips
/// [`NovaOsMonitorSettings::sound_enabled`], its indicator lit when armed and the
/// label reading "SND ON"/"SND OFF". Spawns matching the current state; live
/// flips are re-synced by [`sync_nova_os_monitor_controls`].
fn spawn_nova_os_sound_button(
    controls: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    settings: &NovaOsMonitorSettings,
) {
    let on = settings.sound_enabled;
    controls.spawn((
        Name::new("NovaOsSoundButton"),
        NovaOsSoundButtonMarker,
        Button,
        nova_os_chin_button_node(),
        // The button chrome is static now; the bulb (below), not the border or
        // the label, carries the on/off state (owner playtest).
        BorderColor::all(NOVA_OS_BUTTON_BORDER),
        BackgroundColor(NOVA_OS_BUTTON_DEEP),
        nova_os_chin_button_gradient(),
        observe(on_nova_os_sound_button),
        children![
            (
                Name::new("NovaOsSoundIndicator"),
                NovaOsSoundIndicatorMarker,
                nova_os_bulb_node(),
                BackgroundColor(nova_os_bulb_color(on)),
                nova_os_bulb_gradient(),
                Pickable::IGNORE,
            ),
            (
                // A fixed legend: it never swaps text, so the bulb is the only
                // moving part reporting state.
                Name::new("NovaOsSoundLabel"),
                Text::new("SND"),
                nova_os_text_font(9.0, font),
                TextColor(NOVA_OS_TEXT),
                Pickable::IGNORE,
            ),
        ],
    ));
}

/// The PWR button + green power LED (PoC `#powerBtn`): pressing it drives the
/// existing animated close, the diegetic twin of the `exit` command.
fn spawn_nova_os_power_button(controls: &mut ChildSpawnerCommands, font: Handle<Font>) {
    controls.spawn((
        Name::new("NovaOsPowerButton"),
        NovaOsPowerButtonMarker,
        Button,
        nova_os_chin_button_node(),
        BorderColor::all(NOVA_OS_BUTTON_BORDER),
        BackgroundColor(NOVA_OS_BUTTON_DEEP),
        nova_os_chin_button_gradient(),
        observe(on_nova_os_power_button),
        children![
            (
                Name::new("NovaOsPowerLed"),
                NovaOsPowerLedMarker,
                nova_os_bulb_node(),
                // Lit green while powered; flashes orange during the power-down
                // close (see `drive_nova_os_power_led`).
                BackgroundColor(NOVA_OS_PHOSPHOR),
                nova_os_bulb_gradient(),
                Pickable::IGNORE,
            ),
            (
                Name::new("NovaOsPowerLabel"),
                Text::new("PWR"),
                nova_os_text_font(9.0, font),
                TextColor(NOVA_OS_TEXT),
                Pickable::IGNORE,
            ),
        ],
    ));
}

/// Shared node style for the SND/PWR chin buttons (PoC `.power-btn`): a small
/// pill with an indicator glyph beside a caption.
fn nova_os_chin_button_node() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
        border: UiRect::all(Val::Px(1.0)),
        border_radius: BorderRadius::all(Val::Px(4.0)),
        ..default()
    }
}

/// The moulded 3D fill of a chin button (PoC `.power-btn`): a top-lit vertical
/// gradient over the dark base, plus a 1px inner top-highlight lip so the key
/// catches the light like raised plastic.
fn nova_os_chin_button_gradient() -> BackgroundGradient {
    BackgroundGradient(vec![
        LinearGradient::degrees(
            180.0,
            vec![
                ColorStop::percent(NOVA_OS_BUTTON_LIT, 0.0),
                ColorStop::percent(NOVA_OS_BUTTON_DEEP, 100.0),
            ],
        )
        .into(),
        // 1px lit lip along the top edge (PoC `inset 0 1px 0 rgba(255,255,255,.12)`).
        LinearGradient::degrees(
            180.0,
            vec![
                ColorStop::px(NOVA_OS_CASE_HIGHLIGHT, 0.0),
                ColorStop::px(NOVA_OS_CASE_HIGHLIGHT, 1.0),
                ColorStop::px(Color::NONE, 1.0),
            ],
        )
        .into(),
    ])
}

/// A 7px round indicator bulb inside a chin button (the SND / PWR LED). Its base
/// colour reports state; the glassy cap is fixed by [`nova_os_bulb_gradient`].
fn nova_os_bulb_node() -> Node {
    Node {
        width: Val::Px(7.0),
        height: Val::Px(7.0),
        border_radius: BorderRadius::MAX,
        ..default()
    }
}

/// A fixed upper-left glassy highlight over an indicator bulb, so the lit and
/// unlit states both read as a rounded glass LED rather than a flat dot.
fn nova_os_bulb_gradient() -> BackgroundGradient {
    BackgroundGradient(vec![Gradient::from(RadialGradient::new(
        UiPosition::anchor(Vec2::new(-0.2, -0.25)),
        RadialGradientShape::ClosestSide,
        vec![
            ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.5), 0.0),
            ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.12), 45.0),
            ColorStop::percent(Color::NONE, 100.0),
        ],
    ))])
}

/// Lit phosphor vs unlit dark-green for the SND bulb: the bulb going dark is how
/// the muted state reads now that the label never changes.
fn nova_os_bulb_color(on: bool) -> Color {
    if on {
        NOVA_OS_PHOSPHOR
    } else {
        NOVA_OS_BULB_OFF
    }
}

fn spawn_nova_os_terminal_content(
    screen: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    ship_name: &str,
) {
    screen
        .spawn((
            Name::new("NovaOsTerminalContent"),
            NovaOsTerminalContentMarker,
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            ZIndex(NOVA_OS_CONTENT_Z),
            Pickable::IGNORE,
        ))
        .with_children(|terminal| {
            terminal
                .spawn((
                    NovaOsTopbarMarker,
                    Node {
                        min_height: Val::Px(32.0),
                        padding: UiRect::bottom(Val::Px(10.0)),
                        border: UiRect::bottom(Val::Px(1.0)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(12.0),
                        ..default()
                    },
                    BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.36)),
                ))
                .with_children(|topbar| {
                    topbar
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            min_width: Val::Px(0.0),
                            ..default()
                        })
                        .with_children(|brand| {
                            brand.spawn((
                                NovaOsLampMarker,
                                Node {
                                    width: Val::Px(10.0),
                                    height: Val::Px(10.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    flex_shrink: 0.0,
                                    ..default()
                                },
                                BorderColor::all(NOVA_OS_PHOSPHOR),
                                BackgroundColor(NOVA_OS_PHOSPHOR),
                            ));
                            brand.spawn((
                                Text::new(format!(
                                    "NOVA OS {} / COCKPIT LINK",
                                    nova_os_version_label()
                                )),
                                nova_os_text_font(DRAWER_SECTION_TITLE_FONT_PX, font.clone()),
                                TextColor(NOVA_OS_PHOSPHOR),
                            ));
                        });
                    topbar.spawn((
                        NovaOsStatusMarker,
                        Text::new(nova_os_status_text(ship_name, None)),
                        nova_os_text_font(DRAWER_SECTION_TITLE_FONT_PX, font.clone()),
                        TextColor(NOVA_OS_PHOSPHOR_DIM),
                    ));
                });

            terminal
                .spawn((
                    NovaOsTerminalSurfaceMarker,
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.36)),
                    BackgroundColor(Color::srgba(0.0, 5.0 / 255.0, 2.0 / 255.0, 0.72)),
                ))
                .with_children(|terminal_panel| {
                    terminal_panel
                        .spawn((
                            NovaOsTerminalScrollbackMarker,
                            NovaOsScrollViewportMarker,
                            ScrollPosition::default(),
                            Hovered::default(),
                            Node {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                min_height: Val::Px(0.0),
                                padding: UiRect::axes(
                                    Val::Px(NOVA_OS_TERMINAL_PAD_X_PX),
                                    Val::Px(NOVA_OS_TERMINAL_PAD_Y_PX),
                                ),
                                overflow: Overflow::scroll_y(),
                                row_gap: Val::Px(5.0),
                                ..default()
                            },
                        ))
                        .with_children(|scrollback| {
                            for row in nova_os_welcome_rows() {
                                spawn_terminal_row(scrollback, &row, font.clone());
                            }
                        });
                    terminal_panel
                        .spawn((
                            NovaOsPromptRowMarker,
                            Node {
                                min_height: Val::Px(NOVA_OS_PROMPT_ROW_HEIGHT_PX),
                                padding: UiRect::axes(
                                    Val::Px(NOVA_OS_TERMINAL_PAD_X_PX),
                                    Val::Px(7.0),
                                ),
                                border: UiRect::top(Val::Px(1.0)),
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                row_gap: Val::Px(2.0),
                                ..default()
                            },
                            BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.45)),
                            // Near-opaque black-green so the input reads as a
                            // dark box sitting ABOVE the screen (HTML `.prompt-row`).
                            BackgroundColor(Color::srgba(0.0, 0.016, 0.008, 0.97)),
                            ZIndex(NOVA_OS_OVERLAY_Z + 1),
                        ))
                        .with_children(|prompt_row| {
                            prompt_row
                                .spawn((
                                    NovaOsPromptInputLineMarker,
                                    Node {
                                        width: Val::Percent(100.0),
                                        min_height: Val::Px(24.0),
                                        flex_direction: FlexDirection::Row,
                                        align_items: AlignItems::Center,
                                        column_gap: Val::Px(8.0),
                                        min_width: Val::Px(0.0),
                                        ..default()
                                    },
                                ))
                                .with_children(|input_line| {
                                    input_line.spawn((
                                        NovaOsPromptPrefixMarker,
                                        Text::new("nova>"),
                                        nova_os_text_font(DRAWER_LINE_FONT_PX, font.clone()),
                                        TextColor(NOVA_OS_AMBER),
                                        Node {
                                            flex_shrink: 0.0,
                                            ..default()
                                        },
                                    ));
                                    input_line
                                        .spawn((
                                            NovaOsPromptInputWrapMarker,
                                            Node {
                                                flex_grow: 1.0,
                                                min_width: Val::Px(0.0),
                                                // Hold the wrap at a full text line
                                                // box even when every piece is empty:
                                                // the block caret is absolute with
                                                // top:0/bottom:0, so it stretches to
                                                // THIS node. With 0 chars typed all
                                                // three text children are "" and the
                                                // wrap would collapse to 0 height,
                                                // hiding the caret (owner playtest:
                                                // caret invisible before typing). 1.2
                                                // is Bevy's default line-height factor
                                                // (not an arbitrary pad), so this floor
                                                // equals the line box the caret stretches
                                                // to once text is present - empty and
                                                // typed carets stay the same height.
                                                min_height: Val::Px(DRAWER_LINE_FONT_PX * 1.2),
                                                flex_direction: FlexDirection::Row,
                                                align_items: AlignItems::Center,
                                                overflow: Overflow::clip_x(),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|input_wrap| {
                                            // Fish-style inline input: typed text
                                            // left of the caret, a block caret,
                                            // typed text right of it, then the dim
                                            // completion ghost - all NoWrap so the
                                            // completion continues on the SAME line.
                                            input_wrap.spawn((
                                                NovaOsTerminalPromptMarker,
                                                Text::new(""),
                                                nova_os_text_font(
                                                    DRAWER_LINE_FONT_PX,
                                                    font.clone(),
                                                ),
                                                TextColor(NOVA_OS_PHOSPHOR),
                                                nova_os_prompt_text_layout(),
                                                Node {
                                                    flex_shrink: 0.0,
                                                    ..default()
                                                },
                                                ZIndex(1),
                                            ));
                                            input_wrap.spawn((
                                                NovaOsTerminalCaretMarker,
                                                Node {
                                                    // ABSOLUTE so the block does
                                                    // not advance the row: it sits
                                                    // OVER the character at the
                                                    // cursor - the first completion
                                                    // ghost letter when the cursor
                                                    // is at the end - instead of
                                                    // pushing the ghost one cell to
                                                    // the right (owner playtest).
                                                    // `left` is set from the MEASURED
                                                    // typed-text width by
                                                    // `position_nova_os_block_caret`;
                                                    // top + bottom stretch it to the
                                                    // line height (PoC `.caret`, a
                                                    // block one glyph wide).
                                                    position_type: PositionType::Absolute,
                                                    left: Val::Px(0.0),
                                                    top: Val::Px(0.0),
                                                    bottom: Val::Px(0.0),
                                                    width: Val::Px(
                                                        DRAWER_LINE_FONT_PX
                                                            * NOVA_OS_CARET_WIDTH_FRACTION,
                                                    ),
                                                    ..default()
                                                },
                                                // Slightly translucent so the letter
                                                // under the block still reads (PoC
                                                // `.caret` opacity 0.85).
                                                BackgroundColor(NOVA_OS_AMBER.with_alpha(0.85)),
                                                ZIndex(2),
                                            ));
                                            input_wrap.spawn((
                                                NovaOsTerminalPromptAfterMarker,
                                                Text::new(""),
                                                nova_os_text_font(
                                                    DRAWER_LINE_FONT_PX,
                                                    font.clone(),
                                                ),
                                                TextColor(NOVA_OS_PHOSPHOR),
                                                nova_os_prompt_text_layout(),
                                                Node {
                                                    flex_shrink: 0.0,
                                                    ..default()
                                                },
                                                ZIndex(1),
                                            ));
                                            input_wrap.spawn((
                                                NovaOsTerminalGhostMarker,
                                                Text::new(""),
                                                nova_os_text_font(
                                                    DRAWER_LINE_FONT_PX,
                                                    font.clone(),
                                                ),
                                                TextColor(NOVA_OS_TEXT.with_alpha(0.34)),
                                                nova_os_prompt_text_layout(),
                                                Node {
                                                    flex_shrink: 0.0,
                                                    ..default()
                                                },
                                            ));
                                        });
                                });
                            prompt_row.spawn((
                                NovaOsTerminalHintMarker,
                                Text::new(""),
                                nova_os_text_font(12.0, font.clone()),
                                TextColor(NOVA_OS_PHOSPHOR_MUTED),
                                Node {
                                    width: Val::Percent(100.0),
                                    min_height: Val::Px(16.0),
                                    min_width: Val::Px(0.0),
                                    ..default()
                                },
                            ));
                        });
                });

            terminal
                .spawn((
                    NovaOsFooterHintsMarker,
                    Node {
                        min_height: Val::Px(18.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        // Wrap so the full keybind set never overflows the row on a
                        // narrow screen; `rebuild_nova_os_footer_hints` refills these
                        // from the same `NOVA_OS_TERMINAL_HINTS` on the first mode eval.
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(12.0),
                        row_gap: Val::Px(2.0),
                        ..default()
                    },
                ))
                .with_children(|footer| {
                    for &hint in NOVA_OS_TERMINAL_HINTS {
                        footer.spawn((
                            Text::new(hint),
                            nova_os_text_font(11.0, font.clone()),
                            TextColor(NOVA_OS_PHOSPHOR_MUTED),
                        ));
                    }
                });
        });
}

/// Despawn the NOVA OS shell when the player ship goes away.
#[allow(clippy::type_complexity)]
fn remove_nova_os(
    _remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    mut log: ResMut<NovaOsFlightLog>,
    mut terminal: ResMut<NovaOsTerminal>,
    q_parts: Query<
        Entity,
        Or<(
            With<NovaOsRootMarker>,
            With<NovaOsBackdropMarker>,
            With<NovaOsImageCameraMarker>,
            With<NovaOsImageContentRootMarker>,
            With<NovaOsForwardedPointerMarker>,
        )>,
    >,
) {
    log.clear();
    terminal.reset_session();
    for entity in &q_parts {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<NovaOsRtt>();
}

#[cfg(test)]
mod tests {
    use bevy::{
        asset::AssetPlugin, ecs::system::RunSystemOnce, input::touch::TouchPhase,
        state::app::StatesPlugin,
    };
    use bevy_common_systems::prelude::{Objective, PlaySfx};

    use super::*;

    /// A headless app with just the states + the NOVA OS toggle, enough to drive
    /// the interaction-model state machine.
    fn toggle_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
        app.init_state::<PauseStates>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<NovaOsCloseTransition>();
        app.add_systems(Update, toggle_nova_os.run_if(in_state(GameStates::Playing)));
        // Enter Playing so the toggle runs.
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app
    }

    fn press_tab(app: &mut App) {
        if let Some(mut keyboard) = app
            .world_mut()
            .get_resource_mut::<Messages<KeyboardInput>>()
        {
            keyboard.write(KeyboardInput {
                key_code: KeyCode::Tab,
                logical_key: Key::Tab,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        // Clear the just_pressed edge like nova_menu's `press_escape` (no
        // InputPlugin in this rig, so nothing clears it automatically - a stale
        // edge would re-fire the toggle on the next update).
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Tab);
        keys.clear();
        app.update();
    }

    fn press_key(app: &mut App, key_code: KeyCode, logical_key: Key, text: Option<&str>) {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Pressed,
            text: text.map(Into::into),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }

    fn press_text(app: &mut App, text: &str) {
        press_key(app, KeyCode::KeyA, Key::Character(text.into()), Some(text));
    }

    fn type_text(terminal: &mut NovaOsTerminal, text: &str) {
        terminal.insert_text(text);
    }

    fn init_terminal_input_resources(app: &mut App) {
        app.init_resource::<NovaOsTerminal>();
        app.init_resource::<NovaOsFlightLog>();
        app.init_resource::<GameObjectives>();
        // handle_terminal_keyboard reads the SND toggle (task 20260726-214639).
        app.init_resource::<NovaOsMonitorSettings>();
        app.world_mut().init_resource::<Messages<KeyboardInput>>();
    }

    fn terminal_command_app() -> App {
        let mut app = toggle_app();
        init_terminal_input_resources(&mut app);
        app.add_systems(
            Update,
            handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
        );
        press_tab(&mut app);
        assert_eq!(pause_state(&app), PauseStates::NovaOs);
        app
    }

    fn submit_terminal_command(app: &mut App, command: &str) {
        press_text(app, command);
        press_key(app, KeyCode::Enter, Key::Enter, None);
        app.update();
    }

    fn terminal_scrollback_texts(app: &App) -> Vec<String> {
        app.world()
            .resource::<NovaOsTerminal>()
            .scrollback()
            .iter()
            .map(|row| row.text.clone())
            .collect()
    }

    fn pause_state(app: &App) -> PauseStates {
        app.world().resource::<State<PauseStates>>().get().clone()
    }

    // --- NOVA OS sound (task 20260726-214639) ---

    /// Records which NOVA OS cues were triggered (by handle identity), so tests
    /// can assert WHICH sound played on each terminal event without an audio
    /// device. Mirrors `objective_feedback`'s `sfx_app` capture.
    #[derive(Resource, Default)]
    struct SoundCapture(Vec<UiSfx>);

    fn nova_os_sound_app() -> App {
        let mut app = toggle_app();
        init_terminal_input_resources(&mut app);
        app.init_resource::<NovaOsCloseTransition>();
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<AudioSource>();
        let bank = SoundBank::load(
            app.world().resource::<AssetServer>(),
            crate::audio::UI_SFX_FILES,
        );
        app.insert_resource(bank);
        app.init_resource::<SoundCapture>();
        app.add_observer(
            |sfx: On<PlaySfx>, bank: Res<SoundBank<UiSfx>>, mut cap: ResMut<SoundCapture>| {
                for (key, _) in crate::audio::UI_SFX_FILES {
                    if sfx.handle == bank.get(key) {
                        cap.0.push(key);
                        break;
                    }
                }
            },
        );
        app.add_systems(
            Update,
            (
                handle_terminal_keyboard,
                play_nova_os_power_down.run_if(in_state(PauseStates::NovaOs)),
            )
                .run_if(in_state(GameStates::Playing)),
        );
        app.add_systems(OnEnter(PauseStates::NovaOs), start_nova_os_sound);
        app.add_systems(OnExit(PauseStates::NovaOs), stop_nova_os_bed);
        app
    }

    fn open_nova_os(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::NovaOs);
        app.update();
    }

    fn clear_capture(app: &mut App) {
        app.world_mut().resource_mut::<SoundCapture>().0.clear();
    }

    fn fired(app: &App, cue: UiSfx) -> bool {
        app.world().resource::<SoundCapture>().0.contains(&cue)
    }

    fn bed_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<NovaOsBedSfx>>()
            .iter(app.world())
            .count()
    }

    fn set_prompt(app: &mut App, command: &str) {
        let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
        terminal.reset_prompt();
        terminal.insert_text(command);
    }

    fn press_enter(app: &mut App) {
        press_key(app, KeyCode::Enter, Key::Enter, None);
    }

    #[test]
    fn nova_os_sound_cues_fire_on_terminal_events() {
        let mut app = nova_os_sound_app();

        // Open: the power-up sweep plays and the ambient bed spawns.
        open_nova_os(&mut app);
        assert!(
            fired(&app, UiSfx::NovaOsPowerUp),
            "opening the computer plays the power-up sweep"
        );
        assert_eq!(bed_count(&mut app), 1, "the ambient bed spawns on open");

        // A keystroke plays the (throttled) typing click.
        clear_capture(&mut app);
        set_prompt(&mut app, "");
        press_text(&mut app, "h");
        assert!(fired(&app, UiSfx::NovaOsKey), "typing plays the key click");

        // A valid command: the enter thunk plus the confirmation beep.
        clear_capture(&mut app);
        set_prompt(&mut app, "help");
        press_enter(&mut app);
        assert!(
            fired(&app, UiSfx::NovaOsEnter),
            "submitting plays the enter thunk"
        );
        assert!(
            fired(&app, UiSfx::NovaOsOk),
            "a valid command plays the ok beep"
        );

        // An unknown command: the error buzz.
        clear_capture(&mut app);
        set_prompt(&mut app, "zzz");
        press_enter(&mut app);
        assert!(
            fired(&app, UiSfx::NovaOsError),
            "an unknown command plays the error buzz"
        );

        // Requesting a close plays the power-down sweep.
        clear_capture(&mut app);
        app.world_mut()
            .resource_mut::<NovaOsCloseTransition>()
            .closing = true;
        app.update();
        assert!(
            fired(&app, UiSfx::NovaOsPowerDown),
            "requesting a close plays the power-down sweep"
        );
    }

    #[test]
    fn nova_os_ambient_bed_tracks_nova_os_state() {
        let mut app = nova_os_sound_app();
        assert_eq!(bed_count(&mut app), 0, "no bed before the computer opens");

        open_nova_os(&mut app);
        assert_eq!(bed_count(&mut app), 1, "one bed while the computer is open");

        // Leaving the NOVA OS despawns the bed. (The freeze loop-pause exemption
        // is structural, not exercised here: `audio::pause_loops` queries only
        // ThrusterLoopSfx/RcsLoopSfx, so NovaOsBedSfx is never paused - see the
        // task note. Asserting the sink stays playing would need an audio
        // device.)
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
        app.update();
        assert_eq!(
            bed_count(&mut app),
            0,
            "the bed despawns when the computer closes"
        );
    }

    #[test]
    fn nova_os_snd_off_silences_cues() {
        let mut app = nova_os_sound_app();
        app.world_mut()
            .resource_mut::<NovaOsMonitorSettings>()
            .sound_enabled = false;

        // Open with SND off: no power-up cue (the bed still spawns, but silent -
        // apply_nova_os_bed_volume drives it to 0).
        open_nova_os(&mut app);
        assert!(
            !fired(&app, UiSfx::NovaOsPowerUp),
            "SND off silences the power-up sweep"
        );

        // Typing and submitting are silent too.
        set_prompt(&mut app, "help");
        press_enter(&mut app);
        assert!(
            app.world().resource::<SoundCapture>().0.is_empty(),
            "SND off silences every terminal cue, got {:?}",
            app.world().resource::<SoundCapture>().0
        );
    }

    #[test]
    fn nova_os_bed_gain_respects_snd_and_master() {
        // The bed's volume logic (the sink write needs an audio device, so the
        // gain is factored out pure). SND on at full master -> the base volume.
        assert_eq!(nova_os_bed_gain(true, 1.0), NOVA_OS_BED_VOLUME);
        // SND off -> dead silent, whatever the master.
        assert_eq!(nova_os_bed_gain(false, 1.0), 0.0);
        // A zero master output gain (volume 0 OR a HarnessMute'd run) -> silent.
        assert_eq!(nova_os_bed_gain(true, 0.0), 0.0);
        // Half master scales the hum.
        assert_eq!(nova_os_bed_gain(true, 0.5), NOVA_OS_BED_VOLUME * 0.5);
    }

    #[test]
    fn tab_toggles_nova_os_state() {
        let mut app = toggle_app();
        assert_eq!(pause_state(&app), PauseStates::Unpaused);
        press_tab(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::NovaOs,
            "Tab from Unpaused opens the NOVA OS"
        );
        press_tab(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::NovaOs,
            "Tab inside the NOVA OS stays with NOVA OS so the terminal can autocomplete"
        );
    }

    #[test]
    fn tab_opens_nova_os_then_completes_terminal_command() {
        let mut app = toggle_app();
        init_terminal_input_resources(&mut app);
        app.add_systems(
            Update,
            handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
        );

        press_tab(&mut app);
        assert_eq!(pause_state(&app), PauseStates::NovaOs);
        press_text(&mut app, "he");
        press_tab(&mut app);

        let terminal = app.world().resource::<NovaOsTerminal>();
        assert_eq!(terminal.prompt(), "help");
        assert_eq!(terminal.cursor(), 4);
    }

    #[test]
    fn terminal_ignores_text_typed_before_nova_os_opens() {
        let mut app = toggle_app();
        init_terminal_input_resources(&mut app);
        app.add_systems(
            Update,
            handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
        );

        press_text(&mut app, "flight");
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().prompt(),
            "",
            "keyboard text typed during flight is drained but not inserted"
        );

        press_tab(&mut app);
        assert_eq!(pause_state(&app), PauseStates::NovaOs);
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().prompt(),
            "",
            "opening the NOVA OS does not replay stale flight text into the prompt"
        );
    }

    #[test]
    fn keyboard_input_updates_visible_prompt_text() {
        let mut app = toggle_app();
        init_terminal_input_resources(&mut app);
        app.add_systems(
            Update,
            (handle_terminal_keyboard, rebuild_terminal_ui)
                .chain()
                .run_if(in_state(GameStates::Playing)),
        );
        spawn_nova_os_shell(&mut app);

        press_tab(&mut app);
        assert_eq!(pause_state(&app), PauseStates::NovaOs);
        press_text(&mut app, "he");

        let prompt = app
            .world_mut()
            .query_filtered::<&Text, With<NovaOsTerminalPromptMarker>>()
            .single(app.world())
            .expect("one visible prompt text entity");
        assert_eq!(
            prompt.0, "he",
            "typed text left of the caret, no baked-in `|`"
        );

        let ghost = app
            .world_mut()
            .query_filtered::<&Text, With<NovaOsTerminalGhostMarker>>()
            .single(app.world())
            .expect("one visible ghost text entity");
        assert_eq!(
            ghost.0, "lp",
            "completion continues inline with no leading space (fish-style)"
        );
    }

    #[test]
    fn nova_os_inline_completion_is_same_line_continuation() {
        // The ghost, the before-cursor and after-cursor prompt pieces must all
        // render with `NoWrap` so a completion never wraps below the input line
        // (the reported "completion appears below the line" bug).
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "hel");
        assert_eq!(prompt_before_cursor(&terminal), "hel");
        assert_eq!(prompt_after_cursor(&terminal), "");
        assert_eq!(
            prompt_completion_ghost(&terminal),
            "p",
            "ghost is the raw suffix, no leading space"
        );

        // With the caret moved into the middle, the block caret splits the typed
        // text: `he` renders left of it, `lp` (from a full `help`) to its right.
        type_text(&mut terminal, "p");
        terminal.move_cursor_left();
        terminal.move_cursor_left();
        assert_eq!(prompt_before_cursor(&terminal), "he");
        assert_eq!(prompt_after_cursor(&terminal), "lp");

        let mut app = toggle_app();
        init_terminal_input_resources(&mut app);
        app.add_systems(
            Update,
            (handle_terminal_keyboard, rebuild_terminal_ui)
                .chain()
                .run_if(in_state(GameStates::Playing)),
        );
        spawn_nova_os_shell(&mut app);
        press_tab(&mut app);
        press_text(&mut app, "hel");

        for marker_layout in app
            .world_mut()
            .query_filtered::<&TextLayout, Or<(
                With<NovaOsTerminalPromptMarker>,
                With<NovaOsTerminalPromptAfterMarker>,
                With<NovaOsTerminalGhostMarker>,
            )>>()
            .iter(app.world())
        {
            assert_eq!(
                marker_layout.linebreak,
                LineBreak::NoWrap,
                "prompt/ghost pieces must not wrap to a line below the input"
            );
        }
    }

    #[test]
    fn nova_os_block_caret_is_absolute_and_tracks_measured_text_width() {
        // The block caret is ABSOLUTE (so it never advances the row and pushes the
        // ghost a cell right) and is positioned at the MEASURED rendered width of
        // the typed-before text - so it sits ON the first after-cursor /
        // completion-ghost letter regardless of the font's glyph advance (owner
        // playtest: "at the same position as the caret we have the first letter
        // that can be used"). `position_nova_os_block_caret` copies the before-text
        // node's `ComputedNode` width (converted to logical px) onto the caret.
        let mut app = toggle_app();
        init_terminal_input_resources(&mut app);
        app.add_systems(
            Update,
            (handle_terminal_keyboard, rebuild_terminal_ui)
                .chain()
                .run_if(in_state(GameStates::Playing)),
        );
        spawn_nova_os_shell(&mut app);
        press_tab(&mut app);
        press_text(&mut app, "hel");

        let (caret_entity, caret) = app
            .world_mut()
            .query_filtered::<(Entity, &Node), With<NovaOsTerminalCaretMarker>>()
            .single(app.world())
            .map(|(e, n)| (e, n.clone()))
            .expect("one block caret");
        assert_eq!(
            caret.position_type,
            PositionType::Absolute,
            "the caret is absolute so it never advances the row (would push the ghost right)"
        );

        // MinimalPlugins runs no UI layout, so stamp a known rendered width on the
        // before-text node (as the layout pass would) and a 2x scale factor to
        // prove the physical->logical conversion. The caret must copy that width.
        let before_entity = app
            .world_mut()
            .query_filtered::<Entity, With<NovaOsTerminalPromptMarker>>()
            .single(app.world())
            .expect("one before-cursor prompt text");
        // A width deliberately NOT a multiple of the old `chars * 0.6em` cell
        // (16 * 0.6 = 9.6px), so the asserted number itself proves the caret is
        // measure-derived rather than char-derived (review R2.1).
        app.world_mut()
            .entity_mut(before_entity)
            .insert(ComputedNode {
                size: Vec2::new(50.0, 18.0),
                inverse_scale_factor: 0.5,
                ..ComputedNode::DEFAULT
            });

        app.world_mut()
            .run_system_once(position_nova_os_block_caret)
            .expect("caret positioner runs");

        let caret_left = app
            .world()
            .entity(caret_entity)
            .get::<Node>()
            .expect("caret node")
            .left;
        assert_eq!(
            caret_left,
            Val::Px(25.0),
            "the caret lands on the MEASURED typed-text width (50.0 physical * 0.5 \
             scale = 25.0 logical px, not any `chars * 9.6` cell step)"
        );
    }

    #[test]
    fn terminal_log_command_prints_flight_log_rows() {
        let mut log = NovaOsFlightLog::default();
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::Comms,
            objective_id: None,
            speaker: Some("Control".to_string()),
            message: "Hold course.".to_string(),
            icon: None,
        });
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::ObjectivePosted,
            objective_id: Some("burn".to_string()),
            speaker: None,
            message: "Burn for Beacon 1".to_string(),
            icon: None,
        });
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::ObjectiveCompleted,
            objective_id: Some("burn".to_string()),
            speaker: None,
            message: "Burn for Beacon 1".to_string(),
            icon: None,
        });
        let snapshot = terminal_snapshot_from_world(&log, &GameObjectives::default(), None, &[], 0);
        let mut terminal = NovaOsTerminal::default();

        type_text(&mut terminal, "log");
        terminal.submit(&snapshot);

        let printed: Vec<&str> = terminal
            .scrollback()
            .iter()
            .map(|row| row.text.as_str())
            .collect();
        // HTML-style numbered rows, no header.
        assert!(!printed.iter().any(|row| *row == "Flight log:"));
        assert!(printed.contains(&"0001 COMMS CONTROL > Hold course."));
        assert!(printed.contains(&"0002 OBJ + Burn for Beacon 1"));
        assert!(printed.contains(&"0003 OBJ x Burn for Beacon 1"));
    }

    #[test]
    fn terminal_objectives_command_prints_active_objectives() {
        let objectives = GameObjectives {
            objectives: vec![
                Objective::new("beacon", "Recover the beacon"),
                Objective::new("dock", "Dock at the relay"),
            ],
        };
        let snapshot =
            terminal_snapshot_from_world(&NovaOsFlightLog::default(), &objectives, None, &[], 0);
        let mut terminal = NovaOsTerminal::default();

        type_text(&mut terminal, "objectives");
        terminal.submit(&snapshot);

        let printed: Vec<&str> = terminal
            .scrollback()
            .iter()
            .map(|row| row.text.as_str())
            .collect();
        // HTML-style `OBJ + <message>` rows, no header.
        assert!(!printed.iter().any(|row| *row == "Active objectives:"));
        assert!(printed.contains(&"OBJ + Recover the beacon"));
        assert!(printed.contains(&"OBJ + Dock at the relay"));

        let empty_snapshot = terminal_snapshot_from_world(
            &NovaOsFlightLog::default(),
            &GameObjectives::default(),
            None,
            &[],
            0,
        );
        let mut empty = NovaOsTerminal::default();
        type_text(&mut empty, "objectives");
        empty.submit(&empty_snapshot);
        assert_eq!(
            empty.scrollback().last().map(|row| row.text.as_str()),
            Some("No active objectives.")
        );
    }

    #[test]
    fn terminal_objectives_command_reads_live_resource_updates() {
        let mut app = terminal_command_app();
        set_objectives(
            &mut app,
            vec![Objective::new("beacon", "Recover the beacon")],
        );

        submit_terminal_command(&mut app, "objectives");
        assert!(
            terminal_scrollback_texts(&app)
                .iter()
                .any(|row| row == "OBJ + Recover the beacon"),
            "first command submit reads the current objective resource"
        );

        set_objectives(&mut app, vec![Objective::new("dock", "Dock at the relay")]);
        submit_terminal_command(&mut app, "objectives");

        let printed = terminal_scrollback_texts(&app);
        assert!(
            printed.iter().any(|row| row == "OBJ + Dock at the relay"),
            "second command submit reads the changed objective resource"
        );
    }

    #[test]
    fn terminal_ship_command_prints_section_status() {
        let ship_name = Name::new("Rust Tally");
        let sections = vec![
            ShipSectionStatus {
                name: "Port engine".to_string(),
                kind: SectionDamageClass::Thruster,
                health: Some(Health {
                    current: 18.0,
                    max: 100.0,
                }),
                inactive: false,
                zero_health: false,
                ammo: None,
            },
            ShipSectionStatus {
                name: "Bow gun".to_string(),
                kind: SectionDamageClass::Turret,
                health: Some(Health {
                    current: 0.0,
                    max: 60.0,
                }),
                inactive: true,
                zero_health: true,
                ammo: Some(SectionAmmo {
                    rounds: 2,
                    capacity: 6,
                }),
            },
        ];
        let snapshot = terminal_snapshot_from_world(
            &NovaOsFlightLog::default(),
            &GameObjectives::default(),
            Some(ship_name.as_str()),
            &sections,
            0,
        );
        let mut terminal = NovaOsTerminal::default();

        type_text(&mut terminal, "ship");
        terminal.submit(&snapshot);

        let printed: Vec<&str> = terminal
            .scrollback()
            .iter()
            .map(|row| row.text.as_str())
            .collect();
        assert!(printed.contains(&"SHIP RUST TALLY"));
        assert!(printed.contains(&"THRUSTER Port engine - 18/100 HP"));
        assert!(printed.contains(&"  status: critical"));
        assert!(printed.contains(&"TURRET Bow gun - 0/60 HP; ammo 2/6"));
        assert!(printed.contains(&"  status: neutralized"));
    }

    #[test]
    fn terminal_ship_command_reads_live_player_sections() {
        let mut app = terminal_command_app();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Name::new("Rust Tally"),
            ))
            .id();
        let thruster = app
            .world_mut()
            .spawn((
                SectionMarker,
                ThrusterSectionMarker,
                SectionDamageClass::Thruster,
                Health {
                    current: 18.0,
                    max: 100.0,
                },
                ChildOf(ship),
                Name::new("Port engine"),
            ))
            .id();
        app.world_mut().spawn((
            SectionMarker,
            TurretSectionMarker,
            Health {
                current: 0.0,
                max: 60.0,
            },
            SectionInactiveMarker,
            HealthZeroMarker,
            SectionAmmo {
                rounds: 2,
                capacity: 6,
            },
            ChildOf(ship),
            Name::new("Bow gun"),
        ));

        submit_terminal_command(&mut app, "ship");
        let printed = terminal_scrollback_texts(&app);
        assert!(printed.iter().any(|row| row == "SHIP RUST TALLY"));
        assert!(printed
            .iter()
            .any(|row| row == "THRUSTER Port engine - 18/100 HP"));
        assert!(
            printed
                .iter()
                .any(|row| row == "TURRET Bow gun - 0/60 HP; ammo 2/6"),
            "live snapshot classifies turret marker fallback without SectionDamageClass"
        );

        app.world_mut().entity_mut(thruster).insert(Health {
            current: 80.0,
            max: 100.0,
        });
        submit_terminal_command(&mut app, "ship");
        assert!(
            terminal_scrollback_texts(&app)
                .iter()
                .any(|row| row == "THRUSTER Port engine - 80/100 HP"),
            "second command submit reads changed live section health"
        );
    }

    #[test]
    fn terminal_ui_renders_prompt_hint_and_invalid_coloring() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaOsTerminal>();
        spawn_nova_os_shell(&mut app);
        {
            let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
            terminal.insert_text("hlep");
        }

        app.world_mut()
            .run_system_once(rebuild_terminal_ui)
            .expect("terminal UI rebuild runs");

        let (prompt, prompt_color, prompt_node) = app
            .world_mut()
            .query_filtered::<(&Text, &TextColor, &Node), With<NovaOsTerminalPromptMarker>>()
            .single(app.world())
            .expect("one terminal prompt");
        assert_eq!(prompt.0, "hlep");
        assert_eq!(prompt_color.0, theme::semantic::THREAT);
        assert_eq!(
            prompt_node.flex_shrink, 0.0,
            "typed input must not collapse inside the prompt row"
        );
        // The terminal text carries no per-glyph shadow (crisp phosphor).
        assert!(
            app.world_mut()
                .query_filtered::<&TextShadow, With<NovaOsTerminalPromptMarker>>()
                .iter(app.world())
                .next()
                .is_none(),
            "terminal prompt text has no shadow/bloom glyph"
        );

        let (ghost, ghost_node) = app
            .world_mut()
            .query_filtered::<(&Text, &Node), With<NovaOsTerminalGhostMarker>>()
            .single(app.world())
            .expect("one terminal autocomplete ghost");
        assert_eq!(ghost.0, "");
        assert_eq!(
            ghost_node.flex_shrink, 0.0,
            "autocomplete ghost must not collapse the typed input"
        );
        assert_eq!(
            ghost_node.position_type,
            PositionType::Relative,
            "autocomplete ghost stays inline after the visible prompt text"
        );

        let (prefix, prefix_color) = app
            .world_mut()
            .query_filtered::<(&Text, &TextColor), With<NovaOsPromptPrefixMarker>>()
            .single(app.world())
            .expect("one terminal prompt prefix");
        assert_eq!(prefix.0, "nova>");
        assert_eq!(prefix_color.0, NOVA_OS_AMBER);

        let (hint, hint_color, hint_node) = app
            .world_mut()
            .query_filtered::<(&Text, &TextColor, &Node), With<NovaOsTerminalHintMarker>>()
            .single(app.world())
            .expect("one terminal hint");
        assert_eq!(hint.0, "did you mean help?");
        assert_eq!(hint_color.0, theme::semantic::THREAT);
        assert_eq!(
            hint_node.width,
            Val::Percent(100.0),
            "invalid-command suggestions live below the input line instead of stealing prompt width"
        );

        let input_wrap = app
            .world_mut()
            .query_filtered::<&Node, With<NovaOsPromptInputWrapMarker>>()
            .single(app.world())
            .expect("one prompt input wrap");
        assert_eq!(input_wrap.flex_grow, 1.0);
        assert_eq!(
            input_wrap.overflow,
            Overflow::clip_x(),
            "typed input owns the prompt lane and clips inside it"
        );
    }

    /// One right-stick-click press: press + update (toggle sets NextState), then
    /// release + clear + update (applies the transition; the clear stops the
    /// stale edge re-firing next frame - same shape as `press_tab`).
    fn press_pad(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<GamepadButton>>()
            .press(GamepadButton::RightThumb);
        app.update();
        let mut pad = app.world_mut().resource_mut::<ButtonInput<GamepadButton>>();
        pad.release(GamepadButton::RightThumb);
        pad.clear();
        app.update();
    }

    /// The gamepad right-stick click (`RightThumb`) opens the NOVA OS too (task
    /// 20260724-134312). Narrowing the pad button away fails this.
    #[test]
    fn pad_opens_nova_os_and_requests_animated_close() {
        let mut app = toggle_app();
        app.init_resource::<ButtonInput<GamepadButton>>();
        assert_eq!(pause_state(&app), PauseStates::Unpaused);

        press_pad(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::NovaOs,
            "the right-stick click opens the NOVA OS"
        );
        press_pad(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::NovaOs,
            "the right-stick click keeps gameplay paused while close animation runs"
        );
        assert!(
            app.world().resource::<NovaOsCloseTransition>().closing,
            "the right-stick click requests the animated nova_os close"
        );
    }

    #[test]
    fn tab_is_inert_while_the_pause_menu_owns_the_freeze() {
        let mut app = toggle_app();
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update();
        press_tab(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::Paused,
            "Tab does nothing while the pause menu is up"
        );
    }

    // (The tab-handle anchor test moved to `objective_hint` -
    // `objective_hint_provides_the_nova_os_anchor` - now that the hint is the
    // reveal's tuck-anchor source, task 20260724-134312.)

    fn objectives_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GameObjectives>();
        app.init_resource::<StoryFeed>();
        app.init_resource::<NovaOsFlightLog>();
        app.add_systems(
            Update,
            (
                sync_nova_os_logs,
                rebuild_nova_os_objectives,
                rebuild_nova_os_flight_log,
            )
                .chain()
                .run_if(
                    resource_changed::<GameObjectives>
                        .or_else(resource_changed::<StoryFeed>)
                        .or_else(nova_os_lists_just_spawned),
                ),
        );
        app
    }

    fn spawn_objectives_list(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                NovaOsObjectivesListMarker,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    ..default()
                },
            ))
            .id()
    }

    fn spawn_flight_log_list(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                NovaOsFlightLogListMarker,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    ..default()
                },
            ))
            .id()
    }

    fn spawn_nova_os_shell(app: &mut App) {
        app.add_observer(setup_nova_os);
        app.world_mut().spawn((
            Name::new("Survey Cutter"),
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
        ));
        app.update();
    }

    fn spawn_nova_os_shell_with_crt(app: &mut App) {
        app.init_asset::<NovaOsCrtMaterial>();
        app.init_asset::<Font>();
        // The chin's brand plate loads a logo image, so the render-capable rig
        // must register the `Image` asset too (production has it via DefaultPlugins).
        app.init_asset::<Image>();
        spawn_nova_os_shell(app);
    }

    fn assert_scrollable_viewport(app: &App, viewport: Entity, label: &str) {
        let node = app.world().entity(viewport).get::<Node>().expect(label);
        assert_eq!(
            node.overflow,
            Overflow::scroll_y(),
            "{label} clips overflowing rows on the y axis"
        );
        assert_eq!(
            node.flex_grow, 1.0,
            "{label} consumes the panel's remaining height instead of growing past it"
        );
        assert!(
            app.world().entity(viewport).contains::<ScrollPosition>(),
            "{label} carries ScrollPosition so wheel input can move it"
        );
    }

    #[test]
    fn nova_os_terminal_scrollback_lives_in_scrollable_viewport() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        spawn_nova_os_shell(&mut app);

        let list = app
            .world_mut()
            .query_filtered::<Entity, With<NovaOsTerminalScrollbackMarker>>()
            .single(app.world())
            .expect("terminal scrollback viewport");

        assert_scrollable_viewport(&app, list, "terminal scrollback viewport");
    }

    #[test]
    fn nova_os_keeps_log_objective_state_without_visible_panes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        spawn_nova_os_shell(&mut app);

        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<NovaOsFlightLogListMarker>>()
                .iter(app.world())
                .count(),
            0,
            "the PoC-style monitor does not show a permanent Flight Log pane"
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<NovaOsObjectivesListMarker>>()
                .iter(app.world())
                .count(),
            0,
            "the PoC-style monitor does not show a permanent Objectives pane"
        );

        let mut data_app = objectives_app();
        spawn_flight_log_list(&mut data_app);
        spawn_objectives_list(&mut data_app);
        set_objectives(
            &mut data_app,
            vec![Objective::new("b1", "Strip 3 salvage crates")],
        );
        push_story_line(&mut data_app, "Okono", "Telemetry is dirty but readable.");
        data_app.update();

        assert_eq!(
            flight_log_texts(&mut data_app),
            vec![
                "COMMS OKONO > Telemetry is dirty but readable.".to_string(),
                "OBJ + Strip 3 salvage crates".to_string(),
            ],
            "backing log/objective logic still derives rows for terminal commands"
        );
    }

    #[test]
    fn nova_os_wheel_scrolls_viewports_and_clamps_at_top() {
        use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

        let scroll_after = |start_y: f32, wheel_y: f32| -> f32 {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.world_mut().init_resource::<Messages<MouseWheel>>();
            app.world_mut().spawn((
                NovaOsScrollViewportMarker,
                ScrollPosition(Vec2::new(0.0, start_y)),
            ));
            app.world_mut().write_message(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: wheel_y,
                window: Entity::PLACEHOLDER,
                phase: TouchPhase::Moved,
            });
            app.world_mut()
                .run_system_once(scroll_nova_os_panels)
                .expect("nova_os scroll system runs");
            app.world_mut()
                .query::<&ScrollPosition>()
                .single(app.world())
                .expect("one scroll position")
                .0
                .y
        };

        assert!(
            scroll_after(0.0, -1.0) > 0.0,
            "wheel down from the top scrolls the NOVA OS panel down"
        );
        assert_eq!(
            scroll_after(12.0, 1.0),
            0.0,
            "wheel up clamps at the top instead of going negative"
        );
    }

    #[test]
    fn nova_os_wheel_scroll_clamps_at_content_bottom() {
        use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().init_resource::<Messages<MouseWheel>>();
        let viewport = app
            .world_mut()
            .spawn((
                NovaOsScrollViewportMarker,
                ScrollPosition(Vec2::new(0.0, 95.0)),
                ComputedNode {
                    size: Vec2::new(100.0, 100.0),
                    content_size: Vec2::new(100.0, 200.0),
                    scrollbar_size: Vec2::ZERO,
                    ..default()
                },
            ))
            .id();

        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });
        app.world_mut()
            .run_system_once(scroll_nova_os_panels)
            .expect("nova_os scroll system runs");

        assert_eq!(
            app.world()
                .entity(viewport)
                .get::<ScrollPosition>()
                .unwrap()
                .0
                .y,
            100.0,
            "stored nova_os scroll offset clamps to the content bottom"
        );
    }

    #[test]
    fn nova_os_wheel_scrolls_only_hovered_viewport_when_one_is_hovered() {
        use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().init_resource::<Messages<MouseWheel>>();
        let hovered = app
            .world_mut()
            .spawn((
                NovaOsScrollViewportMarker,
                Hovered(true),
                ScrollPosition(Vec2::ZERO),
            ))
            .id();
        let not_hovered = app
            .world_mut()
            .spawn((
                NovaOsScrollViewportMarker,
                Hovered(false),
                ScrollPosition(Vec2::ZERO),
            ))
            .id();

        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });
        app.world_mut()
            .run_system_once(scroll_nova_os_panels)
            .expect("nova_os scroll system runs");

        let hovered_y = app
            .world()
            .entity(hovered)
            .get::<ScrollPosition>()
            .unwrap()
            .0
            .y;
        let not_hovered_y = app
            .world()
            .entity(not_hovered)
            .get::<ScrollPosition>()
            .unwrap()
            .0
            .y;
        assert!(
            hovered_y > 0.0,
            "the hovered viewport receives the wheel scroll"
        );
        assert_eq!(
            not_hovered_y, 0.0,
            "a non-hovered viewport does not scroll when another nova_os viewport is hovered"
        );
    }

    fn set_objectives(app: &mut App, objectives: Vec<Objective>) {
        app.world_mut().resource_mut::<GameObjectives>().objectives = objectives;
    }

    fn push_story_line(app: &mut App, speaker: &str, text: &str) {
        app.world_mut()
            .resource_mut::<StoryFeed>()
            .0
            .push(StoryLine {
                speaker: speaker.to_string(),
                text: text.to_string(),
                dwell: None,
                icon: None,
            });
    }

    fn row_entities(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<NovaOsObjectiveRowMarker>>()
            .iter(app.world())
            .collect()
    }

    fn row_text(app: &App, row: Entity) -> String {
        let mut text = None;
        for child in app
            .world()
            .entity(row)
            .get::<Children>()
            .expect("row children")
        {
            if let Some(found) = text_in_tree(app, *child) {
                text = Some(found);
                break;
            }
        }
        text.expect("row has objective text")
    }

    fn text_in_tree(app: &App, entity: Entity) -> Option<String> {
        let entity_ref = app.world().entity(entity);
        if entity_ref.contains::<NovaOsObjectiveTextMarker>() {
            return entity_ref.get::<Text>().map(|text| text.0.clone());
        }
        entity_ref
            .get::<Children>()
            .and_then(|children| children.iter().find_map(|child| text_in_tree(app, child)))
    }

    fn all_texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect()
    }

    fn flight_log_texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query_filtered::<&Text, With<NovaOsFlightLogTextMarker>>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect()
    }

    #[test]
    fn nova_os_objectives_section_uses_styled_rows() {
        let mut app = objectives_app();
        set_objectives(
            &mut app,
            vec![
                Objective::new("b1", "Burn for Beacon 1"),
                Objective::new("b2", "Dock at the relay"),
            ],
        );
        let list = spawn_objectives_list(&mut app);
        app.update();

        let rows = row_entities(&mut app);
        let direct_text_children = app
            .world()
            .entity(list)
            .get::<Children>()
            .expect("list children")
            .iter()
            .filter(|child| app.world().entity(*child).contains::<Text>())
            .count();
        assert_eq!(
            direct_text_children, 0,
            "objectives render as row nodes, not direct bare Text children"
        );
        let row_ids: Vec<String> = rows
            .iter()
            .map(|&row| {
                app.world()
                    .entity(row)
                    .get::<NovaOsObjectiveId>()
                    .expect("row id")
                    .0
                    .clone()
            })
            .collect();
        assert_eq!(
            row_ids,
            vec!["b1".to_string(), "b2".to_string()],
            "the objectives section renders one styled row per active objective"
        );
        for &row in &rows {
            assert_eq!(
                *app.world()
                    .entity(row)
                    .get::<NovaOsObjectiveRowStatus>()
                    .expect("row status"),
                NovaOsObjectiveRowStatus::Active
            );
            assert!(
                app.world().entity(row).get::<BackgroundColor>().is_some(),
                "styled rows carry a fill"
            );
            assert!(
                app.world().entity(row).get::<BorderColor>().is_some(),
                "styled rows carry a border"
            );
            let has_glyph = app
                .world()
                .entity(row)
                .get::<Children>()
                .expect("row children")
                .iter()
                .any(|child| {
                    app.world()
                        .entity(child)
                        .contains::<NovaOsObjectiveGlyphMarker>()
                });
            assert!(has_glyph, "styled rows carry a status glyph");
        }
        assert_eq!(row_text(&app, rows[0]), "Burn for Beacon 1");
    }

    #[test]
    fn nova_os_monitor_has_combined_flight_log_stream() {
        let mut app = objectives_app();
        let list = spawn_flight_log_list(&mut app);
        app.update();

        assert!(
            app.world().entity(list).get::<Children>().is_some(),
            "the monitor owns one stream container with an empty row"
        );
        let empty = app
            .world_mut()
            .query_filtered::<Entity, With<NovaOsFlightLogEmptyMarker>>()
            .single(app.world())
            .expect("combined log empty state");
        assert!(
            app.world().entity(empty).get::<BackgroundColor>().is_some(),
            "combined log empty state carries nova_os chrome fill"
        );
    }

    #[test]
    fn nova_os_combined_log_renders_story_feed_rows() {
        let mut app = objectives_app();
        spawn_flight_log_list(&mut app);
        app.update();

        push_story_line(&mut app, "Okono", "Strip it clean.");
        app.update();

        assert_eq!(
            flight_log_texts(&mut app),
            vec!["COMMS OKONO > Strip it clean.".to_string()],
            "story feed lines append as comms rows in the combined stream"
        );
        let icon = app
            .world_mut()
            .query_filtered::<&NovaOsFlightLogIconMarker, With<NovaOsFlightLogRowMarker>>()
            .single(app.world())
            .expect("comms row has an icon marker");
        assert_eq!(icon.kind, NovaOsFlightLogIconKind::Fallback);
    }

    #[test]
    fn nova_os_combined_log_records_objective_events_once() {
        let mut app = objectives_app();
        spawn_flight_log_list(&mut app);
        app.update();

        set_objectives(&mut app, vec![Objective::new("b1", "Burn for Beacon 1")]);
        app.update();
        set_objectives(&mut app, vec![Objective::new("b1", "Recovered: 1/3")]);
        app.update();
        set_objectives(&mut app, Vec::new());
        app.update();

        assert_eq!(
            flight_log_texts(&mut app),
            vec![
                "OBJ + Recovered: 1/3".to_string(),
                "OBJ x Recovered: 1/3".to_string(),
            ],
            "an objective text update edits the posted row rather than appending a duplicate"
        );
    }

    #[test]
    fn nova_os_combined_log_interleaves_comms_and_objective_rows() {
        let mut app = objectives_app();
        spawn_flight_log_list(&mut app);
        app.update();

        push_story_line(&mut app, "Okono", "First transmission.");
        app.update();
        set_objectives(&mut app, vec![Objective::new("b1", "Burn for Beacon 1")]);
        app.update();
        push_story_line(&mut app, "Relay", "Telemetry locked.");
        app.update();
        set_objectives(&mut app, Vec::new());
        app.update();

        assert_eq!(
            flight_log_texts(&mut app),
            vec![
                "COMMS OKONO > First transmission.".to_string(),
                "OBJ + Burn for Beacon 1".to_string(),
                "COMMS RELAY > Telemetry locked.".to_string(),
                "OBJ x Burn for Beacon 1".to_string(),
            ],
            "comms and objective rows share one chronological stream"
        );
    }

    #[test]
    fn nova_os_monitor_shows_only_active_objectives() {
        let mut app = objectives_app();
        set_objectives(
            &mut app,
            vec![
                Objective::new("b1", "Burn for Beacon 1"),
                Objective::new("b2", "Dock at the relay"),
            ],
        );
        spawn_objectives_list(&mut app);
        app.update();

        set_objectives(&mut app, vec![Objective::new("b2", "Dock at the relay")]);
        app.update();

        let rows = row_entities(&mut app);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            *app.world()
                .entity(rows[0])
                .get::<NovaOsObjectiveRowStatus>()
                .expect("row status"),
            NovaOsObjectiveRowStatus::Active
        );
        assert_eq!(row_text(&app, rows[0]), "Dock at the relay");
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsObjectiveStrikeMarker>>()
                .iter(app.world())
                .next()
                .is_none(),
            "completed objectives are not duplicated as struck-through right-panel rows"
        );
    }

    #[test]
    fn nova_os_final_objective_moves_to_flight_log_only() {
        let mut app = objectives_app();
        set_objectives(&mut app, vec![Objective::new("b1", "Burn for Beacon 1")]);
        spawn_objectives_list(&mut app);
        spawn_flight_log_list(&mut app);
        app.update();

        set_objectives(&mut app, Vec::new());
        app.update();

        assert!(row_entities(&mut app).is_empty());
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<NovaOsObjectiveEmptyMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "the monitor returns to its no-active-objectives empty state"
        );
        assert_eq!(
            flight_log_texts(&mut app),
            vec![
                "OBJ + Burn for Beacon 1".to_string(),
                "OBJ x Burn for Beacon 1".to_string(),
            ],
            "the completed objective remains only in the left Flight Log"
        );
    }

    #[test]
    fn terminal_commands_clear_on_nova_os_teardown() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaOsFlightLog>();
        app.init_resource::<NovaOsTerminal>();
        app.add_observer(setup_nova_os);
        app.add_observer(remove_nova_os);

        let player = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        app.update();
        {
            let mut log = app.world_mut().resource_mut::<NovaOsFlightLog>();
            log.entries.push(NovaOsFlightLogEntry {
                kind: NovaOsFlightLogEntryKind::ObjectiveCompleted,
                objective_id: Some("b1".to_string()),
                speaker: None,
                message: "Burn for Beacon 1".to_string(),
                icon: None,
            });
            log.previous_active = vec![Objective::new("b2", "Dock at the relay")];
            log.seen_story = 1;
        }
        {
            let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
            type_text(&mut terminal, "log");
            terminal.submit(&TerminalCommandSnapshot {
                log_rows: vec![TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "OBJ x Burn for Beacon 1".to_string(),
                }],
                objective_rows: Vec::new(),
                ship_rows: Vec::new(),
                unread_events: 0,
                unread_hook: None,
            });
            assert!(
                terminal
                    .scrollback()
                    .iter()
                    .any(|row| row.text.contains("Burn for Beacon 1")),
                "delivery guard: terminal contains scenario output before teardown"
            );
        }

        app.world_mut()
            .entity_mut(player)
            .remove::<PlayerSpaceshipMarker>();
        app.update();

        let log = app.world().resource::<NovaOsFlightLog>();
        assert!(
            log.entries.is_empty() && log.previous_active.is_empty() && log.seen_story == 0,
            "nova_os teardown clears the retained left-panel log"
        );
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().scrollback(),
            nova_os_welcome_rows(),
            "nova_os teardown clears printed command output before the next player ship"
        );
    }

    #[test]
    fn nova_os_objectives_empty_state_is_styled() {
        let mut app = objectives_app();
        spawn_objectives_list(&mut app);
        app.update();

        let empty = app
            .world_mut()
            .query_filtered::<Entity, With<NovaOsObjectiveEmptyMarker>>()
            .single(app.world())
            .expect("styled empty row");
        assert!(
            app.world().entity(empty).get::<BackgroundColor>().is_some(),
            "empty state carries nova_os chrome fill"
        );
        assert!(
            app.world().entity(empty).get::<BorderColor>().is_some(),
            "empty state carries nova_os chrome border"
        );
    }

    #[test]
    fn nova_os_objectives_rebuild_replaces_stale_rows() {
        let mut app = objectives_app();
        set_objectives(&mut app, vec![Objective::new("b1", "Burn")]);
        spawn_objectives_list(&mut app);
        app.update();
        let first_rows = row_entities(&mut app);
        assert_eq!(first_rows.len(), 1);

        set_objectives(&mut app, vec![Objective::new("b1", "Recovered: 1/3")]);
        app.update();

        let rows = row_entities(&mut app);
        assert_eq!(rows.len(), 1, "old row entity was replaced");
        assert_ne!(rows[0], first_rows[0], "rebuild despawns stale rows");
        assert_eq!(row_text(&app, rows[0]), "Recovered: 1/3");
    }

    /// The open NOVA OS is a modal: its monitor and backdrop must carry an explicit
    /// `GlobalZIndex` above the HUD chrome (which carries none = 0), or the
    /// top-right objectives panel and other flight HUD draw over it. Mirrors
    /// nova_menu's overlay-z assertion. Fails before the fix (no `GlobalZIndex`).
    #[test]
    fn nova_os_renders_above_the_hud() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(setup_nova_os);
        // setup_nova_os fires on the player ship's PlayerSpaceshipMarker add.
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.update();

        let backdrop_z = app
            .world_mut()
            .query_filtered::<&GlobalZIndex, With<NovaOsBackdropMarker>>()
            .single(app.world())
            .expect("the NOVA OS backdrop carries an explicit GlobalZIndex")
            .0;
        assert!(
            backdrop_z > 0,
            "the backdrop must stack above the HUD chrome (z = {backdrop_z})"
        );
        let monitor_zs: Vec<i32> = app
            .world_mut()
            .query_filtered::<&GlobalZIndex, With<NovaOsMonitorMarker>>()
            .iter(app.world())
            .map(|z| z.0)
            .collect();
        assert_eq!(
            monitor_zs.len(),
            1,
            "the shell spawns one NOVA OS monitor, not left/right panels"
        );
        assert!(
            monitor_zs[0] >= backdrop_z,
            "the monitor sits at or above the backdrop (monitor {}, backdrop {backdrop_z})",
            monitor_zs[0]
        );
        // Diagnostic NOVA OS-exempt chrome must out-rank the backdrop so the
        // deepened gray field cannot dim it.
        assert!(
            DRAWER_EXEMPT_Z > backdrop_z,
            "exempt chrome z ({DRAWER_EXEMPT_Z}) must beat the backdrop ({backdrop_z})"
        );
    }

    /// The shell builds one inset physical monitor with the CRT layers the
    /// follow-up terminal tasks can fill, not two permanent side panels.
    #[test]
    fn nova_os_spawns_single_nova_os_monitor() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(setup_nova_os);
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.update();

        let monitors: Vec<Node> = app
            .world_mut()
            .query_filtered::<&Node, With<NovaOsMonitorMarker>>()
            .iter(app.world())
            .cloned()
            .collect();
        assert_eq!(monitors.len(), 1);
        let monitor = &monitors[0];
        assert_eq!(monitor.position_type, PositionType::Absolute);
        assert_eq!(monitor.top, Val::Px(NOVA_OS_MONITOR_INSET_Y_PX));
        assert_eq!(monitor.bottom, Val::Px(NOVA_OS_MONITOR_INSET_Y_PX));
        assert_eq!(monitor.left, Val::Px(NOVA_OS_MONITOR_INSET_X_PX));
        assert_eq!(monitor.right, Val::Px(NOVA_OS_MONITOR_INSET_X_PX));
        let extra_roots = app
            .world_mut()
            .query_filtered::<(), (With<NovaOsRootMarker>, Without<NovaOsMonitorMarker>)>()
            .iter(app.world())
            .count();
        assert_eq!(extra_roots, 0, "there are no leftover side-panel roots");
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsBezelMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "monitor has a physical bezel"
        );
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsScreenMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "monitor has an inset phosphor screen"
        );
        // The CRT treatment is now the render-to-texture sampling shader, not the
        // old overlay nodes (task 20260726-193233). Headless (no image/material
        // assets) this rig falls back to the terminal directly on the screen with
        // no sampling surface; the sampling surface is asserted by
        // `nova_os_screen_samples_offscreen_image` under the with-CRT harness.
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsTerminalContentMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "headless fallback renders the terminal directly on the screen"
        );
    }

    /// The casing + glass depth pass (task 20260726-193219) gives the monitor its
    /// physical details: rounded casing/bezel/screen, the moulding seam, four
    /// corner screws, the vent strip, and the chin bar carrying the brand plate
    /// and a reserved (empty) controls slot.
    #[test]
    fn nova_os_monitor_has_physical_casing_details() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(setup_nova_os);
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.update();

        // Rounded casing stack: asymmetric shell corners, rounded bezel + screen.
        let monitor = app
            .world_mut()
            .query_filtered::<&Node, With<NovaOsMonitorMarker>>()
            .single(app.world())
            .expect("one monitor")
            .clone();
        assert_eq!(
            monitor.border_radius.top_left,
            Val::Px(NOVA_OS_CASE_RADIUS_TOP_PX),
            "casing has the larger top corner radius"
        );
        assert_eq!(
            monitor.border_radius.bottom_left,
            Val::Px(NOVA_OS_CASE_RADIUS_BOTTOM_PX),
            "casing has the tighter bottom corner radius"
        );
        let bezel = app
            .world_mut()
            .query_filtered::<&Node, With<NovaOsBezelMarker>>()
            .single(app.world())
            .expect("one bezel")
            .clone();
        assert_eq!(
            bezel.border_radius.top_left,
            Val::Px(NOVA_OS_BEZEL_RADIUS_PX)
        );
        let screen = app
            .world_mut()
            .query_filtered::<&Node, With<NovaOsScreenMarker>>()
            .single(app.world())
            .expect("one screen")
            .clone();
        assert_eq!(
            screen.border_radius.top_left,
            Val::Px(NOVA_OS_SCREEN_RADIUS_PX)
        );

        // The screen edge is a dark recess line, NOT the old flat bright-phosphor
        // frame: the crisp glowing edge now comes from the shader's barrel-bowed
        // rim (feedback item 2 / DECISION.md). This pins the demotion so the flat
        // straight frame cannot silently return.
        let screen_border = app
            .world_mut()
            .query_filtered::<&BorderColor, With<NovaOsScreenMarker>>()
            .single(app.world())
            .expect("one screen")
            .clone();
        assert_eq!(
            screen_border.top,
            NOVA_OS_CASE_EDGE.with_alpha(0.85),
            "the screen edge is a dark recess line"
        );
        assert_ne!(
            screen_border.top,
            NOVA_OS_PHOSPHOR.with_alpha(0.52),
            "the flat bright-phosphor screen frame is gone"
        );

        // Four moulded corner screws.
        assert_eq!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsScrewMarker>>()
                .iter(app.world())
                .count(),
            4,
            "four corner screws"
        );

        // Single-instance detail nodes: vent strip, moulding seam, chin, plate,
        // and the reserved controls row.
        for (count, expected, label) in [
            (
                app.world_mut()
                    .query_filtered::<(), With<NovaOsVentMarker>>()
                    .iter(app.world())
                    .count(),
                1,
                "vent strip",
            ),
            (
                app.world_mut()
                    .query_filtered::<(), With<NovaOsSeamMarker>>()
                    .iter(app.world())
                    .count(),
                1,
                "moulding seam",
            ),
            (
                app.world_mut()
                    .query_filtered::<(), With<NovaOsChinMarker>>()
                    .iter(app.world())
                    .count(),
                1,
                "chin bar",
            ),
            (
                app.world_mut()
                    .query_filtered::<(), With<NovaOsBrandPlateMarker>>()
                    .iter(app.world())
                    .count(),
                1,
                "brand plate",
            ),
            (
                app.world_mut()
                    .query_filtered::<(), With<NovaOsControlsRowMarker>>()
                    .iter(app.world())
                    .count(),
                1,
                "reserved controls row",
            ),
        ] {
            assert_eq!(count, expected, "monitor has exactly one {label}");
        }

        // The brand plate carries the stamped wordmark + spec line.
        let plate_texts: Vec<String> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect();
        assert!(
            plate_texts.iter().any(|t| t.contains("NOVACRT 9000")),
            "brand plate shows the NovaCRT 9000 wordmark"
        );
        assert!(
            plate_texts.iter().any(|t| t.contains("P22 GREEN PHOSPHOR")),
            "brand plate shows the phosphor spec line"
        );

        // The phosphor rim (glow + line) and the glass sheen trace the screen.
        assert_eq!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsPhosphorRimMarker>>()
                .iter(app.world())
                .count(),
            2,
            "phosphor rim has a glow + line pair"
        );
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsGlassMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "screen has a glass sheen layer"
        );
    }

    #[test]
    fn nova_os_matches_nova_os_terminal_poc_structure() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        spawn_nova_os_shell(&mut app);

        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsTopbarMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "screen has the PoC topbar"
        );
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsLampMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "topbar has the lit status lamp"
        );
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsStatusMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "topbar has the right-side status text"
        );
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsTerminalSurfaceMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "screen has one terminal surface"
        );
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsPromptRowMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "terminal surface has the PoC prompt row"
        );
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsPromptInputLineMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "prompt strip has a dedicated input line"
        );
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsPromptInputWrapMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "prompt strip has a full-width input wrap like the HTML PoC"
        );
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsFooterHintsMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "screen has the PoC footer hint row"
        );

        let texts = all_texts(&mut app);
        for expected in [
            format!("NOVA OS {} / COCKPIT LINK", nova_os_version_label()),
            // The topbar carries the ship/link head plus a live FPS segment; it
            // spawns with a fixed-width `--` placeholder before the diagnostic has
            // a reading (task 20260727-014806; fixed width 20260727-135213).
            "SHIP: SURVEY CUTTER     LINK: LOCAL     FPS:  --".to_string(),
            format!("NOVA OS {}", nova_os_version_label()),
            "POST ......... flight computer / ok".to_string(),
            "CORE ......... 64K static / ok".to_string(),
            "DISPLAY ...... green phosphor crt / warm".to_string(),
            "LINK ......... cockpit bus / local".to_string(),
            "Hint: type `help` and press Enter.".to_string(),
            "nova>".to_string(),
            // The footer lists the full current keybind set (task 20260727-135213).
            "TAB: COMPLETE".to_string(),
            "UP/DN: HISTORY".to_string(),
            "PGUP/PGDN: SCROLL".to_string(),
            "ESC: CLOSE".to_string(),
        ] {
            assert!(
                texts.iter().any(|text| text == &expected),
                "missing PoC text: {expected}"
            );
        }
        assert!(
            !texts.iter().any(|text| text.contains("DRAWER PAUSED")),
            "the topbar should not repeat a useless paused label"
        );
        assert!(
            !texts
                .iter()
                .any(|text| text == "FLIGHT LOG" || text == "OBJECTIVES"),
            "NOVA OS no longer renders permanent side-panel headings inside the screen"
        );
    }

    #[test]
    fn topbar_status_line_carries_a_live_fps_segment() {
        // The pure line builder appends the FPS segment after the ship/link head,
        // with a fixed-width `--` placeholder until the diagnostic reads. The FPS
        // is right-aligned to 3 chars so the topbar never reflows as the reading
        // changes digit count (owner playtest: 100 -> 99 must not shift).
        assert_eq!(
            nova_os_status_text("CERES QUEEN", Some(60)),
            "SHIP: CERES QUEEN     LINK: LOCAL     FPS:  60"
        );
        assert_eq!(
            nova_os_status_text("CERES QUEEN", None),
            "SHIP: CERES QUEEN     LINK: LOCAL     FPS:  --"
        );

        // Fixed width: the FPS segment is the SAME length across digit counts, so
        // 100 -> 99 does not change the rendered width (the reported bug).
        assert_eq!(nova_os_fps_segment(Some(100)).len(), 3);
        assert_eq!(nova_os_fps_segment(Some(99)).len(), 3);
        assert_eq!(nova_os_fps_segment(Some(9)).len(), 3);
        assert_eq!(nova_os_fps_segment(None).len(), 3);
        assert_eq!(nova_os_fps_segment(Some(99)), " 99");
        assert_eq!(nova_os_fps_segment(Some(100)), "100");

        // The live rewrite replaces only the FPS tail, preserving the head.
        let spawned = nova_os_status_text("CERES QUEEN", None);
        assert_eq!(
            topbar_line_with_fps(&spawned, Some(144)),
            "SHIP: CERES QUEEN     LINK: LOCAL     FPS: 144"
        );
        assert_eq!(
            topbar_line_with_fps("SHIP: CERES QUEEN     LINK: LOCAL     FPS: 144", None),
            "SHIP: CERES QUEEN     LINK: LOCAL     FPS:  --"
        );
        // A line missing the marker (older spawn) still gets an FPS segment.
        assert_eq!(
            topbar_line_with_fps("SHIP: CERES QUEEN     LINK: LOCAL", Some(30)),
            "SHIP: CERES QUEEN     LINK: LOCAL     FPS:  30"
        );
    }

    #[test]
    fn drive_topbar_fps_writes_the_smoothed_reading_onto_the_status_line() {
        use bevy::diagnostic::{
            Diagnostic, DiagnosticMeasurement, DiagnosticsStore, FrameTimeDiagnosticsPlugin,
        };

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        spawn_nova_os_shell(&mut app);

        // Seed a DiagnosticsStore with an FPS reading, mirroring what
        // FrameTimeDiagnosticsPlugin publishes in production.
        let mut store = DiagnosticsStore::default();
        let mut fps = Diagnostic::new(FrameTimeDiagnosticsPlugin::FPS);
        fps.add_measurement(DiagnosticMeasurement {
            time: std::time::Instant::now(),
            value: 59.6,
        });
        store.add(fps);
        app.insert_resource(store);

        app.world_mut()
            .run_system_once(drive_nova_os_topbar_fps)
            .unwrap();

        let texts = all_texts(&mut app);
        assert!(
            texts
                .iter()
                .any(|text| text == "SHIP: SURVEY CUTTER     LINK: LOCAL     FPS:  60"),
            "the topbar shows the rounded smoothed FPS (fixed 3-wide) while the NOVA OS is open; got {texts:?}"
        );
    }

    #[test]
    fn nova_os_screen_samples_offscreen_image() {
        // Render-capable: the screen node hosts ONE sampling surface bound to the
        // offscreen image, the terminal content lives in the image-camera content
        // root, and the old overlay path is gone (task 20260726-193233).
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        spawn_nova_os_shell_with_crt(&mut app);

        let surfaces = app
            .world_mut()
            .query_filtered::<&MaterialNode<NovaOsCrtMaterial>, With<NovaOsSamplingSurfaceMarker>>()
            .iter(app.world())
            .count();
        assert_eq!(
            surfaces, 1,
            "the screen has one shader-backed sampling surface in render-capable apps"
        );

        let (rtt_content_root, rtt_image) = {
            let rtt = app
                .world()
                .get_resource::<NovaOsRtt>()
                .expect("render-capable build inserts the NovaOsRtt pipeline");
            (rtt.content_root, rtt.image.clone())
        };

        // The terminal content renders through the image camera, i.e. under the
        // content root, not directly under the screen node.
        let content_parent = app
            .world_mut()
            .query_filtered::<&ChildOf, With<NovaOsTerminalContentMarker>>()
            .single(app.world())
            .expect("terminal content exists")
            .parent();
        assert_eq!(
            content_parent, rtt_content_root,
            "terminal content is routed to the offscreen content root"
        );

        // The sampling material binds the offscreen image (not the default handle).
        let material = app
            .world()
            .resource::<Assets<NovaOsCrtMaterial>>()
            .iter()
            .next()
            .expect("one CRT material")
            .1;
        assert_eq!(
            material.source, rtt_image,
            "the sampling material binds the offscreen image target"
        );
        assert_eq!(
            material.data.vignette_strength, NOVA_OS_CRT_VIGNETTE_STRENGTH,
            "CRT material carries the near-black corner pass"
        );
    }

    #[test]
    fn nova_os_crt_material_receives_resolution_time_and_power() {
        // `animate_nova_os_crt` feeds the sampling surface's ComputedNode size into
        // the material's `resolution` uniform (resolution-aware scanlines + bloom
        // taps), stamps `time`, and pipes NovaOsOpenness in as the `power` level
        // (the raster power-on/off collapse).
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<NovaOsCrtMaterial>();
        app.init_resource::<Time<Real>>();
        app.init_resource::<NovaOsMonitorSettings>();
        app.init_resource::<NovaOsDegauss>();
        app.add_systems(Update, animate_nova_os_crt);

        // The eased openness the shader reads as its power level.
        app.world_mut()
            .spawn((NovaOsRootMarker, NovaOsOpenness(0.5)));
        let handle = app
            .world_mut()
            .resource_mut::<Assets<NovaOsCrtMaterial>>()
            .add(NovaOsCrtMaterial::default());
        app.world_mut().spawn((
            NovaOsSamplingSurfaceMarker,
            MaterialNode(handle.clone()),
            ComputedNode {
                size: Vec2::new(800.0, 600.0),
                ..default()
            },
        ));

        app.update();

        let material = app
            .world()
            .resource::<Assets<NovaOsCrtMaterial>>()
            .get(&handle)
            .expect("CRT material still present");
        assert_eq!(
            material.data.resolution,
            Vec2::new(800.0, 600.0),
            "the screen's panel pixel size is fed into the resolution uniform"
        );
        assert!(
            material.data.time.is_finite(),
            "the shimmer time uniform is stamped each frame"
        );
        assert_eq!(
            material.data.power, 0.5,
            "NovaOsOpenness drives the CRT power collapse uniform"
        );
    }

    #[test]
    fn nova_os_app_mode_change_pulses_and_decays_the_degauss_uniform() {
        // Task 20260727-014148: a real app launch/exit/switch (a mode change that
        // gets past `sync_nova_os_app_ui`'s diff-guard) kicks the degauss coil, and
        // `animate_nova_os_crt` bleeds the 0..1 envelope back down over real time.
        // Driven with `run_system_once` + a hand-advanced `Time<Real>` so the decay
        // is deterministic rather than tied to the wall clock.
        use std::time::Duration;

        use bevy::ecs::system::RunSystemOnce;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<NovaOsCrtMaterial>();
        // The app-launch spawn loads a font through the AssetServer this rig has
        // (AssetPlugin), so the Font/Image asset types must be registered or the
        // load panics (mirrors `chin_controls_app`).
        app.init_asset::<Font>();
        app.init_asset::<Image>();
        app.init_resource::<Time<Real>>();
        app.init_resource::<NovaOsMonitorSettings>();
        app.init_resource::<NovaOsDegauss>();
        app.init_resource::<NovaOsFlightLog>();
        app.init_resource::<NovaOsTerminal>();
        let mut registry = NovaOsAppRegistry::default();
        registry.register(SampleApp);
        app.insert_resource(registry);

        app.world_mut()
            .spawn((NovaOsRootMarker, NovaOsOpenness(1.0)));
        // A screen entity so the launch actually spawns an app root - that is what
        // makes the NEXT frame a no-op (current == desired) so the pulse fires once,
        // not every frame.
        app.world_mut().spawn(NovaOsScreenMarker);
        let handle = app
            .world_mut()
            .resource_mut::<Assets<NovaOsCrtMaterial>>()
            .add(NovaOsCrtMaterial::default());
        app.world_mut().spawn((
            NovaOsSamplingSurfaceMarker,
            MaterialNode(handle.clone()),
            ComputedNode {
                size: Vec2::new(800.0, 600.0),
                ..default()
            },
        ));

        let degauss_uniform = |app: &App| {
            app.world()
                .resource::<Assets<NovaOsCrtMaterial>>()
                .get(&handle)
                .expect("CRT material still present")
                .data
                .degauss
        };

        // Idle: no pulse yet.
        app.world_mut()
            .run_system_once(animate_nova_os_crt)
            .unwrap();
        assert_eq!(
            degauss_uniform(&app),
            0.0,
            "the CRT sits idle before any launch"
        );

        // Launch an app: the mode change pulses the coil, and the same frame's
        // animate stamps the (near-full) envelope into the uniform.
        app.world_mut()
            .resource_mut::<NovaOsTerminal>()
            .enter_app("sample");
        app.world_mut()
            .run_system_once(sync_nova_os_app_ui)
            .unwrap();
        app.world_mut()
            .run_system_once(animate_nova_os_crt)
            .unwrap();
        let peak = degauss_uniform(&app);
        assert!(
            peak > 0.9,
            "a launch kicks the degauss envelope to (near) full, got {peak}"
        );

        // Half the pulse duration later, with no new mode change, the envelope has
        // bled partway down but is still lit.
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs_f32(NOVA_OS_DEGAUSS_DURATION * 0.5));
        app.world_mut()
            .run_system_once(sync_nova_os_app_ui)
            .unwrap();
        app.world_mut()
            .run_system_once(animate_nova_os_crt)
            .unwrap();
        let mid = degauss_uniform(&app);
        assert!(
            mid < peak && mid > 0.0,
            "the envelope decays but is still lit mid-pulse, got {mid} (peak {peak})"
        );

        // Past the full duration it has settled back to an exact no-op.
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs_f32(NOVA_OS_DEGAUSS_DURATION));
        app.world_mut()
            .run_system_once(animate_nova_os_crt)
            .unwrap();
        assert_eq!(
            degauss_uniform(&app),
            0.0,
            "the degauss envelope settles back to zero (readability preserved at rest)"
        );
    }

    /// A headless app with the NOVA OS chin controls spawned and the computer
    /// open, mirroring `nova_os_app_ui_spawns_chrome_and_close_button_exits`'s
    /// rig (task 20260726-214617).
    fn chin_controls_app() -> App {
        let mut app = App::new();
        // AssetPlugin so `init_asset` works and the AssetServer can hand back
        // (asynchronously-failing) handles for the font/logo loads; the Font +
        // Image asset types must be registered or those loads panic. This
        // mirrors `spawn_nova_os_shell_with_crt`'s callers.
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
        app.init_state::<PauseStates>();
        app.init_resource::<NovaOsFlightLog>();
        app.init_resource::<NovaOsTerminal>();
        app.init_resource::<NovaOsAppRegistry>();
        app.init_resource::<NovaOsCloseTransition>();
        app.init_resource::<NovaOsMonitorSettings>();
        app.init_resource::<NovaOsDegauss>();
        app.init_resource::<Time<Real>>();
        app.init_asset::<Font>();
        app.init_asset::<Image>();
        app.init_asset::<NovaOsCrtMaterial>();
        app.add_observer(setup_nova_os);
        app.add_systems(
            Update,
            (animate_nova_os_crt, sync_nova_os_monitor_controls)
                .run_if(in_state(PauseStates::NovaOs)),
        );
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::NovaOs);
        app.update();
        app
    }

    /// The `Button` entity (not the dial) for a chin knob.
    fn knob_button(app: &mut App, which: NovaOsKnob) -> Entity {
        app.world_mut()
            .query_filtered::<(Entity, &NovaOsKnob), With<Button>>()
            .iter(app.world())
            .find(|(_, knob)| **knob == which)
            .map(|(entity, _)| entity)
            .expect("the knob button spawned")
    }

    /// A knob's dial-pointer rotation, in radians.
    fn dial_rotation(app: &mut App, which: NovaOsKnob) -> f32 {
        app.world_mut()
            .query_filtered::<(&NovaOsKnob, &UiTransform), With<NovaOsKnobDialMarker>>()
            .iter(app.world())
            .find(|(knob, _)| **knob == which)
            .map(|(_, transform)| transform.rotation.as_radians())
            .expect("the dial spawned")
    }

    /// The base colour of a marked indicator bulb (the SND bulb or the PWR LED),
    /// i.e. the state-reporting `BackgroundColor` under the fixed glassy cap.
    fn bulb_color<M: Component>(app: &mut App) -> Color {
        app.world_mut()
            .query_filtered::<&BackgroundColor, With<M>>()
            .iter(app.world())
            .next()
            .map(|background| background.0)
            .expect("the bulb spawned")
    }

    #[test]
    fn nova_os_chin_knobs_cycle_detents() {
        let mut app = chin_controls_app();

        // A sampling surface + material gives `animate_nova_os_crt` a uniform
        // target (the render-capable RTT path is not built headless).
        let handle = app
            .world_mut()
            .resource_mut::<Assets<NovaOsCrtMaterial>>()
            .add(NovaOsCrtMaterial::default());
        app.world_mut().spawn((
            NovaOsSamplingSurfaceMarker,
            MaterialNode(handle.clone()),
            ComputedNode {
                size: Vec2::new(800.0, 600.0),
                ..default()
            },
        ));

        let bright = knob_button(&mut app, NovaOsKnob::Bright);
        assert_eq!(
            app.world()
                .resource::<NovaOsMonitorSettings>()
                .bright_detent,
            NOVA_OS_BRIGHT_DEFAULT_DETENT,
            "BRIGHT boots at the neutral detent"
        );

        // One click advances the detent, rotates the dial and drives the
        // brightness uniform.
        app.world_mut().trigger(Activate { entity: bright });
        app.update();
        assert_eq!(
            app.world()
                .resource::<NovaOsMonitorSettings>()
                .bright_detent,
            2,
            "a BRIGHT click advances one detent"
        );
        assert!(
            (dial_rotation(&mut app, NovaOsKnob::Bright) - NOVA_OS_KNOB_ANGLES[2].to_radians())
                .abs()
                < 1e-3,
            "the dial pointer rotates to the new detent angle"
        );
        let brightness = app
            .world()
            .resource::<Assets<NovaOsCrtMaterial>>()
            .get(&handle)
            .unwrap()
            .data
            .brightness;
        assert_eq!(
            brightness, NOVA_OS_BRIGHT_DETENTS[2],
            "the CRT brightness uniform follows the BRIGHT detent"
        );

        // Four detents wrap back to the start.
        for _ in 0..3 {
            app.world_mut().trigger(Activate { entity: bright });
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<NovaOsMonitorSettings>()
                .bright_detent,
            NOVA_OS_BRIGHT_DEFAULT_DETENT,
            "the 4 BRIGHT detents cycle and wrap"
        );

        // SCAN cycles independently and drives the scanline uniform.
        let scan = knob_button(&mut app, NovaOsKnob::Scan);
        app.world_mut().trigger(Activate { entity: scan });
        app.update();
        assert_eq!(
            app.world().resource::<NovaOsMonitorSettings>().scan_detent,
            3,
            "a SCAN click advances one detent, independent of BRIGHT"
        );
        let scanline = app
            .world()
            .resource::<Assets<NovaOsCrtMaterial>>()
            .get(&handle)
            .unwrap()
            .data
            .scanline_strength;
        assert_eq!(
            scanline, NOVA_OS_SCAN_DETENTS[3],
            "the CRT scanline uniform follows the SCAN detent"
        );
    }

    #[test]
    fn nova_os_snd_toggles_sound_resource() {
        let mut app = chin_controls_app();
        assert!(
            app.world()
                .resource::<NovaOsMonitorSettings>()
                .sound_enabled,
            "the monitor speaker defaults ON"
        );
        let snd = app
            .world_mut()
            .query_filtered::<Entity, With<NovaOsSoundButtonMarker>>()
            .iter(app.world())
            .next()
            .expect("the SND button spawned");

        // Armed: the bulb is lit phosphor and the label is the fixed legend.
        assert_eq!(
            bulb_color::<NovaOsSoundIndicatorMarker>(&mut app),
            NOVA_OS_PHOSPHOR,
            "the SND bulb is lit while the monitor is armed"
        );
        assert!(
            all_texts(&mut app).iter().any(|text| text == "SND"),
            "the SND label is a fixed legend"
        );
        assert!(
            all_texts(&mut app).iter().all(|text| text != "SND ON"),
            "the SND label no longer swaps to an ON/OFF state string"
        );

        app.world_mut().trigger(Activate { entity: snd });
        app.update();
        assert!(
            !app.world()
                .resource::<NovaOsMonitorSettings>()
                .sound_enabled,
            "a SND click mutes the monitor"
        );
        // Muted: the state now reads off the bulb going dark, not a label swap.
        assert_eq!(
            bulb_color::<NovaOsSoundIndicatorMarker>(&mut app),
            NOVA_OS_BULB_OFF,
            "the SND bulb goes dark to report the muted state"
        );
        assert!(
            all_texts(&mut app).iter().any(|text| text == "SND"),
            "the SND label stays the fixed legend when muted"
        );

        app.world_mut().trigger(Activate { entity: snd });
        app.update();
        assert!(
            app.world()
                .resource::<NovaOsMonitorSettings>()
                .sound_enabled,
            "a second SND click re-arms the monitor"
        );
        assert_eq!(
            bulb_color::<NovaOsSoundIndicatorMarker>(&mut app),
            NOVA_OS_PHOSPHOR,
            "re-arming re-lights the SND bulb"
        );
    }

    #[test]
    fn nova_os_pwr_drives_close_transition() {
        let mut app = chin_controls_app();
        assert!(
            !app.world().resource::<NovaOsCloseTransition>().closing,
            "the computer is open, not closing"
        );
        let pwr = app
            .world_mut()
            .query_filtered::<Entity, With<NovaOsPowerButtonMarker>>()
            .iter(app.world())
            .next()
            .expect("the PWR button spawned");

        app.world_mut().trigger(Activate { entity: pwr });
        app.update();
        assert!(
            app.world().resource::<NovaOsCloseTransition>().closing,
            "PWR drives the existing animated close"
        );
    }

    #[test]
    fn nova_os_pwr_led_flashes_orange_while_closing() {
        let mut app = chin_controls_app();

        // Powered and idle: the LED sits at lit phosphor green.
        app.world_mut()
            .run_system_once(drive_nova_os_power_led)
            .unwrap();
        assert_eq!(
            bulb_color::<NovaOsPowerLedMarker>(&mut app),
            NOVA_OS_PHOSPHOR,
            "the PWR LED is green while the monitor is powered"
        );

        // Powering down (the state PWR sets): the LED flashes orange before the
        // raster collapse finishes the close.
        app.world_mut()
            .resource_mut::<NovaOsCloseTransition>()
            .closing = true;
        app.world_mut()
            .run_system_once(drive_nova_os_power_led)
            .unwrap();
        assert_eq!(
            bulb_color::<NovaOsPowerLedMarker>(&mut app),
            NOVA_OS_ORANGE,
            "the PWR LED turns orange while the monitor powers down"
        );
    }

    #[test]
    fn mirror_hover_serves_content_but_never_clobbers_window_ui() {
        // `mirror_nova_os_hover` must feed `Hovered` for the forwarded pointer
        // ONLY on entities rendered through the image (descendants of the content
        // root). It must NOT touch window-space UI - otherwise it force-writes
        // `Hovered(false)` on the real cursor's targets every frame (regressing the
        // chin knobs, menus, any Button). Regression pin for review finding M1.
        use bevy::{
            ecs::entity::EntityHashMap, picking::backend::HitData, platform::collections::HashMap,
        };

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, mirror_nova_os_hover);

        let content_root = app.world_mut().spawn(Hovered::default()).id();
        // A terminal node under the content root (served by the forwarded pointer).
        let terminal_node = app
            .world_mut()
            .spawn((Hovered::default(), ChildOf(content_root)))
            .id();
        // A window-space node hovered by the REAL mouse pointer, NOT under the
        // content root.
        let window_node = app.world_mut().spawn(Hovered(true)).id();

        // The NovaOsRtt pipeline (only content_root matters here).
        app.insert_resource(NovaOsRtt {
            image: Handle::default(),
            camera: Entity::PLACEHOLDER,
            content_root,
            pointer: Entity::PLACEHOLDER,
        });

        // The forwarded pointer's HoverMap hits the terminal node.
        let mut inner = EntityHashMap::default();
        inner.insert(
            terminal_node,
            HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        );
        let mut map: HashMap<PointerId, EntityHashMap<HitData>> = HashMap::default();
        map.insert(nova_os_pointer_id(), inner);
        app.insert_resource(HoverMap(map));

        app.update();

        assert!(
            app.world()
                .entity(terminal_node)
                .get::<Hovered>()
                .unwrap()
                .get(),
            "content-root node hit by the forwarded pointer is mirrored to Hovered(true)"
        );
        assert!(
            app.world()
                .entity(window_node)
                .get::<Hovered>()
                .unwrap()
                .get(),
            "window-space Hovered(true) is NOT clobbered by the forwarded-pointer mirror"
        );
    }

    #[test]
    fn nova_os_exit_closes_computer() {
        // `exit` requests the same animated close as Esc/Start: it flips the
        // shared close transition (which `drive_nova_os_slide` then eases shut).
        let mut app = terminal_command_app();
        assert_eq!(pause_state(&app), PauseStates::NovaOs);
        assert!(!app.world().resource::<NovaOsCloseTransition>().closing);

        submit_terminal_command(&mut app, "exit");

        assert!(
            app.world().resource::<NovaOsCloseTransition>().closing,
            "exit requests the animated close of the computer"
        );
    }

    /// `drive_nova_os_slide` now drives the single monitor's visibility and
    /// openness while retaining the real-time transition used by the old panels.
    #[test]
    fn slide_drives_single_monitor_openness() {
        use std::time::Duration;

        let mut app = App::new();
        // Disable the real TimePlugin so its per-frame clock update cannot
        // overwrite the deltas we advance by hand; drive_nova_os_slide reads
        // Time<Real>, which we own here.
        app.add_plugins(MinimalPlugins.build().disable::<bevy::time::TimePlugin>());
        app.insert_resource(Time::<Real>::default());
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.init_resource::<NovaOsCloseTransition>();
        app.add_systems(Update, drive_nova_os_slide);

        let backdrop = app
            .world_mut()
            .spawn((
                NovaOsBackdropMarker,
                BackgroundColor(theme::semantic::BACKDROP.with_alpha(0.0)),
                Visibility::Hidden,
            ))
            .id();
        let _ = backdrop;
        let monitor = app
            .world_mut()
            .spawn((
                NovaOsRootMarker,
                NovaOsMonitorMarker,
                NovaOsOpenness(0.0),
                Visibility::Hidden,
                Node::default(),
            ))
            .id();

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::NovaOs);
        app.update();
        for _ in 0..4 {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_millis(30));
            app.update();
        }

        let openness = app.world().get::<NovaOsOpenness>(monitor).unwrap().0;
        assert!(
            openness > 0.0 && openness <= 1.0,
            "monitor openness advances toward visible (openness {openness})"
        );
        assert_eq!(
            *app.world().get::<Visibility>(monitor).unwrap(),
            Visibility::Visible
        );

        app.world_mut()
            .resource_mut::<NovaOsCloseTransition>()
            .closing = true;
        for _ in 0..8 {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_millis(30));
            app.update();
        }
        app.update();

        assert_eq!(
            pause_state(&app),
            PauseStates::Unpaused,
            "gameplay resumes only after the NOVA OS close animation finishes"
        );
        assert_eq!(
            *app.world().get::<Visibility>(monitor).unwrap(),
            Visibility::Hidden
        );
    }

    // --- NOVA OS app runtime lifecycle (task 20260726-115334) ---

    /// A test-only sample app registered into the registry to exercise the app
    /// runtime without waiting for the real `map` app. It renders one body row and
    /// exits on its own `q` key, so a test can prove the app owns input.
    struct SampleApp;

    impl NovaOsAppRuntime for SampleApp {
        fn id(&self) -> &'static str {
            "sample"
        }
        fn title(&self) -> &'static str {
            "Sample"
        }
        fn summary(&self) -> &'static str {
            "Test-only lifecycle app"
        }
        fn spawn_body(&self, body: &mut ChildSpawnerCommands, font: Handle<Font>) {
            body.spawn((
                Text::new("SAMPLE APP BODY"),
                nova_os_text_font(DRAWER_LINE_FONT_PX, font),
                TextColor(NOVA_OS_TEXT),
            ));
        }
        fn handle_key(&self, key: &Key) -> NovaOsAppInputOutcome {
            match key {
                Key::Character(c) if c.as_str() == "q" => NovaOsAppInputOutcome::Exit,
                _ => NovaOsAppInputOutcome::Continue,
            }
        }
    }

    /// A second test-only app that exits on ENTER, used to prove the launching
    /// Enter does not bleed into the app it just opened.
    struct EnterExitApp;

    impl NovaOsAppRuntime for EnterExitApp {
        fn id(&self) -> &'static str {
            "enterapp"
        }
        fn title(&self) -> &'static str {
            "Enter"
        }
        fn summary(&self) -> &'static str {
            "Test-only app that exits on Enter"
        }
        fn spawn_body(&self, _body: &mut ChildSpawnerCommands, _font: Handle<Font>) {}
        fn handle_key(&self, key: &Key) -> NovaOsAppInputOutcome {
            match key {
                Key::Enter => NovaOsAppInputOutcome::Exit,
                _ => NovaOsAppInputOutcome::Continue,
            }
        }
    }

    /// Escape via `ButtonInput`, mirroring `press_tab`: press, update, then release
    /// and clear the just-pressed edge (no `InputPlugin` clears it here).
    fn press_escape(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Escape);
        keys.clear();
        app.update();
    }

    /// Headless rig with the NOVA OS OPEN, the sample app registered, and the app
    /// runtime input systems wired (state machine only, no UI). Mirrors
    /// `terminal_command_app` plus the registry, app-command sync, app keyboard and
    /// the context-sensitive Escape route.
    fn app_runtime_app() -> App {
        let mut app = toggle_app();
        init_terminal_input_resources(&mut app);
        let mut registry = NovaOsAppRegistry::default();
        registry.register(SampleApp);
        registry.register(EnterExitApp);
        app.insert_resource(registry);
        app.add_systems(
            Update,
            (
                sync_nova_os_app_commands.run_if(
                    resource_changed::<NovaOsAppRegistry>.or_else(resource_added::<NovaOsTerminal>),
                ),
                handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
                handle_nova_os_app_keyboard.run_if(in_state(GameStates::Playing)),
                close_nova_os_from_menu_keys.run_if(in_state(GameStates::Playing)),
            )
                .chain(),
        );
        press_tab(&mut app);
        assert_eq!(pause_state(&app), PauseStates::NovaOs);
        app.update();
        assert!(
            app.world()
                .resource::<NovaOsTerminal>()
                .app_commands()
                .iter()
                .any(|command| command.id == "sample"),
            "the registered sample app is mirrored into the terminal command set",
        );
        app
    }

    #[test]
    fn terminal_command_launches_registered_app() {
        let mut app = app_runtime_app();
        submit_terminal_command(&mut app, "sample");

        let terminal = app.world().resource::<NovaOsTerminal>();
        assert_eq!(
            terminal.active_mode(),
            TerminalMode::App { id: "sample" },
            "submitting a registered app word enters app mode",
        );
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text.contains("launching sample")),
            "launch prints a status row into the scrollback",
        );
        assert_eq!(
            pause_state(&app),
            PauseStates::NovaOs,
            "the computer stays open while an app runs",
        );
    }

    #[test]
    fn nova_os_app_launch_word_rejects_arguments() {
        let mut app = app_runtime_app();
        submit_terminal_command(&mut app, "sample foo");

        let terminal = app.world().resource::<NovaOsTerminal>();
        assert_eq!(
            terminal.active_mode(),
            TerminalMode::Prompt,
            "an app word with arguments does not launch",
        );
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text == "sample takes no arguments"),
            "the argument rejection is reported",
        );
    }

    #[test]
    fn nova_os_launch_keystroke_does_not_bleed_into_the_app() {
        // The Enter that submits `enterapp` must not reach the app it launches -
        // `EnterExitApp` exits on Enter, so a bleed would close it on the same
        // frame it opened.
        let mut app = app_runtime_app();
        submit_terminal_command(&mut app, "enterapp");
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().active_mode(),
            TerminalMode::App { id: "enterapp" },
            "the launching Enter did not bleed through to exit the app",
        );

        // A SUBSEQUENT Enter does reach the app (it is genuinely Enter-sensitive).
        press_key(&mut app, KeyCode::Enter, Key::Enter, None);
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().active_mode(),
            TerminalMode::Prompt,
            "a later Enter reaches the app and exits it",
        );
    }

    #[test]
    fn nova_os_app_close_restores_terminal_state() {
        let mut app = app_runtime_app();
        // Build some scrollback before launching so we can prove it survives.
        submit_terminal_command(&mut app, "help");
        let before = terminal_scrollback_texts(&app);
        submit_terminal_command(&mut app, "sample");
        assert!(matches!(
            app.world().resource::<NovaOsTerminal>().active_mode(),
            TerminalMode::App { .. }
        ));

        // Escape exits the app back to the terminal, NOT the NOVA OS.
        press_escape(&mut app);

        let terminal = app.world().resource::<NovaOsTerminal>();
        assert_eq!(
            terminal.active_mode(),
            TerminalMode::Prompt,
            "Escape from app mode returns to the terminal",
        );
        assert!(
            !app.world().resource::<NovaOsCloseTransition>().closing,
            "exiting the app does not request a computer close",
        );
        assert_eq!(
            pause_state(&app),
            PauseStates::NovaOs,
            "the computer stays open after the app exits",
        );
        assert_eq!(terminal.prompt(), "", "the prompt is restored empty");
        for row in &before {
            assert!(
                terminal.scrollback().iter().any(|r| &r.text == row),
                "pre-app scrollback row preserved after the app: {row}",
            );
        }
    }

    #[test]
    fn nova_os_app_mode_owns_input_and_escape_exits_app() {
        let mut app = app_runtime_app();
        submit_terminal_command(&mut app, "sample");
        assert!(matches!(
            app.world().resource::<NovaOsTerminal>().active_mode(),
            TerminalMode::App { .. }
        ));

        // Typing while the app owns the screen does not reach the terminal prompt.
        press_text(&mut app, "x");
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().prompt(),
            "",
            "app mode owns input: typing does not edit the terminal prompt",
        );

        // The app's own key drives its exit back to the terminal.
        press_key(
            &mut app,
            KeyCode::KeyQ,
            Key::Character("q".into()),
            Some("q"),
        );
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().active_mode(),
            TerminalMode::Prompt,
            "an app-owned key exits the app",
        );
        assert_eq!(
            pause_state(&app),
            PauseStates::NovaOs,
            "exiting the app keeps the computer open",
        );

        // Back at the prompt, Escape now closes the whole computer.
        press_escape(&mut app);
        assert!(
            app.world().resource::<NovaOsCloseTransition>().closing,
            "from terminal mode Escape requests the computer close",
        );
    }

    #[test]
    fn nova_os_app_state_resets_on_teardown() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaOsFlightLog>();
        app.init_resource::<NovaOsTerminal>();
        app.add_observer(setup_nova_os);
        app.add_observer(remove_nova_os);

        let player = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        app.update();

        // An app is running when the player ship goes away.
        app.world_mut()
            .resource_mut::<NovaOsTerminal>()
            .enter_app("sample");

        app.world_mut()
            .entity_mut(player)
            .remove::<PlayerSpaceshipMarker>();
        app.update();

        let terminal = app.world().resource::<NovaOsTerminal>();
        assert_eq!(
            terminal.active_mode(),
            TerminalMode::Prompt,
            "teardown clears stale app state back to the terminal",
        );
        assert_eq!(
            terminal.scrollback(),
            nova_os_welcome_rows(),
            "teardown restores the welcome screen",
        );
    }

    #[test]
    fn nova_os_app_ui_spawns_chrome_and_close_button_exits() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
        app.init_state::<PauseStates>();
        app.init_resource::<NovaOsFlightLog>();
        app.init_resource::<NovaOsTerminal>();
        app.init_resource::<NovaOsDegauss>();
        let mut registry = NovaOsAppRegistry::default();
        registry.register(SampleApp);
        app.insert_resource(registry);
        app.add_observer(setup_nova_os);
        app.add_observer(remove_nova_os);
        app.add_systems(
            Update,
            sync_nova_os_app_ui.run_if(in_state(PauseStates::NovaOs)),
        );

        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::NovaOs);
        app.update();

        // Launch, then let the UI reconcile.
        app.world_mut()
            .resource_mut::<NovaOsTerminal>()
            .enter_app("sample");
        app.update();

        let app_roots = app
            .world_mut()
            .query_filtered::<Entity, With<NovaOsAppRoot>>()
            .iter(app.world())
            .count();
        assert_eq!(app_roots, 1, "launch spawns exactly one app root");
        let close = app
            .world_mut()
            .query_filtered::<Entity, With<NovaOsAppCloseMarker>>()
            .iter(app.world())
            .next()
            .expect("the app chrome has a close control");
        let content_visibility = app
            .world_mut()
            .query_filtered::<&Visibility, With<NovaOsTerminalContentMarker>>()
            .iter(app.world())
            .next()
            .copied();
        assert_eq!(
            content_visibility,
            Some(Visibility::Hidden),
            "the terminal content is hidden while an app owns the screen",
        );

        // The chrome close control returns to the terminal, the same route as Escape.
        app.world_mut().trigger(Activate { entity: close });
        app.update();

        assert_eq!(
            app.world().resource::<NovaOsTerminal>().active_mode(),
            TerminalMode::Prompt,
            "the chrome close control exits the app",
        );
        let app_roots_after = app
            .world_mut()
            .query_filtered::<Entity, With<NovaOsAppRoot>>()
            .iter(app.world())
            .count();
        assert_eq!(app_roots_after, 0, "exiting despawns the app root");
        let content_after = app
            .world_mut()
            .query_filtered::<&Visibility, With<NovaOsTerminalContentMarker>>()
            .iter(app.world())
            .next()
            .copied();
        assert_eq!(
            content_after,
            Some(Visibility::Inherited),
            "exiting reveals the terminal content again",
        );
    }

    // --- NOVA OS terminal UX parity (task 20260726-214708) ---

    /// The first open types the boot banner row-by-row on real time and reports
    /// the unread-events count; `clear` reprints it instantly.
    #[test]
    fn nova_os_boot_banner_staggers_and_counts_unread() {
        use std::time::Duration;

        let mut app = App::new();
        // Own the real clock so the staggered reveal is deterministic (same rig as
        // `slide_drives_single_monitor_openness`).
        app.add_plugins(MinimalPlugins.build().disable::<bevy::time::TimePlugin>());
        app.insert_resource(Time::<Real>::default());
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.init_resource::<NovaOsTerminal>();

        let mut log = NovaOsFlightLog::default();
        for i in 0..4 {
            log.entries.push(NovaOsFlightLogEntry {
                kind: NovaOsFlightLogEntryKind::Comms,
                objective_id: None,
                speaker: Some("SYS".to_string()),
                message: format!("event {i}"),
                icon: None,
            });
        }
        app.insert_resource(log);
        app.add_systems(OnEnter(PauseStates::NovaOs), begin_nova_os_boot);
        app.add_systems(Update, drain_nova_os_boot);

        // Open the computer for the first time: the banner is QUEUED, not printed.
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::NovaOs);
        app.update();
        let full = nova_os_boot_banner_rows(4, Some("event 3".to_string())).len();
        assert!(
            app.world()
                .resource::<NovaOsTerminal>()
                .scrollback()
                .is_empty(),
            "the boot banner is staggered, not printed instantly",
        );

        // A few 130 ms ticks reveal a FEW rows, not all of them at once.
        for _ in 0..3 {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_millis(130));
            app.update();
        }
        let partway = app.world().resource::<NovaOsTerminal>().scrollback().len();
        assert!(
            partway >= 1 && partway < full,
            "rows reveal gradually (revealed {partway} of {full})",
        );

        // Draining fully lands the whole banner including the unread-events line.
        for _ in 0..12 {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_millis(130));
            app.update();
        }
        let terminal = app.world().resource::<NovaOsTerminal>();
        assert_eq!(terminal.scrollback().len(), full);
        assert!(
            !terminal.has_pending_boot_rows(),
            "the reveal queue drains dry"
        );
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text.contains("4 unread events")),
            "the boot banner reports the unread-events count",
        );

        // `clear` reprints the banner instantly (no staggered queue), still with
        // the current unread count from the snapshot.
        let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
        terminal.insert_text("clear");
        terminal.submit(&TerminalCommandSnapshot {
            unread_events: 2,
            unread_hook: Some("Torpedo bay is down".to_string()),
            ..Default::default()
        });
        assert!(
            !terminal.has_pending_boot_rows(),
            "clear reprints instantly rather than re-staggering",
        );
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text.contains("2 unread events")),
            "clear reprints the current unread-events line",
        );
    }

    /// PageUp/PageDown page the scrollback viewport, clamped to its content.
    #[test]
    fn nova_os_page_keys_scroll_scrollback() {
        let mut app = terminal_command_app();
        let scrollback = app
            .world_mut()
            .spawn((
                NovaOsTerminalScrollbackMarker,
                ScrollPosition(Vec2::new(0.0, 100.0)),
                ComputedNode {
                    size: Vec2::new(100.0, 100.0),
                    content_size: Vec2::new(100.0, 400.0),
                    scrollbar_size: Vec2::ZERO,
                    ..default()
                },
            ))
            .id();

        // PageUp pages toward the top.
        press_key(&mut app, KeyCode::PageUp, Key::PageUp, None);
        let after_up = app
            .world()
            .entity(scrollback)
            .get::<ScrollPosition>()
            .unwrap()
            .0
            .y;
        assert!(
            after_up < 100.0,
            "PageUp pages the scrollback toward the top (y {after_up})",
        );

        // PageDown pages back toward the bottom.
        press_key(&mut app, KeyCode::PageDown, Key::PageDown, None);
        let after_down = app
            .world()
            .entity(scrollback)
            .get::<ScrollPosition>()
            .unwrap()
            .0
            .y;
        assert!(
            after_down > after_up,
            "PageDown pages the scrollback toward the bottom (y {after_down})",
        );
        assert!(
            after_down <= 300.0,
            "the page offset clamps to the content bottom (max 300, got {after_down})",
        );
    }

    /// A test-only app that overrides `hints` to a distinct footer set.
    struct HintsApp;

    impl NovaOsAppRuntime for HintsApp {
        fn id(&self) -> &'static str {
            "hintsapp"
        }
        fn title(&self) -> &'static str {
            "Hints"
        }
        fn summary(&self) -> &'static str {
            "Test-only app with its own footer hints"
        }
        fn spawn_body(&self, _body: &mut ChildSpawnerCommands, _font: Handle<Font>) {}
        fn hints(&self) -> &'static [&'static str] {
            &[
                "1/2/3: DO A THING",
                "ESC: BACK TO TERMINAL",
                "SHIFT+ESC: CLOSE",
            ]
        }
    }

    fn footer_hint_texts(app: &App, footer: Entity) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(children) = app.world().entity(footer).get::<Children>() {
            for child in children {
                if let Some(text) = app.world().entity(*child).get::<Text>() {
                    out.push(text.0.clone());
                }
            }
        }
        out
    }

    /// The footer hint row swaps to the running app's hints while it owns the
    /// screen, and back to the terminal set at the prompt.
    #[test]
    fn nova_os_footer_hints_follow_active_surface() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.init_resource::<NovaOsTerminal>();
        let mut registry = NovaOsAppRegistry::default();
        registry.register(HintsApp);
        app.insert_resource(registry);
        app.add_systems(Update, rebuild_nova_os_footer_hints);

        let footer = app.world_mut().spawn(NovaOsFooterHintsMarker).id();

        // At the prompt the footer shows the terminal hint set.
        app.update();
        assert_eq!(
            footer_hint_texts(&app, footer),
            NOVA_OS_TERMINAL_HINTS
                .iter()
                .map(|hint| hint.to_string())
                .collect::<Vec<_>>(),
            "the terminal surface shows the terminal hints",
        );

        // Entering the app swaps the footer to the app's own hints.
        app.world_mut()
            .resource_mut::<NovaOsTerminal>()
            .enter_app("hintsapp");
        app.update();
        assert_eq!(
            footer_hint_texts(&app, footer),
            vec![
                "1/2/3: DO A THING".to_string(),
                "ESC: BACK TO TERMINAL".to_string(),
                "SHIFT+ESC: CLOSE".to_string(),
            ],
            "an active app swaps the footer to its own hints",
        );

        // Backing out to the prompt restores the terminal hints.
        app.world_mut().resource_mut::<NovaOsTerminal>().exit_app();
        app.update();
        assert_eq!(
            footer_hint_texts(&app, footer),
            NOVA_OS_TERMINAL_HINTS
                .iter()
                .map(|hint| hint.to_string())
                .collect::<Vec<_>>(),
            "returning to the prompt restores the terminal hints",
        );
    }

    /// Press a `<modifier>+<key>` chord via `ButtonInput`, mirroring `press_tab`.
    fn press_chord(app: &mut App, modifier: KeyCode, key: KeyCode) {
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(modifier);
            keys.press(key);
        }
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(modifier);
        keys.release(key);
        keys.clear();
        app.update();
    }

    /// Ctrl+C exits a running app back to the terminal; Shift+Esc closes the whole
    /// computer from inside an app.
    #[test]
    fn nova_os_app_exit_chords() {
        let mut app = app_runtime_app();
        submit_terminal_command(&mut app, "sample");
        assert!(matches!(
            app.world().resource::<NovaOsTerminal>().active_mode(),
            TerminalMode::App { .. }
        ));

        // Ctrl+C backs out to the terminal without closing the computer.
        press_chord(&mut app, KeyCode::ControlLeft, KeyCode::KeyC);
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().active_mode(),
            TerminalMode::Prompt,
            "Ctrl+C exits the app back to the terminal",
        );
        assert!(
            !app.world().resource::<NovaOsCloseTransition>().closing,
            "Ctrl+C does not close the computer",
        );
        assert_eq!(pause_state(&app), PauseStates::NovaOs);

        // Relaunch, then Shift+Esc closes the computer from INSIDE the app.
        submit_terminal_command(&mut app, "sample");
        assert!(matches!(
            app.world().resource::<NovaOsTerminal>().active_mode(),
            TerminalMode::App { .. }
        ));
        press_chord(&mut app, KeyCode::ShiftLeft, KeyCode::Escape);
        assert!(
            app.world().resource::<NovaOsCloseTransition>().closing,
            "Shift+Esc closes the computer from inside an app",
        );
    }

    /// An objective flipping while the computer is open announces itself into the
    /// live scrollback (PoC `checkObjectives`), without needing a `log` command.
    #[test]
    fn nova_os_objective_flip_announces_in_open_terminal() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.init_resource::<NovaOsTerminal>();
        app.init_resource::<NovaOsFlightLog>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<StoryFeed>();
        app.add_systems(
            Update,
            (sync_nova_os_logs, announce_objectives_in_terminal)
                .chain()
                .run_if(resource_changed::<GameObjectives>),
        );

        // Open the computer and post an objective.
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::NovaOs);
        app.world_mut()
            .resource_mut::<GameObjectives>()
            .objectives
            .push(Objective::new("burn", "Burn for Beacon 1"));
        app.update();
        assert!(
            !app.world()
                .resource::<NovaOsTerminal>()
                .scrollback()
                .iter()
                .any(|row| row.text.contains("OBJ x")),
            "posting an objective does not yet announce a completion",
        );

        // Complete it: the OBJ x line lands in the live scrollback.
        app.world_mut()
            .resource_mut::<GameObjectives>()
            .objectives
            .clear();
        app.update();
        assert!(
            app.world()
                .resource::<NovaOsTerminal>()
                .scrollback()
                .iter()
                .any(|row| row.text == "OBJ x Burn for Beacon 1"),
            "an objective completing while open announces into the scrollback",
        );
    }
}
