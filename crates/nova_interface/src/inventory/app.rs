//! The Inventory pane body: the category filters, the player and partner
//! columns and the inspector; the system that fills them from live
//! inventories; and the observers of the filter and row clicks.
//!
//! Touch this module when changing what the Inventory pane shows or how a row
//! is selected.

use bevy::{
    prelude::*,
    ui_widgets::{Activate, Button},
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::{DockedShip, DockingConnection};
use nova_ui::{
    theme::UiColor,
    widget::{ThemedBorder, ThemedFill, ThemedImageTint, ThemedText},
};

use super::{InventoryRuntime, InventorySideType};
use crate::{
    icons::{icon_node, InterfaceIcons},
    pane::{play_menu_select, themed_label},
};

/// Height of one inventory row, in logical px.
const ROW_PX: f32 = 28.0;
/// Width of the quantity column, in logical px.
const QTY_PX: f32 = 64.0;

/// The category filters left to right, after All. Display order only: icon
/// masks stay stored in [`ItemCategoryType`] declaration order.
pub(crate) const FILTER_ORDER: [ItemCategoryType; 5] = [
    ItemCategoryType::Food,
    ItemCategoryType::Ammo,
    ItemCategoryType::Repair,
    ItemCategoryType::Raw,
    ItemCategoryType::Parts,
];

/// A category filter button; `None` shows every category.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct InventoryFilterChip(pub(crate) Option<ItemCategoryType>);

/// The node one side's rows are listed in. `drawn` is what the rows show, so
/// [`update_inventory_panel`] rebuilds them only when it changes.
#[derive(Component, Debug)]
pub(crate) struct InventoryColumn {
    pub(crate) side: InventorySideType,
    drawn: Option<ColumnDraw>,
}

/// The heading over one side's rows: the ship's name.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct InventoryColumnTitle(pub(crate) InventorySideType);

/// One side's whole panel: border, title, headings and rows. Hidden, not
/// removed, while the side has no ship, so its slot keeps the columns equal.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct InventoryColumnPanel(pub(crate) InventorySideType);

/// A drawn stack. A click selects it for the inspector.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct InventoryRow {
    pub(crate) side: InventorySideType,
    pub(crate) item: ItemType,
}

/// A part of the inspector that shows with the selection or without it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InspectorPart {
    /// The hint with nothing selected.
    Hint,
    /// The selected item.
    Item,
}

/// A text of the inspector that [`update_inventory_panel`] fills.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InventoryInspectorField {
    /// The item's name.
    Name,
    /// The item's category.
    Category,
    /// What the item is.
    About,
    /// How many the selected side carries, and which ship that is.
    Stock,
}

/// The selected item's category icon in the inspector.
#[derive(Component)]
pub(crate) struct InventoryInspectorIcon;

/// What one column's rows show.
#[derive(Clone, PartialEq, Eq, Debug)]
struct ColumnDraw {
    /// Every stack the side carries, or `None` when there is no partner.
    stacks: Option<Vec<(ItemType, u32)>>,
    /// The filter the rows apply; `None` as well when there are no stacks, so
    /// a filter click does not redraw an absent partner.
    filter: Option<ItemCategoryType>,
}

/// The item's display name.
pub(crate) fn item_label(item: ItemType) -> &'static str {
    match item {
        ItemType::HullPlate => "Hull plate",
    }
}

/// A one-line "what it is" for the inspector.
fn item_about(item: ItemType) -> &'static str {
    match item {
        ItemType::HullPlate => "Structural plating for hull sections.",
    }
}

/// The filter and inspector word for a category.
fn category_label(category: ItemCategoryType) -> &'static str {
    match category {
        ItemCategoryType::Raw => "Raw",
        ItemCategoryType::Repair => "Repair",
        ItemCategoryType::Ammo => "Ammo",
        ItemCategoryType::Food => "Food",
        ItemCategoryType::Parts => "Parts",
    }
}

/// The tint of a category's icon, row and quantity.
fn category_color(category: ItemCategoryType) -> UiColor {
    match category {
        ItemCategoryType::Raw => UiColor::Secondary,
        ItemCategoryType::Repair => UiColor::Nominal,
        ItemCategoryType::Ammo => UiColor::Danger,
        ItemCategoryType::Food => UiColor::Accent,
        ItemCategoryType::Parts => UiColor::AccentHigh,
    }
}

/// Fill and border alpha of a row, selected or not.
fn row_alphas(selected: bool) -> (f32, f32) {
    if selected {
        (0.32, 1.0)
    } else {
        (0.1, 0.6)
    }
}

/// Colour, fill alpha and border alpha of a filter chip, current or not.
fn chip_paint(current: bool) -> (UiColor, f32, f32) {
    if current {
        (UiColor::Accent, 0.25, 1.0)
    } else {
        (UiColor::Secondary, 0.08, 0.4)
    }
}

/// A row of controls.
fn control_row(justify: JustifyContent) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: justify,
        column_gap: px(12),
        flex_shrink: 0.0,
        ..default()
    }
}

/// The inventory: the filters over the two columns, and the inspector beside
/// them. [`update_inventory_panel`] fills the columns and the inspector.
pub(crate) fn inventory_body(body: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    body.spawn(Node {
        flex_grow: 1.0,
        min_height: px(0),
        flex_direction: FlexDirection::Row,
        column_gap: px(12),
        ..default()
    })
    .with_children(|split| {
        split
            .spawn(Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                ..default()
            })
            .with_children(|stores| {
                stores
                    .spawn(Node {
                        column_gap: px(8),
                        ..control_row(JustifyContent::FlexStart)
                    })
                    .with_children(|bar| {
                        filter_chip(bar, None, icons);
                        for category in FILTER_ORDER {
                            filter_chip(bar, Some(category), icons);
                        }
                    });
                stores
                    .spawn(Node {
                        flex_grow: 1.0,
                        min_height: px(0),
                        flex_direction: FlexDirection::Row,
                        column_gap: px(12),
                        ..default()
                    })
                    .with_children(|columns| {
                        for side in [InventorySideType::Own, InventorySideType::Partner] {
                            inventory_column(columns, side);
                        }
                    });
            });
        inspector(split, icons);
    });
}

/// One filter button: the category icon and word, or `All`.
fn filter_chip(
    bar: &mut ChildSpawnerCommands,
    chip: Option<ItemCategoryType>,
    icons: &InterfaceIcons,
) {
    let label = chip.map_or("All", category_label);
    let (color, fill, border) = chip_paint(chip.is_none());
    bar.spawn((
        Name::new(format!("InventoryFilter{label}")),
        Button,
        InventoryFilterChip(chip),
        Node {
            height: px(26),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(4),
            padding: UiRect::axes(px(8), px(0)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(color, fill),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(color, border),
    ))
    .observe(on_inventory_filter_chip)
    .with_children(|chip_node| {
        if let Some(category) = chip {
            chip_node.spawn(icon_node(
                icons.category(category),
                category_color(category),
                16.0,
            ));
        }
        chip_node.spawn((themed_label(label, 12.0, UiColor::Body), Pickable::IGNORE));
    });
}

/// One side's panel: its title, the column headings and the row list.
fn inventory_column(columns: &mut ChildSpawnerCommands, side: InventorySideType) {
    columns
        .spawn((
            InventoryColumnPanel(side),
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                padding: UiRect::all(px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(UiColor::Secondary, 0.35),
        ))
        .with_children(|panel| {
            panel.spawn((
                InventoryColumnTitle(side),
                themed_label("", 15.0, UiColor::Primary),
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
            ));
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    padding: UiRect::axes(px(8), px(0)),
                    column_gap: px(8),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|headings| {
                    headings.spawn((
                        themed_label("Item", 11.0, UiColor::Label),
                        Node {
                            flex_grow: 1.0,
                            margin: UiRect::left(px(27)),
                            ..default()
                        },
                    ));
                    headings.spawn((
                        themed_label("Qty", 11.0, UiColor::Label),
                        Node {
                            width: px(QTY_PX),
                            ..default()
                        },
                        TextLayout::new(Justify::Right, LineBreak::NoWrap),
                    ));
                });
            panel.spawn((
                InventoryColumn { side, drawn: None },
                Node {
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                },
            ));
        });
}

/// The inspector: a hint with nothing selected, else the item's icon, name,
/// category, description and stock. It takes 20% of the row beside the
/// stores, so the stock line wraps at words.
fn inspector(split: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    split
        .spawn((
            Name::new("InventoryInspector"),
            Node {
                width: percent(20),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                padding: UiRect::all(px(12)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColor::Secondary, 0.08),
            BorderColor::all(Color::NONE),
            ThemedBorder::new(UiColor::Secondary),
        ))
        .with_children(|panel| {
            panel.spawn((
                InspectorPart::Hint,
                themed_label("Click an item to inspect it.", 13.0, UiColor::Body),
            ));
            panel
                .spawn((
                    InspectorPart::Item,
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                ))
                .with_children(|item| {
                    item.spawn(control_row(JustifyContent::FlexStart))
                        .with_children(|head| {
                            head.spawn((
                                Node {
                                    width: px(56),
                                    height: px(56),
                                    flex_shrink: 0.0,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(px(4)),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                ThemedFill::alpha(UiColor::Surface, 0.6),
                                BorderColor::all(Color::NONE),
                                ThemedBorder::new(UiColor::Secondary),
                            ))
                            .with_children(|frame| {
                                frame.spawn((
                                    InventoryInspectorIcon,
                                    icon_node(
                                        icons.category(ItemCategoryType::Repair),
                                        category_color(ItemCategoryType::Repair),
                                        40.0,
                                    ),
                                ));
                            });
                            head.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: px(4),
                                ..default()
                            })
                            .with_children(|names| {
                                names.spawn((
                                    InventoryInspectorField::Name,
                                    themed_label("", 16.0, UiColor::Primary),
                                ));
                                names.spawn((
                                    InventoryInspectorField::Category,
                                    themed_label("", 12.0, UiColor::Body),
                                ));
                            });
                        });
                    item.spawn((
                        InventoryInspectorField::About,
                        themed_label("", 12.0, UiColor::Body),
                    ));
                    item.spawn(control_row(JustifyContent::SpaceBetween))
                        .with_children(|fact| {
                            fact.spawn(themed_label("Stock", 12.0, UiColor::Label));
                            fact.spawn((
                                InventoryInspectorField::Stock,
                                themed_label("", 13.0, UiColor::Body),
                                TextLayout::new(Justify::Right, LineBreak::WordBoundary),
                            ));
                        });
                });
        });
}

/// One stack's row: the category icon, the item name and the count.
fn inventory_row(
    list: &mut ChildSpawnerCommands,
    side: InventorySideType,
    (item, count): (ItemType, u32),
    selected: bool,
    icons: &InterfaceIcons,
) {
    let category = item.category();
    let tone = category_color(category);
    let (fill, border) = row_alphas(selected);
    list.spawn((
        Name::new(format!("InventoryRow{side:?}{item:?}")),
        Button,
        InventoryRow { side, item },
        Node {
            height: px(ROW_PX),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            padding: UiRect::axes(px(8), px(0)),
            border: UiRect::left(px(3)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(tone, fill),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(tone, border),
    ))
    .observe(on_inventory_row)
    .with_children(|row| {
        row.spawn(icon_node(icons.category(category), tone, 18.0));
        row.spawn((
            themed_label(item_label(item), 13.0, UiColor::Body),
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                ..default()
            },
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
            Pickable::IGNORE,
        ));
        row.spawn((
            themed_label(&format!("x{count}"), 13.0, tone),
            Node {
                width: px(QTY_PX),
                ..default()
            },
            TextLayout::new(Justify::Right, LineBreak::NoWrap),
            Pickable::IGNORE,
        ));
    });
}

/// Show the clicked category, or every category for `All`, with one click if
/// the filter changes.
fn on_inventory_filter_chip(
    activate: On<Activate>,
    q_chip: Query<&InventoryFilterChip>,
    mut runtime: ResMut<InventoryRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if let Ok(chip) = q_chip.get(activate.entity) {
        if runtime.filter != chip.0 {
            runtime.filter = chip.0;
            play_menu_select(&mut commands, bank.as_deref());
        }
    }
}

/// Select the clicked row for the inspector, with one click if the selection
/// changes.
fn on_inventory_row(
    activate: On<Activate>,
    q_row: Query<&InventoryRow>,
    mut runtime: ResMut<InventoryRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if let Ok(row) = q_row.get(activate.entity) {
        let picked = Some((row.side, row.item));
        if runtime.selected != picked {
            runtime.selected = picked;
            play_menu_select(&mut commands, bank.as_deref());
        }
    }
}

/// One side as the pane draws it: the heading and its stacks, or `None` for a
/// partner that is not there.
struct SideView {
    title: String,
    stacks: Option<Vec<(ItemType, u32)>>,
}

impl SideView {
    fn count(&self, item: ItemType) -> Option<u32> {
        self.stacks
            .as_ref()?
            .iter()
            .find(|(each, _)| *each == item)
            .map(|(_, count)| *count)
    }
}

/// Fill the Inventory pane from the live inventories while its body exists.
///
/// The player ship's column always shows; the partner's shows the ship its
/// [`DockedShip`] connection names, and is hidden while the player is not
/// docked. With no player ship, or
/// more than one as the HUD treats it, the pane is left as it is. A player
/// ship without its required [`ShipInventory`], or a docked player whose
/// connection or partner inventory is missing, is a broken record, so it
/// panics rather than drawing a guess.
#[expect(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "one system reading both inventories and writing every pane part"
)]
pub(crate) fn update_inventory_panel(
    mut commands: Commands,
    mut runtime: ResMut<InventoryRuntime>,
    icons: Res<InterfaceIcons>,
    q_player: Query<
        (
            Entity,
            Option<&Name>,
            Option<&ShipInventory>,
            Option<&DockedShip>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_connection: Query<&DockingConnection>,
    q_ship: Query<(Option<&Name>, &ShipInventory), With<SpaceshipRootMarker>>,
    mut q_column: Query<(Entity, &mut InventoryColumn)>,
    mut q_panel: Query<(&InventoryColumnPanel, &mut Visibility)>,
    mut q_title: Query<(&InventoryColumnTitle, &mut Text), Without<InventoryInspectorField>>,
    mut q_row: Query<(&InventoryRow, &mut ThemedFill, &mut ThemedBorder)>,
    mut q_chip: Query<
        (&InventoryFilterChip, &mut ThemedFill, &mut ThemedBorder),
        Without<InventoryRow>,
    >,
    mut q_part: Query<(&InspectorPart, &mut Node)>,
    mut q_field: Query<(&InventoryInspectorField, &mut Text, &mut ThemedText)>,
    mut q_icon: Query<(&mut ImageNode, &mut ThemedImageTint), With<InventoryInspectorIcon>>,
) {
    if q_column.is_empty() {
        return;
    }
    let Ok((ship, name, inventory, docked)) = q_player.single() else {
        return;
    };
    let inventory = inventory.unwrap_or_else(|| {
        panic!("player ship {ship:?} has no ShipInventory, which every ship root requires")
    });
    let own = SideView {
        title: name.map_or_else(|| "Your ship".to_string(), |name| name.to_string()),
        stacks: Some(inventory.stacks().collect()),
    };
    let partner = match docked {
        Some(docked) => {
            let connection = q_connection.get(docked.connection).unwrap_or_else(|_| {
                panic!(
                    "player ship {ship:?} is docked through {:?}, which has no DockingConnection",
                    docked.connection
                )
            });
            assert!(
                connection.joins(ship),
                "player ship {ship:?} is docked through {:?}, which does not join it",
                docked.connection
            );
            let other = if connection.first_ship == ship {
                connection.second_ship
            } else {
                connection.first_ship
            };
            let (name, inventory) = q_ship.get(other).unwrap_or_else(|_| {
                panic!("docked partner {other:?} is not a ship root with a ShipInventory")
            });
            SideView {
                title: name.map_or_else(|| "Docked ship".to_string(), |name| name.to_string()),
                stacks: Some(inventory.stacks().collect()),
            }
        }
        None => SideView {
            title: "Docked ship".to_string(),
            stacks: None,
        },
    };
    let side = |which: InventorySideType| match which {
        InventorySideType::Own => &own,
        InventorySideType::Partner => &partner,
    };

    let filter = runtime.filter;
    let shown = |item: ItemType| filter.is_none_or(|category| item.category() == category);
    if let Some((which, item)) = runtime.selected {
        if side(which).count(item).is_none() || !shown(item) {
            runtime.selected = None;
        }
    }
    let selected = runtime.selected;

    for (list, mut column) in &mut q_column {
        let view = side(column.side);
        let draw = ColumnDraw {
            stacks: view.stacks.clone(),
            filter: view.stacks.as_ref().and(filter),
        };
        if column.drawn.as_ref() == Some(&draw) {
            continue;
        }
        let which = column.side;
        commands.entity(list).despawn_children();
        commands.entity(list).with_children(|rows| {
            let Some(stacks) = &draw.stacks else {
                return;
            };
            if stacks.is_empty() {
                rows.spawn(themed_label("Inventory empty.", 12.0, UiColor::Label));
                return;
            }
            let visible: Vec<(ItemType, u32)> = stacks
                .iter()
                .copied()
                .filter(|(item, _)| shown(*item))
                .collect();
            if visible.is_empty() {
                rows.spawn(themed_label(
                    "Nothing in this category.",
                    12.0,
                    UiColor::Label,
                ));
            }
            for stack in visible {
                let picked = selected == Some((which, stack.0));
                inventory_row(rows, which, stack, picked, &icons);
            }
        });
        column.drawn = Some(draw);
    }

    for (panel, mut visibility) in &mut q_panel {
        let wanted = if side(panel.0).stacks.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }

    for (title, mut text) in &mut q_title {
        let wanted = &side(title.0).title;
        if text.0 != *wanted {
            text.0.clone_from(wanted);
        }
    }

    for (row, mut fill, mut border) in &mut q_row {
        let (fill_alpha, border_alpha) = row_alphas(selected == Some((row.side, row.item)));
        if fill.alpha != fill_alpha {
            fill.alpha = fill_alpha;
        }
        if border.alpha != border_alpha {
            border.alpha = border_alpha;
        }
    }

    for (chip, mut fill, mut border) in &mut q_chip {
        let (color, fill_alpha, border_alpha) = chip_paint(chip.0 == filter);
        if fill.color != color || fill.alpha != fill_alpha {
            fill.color = color;
            fill.alpha = fill_alpha;
        }
        if border.color != color || border.alpha != border_alpha {
            border.color = color;
            border.alpha = border_alpha;
        }
    }

    for (part, mut node) in &mut q_part {
        let display = match (part, selected.is_some()) {
            (InspectorPart::Hint, false) | (InspectorPart::Item, true) => Display::Flex,
            _ => Display::None,
        };
        if node.display != display {
            node.display = display;
        }
    }
    let Some((which, item)) = selected else {
        return;
    };
    let category = item.category();
    let tone = category_color(category);
    let view = side(which);
    let count = view
        .count(item)
        .expect("the selection was cleared above unless its side carries the item");
    for (field, mut text, mut themed) in &mut q_field {
        let (value, color) = match field {
            InventoryInspectorField::Name => (item_label(item).to_string(), UiColor::Primary),
            InventoryInspectorField::Category => (category_label(category).to_string(), tone),
            InventoryInspectorField::About => (item_about(item).to_string(), UiColor::Body),
            InventoryInspectorField::Stock => {
                (format!("x{count} in {}", view.title), UiColor::Body)
            }
        };
        if text.0 != value {
            text.0 = value;
        }
        if themed.color != color {
            themed.color = color;
        }
    }
    for (mut image, mut tint) in &mut q_icon {
        let wanted = icons.category(category);
        if image.image != wanted {
            image.image = wanted;
        }
        if tint.color != tone {
            tint.color = tone;
        }
    }
}
