//! The Station pane's "Escorts for hire" list and the Hire action.
//!
//! While the ship is docked at a living station that offers Refit and hires
//! out escorts ([`HullOfferType::Hire`] offers in its [`HullOffers`]), the
//! pane lists them under the hulls for sale: the name, the length and
//! section count, how many sections of each kind, the price, and a Hire
//! button or the reason the hire is refused. A click raises an
//! [`EscortHireCommand`]; [`apply_station_escort_hire_commands`] plans it with
//! [`plan_escort_hire`], moves the credits, and raises the
//! [`EscortHireRequest`] the scenario crate spawns the escort from.
//!
//! Touch this module when changing what the list shows or what Hire moves.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, GeneratedBodyId};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::{
    offer_cell, offer_rows,
    shipyard::{hull_kinds_text, hull_size_text},
    StationRuntime,
};
use crate::{
    pane::{compact_button, play_menu_select, themed_label},
    ship::{ResolvedSite, StationSite},
};

/// A request to hire escort `offer` of the docked station's [`HullOffers`],
/// raised by an "Escorts for hire" Hire button.
#[derive(Message, Clone, Debug)]
pub struct EscortHireCommand {
    /// The offer's index in the station's [`HullOffers`].
    pub offer: usize,
}

/// The escort list container, hidden while the refit gate does not hold or
/// the docked station hires out no escort.
#[derive(Component)]
pub(crate) struct StationEscortList;

/// The rows inside [`StationEscortList`], and the row texts they were built
/// from, so the rows respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct StationEscortRows(Vec<EscortRow>);

/// One escort for hire as the list shows it.
#[derive(Clone, Debug, PartialEq)]
struct EscortRow {
    offer: usize,
    name: String,
    size: String,
    kinds: String,
    price: String,
    /// The refusal line, or `None` when the hire is allowed.
    refusal: Option<String>,
}

/// Why a hire of the escort named `name` is refused; the list and the
/// handler share it.
fn escort_refusal_text(name: &str, refusal: &EscortHireRefusalType) -> String {
    match refusal {
        EscortHireRefusalType::NotDocked => "hire: dock at a station that refits".to_string(),
        EscortHireRefusalType::NoRefitService => "hire: this dock offers no refit".to_string(),
        EscortHireRefusalType::FleetFull { cap } => {
            format!("hire: the fleet holds its {cap} escorts")
        }
        EscortHireRefusalType::InsufficientCredits { need, have } => {
            format!("hire: {name} costs {need} cr, the ship holds {have} cr")
        }
        EscortHireRefusalType::CreditOverflow => {
            format!("hire: the station's balance after hiring {name} does not fit")
        }
    }
}

/// The fleet sides the escorts of `player` hold now.
fn held_sides(player: Entity, q_fleet: &Query<&AIFollow, With<PlayerFleetMarker>>) -> Vec<Vec3> {
    q_fleet
        .iter()
        .filter(|follow| follow.leader == player)
        .map(|follow| follow.side)
        .collect()
}

/// Plan hiring `offer` for the resolved player ship.
fn plan_offer(
    items: &GameItems,
    resolved: &ResolvedSite<'_>,
    offer: &HullOffer,
    held: &[Vec3],
    q_credits: &Query<&ShipCredits>,
) -> Result<EscortHirePlan, EscortHireRefusalType> {
    let own_cr = q_credits
        .get(resolved.player)
        .expect("the player ship carries ShipCredits")
        .0;
    let partner = resolved.partner.and_then(|(partner, offers)| {
        Some(EscortHirePartner {
            offers,
            credits_cr: q_credits.get(partner).ok()?.0,
        })
    });
    plan_escort_hire(items, offer, partner, held, own_cr)
}

/// Build the escort list under the hull list: a header and the rows
/// [`update_station_escort_list`] fills.
pub(crate) fn spawn_station_escort_list(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            StationEscortList,
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
            list.spawn(themed_label("Escorts for hire", 13.0, UiColor::Accent));
            list.spawn((StationEscortRows::default(), offer_rows()));
        });
}

/// Show the escort list while the refit gate holds and the docked station
/// hires out escorts, and respawn its rows when an offer, a price or a
/// refusal changes.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the offers, the site, the balances, the fleet and the list"
)]
pub(crate) fn update_station_escort_list(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_offers: Query<&HullOffers>,
    q_credits: Query<&ShipCredits>,
    q_fleet: Query<&AIFollow, With<PlayerFleetMarker>>,
    mut q_list: Query<&mut Node, With<StationEscortList>>,
    mut q_rows: Query<(Entity, &mut StationEscortRows)>,
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
        if let Some(offers) = offers {
            let held = held_sides(resolved.player, &q_fleet);
            rows = offers
                .0
                .iter()
                .enumerate()
                .filter(|(_, offer)| matches!(offer.terms, HullOfferType::Hire { .. }))
                .map(|(index, offer)| EscortRow {
                    offer: index,
                    name: offer.name.clone(),
                    size: hull_size_text(offer),
                    kinds: hull_kinds_text(offer),
                    price: format!("{} cr", escort_price_cr(&items, offer)),
                    refusal: plan_offer(&items, &resolved, offer, &held, &q_credits)
                        .err()
                        .map(|refusal| escort_refusal_text(&offer.name, &refusal)),
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
                            Name::new(format!("EscortHire-{}", row.offer)),
                            compact_button(ButtonSpec::new("Hire").fit().primary()),
                        ))
                        .observe(on_escort_hire_button(row.offer));
                    }
                }
            });
        }
    });
    shown.0 = rows;
}

/// Raise an [`EscortHireCommand`] for offer `offer` when its Hire button is
/// clicked.
fn on_escort_hire_button(
    offer: usize,
) -> impl FnMut(On<Activate>, MessageWriter<EscortHireCommand>, Option<Res<SoundBank<UiSfx>>>, Commands)
{
    move |_activate, mut hires, bank, mut commands| {
        hires.write(EscortHireCommand { offer });
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Apply [`EscortHireCommand`]s by the [`plan_escort_hire`] rule: charge the
/// price with `EscortHire` credit changes on both sides, queue one
/// `EscortHired` report, and raise the [`EscortHireRequest`] that spawns the
/// escort. A refusal changes nothing and shows its reason on the note line.
///
/// # Panics
///
/// On a docked station that hires out nothing, has no such offer, offers it
/// for sale, or carries no generated id: the list only shows a Hire button
/// for a hire offer of a generated station's catalogue.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the offers, the site, the fleet, the ids and both balances"
)]
#[expect(
    clippy::type_complexity,
    reason = "both balances and report queues in one query"
)]
pub(crate) fn apply_station_escort_hire_commands(
    mut messages: MessageReader<EscortHireCommand>,
    mut runtime: ResMut<StationRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_offers: Query<(&HullOffers, &GeneratedBodyId)>,
    q_fleet: Query<&AIFollow, With<PlayerFleetMarker>>,
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
    // One hire per frame: the second would plan against a fleet whose new
    // escort has not spawned yet and take the same side.
    let Some(command) = messages.read().last().cloned() else {
        return;
    };
    let Some((station, _)) = resolved.partner else {
        runtime.say(escort_refusal_text("", &EscortHireRefusalType::NotDocked));
        return;
    };
    let (offers, station_id) = q_offers.get(station).unwrap_or_else(|_| {
        panic!("hire: docked station {station:?} has no catalogue or no generated id")
    });
    let offer = offers
        .0
        .get(command.offer)
        .unwrap_or_else(|| panic!("hire: station {station:?} has no offer {}", command.offer));
    let held = held_sides(resolved.player, &q_fleet);
    let plan = match plan_offer(&items, &resolved, offer, &held, &q_credits.p0()) {
        Ok(plan) => plan,
        Err(refusal) => {
            runtime.say(escort_refusal_text(&offer.name, &refusal));
            return;
        }
    };
    let escort = escort_id(station_id, plan.slot);
    let mut q_credits = q_credits.p1();
    let Ok([(mut own_cr, mut own_reports), (mut station_cr, mut station_reports)]) =
        q_credits.get_many_mut([resolved.player, station])
    else {
        panic!("hire: the player ship and station {station:?} each carry credits");
    };
    own_reports.change_credits(&mut own_cr, plan.own_cr, CreditChangeCauseType::EscortHire);
    station_reports.change_credits(
        &mut station_cr,
        plan.station_cr,
        CreditChangeCauseType::EscortHire,
    );
    own_reports.0.push(ShipInventoryReport::EscortHired {
        escort: escort.to_string(),
        hull: offer.name.clone(),
        price_cr: plan.price_cr,
    });
    runtime.say(format!("hired {} for {} cr", offer.name, plan.price_cr));
    commands.trigger(EscortHireRequest {
        entity: resolved.player,
        station,
        offer: command.offer,
        escort,
        side: plan.side,
    });
}
