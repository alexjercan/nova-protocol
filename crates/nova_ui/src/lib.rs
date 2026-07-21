//! Shared UI theme and widgets for the Nova Protocol game.
//!
//! One source of truth for the in-game UI look (menu, editor, HUD chrome),
//! mirroring the web app's palette. `theme` holds the palette + metrics;
//! `widget` holds the themed button + selection machinery and small layout
//! helpers. Palette/metrics only - real web fonts are a separate concern.

#![warn(missing_docs)]

pub mod theme;
pub mod widget;

/// Glob-import surface: `use nova_ui::prelude::*` brings the [`theme`] palette and
/// the themed-button widgets ([`themed_button`](widget::themed_button),
/// [`ThemedButton`](widget::ThemedButton), [`Selected`](widget::Selected), ...)
/// into scope.
pub mod prelude {
    pub use crate::{
        theme,
        widget::{
            button_on_setting, panel_header, register, separator, themed_button, ButtonValue,
            Selected, ThemedButton,
        },
    };
}
