//! The Station pane's "This station" block and the station Buy action.
//!
//! While the player ship is docked at a living station a civilization keeps
//! (generated with the world, so it wears a [`GeneratedBodyId`]), the pane
//! shows a card with the station's name, its price ([`station_price_cr`])
//! and a Buy button, disabled with the price painted Danger while the
//! balance is short. A station a civilization sells to its allies only
//! shows no price and a disabled Buy beside an `ALLIES ONLY` tag. A click
//! raises a [`StationBuyCommand`] on the second press: the first arms the
//! buy, the button reads `Confirm` in the danger variant beside Cancel, and
//! a line under them reads `Buy station?` (see
//! [`StationRuntime::confirm_buy`]).
//! [`apply_station_buy_commands`] plans it with
//! [`plan_station_buy`], charges the price as a `StationBuy` credit change,
//! queues one `StationBought` report and makes the station the player's
//! ([`owned_station`]). Docked at a station the player owns, the name wears
//! an `OWNED` tag. A ship, a dead
//! station and a station a scenario placed by hand show no block: no
//! civilization sells them.
//!
//! Touch this module when changing what the block shows or what Buy does.

use bevy::{ecs::system::SystemParam, prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, EntityId, GeneratedBodyId};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_sound::prelude::*;
use nova_ui::{prelude::*, theme::UiColorType};

use super::{offer_cell, offer_rows, spawn_card_foot, StationRuntime};
use crate::{
    icons::InterfaceIcons,
    pane::{gated_verb, ActionOutcomeType, InterfacePaneType},
};

/// The name of the station Buy button.
pub const STATION_BUY_BUTTON: &str = "StationBuy";

/// The name of the Cancel button the armed Buy draws where Buy was.
const STATION_BUY_CANCEL_BUTTON: &str = "StationBuyCancel";

/// Width of that Cancel button, in logical px. The `Buy station` button is
/// about 90 px wide; Cancel must cover all of it.
const BUY_BOX_PX: f32 = 96.0;

/// The name of the question line the armed Buy button draws under it.
pub const STATION_BUY_ASK: &str = "StationBuyAsk";

/// A request to buy the docked station, raised by the "This station" Buy
/// button.
#[derive(Message, Clone, Debug)]
pub struct StationBuyCommand;

/// The block's container, hidden while the player is not docked at a station
/// a civilization keeps or the player owns.
#[derive(Component)]
pub(crate) struct StationSaleBlock;

/// The rows inside [`StationSaleBlock`], and the row they were built from, so
/// the rows respawn only when it changes.
#[derive(Component, Default)]
pub(crate) struct StationSaleRows(Option<StationRow>);

/// The docked station as the block shows it.
#[derive(Clone, Debug, PartialEq)]
struct StationRow {
    name: String,
    /// Which tag, if any, the name wears and whether Buy shows.
    sale: StationSaleType,
    /// The price of a station for sale, or `None` while it is not for sale.
    price_cr: Option<u64>,
    /// The balance is short of the price.
    short: bool,
    /// How Buy station answers the row.
    gate: VerbGate,
    /// Whether the first press armed the buy and the button asks to confirm.
    confirming: bool,
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
            &'static Allegiance,
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
        let (name, stock, credits, children, derelict, owned, generated, side) =
            self.stations.get(station).ok()?;
        let sale = if owned {
            StationSaleType::Owned
        } else if !generated || derelict {
            StationSaleType::NotForSale
        } else if *side == Allegiance::Player {
            StationSaleType::ForSale
        } else {
            StationSaleType::NotAllied
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

/// How Buy station answers a refused buy: hidden where no buy can happen,
/// tagged when the civilization sells to allies only, disabled while the
/// balance is short (the price shows red).
fn station_gate(refusal: Option<&StationBuyRefusalType>) -> VerbGate {
    match refusal {
        None => VerbGate::Ready,
        Some(
            StationBuyRefusalType::NotDocked
            | StationBuyRefusalType::NotForSale
            | StationBuyRefusalType::AlreadyOwned,
        ) => VerbGate::Hidden,
        Some(StationBuyRefusalType::NotAllied) => VerbGate::Tagged("ALLIES ONLY"),
        Some(StationBuyRefusalType::InsufficientCredits { .. }) => VerbGate::Disabled,
    }
}

/// Build the block: a heading and the card [`update_station_sale_block`]
/// fills, in a card grid so it takes an offer card's width.
pub(crate) fn spawn_station_sale_block(panel: &mut ChildSpawnerCommands) {
    let mut block = spawn_block(
        panel,
        "This station",
        (StationSaleBlock, Name::new("StationSaleBlock")),
    );
    block
        .entry::<Node>()
        .and_modify(|mut node| node.display = Display::None);
    block.with_children(|block| {
        block.spawn(offer_rows()).with_children(|grid| {
            grid.spawn((StationSaleRows::default(), offer_cell()));
        });
    });
}

/// Show the block while the player is docked at a station for sale or owned,
/// and respawn its rows when the price, the refusal or the ownership changes.
pub(crate) fn update_station_sale_block(
    mut commands: Commands,
    runtime: Res<StationRuntime>,
    sale: StationSale,
    q_credits: Query<&ShipCredits>,
    icons: Res<InterfaceIcons>,
    mut q_list: Query<&mut Node, With<StationSaleBlock>>,
    mut q_rows: Query<(Entity, &mut StationSaleRows)>,
) {
    let Ok(mut list) = q_list.single_mut() else {
        return;
    };
    let Ok((rows_entity, mut shown)) = q_rows.single_mut() else {
        return;
    };
    let row = sale.docked().and_then(|docked| match docked.sale {
        StationSaleType::NotForSale => None,
        StationSaleType::NotAllied => Some(StationRow {
            sale: docked.sale,
            price_cr: None,
            short: false,
            gate: station_gate(Some(&StationBuyRefusalType::NotAllied)),
            confirming: false,
            name: docked.name,
        }),
        StationSaleType::Owned => Some(StationRow {
            sale: docked.sale,
            price_cr: None,
            short: false,
            name: docked.name,
            gate: VerbGate::Ready,
            confirming: false,
        }),
        StationSaleType::ForSale => {
            let own_cr = q_credits.get(docked.player).map_or(0, |credits| credits.0);
            let refusal = plan_station_buy(docked.price_cr, Some(docked.sale), own_cr).err();
            Some(StationRow {
                sale: docked.sale,
                price_cr: Some(docked.price_cr),
                short: matches!(
                    refusal,
                    Some(StationBuyRefusalType::InsufficientCredits { .. })
                ),
                confirming: runtime.confirm_buy && refusal.is_none(),
                gate: station_gate(refusal.as_ref()),
                name: docked.name,
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
            cell.spawn(control_row(JustifyContent::FlexStart))
                .with_children(|head| {
                    head.spawn(text(&row.name, TextRole::Value));
                    if row.sale == StationSaleType::Owned {
                        head.spawn(tag("OWNED", UiColorType::Nominal));
                    }
                });
            if row.sale == StationSaleType::Owned {
                return;
            }
            let price = row.price_cr.map(|price_cr| (price_cr, row.short));
            spawn_card_foot(cell, &icons, price, |verbs| {
                if row.confirming {
                    // Cancel takes the whole box Buy had, so the second
                    // press of a double click lands on it, not on Confirm.
                    // Any button but Buy disarms, so it needs no handler of
                    // its own.
                    verbs
                        .spawn(Node {
                            width: px(BUY_BOX_PX),
                            flex_shrink: 0.0,
                            ..default()
                        })
                        .with_children(|slot| {
                            slot.spawn((
                                Name::new(STATION_BUY_CANCEL_BUTTON),
                                button(ButtonSpec::control("Cancel").fill()),
                                ButtonCueType::Back,
                            ));
                        });
                    verbs
                        .spawn((
                            Name::new(STATION_BUY_BUTTON),
                            button(ButtonSpec::control("Confirm").danger()),
                            ButtonCueType::Owned,
                        ))
                        .observe(on_station_buy_button);
                } else {
                    gated_verb(
                        verbs,
                        STATION_BUY_BUTTON.to_string(),
                        ButtonSpec::control("Buy station"),
                        row.gate.clone(),
                    )
                    .observe(on_station_buy_button);
                }
            });
            if row.confirming {
                cell.spawn((
                    Name::new(STATION_BUY_ASK),
                    text("Buy station?", TextRole::Body),
                ));
            }
        });
    }
    shown.0 = row;
}

/// Take the Buy button's step: the first press arms the buy, the second
/// raises a [`StationBuyCommand`]. The arming press plays the select cue; the
/// buy is silent, as its outcome plays the one cue.
fn on_station_buy_button(
    _activate: On<Activate>,
    mut runtime: ResMut<StationRuntime>,
    mut buys: MessageWriter<StationBuyCommand>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if runtime.confirm_buy {
        runtime.confirm_buy = false;
        buys.write(StationBuyCommand);
    } else {
        runtime.confirm_buy = true;
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Disarm the buy when the player presses any other button.
pub(crate) fn disarm_station_buy_on_other_press(
    activate: On<Activate>,
    q_names: Query<&Name>,
    mut runtime: ResMut<StationRuntime>,
) {
    if !runtime.confirm_buy {
        return;
    }
    let is_buy = q_names
        .get(activate.entity)
        .is_ok_and(|name| name.as_str() == STATION_BUY_BUTTON);
    if !is_buy {
        runtime.confirm_buy = false;
    }
}

/// Disarm the buy once the Station pane is not the open pane, or the buy is
/// refused: not docked at a station for sale, or short of its price.
pub(crate) fn disarm_station_buy(
    mut runtime: ResMut<StationRuntime>,
    pause: Res<State<PauseStates>>,
    pane: Res<InterfacePaneType>,
    sale: StationSale,
    q_credits: Query<&ShipCredits>,
) {
    if !runtime.confirm_buy {
        return;
    }
    let open = *pause.get() == PauseStates::Interface && *pane == InterfacePaneType::Station;
    let allowed = sale.docked().is_some_and(|docked| {
        let own_cr = q_credits.get(docked.player).map_or(0, |credits| credits.0);
        plan_station_buy(docked.price_cr, Some(docked.sale), own_cr).is_ok()
    });
    if !(open && allowed) {
        runtime.confirm_buy = false;
    }
}

/// Apply [`StationBuyCommand`]s by the [`plan_station_buy`] rule: charge the
/// price with a `StationBuy` credit change on the player ship, queue one
/// `StationBought` report and make the station the player's
/// ([`owned_station`]: owned, on the Player side, no longer named for its
/// civilization). The price goes to the civilization, so the station's
/// balance does not move.
/// A refusal changes nothing and plays the deny cue.
///
/// # Panics
///
/// On a station for sale with no scenario id: every generated station wears
/// one.
pub(crate) fn apply_station_buy_commands(
    mut messages: MessageReader<StationBuyCommand>,
    mut runtime: ResMut<StationRuntime>,
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
        runtime.cue.raise(ActionOutcomeType::Refused);
        return;
    };
    let Ok((mut own_cr, mut reports)) = q_credits.get_mut(docked.player) else {
        panic!("station buy: the player ship carries credits and reports");
    };
    let plan = match plan_station_buy(docked.price_cr, Some(docked.sale), own_cr.0) {
        Ok(plan) => plan,
        Err(refusal) => {
            debug!("nova_interface: refused: {refusal:?}");
            runtime.cue.raise(ActionOutcomeType::Refused);
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
    commands.entity(docked.station).insert(owned_station());
    runtime.cue.raise(ActionOutcomeType::Done);
    info!(
        "station buy: {} ({:?}) is the player's for {} cr",
        docked.name, docked.station, plan.price_cr
    );
}
