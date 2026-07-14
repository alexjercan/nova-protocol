//! The Nova Protocol UI theme, mirroring the web app's `/wiki/sections/` page
//! (an "industrial HUD" look: deep navy panels, hard 1px borders, sharp 2px
//! corners, cyan/amber accents, crisp hover with no glow). Palette values are the
//! CSS variables from `web/src/style.css`.
//!
//! This is the single source of truth for the whole game UI (menu, editor, HUD).
//! Palette + metrics only - typography (real web fonts) is a separate concern.

use bevy::prelude::*;

// -- Palette (web/src/style.css CSS variables) --

/// Page background (`--space-1` #0b0f1c): the deep field behind everything.
pub const BG: Color = Color::srgb_u8(11, 15, 28);
/// Default panel/surface (`--panel` #141a2e): rails, cards, tooltips.
pub const PANEL: Color = Color::srgb_u8(20, 26, 46);
/// Raised/elevated surface (`--panel-2` #1a2138): hover fill.
pub const PANEL_RAISED: Color = Color::srgb_u8(26, 33, 56);
/// Standard 1px border (`--border` #233052).
pub const BORDER: Color = Color::srgb_u8(35, 48, 82);
/// Brightened border on hover/active (`--border-bright` #3a4d7a).
pub const BORDER_BRIGHT: Color = Color::srgb_u8(58, 77, 122);
/// Primary accent (`--cyan` #5cc8ff).
pub const CYAN: Color = Color::srgb_u8(92, 200, 255);
/// Highlight / active text (`--cyan-bright` #8fe0ff).
pub const CYAN_BRIGHT: Color = Color::srgb_u8(143, 224, 255);
/// Secondary accent (`--amber` #ffb877): badges, HP figures, ammo.
pub const AMBER: Color = Color::srgb_u8(255, 184, 119);
/// Primary text (`--text` #e8eefc).
pub const TEXT: Color = Color::srgb_u8(232, 238, 252);
/// Secondary/tertiary text (`--text-muted` #8b95b0).
pub const TEXT_MUTED: Color = Color::srgb_u8(139, 149, 176);

/// A cyan-tinted panel used as the SELECTED fill (there is no alpha compositing
/// in a solid `BackgroundColor`, so this is a pre-mixed cyan-over-panel).
pub const SELECTED_FILL: Color = Color::srgb_u8(24, 54, 78);

// -- Metrics --

/// Sharp corner radius (`--radius: 2px`).
pub const RADIUS: f32 = 2.0;
/// Hard 1px border width.
pub const BORDER_W: f32 = 1.0;
/// Placeholder/thumbnail icon size (the wiki `.wiki-child__icon` is 44x44).
pub const ICON: f32 = 44.0;
