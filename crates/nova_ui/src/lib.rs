//! Shared UI theme and widgets for the Nova Protocol game.
//!
//! One source of truth for the in-game UI look (menu, editor, HUD chrome),
//! mirroring the web app's palette. `theme` holds the palette + metrics;
//! `units` holds the player-facing distance/speed formatting policy; `widget`
//! holds the themed button + selection machinery and small layout helpers;
//! `font` holds the shared UI typeface handle preloaded at startup.

#![warn(missing_docs)]

pub mod font;
pub mod theme;
pub mod units;
pub mod widget;

/// Glob-import surface: `use nova_ui::prelude::*` brings the [`theme`] palette,
/// the [`units`] formatters and the themed-button widgets
/// ([`themed_button`](widget::themed_button),
/// [`ThemedButton`](widget::ThemedButton), [`Selected`](widget::Selected), ...)
/// into scope.
pub mod prelude {
    pub use crate::{
        font::UiFont,
        theme, units,
        widget::{
            button_on_setting, panel_header, register, separator, themed_button, ButtonValue,
            Selected, ThemedButton,
        },
    };
}
