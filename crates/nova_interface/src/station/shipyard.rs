//! The Station pane's "Your ships" and "Ships for sale" lists, and the Buy
//! and Board actions.
//!
//! While the ship is docked at a living station that offers Refit and has
//! hulls parked at its ports ([`ParkedHulls`]), the pane lists every one as
//! a card: the hull's own model turning (`hull_preview`), its name, its
//! length and its price. A ship the player owns ([`ParkedHullType::Owned`])
//! lists under "Your ships", above the hulls for sale, and says "your ship"
//! in place of a price. Each list shows only while it has a card. A hull for
//! sale has a Buy button, greyed with the reason as its tooltip while the
//! buy is refused; an owned ship has a Board button. A click
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
use nova_ui::theme::UiColor;

use super::{
    card_actions, offer_cell, offer_rows, spawn_card_price, spawn_hull_card_head,
    spawn_hull_info_button, StationRuntime,
};
use crate::{
    icons::InterfaceIcons,
    pane::{refusable_button, themed_label, ActionOutcomeType},
    ship::{ResolvedSite, ShipRuntime, StationSite},
};

/// A request to buy a hull for sale, or to board a ship the player owns,
/// parked at the docked station, raised by a "Ships for sale" Buy or a
/// "Your ships" Board button.
#[derive(Message, Clone, Debug)]
pub struct HullBuyCommand {
    /// The parked hull.
    pub hull: Entity,
}

/// The length line's part after an owned ship's length, in place of a
/// price.
const OWNED_DETAIL: &str = "your ship";

/// The hull list container, hidden while the refit gate does not hold or
/// the docked station has no hull parked for sale or owned.
#[derive(Component)]
pub(crate) struct StationHullList;

/// One list inside [`StationHullList`], its header and its rows, hidden while
/// it has no card.
#[derive(Component)]
pub(crate) struct StationHullSection;

/// The rows of one [`StationHullSection`], and the row texts they were built
/// from, so the rows respawn only when one changes.
#[derive(Component)]
pub(crate) struct StationHullRows {
    /// The listing these rows show: [`ParkedHullType::Owned`] or
    /// [`ParkedHullType::Sale`].
    terms: ParkedHullType,
    rows: Vec<HullRow>,
}

impl StationHullRows {
    /// The hulls the rows list, in list order.
    pub(crate) fn hulls(&self) -> impl Iterator<Item = Entity> + '_ {
        self.rows.iter().map(|row| row.hull)
    }
}

/// One parked hull as the list shows it.
#[derive(Clone, Debug, PartialEq)]
struct HullRow {
    hull: Entity,
    /// The hull's place among the station's owned and for-sale hulls, in
    /// parked order, which names its buttons, so a name stays put when a
    /// hull moves between the two lists.
    index: usize,
    terms: ParkedHullType,
    name: String,
    /// The length, then "your ship" for an owned ship.
    length: String,
    /// The price of a hull for sale, or `None` for an owned ship.
    price_cr: Option<u64>,
    /// What the balance lacks of the price, or `None` while it is not short.
    short_cr: Option<u64>,
    /// The action button's label: Buy or Board.
    label: &'static str,
    /// Why the action is refused, or `None` when it is allowed.
    refusal: Option<String>,
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
    let partner = resolved.partner.and_then(|(partner, offers, _)| {
        Some(HullBuyPartner {
            offers,
            credits_cr: q_credits.get(partner).ok()?.0,
        })
    });
    plan_hull_buy(price, partner, own_cr.0)
}

/// Build the hull list: a "Your ships" and a "Ships for sale" section, each a
/// header and the rows [`update_station_hull_list`] fills.
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
            for (header, terms) in [
                ("Your ships", ParkedHullType::Owned),
                ("Ships for sale", ParkedHullType::Sale),
            ] {
                list.spawn((
                    StationHullSection,
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ))
                .with_children(|section| {
                    section.spawn(themed_label(header, 13.0, UiColor::Accent));
                    section.spawn((
                        StationHullRows {
                            terms,
                            rows: Vec::new(),
                        },
                        offer_rows(),
                    ));
                });
            }
        });
}

/// Show the hull list while the refit gate holds and the docked station
/// has hulls parked, each section while it has a card, and respawn a
/// section's rows when a hull, a price or a refusal changes.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the parked hulls, the site, both balances, the icons and the list"
)]
pub(crate) fn update_station_hull_list(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_parked: Query<&ParkedHulls>,
    q_hulls: ParkedHullQuery,
    q_credits: Query<&ShipCredits>,
    icons: Res<InterfaceIcons>,
    mut q_list: Query<&mut Node, (With<StationHullList>, Without<StationHullSection>)>,
    mut q_sections: Query<&mut Node, (With<StationHullSection>, Without<StationHullList>)>,
    mut q_rows: Query<(Entity, &ChildOf, &mut StationHullRows)>,
) {
    let Ok(mut list) = q_list.single_mut() else {
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
            .and_then(|(partner, ..)| q_parked.get(partner).ok());
        if let Some(parked) = parked {
            rows = parked
                .iter()
                .filter_map(|hull| {
                    let (name, terms, design) = q_hulls.get(hull).ok()?;
                    Some((hull, name, *terms, design))
                })
                .filter(|(_, _, terms, _)| !matches!(terms, ParkedHullType::Hire { .. }))
                .enumerate()
                .map(|(index, (hull, name, terms, design))| {
                    let length = hull_length_text(&design.0);
                    let name = name.to_string();
                    match terms {
                        ParkedHullType::Sale => {
                            let price_cr = parked_price_cr(&items, design);
                            let refusal = plan_offer(&resolved, price_cr, &q_credits).err();
                            HullRow {
                                hull,
                                index,
                                terms,
                                length,
                                price_cr: Some(price_cr),
                                short_cr: match refusal {
                                    Some(HullBuyRefusalType::InsufficientCredits {
                                        need,
                                        have,
                                    }) => Some(need - u64::from(have)),
                                    _ => None,
                                },
                                label: "Buy",
                                refusal: refusal.map(|refusal| hull_refusal_text(&name, &refusal)),
                                name,
                            }
                        }
                        ParkedHullType::Owned => HullRow {
                            hull,
                            index,
                            terms,
                            name,
                            length: format!("{length}, {OWNED_DETAIL}"),
                            price_cr: None,
                            short_cr: None,
                            label: "Board",
                            refusal: None,
                        },
                        ParkedHullType::Hire { .. } => unreachable!("hires are filtered out above"),
                    }
                })
                .collect();
        }
    }
    show(&mut list, !rows.is_empty());
    for (rows_entity, section, mut shown) in &mut q_rows {
        let section_rows: Vec<HullRow> = rows
            .iter()
            .filter(|row| row.terms == shown.terms)
            .cloned()
            .collect();
        if let Ok(mut node) = q_sections.get_mut(section.parent()) {
            show(&mut node, !section_rows.is_empty());
        }
        if shown.rows == section_rows {
            continue;
        }
        commands.entity(rows_entity).despawn_children();
        commands.entity(rows_entity).with_children(|parent| {
            for row in &section_rows {
                spawn_hull_card(parent, &icons, row);
            }
        });
        shown.rows = section_rows;
    }
}

/// Show `node` when `shown` holds, else hide it, writing only on a change.
fn show(node: &mut Node, shown: bool) {
    let display = if shown { Display::Flex } else { Display::None };
    if node.display != display {
        node.display = display;
    }
}

/// One hull card: the head, the price block of a hull for sale, and the Buy
/// or Board button beside Info, named by the row's index.
fn spawn_hull_card(parent: &mut ChildSpawnerCommands, icons: &InterfaceIcons, row: &HullRow) {
    let index = row.index;
    parent.spawn(offer_cell()).with_children(|cell| {
        spawn_hull_card_head(cell, row.hull, &row.name, &row.length);
        let line = match row.price_cr {
            Some(price_cr) => {
                spawn_card_price(cell, icons, price_cr, row.short_cr);
                format!("{}  {}, {}", row.name, row.length, cr_text(price_cr))
            }
            None => format!("{}  {}", row.name, row.length),
        };
        cell.spawn(card_actions()).with_children(|actions| {
            refusable_button(
                actions,
                format!("Hull{}-{index}", row.label),
                row.label,
                row.refusal.as_deref(),
            )
            .observe(on_hull_buy_button(row.hull));
            spawn_hull_info_button(actions, format!("HullInfo-{index}"), row.hull, line);
        });
    });
}

/// Raise a [`HullBuyCommand`] for the parked hull `hull` when its Buy or
/// Board button is clicked. The press is silent: the outcome plays the one
/// cue.
fn on_hull_buy_button(hull: Entity) -> impl FnMut(On<Activate>, MessageWriter<HullBuyCommand>) {
    move |_activate, mut buys| {
        buys.write(HullBuyCommand { hull });
    }
}

/// Apply [`HullBuyCommand`]s. A buy of a hull for sale follows the
/// [`plan_hull_buy`] rule: it charges the price with `ShipBuy` credit
/// changes on both sides and queues one `ShipBought` report. A board of a
/// ship the player owns moves nothing. Either raises the [`HullBuyRequest`]
/// that swaps the hulls, and the old ship parks as the player's own with
/// its hold. The selection and any pending rebind or repair target clear,
/// since every section goes. A refusal changes nothing and shows its reason
/// on the notice line.
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
    let Some((station, ..)) = resolved.partner else {
        runtime.say(
            ActionOutcomeType::Refused,
            hull_refusal_text("", &HullBuyRefusalType::NotDocked),
        );
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
                    runtime.say(
                        ActionOutcomeType::Refused,
                        hull_refusal_text(&name, &refusal),
                    );
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
    runtime.say(ActionOutcomeType::Done, note);
}
