//! Shared UI theme and widgets for the Nova Protocol game.
//!
//! One source of truth for the in-game UI look (menu, editor, HUD chrome),
//! mirroring the web app's palette. `theme` holds the palette + metrics;
//! `widget` holds the themed button + selection machinery and small layout
//! helpers. Palette/metrics only - real web fonts are a separate concern.

pub mod theme;
pub mod widget;

pub mod prelude {
    pub use crate::{
        theme,
        widget::{
            button_on_setting, panel_header, register, separator, themed_button, ButtonValue,
            Selected, ThemedButton,
        },
    };
}
