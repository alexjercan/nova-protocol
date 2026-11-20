//! The layout kit: one metrics scale and the builders every tool surface
//! composes its blocks from.
//!
//! The theme owns paint only. Every size, padding, gap and border width a
//! control takes comes from [`metrics`], so two themes lay a screen out to the
//! same rects and a theme switch never moves a control.
//!
//! The kit also owns three behaviours that used to be copied per pane:
//!
//! - [`VerbGate`]: how a verb answers its place and state. A verb that cannot
//!   work here is hidden, a verb short of a resource is disabled, and a verb
//!   blocked by a hidden cause shows one [`tag`] word. No verb carries a
//!   refusal sentence.
//! - [`Short`]: a value that blocks its verb paints in the danger tone.
//! - [`amount_row`]: a field, its "/ max", a slider and Max on one line, kept
//!   in step by [`sync_amount_rows`] and reported once per move as
//!   [`AmountChanged`].

use bevy::{
    ecs::relationship::RelatedSpawner,
    picking::hover::Hovered,
    platform::collections::HashSet,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{
        Activate, Slider, SliderPrecision, SliderRange, SliderStep, SliderValue, TrackClick,
    },
};

use super::{
    button, slider_track, text_field, ButtonSpec, TextFieldFocused, TextFieldSpec, TextFieldValue,
    ThemedBorder, ThemedFill, ThemedRadius, ThemedText, UiText,
};
use crate::theme::{ActiveUiTheme, UiColorType, UiMetric, BORDER_W};

/// The one size scale of the tool surfaces, in logical px at UI scale 1.
///
/// A theme cannot move any of these: a value here is layout, and layout is
/// the screen's.
///
/// The spacing scale. Every gap and padding on a screen is one of these:
///
/// | Token | px | Use |
/// |---|---|---|
/// | [`GAP_S`](metrics::GAP_S) | 4 | inside a control, tab to tab, chip to chip, list row to list row |
/// | [`GAP`](metrics::GAP) | 8 | button to button in a stack and in a row, heading to content |
/// | [`GAP_L`](metrics::GAP_L) | 12 | group to group, panel to panel, card and side-panel padding, actions row to content |
/// | [`GAP_XL`](metrics::GAP_XL) | 16 | list pane to details pane, interface root padding |
/// | [`PANEL_PAD_MENU`](metrics::PANEL_PAD_MENU) | 20 | menu window and dialog padding |
///
/// The gap between two siblings belongs to their container: a control carries
/// no margin. [`button_stack`], [`action_row`] and [`group_stack`] are the
/// containers with the gap built in.
pub mod metrics {
    /// Every button, field, chip, tab, slider slot and verb row.
    pub const CONTROL_H: f32 = 24.0;
    /// A main-menu button: the menu screens only ([`super::ButtonSpec::menu`]).
    pub const MENU_H: f32 = 40.0;
    /// A key-value row, a block header row.
    pub const ROW_H: f32 = 20.0;
    /// A selectable list row. A verb in it stays [`CONTROL_H`] and centred.
    pub const LIST_ROW_H: f32 = 28.0;
    /// A one-word [`super::tag`].
    pub const TAG_H: f32 = 16.0;
    /// A progress or condition [`super::bar`].
    pub const BAR_H: f32 = 6.0;
    /// The slider track inside its [`CONTROL_H`] slot.
    pub const SLIDER_TRACK_H: f32 = 10.0;
    /// Inside a control (icon and label), tab to tab, chip to chip, tag to
    /// tag, list row to list row.
    pub const GAP_S: f32 = 4.0;
    /// Button to button in a stack and in a row, row to row in a block, a
    /// heading to its content.
    pub const GAP: f32 = 8.0;
    /// Group to group, panel to panel, block to block, the padding of a card
    /// and of a side panel, an actions row to the content over it.
    pub const GAP_L: f32 = 12.0;
    /// A list pane to its details pane, and the interface root's padding.
    pub const GAP_XL: f32 = 16.0;
    /// The padding of a menu window and of a dialog.
    pub const PANEL_PAD_MENU: f32 = 20.0;
    /// A control's side padding. Its vertical padding is zero: the height is
    /// fixed.
    pub const PAD_X: f32 = 8.0;
    /// The narrowest a labelled verb gets.
    pub const VERB_MIN_W: f32 = 56.0;
    /// The amount field of an [`super::amount_row`].
    pub const AMOUNT_FIELD_W: f32 = 56.0;
    /// The label column of a [`super::spawn_kv`] row.
    pub const LABEL_W: f32 = 84.0;
    /// An [`super::inspector`]'s width.
    pub const SIDE_PANEL_W: f32 = 300.0;
    /// The icon frame at an inspector's head.
    pub const PREVIEW: f32 = 56.0;
    /// Labels, tags, column heads, block titles.
    pub const TEXT_LABEL: f32 = 11.0;
    /// Body text, values, control labels.
    pub const TEXT: f32 = 12.0;
    /// An inspector title, a pane head title.
    pub const TEXT_TITLE: f32 = 14.0;
    /// A main-menu button's label.
    pub const TEXT_MENU: f32 = 16.0;
}

use metrics::*;

// -- text --------------------------------------------------------------------

/// What a span of text is: its size and its theme colour come from this, never
/// from the spawn site.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    /// A label, a tag, a column head: small, secondary ink.
    Label,
    /// Running text: body size, body ink.
    Body,
    /// A value: body size, full ink.
    Value,
    /// An inspector or pane title.
    Title,
    /// A block title: label size, upper case, accent ink. The one block-title
    /// style.
    Heading,
}

impl TextRole {
    /// The font size, in logical px.
    pub const fn size(self) -> f32 {
        match self {
            Self::Label | Self::Heading => TEXT_LABEL,
            Self::Body | Self::Value => TEXT,
            Self::Title => TEXT_TITLE,
        }
    }

    /// The theme colour the role paints in.
    pub const fn tone(self) -> UiColorType {
        match self {
            Self::Label => UiColorType::TextSecondary,
            Self::Body => UiColorType::TextBody,
            Self::Value | Self::Title => UiColorType::Text,
            Self::Heading => UiColorType::Accent,
        }
    }
}

/// A text span in `role`'s size and colour. A [`TextRole::Heading`] is upper
/// case.
pub fn text(value: impl Into<String>, role: TextRole) -> impl Bundle {
    text_toned(value, role, role.tone())
}

/// A text span in `role`'s size, painted `tone` instead of the role's own
/// colour: a state colour such as a danger figure or a nominal lamp word.
pub fn text_toned(value: impl Into<String>, role: TextRole, tone: UiColorType) -> impl Bundle {
    let value = value.into();
    let value = if role == TextRole::Heading {
        value.to_uppercase()
    } else {
        value
    };
    (
        role,
        UiText,
        Text::new(value),
        TextFont {
            font_size: FontSize::Px(role.size()),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(tone),
    )
}

/// A value that blocks its verb. While true, [`apply_short_tone`] paints the
/// span in the danger tone; false returns it to the tone it was spawned in.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Short(pub bool);

/// The tone a [`Short`] span returns to, captured the first time it is seen.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct ShortBase(UiColorType);

/// Paint every [`Short`] span Danger while it is short, and its own tone
/// otherwise.
pub(super) fn apply_short_tone(
    mut commands: Commands,
    mut q: Query<(Entity, &Short, &mut ThemedText, Option<&ShortBase>), Changed<Short>>,
) {
    for (entity, short, mut themed, base) in &mut q {
        let base = match base {
            Some(base) => base.0,
            None => {
                commands.entity(entity).try_insert(ShortBase(themed.color));
                themed.color
            }
        };
        let tone = if short.0 { UiColorType::Danger } else { base };
        if themed.color != tone {
            themed.color = tone;
        }
    }
}

// -- buttons and gates ---------------------------------------------------------

/// How a verb answers its place and its state.
///
/// A verb that cannot work in this place or mode is [`Hidden`](Self::Hidden):
/// the pane head names the mode once. A verb short of a resource is
/// [`Disabled`](Self::Disabled), and the caller marks the blocking value
/// [`Short`]. A verb blocked by a cause the screen does not show is
/// [`Tagged`](Self::Tagged) with one upper-case word. A verb never carries a
/// sentence.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub enum VerbGate {
    /// The verb works.
    #[default]
    Ready,
    /// The verb cannot work here: `Display::None`, so it takes no space.
    Hidden,
    /// A resource is short: disabled, no text.
    Disabled,
    /// A hidden cause: disabled, with this word as a [`tag`] before it.
    Tagged(&'static str),
}

/// The [`tag`] a [`VerbGate::Tagged`] verb shows before itself.
#[derive(Component)]
pub(super) struct GateTag(Entity);

/// Apply each changed [`VerbGate`]: the verb's `Display`, its
/// `InteractionDisabled`, and the tag before it in its row.
pub(super) fn apply_verb_gates(
    mut commands: Commands,
    mut q: Query<
        (
            Entity,
            &VerbGate,
            &mut Node,
            Has<InteractionDisabled>,
            Option<&GateTag>,
            Option<&ChildOf>,
        ),
        Changed<VerbGate>,
    >,
    q_children: Query<&Children>,
) {
    for (entity, gate, mut node, disabled, tag, parent) in &mut q {
        let display = if *gate == VerbGate::Hidden {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
        let want_disabled = matches!(gate, VerbGate::Disabled | VerbGate::Tagged(_));
        match (want_disabled, disabled) {
            (true, false) => {
                commands.entity(entity).try_insert(InteractionDisabled);
            }
            (false, true) => {
                commands.entity(entity).try_remove::<InteractionDisabled>();
            }
            _ => {}
        }
        if let Some(tag) = tag {
            commands.entity(tag.0).try_despawn();
            commands.entity(entity).try_remove::<GateTag>();
        }
        if let (VerbGate::Tagged(word), Some(parent)) = (gate, parent) {
            let index = q_children
                .get(parent.parent())
                .ok()
                .and_then(|children| children.iter().position(|child| child == entity))
                .unwrap_or(0);
            let tag = commands
                .spawn((self::tag(word, UiColorType::TextSecondary), GateTagOf))
                .id();
            commands
                .entity(parent.parent())
                .insert_children(index, &[tag]);
            commands.entity(entity).try_insert(GateTag(tag));
        }
    }
}

/// Marks the [`tag`] a [`VerbGate::Tagged`] verb spawned, so a test can find
/// it apart from tags a screen spawns itself.
#[derive(Component)]
pub struct GateTagOf;

/// A square [`CONTROL_H`] verb that shows only `icon`, named `name`.
pub fn icon_verb(icon: Handle<Image>, name: &'static str) -> impl Bundle {
    (
        Name::new(name),
        button(ButtonSpec::control("").ghost().icon(icon)),
    )
}

// -- rows ------------------------------------------------------------------------

/// Which end of a [`verb_row`] the verbs gather at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowAlign {
    /// The start of the row.
    Start,
    /// The end of the row: a block's verbs.
    End,
}

/// A row of verbs: [`CONTROL_H`] high, [`GAP`] apart, centred on one line,
/// never shrunk. The primary verb goes last. A fixed-width verb cell spreads
/// it: `Node { width, ..verb_row(RowAlign::End) }`.
pub fn verb_row(align: RowAlign) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: match align {
            RowAlign::Start => JustifyContent::FlexStart,
            RowAlign::End => JustifyContent::FlexEnd,
        },
        column_gap: px(GAP),
        min_height: px(CONTROL_H),
        flex_shrink: 0.0,
        ..default()
    }
}

/// A row of mixed content - text, tags, controls - centred on one line, [`GAP`]
/// apart, never shrunk.
pub fn control_row(justify: JustifyContent) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: justify,
        column_gap: px(GAP),
        flex_shrink: 0.0,
        ..default()
    }
}

/// A column of buttons, [`GAP`] apart: a menu's button list, a dialog's
/// answers. The stack owns the gap, so a button in it carries no margin. Each
/// button takes the stack's width unless the caller sets `align_items`.
pub fn button_stack() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(GAP),
        ..default()
    }
}

/// A row of buttons, [`GAP`] apart, gathered by `justify`. It wraps, so a
/// narrow pane drops a button to a second line [`GAP`] under the first
/// instead of clipping it. For a commit row of any button height; a row of
/// [`CONTROL_H`] verbs that must stay on one line is a [`verb_row`].
pub fn action_row(justify: JustifyContent) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        justify_content: justify,
        column_gap: px(GAP),
        row_gap: px(GAP),
        ..default()
    }
}

/// A column of groups, [`GAP_L`] apart: the [`button_stack`]s of a menu, the
/// cards of a pane. A group is closer inside than it is to the next group,
/// which is what makes it read as one.
pub fn group_stack() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(GAP_L),
        ..default()
    }
}

/// A [`ROW_H`] row: `label` at [`LABEL_W`], `value` right-aligned in the rest.
/// A long value wraps. Returns the value's entity, so the caller can mark it.
pub fn spawn_kv<'a>(
    parent: &'a mut ChildSpawnerCommands,
    label: &str,
    value: impl Bundle,
) -> EntityCommands<'a> {
    let (_, value) = spawn_kv_parts(parent, (), text(label, TextRole::Label), value);
    parent.commands_mut().entity(value)
}

/// A [`spawn_kv`] row whose label word and shown state follow what it reads,
/// as a panel that names a contact's range and a chart cell's coordinates in
/// one slot does. `row` marks the row, whose `Display` hides it whole;
/// `label` marks the [`TextRole::Label`] span, which starts empty. Returns
/// the row.
pub fn spawn_kv_slot(
    parent: &mut ChildSpawnerCommands,
    row: impl Bundle,
    label: impl Bundle,
    value: impl Bundle,
) -> Entity {
    spawn_kv_parts(parent, row, (label, text("", TextRole::Label)), value).0
}

/// Spawn a kv row from its marked row, its label span and its value; returns
/// the row and the value.
fn spawn_kv_parts(
    parent: &mut ChildSpawnerCommands,
    row: impl Bundle,
    label: impl Bundle,
    value: impl Bundle,
) -> (Entity, Entity) {
    let row = parent
        .spawn((
            row,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(GAP),
                min_height: px(ROW_H),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let commands = parent.commands_mut();
    commands.spawn((
        label,
        Node {
            width: px(LABEL_W),
            flex_shrink: 0.0,
            ..default()
        },
        ChildOf(row),
    ));
    let cell = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            ChildOf(row),
        ))
        .id();
    let value = commands
        .spawn((
            value,
            TextLayout::new(Justify::Right, LineBreak::WordBoundary),
            ChildOf(cell),
        ))
        .id();
    (row, value)
}

/// Marks a block's header row, whose right side holds tags or a header verb.
#[derive(Component)]
pub struct BlockHead;

/// Marks a block's title text, so a block whose title follows its state (an
/// Inventory draft names its action) can write it.
#[derive(Component)]
pub struct BlockTitle;

/// The frame of a section block: a column [`GAP`] between rows, under its
/// own top rule and [`GAP_L`] of space. The rule belongs to the block, so a
/// hidden block takes its rule with it. A block that rebuilds its rows whole,
/// header included, wears this and spawns its header with [`block_header`];
/// a block with a fixed title uses [`spawn_block`].
pub fn block() -> (Node, BorderColor, ThemedBorder) {
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(GAP),
            padding: UiRect::top(px(GAP_L)),
            border: UiRect::top(px(BORDER_W)),
            flex_shrink: 0.0,
            ..default()
        },
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColorType::TextSecondary, 0.35),
    )
}

/// A block's [`ROW_H`] header row: `title` as a [`TextRole::Heading`] marked
/// [`BlockTitle`] on the left, a [`BlockHead`] slot filling the rest.
pub fn block_header(title: impl Into<String>) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(GAP_S),
            min_height: px(ROW_H),
            ..default()
        },
        Pickable::IGNORE,
        children![
            (BlockTitle, text(title, TextRole::Heading)),
            (
                BlockHead,
                Node {
                    flex_grow: 1.0,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    column_gap: px(GAP_S),
                    ..default()
                },
            )
        ],
    )
}

/// A section [`block`] whose first row is the [`block_header`] for `title`.
/// `marker` goes on the block; the block is returned, and rows spawned into
/// it follow the header.
pub fn spawn_block<'a>(
    parent: &'a mut ChildSpawnerCommands,
    title: &str,
    marker: impl Bundle,
) -> EntityCommands<'a> {
    parent.spawn((marker, block(), children![block_header(title.to_string())]))
}

// -- tags and chips --------------------------------------------------------------

/// Marks a [`tag`], carrying its tone and its word.
#[derive(Component)]
pub struct ThemedTag {
    tone: UiColorType,
}

/// Marks a tag's word span.
#[derive(Component)]
pub(super) struct TagLabel;

/// A one-word tag: [`TAG_H`] high, [`GAP_S`] side padding, a [`BORDER_W`]
/// border, the word upper case at [`TEXT_LABEL`]. The theme paints its border
/// and fill from the `badge` role; a transparent border is the border-less
/// look, at the same size.
pub fn tag(word: &str, tone: UiColorType) -> impl Bundle {
    (
        ThemedTag { tone },
        Node {
            height: px(TAG_H),
            padding: UiRect::axes(px(GAP_S), px(0)),
            border: UiRect::all(px(BORDER_W)),
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![(
            TagLabel,
            UiText,
            Text::new(word.to_uppercase()),
            TextFont {
                font_size: FontSize::Px(TEXT_LABEL),
                ..default()
            },
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
            TextColor(Color::NONE),
        )],
    )
}

/// Paint tags on a theme change and on spawn.
pub(super) fn reconcile_tags(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(
        Entity,
        &ThemedTag,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
        &Children,
    )>,
    changed: Query<Entity, Changed<ThemedTag>>,
    mut q_label: Query<&mut TextColor, With<TagLabel>>,
) {
    let restyle_all = theme.is_changed();
    let dirty: HashSet<Entity> = changed.iter().collect();
    if !restyle_all && dirty.is_empty() {
        return;
    }
    let paint = theme.badge();
    let radius = theme.metric(UiMetric::Radius);
    for (entity, tag, mut node, mut bg, mut border, children) in &mut q {
        if !restyle_all && !dirty.contains(&entity) {
            continue;
        }
        let color = theme.color(tag.tone);
        node.border_radius = BorderRadius::all(px(radius));
        border.set_all(color.with_alpha(color.alpha() * paint.border_alpha));
        *bg = color.with_alpha(color.alpha() * paint.fill_alpha).into();
        for &child in children {
            if let Ok(mut text_color) = q_label.get_mut(child) {
                *text_color = TextColor(color);
            }
        }
    }
}

/// A bar of [`chip`]s, [`GAP_S`] apart, wrapping when the panel is narrow.
pub fn chip_bar() -> impl Bundle {
    Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        column_gap: px(GAP_S),
        row_gap: px(GAP_S),
        flex_shrink: 0.0,
        ..default()
    }
}

/// One option of a [`chip_bar`]: a [`CONTROL_H`] ghost verb with an optional
/// icon before its label. The caller adds the value and [`super::Selected`].
pub fn chip(label: &str, icon: Option<Handle<Image>>) -> impl Bundle {
    let spec = ButtonSpec::control(label).ghost();
    button(match icon {
        Some(icon) => spec.icon(icon),
        None => spec,
    })
}

// -- surfaces ----------------------------------------------------------------------

/// The fill and border of an inspector or card.
pub fn panel_paint() -> impl Bundle {
    (
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColorType::TextSecondary, 0.08),
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColorType::TextSecondary),
    )
}

/// A side panel or inspector: [`SIDE_PANEL_W`] wide, [`GAP_L`] padding and
/// block gap. Content past its height takes `overflow`.
pub fn inspector(overflow: Overflow) -> impl Bundle {
    (
        Node {
            width: px(SIDE_PANEL_W),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(GAP_L),
            padding: UiRect::all(px(GAP_L)),
            border: UiRect::all(px(BORDER_W)),
            overflow,
            ..default()
        },
        panel_paint(),
        ThemedRadius::control(),
    )
}

/// A card: [`GAP_L`] padding, [`GAP`] between rows, the panel paint.
pub fn card() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(GAP),
            padding: UiRect::all(px(GAP_L)),
            border: UiRect::all(px(BORDER_W)),
            ..default()
        },
        panel_paint(),
        ThemedRadius::control(),
    )
}

/// The [`PREVIEW`] square at an inspector's head that frames the selection's
/// icon.
pub fn preview_frame() -> impl Bundle {
    (
        Node {
            width: px(PREVIEW),
            height: px(PREVIEW),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(BORDER_W)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColorType::Surface, 0.6),
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColorType::TextSecondary),
        ThemedRadius::control(),
    )
}

/// A fixed list column `width` wide that holds its content at its left edge,
/// so a column head and every value under it share one edge. A fixed-width
/// right-justified text does not right-align in bevy_ui; flex placement is
/// what aligns a cell.
pub fn cell(width: f32) -> impl Bundle {
    (
        Node {
            width: px(width),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// A one-pixel rule between groups of a panel.
pub fn divider() -> impl Bundle {
    (
        Node {
            height: px(BORDER_W),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColorType::TextSecondary, 0.35),
    )
}

/// A progress or condition bar at a fraction `[0, 1]`. Write the fraction to
/// [`Bar`]; [`sync_bars`] moves the fill.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Bar(pub f32);

/// Marks a [`bar`]'s fill.
#[derive(Component)]
pub struct BarFill;

/// A [`BAR_H`] bar in `tone`: the one bar look.
pub fn bar(fraction: f32, tone: UiColorType) -> impl Bundle {
    let fraction = fraction.clamp(0.0, 1.0);
    (
        Bar(fraction),
        Node {
            width: percent(100),
            height: px(BAR_H),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(tone, 0.18),
        ThemedRadius::control(),
        children![(
            BarFill,
            Node {
                width: percent(fraction * 100.0),
                height: percent(100),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::new(tone),
            ThemedRadius::control(),
        )],
    )
}

/// Move each changed [`Bar`]'s fill to its fraction.
pub(super) fn sync_bars(
    q_bar: Query<(&Bar, &Children), Changed<Bar>>,
    mut q_fill: Query<&mut Node, With<BarFill>>,
) {
    for (bar, children) in &q_bar {
        for &child in children {
            if let Ok(mut node) = q_fill.get_mut(child) {
                node.width = percent(bar.0.clamp(0.0, 1.0) * 100.0);
            }
        }
    }
}

// -- amount row ----------------------------------------------------------------------

/// The value of an [`amount_row`] and its range. The caller writes it to set
/// the row; the row writes `value` back when the player moves it, and fires
/// [`AmountChanged`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Amount {
    /// The amount.
    pub value: u32,
    /// The least it can be.
    pub min: u32,
    /// The most it can be.
    pub max: u32,
}

impl Amount {
    /// `value` clamped into the range.
    pub fn clamped(self, value: u32) -> u32 {
        value.clamp(self.min, self.max.max(self.min))
    }
}

/// Fired on an [`amount_row`] when the field, the slider, the wheel or Max
/// moves its value. Not fired when the caller writes [`Amount`].
#[derive(EntityEvent, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmountChanged {
    /// The row.
    pub entity: Entity,
    /// The new value.
    pub value: u32,
}

/// What an [`amount_row`]'s parts are named: `{name}Field`, `{name}Top`,
/// `{name}Slider` and `{name}Max`, so a walk can find them.
#[derive(Clone, Copy, Debug)]
pub struct AmountSpec {
    /// The name prefix.
    pub name: &'static str,
}

/// What the parts of an amount row show, so [`sync_amount_rows`] writes a
/// part only when the [`Amount`] moved past it.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct AmountShown {
    value: u32,
    min: u32,
    max: u32,
}

/// Marks an amount row's field.
#[derive(Component)]
pub struct AmountField;

/// Marks an amount row's "/ max" label.
#[derive(Component)]
pub struct AmountTop;

/// Marks an amount row's slider.
#[derive(Component)]
pub struct AmountSlider;

/// Marks an amount row's Max verb.
#[derive(Component)]
pub struct AmountMax;

/// One amount line: field | "/ max" | slider | Max, every part [`CONTROL_H`]
/// high on one centre line. Starts at 1 of 1; the caller writes [`Amount`].
pub fn amount_row(spec: AmountSpec) -> impl Bundle {
    let name = spec.name;
    (
        Amount {
            value: 1,
            min: 1,
            max: 1,
        },
        AmountShown {
            value: 1,
            min: 1,
            max: 1,
        },
        Hovered::default(),
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(GAP),
            height: px(CONTROL_H),
            flex_shrink: 0.0,
            ..default()
        },
        Children::spawn(SpawnWith(move |row: &mut RelatedSpawner<ChildOf>| {
            row.spawn(Node {
                width: px(AMOUNT_FIELD_W),
                flex_shrink: 0.0,
                ..default()
            })
            .with_child((
                AmountField,
                Name::new(format!("{name}Field")),
                text_field(TextFieldSpec::new("1").max_chars(5).tool()),
            ));
            row.spawn((
                AmountTop,
                Name::new(format!("{name}Top")),
                text("/ 1", TextRole::Value),
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            row.spawn(Node {
                flex_grow: 1.0,
                min_width: px(0),
                height: px(CONTROL_H),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_child((
                AmountSlider,
                Name::new(format!("{name}Slider")),
                Slider {
                    track_click: TrackClick::Snap,
                    ..default()
                },
                SliderValue(1.0),
                SliderRange::new(1.0, 1.0),
                SliderStep(1.0),
                SliderPrecision(0),
                slider_track(1.0),
            ));
            row.spawn((
                AmountMax,
                Name::new(format!("{name}Max")),
                button(ButtonSpec::control("Max").ghost()),
            ));
        })),
    )
}

/// The amount row an entity sits in, walking up from a part.
fn amount_row_of(
    mut entity: Entity,
    q_parent: &Query<&ChildOf>,
    is_row: impl Fn(Entity) -> bool,
) -> Option<Entity> {
    for _ in 0..4 {
        if is_row(entity) {
            return Some(entity);
        }
        entity = q_parent.get(entity).ok()?.parent();
    }
    None
}

/// Keep every [`amount_row`] in step: a typed field or a dragged slider moves
/// the [`Amount`] (clamped) and fires one [`AmountChanged`]; then a moved
/// [`Amount`] is written to the field, the "/ max" label and the slider.
#[expect(
    clippy::type_complexity,
    reason = "one system owns every part of the row"
)]
pub(super) fn sync_amount_rows(
    mut commands: Commands,
    mut q_rows: Query<(Entity, &mut Amount, &mut AmountShown)>,
    q_parent: Query<&ChildOf>,
    q_children: Query<&Children>,
    mut q_field: Query<
        (Entity, &mut TextFieldValue, Has<TextFieldFocused>),
        (With<AmountField>, Without<AmountSlider>),
    >,
    q_slider: Query<(Entity, Ref<SliderValue>, &SliderRange), With<AmountSlider>>,
    mut q_top: Query<&mut Text, With<AmountTop>>,
) {
    if q_rows.is_empty() {
        return;
    }
    // The player's moves first: a typed field, a dragged slider. A part
    // spawned this frame holds the row's spawn values, not a move: a caller
    // that writes the [`Amount`] on the row's first frame (a view reopened
    // on a kept count) would otherwise get it reset to 1.
    let mut moved: Vec<(Entity, u32, bool)> = Vec::new();
    for (field, value, _) in &mut q_field {
        if !value.is_changed() || value.is_added() {
            continue;
        }
        if let Ok(typed) = value.0.trim().parse::<u32>() {
            moved.push((field, typed, true));
        }
    }
    for (slider, value, _) in &q_slider {
        if value.is_changed() && !value.is_added() {
            moved.push((slider, value.0.round().max(0.0) as u32, false));
        }
    }
    for (part, typed, from_field) in moved {
        let Some(row) = amount_row_of(part, &q_parent, |e| q_rows.contains(e)) else {
            continue;
        };
        let Ok((_, mut amount, _)) = q_rows.get_mut(row) else {
            continue;
        };
        let next = amount.clamped(typed);
        if next != amount.value {
            amount.value = next;
            commands.trigger(AmountChanged {
                entity: row,
                value: next,
            });
        } else if from_field && typed != next {
            // Clamped back to the value it already holds: rewrite the field.
            amount.set_changed();
        }
    }

    // Then every moved amount out to its parts.
    for (row, amount, mut shown) in &mut q_rows {
        let next = AmountShown {
            value: amount.value,
            min: amount.min,
            max: amount.max,
        };
        if !amount.is_changed() && *shown == next {
            continue;
        }
        *shown = next;
        for part in q_children.iter_descendants(row) {
            if let Ok((_, mut field, focused)) = q_field.get_mut(part) {
                let text = amount.value.to_string();
                let typed = field.0.trim().parse::<u32>().ok();
                // A focused field that already reads the value keeps what the
                // player typed (a leading zero, a space).
                if field.0 != text && !(focused && typed == Some(amount.value)) {
                    field.0 = text;
                }
            }
            if let Ok((_, value, range)) = q_slider.get(part) {
                // Both are immutable components: a write is an insert.
                let wanted = SliderRange::new(amount.min as f32, amount.max.max(amount.min) as f32);
                if *range != wanted {
                    commands.entity(part).try_insert(wanted);
                }
                if value.0 != amount.value as f32 {
                    commands
                        .entity(part)
                        .try_insert(SliderValue(amount.value as f32));
                }
            }
            if let Ok(mut top) = q_top.get_mut(part) {
                let text = format!("/ {}", amount.max);
                if top.0 != text {
                    top.0 = text;
                }
            }
        }
    }
}

/// Max on an [`amount_row`]: move the amount to its top.
pub(super) fn amount_max_pressed(
    activate: On<Activate>,
    mut commands: Commands,
    q_max: Query<(), With<AmountMax>>,
    q_parent: Query<&ChildOf>,
    mut q_amount: Query<&mut Amount>,
) {
    if !q_max.contains(activate.entity) {
        return;
    }
    let Some(row) = amount_row_of(activate.entity, &q_parent, |e| q_amount.contains(e)) else {
        return;
    };
    if let Ok(mut amount) = q_amount.get_mut(row) {
        let top = amount.clamped(amount.max);
        if amount.value != top {
            amount.value = top;
            commands.trigger(AmountChanged {
                entity: row,
                value: top,
            });
        }
    }
}

/// The wheel over an [`amount_row`]: one step per notch.
pub(super) fn amount_wheel(
    scroll: On<PointerScroll>,
    mut commands: Commands,
    mut q_amount: Query<&mut Amount>,
) {
    if scroll.y == 0.0 {
        return;
    }
    let row = scroll.event_target();
    let Ok(mut amount) = q_amount.get_mut(row) else {
        return;
    };
    let step: i64 = if scroll.y > 0.0 { 1 } else { -1 };
    let next = amount.clamped((i64::from(amount.value) + step).max(0) as u32);
    if next != amount.value {
        amount.value = next;
        commands.trigger(AmountChanged {
            entity: row,
            value: next,
        });
    }
}
