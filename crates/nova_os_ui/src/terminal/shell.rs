//! The monitor's own behaviour: chin control observers, the power/boot/shutdown
//! sequence, app launch and teardown, and the header/status reconcilers.
//!
//! This is the layer between the chrome in `casing` and the app runtimes -
//! it owns WHICH surface is live, not what that surface draws.
//!
//! Touch this module when changing what a monitor control does or how an app
//! is entered and left.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_gameplay::{
    audio::prelude::{SoundBank, UiSfx, NOVA_OS_COIL_VOLUME},
    cheats::prelude::RunCheats,
    markers::prelude::{PlayerSpaceshipMarker, SpaceshipRootMarker},
    PauseStates,
};
use nova_hud::prelude::HudNovaOsExempt;
use nova_input::prelude::InputBindings;
use nova_os::prelude::*;
use nova_ui::{font::UiFont, theme};

use super::{casing::*, components::*, content::*, sound::*, style::*};

/// The header's app close control: clicking it returns to the terminal, the same
/// route as Escape, and plays the degauss coil on a real exit. Shown only while
/// an app owns the screen (visibility toggled by [`reconcile_nova_os_header`]).
pub(crate) fn on_nova_os_app_close(
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
/// [`super::crt::animate_nova_os_crt`]).
pub(crate) fn on_nova_os_bright_knob(
    _activate: On<Activate>,
    mut settings: ResMut<NovaOsMonitorSettings>,
) {
    settings.cycle(NovaOsKnob::Bright);
}

/// SCAN knob click: advance the scanline detent.
pub(crate) fn on_nova_os_scan_knob(
    _activate: On<Activate>,
    mut settings: ResMut<NovaOsMonitorSettings>,
) {
    settings.cycle(NovaOsKnob::Scan);
}

/// SND button click: toggle the monitor speaker flag (default ON). The NOVA OS
/// sound task consumes the flag to mute/unmute the bed + cues; the on-screen
/// state reads off the bulb flipping, since the label is a fixed "SND" legend.
pub(crate) fn on_nova_os_sound_button(
    _activate: On<Activate>,
    mut settings: ResMut<NovaOsMonitorSettings>,
) {
    settings.sound_enabled = !settings.sound_enabled;
}

/// PWR button click: drive the existing animated close, the diegetic twin of the
/// `exit` command. Always powers the monitor off (from an app or the prompt).
pub(crate) fn on_nova_os_power_button(
    _activate: On<Activate>,
    mut close: ResMut<NovaOsCloseTransition>,
) {
    close.closing = true;
}

/// Reconcile the chin controls' look with [`NovaOsMonitorSettings`] after a knob
/// turn or SND toggle: rotate each dial pointer to its detent angle, and light /
/// dim the SND bulb. Spawn-time state is set directly in
/// [`spawn_nova_os_knob`]/[`spawn_nova_os_sound_button`]; this handles live
/// changes (gated on `resource_changed`, which also harmlessly re-applies the
/// current state on the init frame). The SND label no longer swaps text: the
/// bulb colour is the only moving part reporting the mute state.
pub(crate) fn sync_nova_os_monitor_controls(
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
pub(crate) fn drive_nova_os_power_led(
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
/// launch spawns the app root (body only) as an absolute-fill child of the
/// persistent `<main>` region and hides the terminal surface; exit despawns the
/// app root and reveals the terminal, whose scrollback was never touched. The
/// header and footer are siblings of `<main>`, so they stay put across the swap
/// (the header breadcrumb + close control are reconciled by
/// [`reconcile_nova_os_header`]). Runs while the computer is open and
/// diff-guards itself, so a NOVA OS reopened onto a persisted app rebuilds the
/// app and a plain reopen keeps the terminal.
pub(crate) fn sync_nova_os_app_ui(
    mut commands: Commands,
    terminal: Res<NovaOsTerminal>,
    registry: Res<NovaOsCommandRegistry>,
    ui_font: Option<Res<UiFont>>,
    mut degauss: ResMut<NovaOsDegauss>,
    q_main: Query<Entity, With<NovaOsMainMarker>>,
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
    // play on the same transitions.
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
    // The app body fills the persistent `<main>` region (an absolute-fill child),
    // so it renders through the CRT shader exactly as the terminal does and never
    // covers the header or footer. Same target render-capable or headless.
    let target = q_main.single().ok();
    let (Some(id), Some(target)) = (desired, target) else {
        return;
    };
    let Some(app) = registry.app_runtime(id) else {
        return;
    };
    let font = nova_os_font(ui_font.as_deref());
    commands.entity(target).with_children(|parent| {
        spawn_nova_os_app(parent, app, font);
    });
}

/// Spawn one app surface: just the app's body, absolute-filling the persistent
/// `<main>` region at content depth so the shared CRT overlay sits on top of it
/// exactly as it does over the terminal. The app has no chrome bar of its own -
/// the persistent header carries its breadcrumb + close control, and the footer
/// carries its keybinds. `<main>` is already inset by the content root's
/// safe-area padding and sits between the header and footer, so the app body
/// needs no safe-area padding or footer-reserve margin of its own.
pub(crate) fn spawn_nova_os_app(
    main: &mut ChildSpawnerCommands,
    app: &dyn NovaOsAppRuntime,
    font: Handle<Font>,
) {
    main.spawn((
        Name::new(format!("NovaOsApp:{}", app.id())),
        NovaOsAppRoot { id: app.id() },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            min_height: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(NOVA_OS_SCREEN),
        ZIndex(NOVA_OS_CONTENT_Z),
    ))
    .with_children(|body| {
        app.spawn_body(body, font.clone());
    });
}
pub(crate) fn terminal_ui_just_spawned(
    q_prompt: Query<(), Added<NovaOsTerminalPromptMarker>>,
    q_scrollback: Query<(), Added<NovaOsTerminalScrollbackMarker>>,
) -> bool {
    !q_prompt.is_empty() || !q_scrollback.is_empty()
}

/// On the FIRST NOVA OS open of a session, kick off the staggered boot banner:
/// clear the scrollback and queue the welcome + unread-events rows for
/// [`drain_nova_os_boot`] to reveal one-by-one (PoC `printBanner`). Subsequent
/// opens keep the scrollback the player left behind.
pub(crate) fn begin_nova_os_boot(mut terminal: ResMut<NovaOsTerminal>, log: Res<NovaOsFlightLog>) {
    if terminal.is_booted() {
        return;
    }
    let unread = log.entries.len().saturating_sub(terminal.seen_events());
    let hook = nova_os_unread_hook(&log, terminal.seen_events());
    terminal.begin_boot(nova_os_boot_banner_rows(unread, hook));
}

/// When the computer closes, remember how many flight-log entries have been seen
/// so a later boot's unread-events count only covers what arrived afterward.
pub(crate) fn mark_nova_os_events_seen(
    mut terminal: ResMut<NovaOsTerminal>,
    log: Res<NovaOsFlightLog>,
) {
    terminal.set_seen_events(log.entries.len());
}

/// Reveal the queued boot-banner rows one-by-one on real time (PoC `printBanner`
/// ~130 ms cadence; virtual time is frozen while the computer is open). Runs only
/// while rows are pending, so once the banner finishes it stops touching the
/// terminal's change detection.
pub(crate) fn drain_nova_os_boot(
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

pub(crate) fn nova_os_footer_just_spawned(
    q_footer: Query<(), Added<NovaOsFooterHintsMarker>>,
) -> bool {
    !q_footer.is_empty()
}

pub(crate) fn nova_os_header_just_spawned(q_brand: Query<(), Added<NovaOsBrandMarker>>) -> bool {
    !q_brand.is_empty()
}

/// Reconcile the persistent header with the active surface: the brand text
/// names the live shell (`// SHELL`, `// COMMANDS`) or the running app's
/// breadcrumb, the topbar head swaps between the ship the computer belongs to
/// and the run's cheat state, and the header close control shows only while an
/// app owns the screen.
///
/// Keyed on the whole (shell, mode, ship, arming) tuple so ordinary prompt
/// edits do not rewrite the header; forced once when the header is freshly
/// spawned so a reopen starts from the right state.
pub(crate) fn reconcile_nova_os_header(
    terminal: Res<NovaOsTerminal>,
    cheats: Option<Res<RunCheats>>,
    q_added: Query<(), Added<NovaOsBrandMarker>>,
    q_ship: Query<Option<&Name>, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    mut q_brand: Query<&mut Text, With<NovaOsBrandMarker>>,
    mut q_status: Query<
        (&mut Text, &mut TextColor),
        (With<NovaOsStatusMarker>, Without<NovaOsBrandMarker>),
    >,
    mut q_close: Query<&mut Visibility, With<NovaOsAppCloseMarker>>,
    mut last: Local<Option<NovaOsHeaderState>>,
) {
    let ship = nova_os_ship_name(q_ship.iter().next().flatten());
    let state = NovaOsHeaderState {
        shell: terminal.active_shell(),
        mode: terminal.active_mode(),
        armed: cheats.is_some_and(|cheats| cheats.is_armed()),
        ship,
    };
    if q_added.is_empty() && last.as_ref() == Some(&state) {
        return;
    }
    for mut text in &mut q_brand {
        text.0 = nova_os_header_breadcrumb(state.shell, state.mode);
    }
    // The FPS tail belongs to `drive_nova_os_topbar_fps`, which runs on real
    // time; splice the new head in front of whatever reading is on screen.
    let head = match state.shell {
        ShellKind::NovaOs => nova_os_topbar_head(&state.ship),
        ShellKind::Commands => command_topbar_head(state.armed),
    };
    let armed_shell = state.shell == ShellKind::Commands && state.armed;
    for (mut text, mut color) in &mut q_status {
        let tail = text
            .0
            .split_once(NOVA_OS_TOPBAR_FPS_MARKER)
            .map(|(_, tail)| tail.to_string())
            .unwrap_or_else(|| nova_os_fps_segment(None));
        text.0 = format!("{head}{NOVA_OS_TOPBAR_FPS_MARKER}{tail}");
        // An armed run says so in the same amber the shell warns in, so the
        // state is legible without reading the word.
        let next = if armed_shell {
            NOVA_OS_AMBER
        } else {
            NOVA_OS_PHOSPHOR_MUTED
        };
        if color.0 != next {
            color.0 = next;
        }
    }
    let close_visibility = match state.mode {
        TerminalMode::App { .. } => Visibility::Inherited,
        TerminalMode::Prompt => Visibility::Hidden,
    };
    for mut visibility in &mut q_close {
        *visibility = close_visibility;
    }
    *last = Some(state);
}

/// Everything the header draws, so the reconciler can compare a whole frame's
/// worth of answer against the last one it painted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NovaOsHeaderState {
    shell: ShellKind,
    mode: TerminalMode,
    armed: bool,
    ship: String,
}

/// Rebuild the footer hint row whenever the active surface changes, so the hints
/// swap per surface (terminal vs a running app). Keyed on `active_mode` via a
/// `Local`, so ordinary prompt edits (which change the terminal resource but not
/// the mode) do not thrash the footer. Forced once when the footer is freshly
/// spawned (the `Local` survives a shell teardown/respawn, so without this a
/// respawn whose mode matches the stale `Local` would skip refilling the new
/// footer), mirroring [`reconcile_nova_os_header`].
///
/// A rebind is the third trigger: the hints name the keys the actions hold, so
/// a footer keyed on the mode alone would keep printing the old key for as long
/// as the player stayed on the surface where they moved it.
pub(crate) fn rebuild_nova_os_footer_hints(
    terminal: Res<NovaOsTerminal>,
    registry: Res<NovaOsCommandRegistry>,
    bindings: Res<InputBindings>,
    ui_font: Option<Res<UiFont>>,
    mut commands: Commands,
    q_added: Query<(), Added<NovaOsFooterHintsMarker>>,
    q_footer: Query<(Entity, Option<&Children>), With<NovaOsFooterHintsMarker>>,
    mut last_mode: Local<Option<(ShellKind, TerminalMode)>>,
) {
    let surface = (terminal.active_shell(), terminal.active_mode());
    if q_added.is_empty() && !bindings.is_changed() && *last_mode == Some(surface) {
        return;
    }
    *last_mode = Some(surface);
    let hints = nova_os_footer_hints(
        terminal.active_shell(),
        terminal.active_mode(),
        &registry,
        &bindings,
    );
    let font = nova_os_font(ui_font.as_deref());
    for (footer, children) in &q_footer {
        if let Some(children) = children {
            for &child in children {
                commands.entity(child).despawn();
            }
        }
        commands.entity(footer).with_children(|footer| {
            for hint in &hints {
                footer.spawn((
                    Text::new(hint),
                    nova_os_text_font(11.0, font.clone()),
                    TextColor(NOVA_OS_PHOSPHOR_MUTED),
                ));
            }
        });
    }
}

/// Reconcile the terminal surface with [`NovaOsTerminal`]: respawn the scrollback
/// rows when the rows changed, and refresh the four prompt-line texts. The
/// resource is marked changed by every prompt edit - including caret movement,
/// which changes nothing on screen - so the row loop is keyed on the scrollback's
/// own revision instead, and the prompt writes go through `set_if_neq`. Rebuilding
/// on the resource alone respawned every row on each keystroke.
pub(crate) fn rebuild_terminal_ui(
    mut commands: Commands,
    terminal: Res<NovaOsTerminal>,
    ui_font: Option<Res<UiFont>>,
    // A freshly spawned scrollback carries no rows, so it must rebuild whatever
    // the `Local`s remember from the shell it replaced.
    q_added: Query<(), Added<NovaOsTerminalScrollbackMarker>>,
    mut q_scrollback: Query<
        (Entity, Option<&Children>, &mut ScrollPosition),
        With<NovaOsTerminalScrollbackMarker>,
    >,
    mut text_targets: ParamSet<(
        Query<(&mut Text, &mut TextColor), With<NovaOsTerminalPromptMarker>>,
        Query<(&mut Text, &mut TextColor), With<NovaOsTerminalPromptAfterMarker>>,
        Query<(&mut Text, &mut TextColor), With<NovaOsTerminalHintMarker>>,
        Query<(&mut Text, &mut TextColor), With<NovaOsTerminalGhostMarker>>,
        Query<&mut Text, With<NovaOsPromptPrefixMarker>>,
    )>,
    // The scrollback revision and length last time we rebuilt. The length gates
    // the auto-scroll, so we only pin to the bottom when NEW output arrived:
    // pinning on every terminal change yanked a manual PageUp/wheel scroll
    // straight back down (owner playtest).
    mut last_revision: Local<Option<u64>>,
    mut last_len: Local<usize>,
) {
    let font = nova_os_font(ui_font.as_deref());
    if let Ok((list, children, mut scroll)) = q_scrollback.single_mut() {
        // `reset_session` (a respawned ship) rewinds the scrollback to the welcome
        // rows while the `Local`s still hold the dead session's counts, which left
        // auto-scroll dead for the next ~190 rows.
        let just_spawned = !q_added.is_empty();
        let revision = terminal.scrollback_revision();
        if just_spawned || *last_revision != Some(revision) {
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
            let len = terminal.scrollback().len();
            let previous = if just_spawned { 0 } else { *last_len };
            if len > previous {
                // Request the bottom; `normalize_nova_os_scroll` turns this into
                // the real maximum once layout has measured the new rows.
                scroll.0.y = SCROLL_TO_BOTTOM;
            }
            *last_len = len;
            *last_revision = Some(revision);
        }
    }

    let prompt_color = prompt_color(&terminal);
    for (mut text, mut color) in &mut text_targets.p0() {
        set_text_if_neq(&mut text, prompt_before_cursor(&terminal));
        color.set_if_neq(TextColor(prompt_color));
    }
    for (mut text, mut color) in &mut text_targets.p1() {
        set_text_if_neq(&mut text, prompt_after_cursor(&terminal));
        color.set_if_neq(TextColor(prompt_color));
    }
    for (mut text, mut color) in &mut text_targets.p2() {
        set_text_if_neq(&mut text, prompt_hint_display(&terminal));
        let hint_color = match terminal.parse_status() {
            TerminalParseStatus::Invalid => theme::semantic::THREAT,
            TerminalParseStatus::ValidPrefix => NOVA_OS_PHOSPHOR_MUTED,
            TerminalParseStatus::Empty | TerminalParseStatus::Valid => NOVA_OS_PHOSPHOR_DIM,
        };
        color.set_if_neq(TextColor(hint_color));
    }
    for (mut text, mut color) in &mut text_targets.p3() {
        set_text_if_neq(&mut text, prompt_completion_ghost(&terminal));
        color.set_if_neq(TextColor(NOVA_OS_TEXT.with_alpha(0.34)));
    }
    // Which shell is typing is the one thing the prompt line says out loud, so
    // it is painted from the model rather than baked into the node: switching
    // shells has to change `nova>` to `cmd>` without respawning the strip.
    // The trailing space lives in the layout's column gap.
    for mut text in &mut text_targets.p4() {
        set_text_if_neq(&mut text, terminal.prompt_prefix().trim_end().to_string());
    }
}

/// The "scroll to the bottom" request written by [`rebuild_terminal_ui`]. Bevy
/// clamps `ScrollPosition` during layout but writes the clamped value only into
/// `ComputedNode`, so the request stays in the component until
/// [`normalize_nova_os_scroll`] replaces it with the measured maximum.
const SCROLL_TO_BOTTOM: f32 = f32::MAX;

/// Replace a satisfied [`SCROLL_TO_BOTTOM`] request (and any other overshoot)
/// with the real maximum, now that layout has measured the content. Without this
/// the stored position stays `f32::MAX` for ever - `f32::MAX - page` is still
/// `f32::MAX` - so the first PageUp after a command did nothing. Runs before the
/// keyboard and wheel handlers so they subtract from a real number.
pub(crate) fn normalize_nova_os_scroll(
    mut q_panels: Query<(&mut ScrollPosition, &ComputedNode), With<NovaOsScrollViewportMarker>>,
) {
    for (mut scroll, computed_node) in &mut q_panels {
        let clamped = scroll
            .0
            .y
            .clamp(0.0, nova_ui::screen::max_scroll_y(Some(computed_node)));
        if scroll.0.y != clamped {
            scroll.0.y = clamped;
        }
    }
}

/// `Text` is not `PartialEq`, so `set_if_neq` cannot be used on it directly:
/// compare the string and only then take the `DerefMut`.
fn set_text_if_neq(text: &mut Mut<Text>, next: String) {
    if text.0 != next {
        text.0 = next;
    }
}

/// Blink the terminal caret with a steady on/off cadence, driven by real time
/// so it keeps blinking while the sim is frozen. The caret is a small amber
/// block node, so the blink just toggles its background alpha.
pub(crate) fn blink_nova_os_caret(
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
pub(crate) fn position_nova_os_block_caret(
    q_before: Query<&ComputedNode, With<NovaOsTerminalPromptMarker>>,
    mut q_caret: Query<&mut Node, With<NovaOsTerminalCaretMarker>>,
) {
    let Ok(before) = q_before.single() else {
        return;
    };
    let width = before.size().x * before.inverse_scale_factor();
    let left = Val::Px(width);
    for mut node in &mut q_caret {
        // Runs every frame the computer is open, so an unguarded write would mark
        // the caret's `Node` changed - and re-lay out its parent - every frame.
        if node.left != left {
            node.left = left;
        }
    }
}

/// Refresh the live `FPS: <n>` segment on the NOVA OS topbar each frame while the
/// computer is open. The flight status bar (which normally carries the FPS item)
/// is hidden in `PauseStates::NovaOs`, so this is the only FPS readout on screen
/// then. Runs on the real-time NOVA OS group beside the caret blink because the
/// virtual clock is frozen while the NOVA OS is open.
pub(crate) fn drive_nova_os_topbar_fps(
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
pub(crate) fn spawn_terminal_row(
    parent: &mut ChildSpawnerCommands,
    row: &TerminalRow,
    font: Handle<Font>,
) {
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

pub(crate) fn prompt_color(terminal: &NovaOsTerminal) -> Color {
    match terminal.parse_status() {
        TerminalParseStatus::Invalid => theme::semantic::THREAT,
        TerminalParseStatus::Empty
        | TerminalParseStatus::Valid
        | TerminalParseStatus::ValidPrefix => NOVA_OS_PHOSPHOR,
    }
}

/// Ease [`NovaOsOpenness`] toward the state-driven target (1 open, 0 closed)
/// with REAL time, and map it onto the panel offset, the backdrop alpha and
/// both nodes' visibility. Real time because virtual time is paused while the
/// NOVA OS is open (see the module docs).
pub(crate) fn drive_nova_os_slide(
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
        // Back to whatever the CRT was opened over, not always to flight: `:`
        // from the pause menu leaves the player in the pause menu.
        next.set(close.return_to);
    }
}

/// Hidden once fully closed (so a closed NOVA OS never eats a raycast), visible
/// otherwise.
pub(crate) fn visibility_for(openness: f32) -> Visibility {
    if openness <= f32::EPSILON {
        Visibility::Hidden
    } else {
        Visibility::Visible
    }
}

/// Move `current` toward `target` by at most `step` (a linear approach; the
/// step is a fraction of the full travel per frame).
pub(crate) fn approach(current: f32, target: f32, step: f32) -> f32 {
    if current < target {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    }
}

/// Lift NOVA OS-exempt diagnostic/status chrome above the NOVA OS backdrop only
/// while the NOVA OS is open. Its base z is 0, so when the NOVA OS is closed -
/// including while the pause overlay owns the freeze, which sits at the same z
/// as the NOVA OS backdrop - the exempt chrome stays at the base HUD z and the
/// pause overlay covers it normally.
///
/// The chrome is the HUD's, but the z-band it is lifted into is this monitor's,
/// so the write lives here: the HUD does not know the backdrop's depth.
pub(crate) fn lift_exempt_chrome_over_nova_os(
    pause: Res<State<PauseStates>>,
    mut q_exempt: Query<&mut GlobalZIndex, With<HudNovaOsExempt>>,
) {
    let z = if *pause.get() == PauseStates::NovaOs {
        DRAWER_EXEMPT_Z
    } else {
        0
    };
    for mut zindex in &mut q_exempt {
        zindex.set_if_neq(GlobalZIndex(z));
    }
}

#[cfg(test)]
mod exempt_chrome_tests {
    use bevy::state::app::StatesPlugin;

    use super::*;

    /// The exempt diagnostic/status chrome is lifted above the NOVA OS
    /// backdrop ONLY while the NOVA OS is open. When the PAUSE menu owns the
    /// freeze it drops back to the base HUD z, so the pause overlay (which sits
    /// at the same z as the NOVA OS backdrop) still covers it - not the other
    /// way round. The bug this pins: a static high z made the status strip
    /// poke over the pause menu.
    #[test]
    fn exempt_chrome_lifts_only_while_nova_os_open() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_systems(Update, lift_exempt_chrome_over_nova_os);
        let widget = app
            .world_mut()
            .spawn((HudNovaOsExempt, GlobalZIndex::default()))
            .id();
        let z = |app: &App| app.world().get::<GlobalZIndex>(widget).unwrap().0;

        app.update();
        assert_eq!(z(&app), 0, "base z while unpaused");

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::NovaOs);
        app.update();
        assert_eq!(
            z(&app),
            DRAWER_EXEMPT_Z,
            "lifted above the backdrop while the NOVA OS is open"
        );

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update();
        assert_eq!(
            z(&app),
            0,
            "dropped to base z when the pause menu - not the NOVA OS - owns the freeze"
        );
    }
}
