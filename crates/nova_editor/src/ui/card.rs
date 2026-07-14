//! Component cards: the wiki-style tiles in the drawer. Each card is a themed
//! button (so it reuses the selection/highlight machinery) carrying a
//! placeholder icon and the section's name, and it hovers a tooltip
//! (see `tooltip.rs`) with the full name/HP/description.

use bevy::{picking::hover::Hovered, prelude::*, ui_widgets::Button};
use nova_gameplay::prelude::*;

use crate::{
    config::SectionChoice,
    ui::{
        theme,
        widget::{ButtonValue, EditorButton},
    },
};

/// A component card, carrying the catalog id of the section it places. The
/// tooltip system reads this to look the section up in `GameSections`.
#[derive(Component, Debug, Clone)]
pub(crate) struct ComponentCard {
    pub(crate) id: String,
}

/// Child nodes must not steal hover from the card, or the tooltip would flicker
/// as the pointer crosses the icon/label.
const IGNORE: Pickable = Pickable {
    should_block_lower: false,
    is_hoverable: false,
};

/// A distinct tint per section kind, so the placeholder icons read apart at a
/// glance even before real art exists.
fn kind_tint(kind: &SectionKind) -> Color {
    match kind {
        SectionKind::Hull(_) => Color::srgb_u8(90, 110, 150),
        SectionKind::Thruster(_) => theme::AMBER,
        SectionKind::Controller(_) => theme::CYAN,
        SectionKind::Turret(_) => Color::srgb_u8(220, 110, 90),
        SectionKind::Torpedo(_) => Color::srgb_u8(170, 120, 210),
    }
}

/// A single-letter glyph standing in for the (not-yet-drawn) component icon.
fn kind_glyph(kind: &SectionKind) -> &'static str {
    match kind {
        SectionKind::Hull(_) => "H",
        SectionKind::Thruster(_) => "T",
        SectionKind::Controller(_) => "C",
        SectionKind::Turret(_) => "U",
        SectionKind::Torpedo(_) => "B",
    }
}

/// The placeholder component icon: a 44x44 bright-bordered tile with a kind tint
/// and a glyph, mirroring the web wiki's hatched `.wiki-child__icon`. Isolated
/// behind this helper so a later task can swap in a real texture without
/// touching the card layout.
pub(crate) fn component_icon(kind: &SectionKind) -> impl Bundle {
    let tint = kind_tint(kind);
    (
        Node {
            width: px(theme::ICON),
            height: px(theme::ICON),
            border: UiRect::all(px(theme::BORDER_W)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            flex_shrink: 0.0,
            ..default()
        },
        // A tinted-but-dim fill (the tint at low intensity) with a bright border
        // in the tint colour, so the tile reads as "iconic" without real art.
        BackgroundColor(tint.with_alpha(0.18)),
        BorderColor::all(tint),
        IGNORE,
        children![(
            Text::new(kind_glyph(kind)),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(tint),
            IGNORE,
        )],
    )
}

/// A component card bundle for `section`. Named after the section (the autopilot
/// and any name lookup find it), carries the placement value + the `ComponentCard`
/// tag, and lays out icon + name in a row.
pub(crate) fn component_card(section: &SectionConfig) -> impl Bundle {
    (
        Name::new(section.base.name.clone()),
        ComponentCard {
            id: section.base.id.clone(),
        },
        EditorButton,
        Button,
        Hovered::default(),
        ButtonValue(SectionChoice::Section(section.base.id.clone())),
        Node {
            width: percent(100),
            min_height: px(56),
            margin: UiRect::vertical(px(4)),
            padding: UiRect::all(px(8)),
            border: UiRect::all(px(theme::BORDER_W)),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(theme::BORDER),
        BackgroundColor(theme::PANEL),
        children![
            component_icon(&section.kind),
            (
                Text::new(section.base.name.clone()),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(theme::TEXT),
                IGNORE,
            )
        ],
    )
}
