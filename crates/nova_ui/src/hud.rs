//! The flight-HUD chrome language: the phosphor **chip**.
//!
//! Every readout the flight HUD projects over the world - speed, autopilot
//! mode, target distance, objective name, beacon range, comms card, status
//! bar - is the same shape: a dark translucent slab with a 1px accent border,
//! the accent's text on it and a dimmer tone for unit suffixes. The look is
//! carried verbatim from the accepted demo `web/design/hud_rework_poc.html`
//! (its `.chip` rule); this module is the ONE place it is expressed so the
//! sites stay in a family instead of drifting apart.
//!
//! HUD chrome follows the active UI theme like the rest of the chrome does, so
//! every chip method takes an [`ActiveUiTheme`]. What it does NOT take from the
//! theme is which tone a readout wears - [`ChipTone`] is a MEANING vocabulary
//! (nav/status green, amber for objectives and autopilot mode, red for locks
//! and threats, blue for comms), and a theme only decides what those four
//! families look like, never which one a chip gets. Pick the tone by MEANING,
//! never by taste.
//!
//! The projected reticles, lock crosshairs and faction markers stay off the
//! theme entirely - those are [`theme::semantic`] and [`theme::combat`], the
//! documented exemption.

/// Glob-import surface for the flight-HUD chip language.
pub mod prelude {
    pub use super::{
        body_colour, chip_node, chip_paint, quiet_chip, text_chip, ChipInk, ChipText, ChipTone,
        ThemedChip,
    };
}

use bevy::{platform::collections::HashSet, prelude::*};

use crate::theme::{ActiveUiTheme, UiColor, UiThemeSystems};

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
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Reflect, serde::Serialize, serde::Deserialize,
)]
pub enum ChipTone {
    /// Flight/nav/status readouts - the HUD's default voice, drawn in whatever
    /// the active theme calls its primary ink.
    #[default]
    Readout,
    /// Objective and autopilot-mode chips - the "do this now" amber.
    Amber,
    /// Locks, threats and the hostile target inset - combat red.
    Threat,
    /// Comms traffic - the incoming-transmission blue.
    Comms,
}

impl ChipTone {
    /// The semantic colour this tone's family is drawn from.
    fn family(self) -> UiColor {
        match self {
            ChipTone::Readout => UiColor::Primary,
            ChipTone::Amber => UiColor::Accent,
            ChipTone::Threat => UiColor::Danger,
            ChipTone::Comms => UiColor::Info,
        }
    }

    /// The chip's foreground (the value text).
    pub fn text(self, theme: &ActiveUiTheme) -> Color {
        match self {
            // The threat text is a PALE red for readability over a red-tinted
            // slab, while its border stays the saturated family colour. Lifted
            // from the family rather than authored beside it, so a theme that
            // moves danger moves both together.
            ChipTone::Threat => body_colour(theme.color(UiColor::Danger)),
            other => theme.color(other.family()),
        }
    }

    /// The dimmer tone for unit suffixes and secondary text (`.u` in the demo).
    pub fn unit(self, theme: &ActiveUiTheme) -> Color {
        match self {
            ChipTone::Readout => theme.color(UiColor::Secondary),
            ChipTone::Amber => theme.color(UiColor::AccentLow),
            ChipTone::Threat => theme.color(UiColor::Danger),
            ChipTone::Comms => theme.color_alpha(UiColor::Info, 0.7),
        }
    }

    /// The 1px border colour - the tone at [`CHIP_BORDER_ALPHA`].
    pub fn border(self, theme: &ActiveUiTheme) -> Color {
        let alpha = match self {
            ChipTone::Threat | ChipTone::Amber => 0.45,
            ChipTone::Comms => 0.4,
            ChipTone::Readout => CHIP_BORDER_ALPHA,
        };
        theme.color_alpha(self.family(), alpha)
    }

    /// The slab fill. One fill for the whole family except the threat inset,
    /// which sits on a red-tinted slab so a hostile readout is unmistakable.
    pub fn fill(self) -> Color {
        match self {
            ChipTone::Threat => Color::srgba(0.055, 0.016, 0.016, 0.66),
            _ => CHIP_FILL,
        }
    }

    /// The tone as a colour to READ a sentence in, rather than to label with.
    ///
    /// A chip carries two or three words and can afford full saturation; a
    /// comms line is a paragraph, and the same accent under it is tiring and
    /// low-contrast against the slab. This is the accent lifted
    /// [`BODY_LIFT`] of the way to white, which keeps the family recognisable
    /// while the words stay the brightest thing on the card.
    ///
    /// Derived rather than authored so a tone cannot ship a reading colour that
    /// disagrees with its own accent - and so a cue a MOD accents gets one
    /// without picking a second hex value out of the air.
    pub fn body(self, theme: &ActiveUiTheme) -> Color {
        body_colour(self.text(theme))
    }
}

/// Any accent as a colour to READ a sentence in.
///
/// [`ChipTone::body`]'s rule, opened up to an accent nothing in this crate
/// named: an authored narrative cue carries its own colour, and the reading
/// copy under it has to be derived from that rather than authored beside it -
/// two hex values that can disagree is a card drawn in two voices.
pub fn body_colour(accent: Color) -> Color {
    let accent = accent.to_srgba();
    Color::srgb(lift(accent.red), lift(accent.green), lift(accent.blue))
}

/// How far [`body_colour`] lifts an accent toward white.
const BODY_LIFT: f32 = 0.75;

/// One component of [`BODY_LIFT`], in linear-free sRGB space - the same space
/// the palette constants are written in, so the result reads as a paler version
/// of the hex it came from.
fn lift(component: f32) -> f32 {
    component + (1.0 - component) * BODY_LIFT
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

/// Marks a HUD chip, so [`reconcile_chips`] paints its slab and its hairline
/// border from the active theme and repaints them when the theme changes.
///
/// A chip spawns UNPAINTED, like every other themed widget: a bundle factory
/// cannot reach the world, so paint that is baked in at spawn is paint that
/// goes stale the moment the player changes theme.
#[derive(Component, Clone, Copy, Debug)]
pub struct ThemedChip {
    tone: ChipTone,
    /// A quiet chip wears the calmer slab and a border at a third of the alpha,
    /// so persistent chrome does not compete with the live readouts.
    quiet: bool,
}

impl ThemedChip {
    /// A live readout chip.
    pub fn loud(tone: ChipTone) -> Self {
        Self { tone, quiet: false }
    }

    /// A calm chrome chip (the status bar, the dock's unavailable keys).
    pub fn quiet(tone: ChipTone) -> Self {
        Self { tone, quiet: true }
    }

    /// This chip's `(fill, border)` in `theme`.
    fn paint(self, theme: &ActiveUiTheme) -> (Color, Color) {
        if self.quiet {
            (CHIP_FILL_QUIET, self.tone.border(theme).with_alpha(0.22))
        } else {
            (self.tone.fill(), self.tone.border(theme))
        }
    }
}

/// Which of a tone's three inks a chip span is written in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChipInk {
    /// The value text - [`ChipTone::text`].
    Value,
    /// A unit suffix or a secondary label - [`ChipTone::unit`].
    Unit,
    /// A sentence to read rather than a label - [`ChipTone::body`].
    Body,
}

/// Marks a chip's text, so [`reconcile_chip_text`] writes its `TextColor` from
/// the active theme and rewrites it on a theme change.
///
/// Sits on the chip node itself when the chip IS the text, and on a child span
/// when a chip carries several inks (a value and its unit suffix).
#[derive(Component, Clone, Copy, Debug)]
pub struct ChipText {
    tone: ChipTone,
    ink: ChipInk,
}

impl ChipText {
    /// The value ink of `tone`.
    pub fn value(tone: ChipTone) -> Self {
        Self {
            tone,
            ink: ChipInk::Value,
        }
    }

    /// The unit-suffix ink of `tone`.
    pub fn unit(tone: ChipTone) -> Self {
        Self {
            tone,
            ink: ChipInk::Unit,
        }
    }

    /// The reading ink of `tone`.
    pub fn body(tone: ChipTone) -> Self {
        Self {
            tone,
            ink: ChipInk::Body,
        }
    }

    /// This span's colour in `theme`.
    fn color(self, theme: &ActiveUiTheme) -> Color {
        match self.ink {
            ChipInk::Value => self.tone.text(theme),
            ChipInk::Unit => self.tone.unit(theme),
            ChipInk::Body => self.tone.body(theme),
        }
    }
}

/// The paint components of a chip in `tone` - spread over a node that already
/// carries [`chip_node`] geometry (or a positioned variant of it).
pub fn chip_paint(tone: ChipTone) -> impl Bundle {
    (
        ThemedChip::loud(tone),
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
    )
}

/// A complete text chip: geometry, paint and the tone's text colour. The caller
/// adds the `Text`/`TextFont` (font size varies per site) and any positioning.
pub fn text_chip(tone: ChipTone) -> impl Bundle {
    (
        chip_node(),
        chip_paint(tone),
        ChipText::value(tone),
        TextColor(Color::NONE),
    )
}

/// A quiet chip (the status bar): the same shape at a calmer border and fill,
/// so persistent chrome does not compete with the live readouts.
pub fn quiet_chip(tone: ChipTone) -> impl Bundle {
    (
        chip_node(),
        ThemedChip::quiet(tone),
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        ChipText::unit(tone),
        TextColor(Color::NONE),
    )
}

/// Register the chip reconcilers. Both run AFTER [`UiThemeSystems`], so a theme
/// flip reaches the HUD in the same frame it resolves.
pub(crate) fn build(app: &mut App) {
    app.add_systems(
        Update,
        (reconcile_chips, reconcile_chip_text).after(UiThemeSystems),
    );
}

/// Paint chips on a theme change and on spawn.
fn reconcile_chips(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(Entity, &ThemedChip, &mut BackgroundColor, &mut BorderColor)>,
    added: Query<Entity, Added<ThemedChip>>,
) {
    let restyle_all = theme.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    for (entity, chip, mut bg, mut border) in &mut q {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        let (fill, edge) = chip.paint(&theme);
        *bg = fill.into();
        border.set_all(edge);
    }
}

/// Write chip text colours on a theme change and on spawn.
fn reconcile_chip_text(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(Entity, &ChipText, &mut TextColor)>,
    added: Query<Entity, Added<ChipText>>,
) {
    let restyle_all = theme.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    for (entity, span, mut color) in &mut q {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        *color = TextColor(span.color(&theme));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The chip family shares ONE fill and geometry so the HUD reads as a
    /// single instrument; only the threat inset departs (a red-tinted slab).
    #[test]
    fn the_chip_family_shares_its_slab() {
        for tone in [ChipTone::Readout, ChipTone::Amber, ChipTone::Comms] {
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
        let theme = ActiveUiTheme::default();
        let luma = |color: Color| {
            let c = color.to_srgba();
            0.2126 * c.red + 0.7152 * c.green + 0.0722 * c.blue
        };
        for tone in [
            ChipTone::Readout,
            ChipTone::Amber,
            ChipTone::Threat,
            ChipTone::Comms,
        ] {
            assert!(
                luma(tone.text(&theme)) >= luma(tone.unit(&theme)),
                "{tone:?}: the value must not be dimmer than its unit suffix"
            );
            assert!(
                tone.border(&theme).alpha() < 1.0,
                "{tone:?}: the border is a hairline, not a solid frame"
            );
        }
    }

    /// A chip and its text follow the active theme LIVE. Fails if the chip
    /// reconcilers are unregistered, which is the defect the HUD had before:
    /// a chip kept the look its screen was built in.
    ///
    /// Flipped onto a MOD theme rather than onto `base/hardware`: the two
    /// shipped looks share one palette (a hardware casing draws the same
    /// phosphor ink), so a base-to-base flip moves a chip's slab and border but
    /// not its ink - and the ink is what this reconciler writes.
    #[test]
    fn a_chip_repaints_on_a_theme_change() {
        use crate::{
            theme::{
                base::base_ui_themes,
                config::{UiThemeConfig, PHOSPHOR_THEME_ID},
                GameUiThemes, SelectedUiTheme,
            },
            NovaUiPlugin,
        };

        const AMBER_CRT: &str = "wildcat/amber_crt";
        let mut themes = base_ui_themes();
        themes.push(UiThemeConfig {
            id: AMBER_CRT.to_string(),
            name: "Amber CRT".to_string(),
            inherit: Some(PHOSPHOR_THEME_ID.to_string()),
            palette: [("primary".to_string(), "#ffb84a".to_string())]
                .into_iter()
                .collect(),
            metrics: None,
            roles: crate::theme::config::UiRoles::default(),
        });

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, NovaUiPlugin));
        app.insert_resource(GameUiThemes(themes));
        let chip = app.world_mut().spawn(text_chip(ChipTone::Readout)).id();
        app.update();

        let read = |app: &App| {
            let entity = app.world().entity(chip);
            (
                entity.get::<BackgroundColor>().unwrap().0,
                entity.get::<TextColor>().unwrap().0,
            )
        };
        let (fill, ink) = read(&app);
        assert_eq!(fill, ChipTone::Readout.fill(), "the slab painted on spawn");
        assert_ne!(ink, Color::NONE, "and so did the value ink");

        *app.world_mut().resource_mut::<SelectedUiTheme>() = SelectedUiTheme(AMBER_CRT.to_string());
        app.update();
        let (_, amber_ink) = read(&app);
        assert_ne!(amber_ink, ink, "a theme that moves `primary` moves the ink");
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
