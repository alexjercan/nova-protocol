//! The editor's visual theme, mirroring the web wiki's `/wiki/sections/` page
//! (an "industrial HUD" look: deep navy panels, hard 1px borders, sharp 2px
//! corners, cyan/amber accents, crisp hover with no glow). Palette values are
//! the CSS variables from `web/src/style.css` (task 20260714-204219).
//!
//! Scoped to `nova_editor` deliberately: the spike parked re-theming `nova_menu`
//! to the same palette as a later task, so these consts live here, not in a
//! shared crate.

use bevy::prelude::*;

// -- Palette (web/src/style.css CSS variables) --

/// Page background (`--space-1` #0b0f1c): the deep field behind everything.
pub(crate) const BG: Color = Color::srgb_u8(11, 15, 28);
/// Default panel/surface (`--panel` #141a2e): rails, cards, tooltips.
pub(crate) const PANEL: Color = Color::srgb_u8(20, 26, 46);
/// Raised/elevated surface (`--panel-2` #1a2138): hover fill.
pub(crate) const PANEL_RAISED: Color = Color::srgb_u8(26, 33, 56);
/// Standard 1px border (`--border` #233052).
pub(crate) const BORDER: Color = Color::srgb_u8(35, 48, 82);
/// Brightened border on hover/active (`--border-bright` #3a4d7a).
pub(crate) const BORDER_BRIGHT: Color = Color::srgb_u8(58, 77, 122);
/// Primary accent (`--cyan` #5cc8ff).
pub(crate) const CYAN: Color = Color::srgb_u8(92, 200, 255);
/// Highlight / active text (`--cyan-bright` #8fe0ff).
pub(crate) const CYAN_BRIGHT: Color = Color::srgb_u8(143, 224, 255);
/// Secondary accent (`--amber` #ffb877): the "soon" badge, HP figures.
pub(crate) const AMBER: Color = Color::srgb_u8(255, 184, 119);
/// Primary text (`--text` #e8eefc).
pub(crate) const TEXT: Color = Color::srgb_u8(232, 238, 252);
/// Secondary/tertiary text (`--text-muted` #8b95b0).
pub(crate) const TEXT_MUTED: Color = Color::srgb_u8(139, 149, 176);

/// A cyan-tinted panel used as the SELECTED fill (there is no alpha compositing
/// in a solid `BackgroundColor`, so this is a pre-mixed cyan-over-panel).
pub(crate) const SELECTED_FILL: Color = Color::srgb_u8(24, 54, 78);

// -- Metrics --

/// Sharp corner radius (`--radius: 2px`).
pub(crate) const RADIUS: f32 = 2.0;
/// Hard 1px border width.
pub(crate) const BORDER_W: f32 = 1.0;
/// Placeholder component icon size (the wiki `.wiki-child__icon` is 44x44).
pub(crate) const ICON: f32 = 44.0;

/// Left rail width (px). Kept narrow so the rail + drawer stay clear of screen
/// centre on the 1024-wide window, where the editor preview ship projects - a
/// UI panel over that point would block the placement raycast (see the drawer).
pub(crate) const RAIL_W: f32 = 150.0;
/// Component drawer width (px). RAIL_W + DRAWER_W = 430 < 512 (half of 1024),
/// so the centred build area stays pickable.
pub(crate) const DRAWER_W: f32 = 280.0;
