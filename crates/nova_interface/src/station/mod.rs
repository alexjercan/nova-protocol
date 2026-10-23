//! The TAB interface's Station pane: what the docked station sells.
//!
//! The pane's tab shows only while the player ship is docked at a living
//! station (see [`StationDock`]). The pane holds a notice line over one
//! scrolling panel:
//!
//! - "This station": the station's price and a Buy button that raises a
//!   `StationBuyCommand`, while a civilization keeps it. Docked at a station
//!   the player owns, it says so.
//! - "Your ships": a card per ship the player owns parked at the station,
//!   with its 3D view, its name and length, a Board button that raises a
//!   `HullBuyCommand`, and Info (`hull_info`). Shown only while it has a card.
//! - "Ships for sale": a card per hull the station sells, with its 3D view,
//!   its name, length and price, a Buy button that raises a `HullBuyCommand`,
//!   and Info.
//! - "Escorts for hire": the fleet's slots in its header, then a card per
//!   escort the station hires out, with its 3D view, its name, length and
//!   price, a Hire button that raises an `EscortHireCommand`, and Info.
//!
//! A card's price is a credit figure, with `(missing <figure>)` in red beside
//! it while the balance is short. A refused action keeps its button, greyed,
//! with the reason as its tooltip.
//!
//! Every action's result (bought, boarded, hired, settled, or refused) shows
//! on the notice line above the panel for four seconds, green for a result
//! and red for a refusal, and plays one cue: the interface click for a
//! result, the deny buzz for a refusal (`play_outcome_cue`). The line never
//! scrolls with the panel, so it stays in view however long the lists run.
//! The action buttons do not click on press: the outcome is the one sound.
//!
//! A line says so when the station offers none of the lists and parks none
//! of the player's ships. Refit (rebuild, swap, add) stays in the Ship pane,
//! beside the ship it changes; the wreck strip is in the Inventory pane; the
//! station's job board is in the Jobs pane (`crate::jobs`).
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `station_buy` | The "This station" block and the station buy action handler. |
//! | `shipyard` | The "Your ships" and "Ships for sale" lists and the buy and board action handler. |
//! | `hull_preview` | The 3D view of each hull on a "Ships for sale" or "Escorts for hire" card. |
//! | `escorts` | The "Escorts for hire" list and the hire action handler. |
//! | `hull_info` | A card's Info: the hull in the Ship pane's view, read-only. |
//! | `settle` | The Settle action: the fine the Map chart's stance block pays at the docked station. |

mod escorts;
mod hull_info;
mod hull_preview;
mod settle;
mod shipyard;
mod station_buy;
#[cfg(test)]
mod tests;

use bevy::{
    ecs::system::SystemParam, input::mouse::MouseWheel, picking::hover::Hovered, prelude::*,
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{screen::drive_wheel_scroll, theme::UiColor, widget::ThemedText};

pub use self::{
    escorts::EscortHireCommand,
    hull_info::HULL_INFO_CLOSE,
    settle::StanceSettleCommand,
    shipyard::HullBuyCommand,
    station_buy::{StationBuyCommand, STATION_BUY_BUTTON},
};
pub(crate) use self::{
    escorts::*, hull_info::*, hull_preview::*, settle::*, shipyard::*, station_buy::*,
};
use crate::{
    icons::InterfaceIcons,
    pane::{panel_paint, play_outcome_cue, spawn_credit_figure, themed_label, ActionOutcomeType},
};

/// Glob-import surface: `use nova_interface::station::prelude::*`.
pub mod prelude {
    // Named by module path, as in `ship::prelude`.
    pub use super::{
        escorts::EscortHireCommand,
        hull_info::HULL_INFO_CLOSE,
        settle::StanceSettleCommand,
        shipyard::HullBuyCommand,
        station_buy::{StationBuyCommand, STATION_BUY_BUTTON},
        StationRuntime, STATION_NOTICE, STATION_PANEL,
    };
}

/// The name of the Station pane's scrolling panel.
pub const STATION_PANEL: &str = "StationPanel";

/// The name of the Station pane's notice line, above the panel.
pub const STATION_NOTICE: &str = "StationNotice";

/// Seconds an action result stays on the notice line, on the real clock:
/// the interface holds the virtual one. Matches the HUD notice's dwell.
const NOTE_SECS: f32 = 4.0;

/// The Station pane's state that outlives its body: the notice line.
#[derive(Resource, Default, Debug)]
pub struct StationRuntime {
    /// The last action result, its outcome and the seconds it has left.
    pub(crate) note: Option<(ActionOutcomeType, String, f32)>,
    /// Set by [`say`](Self::say), taken by [`update_station_panel`] once it
    /// plays the cue. It waits for the sound bank, so a result raised before
    /// the bank lands is heard, not lost; it plays once whether or not the
    /// pane body stands.
    pending_cue: Option<ActionOutcomeType>,
}

impl StationRuntime {
    /// The transient note the pane shows, while it lasts.
    pub fn note(&self) -> Option<&str> {
        self.note.as_ref().map(|(_, note, _)| note.as_str())
    }

    /// The transient notice the pane shows, with its outcome, while it lasts.
    pub fn notice(&self) -> Option<(ActionOutcomeType, &str)> {
        self.note
            .as_ref()
            .map(|(outcome, note, _)| (*outcome, note.as_str()))
    }

    /// Show `note` on the notice line for [`NOTE_SECS`] in `outcome`'s tone,
    /// and queue its cue. A second result the same frame replaces the first
    /// and its cue, so one frame plays at most one cue.
    pub(crate) fn say(&mut self, outcome: ActionOutcomeType, note: String) {
        self.note = Some((outcome, note, NOTE_SECS));
        self.pending_cue = Some(outcome);
    }
}

/// The living station the player ship is docked with, if any. The Station
/// tab shows only while there is one.
#[derive(SystemParam)]
pub(crate) struct StationDock<'w, 's> {
    player: Query<
        'w,
        's,
        &'static DockedShip,
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    connections: Query<'w, 's, &'static DockingConnection>,
    stations: Query<'w, 's, (), (With<StationMarker>, Without<DerelictShipMarker>)>,
}

impl StationDock<'_, '_> {
    /// Whether the player ship is docked with a living station.
    pub(crate) fn docked(&self) -> bool {
        self.player.single().ok().is_some_and(|docked| {
            self.connections
                .get(docked.connection)
                .is_ok_and(|connection| {
                    self.stations.contains(connection.first_ship)
                        || self.stations.contains(connection.second_ship)
                })
        })
    }
}

/// The Station pane's scrolling panel.
#[derive(Component)]
pub(crate) struct StationPanelMarker;

/// The notice line above the panel: the transient action result.
#[derive(Component)]
pub(crate) struct StationNoticeMarker;

/// The line inside the panel shown while none of the lists is.
#[derive(Component)]
pub(crate) struct StationEmptyLine;

/// Height of the notice line, in logical px: a 13 px line's box, so the
/// panel under it does not move when a notice comes and goes.
const NOTICE_LINE_PX: f32 = 18.0;

/// Build the Station pane body: the notice line, then one panel with the
/// station block, the hull list, the escort list and the empty line.
pub(crate) fn station_body(body: &mut ChildSpawnerCommands) {
    body.spawn((
        StationNoticeMarker,
        Name::new(STATION_NOTICE),
        themed_label("", 13.0, UiColor::Label),
        Node {
            min_height: px(NOTICE_LINE_PX),
            flex_shrink: 0.0,
            ..default()
        },
    ));
    body.spawn((
        StationPanelMarker,
        Name::new(STATION_PANEL),
        Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(12)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        panel_paint(),
        ScrollPosition::default(),
        Hovered::default(),
    ))
    .with_children(|panel| {
        spawn_station_sale_block(panel);
        spawn_station_hull_list(panel);
        spawn_station_escort_list(panel);
        panel.spawn((
            StationEmptyLine,
            themed_label(
                "This station sells no ships, hires no escorts and is not for sale.",
                13.0,
                UiColor::Label,
            ),
        ));
    });
}

/// The row of offer cards inside a list: cards wrap across the wide panel.
pub(crate) fn offer_rows() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(24),
        row_gap: px(12),
        ..default()
    }
}

/// Spawn the head of a hull card in `cell`: the 3D view of `hull`, the
/// hull's `name` line and its `length` line.
pub(crate) fn spawn_hull_card_head(
    cell: &mut ChildSpawnerCommands,
    hull: Entity,
    name: &str,
    length: &str,
) {
    // `sync_hull_previews` hands the image its hull's view. The frame takes
    // the card's width: a fixed width here makes taffy measure the labels at
    // min-content width and grow the list by hundreds of px. The image is
    // absolute so its measure cannot size the card either. The height keeps
    // the render target's aspect at the card's width.
    cell.spawn(Node {
        height: px(OFFER_CELL_PX * HULL_PREVIEW_PX.y as f32 / HULL_PREVIEW_PX.x as f32),
        flex_shrink: 0.0,
        ..default()
    })
    .with_child((
        HullPreviewSlot { hull },
        ImageNode::default(),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
    ));
    cell.spawn(themed_label(name, 13.0, UiColor::Primary));
    cell.spawn(themed_label(length, 12.0, UiColor::Body));
}

/// A card's price block: the figure at 15 px, muted while short, then
/// `(missing <figure>)` in Danger at the same size.
pub(crate) fn spawn_card_price(
    cell: &mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    price_cr: u64,
    short_cr: Option<u64>,
) {
    cell.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        column_gap: px(10),
        ..default()
    })
    .with_children(|line| {
        let tone = if short_cr.is_some() {
            UiColor::Secondary
        } else {
            UiColor::Accent
        };
        spawn_credit_figure(line, icons, price_cr, 15.0, tone, ());
        if let Some(short_cr) = short_cr {
            // The ")" sits flush on the digits; the gap only splits the word
            // from the figure.
            line.spawn(Node {
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|missing| {
                missing
                    .spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: px(5),
                        ..default()
                    })
                    .with_children(|word| {
                        word.spawn(themed_label("(missing", 15.0, UiColor::Danger));
                        spawn_credit_figure(word, icons, short_cr, 15.0, UiColor::Danger, ());
                    });
                missing.spawn(themed_label(")", 15.0, UiColor::Danger));
            });
        }
    });
}

/// One offer card in [`offer_rows`].
pub(crate) fn offer_cell() -> Node {
    Node {
        width: px(OFFER_CELL_PX),
        flex_direction: FlexDirection::Column,
        row_gap: px(4),
        ..default()
    }
}

/// The row of a card's buttons: Buy, Board or Hire, then Info.
pub(crate) fn card_actions() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        column_gap: px(8),
        ..default()
    }
}

/// Width of one offer card, in logical px: three fit a 1024 px window.
const OFFER_CELL_PX: f32 = 280.0;

/// Wheel-scroll the panel while the pointer is over it.
pub(crate) fn scroll_station_panel(
    mut wheel: MessageReader<MouseWheel>,
    q_panel: Query<
        (&mut ScrollPosition, Option<&ComputedNode>, Option<&Hovered>),
        With<StationPanelMarker>,
    >,
) {
    if !q_panel
        .iter()
        .any(|(_, _, hovered)| hovered.is_some_and(Hovered::get))
    {
        wheel.clear();
        return;
    }
    drive_wheel_scroll(wheel, q_panel);
}

/// Count the notice down on the real clock and paint it in its outcome's
/// tone, play the cue a result queued, and show the empty line while no list
/// shows.
pub(crate) fn update_station_panel(
    time: Res<Time<Real>>,
    mut runtime: ResMut<StationRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    q_lists: Query<
        &Node,
        (
            Or<(
                With<StationSaleBlock>,
                With<StationHullList>,
                With<StationEscortList>,
            )>,
            Without<StationEmptyLine>,
        ),
    >,
    mut q_empty: Query<&mut Node, With<StationEmptyLine>>,
    mut q_notice: Query<(&mut Text, &mut ThemedText), With<StationNoticeMarker>>,
) {
    if let Some((.., remaining)) = runtime.note.as_mut() {
        *remaining -= time.delta_secs().min(0.25);
        if *remaining <= 0.0 {
            runtime.note = None;
        }
    }
    if bank.is_some() {
        if let Some(outcome) = runtime.pending_cue.take() {
            play_outcome_cue(&mut commands, bank.as_deref(), outcome);
        }
    }
    let (note, tone) = match runtime.notice() {
        Some((ActionOutcomeType::Done, note)) => (note, UiColor::Nominal),
        Some((ActionOutcomeType::Refused, note)) => (note, UiColor::Danger),
        None => ("", UiColor::Label),
    };
    for (mut text, mut themed) in &mut q_notice {
        if text.0 != note {
            text.0 = note.to_string();
        }
        if themed.color != tone {
            themed.color = tone;
        }
    }
    let any_list = q_lists.iter().any(|node| node.display != Display::None);
    let display = if any_list {
        Display::None
    } else {
        Display::Flex
    };
    for mut node in &mut q_empty {
        if node.display != display {
            node.display = display;
        }
    }
}

/// Drives the Station pane's lists and actions.
pub(crate) struct StationPanePlugin;

impl Plugin for StationPanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StationRuntime>();
        app.init_resource::<HullPreviews>();
        app.init_resource::<HullInfoLine>();
        app.add_message::<HullBuyCommand>();
        app.add_message::<EscortHireCommand>();
        app.add_message::<StationBuyCommand>();
        app.add_message::<StanceSettleCommand>();
        app.add_systems(
            Update,
            (
                end_hull_info,
                sync_hull_info_header,
                apply_station_buy_commands,
                apply_station_hull_buy_commands,
                apply_station_escort_hire_commands,
                apply_station_settle_commands,
                scroll_station_panel,
                update_station_sale_block,
                update_station_hull_list,
                update_station_escort_list,
                sync_hull_previews,
                mirror_hull_previews,
                update_station_panel,
            )
                .chain()
                .in_set(StationPaneSystems),
        );
    }
}

/// System set for the Station pane's per-frame work. It runs before
/// `ShipPaneSystems`: a hull buy clears the Ship pane's selection, so the
/// Ship pane does not read it back the same frame from sections queued to go.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StationPaneSystems;
