//! Every key, wheel and gamepad path into the NOVA OS: the open/close toggle,
//! terminal typing, app-local keys, and panel scrolling.
//!
//! Once the NOVA OS owns the keyboard it must consume the keys gameplay would
//! otherwise act on, so the toggle and the terminal handler are ordered
//! against each other rather than merely both present.
//!
//! Touch this module when adding an input the terminal or an app reacts to.

use bevy::{
    ecs::system::SystemParam,
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    picking::hover::Hovered,
    prelude::*,
};
use nova_gameplay::{
    audio::prelude::{
        SoundBank, UiSfx, NOVA_OS_BACK_VOLUME, NOVA_OS_COIL_VOLUME, NOVA_OS_ENTER_VOLUME,
        NOVA_OS_ERROR_VOLUME, NOVA_OS_KEY_MIN_INTERVAL, NOVA_OS_KEY_VOLUME, NOVA_OS_OK_VOLUME,
        NOVA_OS_TICK_VOLUME,
    },
    objectives::prelude::GameObjectives,
    prelude::*,
    PauseStates,
};
use nova_input::prelude::*;
use nova_os::prelude::*;
use nova_ship::prelude::*;
use nova_ui::screen::{max_scroll_y, page_step};

use super::{components::*, content::*, sound::*, style::*};
use crate::ship::prelude::SectionCode;

/// `novaos_toggle` opens the shared freeze axis, and its key becomes
/// autocomplete while open. The pad half still toggles `Unpaused <-> NovaOs`;
/// both inputs are inert while the pause menu owns the freeze (`Paused`) -
/// which is also how a live outcome (it forces `Paused`) blocks the NOVA OS
/// without a cross-crate dependency.
///
/// The desk and pad halves are read apart, and that asymmetry is the point: a
/// keyboard closes the monitor with Escape, which a pad has not got, so the pad
/// button has to close what it opened.
///
/// OPENING also needs a ship to be the computer OF. `Playing` covers the
/// editor's build mode as well as flight, and there the key used to arm the
/// freeze axis over a scene with no ship: the monitor never drew, but the state
/// stuck, so pressing Play dropped the player straight into a NOVA OS they
/// never asked for. Closing stays ungated - a computer that opened must always
/// be closable.
pub(crate) fn toggle_nova_os(
    sources: InputSources,
    bindings: Res<InputBindings>,
    player: Query<(), With<PlayerSpaceshipMarker>>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    mut close: ResMut<NovaOsCloseTransition>,
) {
    let Some(action) = bindings.get("novaos_toggle") else {
        return;
    };
    let pad = sources.just_pressed_pad(action);
    let desk = sources.just_pressed_desk(action);
    if !desk && !pad {
        return;
    }
    match current.get() {
        PauseStates::Unpaused if player.is_empty() => {}
        PauseStates::Unpaused => {
            close.closing = false;
            next.set(PauseStates::NovaOs);
        }
        PauseStates::NovaOs if pad && !desk => {
            close.closing = true;
        }
        PauseStates::NovaOs | PauseStates::Paused => {}
    }
}
/// Whether either Control key is down. Three NOVA OS keyboard handlers ask this
/// same question - the prompt, the app router and the back-out owner - and they
/// must agree, or one of them types a literal character for a chord another one
/// is about to consume.
pub(crate) fn control_held(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)
}

pub(crate) fn close_nova_os_from_menu_keys(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    current: Res<State<PauseStates>>,
    mut close: ResMut<NovaOsCloseTransition>,
    mut terminal: ResMut<NovaOsTerminal>,
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    settings: Res<NovaOsMonitorSettings>,
    ship: Option<Res<crate::ship::ShipRuntime>>,
) {
    if *current.get() != PauseStates::NovaOs {
        return;
    }
    // This is the ONE owner of the back-out / app-exit gestures (per the
    // `context-key-handled-in-one-owner` lesson): Escape, gamepad Start, and the
    // Ctrl+C / Ctrl+[ app-exit chord are all branched on `active_mode` here, so a
    // single press can never both exit an app and close the NOVA OS.
    // Escape is the one gesture that stays off the registry: it is the
    // universal back-out, and a rebind that stranded a player inside a mode
    // would have no way out. Start is its pad twin, read off the `Gamepad`
    // COMPONENT - bevy 0.19 registers no `ButtonInput<GamepadButton>`, so the
    // `Option<Res<..>>` this used to ask for was `None` on every real run.
    let start = gamepads
        .iter()
        .any(|pad| pad.digital().just_pressed(GamepadButton::Start));
    let escape = keys.just_pressed(KeyCode::Escape) || start;
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let ctrl = control_held(&keys);
    // Ctrl+C / Ctrl+[ exit a running app back to the terminal (PoC's chord).
    let ctrl_exit =
        ctrl && (keys.just_pressed(KeyCode::KeyC) || keys.just_pressed(KeyCode::BracketLeft));
    if escape && ship.is_some_and(|ship| ship.rebinding.is_some()) {
        return;
    }
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

/// Whether a key is the player SAYING something, rather than holding a
/// modifier down on the way to saying it. Only a deliberate key finishes the
/// boot reveal: a Shift pressed before the letter it capitalises is not an
/// instruction to skip anything.
fn is_deliberate(key: &Key) -> bool {
    !matches!(
        key,
        Key::Control
            | Key::Shift
            | Key::Alt
            | Key::Super
            | Key::AltGraph
            | Key::CapsLock
            | Key::NumLock
            | Key::Fn
            | Key::FnLock
            | Key::Meta
            | Key::Hyper
            | Key::Symbol
            | Key::SymbolLock
    )
}

pub(crate) fn handle_terminal_keyboard(
    mut keyboard: MessageReader<KeyboardInput>,
    pause: Res<State<PauseStates>>,
    keys: Res<ButtonInput<KeyCode>>,
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
            Option<&SectionClass>,
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
    map_contacts: crate::map::MapContacts,
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
        // The boot banner is an animation, not a gate. A player who already
        // knows the command they want finishes the reveal by starting to type
        // it - and the key that finished it still does its own job, so the
        // first letter is not eaten. Read through the immutable `Deref` first
        // so an ordinary keystroke does not mark the terminal changed.
        if terminal.has_pending_boot_rows() && is_deliberate(&event.logical_key) {
            terminal.finish_boot();
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
                snapshot =
                    snapshot.with_output("map view", crate::map::terminal_map_rows(&map_contacts));
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
            Key::Home => terminal.move_cursor_to_start(),
            Key::End => terminal.move_cursor_to_end(),
            Key::ArrowUp => terminal.history_previous(),
            Key::ArrowDown => terminal.history_next(),
            // Page the scrollback from the keyboard (PoC's PageUp/PageDown): a
            // cockpit player may never have a hand on the mouse. Clamped like
            // `scroll_nova_os_panels`.
            key @ (Key::PageUp | Key::PageDown) => {
                if let Ok((mut scroll, computed_node)) = q_scrollback.single_mut() {
                    let page = page_step(computed_node);
                    let delta = if matches!(key, Key::PageUp) {
                        -page
                    } else {
                        page
                    };
                    scroll.0.y = (scroll.0.y + delta).clamp(0.0, max_scroll_y(computed_node));
                }
            }
            // A held Control makes this a readline chord rather than a
            // character. The four the prompt answers are keyed on the physical
            // key, not the text: with Control down the produced text is a
            // control character on some platforms and the letter on others.
            //
            // Ctrl+C and Ctrl+[ are NOT here - they are the app-exit chord,
            // owned by `close_nova_os_from_menu_keys`. They still land in the
            // fallthrough, which is the point of the arm: an unanswered chord
            // must not type its letter at the prompt.
            Key::Character(_) | Key::Space if control_held(&keys) => match event.key_code {
                KeyCode::KeyA => terminal.move_cursor_to_start(),
                KeyCode::KeyE => terminal.move_cursor_to_end(),
                KeyCode::KeyU => terminal.kill_to_start(),
                KeyCode::KeyK => terminal.kill_to_end(),
                _ => {}
            },
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
    let ctrl_held = control_held(&keys);
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
        scroll.0.y = (scroll.0.y - dy).clamp(0.0, max_scroll_y(computed_node));
    }
}

/// The input surface a NOVA OS app reads: the activity gate, pointer and action
/// state, and the frame clock.
///
/// F81: `map_input` and `ship_input` each declared this same seven-parameter
/// cluster behind `#[allow(clippy::too_many_arguments)]`. Bundling it also
/// makes the Control guard non-optional - [`Self::pressed`] and
/// [`Self::just_pressed`] withhold every key while Control is down, because
/// Control is the app-exit chord (F34). Reading `ButtonInput` directly is what
/// let Ctrl+`[` both leave the app and cycle its selection.
///
/// The apps name ACTIONS, not keys: the two viewers were the last cluster of
/// hardcoded `KeyCode`s in the tree, so a player could rebind flight but not
/// the computer they fly it with.
#[derive(SystemParam)]
pub(crate) struct NovaOsAppInput<'w, 's> {
    pause: Res<'w, State<PauseStates>>,
    terminal: Res<'w, NovaOsTerminal>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    bindings: Res<'w, InputBindings>,
    sources: InputSources<'w, 's>,
    motion: MessageReader<'w, 's, bevy::input::mouse::MouseMotion>,
    wheel: MessageReader<'w, 's, bevy::input::mouse::MouseWheel>,
    time: Res<'w, Time>,
}

impl NovaOsAppInput<'_, '_> {
    /// Whether the app with this launch word owns the NOVA OS screen right now.
    /// At the prompt the mouse and keys belong to the terminal instead.
    pub(crate) fn app_is_active(&self, app_id: &'static str) -> bool {
        *self.pause.get() == PauseStates::NovaOs
            && self.terminal.active_mode() == TerminalMode::App { id: app_id }
    }

    /// This frame's accumulated mouse motion. Drains the reader.
    pub(crate) fn motion_delta(&mut self) -> Vec2 {
        self.motion.read().map(|m| m.delta).sum()
    }

    /// This frame's accumulated vertical wheel travel. Drains the reader.
    pub(crate) fn wheel_delta(&mut self) -> f32 {
        self.wheel.read().map(|w| w.y).sum()
    }

    /// Frame delta, floored so one stalled frame cannot swing an orbit wildly.
    pub(crate) fn dt(&self) -> f32 {
        self.time.delta_secs().max(1.0 / 240.0)
    }

    /// Mouse buttons are not part of the Control chord, so they pass through.
    pub(crate) fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons.pressed(button)
    }

    /// A held action, withheld while Control owns the chord.
    ///
    /// An unregistered name is simply not pressed. The apps run inside the
    /// monitor, and a missing row is a wiring mistake worth a quiet no-op
    /// rather than a panic in the middle of a frame.
    pub(crate) fn pressed(&self, action: &str) -> bool {
        !control_held(&self.keys)
            && self
                .bindings
                .get(action)
                .is_some_and(|action| self.sources.pressed(action))
    }

    /// An action pressed this frame, withheld while Control owns the chord.
    pub(crate) fn just_pressed(&self, action: &str) -> bool {
        !control_held(&self.keys)
            && self
                .bindings
                .get(action)
                .is_some_and(|action| self.sources.just_pressed(action))
    }
}
