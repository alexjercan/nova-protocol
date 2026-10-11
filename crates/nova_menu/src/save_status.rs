//! A transient save notice while an open world saves or reports a failure.

use std::time::Duration;

use bevy::prelude::*;
use nova_gameplay::prelude::GameStates;
use nova_hud::prelude::HudTier;
use nova_ui::{prelude::HUD_Z, theme::UiColor, widget::ThemedText};
use nova_world_base::prelude::{WorldSaveSession, WorldSaveStatus};

/// Marker for the one notice [`sync_save_notice`] keeps.
#[derive(Component)]
pub(crate) struct SaveNotice;

/// The notice's text and theme colour for `status`.
fn status_text(status: &WorldSaveStatus, show_saved: bool) -> Option<(String, UiColor)> {
    match status {
        WorldSaveStatus::Unsaved => None,
        WorldSaveStatus::Saved { .. } if show_saved => {
            Some(("World saved".to_string(), UiColor::Label))
        }
        WorldSaveStatus::Saved { .. } => None,
        WorldSaveStatus::Writing | WorldSaveStatus::Waiting(_) => {
            Some(("Saving world...".to_string(), UiColor::Label))
        }
        WorldSaveStatus::Failed(err) => Some((format!("SAVE FAILED: {err}"), UiColor::Danger)),
    }
}

/// Show a save in progress, a new success for three real seconds, or a
/// failure until the next attempt. An opened world's old success is silent.
pub(crate) fn sync_save_notice(
    mut commands: Commands,
    session: Option<Res<WorldSaveSession>>,
    time: Res<Time<Real>>,
    mut previous: Local<Option<WorldSaveStatus>>,
    mut saved_at: Local<Option<Duration>>,
    mut notices: Query<(Entity, &mut Text, &mut ThemedText), With<SaveNotice>>,
) {
    let Some(session) = session else {
        *previous = None;
        *saved_at = None;
        for (entity, ..) in &notices {
            commands.entity(entity).despawn();
        }
        return;
    };
    if session.is_added() {
        *previous = None;
        *saved_at = None;
    }
    let status = session.status();
    if previous.as_ref() != Some(status) {
        if matches!(status, WorldSaveStatus::Saved { .. })
            && previous.as_ref().is_some_and(|status| {
                matches!(
                    status,
                    WorldSaveStatus::Waiting(_) | WorldSaveStatus::Writing
                )
            })
        {
            *saved_at = Some(time.elapsed());
        } else if !matches!(status, WorldSaveStatus::Saved { .. }) {
            *saved_at = None;
        }
        *previous = Some(status.clone());
    }
    let show_saved =
        saved_at.is_some_and(|at| time.elapsed().saturating_sub(at) < Duration::from_secs(3));
    let Some((text, color)) = status_text(status, show_saved) else {
        for (entity, ..) in &notices {
            commands.entity(entity).despawn();
        }
        return;
    };
    if let Some((_, mut notice_text, mut themed)) = notices.iter_mut().next() {
        if notice_text.0 != text {
            notice_text.0 = text;
        }
        if themed.color != color {
            *themed = ThemedText::new(color);
        }
        return;
    }
    commands.spawn((
        Name::new("Save Notice"),
        SaveNotice,
        HudTier::Status,
        Visibility::Inherited,
        DespawnOnExit(GameStates::Playing),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(30),
            right: px(10),
            ..default()
        },
        GlobalZIndex(HUD_Z),
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(color),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unsaved_or_preexisting_saved_world_has_no_notice() {
        assert!(status_text(&WorldSaveStatus::Unsaved, false).is_none());
        assert!(status_text(&WorldSaveStatus::Saved { generation: 1 }, false).is_none());
        assert!(status_text(&WorldSaveStatus::Saved { generation: 2 }, true).is_some());
        assert!(status_text(&WorldSaveStatus::Waiting(String::new()), false).is_some());
        assert!(status_text(&WorldSaveStatus::Writing, false).is_some());
    }
}
