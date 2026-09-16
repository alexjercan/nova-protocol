use avian3d::prelude::{Physics, PhysicsTime};
use bevy::{
    ui_widgets::Activate,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_ui::{skin::UiSkin, theme, widget::prelude::*};

use super::*;

#[derive(Component, Clone, Copy)]
enum PauseAction {
    Resume,
    Restart,
    Lobby,
    Quit,
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, toggle_pause);
    app.add_systems(
        OnEnter(PauseStates::Paused),
        (pause_clocks, release_cursor, spawn_pause),
    );
    app.add_systems(
        OnExit(PauseStates::Paused),
        (unpause_clocks, restore_cursor),
    );
    app.add_observer(on_pause_action);
}

fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    escape_owner: Option<Res<EscapeOwner>>,
    game: Res<State<GameStates>>,
    pause: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    lobby: Query<(), With<lobby::LobbyRoot>>,
    flow: Res<result::MatchFlow>,
) {
    if !keys.just_pressed(KeyCode::Escape)
        || escape_owner.is_some_and(|owner| owner.0)
        || *game.get() != GameStates::Playing
        || !lobby.is_empty()
        || !flow.can_pause()
    {
        return;
    }
    match pause.get() {
        PauseStates::Unpaused => next.set(PauseStates::Paused),
        PauseStates::Paused => next.set(PauseStates::Unpaused),
        PauseStates::NovaOs => {}
    }
}

fn pause_clocks(mut virtual_time: ResMut<Time<Virtual>>, mut physics_time: ResMut<Time<Physics>>) {
    virtual_time.pause();
    physics_time.pause();
}

fn unpause_clocks(
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
) {
    virtual_time.unpause();
    physics_time.unpause();
}

fn release_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

fn restore_cursor(
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    players: Query<(), With<PlayerSpaceshipMarker>>,
    lobby: Query<(), With<lobby::LobbyRoot>>,
) {
    if lobby.is_empty() && !players.is_empty() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

fn pause_button(label: &str, action: PauseAction, primary: bool) -> impl Bundle {
    let spec = if primary {
        ButtonSpec::new(label).primary()
    } else {
        ButtonSpec::new(label)
    };
    (button(spec.block()), action)
}

fn spawn_pause(mut commands: Commands, skin: Res<UiSkin>) {
    commands
        .spawn((
            DespawnOnExit(PauseStates::Paused),
            Name::new("WFC Arena Pause"),
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
            GlobalZIndex(20),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(360),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(px(22)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        row_gap: px(7),
                        ..default()
                    },
                    nova_ui::widget::panel(*skin),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        UiText,
                        Text::new("MATCH PAUSED"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(theme::SCREEN_TEXT),
                        Node {
                            margin: UiRect::bottom(px(10)),
                            ..default()
                        },
                    ));
                    panel.spawn(pause_button("RESUME", PauseAction::Resume, true));
                    panel.spawn(pause_button("RESTART MATCH", PauseAction::Restart, false));
                    panel.spawn(pause_button("RETURN TO LOBBY", PauseAction::Lobby, false));
                    panel.spawn(pause_button("QUIT", PauseAction::Quit, false));
                });
        });
}

#[expect(
    clippy::too_many_arguments,
    reason = "one system over the whole pause screen"
)]
fn on_pause_action(
    activate: On<Activate>,
    mut commands: Commands,
    actions: Query<&PauseAction>,
    mut next: ResMut<NextState<PauseStates>>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    skin: Res<UiSkin>,
    model: Option<Res<lobby::LobbyModel>>,
    mut roster: ResMut<Roster>,
) {
    let Ok(action) = actions.get(activate.event_target()) else {
        return;
    };
    match action {
        PauseAction::Resume => next.set(PauseStates::Unpaused),
        PauseAction::Restart => {
            commands.trigger(LoadScenario(arena(
                &game_assets,
                &sections,
                &styles,
                &mut roster,
            )));
            commands.insert_resource(Scoreboard::default());
            result::begin_match(&mut commands, roster.ships.len());
            next.set(PauseStates::Unpaused);
        }
        PauseAction::Lobby => {
            let Some(model) = model else {
                return;
            };
            commands.trigger(UnloadScenario);
            commands.insert_resource(Scoreboard::default());
            result::leave_match(&mut commands);
            lobby::spawn_lobby(&mut commands, &model, &styles, *skin);
            next.set(PauseStates::Unpaused);
        }
        PauseAction::Quit => {
            commands.write_message(AppExit::Success);
        }
    }
}
