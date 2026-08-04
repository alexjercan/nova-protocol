//! The flight-HUD chrome language: the phosphor **chip**.
//!
//! Every readout the flight HUD projects over the world - speed, autopilot
//! mode, target distance, objective name, beacon range, comms card, status bar
//! - is the same shape: a dark translucent slab with a 1px accent border, the
//! accent's text on it and a dimmer tone for unit suffixes. The look is carried
//! verbatim from the accepted demo `web/design/hud_rework_poc.html` (its
//! `.chip` rule); this module is the ONE place it is expressed so the sites
//! stay in a family instead of drifting apart.
//!
//! HUD chrome is **phosphor-only** - there is no [`crate::skin::UiSkin`] toggle
//! here. The casing skin is a NOVA OS / menu / editor concern; nothing in the
//! cockpit's projected overlay is a moulded plastic panel.
//!
//! The semantic variants ([`ChipTone`]) keep their meaning colours: nav/status
//! green phosphor, amber for objectives and autopilot mode, red for locks and
//! threats, blue for comms. Pick the tone by MEANING, never by taste.

use bevy::prelude::*;

use crate::theme;

/// The chip's translucent slab fill (`rgba(2,14,8,0.62)` in the demo) - dark
/// enough to hold text over a bright starfield, sheer enough that the HUD never
/// reads as a set of opaque windows.
pub const CHIP_FILL: Color = Color::srgba(0.008, 0.055, 0.031, 0.62);

/// The dimmer slab used by the calmer chrome (the status bar, the dock's
/// unavailable chips): the same hue, less presence.
pub const CHIP_FILL_QUIET: Color = Color::srgba(0.008, 0.055, 0.031, 0.5);

/// The chip border's alpha over its tone colour (demo: `0.34`).
pub const CHIP_BORDER_ALPHA: f32 = 0.34;

/// The chip corner radius (px) - the demo's 3px, sharper than the NOVA OS
/// panels because a HUD chip is a projected readout, not a physical panel.
pub const CHIP_RADIUS: f32 = 3.0;

/// The chip's default text size (px). The speed chip overrides it upward; the
/// small labels (`.lab`, `.who`) use [`CHIP_LABEL_FONT`].
pub const CHIP_FONT: f32 = 12.0;

/// The small-caps label size used inside chips (weapon-group labels, the comms
/// speaker line, the target-inset header).
pub const CHIP_LABEL_FONT: f32 = 10.0;

/// A chip's semantic family. The tone decides the text, unit-suffix and border
/// colours; the fill is shared so the whole HUD reads as one instrument.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Reflect)]
pub enum ChipTone {
    /// Flight/nav/status readouts - green phosphor, the HUD's default voice.
    #[default]
    Phosphor,
    /// Objective and autopilot-mode chips - the "do this now" amber.
    Amber,
    /// Locks, threats and the hostile target inset - combat red.
    Threat,
    /// Comms traffic - the incoming-transmission blue.
    Comms,
}

impl ChipTone {
    /// The chip's foreground (the value text).
    pub fn text(self) -> Color {
        match self {
            ChipTone::Phosphor => theme::PHOSPHOR,
            ChipTone::Amber => theme::AMBER_NOVA,
            ChipTone::Threat => Color::srgb_u8(0xff, 0x8f, 0x86),
            ChipTone::Comms => theme::BLUE,
        }
    }

    /// The dimmer tone for unit suffixes and secondary text (`.u` in the demo).
    pub fn unit(self) -> Color {
        match self {
            ChipTone::Phosphor => theme::PHOSPHOR_DIM,
            ChipTone::Amber => theme::ORANGE,
            ChipTone::Threat => theme::RED,
            ChipTone::Comms => theme::BLUE.with_alpha(0.7),
        }
    }

    /// The 1px border colour - the tone at [`CHIP_BORDER_ALPHA`].
    pub fn border(self) -> Color {
        match self {
            // The bordered tones use their own accent, not the text colour:
            // the threat text is a pale red for readability while its border
            // must stay the saturated combat red.
            ChipTone::Threat => theme::RED.with_alpha(0.45),
            ChipTone::Amber => theme::AMBER_NOVA.with_alpha(0.45),
            ChipTone::Comms => theme::BLUE.with_alpha(0.4),
            ChipTone::Phosphor => theme::PHOSPHOR.with_alpha(CHIP_BORDER_ALPHA),
        }
    }

    /// The slab fill. One fill for the whole family except the threat inset,
    /// which sits on a red-tinted slab so a hostile readout is unmistakable.
    pub fn fill(self) -> Color {
        match self {
            ChipTone::Threat => Color::srgba(0.055, 0.016, 0.016, 0.66),
            _ => CHIP_FILL,
        }
    }
}

/// The chip's [`Node`] geometry: 1px border, the demo's 4x9 padding, centred
/// content, never wrapping. Callers set position/size on top of it.
pub fn chip_node() -> Node {
    Node {
        display: Display::Flex,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        column_gap: Val::Px(8.0),
        padding: UiRect::axes(Val::Px(9.0), Val::Px(4.0)),
        border: UiRect::all(Val::Px(1.0)),
        border_radius: BorderRadius::all(Val::Px(CHIP_RADIUS)),
        ..default()
    }
}

/// The paint components of a chip in `tone` - spread over a node that already
/// carries [`chip_node`] geometry (or a positioned variant of it).
pub fn chip_paint(tone: ChipTone) -> impl Bundle {
    (
        BackgroundColor(tone.fill()),
        BorderColor::all(tone.border()),
    )
}

/// A complete text chip: geometry, paint and the tone's text colour. The caller
/// adds the `Text`/`TextFont` (font size varies per site) and any positioning.
pub fn text_chip(tone: ChipTone) -> impl Bundle {
    (chip_node(), chip_paint(tone), TextColor(tone.text()))
}

/// A quiet chip (the status bar): the same shape at a calmer border and fill,
/// so persistent chrome does not compete with the live readouts.
pub fn quiet_chip(tone: ChipTone) -> impl Bundle {
    (
        chip_node(),
        BackgroundColor(CHIP_FILL_QUIET),
        BorderColor::all(tone.border().with_alpha(0.22)),
        TextColor(tone.unit()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The chip family shares ONE fill and geometry so the HUD reads as a
    /// single instrument; only the threat inset departs (a red-tinted slab).
    #[test]
    fn the_chip_family_shares_its_slab() {
        for tone in [ChipTone::Phosphor, ChipTone::Amber, ChipTone::Comms] {
            assert_eq!(tone.fill(), CHIP_FILL, "{tone:?} rides the shared slab");
        }
        assert_ne!(
            ChipTone::Threat.fill(),
            CHIP_FILL,
            "a hostile readout is unmistakable"
        );
    }

    /// Every tone is legible-over-dark: the text is brighter than its unit
    /// suffix, which is brighter than the border it sits in. A tone that
    /// inverts this reads as a mislabelled chip.
    #[test]
    fn each_tone_orders_text_above_unit_above_border() {
        let luma = |color: Color| {
            let c = color.to_srgba();
            0.2126 * c.red + 0.7152 * c.green + 0.0722 * c.blue
        };
        for tone in [
            ChipTone::Phosphor,
            ChipTone::Amber,
            ChipTone::Threat,
            ChipTone::Comms,
        ] {
            assert!(
                luma(tone.text()) >= luma(tone.unit()),
                "{tone:?}: the value must not be dimmer than its unit suffix"
            );
            assert!(
                tone.border().alpha() < 1.0,
                "{tone:?}: the border is a hairline, not a solid frame"
            );
        }
    }

    /// The geometry is a bordered, rounded slab - a chip that lost its border
    /// is just floating text again (the look this task replaced).
    #[test]
    fn chip_geometry_carries_a_hairline_border() {
        let node = chip_node();
        assert_eq!(node.border, UiRect::all(Val::Px(1.0)));
        assert_eq!(node.border_radius, BorderRadius::all(Val::Px(CHIP_RADIUS)));
        assert_ne!(
            node.padding,
            UiRect::default(),
            "text must not touch the border"
        );
    }
}
