use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    picking::hover::Hovered,
    prelude::*,
};
use bevy_common_systems::prelude::{GameObjectives, SoundBank};
use nova_os::prelude::*;

use super::{components::*, content::*, sound::*, style::*};
use crate::{
    audio::{
        UiSfx, NOVA_OS_BACK_VOLUME, NOVA_OS_COIL_VOLUME, NOVA_OS_ENTER_VOLUME,
        NOVA_OS_ERROR_VOLUME, NOVA_OS_KEY_MIN_INTERVAL, NOVA_OS_KEY_VOLUME, NOVA_OS_OK_VOLUME,
        NOVA_OS_TICK_VOLUME,
    },
    prelude::*,
    PauseStates,
};

/// Tab opens the shared freeze axis and becomes autocomplete while open. The
/// gamepad right-stick click still toggles `Unpaused <-> NovaOs`; both inputs are
/// inert while the pause menu owns the freeze (`Paused`) - which is also how a
/// live outcome (it forces `Paused`) blocks the NOVA OS without a cross-crate
/// dependency. The pad button is `RightThumb`, the one free button, mirroring
/// `nova_menu`'s optional-gamepad guard.
pub(crate) fn toggle_nova_os(
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
pub(crate) fn close_nova_os_from_menu_keys(
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

pub(crate) fn handle_terminal_keyboard(
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
            Option<&SectionCode>,
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
    map_contacts: crate::hud::nova_os_map::MapContacts,
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
                let mut snapshot = terminal_snapshot_from_world(
                    &log,
                    &objectives,
                    ship_name.as_deref(),
                    &sections,
                    terminal.seen_events(),
                );
                // The `map view` CLI rows come from the shared map contact model.
                snapshot = snapshot.with_output(
                    "map view",
                    crate::hud::nova_os_map::terminal_map_rows(&map_contacts),
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

/// Mirror the whole registered command set (core builtins plus registered apps
/// and their subcommands) into the terminal so parsing, completion and `help`
/// treat them uniformly. Reading `command_specs` through the `ResMut` `Deref` does
/// not mark the terminal changed, so once mirrored this early-returns without
/// thrashing `rebuild_terminal_ui`.
pub(crate) fn sync_nova_os_commands(
    registry: Res<NovaOsCommandRegistry>,
    mut terminal: ResMut<NovaOsTerminal>,
) {
    let specs = registry.specs();
    // Compare through the immutable `Deref` so an up-to-date terminal is never
    // marked changed; only a real change takes the `&mut` path below.
    let up_to_date = terminal.command_specs().len() == specs.len()
        && terminal
            .command_specs()
            .iter()
            .map(|command| command.name)
            .eq(specs.iter().map(|command| command.name));
    if up_to_date {
        return;
    }
    terminal.set_commands(specs);
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
pub(crate) fn handle_nova_os_app_keyboard(
    mut keyboard: MessageReader<KeyboardInput>,
    pause: Res<State<PauseStates>>,
    keys: Res<ButtonInput<KeyCode>>,
    registry: Res<NovaOsCommandRegistry>,
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
    let Some(app) = live.and_then(|id| registry.app_runtime(id)) else {
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
pub(crate) fn scroll_nova_os_panels(
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

pub(crate) fn max_nova_os_scroll_y(computed_node: Option<&ComputedNode>) -> f32 {
    computed_node
        .map(|node| (node.content_size.y - node.size.y + node.scrollbar_size.y).max(0.0))
        .unwrap_or(f32::MAX)
}
