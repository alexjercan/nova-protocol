//! The TAB interface's Station pane: what the docked station sells.
//!
//! The pane's tab shows only while the player ship is docked at a living
//! station (see [`StationDock`]). The pane is one scrolling column of
//! blocks, each under its own rule:
//!
//! - "This station": a card with the station's name, its price and a Buy
//!   button that raises a `StationBuyCommand`, while a civilization keeps it.
//!   Docked at a station the player owns, the name wears an `OWNED` tag.
//! - "Your ships": a card per ship the player owns parked at the station,
//!   with its 3D view, its name and length, a Board button that raises a
//!   `HullBuyCommand`, and Info (`hull_info`). Shown only while it has a card.
//! - "Ships for sale": a card per hull the station sells, with its 3D view,
//!   its name, length and price, a Buy button that raises a `HullBuyCommand`,
//!   and Info.
//! - "Escorts": the fleet's slots, then a card per escort the station hires
//!   out, with its 3D view, its name, length and price, a Hire button that
//!   raises an `EscortHireCommand`, and Info.
//!
//! Cards fill a grid: as many columns as fit, sharing the width evenly. A
//! card's price is a credit figure, painted Danger with its verb disabled
//! while the balance is short. A verb that cannot work is hidden or
//! disabled by its [`VerbGate`](nova_ui::prelude::VerbGate), with one tag
//! word for a cause the pane does not show (`ALLIES ONLY`).
//!
//! Every action's result (bought, boarded, hired, or refused) plays one
//! cue: the interface click for a result, the deny buzz for a refusal
//! ([`ActionCue`]); the balance, the lists and the fleet show the change.
//! The action buttons do not click on press: the outcome is the one sound.
//!
//! The empty word `Empty` shows while the station offers none of the lists
//! and parks none of the player's ships. Refit (rebuild, swap, add) stays in
//! the Ship pane, beside the ship it changes; the wreck strip is in the
//! Inventory pane; the station's job board is in the Jobs pane
//! (`crate::jobs`).
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
//! | `settle` | The Settle action: the fine the Map chart's stance block pays at the docked station, its cue played by the Map pane. |

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
use nova_sound::prelude::*;
use nova_ui::{
    prelude::{card, control_row, metrics, text, verb_row, RowAlign, TextRole},
    screen::drive_wheel_scroll,
    theme::UiColorType,
};

pub use self::{
    escorts::EscortHireCommand,
    hull_info::HULL_INFO_CLOSE,
    settle::StanceSettleCommand,
    shipyard::HullBuyCommand,
    station_buy::{StationBuyCommand, STATION_BUY_ASK, STATION_BUY_BUTTON},
};
pub(crate) use self::{
    escorts::*, hull_info::*, hull_preview::*, settle::*, shipyard::*, station_buy::*,
};
use crate::{
    icons::InterfaceIcons,
    pane::{spawn_credit_figure, ActionCue},
};

/// Glob-import surface: `use nova_interface::station::prelude::*`.
pub mod prelude {
    // Named by module path, as in `ship::prelude`.
    pub use super::{
        escorts::EscortHireCommand,
        hull_info::HULL_INFO_CLOSE,
        settle::StanceSettleCommand,
        shipyard::HullBuyCommand,
        station_buy::{StationBuyCommand, STATION_BUY_ASK, STATION_BUY_BUTTON},
        StationRuntime, STATION_PANEL,
    };
}

/// The name of the Station pane's scrolling panel.
pub const STATION_PANEL: &str = "StationPanel";

/// The Station pane's state that outlives its body: the result cue and the
/// armed station buy.
#[derive(Resource, Default, Debug)]
pub struct StationRuntime {
    /// The last Buy, Board or Hire result's cue.
    pub cue: ActionCue,
    /// The first press of the station Buy armed it: the button reads
    /// Confirm and the next press buys. Escape, another button, a pane switch,
    /// closing the interface and a refused buy disarm it.
    pub(crate) confirm_buy: bool,
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

/// The line inside the panel shown while none of the lists is.
#[derive(Component)]
pub(crate) struct StationEmptyLine;

/// Build the Station pane body: one scrolling column with the station block,
/// the hull list, the escort list and the empty word. Each list is a block
/// under its own rule; the cards carry the panel paint.
pub(crate) fn station_body(body: &mut ChildSpawnerCommands) {
    body.spawn((
        StationPanelMarker,
        Name::new(STATION_PANEL),
        Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(metrics::GAP_L),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        Hovered::default(),
    ))
    .with_children(|panel| {
        spawn_station_sale_block(panel);
        spawn_station_hull_list(panel);
        spawn_station_escort_list(panel);
        panel.spawn((StationEmptyLine, text("Empty", TextRole::Label)));
    });
}

/// The grid of offer cards inside a list: as many [`OFFER_CELL_PX`] columns
/// as fit, sharing the rest of the width evenly.
pub(crate) fn offer_rows() -> Node {
    Node {
        display: Display::Grid,
        grid_template_columns: vec![RepeatedGridTrack::minmax(
            GridTrackRepetition::AutoFill,
            MinTrackSizingFunction::Px(OFFER_CELL_PX),
            MaxTrackSizingFunction::Fraction(1.0),
        )],
        grid_auto_rows: vec![GridTrack::auto()],
        column_gap: px(metrics::GAP_L),
        row_gap: px(metrics::GAP_L),
        ..default()
    }
}

/// One offer card in [`offer_rows`]: it fills its grid cell.
pub(crate) fn offer_cell() -> impl Bundle {
    card()
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
    // the card's width and the render target's aspect. The image is
    // absolute so its measure cannot size the card.
    cell.spawn(Node {
        width: percent(100),
        aspect_ratio: Some(HULL_PREVIEW_PX.x as f32 / HULL_PREVIEW_PX.y as f32),
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
    cell.spawn(text(name, TextRole::Value));
    cell.spawn(text(length, TextRole::Body));
}

/// A card's foot: the price at its start, painted Danger while the balance
/// is `short` of it, and the verbs `verbs` spawns at its end. `price` `None`
/// leaves the start blank. The foot sits at the card's bottom, so feet line
/// up across a grid row.
pub(crate) fn spawn_card_foot(
    cell: &mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    price: Option<(u64, bool)>,
    verbs: impl FnOnce(&mut ChildSpawnerCommands),
) {
    cell.spawn(Node {
        width: percent(100),
        margin: UiRect::top(auto()),
        flex_wrap: FlexWrap::Wrap,
        row_gap: px(metrics::GAP_S),
        ..control_row(JustifyContent::SpaceBetween)
    })
    .with_children(|foot| {
        match price {
            Some((price_cr, short)) => {
                let tone = if short {
                    UiColorType::Danger
                } else {
                    UiColorType::Accent
                };
                spawn_credit_figure(foot, icons, price_cr, TextRole::Value, tone, ());
            }
            None => {
                foot.spawn(Node::default());
            }
        }
        foot.spawn(verb_row(RowAlign::End)).with_children(verbs);
    });
}

/// Least width of one offer card, in logical px: three fit a 1024 px window.
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

/// Play the cue a result queued, and show the empty line while no list
/// shows.
pub(crate) fn update_station_panel(
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
) {
    runtime.cue.tick(bank.as_deref(), &mut commands);
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
        app.add_observer(disarm_station_buy_on_other_press);
        app.add_systems(
            Update,
            (
                end_hull_info,
                disarm_station_buy,
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
