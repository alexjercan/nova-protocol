//! The list-beside-details screen: the shape `nova_menu` builds for its mods
//! screen (installed and explore-online tabs alike) and again for its scenarios
//! screen.

use bevy::prelude::*;

use super::scroll::ScrollViewport;
use crate::theme;

/// Width of the list pane, as a share of the screen card.
const LIST_PANE_PERCENT: f32 = 40.0;

/// The full-screen overlay a menu screen spawns above the menu card: centered,
/// transparent to picking, and stacked explicitly.
///
/// The [`GlobalZIndex`] is load-bearing. Sibling z-order otherwise falls back to
/// entity-id ordering, and ids freed by a despawned scene get recycled, so the
/// overlay stacked nondeterministically against the menu card.
pub fn overlay_root() -> impl Bundle {
    (
        Visibility::Hidden,
        Pickable {
            should_block_lower: false,
            is_hoverable: false,
        },
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        GlobalZIndex(1),
    )
}

/// The list-beside-details composition: `left` (a [`list_pane`], with or without
/// a tab row above the list) beside `details` (a [`details_pane`]).
pub fn list_detail_screen(left: impl Bundle, details: impl Bundle) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_self: AlignSelf::Stretch,
            flex_grow: 1.0,
            min_height: px(0),
            min_width: px(0),
            column_gap: px(16),
            margin: UiRect::vertical(px(10)),
            ..default()
        },
        children![left, details],
    )
}

/// The left pane of a [`list_detail_screen`], pinned at its share of the width.
///
/// PINNED: a flex row shrinks EVERY shrinkable item, so at the default
/// `flex_shrink` this pane gave up width whenever the selected item's details
/// wanted more - measured as a 141..331 px swing purely from the selection. The
/// split is a property of the SCREEN, not of the selection; the details pane
/// absorbs all the slack instead.
pub fn list_pane() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        width: percent(LIST_PANE_PERCENT),
        min_height: px(0),
        flex_grow: 0.0,
        flex_shrink: 0.0,
        ..default()
    }
}

/// A scrollable column filling whatever box it is given. `min_height: 0` is what
/// lets it shrink below its content height, without which the overflow never
/// scrolls.
pub fn scroll_column() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        align_self: AlignSelf::Stretch,
        flex_grow: 1.0,
        min_height: px(0),
        min_width: px(0),
        overflow: Overflow::scroll_y(),
        ..default()
    }
}

/// Wires a node spawned with [`scroll_column`] (or a scrolling [`list_pane`]) to
/// the shared wheel driver and every-frame clamp.
pub fn scroll_viewport() -> impl Bundle {
    (ScrollViewport, ScrollPosition::default())
}

/// The details pane beside a list: takes all the slack, wraps rather than
/// pushing the list narrower, and is separated by a rule.
pub fn details_pane() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            min_height: px(0),
            min_width: px(0),
            padding: UiRect::left(px(16)),
            border: UiRect::left(px(theme::BORDER_W)),
            ..default()
        },
        BorderColor::all(theme::PHOSPHOR_MUTED),
    )
}

/// A screen footer holding `back` in a fixed-width slot, so a percent-width
/// button does not span the whole card.
pub fn footer_back_slot(back: impl Bundle) -> impl Bundle {
    (
        Node {
            align_self: AlignSelf::Stretch,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexStart,
            ..default()
        },
        children![(
            Node {
                width: px(200),
                ..default()
            },
            children![back],
        )],
    )
}
