//! The Market page: the INVENTORY pane as the game draws it, with prices,
//! on the shared layout kit. The category chip bar over the Tender, Trader
//! and Storage columns, the MARKET strip under them, and the inspector with
//! the kit's amount row and the side's verbs: Buy all and Buy on a station
//! row, Sell all and Sell on a hold row, Store and Take on a storage row. A
//! trade or a move shifts the mock stocks and credits.

use bevy::prelude::*;
use nova_ui::{
    prelude::{
        amount_row, button, chip, chip_bar, metrics, spawn_block, spawn_kv, tag, text, verb_row,
        Amount, AmountChanged, AmountSpec, ButtonSpec, RowAlign, Selected, Short, TextRole,
        VerbGate,
    },
    theme::UiColorType,
};

use crate::{
    chrome::{
        acts, bare_meter, cell, column, column_panel, grow_cell, interface_card, row_node,
        side_panel_wide, spawn_row, table_head, view_split,
    },
    data::{delta_percent, thousands, Category, Ware, OWN_HOLDS, STORAGE_CAP_KG, WARES},
    icons::{icon, GalleryIcons},
    Click, MarketDraft, MarketState, TradeSide, FILTER_PREFIX, MARKET_BUY, MARKET_BUY_ALL,
    MARKET_REFUSED, MARKET_SELL, MARKET_SELL_ALL, MARKET_STORE, MARKET_TAKE, TRADE_ROW_PREFIX,
};

const TABS: [&str; 5] = ["Map", "Ship", "Inventory", "Jobs", "Station"];
/// Hold capacity of the docked cargoa, in kg.
const HOLD_KG: u32 = 30_000;

/// The inspector's amount row.
#[derive(Component)]
pub struct MarketDraftAmount;
/// The inspector's price value, painted Danger while the balance is short.
#[derive(Component)]
pub struct MarketDraftPrice;
/// The inspector's Buy, gated on the price.
#[derive(Component)]
pub struct MarketBuyVerb;

impl MarketState {
    pub fn hold_kg(&self) -> u32 {
        WARES.iter().zip(&self.ship).map(|(w, q)| w.kg * q).sum()
    }

    pub fn stored_kg(&self) -> u32 {
        WARES.iter().zip(&self.storage).map(|(w, q)| w.kg * q).sum()
    }

    /// The most the open draft may ask for: the stock on the selected side.
    /// A storage row serves Store and Take both, so it takes the larger of
    /// the hold and the storage.
    pub fn draft_max(&self) -> u32 {
        match self.selected {
            Some((TradeSide::Tender, i)) => self.ship[i],
            Some((TradeSide::Trader, i)) => self.station[i],
            Some((TradeSide::Storage, i)) => self.ship[i].max(self.storage[i]),
            None => 0,
        }
    }
}

fn kg_text(kg: u32) -> String {
    format!("{:.1} t", kg as f32 / 1000.0)
}

pub fn market_page(body: &mut ChildSpawnerCommands, icons: &GalleryIcons, market: &MarketState) {
    interface_card(body, "Inventory", &TABS, "Inventory", None, |body| {
        body.spawn(view_split()).with_children(|split| {
            split.spawn(column()).with_children(|stores| {
                stores.spawn(chip_bar()).with_children(|bar| {
                    filter_chip(bar, icons, None, market);
                    for category in Category::ALL {
                        filter_chip(bar, icons, Some(category), market);
                    }
                });
                stores
                    .spawn(Node {
                        flex_grow: 1.0,
                        min_height: px(0),
                        flex_direction: FlexDirection::Row,
                        column_gap: px(metrics::GAP_L),
                        ..default()
                    })
                    .with_children(|columns| {
                        trade_column(columns, icons, TradeSide::Tender, market);
                        trade_column(columns, icons, TradeSide::Trader, market);
                        storage_column(columns, icons, market);
                    });
                market_strip(stores, market);
            });
            split
                .spawn(side_panel_wide(26.0, Overflow::scroll_y()))
                .with_children(|side| inspector(side, icons, market));
        });
    });
}

/// One chip of the category bar, `All` for `None`, lit while current.
fn filter_chip(
    bar: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    category: Option<Category>,
    market: &MarketState,
) {
    let label = category.map_or("All", Category::label);
    let mut chip_entity = bar.spawn((
        Name::new(format!("{FILTER_PREFIX} {label}")),
        acts(
            chip(label, category.map(|category| icons.get(category.icon()))),
            Click::Filter(category),
        ),
    ));
    if market.filter == category {
        chip_entity.insert(Selected);
    }
}

fn listed(market: &MarketState, side: TradeSide, i: usize, w: &Ware) -> bool {
    let qty = match side {
        TradeSide::Tender => market.ship[i],
        TradeSide::Trader => market.station[i],
        TradeSide::Storage => market.storage[i],
    };
    qty > 0 && market.filter.is_none_or(|c| c == w.category)
}

/// A column's fact: the label over its value.
fn fact(row: &mut ChildSpawnerCommands, label: &str, value: String) {
    row.spawn(Node {
        flex_direction: FlexDirection::Column,
        ..default()
    })
    .with_children(|fact| {
        fact.spawn(text(label, TextRole::Label));
        fact.spawn(text(value, TextRole::Value));
    });
}

/// A column's head: its title, then its facts on one row.
fn column_head(
    panel: &mut ChildSpawnerCommands,
    title: &str,
    facts: impl FnOnce(&mut ChildSpawnerCommands),
) {
    panel.spawn(text(title, TextRole::Title));
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(metrics::GAP_L),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(facts);
}

fn trade_column(
    columns: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    side: TradeSide,
    market: &MarketState,
) {
    columns.spawn(column_panel()).with_children(|panel| {
        let title = match side {
            TradeSide::Tender => "Tender: cargoa",
            TradeSide::Trader => "Trader: Dust Gate",
            TradeSide::Storage => unreachable!("the storage column has its own panel"),
        };
        column_head(panel, title, |facts| match side {
            TradeSide::Tender => {
                fact(
                    facts,
                    "Cargo",
                    format!("{} / {}", kg_text(market.hold_kg()), kg_text(HOLD_KG)),
                );
                fact(
                    facts,
                    "Credits",
                    format!("{} cr", thousands(market.credits)),
                );
            }
            TradeSide::Trader => {
                let stock: u32 = WARES
                    .iter()
                    .zip(&market.station)
                    .map(|(w, q)| w.kg * q)
                    .sum();
                fact(facts, "Stock", kg_text(stock));
                fact(facts, "Restock", "12 min".to_string());
            }
            TradeSide::Storage => {}
        });
        table_head(
            panel,
            &[
                (18.0, ""),
                (0.0, "Item"),
                (40.0, "Qty"),
                (48.0, "Price"),
                (44.0, "Deal"),
            ],
        );
        for (i, w) in WARES.iter().enumerate() {
            if !listed(market, side, i, w) {
                continue;
            }
            let (qty, price) = match side {
                TradeSide::Tender => (market.ship[i], w.bid),
                TradeSide::Trader | TradeSide::Storage => (market.station[i], w.ask),
            };
            let delta = delta_percent(price, w.reference);
            spawn_row(
                panel,
                Click::Trade(side, i),
                format!("{TRADE_ROW_PREFIX} {side:?} {i}"),
                market.selected == Some((side, i)),
            )
            .insert(Node {
                height: px(metrics::LIST_ROW_H),
                padding: UiRect::horizontal(px(metrics::PAD_X)),
                border: UiRect::all(px(1)),
                column_gap: px(metrics::GAP),
                ..row_node()
            })
            .with_children(|r| {
                r.spawn(icon(icons, w.category.icon(), 16.0));
                r.spawn(grow_cell(w.name, metrics::TEXT, UiColorType::Text));
                r.spawn(cell(
                    40.0,
                    &format!("x{qty}"),
                    metrics::TEXT,
                    UiColorType::TextBody,
                    true,
                ));
                r.spawn(cell(
                    48.0,
                    &format!("{} cr", thousands(price)),
                    metrics::TEXT,
                    UiColorType::Accent,
                    true,
                ));
                r.spawn(Node {
                    width: px(44.0),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                })
                .with_children(|c| {
                    c.spawn(deal_badge(side, delta));
                });
            });
        }
    });
}

/// The Storage column: the station's storage with its stored mass against
/// the cap, then one headed section per own ship hold, as the game's
/// `InventorySideType::Storage` column. Only the station's rows select;
/// an own hold is listed to read.
fn storage_column(columns: &mut ChildSpawnerCommands, icons: &GalleryIcons, market: &MarketState) {
    columns.spawn(column_panel()).with_children(|panel| {
        column_head(panel, "Storage: Dust Gate", |facts| {
            fact(
                facts,
                "Stored",
                format!(
                    "{} / {}",
                    kg_text(market.stored_kg()),
                    kg_text(STORAGE_CAP_KG)
                ),
            );
        });
        table_head(
            panel,
            &[(18.0, ""), (0.0, "Item"), (48.0, "Qty"), (58.0, "Mass")],
        );
        for (i, w) in WARES.iter().enumerate() {
            if !listed(market, TradeSide::Storage, i, w) {
                continue;
            }
            let qty = market.storage[i];
            spawn_row(
                panel,
                Click::Trade(TradeSide::Storage, i),
                format!("{TRADE_ROW_PREFIX} Storage {i}"),
                market.selected == Some((TradeSide::Storage, i)),
            )
            .insert(Node {
                height: px(metrics::LIST_ROW_H),
                padding: UiRect::horizontal(px(metrics::PAD_X)),
                border: UiRect::all(px(1)),
                column_gap: px(metrics::GAP),
                ..row_node()
            })
            .with_children(|r| stack_cells(r, icons, w, qty));
        }
        for hold in &OWN_HOLDS {
            let load: u32 = hold.stacks.iter().map(|(i, q)| WARES[*i].kg * q).sum();
            panel
                .spawn(Node {
                    margin: UiRect::top(px(metrics::GAP)),
                    padding: UiRect::horizontal(px(metrics::PAD_X)),
                    min_height: px(metrics::ROW_H),
                    justify_content: JustifyContent::SpaceBetween,
                    ..row_node()
                })
                .with_children(|head| {
                    head.spawn(text(hold.ship, TextRole::Value));
                    head.spawn(text(
                        format!("{} / {}", kg_text(load), kg_text(hold.cap_kg)),
                        TextRole::Label,
                    ));
                });
            for (i, qty) in hold.stacks {
                let w = &WARES[*i];
                if !market.filter.is_none_or(|c| c == w.category) {
                    continue;
                }
                panel
                    .spawn(Node {
                        height: px(metrics::LIST_ROW_H),
                        padding: UiRect::horizontal(px(metrics::PAD_X)),
                        column_gap: px(metrics::GAP),
                        ..row_node()
                    })
                    .with_children(|r| stack_cells(r, icons, w, *qty));
            }
        }
    });
}

/// The cells of a storage or own-hold row: the icon, the name, the count
/// and the mass.
fn stack_cells(r: &mut ChildSpawnerCommands, icons: &GalleryIcons, w: &Ware, qty: u32) {
    r.spawn(icon(icons, w.category.icon(), 16.0));
    r.spawn(grow_cell(w.name, metrics::TEXT, UiColorType::Text));
    r.spawn(cell(
        48.0,
        &format!("x{qty}"),
        metrics::TEXT,
        UiColorType::TextBody,
        true,
    ));
    r.spawn(cell(
        58.0,
        &kg_text(qty * w.kg),
        metrics::TEXT,
        UiColorType::TextBody,
        true,
    ));
}

/// The `Deal` tag: the percent off the reference, green when it favours
/// the player (a Tender bid over the reference, a Trader ask under it),
/// amber when not, secondary at it.
fn deal_badge(side: TradeSide, delta: i32) -> impl Bundle {
    let gain = match side {
        TradeSide::Tender => delta.signum(),
        TradeSide::Trader => -delta.signum(),
        TradeSide::Storage => unreachable!("the storage column shows no price"),
    };
    let kind = match gain {
        1 => UiColorType::Nominal,
        -1 => UiColorType::Accent,
        _ => UiColorType::TextSecondary,
    };
    tag(&format!("{delta:+}%"), kind)
}

/// The MARKET strip: the best buyer and the cheapest seller, the restock
/// meter, and the refusal tag.
fn market_strip(stores: &mut ChildSpawnerCommands, market: &MarketState) {
    let pays_most = WARES
        .iter()
        .max_by_key(|w| delta_percent(w.bid, w.reference))
        .expect("the ware table is not empty");
    let cheapest = WARES
        .iter()
        .enumerate()
        .filter(|(i, _)| market.station[*i] > 0)
        .map(|(_, w)| w)
        .min_by_key(|w| delta_percent(w.ask, w.reference))
        .unwrap_or(&WARES[0]);
    stores
        .spawn((
            Node {
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                column_gap: px(metrics::GAP),
                row_gap: px(metrics::GAP_S),
                padding: UiRect::all(px(metrics::GAP)),
                border: UiRect::all(px(1)),
                ..default()
            },
            nova_ui::prelude::panel_paint(),
        ))
        .with_children(|row| {
            row.spawn(text("Market", TextRole::Heading));
            row.spawn(text("Buys", TextRole::Label));
            row.spawn(tag(
                &format!(
                    "{} {:+}%",
                    pays_most.name,
                    delta_percent(pays_most.bid, pays_most.reference)
                ),
                UiColorType::Accent,
            ));
            row.spawn(text("Sells", TextRole::Label));
            row.spawn(tag(
                &format!(
                    "{} {:+}%",
                    cheapest.name,
                    delta_percent(cheapest.ask, cheapest.reference)
                ),
                UiColorType::Nominal,
            ));
            row.spawn(text("Restock", TextRole::Label));
            let ore = market.station[0];
            row.spawn(bare_meter(140.0, ore as f32 / 40.0));
            row.spawn(text(format!("{ore} / 40 t"), TextRole::Value));
            row.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            row.spawn((
                Name::new(MARKET_REFUSED),
                tag(
                    &format!("REFUSED {} min", market.refused_min),
                    UiColorType::Danger,
                ),
            ));
        });
}

fn inspector(side: &mut ChildSpawnerCommands, icons: &GalleryIcons, market: &MarketState) {
    // Nothing selected: the inspector stays empty.
    let Some((trade_side, i)) = market.selected else {
        return;
    };
    let w = &WARES[i];
    side.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(metrics::GAP),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|head| {
        head.spawn(icon(icons, w.category.icon(), 24.0));
        head.spawn(text(w.name, TextRole::Title));
        head.spawn(tag(w.category.label(), UiColorType::TextSecondary));
    });
    if trade_side == TradeSide::Storage {
        storage_inspector(side, market, i, w);
        return;
    }
    spawn_block(side, "Prices", ()).with_children(|block| {
        spawn_kv(
            block,
            "Reference",
            text(format!("{} cr", thousands(w.reference)), TextRole::Value),
        );
        spawn_kv(
            block,
            "Pays",
            text(
                format!(
                    "{} cr  {:+}%",
                    thousands(w.bid),
                    delta_percent(w.bid, w.reference)
                ),
                TextRole::Value,
            ),
        );
        spawn_kv(
            block,
            "Asks",
            text(
                format!(
                    "{} cr  {:+}%",
                    thousands(w.ask),
                    delta_percent(w.ask, w.reference)
                ),
                TextRole::Value,
            ),
        );
        spawn_kv(
            block,
            "In hold",
            text(
                format!("x{}  {}", market.ship[i], kg_text(market.ship[i] * w.kg)),
                TextRole::Value,
            ),
        );
        spawn_kv(
            block,
            "In stock",
            text(format!("x{}", market.station[i]), TextRole::Value),
        );
    });
    // A station row buys, a hold row sells: the other side's verbs have no
    // use here and are not drawn.
    let (title, verb, verb_all, name, name_all, click, click_all) = match trade_side {
        TradeSide::Tender => (
            "Sell",
            "Sell",
            "Sell all",
            MARKET_SELL,
            MARKET_SELL_ALL,
            Click::Sell,
            Click::SellAll,
        ),
        TradeSide::Trader | TradeSide::Storage => (
            "Buy",
            "Buy",
            "Buy all",
            MARKET_BUY,
            MARKET_BUY_ALL,
            Click::Buy,
            Click::BuyAll,
        ),
    };
    let max = market.draft_max();
    spawn_block(side, title, ()).with_children(|block| {
        draft_row(block, max);
        spawn_kv(
            block,
            "Price",
            (
                MarketDraftPrice,
                Short::default(),
                text("", TextRole::Value),
            ),
        );
        block.spawn(verb_row(RowAlign::End)).with_children(|row| {
            let gate = if max == 0 {
                VerbGate::Disabled
            } else {
                VerbGate::Ready
            };
            row.spawn((
                Name::new(name_all),
                acts(button(ButtonSpec::control(verb_all)), click_all),
                gate.clone(),
            ));
            let mut main = row.spawn((
                Name::new(name),
                acts(button(ButtonSpec::control(verb).primary()), click),
                gate,
            ));
            if trade_side == TradeSide::Trader {
                main.insert(MarketBuyVerb);
            }
        });
    });
}

/// The kit's amount row for the draft, at the draft's quantity.
fn draft_row(block: &mut ChildSpawnerCommands, max: u32) {
    block
        .spawn((
            MarketDraftAmount,
            amount_row(AmountSpec {
                name: "Gallery Draft",
            }),
        ))
        .insert(Amount {
            value: 1.min(max),
            min: 1.min(max),
            max,
        })
        .observe(on_draft_amount);
}

/// The inspector of a storage row: the counts in the hold and in storage,
/// the amount row, then Store and Take.
fn storage_inspector(side: &mut ChildSpawnerCommands, market: &MarketState, i: usize, w: &Ware) {
    spawn_block(side, "Storage", ()).with_children(|block| {
        spawn_kv(
            block,
            "In hold",
            text(
                format!("x{}  {}", market.ship[i], kg_text(market.ship[i] * w.kg)),
                TextRole::Value,
            ),
        );
        spawn_kv(
            block,
            "In storage",
            text(
                format!(
                    "x{}  {}",
                    market.storage[i],
                    kg_text(market.storage[i] * w.kg)
                ),
                TextRole::Value,
            ),
        );
        spawn_kv(
            block,
            "Stored",
            text(
                format!(
                    "{} / {}",
                    kg_text(market.stored_kg()),
                    kg_text(STORAGE_CAP_KG)
                ),
                TextRole::Value,
            ),
        );
    });
    spawn_block(side, "Move", ()).with_children(|block| {
        draft_row(block, market.draft_max());
        block.spawn(verb_row(RowAlign::End)).with_children(|row| {
            let gate = |have: u32| {
                if have == 0 {
                    VerbGate::Disabled
                } else {
                    VerbGate::Ready
                }
            };
            row.spawn((
                Name::new(MARKET_TAKE),
                acts(button(ButtonSpec::control("Take")), Click::Take),
                gate(market.storage[i]),
            ));
            row.spawn((
                Name::new(MARKET_STORE),
                acts(button(ButtonSpec::control("Store").primary()), Click::Store),
                gate(market.ship[i]),
            ));
        });
    });
}

/// The amount row moved: take its value as the draft quantity.
fn on_draft_amount(changed: On<AmountChanged>, mut draft: ResMut<MarketDraft>) {
    if draft.qty != changed.value {
        draft.qty = changed.value;
    }
}

/// Write the draft's price into the inspector: the price alone, painted
/// Danger with Buy disabled while the balance is short.
pub fn sync_draft_price(
    draft: Res<MarketDraft>,
    market: Res<MarketState>,
    mut q_price: Query<(&mut Text, &mut Short, Ref<MarketDraftPrice>)>,
    mut q_buy: Query<&mut VerbGate, With<MarketBuyVerb>>,
) {
    // A rebuilt inspector arrives a frame after the state moved; its fresh
    // price needs the first write too.
    let fresh = q_price.iter().any(|(_, _, price)| price.is_added());
    if !draft.is_changed() && !market.is_changed() && !fresh {
        return;
    }
    let Some((side, i)) = market.selected else {
        return;
    };
    let qty = draft.qty.min(market.draft_max());
    let w = &WARES[i];
    let (cost, short) = match side {
        TradeSide::Trader => (qty * w.ask, qty * w.ask > market.credits),
        TradeSide::Tender => (qty * w.bid, false),
        TradeSide::Storage => return,
    };
    for (mut text, mut flag, _) in &mut q_price {
        let wanted = format!("{} cr", thousands(cost));
        if text.0 != wanted {
            text.0 = wanted;
        }
        if flag.0 != short {
            flag.0 = short;
        }
    }
    for mut gate in &mut q_buy {
        let wanted = if short || qty == 0 {
            VerbGate::Disabled
        } else {
            VerbGate::Ready
        };
        if *gate != wanted {
            *gate = wanted;
        }
    }
}

/// Apply a buy of `qty` to the mock stocks. Returns the toast headline,
/// empty when nothing moved.
pub fn buy(market: &mut MarketState, qty: u32) -> String {
    let Some((_, i)) = market.selected else {
        return String::new();
    };
    let w = &WARES[i];
    let qty = qty.min(market.station[i]);
    let cost = qty * w.ask;
    if qty == 0 {
        return String::new();
    }
    if cost > market.credits {
        return String::new();
    }
    market.station[i] -= qty;
    market.ship[i] += qty;
    market.credits -= cost;
    format!("+{qty} {}", w.name)
}

/// Apply a sale of `qty` to the mock stocks. Returns the toast headline.
pub fn sell(market: &mut MarketState, qty: u32) -> String {
    let Some((_, i)) = market.selected else {
        return String::new();
    };
    let w = &WARES[i];
    let qty = qty.min(market.ship[i]);
    if qty == 0 {
        return String::new();
    }
    let pay = qty * w.bid;
    market.ship[i] -= qty;
    market.station[i] += qty;
    market.credits += pay;
    format!("-{qty} {}", w.name)
}

/// Move `qty` of the selected stack from the hold into storage, within the
/// cap. Returns the toast headline.
pub fn store(market: &mut MarketState, qty: u32) -> String {
    let Some((_, i)) = market.selected else {
        return String::new();
    };
    let w = &WARES[i];
    let room = (STORAGE_CAP_KG.saturating_sub(market.stored_kg())) / w.kg.max(1);
    let qty = qty.min(market.ship[i]);
    if qty == 0 {
        return String::new();
    }
    if qty > room {
        return String::new();
    }
    market.ship[i] -= qty;
    market.storage[i] += qty;
    format!("-{qty} {}", w.name)
}

/// Move `qty` of the selected stack from storage into the hold. Returns the
/// toast headline.
pub fn take(market: &mut MarketState, qty: u32) -> String {
    let Some((_, i)) = market.selected else {
        return String::new();
    };
    let w = &WARES[i];
    let qty = qty.min(market.storage[i]);
    if qty == 0 {
        return String::new();
    }
    market.storage[i] -= qty;
    market.ship[i] += qty;
    format!("+{qty} {}", w.name)
}
