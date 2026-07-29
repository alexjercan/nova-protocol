//! The left category rail, styled after the web wiki sidebar: an active
//! "Components" category that opens the drawer, plus greyed coming-soon rows
//! (Ships/Objects/Events/Objectives) that advertise "the rest" (task
//! 20260714-081703).

use bevy::{picking::hover::Hovered, prelude::*, ui_widgets::Button};
use nova_ui::{
    prelude::{ThemedButton, UiSkin},
    theme,
    widget::{badge, BadgeKind},
};

use crate::ui::drawer::toggle_drawer;

/// The active "Components" category row: a themed button that toggles the
/// drawer. Uses `ThemedButton` so it gets the shared hover colouring, but
/// carries no `ButtonValue`, so pressing it never touches `SectionChoice`.
pub(crate) fn components_category() -> impl Bundle {
    (
        Name::new("Components Category"),
        ThemedButton,
        Button,
        Hovered::default(),
        bevy::ui_widgets::observe(toggle_drawer),
        Node {
            width: percent(100),
            min_height: px(30),
            margin: UiRect::vertical(px(2)),
            padding: UiRect::axes(px(10), px(6)),
            border: UiRect::all(px(theme::BORDER_W)),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(theme::PHOSPHOR_MUTED),
        BackgroundColor(theme::SCREEN_0),
        children![(
            Text::new("Components"),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR),
        )],
    )
}

/// A greyed, non-interactive coming-soon category row with an amber "soon"
/// badge - the categories "the rest" will make real.
pub(crate) fn coming_soon_category(label: &str, skin: UiSkin) -> impl Bundle {
    (
        Name::new(format!("{label} Category (soon)")),
        Node {
            width: percent(100),
            min_height: px(30),
            margin: UiRect::vertical(px(2)),
            padding: UiRect::axes(px(10), px(6)),
            border: UiRect::all(px(theme::BORDER_W)),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(6),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(theme::PHOSPHOR_MUTED.with_alpha(0.5)),
        BackgroundColor(theme::SPACE),
        children![
            (
                Text::new(label.to_string()),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_MUTED),
            ),
            // The shared badge widget (amber), matching the widget_zoo.
            badge(BadgeKind::Amber, "soon", skin),
        ],
    )
}
