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

/// Semantic HUD accents: the meaning-carrying gameplay colours (threat, ally,
/// nav, objective, ...), centralized here so the HUD has ONE palette source.
///
/// These are the game's FUNCTIONAL colours (a hostile reticle must be red, an
/// ally green), distinct from the neutral chrome above - so they keep their own
/// tuned hues rather than snapping to the cyan/amber brand accents. Values are
/// the canonical HUD literals verbatim (task 20260714-214118): centralizing them
/// changes nothing visually. Per-widget tuned variants (the many slightly-
/// different combat reds/ambers) intentionally stay local to their file; only the
/// shared, exactly-repeated accents live here.
pub mod semantic {
    use bevy::prelude::Color;

    /// Navigation / flight-computer accent (nav crosshair, flight chips).
    pub const NAV: Color = Color::srgba(0.3, 0.9, 1.0, 0.9);
    /// Objective "do this now" accent (objectives panel, markers).
    pub const OBJECTIVE: Color = Color::srgba(1.0, 0.85, 0.3, 0.95);
    /// Threat / combat lock (hostile reticle, lock indicators, hostile faction) -
    /// the exactly-repeated combat red (reticle + lock + faction-hostile).
    pub const THREAT: Color = Color::srgba(1.0, 0.35, 0.3, 1.0);
    /// Own / allied target.
    pub const ALLY: Color = Color::srgba(0.35, 0.9, 0.55, 1.0);
    /// Neutral target (light steel).
    pub const NEUTRAL: Color = Color::srgba(0.85, 0.88, 0.9, 0.9);
    /// The recurring dark readout backdrop (health bar, focus meter).
    pub const BACKDROP: Color = Color::srgba(0.15, 0.15, 0.15, 0.8);

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The HUD consts were centralized here at their EXACT original values
        /// (task 20260714-214118), so the restyle changed nothing visually. Pin
        /// them: any future edit that shifts a semantic hue must be deliberate,
        /// because it moves every HUD widget that references it.
        #[test]
        fn semantic_accents_match_the_original_hud_literals() {
            assert_eq!(NAV, Color::srgba(0.3, 0.9, 1.0, 0.9));
            assert_eq!(OBJECTIVE, Color::srgba(1.0, 0.85, 0.3, 0.95));
            assert_eq!(THREAT, Color::srgba(1.0, 0.35, 0.3, 1.0));
            assert_eq!(ALLY, Color::srgba(0.35, 0.9, 0.55, 1.0));
            assert_eq!(NEUTRAL, Color::srgba(0.85, 0.88, 0.9, 0.9));
            assert_eq!(BACKDROP, Color::srgba(0.15, 0.15, 0.15, 0.8));
        }
    }
}
