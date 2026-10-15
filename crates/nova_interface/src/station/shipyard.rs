//! The Station pane's "Ships for sale" list and the Buy action.
//!
//! While the ship is docked at a living station that offers Refit and has
//! hulls parked at its ports ([`ParkedHulls`]), the pane lists every one as
//! a card: the hull's own model turning (`hull_preview`) and one line with
//! its name, length and price, or "your ship" for a ship the player owns
//! ([`ParkedHullType::Owned`]). A hull for sale has a Buy button, or the
//! reason the buy is refused; an owned ship has a Board button. A click
//! raises a [`HullBuyCommand`]; [`apply_station_hull_buy_commands`] plans a
//! buy with [`plan_hull_buy`] and moves the price, or boards for free, and
//! raises the [`HullBuyRequest`] the scenario crate swaps the hulls from.
//! The old ship stays the player's, parked with its hold.
//!
//! Touch this module when changing what the list shows or what Buy and
//! Board move.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, Meters};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::{
    card_actions, offer_cell, offer_rows, spawn_hull_card_head, spawn_hull_info_button,
    StationRuntime,
};
use crate::{
    inventory::cr_text,
    pane::{compact_button, play_menu_select, themed_label},
    ship::{ResolvedSite, ShipRuntime, StationSite},
};

/// A request to buy a hull for sale, or to board a ship the player owns,
/// parked at the docked station, raised by a "Ships for sale" Buy or Board
/// button.
#[derive(Message, Clone, Debug)]
pub struct HullBuyCommand {
    /// The parked hull.
    pub hull: Entity,
}

/// The card line after an owned ship's length, in place of a price.
const OWNED_DETAIL: &str = "your ship";

/// The hull list container, hidden while the refit gate does not hold or
/// the docked station sells no hull.
#[derive(Component)]
pub(crate) struct StationHullList;

/// The rows inside [`StationHullList`], and the row texts they were built from,
/// so the rows respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct StationHullRows(Vec<HullRow>);

impl StationHullRows {
    /// The hulls the rows list, in list order.
    pub(crate) fn hulls(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().map(|row| row.hull)
    }
}

/// One parked hull as the list shows it.
#[derive(Clone, Debug, PartialEq)]
struct HullRow {
    hull: Entity,
    name: String,
    /// The card line after the name: length and price, or "your ship".
    detail: String,
    /// The card's action button label, Buy or Board, or the refusal line.
    action: Result<&'static str, String>,
}

/// The hull's length: the span of its section centres along its longest
/// axis, plus one cell, in metres: `50 m`.
pub(crate) fn hull_length_text(sections: &[DesignSection]) -> String {
    let (low, high) = sections.iter().fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(low, high), section| (low.min(section.position), high.max(section.position)),
    );
    let span = (high - low).max_element().max(0.0) + 1.0;
    format!("{:.0} m", Meters::from_engine(span).0)
}

/// Why a buy of the hull named `name` is refused; the list and the handler
/// share it.
fn hull_refusal_text(name: &str, refusal: &HullBuyRefusalType) -> String {
    match refusal {
        HullBuyRefusalType::NotDocked => "buy: dock at a station that refits".to_string(),
        HullBuyRefusalType::NoRefitService => "buy: this dock offers no refit".to_string(),
        HullBuyRefusalType::InsufficientCredits { need, have } => format!(
            "buy: {name} costs {}; you have {}",
            cr_text(*need),
            cr_text(*have)
        ),
        HullBuyRefusalType::CreditOverflow => {
            format!("buy: a balance after buying {name} does not fit")
        }
    }
}

/// The parked hulls the list shows: every hull parked at a station, its
/// name, how it is listed and its design.
type ParkedHullQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Name,
        &'static ParkedHullType,
        &'static ShipDesignSections,
    ),
    With<ParkedHull>,
>;

/// A hull for sale's price: its recipes at ask.
fn parked_price_cr(items: &GameItems, design: &ShipDesignSections) -> u64 {
    hull_price_cr(
        items,
        design.0.iter().map(|section| &section.config.base.build),
    )
}

/// Plan buying a hull at `price` for the resolved player ship.
fn plan_offer(
    resolved: &ResolvedSite<'_>,
    price: u64,
    q_credits: &Query<&ShipCredits>,
) -> Result<HullBuyPlan, HullBuyRefusalType> {
    let own_cr = q_credits
        .get(resolved.player)
        .expect("the player ship carries ShipCredits");
    let partner = resolved.partner.and_then(|(partner, offers)| {
        Some(HullBuyPartner {
            offers,
            credits_cr: q_credits.get(partner).ok()?.0,
        })
    });
    plan_hull_buy(price, partner, own_cr.0)
}

/// Build the hull list: a header and the rows
/// [`update_station_hull_list`] fills.
pub(crate) fn spawn_station_hull_list(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            StationHullList,
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|list| {
            list.spawn(crate::pane::divider());
            list.spawn(themed_label("Ships for sale", 13.0, UiColor::Accent));
            list.spawn((StationHullRows::default(), offer_rows()));
        });
}

/// Show the hull list while the refit gate holds and the docked station
/// has hulls parked, and respawn its rows when a hull, a price or a refusal
/// changes.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the parked hulls, the site, both balances and the list"
)]
pub(crate) fn update_station_hull_list(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_parked: Query<&ParkedHulls>,
    q_hulls: ParkedHullQuery,
    q_credits: Query<&ShipCredits>,
    mut q_list: Query<&mut Node, With<StationHullList>>,
    mut q_rows: Query<(Entity, &mut StationHullRows)>,
) {
    let Ok(mut list) = q_list.single_mut() else {
        return;
    };
    let Ok((rows_entity, mut shown)) = q_rows.single_mut() else {
        return;
    };
    let resolved = if site.gate(StationServiceType::Refit) {
        site.resolve(StationServiceType::Refit)
    } else {
        None
    };
    let mut rows = Vec::new();
    if let (Some(resolved), Some(items)) = (resolved, items) {
        let parked = resolved
            .partner
            .and_then(|(partner, _)| q_parked.get(partner).ok());
        if let Some(parked) = parked {
            rows = parked
                .iter()
                .filter_map(|hull| {
                    let (name, terms, design) = q_hulls.get(hull).ok()?;
                    let length = hull_length_text(&design.0);
                    let (detail, action) = match terms {
                        ParkedHullType::Sale => {
                            let price_cr = parked_price_cr(&items, design);
                            let planned = plan_offer(&resolved, price_cr, &q_credits);
                            (
                                format!("{length}, {}", cr_text(price_cr)),
                                planned
                                    .map(|_| "Buy")
                                    .map_err(|refusal| hull_refusal_text(name.as_str(), &refusal)),
                            )
                        }
                        ParkedHullType::Owned => (format!("{length}, {OWNED_DETAIL}"), Ok("Board")),
                        ParkedHullType::Hire { .. } => return None,
                    };
                    Some(HullRow {
                        hull,
                        name: name.to_string(),
                        detail,
                        action,
                    })
                })
                .collect();
        }
    }
    let display = if rows.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    if list.display != display {
        list.display = display;
    }
    if shown.0 == rows {
        return;
    }
    commands.entity(rows_entity).despawn_children();
    commands.entity(rows_entity).with_children(|parent| {
        for (index, row) in rows.iter().enumerate() {
            parent.spawn(offer_cell()).with_children(|cell| {
                spawn_hull_card_head(cell, row.hull, &row.name, &row.detail);
                if let Err(refusal) = &row.action {
                    cell.spawn(themed_label(refusal, 12.0, UiColor::Danger));
                }
                cell.spawn(card_actions()).with_children(|actions| {
                    if let Ok(label) = row.action {
                        actions
                            .spawn((
                                Name::new(format!("Hull{label}-{index}")),
                                compact_button(ButtonSpec::new(label).fit().primary()),
                            ))
                            .observe(on_hull_buy_button(row.hull));
                    }
                    spawn_hull_info_button(
                        actions,
                        format!("HullInfo-{index}"),
                        row.hull,
                        format!("{}  {}", row.name, row.detail),
                    );
                });
            });
        }
    });
    shown.0 = rows;
}

/// Raise a [`HullBuyCommand`] for the parked hull `hull` when its Buy or
/// Board button is clicked.
fn on_hull_buy_button(
    hull: Entity,
) -> impl FnMut(On<Activate>, MessageWriter<HullBuyCommand>, Option<Res<SoundBank<UiSfx>>>, Commands)
{
    move |_activate, mut buys, bank, mut commands| {
        buys.write(HullBuyCommand { hull });
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Apply [`HullBuyCommand`]s. A buy of a hull for sale follows the
/// [`plan_hull_buy`] rule: it charges the price with `ShipBuy` credit
/// changes on both sides and queues one `ShipBought` report. A board of a
/// ship the player owns moves nothing. Either raises the [`HullBuyRequest`]
/// that swaps the hulls, and the old ship parks as the player's own with
/// its hold. The selection and any pending rebind or repair target clear,
/// since every section goes. A refusal changes nothing and shows its reason
/// on the note line.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the parked hulls, the site and both balances"
)]
pub(crate) fn apply_station_hull_buy_commands(
    mut messages: MessageReader<HullBuyCommand>,
    mut runtime: ResMut<StationRuntime>,
    mut ship: ResMut<ShipRuntime>,
    mut held: ResMut<HeldPart>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_hulls: Query<(&ParkedHull, &Name, &ParkedHullType, &ShipDesignSections)>,
    mut q_credits: ParamSet<(
        Query<&ShipCredits>,
        Query<(&mut ShipCredits, &mut ShipInventoryReports)>,
    )>,
) {
    if messages.is_empty() {
        return;
    }
    let (Some(resolved), Some(items)) = (site.resolve(StationServiceType::Refit), items) else {
        messages.clear();
        return;
    };
    // One buy replaces every section, so a second command this frame would
    // plan against sections already queued to despawn.
    let Some(command) = messages.read().last().cloned() else {
        return;
    };
    let Some((station, _)) = resolved.partner else {
        runtime.say(hull_refusal_text("", &HullBuyRefusalType::NotDocked));
        return;
    };
    let Ok((parked, name, terms, design)) = q_hulls.get(command.hull) else {
        panic!("buy: {:?} is no parked hull", command.hull);
    };
    assert_eq!(
        parked.0, station,
        "buy: hull {:?} is not parked at the docked station {station:?}",
        command.hull
    );
    let name = name.to_string();
    let note = match terms {
        ParkedHullType::Sale => {
            let price_cr = parked_price_cr(&items, design);
            let plan = match plan_offer(&resolved, price_cr, &q_credits.p0()) {
                Ok(plan) => plan,
                Err(refusal) => {
                    runtime.say(hull_refusal_text(&name, &refusal));
                    return;
                }
            };
            let mut q_credits = q_credits.p1();
            let Ok([(mut own_cr, mut own_reports), (mut station_cr, mut station_reports)]) =
                q_credits.get_many_mut([resolved.player, station])
            else {
                panic!(
                    "buy: the player ship and station {station:?} each carry credits and reports"
                );
            };
            own_reports.change_credits(&mut own_cr, plan.own_cr, CreditChangeCauseType::ShipBuy);
            station_reports.change_credits(
                &mut station_cr,
                plan.station_cr,
                CreditChangeCauseType::ShipBuy,
            );
            own_reports.0.push(ShipInventoryReport::ShipBought {
                hull: name.clone(),
                price_cr: plan.price_cr,
            });
            format!(
                "bought {name} for {}; your last ship is parked here.",
                cr_text(plan.price_cr)
            )
        }
        ParkedHullType::Owned => format!("boarded {name}; your last ship is parked here."),
        ParkedHullType::Hire { .. } => panic!("buy: {name} is an escort for hire, not for sale"),
    };
    commands.trigger(HullBuyRequest {
        entity: resolved.player,
        station,
        hull: command.hull,
    });
    ship.selected = None;
    ship.rebinding = None;
    ship.repair_target = None;
    held.0 = None;
    ship.reselect = None;
    runtime.say(note);
}
