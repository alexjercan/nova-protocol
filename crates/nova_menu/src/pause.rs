//! The pause overlay: ESC freezes the sim and raises a modal panel with
//! Resume / Retry / Settings / Back to Main Menu / Exit.
//!
//! ESC is not the only way in. Losing the window pauses interactive play too
//! ([`pause_on_focus_loss`]), and the panel itself is RECONCILED
//! ([`reconcile_pause_overlay`]) rather than spawned on the way in, because
//! `Paused` is a freeze axis the outcome frame and the refusal report share -
//! each holds it while drawing a modal of its own, and either can arrive or
//! clear without the state changing.

use bevy::{
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
    ui_widgets::{observe, Activate},
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_gameplay::prelude::*;
use nova_os::prelude::{NovaOsTerminal, ShellKind};
use nova_os_ui::prelude::NovaOsCloseTransition;
use nova_scenario::prelude::*;
use nova_ui::{
    prelude::{UiSkin, PAUSE_SETTINGS_Z, PAUSE_Z},
    screen::{scroll_bar, scroll_column, scroll_viewport},
    theme,
    widget::{panel, ButtonVariant, UiText},
};

#[cfg(not(target_arch = "wasm32"))]
use crate::menu_ui::on_exit;
use crate::{
    settings::{
        build_settings_tabs, PauseSettingsPanel, SettingsActiveTab, SettingsTabBody,
        SETTINGS_PANEL_H, SETTINGS_PANEL_MAX_H_PCT, SETTINGS_PANEL_MAX_W, SETTINGS_PANEL_WIDTH_PCT,
    },
    widgets::{back_button, button, button_variant},
};

/// ESC (or the gamepad Start button) toggles the pause overlay. Plain
/// press-to-toggle.
///
/// Escape is deliberately NOT a registry action: it is the universal back-out,
/// and a rebind that stranded a player inside a mode would leave no way out.
/// The settings list carries it as a fixed row instead.
pub(crate) fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    escape_owner: Option<Res<EscapeOwner>>,
    rebind: Option<Res<crate::settings::PendingRebind>>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    outcome: Option<Res<CurrentOutcome>>,
    failure: Option<Res<ScenarioStartFailure>>,
    mut commands: Commands,
) {
    // A scene surface that owns Escape this frame answers it as BACK - leaving
    // the editor's parts gallery, putting down an armed part - and the pause
    // overlay must not stack on top of that answer. Optional so a headless rig
    // without the gameplay plugin keeps the plain toggle.
    if escape_owner.is_some_and(|owner| owner.0) {
        return;
    }
    // A chip waiting for a key answers Escape itself, as the cancel. Both
    // systems read the same edge in the same schedule, so without this rung one
    // press would cancel the capture AND close the overlay the capture was
    // shown in - losing the screen the player was working on. The next press
    // closes it, because the chip is no longer armed.
    if rebind.is_some_and(|rebind| rebind.is_armed()) {
        return;
    }
    // A shown outcome frame is its own paused modal (`sync_outcome_pause` holds the app
    // in `Paused` while `CurrentOutcome` is set), with its own Continue/Retry/Main Menu
    // buttons: ESC/Start must not toggle here, or it would either resume the sim behind
    // the still-open overlay or stack the pause panel over it.
    if outcome.is_some_and(|outcome| outcome.0.is_some()) {
        return;
    }
    // The refusal report is the same shape of modal and a harder dead end: the
    // scenario it interrupted is torn down, so a Resume button over it would
    // resume nothing and a Retry would reload a scenario that no longer exists.
    // Main Menu, on the report itself, is the way out.
    if failure.is_some_and(|failure| failure.0.is_some()) {
        return;
    }
    // Off the `Gamepad` COMPONENT: bevy 0.19 registers no
    // `ButtonInput<GamepadButton>` resource, so the `Option<Res<..>>` this used
    // to ask for was `None` on every real run and Start never paused anything.
    let pad = gamepads
        .iter()
        .any(|pad| pad.digital().just_pressed(GamepadButton::Start));
    if keys.just_pressed(KeyCode::Escape) || pad {
        let destination = match current.get() {
            PauseStates::Unpaused => PauseStates::Paused,
            PauseStates::Paused => PauseStates::Unpaused,
            // The Tab NOVA OS owns its close animation. ESC/Start while NovaOs
            // is active is handled by nova_gameplay so clocks stay paused until
            // the NOVA OS has slid fully off screen.
            PauseStates::NovaOs => PauseStates::NovaOs,
        };
        if destination == *current.get() {
            return;
        }
        next.set(destination);
        // The overlay open/close toggle: a soft UI blip on both directions. The
        // Resume/Exit buttons close it with their own MenuSelect click, so only the
        // ESC/pad toggle needs this.
        if let Some(bank) = bank {
            commands.play_sfx(
                bank.get(UiSfx::UiToggle),
                AudioRoute::Interface,
                UI_TOGGLE_VOLUME,
            );
        }
    }
}

/// `:` opens the game command shell over whatever is on screen.
///
/// It lives beside [`toggle_pause`] rather than with the CRT because it is a
/// GLOBAL modal gesture, and this crate is the one that already owns those: it
/// can see the rebind capture that must swallow the key, and it runs over the
/// main menu, the editor and flight alike.
///
/// Read as a logical character, not a key code, so a layout where `:` is not
/// Shift+Semicolon still opens the shell with the key that prints one.
pub(crate) fn open_command_shell(
    mut keyboard: MessageReader<KeyboardInput>,
    game_state: Option<Res<State<GameStates>>>,
    current: Res<State<PauseStates>>,
    rebind: Option<Res<crate::settings::PendingRebind>>,
    mut next: ResMut<NextState<PauseStates>>,
    mut close: ResMut<NovaOsCloseTransition>,
    mut terminal: ResMut<NovaOsTerminal>,
) {
    let typed_colon = keyboard
        .read()
        .any(|event| event.state == ButtonState::Pressed && event.text.as_deref() == Some(":"));
    if !typed_colon {
        return;
    }
    // Inside the CRT the key is text: the shell is already open and the player
    // is typing into it. Same for a chip waiting to capture a key.
    if *current.get() == PauseStates::NovaOs {
        return;
    }
    if rebind.is_some_and(|rebind| rebind.is_armed()) {
        return;
    }
    // A half-loaded world has nothing to inspect and no settings surface to
    // return to.
    if game_state.is_some_and(|state| *state.get() == GameStates::Loading) {
        return;
    }
    // The ground floor: Escape closes the computer rather than climbing into a
    // NOVA OS session the player never opened.
    terminal.open_shell(ShellKind::Commands);
    close.closing = false;
    // The shell is a surface OVER what is already there, so closing it puts
    // that back: `:` from the pause menu returns to the pause menu, not to a
    // running world the player never asked to be in.
    close.return_to = *current.get();
    next.set(PauseStates::NovaOs);
}

/// Whether losing the window pauses interactive play in this process.
///
/// Off for a scripted run. A probe drives an X display nobody is looking at and
/// a `--norender` run has no window at all, so a focus pause would freeze every
/// harnessed walk on its first frame and every one of them would stall. Read
/// once at build (`harness_env_active`) rather than per frame, and a resource
/// rather than a constant so a range can state which side of the policy it is
/// proving.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusPause(pub bool);

impl Default for FocusPause {
    fn default() -> Self {
        Self(!nova_gameplay::prelude::harness_env_active())
    }
}

/// Whether an interactive run should be holding a focus pause right now.
///
/// The WINDOW is asked, not a focus event: a run that enters `Playing` already
/// unfocused never sees an edge, and the player who alt-tabbed during the load
/// is owed the same pause as the one who alt-tabbed during flight. No window
/// (a headless rig, `--norender`) reads as focused - there is nothing to lose.
pub(crate) fn focus_lost(policy: Option<&FocusPause>, window: Option<&Window>) -> bool {
    policy.is_some_and(|policy| policy.0) && window.is_some_and(|window| !window.focused)
}

/// Pause interactive play when the window goes away.
///
/// The pause is the ORDINARY one: the same panel, the same freeze, the same
/// Resume. Coming back never resumes by itself - the player left the game
/// running by accident once, and the fix is not to drop them back into a fight
/// the moment they click the taskbar.
///
/// Only an unpaused run acquires it. A pause already held stays held whatever
/// owns it, so an open NOVA OS remains the active modal and an outcome frame
/// remains the one modal over its own pause.
pub(crate) fn pause_on_focus_loss(
    policy: Option<Res<FocusPause>>,
    pause: Res<State<PauseStates>>,
    scenario: Option<Res<CurrentScenario>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut next: ResMut<NextState<PauseStates>>,
) {
    if *pause.get() != PauseStates::Unpaused {
        return;
    }
    // A live scenario is what makes the run interactive gameplay. The editor's
    // build mode is a workbench: alt-tabbing to a reference image and coming
    // back to a pause panel over the parts gallery helps nobody.
    if !scenario.is_some_and(|scenario| scenario.is_some()) {
        return;
    }
    if !focus_lost(policy.as_deref(), q_window.iter().next()) {
        return;
    }
    next.set(PauseStates::Paused);
}

/// Freeze the simulation for the pause overlay: virtual time (Update deltas +
/// FixedUpdate accumulation, which physics follows) and avian's own physics
/// clock, so nothing integrates regardless of which clock a system reads.
///
/// The hold is named rather than unconditional. The CRT terminal freezes the
/// same clocks, and the two surfaces overlap: `:` opens the command shell over
/// a paused game, and an unconditional unpause on its close handed a paused
/// player a running world.
pub(crate) fn hold_clocks_for_pause_menu(mut clocks: Clocks) {
    clocks.hold(FreezeOwner::PauseMenu);
}

/// Drop the pause overlay's hold. The world runs again only if the terminal is
/// not also holding it.
pub(crate) fn release_clocks_for_pause_menu(mut clocks: Clocks) {
    clocks.release(FreezeOwner::PauseMenu);
}

/// Freeze the simulation for the CRT terminal, in either shell. Switching
/// shells never passes through this, so the world does not tick between the
/// release and the re-hold it would otherwise need.
///
/// GAMEPLAY only. `:` opens the command shell over the main menu as well, and
/// what runs behind it there is the menu's own ambience backdrop - a cinematic
/// the shell is drawn over, not a game the player is being kept out of.
/// Stopping it would leave the front door on a still frame for as long as the
/// shell is up. `Playing` is the whole of what the terminal freezes, the
/// editor's build mode included.
pub(crate) fn hold_clocks_for_terminal(game: Option<Res<State<GameStates>>>, mut clocks: Clocks) {
    if game.is_some_and(|game| *game.get() != GameStates::Playing) {
        return;
    }
    clocks.hold(FreezeOwner::Terminal);
}

/// Drop the terminal's hold.
pub(crate) fn release_clocks_for_terminal(mut clocks: Clocks) {
    clocks.release(FreezeOwner::Terminal);
}

/// The scenario locks and hides the cursor (nova_editor's grab systems); the
/// overlay needs it back to be clickable.
pub(crate) fn release_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

/// Keep frozen interactive surfaces in control if a later flight reconciler
/// tries to reclaim the pointer after the state-transition hook ran.
pub(crate) fn keep_frozen_cursor_released(
    pause: Res<State<PauseStates>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if pause.get().is_frozen() && (cursor.grab_mode != CursorGrabMode::None || !cursor.visible) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// Re-grab on resume, but only during scenario play: a live player ship is what
/// distinguishes it (PlayerSpaceshipMarker is only inserted by the scenario spawn path;
/// the editor's build-mode preview never carries it). Grabs unconditionally now, debug
/// builds included; the F11 inspector reclaims the cursor while it is up (nova_debug's
/// `sync_inspector_cursor`).
pub(crate) fn restore_cursor(
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    game_state: Res<State<GameStates>>,
    outcome: Option<Res<CurrentOutcome>>,
) {
    // The Back path exits Paused and Playing in the same transition batch
    // (GameStates applies first, it is init'd first): never re-grab when the
    // destination is the menu (review R1.4).
    if *game_state.get() != GameStates::Playing {
        return;
    }
    // A live outcome overlay owns the cursor (outcome review R1.1): on Victory the ship
    // survives, so without this guard exiting Paused with the overlay still up would
    // re-lock the mouse and strand its buttons - sync_outcome_cursor only frees on
    // outcome CHANGE. The outcome now drives the pause itself and ESC is inert over
    // it, so this is a defensive guard rather than the normal path.
    if outcome.is_some_and(|outcome| outcome.0.is_some()) {
        return;
    }
    if !q_player.is_empty() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Safety net for the Back to Main Menu path (and any future exit from
/// Playing while paused): reset the pause state and clocks.
pub(crate) fn force_unpause(mut next: ResMut<NextState<PauseStates>>, mut clocks: Clocks) {
    next.set(PauseStates::Unpaused);
    // Every surface that took a hold is being torn down with the scene, so
    // none of them is left to release its own.
    clocks.release_all();
}

/// Marker for the pause overlay root, so the panel can be counted and
/// reconciled rather than looked up by name.
#[derive(Component)]
pub(crate) struct PauseOverlay;

/// The pause overlay: a dim full-screen layer with a centered panel.
///
/// Reconciled every frame rather than spawned at `OnEnter(Paused)`, because the
/// pause STATE is shared with the modals that take the screen from it. An
/// outcome frame and the FAILED TO START report both hold `Paused` and draw a
/// surface of their own, and either can arrive or clear while the state itself
/// never changes - so a panel built on the way in could neither be taken away
/// when an outcome landed behind it nor handed back when that outcome cleared
/// into a pause the player still owns (the unfocused auto-advance).
///
/// It reads the pause the app is about to be in, not the one it is in: the
/// decisions that open and close the menu are made in `Update` and apply next
/// frame, and a panel built from the state before the transition would flash
/// for a frame on the way out.
///
/// `CurrentScenario` is optional for the same reason it is in the loader's
/// consumers: headless menu rigs run without the scenario loader.
pub(crate) fn reconcile_pause_overlay(
    mut commands: Commands,
    pause: Res<State<PauseStates>>,
    next_pause: Res<NextState<PauseStates>>,
    scenario: Option<Res<CurrentScenario>>,
    skin: Res<UiSkin>,
    active_settings_tab: Res<SettingsActiveTab>,
    outcome: Option<Res<CurrentOutcome>>,
    failure: Option<Res<ScenarioStartFailure>>,
    q_overlay: Query<Entity, With<PauseOverlay>>,
    q_owned: Query<Entity, Or<(With<PauseOverlay>, With<PauseSettingsPanel>)>>,
) {
    let pausing = match next_pause.as_ref() {
        NextState::Pending(pending) | NextState::PendingIfNeq(pending) => *pending,
        NextState::Unchanged => *pause.get(),
    };
    // The outcome frame and the refusal report enter the same `Paused` to
    // freeze the sim, but each is its own modal with its own buttons: the pause
    // panel (and its Settings modal) may not stack under either. Exactly one
    // modal, whichever arrived last.
    let taken = outcome.is_some_and(|outcome| outcome.0.is_some())
        || failure.is_some_and(|failure| failure.0.is_some());
    let wanted = pausing == PauseStates::Paused && !taken;
    if wanted == !q_overlay.is_empty() {
        return;
    }
    if !wanted {
        for entity in q_owned.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }
    // Retry only makes sense over a live scenario. The editor's build mode
    // pauses through this same overlay but never has one loaded, so it gets
    // no dead button.
    let live = scenario.is_some_and(|scenario| scenario.is_some());
    commands
        .spawn((
            PauseOverlay,
            DespawnOnExit(PauseStates::Paused),
            Name::new("Pause Overlay"),
            // A modal blocker, unlike the main menu root: the editor's
            // buttons and section-picking live beneath this overlay and must
            // not receive clicks through it (review R1.2).
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            // Above the HUD chrome.
            GlobalZIndex(PAUSE_Z),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Pause Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        width: px(280),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    panel(*skin),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Pause Title"),
                        UiText,
                        Text::new("Paused"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(theme::SCREEN_TEXT),
                    ));
                    parent.spawn((
                        Name::new("Resume Button"),
                        button_variant("Resume", ButtonVariant::Primary, Some("Esc")),
                        observe(on_resume),
                    ));
                    if live {
                        parent.spawn((
                            Name::new("Pause Retry Button"),
                            button("Retry"),
                            observe(on_retry),
                        ));
                    }
                    parent.spawn((
                        Name::new("Pause Settings Button"),
                        button("Settings"),
                        observe(on_pause_settings),
                    ));
                    parent.spawn((
                        Name::new("Back To Menu Button"),
                        button("Back to Main Menu"),
                        observe(on_back_to_menu),
                    ));
                    // No process to quit on wasm; the browser tab owns the
                    // lifecycle (same rule as the main menu's Exit).
                    #[cfg(not(target_arch = "wasm32"))]
                    parent.spawn((
                        Name::new("Pause Exit Button"),
                        button_variant("Exit", ButtonVariant::Danger, None),
                        observe(on_exit),
                    ));
                });
        });

    // The pause Settings modal: the SAME shared body as the main menu, hidden
    // until the pause Settings button toggles it, and despawned with the pause
    // overlay. Above the pause overlay and a modal blocker so the pause buttons
    // underneath cannot receive clicks through it.
    commands
        .spawn((
            DespawnOnExit(PauseStates::Paused),
            Name::new("Pause Settings Panel Root"),
            PauseSettingsPanel,
            Visibility::Hidden,
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GlobalZIndex(PAUSE_SETTINGS_Z),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Pause Settings Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        // A share of the window up to its own maximum, and a
                        // FIXED height so the panel does not resize under the
                        // pointer when a tab with fewer rows opens. Both caps
                        // keep it on screen on a small display; see
                        // `SETTINGS_PANEL_WIDTH_PCT`.
                        width: percent(SETTINGS_PANEL_WIDTH_PCT),
                        max_width: px(SETTINGS_PANEL_MAX_W),
                        height: px(SETTINGS_PANEL_H),
                        max_height: percent(SETTINGS_PANEL_MAX_H_PCT),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    panel(*skin),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Pause Settings Title"),
                        UiText,
                        Text::new("Settings"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(theme::SCREEN_TEXT),
                        Node {
                            margin: UiRect::bottom(px(12)),
                            ..default()
                        },
                    ));
                    build_settings_tabs(parent, *skin, active_settings_tab.0);
                    parent
                        .spawn((
                            Name::new("Pause Settings Body Row"),
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Stretch,
                                // Takes the panel's leftover height, so the
                                // scroll viewport clips instead of the fixed
                                // panel overflowing past the Back button.
                                flex_grow: 1.0,
                                min_height: px(0),
                                ..default()
                            },
                        ))
                        .with_children(|row| {
                            // Empty, like the main menu's: `refresh_settings_tab`
                            // is the one path that fills either.
                            row.spawn((
                                Name::new("Pause Settings Body"),
                                SettingsTabBody,
                                scroll_column(),
                                scroll_viewport(),
                            ));
                            row.spawn((Name::new("Pause Settings Scroll Bar"), scroll_bar(*skin)));
                        });
                    parent.spawn((
                        Name::new("Pause Settings Back Button"),
                        back_button("Back"),
                        observe(on_pause_settings_back),
                    ));
                });
        });
}

/// Toggle the pause Settings modal open/closed.
pub(crate) fn on_pause_settings(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<PauseSettingsPanel>>,
) {
    **panel = match **panel {
        Visibility::Hidden => Visibility::Visible,
        _ => Visibility::Hidden,
    };
}

/// Close the pause Settings modal, back to the pause overlay.
pub(crate) fn on_pause_settings_back(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<PauseSettingsPanel>>,
) {
    **panel = Visibility::Hidden;
}

pub(crate) fn on_resume(_activate: On<Activate>, mut next: ResMut<NextState<PauseStates>>) {
    next.set(PauseStates::Unpaused);
}

/// The pause overlay's Retry: restart the running scenario from scratch by
/// re-triggering [`LoadScenario`] with the live config - the same
/// teardown-then-spawn path every load takes, so the event world (including
/// any lingering `NextScenario`), a declared outcome, and every scoped entity
/// reset exactly like on a scenario switch. Unpauses in the same activation;
/// the cursor re-grab rides the new player ship's spawn
/// (`regrab_cursor_on_player_spawn`), as for the outcome overlay's Retry.
pub(crate) fn on_retry(
    _activate: On<Activate>,
    current: Option<Res<CurrentScenario>>,
    mut pause: ResMut<NextState<PauseStates>>,
    mut commands: Commands,
) {
    // The button only spawns over a live scenario (setup_pause_ui), but the
    // scenario could in principle die between spawn and click: stay a no-op
    // rather than reload a stale config.
    let Some(scenario) = current.and_then(|current| current.0.clone()) else {
        return;
    };
    commands.trigger(LoadScenario(scenario));
    pause.set(PauseStates::Unpaused);
}

/// Back out to the front door. Unpauses in the same transition batch (a
/// force_unpause on OnExit(Playing) alone would apply one frame late,
/// leaving the overlay over the menu for a frame - review R1.4); entering
/// MainMenu loads the ambience backdrop (tearing the gameplay scenario down)
/// and the editor resets its own inner state on OnExit(Playing).
pub(crate) fn on_back_to_menu(
    _activate: On<Activate>,
    mut state: ResMut<NextState<GameStates>>,
    mut pause: ResMut<NextState<PauseStates>>,
) {
    state.set(GameStates::MainMenu);
    pause.set(PauseStates::Unpaused);
}
