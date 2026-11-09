//! The block palette: every kind in the owning tables, grouped by the role a
//! block plays, in the order a script is built.
//!
//! Listing goes through [`BlockItemType::all`], so a new row in a table lands
//! here without a line in this file. Each group's heading wears the colour its
//! blocks wear on the canvas. Each entry is a short line in its block's shape: a
//! hat's round top, a stack piece's notch and tab, a C-block's mouth, a
//! condition's points, a value's round ends. A click on a heading folds its
//! group. Dropping a block on the palette bins it.
//!
//! The rail at the left edge holds an All row and one dot per group. A click
//! on a group shows only that group's blocks; a second click on it, or a click
//! on All, shows every group again. The group and the filter box both apply,
//! and the rail marks the group it shows.

use std::collections::HashSet;

use bevy::{
    picking::{hover::Hovered, Pickable},
    prelude::*,
    ui_widgets::{observe, Activate, Button},
};
use nova_ui::{
    prelude::*,
    theme::{self, UiColorType},
    widget::{ListRow, Selected, ThemedBorder, ThemedRadius, ThemedText},
};

use super::{
    paint::paint_points,
    view::{BlockView, OutlineType},
};
use crate::{
    event::{BlockItemType, BlockRoleType},
    palette::{block_colour, BLOCK_SOCKET, BLOCK_TEXT, BLOCK_TEXT_DIM},
    ui::inspector::InspectorHint,
};

/// The palette's width.
pub(crate) const PALETTE_W: f32 = 240.0;

/// A group heading's height.
const HEADING_H: f32 = 18.0;

/// An entry's line height.
const ENTRY_H: f32 = 20.0;

/// An entry's text size.
const ENTRY_FONT: f32 = 11.0;

/// A hat's top corner radius on the palette.
const ENTRY_HAT_RADIUS: f32 = 9.0;

/// Where a stack piece's notch and tab stand in from its left edge.
const ENTRY_TAB_LEFT: f32 = 8.0;

/// A notch's or a tab's width and height.
const ENTRY_TAB: Vec2 = Vec2::new(12.0, 3.0);

/// A C-block's left bar and the mouth it frames.
const ENTRY_MOUTH: Vec2 = Vec2::new(6.0, 5.0);

/// The rail's width: room for the longest group name.
const RAIL_W: f32 = 58.0;

/// A rail dot's size.
const RAIL_DOT: f32 = 14.0;

/// The palette's root.
#[derive(Component)]
pub(crate) struct BlockPalette;

/// The filter box above the list.
#[derive(Component)]
pub(crate) struct PaletteFilter;

/// One row: pressing it picks up a new block.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PaletteEntry(pub(crate) BlockItemType);

/// A group heading, hidden when the filter leaves it nothing.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PaletteHeading(pub(crate) BlockRoleType);

/// The `-` or `+` on a heading.
#[derive(Component)]
pub(crate) struct HeadingFoldMark;

/// The scrolling list of headings and entries.
#[derive(Component)]
pub(crate) struct PaletteList;

/// A rail row: a click shows only its group. `None` is the All row.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PaletteRailChip(pub(crate) Option<BlockRoleType>);

/// The group the rail shows. `None` shows every group.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PaletteGroup(pub(crate) Option<BlockRoleType>);

impl PaletteGroup {
    /// What the rail shows after a click on `pressed`: a group shows alone,
    /// and a second click on it, or a click on All, shows every group.
    pub(crate) fn after_press(self, pressed: Option<BlockRoleType>) -> Self {
        Self(pressed.filter(|role| self.0 != Some(*role)))
    }

    /// The rail row's word for what it shows.
    pub(crate) fn label(self) -> &'static str {
        self.0.map_or("All", BlockRoleType::short)
    }
}

/// Whether the palette lists `item` for the filter text `wanted` (trimmed and
/// lower case) and the rail's `group`. Both apply.
pub(crate) fn palette_shows(item: BlockItemType, wanted: &str, group: PaletteGroup) -> bool {
    let in_group = group.0.is_none_or(|role| item.role() == role);
    let named = wanted.is_empty()
        || item.label().to_lowercase().contains(wanted)
        || item.role().label().to_lowercase().contains(wanted)
        || item
            .subject()
            .is_some_and(|subject| subject.label().to_lowercase().contains(wanted));
    in_group && named
}

/// The groups a click on their heading folded.
#[derive(Resource, Default, Debug)]
pub(crate) struct PaletteFolds(pub(crate) HashSet<BlockRoleType>);

/// The palette's entries, grouped and in group order: by role, then by subject,
/// then in table order.
pub(crate) fn palette_items() -> Vec<BlockItemType> {
    let mut items = BlockItemType::all();
    items.sort_by_key(|item| {
        let role = BlockRoleType::ALL
            .iter()
            .position(|role| *role == item.role())
            .unwrap_or(BlockRoleType::ALL.len());
        (role, item.subject())
    });
    items
}

/// A line of text in an entry.
fn entry_words(text: String) -> impl Bundle {
    (
        UiText,
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(ENTRY_FONT),
            ..default()
        },
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        TextColor(BLOCK_TEXT),
        Pickable::IGNORE,
    )
}

/// A notch or a tab: a small piece at a stack piece's top or foot.
fn entry_tab(colour: Color, top: bool) -> impl Bundle {
    let mut node = Node {
        width: px(ENTRY_TAB.x),
        height: px(ENTRY_TAB.y),
        border_radius: BorderRadius::bottom(px(2)),
        flex_shrink: 0.0,
        ..default()
    };
    if top {
        node.position_type = PositionType::Absolute;
        node.left = px(ENTRY_TAB_LEFT);
        node.top = px(0);
    } else {
        node.margin = UiRect::left(px(ENTRY_TAB_LEFT));
    }
    (node, BackgroundColor(colour), Pickable::IGNORE)
}

/// What a palette entry shows: one short line in the block's own shape. The
/// canvas draws the full block; the palette draws only what tells the shapes
/// apart, so a group of twenty hats stays short.
fn entry_picture(entry: &mut ChildSpawnerCommands, item: BlockItemType) {
    let view = BlockView::preview(item);
    let colour = view.colour;
    let line = |radius: BorderRadius| Node {
        height: px(ENTRY_H),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        padding: UiRect::horizontal(px(8)),
        border_radius: radius,
        flex_shrink: 0.0,
        ..default()
    };
    match view.outline {
        OutlineType::Hat => {
            entry
                .spawn((
                    line(BorderRadius {
                        top_left: px(ENTRY_HAT_RADIUS).into(),
                        top_right: px(ENTRY_HAT_RADIUS).into(),
                        bottom_left: px(2).into(),
                        bottom_right: px(2).into(),
                    }),
                    BackgroundColor(colour),
                    Pickable::IGNORE,
                ))
                .with_child(entry_words(item.label()));
            entry.spawn(entry_tab(colour, false));
        }
        OutlineType::Stack | OutlineType::Gate => {
            entry
                .spawn((
                    line(BorderRadius::all(px(2))),
                    BackgroundColor(colour),
                    Pickable::IGNORE,
                ))
                .with_children(|head| {
                    head.spawn(entry_tab(BLOCK_SOCKET, true));
                    head.spawn(entry_words(item.label()));
                });
            entry.spawn(entry_tab(colour, false));
        }
        OutlineType::Chain => {
            entry
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|block| {
                    block
                        .spawn((
                            line(BorderRadius::top(px(2))),
                            BackgroundColor(colour),
                            Pickable::IGNORE,
                        ))
                        .with_children(|head| {
                            head.spawn(entry_tab(BLOCK_SOCKET, true));
                            head.spawn(entry_words(item.label()));
                        });
                    block.spawn((
                        Node {
                            width: px(ENTRY_MOUTH.x),
                            height: px(ENTRY_MOUTH.y),
                            ..default()
                        },
                        BackgroundColor(colour),
                        Pickable::IGNORE,
                    ));
                    block.spawn((
                        Node {
                            height: px(ENTRY_TAB.y),
                            border_radius: BorderRadius::bottom(px(2)),
                            ..default()
                        },
                        BackgroundColor(colour),
                        Pickable::IGNORE,
                    ));
                });
            entry.spawn(entry_tab(colour, false));
        }
        OutlineType::Boolean => {
            entry
                .spawn((
                    Node {
                        padding: UiRect::horizontal(px(2)),
                        margin: UiRect::horizontal(px(ENTRY_H / 2.0)),
                        ..line(BorderRadius::ZERO)
                    },
                    BackgroundColor(colour),
                    Pickable::IGNORE,
                ))
                .with_children(|pill| {
                    paint_points(pill, ENTRY_H, colour);
                    pill.spawn(entry_words(item.label()));
                });
        }
        OutlineType::Reporter => {
            entry
                .spawn((
                    line(BorderRadius::all(px(ENTRY_H / 2.0))),
                    BackgroundColor(colour),
                    Pickable::IGNORE,
                ))
                .with_child(entry_words(item.label()));
        }
    }
}

/// A rail row: a dot and its group's short name, or a hollow dot and All.
///
/// A [`ListRow`], so the shared reconciler paints the hover and the mark of the
/// group shown from the theme. The dot keeps its group's block colour: it is
/// the key to the canvas, not a theme colour.
fn rail_chip(role: Option<BlockRoleType>) -> impl Bundle {
    let label = PaletteGroup(role).label();
    let dot = (
        Node {
            width: px(RAIL_DOT),
            height: px(RAIL_DOT),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(RAIL_DOT / 2.0)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColorType::TextMuted),
        BackgroundColor(role.map_or(Color::NONE, block_colour)),
        Pickable::IGNORE,
    );
    (
        Name::new(format!("Palette Rail {label}")),
        PaletteRailChip(role),
        ListRow,
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(2),
            padding: UiRect::vertical(px(3)),
            border: UiRect::all(px(theme::BORDER_W)),
            flex_shrink: 0.0,
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        observe(on_rail_press),
        children![
            dot,
            (
                UiText,
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(9.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColorType::Text),
                Pickable::IGNORE,
            ),
        ],
    )
}

/// Fill `area` with the palette.
pub(crate) fn spawn_block_palette(area: &mut ChildSpawnerCommands) {
    area.spawn((
        Name::new("Block Palette"),
        BlockPalette,
        Node {
            width: px(PALETTE_W),
            flex_shrink: 0.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            padding: UiRect::all(px(6)),
            ..default()
        },
    ))
    .with_children(|palette| {
        palette.spawn((
            Name::new("Palette Filter"),
            PaletteFilter,
            text_field(
                TextFieldSpec::new("")
                    .placeholder("Filter blocks")
                    .max_chars(32)
                    .tool(),
            ),
        ));
        palette
            .spawn((
                Name::new("Palette Body"),
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    min_height: px(0),
                    column_gap: px(4),
                    ..default()
                },
            ))
            .with_children(|body| {
                body.spawn((
                    Name::new("Palette Rail"),
                    Node {
                        width: px(RAIL_W),
                        flex_shrink: 0.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        ..default()
                    },
                ))
                .with_children(|rail| {
                    rail.spawn(rail_chip(None));
                    for role in BlockRoleType::ALL {
                        rail.spawn(rail_chip(Some(role)));
                    }
                });
                body.spawn((
                    Name::new("Palette Scroll"),
                    Node {
                        flex_grow: 1.0,
                        min_width: px(0),
                        ..scroll_row()
                    },
                ))
                .with_children(|row| {
                    row.spawn((
                        Name::new("Palette List"),
                        PaletteList,
                        Node {
                            row_gap: px(2),
                            align_items: AlignItems::FlexStart,
                            ..scroll_column()
                        },
                        scroll_viewport(),
                    ))
                    .with_children(spawn_entries);
                    row.spawn(scroll_bar());
                });
            });
    });
}

fn heading(role: BlockRoleType) -> impl Bundle {
    (
        Name::new(format!("Palette Heading {}", role.label())),
        PaletteHeading(role),
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            height: px(HEADING_H),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(5),
            padding: UiRect::horizontal(px(5)),
            margin: UiRect::top(px(4)),
            border_radius: BorderRadius::all(px(3)),
            flex_shrink: 0.0,
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(block_colour(role)),
        observe(on_heading_press),
        children![
            (
                HeadingFoldMark,
                UiText,
                Text::new("-"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(BLOCK_TEXT),
                Pickable::IGNORE,
            ),
            (
                UiText,
                Text::new(role.short()),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                TextColor(BLOCK_TEXT),
                Pickable::IGNORE,
            ),
            (
                UiText,
                Text::new(role.note()),
                TextFont {
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                TextColor(BLOCK_TEXT_DIM),
                Pickable::IGNORE,
            ),
        ],
    )
}

fn spawn_entries(list: &mut ChildSpawnerCommands) {
    let items = palette_items();
    for role in BlockRoleType::ALL {
        list.spawn(heading(role));
        for item in items.iter().filter(|item| item.role() == role) {
            let mut title = role.label().to_string();
            if let Some(subject) = item.subject() {
                title.push_str(" / ");
                title.push_str(subject.label());
            }
            title.push_str(" / ");
            title.push_str(&item.label());
            let hint = item.hint();
            let body = if hint.is_empty() {
                item.placement().to_string()
            } else {
                format!("{} {hint}", item.placement())
            };
            list.spawn((
                Name::new(format!("Palette {}", item.label())),
                PaletteEntry(*item),
                Hovered::default(),
                InspectorHint { title, body },
                Pickable {
                    should_block_lower: true,
                    is_hoverable: true,
                },
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    max_width: percent(100),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
            ))
            .with_children(|entry| entry_picture(entry, *item));
        }
    }
}

/// A rail click: show only that group, or every group again. The list goes
/// back to its top, where what is left of it starts.
fn on_rail_press(
    activate: On<Activate>,
    chips: Query<&PaletteRailChip>,
    mut lists: Query<&mut ScrollPosition, With<PaletteList>>,
    mut group: ResMut<PaletteGroup>,
) {
    let Ok(chip) = chips.get(activate.entity) else {
        return;
    };
    *group = group.after_press(chip.0);
    for mut scroll in &mut lists {
        if scroll.0.y != 0.0 {
            scroll.0.y = 0.0;
        }
    }
}

/// Mark the rail row of the group shown.
pub(crate) fn sync_palette_rail(
    mut commands: Commands,
    group: Res<PaletteGroup>,
    chips: Query<(Entity, &PaletteRailChip, Has<Selected>)>,
) {
    for (entity, chip, marked) in &chips {
        let wanted = chip.0 == group.0;
        if wanted && !marked {
            commands.entity(entity).insert(Selected);
        } else if !wanted && marked {
            commands.entity(entity).remove::<Selected>();
        }
    }
}

/// A click on a heading folds or opens its group.
fn on_heading_press(
    activate: On<Activate>,
    headings: Query<&PaletteHeading>,
    mut folds: ResMut<PaletteFolds>,
) {
    let Ok(heading) = headings.get(activate.entity) else {
        return;
    };
    if !folds.0.remove(&heading.0) {
        folds.0.insert(heading.0);
    }
}

/// Narrow the list to the rail's group, what the filter box holds and what is
/// not folded. A filter or a group opens what it shows, so a match is never
/// hidden in a folded group.
pub(crate) fn sync_palette_filter(
    filters: Query<Ref<TextFieldValue>, With<PaletteFilter>>,
    folds: Res<PaletteFolds>,
    group: Res<PaletteGroup>,
    mut entries: Query<(&PaletteEntry, &mut Node), Without<PaletteHeading>>,
    mut headings: Query<(&PaletteHeading, &mut Node, &Children), Without<PaletteEntry>>,
    mut marks: Query<&mut Text, With<HeadingFoldMark>>,
) {
    let Ok(filter) = filters.single() else {
        return;
    };
    if !filter.is_changed() && !folds.is_changed() && !group.is_changed() {
        return;
    }
    let wanted = filter.0.trim().to_lowercase();
    let matches = |item: BlockItemType| palette_shows(item, &wanted, *group);
    let open =
        |role: BlockRoleType| !wanted.is_empty() || group.0.is_some() || !folds.0.contains(&role);
    let display = |on: bool| if on { Display::Flex } else { Display::None };
    for (entry, mut node) in &mut entries {
        let on = display(matches(entry.0) && open(entry.0.role()));
        if node.display != on {
            node.display = on;
        }
    }
    let items = BlockItemType::all();
    for (heading, mut node, children) in &mut headings {
        let any = items
            .iter()
            .any(|item| item.role() == heading.0 && matches(*item));
        let on = display(any);
        if node.display != on {
            node.display = on;
        }
        let mark = if open(heading.0) { "-" } else { "+" };
        for child in children {
            if let Ok(mut text) = marks.get_mut(*child) {
                if text.0 != mark {
                    text.0 = mark.to_string();
                }
            }
        }
    }
}
