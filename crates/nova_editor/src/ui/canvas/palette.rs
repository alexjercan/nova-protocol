//! The block palette: every kind in the owning tables, under coloured headings.
//!
//! Listing goes through [`BlockItemType::all`], so a new row in a table lands
//! here without a line in this file. Dropping a block on the palette bins it.

use bevy::{
    picking::{hover::Hovered, Pickable},
    prelude::*,
};
use nova_scenario::prelude::BlockCategoryType;
use nova_ui::prelude::*;

use super::{
    paint::{paint_stacked, Paint},
    view::BlockView,
};
use crate::{
    event::BlockItemType,
    palette::{block_colour, BLOCK_TEXT},
    ui::inspector::InspectorHint,
};

/// The palette's width.
pub(crate) const PALETTE_W: f32 = 200.0;

/// The palette's root.
#[derive(Component)]
pub(crate) struct BlockPalette;

/// The filter box above the list.
#[derive(Component)]
pub(crate) struct PaletteFilter;

/// One row: pressing it picks up a new block.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PaletteEntry(pub(crate) BlockItemType);

/// A category heading, hidden when the filter leaves it nothing.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PaletteHeading(pub(crate) BlockCategoryType);

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
            row_gap: px(6),
            padding: UiRect::all(px(8)),
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
            .spawn((Name::new("Palette Scroll"), scroll_row()))
            .with_children(|row| {
                row.spawn((
                    Name::new("Palette List"),
                    Node {
                        row_gap: px(4),
                        align_items: AlignItems::FlexStart,
                        ..scroll_column()
                    },
                    scroll_viewport(),
                ))
                .with_children(spawn_entries);
                row.spawn(scroll_bar());
            });
    });
}

fn spawn_entries(list: &mut ChildSpawnerCommands) {
    let items = BlockItemType::all();
    for category in BlockCategoryType::ALL {
        list.spawn((
            Name::new(format!("Palette Heading {}", category.label())),
            PaletteHeading(category),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                margin: UiRect::top(px(6)),
                ..default()
            },
            children![
                (
                    Node {
                        width: px(10),
                        height: px(10),
                        border_radius: BorderRadius::all(px(5)),
                        ..default()
                    },
                    BackgroundColor(block_colour(category)),
                ),
                (
                    UiText,
                    Text::new(category.label()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(BLOCK_TEXT),
                ),
            ],
        ));
        for item in items.iter().filter(|item| item.category() == category) {
            let view = BlockView::preview(*item);
            list.spawn((
                Name::new(format!("Palette {}", item.label())),
                PaletteEntry(*item),
                Hovered::default(),
                InspectorHint {
                    title: item.label(),
                    body: item.hint(),
                },
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
            .with_children(|entry| {
                paint_stacked(entry, &view, None, false, &mut Paint::picture());
            });
        }
    }
}

/// Narrow the list to what the filter box holds.
pub(crate) fn sync_palette_filter(
    filters: Query<&TextFieldValue, (With<PaletteFilter>, Changed<TextFieldValue>)>,
    mut entries: Query<(&PaletteEntry, &mut Node), Without<PaletteHeading>>,
    mut headings: Query<(&PaletteHeading, &mut Node), Without<PaletteEntry>>,
) {
    let Ok(filter) = filters.single() else {
        return;
    };
    let wanted = filter.0.trim().to_lowercase();
    let shown = |item: BlockItemType| {
        wanted.is_empty()
            || item.label().to_lowercase().contains(&wanted)
            || item.category().label().to_lowercase().contains(&wanted)
    };
    let display = |on: bool| if on { Display::Flex } else { Display::None };
    for (entry, mut node) in &mut entries {
        node.display = display(shown(entry.0));
    }
    let items = BlockItemType::all();
    for (heading, mut node) in &mut headings {
        let any = items
            .iter()
            .any(|item| item.category() == heading.0 && shown(*item));
        node.display = display(any);
    }
}
