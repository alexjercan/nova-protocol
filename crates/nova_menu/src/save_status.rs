//! The open world's save status line: a small top-right readout of what the
//! last save of the open [`WorldSaveSession`] did.

use bevy::prelude::*;
use nova_gameplay::prelude::GameStates;
use nova_ui::{prelude::HUD_Z, theme::UiColor, widget::ThemedText};
use nova_world_base::prelude::{WorldSaveSession, WorldSaveStatus};

/// Marker for the one status line [`sync_save_status_line`] keeps.
#[derive(Component)]
pub(crate) struct SaveStatusLine;

/// The line's text and theme colour for `status`.
fn status_text(status: &WorldSaveStatus) -> (String, UiColor) {
    match status {
        WorldSaveStatus::Unsaved => ("Not saved yet".to_string(), UiColor::Label),
        WorldSaveStatus::Saved { .. } => ("World saved".to_string(), UiColor::Label),
        WorldSaveStatus::Writing => ("Saving world...".to_string(), UiColor::Label),
        WorldSaveStatus::Waiting(why) => (format!("Waiting to save: {why}"), UiColor::Label),
        WorldSaveStatus::Failed(err) => (format!("SAVE FAILED: {err}"), UiColor::Danger),
    }
}

/// Spawn, update or despawn the one save status line: present while a
/// [`WorldSaveSession`] exists, gone otherwise. A line is written only when
/// what it says changes, so the text is not laid out again each frame.
pub(crate) fn sync_save_status_line(
    mut commands: Commands,
    session: Option<Res<WorldSaveSession>>,
    mut lines: Query<(Entity, &mut Text, &mut ThemedText), With<SaveStatusLine>>,
) {
    let Some(session) = session else {
        for (entity, ..) in &lines {
            commands.entity(entity).despawn();
        }
        return;
    };
    let (text, color) = status_text(session.status());
    if let Some((_, mut line_text, mut themed)) = lines.iter_mut().next() {
        if line_text.0 != text {
            line_text.0 = text;
        }
        if themed.color != color {
            *themed = ThemedText::new(color);
        }
        return;
    }
    commands.spawn((
        Name::new("Save Status Line"),
        SaveStatusLine,
        // This system runs only in Playing, so it cannot remove the line
        // after a leave to the menu.
        DespawnOnExit(GameStates::Playing),
        Node {
            position_type: PositionType::Absolute,
            top: px(30),
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
