//! The Inventory pane body: the category filters, the player and partner
//! columns and the inspector with its transfer form; the systems that fill
//! them from live inventories and apply a confirmed transfer; and the
//! observers of the filter, row and form controls.
//!
//! Touch this module when changing what the Inventory pane shows, how a row
//! is selected or how a transfer is confirmed.

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    ui_widgets::{
        Activate, Button, Slider, SliderPrecision, SliderRange, SliderStep, SliderValue,
        TrackClick, ValueChange,
    },
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::{DockedShip, DockingConnection};
use nova_ui::{
    theme::UiColor,
    widget::{
        button, slider_track, text_field, ButtonSpec, TextFieldError, TextFieldFocused,
        TextFieldSpec, TextFieldValue, ThemedBorder, ThemedFill, ThemedImageTint, ThemedText,
    },
};

use super::{InventoryDraft, InventoryRuntime, InventorySideType, InventoryTransferCommand};
use crate::{
    icons::{icon_node, InterfaceIcons},
    pane::{play_menu_select, themed_label},
};

/// Height of one inventory row, in logical px.
const ROW_PX: f32 = 28.0;
/// Width of the quantity column, in logical px.
const QTY_PX: f32 = 64.0;
/// Seconds a transfer result stays on the note line, as on the Ship pane.
const NOTE_SECONDS: f32 = 2.5;

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
    /// The transfer form, while a draft is open.
    Form,
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
    /// What moving items takes where the player ship is now.
    Context,
    /// The last transfer result.
    Note,
    /// Which way the open draft moves items, and with which ship.
    DraftTitle,
    /// How many the draft's source ship carries.
    DraftStock,
    /// What the target ship carries after the draft, or why it cannot move.
    DraftSummary,
}

/// The draft's quantity row: the wheel over it steps the quantity by one.
#[derive(Component)]
pub(crate) struct InventoryDraftWheel;

/// The draft's quantity slider, from zero to the source stock.
#[derive(Component)]
pub(crate) struct InventoryDraftSlider;

/// The draft's typed quantity.
#[derive(Component)]
pub(crate) struct InventoryDraftField;

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
                    item.spawn((
                        InspectorPart::Form,
                        Node {
                            display: Display::None,
                            flex_direction: FlexDirection::Column,
                            row_gap: px(10),
                            ..default()
                        },
                    ))
                    .with_children(draft_form);
                });
            panel.spawn((
                InventoryInspectorField::Context,
                themed_label("", 12.0, UiColor::Label),
            ));
            panel.spawn((
                InventoryInspectorField::Note,
                themed_label("", 12.0, UiColor::Accent),
            ));
        });
}

/// The transfer form: the draft's direction; the typed quantity, the source
/// stock and All on a row the wheel steps; the quantity slider; the target's
/// count after the move; and Confirm.
fn draft_form(form: &mut ChildSpawnerCommands) {
    form.spawn((
        InventoryInspectorField::DraftTitle,
        themed_label("", 13.0, UiColor::Accent),
    ));
    form.spawn((InventoryDraftWheel, control_row(JustifyContent::FlexStart)))
        .observe(wheel_inventory_draft)
        .with_children(|row| {
            row.spawn(Node {
                width: px(72),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|cell| {
                cell.spawn((
                    InventoryDraftField,
                    text_field(TextFieldSpec::new("1").max_chars(5).dense()),
                ));
            });
            row.spawn((
                InventoryInspectorField::DraftStock,
                themed_label("", 15.0, UiColor::Primary),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
            ));
            row.spawn((
                Name::new("InventoryDraftAll"),
                compact_button(ButtonSpec::new("All").fit().ghost()),
            ))
            .observe(fill_inventory_draft);
        });
    form.spawn((
        InventoryDraftSlider,
        Slider {
            track_click: TrackClick::Snap,
            ..default()
        },
        SliderValue(1.0),
        SliderRange::new(0.0, 1.0),
        SliderStep(1.0),
        SliderPrecision(0),
        slider_track(0.0),
    ))
    .observe(slide_inventory_draft);
    form.spawn((
        InventoryInspectorField::DraftSummary,
        themed_label("", 13.0, UiColor::Body),
    ));
    form.spawn(control_row(JustifyContent::FlexStart))
        .with_children(|row| {
            row.spawn((
                Name::new("InventoryDraftConfirm"),
                compact_button(ButtonSpec::new("Confirm").fit().primary()),
            ))
            .observe(confirm_inventory_draft);
        });
}

/// A button sized for the narrow inspector.
fn compact_button(spec: ButtonSpec) -> impl Bundle {
    button(ButtonSpec {
        min_height: 22.0,
        font_size: 12.0,
        ..spec
    })
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

/// Select the clicked row for the inspector and open the transfer it offers
/// at one unit, with one click if either changes. The selected row again
/// keeps its open draft and its quantity.
fn on_inventory_row(
    activate: On<Activate>,
    q_row: Query<&InventoryRow>,
    ships: InventoryShips,
    mut runtime: ResMut<InventoryRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok(row) = q_row.get(activate.entity) else {
        return;
    };
    let picked = Some((row.side, row.item));
    let offer = ships
        .pair()
        .and_then(|pair| offered_transfer(row.side, ships.partner_lootable(pair)));
    let open = runtime.draft.map(|draft| (draft.transfer, draft.item));
    if runtime.selected == picked && open == offer.map(|transfer| (transfer, row.item)) {
        return;
    }
    runtime.selected = picked;
    runtime.draft = offer.map(|transfer| InventoryDraft {
        transfer,
        item: row.item,
        quantity: Some(1),
    });
    play_menu_select(&mut commands, bank.as_deref());
}

/// The transfer a row offers: Give from the player's row while docked, Take
/// from the partner's row while the partner is neutralized or lootable.
/// `partner_lootable` is `None` while undocked.
fn offered_transfer(
    side: InventorySideType,
    partner_lootable: Option<bool>,
) -> Option<ItemTransferType> {
    match (side, partner_lootable) {
        (InventorySideType::Own, Some(_)) => Some(ItemTransferType::Give),
        (InventorySideType::Partner, Some(true)) => Some(ItemTransferType::Take),
        _ => None,
    }
}

/// The player ship and the ship it is docked with, as the Inventory pane reads
/// and moves them. One resolver for the panel, the row click, the form and
/// the transfer handler, so all four fail the same way on a broken record.
#[derive(SystemParam)]
pub(crate) struct InventoryShips<'w, 's> {
    players: Query<'w, 's, (Entity, Option<&'static DockedShip>), With<PlayerSpaceshipMarker>>,
    connections: Query<'w, 's, &'static DockingConnection>,
    ships: Query<
        'w,
        's,
        (
            Option<&'static Name>,
            &'static mut ShipInventory,
            Has<NeutralizedMarker>,
            Has<LootableShipMarker>,
        ),
        With<SpaceshipRootMarker>,
    >,
}

/// The player ship and its docked partner, both checked to carry a
/// [`ShipInventory`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct InventoryPair {
    own: Entity,
    partner: Option<Entity>,
}

impl InventoryShips<'_, '_> {
    /// The player ship and its docked partner; `None` with no player ship, or
    /// more than one as the HUD treats it.
    ///
    /// # Panics
    ///
    /// On a player ship without its required [`ShipInventory`], or a docked
    /// player whose connection or partner inventory is missing: a broken
    /// record, not a state to draw a guess for.
    fn pair(&self) -> Option<InventoryPair> {
        let (own, docked) = self.players.single().ok()?;
        assert!(
            self.ships.contains(own),
            "player ship {own:?} has no ShipInventory, which every ship root requires"
        );
        let partner = docked.map(|docked| {
            let connection = self.connections.get(docked.connection).unwrap_or_else(|_| {
                panic!(
                    "player ship {own:?} is docked through {:?}, which has no DockingConnection",
                    docked.connection
                )
            });
            assert!(
                connection.joins(own),
                "player ship {own:?} is docked through {:?}, which does not join it",
                docked.connection
            );
            let other = if connection.first_ship == own {
                connection.second_ship
            } else {
                connection.first_ship
            };
            assert!(
                self.ships.contains(other),
                "docked partner {other:?} is not a ship root with a ShipInventory"
            );
            other
        });
        Some(InventoryPair { own, partner })
    }

    /// A ship's heading: its name, or what it is to the player.
    fn title(&self, ship: Entity, side: InventorySideType) -> String {
        match (self.ship(ship).0, side) {
            (Some(name), _) => name.to_string(),
            (None, InventorySideType::Own) => "Your ship".to_string(),
            (None, InventorySideType::Partner) => "Docked ship".to_string(),
        }
    }

    /// Whether the docked partner may be taken from; `None` while undocked.
    fn partner_lootable(&self, pair: InventoryPair) -> Option<bool> {
        pair.partner.map(|partner| self.ship(partner).2)
    }

    /// A resolved ship's name, inventory, and whether a docked ship may Take
    /// from it: it is neutralized or carries [`LootableShipMarker`].
    fn ship(&self, ship: Entity) -> (Option<&Name>, &ShipInventory, bool) {
        let (name, inventory, neutralized, lootable) = self
            .ships
            .get(ship)
            .expect("InventoryShips::pair checked every ship it names");
        (name, inventory, neutralized || lootable)
    }

    /// What the source ship of `draft` carries of its item, and never less than
    /// one, so the wheel's range is never empty.
    fn draft_stock(&self, draft: InventoryDraft) -> u32 {
        let source = self.pair().and_then(|pair| match draft.transfer {
            ItemTransferType::Take => pair.partner,
            ItemTransferType::Give => Some(pair.own),
        });
        source
            .map_or(0, |ship| self.ship(ship).1.count(draft.item))
            .max(1)
    }
}

/// Play one interface cue, if the sound bank has loaded.
fn play_cue(commands: &mut Commands, bank: Option<&SoundBank<UiSfx>>, cue: UiSfx, volume: f32) {
    if let Some(bank) = bank {
        commands.play_sfx(bank.get(cue), AudioRoute::Interface, volume);
    }
}

/// Set the open draft's quantity, or do nothing if it already holds it. A new
/// number ticks; text that is not a number is silent until Confirm refuses
/// it. Every quantity control but All goes through here, so one change makes
/// at most one tick.
fn set_draft_quantity(
    runtime: &mut InventoryRuntime,
    quantity: Option<u32>,
    commands: &mut Commands,
    bank: Option<&SoundBank<UiSfx>>,
) {
    let Some(draft) = runtime.draft else {
        return;
    };
    if draft.quantity == quantity {
        return;
    }
    runtime.draft = Some(InventoryDraft { quantity, ..draft });
    if quantity.is_some() {
        play_cue(commands, bank, UiSfx::UiTick, UI_TICK_VOLUME);
    }
}

/// Step the open draft's quantity by one per wheel event over the quantity
/// row, between one and what the source ship carries.
fn wheel_inventory_draft(
    scroll: On<Pointer<Scroll>>,
    ships: InventoryShips,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut runtime: ResMut<InventoryRuntime>,
) {
    let Some(draft) = runtime.draft else {
        return;
    };
    if scroll.y == 0.0 {
        return;
    }
    let step = if scroll.y > 0.0 { 1 } else { -1 };
    let have = i64::from(ships.draft_stock(draft));
    let quantity = (i64::from(draft.quantity.unwrap_or(0)) + step).clamp(1, have) as u32;
    set_draft_quantity(&mut runtime, Some(quantity), &mut commands, bank.as_deref());
}

/// Take the slider's value as the open draft's quantity. An empty track is
/// zero, which Confirm refuses. A drag reports every frame it moves; only a
/// new whole quantity ticks.
fn slide_inventory_draft(
    change: On<ValueChange<f32>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut runtime: ResMut<InventoryRuntime>,
) {
    let quantity = change.value.round().max(0.0) as u32;
    set_draft_quantity(&mut runtime, Some(quantity), &mut commands, bank.as_deref());
}

/// Set the open draft to everything the source ship carries, with a click.
fn fill_inventory_draft(
    _: On<Activate>,
    ships: InventoryShips,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut runtime: ResMut<InventoryRuntime>,
) {
    let Some(draft) = runtime.draft else {
        return;
    };
    runtime.draft = Some(InventoryDraft {
        quantity: Some(ships.draft_stock(draft)),
        ..draft
    });
    play_menu_select(&mut commands, bank.as_deref());
}

/// Send the open draft to [`apply_inventory_transfer_commands`]. With none
/// open, as on the second click of a double click, nothing happens.
fn confirm_inventory_draft(
    _: On<Activate>,
    runtime: Res<InventoryRuntime>,
    mut transfers: MessageWriter<InventoryTransferCommand>,
) {
    if let Some(draft) = runtime.draft {
        transfers.write(InventoryTransferCommand {
            transfer: draft.transfer,
            item: draft.item,
            quantity: draft.quantity,
        });
    }
}

/// Take the typed text as the open draft's quantity: a whole number, or
/// `None`, which Confirm refuses.
pub(crate) fn type_inventory_draft(
    fields: Query<&TextFieldValue, (With<InventoryDraftField>, Changed<TextFieldValue>)>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut runtime: ResMut<InventoryRuntime>,
) {
    for value in &fields {
        let typed = value.trim().parse::<u32>().ok();
        set_draft_quantity(&mut runtime, typed, &mut commands, bank.as_deref());
    }
}

/// Show the open draft's quantity on the slider and the field, writing only a
/// difference, so the nodes stay and a drag does not fight itself.
///
/// The slider runs from zero, an empty track, to the source stock, and hides
/// while the source carries one or none, where it could not move; the field
/// and All still set the quantity. While the field holds text that is not a
/// number, the slider keeps the last number and the field is marked as an
/// error. The field is rewritten only when the draft holds a number its text
/// does not read as. A rewrite of the focused field puts the caret at the end.
/// With no draft open the field lets go of the keyboard, so a hidden field
/// never holds it.
#[expect(
    clippy::type_complexity,
    reason = "the form's slider and field with their focus and error state"
)]
pub(crate) fn sync_inventory_draft_controls(
    mut commands: Commands,
    ships: InventoryShips,
    runtime: Res<InventoryRuntime>,
    mut sliders: Query<(Entity, &SliderValue, &SliderRange, &mut Node), With<InventoryDraftSlider>>,
    mut fields: Query<
        (
            Entity,
            &mut TextFieldValue,
            Has<TextFieldFocused>,
            Has<TextFieldError>,
        ),
        With<InventoryDraftField>,
    >,
) {
    let Some(draft) = runtime.draft else {
        for (field, _, focused, _) in &fields {
            if focused {
                commands.entity(field).remove::<TextFieldFocused>();
            }
        }
        return;
    };
    let have = ships.draft_stock(draft);
    for (slider, value, range, mut node) in &mut sliders {
        let display = if have > 1 {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let wanted = SliderRange::new(0.0, have as f32);
        if *range != wanted {
            commands.entity(slider).insert(wanted);
        }
        if let Some(quantity) = draft.quantity {
            let shown = quantity.min(have) as f32;
            if value.0 != shown {
                commands.entity(slider).insert(SliderValue(shown));
            }
        }
    }
    for (field, mut value, focused, marked) in &mut fields {
        let Some(quantity) = draft.quantity else {
            if !marked {
                commands.entity(field).insert(TextFieldError(String::new()));
            }
            continue;
        };
        if marked {
            commands.entity(field).remove::<TextFieldError>();
        }
        if value.trim().parse::<u32>().ok() == Some(quantity) {
            continue;
        }
        value.0 = quantity.to_string();
        if focused {
            commands
                .entity(field)
                .insert(TextFieldFocused::at_end(&value.0));
        }
    }
}

/// Apply each confirmed [`InventoryTransferCommand`] between the player ship
/// and its docked partner by the [`plan_item_transfer`] rule, and flash the
/// result on the note line.
///
/// A move removes from the source and adds to the target in this one run,
/// closes the draft and clicks. A refusal changes no inventory, keeps the
/// draft open so the quantity can change, and buzzes. Each command reads the
/// state the previous one left. With no player ship, or more than one, the
/// commands are dropped, as the panel draws nothing then.
pub(crate) fn apply_inventory_transfer_commands(
    mut transfers: MessageReader<InventoryTransferCommand>,
    mut ships: InventoryShips,
    mut runtime: ResMut<InventoryRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Some(pair) = ships.pair() else {
        transfers.clear();
        return;
    };
    for command in transfers.read() {
        let (note, cue, volume) = match transfer_items(&mut ships, pair, *command) {
            Ok(note) => {
                runtime.draft = None;
                (note, UiSfx::MenuSelect, MENU_SELECT_VOLUME)
            }
            Err(note) => (note, UiSfx::EditorDeny, EDITOR_DENY_VOLUME),
        };
        runtime.note = Some((note, NOTE_SECONDS));
        play_cue(&mut commands, bank.as_deref(), cue, volume);
    }
}

/// Move one command's items, or say why not. Both texts are the note line.
fn transfer_items(
    ships: &mut InventoryShips,
    pair: InventoryPair,
    command: InventoryTransferCommand,
) -> Result<String, String> {
    let Some(partner) = pair.partner else {
        return Err("Refused: not docked".to_string());
    };
    let InventoryTransferCommand {
        transfer,
        item,
        quantity,
    } = command;
    let label = item_label(item);
    let own_title = ships.title(pair.own, InventorySideType::Own);
    let partner_title = ships.title(partner, InventorySideType::Partner);
    let (source, target, source_title, target_title) = match transfer {
        ItemTransferType::Take => (partner, pair.own, &partner_title, &own_title),
        ItemTransferType::Give => (pair.own, partner, &own_title, &partner_title),
    };
    let (_, own, _) = ships.ship(pair.own);
    let (_, theirs, lootable) = ships.ship(partner);
    let moved =
        plan_item_transfer(transfer, lootable, item, quantity, own, theirs).map_err(|refusal| {
            match refusal {
                ItemTransferRefusalType::NotLootable => {
                    format!("Refused: {partner_title} is not neutralized or lootable")
                }
                ItemTransferRefusalType::NoQuantity => "Refused: enter a quantity".to_string(),
                ItemTransferRefusalType::ZeroQuantity => "Refused: quantity is zero".to_string(),
                ItemTransferRefusalType::Short { held } => {
                    format!("Refused: only {held} {label} in {source_title}")
                }
                ItemTransferRefusalType::Overflow { .. } => {
                    format!("Refused: {target_title} cannot hold more {label}")
                }
            }
        })?;
    let [(_, mut from, ..), (_, mut to, ..)] = ships
        .ships
        .get_many_mut([source, target])
        .expect("InventoryShips::pair checked both ships, and a ship does not dock with itself");
    from.remove(item, moved);
    to.add(item, moved);
    Ok(match transfer {
        ItemTransferType::Take => format!("Took {moved} {label} from {partner_title}"),
        ItemTransferType::Give => format!("Gave {moved} {label} to {partner_title}"),
    })
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
/// docked. With no player ship, or more than one as the HUD treats it, the
/// pane is left as it is. A broken ship record panics through
/// [`InventoryShips`]. A draft its selected row no longer offers, after an
/// undock or a filter, is closed. The note line counts down in real time,
/// since the interface pauses virtual time.
#[expect(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "one system reading both inventories and writing every pane part"
)]
pub(crate) fn update_inventory_panel(
    mut commands: Commands,
    mut runtime: ResMut<InventoryRuntime>,
    icons: Res<InterfaceIcons>,
    time: Res<Time<Real>>,
    ships: InventoryShips,
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
    let Some(pair) = ships.pair() else {
        return;
    };
    if let Some((_, remaining)) = runtime.note.as_mut() {
        *remaining -= time.delta_secs();
        if *remaining <= 0.0 {
            runtime.note = None;
        }
    }
    let own = SideView {
        title: ships.title(pair.own, InventorySideType::Own),
        stacks: Some(ships.ship(pair.own).1.stacks().collect()),
    };
    let partner = match pair.partner {
        Some(other) => SideView {
            title: ships.title(other, InventorySideType::Partner),
            stacks: Some(ships.ship(other).1.stacks().collect()),
        },
        None => SideView {
            title: "Docked ship".to_string(),
            stacks: None,
        },
    };
    let partner_lootable = ships.partner_lootable(pair);
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
    if let Some(draft) = runtime.draft {
        let offer = selected.and_then(|(which, item)| {
            offered_transfer(which, partner_lootable).map(|transfer| (transfer, item))
        });
        if offer != Some((draft.transfer, draft.item)) {
            runtime.draft = None;
        }
    }
    let draft = runtime.draft;

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
        let shown = match part {
            InspectorPart::Hint => selected.is_none(),
            InspectorPart::Item => selected.is_some(),
            InspectorPart::Form => draft.is_some(),
        };
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }

    let context = match partner_lootable {
        None => "Dock with a ship to move items.",
        Some(true) => "Take from the docked ship or give from yours.",
        Some(false) => "Give from your ship. Take needs a neutralized or lootable ship.",
    };
    let note = runtime
        .note
        .as_ref()
        .map_or(String::new(), |(note, _)| note.clone());
    let picked = selected.map(|(which, item)| {
        let view = side(which);
        let count = view
            .count(item)
            .expect("the selection was cleared above unless its side carries the item");
        (item, format!("x{count} in {}", view.title))
    });
    let form = draft.map(|draft| {
        let (source, target) = match draft.transfer {
            ItemTransferType::Take => (&partner, &own),
            ItemTransferType::Give => (&own, &partner),
        };
        let title = match draft.transfer {
            ItemTransferType::Take => format!("Take from {}", partner.title),
            ItemTransferType::Give => format!("Give to {}", partner.title),
        };
        let have = source.count(draft.item).unwrap_or(0);
        let summary = match draft.quantity {
            Some(quantity) => (
                format!(
                    "{} after: x{}",
                    target.title,
                    target
                        .count(draft.item)
                        .unwrap_or(0)
                        .saturating_add(quantity)
                ),
                UiColor::Body,
            ),
            None => ("Type a whole number".to_string(), UiColor::Danger),
        };
        (title, format!("of {have}"), summary)
    });
    for (field, mut text, mut themed) in &mut q_field {
        let (value, color) = match (field, picked.as_ref(), form.as_ref()) {
            (InventoryInspectorField::Context, ..) => (context.to_string(), UiColor::Label),
            (InventoryInspectorField::Note, ..) => (note.clone(), UiColor::Accent),
            (InventoryInspectorField::Name, Some((item, _)), _) => {
                (item_label(*item).to_string(), UiColor::Primary)
            }
            (InventoryInspectorField::Category, Some((item, _)), _) => (
                category_label(item.category()).to_string(),
                category_color(item.category()),
            ),
            (InventoryInspectorField::About, Some((item, _)), _) => {
                (item_about(*item).to_string(), UiColor::Body)
            }
            (InventoryInspectorField::Stock, Some((_, stock)), _) => (stock.clone(), UiColor::Body),
            (InventoryInspectorField::DraftTitle, _, Some((title, ..))) => {
                (title.clone(), UiColor::Accent)
            }
            (InventoryInspectorField::DraftStock, _, Some((_, have, _))) => {
                (have.clone(), UiColor::Primary)
            }
            (InventoryInspectorField::DraftSummary, _, Some((.., summary))) => summary.clone(),
            _ => continue,
        };
        if text.0 != value {
            text.0 = value;
        }
        if themed.color != color {
            themed.color = color;
        }
    }
    let Some((item, _)) = picked else {
        return;
    };
    let tone = category_color(item.category());
    for (mut image, mut tint) in &mut q_icon {
        let wanted = icons.category(item.category());
        if image.image != wanted {
            image.image = wanted;
        }
        if tint.color != tone {
            tint.color = tone;
        }
    }
}
