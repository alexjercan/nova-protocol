//! The Nova Protocol UI theme.
//!
//! The primary palette is the **NOVA OS** language (green phosphor on a
//! near-black screen inside a dark moulded casing), carried verbatim from the
//! accepted PoC `web/design/nova_ui_rework_poc.html` (its `:root` tokens). The
//! widget layer ([`crate::widget`]) renders every control from these tokens in
//! one of two skins - the phosphor CLI look (default) and the light-3D hardware
//! casing (see [`crate::skin::UiSkin`]).
//!
//! The flat navy/cyan `web/src/style.css` palette that this theme used to mirror
//! has been fully retired. Site and game are no longer two hand-synced lists:
//! `web/src/style.css` now mirrors the SAME PoC `:root` block this module does,
//! and `web/tests/theme.test.ts` parses both and fails on drift. Change the PoC
//! first, then both consumers.
//!
//! The site is single-skin: it draws only the PHOSPHOR look, and its test fails
//! if any hardware-skin material token is consumed. The game keeps both skins -
//! only here is [`crate::skin::UiSkin`] a user choice.
//!
//! Palette + metrics only - typography routes through [`crate::font::UiFont`].

use bevy::prelude::*;

// NOVA OS palette (the PoC :root tokens).

/// Deepest field behind everything (`--space`).
pub const SPACE: Color = Color::srgb_u8(0x03, 0x06, 0x0b);

/// Casing gradient tone 0 - the darkest case face (`--case-0`).
pub const CASE_0: Color = Color::srgb_u8(0x0a, 0x0d, 0x10);
/// Casing gradient tone 1 (`--case-1`).
pub const CASE_1: Color = Color::srgb_u8(0x16, 0x1b, 0x20);
/// Casing gradient tone 2 (`--case-2`).
pub const CASE_2: Color = Color::srgb_u8(0x23, 0x2a, 0x31);
/// Casing gradient tone 3 - the lightest, top of a moulded face (`--case-3`).
pub const CASE_3: Color = Color::srgb_u8(0x2f, 0x38, 0x3f);
/// The near-black seam between case pieces (`--case-edge`).
pub const CASE_EDGE: Color = Color::srgb_u8(0x05, 0x07, 0x0a);

/// Hover face top tone (`--face-hot` 0%).
pub const CASE_HOT_HI: Color = Color::srgb_u8(0x3a, 0x44, 0x4c);
/// Hover face mid tone (`--face-hot` 55%).
pub const CASE_HOT_MID: Color = Color::srgb_u8(0x22, 0x2a, 0x31);
/// Hover face bottom tone (`--face-hot` 100%).
pub const CASE_HOT_LO: Color = Color::srgb_u8(0x12, 0x17, 0x1b);

/// CRT screen surface, darkest (`--screen-0`).
pub const SCREEN_0: Color = Color::srgb_u8(0x00, 0x13, 0x04);
/// CRT screen surface, lifted (`--screen-1`).
pub const SCREEN_1: Color = Color::srgb_u8(0x00, 0x2b, 0x0f);

/// Primary phosphor green (`--phosphor`): live text, active borders, glyphs.
pub const PHOSPHOR: Color = Color::srgb_u8(0x36, 0xff, 0x79);
/// Dim phosphor (`--phosphor-dim`): secondary text, idle fills.
pub const PHOSPHOR_DIM: Color = Color::srgb_u8(0x19, 0xa6, 0x4f);
/// Muted phosphor (`--phosphor-muted`): labels, section heads, tags.
pub const PHOSPHOR_MUTED: Color = Color::srgb_u8(0x0d, 0x6e, 0x35);
/// Bright phosphor, top of the primary-button gradient (`#7dffab`).
pub const PHOSPHOR_HI: Color = Color::srgb_u8(0x7d, 0xff, 0xab);
/// Deep phosphor, bottom of the primary-button gradient (`#12b552`).
pub const PHOSPHOR_LO: Color = Color::srgb_u8(0x12, 0xb5, 0x52);

/// Amber accent (`--amber`): key-chips, hardware selection, warnings.
pub const AMBER_NOVA: Color = Color::srgb_u8(0xff, 0xb8, 0x4a);
/// Bright amber, top of the amber selection gradient (`#ffd07a`).
pub const AMBER_HI: Color = Color::srgb_u8(0xff, 0xd0, 0x7a);
/// Deep amber, bottom of the amber selection gradient (`#e6952f`).
pub const AMBER_LO: Color = Color::srgb_u8(0xe6, 0x95, 0x2f);

/// Orange accent (`--orange`).
pub const ORANGE: Color = Color::srgb_u8(0xff, 0x7b, 0x2d);
/// Red accent / danger family (`--red`).
pub const RED: Color = Color::srgb_u8(0xff, 0x4e, 0x42);
/// Blue accent / info (`--blue`).
pub const BLUE: Color = Color::srgb_u8(0x36, 0xa3, 0xff);

/// Phosphor body text (`--text` #b9ffc9): the greenish white of readable copy.
pub const SCREEN_TEXT: Color = Color::srgb_u8(0xb9, 0xff, 0xc9);
/// The dark ink of glyphs sitting on a bright (inverted phosphor / amber) fill.
pub const INK: Color = Color::srgb_u8(0x04, 0x14, 0x0a);

// -- Metrics --

/// Sharp corner radius (phosphor uses 2px corners).
pub const RADIUS: f32 = 2.0;
/// Softer corner radius for the hardware skin's moulded controls (7px).
pub const RADIUS_HW: f32 = 7.0;
/// Panel corner radius (10px).
pub const PANEL_RADIUS: f32 = 10.0;
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
/// the canonical HUD literals verbatim, so centralizing them changed nothing
/// visually.
///
/// A colour whose ALPHA varies per widget cannot live here, because a `Color`
/// carries one. Those are [`super::combat`]: a hue family plus the widget's own
/// alpha at the point of use.
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

        /// The HUD consts were centralized here at their EXACT original values,
        /// so the restyle changed nothing visually. Pin
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

/// The combat hue FAMILIES: the reds every targeting widget is drawn in, held
/// apart from [`semantic`] because each widget carries its own alpha over the
/// same hue - a dim marker and the selected one are one colour at two
/// opacities, not two colours.
///
/// Three families, three meanings, and they are meant to stay three:
/// something is INBOUND at you, your guns are LOCKED on something, or your
/// weapons are HOT. Before this they were eight literals within 0.05 of each
/// other across six files, which is drift rather than meaning - the hues that
/// moved onto a family here are the ones that had drifted off it.
pub mod combat {
    use bevy::prelude::{Color, Srgba};

    /// Something is coming AT the player: an incoming torpedo, and the
    /// candidates the radar sweep is offering. The most alarming of the three.
    pub const INBOUND: Srgba = Srgba::rgb(1.0, 0.22, 0.22);

    /// The player's guns are ON something: the radar combat crosshair, the
    /// framed target's highlight, the torpedo focus meter, an unselected
    /// component marker.
    pub const LOCK: Srgba = Srgba::rgb(1.0, 0.35, 0.25);

    /// Hot metal: the safety is off, or this is the one part the fine lock has
    /// picked. Warmer than [`LOCK`] on purpose - it reads as heat, and it is
    /// the family the target inset's frame wears while the weapons are up.
    pub const HOT: Srgba = Srgba::rgb(1.0, 0.45, 0.3);

    /// One family at one widget's opacity.
    ///
    /// `const` so a widget's colour is still a `const`, which is what keeps
    /// these greppable and comparable in a test.
    pub const fn at(family: Srgba, alpha: f32) -> Color {
        Color::Srgba(Srgba::new(family.red, family.green, family.blue, alpha))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The families are apart enough to read apart, and `at` moves nothing
        /// but the alpha.
        #[test]
        fn a_family_keeps_its_hue_at_every_opacity() {
            assert_eq!(at(LOCK, 0.22), Color::srgba(1.0, 0.35, 0.25, 0.22));
            assert_eq!(at(LOCK, 0.9), Color::srgba(1.0, 0.35, 0.25, 0.9));
            assert_eq!(at(HOT, 0.95), Color::srgba(1.0, 0.45, 0.3, 0.95));
            assert_eq!(at(INBOUND, 0.45), Color::srgba(1.0, 0.22, 0.22, 0.45));
        }

        /// The three are ordered by how alarming they are, coolest last: an
        /// inbound torpedo is the reddest thing on the screen and hot metal
        /// the most orange. A future edit that crosses them over has collapsed
        /// the distinction the families exist to carry.
        #[test]
        fn the_families_stay_in_their_order() {
            assert!(INBOUND.green < LOCK.green, "inbound is the reddest");
            assert!(LOCK.green < HOT.green, "hot metal is the most orange");
        }
    }
}
