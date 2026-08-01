//! Small stateless chrome: section headers, separators, status badges,
//! checkboxes and pill toggles. Each is a plain bundle factory - none carries a
//! reconciler, so a skin flip reaches them when their screen is next rebuilt.

use bevy::{ecs::relationship::RelatedSpawner, prelude::*};

use super::UiText;
use crate::{skin::UiSkin, theme};

/// A small uppercase section header (e.g. "COMPONENTS"), flat phosphor. No
/// TextShadow: a hard drop shadow makes the header feel floaty; a terminal
/// header should read as imprinted on the screen.
pub fn panel_header(text: &str) -> impl Bundle {
    (
        UiText,
        Text::new(text.to_uppercase()),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(theme::PHOSPHOR),
        Node {
            margin: UiRect::bottom(px(8)),
            ..default()
        },
    )
}

/// A thin horizontal separator (phosphor-muted rule).
pub fn separator() -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: px(theme::BORDER_W),
            margin: UiRect::vertical(px(8)),
            ..default()
        },
        BackgroundColor(theme::PHOSPHOR_MUTED.with_alpha(0.5)),
    )
}

// NOTE: the rest of the demo-1 widget set reads the CURRENT skin at spawn - only
// the button family is live-reactive. A widget here needs its own marker +
// reconciler before it can restyle in place.

/// Semantic colour families for [`badge`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeKind {
    /// Phosphor green - nominal / online.
    Green,
    /// Amber - warning.
    Amber,
    /// Blue - info.
    Blue,
    /// Red - fault.
    Red,
    /// Muted grey - idle / neutral tag.
    Mute,
}

impl BadgeKind {
    fn color(self) -> Color {
        match self {
            BadgeKind::Green => theme::PHOSPHOR,
            BadgeKind::Amber => theme::AMBER_NOVA,
            BadgeKind::Blue => theme::BLUE,
            BadgeKind::Red => theme::RED,
            BadgeKind::Mute => theme::PHOSPHOR_MUTED.with_alpha(0.9),
        }
    }
}

/// A status badge. Phosphor skin: a bracketed `[TAG]` text span in the family
/// colour, no chrome. Hardware skin: a bordered, tinted chip.
pub fn badge(kind: BadgeKind, text: &str, skin: UiSkin) -> impl Bundle {
    let color = kind.color();
    let phosphor = skin.is_phosphor();
    let label = if phosphor {
        format!("[{}]", text.to_uppercase())
    } else {
        text.to_uppercase()
    };
    let (border, bg, border_w) = if phosphor {
        (Color::NONE, Color::NONE, 0.0)
    } else {
        (
            color.with_alpha(0.4),
            color.with_alpha(0.06),
            theme::BORDER_W,
        )
    };
    (
        Node {
            padding: UiRect::axes(px(if phosphor { 2.0 } else { 8.0 }), px(3.0)),
            border: UiRect::all(px(border_w)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(bg),
        children![(
            UiText,
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(color),
        )],
    )
}

/// A 22px checkbox. Off: bordered empty square. On: inverted (filled fill, dark
/// `x`). Phosphor uses 2px corners + phosphor; hardware uses 5px + a recessed
/// case face turning phosphor when on.
pub fn checkbox(on: bool, skin: UiSkin) -> impl Bundle {
    let phosphor = skin.is_phosphor();
    let radius = if phosphor { theme::RADIUS } else { 5.0 };
    let (bg, border, glyph) = checkbox_colors(on, skin);
    (
        Node {
            width: px(22),
            height: px(22),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(radius)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(bg),
        children![(
            UiText,
            Text::new(checkbox_glyph(on)),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(glyph),
        )],
    )
}

/// The `(background, border, glyph)` colours of a checkbox in `(on, skin)` - the
/// single source both [`checkbox`] and an in-place restyle (the mods screen's
/// enable-checkbox sync) paint from.
pub fn checkbox_colors(on: bool, skin: UiSkin) -> (Color, Color, Color) {
    if on {
        (theme::PHOSPHOR, theme::PHOSPHOR, theme::INK)
    } else if skin.is_phosphor() {
        (
            Color::NONE,
            theme::PHOSPHOR.with_alpha(0.4),
            theme::PHOSPHOR,
        )
    } else {
        (theme::CASE_0, theme::CASE_EDGE, theme::PHOSPHOR)
    }
}

/// The checkbox glyph: `x` when checked, empty otherwise.
pub fn checkbox_glyph(on: bool) -> &'static str {
    if on {
        "x"
    } else {
        ""
    }
}

/// A pill toggle switch (demo `.toggle`): a 44x22 track with a sliding dot.
/// Off: the dot sits left, muted. On: the dot slides right and the track tints
/// phosphor (phosphor skin) with a glowing dot. Phosphor uses 2px square-ish
/// corners; hardware uses a rounded pill + a recessed well look. Not a
/// `ThemedButton` - toggles carry their own click handler and re-spawn on flip.
pub fn toggle(on: bool, skin: UiSkin) -> impl Bundle {
    let phosphor = skin.is_phosphor();
    let radius = if phosphor { theme::RADIUS } else { 11.0 };
    let dot_radius = if phosphor { 1.0 } else { 9.0 };
    let (bg, border) = if on {
        (theme::PHOSPHOR.with_alpha(0.14), theme::PHOSPHOR)
    } else if phosphor {
        (
            Color::srgba(0.0, 0.0, 0.0, 0.4),
            theme::PHOSPHOR.with_alpha(0.35),
        )
    } else {
        (Color::srgba(0.0, 0.0, 0.0, 0.5), theme::CASE_EDGE)
    };
    let dot = if on {
        theme::PHOSPHOR
    } else if phosphor {
        theme::PHOSPHOR_MUTED
    } else {
        Color::srgb_u8(0x55, 0x63, 0x6c)
    };
    // The "on" dot glows (phosphor skin); the off dot is flat.
    let dot_shadow = if on && phosphor {
        Some(BoxShadow::new(
            theme::PHOSPHOR.with_alpha(0.35),
            Val::ZERO,
            Val::ZERO,
            Val::ZERO,
            Val::Px(5.0),
        ))
    } else {
        None
    };
    (
        Node {
            width: px(44),
            height: px(22),
            flex_shrink: 0.0,
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(radius)),
            justify_content: if on {
                JustifyContent::FlexEnd
            } else {
                JustifyContent::FlexStart
            },
            align_items: AlignItems::Center,
            padding: UiRect::all(px(2)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(bg),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            let mut dot_entity = parent.spawn((
                Node {
                    width: px(16),
                    height: px(16),
                    border_radius: BorderRadius::all(px(dot_radius)),
                    ..default()
                },
                BackgroundColor(dot),
            ));
            if let Some(shadow) = dot_shadow {
                dot_entity.insert(shadow);
            }
        })),
    )
}
