//! Every key, wheel and gamepad path into the command modal and the TAB
//! interface: the one back-out owner, terminal typing, and panel scrolling.
//!
//! Once the modal owns the keyboard it must consume the keys gameplay would
//! otherwise act on, so the back-out owner and the terminal handler are
//! ordered against each other rather than merely both present.
//!
//! Touch this module when adding an input the terminal or a pane reacts to.

use bevy::{
    ecs::system::SystemParam,
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    picking::hover::Hovered,
    prelude::*,
};
use nova_command::prelude::*;
use nova_gameplay::{
    audio::prelude::{
        SoundBank, UiSfx, NOVA_OS_BACK_VOLUME, NOVA_OS_ENTER_VOLUME, NOVA_OS_ERROR_VOLUME,
        NOVA_OS_KEY_MIN_INTERVAL, NOVA_OS_KEY_VOLUME, NOVA_OS_OK_VOLUME, NOVA_OS_TICK_VOLUME,
    },
    PauseStates,
};
use nova_input::prelude::*;
use nova_ui::screen::{drive_wheel_scroll, max_scroll_y, page_step};

use super::{components::*, sound::*};
use crate::pane::InterfacePaneType;

/// Whether either Control key is down. The prompt reads it to tell a readline
/// chord from a typed character.
pub(crate) fn control_held(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)
}

/// The ONE owner of the back-out gesture for both surfaces (per the
/// `context-key-handled-in-one-owner` lesson), so one press never closes two
/// layers. Precedence: an armed rebind capture keeps the key, then the modal
/// closes to the state it covered, then the pane closes to flight. The pause
/// menu's own toggle ignores both surfaces, so it comes last.
///
/// Escape is the one gesture that stays off the registry: it is the universal
/// back-out, and a rebind that stranded a player inside a surface would have no
/// way out. Start is its pad twin, read off the `Gamepad` COMPONENT - bevy 0.19
/// registers no `ButtonInput<GamepadButton>`.
pub(crate) fn close_surface_from_menu_keys(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    mut close: ResMut<NovaOsCloseTransition>,
    ship: Option<Res<crate::ship::ShipRuntime>>,
) {
    let start = gamepads
        .iter()
        .any(|pad| pad.digital().just_pressed(GamepadButton::Start));
    if !(keys.just_pressed(KeyCode::Escape) || start) {
        return;
    }
    if ship.is_some_and(|ship| ship.rebind_armed()) {
        return;
    }
    match current.get() {
        PauseStates::Commands => close.closing = true,
        PauseStates::Interface => next.set(PauseStates::Unpaused),
        PauseStates::Unpaused | PauseStates::Paused => {}
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
    mut terminal: ResMut<CommandTerminal>,
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
    let modal_open = *pause.get() == PauseStates::Commands;
    // The `bank` is absent on rigs without the sound assets (headless), so each
    // branch guards on it and cues are a no-op there.
    let now = time.elapsed_secs();
    for event in keyboard.read() {
        if !modal_open {
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
                let outcome = terminal.submit();
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
                        // The command dispatcher runs between this system and
                        // the paint, and it cues its own answer: an ok here
                        // would fire before the command had one.
                        TerminalSubmitOutcome::Dispatched => (None, 0.0),
                        TerminalSubmitOutcome::Ran => (Some(UiSfx::NovaOsOk), NOVA_OS_OK_VOLUME),
                        TerminalSubmitOutcome::Errored => {
                            (Some(UiSfx::NovaOsError), NOVA_OS_ERROR_VOLUME)
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
            // An unanswered chord lands in the fallthrough and must not type
            // its letter at the prompt.
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

    // The `close` command requests the same animated close as Esc/Start.
    // Peeked first: taking through `ResMut` on an idle frame would mark the
    // terminal changed and defeat every paint gate that keys on that.
    if terminal.has_pending_close() {
        terminal.take_pending_close();
        close.closing = true;
    }
}

/// Wheel-scroll the NOVA OS panels through the shared driver, so one notch
/// moves the drawer exactly as far as it moves a menu pane.
///
/// A registration of the drawer's own rather than the shared
/// `nova_ui::screen::ScrollViewport` marker, because the gates differ: the
/// drawer answers the wheel only while the monitor owns the screen, and only
/// after `mirror_nova_os_hover` has copied the sampled image's hover onto these
/// nodes. The arithmetic behind it is the shared one.
pub(crate) fn scroll_nova_os_panels(
    wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    q_panels: Query<
        (&mut ScrollPosition, Option<&ComputedNode>, Option<&Hovered>),
        With<NovaOsScrollViewportMarker>,
    >,
) {
    drive_wheel_scroll(wheel, q_panels);
}

/// The input surface a TAB pane's viewer reads: the activity gate, pointer and
/// action state, and the frame clock.
///
/// The panes name ACTIONS, not keys, so a player can rebind the viewer like
/// flight.
#[derive(SystemParam)]
pub(crate) struct NovaOsAppInput<'w, 's> {
    pause: Res<'w, State<PauseStates>>,
    pane: Res<'w, InterfacePaneType>,
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    bindings: Res<'w, InputBindings>,
    sources: InputSources<'w, 's>,
    motion: MessageReader<'w, 's, bevy::input::mouse::MouseMotion>,
    wheel: MessageReader<'w, 's, bevy::input::mouse::MouseWheel>,
    time: Res<'w, Time>,
}

impl NovaOsAppInput<'_, '_> {
    /// Whether this pane owns the interface right now. Under the command modal
    /// the keys belong to the prompt instead.
    pub(crate) fn app_is_active(&self, pane: InterfacePaneType) -> bool {
        *self.pause.get() == PauseStates::Interface && *self.pane == pane
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

    /// Whether a mouse button is held.
    pub(crate) fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons.pressed(button)
    }

    /// A held action.
    ///
    /// An unregistered name is simply not pressed. A missing row is a wiring
    /// mistake worth a quiet no-op rather than a panic in the middle of a frame.
    pub(crate) fn pressed(&self, action: &str) -> bool {
        self.bindings
            .get(action)
            .is_some_and(|action| self.sources.pressed(action))
    }

    /// An action pressed this frame.
    pub(crate) fn just_pressed(&self, action: &str) -> bool {
        self.bindings
            .get(action)
            .is_some_and(|action| self.sources.just_pressed(action))
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        ecs::system::RunSystemOnce,
        input::{
            mouse::{MouseScrollUnit, MouseWheel},
            touch::TouchPhase,
        },
    };
    use nova_ui::screen::{scroll_viewports, ScrollViewport};

    use super::*;

    /// One wheel notch moves the NOVA OS drawer as far as it moves any other
    /// scrolling pane.
    ///
    /// The drawer carried its own wheel driver off its own step constant and
    /// travelled a third of the distance the rest of the game does, so the same
    /// gesture read as a different control depending on which surface was under
    /// the pointer. Comparing the two drivers side by side is the assertion: a
    /// number copied into this test would drift the same way the constant did.
    #[test]
    fn a_wheel_notch_moves_the_drawer_as_far_as_any_other_pane() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().init_resource::<Messages<MouseWheel>>();
        let drawer = app
            .world_mut()
            .spawn((NovaOsScrollViewportMarker, ScrollPosition(Vec2::ZERO)))
            .id();
        let pane = app
            .world_mut()
            .spawn((ScrollViewport, ScrollPosition(Vec2::ZERO)))
            .id();
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });

        // Each reader keeps its own cursor, so both drivers see the one notch.
        app.world_mut()
            .run_system_once(scroll_nova_os_panels)
            .expect("the drawer driver runs");
        app.world_mut()
            .run_system_once(scroll_viewports)
            .expect("the shared driver runs");

        let travelled = |entity: Entity| {
            app.world()
                .entity(entity)
                .get::<ScrollPosition>()
                .expect("a scroll position")
                .0
                .y
        };
        assert!(
            travelled(pane) > 0.0,
            "a notch has to move the shared pane, or this proves nothing"
        );
        assert_eq!(
            travelled(drawer),
            travelled(pane),
            "the drawer scrolls by the shared step, not one of its own"
        );
    }
}
