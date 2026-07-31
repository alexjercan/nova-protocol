use bevy::{prelude::*, ui_widgets::Activate};
use bevy_common_systems::prelude::SoundBank;
use nova_os::prelude::*;
use nova_ui::{font::UiFont, theme};

use super::{casing::*, components::*, content::*, sound::*, style::*};
use crate::{
    audio::{UiSfx, NOVA_OS_COIL_VOLUME},
    PauseStates,
};

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
/// state reads off the bulb flipping (the label is now a fixed "SND" legend).
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
/// Run condition: the objectives list container was spawned this frame, so its
/// initial contents must be built from the current `GameObjectives` even
/// though the resource itself did not change.
pub(crate) fn nova_os_lists_just_spawned(
    q_objectives: Query<(), Added<NovaOsObjectivesListMarker>>,
    q_log: Query<(), Added<NovaOsFlightLogListMarker>>,
) -> bool {
    !q_objectives.is_empty() || !q_log.is_empty()
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

/// Reconcile the persistent header with the active surface: the brand text swaps
/// between the SHELL breadcrumb and the `APPS / <ID>` breadcrumb, and the header
/// close control shows only while an app owns the screen. Keyed on `active_mode`
/// (like [`rebuild_nova_os_footer_hints`]) so ordinary prompt edits do not
/// rewrite the header; forced once when the header is freshly spawned so a reopen
/// starts from the right state.
pub(crate) fn reconcile_nova_os_header(
    terminal: Res<NovaOsTerminal>,
    q_added: Query<(), Added<NovaOsBrandMarker>>,
    mut q_brand: Query<&mut Text, With<NovaOsBrandMarker>>,
    mut q_close: Query<&mut Visibility, With<NovaOsAppCloseMarker>>,
    mut last_mode: Local<Option<TerminalMode>>,
) {
    let mode = terminal.active_mode();
    if q_added.is_empty() && *last_mode == Some(mode) {
        return;
    }
    *last_mode = Some(mode);
    for mut text in &mut q_brand {
        text.0 = nova_os_header_breadcrumb(mode);
    }
    let close_visibility = match mode {
        TerminalMode::App { .. } => Visibility::Inherited,
        TerminalMode::Prompt => Visibility::Hidden,
    };
    for mut visibility in &mut q_close {
        *visibility = close_visibility;
    }
}

/// Rebuild the footer hint row whenever the active surface changes, so the hints
/// swap per surface (terminal vs a running app). Keyed on `active_mode` via a
/// `Local`, so ordinary prompt edits (which change the terminal resource but not
/// the mode) do not thrash the footer. Forced once when the footer is freshly
/// spawned (the `Local` survives a shell teardown/respawn, so without this a
/// respawn whose mode matches the stale `Local` would skip refilling the new
/// footer), mirroring [`reconcile_nova_os_header`].
pub(crate) fn rebuild_nova_os_footer_hints(
    terminal: Res<NovaOsTerminal>,
    registry: Res<NovaOsCommandRegistry>,
    ui_font: Option<Res<UiFont>>,
    mut commands: Commands,
    q_added: Query<(), Added<NovaOsFooterHintsMarker>>,
    q_footer: Query<(Entity, Option<&Children>), With<NovaOsFooterHintsMarker>>,
    mut last_mode: Local<Option<TerminalMode>>,
) {
    if q_added.is_empty() && *last_mode == Some(terminal.active_mode()) {
        return;
    }
    *last_mode = Some(terminal.active_mode());
    let hints = nova_os_footer_hints(terminal.active_mode(), &registry);
    let font = nova_os_font(ui_font.as_deref());
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

pub(crate) fn rebuild_terminal_ui(
    mut commands: Commands,
    terminal: Res<NovaOsTerminal>,
    ui_font: Option<Res<UiFont>>,
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
    // The scrollback length last time we rebuilt, so we only auto-scroll to the
    // bottom when NEW output arrived. The terminal resource changes for many
    // reasons (prompt edits, app-command mirroring, seen-events); pinning to the
    // bottom on every one of those yanked a manual PageUp/wheel scroll straight
    // back down (owner playtest).
    mut last_len: Local<usize>,
) {
    let font = nova_os_font(ui_font.as_deref());
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
        let len = terminal.scrollback().len();
        if len > *last_len {
            scroll.0.y = f32::MAX;
        }
        *last_len = len;
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
    for mut node in &mut q_caret {
        node.left = Val::Px(width);
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
        next.set(PauseStates::Unpaused);
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
