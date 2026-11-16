//! The Ship pane's Distress block: the SOS call a stranded open-world player
//! can place, and the call while it runs.
//!
//! The block shows while the player ship flies undocked in the open world,
//! with no station berth under way, and is stranded ([`rescue_quote`]): no
//! working thruster, flight computer or free docking port. A ship that lost
//! only its guns flies and docks, so it is not stranded. The block names
//! why, and its button shows what the rescue would charge now. A click asks
//! [`find_rescue_station`] for the nearest station that answers and places
//! the [`ShipRescueCall`]; no station answering refuses on the note line. A
//! ship whose only thruster, flight computer or port is live but not working,
//! with no lost one to rebuild, is refused at the click too: the rescue
//! rebuilds lost sections only, and would dock a ship that still cannot fly.
//! While the call runs the block shows its bar, its time left and Cancel.
//! The call runs on the game clock, which this pane stops.
//!
//! Touch this module when changing how the pane offers, shows or cancels a
//! rescue call.

use avian3d::prelude::Position;
use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::Meters3;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::{OpenWorldSession, StationBerth};
use nova_ship::prelude::*;
use nova_ui::{
    theme::UiColorType,
    widget::{ButtonSpec, ThemedFill, ThemedRadius},
};
use nova_world_base::prelude::find_rescue_station;

use super::{design_section_code, ShipRuntime};
use crate::pane::{compact_button, control_row, divider, themed_label};

/// The Distress block, hidden unless the player ship is stranded or runs a
/// call.
#[derive(Component)]
pub(crate) struct ShipRescueBlock;

/// A part of [`ShipRescueBlock`] that shows only in one state.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShipRescuePart {
    /// The SOS offer: why the ship is stranded and the call button.
    Offer,
    /// The running call: its line, bar and Cancel.
    Running,
}

/// A live text of [`ShipRescueBlock`].
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShipRescueField {
    /// Why the ship is stranded.
    Reasons,
    /// The bill the call would take now.
    Bill,
    /// The game seconds the running call has left.
    Time,
}

/// The running call's progress bar fill.
#[derive(Component)]
pub(crate) struct ShipRescueFill;

/// The offer's note under the title. The call runs on the game clock, which
/// this pane stops.
pub(crate) const SHIP_RESCUE_NOTE: &str = "The call runs only while you fly.";

/// The pane note when no station answers a call.
pub(crate) const SHIP_RESCUE_NO_STATION: &str = "no station answers the call";

/// The pane note when the ship is docked or berthing: it is not stranded.
pub(crate) const SHIP_RESCUE_DOCKED: &str = "the ship is docked: no SOS";

/// Build the Distress block: a title, the clock note, the offer and the
/// running call, filled by [`update_ship_rescue`].
pub(crate) fn spawn_ship_rescue(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            ShipRescueBlock,
            Name::new("ShipRescue"),
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|block| {
            block.spawn(divider());
            block.spawn(themed_label("Distress", 13.0, UiColorType::Accent));
            block.spawn(themed_label(SHIP_RESCUE_NOTE, 12.0, UiColorType::TextBody));
            block
                .spawn((
                    ShipRescuePart::Offer,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                ))
                .with_children(|offer| {
                    offer.spawn((
                        ShipRescueField::Reasons,
                        themed_label("", 12.0, UiColorType::Text),
                    ));
                    offer
                        .spawn(control_row(JustifyContent::FlexStart))
                        .with_children(|line| {
                            line.spawn((
                                ShipRescueField::Bill,
                                themed_label("", 12.0, UiColorType::TextBody),
                                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                                Node {
                                    flex_grow: 1.0,
                                    flex_basis: Val::Px(0.0),
                                    min_width: Val::Px(0.0),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                            ));
                            line.spawn((
                                Name::new("ShipSosCall"),
                                compact_button(ButtonSpec::new("Call SOS").fit()),
                            ))
                            .observe(on_sos_button);
                        });
                });
            block
                .spawn((
                    ShipRescuePart::Running,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(3.0),
                        ..default()
                    },
                ))
                .with_children(|running| {
                    running
                        .spawn(control_row(JustifyContent::FlexStart))
                        .with_children(|line| {
                            line.spawn((
                                themed_label("SOS sent", 12.0, UiColorType::Text),
                                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                                Node {
                                    flex_grow: 1.0,
                                    flex_basis: Val::Px(0.0),
                                    min_width: Val::Px(0.0),
                                    ..default()
                                },
                            ));
                            line.spawn((
                                ShipRescueField::Time,
                                themed_label("", 12.0, UiColorType::TextBody),
                                TextLayout::new(Justify::Right, LineBreak::NoWrap),
                                Node {
                                    flex_shrink: 0.0,
                                    ..default()
                                },
                            ));
                            line.spawn((
                                Name::new("ShipSosCancel"),
                                compact_button(ButtonSpec::new("Cancel").fit().ghost()),
                            ))
                            .observe(on_sos_cancel_button);
                        });
                    running
                        .spawn((
                            Node {
                                height: Val::Px(6.0),
                                flex_shrink: 0.0,
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                            ThemedFill::alpha(UiColorType::TextSecondary, 0.25),
                            ThemedRadius::control(),
                        ))
                        .with_children(|track| {
                            track.spawn((
                                ShipRescueFill,
                                Node {
                                    width: Val::Percent(0.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                ThemedFill::new(UiColorType::Accent),
                            ));
                        });
                });
        });
}

/// The pane words for each reason, such as `no drive, no free port`.
fn reason_words(reasons: &[StrandedType]) -> String {
    reasons
        .iter()
        .map(|reason| match reason {
            StrandedType::NoThruster => "no drive",
            StrandedType::NoFlightComputer => "no flight computer",
            StrandedType::NoDockingPort => "no free port",
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The reasons line of a stranded ship, such as `Stranded: no drive, no
/// free port.`
pub(crate) fn stranded_line(reasons: &[StrandedType]) -> String {
    format!("Stranded: {}.", reason_words(reasons))
}

/// The bill line, or the note that the rescue cannot fix the ship: a
/// reason with no lost section to rebuild.
fn offer_line(quote: &RescueQuote) -> String {
    if quote.unfixed.is_empty() {
        rescue_bill_line(quote)
    } else {
        unfixed_note(quote)
    }
}

/// Why a rescue cannot fix the ship, such as `SOS cannot fix: no drive`.
fn unfixed_note(quote: &RescueQuote) -> String {
    format!("SOS cannot fix: {}", reason_words(&quote.unfixed))
}

/// The bill line of a quote: what the call takes now, and that it is all
/// the ship has when it holds less than the bill.
pub(crate) fn rescue_bill_line(quote: &RescueQuote) -> String {
    let rebuilds = match quote.chain.len() {
        0 => String::new(),
        1 => ", 1 rebuild".to_string(),
        count => format!(", {count} rebuilds"),
    };
    let short = if u64::from(quote.bill.charged_cr) < quote.bill.owed_cr {
        " (all you have)"
    } else {
        ""
    };
    format!(
        "{}{short}{rebuilds}, {:.0} s",
        cr_text(quote.bill.charged_cr),
        RESCUE_SECS
    )
}

/// The player ship the block reads, when it flies a designed hull.
fn player_ship(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next()
}

/// Show the offer while the open-world player ship is stranded, the running
/// call while it runs, and nothing otherwise; fill the texts and the bar.
pub(crate) fn update_ship_rescue(world: &mut World) {
    let player = player_ship(world);
    let open_world = world.contains_resource::<OpenWorldSession>();
    let (state, texts, progress) = match player {
        Some(player) => match world.get::<ShipRescueCall>(player) {
            Some(call) => (
                Some(ShipRescuePart::Running),
                vec![(
                    ShipRescueField::Time,
                    format!("{:.0} s", call.remaining_secs().ceil()),
                )],
                call.progress(),
            ),
            None => match (open_world && world.get::<StationBerth>(player).is_none())
                .then(|| rescue_quote(world, player))
                .flatten()
            {
                Some(quote) => (
                    Some(ShipRescuePart::Offer),
                    vec![
                        (ShipRescueField::Reasons, stranded_line(&quote.reasons)),
                        (ShipRescueField::Bill, offer_line(&quote)),
                    ],
                    0.0,
                ),
                None => (None, Vec::new(), 0.0),
            },
        },
        None => (None, Vec::new(), 0.0),
    };
    let mut q_block = world.query_filtered::<&mut Node, With<ShipRescueBlock>>();
    for mut node in q_block.iter_mut(world) {
        let display = if state.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    let mut q_part = world.query::<(&ShipRescuePart, &mut Node)>();
    for (part, mut node) in q_part.iter_mut(world) {
        let display = if state == Some(*part) {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    let mut q_text = world.query::<(&ShipRescueField, &mut Text)>();
    for (field, mut text) in q_text.iter_mut(world) {
        if let Some((_, value)) = texts.iter().find(|(shown, _)| shown == field) {
            if text.0 != *value {
                text.0.clone_from(value);
            }
        }
    }
    let mut q_fill = world.query_filtered::<&mut Node, With<ShipRescueFill>>();
    for mut node in q_fill.iter_mut(world) {
        let width = Val::Percent(progress * 100.0);
        if node.width != width {
            node.width = width;
        }
    }
}

/// Place the rescue call of `player`: the nearest station that answers,
/// the chain [`rescue_quote`] plans now, and the note that says so. A ship
/// docked or berthing, no longer stranded, one the chain cannot fix, or no
/// station answering, places nothing.
pub(crate) fn place_rescue_call(world: &mut World, player: Entity) {
    let note = |world: &mut World, text: String| {
        world.resource_mut::<ShipRuntime>().note = Some((text, 2.5));
    };
    if world.get::<ShipRescueCall>(player).is_some() {
        return;
    }
    if world.get::<DockedShip>(player).is_some() || world.get::<StationBerth>(player).is_some() {
        note(world, SHIP_RESCUE_DOCKED.to_string());
        return;
    }
    let Some(quote) = rescue_quote(world, player) else {
        note(world, "the ship is no longer stranded".to_string());
        return;
    };
    if !quote.unfixed.is_empty() {
        info!(
            "place_rescue_call: refused; nothing lost to rebuild for {:?}",
            quote.unfixed
        );
        note(world, unfixed_note(&quote));
        return;
    }
    let Some(from) = world
        .get::<Position>(player)
        .map(|position| Meters3::from_engine(position.0))
    else {
        return;
    };
    let station = match find_rescue_station(world, from) {
        Ok(Some(station)) => station,
        Ok(None) => {
            note(world, SHIP_RESCUE_NO_STATION.to_string());
            return;
        }
        Err(fault) => {
            error!("place_rescue_call: the station search failed: {fault}");
            note(world, SHIP_RESCUE_NO_STATION.to_string());
            return;
        }
    };
    let design = world
        .get::<ShipDesignSections>(player)
        .expect("rescue_quote read the design");
    let chain = quote
        .chain
        .iter()
        .map(|section| RescueRebuild {
            section: section.clone(),
            code: design_section_code(design, section)
                .expect("a rescue chain section is a design section"),
        })
        .collect();
    info!(
        "place_rescue_call: SOS to station {}, rebuilding {:?}",
        station.id, quote.chain
    );
    world.entity_mut(player).insert(ShipRescueCall::new(
        station.id,
        station.civilization,
        station.spawn,
        chain,
    ));
    note(
        world,
        format!("SOS sent; a station answers in {RESCUE_SECS:.0} s"),
    );
}

/// Place the call when the SOS button is clicked.
fn on_sos_button(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| {
        if let Some(player) = player_ship(world) {
            place_rescue_call(world, player);
        }
    });
}

/// Cancel the running call when its Cancel button is clicked, and say so.
fn on_sos_cancel_button(
    _activate: On<Activate>,
    mut commands: Commands,
    q_player: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    mut runtime: ResMut<ShipRuntime>,
) {
    let Ok(player) = q_player.single() else {
        return;
    };
    commands.trigger(RescueCallCancelRequest { entity: player });
    runtime.note = Some(("cancelled the SOS call".to_string(), 2.5));
}
