//! Small chrome: section headers, separators, status badges, colour swatches,
//! checkboxes and pill toggles.
//!
//! Every factory here spawns UNPAINTED and carries a marker, and every marker
//! has a reconciler below. That is what makes a theme flip reach chrome that is
//! already on screen: before, this whole module read the current look ONCE at
//! spawn, so a header, a badge or a toggle kept its old colours until its
//! screen was next rebuilt.
//!
//! The on/off controls ([`checkbox`], [`toggle`]) keep their state in their own
//! marker, so a caller flips the control by writing the component and the
//! reconciler owns the paint - there is no second "restyle it in place" path to
//! keep in step.

use bevy::{
    ecs::relationship::RelatedSpawner, platform::collections::HashSet, prelude::*, text::TextColor,
};

use super::UiText;
use crate::theme::{ActiveUiTheme, OnOff, UiColor, UiMetric, BORDER_W};

/// Marks a text span whose colour is a theme colour, so it re-tints live.
///
/// The general case for chrome text that is not a button label or a panel head:
/// a section header, a row legend, a readout. `alpha` multiplies the theme
/// colour, so "label at 60%" needs no palette entry of its own.
#[derive(Component, Debug, Clone, Copy)]
pub struct ThemedText {
    /// Which theme colour the span takes.
    pub color: UiColor,
    /// Alpha applied to it.
    pub alpha: f32,
}

impl ThemedText {
    /// A span at the theme colour's own alpha.
    pub fn new(color: UiColor) -> Self {
        Self { color, alpha: 1.0 }
    }

    /// A span at a fraction of the theme colour.
    pub fn alpha(color: UiColor, alpha: f32) -> Self {
        Self { color, alpha }
    }
}

/// Re-tint [`ThemedText`] spans on a theme change and on spawn.
pub(super) fn reconcile_themed_text(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(Entity, &ThemedText, &mut TextColor)>,
    changed: Query<Entity, Changed<ThemedText>>,
) {
    let restyle_all = theme.is_changed();
    let dirty: HashSet<Entity> = changed.iter().collect();
    if !restyle_all && dirty.is_empty() {
        return;
    }
    for (entity, themed, mut color) in &mut q {
        if !restyle_all && !dirty.contains(&entity) {
            continue;
        }
        *color = TextColor(theme.color_alpha(themed.color, themed.alpha));
    }
}

/// Marks a span whose `TextShadow` is a theme colour, so the glow under it
/// follows the theme the ink above it already does.
///
/// Separate from [`ThemedText`] because the two are independent: the menu
/// title is body-coloured ink over a primary-coloured bloom.
#[derive(Component, Debug, Clone, Copy)]
pub struct ThemedTextShadow {
    /// Which theme colour the shadow takes.
    pub color: UiColor,
    /// Alpha applied to it.
    pub alpha: f32,
}

impl ThemedTextShadow {
    /// A shadow at the theme colour's own alpha.
    pub fn new(color: UiColor) -> Self {
        Self { color, alpha: 1.0 }
    }

    /// A shadow at a fraction of the theme colour.
    pub fn alpha(color: UiColor, alpha: f32) -> Self {
        Self { color, alpha }
    }
}

/// Re-tint [`ThemedTextShadow`] glows on a theme change and on spawn. The
/// shadow's OFFSET is the caller's - only its colour is the theme's.
pub(super) fn reconcile_themed_text_shadows(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(Entity, &ThemedTextShadow, &mut TextShadow)>,
    changed: Query<Entity, Changed<ThemedTextShadow>>,
) {
    let restyle_all = theme.is_changed();
    let dirty: HashSet<Entity> = changed.iter().collect();
    if !restyle_all && dirty.is_empty() {
        return;
    }
    for (entity, themed, mut shadow) in &mut q {
        if !restyle_all && !dirty.contains(&entity) {
            continue;
        }
        shadow.color = theme.color_alpha(themed.color, themed.alpha);
    }
}

/// Marks an [`ImageNode`] whose TINT is a theme colour, so a picture that is
/// chrome - a keycap, a glyph sheet - dims and lights with the rest of it.
///
/// Art that carries its own colour wears no marker: this is for the white-on-
/// transparent sheets the UI tints, never for an illustration.
#[derive(Component, Debug, Clone, Copy)]
pub struct ThemedImageTint {
    /// Which theme colour the picture is tinted with.
    pub color: UiColor,
    /// Alpha applied to it.
    pub alpha: f32,
}

impl ThemedImageTint {
    /// A picture at the theme colour's own alpha.
    pub fn new(color: UiColor) -> Self {
        Self { color, alpha: 1.0 }
    }

    /// A picture at a fraction of the theme colour.
    pub fn alpha(color: UiColor, alpha: f32) -> Self {
        Self { color, alpha }
    }
}

/// Re-tint [`ThemedImageTint`] pictures on a theme change and on spawn.
pub(super) fn reconcile_themed_images(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(Entity, &ThemedImageTint, &mut ImageNode)>,
    changed: Query<Entity, Changed<ThemedImageTint>>,
) {
    let restyle_all = theme.is_changed();
    let dirty: HashSet<Entity> = changed.iter().collect();
    if !restyle_all && dirty.is_empty() {
        return;
    }
    for (entity, themed, mut image) in &mut q {
        if !restyle_all && !dirty.contains(&entity) {
            continue;
        }
        image.color = theme.color_alpha(themed.color, themed.alpha);
    }
}

/// Marks a node whose BACKGROUND is a theme colour, so it repaints live.
///
/// The general case for a surface that is not one of the widget families above:
/// a screen backdrop, a card, a readout well. Pair it with `BackgroundColor`.
#[derive(Component, Debug, Clone, Copy)]
pub struct ThemedFill {
    /// Which theme colour the surface takes.
    pub color: UiColor,
    /// Alpha applied to it.
    pub alpha: f32,
}

impl ThemedFill {
    /// A surface at the theme colour's own alpha.
    pub fn new(color: UiColor) -> Self {
        Self { color, alpha: 1.0 }
    }

    /// A surface at a fraction of the theme colour.
    pub fn alpha(color: UiColor, alpha: f32) -> Self {
        Self { color, alpha }
    }
}

/// Marks a node whose BORDER is a theme colour, so it repaints live. Pair it
/// with `BorderColor`.
#[derive(Component, Debug, Clone, Copy)]
pub struct ThemedBorder {
    /// Which theme colour the border takes.
    pub color: UiColor,
    /// Alpha applied to it.
    pub alpha: f32,
}

impl ThemedBorder {
    /// A border at the theme colour's own alpha.
    pub fn new(color: UiColor) -> Self {
        Self { color, alpha: 1.0 }
    }

    /// A border at a fraction of the theme colour.
    pub fn alpha(color: UiColor, alpha: f32) -> Self {
        Self { color, alpha }
    }
}

/// Marks a node whose CORNER RADIUS is a theme metric, so it follows a look
/// that rounds its corners differently.
#[derive(Component, Debug, Clone, Copy)]
pub struct ThemedRadius(pub UiMetric);

impl ThemedRadius {
    /// The control radius - buttons, chips, wells.
    pub fn control() -> Self {
        Self(UiMetric::Radius)
    }

    /// The panel radius - cards and modals.
    pub fn panel() -> Self {
        Self(UiMetric::PanelRadius)
    }
}

/// Paint [`ThemedFill`], [`ThemedBorder`] and [`ThemedRadius`] nodes on a theme
/// change and whenever their marker is written.
///
/// One system for the three because they land on the same entities and a node
/// that takes two of them should take them in one pass, not in whichever order
/// three systems happened to be registered.
#[expect(
    clippy::type_complexity,
    reason = "three optional paint markers against three optional targets"
)]
pub(super) fn reconcile_themed_nodes(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(
        Entity,
        Option<&ThemedFill>,
        Option<&ThemedBorder>,
        Option<&ThemedRadius>,
        Option<&mut BackgroundColor>,
        Option<&mut BorderColor>,
        Option<&mut Node>,
    )>,
    changed: Query<
        Entity,
        Or<(
            Changed<ThemedFill>,
            Changed<ThemedBorder>,
            Changed<ThemedRadius>,
        )>,
    >,
) {
    let restyle_all = theme.is_changed();
    let dirty: HashSet<Entity> = changed.iter().collect();
    if !restyle_all && dirty.is_empty() {
        return;
    }
    for (entity, fill, border, radius, bg, border_color, node) in &mut q {
        if fill.is_none() && border.is_none() && radius.is_none() {
            continue;
        }
        if !restyle_all && !dirty.contains(&entity) {
            continue;
        }
        if let (Some(fill), Some(mut bg)) = (fill, bg) {
            *bg = theme.color_alpha(fill.color, fill.alpha).into();
        }
        if let (Some(border), Some(mut border_color)) = (border, border_color) {
            border_color.set_all(theme.color_alpha(border.color, border.alpha));
        }
        if let (Some(radius), Some(mut node)) = (radius, node) {
            node.border_radius = BorderRadius::all(px(theme.metric(radius.0)));
        }
    }
}

/// A small uppercase section header (e.g. "COMPONENTS"). No TextShadow: a hard
/// drop shadow makes the header feel floaty; a header should read as imprinted
/// on the surface, not hovering over it.
pub fn panel_header(text: &str) -> impl Bundle {
    (
        UiText,
        ThemedText::new(UiColor::Primary),
        Text::new(text.to_uppercase()),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::NONE),
        Node {
            margin: UiRect::bottom(px(8)),
            ..default()
        },
    )
}

/// Marks a rule drawn in the theme's separator tone, and says WHICH channel of
/// the node carries it.
///
/// Explicit, because `Node` requires a `BackgroundColor`: an entity that means
/// to draw the rule as a border still has a background for a "paint whichever
/// channel exists" reconciler to find, and painting it washes the whole node in
/// the separator tone. That is what turned the details pane beside every list
/// into a flat green slab under the hardware look.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemedSeparator {
    /// The node's own background - a node that IS the rule ([`separator`]).
    Rule,
    /// The node's borders - a pane that draws the rule as one of its edges
    /// (the details pane beside a list).
    Edge,
}

/// A thin horizontal separator rule.
pub fn separator() -> impl Bundle {
    (
        ThemedSeparator::Rule,
        Node {
            width: percent(100),
            height: px(BORDER_W),
            margin: UiRect::vertical(px(8)),
            ..default()
        },
        BackgroundColor(Color::NONE),
    )
}

/// Paint separator rules on a theme change and on spawn.
pub(super) fn reconcile_separators(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(
        Entity,
        &ThemedSeparator,
        Option<&mut BackgroundColor>,
        Option<&mut BorderColor>,
    )>,
    added: Query<Entity, Added<ThemedSeparator>>,
) {
    let restyle_all = theme.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    let rule = theme.separator();
    for (entity, channel, bg, border) in &mut q {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        match channel {
            ThemedSeparator::Rule => {
                if let Some(mut bg) = bg {
                    *bg = rule.into();
                }
            }
            ThemedSeparator::Edge => {
                if let Some(mut border) = border {
                    border.set_all(rule);
                }
            }
        }
    }
}

/// Semantic colour families for [`badge`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeKind {
    /// Nominal / online.
    Green,
    /// Warning.
    Amber,
    /// Info.
    Blue,
    /// Fault.
    Red,
    /// Idle / neutral tag.
    Mute,
}

impl BadgeKind {
    /// The theme colour and the alpha this family reads at.
    fn tone(self) -> (UiColor, f32) {
        match self {
            BadgeKind::Green => (UiColor::Nominal, 1.0),
            BadgeKind::Amber => (UiColor::Accent, 1.0),
            BadgeKind::Blue => (UiColor::Info, 1.0),
            BadgeKind::Red => (UiColor::Danger, 1.0),
            BadgeKind::Mute => (UiColor::Label, 0.9),
        }
    }
}

/// Marks a [`badge`], carrying the family and the RAW legend.
///
/// The raw legend, not the rendered one: the theme decides whether a badge
/// reads `[ONLINE]` or `ONLINE`, so the brackets cannot be baked in at spawn or
/// a flip would either keep them or double them.
#[derive(Component)]
pub struct ThemedBadge {
    kind: BadgeKind,
    text: String,
}

/// Marks a badge's legend span.
#[derive(Component)]
pub(super) struct BadgeLabel;

/// A status badge. Its shape is the theme's: the terminal look draws a
/// bracketed `[TAG]` text span in the family colour with no chrome, the
/// hardware look a bordered, tinted chip.
pub fn badge(kind: BadgeKind, text: &str) -> impl Bundle {
    (
        ThemedBadge {
            kind,
            text: text.to_uppercase(),
        },
        Node {
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![(
            BadgeLabel,
            UiText,
            Text::new(String::new()),
            TextFont {
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(Color::NONE),
        )],
    )
}

/// Paint badges on a theme change and on spawn, legend included.
pub(super) fn reconcile_badges(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(
        Entity,
        &ThemedBadge,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
        &Children,
    )>,
    changed: Query<Entity, Changed<ThemedBadge>>,
    mut q_label: Query<(&mut Text, &mut TextColor), With<BadgeLabel>>,
) {
    let restyle_all = theme.is_changed();
    let dirty: HashSet<Entity> = changed.iter().collect();
    if !restyle_all && dirty.is_empty() {
        return;
    }
    let paint = theme.badge();
    let radius = theme.metric(UiMetric::Radius);
    for (entity, badge, mut node, mut bg, mut border, children) in &mut q {
        if !restyle_all && !dirty.contains(&entity) {
            continue;
        }
        let (family, alpha) = badge.kind.tone();
        let color = theme.color_alpha(family, alpha);
        node.padding = UiRect::axes(px(paint.padding_x), px(3.0));
        node.border = UiRect::all(px(paint.border_width));
        node.border_radius = BorderRadius::all(px(radius));
        border.set_all(color.with_alpha(color.alpha() * paint.border_alpha));
        *bg = color.with_alpha(color.alpha() * paint.fill_alpha).into();
        for &child in children {
            if let Ok((mut text, mut text_color)) = q_label.get_mut(child) {
                text.0 = if paint.bracketed {
                    format!("[{}]", badge.text)
                } else {
                    badge.text.clone()
                };
                *text_color = TextColor(color);
            }
        }
    }
}

/// Marks a [`checkbox`], carrying whether it is checked.
///
/// Flip the box by writing this component; the reconciler repaints it and
/// rewrites its glyph. There is no separate "recolour it in place" helper,
/// because two paint paths for one control is how they drift.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemedCheckbox(pub bool);

/// Marks a checkbox's glyph span.
#[derive(Component)]
pub(super) struct CheckboxGlyph;

/// A 22px checkbox. Off: a bordered empty square. On: the theme's inverted
/// square with a dark `x`.
pub fn checkbox(on: bool) -> impl Bundle {
    (
        ThemedCheckbox(on),
        Node {
            width: px(22),
            height: px(22),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(BORDER_W)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![(
            CheckboxGlyph,
            UiText,
            Text::new(String::new()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::NONE),
        )],
    )
}

/// The checkbox glyph: `x` when checked, empty otherwise. Public because menu
/// rows draw the same mark as plain text beside a label, with no box.
pub fn checkbox_glyph(on: bool) -> &'static str {
    if on {
        "x"
    } else {
        ""
    }
}

/// Paint checkboxes on a theme change and whenever their state is written.
pub(super) fn reconcile_checkboxes(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(
        Entity,
        &ThemedCheckbox,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
        &Children,
    )>,
    changed: Query<Entity, Changed<ThemedCheckbox>>,
    mut q_glyph: Query<(&mut Text, &mut TextColor), With<CheckboxGlyph>>,
) {
    let restyle_all = theme.is_changed();
    let dirty: HashSet<Entity> = changed.iter().collect();
    if !restyle_all && dirty.is_empty() {
        return;
    }
    let paint = theme.checkbox();
    for (entity, checkbox, mut node, mut bg, mut border, children) in &mut q {
        if !restyle_all && !dirty.contains(&entity) {
            continue;
        }
        let state = paint.state(OnOff::from(checkbox.0));
        node.border_radius = BorderRadius::all(px(paint.radius));
        *bg = state.fill.base.into();
        border.set_all(state.border);
        for &child in children {
            if let Ok((mut text, mut color)) = q_glyph.get_mut(child) {
                text.0 = checkbox_glyph(checkbox.0).to_string();
                *color = TextColor(state.text);
            }
        }
    }
}

/// Marks a colour [`swatch`], whose BORDER is the theme's and whose fill is the
/// colour being shown.
#[derive(Component)]
pub struct ThemedSwatch;

/// A 22px block of one colour, for a row that edits a colour.
///
/// `None` - text that is not a colour yet, because it is half typed - paints
/// the empty case rather than black: a swatch showing a colour nobody chose
/// would be a lie, and black is a colour somebody might have.
pub fn swatch(colour: Option<Color>) -> impl Bundle {
    (
        ThemedSwatch,
        Node {
            width: px(22),
            height: px(22),
            flex_shrink: 0.0,
            border: UiRect::all(px(BORDER_W)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(colour.unwrap_or(Color::NONE)),
    )
}

/// Paint swatch BORDERS on a theme change and on spawn. The fill is the caller's
/// colour and is never touched - that is the whole content of the widget.
pub(super) fn reconcile_swatches(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(Entity, &mut Node, &mut BorderColor), With<ThemedSwatch>>,
    added: Query<Entity, Added<ThemedSwatch>>,
) {
    let restyle_all = theme.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    let border = theme.color_alpha(UiColor::Primary, 0.4);
    let radius = theme.metric(UiMetric::Radius);
    for (entity, mut node, mut border_color) in &mut q {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        node.border_radius = BorderRadius::all(px(radius));
        border_color.set_all(border);
    }
}

/// Marks a [`toggle`], carrying whether it is on. Write the component to flip
/// it; the reconciler moves and repaints the knob.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemedToggle(pub bool);

/// Marks a toggle's sliding knob.
#[derive(Component)]
pub(super) struct ToggleKnob;

/// A pill toggle switch: a 44x22 track with a sliding knob. Off: the knob sits
/// left, muted. On: it slides right and the track lights. Not a
/// `ThemedButton` - toggles carry their own click handler.
pub fn toggle(on: bool) -> impl Bundle {
    (
        ThemedToggle(on),
        Node {
            width: px(44),
            height: px(22),
            flex_shrink: 0.0,
            border: UiRect::all(px(BORDER_W)),
            align_items: AlignItems::Center,
            padding: UiRect::all(px(2)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            parent.spawn((
                ToggleKnob,
                Node {
                    width: px(16),
                    height: px(16),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ));
        })),
    )
}

/// Paint toggles on a theme change and whenever their state is written: the
/// track, the knob's tone and corner, the knob's glow, and which end it sits at.
pub(super) fn reconcile_toggles(
    theme: Res<ActiveUiTheme>,
    mut commands: Commands,
    mut q: Query<(
        Entity,
        &ThemedToggle,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
        &Children,
    )>,
    changed: Query<Entity, Changed<ThemedToggle>>,
    mut q_knob: Query<(&mut Node, &mut BackgroundColor), (With<ToggleKnob>, Without<ThemedToggle>)>,
) {
    let restyle_all = theme.is_changed();
    let dirty: HashSet<Entity> = changed.iter().collect();
    if !restyle_all && dirty.is_empty() {
        return;
    }
    let paint = theme.toggle();
    for (entity, toggle, mut node, mut bg, mut border, children) in &mut q {
        if !restyle_all && !dirty.contains(&entity) {
            continue;
        }
        let state = paint.state(OnOff::from(toggle.0));
        node.border_radius = BorderRadius::all(px(paint.radius));
        node.justify_content = if toggle.0 {
            JustifyContent::FlexEnd
        } else {
            JustifyContent::FlexStart
        };
        *bg = state.fill.base.into();
        border.set_all(state.border);
        for &child in children {
            let Ok((mut knob_node, mut knob_bg)) = q_knob.get_mut(child) else {
                continue;
            };
            knob_node.border_radius = BorderRadius::all(px(paint.knob_radius));
            // The state's TEXT tone is the knob and its EFFECT is the knob's
            // glow: a toggle has no legend, so the label slot carries the one
            // mark it does have.
            *knob_bg = state.text.into();
            // `try_*`: a toggle can be despawned the same frame this deferred
            // command is queued (a state teardown).
            let mut knob = commands.entity(child);
            match &state.shadow {
                Some(shadow) => {
                    knob.try_insert(shadow.clone());
                }
                None => {
                    knob.try_remove::<BoxShadow>();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        theme::{HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
        widget::fixtures::{bg, hardware, phosphor, select, themed_app},
    };

    fn label_of(app: &mut App, entity: Entity) -> String {
        let children: Vec<Entity> = app
            .world()
            .entity(entity)
            .get::<Children>()
            .unwrap()
            .iter()
            .collect();
        let mut q = app.world_mut().query::<&Text>();
        for child in children {
            if let Ok(text) = q.get(app.world(), child) {
                return text.0.clone();
            }
        }
        panic!("the widget has a text span");
    }

    /// A badge's SHAPE is the theme's, not the caller's: the terminal look
    /// brackets the legend and draws no chrome, the hardware look drops the
    /// brackets and draws a tinted chip. The legend is stored raw so a flip can
    /// go both ways - baking the brackets in at spawn left `[[TAG]]` behind.
    #[test]
    fn a_badge_takes_its_brackets_and_its_chip_from_the_theme() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let chip = app
            .world_mut()
            .spawn(badge(BadgeKind::Green, "online"))
            .id();
        app.update();
        assert_eq!(label_of(&mut app, chip), "[ONLINE]");
        assert_eq!(
            bg(&app, chip).alpha(),
            0.0,
            "no chip behind the legend on the terminal look"
        );

        select(&mut app, HARDWARE_THEME_ID);
        app.update();
        assert_eq!(label_of(&mut app, chip), "ONLINE");
        let expected = phosphor().color(UiColor::Primary);
        assert_eq!(
            bg(&app, chip),
            expected.with_alpha(hardware().badge().fill_alpha),
            "the hardware look tints a chip behind it"
        );
    }

    /// Writing [`ThemedCheckbox`] is the only way to flip a box: the reconciler
    /// owns the fill, the border and the glyph together, so a caller cannot set
    /// one and forget another.
    #[test]
    fn writing_the_checkbox_state_repaints_it_and_rewrites_its_glyph() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let box_e = app.world_mut().spawn(checkbox(false)).id();
        app.update();
        assert_eq!(label_of(&mut app, box_e), "");

        *app.world_mut()
            .entity_mut(box_e)
            .get_mut::<ThemedCheckbox>()
            .unwrap() = ThemedCheckbox(true);
        app.update();
        assert_eq!(label_of(&mut app, box_e), "x");
        assert_eq!(
            bg(&app, box_e),
            phosphor().checkbox().state(OnOff::On).fill.base,
            "and the box filled"
        );
    }

    /// Chrome re-themes LIVE. Every widget here used to read the look once at
    /// spawn, so a header, a separator and a toggle kept their old colours
    /// until the screen was rebuilt.
    #[test]
    fn chrome_repaints_on_a_theme_change() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let rule = app.world_mut().spawn(separator()).id();
        let switch = app.world_mut().spawn(toggle(true)).id();
        app.update();
        assert_eq!(bg(&app, rule), phosphor().separator());

        select(&mut app, HARDWARE_THEME_ID);
        app.update();
        assert_eq!(bg(&app, rule), hardware().separator());
        assert_eq!(
            app.world()
                .entity(switch)
                .get::<Node>()
                .unwrap()
                .border_radius
                .top_left,
            px(hardware().toggle().radius),
            "the toggle took the hardware pill corner"
        );
    }
}

#[cfg(test)]
mod separator_tests {
    use super::*;
    use crate::{
        screen::details_pane,
        theme::HARDWARE_THEME_ID,
        widget::fixtures::{bg, hardware, themed_app},
    };

    /// A pane that draws the rule as one of its EDGES keeps its face unpainted.
    ///
    /// `Node` requires a `BackgroundColor`, so a reconciler that painted
    /// whichever channel it found painted this one too - and washed the whole
    /// details pane beside every list in the separator tone. On the hardware
    /// look that was a flat green slab where the mod and lesson previews go.
    #[test]
    fn a_pane_takes_the_rule_on_its_border_and_not_on_its_face() {
        let mut app = themed_app(HARDWARE_THEME_ID);
        let pane = app.world_mut().spawn(details_pane()).id();
        app.update();

        assert_eq!(
            bg(&app, pane),
            Color::NONE,
            "the details pane painted its whole face in the separator tone"
        );
        let border = app.world().entity(pane).get::<BorderColor>().unwrap();
        assert_eq!(border.left, hardware().separator(), "the pane's own rule");
    }

    /// A node that IS the rule still takes it on its face.
    #[test]
    fn a_rule_takes_the_separator_tone_on_its_face() {
        let mut app = themed_app(HARDWARE_THEME_ID);
        let rule = app.world_mut().spawn(separator()).id();
        app.update();
        assert_eq!(bg(&app, rule), hardware().separator());
    }
}
