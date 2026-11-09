//! Tabs: a [`tab_strip`] of equal [`tab`] buttons, one of them marked
//! [`Selected`](super::Selected).
//!
//! A tab is a [`ListRow`], so the row reconciler paints the mark and the hover.
//! Which tab is marked is the caller's state, and the caller inserts the mark.

use bevy::{picking::hover::Hovered, prelude::*, ui_widgets::Button};

use super::{ListRow, ThemedRadius, ThemedText, UiText};
use crate::theme::{density, UiColorType, BORDER_W};

/// The text of a [`tab`], for a caller that writes a count into it.
#[derive(Component)]
pub struct TabLabel;

/// One tab. The caller adds a [`Name`], its own value marker and an observer.
/// With `fill`, the tabs of a strip share its width equally.
pub fn tab(label: &str, fill: bool) -> impl Bundle {
    (
        ListRow,
        Button,
        Hovered::default(),
        Node {
            flex_grow: if fill { 1.0 } else { 0.0 },
            flex_basis: if fill { px(0) } else { Val::Auto },
            min_width: px(0),
            min_height: px(density::CONTROL_H - 2.0),
            padding: UiRect::axes(px(8), px(1)),
            border: UiRect::all(px(BORDER_W)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            overflow: Overflow::clip(),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![(
            TabLabel,
            UiText,
            Text::new(label.to_uppercase()),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextFont {
                font_size: FontSize::Px(density::TEXT_SMALL),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColorType::Text),
        )],
    )
}

/// The row a set of [`tab`]s sits in.
pub fn tab_strip() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Stretch,
        column_gap: px(4),
        ..default()
    }
}
