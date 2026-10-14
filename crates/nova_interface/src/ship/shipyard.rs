//! The Ship pane's "Ships for sale" list and the Buy action.
//!
//! While the ship is docked at a living station that offers Refit and sells
//! hulls ([`HullOffers`]), the panel lists every hull under the add list:
//! its name, its length and section count, how many sections of each kind it
//! carries, the price and the trade-in, and a Buy button or the reason the
//! buy is refused. A click raises a [`HullBuyCommand`];
//! [`apply_ship_hull_buy_commands`] plans it with [`plan_hull_buy`], moves the
//! hold into the station's storage, moves the credits both ways, and raises
//! the [`HullBuyRequest`] the scenario crate replaces the hull from.
//!
//! Touch this module when changing what the list shows or what Buy moves.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, Meters};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::{
    rebuild::{ResolvedSite, StationSite},
    refit::{live_bindings, SectionBindingRow},
    scene::ShipRuntime,
    sections::code_prefix,
};
use crate::pane::{compact_button, play_menu_select, themed_label};

/// A request to buy hull `offer` of the docked station's [`HullOffers`],
/// raised by a "Ships for sale" Buy button.
#[derive(Message, Clone, Debug)]
pub struct HullBuyCommand {
    /// The offer's index in the station's [`HullOffers`].
    pub offer: usize,
}

/// The hull list container, hidden while the refit gate does not hold or
/// the docked station sells no hull.
#[derive(Component)]
pub(crate) struct ShipHullList;

/// The rows inside [`ShipHullList`], and the row texts they were built from,
/// so the rows respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct ShipHullRows(Vec<HullRow>);

/// One offered hull as the list shows it.
#[derive(Clone, Debug, PartialEq)]
struct HullRow {
    offer: usize,
    name: String,
    size: String,
    kinds: String,
    price: String,
    /// The refusal line, or `None` when the buy is allowed.
    refusal: Option<String>,
}

/// Every section kind in the order the kind summary lists them.
const SUMMARY_ORDER: [SectionClass; 9] = [
    SectionClass::Hull,
    SectionClass::Controller,
    SectionClass::Thruster,
    SectionClass::Turret,
    SectionClass::Torpedo,
    SectionClass::Railgun,
    SectionClass::Mining,
    SectionClass::CargoIntake,
    SectionClass::Docking,
];

/// The hull's length and section count: the span of its section centres
/// along its longest axis, plus one cell, in metres.
fn hull_size_text(offer: &HullOffer) -> String {
    let (low, high) = offer.sections.iter().fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(low, high), section| (low.min(section.position), high.max(section.position)),
    );
    let span = (high - low).max_element().max(0.0) + 1.0;
    format!(
        "{:.0} m long, {} sections",
        Meters::from_engine(span).0,
        offer.sections.len()
    )
}

/// How many sections of each kind the hull carries, by section code prefix:
/// `4 HULL, 1 CTL, 2 THR, 1 DOCK`.
fn hull_kinds_text(offer: &HullOffer) -> String {
    SUMMARY_ORDER
        .iter()
        .filter_map(|&class| {
            let count = offer
                .sections
                .iter()
                .filter(|section| section.config.kind.class() == class)
                .count();
            (count > 0).then(|| format!("{count} {}", code_prefix(class)))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Why a buy of the hull named `name` is refused; the list and the handler
/// share it.
fn hull_refusal_text(name: &str, refusal: &HullBuyRefusalType) -> String {
    match refusal {
        HullBuyRefusalType::NotDocked => "buy: dock at a station that refits".to_string(),
        HullBuyRefusalType::NoRefitService => "buy: this dock offers no refit".to_string(),
        HullBuyRefusalType::NoFreeKey => {
            format!("buy: {name} carries more mining emitters than there are mining keys")
        }
        HullBuyRefusalType::StorageFull { need_g, free_g } => format!(
            "buy: the hold's {} does not fit the station's free {}",
            kg_text(u64::from(*need_g)),
            kg_text(u64::from(*free_g))
        ),
        HullBuyRefusalType::InsufficientCredits { need, have } => {
            format!("buy: {name} costs {need} cr, the ship and the trade-in make {have} cr")
        }
        HullBuyRefusalType::StationShortOfCredits { need, have } => {
            format!("buy: the station owes {need} cr for the trade-in and holds {have} cr")
        }
        HullBuyRefusalType::CreditOverflow => {
            format!("buy: a balance after buying {name} does not fit")
        }
    }
}

/// The player ship's trade-in value: each live section's design recipe, see
/// [`hull_trade_in_cr`], at its health share.
///
/// # Panics
///
/// On a live section the design does not list or that carries no `Health`:
/// every spawned design section carries both.
fn live_trade_in_cr(
    items: &GameItems,
    resolved: &ResolvedSite<'_>,
    q_health: &Query<&Health, With<SectionMarker>>,
) -> u64 {
    hull_trade_in_cr(
        items,
        resolved.live.iter().map(|live| {
            let section = resolved
                .design
                .get(&live.id)
                .unwrap_or_else(|| panic!("buy: live section '{}' is not in the design", live.id));
            let health = q_health
                .get(live.entity)
                .unwrap_or_else(|_| panic!("buy: live section '{}' has no Health", live.id));
            (&section.config.base.build, health.current / health.max)
        }),
    )
}

/// Plan buying offer `offer` for the resolved player ship.
fn plan_offer(
    items: &GameItems,
    resolved: &ResolvedSite<'_>,
    offer: &HullOffer,
    trade_in_cr: u64,
    bound: &[(String, SectionClass, Vec<nova_input::prelude::InputSource>)],
    q_holds: &Query<(&ShipInventory, &ShipCredits, Option<&StationStorage>)>,
) -> Result<HullBuyPlan, HullBuyRefusalType> {
    let (hold, own_cr, _) = q_holds
        .get(resolved.player)
        .expect("the player ship carries a ShipInventory and ShipCredits");
    let partner = resolved.partner.and_then(|(partner, offers)| {
        let (_, credits, storage) = q_holds.get(partner).ok()?;
        Some(HullBuyPartner {
            offers,
            storage: storage?,
            credits_cr: credits.0,
        })
    });
    plan_hull_buy(
        items,
        offer,
        trade_in_cr,
        bound
            .iter()
            .map(|(id, class, sources)| (id.as_str(), *class, sources.as_slice())),
        partner,
        hold,
        own_cr.0,
    )
}

/// Build the hull list under the add list: a header and the rows
/// [`update_ship_hull_list`] fills.
pub(crate) fn spawn_ship_hull_list(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            ShipHullList,
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
            list.spawn((
                ShipHullRows::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ));
        });
}

/// Show the hull list while the refit gate holds and the docked station
/// sells hulls, and respawn its rows when an offer, a price or a refusal
/// changes.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the offers, the site, both holds, the bindings and the list"
)]
pub(crate) fn update_ship_hull_list(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_offers: Query<&HullOffers>,
    q_holds: Query<(&ShipInventory, &ShipCredits, Option<&StationStorage>)>,
    q_bindings: Query<SectionBindingRow, With<SectionMarker>>,
    q_health: Query<&Health, With<SectionMarker>>,
    mut q_list: Query<&mut Node, With<ShipHullList>>,
    mut q_rows: Query<(Entity, &mut ShipHullRows)>,
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
        let offers = resolved
            .partner
            .and_then(|(partner, _)| q_offers.get(partner).ok());
        if let Some(offers) = offers.filter(|offers| !offers.0.is_empty()) {
            let trade_in_cr = live_trade_in_cr(&items, &resolved, &q_health);
            let bound = live_bindings(&resolved, &q_bindings);
            rows = offers
                .0
                .iter()
                .enumerate()
                .map(|(index, offer)| {
                    let planned =
                        plan_offer(&items, &resolved, offer, trade_in_cr, &bound, &q_holds);
                    let price_cr = hull_price_cr(
                        &items,
                        offer
                            .sections
                            .iter()
                            .map(|section| &section.config.base.build),
                    );
                    HullRow {
                        offer: index,
                        name: offer.name.clone(),
                        size: hull_size_text(offer),
                        kinds: hull_kinds_text(offer),
                        price: format!("{price_cr} cr, trade-in {trade_in_cr} cr"),
                        refusal: planned
                            .err()
                            .map(|refusal| hull_refusal_text(&offer.name, &refusal)),
                    }
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
        for row in &rows {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|cell| {
                    cell.spawn(themed_label(&row.name, 13.0, UiColor::Primary));
                    cell.spawn(themed_label(&row.size, 12.0, UiColor::Body));
                    cell.spawn(themed_label(&row.kinds, 12.0, UiColor::Body));
                    cell.spawn(themed_label(&row.price, 12.0, UiColor::Body));
                    match &row.refusal {
                        Some(refusal) => {
                            cell.spawn(themed_label(refusal, 12.0, UiColor::Danger));
                        }
                        None => {
                            cell.spawn((
                                Name::new(format!("ShipBuy-{}", row.offer)),
                                compact_button(ButtonSpec::new("Buy").fit().primary()),
                            ))
                            .observe(on_ship_buy_button(row.offer));
                        }
                    }
                });
        }
    });
    shown.0 = rows;
}

/// Raise a [`HullBuyCommand`] for offer `offer` when its Buy button is
/// clicked.
fn on_ship_buy_button(
    offer: usize,
) -> impl FnMut(On<Activate>, MessageWriter<HullBuyCommand>, Option<Res<SoundBank<UiSfx>>>, Commands)
{
    move |_activate, mut buys, bank, mut commands| {
        buys.write(HullBuyCommand { offer });
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Apply [`HullBuyCommand`]s by the [`plan_hull_buy`] rule: move every stack
/// of the hold into the station's storage with one `Stored` report each,
/// charge the price and pay the trade-in with `ShipBuy` credit changes on
/// both sides, queue one `ShipBought` report, and raise the
/// [`HullBuyRequest`] that replaces the hull. The selection and any pending
/// rebind or repair target clear, since every section goes. A refusal
/// changes nothing and shows its reason on the note line.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the offers, the site, the bindings, the health and both holds"
)]
#[expect(
    clippy::type_complexity,
    reason = "both holds, balances, report queues and the station storage in one query"
)]
pub(crate) fn apply_ship_hull_buy_commands(
    mut messages: MessageReader<HullBuyCommand>,
    mut runtime: ResMut<ShipRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_offers: Query<&HullOffers>,
    q_bindings: Query<SectionBindingRow, With<SectionMarker>>,
    q_health: Query<&Health, With<SectionMarker>>,
    mut q_holds: ParamSet<(
        Query<(&ShipInventory, &ShipCredits, Option<&StationStorage>)>,
        Query<(
            &mut ShipInventory,
            &mut ShipCredits,
            &mut ShipInventoryReports,
            Option<&mut StationStorage>,
        )>,
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
        runtime.note = Some((hull_refusal_text("", &HullBuyRefusalType::NotDocked), 2.5));
        return;
    };
    let offers = q_offers
        .get(station)
        .unwrap_or_else(|_| panic!("buy: docked station {station:?} sells no hull"));
    let offer = offers
        .0
        .get(command.offer)
        .unwrap_or_else(|| panic!("buy: station {station:?} has no offer {}", command.offer));
    let trade_in_cr = live_trade_in_cr(&items, &resolved, &q_health);
    let bound = live_bindings(&resolved, &q_bindings);
    let planned = plan_offer(&items, &resolved, offer, trade_in_cr, &bound, &q_holds.p0());
    let plan = match planned {
        Ok(plan) => plan,
        Err(refusal) => {
            runtime.note = Some((hull_refusal_text(&offer.name, &refusal), 2.5));
            return;
        }
    };
    let mut q_holds = q_holds.p1();
    let Ok(
        [(mut own, mut own_cr, mut own_reports, _), (_, mut station_cr, mut station_reports, storage)],
    ) = q_holds.get_many_mut([resolved.player, station])
    else {
        panic!("buy: the player ship and station {station:?} each carry a hold");
    };
    let mut storage = storage.expect("plan_hull_buy refuses a partner without station storage");
    for (item, count) in &plan.stored {
        own.remove(item, *count);
        storage.add(&items, item, *count);
        own_reports.0.push(ShipInventoryReport::Stored {
            station,
            item: item.clone(),
            count: *count,
        });
    }
    own_reports.change_credits(&mut own_cr, plan.own_cr, CreditChangeCauseType::ShipBuy);
    station_reports.change_credits(
        &mut station_cr,
        plan.station_cr,
        CreditChangeCauseType::ShipBuy,
    );
    own_reports.0.push(ShipInventoryReport::ShipBought {
        hull: offer.name.clone(),
        price_cr: plan.price_cr,
        trade_in_cr: plan.trade_in_cr,
    });
    commands.trigger(HullBuyRequest {
        entity: resolved.player,
        station,
        offer: command.offer,
        bindings: plan.bindings.clone(),
    });
    runtime.selected = None;
    runtime.rebinding = None;
    runtime.repair_target = None;
    runtime.add_point = None;
    runtime.reselect = None;
    runtime.note = Some((
        format!(
            "bought {} for {} cr, {} cr for the old hull",
            offer.name, plan.price_cr, plan.trade_in_cr
        ),
        2.5,
    ));
}
