//! The Market page: the INVENTORY pane as the game draws it, with prices.
//! The category chips over the Tender, Trader and Storage columns, the
//! MARKET strip under them, and the inspector with the game's quantity form
//! plus Buy, Sell, Buy all and Sell all on a trade row, or Store and Take on
//! a storage row. A trade or a move shifts the mock stocks and credits.

use bevy::{
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{
        Slider, SliderPrecision, SliderRange, SliderStep, SliderValue, TrackClick, ValueChange,
    },
};
use nova_ui::{prelude::*, theme::UiColor};

use crate::{
    chrome::*,
    data::{delta_percent, thousands, Category, Ware, OWN_HOLDS, STORAGE_CAP_KG, WARES},
    icons::{icon, GalleryIcons},
    Click, MarketDraft, MarketState, TradeSide, FILTER_PREFIX, MARKET_BUY, MARKET_BUY_ALL,
    MARKET_REFUSED, MARKET_SELL, MARKET_SELL_ALL, MARKET_STORE, MARKET_TAKE, TRADE_ROW_PREFIX,
};

const TABS: [&str; 5] = ["Map", "Ship", "Inventory", "Jobs", "Station"];
/// Hold capacity of the docked cargoa, in kg.
const HOLD_KG: u32 = 30_000;

/// The quantity field of the inspector's form.
#[derive(Component)]
pub struct MarketDraftField;
/// The quantity slider of the inspector's form.
#[derive(Component)]
pub struct MarketDraftSlider;
/// The result line of the inspector's form.
#[derive(Component)]
pub struct MarketDraftSummary;

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
                stores
                    .spawn(Node {
                        column_gap: px(8),
                        ..control_row(JustifyContent::FlexStart)
                    })
                    .with_children(|bar| {
                        chip(
                            bar,
                            icons,
                            None,
                            "All",
                            market.filter.is_none(),
                            format!("{FILTER_PREFIX} All"),
                            Click::Filter(None),
                        );
                        for category in Category::ALL {
                            chip(
                                bar,
                                icons,
                                Some(category.icon()),
                                category.label(),
                                market.filter == Some(category),
                                format!("{FILTER_PREFIX} {}", category.label()),
                                Click::Filter(Some(category)),
                            );
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

fn listed(market: &MarketState, side: TradeSide, i: usize, w: &Ware) -> bool {
    let qty = match side {
        TradeSide::Tender => market.ship[i],
        TradeSide::Trader => market.station[i],
        TradeSide::Storage => market.storage[i],
    };
    qty > 0 && market.filter.is_none_or(|c| c == w.category)
}

fn trade_column(
    columns: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    side: TradeSide,
    market: &MarketState,
) {
    columns.spawn(column_panel()).with_children(|panel| {
        panel
            .spawn(control_row(JustifyContent::SpaceBetween))
            .with_children(|head| {
                let title = match side {
                    TradeSide::Tender => "Tender: cargoa",
                    TradeSide::Trader => "Trader: Dust Gate",
                    TradeSide::Storage => unreachable!("the storage column has its own panel"),
                };
                head.spawn(text(title, 15.0, UiColor::Primary));
            });
        panel
            .spawn(Node {
                column_gap: px(18),
                ..control_row(JustifyContent::FlexStart)
            })
            .with_children(|facts| match side {
                TradeSide::Tender => {
                    fact(
                        facts,
                        "Cargo",
                        &format!("{} / {}", kg_text(market.hold_kg()), kg_text(HOLD_KG)),
                    );
                    fact(
                        facts,
                        "Credits",
                        &format!("{} cr", thousands(market.credits)),
                    );
                }
                TradeSide::Trader => {
                    let stock: u32 = WARES
                        .iter()
                        .zip(&market.station)
                        .map(|(w, q)| w.kg * q)
                        .sum();
                    fact(facts, "Stock", &kg_text(stock));
                    fact(facts, "Restock", "in 12 min");
                    fact(
                        facts,
                        "Refusal",
                        &format!("{} min left", market.refused_min),
                    );
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
        let mut any = false;
        for (i, w) in WARES.iter().enumerate() {
            if !listed(market, side, i, w) {
                continue;
            }
            any = true;
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
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                column_gap: px(8),
                ..row_node()
            })
            .with_children(|r| {
                r.spawn(icon(icons, w.category.icon(), 16.0));
                r.spawn(grow_cell(w.name, 13.0, UiColor::Primary));
                r.spawn(cell(40.0, &format!("x{qty}"), 12.0, UiColor::Body, true));
                r.spawn(cell(
                    48.0,
                    &format!("{} cr", thousands(price)),
                    12.0,
                    UiColor::Accent,
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
        if !any {
            panel.spawn((
                Node {
                    padding: UiRect::axes(px(8), px(8)),
                    ..default()
                },
                children![text("Nothing in this category.", 12.0, UiColor::Label)],
            ));
        }
    });
}

/// The Storage column: the station's storage with its stored mass against
/// the cap, then one headed section per own ship hold, as the game's
/// `InventorySideType::Storage` column. Only the station's rows select;
/// an own hold is listed to read.
fn storage_column(columns: &mut ChildSpawnerCommands, icons: &GalleryIcons, market: &MarketState) {
    columns
        .spawn(column_panel())
        .insert(Node {
            flex_grow: 0.65,
            ..column_panel_node()
        })
        .with_children(|panel| {
            panel.spawn(text("Storage: Dust Gate", 15.0, UiColor::Primary));
            panel
                .spawn(Node {
                    column_gap: px(18),
                    ..control_row(JustifyContent::FlexStart)
                })
                .with_children(|facts| {
                    fact(
                        facts,
                        "Stored",
                        &format!(
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
            let mut any = false;
            for (i, w) in WARES.iter().enumerate() {
                if !listed(market, TradeSide::Storage, i, w) {
                    continue;
                }
                any = true;
                let qty = market.storage[i];
                spawn_row(
                    panel,
                    Click::Trade(TradeSide::Storage, i),
                    format!("{TRADE_ROW_PREFIX} Storage {i}"),
                    market.selected == Some((TradeSide::Storage, i)),
                )
                .insert(Node {
                    padding: UiRect::axes(px(8), px(3)),
                    border: UiRect::all(px(1)),
                    column_gap: px(8),
                    ..row_node()
                })
                .with_children(|r| stack_cells(r, icons, w, qty));
            }
            if !any {
                panel.spawn((
                    Node {
                        padding: UiRect::axes(px(8), px(8)),
                        ..default()
                    },
                    children![text(
                        "Nothing stored in this category.",
                        12.0,
                        UiColor::Label
                    )],
                ));
            }
            for hold in &OWN_HOLDS {
                let load: u32 = hold.stacks.iter().map(|(i, q)| WARES[*i].kg * q).sum();
                panel
                    .spawn(Node {
                        column_gap: px(8),
                        margin: UiRect::top(px(8)),
                        padding: UiRect::axes(px(8), px(2)),
                        ..control_row(JustifyContent::SpaceBetween)
                    })
                    .with_children(|head| {
                        head.spawn(text(hold.ship, 13.0, UiColor::Primary));
                        head.spawn(text(
                            &format!("{} / {}", kg_text(load), kg_text(hold.cap_kg)),
                            12.0,
                            UiColor::Label,
                        ));
                    });
                for (i, qty) in hold.stacks {
                    let w = &WARES[*i];
                    if !market.filter.is_none_or(|c| c == w.category) {
                        continue;
                    }
                    panel
                        .spawn(Node {
                            padding: UiRect::axes(px(9), px(4)),
                            column_gap: px(8),
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
    r.spawn(grow_cell(w.name, 13.0, UiColor::Primary));
    r.spawn(cell(48.0, &format!("x{qty}"), 12.0, UiColor::Body, true));
    r.spawn(cell(58.0, &kg_text(qty * w.kg), 12.0, UiColor::Body, true));
}

/// The `Deal` badge: the percent off the reference, green when it favours
/// the player (a Tender bid over the reference, a Trader ask under it),
/// amber when not, mute at it.
fn deal_badge(side: TradeSide, delta: i32) -> impl Bundle {
    let gain = match side {
        TradeSide::Tender => delta.signum(),
        TradeSide::Trader => -delta.signum(),
        TradeSide::Storage => unreachable!("the storage column shows no price"),
    };
    let kind = match gain {
        1 => BadgeKind::Green,
        -1 => BadgeKind::Amber,
        _ => BadgeKind::Mute,
    };
    badge(kind, &format!("{delta:+}%"))
}

/// The MARKET strip: the best buyer and the cheapest seller, the restock
/// meter, and the refusal.
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
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            panel_paint(),
        ))
        .with_children(|strip| {
            strip
                .spawn(Node {
                    column_gap: px(10),
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: px(4),
                    ..control_row(JustifyContent::FlexStart)
                })
                .with_children(|row| {
                    row.spawn(text("MARKET", 11.0, UiColor::Label));
                    row.spawn(text("Pays most", 12.0, UiColor::Body));
                    row.spawn(tag(
                        BadgeKind::Amber,
                        &format!(
                            "{} {:+}%",
                            pays_most.name,
                            delta_percent(pays_most.bid, pays_most.reference)
                        ),
                    ));
                    row.spawn(text("Sells cheapest", 12.0, UiColor::Body));
                    row.spawn(tag(
                        BadgeKind::Green,
                        &format!(
                            "{} {:+}%",
                            cheapest.name,
                            delta_percent(cheapest.ask, cheapest.reference)
                        ),
                    ));
                    row.spawn(Node {
                        flex_grow: 1.0,
                        ..default()
                    });
                    row.spawn((
                        Name::new(MARKET_REFUSED),
                        tag(
                            BadgeKind::Red,
                            &format!("REFUSED {} min", market.refused_min),
                        ),
                    ));
                });
            strip
                .spawn(Node {
                    column_gap: px(10),
                    ..control_row(JustifyContent::FlexStart)
                })
                .with_children(|row| {
                    row.spawn(text("Restock in 12 min", 12.0, UiColor::Body));
                    let ore = market.station[0];
                    row.spawn(bare_meter(140.0, ore as f32 / 40.0));
                    row.spawn(text(
                        &format!("iron ore stock {ore} t / target 40 t"),
                        12.0,
                        UiColor::Label,
                    ));
                    row.spawn(text(
                        "Refusal: docked Vane Dock first; the yard's quarantine lifts when the timer ends.",
                        11.0,
                        UiColor::Label,
                    ));
                });
        });
}

fn inspector(side: &mut ChildSpawnerCommands, icons: &GalleryIcons, market: &MarketState) {
    let Some((trade_side, i)) = market.selected else {
        section(side, "Item");
        side.spawn(text(
            "Select an item to read its prices and trade it.",
            12.0,
            UiColor::Label,
        ));
        return;
    };
    let w = &WARES[i];
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|head| {
        head.spawn(icon(icons, w.category.icon(), 24.0));
        head.spawn(text(w.name, 15.0, UiColor::Primary));
    });
    side.spawn(Node {
        column_gap: px(6),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|tags| {
        tags.spawn(tag(BadgeKind::Mute, &w.category.label().to_uppercase()));
        tags.spawn(tag(
            BadgeKind::Blue,
            match trade_side {
                TradeSide::Tender => "IN HOLD",
                TradeSide::Trader => "IN STOCK",
                TradeSide::Storage => "IN STORAGE",
            },
        ));
    });
    side.spawn(divider());
    if trade_side == TradeSide::Storage {
        storage_inspector(side, market, i, w);
        return;
    }
    kv(side, "Reference", &format!("{} cr", thousands(w.reference)));
    kv(
        side,
        "Pays",
        &format!(
            "{} cr ({:+}%)",
            thousands(w.bid),
            delta_percent(w.bid, w.reference)
        ),
    );
    kv(
        side,
        "Asks",
        &format!(
            "{} cr ({:+}%)",
            thousands(w.ask),
            delta_percent(w.ask, w.reference)
        ),
    );
    kv(
        side,
        "In hold",
        &format!("x{} ({})", market.ship[i], kg_text(market.ship[i] * w.kg)),
    );
    kv(side, "In stock", &format!("x{}", market.station[i]));
    side.spawn(text(w.reason, 11.0, UiColor::Label));
    side.spawn(divider());
    side.spawn(text(
        match trade_side {
            TradeSide::Tender => "Sell from the hold",
            TradeSide::Trader | TradeSide::Storage => "Buy from the stock",
        },
        13.0,
        UiColor::Accent,
    ));
    draft_form(side, market.draft_max());
    side.spawn(Node {
        column_gap: px(8),
        flex_wrap: FlexWrap::Wrap,
        row_gap: px(6),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|row| {
        let can_buy = market.station[i] > 0;
        let can_sell = market.ship[i] > 0;
        let mut buy = row.spawn((
            Name::new(MARKET_BUY),
            acts(
                compact_button(ButtonSpec::new("Buy").fit().primary()),
                Click::Buy,
            ),
        ));
        if !can_buy {
            buy.insert((InteractionDisabled, Tooltip("Nothing in stock".into())));
        }
        let mut sell = row.spawn((
            Name::new(MARKET_SELL),
            acts(
                compact_button(ButtonSpec::new("Sell").fit().primary()),
                Click::Sell,
            ),
        ));
        if !can_sell {
            sell.insert((InteractionDisabled, Tooltip("Nothing in the hold".into())));
        }
        let mut buy_all = row.spawn((
            Name::new(MARKET_BUY_ALL),
            acts(
                compact_button(ButtonSpec::new("Buy all").fit()),
                Click::BuyAll,
            ),
        ));
        if !can_buy {
            buy_all.insert(InteractionDisabled);
        }
        let mut sell_all = row.spawn((
            Name::new(MARKET_SELL_ALL),
            acts(
                compact_button(ButtonSpec::new("Sell all").fit()),
                Click::SellAll,
            ),
        ));
        if !can_sell {
            sell_all.insert(InteractionDisabled);
        }
    });
    if let Some(note) = &market.note {
        side.spawn(text(note, 12.0, UiColor::Nominal));
    }
}

/// The quantity form: the field, `of max`, All, the slider and the summary
/// line that `sync_draft` keeps in step.
fn draft_form(side: &mut ChildSpawnerCommands, max: u32) {
    side.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        row_gap: px(6),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|row| {
        row.spawn(Node {
            width: px(72),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|cell| {
            cell.spawn((
                Name::new("Gallery Draft Quantity"),
                MarketDraftField,
                text_field(TextFieldSpec::new("1").max_chars(5).dense()),
            ));
        });
        row.spawn((
            text(&format!("of {max}"), 15.0, UiColor::Primary),
            Node {
                flex_grow: 1.0,
                ..default()
            },
        ));
        row.spawn((
            Name::new("Gallery Draft All"),
            acts(
                compact_button(ButtonSpec::new("All").fit().ghost()),
                Click::DraftAll,
            ),
        ));
    });
    side.spawn((
        MarketDraftSlider,
        Slider {
            track_click: TrackClick::Snap,
            ..default()
        },
        SliderValue(1.0),
        SliderRange::new(0.0, max.max(1) as f32),
        SliderStep(1.0),
        SliderPrecision(0),
        slider_track(if max == 0 { 0.0 } else { 1.0 / max as f32 }),
    ))
    .observe(on_draft_slide);
    side.spawn((
        MarketDraftSummary,
        text("", 13.0, UiColor::Body),
        Node {
            min_height: px(13.0 * 1.2 * 3.0),
            ..default()
        },
    ));
}

/// The inspector of a storage row: the counts in the hold and in storage,
/// the quantity form, then Store and Take.
fn storage_inspector(side: &mut ChildSpawnerCommands, market: &MarketState, i: usize, w: &Ware) {
    kv(
        side,
        "In hold",
        &format!("x{} ({})", market.ship[i], kg_text(market.ship[i] * w.kg)),
    );
    kv(
        side,
        "In storage",
        &format!(
            "x{} ({})",
            market.storage[i],
            kg_text(market.storage[i] * w.kg)
        ),
    );
    kv(
        side,
        "Storage",
        &format!(
            "{} / {}",
            kg_text(market.stored_kg()),
            kg_text(STORAGE_CAP_KG)
        ),
    );
    side.spawn(text(
        "Storage keeps a stack for free while the station lives; it is lost with the station.",
        11.0,
        UiColor::Label,
    ));
    side.spawn(divider());
    side.spawn(text(
        "Move between the hold and storage",
        13.0,
        UiColor::Accent,
    ));
    draft_form(side, market.draft_max());
    side.spawn(Node {
        column_gap: px(8),
        flex_wrap: FlexWrap::Wrap,
        row_gap: px(6),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|row| {
        let mut store = row.spawn((
            Name::new(MARKET_STORE),
            acts(
                compact_button(ButtonSpec::new("Store").fit().primary()),
                Click::Store,
            ),
        ));
        if market.ship[i] == 0 {
            store.insert((InteractionDisabled, Tooltip("Nothing in the hold".into())));
        }
        let mut take = row.spawn((
            Name::new(MARKET_TAKE),
            acts(
                compact_button(ButtonSpec::new("Take").fit().primary()),
                Click::Take,
            ),
        ));
        if market.storage[i] == 0 {
            take.insert((InteractionDisabled, Tooltip("Nothing in storage".into())));
        }
    });
    if let Some(note) = &market.note {
        side.spawn(text(note, 12.0, UiColor::Nominal));
    }
}

/// The slider moved: take its value as the draft quantity.
fn on_draft_slide(change: On<ValueChange<f32>>, mut draft: ResMut<MarketDraft>) {
    let qty = change.value.round().max(0.0) as u32;
    if draft.qty != qty {
        draft.qty = qty;
    }
}

/// The field changed: take its number as the draft quantity, clamped to
/// the stock.
pub fn read_draft_field(
    fields: Query<&TextFieldValue, (Changed<TextFieldValue>, With<MarketDraftField>)>,
    market: Res<MarketState>,
    mut draft: ResMut<MarketDraft>,
) {
    for value in &fields {
        let Ok(qty) = value.0.trim().parse::<u32>() else {
            continue;
        };
        let qty = qty.min(market.draft_max());
        if draft.qty != qty {
            draft.qty = qty;
        }
    }
}

/// Write the draft back into the form in place: the slider's value, the
/// field's text while it is not being typed in, and the summary line.
pub fn sync_draft(
    mut commands: Commands,
    draft: Res<MarketDraft>,
    market: Res<MarketState>,
    sliders: Query<(Entity, &SliderValue), With<MarketDraftSlider>>,
    mut fields: Query<(&mut TextFieldValue, Has<TextFieldFocused>), With<MarketDraftField>>,
    mut summaries: Query<(&mut Text, Ref<MarketDraftSummary>)>,
) {
    // A rebuilt form arrives a frame after the state moved; its fresh
    // summary needs the first write too.
    let fresh = summaries.iter().any(|(_, summary)| summary.is_added());
    if !draft.is_changed() && !market.is_changed() && !fresh {
        return;
    }
    let qty = draft.qty.min(market.draft_max());
    // `SliderValue` is immutable; the widget takes a fresh insert.
    for (entity, value) in &sliders {
        if value.0 != qty as f32 {
            commands.entity(entity).insert(SliderValue(qty as f32));
        }
    }
    for (mut value, focused) in &mut fields {
        let wanted = qty.to_string();
        if !focused && value.0 != wanted {
            value.0 = wanted;
        }
    }
    let Some((side, i)) = market.selected else {
        return;
    };
    let w = &WARES[i];
    let summary = match side {
        TradeSide::Trader => {
            let cost = qty * w.ask;
            if qty == 0 {
                "Pick a quantity.".to_string()
            } else if cost > market.credits {
                format!(
                    "Not enough credits: {} cr for x{qty}, {} cr in hand.",
                    thousands(cost),
                    thousands(market.credits)
                )
            } else {
                format!(
                    "Buy x{qty} {} for {} cr; {} cr left.",
                    w.name,
                    thousands(cost),
                    thousands(market.credits - cost)
                )
            }
        }
        TradeSide::Tender => {
            let pay = qty * w.bid;
            if qty == 0 {
                "Pick a quantity.".to_string()
            } else {
                format!(
                    "Sell x{qty} {} for {} cr; {} cr after.",
                    w.name,
                    thousands(pay),
                    thousands(market.credits + pay)
                )
            }
        }
        TradeSide::Storage => {
            if qty == 0 {
                "Pick a quantity.".to_string()
            } else {
                format!(
                    "Store up to x{} from the hold, take up to x{} from storage; x{qty} picked.",
                    market.ship[i], market.storage[i]
                )
            }
        }
    };
    for (mut text, _) in &mut summaries {
        if text.0 != summary {
            text.0 = summary.clone();
        }
    }
}

/// Apply a buy of `qty` to the mock stocks. Returns the note to show.
pub fn buy(market: &mut MarketState, qty: u32) -> String {
    let Some((_, i)) = market.selected else {
        return "Select an item first.".into();
    };
    let w = &WARES[i];
    let qty = qty.min(market.station[i]);
    let cost = qty * w.ask;
    if qty == 0 {
        return "Nothing to buy.".into();
    }
    if cost > market.credits {
        return format!("Not enough credits: {} cr needed.", thousands(cost));
    }
    market.station[i] -= qty;
    market.ship[i] += qty;
    market.credits -= cost;
    format!("Bought x{qty} {} for {} cr.", w.name, thousands(cost))
}

/// Apply a sale of `qty` to the mock stocks. Returns the note to show.
pub fn sell(market: &mut MarketState, qty: u32) -> String {
    let Some((_, i)) = market.selected else {
        return "Select an item first.".into();
    };
    let w = &WARES[i];
    let qty = qty.min(market.ship[i]);
    if qty == 0 {
        return "Nothing to sell.".into();
    }
    let pay = qty * w.bid;
    market.ship[i] -= qty;
    market.station[i] += qty;
    market.credits += pay;
    format!("Sold x{qty} {} for {} cr.", w.name, thousands(pay))
}

/// Move `qty` of the selected stack from the hold into storage, within the
/// cap. Returns the note to show.
pub fn store(market: &mut MarketState, qty: u32) -> String {
    let Some((_, i)) = market.selected else {
        return "Select a stack first.".into();
    };
    let w = &WARES[i];
    let room = (STORAGE_CAP_KG.saturating_sub(market.stored_kg())) / w.kg.max(1);
    let qty = qty.min(market.ship[i]);
    if qty == 0 {
        return "Nothing in the hold to store.".into();
    }
    if qty > room {
        return format!(
            "Storage is full: room for x{room} {} under the {} cap.",
            w.name,
            kg_text(STORAGE_CAP_KG)
        );
    }
    market.ship[i] -= qty;
    market.storage[i] += qty;
    format!("Stored x{qty} {}.", w.name)
}

/// Move `qty` of the selected stack from storage into the hold. Returns the
/// note to show.
pub fn take(market: &mut MarketState, qty: u32) -> String {
    let Some((_, i)) = market.selected else {
        return "Select a stack first.".into();
    };
    let w = &WARES[i];
    let qty = qty.min(market.storage[i]);
    if qty == 0 {
        return "Nothing in storage to take.".into();
    }
    market.storage[i] -= qty;
    market.ship[i] += qty;
    format!("Took x{qty} {} aboard.", w.name)
}
