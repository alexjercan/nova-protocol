//! Leaving a saved world: the leave save every way out waits on, and Load
//! last save.
//!
//! Back to Main Menu, Exit and the window's close button each start a
//! [`PendingLeave`] instead of leaving at once. The game stays paused, so the
//! player's input and the ship's systems stay off, but the pause menu's
//! clock hold is released: bodies settle, and the session takes its leave
//! save. The way out happens only once that save is on disk. A save that
//! fails shows why, and the player chooses: try again, or leave without
//! saving and keep the last good save. After a death the session is spent:
//! the ways out write nothing, wait only for a write in flight, and keep the
//! last save.
//!
//! Load last save (the pause Retry in a saved world, and the primary action of
//! its Defeat) writes nothing. It waits for a write in flight, drops the
//! session and its lock, and opens the world from disk again. A world that no
//! longer opens is reported as a refused start, and the scenario ends.
//!
//! Without a [`WorldSaveSession`] - the web build, a scenario, the editor -
//! none of this runs, and every button does what it always did.

use bevy::{
    prelude::*,
    ui_widgets::{observe, Activate},
    window::WindowCloseRequested,
};
use nova_assets::prelude::LoadedSectionPacks;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::{
    CurrentScenario, LoadScenario, ScenarioStartFailure, ScenarioStartFailureReport, UnloadScenario,
};
use nova_ui::{
    prelude::PAUSE_Z,
    theme,
    theme::UiColor,
    widget::{panel, ButtonVariant, ThemedRadius, ThemedText, UiText},
};
use nova_world_base::prelude::{open_world, resume_world, WorldSaveSession, WorldSaveStatus};

use crate::{widgets::button_variant, world_setup::WorldsRoot};

/// Where a leave goes once the world is saved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LeaveTarget {
    /// Back to the main menu.
    Menu,
    /// Quit the game.
    Exit,
    /// Load last save: reopen the world from disk, writing nothing.
    Retry,
}

/// A leave that waits on the world's save before it happens.
#[derive(Resource, Debug)]
pub(crate) struct PendingLeave {
    pub(crate) target: LeaveTarget,
}

/// Marker for the leave overlay root.
#[derive(Component)]
pub(crate) struct LeaveOverlay;

/// Start leaving the saved world for `target`.
///
/// The game pauses (a window closed mid-flight was not paused yet), so input
/// stays off for the whole wait. Menu and Exit want the leave save, unless
/// the session is spent (the player died): that world is not the saved one,
/// and the last save stays. Retry stops every save, so nothing is written
/// before the world reopens.
pub(crate) fn begin_leave(
    target: LeaveTarget,
    commands: &mut Commands,
    session: &mut WorldSaveSession,
    pause: &mut NextState<PauseStates>,
) {
    match target {
        LeaveTarget::Menu | LeaveTarget::Exit if !session.is_spent() => session.request_leave(),
        LeaveTarget::Menu | LeaveTarget::Exit => {}
        LeaveTarget::Retry => session.stop_saving(),
    }
    pause.set_if_neq(PauseStates::Paused);
    commands.insert_resource(PendingLeave { target });
}

/// Finish the pending leave once the session allows it.
///
/// Menu and Exit go only when the session is idle AND saved: a request is
/// idle only after the leave save itself was taken, so a saved status from
/// an earlier crossing cannot end the wait. While the save waits, the pause
/// menu's clock hold is released, so bodies settle; on a failure the hold is
/// taken back and the overlay asks the player. A spent session wants no leave
/// save; Menu and Exit go once no write is in flight.
///
/// Retry goes once no write is in flight.
pub(crate) fn drive_pending_leave(
    mut commands: Commands,
    pending: Res<PendingLeave>,
    session: Option<Res<WorldSaveSession>>,
    mut clocks: Clocks,
    mut state: ResMut<NextState<GameStates>>,
    mut pause: ResMut<NextState<PauseStates>>,
    mut exit: MessageWriter<AppExit>,
) {
    // Every path that drops the session drops the pending leave with it.
    let session = session.expect("a pending leave outlived its world session");
    match pending.target {
        LeaveTarget::Menu | LeaveTarget::Exit => {
            if session.is_spent() {
                if session.is_writing() {
                    return;
                }
            } else if !session.is_idle() {
                clocks.release(FreezeOwner::PauseMenu);
                return;
            } else if !matches!(session.status(), WorldSaveStatus::Saved { .. }) {
                clocks.hold(FreezeOwner::PauseMenu);
                return;
            }
            commands.remove_resource::<WorldSaveSession>();
            commands.remove_resource::<PendingLeave>();
            if pending.target == LeaveTarget::Exit {
                exit.write(AppExit::Success);
            } else {
                state.set(GameStates::MainMenu);
                pause.set(PauseStates::Unpaused);
            }
        }
        LeaveTarget::Retry => {
            if session.is_writing() {
                return;
            }
            // In one command: the session's lock must be gone before the
            // same world is opened again, or the open refuses it as locked.
            // The write check runs again when the command applies: a write
            // that started after this system ran is waited for next frame.
            commands.queue(|world: &mut World| {
                if world
                    .get_resource::<WorldSaveSession>()
                    .is_none_or(WorldSaveSession::is_writing)
                {
                    return;
                }
                world.remove_resource::<PendingLeave>();
                let Some(session) = world.remove_resource::<WorldSaveSession>() else {
                    return;
                };
                let slug = session.folder().slug.clone();
                let name = session.name().to_string();
                drop(session);
                let opened = match (
                    &world.resource::<WorldsRoot>().0,
                    world.get_resource::<LoadedSectionPacks>(),
                ) {
                    (Some(root), Some(packs)) => {
                        open_world(root, &slug, packs).map_err(|refusal| refusal.to_string())
                    }
                    (None, _) => Err("this system has no folder for saved worlds".to_string()),
                    (_, None) => Err("no content is loaded".to_string()),
                };
                match opened {
                    Ok((folder, lock, header, state)) => {
                        resume_world(world, folder, lock, &header, state);
                        if let Some(scenario) = world.resource::<CurrentScenario>().0.clone() {
                            world.trigger(LoadScenario(scenario));
                        }
                        world
                            .resource_mut::<NextState<PauseStates>>()
                            .set(PauseStates::Unpaused);
                    }
                    Err(refusal) => {
                        world.trigger(UnloadScenario);
                        world.resource_mut::<ScenarioStartFailure>().0 =
                            Some(ScenarioStartFailureReport {
                                scenario_name: name,
                                messages: vec![refusal],
                            });
                    }
                }
            });
        }
    }
}

/// Draw the leave overlay while a leave is pending: what the save is doing,
/// and on a failure why, with the player's two choices.
///
/// Redrawn only when what it says changes.
pub(crate) fn sync_leave_overlay(
    mut commands: Commands,
    pending: Option<Res<PendingLeave>>,
    session: Option<Res<WorldSaveSession>>,
    overlays: Query<Entity, With<LeaveOverlay>>,
    mut drawn: Local<Option<(String, String, bool)>>,
) {
    let wanted =
        pending.zip(session).map(
            |(pending, session)| match (pending.target, session.status()) {
                (LeaveTarget::Retry, _) => (
                    "Loading last save...".to_string(),
                    "Progress since the last save is discarded.".to_string(),
                    false,
                ),
                // A spent session wants no leave save, so none can fail.
                (_, WorldSaveStatus::Failed(error)) if session.is_idle() && !session.is_spent() => {
                    (
                        "SAVE FAILED".to_string(),
                        format!("{error}. The last save is kept."),
                        true,
                    )
                }
                (_, WorldSaveStatus::Waiting(why)) => {
                    ("Saving world...".to_string(), why.clone(), false)
                }
                _ => ("Saving world...".to_string(), String::new(), false),
            },
        );
    if wanted == *drawn && overlays.is_empty() == wanted.is_none() {
        return;
    }
    for overlay in &overlays {
        commands.entity(overlay).despawn();
    }
    drawn.clone_from(&wanted);
    let Some((title, body, failed)) = wanted else {
        return;
    };
    commands
        .spawn((
            LeaveOverlay,
            DespawnOnExit(GameStates::Playing),
            Name::new("Leave Overlay"),
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
            GlobalZIndex(PAUSE_Z),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Leave Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        width: px(360),
                        padding: UiRect::all(px(20)),
                        row_gap: px(8),
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                    ThemedRadius::control(),
                    panel(),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Leave Title"),
                        UiText,
                        Text::new(title),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(if failed {
                            UiColor::Danger
                        } else {
                            UiColor::Body
                        }),
                    ));
                    parent.spawn((
                        Name::new("Leave Detail"),
                        UiText,
                        Text::new(body),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Label),
                    ));
                    if failed {
                        parent.spawn((
                            Name::new("Leave Try Again Button"),
                            button_variant("Try again", ButtonVariant::Primary, None),
                            observe(on_leave_try_again),
                        ));
                        parent.spawn((
                            Name::new("Leave Without Saving Button"),
                            button_variant("Leave without saving", ButtonVariant::Danger, None),
                            observe(on_leave_without_saving),
                        ));
                    }
                });
        });
}

/// Ask for the leave save again after a failure.
pub(crate) fn on_leave_try_again(
    _activate: On<Activate>,
    mut session: Option<ResMut<WorldSaveSession>>,
) {
    if let Some(session) = session.as_mut() {
        session.request_leave();
    }
}

/// The player's consent to leave without the leave save: the last good save
/// stays as it is, and the leave goes where it was going.
pub(crate) fn on_leave_without_saving(
    _activate: On<Activate>,
    mut commands: Commands,
    pending: Option<Res<PendingLeave>>,
    mut state: ResMut<NextState<GameStates>>,
    mut pause: ResMut<NextState<PauseStates>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(pending) = pending else {
        return;
    };
    commands.remove_resource::<WorldSaveSession>();
    commands.remove_resource::<PendingLeave>();
    match pending.target {
        LeaveTarget::Exit => {
            exit.write(AppExit::Success);
        }
        LeaveTarget::Menu => {
            state.set(GameStates::MainMenu);
            pause.set(PauseStates::Unpaused);
        }
        LeaveTarget::Retry => unreachable!("Load last save offers no leave without saving"),
    }
}

/// The window's close button. With a saved world in play it is Exit, which
/// waits on the leave save; a close while a leave is already pending does
/// nothing, so it cannot cut a write short. Anywhere else the game quits at
/// once, as it always did.
pub(crate) fn on_window_close_requested(
    mut closes: MessageReader<WindowCloseRequested>,
    mut commands: Commands,
    game: Option<Res<State<GameStates>>>,
    pending: Option<Res<PendingLeave>>,
    session: Option<ResMut<WorldSaveSession>>,
    mut pause: ResMut<NextState<PauseStates>>,
    mut exit: MessageWriter<AppExit>,
) {
    if closes.read().count() == 0 || pending.is_some() {
        return;
    }
    let playing = game.is_some_and(|game| *game.get() == GameStates::Playing);
    match session {
        Some(mut session) if playing => {
            begin_leave(LeaveTarget::Exit, &mut commands, &mut session, &mut pause);
        }
        _ => {
            exit.write(AppExit::Success);
        }
    }
}
