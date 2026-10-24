//! The MARKET strip under the Inventory pane's columns: what the docked
//! trader pays most for and sells cheapest against the reference prices,
//! the stock it is furthest from restocking with the play time to fill it,
//! and the REFUSED badge while its civilization refuses the player.
//!
//! The strip draws only while the partner trades by a
//! [`TraderMarket`]: a generated trader with a table and a target. An
//! authored trader deals at reference and has no figure off it, and a
//! lootable or own partner trades nothing. Every figure is computed by
//! `nova_gameplay::market` and only rendered here.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ui::{
    theme::UiColor,
    widget::{badge, slider_track, BadgeKind},
};
use nova_world_base::prelude::CivilizationField;

use super::InventoryShips;
use crate::pane::{control_row, panel_paint, themed_label};

/// The `Name` of the strip's node, for tests and captures.
pub const INVENTORY_MARKET_STRIP: &str = "InventoryMarketStrip";

/// The `Name` of the REFUSED badge in the strip.
pub const INVENTORY_MARKET_REFUSED: &str = "InventoryMarketRefused";

/// The width of the restock meter, in logical px.
const METER_PX: f32 = 140.0;

/// The strip's node. It is hidden while [`MarketStrip::drawn`] is `None`.
#[derive(Component, Default, Debug)]
pub(crate) struct MarketStrip {
    /// What the strip shows, so a frame that changes nothing rebuilds
    /// nothing. `None` while it is hidden.
    drawn: Option<MarketStripDraw>,
}

impl MarketStrip {
    /// What the strip shows, for the pane tests: the rig has no theme, so a
    /// badge there never gets its legend text.
    #[cfg(test)]
    pub(crate) fn drawn(&self) -> Option<&MarketStripDraw> {
        self.drawn.as_ref()
    }
}

/// One lead badge: the item's name and its signed percent off the reference.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct LeadDraw {
    pub(crate) name: String,
    pub(crate) delta_percent: i32,
}

/// The restock line: the most starved targeted item and its fill.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct RestockDraw {
    pub(crate) name: String,
    pub(crate) stock: u32,
    pub(crate) target: u32,
    /// Whole play minutes until the trader holds its target of it, rounded
    /// up, so a gap never reads as `0 min`.
    pub(crate) minutes_away: u32,
    pub(crate) fraction: f32,
}

/// Everything the strip shows.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct MarketStripDraw {
    pub(crate) pays_most: Vec<LeadDraw>,
    pub(crate) sells_cheapest: Vec<LeadDraw>,
    pub(crate) restock: Option<RestockDraw>,
    /// `Some(minutes)` while the civilization refuses with an end in sight,
    /// `Some(None)` while only a settlement ends it, `None` while it deals.
    pub(crate) refused: Option<Option<u32>>,
}

/// Spawn the strip hidden under the columns.
pub(crate) fn spawn_market_strip(stores: &mut ChildSpawnerCommands) {
    stores.spawn((
        MarketStrip::default(),
        Name::new(INVENTORY_MARKET_STRIP),
        Node {
            display: Display::None,
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            padding: UiRect::all(px(8)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        panel_paint(),
    ));
}

/// Whole minutes of `seconds`, rounded up; a positive rest never reads as
/// zero.
fn minutes_up(seconds: f64) -> u32 {
    u32::try_from((seconds / 60.0).ceil().max(0.0) as i64).unwrap_or(u32::MAX)
}

/// Draw the strip for the docked trader, or hide it. The refusal reads the
/// civilization stances and the play clock; without either resource, as in
/// a pane test with no world, the strip shows no refusal.
pub(crate) fn update_market_strip(
    mut commands: Commands,
    ships: InventoryShips,
    stances: Option<Res<CivilizationStances>>,
    clock: Option<Res<WorldClock>>,
    mut q_strip: Query<(Entity, &mut MarketStrip, &mut Node)>,
) {
    let Ok((strip, mut state, mut node)) = q_strip.single_mut() else {
        return;
    };
    let draw =
        ships
            .pair()
            .and_then(|pair| ships.trader_market(pair))
            .map(|(table, market, hold)| {
                let items = ships.items();
                let name = |item: &ItemDesignId| items.design(item).name.clone();
                let lead = |lead: MarketLead| LeadDraw {
                    name: name(&lead.item),
                    delta_percent: lead.delta_percent,
                };
                let summary = market_summary(table, market, items.iter(), |item| hold.count(item));
                let restock =
                    restock_gap(table, market, |item| hold.count(item)).map(|gap| RestockDraw {
                        name: name(&gap.item),
                        stock: gap.stock,
                        target: gap.target,
                        minutes_away: minutes_up(gap.seconds_away),
                        fraction: gap.stock as f32 / gap.target as f32,
                    });
                let refused =
                    stances
                        .as_deref()
                        .zip(clock.as_deref())
                        .and_then(|(stances, clock)| {
                            let seeded = CivilizationField::new(market.civilization.world_seed)
                                .civilization(market.civilization.node)
                                .stance;
                            refusal(stances, &market.civilization, seeded, clock.play_seconds)
                                .map(|refusal| refusal.seconds_left.map(minutes_up))
                        });
                MarketStripDraw {
                    pays_most: summary.pays_most.into_iter().map(lead).collect(),
                    sells_cheapest: summary.sells_cheapest.into_iter().map(lead).collect(),
                    restock,
                    refused,
                }
            });
    if state.drawn == draw {
        return;
    }
    let display = if draw.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    if node.display != display {
        node.display = display;
    }
    commands.entity(strip).despawn_children();
    if let Some(draw) = &draw {
        commands
            .entity(strip)
            .with_children(|strip| spawn_strip_rows(strip, draw));
    }
    state.drawn = draw;
}

/// The lead badge text: `iron ore +40%`.
fn lead_text(lead: &LeadDraw) -> String {
    format!("{} {:+}%", lead.name, lead.delta_percent)
}

/// The strip's two rows: the leads and the refusal, then the restock meter.
fn spawn_strip_rows(strip: &mut ChildSpawnerCommands, draw: &MarketStripDraw) {
    strip
        .spawn(Node {
            column_gap: px(10),
            flex_wrap: FlexWrap::Wrap,
            row_gap: px(4),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|row| {
            row.spawn(themed_label("MARKET", 11.0, UiColor::Label));
            row.spawn(themed_label("Buys", 12.0, UiColor::Body));
            for lead in &draw.pays_most {
                row.spawn(badge(BadgeKind::Amber, &lead_text(lead)));
            }
            row.spawn(themed_label("Sells", 12.0, UiColor::Body));
            if draw.sells_cheapest.is_empty() {
                row.spawn(themed_label("Empty", 12.0, UiColor::Label));
            }
            for lead in &draw.sells_cheapest {
                row.spawn(badge(BadgeKind::Green, &lead_text(lead)));
            }
            row.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            if let Some(minutes) = draw.refused {
                let text = match minutes {
                    Some(minutes) => format!("REFUSED {minutes} min"),
                    None => "REFUSED".to_string(),
                };
                row.spawn((
                    Name::new(INVENTORY_MARKET_REFUSED),
                    badge(BadgeKind::Red, &text),
                ));
            }
        });
    strip
        .spawn(Node {
            column_gap: px(10),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|row| {
            row.spawn(themed_label("Restock", 12.0, UiColor::Body));
            match &draw.restock {
                Some(restock) => {
                    row.spawn((
                        Node {
                            width: px(METER_PX),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        children![slider_track(restock.fraction)],
                    ));
                    row.spawn(themed_label(
                        &format!(
                            "{} {} / {}, {} min",
                            restock.name, restock.stock, restock.target, restock.minutes_away
                        ),
                        12.0,
                        UiColor::Label,
                    ));
                }
                None => {
                    row.spawn((
                        Node {
                            width: px(METER_PX),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        children![slider_track(1.0)],
                    ));
                    row.spawn(themed_label("Stocked", 12.0, UiColor::Label));
                }
            }
        });
}
