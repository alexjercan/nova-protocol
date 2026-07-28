//! Shared UI theme and widgets for the Nova Protocol game.
//!
//! One source of truth for the in-game UI look (menu, editor, HUD chrome),
//! built on the NOVA OS palette. `theme` holds the palette + metrics; `skin`
//! holds the [`UiSkin`](skin::UiSkin) selector (phosphor terminal vs hardware
//! casing); `units` holds the player-facing distance/speed formatting policy;
//! `widget` holds the skin-aware themed button + selection machinery and small
//! layout helpers; `font` holds the shared UI typeface handle preloaded at
//! startup.

#![warn(missing_docs)]

pub mod font;
pub mod skin;
pub mod theme;
pub mod units;
pub mod widget;

/// Glob-import surface: `use nova_ui::prelude::*` brings the [`theme`] palette,
/// the [`UiSkin`](skin::UiSkin) selector, the [`units`] formatters and the
/// themed-button widgets ([`themed_button`](widget::themed_button),
/// [`ThemedButton`](widget::ThemedButton), [`Selected`](widget::Selected), ...)
/// into scope.
pub mod prelude {
    pub use crate::{
        font::UiFont,
        skin::UiSkin,
        theme, units,
        widget::{
            badge, button, button_on_setting, checkbox, list_row, menu_button, panel, panel_head,
            panel_header, register, segmented, separator, slider_track, themed_button, BadgeKind,
            ButtonSpec, ButtonValue, ButtonVariant, Selected, ThemedButton,
        },
    };
}
