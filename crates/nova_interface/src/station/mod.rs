//! The TAB interface's Station pane: what the docked station sells.
//!
//! The pane's tab shows only while the player ship is docked at a living
//! station (see [`StationDock`]). The pane holds one scrolling panel:
//!
//! - "This station": the station's price and a Buy button that raises a
//!   `StationBuyCommand`, while a civilization keeps it. Docked at a station
//!   the player owns, it says so.
//! - "Ships for sale": each hull the station sells, with its price, the
//!   trade-in and a Buy button that raises a `HullBuyCommand`.
//! - "Escorts for hire": each escort the station hires out, with its price
//!   and a Hire button that raises an `EscortHireCommand`.
//!
//! A line says so when the station offers none of the three. Refit (rebuild,
//! swap, add) and the wreck strip stay in the Ship pane, beside the ship they
//! change.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `station_buy` | The "This station" block and the station buy action handler. |
//! | `shipyard` | The "Ships for sale" list and the buy action handler. |
//! | `escorts` | The "Escorts for hire" list and the hire action handler. |

mod escorts;
mod shipyard;
mod station_buy;

use bevy::{
    ecs::system::SystemParam, input::mouse::MouseWheel, picking::hover::Hovered, prelude::*,
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{screen::drive_wheel_scroll, theme::UiColor};

pub use self::{
    escorts::EscortHireCommand,
    shipyard::HullBuyCommand,
    station_buy::{StationBuyCommand, STATION_BUY_BUTTON},
};
pub(crate) use self::{escorts::*, shipyard::*, station_buy::*};
use crate::pane::{panel_paint, themed_label};

/// Glob-import surface: `use nova_interface::station::prelude::*`.
pub mod prelude {
    // Named by module path, as in `ship::prelude`.
    pub use super::{
        escorts::EscortHireCommand,
        shipyard::HullBuyCommand,
        station_buy::{StationBuyCommand, STATION_BUY_BUTTON},
        StationRuntime, STATION_PANEL,
    };
}

/// The name of the Station pane's scrolling panel.
pub const STATION_PANEL: &str = "StationPanel";

/// Seconds an action result stays on the note line.
const NOTE_SECS: f32 = 2.5;

/// The Station pane's state that outlives its body: the note line.
#[derive(Resource, Default, Debug)]
pub struct StationRuntime {
    /// A transient note, such as why a buy was refused, and the seconds it
    /// has left.
    pub(crate) note: Option<(String, f32)>,
}

impl StationRuntime {
    /// The transient note the pane shows, while it lasts.
    pub fn note(&self) -> Option<&str> {
        self.note.as_ref().map(|(note, _)| note.as_str())
    }

    /// Show `note` on the note line for [`NOTE_SECS`].
    pub(crate) fn say(&mut self, note: String) {
        self.note = Some((note, NOTE_SECS));
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

/// A text of the Station panel that [`update_station_panel`] refreshes.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StationPanelField {
    /// Shown while none of the three lists is.
    Empty,
    /// The transient action result.
    Note,
}

/// Build the Station pane body: one panel with the station block, the hull
/// list, the escort list, the empty line and the note line.
pub(crate) fn station_body(body: &mut ChildSpawnerCommands) {
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
            StationPanelField::Empty,
            themed_label(
                "This station sells no ships, hires no escorts and is not for sale.",
                13.0,
                UiColor::Label,
            ),
        ));
        panel.spawn((
            StationPanelField::Note,
            themed_label("", 12.0, UiColor::Accent),
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

/// One offer card in [`offer_rows`].
pub(crate) fn offer_cell() -> Node {
    Node {
        width: px(OFFER_CELL_PX),
        flex_direction: FlexDirection::Column,
        row_gap: px(4),
        ..default()
    }
}

/// Width of one offer card, in logical px: a refusal line wraps in two.
const OFFER_CELL_PX: f32 = 320.0;

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

/// Count the note down on the real clock, and show the empty line while no
/// list shows.
pub(crate) fn update_station_panel(
    time: Res<Time<Real>>,
    mut runtime: ResMut<StationRuntime>,
    q_lists: Query<
        &Node,
        (
            Or<(
                With<StationSaleBlock>,
                With<StationHullList>,
                With<StationEscortList>,
            )>,
            Without<StationPanelField>,
        ),
    >,
    mut q_field: Query<(&StationPanelField, &mut Node, Option<&mut Text>)>,
) {
    if let Some((_, remaining)) = runtime.note.as_mut() {
        *remaining -= time.delta_secs().min(0.25);
        if *remaining <= 0.0 {
            runtime.note = None;
        }
    }
    let any_list = q_lists.iter().any(|node| node.display != Display::None);
    for (field, mut node, text) in &mut q_field {
        match field {
            StationPanelField::Empty => {
                let display = if any_list {
                    Display::None
                } else {
                    Display::Flex
                };
                if node.display != display {
                    node.display = display;
                }
            }
            StationPanelField::Note => {
                let note = runtime.note().unwrap_or_default();
                if let Some(mut text) = text {
                    if text.0 != note {
                        text.0 = note.to_string();
                    }
                }
            }
        }
    }
}

/// Drives the Station pane's lists and actions.
pub(crate) struct StationPanePlugin;

impl Plugin for StationPanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StationRuntime>();
        app.add_message::<HullBuyCommand>();
        app.add_message::<EscortHireCommand>();
        app.add_message::<StationBuyCommand>();
        app.add_systems(
            Update,
            (
                apply_station_buy_commands,
                apply_station_hull_buy_commands,
                apply_station_escort_hire_commands,
                scroll_station_panel,
                update_station_sale_block,
                update_station_hull_list,
                update_station_escort_list,
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
