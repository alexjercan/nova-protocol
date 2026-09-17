//! The authored UI-theme format: the serde types a `Content::UiTheme` item
//! carries.
//!
//! A theme is DATA - a palette of named colour variables, three metrics, and a
//! table of semantic ROLES that name the paint of every player-facing control
//! in every state it has. Nothing here touches bevy: resolution into live
//! bevy `Color`s, gradients and shadows is [`super::resolve`]'s job, so a theme can
//! be parsed, linted and diffed by the offline content tools with no renderer.
//!
//! # What a theme may say
//!
//! Foreground, background and border colour; border width and corner radius;
//! solid and gradient fills; glow and drop shadows; and the paint of every
//! interaction state a role requires. Two roles additionally carry SHAPE,
//! because the shipped looks differ there and the difference is the look:
//! [`SliderRole::meter`] picks the block meter or the solid fill, and
//! [`BadgeRole::bracketed`] picks the `[TAG]` text form or the bordered chip.
//!
//! A theme may NOT say where anything goes: no position, size, padding, gap,
//! flex/grid, z order, visibility, pointer behaviour or font size. Those are
//! structure, and structure is the screen's, not the theme's.
//!
//! # Inheritance
//!
//! `inherit: None` marks a ROOT theme, which must define every role and every
//! required palette variable. `inherit: Some("base/phosphor")` marks a derived
//! theme: the roles it omits come from its parent whole. A role it DOES declare
//! replaces the inherited one COMPLETELY - individual interaction states are
//! never inherited, so a theme can never leave a control half-painted in two
//! looks.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The stable id of the phosphor terminal theme the base mod ships - the
/// shipped default, and the theme a missing or broken selection falls back to.
pub const PHOSPHOR_THEME_ID: &str = "base/phosphor";

/// The stable id of the light-3D hardware casing theme the base mod ships.
pub const HARDWARE_THEME_ID: &str = "base/hardware";

/// One authored UI theme: identity, what it inherits, its palette, its metrics
/// and its role table.
///
/// Registered by id into `GameUiThemes` exactly like any other content kind, so
/// a mod ships a look by declaring a new id and restyles a base one by
/// declaring its id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiThemeConfig {
    /// Stable id - the overlay key, the persisted setting's value, and what
    /// [`inherit`](Self::inherit) names. Conventionally `<mod>/<name>`.
    pub id: String,
    /// Display name for the Settings row.
    pub name: String,
    /// The theme this one derives from, or `None` for a root theme. A root
    /// theme must be complete; a derived theme inherits every role it omits.
    pub inherit: Option<String>,
    /// Named colour variables a [`ColorValue::Variable`] resolves through.
    /// Six-digit `#rrggbb`. Every [`super::UiColor`] name must be present in a
    /// root theme's map; extra names are free, and are how a theme gives its
    /// role table tones the screens never ask for directly.
    #[serde(default)]
    pub palette: BTreeMap<String, String>,
    /// Border width and the two corner radii, or `None` to inherit the
    /// parent's whole. A root theme must carry them.
    pub metrics: Option<UiMetrics>,
    /// The per-control paint tables.
    #[serde(default)]
    pub roles: UiRoles,
}

/// The three shared numbers every screen's chrome is drawn with.
///
/// Not per-role: these are what a border and a corner MEAN in a theme, and the
/// two shipped looks differ in exactly these three. A role that needs its own
/// radius carries one (see [`SurfacePaint::radius`]).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiMetrics {
    /// Hairline border width, in logical pixels.
    pub border_width: f32,
    /// Control corner radius (phosphor is square-ish, hardware moulded).
    pub radius: f32,
    /// Panel corner radius - larger than a control's in both shipped looks.
    pub panel_radius: f32,
}

/// A colour: a palette variable or a literal, at an alpha.
///
/// The wrapper's `alpha` OVERWRITES the source's rather than multiplying it, so
/// the same variable at `0.12` reads the same whichever theme defines it.
/// Transparent paint is alpha `0.0`, never a missing colour.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeColor {
    /// Where the RGB comes from.
    pub value: ColorValue,
    /// Opacity `0.0..=1.0`. An `f32` with a serde default of `1.0` rather than
    /// an `Option`: an opaque colour is the common case, and an absent alpha
    /// has one meaning, not two.
    #[serde(default = "opaque")]
    pub alpha: f32,
}

fn opaque() -> f32 {
    1.0
}

impl ThemeColor {
    /// A palette variable at full opacity.
    pub fn var(name: &str) -> Self {
        Self {
            value: ColorValue::Variable(name.to_string()),
            alpha: 1.0,
        }
    }

    /// A palette variable at an explicit alpha.
    pub fn var_alpha(name: &str, alpha: f32) -> Self {
        Self {
            value: ColorValue::Variable(name.to_string()),
            alpha,
        }
    }

    /// A six-digit `#rrggbb` literal at full opacity.
    pub fn literal(hex: &str) -> Self {
        Self {
            value: ColorValue::Literal(hex.to_string()),
            alpha: 1.0,
        }
    }

    /// A six-digit `#rrggbb` literal at an explicit alpha.
    pub fn literal_alpha(hex: &str, alpha: f32) -> Self {
        Self {
            value: ColorValue::Literal(hex.to_string()),
            alpha,
        }
    }
}

/// Where a [`ThemeColor`]'s RGB comes from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ColorValue {
    /// A name in the theme's (or an ancestor's) palette. Unknown names are a
    /// lint error, never a silent black.
    Variable(String),
    /// A six-digit `#rrggbb` literal. For a one-off tone no other role shares;
    /// anything reused belongs in the palette.
    Literal(String),
}

/// One stop of a gradient: a colour at a percentage along it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    /// The colour at this stop.
    pub color: ThemeColor,
    /// Position along the gradient, `0.0..=100.0`.
    pub percent: f32,
}

impl Stop {
    /// A stop at a percentage.
    pub fn new(color: ThemeColor, percent: f32) -> Self {
        Self { color, percent }
    }
}

/// Where a radial gradient is centred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadialAnchor {
    /// Centred on the top edge - the phosphor panel's bloom.
    Top,
    /// Centred on the surface.
    Center,
}

/// A surface's background: flat, a linear bevel, or a radial bloom.
///
/// Both gradient forms carry an explicit `base` - the flat colour PAINTED
/// UNDER the gradient. It is not derivable from the stops: the phosphor panel
/// is a dark screen face under a translucent glow, so its stops say nothing
/// about the colour a player actually sees. A theme states it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Fill {
    /// One flat colour.
    Solid(ThemeColor),
    /// A linear gradient at an angle in degrees (180 = top to bottom), which is
    /// how a moulded face gets its light.
    Linear {
        /// The flat colour under the gradient.
        base: ThemeColor,
        /// Gradient angle in degrees.
        degrees: f32,
        /// Two or more stops, in order.
        stops: Vec<Stop>,
    },
    /// A radial gradient - the phosphor panel's glow blooming from one edge.
    /// Always shaped to the farthest side, which is the only bloom the shipped
    /// looks draw.
    Radial {
        /// The flat colour under the gradient.
        base: ThemeColor,
        /// Where the gradient is centred.
        anchor: RadialAnchor,
        /// Two or more stops, in order.
        stops: Vec<Stop>,
    },
}

impl Fill {
    /// A flat colour fill.
    pub fn solid(color: ThemeColor) -> Self {
        Fill::Solid(color)
    }

    /// A top-to-bottom bevel over a base, with stops at the given percentages.
    pub fn linear(base: ThemeColor, stops: Vec<Stop>) -> Self {
        Fill::Linear {
            base,
            degrees: 180.0,
            stops,
        }
    }

    /// Nothing drawn: the surface behind shows through.
    pub fn none() -> Self {
        Fill::Solid(ThemeColor {
            value: ColorValue::Literal("#000000".to_string()),
            alpha: 0.0,
        })
    }
}

/// A glow or drop shadow cast by a surface.
///
/// One type for both: a drop shadow is an offset dark blur and a glow is an
/// un-offset coloured one, and separating them would be two names for one
/// primitive.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    /// The cast colour.
    pub color: ThemeColor,
    /// Vertical offset in logical pixels; `0.0` for a centred glow.
    pub offset_y: f32,
    /// Blur radius in logical pixels.
    pub blur: f32,
}

/// The complete paint of one control in one interaction state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatePaint {
    /// The background.
    pub fill: Fill,
    /// The border colour (width comes from [`UiMetrics::border_width`]).
    pub border: ThemeColor,
    /// The label colour.
    pub text: ThemeColor,
    /// A glow or shadow, or `None` for a flat face. `Option` because absence
    /// here has one defined meaning - cast nothing - and a "transparent shadow"
    /// would still cost a `BoxShadow` component per control.
    pub effect: Option<Effect>,
}

/// A button's paint in every state it resolves, in precedence order: disabled,
/// then selected-and-pressed, pressed, selected, hovered, normal.
///
/// Every state is explicit. A theme cannot leave one out and inherit it from
/// another state, because the six are what a button IS - a control that painted
/// five of them would be a control with a dead state nobody noticed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ButtonRole {
    /// Idle.
    pub normal: StatePaint,
    /// Pointer over it.
    pub hovered: StatePaint,
    /// Held down.
    pub pressed: StatePaint,
    /// The active choice of its group.
    pub selected: StatePaint,
    /// The active choice, held down.
    pub selected_pressed: StatePaint,
    /// Not interactive.
    pub disabled: StatePaint,
}

/// A plain bordered surface: a panel, a segmented container, a slider well.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfacePaint {
    /// The background.
    pub fill: Fill,
    /// The border colour.
    pub border: ThemeColor,
    /// A glow or shadow, or `None`.
    pub effect: Option<Effect>,
    /// This surface's own corner radius, or `None` to take
    /// [`UiMetrics::radius`]. The panel and its head take the panel radius by
    /// naming it here; a control takes the control radius by omitting it.
    pub radius: Option<f32>,
}

/// A panel header band: the rule under it and the three text tones on it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeadPaint {
    /// The band's bottom border.
    pub border: ThemeColor,
    /// The header title.
    pub title: ThemeColor,
    /// The rule that fills the rest of the row.
    pub rule: ThemeColor,
    /// The trailing tag chip's text.
    pub tag: ThemeColor,
}

/// A list row's three states. Selection outranks hover, so there is no
/// selected-and-hovered state to author.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowRole {
    /// Idle.
    pub normal: RowPaint,
    /// Pointer over it.
    pub hovered: RowPaint,
    /// The active row.
    pub selected: RowPaint,
}

/// A list row's paint. A row carries no text colour of its own: what it holds
/// is arbitrary (icons, badges, several spans), each already themed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowPaint {
    /// The background.
    pub fill: Fill,
    /// The border colour.
    pub border: ThemeColor,
}

/// How a slider shows its value.
///
/// SHAPE, not paint - and authored because the two shipped looks differ here
/// and the difference is the look: the terminal draws a row of lit blocks, the
/// hardware a continuous fill behind a moulded well.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SliderMeter {
    /// A row of discrete blocks, lit up to the value.
    Blocks {
        /// How many blocks the track holds.
        segments: usize,
        /// Gap between blocks, in logical pixels.
        gap: f32,
    },
    /// One continuous bar filled to the value.
    Fill,
}

impl SliderMeter {
    /// How many blocks this meter draws, or `None` when it is a solid fill.
    /// The block count is the THEME's, so a caller that counts children (a
    /// test, a layout that reserves room) asks the theme rather than a
    /// constant.
    pub fn segments(self) -> Option<usize> {
        match self {
            SliderMeter::Blocks { segments, .. } => Some(segments),
            SliderMeter::Fill => None,
        }
    }
}

/// A slider track: the well it sits in, how tall it is, and how it meters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliderRole {
    /// The well.
    pub surface: SurfacePaint,
    /// Track height in logical pixels. Authored because the block meter and the
    /// solid fill are not legible at one height.
    pub height: f32,
    /// How the value is shown.
    pub meter: SliderMeter,
    /// The lit part of the meter.
    pub lit: ThemeColor,
    /// The unlit part - a dark block in the terminal look, the well itself in
    /// the moulded one.
    pub unlit: ThemeColor,
}

/// A text field's four states. Error outranks focus, which outranks hover.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldRole {
    /// Idle.
    pub normal: StatePaint,
    /// Pointer over it.
    pub hovered: StatePaint,
    /// Holding the caret.
    pub focused: StatePaint,
    /// Holding an invalid value.
    pub error: StatePaint,
}

/// A checkbox: a small square that inverts when checked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckboxRole {
    /// The box's corner radius, or `None` to take [`UiMetrics::radius`].
    pub radius: Option<f32>,
    /// Unchecked. Its `text` is the (absent) glyph's colour.
    pub off: StatePaint,
    /// Checked. Its `text` is the glyph.
    pub on: StatePaint,
}

/// A pill toggle: a track with a knob that slides across it.
///
/// Held apart from [`CheckboxRole`] rather than sharing one on/off type: a
/// toggle has a knob and a checkbox does not, and one type for both would carry
/// a knob field the checkbox leaves meaningless.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToggleRole {
    /// The track's corner radius, or `None` to take [`UiMetrics::radius`].
    pub radius: Option<f32>,
    /// The knob's corner radius - square in the terminal look, round in the
    /// moulded one.
    pub knob_radius: f32,
    /// Off. Its `text` is the knob colour; its `effect` the knob's glow.
    pub off: StatePaint,
    /// On. Its `text` is the knob colour; its `effect` the knob's glow.
    pub on: StatePaint,
}

/// A status badge: the chip chrome, and whether it is a chip at all.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BadgeRole {
    /// SHAPE: `true` renders the terminal's `[TAG]` text form with no chrome,
    /// `false` the bordered chip. Authored for the same reason
    /// [`SliderMeter`] is - the two shipped looks differ here, and a badge that
    /// wore a chip in the terminal look would read as a different widget.
    pub bracketed: bool,
    /// Horizontal padding in logical pixels.
    pub padding_x: f32,
    /// Border width; `0.0` for the bracketed form.
    pub border_width: f32,
    /// Alpha applied to the badge family's colour for its border.
    pub border_alpha: f32,
    /// Alpha applied to the badge family's colour for its fill.
    pub fill_alpha: f32,
}

/// A scrollbar: the track it runs in and the thumb that rides it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarRole {
    /// The track behind the thumb.
    pub track: ThemeColor,
    /// The thumb.
    pub thumb: ThemeColor,
}

/// Every semantic role a theme paints.
///
/// `None` means INHERIT THIS ROLE WHOLE from the parent - absence has exactly
/// one defined behaviour, which is what an `Option` is for here. A root theme
/// (`inherit: None`) must fill every field; [`super::resolve`] rejects one that
/// does not, by name, at lint and again at load.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiRoles {
    /// The neutral button.
    pub button: Option<ButtonRole>,
    /// The call-to-action button - a permanent selection in both shipped looks.
    pub button_primary: Option<ButtonRole>,
    /// The destructive button.
    pub button_danger: Option<ButtonRole>,
    /// The border-only button.
    pub button_ghost: Option<ButtonRole>,
    /// A bordered panel surface.
    pub panel: Option<SurfacePaint>,
    /// A panel's header band.
    pub panel_head: Option<HeadPaint>,
    /// A list row.
    pub list_row: Option<RowRole>,
    /// A segmented control's recessed container.
    pub segmented: Option<SurfacePaint>,
    /// A slider track.
    pub slider_track: Option<SliderRole>,
    /// A text field.
    pub text_field: Option<FieldRole>,
    /// A checkbox.
    pub checkbox: Option<CheckboxRole>,
    /// A pill toggle.
    pub toggle: Option<ToggleRole>,
    /// A status badge.
    pub badge: Option<BadgeRole>,
    /// A scrollbar.
    pub scroll_bar: Option<BarRole>,
    /// A keycap chip.
    pub key_chip: Option<StatePaint>,
    /// A horizontal rule between sections.
    pub separator: Option<ThemeColor>,
}
