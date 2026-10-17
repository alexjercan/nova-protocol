//! The Icons page: every gallery icon by family at 16, 24 and 48 px, in its
//! own tint and in the plain body ink, so a mask's read can be judged at the
//! sizes the pages use it.

use bevy::prelude::*;
use nova_ui::{prelude::*, theme::UiColor};

use crate::{
    chrome::*,
    icons::{icon_node, GalleryIcons, IconType},
    ICON_SHEET,
};

const SIZES: [f32; 3] = [16.0, 24.0, 48.0];

pub fn icon_sheet_page(body: &mut ChildSpawnerCommands, icons: &GalleryIcons) {
    menu_card(body, "Icons", "SHEET", |body| {
        body.spawn((
            Name::new(ICON_SHEET),
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .with_children(|sheet| {
            sheet.spawn(text(
                "Procedural coverage masks, drawn once at startup and tinted by the theme. Columns: 16, 24 and 48 px in the icon's tint, then 24 px in body ink.",
                12.0,
                UiColor::Body,
            ));
            for (family, members) in IconType::FAMILIES {
                sheet.spawn(divider());
                section(sheet, family);
                sheet
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(12),
                        row_gap: px(8),
                        ..default()
                    })
                    .with_children(|row| {
                        for &icon in members {
                            row.spawn((
                                Node {
                                    width: px(180),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(4),
                                    padding: UiRect::all(px(6)),
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(px(4)),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                ThemedFill::alpha(UiColor::Secondary, 0.06),
                                BorderColor::all(Color::NONE),
                                ThemedBorder::alpha(UiColor::Secondary, 0.35),
                            ))
                            .with_children(|card| {
                                card.spawn(Node {
                                    height: px(52),
                                    column_gap: px(12),
                                    align_items: AlignItems::FlexEnd,
                                    ..control_row(JustifyContent::FlexStart)
                                })
                                .with_children(|sizes| {
                                    for size in SIZES {
                                        sizes.spawn(icon_node(icons, icon, icon.color(), size));
                                    }
                                    sizes.spawn(icon_node(icons, icon, UiColor::Body, 24.0));
                                });
                                card.spawn(text(icon.label(), 12.0, UiColor::Primary));
                            });
                        }
                    });
            }
        });
    });
}
