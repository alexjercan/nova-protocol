//! The Nova Protocol UI theme: the authored format, its resolver, the shipped
//! base themes, and the functional gameplay colours that are NOT themed.
//!
//! The visual language is **NOVA OS** (green phosphor on a near-black screen
//! inside a dark moulded casing), carried verbatim from the accepted PoC
//! `web/design/nova_ui_rework_poc.html` (its `:root` tokens). Every one of those
//! tokens now lives in [`base`] as authored theme DATA rather than as a `const`
//! here, because a theme is mod content like a section or a ship is: the base
//! mod ships `base/phosphor` and `base/hardware`, and a mod adds a look by
//! declaring a new id.
//!
//! The four pieces:
//!
//! - [`config`] - the serde format a `Content::UiTheme` item carries;
//! - [`resolve`] - the inheritance walk, the palette resolution, and
//!   [`ActiveUiTheme`], the complete paint table the widgets read;
//! - [`base`] - the two shipped themes, as builders `content gen` serializes;
//! - [`registry`] - [`GameUiThemes`], [`SelectedUiTheme`] and the one system
//!   that resolves them into [`ActiveUiTheme`].
//!
//! # What is NOT themed
//!
//! [`semantic`] and [`combat`] below. Those carry gameplay MEANING rather than
//! style - a hostile reticle must be red and an ally green in every theme, so a
//! theme that could move them could make the game unreadable. They are the
//! documented exemption to the coverage contract; everything a player would
//! call chrome goes through [`ActiveUiTheme`].
//!
//! The site mirrors the same PoC `:root` block in `web/src/style.css`, and
//! `web/tests/theme.test.ts` parses both and fails on drift. It is
//! single-skin - it draws only the phosphor look. Change the PoC first, then
//! both consumers.
//!
//! Palette and metrics only - typography routes through [`crate::font::UiFont`].

pub mod base;
pub mod config;
pub mod registry;
pub mod resolve;

#[cfg(test)]
mod tests;

/// Glob-import surface for the theme: the authored format, the live resolved
/// theme and the semantic names screens ask it for.
pub mod prelude {
    pub use super::{
        config::{SliderMeter, UiThemeConfig, HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
        registry::{GameUiThemes, SelectedUiTheme, UiThemeDiagnostic, UiThemeSystems},
        resolve::{
            ActiveUiTheme, ButtonState, FieldState, OnOff, RowState, ThemeButton, ThemeIssue,
            UiColor, UiMetric,
        },
    };
}

pub use config::{SliderMeter, UiThemeConfig, HARDWARE_THEME_ID, PHOSPHOR_THEME_ID};
pub use registry::{GameUiThemes, SelectedUiTheme, UiThemeDiagnostic, UiThemeSystems};
pub use resolve::{
    lint_theme, resolve_theme, ActiveUiTheme, ButtonState, FieldState, OnOff, ResolvedBadge,
    ResolvedBar, ResolvedCheckbox, ResolvedFill, ResolvedHead, ResolvedRow, ResolvedSlider,
    ResolvedState, ResolvedSurface, ResolvedToggle, RowState, ThemeButton, ThemeIssue, UiColor,
    UiMetric,
};

/// Placeholder/thumbnail icon size (the wiki `.wiki-child__icon` is 44x44).
///
/// A LAYOUT number, not paint: a theme may not move it, so it stays a const
/// here rather than joining the three metrics a theme owns.
pub const ICON: f32 = 44.0;

/// The hairline every themed border is DRAWN at, in logical pixels.
///
/// Layout, not paint, which is why it is a `const` beside [`ICON`] rather than
/// a role: a `Node`'s `border` is a box-model size, so it changes what the
/// layout reserves and how the text inside sits. Both shipped themes ask for
/// 1px ([`UiMetric::BorderWidth`]), and a theme that wanted 2px would reflow
/// every screen rather than restyle it.
pub const BORDER_W: f32 = 1.0;

/// Semantic HUD accents: the meaning-carrying gameplay colours (threat, ally,
/// nav, objective, ...), centralized here so the HUD has ONE palette source.
///
/// These are the game's FUNCTIONAL colours (a hostile reticle must be red, an
/// ally green), which is why they are consts here rather than theme roles: a
/// theme that could move them could make the game unreadable. Values are the
/// canonical HUD literals verbatim.
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
    /// The comms channel: what a narrative cue that names no accent is drawn in.
    ///
    /// Semantic, not chrome, and a `const` rather than a theme role for a second
    /// reason: it is the serde DEFAULT of a cue's `accent` field, so it is
    /// baked into authored content at parse time and cannot depend on which
    /// theme happens to be live.
    pub const COMMS: Color = Color::srgb_u8(0x36, 0xa3, 0xff);
    /// The crew channel: a line spoken inside the player's own ship, in the
    /// instrument green the HUD reads its own numbers in. Authored into a cue
    /// the same way [`COMMS`] is, so it is a `const` for the same reason.
    pub const CREW: Color = Color::srgb_u8(0x36, 0xff, 0x79);
    /// The overheard channel: a line the player is not the addressee of, in
    /// the amber the HUD keeps for what demands attention.
    pub const OVERHEARD: Color = Color::srgb_u8(0xff, 0xb8, 0x4a);

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
            // Carried verbatim from the retired `theme::BLUE` (PoC `--blue`).
            assert_eq!(COMMS, Color::srgb_u8(0x36, 0xa3, 0xff));
            assert_eq!(CREW, Color::srgb_u8(0x36, 0xff, 0x79));
            assert_eq!(OVERHEARD, Color::srgb_u8(0xff, 0xb8, 0x4a));
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
            const {
                assert!(INBOUND.green < LOCK.green, "inbound is the reddest");
                assert!(LOCK.green < HOT.green, "hot metal is the most orange");
            }
        }
    }
}
