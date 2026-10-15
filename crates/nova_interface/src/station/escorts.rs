//! The Station pane's "Escorts for hire" list and the Hire action.
//!
//! While the ship is docked at a living station that offers Refit and hires
//! out escorts (hulls it holds listed [`ParkedHullType::Hire`], coasting
//! beside it), the pane lists them under the hulls for sale, each as a card:
//! the escort's own model turning (`hull_preview`), one line with its name,
//! length and price, and a Hire button or the reason the hire is refused. A
//! click raises an
//! [`EscortHireCommand`]; [`apply_station_escort_hire_commands`] plans it with
//! [`plan_escort_hire`], moves the credits, and raises the
//! [`EscortHireRequest`] the scenario crate makes the escort from.
//!
//! Touch this module when changing what the list shows or what Hire moves.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, GeneratedBodyId};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::{
    offer_cell, offer_rows, shipyard::hull_length_text, spawn_hull_card_head, StationRuntime,
};
use crate::{
    inventory::cr_text,
    pane::{compact_button, play_menu_select, themed_label},
    ship::{ResolvedSite, StationSite},
};

/// A request to hire the docked station's escort for hire `hull`, raised by
/// an "Escorts for hire" Hire button.
#[derive(Message, Clone, Debug)]
pub struct EscortHireCommand {
    /// The escort for hire: a [`ParkedHull`] of the docked station listed
    /// [`ParkedHullType::Hire`].
    pub hull: Entity,
}

/// The escort list container, hidden while the refit gate does not hold or
/// the docked station hires out no escort.
#[derive(Component)]
pub(crate) struct StationEscortList;

/// The rows inside [`StationEscortList`], and the row texts they were built
/// from, so the rows respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct StationEscortRows(Vec<EscortRow>);

impl StationEscortRows {
    /// The escorts for hire the rows list, in list order.
    pub(crate) fn hulls(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().map(|row| row.hull)
    }
}

/// One escort for hire as the list shows it.
#[derive(Clone, Debug, PartialEq)]
struct EscortRow {
    hull: Entity,
    name: String,
    /// The card line after the name: length and price.
    detail: String,
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
            format!(
                "hire: {name} costs {}, the ship holds {}",
                cr_text(*need),
                cr_text(*have)
            )
        }
        EscortHireRefusalType::CreditOverflow => {
            format!("hire: the station's balance after hiring {name} does not fit")
        }
    }
}

/// The hulls the escort list reads: each hull a station holds, its name, how
/// it is listed and its design.
type HireQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Name,
        &'static ParkedHullType,
        &'static ShipDesignSections,
    ),
    With<ParkedHull>,
>;

/// The fleet sides the escorts of `player` hold now.
fn held_sides(player: Entity, q_fleet: &Query<&AIFollow, With<PlayerFleetMarker>>) -> Vec<Vec3> {
    q_fleet
        .iter()
        .filter(|follow| follow.leader == player)
        .map(|follow| follow.side)
        .collect()
}

/// Plan hiring an escort at `price` for the resolved player ship.
fn plan_offer(
    resolved: &ResolvedSite<'_>,
    price: u64,
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
    plan_escort_hire(price, partner, held, own_cr)
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
    reason = "one system reading the hulls, the site, the balances, the fleet and the list"
)]
pub(crate) fn update_station_escort_list(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_parked: Query<&ParkedHulls>,
    q_hires: HireQuery,
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
        let parked = resolved
            .partner
            .and_then(|(partner, _)| q_parked.get(partner).ok());
        if let Some(parked) = parked {
            let held = held_sides(resolved.player, &q_fleet);
            rows = parked
                .iter()
                .filter_map(|hull| {
                    let (name, terms, design) = q_hires.get(hull).ok()?;
                    if !matches!(terms, ParkedHullType::Hire { .. }) {
                        return None;
                    }
                    let price = escort_price_cr(&items, &design.0);
                    Some(EscortRow {
                        hull,
                        name: name.to_string(),
                        detail: format!("{}, {}", hull_length_text(&design.0), cr_text(price)),
                        refusal: plan_offer(&resolved, price, &held, &q_credits)
                            .err()
                            .map(|refusal| escort_refusal_text(name.as_str(), &refusal)),
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
                match &row.refusal {
                    Some(refusal) => {
                        cell.spawn(themed_label(refusal, 12.0, UiColor::Danger));
                    }
                    None => {
                        cell.spawn((
                            Name::new(format!("EscortHire-{index}")),
                            compact_button(ButtonSpec::new("Hire").fit().primary()),
                        ))
                        .observe(on_escort_hire_button(row.hull));
                    }
                }
            });
        }
    });
    shown.0 = rows;
}

/// Raise an [`EscortHireCommand`] for the escort for hire `hull` when its
/// Hire button is clicked.
fn on_escort_hire_button(
    hull: Entity,
) -> impl FnMut(On<Activate>, MessageWriter<EscortHireCommand>, Option<Res<SoundBank<UiSfx>>>, Commands)
{
    move |_activate, mut hires, bank, mut commands| {
        hires.write(EscortHireCommand { hull });
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
/// On a hull that is not an escort for hire of the docked station, or a
/// station with no generated id: the list only shows a Hire button for an
/// escort for hire of a generated station.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the hulls, the site, the fleet, the ids and both balances"
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
    q_hires: Query<(&ParkedHull, &Name, &ParkedHullType, &ShipDesignSections)>,
    q_ids: Query<&GeneratedBodyId>,
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
    let station_id = q_ids
        .get(station)
        .unwrap_or_else(|_| panic!("hire: docked station {station:?} has no generated id"));
    let Ok((parked, name, &ParkedHullType::Hire { slot }, design)) = q_hires.get(command.hull)
    else {
        panic!("hire: {:?} is no escort for hire", command.hull);
    };
    assert_eq!(
        parked.0, station,
        "hire: escort {:?} is not held by the docked station {station:?}",
        command.hull
    );
    let name = name.to_string();
    let held = held_sides(resolved.player, &q_fleet);
    let price = escort_price_cr(&items, &design.0);
    let plan = match plan_offer(&resolved, price, &held, &q_credits.p0()) {
        Ok(plan) => plan,
        Err(refusal) => {
            runtime.say(escort_refusal_text(&name, &refusal));
            return;
        }
    };
    let escort = escort_id(station_id, slot);
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
        hull: name.clone(),
        price_cr: plan.price_cr,
    });
    runtime.say(format!("hired {name} for {}", cr_text(plan.price_cr)));
    commands.trigger(EscortHireRequest {
        entity: resolved.player,
        station,
        hull: command.hull,
        escort,
        side: plan.side,
    });
}
