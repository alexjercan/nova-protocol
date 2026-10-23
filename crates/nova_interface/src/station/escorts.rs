//! The Station pane's "Escorts for hire" list and the Hire action.
//!
//! While the ship is docked at a living station that offers Refit and hires
//! out escorts (hulls it holds listed [`ParkedHullType::Hire`], coasting
//! beside it), the pane lists them under the hulls for sale. The header
//! shows the fleet's slots, one box per [`FLEET_SIDES`] entry with the
//! escort that holds it. Each escort is a card: its own model turning
//! (`hull_preview`), its name, its length, its price and a Hire button,
//! greyed with the reason as its tooltip while the hire is refused; a
//! station whose side is not the player's (its civilization no Ally) refuses
//! every escort. A click raises an [`EscortHireCommand`];
//! [`apply_station_escort_hire_commands`] plans it with [`plan_escort_hire`],
//! moves the credits, and raises the [`EscortHireRequest`] the scenario
//! crate makes the escort from.
//!
//! Touch this module when changing what the list shows or what Hire moves.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, GeneratedBodyId};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{theme::UiColor, widget::ThemedBorder};

use super::{
    card_actions, offer_cell, offer_rows, shipyard::hull_length_text, spawn_card_price,
    spawn_hull_card_head, spawn_hull_info_button, StationRuntime,
};
use crate::{
    icons::InterfaceIcons,
    pane::{refusable_button, themed_label, ActionOutcomeType},
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

/// The header and rows inside [`StationEscortList`], and what they were
/// built from, so they respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct StationEscortRows {
    /// The name of the escort in each fleet slot, in [`FLEET_SIDES`] order.
    slots: Vec<Option<String>>,
    rows: Vec<EscortRow>,
}

impl StationEscortRows {
    /// The escorts for hire the rows list, in list order.
    pub(crate) fn hulls(&self) -> impl Iterator<Item = Entity> + '_ {
        self.rows.iter().map(|row| row.hull)
    }
}

/// One escort for hire as the list shows it.
#[derive(Clone, Debug, PartialEq)]
struct EscortRow {
    hull: Entity,
    name: String,
    length: String,
    price_cr: u64,
    /// What the balance lacks of the price, or `None` while it is not short.
    short_cr: Option<u64>,
    /// Why the hire is refused, or `None` when it is allowed.
    refusal: Option<String>,
}

/// Why a hire of the escort named `name` is refused; the list and the
/// handler share it.
fn escort_refusal_text(name: &str, refusal: &EscortHireRefusalType) -> String {
    match refusal {
        EscortHireRefusalType::NotDocked => "hire: dock at a station that refits".to_string(),
        EscortHireRefusalType::NoRefitService => "hire: this dock offers no refit".to_string(),
        EscortHireRefusalType::NotAllied => {
            "hire: this station's civilization is no ally of yours".to_string()
        }
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

/// Each fleet slot in [`FLEET_SIDES`] order with the name of the escort of
/// `player` that holds it.
///
/// # Panics
///
/// On an escort whose side is not one of [`FLEET_SIDES`]: invalid content.
fn held_slots(
    player: Entity,
    q_fleet: &Query<(&AIFollow, &Name), With<PlayerFleetMarker>>,
) -> Vec<Option<String>> {
    let mut slots = vec![None; FLEET_SIDES.len()];
    for (follow, name) in q_fleet.iter().filter(|(follow, _)| follow.leader == player) {
        let slot = FLEET_SIDES
            .iter()
            .position(|side| *side == follow.side)
            .unwrap_or_else(|| {
                panic!(
                    "fleet: escort {name} flies side {}, not one of FLEET_SIDES",
                    follow.side
                )
            });
        slots[slot] = Some(name.to_string());
    }
    slots
}

/// The sides of the filled `slots`, as [`plan_escort_hire`] reads them.
fn held_sides(slots: &[Option<String>]) -> Vec<Vec3> {
    FLEET_SIDES
        .iter()
        .zip(slots)
        .filter(|(_, name)| name.is_some())
        .map(|(side, _)| *side)
        .collect()
}

/// Plan hiring an escort at `price` for the resolved player ship. The
/// partner is allied while it flies the player's side: an Ally
/// civilization's station or one the player owns.
fn plan_offer(
    resolved: &ResolvedSite<'_>,
    price: u64,
    held: &[Vec3],
    q_credits: &Query<&ShipCredits>,
    q_sides: &Query<&Allegiance>,
) -> Result<EscortHirePlan, EscortHireRefusalType> {
    let own_cr = q_credits
        .get(resolved.player)
        .expect("the player ship carries ShipCredits")
        .0;
    let partner = resolved.partner.and_then(|(partner, offers, _)| {
        Some(EscortHirePartner {
            offers,
            allied: q_sides.get(partner).ok() == Some(&Allegiance::Player),
            credits_cr: q_credits.get(partner).ok()?.0,
        })
    });
    plan_escort_hire(price, partner, held, own_cr)
}

/// Build the escort list under the hull list: the header and rows
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
            list.spawn((
                StationEscortRows::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
            ));
        });
}

/// `FLEET  [STBD name] [PORT --]`: one bordered box per [`FLEET_SIDES`]
/// entry.
fn spawn_fleet_row(list: &mut ChildSpawnerCommands, slots: &[Option<String>]) {
    list.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        column_gap: px(8),
        row_gap: px(4),
        ..default()
    })
    .with_children(|row| {
        row.spawn(themed_label("FLEET", 11.0, UiColor::Label));
        for (side, name) in FLEET_SIDES.iter().zip(slots) {
            let border = if name.is_some() {
                ThemedBorder::new(UiColor::Accent)
            } else {
                ThemedBorder::alpha(UiColor::Secondary, 0.35)
            };
            row.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Baseline,
                    column_gap: px(8),
                    padding: UiRect::axes(px(8), px(3)),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(Color::NONE),
                border,
            ))
            .with_children(|slot| {
                slot.spawn(themed_label(fleet_side_label(*side), 11.0, UiColor::Label));
                match name {
                    Some(name) => slot.spawn(themed_label(name, 12.0, UiColor::Primary)),
                    None => slot.spawn(themed_label("--", 12.0, UiColor::Label)),
                };
            });
        }
    });
}

/// Show the escort list while the refit gate holds and the docked station
/// hires out escorts, and respawn its rows when an offer, a price or a
/// refusal changes.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the hulls, the site, the balances, the sides, the fleet, the icons and the list"
)]
pub(crate) fn update_station_escort_list(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_parked: Query<&ParkedHulls>,
    q_hires: HireQuery,
    q_credits: Query<&ShipCredits>,
    q_sides: Query<&Allegiance>,
    q_fleet: Query<(&AIFollow, &Name), With<PlayerFleetMarker>>,
    icons: Res<InterfaceIcons>,
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
    let mut slots = Vec::new();
    let mut rows = Vec::new();
    if let (Some(resolved), Some(items)) = (resolved, items) {
        slots = held_slots(resolved.player, &q_fleet);
        let parked = resolved
            .partner
            .and_then(|(partner, ..)| q_parked.get(partner).ok());
        if let Some(parked) = parked {
            let held = held_sides(&slots);
            rows = parked
                .iter()
                .filter_map(|hull| {
                    let (name, terms, design) = q_hires.get(hull).ok()?;
                    if !matches!(terms, ParkedHullType::Hire { .. }) {
                        return None;
                    }
                    let price_cr = escort_price_cr(&items, &design.0);
                    let refusal =
                        plan_offer(&resolved, price_cr, &held, &q_credits, &q_sides).err();
                    Some(EscortRow {
                        hull,
                        name: name.to_string(),
                        length: hull_length_text(&design.0),
                        price_cr,
                        short_cr: match refusal {
                            Some(EscortHireRefusalType::InsufficientCredits { need, have }) => {
                                Some(need - u64::from(have))
                            }
                            _ => None,
                        },
                        refusal: refusal
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
    if shown.slots == slots && shown.rows == rows {
        return;
    }
    commands.entity(rows_entity).despawn_children();
    commands.entity(rows_entity).with_children(|parent| {
        parent
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                column_gap: px(24),
                row_gap: px(4),
                ..default()
            })
            .with_children(|header| {
                header.spawn(themed_label("Escorts for hire", 13.0, UiColor::Accent));
                spawn_fleet_row(header, &slots);
            });
        parent.spawn(offer_rows()).with_children(|cards| {
            for (index, row) in rows.iter().enumerate() {
                cards.spawn(offer_cell()).with_children(|cell| {
                    spawn_hull_card_head(cell, row.hull, &row.name, &row.length);
                    spawn_card_price(cell, &icons, row.price_cr, row.short_cr);
                    cell.spawn(card_actions()).with_children(|actions| {
                        refusable_button(
                            actions,
                            format!("EscortHire-{index}"),
                            "Hire",
                            row.refusal.as_deref(),
                        )
                        .observe(on_escort_hire_button(row.hull));
                        spawn_hull_info_button(
                            actions,
                            format!("EscortInfo-{index}"),
                            row.hull,
                            format!("{}  {}, {}", row.name, row.length, cr_text(row.price_cr)),
                        );
                    });
                });
            }
        });
    });
    shown.slots = slots;
    shown.rows = rows;
}

/// Raise an [`EscortHireCommand`] for the escort for hire `hull` when its
/// Hire button is clicked. The press is silent: the outcome plays the one
/// cue.
fn on_escort_hire_button(
    hull: Entity,
) -> impl FnMut(On<Activate>, MessageWriter<EscortHireCommand>) {
    move |_activate, mut hires| {
        hires.write(EscortHireCommand { hull });
    }
}

/// Apply [`EscortHireCommand`]s by the [`plan_escort_hire`] rule: charge the
/// price with `EscortHire` credit changes on both sides, queue one
/// `EscortHired` report, and raise the [`EscortHireRequest`] that spawns the
/// escort. A refusal changes nothing and shows its reason on the notice
/// line.
///
/// # Panics
///
/// On a hull that is not an escort for hire of the docked station, or a
/// station with no generated id: the list only shows a Hire button for an
/// escort for hire of a generated station. On an escort whose side is not
/// one of [`FLEET_SIDES`].
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the hulls, the site, the fleet, the ids, the sides and both balances"
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
    q_sides: Query<&Allegiance>,
    q_fleet: Query<(&AIFollow, &Name), With<PlayerFleetMarker>>,
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
    let Some((station, ..)) = resolved.partner else {
        runtime.say(
            ActionOutcomeType::Refused,
            escort_refusal_text("", &EscortHireRefusalType::NotDocked),
        );
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
    let held = held_sides(&held_slots(resolved.player, &q_fleet));
    let price = escort_price_cr(&items, &design.0);
    let plan = match plan_offer(&resolved, price, &held, &q_credits.p0(), &q_sides) {
        Ok(plan) => plan,
        Err(refusal) => {
            runtime.say(
                ActionOutcomeType::Refused,
                escort_refusal_text(&name, &refusal),
            );
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
    runtime.say(
        ActionOutcomeType::Done,
        format!("hired {name} for {}", cr_text(plan.price_cr)),
    );
    commands.trigger(EscortHireRequest {
        entity: resolved.player,
        station,
        hull: command.hull,
        escort,
        side: plan.side,
    });
}
