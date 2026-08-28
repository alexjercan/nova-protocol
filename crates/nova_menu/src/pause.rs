//! The pause overlay: ESC freezes the sim and raises a modal panel with
//! Resume / Retry / Settings / Back to Main Menu / Exit.

use avian3d::prelude::{Physics, PhysicsTime};
use bevy::{
    prelude::*,
    ui_widgets::{observe, Activate},
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ui::{
    prelude::UiSkin,
    screen::{scroll_bar, scroll_column, scroll_viewport},
    theme,
    widget::{panel, ButtonVariant, UiText},
};

#[cfg(not(target_arch = "wasm32"))]
use crate::menu_ui::on_exit;
use crate::{
    settings::{build_settings_tabs, PauseSettingsPanel, SettingsActiveTab, SettingsTabBody},
    widgets::{button, button_variant},
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
            commands.play_sfx_volume(bank.get(UiSfx::UiToggle), UI_TOGGLE_VOLUME);
        }
    }
}

/// Freeze the simulation: virtual time (Update deltas + FixedUpdate
/// accumulation, which physics follows) and avian's own physics clock, so
/// nothing integrates regardless of which clock a system reads.
pub(crate) fn pause_clocks(
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
) {
    virtual_time.pause();
    physics_time.pause();
}

/// Unconditional: the pause menu is currently the only clock-pauser in the
/// app. A future cutscene/debug freeze that also pauses these clocks will be
/// stomped here and needs a coordination story first (review R1.6).
pub(crate) fn unpause_clocks(
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
) {
    virtual_time.unpause();
    physics_time.unpause();
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
pub(crate) fn force_unpause(
    mut next: ResMut<NextState<PauseStates>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
) {
    next.set(PauseStates::Unpaused);
    virtual_time.unpause();
    physics_time.unpause();
}

/// The pause overlay: a dim full-screen layer with a centered panel.
/// `CurrentScenario` is optional for the same reason it is in the loader's
/// consumers: headless menu rigs run without the scenario loader.
pub(crate) fn setup_pause_ui(
    mut commands: Commands,
    current: Option<Res<CurrentScenario>>,
    skin: Res<UiSkin>,
    active_settings_tab: Res<SettingsActiveTab>,
    outcome: Option<Res<CurrentOutcome>>,
) {
    // The outcome frame also enters `Paused` (`sync_outcome_pause`) to freeze the sim,
    // but it is its own modal with its own buttons: do not stack the pause panel (or
    // its Settings modal) underneath it. The ESC toggle is already inert here, so this
    // only fires on the outcome-driven pause.
    if outcome.is_some_and(|outcome| outcome.0.is_some()) {
        return;
    }
    // Retry only makes sense over a live scenario. The editor's build mode
    // pauses through this same overlay but never has one loaded, so it gets
    // no dead button.
    let live = current.is_some_and(|current| current.is_some());
    commands
        .spawn((
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
            GlobalZIndex(10),
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
    // overlay. Above the pause overlay (GlobalZIndex(10)) and a modal blocker so
    // the pause buttons underneath cannot receive clicks through it.
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
            GlobalZIndex(11),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Pause Settings Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        // Wide enough for a keybind label plus its two chip
                        // columns, and a FIXED height so the panel does not
                        // resize under the pointer when a tab with fewer rows
                        // opens. The cap keeps it on screen on a short display.
                        width: px(620),
                        height: px(560),
                        max_height: percent(92),
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
                        button("Back"),
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
