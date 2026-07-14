//! The component drawer: the panel beside the rail that holds the scrollable
//! grid of component cards. Clicking the "Components" category toggles it, so
//! the 3D build area can be uncovered.

use bevy::{prelude::*, ui_widgets::Activate};

use crate::ui::theme;

/// The toggleable drawer panel (shown/hidden via `Visibility`).
#[derive(Component)]
pub(crate) struct DrawerPanel;

/// A section header inside a panel (e.g. "COMPONENTS").
pub(crate) fn panel_header(text: &str) -> impl Bundle {
    (
        Text::new(text.to_uppercase()),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(theme::CYAN),
        Node {
            margin: UiRect::bottom(px(8)),
            ..default()
        },
    )
}

/// Toggle the drawer's visibility. Wired to the "Components" category button via
/// `observe`.
pub(crate) fn toggle_drawer(
    _activate: On<Activate>,
    mut q_drawer: Query<&mut Visibility, With<DrawerPanel>>,
) {
    for mut visibility in &mut q_drawer {
        *visibility = match *visibility {
            Visibility::Hidden => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    }
}
