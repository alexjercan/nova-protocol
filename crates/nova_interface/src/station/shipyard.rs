//! The Station pane's "Ships for sale" list and the Buy action.
//!
//! While the ship is docked at a living station that offers Refit and has
//! hulls parked at its ports ([`ParkedHulls`]), the pane lists every one, a
//! hull for sale and a hull traded in for rebuy alike: its name, its length
//! and section count, how many sections of each kind it carries, the price
//! and the trade-in, and a Buy button or the reason the buy is refused. A
//! click raises a [`HullBuyCommand`]; [`apply_station_hull_buy_commands`]
//! plans it with [`plan_hull_buy`], moves the credits both ways, and raises
//! the [`HullBuyRequest`] the scenario crate swaps the hulls from. The old
//! hold stays in the old hull.
//!
//! Touch this module when changing what the list shows or what Buy moves.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, Meters};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::{offer_cell, offer_rows, StationRuntime};
use crate::{
    pane::{compact_button, play_menu_select, themed_label},
    ship::{code_prefix, ResolvedSite, ShipRuntime, StationSite},
};

/// A request to buy the hull parked at the docked station, raised by a
/// "Ships for sale" Buy button.
#[derive(Message, Clone, Debug)]
pub struct HullBuyCommand {
    /// The parked hull.
    pub hull: Entity,
}

/// The hull list container, hidden while the refit gate does not hold or
/// the docked station sells no hull.
#[derive(Component)]
pub(crate) struct StationHullList;

/// The rows inside [`StationHullList`], and the row texts they were built from,
/// so the rows respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct StationHullRows(Vec<HullRow>);

/// One parked hull as the list shows it.
#[derive(Clone, Debug, PartialEq)]
struct HullRow {
    hull: Entity,
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
pub(crate) fn hull_size_text(sections: &[DesignSection]) -> String {
    let (low, high) = sections.iter().fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(low, high), section| (low.min(section.position), high.max(section.position)),
    );
    let span = (high - low).max_element().max(0.0) + 1.0;
    format!(
        "{:.0} m long, {} sections",
        Meters::from_engine(span).0,
        sections.len()
    )
}

/// How many sections of each kind the hull carries, by section code prefix:
/// `4 HULL, 1 CTL, 2 THR, 1 DOCK`.
pub(crate) fn hull_kinds_text(sections: &[DesignSection]) -> String {
    SUMMARY_ORDER
        .iter()
        .filter_map(|&class| {
            let count = sections
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

/// A parked hull's price: its recipes at ask for sale, the trade-in it was
/// paid for rebuy.
fn parked_price_cr(items: &GameItems, terms: ParkedHullType, design: &ShipDesignSections) -> u64 {
    match terms {
        ParkedHullType::Sale => hull_price_cr(
            items,
            design.0.iter().map(|section| &section.config.base.build),
        ),
        ParkedHullType::Rebuy { price_cr } => u64::from(price_cr),
    }
}

/// Plan buying a hull at `price` for the resolved player ship.
fn plan_offer(
    resolved: &ResolvedSite<'_>,
    price: u64,
    trade_in_cr: u64,
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
    plan_hull_buy(price, trade_in_cr, partner, own_cr.0)
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
    reason = "one system reading the parked hulls, the site, both balances, the health and the \
              list"
)]
pub(crate) fn update_station_hull_list(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_parked: Query<&ParkedHulls>,
    q_hulls: ParkedHullQuery,
    q_credits: Query<&ShipCredits>,
    q_health: Query<&Health, With<SectionMarker>>,
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
            let trade_in_cr = live_trade_in_cr(&items, &resolved, &q_health);
            rows = parked
                .iter()
                .filter_map(|hull| {
                    let (name, terms, design) = q_hulls.get(hull).ok()?;
                    let price_cr = parked_price_cr(&items, *terms, design);
                    let planned = plan_offer(&resolved, price_cr, trade_in_cr, &q_credits);
                    let listing = match terms {
                        ParkedHullType::Sale => "",
                        ParkedHullType::Rebuy { .. } => " (rebuy)",
                    };
                    Some(HullRow {
                        hull,
                        name: format!("{name}{listing}"),
                        size: hull_size_text(&design.0),
                        kinds: hull_kinds_text(&design.0),
                        price: format!("{price_cr} cr, trade-in {trade_in_cr} cr"),
                        refusal: planned
                            .err()
                            .map(|refusal| hull_refusal_text(name.as_str(), &refusal)),
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
                            Name::new(format!("HullBuy-{index}")),
                            compact_button(ButtonSpec::new("Buy").fit().primary()),
                        ))
                        .observe(on_hull_buy_button(row.hull));
                    }
                }
            });
        }
    });
    shown.0 = rows;
}

/// Raise a [`HullBuyCommand`] for the parked hull `hull` when its Buy
/// button is clicked.
fn on_hull_buy_button(
    hull: Entity,
) -> impl FnMut(On<Activate>, MessageWriter<HullBuyCommand>, Option<Res<SoundBank<UiSfx>>>, Commands)
{
    move |_activate, mut buys, bank, mut commands| {
        buys.write(HullBuyCommand { hull });
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Apply [`HullBuyCommand`]s by the [`plan_hull_buy`] rule: charge the
/// price and pay the trade-in with `ShipBuy` credit changes on both sides,
/// queue one `ShipBought` report, and raise the [`HullBuyRequest`] that
/// swaps the hulls. The old hold stays in the old hull. The selection and
/// any pending rebind or repair target clear, since every section goes. A
/// refusal changes nothing and shows its reason on the note line.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the parked hulls, the site, the health and both balances"
)]
pub(crate) fn apply_station_hull_buy_commands(
    mut messages: MessageReader<HullBuyCommand>,
    mut runtime: ResMut<StationRuntime>,
    mut ship: ResMut<ShipRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_hulls: Query<(&ParkedHull, &Name, &ParkedHullType, &ShipDesignSections)>,
    q_health: Query<&Health, With<SectionMarker>>,
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
    let price_cr = parked_price_cr(&items, *terms, design);
    let trade_in_cr = live_trade_in_cr(&items, &resolved, &q_health);
    let planned = plan_offer(&resolved, price_cr, trade_in_cr, &q_credits.p0());
    let plan = match planned {
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
        panic!("buy: the player ship and station {station:?} each carry credits and reports");
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
        trade_in_cr: plan.trade_in_cr,
    });
    commands.trigger(HullBuyRequest {
        entity: resolved.player,
        station,
        hull: command.hull,
        trade_in_cr: plan.trade_in_cr,
    });
    ship.selected = None;
    ship.rebinding = None;
    ship.repair_target = None;
    ship.held = None;
    ship.reselect = None;
    runtime.say(format!(
        "bought {name} for {} cr, {} cr for the old hull",
        plan.price_cr, plan.trade_in_cr
    ));
}
