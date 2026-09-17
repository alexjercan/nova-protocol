//! Resolving an authored [`UiThemeConfig`] chain into the live
//! [`ActiveUiTheme`] the widgets paint from.
//!
//! Three things happen here, in order, and each one fails LOUDLY rather than
//! substituting a colour nobody chose:
//!
//! 1. the inheritance chain is walked from the selected theme to its root,
//!    rejecting an unknown parent and a cycle;
//! 2. each role is taken from the NEAREST ancestor that declares it, whole - a
//!    declared role replaces the inherited one completely, so no control is
//!    ever painted half in one look and half in another; and
//! 3. every [`ThemeColor`] is resolved against the merged palette, rejecting an
//!    unknown variable and a malformed literal.
//!
//! A theme that survives all three paints every role in every state. That is
//! the contract [`ActiveUiTheme`] carries, and it is why the widget layer can
//! index a role without an `Option` in sight.

use std::collections::BTreeMap;

use bevy::prelude::*;

use super::config::{
    BadgeRole, ButtonRole, CheckboxRole, ColorValue, Effect, Fill, HeadPaint, RadialAnchor,
    RowPaint, SliderMeter, SliderRole, StatePaint, SurfacePaint, ThemeColor, ToggleRole, UiMetrics,
    UiThemeConfig,
};

/// A semantic colour the SCREENS ask for by meaning.
///
/// The closed half of the palette: a theme may name as many extra variables as
/// its role table wants, but these thirteen must exist in every root theme
/// because screen chrome resolves through them. They are what the retired
/// `theme::PHOSPHOR` / `theme::PHOSPHOR_MUTED` / ... constants meant, named for
/// the meaning rather than for the look that happened to ship first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiColor {
    /// Live text, active borders, glyphs - the primary ink of the UI.
    Primary,
    /// Secondary text and idle fills.
    Secondary,
    /// Labels, section heads, tags.
    Label,
    /// Readable body copy.
    Body,
    /// Selection / keycap / warning accent.
    Accent,
    /// The bright end of the accent family.
    AccentHigh,
    /// The deep end of the accent family.
    AccentLow,
    /// Glyphs sitting on a bright fill.
    Inverted,
    /// The panel and screen surface.
    Surface,
    /// The deepest field behind everything.
    Void,
    /// The danger family.
    Danger,
    /// The info family.
    Info,
    /// The healthy / online / lit tone - a passed check, an enabled dependency,
    /// a lamp that is on.
    ///
    /// Separate from [`Primary`](Self::Primary) because the two agree only on a
    /// look whose ink IS green. On the hardware casing the ink is bone white
    /// and the lamp is still green, and a screen that asked for "primary"
    /// because it wanted "good" got a white ONLINE badge.
    Nominal,
}

impl UiColor {
    /// Every semantic colour, in declaration order - what the completeness
    /// check walks and what a root theme must define.
    pub const ALL: [UiColor; 13] = [
        UiColor::Primary,
        UiColor::Secondary,
        UiColor::Label,
        UiColor::Body,
        UiColor::Accent,
        UiColor::AccentHigh,
        UiColor::AccentLow,
        UiColor::Inverted,
        UiColor::Surface,
        UiColor::Void,
        UiColor::Danger,
        UiColor::Info,
        UiColor::Nominal,
    ];

    /// The palette variable name this colour resolves through. Role tables
    /// reference the SAME names, so a theme states each semantic tone once.
    pub fn variable(self) -> &'static str {
        match self {
            UiColor::Primary => "primary",
            UiColor::Secondary => "secondary",
            UiColor::Label => "label",
            UiColor::Body => "body",
            UiColor::Accent => "accent",
            UiColor::AccentHigh => "accent_high",
            UiColor::AccentLow => "accent_low",
            UiColor::Inverted => "inverted",
            UiColor::Surface => "surface",
            UiColor::Void => "void",
            UiColor::Danger => "danger",
            UiColor::Info => "info",
            UiColor::Nominal => "nominal",
        }
    }
}

/// A shared metric the screens ask for by meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiMetric {
    /// Hairline border width.
    BorderWidth,
    /// Control corner radius.
    Radius,
    /// Panel corner radius.
    PanelRadius,
}

/// Which button emphasis a paint table belongs to. Mirrors
/// [`ButtonVariant`](crate::widget::ButtonVariant), which is the component
/// side of the same choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeButton {
    /// The neutral button.
    Default,
    /// The call-to-action button.
    Primary,
    /// The destructive button.
    Danger,
    /// The border-only button.
    Ghost,
}

/// Which of a button's six states is showing, in resolution precedence order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    /// Not interactive - outranks everything.
    Disabled,
    /// The active choice, held down.
    SelectedPressed,
    /// Held down.
    Pressed,
    /// The active choice of its group.
    Selected,
    /// Pointer over it.
    Hovered,
    /// Idle.
    Normal,
}

impl ButtonState {
    /// The state a button is in, resolved from its flags in precedence order.
    ///
    /// The one place that order is written down. The observers and the theme
    /// reconciler both come through here, so the two paths cannot disagree
    /// about what a pressed-and-selected-and-disabled button looks like.
    pub fn resolve(disabled: bool, pressed: bool, selected: bool, hovered: bool) -> Self {
        if disabled {
            ButtonState::Disabled
        } else if selected && pressed {
            ButtonState::SelectedPressed
        } else if pressed {
            ButtonState::Pressed
        } else if selected {
            ButtonState::Selected
        } else if hovered {
            ButtonState::Hovered
        } else {
            ButtonState::Normal
        }
    }
}

/// A resolved background: the flat colour, plus the gradient painted over it
/// when the theme authored one.
///
/// Both, not either: bevy paints a `BackgroundGradient` over a
/// `BackgroundColor`, and a control whose gradient is removed on a theme flip
/// must still have a colour underneath.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFill {
    /// The flat background colour.
    pub base: Color,
    /// The gradient over it, or `None` for a flat face.
    pub gradient: Option<BackgroundGradient>,
}

/// A resolved control state: everything one widget needs to paint itself.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedState {
    /// The background.
    pub fill: ResolvedFill,
    /// The border colour.
    pub border: Color,
    /// The label colour.
    pub text: Color,
    /// The cast glow or shadow, or `None`.
    pub shadow: Option<BoxShadow>,
}

/// A resolved bordered surface.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSurface {
    /// The background.
    pub fill: ResolvedFill,
    /// The border colour.
    pub border: Color,
    /// The cast glow or shadow, or `None`.
    pub shadow: Option<BoxShadow>,
    /// The corner radius, already defaulted to the theme's control radius.
    pub radius: f32,
}

/// A resolved panel header band.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedHead {
    /// The band's bottom border.
    pub border: Color,
    /// The header title.
    pub title: Color,
    /// The rule filling the row.
    pub rule: Color,
    /// The trailing tag chip's text.
    pub tag: Color,
}

/// A resolved list-row state.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRow {
    /// The background.
    pub fill: ResolvedFill,
    /// The border colour.
    pub border: Color,
}

/// A resolved slider track.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSlider {
    /// The well.
    pub surface: ResolvedSurface,
    /// Track height in logical pixels.
    pub height: f32,
    /// How the value is shown.
    pub meter: SliderMeter,
    /// The lit part of the meter.
    pub lit: Color,
    /// The unlit part.
    pub unlit: Color,
}

/// A resolved status badge.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedBadge {
    /// `true` for the `[TAG]` text form, `false` for the bordered chip.
    pub bracketed: bool,
    /// Horizontal padding in logical pixels.
    pub padding_x: f32,
    /// Border width.
    pub border_width: f32,
    /// Alpha applied to the family colour for the border.
    pub border_alpha: f32,
    /// Alpha applied to the family colour for the fill.
    pub fill_alpha: f32,
}

/// A resolved scrollbar.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedBar {
    /// The track.
    pub track: Color,
    /// The thumb.
    pub thumb: Color,
}

/// The live theme: a fully resolved, complete paint table.
///
/// Every accessor returns a value, never an `Option`, because a theme that
/// could not resolve one never became an `ActiveUiTheme` - [`resolve_theme`]
/// returned the naming error instead and the caller fell back. That is what
/// lets the widget layer read paint without a default in sight.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ActiveUiTheme {
    id: String,
    name: String,
    colors: BTreeMap<&'static str, Color>,
    metrics: UiMetrics,
    button: ResolvedButtons,
    panel: ResolvedSurface,
    panel_head: ResolvedHead,
    list_row: ResolvedRows,
    segmented: ResolvedSurface,
    slider_track: ResolvedSlider,
    text_field: ResolvedField,
    checkbox: ResolvedCheckbox,
    toggle: ResolvedToggle,
    badge: ResolvedBadge,
    scroll_bar: ResolvedBar,
    key_chip: ResolvedState,
    separator: Color,
}

#[derive(Debug, Clone, PartialEq)]
struct ResolvedButtons {
    default: [ResolvedState; 6],
    primary: [ResolvedState; 6],
    danger: [ResolvedState; 6],
    ghost: [ResolvedState; 6],
}

#[derive(Debug, Clone, PartialEq)]
struct ResolvedRows {
    normal: ResolvedRow,
    hovered: ResolvedRow,
    selected: ResolvedRow,
}

#[derive(Debug, Clone, PartialEq)]
struct ResolvedField {
    normal: ResolvedState,
    hovered: ResolvedState,
    focused: ResolvedState,
    error: ResolvedState,
}

/// A resolved checkbox: the corner it is cut to, and its two states.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCheckbox {
    /// The box's corner radius, already defaulted.
    pub radius: f32,
    off: ResolvedState,
    on: ResolvedState,
}

impl ResolvedCheckbox {
    /// One state's paint.
    pub fn state(&self, state: OnOff) -> &ResolvedState {
        match state {
            OnOff::Off => &self.off,
            OnOff::On => &self.on,
        }
    }
}

/// A resolved toggle: the track's corner, the knob's, and its two states.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedToggle {
    /// The track's corner radius, already defaulted.
    pub radius: f32,
    /// The knob's corner radius.
    pub knob_radius: f32,
    off: ResolvedState,
    on: ResolvedState,
}

impl ResolvedToggle {
    /// One state's paint. Its `text` is the knob colour and its `shadow` the
    /// knob's glow.
    pub fn state(&self, state: OnOff) -> &ResolvedState {
        match state {
            OnOff::Off => &self.off,
            OnOff::On => &self.on,
        }
    }
}

/// Which state of an on/off control is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnOff {
    /// Unchecked / off.
    Off,
    /// Checked / on.
    On,
}

impl From<bool> for OnOff {
    fn from(on: bool) -> Self {
        if on {
            OnOff::On
        } else {
            OnOff::Off
        }
    }
}

/// Which state of a list row is showing. Selection outranks hover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowState {
    /// Idle.
    Normal,
    /// Pointer over it.
    Hovered,
    /// The active row.
    Selected,
}

impl RowState {
    /// The state a row is in, resolved from its flags in precedence order.
    pub fn resolve(selected: bool, hovered: bool) -> Self {
        if selected {
            RowState::Selected
        } else if hovered {
            RowState::Hovered
        } else {
            RowState::Normal
        }
    }
}

/// Which state of a text field is showing. Error outranks focus outranks hover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldState {
    /// Idle.
    Normal,
    /// Pointer over it.
    Hovered,
    /// Holding the caret.
    Focused,
    /// Holding an invalid value.
    Error,
}

impl FieldState {
    /// The state a field is in, resolved from its flags in precedence order.
    pub fn resolve(error: bool, focused: bool, hovered: bool) -> Self {
        if error {
            FieldState::Error
        } else if focused {
            FieldState::Focused
        } else if hovered {
            FieldState::Hovered
        } else {
            FieldState::Normal
        }
    }
}

impl ActiveUiTheme {
    /// The theme's stable id - what the setting persists.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The theme's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// One semantic colour.
    pub fn color(&self, color: UiColor) -> Color {
        self.colors[color.variable()]
    }

    /// One semantic colour at an explicit alpha - the themed replacement for
    /// the retired `theme::PHOSPHOR.with_alpha(0.4)` idiom.
    pub fn color_alpha(&self, color: UiColor, alpha: f32) -> Color {
        self.color(color).with_alpha(alpha)
    }

    /// One shared metric.
    pub fn metric(&self, metric: UiMetric) -> f32 {
        match metric {
            UiMetric::BorderWidth => self.metrics.border_width,
            UiMetric::Radius => self.metrics.radius,
            UiMetric::PanelRadius => self.metrics.panel_radius,
        }
    }

    /// One button variant in one state.
    pub fn button(&self, variant: ThemeButton, state: ButtonState) -> &ResolvedState {
        let table = match variant {
            ThemeButton::Default => &self.button.default,
            ThemeButton::Primary => &self.button.primary,
            ThemeButton::Danger => &self.button.danger,
            ThemeButton::Ghost => &self.button.ghost,
        };
        &table[state as usize]
    }

    /// The panel surface.
    pub fn panel(&self) -> &ResolvedSurface {
        &self.panel
    }

    /// The panel header band.
    pub fn panel_head(&self) -> &ResolvedHead {
        &self.panel_head
    }

    /// A list row in one state.
    pub fn list_row(&self, state: RowState) -> &ResolvedRow {
        match state {
            RowState::Normal => &self.list_row.normal,
            RowState::Hovered => &self.list_row.hovered,
            RowState::Selected => &self.list_row.selected,
        }
    }

    /// The segmented control's container.
    pub fn segmented(&self) -> &ResolvedSurface {
        &self.segmented
    }

    /// The slider track.
    pub fn slider_track(&self) -> &ResolvedSlider {
        &self.slider_track
    }

    /// A text field in one state.
    pub fn text_field(&self, state: FieldState) -> &ResolvedState {
        match state {
            FieldState::Normal => &self.text_field.normal,
            FieldState::Hovered => &self.text_field.hovered,
            FieldState::Focused => &self.text_field.focused,
            FieldState::Error => &self.text_field.error,
        }
    }

    /// The checkbox.
    pub fn checkbox(&self) -> &ResolvedCheckbox {
        &self.checkbox
    }

    /// The pill toggle.
    pub fn toggle(&self) -> &ResolvedToggle {
        &self.toggle
    }

    /// The status badge's chrome.
    pub fn badge(&self) -> &ResolvedBadge {
        &self.badge
    }

    /// The scrollbar.
    pub fn scroll_bar(&self) -> &ResolvedBar {
        &self.scroll_bar
    }

    /// The keycap chip.
    pub fn key_chip(&self) -> &ResolvedState {
        &self.key_chip
    }

    /// The rule drawn between sections.
    pub fn separator(&self) -> Color {
        self.separator
    }
}

impl Default for ActiveUiTheme {
    /// The BOOTSTRAP theme: base phosphor, resolved from the builders that also
    /// generate its RON.
    ///
    /// Here rather than behind an `Option` because the loading screen and the
    /// asset-failure screen draw BEFORE any content has loaded - the earliest
    /// UI cannot wait on the merge that publishes `GameUiThemes`. The builders
    /// are the one definition, so the bootstrap and the base mod's
    /// `base/phosphor` are the same theme by construction, not by two lists
    /// somebody keeps in step.
    fn default() -> Self {
        resolve_theme(
            super::config::PHOSPHOR_THEME_ID,
            &[super::base::phosphor_theme()],
        )
        .expect("the built-in phosphor theme resolves")
    }
}

/// Why a theme could not be resolved, named so a player-visible diagnostic and
/// a content-lint finding can carry the same sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeIssue {
    /// The theme id the problem is about.
    pub theme: String,
    /// What is wrong, in the authoring vocabulary.
    pub message: String,
}

impl std::fmt::Display for ThemeIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ui theme '{}': {}", self.theme, self.message)
    }
}

fn issue(theme: &str, message: impl Into<String>) -> ThemeIssue {
    ThemeIssue {
        theme: theme.to_string(),
        message: message.into(),
    }
}

/// The ancestry of `id`, nearest first, ending at its root.
///
/// Rejects an unknown id, an unknown parent and a cycle by name. The depth cap
/// is the theme count: a chain longer than that has revisited something, and
/// the visited set catches it first - the cap is the belt to that braces.
fn chain<'a>(id: &str, themes: &'a [UiThemeConfig]) -> Result<Vec<&'a UiThemeConfig>, ThemeIssue> {
    let mut seen: Vec<&str> = Vec::new();
    let mut out: Vec<&UiThemeConfig> = Vec::new();
    let mut cursor = id.to_string();
    loop {
        let Some(config) = themes.iter().find(|t| t.id == cursor) else {
            return Err(if out.is_empty() {
                issue(&cursor, "no theme with this id is registered")
            } else {
                issue(
                    &out[out.len() - 1].id,
                    format!("inherits from unknown theme '{cursor}'"),
                )
            });
        };
        if seen.contains(&config.id.as_str()) {
            return Err(issue(
                &config.id,
                format!("inheritance cycle: {} -> {}", seen.join(" -> "), config.id),
            ));
        }
        seen.push(&config.id);
        out.push(config);
        match &config.inherit {
            Some(parent) => cursor = parent.clone(),
            None => return Ok(out),
        }
    }
}

/// Resolve one theme id against the registered set.
///
/// The whole gate: chain, role completeness, palette completeness, then every
/// colour. An `Err` names one problem in the authoring vocabulary; the caller
/// logs it and falls back rather than painting something nobody authored.
pub fn resolve_theme(id: &str, themes: &[UiThemeConfig]) -> Result<ActiveUiTheme, ThemeIssue> {
    let chain = chain(id, themes)?;
    let head = chain[0];

    // Nearest-first: a child's palette entry shadows an ancestor's, and its
    // extra names join rather than replace.
    let mut palette: BTreeMap<&str, &str> = BTreeMap::new();
    for config in chain.iter().rev() {
        for (name, hex) in &config.palette {
            palette.insert(name.as_str(), hex.as_str());
        }
    }

    let mut colors = BTreeMap::new();
    for color in UiColor::ALL {
        let name = color.variable();
        let Some(hex) = palette.get(name) else {
            return Err(issue(
                id,
                format!("palette is missing the required variable '{name}'"),
            ));
        };
        colors.insert(name, parse_hex(id, name, hex)?);
    }

    let metrics = *chain
        .iter()
        .find_map(|c| c.metrics.as_ref())
        .ok_or_else(|| issue(id, "no theme in the inheritance chain declares metrics"))?;

    let ctx = Ctx {
        id,
        palette: &palette,
        metrics,
    };

    // Each role comes from the nearest ancestor that declares it, whole.
    macro_rules! role {
        ($field:ident) => {
            chain
                .iter()
                .find_map(|c| c.roles.$field.as_ref())
                .ok_or_else(|| {
                    issue(
                        id,
                        concat!(
                            "no theme in the inheritance chain declares the role '",
                            stringify!($field),
                            "'"
                        ),
                    )
                })?
        };
    }

    let theme = ActiveUiTheme {
        id: head.id.clone(),
        name: head.name.clone(),
        colors,
        metrics,
        button: ResolvedButtons {
            default: ctx.button(role!(button))?,
            primary: ctx.button(role!(button_primary))?,
            danger: ctx.button(role!(button_danger))?,
            ghost: ctx.button(role!(button_ghost))?,
        },
        panel: ctx.surface(role!(panel), metrics.panel_radius)?,
        panel_head: ctx.head(role!(panel_head))?,
        list_row: {
            let row = role!(list_row);
            ResolvedRows {
                normal: ctx.row(&row.normal)?,
                hovered: ctx.row(&row.hovered)?,
                selected: ctx.row(&row.selected)?,
            }
        },
        segmented: ctx.surface(role!(segmented), metrics.radius)?,
        slider_track: ctx.slider(role!(slider_track))?,
        text_field: {
            let field = role!(text_field);
            ResolvedField {
                normal: ctx.state(&field.normal)?,
                hovered: ctx.state(&field.hovered)?,
                focused: ctx.state(&field.focused)?,
                error: ctx.state(&field.error)?,
            }
        },
        checkbox: ctx.checkbox(role!(checkbox))?,
        toggle: ctx.toggle(role!(toggle))?,
        badge: ctx.badge(role!(badge)),
        scroll_bar: {
            let bar = role!(scroll_bar);
            ResolvedBar {
                track: ctx.color(&bar.track)?,
                thumb: ctx.color(&bar.thumb)?,
            }
        },
        key_chip: ctx.state(role!(key_chip))?,
        separator: ctx.color(role!(separator))?,
    };
    Ok(theme)
}

/// Every problem with one registered theme, for the content lint.
///
/// [`resolve_theme`] stops at the FIRST problem because a half-resolved theme
/// cannot paint anything; the lint wants the whole list, so it resolves and
/// reports what it got. One finding per theme is enough to refuse it, and the
/// author fixes them one at a time anyway.
pub fn lint_theme(config: &UiThemeConfig, registered: &[UiThemeConfig]) -> Vec<ThemeIssue> {
    match resolve_theme(&config.id, registered) {
        Ok(_) => Vec::new(),
        Err(issue) => vec![issue],
    }
}

/// The palette + metrics one theme resolves its colours against.
struct Ctx<'a> {
    id: &'a str,
    palette: &'a BTreeMap<&'a str, &'a str>,
    metrics: UiMetrics,
}

impl Ctx<'_> {
    fn color(&self, color: &ThemeColor) -> Result<Color, ThemeIssue> {
        let rgb = match &color.value {
            ColorValue::Variable(name) => {
                let hex = self
                    .palette
                    .get(name.as_str())
                    .ok_or_else(|| issue(self.id, format!("unknown palette variable '{name}'")))?;
                parse_hex(self.id, name, hex)?
            }
            ColorValue::Literal(hex) => parse_hex(self.id, "literal", hex)?,
        };
        Ok(rgb.with_alpha(color.alpha))
    }

    fn fill(&self, fill: &Fill) -> Result<ResolvedFill, ThemeIssue> {
        match fill {
            Fill::Solid(color) => Ok(ResolvedFill {
                base: self.color(color)?,
                gradient: None,
            }),
            Fill::Linear {
                base,
                degrees,
                stops,
            } => {
                let stops = self.stops(stops)?;
                Ok(ResolvedFill {
                    base: self.color(base)?,
                    gradient: Some(BackgroundGradient(vec![LinearGradient::degrees(
                        *degrees,
                        stops
                            .into_iter()
                            .map(|(c, p)| ColorStop::percent(c, p))
                            .collect(),
                    )
                    .into()])),
                })
            }
            Fill::Radial {
                base,
                anchor,
                stops,
            } => {
                let stops = self.stops(stops)?;
                Ok(ResolvedFill {
                    base: self.color(base)?,
                    gradient: Some(BackgroundGradient(vec![Gradient::from(
                        RadialGradient::new(
                            match anchor {
                                RadialAnchor::Top => UiPosition::TOP,
                                RadialAnchor::Center => UiPosition::CENTER,
                            },
                            RadialGradientShape::FarthestSide,
                            stops
                                .into_iter()
                                .map(|(c, p)| ColorStop::percent(c, p))
                                .collect(),
                        ),
                    )])),
                })
            }
        }
    }

    fn stops(&self, stops: &[super::config::Stop]) -> Result<Vec<(Color, f32)>, ThemeIssue> {
        if stops.len() < 2 {
            return Err(issue(self.id, "a gradient needs at least two stops"));
        }
        stops
            .iter()
            .map(|stop| Ok((self.color(&stop.color)?, stop.percent)))
            .collect()
    }

    fn shadow(&self, effect: Option<&Effect>) -> Result<Option<BoxShadow>, ThemeIssue> {
        effect
            .map(|e| {
                Ok(BoxShadow::new(
                    self.color(&e.color)?,
                    Val::ZERO,
                    Val::Px(e.offset_y),
                    Val::ZERO,
                    Val::Px(e.blur),
                ))
            })
            .transpose()
    }

    fn state(&self, paint: &StatePaint) -> Result<ResolvedState, ThemeIssue> {
        Ok(ResolvedState {
            fill: self.fill(&paint.fill)?,
            border: self.color(&paint.border)?,
            text: self.color(&paint.text)?,
            shadow: self.shadow(paint.effect.as_ref())?,
        })
    }

    fn button(&self, role: &ButtonRole) -> Result<[ResolvedState; 6], ThemeIssue> {
        // Indexed by `ButtonState as usize`, so the array order IS the
        // precedence order the enum declares.
        Ok([
            self.state(&role.disabled)?,
            self.state(&role.selected_pressed)?,
            self.state(&role.pressed)?,
            self.state(&role.selected)?,
            self.state(&role.hovered)?,
            self.state(&role.normal)?,
        ])
    }

    fn surface(
        &self,
        paint: &SurfacePaint,
        fallback_radius: f32,
    ) -> Result<ResolvedSurface, ThemeIssue> {
        Ok(ResolvedSurface {
            fill: self.fill(&paint.fill)?,
            border: self.color(&paint.border)?,
            shadow: self.shadow(paint.effect.as_ref())?,
            radius: paint.radius.unwrap_or(fallback_radius),
        })
    }

    fn head(&self, paint: &HeadPaint) -> Result<ResolvedHead, ThemeIssue> {
        Ok(ResolvedHead {
            border: self.color(&paint.border)?,
            title: self.color(&paint.title)?,
            rule: self.color(&paint.rule)?,
            tag: self.color(&paint.tag)?,
        })
    }

    fn row(&self, paint: &RowPaint) -> Result<ResolvedRow, ThemeIssue> {
        Ok(ResolvedRow {
            fill: self.fill(&paint.fill)?,
            border: self.color(&paint.border)?,
        })
    }

    fn slider(&self, role: &SliderRole) -> Result<ResolvedSlider, ThemeIssue> {
        Ok(ResolvedSlider {
            surface: self.surface(&role.surface, self.metrics.radius)?,
            height: role.height,
            meter: role.meter,
            lit: self.color(&role.lit)?,
            unlit: self.color(&role.unlit)?,
        })
    }

    fn checkbox(&self, role: &CheckboxRole) -> Result<ResolvedCheckbox, ThemeIssue> {
        Ok(ResolvedCheckbox {
            radius: role.radius.unwrap_or(self.metrics.radius),
            off: self.state(&role.off)?,
            on: self.state(&role.on)?,
        })
    }

    fn toggle(&self, role: &ToggleRole) -> Result<ResolvedToggle, ThemeIssue> {
        Ok(ResolvedToggle {
            radius: role.radius.unwrap_or(self.metrics.radius),
            knob_radius: role.knob_radius,
            off: self.state(&role.off)?,
            on: self.state(&role.on)?,
        })
    }

    fn badge(&self, role: &BadgeRole) -> ResolvedBadge {
        ResolvedBadge {
            bracketed: role.bracketed,
            padding_x: role.padding_x,
            border_width: role.border_width,
            border_alpha: role.border_alpha,
            fill_alpha: role.fill_alpha,
        }
    }
}

/// Parse a six-digit `#rrggbb`. Anything else is an error by name, because a
/// mistyped colour that silently became black is a bug a player reports as
/// "the menu went dark".
fn parse_hex(theme: &str, name: &str, hex: &str) -> Result<Color, ThemeIssue> {
    let body = hex.strip_prefix('#').ok_or_else(|| {
        issue(
            theme,
            format!("colour '{name}' is '{hex}'; expected a six-digit #rrggbb"),
        )
    })?;
    if body.len() != 6 || !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(issue(
            theme,
            format!("colour '{name}' is '{hex}'; expected a six-digit #rrggbb"),
        ));
    }
    let byte = |at: usize| u8::from_str_radix(&body[at..at + 2], 16).unwrap_or(0);
    Ok(Color::srgb_u8(byte(0), byte(2), byte(4)))
}
