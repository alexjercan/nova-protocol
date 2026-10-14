//! The Ship pane's "This station" block and the station Buy action.
//!
//! While the player ship is docked at a living station a civilization keeps
//! (generated with the world, so it wears a [`GeneratedBodyId`]), the panel
//! shows the station's name, its price ([`station_price_cr`]) and a Buy
//! button or the reason the buy is refused. A click raises a
//! [`StationBuyCommand`]; [`apply_ship_station_buy_commands`] plans it with
//! [`plan_station_buy`], charges the price as a `StationBuy` credit change,
//! queues one `StationBought` report and marks the station
//! [`OwnedStationMarker`]. Docked at a station the player owns, the block
//! says so instead. A ship, a dead station and a station a scenario placed by
//! hand show no block: no civilization sells them.
//!
//! Touch this module when changing what the block shows or what Buy does.

use bevy::{ecs::system::SystemParam, prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, EntityId, GeneratedBodyId};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::scene::ShipRuntime;
use crate::pane::{compact_button, play_menu_select, themed_label};

/// The name of the station Buy button.
pub const STATION_BUY_BUTTON: &str = "StationBuy";

/// A request to buy the docked station, raised by the "This station" Buy
/// button.
#[derive(Message, Clone, Debug)]
pub struct StationBuyCommand;

/// The block's container, hidden while the player is not docked at a station
/// a civilization keeps or the player owns.
#[derive(Component)]
pub(crate) struct ShipStationList;

/// The rows inside [`ShipStationList`], and the row they were built from, so
/// the rows respawn only when it changes.
#[derive(Component, Default)]
pub(crate) struct ShipStationRows(Option<StationRow>);

/// The docked station as the block shows it.
#[derive(Clone, Debug, PartialEq)]
struct StationRow {
    name: String,
    /// The price line, or the owned line.
    line: String,
    /// The refusal line, or `None` when the buy is allowed or the station is
    /// already owned.
    refusal: Option<String>,
    owned: bool,
}

/// The docked station a buy reads: the player ship, the station, whether it
/// is for sale, its price and its name.
struct DockedStation {
    player: Entity,
    station: Entity,
    sale: StationSaleType,
    price_cr: u64,
    name: String,
}

/// The player ship, the partner it is docked with, and every station fact
/// the buy rule reads.
#[derive(SystemParam)]
pub(crate) struct StationSale<'w, 's> {
    items: Option<Res<'w, GameItems>>,
    player: Query<
        'w,
        's,
        (Entity, Option<&'static DockedShip>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    connections: Query<'w, 's, &'static DockingConnection>,
    stations: Query<
        'w,
        's,
        (
            Option<&'static Name>,
            &'static ShipInventory,
            &'static ShipCredits,
            Option<&'static Children>,
            Has<DerelictShipMarker>,
            Has<OwnedStationMarker>,
            Has<GeneratedBodyId>,
        ),
        With<StationMarker>,
    >,
    sections: Query<'w, 's, &'static SectionBuildConfig, With<SectionMarker>>,
}

impl StationSale<'_, '_> {
    /// The station the player ship is docked with, or `None` while it is not
    /// docked, docked with a ship, or the item catalog is missing.
    fn docked(&self) -> Option<DockedStation> {
        let items = self.items.as_deref()?;
        let (player, docked) = self.player.single().ok()?;
        let connection = self.connections.get(docked?.connection).ok()?;
        let station = if connection.first_ship == player {
            connection.second_ship
        } else {
            connection.first_ship
        };
        let (name, stock, credits, children, derelict, owned, generated) =
            self.stations.get(station).ok()?;
        let sale = if owned {
            StationSaleType::Owned
        } else if generated && !derelict {
            StationSaleType::ForSale
        } else {
            StationSaleType::NotForSale
        };
        let recipes = children
            .into_iter()
            .flatten()
            .filter_map(|child| self.sections.get(*child).ok())
            .map(|config| &config.0.base.build);
        Some(DockedStation {
            player,
            station,
            sale,
            price_cr: station_price_cr(items, recipes, stock, credits.0),
            name: name.map_or_else(|| "This station".to_string(), ToString::to_string),
        })
    }
}

/// Why a buy of the station named `name` is refused; the block and the
/// handler share it.
fn station_refusal_text(name: &str, refusal: &StationBuyRefusalType) -> String {
    match refusal {
        StationBuyRefusalType::NotDocked => "buy: dock at a living station".to_string(),
        StationBuyRefusalType::NotForSale => format!("buy: no civilization sells {name}"),
        StationBuyRefusalType::AlreadyOwned => format!("buy: {name} is yours already"),
        StationBuyRefusalType::InsufficientCredits { need, have } => {
            format!("buy: {name} costs {need} cr, the ship holds {have} cr")
        }
    }
}

/// Build the block above the lost-section list: a header and the rows
/// [`update_ship_station_list`] fills.
pub(crate) fn spawn_ship_station_list(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            ShipStationList,
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
            list.spawn(themed_label("This station", 13.0, UiColor::Accent));
            list.spawn((
                ShipStationRows::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
            ));
        });
}

/// Show the block while the player is docked at a station for sale or owned,
/// and respawn its rows when the price, the refusal or the ownership changes.
pub(crate) fn update_ship_station_list(
    mut commands: Commands,
    sale: StationSale,
    q_credits: Query<&ShipCredits>,
    mut q_list: Query<&mut Node, With<ShipStationList>>,
    mut q_rows: Query<(Entity, &mut ShipStationRows)>,
) {
    let Ok(mut list) = q_list.single_mut() else {
        return;
    };
    let Ok((rows_entity, mut shown)) = q_rows.single_mut() else {
        return;
    };
    let row = sale.docked().and_then(|docked| match docked.sale {
        StationSaleType::NotForSale => None,
        StationSaleType::Owned => Some(StationRow {
            line: "Yours: it refuses no hull, and its stock sells to you at bid".to_string(),
            name: docked.name,
            refusal: None,
            owned: true,
        }),
        StationSaleType::ForSale => {
            let own_cr = q_credits.get(docked.player).map_or(0, |credits| credits.0);
            Some(StationRow {
                line: format!(
                    "{} cr: twice its sections, stock and credits at ask",
                    docked.price_cr
                ),
                refusal: plan_station_buy(docked.price_cr, Some(docked.sale), own_cr)
                    .err()
                    .map(|refusal| station_refusal_text(&docked.name, &refusal)),
                name: docked.name,
                owned: false,
            })
        }
    });
    let display = if row.is_none() {
        Display::None
    } else {
        Display::Flex
    };
    if list.display != display {
        list.display = display;
    }
    if shown.0 == row {
        return;
    }
    commands.entity(rows_entity).despawn_children();
    if let Some(row) = &row {
        commands.entity(rows_entity).with_children(|cell| {
            cell.spawn(themed_label(&row.name, 13.0, UiColor::Primary));
            let tone = if row.owned {
                UiColor::Accent
            } else {
                UiColor::Body
            };
            cell.spawn(themed_label(&row.line, 12.0, tone));
            match &row.refusal {
                Some(refusal) => {
                    cell.spawn(themed_label(refusal, 12.0, UiColor::Danger));
                }
                None if !row.owned => {
                    cell.spawn((
                        Name::new(STATION_BUY_BUTTON),
                        compact_button(ButtonSpec::new("Buy station").fit().primary()),
                    ))
                    .observe(on_station_buy_button);
                }
                None => {}
            }
        });
    }
    shown.0 = row;
}

/// Raise a [`StationBuyCommand`] when the Buy button is clicked.
fn on_station_buy_button(
    _activate: On<Activate>,
    mut buys: MessageWriter<StationBuyCommand>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    buys.write(StationBuyCommand);
    play_menu_select(&mut commands, bank.as_deref());
}

/// Apply [`StationBuyCommand`]s by the [`plan_station_buy`] rule: charge the
/// price with a `StationBuy` credit change on the player ship, queue one
/// `StationBought` report and mark the station [`OwnedStationMarker`]. The
/// price goes to the civilization, so the station's balance does not move.
/// A refusal changes nothing and shows its reason on the note line.
///
/// # Panics
///
/// On a station for sale with no scenario id: every generated station wears
/// one.
pub(crate) fn apply_ship_station_buy_commands(
    mut messages: MessageReader<StationBuyCommand>,
    mut runtime: ResMut<ShipRuntime>,
    mut commands: Commands,
    sale: StationSale,
    q_ids: Query<&EntityId>,
    // The player ship; a station's balance is not touched, and the sale
    // reads every station's.
    mut q_credits: Query<(&mut ShipCredits, &mut ShipInventoryReports), Without<StationMarker>>,
) {
    if messages.is_empty() {
        return;
    }
    // One buy per frame: the marker lands with the commands, so a second
    // command this frame would plan against a station not yet owned.
    messages.clear();
    let Some(docked) = sale.docked() else {
        runtime.note = Some((
            station_refusal_text("", &StationBuyRefusalType::NotDocked),
            2.5,
        ));
        return;
    };
    let Ok((mut own_cr, mut reports)) = q_credits.get_mut(docked.player) else {
        panic!("station buy: the player ship carries credits and reports");
    };
    let plan = match plan_station_buy(docked.price_cr, Some(docked.sale), own_cr.0) {
        Ok(plan) => plan,
        Err(refusal) => {
            runtime.note = Some((station_refusal_text(&docked.name, &refusal), 2.5));
            return;
        }
    };
    let station_id = q_ids.get(docked.station).unwrap_or_else(|_| {
        panic!(
            "station buy: station {:?} is for sale but has no scenario id",
            docked.station
        )
    });
    reports.change_credits(&mut own_cr, plan.own_cr, CreditChangeCauseType::StationBuy);
    reports.0.push(ShipInventoryReport::StationBought {
        station: station_id.0.clone(),
        price_cr: plan.price_cr,
    });
    commands.entity(docked.station).insert(OwnedStationMarker);
    runtime.note = Some((
        format!("bought {} for {} cr", docked.name, plan.price_cr),
        2.5,
    ));
    info!(
        "station buy: {} ({:?}) is the player's for {} cr",
        docked.name, docked.station, plan.price_cr
    );
}
