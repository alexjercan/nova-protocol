//! The Tab ship-computer drawer: one inset NOVA OS cockpit monitor that opens
//! on Tab, freezing the sim and freeing the cursor while active. The monitor
//! replaces the old left/right panels with a physical terminal screen: dark
//! casing, hard bezel, green phosphor display, accent slots and CRT overlays.
//! This module owns the shell, command prompt, scrollback, input handling and
//! terminal content. The existing objectives and combined flight-log data feed
//! read-only terminal commands, but they are no longer visible as permanent
//! monitor panes.
//!
//! # Interaction model
//!
//! Tab opens the drawer by driving the shared [`PauseStates`] axis
//! (`Unpaused -> Drawer`); once NOVA OS owns the keyboard, Tab completes the
//! terminal prompt instead of closing the monitor. ESC/Start/right-stick close
//! requests are owned here so gameplay stays paused until the close animation
//! reaches zero. The freeze + cursor-free are wired in `nova_menu` on
//! `OnEnter/OnExit(PauseStates::Drawer)`, reusing the exact hooks the pause
//! overlay uses (see this task's DECISION.md - the drawer is a THIRD variant of
//! the one freeze axis, not a separate freeze). The drawer is inert while the
//! pause menu owns the freeze (`PauseStates::Paused`), which also means a live
//! outcome overlay - which forces `Paused` - implicitly blocks the drawer
//! without this crate depending on `nova_scenario`'s `CurrentOutcome`.
//!
//! # Animation clock
//!
//! The slide is driven by [`Time<Real>`], NOT the bcs `Tween` (which advances
//! on the default `Res<Time>` = `Time<Virtual>`). Opening the drawer PAUSES
//! virtual time, so a virtual-clocked tween would freeze mid-slide; the slide
//! must keep moving while the sim is frozen, so it reads real time
//! (`verify-engine-guarantees-in-source`: bcs `tween::advance_tweens` uses
//! `Res<Time>`).

use bevy::{
    asset::{io::Reader, AssetApp, AssetLoader, LoadContext},
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    picking::hover::Hovered,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    ui_render::prelude::{MaterialNode, UiMaterial, UiMaterialPlugin},
};
use bevy_common_systems::prelude::{GameObjectives, Objective};
use nova_ui::theme;

use super::NovaHudSystems;
use crate::{prelude::*, GameStates, PauseStates};

/// Seconds for the monitor to fade/activate fully open (or closed).
const DRAWER_SLIDE_SECS: f32 = 0.22;
/// Backdrop dim at full open. Deepened from the original 0.55 (task
/// 20260724-134335): with the flight HUD hidden while the drawer is open, the
/// backdrop is the ONLY thing separating the drawer from the frozen scene, so
/// it doubles as the "you do not notice the old UI is gone" gray field. The
/// owner chose a deeper gray over a real scene blur at the /flow gate (bevy
/// 0.19 has no UI backdrop-filter; see this task's DECISION.md).
const DRAWER_BACKDROP_ALPHA: f32 = 0.94;
const DRAWER_SECTION_TITLE_FONT_PX: f32 = 14.0;
const DRAWER_LINE_FONT_PX: f32 = 16.0;
const DRAWER_ROW_GAP_PX: f32 = 6.0;
const DRAWER_ROW_PADDING_X_PX: f32 = 8.0;
const DRAWER_ROW_PADDING_Y_PX: f32 = 7.0;
const DRAWER_OBJECTIVE_GLYPH_WIDTH_PX: f32 = 18.0;
const DRAWER_LOG_ICON_SIZE_PX: f32 = 20.0;
const DRAWER_SCROLL_LINE_HEIGHT_PX: f32 = 20.0;

/// Horizontal inset from the viewport edge to the physical monitor casing.
const NOVA_OS_MONITOR_INSET_X_PX: f32 = 42.0;
/// Vertical inset from the viewport edge to the physical monitor casing.
const NOVA_OS_MONITOR_INSET_Y_PX: f32 = 52.0;
const NOVA_OS_BEZEL_PAD_PX: f32 = 26.0;
const NOVA_OS_SCREEN_PAD_PX: f32 = 18.0;
const NOVA_OS_TERMINAL_PAD_X_PX: f32 = 16.0;
const NOVA_OS_TERMINAL_PAD_Y_PX: f32 = 14.0;
const NOVA_OS_PROMPT_ROW_HEIGHT_PX: f32 = 58.0;
const NOVA_OS_FONT_PATH: &str = "fonts/SGr-IosevkaTerm-Regular.ttc";
const NOVA_OS_BACKDROP: Color = Color::srgb_u8(0, 3, 6);
const NOVA_OS_CASE: Color = Color::srgb_u8(5, 10, 15);
const NOVA_OS_CASE_RAISED: Color = Color::srgb_u8(11, 21, 32);
const NOVA_OS_CASE_EDGE: Color = Color::srgb_u8(37, 65, 86);
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
const NOVA_OS_ORANGE: Color = Color::srgb_u8(255, 123, 45);
const NOVA_OS_CONTENT_Z: i32 = 0;
const NOVA_OS_OVERLAY_Z: i32 = 1;
const NOVA_OS_PROMPT_PREFIX: &str = "nova> ";

/// Straight-alpha CRT overlay tint + scanline controls, passed to WGSL. Kept
/// deliberately faint so the overlay never films the text underneath: the tint
/// is a whisper of green, the vignette darkens only the outer edges, and there
/// is no centre glow (see `assets/shaders/nova_os_crt.wgsl`).
const NOVA_OS_CRT_TINT: LinearRgba = LinearRgba::new(0.212, 1.0, 0.475, 0.03);
const NOVA_OS_CRT_SCANLINE_STRENGTH: f32 = 0.06;
const NOVA_OS_CRT_VIGNETTE_STRENGTH: f32 = 0.55;
const NOVA_OS_CRT_GRAIN_STRENGTH: f32 = 0.01;

/// Global stacking-context z for the OPEN drawer: it is a modal, so backdrop and
/// panel rise above the flight HUD chrome (which carries no `GlobalZIndex` = 0).
/// Same modal tier the pause overlay uses (`nova_menu`); the drawer and the
/// pause menu are mutually exclusive `PauseStates` variants, so sharing the tier
/// is fine. The tab handle stays at the HUD z (it is chrome). Task 20260724-121541.
const DRAWER_BACKDROP_Z: i32 = 10;
const DRAWER_PANEL_Z: i32 = 11;
/// z for drawer-exempt diagnostic/status chrome that stays visible while the
/// drawer is open: it must sit above the deepened backdrop so the gray field
/// cannot dim it. Read by status widgets that tag themselves
/// [`super::HudDrawerExempt`].
pub(crate) const DRAWER_EXEMPT_Z: i32 = 12;

/// The drawer UI root whose visibility is driven by [`DrawerOpenness`].
#[derive(Component)]
struct DrawerRootMarker;

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

/// Thin overlay rows that approximate CRT scanlines.
#[derive(Component)]
struct NovaOsScanlineMarker;

/// Transparent edge-darkening/glass overlay on the screen.
#[derive(Component)]
struct NovaOsVignetteMarker;

/// Shader-backed CRT overlay matching the HTML PoC's scanline/glass layer.
#[derive(Component)]
struct NovaOsCrtMaterialMarker;

/// The footer hint row from the PoC.
#[derive(Component)]
struct NovaOsFooterHintsMarker;

/// Orange/yellow casing slots copied from the PoC's physical monitor language.
#[derive(Component)]
struct NovaOsAccentSlotMarker;

/// The dim full-screen backdrop behind the panel.
#[derive(Component)]
struct DrawerBackdropMarker;

/// The container the objectives-section lines are (re)built into.
#[derive(Component)]
struct DrawerObjectivesListMarker;

/// Scrollable viewport around a drawer row list.
#[derive(Component)]
struct DrawerScrollViewportMarker;

/// One objective row in the drawer's mission-log list.
#[derive(Component)]
struct DrawerObjectiveRowMarker;

/// Objective id copied onto each drawer row for rebuild and tests.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
struct DrawerObjectiveId(String);

/// Whether a row is still active or retained as completed history.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum DrawerObjectiveRowStatus {
    Active,
}

/// The small status glyph at the start of a drawer objective row.
#[derive(Component)]
struct DrawerObjectiveGlyphMarker;

/// The text entity for a drawer objective row.
#[derive(Component)]
struct DrawerObjectiveTextMarker;

/// Thin overlay used as a completed row's line-through.
#[cfg(test)]
#[derive(Component)]
struct DrawerObjectiveStrikeMarker;

/// Styled empty-state row for the objective list.
#[derive(Component)]
struct DrawerObjectiveEmptyMarker;

/// The container the combined left-panel flight log is rebuilt into.
#[derive(Component)]
struct DrawerFlightLogListMarker;

/// One row in the left-panel combined flight log stream.
#[derive(Component)]
struct DrawerFlightLogRowMarker;

/// Text entity for a combined flight log row.
#[derive(Component)]
struct DrawerFlightLogTextMarker;

/// Styled empty-state row for the combined flight log.
#[derive(Component)]
struct DrawerFlightLogEmptyMarker;

/// Icon semantics for a combined flight log row.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct DrawerFlightLogIconMarker {
    kind: DrawerFlightLogIconKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DrawerFlightLogIconKind {
    CommsAuthored,
    Fallback,
    Objective,
}

/// Openness in `[0, 1]`: 0 fully closed (off-screen past the panel's edge), 1
/// fully open (flush with that edge). Eased toward the state-driven target with
/// real time so it keeps moving while the sim is frozen.
#[derive(Component, Default)]
struct DrawerOpenness(f32);

/// True after the user requested close; gameplay remains paused until the
/// real-time close animation reaches zero.
#[derive(Resource, Default)]
struct DrawerCloseTransition {
    closing: bool,
}

/// Drawer-local combined flight log derived from [`StoryFeed`] and
/// [`GameObjectives`].
///
/// The monitor placeholder keeps the historical stream: comms rows plus
/// objective posted/completed rows, in the order the HUD observes them.
/// Objective text updates edit the open posted row rather than appending
/// duplicate events.
#[derive(Resource, Default, Debug, Clone)]
struct DrawerFlightLog {
    entries: Vec<DrawerFlightLogEntry>,
    active_objective_entries: Vec<DrawerFlightLogActiveObjective>,
    previous_active: Vec<Objective>,
    seen_story: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DrawerFlightLogActiveObjective {
    id: String,
    entry_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct DrawerFlightLogEntry {
    kind: DrawerFlightLogEntryKind,
    objective_id: Option<String>,
    speaker: Option<String>,
    message: String,
    icon: Option<AssetRef<Image>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrawerFlightLogEntryKind {
    Comms,
    ObjectivePosted,
    ObjectiveCompleted,
}

#[derive(Resource, Debug, Clone)]
struct NovaOsTerminal {
    prompt: String,
    cursor: usize,
    scrollback: Vec<TerminalRow>,
    history: Vec<String>,
    history_cursor: Option<usize>,
    completion_hint: Option<String>,
    parse_status: TerminalParseStatus,
    active_mode: TerminalMode,
    /// Set by the `exit` command; the keyboard system consumes it to drive the
    /// animated close of the computer (mirrors the HTML PoC's `exit`).
    pending_close: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TerminalRow {
    kind: TerminalRowKind,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalRowKind {
    Input,
    Output,
    Dim,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalParseStatus {
    Empty,
    Valid,
    ValidPrefix,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalMode {
    Prompt,
}

#[derive(Debug, Clone, Copy)]
struct TerminalCommand {
    name: &'static str,
    summary: &'static str,
}

#[derive(Debug, Clone, Default)]
struct TerminalCommandSnapshot {
    log_rows: Vec<TerminalRow>,
    objective_rows: Vec<TerminalRow>,
    ship_rows: Vec<TerminalRow>,
}

#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
struct NovaOsCrtMaterial {
    #[uniform(0)]
    data: NovaOsCrtUniform,
}

#[derive(ShaderType, Clone, Debug)]
struct NovaOsCrtUniform {
    tint: LinearRgba,
    scanline_strength: f32,
    vignette_strength: f32,
    grain_strength: f32,
}

#[derive(Default, TypePath)]
struct NovaOsTtcFontLoader;

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
                scanline_strength: NOVA_OS_CRT_SCANLINE_STRENGTH,
                vignette_strength: NOVA_OS_CRT_VIGNETTE_STRENGTH,
                grain_strength: NOVA_OS_CRT_GRAIN_STRENGTH,
            },
        }
    }
}

impl UiMaterial for NovaOsCrtMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/nova_os_crt.wgsl".into()
    }
}

// Order and summaries mirror `nova_os_terminal_poc.html`'s command list. `map`
// and `ship viewer` from the PoC stay out until their stretch app tasks land.
const TERMINAL_COMMANDS: &[TerminalCommand] = &[
    TerminalCommand {
        name: "help",
        summary: "Show this command list",
    },
    TerminalCommand {
        name: "log",
        summary: "Print comms and mission events",
    },
    TerminalCommand {
        name: "objectives",
        summary: "Print active objectives",
    },
    TerminalCommand {
        name: "ship",
        summary: "Print ship status summary",
    },
    TerminalCommand {
        name: "clear",
        summary: "Clear terminal scrollback",
    },
    TerminalCommand {
        name: "exit",
        summary: "Suspend the NOVA OS computer",
    },
];

impl Default for NovaOsTerminal {
    fn default() -> Self {
        let mut terminal = Self {
            prompt: String::new(),
            cursor: 0,
            scrollback: nova_os_welcome_rows(),
            history: Vec::new(),
            history_cursor: None,
            completion_hint: Some("type help".to_string()),
            parse_status: TerminalParseStatus::Empty,
            active_mode: TerminalMode::Prompt,
            pending_close: false,
        };
        terminal.refresh_parse();
        terminal
    }
}

impl NovaOsTerminal {
    fn insert_text(&mut self, text: &str) {
        for ch in text.chars().filter(|ch| !ch.is_control()) {
            self.prompt.insert(self.cursor, ch);
            self.cursor += ch.len_utf8();
        }
        self.history_cursor = None;
        self.refresh_parse();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        if let Some((idx, _)) = self.prompt[..self.cursor].char_indices().last() {
            self.prompt.drain(idx..self.cursor);
            self.cursor = idx;
        }
        self.history_cursor = None;
        self.refresh_parse();
    }

    fn delete(&mut self) {
        if self.cursor >= self.prompt.len() {
            return;
        }
        let end = self.prompt[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| self.cursor + offset)
            .unwrap_or(self.prompt.len());
        self.prompt.drain(self.cursor..end);
        self.history_cursor = None;
        self.refresh_parse();
    }

    fn move_cursor_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        if let Some((idx, _)) = self.prompt[..self.cursor].char_indices().last() {
            self.cursor = idx;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor >= self.prompt.len() {
            return;
        }
        self.cursor = self.prompt[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| self.cursor + offset)
            .unwrap_or(self.prompt.len());
    }

    fn submit(&mut self, snapshot: &TerminalCommandSnapshot) {
        let command_line = self.prompt.trim().to_string();
        if command_line.is_empty() {
            self.reset_prompt();
            return;
        }

        self.scrollback.push(TerminalRow {
            kind: TerminalRowKind::Input,
            text: format!("{NOVA_OS_PROMPT_PREFIX}{command_line}"),
        });
        self.history.push(command_line.clone());
        self.history_cursor = None;

        match parse_command(&command_line) {
            TerminalCommandResult::Help => {
                self.scrollback.extend(terminal_help_rows());
            }
            TerminalCommandResult::Clear => {
                self.reset_scrollback_to_welcome();
            }
            TerminalCommandResult::Log => {
                self.scrollback.extend(snapshot.log_rows.clone());
            }
            TerminalCommandResult::Objectives => {
                self.scrollback.extend(snapshot.objective_rows.clone());
            }
            TerminalCommandResult::Ship => {
                self.scrollback.extend(snapshot.ship_rows.clone());
            }
            TerminalCommandResult::Exit => {
                self.pending_close = true;
            }
            TerminalCommandResult::UnexpectedArguments { command } => {
                self.scrollback.push(TerminalRow {
                    kind: TerminalRowKind::Error,
                    text: format!("{command} takes no arguments"),
                });
            }
            TerminalCommandResult::Unknown {
                command,
                suggestion,
            } => {
                // Two rows, matching the HTML PoC's `command not found` +
                // `did you mean ...?` wording.
                self.scrollback.push(TerminalRow {
                    kind: TerminalRowKind::Error,
                    text: format!("command not found: {command}"),
                });
                if let Some(suggestion) = suggestion {
                    self.scrollback.push(TerminalRow {
                        kind: TerminalRowKind::Warn,
                        text: format!("did you mean {suggestion}?"),
                    });
                }
            }
        }

        self.reset_prompt();
    }

    fn complete(&mut self) {
        let Some(prefix) = current_command_prefix(&self.prompt) else {
            return;
        };
        let matches: Vec<&str> = TERMINAL_COMMANDS
            .iter()
            .map(|command| command.name)
            .filter(|name| name.starts_with(prefix))
            .collect();
        let completion = match matches.as_slice() {
            [only] => Some((*only).to_string()),
            [] => None,
            many => common_prefix(many),
        };
        if let Some(completion) = completion {
            self.replace_current_command(&completion);
        }
        self.refresh_parse();
    }

    fn history_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next = match self.history_cursor {
            Some(cursor) if cursor > 0 => cursor - 1,
            Some(cursor) => cursor,
            None => self.history.len() - 1,
        };
        self.set_history_cursor(next);
    }

    fn history_next(&mut self) {
        let Some(cursor) = self.history_cursor else {
            return;
        };
        if cursor + 1 >= self.history.len() {
            self.history_cursor = None;
            self.prompt.clear();
            self.cursor = 0;
            self.refresh_parse();
            return;
        }
        self.set_history_cursor(cursor + 1);
    }

    fn refresh_parse(&mut self) {
        let trimmed = self.prompt.trim();
        if trimmed.is_empty() {
            self.parse_status = TerminalParseStatus::Empty;
            self.completion_hint = Some("type help".to_string());
            return;
        }
        let Some(prefix) = current_command_prefix(trimmed) else {
            self.parse_status = TerminalParseStatus::Empty;
            self.completion_hint = Some("type help".to_string());
            return;
        };
        if TERMINAL_COMMANDS
            .iter()
            .any(|command| command.name == prefix)
        {
            if command_has_arguments(trimmed) {
                self.parse_status = TerminalParseStatus::Invalid;
                self.completion_hint = Some(format!("{prefix} takes no arguments"));
                return;
            }
            self.parse_status = TerminalParseStatus::Valid;
            self.completion_hint = None;
            return;
        }
        if let Some(command) = TERMINAL_COMMANDS
            .iter()
            .find(|command| command.name.starts_with(prefix))
        {
            self.parse_status = TerminalParseStatus::ValidPrefix;
            self.completion_hint = Some(command.name.to_string());
            return;
        }
        self.parse_status = TerminalParseStatus::Invalid;
        self.completion_hint =
            nearest_command(prefix).map(|suggestion| format!("did you mean {suggestion}?"));
    }

    fn reset_prompt(&mut self) {
        self.prompt.clear();
        self.cursor = 0;
        self.refresh_parse();
    }

    fn reset_scrollback_to_welcome(&mut self) {
        self.scrollback = nova_os_welcome_rows();
    }

    fn reset_session(&mut self) {
        self.prompt.clear();
        self.cursor = 0;
        self.scrollback = nova_os_welcome_rows();
        self.history.clear();
        self.history_cursor = None;
        self.active_mode = TerminalMode::Prompt;
        self.refresh_parse();
    }

    fn replace_current_command(&mut self, replacement: &str) {
        let old_len = current_command_prefix(&self.prompt)
            .map(str::len)
            .unwrap_or(0);
        self.prompt.replace_range(0..old_len, replacement);
        self.cursor = replacement.len();
    }

    fn set_history_cursor(&mut self, cursor: usize) {
        self.history_cursor = Some(cursor);
        self.prompt = self.history[cursor].clone();
        self.cursor = self.prompt.len();
        self.refresh_parse();
    }
}

fn nova_os_welcome_rows() -> Vec<TerminalRow> {
    vec![
        TerminalRow {
            kind: TerminalRowKind::Info,
            text: format!("NOVA OS {}", nova_os_version_label()),
        },
        TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "BIOS CHECK: flight computer / ok".to_string(),
        },
        TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "DISPLAY: green phosphor crt / ok".to_string(),
        },
        TerminalRow {
            kind: TerminalRowKind::Warn,
            text: "Hint: type `help` and press Enter.".to_string(),
        },
    ]
}

fn nova_os_version_label() -> String {
    format!("v{}", nova_info::APP_VERSION)
}

fn nova_os_ship_name(name: Option<&Name>) -> String {
    name.map(|name| name.as_str().to_uppercase())
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

fn nova_os_status_text(ship_name: &str) -> String {
    format!("SHIP: {ship_name}     LINK: LOCAL")
}

fn terminal_help_rows() -> Vec<TerminalRow> {
    let command_width = TERMINAL_COMMANDS
        .iter()
        .map(|command| command.name.len())
        .max()
        .unwrap_or(0);
    std::iter::once(TerminalRow {
        kind: TerminalRowKind::Info,
        text: "Available commands:".to_string(),
    })
    .chain(TERMINAL_COMMANDS.iter().map(move |command| TerminalRow {
        kind: TerminalRowKind::Output,
        text: format!(
            "  {:width$}  {}",
            command.name,
            command.summary,
            width = command_width
        ),
    }))
    .collect()
}

enum TerminalCommandResult {
    Help,
    Clear,
    Log,
    Objectives,
    Ship,
    Exit,
    UnexpectedArguments {
        command: String,
    },
    Unknown {
        command: String,
        suggestion: Option<&'static str>,
    },
}

impl DrawerFlightLog {
    fn clear(&mut self) {
        self.entries.clear();
        self.active_objective_entries.clear();
        self.previous_active.clear();
        self.seen_story = 0;
    }
}

fn parse_command(command_line: &str) -> TerminalCommandResult {
    let command = current_command_prefix(command_line).unwrap_or("");
    let mut parts = command_line.split_whitespace();
    if matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some("ship"), Some("viewer"), None)
    ) {
        return TerminalCommandResult::Unknown {
            command: "ship viewer".to_string(),
            suggestion: None,
        };
    }
    if TERMINAL_COMMANDS.iter().any(|known| known.name == command)
        && command_has_arguments(command_line)
    {
        return TerminalCommandResult::UnexpectedArguments {
            command: command.to_string(),
        };
    }
    match command {
        "help" => TerminalCommandResult::Help,
        "clear" => TerminalCommandResult::Clear,
        "log" => TerminalCommandResult::Log,
        "objectives" => TerminalCommandResult::Objectives,
        "ship" => TerminalCommandResult::Ship,
        "exit" => TerminalCommandResult::Exit,
        unknown => TerminalCommandResult::Unknown {
            command: unknown.to_string(),
            suggestion: nearest_command(unknown),
        },
    }
}

fn terminal_snapshot_from_world(
    log: &DrawerFlightLog,
    objectives: &GameObjectives,
    ship_name: Option<&str>,
    ship_sections: &[ShipSectionStatus],
) -> TerminalCommandSnapshot {
    TerminalCommandSnapshot {
        log_rows: terminal_log_rows(log),
        objective_rows: terminal_objective_rows(objectives),
        ship_rows: terminal_ship_rows(ship_name, ship_sections),
    }
}

fn terminal_log_rows(log: &DrawerFlightLog) -> Vec<TerminalRow> {
    if log.entries.is_empty() {
        return vec![TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "Flight log is empty.".to_string(),
        }];
    }
    std::iter::once(TerminalRow {
        kind: TerminalRowKind::Info,
        text: "Flight log:".to_string(),
    })
    .chain(log.entries.iter().map(|entry| TerminalRow {
        kind: match entry.kind {
            DrawerFlightLogEntryKind::Comms => TerminalRowKind::Output,
            DrawerFlightLogEntryKind::ObjectivePosted => TerminalRowKind::Warn,
            DrawerFlightLogEntryKind::ObjectiveCompleted => TerminalRowKind::Info,
        },
        text: drawer_flight_log_text(entry),
    }))
    .collect()
}

fn terminal_objective_rows(objectives: &GameObjectives) -> Vec<TerminalRow> {
    if objectives.objectives.is_empty() {
        return vec![TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "No active objectives.".to_string(),
        }];
    }
    std::iter::once(TerminalRow {
        kind: TerminalRowKind::Info,
        text: "Active objectives:".to_string(),
    })
    .chain(
        objectives
            .objectives
            .iter()
            .enumerate()
            .map(|(index, objective)| TerminalRow {
                kind: TerminalRowKind::Warn,
                text: format!("{}. {} ({})", index + 1, objective.message, objective.id),
            }),
    )
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

fn current_command_prefix(text: &str) -> Option<&str> {
    text.split_whitespace().next()
}

fn command_has_arguments(text: &str) -> bool {
    text.split_whitespace().nth(1).is_some()
}

fn common_prefix(names: &[&str]) -> Option<String> {
    let first = *names.first()?;
    let mut prefix_len = first.len();
    for name in &names[1..] {
        prefix_len = first
            .char_indices()
            .map(|(idx, _)| idx)
            .chain(std::iter::once(first.len()))
            .take_while(|idx| {
                *idx <= name.len()
                    && first[..*idx]
                        .chars()
                        .zip(name[..*idx].chars())
                        .all(|(a, b)| a == b)
            })
            .last()
            .unwrap_or(0)
            .min(prefix_len);
    }
    if prefix_len == 0 {
        None
    } else {
        Some(first[..prefix_len].to_string())
    }
}

fn nearest_command(input: &str) -> Option<&'static str> {
    TERMINAL_COMMANDS
        .iter()
        .map(|command| (command.name, levenshtein(input, command.name)))
        .filter(|(_, distance)| *distance <= 2)
        .min_by_key(|(_, distance)| *distance)
        .map(|(name, _)| name)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let mut previous: Vec<usize> = (0..=b.chars().count()).collect();
    let mut current = vec![0; previous.len()];
    for (i, ca) in a.chars().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let substitution = previous[j] + usize::from(ca != cb);
            let insertion = current[j] + 1;
            let deletion = previous[j + 1] + 1;
            current[j + 1] = substitution.min(insertion).min(deletion);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.chars().count()]
}

/// The reveal's tuck-target rect in logical pixels. This is task 20260721-211520's
/// tween TARGET: the big cockpit objective animates INTO this rect. It is
/// published each frame by `objective_hint` (the minimalist top-right hint
/// replaced the old drawer tab handle as the anchor source - task
/// 20260724-134312). `None` until the hint has laid out at least once (headless
/// rigs without a UI layout pass leave it `None`).
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct DrawerTabAnchor {
    /// The hint rect in logical window pixels, or `None` before first layout.
    pub rect: Option<Rect>,
}

/// Wires the Tab drawer shell: the toggle, the slide and the objectives section.
/// The reveal's tuck anchor ([`DrawerTabAnchor`]) is published by `objective_hint`.
/// Registered by [`super::NovaHudPlugin`].
pub struct NovaDrawerPlugin;

impl Plugin for NovaDrawerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawerTabAnchor>();
        app.init_resource::<DrawerFlightLog>();
        app.init_resource::<NovaOsTerminal>();
        app.init_resource::<DrawerCloseTransition>();
        app.register_asset_loader(NovaOsTtcFontLoader);
        app.add_plugins(UiMaterialPlugin::<NovaOsCrtMaterial>::default());

        // Tab opens the drawer. It keeps running in all of Playing so an open
        // drawer can reserve Tab for terminal completion instead of closing.
        app.add_systems(
            Update,
            (toggle_drawer, close_drawer_from_menu_keys)
                .chain()
                .run_if(in_state(GameStates::Playing)),
        );

        // Shell upkeep while the HUD is live: ease the slide and rebuild the
        // objectives section on change / first spawn. (The reveal's tuck anchor
        // is published by `objective_hint`.)
        app.add_systems(
            Update,
            (
                drive_drawer_slide,
                (
                    sync_drawer_logs,
                    rebuild_drawer_objectives,
                    rebuild_drawer_flight_log,
                )
                    .chain()
                    .run_if(
                        resource_changed::<GameObjectives>
                            .or_else(resource_changed::<StoryFeed>)
                            .or_else(drawer_lists_just_spawned),
                    ),
            )
                .in_set(NovaHudSystems),
        );
        app.add_systems(
            Update,
            scroll_drawer_panels
                .run_if(in_state(PauseStates::Drawer))
                .run_if(resource_exists::<Messages<bevy::input::mouse::MouseWheel>>)
                .in_set(NovaHudSystems),
        );
        app.add_systems(
            Update,
            (
                handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
                rebuild_terminal_ui
                    .run_if(resource_changed::<NovaOsTerminal>.or_else(terminal_ui_just_spawned)),
            )
                .chain()
                .in_set(NovaHudSystems),
        );

        // The drawer is a flight surface: spawn/despawn it with the player ship,
        // like the rest of the HUD.
        app.add_observer(setup_drawer);
        app.add_observer(remove_drawer);
    }
}

/// Tab opens the shared freeze axis and becomes autocomplete while open. The
/// gamepad right-stick click still toggles `Unpaused <-> Drawer`; both inputs are
/// inert while the pause menu owns the freeze (`Paused`) - which is also how a
/// live outcome (it forces `Paused`) blocks the drawer without a cross-crate
/// dependency. The pad button is `RightThumb`, the one free button (task
/// 20260724-134312), mirroring `nova_menu`'s optional-gamepad guard.
fn toggle_drawer(
    keys: Res<ButtonInput<KeyCode>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    mut close: ResMut<DrawerCloseTransition>,
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
            next.set(PauseStates::Drawer);
        }
        PauseStates::Drawer if pad && !tab => {
            close.closing = true;
        }
        PauseStates::Drawer | PauseStates::Paused => {}
    }
}

fn close_drawer_from_menu_keys(
    keys: Res<ButtonInput<KeyCode>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    current: Res<State<PauseStates>>,
    mut close: ResMut<DrawerCloseTransition>,
) {
    if *current.get() != PauseStates::Drawer {
        return;
    }
    let start = gamepad
        .map(|g| g.just_pressed(GamepadButton::Start))
        .unwrap_or(false);
    if keys.just_pressed(KeyCode::Escape) || start {
        close.closing = true;
    }
}

fn handle_terminal_keyboard(
    mut keyboard: MessageReader<KeyboardInput>,
    pause: Res<State<PauseStates>>,
    log: Res<DrawerFlightLog>,
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
    mut close: ResMut<DrawerCloseTransition>,
) {
    let drawer_prompt_active =
        *pause.get() == PauseStates::Drawer && terminal.active_mode == TerminalMode::Prompt;
    for event in keyboard.read() {
        if !drawer_prompt_active {
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
                );
                terminal.submit(&snapshot);
            }
            Key::Tab => terminal.complete(),
            Key::Backspace => terminal.backspace(),
            Key::Delete => terminal.delete(),
            Key::ArrowLeft => terminal.move_cursor_left(),
            Key::ArrowRight => terminal.move_cursor_right(),
            Key::ArrowUp => terminal.history_previous(),
            Key::ArrowDown => terminal.history_next(),
            Key::Character(_) | Key::Space => {
                if let Some(text) = &event.text {
                    terminal.insert_text(text);
                } else if matches!(event.logical_key, Key::Space) {
                    terminal.insert_text(" ");
                }
            }
            _ => {}
        }
    }

    // The `exit` command requests the same animated close as Esc/Start.
    if terminal.pending_close {
        terminal.pending_close = false;
        close.closing = true;
    }
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
fn drawer_lists_just_spawned(
    q_objectives: Query<(), Added<DrawerObjectivesListMarker>>,
    q_log: Query<(), Added<DrawerFlightLogListMarker>>,
) -> bool {
    !q_objectives.is_empty() || !q_log.is_empty()
}

fn terminal_ui_just_spawned(
    q_prompt: Query<(), Added<NovaOsTerminalPromptMarker>>,
    q_scrollback: Query<(), Added<NovaOsTerminalScrollbackMarker>>,
) -> bool {
    !q_prompt.is_empty() || !q_scrollback.is_empty()
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
        Query<(&mut Text, &mut TextColor, &mut TextShadow), With<NovaOsTerminalPromptMarker>>,
        Query<(&mut Text, &mut TextColor, &mut TextShadow), With<NovaOsTerminalPromptAfterMarker>>,
        Query<(&mut Text, &mut TextColor, &mut TextShadow), With<NovaOsTerminalHintMarker>>,
        Query<(&mut Text, &mut TextColor, &mut TextShadow), With<NovaOsTerminalGhostMarker>>,
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
            for row in &terminal.scrollback {
                spawn_terminal_row(parent, row, font.clone());
            }
        });
        scroll.0.y = f32::MAX;
    }

    let prompt_color = prompt_color(&terminal);
    for (mut text, mut color, mut bloom) in &mut text_targets.p0() {
        text.0 = prompt_before_cursor(&terminal);
        color.0 = prompt_color;
        *bloom = nova_os_text_bloom(prompt_color);
    }
    for (mut text, mut color, mut bloom) in &mut text_targets.p1() {
        text.0 = prompt_after_cursor(&terminal);
        color.0 = prompt_color;
        *bloom = nova_os_text_bloom(prompt_color);
    }
    for (mut text, mut color, mut bloom) in &mut text_targets.p2() {
        text.0 = prompt_hint_display(&terminal);
        let hint_color = match terminal.parse_status {
            TerminalParseStatus::Invalid => theme::semantic::THREAT,
            TerminalParseStatus::ValidPrefix => NOVA_OS_PHOSPHOR_MUTED,
            TerminalParseStatus::Empty | TerminalParseStatus::Valid => NOVA_OS_PHOSPHOR_DIM,
        };
        color.0 = hint_color;
        *bloom = nova_os_text_bloom(hint_color);
    }
    for (mut text, mut color, mut bloom) in &mut text_targets.p3() {
        text.0 = prompt_completion_ghost(&terminal);
        let ghost_color = NOVA_OS_TEXT.with_alpha(0.34);
        color.0 = ghost_color;
        *bloom = nova_os_text_bloom(ghost_color);
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
        nova_os_text_bloom(color),
        TextLayout {
            justify: Justify::Left,
            linebreak: LineBreak::WordBoundary,
        },
    ));
}

/// The typed text left of the caret. The prompt line is rendered as three
/// inline pieces - `before` | caret | `after` - plus the dim ghost, so the fish
/// completion continues on the SAME line right after the typed text with a real
/// caret between them (no `|` glyph baked into the text, no leading space).
fn prompt_before_cursor(terminal: &NovaOsTerminal) -> String {
    terminal.prompt[..terminal.cursor].to_string()
}

/// The typed text right of the caret (empty when the caret sits at the end).
fn prompt_after_cursor(terminal: &NovaOsTerminal) -> String {
    terminal.prompt[terminal.cursor..].to_string()
}

fn prompt_hint_display(terminal: &NovaOsTerminal) -> String {
    if terminal.parse_status == TerminalParseStatus::Invalid {
        terminal.completion_hint.clone().unwrap_or_default()
    } else {
        String::new()
    }
}

fn prompt_completion_ghost(terminal: &NovaOsTerminal) -> String {
    if terminal.parse_status != TerminalParseStatus::ValidPrefix {
        return String::new();
    }
    let Some(prefix) = current_command_prefix(&terminal.prompt) else {
        return String::new();
    };
    TERMINAL_COMMANDS
        .iter()
        .map(|command| command.name)
        .find(|name| name.starts_with(prefix))
        .and_then(|name| name.get(prefix.len()..))
        .map(str::to_string)
        .unwrap_or_default()
}

fn prompt_color(terminal: &NovaOsTerminal) -> Color {
    match terminal.parse_status {
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

fn nova_os_text_bloom(color: Color) -> TextShadow {
    TextShadow {
        offset: Vec2::ZERO,
        color: color.with_alpha(0.30),
    }
}

fn scroll_drawer_panels(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut q_panels: Query<
        (&mut ScrollPosition, Option<&Hovered>, Option<&ComputedNode>),
        With<DrawerScrollViewportMarker>,
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
        scroll.0.y = (scroll.0.y - dy).clamp(0.0, max_drawer_scroll_y(computed_node));
    }
}

fn max_drawer_scroll_y(computed_node: Option<&ComputedNode>) -> f32 {
    computed_node
        .map(|node| (node.content_size.y - node.size.y + node.scrollbar_size.y).max(0.0))
        .unwrap_or(f32::MAX)
}

/// Ease [`DrawerOpenness`] toward the state-driven target (1 open, 0 closed)
/// with REAL time, and map it onto the panel offset, the backdrop alpha and
/// both nodes' visibility. Real time because virtual time is paused while the
/// drawer is open (see the module docs).
fn drive_drawer_slide(
    time: Res<Time<Real>>,
    pause: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    mut close: ResMut<DrawerCloseTransition>,
    mut q_panel: Query<
        (&mut DrawerOpenness, &mut Visibility),
        (With<DrawerRootMarker>, Without<DrawerBackdropMarker>),
    >,
    mut q_backdrop: Query<
        (&mut BackgroundColor, &mut Visibility),
        (With<DrawerBackdropMarker>, Without<DrawerRootMarker>),
    >,
) {
    let drawer_active = *pause.get() == PauseStates::Drawer;
    if !drawer_active {
        close.closing = false;
    }
    let target = if drawer_active && !close.closing {
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

    if drawer_active && close.closing && openness <= f32::EPSILON {
        close.closing = false;
        next.set(PauseStates::Unpaused);
    }
}

/// Hidden once fully closed (so a closed drawer never eats a raycast), visible
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

/// Update the drawer's combined left-panel flight log from the story feed and
/// active objective list.
fn sync_drawer_logs(
    story: Res<StoryFeed>,
    objectives: Res<GameObjectives>,
    mut log: ResMut<DrawerFlightLog>,
) {
    if story.0.len() < log.seen_story {
        log.clear();
    }

    for line in story.0.iter().skip(log.seen_story) {
        log.entries.push(DrawerFlightLogEntry {
            kind: DrawerFlightLogEntryKind::Comms,
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
        log.entries.push(DrawerFlightLogEntry {
            kind: DrawerFlightLogEntryKind::ObjectiveCompleted,
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
        log.entries.push(DrawerFlightLogEntry {
            kind: DrawerFlightLogEntryKind::ObjectivePosted,
            objective_id: Some(objective.id.clone()),
            speaker: None,
            message: objective.message.clone(),
            icon: None,
        });
        log.active_objective_entries
            .push(DrawerFlightLogActiveObjective {
                id: objective.id.clone(),
                entry_index,
            });
    }

    log.previous_active = objectives.objectives.clone();
}

/// Rebuild the right objectives-section rows from the active objectives list.
fn rebuild_drawer_objectives(
    mut commands: Commands,
    objectives: Res<GameObjectives>,
    q_list: Query<(Entity, Option<&Children>), With<DrawerObjectivesListMarker>>,
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
            spawn_drawer_empty_objective_row(parent);
            return;
        }
        for objective in &objectives.objectives {
            spawn_drawer_objective_row(parent, objective);
        }
    });
}

fn spawn_drawer_empty_objective_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Name::new("DrawerObjectiveEmpty"),
            DrawerObjectiveEmptyMarker,
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

fn spawn_drawer_objective_row(parent: &mut ChildSpawnerCommands, objective: &Objective) {
    parent
        .spawn((
            Name::new(format!("DrawerObjective {}", objective.id)),
            DrawerObjectiveRowMarker,
            DrawerObjectiveId(objective.id.clone()),
            DrawerObjectiveRowStatus::Active,
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
                DrawerObjectiveGlyphMarker,
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
                    DrawerObjectiveTextMarker,
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
fn rebuild_drawer_flight_log(
    mut commands: Commands,
    log: Res<DrawerFlightLog>,
    asset_server: Option<Res<AssetServer>>,
    q_list: Query<(Entity, Option<&Children>), With<DrawerFlightLogListMarker>>,
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
            spawn_drawer_empty_flight_log_row(parent);
            return;
        }
        for entry in &log.entries {
            spawn_drawer_flight_log_row(parent, entry, asset_server.as_deref());
        }
    });
}

fn spawn_drawer_empty_flight_log_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Name::new("DrawerFlightLogEmpty"),
            DrawerFlightLogEmptyMarker,
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

fn spawn_drawer_flight_log_row(
    parent: &mut ChildSpawnerCommands,
    entry: &DrawerFlightLogEntry,
    asset_server: Option<&AssetServer>,
) {
    let icon_kind = match entry.kind {
        DrawerFlightLogEntryKind::Comms if entry.icon.is_some() => {
            DrawerFlightLogIconKind::CommsAuthored
        }
        DrawerFlightLogEntryKind::Comms => DrawerFlightLogIconKind::Fallback,
        DrawerFlightLogEntryKind::ObjectivePosted
        | DrawerFlightLogEntryKind::ObjectiveCompleted => DrawerFlightLogIconKind::Objective,
    };
    let accent = match entry.kind {
        DrawerFlightLogEntryKind::Comms => theme::CYAN,
        DrawerFlightLogEntryKind::ObjectivePosted => theme::semantic::OBJECTIVE,
        DrawerFlightLogEntryKind::ObjectiveCompleted => theme::semantic::ALLY,
    };

    parent
        .spawn((
            Name::new("DrawerFlightLogRow"),
            DrawerFlightLogRowMarker,
            DrawerFlightLogIconMarker { kind: icon_kind },
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
            spawn_drawer_flight_log_icon(row, entry, icon_kind, accent, asset_server);
            row.spawn((
                DrawerFlightLogTextMarker,
                Text::new(drawer_flight_log_text(entry)),
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

fn spawn_drawer_flight_log_icon(
    row: &mut ChildSpawnerCommands,
    entry: &DrawerFlightLogEntry,
    icon_kind: DrawerFlightLogIconKind,
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
        (Some(icon), DrawerFlightLogIconKind::CommsAuthored) => {
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
                        DrawerFlightLogIconKind::Objective => ">",
                        DrawerFlightLogIconKind::CommsAuthored
                        | DrawerFlightLogIconKind::Fallback => "#",
                    }),
                    TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                    TextColor(accent),
                ));
            });
        }
    }
}

fn drawer_flight_log_text(entry: &DrawerFlightLogEntry) -> String {
    match entry.kind {
        DrawerFlightLogEntryKind::Comms => format!(
            "COMMS {} > {}",
            entry.speaker.as_deref().unwrap_or("UNKNOWN").to_uppercase(),
            entry.message
        ),
        DrawerFlightLogEntryKind::ObjectivePosted => format!("OBJ + {}", entry.message),
        DrawerFlightLogEntryKind::ObjectiveCompleted => format!("OBJ x {}", entry.message),
    }
}

/// Spawn the drawer shell (backdrop plus inset NOVA OS monitor) when the player
/// ship appears - mirrors the other HUD widgets.
fn setup_drawer(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    mut crt_materials: Option<ResMut<Assets<NovaOsCrtMaterial>>>,
    asset_server: Option<Res<AssetServer>>,
    q_spaceship: Query<
        (Entity, Option<&Name>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let Ok((_, ship_name)) = q_spaceship.get(add.entity) else {
        return;
    };
    let font = nova_os_font(asset_server.as_deref());
    let ship_name = nova_os_ship_name(ship_name);

    // Dim backdrop behind the panel (hidden until the drawer opens). NO
    // `HudTier`: the drawer is a modal overlay on its own axis, so the
    // grave/tilde HUD-visibility cycle must not touch it - `apply_hud_visibility`
    // force-hides a non-shown Chrome tier every frame (even self-driven ones),
    // which would blank the drawer if the player opened it with the HUD
    // minimized. The panel's visibility is driven entirely by `drive_drawer_slide`.
    commands.spawn((
        Name::new("DrawerBackdrop"),
        DrawerBackdropMarker,
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
    // top-right objective hint is the drawer affordance + the reveal's tuck
    // anchor now.)

    // One inset physical monitor. It is hidden until opened by the same
    // real-time openness driver the old drawer panels used.
    commands
        .spawn((
            Name::new("NovaOsMonitor"),
            DrawerRootMarker,
            NovaOsMonitorMarker,
            DrawerOpenness(0.0),
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
                ..default()
            },
            BorderColor::all(NOVA_OS_CASE_EDGE),
            BackgroundColor(NOVA_OS_CASE),
        ))
        .with_children(|monitor| {
            spawn_nova_os_accent_slots(monitor);
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
                        ..default()
                    },
                    BorderColor::all(NOVA_OS_CASE_EDGE),
                    BackgroundColor(NOVA_OS_CASE_RAISED),
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
                                ..default()
                            },
                            BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.52)),
                            BackgroundColor(NOVA_OS_SCREEN),
                        ))
                        .with_children(|screen| {
                            spawn_nova_os_terminal_content(screen, font.clone(), &ship_name);
                            spawn_nova_os_screen_overlays(screen, crt_materials.as_deref_mut());
                        });
                });
        });
}

fn spawn_nova_os_accent_slots(parent: &mut ChildSpawnerCommands) {
    for (name, left, color) in [
        ("NovaOsAccentLeft", Val::Px(16.0), NOVA_OS_AMBER),
        ("NovaOsAccentRight", Val::Auto, NOVA_OS_ORANGE),
    ] {
        let mut node = Node {
            position_type: PositionType::Absolute,
            top: Val::Px(18.0),
            width: Val::Px(8.0),
            height: Val::Px(52.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        };
        if left == Val::Auto {
            node.right = Val::Px(16.0);
        } else {
            node.left = left;
        }
        parent.spawn((
            Name::new(name),
            NovaOsAccentSlotMarker,
            node,
            BorderColor::all(color.with_alpha(0.7)),
            BackgroundColor(color.with_alpha(0.18)),
        ));
    }
}

fn spawn_nova_os_screen_overlays(
    screen: &mut ChildSpawnerCommands,
    crt_materials: Option<&mut Assets<NovaOsCrtMaterial>>,
) {
    // Render-capable apps get the single shader overlay, which carries the whole
    // CRT treatment (faint tint, edge vignette, grain). Only headless/minimal
    // rigs with no material assets fall back to the UI-node scanline + vignette,
    // so the two never stack into the pale-green film the earlier build had.
    if let Some(crt_materials) = crt_materials {
        let material = crt_materials.add(NovaOsCrtMaterial::default());
        screen.spawn((
            Name::new("NovaOsCrtMaterialOverlay"),
            NovaOsCrtMaterialMarker,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                ..default()
            },
            MaterialNode(material),
            ZIndex(NOVA_OS_OVERLAY_Z),
            Pickable::IGNORE,
        ));
        return;
    }

    screen.spawn((
        Name::new("NovaOsScanlines"),
        NovaOsScanlineMarker,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            border: UiRect::vertical(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.055)),
        BackgroundColor(NOVA_OS_PHOSPHOR.with_alpha(0.012)),
        ZIndex(NOVA_OS_OVERLAY_Z),
        Pickable::IGNORE,
    ));
    screen.spawn((
        Name::new("NovaOsVignette"),
        NovaOsVignetteMarker,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            border: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.18)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.12)),
        ZIndex(NOVA_OS_OVERLAY_Z),
        Pickable::IGNORE,
    ));
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
                                nova_os_text_bloom(NOVA_OS_PHOSPHOR),
                            ));
                        });
                    topbar.spawn((
                        NovaOsStatusMarker,
                        Text::new(nova_os_status_text(ship_name)),
                        nova_os_text_font(DRAWER_SECTION_TITLE_FONT_PX, font.clone()),
                        TextColor(NOVA_OS_PHOSPHOR_DIM),
                        nova_os_text_bloom(NOVA_OS_PHOSPHOR_DIM),
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
                            DrawerScrollViewportMarker,
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
                                        nova_os_text_bloom(NOVA_OS_AMBER),
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
                                                nova_os_text_bloom(NOVA_OS_PHOSPHOR),
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
                                                    width: Val::Px(2.0),
                                                    height: Val::Px(DRAWER_LINE_FONT_PX + 2.0),
                                                    flex_shrink: 0.0,
                                                    ..default()
                                                },
                                                BackgroundColor(NOVA_OS_AMBER),
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
                                                nova_os_text_bloom(NOVA_OS_PHOSPHOR),
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
                                                nova_os_text_bloom(NOVA_OS_TEXT.with_alpha(0.34)),
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
                                nova_os_text_bloom(NOVA_OS_PHOSPHOR_MUTED),
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
                        column_gap: Val::Px(12.0),
                        ..default()
                    },
                ))
                .with_children(|footer| {
                    for hint in [
                        "TAB: AUTOCOMPLETE",
                        "ESC: CLOSE COMPUTER",
                        "HINT: TYPE HELP",
                    ] {
                        footer.spawn((
                            Text::new(hint),
                            nova_os_text_font(11.0, font.clone()),
                            TextColor(NOVA_OS_PHOSPHOR_MUTED),
                            nova_os_text_bloom(NOVA_OS_PHOSPHOR_MUTED),
                        ));
                    }
                });
        });
}

/// Despawn the drawer shell when the player ship goes away.
fn remove_drawer(
    _remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    mut log: ResMut<DrawerFlightLog>,
    mut terminal: ResMut<NovaOsTerminal>,
    q_parts: Query<Entity, Or<(With<DrawerRootMarker>, With<DrawerBackdropMarker>)>>,
) {
    log.clear();
    terminal.reset_session();
    for entity in &q_parts {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        asset::AssetPlugin, ecs::system::RunSystemOnce, input::touch::TouchPhase,
        state::app::StatesPlugin,
    };
    use bevy_common_systems::prelude::Objective;

    use super::*;

    /// A headless app with just the states + the drawer toggle, enough to drive
    /// the interaction-model state machine.
    fn toggle_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
        app.init_state::<PauseStates>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<DrawerCloseTransition>();
        app.add_systems(Update, toggle_drawer.run_if(in_state(GameStates::Playing)));
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
        app.init_resource::<DrawerFlightLog>();
        app.init_resource::<GameObjectives>();
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
        assert_eq!(pause_state(&app), PauseStates::Drawer);
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
            .scrollback
            .iter()
            .map(|row| row.text.clone())
            .collect()
    }

    fn pause_state(app: &App) -> PauseStates {
        app.world().resource::<State<PauseStates>>().get().clone()
    }

    #[test]
    fn tab_toggles_drawer_state() {
        let mut app = toggle_app();
        assert_eq!(pause_state(&app), PauseStates::Unpaused);
        press_tab(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::Drawer,
            "Tab from Unpaused opens the drawer"
        );
        press_tab(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::Drawer,
            "Tab inside the drawer stays with NOVA OS so the terminal can autocomplete"
        );
    }

    #[test]
    fn tab_opens_drawer_then_completes_terminal_command() {
        let mut app = toggle_app();
        init_terminal_input_resources(&mut app);
        app.add_systems(
            Update,
            handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
        );

        press_tab(&mut app);
        assert_eq!(pause_state(&app), PauseStates::Drawer);
        press_text(&mut app, "he");
        press_tab(&mut app);

        let terminal = app.world().resource::<NovaOsTerminal>();
        assert_eq!(terminal.prompt, "help");
        assert_eq!(terminal.cursor, 4);
    }

    #[test]
    fn terminal_ignores_text_typed_before_drawer_opens() {
        let mut app = toggle_app();
        init_terminal_input_resources(&mut app);
        app.add_systems(
            Update,
            handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
        );

        press_text(&mut app, "flight");
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().prompt,
            "",
            "keyboard text typed during flight is drained but not inserted"
        );

        press_tab(&mut app);
        assert_eq!(pause_state(&app), PauseStates::Drawer);
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().prompt,
            "",
            "opening the drawer does not replay stale flight text into the prompt"
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
        spawn_drawer_shell(&mut app);

        press_tab(&mut app);
        assert_eq!(pause_state(&app), PauseStates::Drawer);
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
        spawn_drawer_shell(&mut app);
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
    fn terminal_prompt_edits_and_navigates_history() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.move_cursor_left();
        terminal.backspace();
        type_text(&mut terminal, "ar");
        terminal.delete();
        assert_eq!(terminal.prompt, "hear");
        assert_eq!(terminal.cursor, 4);

        terminal.submit(&TerminalCommandSnapshot::default());
        type_text(&mut terminal, "clear");
        terminal.submit(&TerminalCommandSnapshot::default());
        terminal.history_previous();
        assert_eq!(terminal.prompt, "clear");
        terminal.history_previous();
        assert_eq!(terminal.prompt, "hear");
        terminal.history_next();
        assert_eq!(terminal.prompt, "clear");
    }

    #[test]
    fn nova_os_clear_restores_welcome_block() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert!(
            terminal.scrollback.len() > nova_os_welcome_rows().len(),
            "help adds rows after the welcome block"
        );

        type_text(&mut terminal, "clear");
        terminal.submit(&TerminalCommandSnapshot::default());

        assert_eq!(terminal.scrollback, nova_os_welcome_rows());
        assert_eq!(terminal.prompt, "");
        assert_eq!(terminal.completion_hint.as_deref(), Some("type help"));
    }

    #[test]
    fn nova_os_help_rows_are_generated_from_registered_commands() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.submit(&TerminalCommandSnapshot::default());

        let help_rows = &terminal.scrollback[nova_os_welcome_rows().len() + 1..];
        assert_eq!(
            help_rows,
            &[
                TerminalRow {
                    kind: TerminalRowKind::Info,
                    text: "Available commands:".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  help        Show this command list".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  log         Print comms and mission events".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  objectives  Print active objectives".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  ship        Print ship status summary".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  clear       Clear terminal scrollback".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  exit        Suspend the NOVA OS computer".to_string()
                }
            ]
        );
    }

    #[test]
    fn terminal_log_command_prints_flight_log_rows() {
        let mut log = DrawerFlightLog::default();
        log.entries.push(DrawerFlightLogEntry {
            kind: DrawerFlightLogEntryKind::Comms,
            objective_id: None,
            speaker: Some("Control".to_string()),
            message: "Hold course.".to_string(),
            icon: None,
        });
        log.entries.push(DrawerFlightLogEntry {
            kind: DrawerFlightLogEntryKind::ObjectivePosted,
            objective_id: Some("burn".to_string()),
            speaker: None,
            message: "Burn for Beacon 1".to_string(),
            icon: None,
        });
        log.entries.push(DrawerFlightLogEntry {
            kind: DrawerFlightLogEntryKind::ObjectiveCompleted,
            objective_id: Some("burn".to_string()),
            speaker: None,
            message: "Burn for Beacon 1".to_string(),
            icon: None,
        });
        let snapshot = terminal_snapshot_from_world(&log, &GameObjectives::default(), None, &[]);
        let mut terminal = NovaOsTerminal::default();

        type_text(&mut terminal, "log");
        terminal.submit(&snapshot);

        let printed: Vec<&str> = terminal
            .scrollback
            .iter()
            .map(|row| row.text.as_str())
            .collect();
        assert!(printed.contains(&"Flight log:"));
        assert!(printed.contains(&"COMMS CONTROL > Hold course."));
        assert!(printed.contains(&"OBJ + Burn for Beacon 1"));
        assert!(printed.contains(&"OBJ x Burn for Beacon 1"));
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
            terminal_snapshot_from_world(&DrawerFlightLog::default(), &objectives, None, &[]);
        let mut terminal = NovaOsTerminal::default();

        type_text(&mut terminal, "objectives");
        terminal.submit(&snapshot);

        let printed: Vec<&str> = terminal
            .scrollback
            .iter()
            .map(|row| row.text.as_str())
            .collect();
        assert!(printed.contains(&"Active objectives:"));
        assert!(printed.contains(&"1. Recover the beacon (beacon)"));
        assert!(printed.contains(&"2. Dock at the relay (dock)"));

        let empty_snapshot = terminal_snapshot_from_world(
            &DrawerFlightLog::default(),
            &GameObjectives::default(),
            None,
            &[],
        );
        let mut empty = NovaOsTerminal::default();
        type_text(&mut empty, "objectives");
        empty.submit(&empty_snapshot);
        assert_eq!(
            empty.scrollback.last().map(|row| row.text.as_str()),
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
                .any(|row| row == "1. Recover the beacon (beacon)"),
            "first command submit reads the current objective resource"
        );

        set_objectives(&mut app, vec![Objective::new("dock", "Dock at the relay")]);
        submit_terminal_command(&mut app, "objectives");

        let printed = terminal_scrollback_texts(&app);
        assert!(
            printed
                .iter()
                .any(|row| row == "1. Dock at the relay (dock)"),
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
            &DrawerFlightLog::default(),
            &GameObjectives::default(),
            Some(ship_name.as_str()),
            &sections,
        );
        let mut terminal = NovaOsTerminal::default();

        type_text(&mut terminal, "ship");
        terminal.submit(&snapshot);

        let printed: Vec<&str> = terminal
            .scrollback
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
    fn nova_os_prompt_renders_fish_style_completion_ghost() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "he");

        assert_eq!(terminal.parse_status, TerminalParseStatus::ValidPrefix);
        assert_eq!(prompt_before_cursor(&terminal), "he");
        assert_eq!(prompt_after_cursor(&terminal), "");
        assert_eq!(prompt_completion_ghost(&terminal), "lp");
        assert_eq!(prompt_hint_display(&terminal), "");

        type_text(&mut terminal, "zz");
        assert_eq!(prompt_completion_ghost(&terminal), "");
        assert_eq!(prompt_hint_display(&terminal), "did you mean help?");
    }

    #[test]
    fn terminal_unknown_command_suggests_nearest_match() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "hlep");

        assert_eq!(terminal.parse_status, TerminalParseStatus::Invalid);
        assert_eq!(
            terminal.completion_hint.as_deref(),
            Some("did you mean help?")
        );

        terminal.submit(&TerminalCommandSnapshot::default());
        // Two HTML-style rows: the error line then the suggestion line.
        let rows: Vec<(TerminalRowKind, &str)> = terminal
            .scrollback
            .iter()
            .map(|row| (row.kind, row.text.as_str()))
            .collect();
        assert!(rows.contains(&(TerminalRowKind::Error, "command not found: hlep")));
        assert!(rows.contains(&(TerminalRowKind::Warn, "did you mean help?")));
    }

    #[test]
    fn terminal_rejects_unexpected_command_arguments() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help garbage");
        assert_eq!(terminal.parse_status, TerminalParseStatus::Invalid);
        assert_eq!(
            terminal.completion_hint.as_deref(),
            Some("help takes no arguments")
        );
        terminal.submit(&TerminalCommandSnapshot::default());
        assert_eq!(
            terminal.scrollback.last().map(|row| row.text.as_str()),
            Some("help takes no arguments")
        );

        type_text(&mut terminal, "clear garbage");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert!(
            !terminal.scrollback.is_empty(),
            "clear with unexpected arguments reports an error instead of clearing scrollback"
        );
        assert_eq!(
            terminal.scrollback.last().map(|row| row.text.as_str()),
            Some("clear takes no arguments")
        );
    }

    #[test]
    fn terminal_ui_renders_prompt_hint_and_invalid_coloring() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaOsTerminal>();
        spawn_drawer_shell(&mut app);
        {
            let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
            terminal.insert_text("hlep");
        }

        app.world_mut()
            .run_system_once(rebuild_terminal_ui)
            .expect("terminal UI rebuild runs");

        let (prompt, prompt_color, prompt_node, prompt_bloom) = app
            .world_mut()
            .query_filtered::<
                (&Text, &TextColor, &Node, &TextShadow),
                With<NovaOsTerminalPromptMarker>,
            >()
            .single(app.world())
            .expect("one terminal prompt");
        assert_eq!(prompt.0, "hlep");
        assert_eq!(prompt_color.0, theme::semantic::THREAT);
        assert_eq!(
            prompt_node.flex_shrink, 0.0,
            "typed input must not collapse inside the prompt row"
        );
        assert!(
            prompt_bloom.offset == Vec2::ZERO && prompt_bloom.color.alpha() <= 0.31,
            "terminal prompt bloom must stay faint and non-directional"
        );

        let (ghost, ghost_node, ghost_bloom) = app
            .world_mut()
            .query_filtered::<(&Text, &Node, &TextShadow), With<NovaOsTerminalGhostMarker>>()
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
        assert!(
            ghost_bloom.offset == Vec2::ZERO && ghost_bloom.color.alpha() <= 0.31,
            "autocomplete ghost bloom must stay faint and non-directional"
        );

        let (prefix, prefix_color, prefix_bloom) = app
            .world_mut()
            .query_filtered::<(&Text, &TextColor, &TextShadow), With<NovaOsPromptPrefixMarker>>()
            .single(app.world())
            .expect("one terminal prompt prefix");
        assert_eq!(prefix.0, "nova>");
        assert_eq!(prefix_color.0, NOVA_OS_AMBER);
        assert!(
            prefix_bloom.offset == Vec2::ZERO && prefix_bloom.color.alpha() <= 0.31,
            "prompt prefix bloom must stay faint and non-directional"
        );

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

    /// The gamepad right-stick click (`RightThumb`) opens the drawer too (task
    /// 20260724-134312). Narrowing the pad button away fails this.
    #[test]
    fn pad_opens_drawer_and_requests_animated_close() {
        let mut app = toggle_app();
        app.init_resource::<ButtonInput<GamepadButton>>();
        assert_eq!(pause_state(&app), PauseStates::Unpaused);

        press_pad(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::Drawer,
            "the right-stick click opens the drawer"
        );
        press_pad(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::Drawer,
            "the right-stick click keeps gameplay paused while close animation runs"
        );
        assert!(
            app.world().resource::<DrawerCloseTransition>().closing,
            "the right-stick click requests the animated drawer close"
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
    // `objective_hint_provides_the_drawer_anchor` - now that the hint is the
    // reveal's tuck-anchor source, task 20260724-134312.)

    fn objectives_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GameObjectives>();
        app.init_resource::<StoryFeed>();
        app.init_resource::<DrawerFlightLog>();
        app.add_systems(
            Update,
            (
                sync_drawer_logs,
                rebuild_drawer_objectives,
                rebuild_drawer_flight_log,
            )
                .chain()
                .run_if(
                    resource_changed::<GameObjectives>
                        .or_else(resource_changed::<StoryFeed>)
                        .or_else(drawer_lists_just_spawned),
                ),
        );
        app
    }

    fn spawn_objectives_list(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                DrawerObjectivesListMarker,
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
                DrawerFlightLogListMarker,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    ..default()
                },
            ))
            .id()
    }

    fn spawn_drawer_shell(app: &mut App) {
        app.add_observer(setup_drawer);
        app.world_mut().spawn((
            Name::new("Survey Cutter"),
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
        ));
        app.update();
    }

    fn spawn_drawer_shell_with_crt(app: &mut App) {
        app.init_asset::<NovaOsCrtMaterial>();
        app.init_asset::<Font>();
        spawn_drawer_shell(app);
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
        spawn_drawer_shell(&mut app);

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
        spawn_drawer_shell(&mut app);

        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<DrawerFlightLogListMarker>>()
                .iter(app.world())
                .count(),
            0,
            "the PoC-style monitor does not show a permanent Flight Log pane"
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<DrawerObjectivesListMarker>>()
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
    fn drawer_wheel_scrolls_viewports_and_clamps_at_top() {
        use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

        let scroll_after = |start_y: f32, wheel_y: f32| -> f32 {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.world_mut().init_resource::<Messages<MouseWheel>>();
            app.world_mut().spawn((
                DrawerScrollViewportMarker,
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
                .run_system_once(scroll_drawer_panels)
                .expect("drawer scroll system runs");
            app.world_mut()
                .query::<&ScrollPosition>()
                .single(app.world())
                .expect("one scroll position")
                .0
                .y
        };

        assert!(
            scroll_after(0.0, -1.0) > 0.0,
            "wheel down from the top scrolls the drawer panel down"
        );
        assert_eq!(
            scroll_after(12.0, 1.0),
            0.0,
            "wheel up clamps at the top instead of going negative"
        );
    }

    #[test]
    fn drawer_wheel_scroll_clamps_at_content_bottom() {
        use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().init_resource::<Messages<MouseWheel>>();
        let viewport = app
            .world_mut()
            .spawn((
                DrawerScrollViewportMarker,
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
            .run_system_once(scroll_drawer_panels)
            .expect("drawer scroll system runs");

        assert_eq!(
            app.world()
                .entity(viewport)
                .get::<ScrollPosition>()
                .unwrap()
                .0
                .y,
            100.0,
            "stored drawer scroll offset clamps to the content bottom"
        );
    }

    #[test]
    fn drawer_wheel_scrolls_only_hovered_viewport_when_one_is_hovered() {
        use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().init_resource::<Messages<MouseWheel>>();
        let hovered = app
            .world_mut()
            .spawn((
                DrawerScrollViewportMarker,
                Hovered(true),
                ScrollPosition(Vec2::ZERO),
            ))
            .id();
        let not_hovered = app
            .world_mut()
            .spawn((
                DrawerScrollViewportMarker,
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
            .run_system_once(scroll_drawer_panels)
            .expect("drawer scroll system runs");

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
            "a non-hovered viewport does not scroll when another drawer viewport is hovered"
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
            .query_filtered::<Entity, With<DrawerObjectiveRowMarker>>()
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
        if entity_ref.contains::<DrawerObjectiveTextMarker>() {
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
            .query_filtered::<&Text, With<DrawerFlightLogTextMarker>>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect()
    }

    #[test]
    fn drawer_objectives_section_uses_styled_rows() {
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
                    .get::<DrawerObjectiveId>()
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
                    .get::<DrawerObjectiveRowStatus>()
                    .expect("row status"),
                DrawerObjectiveRowStatus::Active
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
                        .contains::<DrawerObjectiveGlyphMarker>()
                });
            assert!(has_glyph, "styled rows carry a status glyph");
        }
        assert_eq!(row_text(&app, rows[0]), "Burn for Beacon 1");
    }

    #[test]
    fn drawer_monitor_has_combined_flight_log_stream() {
        let mut app = objectives_app();
        let list = spawn_flight_log_list(&mut app);
        app.update();

        assert!(
            app.world().entity(list).get::<Children>().is_some(),
            "the monitor owns one stream container with an empty row"
        );
        let empty = app
            .world_mut()
            .query_filtered::<Entity, With<DrawerFlightLogEmptyMarker>>()
            .single(app.world())
            .expect("combined log empty state");
        assert!(
            app.world().entity(empty).get::<BackgroundColor>().is_some(),
            "combined log empty state carries drawer chrome fill"
        );
    }

    #[test]
    fn drawer_combined_log_renders_story_feed_rows() {
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
            .query_filtered::<&DrawerFlightLogIconMarker, With<DrawerFlightLogRowMarker>>()
            .single(app.world())
            .expect("comms row has an icon marker");
        assert_eq!(icon.kind, DrawerFlightLogIconKind::Fallback);
    }

    #[test]
    fn drawer_combined_log_records_objective_events_once() {
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
    fn drawer_combined_log_interleaves_comms_and_objective_rows() {
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
    fn drawer_monitor_shows_only_active_objectives() {
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
                .get::<DrawerObjectiveRowStatus>()
                .expect("row status"),
            DrawerObjectiveRowStatus::Active
        );
        assert_eq!(row_text(&app, rows[0]), "Dock at the relay");
        assert!(
            app.world_mut()
                .query_filtered::<(), With<DrawerObjectiveStrikeMarker>>()
                .iter(app.world())
                .next()
                .is_none(),
            "completed objectives are not duplicated as struck-through right-panel rows"
        );
    }

    #[test]
    fn drawer_final_objective_moves_to_flight_log_only() {
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
                .query_filtered::<Entity, With<DrawerObjectiveEmptyMarker>>()
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
    fn terminal_commands_clear_on_drawer_teardown() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<DrawerFlightLog>();
        app.init_resource::<NovaOsTerminal>();
        app.add_observer(setup_drawer);
        app.add_observer(remove_drawer);

        let player = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        app.update();
        {
            let mut log = app.world_mut().resource_mut::<DrawerFlightLog>();
            log.entries.push(DrawerFlightLogEntry {
                kind: DrawerFlightLogEntryKind::ObjectiveCompleted,
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
            });
            assert!(
                terminal
                    .scrollback
                    .iter()
                    .any(|row| row.text.contains("Burn for Beacon 1")),
                "delivery guard: terminal contains scenario output before teardown"
            );
        }

        app.world_mut()
            .entity_mut(player)
            .remove::<PlayerSpaceshipMarker>();
        app.update();

        let log = app.world().resource::<DrawerFlightLog>();
        assert!(
            log.entries.is_empty() && log.previous_active.is_empty() && log.seen_story == 0,
            "drawer teardown clears the retained left-panel log"
        );
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().scrollback,
            nova_os_welcome_rows(),
            "drawer teardown clears printed command output before the next player ship"
        );
    }

    #[test]
    fn drawer_objectives_empty_state_is_styled() {
        let mut app = objectives_app();
        spawn_objectives_list(&mut app);
        app.update();

        let empty = app
            .world_mut()
            .query_filtered::<Entity, With<DrawerObjectiveEmptyMarker>>()
            .single(app.world())
            .expect("styled empty row");
        assert!(
            app.world().entity(empty).get::<BackgroundColor>().is_some(),
            "empty state carries drawer chrome fill"
        );
        assert!(
            app.world().entity(empty).get::<BorderColor>().is_some(),
            "empty state carries drawer chrome border"
        );
    }

    #[test]
    fn drawer_objectives_rebuild_replaces_stale_rows() {
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

    /// The open drawer is a modal: its monitor and backdrop must carry an explicit
    /// `GlobalZIndex` above the HUD chrome (which carries none = 0), or the
    /// top-right objectives panel and other flight HUD draw over it. Mirrors
    /// nova_menu's overlay-z assertion. Fails before the fix (no `GlobalZIndex`).
    #[test]
    fn drawer_renders_above_the_hud() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(setup_drawer);
        // setup_drawer fires on the player ship's PlayerSpaceshipMarker add.
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.update();

        let backdrop_z = app
            .world_mut()
            .query_filtered::<&GlobalZIndex, With<DrawerBackdropMarker>>()
            .single(app.world())
            .expect("the drawer backdrop carries an explicit GlobalZIndex")
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
        // Diagnostic drawer-exempt chrome must out-rank the backdrop so the
        // deepened gray field cannot dim it.
        assert!(
            DRAWER_EXEMPT_Z > backdrop_z,
            "exempt chrome z ({DRAWER_EXEMPT_Z}) must beat the backdrop ({backdrop_z})"
        );
    }

    /// The shell builds one inset physical monitor with the CRT layers the
    /// follow-up terminal tasks can fill, not two permanent side panels.
    #[test]
    fn drawer_spawns_single_nova_os_monitor() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(setup_drawer);
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
            .query_filtered::<(), (With<DrawerRootMarker>, Without<NovaOsMonitorMarker>)>()
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
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsScanlineMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "screen has a scanline layer"
        );
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsVignetteMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "screen has a vignette/glass layer"
        );
        let content_z = app
            .world_mut()
            .query_filtered::<&ZIndex, With<NovaOsTerminalContentMarker>>()
            .single(app.world())
            .expect("terminal content has local z")
            .0;
        let overlay_zs: Vec<i32> = app
            .world_mut()
            .query_filtered::<&ZIndex, Or<(With<NovaOsScanlineMarker>, With<NovaOsVignetteMarker>)>>()
            .iter(app.world())
            .map(|z| z.0)
            .collect();
        assert_eq!(overlay_zs.len(), 2);
        for overlay_z in &overlay_zs {
            assert!(
                *overlay_z > content_z,
                "CRT overlays render above terminal content (overlay {overlay_z}, content {content_z})"
            );
        }
        let prompt_z = app
            .world_mut()
            .query_filtered::<&ZIndex, With<NovaOsPromptRowMarker>>()
            .single(app.world())
            .expect("prompt strip has local z")
            .0;
        assert!(
            overlay_zs.iter().all(|overlay_z| prompt_z > *overlay_z),
            "the prompt strip renders above CRT overlays so typed input remains visible"
        );
        assert!(
            app.world_mut()
                .query_filtered::<(), With<NovaOsAccentSlotMarker>>()
                .iter(app.world())
                .count()
                >= 2,
            "monitor casing has orange/yellow accent slots"
        );
    }

    #[test]
    fn drawer_matches_nova_os_terminal_poc_structure() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        spawn_drawer_shell(&mut app);

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
            "SHIP: SURVEY CUTTER     LINK: LOCAL".to_string(),
            format!("NOVA OS {}", nova_os_version_label()),
            "BIOS CHECK: flight computer / ok".to_string(),
            "DISPLAY: green phosphor crt / ok".to_string(),
            "Hint: type `help` and press Enter.".to_string(),
            "nova>".to_string(),
            "TAB: AUTOCOMPLETE".to_string(),
            "ESC: CLOSE COMPUTER".to_string(),
            "HINT: TYPE HELP".to_string(),
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
    fn drawer_uses_crt_material_overlay_when_assets_are_available() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        spawn_drawer_shell_with_crt(&mut app);

        let material_nodes = app
            .world_mut()
            .query_filtered::<&MaterialNode<NovaOsCrtMaterial>, With<NovaOsCrtMaterialMarker>>()
            .iter(app.world())
            .count();
        assert_eq!(
            material_nodes, 1,
            "the drawer has one shader-backed CRT overlay in render-capable apps"
        );
        assert_eq!(
            app.world().resource::<Assets<NovaOsCrtMaterial>>().len(),
            1,
            "the CRT overlay owns one material asset"
        );
        let material = app
            .world()
            .resource::<Assets<NovaOsCrtMaterial>>()
            .iter()
            .next()
            .expect("one CRT material")
            .1;
        assert_eq!(
            material.data.vignette_strength, NOVA_OS_CRT_VIGNETTE_STRENGTH,
            "CRT material carries the near-black corner pass"
        );
        assert_eq!(
            material.data.grain_strength, NOVA_OS_CRT_GRAIN_STRENGTH,
            "CRT material carries the subtle square grain pass"
        );
    }

    #[test]
    fn nova_os_registered_commands_match_html_set() {
        // The executable set + order mirror the HTML PoC (minus the app-launch
        // commands `map` / `ship viewer`, which stay in their stretch tasks).
        let registered: Vec<&str> = TERMINAL_COMMANDS
            .iter()
            .map(|command| command.name)
            .collect();
        assert_eq!(
            registered,
            vec!["help", "log", "objectives", "ship", "clear", "exit"]
        );

        assert!(matches!(parse_command("help"), TerminalCommandResult::Help));
        assert!(matches!(
            parse_command("clear"),
            TerminalCommandResult::Clear
        ));
        assert!(matches!(parse_command("log"), TerminalCommandResult::Log));
        assert!(matches!(
            parse_command("objectives"),
            TerminalCommandResult::Objectives
        ));
        assert!(matches!(parse_command("ship"), TerminalCommandResult::Ship));
        assert!(matches!(parse_command("exit"), TerminalCommandResult::Exit));
        for planned in ["map", "ship viewer", "reload", "repair"] {
            assert!(
                matches!(
                    parse_command(planned),
                    TerminalCommandResult::Unknown { .. }
                ),
                "{planned} stays deferred to its own task"
            );
        }
    }

    #[test]
    fn nova_os_help_lists_html_command_set() {
        // `help` output lists exactly the executable set, in HTML order.
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.submit(&TerminalCommandSnapshot::default());
        let listed: Vec<String> = terminal
            .scrollback
            .iter()
            .filter_map(|row| {
                let trimmed = row.text.trim_start();
                TERMINAL_COMMANDS
                    .iter()
                    .map(|command| command.name)
                    .find(|name| trimmed.starts_with(name))
                    .filter(|name| trimmed.starts_with(&format!("{name} ")))
                    .map(str::to_string)
            })
            .collect();
        assert_eq!(
            listed,
            vec!["help", "log", "objectives", "ship", "clear", "exit"]
        );
    }

    #[test]
    fn nova_os_exit_closes_computer() {
        // `exit` requests the same animated close as Esc/Start: it flips the
        // shared close transition (which `drive_drawer_slide` then eases shut).
        let mut app = terminal_command_app();
        assert_eq!(pause_state(&app), PauseStates::Drawer);
        assert!(!app.world().resource::<DrawerCloseTransition>().closing);

        submit_terminal_command(&mut app, "exit");

        assert!(
            app.world().resource::<DrawerCloseTransition>().closing,
            "exit requests the animated close of the computer"
        );
    }

    /// `drive_drawer_slide` now drives the single monitor's visibility and
    /// openness while retaining the real-time transition used by the old panels.
    #[test]
    fn slide_drives_single_monitor_openness() {
        use std::time::Duration;

        let mut app = App::new();
        // Disable the real TimePlugin so its per-frame clock update cannot
        // overwrite the deltas we advance by hand; drive_drawer_slide reads
        // Time<Real>, which we own here.
        app.add_plugins(MinimalPlugins.build().disable::<bevy::time::TimePlugin>());
        app.insert_resource(Time::<Real>::default());
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.init_resource::<DrawerCloseTransition>();
        app.add_systems(Update, drive_drawer_slide);

        let backdrop = app
            .world_mut()
            .spawn((
                DrawerBackdropMarker,
                BackgroundColor(theme::semantic::BACKDROP.with_alpha(0.0)),
                Visibility::Hidden,
            ))
            .id();
        let _ = backdrop;
        let monitor = app
            .world_mut()
            .spawn((
                DrawerRootMarker,
                NovaOsMonitorMarker,
                DrawerOpenness(0.0),
                Visibility::Hidden,
                Node::default(),
            ))
            .id();

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Drawer);
        app.update();
        for _ in 0..4 {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_millis(30));
            app.update();
        }

        let openness = app.world().get::<DrawerOpenness>(monitor).unwrap().0;
        assert!(
            openness > 0.0 && openness <= 1.0,
            "monitor openness advances toward visible (openness {openness})"
        );
        assert_eq!(
            *app.world().get::<Visibility>(monitor).unwrap(),
            Visibility::Visible
        );

        app.world_mut()
            .resource_mut::<DrawerCloseTransition>()
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
            "gameplay resumes only after the drawer close animation finishes"
        );
        assert_eq!(
            *app.world().get::<Visibility>(monitor).unwrap(),
            Visibility::Hidden
        );
    }
}
