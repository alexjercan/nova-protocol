//! Tree rows: one [`tree_row`] per visible node, indented by depth.
//!
//! The rows are a flat list. The caller owns what is open and what is listed;
//! this module owns how a row reads, and the [`tree_keep`] rule a filter box
//! applies to a tree.

use bevy::{picking::hover::Hovered, prelude::*, ui_widgets::Button};

use super::{ListRow, ThemedRadius, ThemedText, UiText};
use crate::theme::{density, UiColorType, BORDER_W};

/// The left padding of a root row.
const INDENT_BASE: f32 = 4.0;

/// The width one level of depth adds.
const INDENT_STEP: f32 = 12.0;

/// The size of all text on a row.
const ROW_TEXT: f32 = density::TEXT;

/// The width of the caret column. A leaf keeps the column, or the marks of a
/// parent and of its child stop lining up.
const CARET_W: f32 = 14.0;

/// The expand arrow of a [`tree_row`]: a button nested in the row.
///
/// `Activate` names the entity that was pressed and does not bubble, so a press
/// on the caret never reads as a press on the row.
#[derive(Component)]
pub struct TreeCaret;

/// What one [`tree_row`] shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeRowSpec {
    /// Levels under the root.
    pub depth: usize,
    /// The caret text. Empty on a leaf: the row then has no caret to press.
    pub caret: String,
    /// The kind mark in front of the label.
    pub lead: String,
    /// The text of the row.
    pub label: String,
    /// The muted child count. Empty for no count.
    pub count: String,
    /// The right-hand column: a state mark or an ordinal.
    pub trail: String,
}

/// One line of a tree. The label takes the free width and clips; the caret,
/// the mark, the count and the trail keep theirs.
pub fn tree_row(spec: &TreeRowSpec) -> impl Bundle {
    #[expect(
        clippy::cast_precision_loss,
        reason = "tree depth is single digits, not a precision question"
    )]
    let indent = px(INDENT_BASE + INDENT_STEP * spec.depth as f32);
    let text = |line: &str, colour: UiColorType| {
        (
            UiText,
            Text::new(line.to_string()),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextFont {
                font_size: FontSize::Px(ROW_TEXT),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(colour),
        )
    };
    let caret_paint = (
        Node {
            width: px(CARET_W),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![text(&spec.caret, UiColorType::TextMuted)],
    );
    (
        ListRow,
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            min_height: px(density::ROW_H),
            flex_shrink: 0.0,
            padding: UiRect {
                left: indent,
                right: px(6),
                top: px(0),
                bottom: px(0),
            },
            border: UiRect::all(px(BORDER_W)),
            align_items: AlignItems::Center,
            column_gap: px(4),
            overflow: Overflow::clip(),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![
            (TreeCaret, Button, Hovered::default(), caret_paint),
            text(&spec.lead, UiColorType::TextMuted),
            // `flex_basis` 0 and `min_width` 0 let a long label give its width
            // back instead of pushing the count out of the row.
            (
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                children![text(&spec.label, UiColorType::Text)],
            ),
            text(&spec.count, UiColorType::TextMuted),
            text(&spec.trail, UiColorType::TextSecondary),
        ],
    )
}

/// Which rows of a depth-ordered tree a filter keeps: every match, and every
/// ancestor of a match, so a kept row still shows where it lives.
///
/// `rows` holds each row's depth and whether it matched, in tree order.
pub fn tree_keep(rows: &[(usize, bool)]) -> Vec<bool> {
    let mut keep = vec![false; rows.len()];
    // The nearest row seen so far at each depth, as the walk's open path.
    let mut path: Vec<usize> = Vec::new();
    for (at, &(depth, matched)) in rows.iter().enumerate() {
        path.truncate(depth);
        path.push(at);
        if matched {
            for &ancestor in &path {
                keep[ancestor] = true;
            }
        }
    }
    keep
}

#[cfg(test)]
mod tests {
    use super::tree_keep;

    #[test]
    fn a_filter_keeps_a_match_and_its_ancestors_and_drops_its_siblings() {
        let rows = [
            (0, false),
            (1, false),
            (2, true),
            (2, false),
            (1, false),
            (2, false),
        ];
        assert_eq!(
            tree_keep(&rows),
            [true, true, true, false, false, false],
            "the match keeps its parent and root; the other branch goes"
        );
    }

    #[test]
    fn a_filter_with_no_match_keeps_nothing() {
        assert_eq!(tree_keep(&[(0, false), (1, false)]), [false, false]);
    }

    #[test]
    fn a_matched_parent_does_not_keep_its_children() {
        assert_eq!(tree_keep(&[(0, true), (1, false)]), [true, false]);
    }
}
