//! The left rail's widgets: the Scene tree rows and the ship settings rows.
//! The theme and the shared button shapes live in `nova_ui`; this module only
//! assembles editor-specific rows out of them.

use bevy::{picking::hover::Hovered, prelude::*, ui_widgets::Button};
use nova_ui::{
    prelude::{ThemedButton, UiSkin},
    theme,
    widget::{checkbox, list_row_colors, ListRow},
};

use crate::{
    config::{SkinToggleCheckbox, StyleChoice},
    node::ObjectChoice,
};

/// The cladding toggle: a tool row that is a SETTING rather than a mode, so it
/// carries the shared `checkbox` widget instead of a `ButtonValue`.
///
/// The row is the button and the box is a picture. `ui_widgets::Button` on the
/// box would swallow the press before the row saw it, and a 22px target in a
/// 150px rail is a worse one than the row it sits in.
pub(crate) fn skin_toggle_row(on: bool, skin: UiSkin) -> impl Bundle {
    (
        ThemedButton,
        Button,
        Hovered::default(),
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
        BorderColor::all(theme::PHOSPHOR_MUTED),
        BackgroundColor(theme::SCREEN_0),
        children![
            (
                Text::new("Ship Skin"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR),
            ),
            (SkinToggleCheckbox, checkbox(on, skin)),
        ],
    )
}

/// One look in the list under the cladding toggle.
///
/// A `ListRow` and not a `themed_button`: the rows are a SELECTION over the
/// merged style catalog, which is what a list row paints, and the shared
/// reconciler then repaints this one from its own `Selected`/`Hovered` without
/// the editor owning any colour.
///
/// Deliberately COMPACT - 22px against a tool button's 34. The rail is 150px
/// wide on a 1024x768 window and the catalog is as long as the content merge
/// makes it, so five looks at tool height push Play off the bottom of the
/// screen. Measured, not guessed.
pub(crate) fn style_row(id: &str, name: &str, selected: bool, skin: UiSkin) -> impl Bundle {
    let (background, border) = list_row_colors(selected, false, skin);
    (
        ListRow,
        StyleChoice(id.to_string()),
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            min_height: px(22),
            margin: UiRect::bottom(px(2)),
            padding: UiRect::axes(px(10), px(2)),
            border: UiRect::all(px(theme::BORDER_W)),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(background),
        children![(
            Text::new(name.to_string()),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR),
        )],
    )
}

/// One kind in the world block's object palette.
///
/// A `ListRow` like the looks above rather than a tool chip, and for the same
/// reason: five kinds at tool height would push the tree off a 150px rail. It
/// is an ACTION, not a mode - pressing it places an object and nothing stays
/// armed - so nothing here is ever marked `Selected`.
pub(crate) fn object_row(choice: ObjectChoice, skin: UiSkin) -> impl Bundle {
    let (background, border) = list_row_colors(false, false, skin);
    (
        ListRow,
        choice,
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            min_height: px(22),
            margin: UiRect::bottom(px(2)),
            padding: UiRect::axes(px(10), px(2)),
            border: UiRect::all(px(theme::BORDER_W)),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(background),
        children![(
            Text::new(choice.label().to_string()),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR),
        )],
    )
}

/// One row of the Scene tree: the scenario root, a ship, or a section of the
/// entered ship.
///
/// The same `ListRow` shape the look rows use, so the shared reconciler paints
/// the selection and the hover and this module owns no colour. `lead` is the
/// tree furniture in front of the label - ASCII connectors for the depth plus
/// a glyph for the node's kind - one muted text node instead of an icon asset,
/// which is also what keeps the tree in the terminal look the rest of the
/// screen wears.
pub(crate) fn scene_row(lead: &str, label: &str, selected: bool, skin: UiSkin) -> impl Bundle {
    let (background, border) = list_row_colors(selected, false, skin);
    // A minted id can outgrow the 150px rail. NoWrap + clip keeps every row
    // one line tall - a wrapped lead column stacked its glyphs vertically and
    // the tree stopped reading as a tree.
    let one_line = TextLayout {
        linebreak: LineBreak::NoWrap,
        ..default()
    };
    (
        ListRow,
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            min_height: px(22),
            margin: UiRect::bottom(px(2)),
            padding: UiRect::axes(px(8), px(2)),
            border: UiRect::all(px(theme::BORDER_W)),
            align_items: AlignItems::Center,
            column_gap: px(6),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            overflow: Overflow::clip(),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(background),
        children![
            (
                Text::new(lead.to_string()),
                one_line,
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_MUTED),
            ),
            (
                Text::new(label.to_string()),
                one_line,
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR),
            )
        ],
    )
}
