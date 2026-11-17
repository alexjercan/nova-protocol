//! The Ship pane's Distress block: the SOS call a stranded open-world player
//! can place, and the call while it runs.
//!
//! The block shows while the player ship flies undocked in the open world,
//! with no station berth under way, and is stranded ([`rescue_quote`]): no
//! working thruster, flight computer or free docking port. A ship that lost
//! only its guns flies and docks, so it is not stranded. The block tags why
//! (`NO DRIVE`, `NO CTL`, `NO PORT`), gives what the rescue would charge now,
//! the rebuilds it carries and its time, and Call SOS. A click asks
//! [`find_rescue_station`] for the nearest station that answers and places
//! the [`ShipRescueCall`]; no station answering tags Call SOS `NO STATION`. A
//! ship whose only thruster, flight computer or port is live but not working,
//! with no lost one to rebuild, has Call SOS tagged `NO FIX`: the rescue
//! rebuilds lost sections only, and would dock a ship that still cannot fly.
//! While the call runs the block shows its time left, its bar and Cancel;
//! the running part is the only sign of a placed or cancelled call. The call
//! runs on the game clock, which this pane stops.
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
    prelude::{
        bar, button, control_row, metrics, spawn_block, spawn_kv, tag, text, verb_row, Bar,
        RowAlign, TextRole, VerbGate,
    },
    theme::UiColorType,
    widget::ButtonSpec,
};
use nova_world_base::prelude::find_rescue_station;

use super::{design_section_code, ShipRuntime};

/// The Distress block, hidden unless the player ship is stranded or runs a
/// call.
#[derive(Component)]
pub(crate) struct ShipRescueBlock;

/// A part of [`ShipRescueBlock`] that shows only in one state.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShipRescuePart {
    /// The SOS offer: why the ship is stranded, the bill and Call SOS.
    Offer,
    /// The running call: its time, bar and Cancel.
    Running,
}

/// A live text of [`ShipRescueBlock`].
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShipRescueField {
    /// The credits the call would take now.
    Fee,
    /// The lost sections the call would rebuild.
    Rebuilds,
    /// The game seconds the running call has left.
    Time,
}

/// The tag of one reason the ship is stranded, shown while it holds.
#[derive(Component)]
pub(crate) struct ShipRescueReason(StrandedType);

/// The Call SOS button.
#[derive(Component)]
pub(crate) struct ShipSosCall;

/// The running call's progress bar.
#[derive(Component)]
pub(crate) struct ShipRescueBar;

/// Call SOS's tag when no station answers.
const RESCUE_NO_STATION: &str = "NO STATION";

/// Build the Distress block: the offer and the running call, filled by
/// [`update_ship_rescue`].
pub(crate) fn spawn_ship_rescue(panel: &mut ChildSpawnerCommands) {
    let mut block = spawn_block(
        panel,
        "Distress",
        (ShipRescueBlock, Name::new("ShipRescue")),
    );
    block
        .entry::<Node>()
        .and_modify(|mut node| node.display = Display::None);
    block.with_children(|block| {
        block
            .spawn((
                ShipRescuePart::Offer,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(metrics::GAP),
                    ..default()
                },
            ))
            .with_children(|offer| {
                offer
                    .spawn(control_row(JustifyContent::FlexStart))
                    .with_children(|tags| {
                        for (reason, word) in [
                            (StrandedType::NoThruster, "NO DRIVE"),
                            (StrandedType::NoFlightComputer, "NO CTL"),
                            (StrandedType::NoDockingPort, "NO PORT"),
                        ] {
                            tags.spawn((ShipRescueReason(reason), tag(word, UiColorType::Danger)));
                        }
                    });
                spawn_kv(
                    offer,
                    "Fee",
                    (ShipRescueField::Fee, text("", TextRole::Value)),
                );
                spawn_kv(
                    offer,
                    "Rebuilds",
                    (ShipRescueField::Rebuilds, text("", TextRole::Value)),
                );
                spawn_kv(
                    offer,
                    "Time",
                    text(format!("{RESCUE_SECS:.0} s"), TextRole::Value),
                );
                offer.spawn(verb_row(RowAlign::End)).with_children(|row| {
                    row.spawn((
                        ShipSosCall,
                        Name::new("ShipSosCall"),
                        button(ButtonSpec::control("Call SOS")),
                        VerbGate::Ready,
                    ))
                    .observe(on_sos_button);
                });
            });
        block
            .spawn((
                ShipRescuePart::Running,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(metrics::GAP),
                    ..default()
                },
            ))
            .with_children(|running| {
                spawn_kv(
                    running,
                    "SOS",
                    (ShipRescueField::Time, text("", TextRole::Value)),
                );
                running.spawn((ShipRescueBar, bar(0.0, UiColorType::Accent)));
                running.spawn(verb_row(RowAlign::End)).with_children(|row| {
                    row.spawn((
                        Name::new("ShipSosCancel"),
                        button(ButtonSpec::control("Cancel").ghost()),
                    ))
                    .observe(on_sos_cancel_button);
                });
            });
    });
}

/// The player ship the block reads, when it flies a designed hull.
fn player_ship(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next()
}

/// Show the offer while the open-world player ship is stranded, the running
/// call while it runs, and nothing otherwise; fill the tags, the texts, Call
/// SOS's gate and the bar. A `NO STATION` tag lasts while the offer shows.
pub(crate) fn update_ship_rescue(world: &mut World) {
    let player = player_ship(world);
    let open_world = world.contains_resource::<OpenWorldSession>();
    let mut texts = Vec::new();
    let mut reasons = Vec::new();
    let mut progress = 0.0;
    let mut call_gate = VerbGate::Ready;
    let state = match player {
        Some(player) => match world.get::<ShipRescueCall>(player) {
            Some(call) => {
                texts.push((
                    ShipRescueField::Time,
                    format!("{:.0} s", call.remaining_secs().ceil()),
                ));
                progress = call.progress();
                Some(ShipRescuePart::Running)
            }
            None => match (open_world && world.get::<StationBerth>(player).is_none())
                .then(|| rescue_quote(world, player))
                .flatten()
            {
                Some(quote) => {
                    texts.push((ShipRescueField::Fee, cr_text(quote.bill.charged_cr)));
                    texts.push((ShipRescueField::Rebuilds, quote.chain.len().to_string()));
                    reasons.clone_from(&quote.reasons);
                    call_gate = if !quote.unfixed.is_empty() {
                        VerbGate::Tagged("NO FIX")
                    } else {
                        world
                            .resource::<ShipRuntime>()
                            .rescue_refused
                            .map_or(VerbGate::Ready, VerbGate::Tagged)
                    };
                    Some(ShipRescuePart::Offer)
                }
                None => None,
            },
        },
        None => None,
    };
    if state != Some(ShipRescuePart::Offer)
        && world.resource::<ShipRuntime>().rescue_refused.is_some()
    {
        world.resource_mut::<ShipRuntime>().rescue_refused = None;
    }
    let show = |node: &mut Node, shown: bool| {
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    };
    let mut q_block = world.query_filtered::<&mut Node, With<ShipRescueBlock>>();
    for mut node in q_block.iter_mut(world) {
        show(&mut node, state.is_some());
    }
    let mut q_part = world.query::<(&ShipRescuePart, &mut Node)>();
    for (part, mut node) in q_part.iter_mut(world) {
        show(&mut node, state == Some(*part));
    }
    let mut q_reason = world.query::<(&ShipRescueReason, &mut Node)>();
    for (reason, mut node) in q_reason.iter_mut(world) {
        show(&mut node, reasons.contains(&reason.0));
    }
    let mut q_text = world.query::<(&ShipRescueField, &mut Text)>();
    for (field, mut text) in q_text.iter_mut(world) {
        if let Some((_, value)) = texts.iter().find(|(shown, _)| shown == field) {
            if text.0 != *value {
                text.0.clone_from(value);
            }
        }
    }
    let mut q_gate = world.query_filtered::<&mut VerbGate, With<ShipSosCall>>();
    for mut gate in q_gate.iter_mut(world) {
        gate.set_if_neq(call_gate.clone());
    }
    let mut q_bar = world.query_filtered::<&mut Bar, With<ShipRescueBar>>();
    for mut bar in q_bar.iter_mut(world) {
        if bar.0 != progress {
            bar.0 = progress;
        }
    }
}

/// Place the rescue call of `player`: the nearest station that answers and
/// the chain [`rescue_quote`] plans now. The running part shows the call. A
/// ship docked or berthing, no longer stranded, or one the chain cannot fix
/// places nothing: the block hides or tags Call SOS already. No station
/// answering tags Call SOS `NO STATION`.
pub(crate) fn place_rescue_call(world: &mut World, player: Entity) {
    if world.get::<ShipRescueCall>(player).is_some()
        || world.get::<DockedShip>(player).is_some()
        || world.get::<StationBerth>(player).is_some()
    {
        return;
    }
    let Some(quote) = rescue_quote(world, player) else {
        return;
    };
    if !quote.unfixed.is_empty() {
        info!(
            "place_rescue_call: refused; nothing lost to rebuild for {:?}",
            quote.unfixed
        );
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
            world.resource_mut::<ShipRuntime>().rescue_refused = Some(RESCUE_NO_STATION);
            return;
        }
        Err(fault) => {
            error!("place_rescue_call: the station search failed: {fault}");
            world.resource_mut::<ShipRuntime>().rescue_refused = Some(RESCUE_NO_STATION);
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
}

/// Place the call when the SOS button is clicked.
fn on_sos_button(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| {
        if let Some(player) = player_ship(world) {
            place_rescue_call(world, player);
        }
    });
}

/// Cancel the running call when its Cancel button is clicked; the block
/// offers SOS again.
fn on_sos_cancel_button(
    _activate: On<Activate>,
    mut commands: Commands,
    q_player: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    if let Ok(player) = q_player.single() {
        commands.trigger(RescueCallCancelRequest { entity: player });
    }
}
