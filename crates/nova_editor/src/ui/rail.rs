//! The left rail's widgets: the Scene tree rows and the ship settings rows.
//! The theme and the shared button shapes live in `nova_ui`; this module only
//! assembles editor-specific rows out of them.

use bevy::{picking::hover::Hovered, prelude::*, ui_widgets::Button};
use nova_ui::{
    prelude::{ThemedButton, UiSkin},
    theme,
    widget::{checkbox, list_row_colors, ListRow},
};

use crate::config::{SkinToggleCheckbox, StyleChoice};

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

/// A root row's left padding, matching the other rails rows' `10px`-ish inset.
const INDENT_BASE: f32 = 8.0;
/// What one level of nesting is worth. Wide enough to read as a step, narrow
/// enough that a section (depth 2) still shows most of its id.
const INDENT_STEP: f32 = 9.0;

/// Row type size (px). One step under the panel type: the rail is a tree of
/// minted ids in 150px, and every point of size is a character of id the row
/// can show before it clips.
const ROW_TEXT: f32 = 11.0;

/// One row of the Scene tree: the scenario root, a ship, or a section of the
/// entered ship.
///
/// The same `ListRow` shape the look rows use, so the shared reconciler paints
/// the selection and the hover and this module owns no colour. `lead` is the
/// glyph in front of the label - one muted text node instead of an icon asset,
/// which is also what keeps the tree in the terminal look the rest of the
/// screen wears.
///
/// `depth` is spent on the row's LEFT PADDING rather than on drawn connectors.
/// Indentation is what the eye reads a tree by, and it costs no width in a
/// 150px rail: `|- ` in front of every child ate 18px of the label on every
/// row, and a minted id is exactly the thing that then ran out of room.
pub(crate) fn scene_row(
    depth: usize,
    lead: &str,
    label: &str,
    trail: &str,
    selected: bool,
    skin: UiSkin,
) -> impl Bundle {
    let (background, border) = list_row_colors(selected, false, skin);
    #[expect(
        clippy::cast_precision_loss,
        reason = "tree depth is single digits, not a precision question"
    )]
    let indent = px(INDENT_BASE + INDENT_STEP * depth as f32);
    // A minted id can outgrow the 150px rail. NoWrap + clip keeps every row
    // one line tall - a wrapped lead column stacked its glyphs vertically and
    // the tree stopped reading as a tree.
    let one_line = TextLayout {
        linebreak: LineBreak::NoWrap,
        ..default()
    };
    let row_font = TextFont {
        font_size: FontSize::Px(ROW_TEXT),
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
            padding: UiRect {
                left: indent,
                right: px(6),
                top: px(2),
                bottom: px(2),
            },
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
                row_font.clone(),
                TextColor(theme::PHOSPHOR_MUTED),
            ),
            // The label is the half that may clip, so it sits in the shrinking
            // column: `flex_basis` 0 plus `min_width` 0 is what lets a long id
            // give its width back instead of pushing the trail out of the row.
            (
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                children![(
                    Text::new(label.to_string()),
                    one_line,
                    row_font.clone(),
                    TextColor(theme::PHOSPHOR),
                )],
            ),
            // Which one this is. Fixed at the row's right edge, because it is
            // the only thing telling a run of same-part rows apart.
            (
                Text::new(trail.to_string()),
                one_line,
                row_font,
                TextColor(theme::PHOSPHOR_MUTED),
            )
        ],
    )
}
