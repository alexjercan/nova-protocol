//! The TAB interface's Recycle pane: strip and loot the docked wreck.
//!
//! The pane's tab shows only while the player ship is docked with a hull it
//! may loot (neutralized, or a derelict carrying `LootableShipMarker`) that
//! is not a living station (see [`RecycleDock`]); at a living station the
//! Station tab shows in its place. The pane holds one scrolling panel:
//!
//! - "Strip the wreck": the parts the whole hull is expected to give, the
//!   time it takes, a Strip button that raises a [`HullStripCommand`], and
//!   the running strip's progress. The strip goes on with the pane shut.
//! - "The wreck's hold": each stack the wreck carries and its credits, each
//!   with a take button that writes the Inventory pane's own take command.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `strip` | The strip block, the strip action handler and the strip timer. |
//! | `loot` | The wreck's hold and its take buttons. |

mod loot;
mod strip;

#[cfg(test)]
mod tests;

use bevy::{
    ecs::system::SystemParam, input::mouse::MouseWheel, picking::hover::Hovered, prelude::*,
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{screen::drive_wheel_scroll, theme::UiColor};

pub(crate) use self::{loot::*, strip::*};
pub use self::{
    loot::{RECYCLE_TAKE_CREDITS, RECYCLE_TAKE_PREFIX},
    strip::{HullStripCommand, RECYCLE_STRIP_BUTTON, RECYCLE_STRIP_PROGRESS},
};
use crate::{
    inventory::{InventoryPaneSystems, InventoryRuntime},
    pane::{panel_paint, themed_label},
};

/// Glob-import surface: `use nova_interface::recycle::prelude::*`.
pub mod prelude {
    // Named by module path, as in `ship::prelude`.
    pub use super::{
        loot::{RECYCLE_TAKE_CREDITS, RECYCLE_TAKE_PREFIX},
        strip::{HullStripCommand, RECYCLE_STRIP_BUTTON, RECYCLE_STRIP_PROGRESS},
        RecycleRuntime, RECYCLE_PANEL,
    };
}

/// The name of the Recycle pane's scrolling panel.
pub const RECYCLE_PANEL: &str = "RecyclePanel";

/// Seconds an action result stays on the note line.
const NOTE_SECS: f32 = 2.5;

/// The Recycle pane's state that outlives its body: the note line.
#[derive(Resource, Default, Debug)]
pub struct RecycleRuntime {
    /// A transient note, such as why a strip was refused or stopped, and the
    /// seconds it has left.
    note: Option<(String, f32)>,
}

impl RecycleRuntime {
    /// The transient note the pane shows, while it lasts.
    pub fn note(&self) -> Option<&str> {
        self.note.as_ref().map(|(note, _)| note.as_str())
    }

    /// Show `note` on the note line for [`NOTE_SECS`].
    pub(crate) fn say(&mut self, note: String) {
        self.note = Some((note, NOTE_SECS));
    }
}

/// The wreck the player ship is docked with, if any. The Recycle tab shows
/// only while there is one.
#[derive(SystemParam)]
pub(crate) struct RecycleDock<'w, 's> {
    player: Query<
        'w,
        's,
        &'static DockedShip,
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    connections: Query<'w, 's, &'static DockingConnection>,
    wrecks: Query<
        'w,
        's,
        (),
        (
            Or<(With<NeutralizedMarker>, With<LootableShipMarker>)>,
            Or<(Without<StationMarker>, With<DerelictShipMarker>)>,
        ),
    >,
}

impl RecycleDock<'_, '_> {
    /// Whether the player ship is docked with a hull it may loot that is not
    /// a living station.
    pub(crate) fn docked(&self) -> bool {
        self.player.single().ok().is_some_and(|docked| {
            self.connections
                .get(docked.connection)
                .is_ok_and(|connection| {
                    self.wrecks.contains(connection.first_ship)
                        || self.wrecks.contains(connection.second_ship)
                })
        })
    }
}

/// The Recycle pane's scrolling panel.
#[derive(Component)]
pub(crate) struct RecyclePanelMarker;

/// The Recycle pane's note line.
#[derive(Component)]
pub(crate) struct RecycleNote;

/// Build the Recycle pane body: one panel with the strip block, the loot
/// list and the note line.
pub(crate) fn recycle_body(body: &mut ChildSpawnerCommands) {
    body.spawn((
        RecyclePanelMarker,
        Name::new(RECYCLE_PANEL),
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
        spawn_recycle_strip(panel);
        spawn_recycle_loot(panel);
        panel.spawn((RecycleNote, themed_label("", 12.0, UiColor::Accent)));
    });
}

/// Wheel-scroll the panel while the pointer is over it.
pub(crate) fn scroll_recycle_panel(
    mut wheel: MessageReader<MouseWheel>,
    q_panel: Query<
        (&mut ScrollPosition, Option<&ComputedNode>, Option<&Hovered>),
        With<RecyclePanelMarker>,
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

/// Count the note down on the real clock and show it; a take's result from
/// the Inventory pane's handler shows while the pane has no note of its own.
pub(crate) fn update_recycle_note(
    time: Res<Time<Real>>,
    mut runtime: ResMut<RecycleRuntime>,
    inventory: Option<Res<InventoryRuntime>>,
    mut q_note: Query<&mut Text, With<RecycleNote>>,
) {
    if let Some((_, remaining)) = runtime.note.as_mut() {
        *remaining -= time.delta_secs().min(0.25);
        if *remaining <= 0.0 {
            runtime.note = None;
        }
    }
    let note = runtime
        .note()
        .or_else(|| {
            inventory
                .as_deref()
                .and_then(|inventory| inventory.note.as_ref())
                .map(|(note, _)| note.as_str())
        })
        .unwrap_or_default();
    for mut text in &mut q_note {
        if text.0 != note {
            text.0 = note.to_string();
        }
    }
}

/// Drives the Recycle pane's strip, its loot list and the strip timer.
pub(crate) struct RecyclePanePlugin;

impl Plugin for RecyclePanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RecycleRuntime>();
        app.add_message::<HullStripCommand>();
        // A take button writes the Inventory pane's command; its handler
        // reads it the same frame.
        app.configure_sets(Update, RecyclePaneSystems.before(InventoryPaneSystems));
        app.add_systems(
            Update,
            (
                apply_hull_strip_commands,
                // Runs with the pane shut too: a strip goes on in flight.
                advance_hull_strip,
                scroll_recycle_panel,
                update_recycle_strip,
                update_recycle_loot,
                update_recycle_note,
            )
                .chain()
                .in_set(RecyclePaneSystems),
        );
    }
}

/// System set for the Recycle pane's per-frame work and the strip timer. It
/// runs before `InventoryPaneSystems`, which applies the take commands its
/// loot list writes.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecyclePaneSystems;
