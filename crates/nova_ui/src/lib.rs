//! Shared UI theme and widgets for the Nova Protocol game.
//!
//! One source of truth for the in-game UI look (menu, editor, HUD chrome),
//! built on the NOVA OS palette. `theme` holds the palette + metrics; `skin`
//! holds the [`UiSkin`](skin::UiSkin) selector (phosphor terminal vs hardware
//! casing); `units` holds the player-facing distance/speed formatting policy;
//! `widget` holds the skin-aware themed button + selection machinery and small
//! layout helpers; `hud` holds the flight-HUD chip language (phosphor-only
//! chrome projected over the world); `font` holds the shared UI typeface handle
//! preloaded at startup; `status_bar` holds the top-right metrics bar;
//! `input_mode` holds the one arbiter that decides who owns the keyboard.

#![warn(missing_docs)]

use bevy::prelude::*;

pub mod font;
pub mod hud;
pub mod input_mode;
pub mod screen;
pub mod skin;
pub mod status_bar;
pub mod theme;
pub mod units;
pub mod widget;

/// Everything `nova_ui` needs running in an app: the themed-widget observers
/// and skin reconcilers, the shared font router, and the status bar driver.
///
/// It is app-global rather than per-screen - doubled observers would write
/// every colour twice per interaction - so the several plugins that use themed
/// widgets (menu, editor, gameplay) each add it only when it is absent:
///
/// ```rust
/// # use bevy::prelude::*;
/// # use nova_ui::NovaUiPlugin;
/// # fn build(app: &mut App) {
/// if !app.is_plugin_added::<NovaUiPlugin>() {
///     app.add_plugins(NovaUiPlugin);
/// }
/// # }
/// ```
pub struct NovaUiPlugin;

impl Plugin for NovaUiPlugin {
    fn build(&self, app: &mut App) {
        trace!("NovaUiPlugin: build");

        input_mode::build(app);
        widget::build(app);
        status_bar::build(app);
        screen::build(app);
    }
}

/// Glob-import surface: `use nova_ui::prelude::*` brings the [`theme`] palette,
/// the [`UiSkin`](skin::UiSkin) selector, the [`units`] formatters and the
/// themed-button widgets ([`themed_button`](widget::themed_button),
/// [`ThemedButton`](widget::ThemedButton), [`Selected`](widget::Selected), ...)
/// into scope, plus the [`screen`] composition helpers and the [`status_bar`]
/// names the composition root spawns.
///
/// Each module owns its own `prelude`, so publishing a new name is a one-file
/// edit rather than an edit here as well.
pub mod prelude {
    pub use crate::{
        font::prelude::*, hud::prelude::*, input_mode::prelude::*, screen::prelude::*,
        skin::prelude::*, status_bar::prelude::*, theme, units, widget::prelude::*,
    };
}
